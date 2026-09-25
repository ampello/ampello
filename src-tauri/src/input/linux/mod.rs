// SPDX-License-Identifier: GPL-3.0-or-later
//! The expansion engine on Linux, for X11 sessions.
//!
//! Keystrokes are observed with the X RECORD extension and delivered with the
//! XTEST extension - the same mechanism tools such as xdotool and AutoKey use.
//! Neither needs elevated privileges.
//!
//! RECORD can watch but not intercept, so unlike Windows and macOS the
//! character that completes a trigger has already reached the application by
//! the time the expansion starts; it is erased along with the trigger and
//! typed again afterwards when "keep the boundary character" is on.
//!
//! Wayland deliberately forbids one application from watching another's
//! keystrokes, so there is nothing to hook: Ampello says so rather than
//! appearing to work.
mod clipboard;

use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use x11rb::connection::Connection;
use x11rb::protocol::record::{self, ConnectionExt as _};
use x11rb::protocol::xproto::ConnectionExt as _;
use x11rb::protocol::xtest::ConnectionExt as _;
use x11rb::rust_connection::RustConnection;

use crate::state::Library;
use ampello_core::{db, CancelKey};
use ampello_core::engine::{BoundaryMode, Engine, Expansion, Key, Trigger};

use super::config::{ClipboardMode, Config, InjectionMode, TypingSpeed};
use super::{EngineStatus, ExpandedCallback};

const KEY_PRESS: u8 = 2;
const KEY_RELEASE: u8 = 3;
const BUTTON_PRESS: u8 = 4;

const MASK_SHIFT: u16 = 0x01;
const MASK_LOCK: u16 = 0x02;
const MASK_CONTROL: u16 = 0x04;
const MASK_ALT: u16 = 0x08;
const MASK_SUPER: u16 = 0x40;
const MASK_LEVEL3: u16 = 0x80;

const SYM_BACKSPACE: u32 = 0xff08;
const SYM_TAB: u32 = 0xff09;
const SYM_RETURN: u32 = 0xff0d;

const SYM_SHIFT_L: u32 = 0xffe1;
const SYM_CONTROL_L: u32 = 0xffe3;
const SYM_CONTROL_R: u32 = 0xffe4;
const SYM_V: u32 = 0x76;

const TYPEABLE_LIMIT: usize = 5_000;
const AUTO_TYPE_LIMIT: usize = 24;
const CANCELLED: &str = "cancelled";

const WAYLAND_HELP: &str = "This is a Wayland session, and Wayland does not let one application \
    watch another's keystrokes, so Ampello cannot expand text here. Log in with an X11 session \
    (\"Ubuntu on Xorg\", \"GNOME on Xorg\", or Plasma X11) to use it.";

struct Shared {
    engine: Mutex<Engine>,
    config: Mutex<Config>,
    jobs: Sender<Job>,

    injecting: AtomicBool,

    keys_seen: AtomicU64,
    expansions: AtomicU64,
    last_error: Mutex<Option<String>>,
}

static SHARED: OnceLock<Arc<Shared>> = OnceLock::new();

enum Job {
    Expand { expansion: Expansion },
    InsertClipboard,
    Stop,
}

/// What is needed to interrupt the recording thread from outside.
struct Recording {
    stop: Arc<AtomicBool>,
    control: Option<(Arc<RustConnection>, record::Context)>,
}

