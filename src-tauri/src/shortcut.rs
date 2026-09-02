// SPDX-License-Identifier: GPL-3.0-or-later
use std::str::FromStr;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use ampello_core::Settings;

use crate::state::AppState;
use crate::window;

pub fn apply(app: &AppHandle, settings: &Settings) -> Option<String> {
    let _ = app.global_shortcut().unregister_all();

    let mut problems: Vec<String> = Vec::new();

    let window_accelerator = settings.global_shortcut.trim().to_string();
    if let Some(shortcut) = parse(&window_accelerator, &mut problems) {
        let handle = app.clone();
        register(
            app,
            shortcut,
            &window_accelerator,
            &mut problems,
            move || {
                window::toggle(&handle);
            },
        );
    }

    let clipboard_accelerator = settings.clipboard_shortcut.trim().to_string();
    if settings.clipboard_shortcut_enabled && settings.expansion_enabled {
        if clipboard_accelerator.eq_ignore_ascii_case(&window_accelerator) {
            log::warn!("the clipboard shortcut is the same as the window shortcut");
            problems.push(format!(
                "{clipboard_accelerator} is already Ampello's window shortcut."
            ));
        } else if let Some(shortcut) = parse(&clipboard_accelerator, &mut problems) {
            let handle = app.clone();
            register(
                app,
                shortcut,
                &clipboard_accelerator,
                &mut problems,
                move || {
                    if let Some(state) = handle.try_state::<AppState>() {
                        state.input.insert_clipboard();
                    }
                },
            );
        }
    }

    if problems.is_empty() {
        None
    } else {
        Some(problems.join(" "))
    }
}

fn parse(accelerator: &str, problems: &mut Vec<String>) -> Option<Shortcut> {
    if accelerator.is_empty() {
        return None;
    }
    match Shortcut::from_str(accelerator) {
        Ok(shortcut) => Some(shortcut),
        Err(error) => {
            log::warn!("“{accelerator}” is not a valid shortcut: {error}");
            problems.push(format!(
                "“{accelerator}” is not a shortcut Ampello understands."
            ));
            None
        }
    }
}

fn register<F>(
    app: &AppHandle,
    shortcut: Shortcut,
    accelerator: &str,
    problems: &mut Vec<String>,
    action: F,
) where
    F: Fn() + Send + Sync + 'static,
{
    let result = app
        .global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                action();
            }
        });

    match result {
        Ok(()) => log::info!("global shortcut registered: {accelerator}"),
        Err(error) => {
            log::warn!("could not register {accelerator}: {error}");
            problems.push(format!(
                "Another application is already using {accelerator}."
            ));
        }
    }
}
