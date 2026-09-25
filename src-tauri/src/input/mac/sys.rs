// SPDX-License-Identifier: GPL-3.0-or-later
//! The few CoreGraphics and CoreFoundation calls the expansion engine needs,
//! declared directly rather than through a wrapper crate: the surface is small
//! and stable, and it keeps Ampello independent of a binding crate's release
//! schedule.
#![allow(non_snake_case, non_upper_case_globals, dead_code)]

use std::ffi::c_void;

pub type CGEventRef = *mut c_void;
pub type CFMachPortRef = *mut c_void;
pub type CFRunLoopSourceRef = *mut c_void;
pub type CFRunLoopRef = *mut c_void;

pub type TapCallback = unsafe extern "C" fn(
    proxy: *mut c_void,
    kind: u32,
    event: CGEventRef,
    user: *mut c_void,
) -> CGEventRef;

// Event kinds.
pub const LEFT_MOUSE_DOWN: u32 = 1;
pub const RIGHT_MOUSE_DOWN: u32 = 3;
pub const KEY_DOWN: u32 = 10;
pub const OTHER_MOUSE_DOWN: u32 = 25;
pub const TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
pub const TAP_DISABLED_BY_USER: u32 = 0xFFFF_FFFF;

// Event fields.
pub const FIELD_KEYCODE: u32 = 9;
pub const FIELD_TARGET_PID: u32 = 40;
pub const FIELD_USER_DATA: u32 = 42;

// Modifier flags.
pub const FLAG_SHIFT: u64 = 0x0002_0000;
pub const FLAG_CONTROL: u64 = 0x0004_0000;
pub const FLAG_ALTERNATE: u64 = 0x0008_0000;
pub const FLAG_COMMAND: u64 = 0x0010_0000;

// Tap placement.
pub const SESSION_EVENT_TAP: u32 = 1;
pub const HEAD_INSERT: u32 = 0;
pub const TAP_OPTION_DEFAULT: u32 = 0;
pub const HID_EVENT_TAP: u32 = 0;
pub const HID_SYSTEM_STATE: i32 = 1;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    pub fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: TapCallback,
        user: *mut c_void,
    ) -> CFMachPortRef;
    pub fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    pub fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    pub fn CGEventSetIntegerValueField(event: CGEventRef, field: u32, value: i64);
    pub fn CGEventGetFlags(event: CGEventRef) -> u64;
    pub fn CGEventSetFlags(event: CGEventRef, flags: u64);
    pub fn CGEventKeyboardGetUnicodeString(
        event: CGEventRef,
        max: usize,
        actual: *mut usize,
        buffer: *mut u16,
    );
    pub fn CGEventKeyboardSetUnicodeString(event: CGEventRef, length: usize, buffer: *const u16);
    pub fn CGEventCreateKeyboardEvent(source: *mut c_void, key: u16, down: bool) -> CGEventRef;
    pub fn CGEventPost(tap: u32, event: CGEventRef);
    pub fn CGEventSourceFlagsState(state: i32) -> u64;
    pub fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
    pub fn CGPreflightListenEventAccess() -> bool;
    pub fn CGRequestListenEventAccess() -> bool;
    pub fn CGPreflightPostEventAccess() -> bool;
    pub fn CGRequestPostEventAccess() -> bool;
    pub fn AXIsProcessTrusted() -> bool;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub static kCFAllocatorDefault: *const c_void;
    pub static kCFRunLoopCommonModes: *const c_void;
    pub static kCFRunLoopDefaultMode: *const c_void;

    pub fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: CFMachPortRef,
        order: isize,
    ) -> CFRunLoopSourceRef;
    pub fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    pub fn CFRunLoopAddSource(run_loop: CFRunLoopRef, source: CFRunLoopSourceRef, mode: *const c_void);
    pub fn CFRunLoopRemoveSource(
        run_loop: CFRunLoopRef,
        source: CFRunLoopSourceRef,
        mode: *const c_void,
    );
    pub fn CFRunLoopRunInMode(mode: *const c_void, seconds: f64, return_after_source: bool) -> i32;
    pub fn CFRelease(object: *const c_void);
}
