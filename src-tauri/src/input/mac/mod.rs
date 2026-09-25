// SPDX-License-Identifier: GPL-3.0-or-later
//! The expansion engine on macOS.
//!
//! Keystrokes are observed and, when a trigger completes, swallowed by a
//! CoreGraphics event tap; the expansion is then delivered by posting
//! synthetic keyboard events (or a Cmd+V for anything long). Every event Ampello
//! posts carries a marker so the tap can tell them from the user's typing.
//!
//! The tap needs the Accessibility and Input Monitoring permissions. Until
//! they are granted the hook thread keeps retrying, so expansion starts by
//! itself as soon as the user switches them on - no restart needed.
mod clipboard;
mod sys;

use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::state::Library;
use ampello_core::{db, CancelKey};
use ampello_core::engine::{BoundaryMode, Engine, Expansion, Key, Trigger};

use super::config::{ClipboardMode, Config, InjectionMode, TypingSpeed};
use super::{EngineStatus, ExpandedCallback};
use sys::*;

// Stamped on every event we post so the tap ignores it.
const MARKER: i64 = 0x414D_504C;

const KEY_RETURN: u16 = 36;
const KEY_DELETE: u16 = 51;

const KEY_V: u16 = 9;

const TYPEABLE_LIMIT: usize = 5_000;
const AUTO_TYPE_LIMIT: usize = 24;
const CANCELLED: &str = "cancelled";

const PERMISSION_HELP: &str = "Ampello needs permission to watch and send keystrokes. Open \
    System Settings > Privacy & Security, and switch Ampello on under both Accessibility and \
    Input Monitoring. Expansion starts as soon as you do.";

struct Shared {
    engine: Mutex<Engine>,
    config: Mutex<Config>,
    jobs: Sender<Job>,

    injecting: AtomicBool,
    last_target: AtomicI64,

    keys_seen: AtomicU64,
    expansions: AtomicU64,
    last_error: Mutex<Option<String>>,
}

static SHARED: OnceLock<Arc<Shared>> = OnceLock::new();

// The tap, so the callback can switch it back on when macOS turns it off.
static TAP: AtomicUsize = AtomicUsize::new(0);

static CANCEL: AtomicBool = AtomicBool::new(false);

enum Job {
    Expand { expansion: Expansion },
    InsertClipboard,
    Stop,
}

pub struct InputService {
    shared: Arc<Shared>,
    library: Arc<Library>,
    stop_hook: Mutex<Option<Arc<AtomicBool>>>,
    running: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
}

impl InputService {
    pub fn start(library: Arc<Library>, on_expanded: ExpandedCallback) -> Self {
        let (jobs, receiver) = mpsc::channel::<Job>();

        let shared = Arc::new(Shared {
            engine: Mutex::new(Engine::new()),
            config: Mutex::new(Config::default()),
            jobs,
            injecting: AtomicBool::new(false),
            last_target: AtomicI64::new(0),
            keys_seen: AtomicU64::new(0),
            expansions: AtomicU64::new(0),
            last_error: Mutex::new(None),
        });

        let service = Self {
            shared: Arc::clone(&shared),
            library: Arc::clone(&library),
            stop_hook: Mutex::new(None),
            running: Arc::new(AtomicBool::new(false)),
            error: Arc::new(Mutex::new(None)),
        };

        service.refresh();
        let _ = SHARED.set(Arc::clone(&shared));

        {
            let shared = Arc::clone(&shared);
            let library = Arc::clone(&library);
            thread::Builder::new()
                .name("ampello-injector".into())
                .spawn(move || worker(shared, receiver, library, on_expanded))
                .expect("could not start Ampello's injector thread");
        }

        service.install_hook();
        service
    }

    fn install_hook(&self) {
        let stop = Arc::new(AtomicBool::new(false));
        let running = Arc::clone(&self.running);
        let error = Arc::clone(&self.error);
        let flag = Arc::clone(&stop);

        let spawned = thread::Builder::new()
            .name("ampello-event-tap".into())
            .spawn(move || hook_thread(flag, running, error));
        match spawned {
            Ok(_) => *self.stop_hook.lock() = Some(stop),
            Err(error) => {
                log::error!("could not start the event tap thread: {error}");
                *self.error.lock() = Some("Ampello could not start its keyboard watcher.".into());
            }
        }
    }

