// SPDX-License-Identifier: GPL-3.0-or-later
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use ampello_core::db;
use ampello_core::SettingsPatch;

use crate::state::AppState;
use crate::{window, OPEN_SETTINGS_EVENT, SETTINGS_EVENT};

const OPEN: &str = "open";
const TOGGLE: &str = "toggle";
const SETTINGS: &str = "settings";
const QUIT: &str = "quit";

pub struct TrayState {
    expansion: CheckMenuItem<Wry>,
}

pub fn create(app: &AppHandle, expansion_enabled: bool) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, OPEN, "Open Ampello", true, None::<&str>)?;
    let expansion = CheckMenuItem::with_id(
        app,
        TOGGLE,
        "Expand snippets",
        true,
        expansion_enabled,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, SETTINGS, "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, "Quit Ampello", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &open,
            &expansion,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id("ampello")
        .tooltip("Ampello")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(on_tray_icon_event);

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;

    app.manage(TrayState { expansion });
    Ok(())
}

pub fn sync(app: &AppHandle, expansion_enabled: bool) {
    if let Some(tray) = app.try_state::<TrayState>() {
        let _ = tray.expansion.set_checked(expansion_enabled);
    }
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        OPEN => window::show(app),
        SETTINGS => {
            window::show(app);
            let _ = app.emit(OPEN_SETTINGS_EVENT, ());
        }
        TOGGLE => toggle_expansion(app),
        QUIT => quit(app),
        _ => {}
    }
}

fn on_tray_icon_event(tray: &TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        window::show(tray.app_handle());
    }
}

fn toggle_expansion(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };

    let current = match state.db().with(db::settings::load) {
        Ok(settings) => settings.expansion_enabled,
        Err(error) => {
            log::warn!("could not read settings from the tray: {error}");
            return;
        }
    };
    let next = !current;

    let applied = state.db().with(|conn| {
        db::settings::apply(
            conn,
            SettingsPatch {
                expansion_enabled: Some(next),
                ..Default::default()
            },
        )
    });
    let settings = match applied {
        Ok(settings) => settings,
        Err(error) => {
            log::warn!("could not change expansion from the tray: {error}");
            return;
        }
    };

    state.input.refresh();

    crate::apply_desktop_settings(app, &settings);
    let _ = app.emit(SETTINGS_EVENT, ());
}

fn quit(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.input.shutdown();
    }
    app.exit(0);
}
