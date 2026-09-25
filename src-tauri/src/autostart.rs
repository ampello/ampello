// SPDX-License-Identifier: GPL-3.0-or-later
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

/// Make the system's startup registration match the setting.
///
/// The registration stores the path of the executable that was running when it
/// was made. It used to be left alone whenever it was merely present, so after
/// an update or a reinstall to a different folder it kept pointing at a program
/// that no longer existed and the login launch silently did nothing. Turning
/// the setting on now always rewrites it, and the result is read back so a
/// registration the system refused is reported instead of assumed.
pub fn sync(app: &AppHandle, wanted: bool) {
    let manager = app.autolaunch();
    let current = manager.is_enabled().unwrap_or(false);

    if !wanted {
        if current {
            match manager.disable() {
                Ok(()) => log::info!("launch at startup: off"),
                Err(error) => log::warn!("could not turn off launch at startup: {error}"),
            }
        }
        return;
    }

    if cfg!(debug_assertions) {
        log::warn!(
            "refusing to register a debug build for launch at startup; \
             install a release build and enable it from there"
        );
        return;
    }

    if let Err(error) = manager.enable() {
        log::warn!("could not turn on launch at startup: {error}");
        return;
    }

    match manager.is_enabled() {
        Ok(true) => log::info!("launch at startup: on"),
        Ok(false) => log::warn!(
            "launch at startup was registered but the system does not report it as enabled; \
             it may be switched off in the system's startup apps settings"
        ),
        Err(error) => log::warn!("could not confirm launch at startup: {error}"),
    }
}
