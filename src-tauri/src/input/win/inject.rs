// SPDX-License-Identifier: GPL-3.0-or-later
use std::cell::Cell;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
    KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MAPVK_VK_TO_VSC, VK_BACK, VK_CONTROL, VK_ESCAPE,
    VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RETURN, VK_RMENU,
    VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_TAB, VK_V,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetGUIThreadInfo, GUITHREADINFO,
};

use super::Config;
use super::InjectionMode;
use super::TypingSpeed;
use super::clipboard;

// Stamped on every event we send so our own hook can ignore it.
// Without this, injected backspaces and pastes feed back into the matcher.
pub const AMPELLO_MARKER: usize = 0x414D_504C;

const TYPEABLE_LIMIT: usize = 5_000;

const AUTO_TYPE_LIMIT: usize = 24;

const BATCH: usize = 64;

// How far ahead of Windows we are willing to get, in events.
//
// `SendInput` returns once Windows has accepted an event, not once anything
// has happened with it, so sending flat out just builds a queue. Every
// keyboard event on the machine passes one thread in order, so running
// thousands ahead leaves the user's Escape stuck behind all of them.
const MAX_IN_FLIGHT: u64 = 128;

const PACE_MS: u64 = 10;

const DRAIN_TIMEOUT: Duration = Duration::from_millis(400);

const WATCH_THRESHOLD: usize = 200;

fn key_event(vk: u16, scan: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: AMPELLO_MARKER,
            },
        },
    }
}

fn scan_of(vk: u16) -> u16 {
    unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) as u16 }
}

fn press(vk: u16) -> [INPUT; 2] {
    let scan = scan_of(vk);
    [key_event(vk, scan, 0), key_event(vk, scan, KEYEVENTF_KEYUP)]
}

fn current_target() -> (usize, usize) {
    unsafe {
        let window = GetForegroundWindow() as usize;
        let mut info: GUITHREADINFO = std::mem::zeroed();
        info.cbSize = size_of::<GUITHREADINFO>() as u32;
        let focus = if GetGUIThreadInfo(0, &mut info) != 0 {
            info.hwndFocus as usize
        } else {
            0
        };
        (window, focus)
    }
}

pub fn foreground() -> usize {
    current_target().0
}

static INJECTED_SENT: AtomicU64 = AtomicU64::new(0);
static INJECTED_SEEN: AtomicU64 = AtomicU64::new(0);

static DISCARD: AtomicBool = AtomicBool::new(false);

static METER_BLIND: AtomicBool = AtomicBool::new(false);

pub fn note_injected() {
    INJECTED_SEEN.fetch_add(1, Ordering::Release);
}

fn in_flight() -> u64 {
    INJECTED_SENT
        .load(Ordering::Acquire)
        .saturating_sub(INJECTED_SEEN.load(Ordering::Acquire))
}

static CANCEL: AtomicBool = AtomicBool::new(false);

static ESCAPE_PENDING: AtomicBool = AtomicBool::new(false);

static WATCHING: AtomicBool = AtomicBool::new(false);

pub fn request_cancel() {
    CANCEL.store(true, Ordering::Release);
    DISCARD.store(true, Ordering::Release);
}

pub fn discarding() -> bool {
    DISCARD.load(Ordering::Acquire)
}

pub fn finish_cancel() {
    let start = Instant::now();
    while in_flight() > 0 && start.elapsed() < DRAIN_TIMEOUT {
        thread::sleep(Duration::from_millis(1));
    }
    DISCARD.store(false, Ordering::Release);

    let events: Vec<INPUT> = [VK_RETURN, VK_TAB, VK_BACK]
        .iter()
        .map(|vk| key_event(*vk, scan_of(*vk), KEYEVENTF_KEYUP))
        .collect();
    let _ = send_batch(&events);
    let _ = release_modifiers();
}

pub fn escape_pending() -> bool {
    ESCAPE_PENDING.load(Ordering::Acquire)
}

pub fn clear_escape_pending() {
    ESCAPE_PENDING.store(false, Ordering::Release);
}