pub struct InputService {
    shared: Arc<Shared>,
    library: Arc<Library>,
    recording: Arc<Mutex<Option<Recording>>>,
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
            keys_seen: AtomicU64::new(0),
            expansions: AtomicU64::new(0),
            last_error: Mutex::new(None),
        });

        let service = Self {
            shared: Arc::clone(&shared),
            library: Arc::clone(&library),
            recording: Arc::new(Mutex::new(None)),
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
        if std::env::var("XDG_SESSION_TYPE").is_ok_and(|kind| kind == "wayland") {
            log::warn!("this is a Wayland session; keystrokes cannot be watched");
            *self.error.lock() = Some(WAYLAND_HELP.into());
            return;
        }

        let stop = Arc::new(AtomicBool::new(false));
        *self.recording.lock() = Some(Recording {
            stop: Arc::clone(&stop),
            control: None,
        });

        let running = Arc::clone(&self.running);
        let error = Arc::clone(&self.error);
        let recording = Arc::clone(&self.recording);
        let spawned = thread::Builder::new()
            .name("ampello-recorder".into())
            .spawn(move || hook_thread(stop, recording, running, error));
        if let Err(error) = spawned {
            log::error!("could not start the recording thread: {error}");
            *self.error.lock() = Some("Ampello could not start its keyboard watcher.".into());
        }
    }

    fn stop_hook(&self) {
        let Some(recording) = self.recording.lock().take() else {
            return;
        };
        recording.stop.store(true, Ordering::SeqCst);
        if let Some((connection, context)) = recording.control {
            // Ends the blocking read on the recording thread.
            let _ = connection.record_disable_context(context);
            let _ = connection.flush();
        }
    }

    pub fn restart(&self) {
        log::info!("restarting the expansion engine");
        self.stop_hook();
        thread::sleep(Duration::from_millis(400));

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
            platform: "linux".into(),
            keystrokes_seen: self.shared.keys_seen.load(Ordering::Relaxed),
            expansions: self.shared.expansions.load(Ordering::Relaxed),
            last_expansion_error: self.shared.last_error.lock().clone(),
        }
    }
}

// --- Keyboard layout -------------------------------------------------------

#[derive(Clone)]
struct Keymap {
    min: u8,
    per: usize,
    syms: Vec<u32>,
}

impl Keymap {
    fn load(connection: &RustConnection) -> Result<Self, String> {
        let setup = connection.setup();
        let (min, max) = (setup.min_keycode, setup.max_keycode);
        let reply = connection
            .get_keyboard_mapping(min, max - min + 1)
            .map_err(|error| error.to_string())?
            .reply()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            min,
            per: (reply.keysyms_per_keycode as usize).max(1),
            syms: reply.keysyms,
        })
    }

    fn count(&self) -> usize {
        self.syms.len() / self.per
    }

    fn sym(&self, keycode: u8, column: usize) -> u32 {
        let Some(row) = (keycode as usize).checked_sub(self.min as usize) else {
            return 0;
        };
        if row >= self.count() || column >= self.per {
            return 0;
        }
        self.syms[row * self.per + column]
    }

    /// The key that produces `sym`, and whether Shift must be held.
    fn find(&self, sym: u32) -> Option<(u8, bool)> {
        for row in 0..self.count() {
            for column in 0..2.min(self.per) {
                if self.syms[row * self.per + column] == sym {
                    return Some((self.min + row as u8, column == 1));
                }
            }
        }
        None
    }

    /// A keycode nothing is bound to, which can be borrowed to type any
    /// character the layout has no key for.
    fn spare(&self) -> Option<u8> {
        (0..self.count()).rev().find_map(|row| {
            let cells = &self.syms[row * self.per..(row + 1) * self.per];
            cells
                .iter()
                .all(|sym| *sym == 0)
                .then_some(self.min + row as u8)
        })
    }

    /// The symbol a key press produces given the modifier state.
    fn resolve(&self, keycode: u8, state: u16) -> u32 {
        let shift = usize::from(state & MASK_SHIFT != 0);
        let group = usize::from((state >> 13) & 0x3);
        let column = if state & MASK_LEVEL3 != 0 && self.per >= 6 {
            4 + shift
        } else if group > 0 && self.per >= group * 2 + 2 {
            group * 2 + shift
        } else {
            shift
        };
        let sym = self.sym(keycode, column);
        if sym == 0 {
            self.sym(keycode, 0)
        } else {
            sym
        }
    }
}

fn sym_of_char(c: char) -> u32 {
    let code = c as u32;
    if (0x20..=0x7e).contains(&code) || (0xa0..=0xff).contains(&code) {
        code
    } else {
        0x0100_0000 + code
    }
}

enum Decoded {
    Char(char),
    Backspace,
    Reset,
    Ignore,
}