    fn stop_hook(&self) {
        if let Some(stop) = self.stop_hook.lock().take() {
            stop.store(true, Ordering::SeqCst);
        }
    }

    pub fn restart(&self) {
        log::info!("restarting the expansion engine");
        self.stop_hook();
        thread::sleep(Duration::from_millis(700));

        self.shared.injecting.store(false, Ordering::Release);
        self.shared.engine.lock().reset();
        *self.shared.last_error.lock() = None;

        self.install_hook();
        self.refresh();
    }

    pub fn refresh(&self) {
        let loaded = self.library.db().with(|conn| {
            let settings = db::settings::load(conn)?;
            let triggers = db::snippets::enabled_triggers(conn)?;
            Ok((settings, triggers))
        });
        let (settings, triggers) = match loaded {
            Ok(value) => value,
            Err(error) => {
                log::error!("could not load triggers: {error}");
                return;
            }
        };

        *self.shared.config.lock() = Config {
            preserve_terminator: settings.preserve_boundary_char,
            restore_clipboard: settings.restore_clipboard,
            injection: InjectionMode::parse(&settings.injection_mode),
            typing: TypingSpeed::parse(&settings.typing_speed),
            clipboard: ClipboardMode::parse(&settings.clipboard_mode),
            attachment_settle_ms: settings.attachment_settle_ms.max(0) as u64,
            cancel: CancelKey::parse(&settings.cancel_key).unwrap_or_default(),
        };

        let mut engine = self.shared.engine.lock();
        engine.set_mode(BoundaryMode::parse(&settings.boundary_mode));
        engine.set_enabled(settings.expansion_enabled);
        engine.set_triggers(
            triggers
                .into_iter()
                .map(|(snippet_id, text)| Trigger { snippet_id, text })
                .collect(),
        );
        log::info!(
            "expansion engine: {} trigger(s), {}",
            engine.trigger_count(),
            if engine.is_enabled() { "on" } else { "off" }
        );
    }

    pub fn insert_clipboard(&self) {
        if self
            .shared
            .injecting
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            log::info!("clipboard shortcut ignored: an insertion is already running");
            return;
        }
        if self.shared.jobs.send(Job::InsertClipboard).is_err() {
            self.shared.injecting.store(false, Ordering::Release);
            log::warn!("clipboard shortcut: the injector thread is no longer running");
        }
    }

    pub fn shutdown(&self) {
        let _ = self.shared.jobs.send(Job::Stop);
        self.stop_hook();
    }

    pub fn status(&self) -> EngineStatus {
        let engine = self.shared.engine.lock();
        EngineStatus {
            running: self.running.load(Ordering::SeqCst),
            enabled: engine.is_enabled(),
            trigger_count: engine.trigger_count(),
            error: self.error.lock().clone(),
            platform: "macos".into(),
            keystrokes_seen: self.shared.keys_seen.load(Ordering::Relaxed),
            expansions: self.shared.expansions.load(Ordering::Relaxed),
            last_expansion_error: self.shared.last_error.lock().clone(),
        }
    }
}

fn hook_thread(stop: Arc<AtomicBool>, running: Arc<AtomicBool>, error: Arc<Mutex<Option<String>>>) {
    let mut asked = false;

    while !stop.load(Ordering::SeqCst) {
        let mask: u64 = (1 << KEY_DOWN)
            | (1 << LEFT_MOUSE_DOWN)
            | (1 << RIGHT_MOUSE_DOWN)
            | (1 << OTHER_MOUSE_DOWN);

        let tap = unsafe {
            CGEventTapCreate(
                SESSION_EVENT_TAP,
                HEAD_INSERT,
                TAP_OPTION_DEFAULT,
                mask,
                tap_callback,
                ptr::null_mut(),
            )
        };

        if tap.is_null() {
            *error.lock() = Some(PERMISSION_HELP.into());
            if !asked {
                asked = true;
                log::warn!("the event tap was refused; asking for permission");
                unsafe {
                    CGRequestListenEventAccess();
                    CGRequestPostEventAccess();
                }
            }
            thread::sleep(Duration::from_secs(2));
            continue;
        }

        unsafe {
            let source = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, tap, 0);
            let run_loop = CFRunLoopGetCurrent();
            CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
            TAP.store(tap as usize, Ordering::SeqCst);

            *error.lock() = None;
            running.store(true, Ordering::SeqCst);
            log::info!("keyboard event tap installed");

            while !stop.load(Ordering::SeqCst) {
                CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.5, false);
            }

            running.store(false, Ordering::SeqCst);
            TAP.store(0, Ordering::SeqCst);
            CGEventTapEnable(tap, false);
            CFRunLoopRemoveSource(run_loop, source, kCFRunLoopCommonModes);
            CFRelease(source);
            CFRelease(tap);
            log::info!("keyboard event tap removed");
        }
    }
}