fn escape_is_down() -> bool {
    unsafe { (GetAsyncKeyState(VK_ESCAPE as i32) as u32) & 0x8000 != 0 }
}

// Escape is polled rather than read from the hook: with an insertion in
// flight the hook sees thousands of injected events first, so it would learn
// about the keystroke only after the insertion it was meant to stop.
fn watch_for_escape() {
    let mut armed = !escape_is_down();

    while WATCHING.load(Ordering::Acquire) {
        if !escape_is_down() {
            armed = true;
        } else if armed {
            CANCEL.store(true, Ordering::Release);
            DISCARD.store(true, Ordering::Release);
            ESCAPE_PENDING.store(true, Ordering::Release);
            log::info!("insertion stopped with Escape");
            return;
        }
        thread::sleep(Duration::from_millis(8));
    }
}

pub const CANCELLED: &str = "cancelled";

pub struct Guard {
    window: usize,
    focus: usize,
    watcher: Option<JoinHandle<()>>,

    started: Instant,
    sent: Cell<u64>,

    interval: Duration,
    next: Cell<Option<Instant>>,
}

impl Guard {
    pub fn capture(length: usize, speed: TypingSpeed) -> Self {
        let (window, focus) = current_target();
        CANCEL.store(false, Ordering::Release);
        DISCARD.store(false, Ordering::Release);
        ESCAPE_PENDING.store(false, Ordering::Release);
        METER_BLIND.store(false, Ordering::Release);
        INJECTED_SENT.store(0, Ordering::Release);
        INJECTED_SEEN.store(0, Ordering::Release);

        let watcher = if length >= WATCH_THRESHOLD {
            WATCHING.store(true, Ordering::Release);
            thread::Builder::new()
                .name("ampello-cancel".into())
                .spawn(watch_for_escape)
                .map_err(|error| log::warn!("could not watch for Escape: {error}"))
                .ok()
        } else {
            None
        };

        Self {
            window,
            focus,
            watcher,
            started: Instant::now(),
            sent: Cell::new(0),
            interval: Duration::from_nanos(
                1_000_000_000 / speed.events_per_second().max(1) as u64,
            ),
            next: Cell::new(None),
        }
    }

    fn govern(&self, events: u64) {
        if self.interval.is_zero() {
            return;
        }
        let now = Instant::now();
        if let Some(due) = self.next.get() {
            if due > now {
                thread::sleep(due - now);
            }
        }

        let base = Instant::now();
        self.next
            .set(Some(base + self.interval.saturating_mul(events as u32)));
    }

    fn note_sent(&self, events: u64) {
        self.sent.set(self.sent.get() + events);
    }

    pub fn check(&self) -> Result<(), String> {
        if CANCEL.load(Ordering::Acquire) {
            return Err(CANCELLED.into());
        }
        let (window, focus) = current_target();
        if self.window != 0 && window != self.window {
            return Err(
                "the active window changed part-way through, so Ampello stopped rather than \
                 typing the rest of the snippet somewhere else"
                    .into(),
            );
        }

        if self.focus != 0 && focus != 0 && focus != self.focus {
            return Err(
                "the caret moved to a different field part-way through, so Ampello stopped"
                    .into(),
            );
        }
        Ok(())
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        WATCHING.store(false, Ordering::Release);
        if let Some(watcher) = self.watcher.take() {
            let _ = watcher.join();
        }

        let sent = self.sent.get();
        if sent > BATCH as u64 {
            log::info!(
                "insertion: {} events in {} ms{}",
                sent,
                self.started.elapsed().as_millis(),
                if METER_BLIND.load(Ordering::Acquire) {
                    ", hook meter unavailable"
                } else {
                    ""
                }
            );
        }
    }
}

fn send_batch(chunk: &[INPUT]) -> Result<(), String> {
    let sent =
        unsafe { SendInput(chunk.len() as u32, chunk.as_ptr(), size_of::<INPUT>() as i32) };
    INJECTED_SENT.fetch_add(sent as u64, Ordering::Release);
    if sent as usize != chunk.len() {
        return Err("Windows blocked Ampello from sending input to this window.".into());
    }
    Ok(())
}

