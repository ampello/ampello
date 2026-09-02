// SPDX-License-Identifier: GPL-3.0-or-later
use std::path::Path;
use std::ptr;
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::{HANDLE, HGLOBAL};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};

const CF_UNICODETEXT: u32 = 13;
const CF_HDROP: u32 = 15;

// Tells the receiving application to copy, never move. Without it a target
// that honours the drop effect - Explorer above all - may delete the file it
// read, which is the one in the attachment store.
const DROPEFFECT_COPY: u32 = 2;

const MAX_FORMAT_BYTES: usize = 16 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 48 * 1024 * 1024;

pub struct Snapshot {
    entries: Vec<(u32, Vec<u8>)>,

    pub complete: bool,
}

impl Snapshot {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

struct Session;

impl Session {
    fn open() -> Result<Self, String> {
        for attempt in 0..8u64 {
            if unsafe { OpenClipboard(ptr::null_mut()) } != 0 {
                return Ok(Session);
            }
            thread::sleep(Duration::from_millis(5 * (attempt + 1)));
        }
        Err("The clipboard is being held by another application.".into())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { CloseClipboard() };
    }
}

pub fn capture() -> Result<Snapshot, String> {
    let _session = Session::open()?;

    let mut entries = Vec::new();
    let mut complete = true;
    let mut total = 0usize;
    let mut format = 0u32;

    loop {
        format = unsafe { EnumClipboardFormats(format) };
        if format == 0 {
            break;
        }

        let handle = unsafe { GetClipboardData(format) };
        if handle.is_null() {
            complete = false;
            continue;
        }

        let global = handle as HGLOBAL;
        let size = unsafe { GlobalSize(global) };
        if size == 0 || size > MAX_FORMAT_BYTES || total.saturating_add(size) > MAX_TOTAL_BYTES {
            complete = false;
            continue;
        }

        let pointer = unsafe { GlobalLock(global) };
        if pointer.is_null() {
            complete = false;
            continue;
        }
        let bytes = unsafe { std::slice::from_raw_parts(pointer as *const u8, size) }.to_vec();
        unsafe { GlobalUnlock(global) };

        total += size;
        entries.push((format, bytes));
    }

    Ok(Snapshot { entries, complete })
}

pub fn restore(snapshot: &Snapshot) -> Result<(), String> {
    let _session = Session::open()?;
    unsafe { EmptyClipboard() };

    for (format, bytes) in &snapshot.entries {
        let Some(handle) = allocate(bytes) else {
            continue;
        };
        if unsafe { SetClipboardData(*format, handle as HANDLE) }.is_null() {
            log::warn!("could not restore clipboard format {format}");
        }
    }
    Ok(())
}

pub fn set_text(text: &str) -> Result<(), String> {
    let mut units: Vec<u16> = text.encode_utf16().collect();
    units.push(0);
    let bytes = unsafe { std::slice::from_raw_parts(units.as_ptr() as *const u8, units.len() * 2) };

    let _session = Session::open()?;
    unsafe { EmptyClipboard() };

    let handle = allocate(bytes).ok_or("Could not allocate memory for the clipboard.")?;
    if unsafe { SetClipboardData(CF_UNICODETEXT, handle as HANDLE) }.is_null() {
        return Err("Windows refused the clipboard write.".into());
    }
    Ok(())
}

pub fn set_files(paths: &[&Path]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("There are no files to put on the clipboard.".into());
    }

    // struct DROPFILES { DWORD pFiles; POINT pt; BOOL fNC; BOOL fWide; }
    // Every field is fixed width, so this is 20 bytes on 32- and 64-bit alike.
    const HEADER: usize = 20;

    let mut list: Vec<u16> = Vec::new();
    for path in paths {
        let full = std::fs::canonicalize(path)
            .map_err(|error| format!("{} cannot be read: {error}", path.display()))?;

        let text = full.to_string_lossy();
        let text = text.strip_prefix(r"\\?\").unwrap_or(&text);
        list.extend(text.encode_utf16());
        list.push(0);
    }
    list.push(0);

    let mut bytes = vec![0u8; HEADER + list.len() * 2];
    bytes[0..4].copy_from_slice(&(HEADER as u32).to_le_bytes());

    bytes[16..20].copy_from_slice(&1u32.to_le_bytes());
    unsafe {
        ptr::copy_nonoverlapping(
            list.as_ptr() as *const u8,
            bytes[HEADER..].as_mut_ptr(),
            list.len() * 2,
        );
    }

    let _session = Session::open()?;
    unsafe { EmptyClipboard() };

    let handle = allocate(&bytes).ok_or("Could not allocate memory for the clipboard.")?;
    if unsafe { SetClipboardData(CF_HDROP, handle as HANDLE) }.is_null() {
        return Err("Windows refused the file list.".into());
    }

    if let Some(format) = register("Preferred DropEffect") {
        if let Some(handle) = allocate(&DROPEFFECT_COPY.to_le_bytes()) {
            if unsafe { SetClipboardData(format, handle as HANDLE) }.is_null() {
                log::warn!("could not mark the drop as a copy");
            }
        }
    }

    Ok(())
}

fn register(name: &str) -> Option<u32> {
    let mut wide: Vec<u16> = name.encode_utf16().collect();
    wide.push(0);
    match unsafe { RegisterClipboardFormatW(wide.as_ptr()) } {
        0 => None,
        format => Some(format),
    }
}

pub fn get_text() -> Result<Option<String>, String> {
    let _session = Session::open()?;

    let handle = unsafe { GetClipboardData(CF_UNICODETEXT) };
    if handle.is_null() {
        return Ok(None);
    }

    unsafe {
        let size = GlobalSize(handle as HGLOBAL);
        if size < 2 {
            return Ok(Some(String::new()));
        }
        if size > MAX_FORMAT_BYTES {
            return Err("The clipboard holds more text than Ampello will insert.".into());
        }
        let pointer = GlobalLock(handle as HGLOBAL) as *const u16;
        if pointer.is_null() {
            return Err("The clipboard's text could not be read.".into());
        }
        let units = std::slice::from_raw_parts(pointer, size / 2);
        let end = units
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(units.len());
        let text = String::from_utf16_lossy(&units[..end]);
        GlobalUnlock(handle as HGLOBAL);
        Ok(Some(text))
    }
}

pub fn clear() -> Result<(), String> {
    let _session = Session::open()?;
    unsafe { EmptyClipboard() };
    Ok(())
}

fn allocate(bytes: &[u8]) -> Option<HGLOBAL> {
    unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1));
        if handle.is_null() {
            return None;
        }
        let pointer = GlobalLock(handle);
        if pointer.is_null() {
            return None;
        }
        ptr::copy_nonoverlapping(bytes.as_ptr(), pointer as *mut u8, bytes.len());
        GlobalUnlock(handle);
        Some(handle)
    }
}
