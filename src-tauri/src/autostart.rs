// SPDX-License-Identifier: GPL-3.0-or-later
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

pub fn sync(app: &AppHandle, wanted: bool) {
    let manager = app.autolaunch();
    let current = manager.is_enabled().unwrap_or(false);
    if current == wanted {
        return;
    }

    if wanted && cfg!(debug_assertions) {
        log::warn!(
            "refusing to register a debug build for launch at startup; \
             install a release build and enable it from there"
        );
        return;
    }

    let outcome = if wanted {
        manager.enable()
    } else {
        manager.disable()
    };
    match outcome {
        Ok(()) => log::info!("launch at startup: {}", if wanted { "on" } else { "off" }),
        Err(error) => log::warn!("could not change launch at startup: {error}"),
    }
}