fn wait_for_drain(guard: &Guard) -> Result<(), String> {
    if METER_BLIND.load(Ordering::Acquire) {
        thread::sleep(Duration::from_millis(PACE_MS));
        return Ok(());
    }

    let start = Instant::now();
    let mut spins = 0u32;

    while in_flight() > MAX_IN_FLIGHT {
        guard.check()?;

        if spins < 64 {
            spins += 1;
            thread::yield_now();
            continue;
        }

        if start.elapsed() > DRAIN_TIMEOUT {
            log::warn!("Ampello's own events are not returning through the hook; pacing blind");
            METER_BLIND.store(true, Ordering::Release);
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

fn send(inputs: &[INPUT], guard: &Guard) -> Result<(), String> {
    let mut sent = 0usize;

    while sent < inputs.len() {
        guard.check()?;
        let end = (sent + BATCH).min(inputs.len());
        guard.govern((end - sent) as u64);
        wait_for_drain(guard)?;
        send_batch(&inputs[sent..end])?;
        guard.note_sent((end - sent) as u64);
        sent = end;
    }
    Ok(())
}

pub fn release_modifiers() -> Result<(), String> {
    let events: Vec<INPUT> = [
        VK_SHIFT, VK_LSHIFT, VK_RSHIFT, VK_CONTROL, VK_LCONTROL, VK_RCONTROL, VK_MENU, VK_LMENU,
        VK_RMENU,
    ]
    .iter()
    .map(|vk| key_event(*vk, scan_of(*vk), KEYEVENTF_KEYUP))
    .collect();
    send_batch(&events)
}

pub fn erase(count: usize, guard: &Guard) -> Result<(), String> {
    if count == 0 {
        return Ok(());
    }
    let mut events = Vec::with_capacity(count * 2);
    for _ in 0..count {
        events.extend_from_slice(&press(VK_BACK));
    }
    send(&events, guard)
}

pub fn type_char(c: char) -> Result<(), String> {
    match c {
        '\n' | '\r' => send_batch(&press(VK_RETURN)),
        '\t' => send_batch(&press(VK_TAB)),
        _ => {
            let mut events = Vec::with_capacity(4);
            push_char(&mut events, c);
            send_batch(&events)
        }
    }
}

// A newline must be a real Return key: sent as a Unicode carriage return it is
// ignored by anything built on a web view. A tab must be the opposite, a
// Unicode character, because the Tab key moves focus instead of indenting.
fn push_char(events: &mut Vec<INPUT>, c: char) {
    match c {
        '\r' => {}
        '\n' => events.extend_from_slice(&press(VK_RETURN)),
        '\t' => push_unit(events, 0x0009),
        _ => {
            let mut buffer = [0u16; 2];
            for unit in c.encode_utf16(&mut buffer) {
                push_unit(events, *unit);
            }
        }
    }
}

fn push_unit(events: &mut Vec<INPUT>, unit: u16) {
    events.push(key_event(0, unit, KEYEVENTF_UNICODE));
    events.push(key_event(0, unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
}

pub fn type_text(text: &str, guard: &Guard) -> Result<(), String> {
    let mut events: Vec<INPUT> = Vec::with_capacity(4);

    for c in text.chars() {
        events.clear();
        push_char(&mut events, c);
        if events.is_empty() {
            continue;
        }

        guard.check()?;
        guard.govern(events.len() as u64);
        wait_for_drain(guard)?;
        send_batch(&events)?;
        guard.note_sent(events.len() as u64);
    }
    Ok(())
}

pub fn wait_for_modifiers_release(limit: Duration) -> bool {
    const MODIFIERS: [u16; 5] = [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN];
    let deadline = Instant::now() + limit;

    loop {
        let held = MODIFIERS
            .iter()
            .any(|vk| unsafe { (GetAsyncKeyState(*vk as i32) as u32) & 0x8000 != 0 });
        if !held {
            return true;
        }
        if Instant::now() >= deadline {
            log::info!("clipboard shortcut: still held after the wait; inserting anyway");
            return false;
        }
        thread::sleep(Duration::from_millis(8));
    }
}

pub fn paste_now(guard: &Guard) -> Result<(), String> {
    paste_shortcut(guard)
}

fn paste_shortcut(guard: &Guard) -> Result<(), String> {
    guard.check()?;
    let ctrl = scan_of(VK_CONTROL);
    let v = scan_of(VK_V);
    send_batch(&[
        key_event(VK_CONTROL, ctrl, 0),
        key_event(VK_V, v, 0),
        key_event(VK_V, v, KEYEVENTF_KEYUP),
        key_event(VK_CONTROL, ctrl, KEYEVENTF_KEYUP),
    ])
}

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

pub fn deliver(content: &str, config: Config, guard: &Guard) -> Result<(), String> {
    if content.is_empty() {
        return Ok(());
    }

    let length = content.chars().count();
    let wants_paste = wants_paste(content, config);

    if !wants_paste {
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

    if let Some(snapshot) = &snapshot {
        if !snapshot.complete && length <= TYPEABLE_LIMIT {
            log::info!("clipboard holds content Ampello cannot restore; typing instead of pasting");
            return type_text(content, guard);
        }
    }

    guard.check()?;
    clipboard::set_text(content)?;

    thread::sleep(Duration::from_millis(30));
    if let Err(error) = paste_shortcut(guard) {
        if let Some(snapshot) = &snapshot {
            let _ = clipboard::restore(snapshot);
        }
        return Err(error);
    }

    thread::sleep(Duration::from_millis(text_settle(content)));

    match snapshot {
        Some(snapshot) if !snapshot.is_empty() => {
            if let Err(error) = clipboard::restore(&snapshot) {
                log::warn!("could not restore the clipboard: {error}");
            }
        }
        Some(_) => {
            let _ = clipboard::clear();
        }
        None => {}
    }

    Ok(())
}

pub struct Payload<'a> {
    pub content: &'a str,

    pub files: &'a [PathBuf],

    pub attachments_first: bool,

    pub strict_order: bool,
}

pub fn deliver_payload(payload: Payload, config: Config, guard: &Guard) -> Result<(), String> {
    if payload.files.is_empty() {
        return deliver(payload.content, config, guard);
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
    if let Some(snapshot) = &snapshot {
        if !snapshot.complete {
            log::warn!(
                "the clipboard holds content Ampello cannot restore; a snippet with files has no \
                 way to avoid overwriting it"
            );
        }
    }

    let outcome = (|| {
        if payload.attachments_first {
            place_files(&payload, config, guard)?;
            place_text(payload.content, config, guard)
        } else {
            place_text(payload.content, config, guard)?;
            place_files(&payload, config, guard)
        }
    })();

    match &snapshot {
        Some(snapshot) if !snapshot.is_empty() => {
            if let Err(error) = clipboard::restore(snapshot) {
                log::warn!("could not restore the clipboard: {error}");
            }
        }
        Some(_) => {
            let _ = clipboard::clear();
        }
        None => {}
    }

    outcome
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

fn place_files(payload: &Payload, config: Config, guard: &Guard) -> Result<(), String> {
    let settle = Duration::from_millis(config.attachment_settle_ms);

    if payload.strict_order {
        for path in payload.files {
            guard.check()?;
            hand_over(&[path.as_path()], guard)?;
            thread::sleep(settle);
        }
        return Ok(());
    }

    guard.check()?;
    let refs: Vec<&Path> = payload.files.iter().map(|path| path.as_path()).collect();
    hand_over(&refs, guard)?;

    thread::sleep(settle);
    Ok(())
}

fn hand_over(paths: &[&Path], guard: &Guard) -> Result<(), String> {
    clipboard::set_files(paths)?;

    thread::sleep(Duration::from_millis(30));
    paste_shortcut(guard)
}
