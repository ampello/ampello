// SPDX-License-Identifier: GPL-3.0-or-later
mod clipboard;
mod inject;
mod keys;

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::Duration;

use parking_lot::Mutex;
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayout, MapVirtualKeyW, ToUnicodeEx, MAPVK_VK_TO_VSC, VK_BACK, VK_ESCAPE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, PostThreadMessageW,
    SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_QUIT, WM_RBUTTONDOWN,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use ampello_core::db;
use ampello_core::engine::{BoundaryMode, Engine, Expansion, Key, Trigger};
use ampello_core::Database;

use super::{EngineStatus, ExpandedCallback};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectionMode {
    Auto,
    Paste,
    Type,
}

impl InjectionMode {
    fn parse(value: &str) -> Self {
        match value {
            "paste" => InjectionMode::Paste,
            "type" => InjectionMode::Type,
            _ => InjectionMode::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardMode {
    Paste,
    Type,
}

impl ClipboardMode {
    fn parse(value: &str) -> Self {
        match value {
            "paste" => ClipboardMode::Paste,
            _ => ClipboardMode::Type,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypingSpeed {
    Fast,
    Balanced,
    Careful,
}

impl TypingSpeed {
    fn parse(value: &str) -> Self {
        match value {
            "fast" => TypingSpeed::Fast,
            "careful" => TypingSpeed::Careful,
            _ => TypingSpeed::Balanced,
        }
    }

    pub fn events_per_second(self) -> u32 {
        match self {
            TypingSpeed::Fast => 500,
            TypingSpeed::Balanced => 300,
            TypingSpeed::Careful => 120,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub preserve_terminator: bool,
    pub restore_clipboard: bool,
    pub injection: InjectionMode,
    pub typing: TypingSpeed,
    pub clipboard: ClipboardMode,

    pub attachment_settle_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            preserve_terminator: true,
            restore_clipboard: true,
            injection: InjectionMode::Auto,
            typing: TypingSpeed::Balanced,
            clipboard: ClipboardMode::Type,
            attachment_settle_ms: 500,
        }
    }
}

struct Shared {
    engine: Mutex<Engine>,
    keyboard_state: Mutex<[u8; 256]>,
    config: Mutex<Config>,
    jobs: Sender<Job>,

    injecting: AtomicBool,

    last_window: AtomicUsize,

    keys_seen: AtomicU64,
    expansions: AtomicU64,

    last_error: Mutex<Option<String>>,
}

static SHARED: OnceLock<Arc<Shared>> = OnceLock::new();

enum Job {
    Expand { expansion: Expansion, window: usize },

    InsertClipboard,
    Stop,
}

pub struct InputService {
    shared: Arc<Shared>,
    db: Arc<Database>,
    hook_thread: AtomicU32,
    error: Mutex<Option<String>>,
}

impl InputService {
    pub fn start(db: Arc<Database>, on_expanded: ExpandedCallback) -> Self {
        let (jobs, receiver) = mpsc::channel::<Job>();

        let shared = Arc::new(Shared {
            engine: Mutex::new(Engine::new()),
            keyboard_state: Mutex::new([0u8; 256]),
            config: Mutex::new(Config::default()),
            jobs,
            injecting: AtomicBool::new(false),
            last_window: AtomicUsize::new(0),
            keys_seen: AtomicU64::new(0),
            expansions: AtomicU64::new(0),
            last_error: Mutex::new(None),
        });

        let service = Self {
            shared: Arc::clone(&shared),
            db: Arc::clone(&db),
            hook_thread: AtomicU32::new(0),
            error: Mutex::new(None),
        };

        service.refresh();

        let _ = SHARED.set(Arc::clone(&shared));

        {
            let shared = Arc::clone(&shared);
            let db = Arc::clone(&db);
            thread::Builder::new()
                .name("ampello-injector".into())
                .spawn(move || worker(shared, receiver, db, on_expanded))
                .expect("could not start Ampello's injector thread");
        }

        service.install_hook();
        service
    }

    fn install_hook(&self) {
        let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, String>>();
        thread::Builder::new()
            .name("ampello-keyboard-hook".into())
            .spawn(move || hook_thread(ready_tx))
            .expect("could not start Ampello's keyboard hook thread");

        match ready_rx.recv() {
            Ok(Ok(thread_id)) => {
                self.hook_thread.store(thread_id, Ordering::SeqCst);
                *self.error.lock() = None;
                log::info!("keyboard hook installed");
            }
            Ok(Err(error)) => {
                log::error!("keyboard hook failed: {error}");
                self.hook_thread.store(0, Ordering::SeqCst);
                *self.error.lock() = Some(error);
            }
            Err(_) => {
                self.hook_thread.store(0, Ordering::SeqCst);
                *self.error.lock() =
                    Some("Ampello's keyboard hook thread stopped unexpectedly.".into());
            }
        }
    }

    pub fn restart(&self) {
        log::info!("restarting the expansion engine");
        self.stop_hook();

        thread::sleep(Duration::from_millis(150));

        self.shared.injecting.store(false, Ordering::Release);
        self.shared.engine.lock().reset();
        *self.shared.keyboard_state.lock() = [0u8; 256];
        *self.shared.last_error.lock() = None;

        self.install_hook();
        self.refresh();
    }

    fn stop_hook(&self) {
        let thread_id = self.hook_thread.swap(0, Ordering::SeqCst);
        if thread_id != 0 {
            unsafe { PostThreadMessageW(thread_id, WM_QUIT, 0, 0) };
        }
    }

    pub fn refresh(&self) {
        let loaded = self.db.with(|conn| {
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
            running: self.hook_thread.load(Ordering::SeqCst) != 0,
            enabled: engine.is_enabled(),
            trigger_count: engine.trigger_count(),
            error: self.error.lock().clone(),
            platform: "windows".into(),
            keystrokes_seen: self.shared.keys_seen.load(Ordering::Relaxed),
            expansions: self.shared.expansions.load(Ordering::Relaxed),
            last_expansion_error: self.shared.last_error.lock().clone(),
        }
    }
}

fn hook_thread(ready: Sender<Result<u32, String>>) {
    unsafe {
        let module = GetModuleHandleW(ptr::null());

        let keyboard = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0);
        if keyboard.is_null() {
            let _ = ready.send(Err(
                "Windows would not install Ampello's keyboard hook. Another tool may already \
                 have it, or Ampello needs to run at the same privilege level as the \
                 applications you type in."
                    .into(),
            ));
            return;
        }

        let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0);
        if mouse.is_null() {
            log::warn!("mouse hook unavailable; clicks will not reset the input buffer");
        }

        let _ = ready.send(Ok(GetCurrentThreadId()));

        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, ptr::null_mut(), 0, 0) > 0 {}

        UnhookWindowsHookEx(keyboard);
        if !mouse.is_null() {
            UnhookWindowsHookEx(mouse);
        }
        log::info!("keyboard hook removed");
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let swallow = catch_unwind(AssertUnwindSafe(|| on_key(wparam, lparam))).unwrap_or(false);
        if swallow {
            return 1;
        }
    }
    CallNextHookEx(ptr::null_mut() as HHOOK, code, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0
        && matches!(
            wparam as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN
        )
    {
        if let Some(shared) = SHARED.get() {
            let _ = catch_unwind(AssertUnwindSafe(|| shared.engine.lock().reset()));
        }
    }
    CallNextHookEx(ptr::null_mut() as HHOOK, code, wparam, lparam)
}

unsafe fn on_key(wparam: WPARAM, lparam: LPARAM) -> bool {
    let Some(shared) = SHARED.get() else {
        return false;
    };
    if lparam == 0 {
        return false;
    }
    let info = &*(lparam as *const KBDLLHOOKSTRUCT);

    if info.dwExtraInfo == inject::AMPELLO_MARKER {
        inject::note_injected();

        return inject::discarding();
    }
    let message = wparam as u32;
    let down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
    let up = message == WM_KEYUP || message == WM_SYSKEYUP;
    if !down && !up {
        return false;
    }

    let vk = info.vkCode as u16;

    if shared.injecting.load(Ordering::Acquire) {
        if down && vk == VK_ESCAPE {
            inject::request_cancel();
            inject::clear_escape_pending();
            log::info!("expansion cancelled with Escape");
            return true;
        }
        return false;
    }

    if vk == VK_ESCAPE && inject::escape_pending() {
        if up {
            inject::clear_escape_pending();
        }
        return true;
    }
    {
        let mut state = shared.keyboard_state.lock();
        keys::update_state(&mut state, vk, down);
    }
    if !down {
        return false;
    }
    shared.keys_seen.fetch_add(1, Ordering::Relaxed);

    let window = GetForegroundWindow() as usize;
    if shared.last_window.swap(window, Ordering::Relaxed) != window {
        shared.engine.lock().reset();
    }

    if vk == VK_BACK {
        shared.engine.lock().on_key(Key::Backspace);
        return false;
    }
    if keys::is_reset_key(vk) {
        shared.engine.lock().reset();
        return false;
    }
    if keys::is_modifier(vk) {
        return false;
    }

    let state = *shared.keyboard_state.lock();
    if keys::is_shortcut(&state) {
        shared.engine.lock().reset();
        return false;
    }

    let Some(text) = translate(vk, info.scanCode, &state) else {
        return false;
    };

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
    if shared.jobs.send(Job::Expand { expansion, window }).is_err() {
        shared.injecting.store(false, Ordering::Release);
        return false;
    }
    true
}

unsafe fn translate(vk: u16, scan_code: u32, state: &[u8; 256]) -> Option<String> {
    let layout = {
        let window = GetForegroundWindow();
        let thread = GetWindowThreadProcessId(window, ptr::null_mut());
        GetKeyboardLayout(thread)
    };

    let scan = if scan_code != 0 {
        scan_code
    } else {
        MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC)
    };

    let mut buffer = [0u16; 8];
    let produced = ToUnicodeEx(
        vk as u32,
        scan,
        state.as_ptr(),
        buffer.as_mut_ptr(),
        buffer.len() as i32,
        0,
        layout,
    );

    if produced <= 0 {
        return None;
    }
    String::from_utf16(&buffer[..produced as usize]).ok()
}

fn worker(
    shared: Arc<Shared>,
    jobs: mpsc::Receiver<Job>,
    db: Arc<Database>,
    on_expanded: ExpandedCallback,
) {
    while let Ok(job) = jobs.recv() {
        let (expansion, window) = match job {
            Job::Stop => break,
            Job::Expand { expansion, window } => (expansion, window),
            Job::InsertClipboard => {
                let config = *shared.config.lock();
                let outcome = catch_unwind(AssertUnwindSafe(|| insert_clipboard(config)))
                    .unwrap_or_else(|_| Err("Ampello's injector panicked.".into()));

                shared.injecting.store(false, Ordering::Release);

                match outcome {
                    Ok(()) => {}
                    Err(error) if error == inject::CANCELLED => {
                        inject::finish_cancel();
                        log::info!("clipboard insertion cancelled");
                    }

                    Err(error) => log::warn!("clipboard insertion failed: {error}"),
                }
                continue;
            }
        };

        let config = *shared.config.lock();
        let outcome = catch_unwind(AssertUnwindSafe(|| expand(&db, &expansion, config, window)))
            .unwrap_or_else(|_| Err("Ampello's injector panicked.".into()));

        shared.injecting.store(false, Ordering::Release);

        match outcome {
            Ok(()) => {
                shared.expansions.fetch_add(1, Ordering::Relaxed);
                *shared.last_error.lock() = None;
                on_expanded(&expansion.snippet_id);
            }
            Err(error) if error == inject::CANCELLED => {
                inject::finish_cancel();
                log::info!("expansion of snippet {} cancelled", expansion.snippet_id);
                *shared.last_error.lock() = Some("Stopped with Escape part-way through.".into());
            }
            Err(error) => {
                log::warn!(
                    "expansion of snippet {} failed: {error}",
                    expansion.snippet_id
                );
                *shared.last_error.lock() = Some(error);
            }
        }
    }
    log::info!("injector thread stopped");
}

fn insert_clipboard(config: Config) -> Result<(), String> {
    inject::wait_for_modifiers_release(Duration::from_millis(1_200));
    inject::release_modifiers()?;

    if config.clipboard == ClipboardMode::Paste {
        let guard = inject::Guard::capture(0, config.typing);
        return inject::paste_now(&guard);
    }

    let text = match clipboard::get_text()? {
        Some(text) => text,

        None => {
            log::info!("clipboard shortcut: the clipboard is not text, pasting instead");
            let guard = inject::Guard::capture(0, config.typing);
            return inject::paste_now(&guard);
        }
    };

    if text.is_empty() {
        return Ok(());
    }

    let guard = inject::Guard::capture(text.chars().count(), config.typing);
    inject::type_text(&text, &guard)
}

fn expand(
    db: &Database,
    expansion: &Expansion,
    config: Config,
    window: usize,
) -> Result<(), String> {
    let snippet = match db.with(|conn| db::snippets::get(conn, &expansion.snippet_id)) {
        Ok(snippet) => snippet,
        Err(error) => {
            let _ = inject::type_char(expansion.terminator);
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

    if window != 0 && inject::foreground() != window {
        let _ = inject::type_char(expansion.terminator);
        return Err("the active window changed before the expansion could start".into());
    }

    let guard = inject::Guard::capture(content.chars().count(), config.typing);

    inject::release_modifiers()?;

    let erase_count = expansion.trigger.encode_utf16().count();
    inject::erase(erase_count, &guard)?;

    inject::deliver_payload(
        inject::Payload {
            content: &content,
            files: &files,
            attachments_first: snippet.attachments_first,
            strict_order: snippet.strict_order,
        },
        config,
        &guard,
    )?;

    if config.preserve_terminator {
        inject::type_char(expansion.terminator)?;
    }

    if let Err(error) = db.with(|conn| db::snippets::record_usage(conn, &expansion.snippet_id)) {
        log::warn!("could not record snippet usage: {error}");
    }
    Ok(())
}