unsafe extern "C" fn tap_callback(
    _proxy: *mut std::ffi::c_void,
    kind: u32,
    event: CGEventRef,
    _user: *mut std::ffi::c_void,
) -> CGEventRef {
    // A panic must not unwind into the system's event machinery.
    let swallow = catch_unwind(AssertUnwindSafe(|| on_event(kind, event))).unwrap_or(false);
    if swallow {
        ptr::null_mut()
    } else {
        event
    }
}

unsafe fn on_event(kind: u32, event: CGEventRef) -> bool {
    match kind {
        // macOS switches a tap off when a callback is slow or it is disturbed;
        // left off, expansion would just stop working.
        TAP_DISABLED_BY_TIMEOUT | TAP_DISABLED_BY_USER => {
            let tap = TAP.load(Ordering::SeqCst);
            if tap != 0 {
                CGEventTapEnable(tap as CFMachPortRef, true);
                log::warn!("the event tap was switched off by the system; switched back on");
            }
            false
        }
        LEFT_MOUSE_DOWN | RIGHT_MOUSE_DOWN | OTHER_MOUSE_DOWN => {
            if let Some(shared) = SHARED.get() {
                shared.engine.lock().reset();
            }
            false
        }
        KEY_DOWN => on_key(event),
        _ => false,
    }
}

unsafe fn on_key(event: CGEventRef) -> bool {
    let Some(shared) = SHARED.get() else {
        return false;
    };
    if CGEventGetIntegerValueField(event, FIELD_USER_DATA) == MARKER {
        return false;
    }
    if shared.injecting.load(Ordering::Acquire) {
        return false;
    }
    shared.keys_seen.fetch_add(1, Ordering::Relaxed);

    let target = CGEventGetIntegerValueField(event, FIELD_TARGET_PID);
    if shared.last_target.swap(target, Ordering::Relaxed) != target {
        shared.engine.lock().reset();
    }

    let keycode = CGEventGetIntegerValueField(event, FIELD_KEYCODE) as u16;
    let flags = CGEventGetFlags(event);

    // Cmd or Ctrl held: a shortcut, not text.
    if flags & (FLAG_COMMAND | FLAG_CONTROL) != 0 {
        shared.engine.lock().reset();
        return false;
    }

    if keycode == KEY_DELETE {
        let mut engine = shared.engine.lock();
        // Option+Delete removes a whole word, which the buffer cannot follow.
        if flags & FLAG_ALTERNATE != 0 {
            engine.reset();
        } else {
            engine.on_key(Key::Backspace);
        }
        return false;
    }

    let mut units = [0u16; 8];
    let mut length: usize = 0;
    CGEventKeyboardGetUnicodeString(event, units.len(), &mut length, units.as_mut_ptr());
    let text = String::from_utf16_lossy(&units[..length.min(units.len())]);
    if text.is_empty() {
        return false;
    }

    // Arrows, function keys, Escape and the like report control or
    // private-use characters. None of them are typed text.
    let typed = text.chars().all(|c| {
        matches!(c, '\r' | '\n' | '\t')
            || !(c.is_control() || ('\u{F700}'..='\u{F8FF}').contains(&c))
    });
    if !typed {
        shared.engine.lock().reset();
        return false;
    }

    let mut expansion = None;
    {
        let mut engine = shared.engine.lock();
        for c in text.chars() {
            let c = if c == '\r' { '\n' } else { c };
            if let Some(found) = engine.on_key(Key::Char(c)) {
                expansion = Some(found);
                break;
            }
        }
    }
    let Some(expansion) = expansion else {
        return false;
    };

    let config = *shared.config.lock();
    if config.preserve_terminator {
        shared
            .engine
            .lock()
            .note_injected_terminator(expansion.terminator);
    }

    shared.injecting.store(true, Ordering::Release);
    if shared.jobs.send(Job::Expand { expansion }).is_err() {
        shared.injecting.store(false, Ordering::Release);
        return false;
    }
    true
}