fn decode(sym: u32, state: u16) -> Decoded {
    match sym {
        SYM_BACKSPACE => Decoded::Backspace,
        SYM_RETURN | 0xff8d => Decoded::Char('\n'),
        SYM_TAB => Decoded::Char('\t'),
        // Modifier keys on their own change nothing.
        0xffe1..=0xffee | 0xfe01..=0xfe13 | 0xff7f => Decoded::Ignore,
        // Escape, navigation, Delete, function keys and the like.
        0xff1b | 0xff50..=0xff58 | 0xff60..=0xff6f | 0xffff | 0xffbe..=0xffe0 => Decoded::Reset,
        0x20..=0x7e | 0xa0..=0xff => Decoded::Char(char::from_u32(sym).unwrap_or(' ')),
        0x0100_0000..=0x0110_ffff => char::from_u32(sym - 0x0100_0000)
            .map(Decoded::Char)
            .unwrap_or(Decoded::Ignore),
        _ => {
            let _ = state;
            Decoded::Ignore
        }
    }
}

fn apply_caps_lock(c: char, state: u16) -> char {
    if state & MASK_LOCK == 0 || !c.is_alphabetic() {
        return c;
    }
    // Caps Lock inverts the case Shift produced.
    let swapped: String = if c.is_lowercase() {
        c.to_uppercase().collect()
    } else {
        c.to_lowercase().collect()
    };
    let mut chars = swapped.chars();
    match (chars.next(), chars.next()) {
        (Some(single), None) => single,
        _ => c,
    }
}

// --- Recording -------------------------------------------------------------

fn hook_thread(
    stop: Arc<AtomicBool>,
    recording: Arc<Mutex<Option<Recording>>>,
    running: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
) {
    while !stop.load(Ordering::SeqCst) {
        match record_once(&stop, &recording, &running, &error) {
            Ok(()) => {}
            Err(problem) => {
                log::warn!("keyboard recording stopped: {problem}");
                *error.lock() = Some(problem);
            }
        }
        running.store(false, Ordering::SeqCst);
        if stop.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(Duration::from_secs(3));
    }
    log::info!("keyboard recording ended");
}

fn record_once(
    stop: &Arc<AtomicBool>,
    recording: &Arc<Mutex<Option<Recording>>>,
    running: &Arc<AtomicBool>,
    error: &Arc<Mutex<Option<String>>>,
) -> Result<(), String> {
    let (control, _) = RustConnection::connect(None)
        .map_err(|e| format!("Ampello could not connect to the X server: {e}"))?;
    let (data, _) = RustConnection::connect(None)
        .map_err(|e| format!("Ampello could not connect to the X server: {e}"))?;
    let control = Arc::new(control);

    control
        .record_query_version(1, 13)
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|_| "This X server does not offer the RECORD extension Ampello needs.".to_string())?;

    let nothing8 = record::Range8 { first: 0, last: 0 };
    let nothing_ext = record::ExtRange {
        major: nothing8,
        minor: record::Range16 { first: 0, last: 0 },
    };
    let range = record::Range {
        core_requests: nothing8,
        core_replies: nothing8,
        ext_requests: nothing_ext,
        ext_replies: nothing_ext,
        delivered_events: nothing8,
        device_events: record::Range8 {
            first: KEY_PRESS,
            last: BUTTON_PRESS,
        },
        errors: nothing8,
        client_started: false,
        client_died: false,
    };

    let context = control.generate_id().map_err(|e| e.to_string())?;
    control
        .record_create_context(context, 0, &[u32::from(record::CS::ALL_CLIENTS)], &[range])
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;

    {
        let mut slot = recording.lock();
        match slot.as_mut() {
            Some(current) if Arc::ptr_eq(&current.stop, stop) => {
                current.control = Some((Arc::clone(&control), context));
            }
            // Superseded by a restart while connecting.
            _ => return Ok(()),
        }
    }

    let mut keymap = Keymap::load(&control)?;
    *error.lock() = None;
    running.store(true, Ordering::SeqCst);
    log::info!("keyboard recording started");

    let stream = data
        .record_enable_context(context)
        .map_err(|e| e.to_string())?;
    for reply in stream {
        let reply = reply.map_err(|e| e.to_string())?;
        // Category 0 carries events from the server; the rest is bookkeeping.
        if reply.category != 0 || reply.client_swapped {
            continue;
        }
        for event in reply.data.chunks_exact(32) {
            let kind = event[0] & 0x7f;
            let keycode = event[1];
            let state = u16::from_le_bytes([event[28], event[29]]);
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                on_event(kind, keycode, state, &mut keymap, &control)
            }));
            if outcome.is_err() {
                log::error!("a keystroke handler panicked; carrying on");
            }
        }
        if stop.load(Ordering::SeqCst) {
            break;
        }
    }
    Ok(())
}

