// SPDX-License-Identifier: GPL-3.0-or-later
use tauri::{AppHandle, Manager};

pub const MAIN: &str = "main";

pub fn show(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN) else {
        return;
    };
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN) {
        let _ = window.hide();
    }
}

pub fn toggle(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN) else {
        return;
    };
    let visible = window.is_visible().unwrap_or(false);
    let focused = window.is_focused().unwrap_or(false);
    if visible && focused {
        let _ = window.hide();
    } else {
        show(app);
    }
}