fn worker(
    shared: Arc<Shared>,
    jobs: mpsc::Receiver<Job>,
    library: Arc<Library>,
    on_expanded: ExpandedCallback,
) {
    while let Ok(job) = jobs.recv() {
        let expansion = match job {
            Job::Stop => break,
            Job::Expand { expansion } => expansion,
            Job::InsertClipboard => {
                let config = *shared.config.lock();
                let outcome = catch_unwind(AssertUnwindSafe(|| insert_clipboard(config)))
                    .unwrap_or_else(|_| Err("Ampello's injector panicked.".into()));
                shared.injecting.store(false, Ordering::Release);
                match outcome {
                    Ok(()) => {}
                    Err(error) if error == CANCELLED => log::info!("clipboard insertion cancelled"),
                    Err(error) => log::warn!("clipboard insertion failed: {error}"),
                }
                continue;
            }
        };

        let config = *shared.config.lock();
        let outcome = catch_unwind(AssertUnwindSafe(|| expand(&library, &expansion, config)))
            .unwrap_or_else(|_| Err("Ampello's injector panicked.".into()));
        shared.injecting.store(false, Ordering::Release);

        match outcome {
            Ok(()) => {
                shared.expansions.fetch_add(1, Ordering::Relaxed);
                *shared.last_error.lock() = None;
                on_expanded(&expansion.snippet_id);
            }
            Err(error) if error == CANCELLED => {
                log::info!("expansion of snippet {} cancelled", expansion.snippet_id);
                *shared.last_error.lock() = Some("Stopped with the cancel key part-way through.".into());
            }
            Err(error) => {
                log::warn!("expansion of snippet {} failed: {error}", expansion.snippet_id);
                *shared.last_error.lock() = Some(error);
            }
        }
    }
    log::info!("injector thread stopped");
}

fn insert_clipboard(config: Config) -> Result<(), String> {
    wait_for_modifiers_release(Duration::from_millis(1_200));

    if config.clipboard == ClipboardMode::Paste {
        let guard = Guard::new(config.typing, config.cancel);
        return paste_shortcut(&guard);
    }

    let Some(text) = clipboard::get_text()? else {
        log::info!("clipboard shortcut: the clipboard is not text, pasting instead");
        let guard = Guard::new(config.typing, config.cancel);
        return paste_shortcut(&guard);
    };
    if text.is_empty() {
        return Ok(());
    }
    let guard = Guard::new(config.typing, config.cancel);
    type_text(&text, &guard)
}

fn expand(library: &Library, expansion: &Expansion, config: Config) -> Result<(), String> {
    let db = library.db();
    let snippet = match db.with(|conn| db::snippets::get(conn, &expansion.snippet_id)) {
        Ok(snippet) => snippet,
        Err(error) => {
            let _ = type_char(expansion.terminator);
            return Err(error.to_string());
        }
    };
    let content = snippet.content;

    let store = db.attachments();
    let mut files = Vec::with_capacity(snippet.attachments.len());
    for attachment in &snippet.attachments {
        let path = store.path_of(&attachment.digest, &attachment.name);
        if path.is_file() {
            files.push(path);
        } else {
            log::warn!(
                "snippet {} refers to a file that is not in the store: {}",
                expansion.snippet_id,
                attachment.name
            );
        }
    }

    // The user is still holding the keys that finished the trigger; posted
    // events must not inherit them.
    wait_for_modifiers_release(Duration::from_millis(300));

    let guard = Guard::new(config.typing, config.cancel);
    erase(expansion.trigger.chars().count(), &guard)?;

    deliver_payload(
        &content,
        &files,
        snippet.attachments_first,
        snippet.strict_order,
        config,
        &guard,
    )?;

    if config.preserve_terminator {
        type_char(expansion.terminator)?;
    }
    if let Err(error) = db.with(|conn| db::snippets::record_usage(conn, &expansion.snippet_id)) {
        log::warn!("could not record snippet usage: {error}");
    }
    Ok(())
}