fn on_event(kind: u8, keycode: u8, state: u16, keymap: &mut Keymap, control: &RustConnection) {
    let Some(shared) = SHARED.get() else {
        return;
    };
    if shared.injecting.load(Ordering::Acquire) {
        return;
    }

    if kind == BUTTON_PRESS {
        shared.engine.lock().reset();
        return;
    }
    if kind != KEY_PRESS {
        return;
    }
    shared.keys_seen.fetch_add(1, Ordering::Relaxed);

    if state & (MASK_CONTROL | MASK_ALT | MASK_SUPER) != 0 {
        shared.engine.lock().reset();
        return;
    }

    let mut sym = keymap.resolve(keycode, state);
    if sym == 0 {
        // The layout changed since it was read.
        if let Ok(fresh) = Keymap::load(control) {
            *keymap = fresh;
            sym = keymap.resolve(keycode, state);
        }
    }

    let expansion = {
        let mut engine = shared.engine.lock();
        match decode(sym, state) {
            Decoded::Backspace => {
                engine.on_key(Key::Backspace);
                None
            }
            Decoded::Reset => {
                engine.reset();
                None
            }
            Decoded::Ignore => None,
            Decoded::Char(c) => engine.on_key(Key::Char(apply_caps_lock(c, state))),
        }
    };
    let Some(expansion) = expansion else {
        return;
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
    }
}

// --- The injector thread ---------------------------------------------------

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
                finish(&shared);
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
        finish(&shared);

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

// The recorder sees our own synthetic events a moment after they are sent;
// staying deaf for a beat keeps them out of the matcher.
fn finish(shared: &Shared) {
    thread::sleep(Duration::from_millis(80));
    shared.injecting.store(false, Ordering::Release);
}

fn insert_clipboard(config: Config) -> Result<(), String> {
    let mut injector = Injector::new(config.typing, config.cancel)?;
    injector.wait_for_modifiers_release(Duration::from_millis(1_200));

    if config.clipboard == ClipboardMode::Paste {
        return injector.paste();
    }
    let Some(text) = clipboard::get_text()? else {
        log::info!("clipboard shortcut: the clipboard is not text, pasting instead");
        return injector.paste();
    };
    if text.is_empty() {
        return Ok(());
    }
    injector.type_text(&text)
}

fn expand(library: &Library, expansion: &Expansion, config: Config) -> Result<(), String> {
    let db = library.db();
    let snippet = db
        .with(|conn| db::snippets::get(conn, &expansion.snippet_id))
        .map_err(|error| error.to_string())?;
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

    let mut injector = Injector::new(config.typing, config.cancel)?;
    injector.wait_for_modifiers_release(Duration::from_millis(300));

    // The boundary character was not swallowed, so it goes too.
    injector.erase(expansion.trigger.chars().count() + 1)?;

    deliver_payload(
        &mut injector,
        &content,
        &files,
        snippet.attachments_first,
        snippet.strict_order,
        config,
    )?;

    if config.preserve_terminator {
        injector.type_char(expansion.terminator)?;
    }
    if let Err(error) = db.with(|conn| db::snippets::record_usage(conn, &expansion.snippet_id)) {
        log::warn!("could not record snippet usage: {error}");
    }
    Ok(())
}

