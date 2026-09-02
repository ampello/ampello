// SPDX-License-Identifier: GPL-3.0-or-later
pub mod autostart;
pub mod commands;
pub mod input;
pub mod library;
pub mod shortcut;
pub mod state;
pub mod tray;
pub mod window;

use std::sync::Arc;

use ampello_core::{Database, Settings};
use tauri::{AppHandle, Emitter, Manager};

use input::InputService;
use state::AppState;

pub const EXPANDED_EVENT: &str = "ampello://expanded";

pub const SETTINGS_EVENT: &str = "ampello://settings-changed";

pub const OPEN_SETTINGS_EVENT: &str = "ampello://open-settings";

pub const HIDDEN_FLAG: &str = "--hidden";

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(2_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("ampello".into()),
                    },
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            window::show(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![HIDDEN_FLAG]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;

            // The personal library is always adopted first: a shared one is a
            // separate library, not a move, and switching to it must not leave
            // the account's own snippets stranded under the previous name.
            adopt_previous_library(&data_dir, &data_dir.join("ampello.db"));

            let location = library::resolve(&data_dir);
            if let Some(problem) = &location.problem {
                log::error!("the shared library is unavailable: {problem}");
            }
            let db_path = location.database_path();
            log::info!(
                "opening {} library at {}",
                if location.shared {
                    "shared"
                } else {
                    "personal"
                },
                db_path.display()
            );

            let database = Arc::new(Database::open(&db_path)?);
            if let Some(damaged) = database.recovered_from() {
                log::error!(
                    "the previous database could not be read; it is kept at {}",
                    damaged.display()
                );
            }
            let settings = database.with(ampello_core::db::settings::load)?;

            let handle = app.handle().clone();
            let input = InputService::start(
                Arc::clone(&database),
                Box::new(move |snippet_id: &str| {
                    let _ = handle.emit(EXPANDED_EVENT, snippet_id.to_string());
                }),
            );

            let start_hidden = std::env::args().any(|argument| argument == HIDDEN_FLAG);
            if start_hidden {
                log::info!("started by the system; staying in the tray");
            }
            app.manage(AppState::new(database, input, start_hidden, location));

            let handle = app.handle().clone();
            tray::create(&handle, settings.expansion_enabled)?;
            apply_desktop_settings(&handle, &settings);

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() != window::MAIN {
                    return;
                }

                if close_to_tray(window.app_handle()) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::update_settings,
            commands::list_snippets,
            commands::search_snippets,
            commands::get_snippet,
            commands::create_snippet,
            commands::update_snippet,
            commands::delete_snippet,
            commands::trigger_available,
            commands::add_attachments,
            commands::pick_attachments,
            commands::remove_attachment,
            commands::reorder_attachments,
            commands::attachment_bytes,
            commands::attachments_size,
            commands::list_categories,
            commands::create_category,
            commands::rename_category,
            commands::delete_category,
            commands::database_info,
            commands::engine_status,
            commands::set_expansion_enabled,
            commands::shortcut_error,
            commands::ready_to_show,
            commands::export_backup,
            commands::import_backup,
            commands::restart_engine,
            commands::library_info,
            commands::choose_shared_library,
            commands::use_personal_library,
            commands::restart_app,
            commands::diagnostics,
        ])
        .build(tauri::generate_context!())
        .expect("Ampello failed to start")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = app.try_state::<AppState>() {
                    state.input.shutdown();
                }
            }
        });
}

// The application was renamed from Repla. The data directory is derived from
// the bundle identifier, so without this a rename would point at an empty
// folder and leave the user's library on disk under a name nothing looks for.
const PREVIOUS_IDENTIFIER: &str = "com.yohann.repla";
const PREVIOUS_DATABASE: &str = "repla.db";

fn adopt_previous_library(data_dir: &std::path::Path, db_path: &std::path::Path) {
    // Never runs against a library already in use.
    if db_path.exists() {
        return;
    }
    let Some(parent) = data_dir.parent() else {
        return;
    };
    let previous = parent.join(PREVIOUS_IDENTIFIER);
    let previous_db = previous.join(PREVIOUS_DATABASE);
    if !previous_db.is_file() {
        return;
    }

    log::info!(
        "found a library under the previous name at {}; moving it across",
        previous.display()
    );

    // The write-ahead log holds writes the .db file does not yet; moving the
    // database without it silently loses the most recent edits.
    for suffix in ["", "-wal", "-shm"] {
        let from = previous.join(format!("{PREVIOUS_DATABASE}{suffix}"));
        if !from.exists() {
            continue;
        }
        let to = std::path::PathBuf::from(format!("{}{suffix}", db_path.display()));
        if let Err(error) = std::fs::rename(&from, &to) {
            log::error!("could not move {}: {error}", from.display());
            return;
        }
    }

    let attachments_from = previous.join("attachments");
    let attachments_to = data_dir.join("attachments");
    if attachments_from.is_dir() && !attachments_to.exists() {
        if let Err(error) = std::fs::rename(&attachments_from, &attachments_to) {
            log::error!("could not move the attachment store: {error}");
        }
    }

    log::info!("the previous library is now at {}", db_path.display());
}

fn close_to_tray(app: &AppHandle) -> bool {
    app.try_state::<AppState>()
        .and_then(|state| state.db.with(ampello_core::db::settings::load).ok())
        .map(|settings| settings.close_to_tray)
        .unwrap_or(true)
}

pub fn apply_desktop_settings(app: &AppHandle, settings: &Settings) {
    autostart::sync(app, settings.launch_at_startup);
    tray::sync(app, settings.expansion_enabled);

    let problem = shortcut::apply(app, settings);
    if let Some(state) = app.try_state::<AppState>() {
        *state.shortcut_error.lock() = problem;
    }
}