// --- Posting events -------------------------------------------------------

fn post_key(keycode: u16, down: bool, flags: u64, text: Option<&[u16]>) -> Result<(), String> {
    unsafe {
        let event = CGEventCreateKeyboardEvent(ptr::null_mut(), keycode, down);
        if event.is_null() {
            return Err("macOS would not create a keyboard event.".into());
        }
        CGEventSetFlags(event, flags);
        if let Some(text) = text {
            CGEventKeyboardSetUnicodeString(event, text.len(), text.as_ptr());
        }
        CGEventSetIntegerValueField(event, FIELD_USER_DATA, MARKER);
        CGEventPost(HID_EVENT_TAP, event);
        CFRelease(event);
    }
    Ok(())
}

fn press(keycode: u16, flags: u64) -> Result<(), String> {
    post_key(keycode, true, flags, None)?;
    post_key(keycode, false, flags, None)
}

fn press_text(text: &[u16]) -> Result<(), String> {
    post_key(0, true, 0, Some(text))?;
    post_key(0, false, 0, Some(text))
}

fn type_char(c: char) -> Result<(), String> {
    match c {
        '\r' => Ok(()),
        '\n' => press(KEY_RETURN, 0),
        _ => {
            let mut buffer = [0u16; 2];
            let units = c.encode_utf16(&mut buffer);
            press_text(units)
        }
    }
}

fn erase(count: usize, guard: &Guard) -> Result<(), String> {
    for _ in 0..count {
        guard.check()?;
        guard.pace(2);
        press(KEY_DELETE, 0)?;
    }
    Ok(())
}

fn type_text(text: &str, guard: &Guard) -> Result<(), String> {
    for c in text.chars() {
        guard.check()?;
        guard.pace(2);
        type_char(c)?;
    }
    Ok(())
}

fn paste_shortcut(guard: &Guard) -> Result<(), String> {
    guard.check()?;
    press(KEY_V, FLAG_COMMAND)
}

fn wait_for_modifiers_release(limit: Duration) {
    let held = FLAG_SHIFT | FLAG_CONTROL | FLAG_ALTERNATE | FLAG_COMMAND;
    let deadline = Instant::now() + limit;
    while unsafe { CGEventSourceFlagsState(HID_SYSTEM_STATE) } & held != 0 {
        if Instant::now() >= deadline {
            return;
        }
        thread::sleep(Duration::from_millis(8));
    }
}

// --- Cancelling and pacing -------------------------------------------------

struct Guard {
    // The cancel key may still be down from before the insertion began; it only
    // cancels once it has been released and pressed again.
    keycode: u16,
    armed: Cell<bool>,
    interval: Duration,
    next: Cell<Option<Instant>>,
}

impl Guard {
    fn new(speed: TypingSpeed, cancel: CancelKey) -> Self {
        let keycode = keycode_of(cancel);
        CANCEL.store(false, Ordering::Release);
        Self {
            keycode,
            armed: Cell::new(!key_down(keycode)),
            interval: Duration::from_nanos(1_000_000_000 / speed.events_per_second().max(1) as u64),
            next: Cell::new(None),
        }
    }

    fn check(&self) -> Result<(), String> {
        if CANCEL.load(Ordering::Acquire) {
            return Err(CANCELLED.into());
        }
        if !key_down(self.keycode) {
            self.armed.set(true);
        } else if self.armed.get() {
            CANCEL.store(true, Ordering::Release);
            log::info!("insertion stopped with the cancel key");
            return Err(CANCELLED.into());
        }
        Ok(())
    }