// --- Sending input ---------------------------------------------------------

struct Injector {
    connection: RustConnection,
    root: u32,
    keymap: Keymap,
    borrowed: Option<u8>,

    escape: Option<u8>,
    // The cancel key may still be down from before the insertion began; it only
    // cancels once it has been released and pressed again.
    armed: Cell<bool>,
    interval: Duration,
    next: Cell<Option<Instant>>,
}

impl Injector {
    fn new(speed: TypingSpeed, cancel: CancelKey) -> Result<Self, String> {
        let (connection, screen) = RustConnection::connect(None)
            .map_err(|e| format!("Ampello could not connect to the X server: {e}"))?;
        let root = connection.setup().roots[screen].root;
        connection
            .xtest_get_version(2, 1)
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|_| "This X server does not offer the XTEST extension Ampello needs.".to_string())?;
        let keymap = Keymap::load(&connection)?;
        let escape = keymap.find(sym_of_cancel(cancel)).map(|(keycode, _)| keycode);

        let injector = Self {
            connection,
            root,
            keymap,
            borrowed: None,
            escape,
            armed: Cell::new(true),
            interval: Duration::from_nanos(1_000_000_000 / speed.events_per_second().max(1) as u64),
            next: Cell::new(None),
        };
        injector.armed.set(!injector.escape_down());
        Ok(injector)
    }

    fn escape_down(&self) -> bool {
        let Some(keycode) = self.escape else {
            return false;
        };
        let Ok(cookie) = self.connection.query_keymap() else {
            return false;
        };
        let Ok(reply) = cookie.reply() else {
            return false;
        };
        reply.keys[(keycode / 8) as usize] & (1 << (keycode % 8)) != 0
    }

    fn check(&self) -> Result<(), String> {
        if !self.escape_down() {
            self.armed.set(true);
        } else if self.armed.get() {
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

    fn fake(&self, down: bool, keycode: u8) -> Result<(), String> {
        self.connection
            .xtest_fake_input(
                if down { KEY_PRESS } else { KEY_RELEASE },
                keycode,
                0,
                self.root,
                0,
                0,
                0,
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn flush(&self) -> Result<(), String> {
        self.connection.flush().map_err(|e| e.to_string())
    }

    fn tap(&self, keycode: u8) -> Result<(), String> {
        self.fake(true, keycode)?;
        self.fake(false, keycode)?;
        self.flush()
    }

    fn key_for(&self, sym: u32) -> Result<u8, String> {
        self.keymap
            .find(sym)
            .map(|(keycode, _)| keycode)
            .ok_or_else(|| "The keyboard layout has no key Ampello needs.".to_string())
    }

    fn wait_for_modifiers_release(&self, limit: Duration) {
        let held = MASK_SHIFT | MASK_CONTROL | MASK_ALT | MASK_SUPER;
        let deadline = Instant::now() + limit;
        loop {
            let mask = self
                .connection
                .query_pointer(self.root)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .map(|reply| u16::from(reply.mask))
                .unwrap_or(0);
            if mask & held == 0 || Instant::now() >= deadline {
                return;
            }
            thread::sleep(Duration::from_millis(8));
        }
    }

    fn erase(&self, count: usize) -> Result<(), String> {
        let backspace = self.key_for(SYM_BACKSPACE)?;
        for _ in 0..count {
            self.check()?;
            self.pace(2);
            self.tap(backspace)?;
        }
        Ok(())
    }

    fn type_char(&mut self, c: char) -> Result<(), String> {
        let sym = match c {
            '\r' => return Ok(()),
            '\n' => SYM_RETURN,
            '\t' => SYM_TAB,
            _ => sym_of_char(c),
        };

        if let Some((keycode, shifted)) = self.keymap.find(sym) {
            if !shifted {
                return self.tap(keycode);
            }
            let shift = self.key_for(SYM_SHIFT_L)?;
            self.fake(true, shift)?;
            self.fake(true, keycode)?;
            self.fake(false, keycode)?;
            self.fake(false, shift)?;
            return self.flush();
        }

        // No key produces it: borrow an unused keycode for a moment.
        let spare = self
            .borrowed
            .or_else(|| self.keymap.spare())
            .ok_or_else(|| "There is no free keycode to type a special character.".to_string())?;
        self.borrowed = Some(spare);
        self.connection
            .change_keyboard_mapping(1, spare, self.keymap.per as u8, &vec![sym; self.keymap.per])
            .map_err(|e| e.to_string())?;
        self.flush()?;
        thread::sleep(Duration::from_millis(10));
        self.tap(spare)?;
        thread::sleep(Duration::from_millis(10));
        Ok(())
    }

    fn type_text(&mut self, text: &str) -> Result<(), String> {
        for c in text.chars() {
            self.check()?;
            self.pace(2);
            self.type_char(c)?;
        }
        Ok(())
    }

    fn paste(&self) -> Result<(), String> {
        self.check()?;
        let control = self
            .key_for(SYM_CONTROL_L)
            .or_else(|_| self.key_for(SYM_CONTROL_R))?;
        let v = self.key_for(SYM_V)?;
        self.fake(true, control)?;
        self.fake(true, v)?;
        self.fake(false, v)?;
        self.fake(false, control)?;
        self.flush()
    }
}

impl Drop for Injector {
    fn drop(&mut self) {
        if let Some(spare) = self.borrowed {
            let _ = self.connection.change_keyboard_mapping(
                1,
                spare,
                self.keymap.per as u8,
                &vec![0u32; self.keymap.per],
            );
            let _ = self.connection.flush();
        }
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

fn place_text(
    injector: &mut Injector,
    content: &str,
    config: Config,
) -> Result<(), String> {
    if content.is_empty() {
        return Ok(());
    }
    if !wants_paste(content, config) {
        return injector.type_text(content);
    }
    injector.check()?;
    clipboard::set_text(content)?;
    thread::sleep(Duration::from_millis(60));
    injector.paste()?;
    thread::sleep(Duration::from_millis(text_settle(content)));
    Ok(())
}

fn hand_over(injector: &Injector, paths: &[&Path]) -> Result<(), String> {
    clipboard::set_files(paths)?;
    thread::sleep(Duration::from_millis(80));
    injector.paste()
}

fn place_files(
    injector: &Injector,
    files: &[PathBuf],
    strict_order: bool,
    config: Config,
) -> Result<(), String> {
    let settle = Duration::from_millis(config.attachment_settle_ms);

    if strict_order {
        for path in files {
            injector.check()?;
            hand_over(injector, &[path.as_path()])?;
            thread::sleep(settle);
        }
        return Ok(());
    }

    injector.check()?;
    let refs: Vec<&Path> = files.iter().map(|path| path.as_path()).collect();
    hand_over(injector, &refs)?;
    thread::sleep(settle);
    Ok(())
}

fn deliver_payload(
    injector: &mut Injector,
    content: &str,
    files: &[PathBuf],
    attachments_first: bool,
    strict_order: bool,
    config: Config,
) -> Result<(), String> {
    let needs_clipboard = !files.is_empty() || wants_paste(content, config);
    if !needs_clipboard {
        return injector.type_text(content);
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
                return injector.type_text(content);
            }
        }
    }

    let outcome = (|| {
        if files.is_empty() {
            return place_text(injector, content, config);
        }
        if attachments_first {
            place_files(injector, files, strict_order, config)?;
            place_text(injector, content, config)
        } else {
            place_text(injector, content, config)?;
            place_files(injector, files, strict_order, config)
        }
    })();

    if let Some(snapshot) = &snapshot {
        if let Err(error) = clipboard::restore(snapshot) {
            log::warn!("could not restore the clipboard: {error}");
        }
    }
    outcome
}

fn sym_of_cancel(key: CancelKey) -> u32 {
    match key {
        CancelKey::Escape => 0xff1b,
        CancelKey::Pause => 0xff13,
        CancelKey::ScrollLock => 0xff14,
        CancelKey::Function(number) => 0xffbe + (number as u32 - 1),
    }
}
