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

pub const LIBRARY_EVENT: &str = "ampello://library-changed";

/// Another running copy of Ampello edited the library we have open.
pub const EXTERNAL_CHANGE_EVENT: &str = "ampello://external-change";

pub const HIDDEN_FLAG: &str = "--hidden";

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(2_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Folder {
                        path: data_dir().join("logs"),
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
            let data_dir = data_dir();
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
            if location.shared {
                library::share_permissions(&location.dir);
            }
            if let Some(damaged) = database.recovered_from() {
                log::error!(
                    "the previous database could not be read; it is kept at {}",
                    damaged.display()
                );
            }
            let settings = database.with(ampello_core::db::settings::load)?;
            let library = Arc::new(state::Library::new(Arc::clone(&database), location));

            let handle = app.handle().clone();
            let input = InputService::start(
                Arc::clone(&library),
                Box::new(move |snippet_id: &str| {
                    let _ = handle.emit(EXPANDED_EVENT, snippet_id.to_string());
                }),
            );

            let start_hidden = std::env::args().any(|argument| argument == HIDDEN_FLAG);
            if start_hidden {
                log::info!("started by the system; staying in the tray");
            }
            let watch_library = Arc::clone(&library);
            app.manage(AppState::new(library, input, start_hidden));
            watch_for_external_changes(app.handle().clone(), watch_library);

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
            commands::add_attachment_data,
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
            commands::diagnostics,
        ])
        .build(tauri::generate_context!())
        .expect("Ampello failed to start")
        .run(|app, event| match event {
            tauri::RunEvent::Exit => {
                if let Some(state) = app.try_state::<AppState>() {
                    state.input.shutdown();
                }
            }
            // Clicking the Dock icon while the window is hidden in the tray.
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => window::show(app),
            _ => {}
        });
}

/// Where the library lives: `%APPDATA%\Ampello` on Windows and
/// `~/Library/Application Support/Ampello` on macOS, the forms ordinary
/// applications on each use, rather than a reverse-DNS bundle identifier.
pub fn data_dir() -> std::path::PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return std::path::PathBuf::from(appdata).join("Ampello");
    }
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    if cfg!(target_os = "macos") {
        return home.join("Library/Application Support/Ampello");
    }
    home.join(".local/share/ampello")
}

// Locations the library has previously occupied, newest first. Each rename,
// of the application and then of the folder convention, left snippets on disk
// under a name nothing looks for any more.
const PREVIOUS_LOCATIONS: &[(&str, &str)] = &[
    ("com.yohann.ampello", "ampello.db"),
    ("com.yohann.repla", "repla.db"),
];

fn adopt_previous_library(data_dir: &std::path::Path, db_path: &std::path::Path) {
    // Never runs against a library already in use.
    if db_path.exists() {
        return;
    }
    let Some(parent) = data_dir.parent() else {
        return;
    };

    let Some((previous, previous_db_name)) = PREVIOUS_LOCATIONS
        .iter()
        .map(|(folder, db)| (parent.join(folder), *db))
        .find(|(dir, db)| dir.join(db).is_file())
    else {
        return;
    };

    log::info!(
        "found a library at the previous location {}; moving it across",
        previous.display()
    );

    // The write-ahead log holds writes the .db file does not yet; moving the
    // database without it silently loses the most recent edits.
    for suffix in ["", "-wal", "-shm"] {
        let from = previous.join(format!("{previous_db_name}{suffix}"));
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
        .and_then(|state| state.db().with(ampello_core::db::settings::load).ok())
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

/// Notice edits made by another running copy of Ampello.
///
/// Two accounts pointed at one shared library each hold their own open
/// connection; nothing told the other one that a snippet had changed, so its
/// window and its expansion engine stayed stale until a restart. SQLite exposes
/// a per-connection counter that moves when any *other* connection commits, so
/// a cheap poll is enough to know when to reload.
fn watch_for_external_changes(app: AppHandle, library: Arc<state::Library>) {
    let spawned = std::thread::Builder::new()
        .name("ampello-library-watch".into())
        .spawn(move || {
            let mut watched = library.db();
            let mut seen = watched.data_version().ok();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(1000));
                {
                    let location = library.location();
                    if location.shared {
                        library::share_permissions(&location.dir);
                    }
                }

                let current = library.db();
                if !Arc::ptr_eq(&current, &watched) {
                    // The library was exchanged in-process; the switch already
                    // told the interface, so only the baseline resets.
                    watched = current;
                    seen = watched.data_version().ok();
                    continue;
                }

                let version = match watched.data_version() {
                    Ok(version) => version,
                    Err(error) => {
                        log::warn!("could not check the library for changes: {error}");
                        continue;
                    }
                };
                if seen == Some(version) {
                    continue;
                }
                seen = Some(version);

                log::info!("the library was changed by another Ampello; reloading");
                if let Some(state) = app.try_state::<AppState>() {
                    state.input.refresh();
                }
                let _ = app.emit(EXTERNAL_CHANGE_EVENT, ());
            }
        });
    if let Err(error) = spawned {
        log::warn!("could not watch the library for changes: {error}");
    }
}