    fn pace(&self, events: u32) {
        if self.interval.is_zero() {
            return;
        }
        let now = Instant::now();
        if let Some(due) = self.next.get() {
            if due > now {
                thread::sleep(due - now);
            }
        }
        self.next
            .set(Some(Instant::now() + self.interval.saturating_mul(events)));
    }
}

fn key_down(keycode: u16) -> bool {
    unsafe { CGEventSourceKeyState(HID_SYSTEM_STATE, keycode) }
}

// Virtual key codes. A Mac keyboard has no Pause or Scroll Lock, so those two
// stand for F15 and F14, which is where those keys sit on an extended one.
fn keycode_of(key: CancelKey) -> u16 {
    const FUNCTION: [u16; 12] = [122, 120, 99, 118, 96, 97, 98, 100, 101, 109, 103, 111];
    match key {
        CancelKey::Escape => 53,
        CancelKey::Pause => 113,
        CancelKey::ScrollLock => 107,
        CancelKey::Function(number) => FUNCTION[(number as usize - 1).min(11)],
    }
}

// --- Delivering a payload --------------------------------------------------

fn wants_paste(content: &str, config: Config) -> bool {
    match config.injection {
        InjectionMode::Paste => true,
        InjectionMode::Type => false,
        InjectionMode::Auto => {
            content.chars().count() > AUTO_TYPE_LIMIT
                || content.contains('\n')
                || content.contains('\t')
        }
    }
}

fn text_settle(content: &str) -> u64 {
    if content.chars().count() > 20_000 {
        260
    } else {
        150
    }
}

fn place_text(content: &str, config: Config, guard: &Guard) -> Result<(), String> {
    if content.is_empty() {
        return Ok(());
    }
    if !wants_paste(content, config) {
        return type_text(content, guard);
    }
    guard.check()?;
    clipboard::set_text(content)?;
    thread::sleep(Duration::from_millis(30));
    paste_shortcut(guard)?;
    thread::sleep(Duration::from_millis(text_settle(content)));
    Ok(())
}

fn hand_over(paths: &[&Path], guard: &Guard) -> Result<(), String> {
    clipboard::set_files(paths)?;
    thread::sleep(Duration::from_millis(60));
    paste_shortcut(guard)
}

fn place_files(
    files: &[PathBuf],
    strict_order: bool,
    config: Config,
    guard: &Guard,
) -> Result<(), String> {
    let settle = Duration::from_millis(config.attachment_settle_ms);

    if strict_order {
        for path in files {
            guard.check()?;
            hand_over(&[path.as_path()], guard)?;
            thread::sleep(settle);
        }
        return Ok(());
    }

    guard.check()?;
    let refs: Vec<&Path> = files.iter().map(|path| path.as_path()).collect();
    hand_over(&refs, guard)?;
    thread::sleep(settle);
    Ok(())
}

fn deliver_payload(
    content: &str,
    files: &[PathBuf],
    attachments_first: bool,
    strict_order: bool,
    config: Config,
    guard: &Guard,
) -> Result<(), String> {
    let needs_clipboard = !files.is_empty() || wants_paste(content, config);
    if !needs_clipboard {
        return type_text(content, guard);
    }

    let snapshot = if config.restore_clipboard {
        match clipboard::capture() {
            Ok(snapshot) => Some(snapshot),
            Err(error) => {
                log::warn!("could not read the clipboard before pasting: {error}");
                None
            }
        }
    } else {
        None
    };

    if files.is_empty() {
        if let Some(snapshot) = &snapshot {
            if !snapshot.complete() && content.chars().count() <= TYPEABLE_LIMIT {
                log::info!("clipboard holds content Ampello cannot restore; typing instead");
                return type_text(content, guard);
            }
        }
    }

    let outcome = (|| {
        if files.is_empty() {
            return place_text(content, config, guard);
        }
        if attachments_first {
            place_files(files, strict_order, config, guard)?;
            place_text(content, config, guard)
        } else {
            place_text(content, config, guard)?;
            place_files(files, strict_order, config, guard)
        }
    })();

    if let Some(snapshot) = &snapshot {
        if let Err(error) = clipboard::restore(snapshot) {
            log::warn!("could not restore the clipboard: {error}");
        }
    }
    outcome
}
