// SPDX-License-Identifier: GPL-3.0-or-later
use ampello_core::attachments;
use ampello_core::backup;
use ampello_core::db;
use ampello_core::db::now_ms;
use ampello_core::models::{
    Category, DatabaseInfo, NewSnippet, Snippet, SnippetPatch, SnippetSummary,
};
use ampello_core::Error;
use ampello_core::{Result, Settings, SettingsPatch};

use crate::input::EngineStatus;
use crate::state::AppState;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings> {
    state.db().with(db::settings::load)
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> Result<Settings> {
    let settings = state.db().with(|conn| db::settings::apply(conn, patch))?;
    state.input.refresh();
    crate::apply_desktop_settings(&app, &settings);
    Ok(settings)
}

#[tauri::command]
pub fn list_snippets(state: State<'_, AppState>) -> Result<Vec<SnippetSummary>> {
    state.db().with(db::snippets::list_summaries)
}

#[tauri::command]
pub fn search_snippets(state: State<'_, AppState>, query: String) -> Result<Vec<SnippetSummary>> {
    state.db().with(|conn| db::snippets::search(conn, &query))
}

#[tauri::command]
pub fn get_snippet(state: State<'_, AppState>, id: String) -> Result<Snippet> {
    let snippet = state.db().with(|conn| db::snippets::get(conn, &id))?;
    Ok(with_presence(&state, snippet))
}

#[tauri::command]
pub fn create_snippet(state: State<'_, AppState>, snippet: NewSnippet) -> Result<Snippet> {
    let created = state
        .db()
        .with(|conn| db::snippets::create(conn, snippet))?;
    state.input.refresh();
    Ok(created)
}

#[tauri::command]
pub fn update_snippet(
    state: State<'_, AppState>,
    id: String,
    patch: SnippetPatch,
) -> Result<Snippet> {
    let updated = state
        .db()
        .with(|conn| db::snippets::update(conn, &id, patch))?;
    state.input.refresh();
    Ok(with_presence(&state, updated))
}

#[tauri::command]
pub fn delete_snippet(state: State<'_, AppState>, id: String) -> Result<()> {
    state.db().with(|conn| db::snippets::delete(conn, &id))?;
    state.input.refresh();

    collect_garbage(&state);
    Ok(())
}

#[tauri::command]
pub fn trigger_available(
    state: State<'_, AppState>,
    trigger: String,
    except_id: Option<String>,
) -> Result<bool> {
    state
        .db()
        .with(|conn| db::snippets::trigger_available(conn, &trigger, except_id.as_deref()))
}

#[tauri::command]
pub fn list_categories(state: State<'_, AppState>) -> Result<Vec<Category>> {
    state.db().with(db::categories::list)
}

#[tauri::command]
pub fn create_category(state: State<'_, AppState>, name: String) -> Result<Category> {
    state.db().with(|conn| db::categories::create(conn, &name))
}

#[tauri::command]
pub fn rename_category(state: State<'_, AppState>, id: String, name: String) -> Result<Category> {
    state
        .db()
        .with(|conn| db::categories::rename(conn, &id, &name))
}

#[tauri::command]
pub fn delete_category(state: State<'_, AppState>, id: String) -> Result<()> {
    state.db().with(|conn| db::categories::delete(conn, &id))
}

#[tauri::command]
pub fn database_info(state: State<'_, AppState>) -> Result<DatabaseInfo> {
    state.db().info()
}

#[tauri::command]
pub fn engine_status(state: State<'_, AppState>) -> EngineStatus {
    state.input.status()
}

#[tauri::command]
pub fn set_expansion_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<Settings> {
    let settings = state.db().with(|conn| {
        db::settings::apply(
            conn,
            SettingsPatch {
                expansion_enabled: Some(enabled),
                ..Default::default()
            },
        )
    })?;
    state.input.refresh();

    crate::apply_desktop_settings(&app, &settings);
    Ok(settings)
}

#[tauri::command]
pub fn ready_to_show(app: AppHandle, state: State<'_, AppState>) {
    use std::sync::atomic::Ordering;
    if state.start_hidden.swap(false, Ordering::SeqCst) {
        return;
    }
    crate::window::show(&app);
}

#[tauri::command]
pub fn shortcut_error(state: State<'_, AppState>) -> Option<String> {
    state.shortcut_error.lock().clone()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub snippets: usize,
    pub collections: usize,
}

#[tauri::command]
pub fn export_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    format: String,
) -> Result<Option<ExportResult>> {
    use tauri_plugin_dialog::DialogExt;

    let yaml = format != "json";
    let backup = state.db().with(|conn| backup::export(conn, now_ms()))?;

    let attachments: usize = backup
        .snippets
        .iter()
        .map(|snippet| snippet.attachments.len())
        .sum();

    let (extension, label) = if attachments > 0 {
        ("ampellozip", "Ampello archive")
    } else if yaml {
        ("yaml", "YAML")
    } else {
        ("json", "JSON")
    };

    let chosen = app
        .dialog()
        .file()
        .set_title("Export snippets")
        .set_file_name(format!("ampello-snippets.{extension}"))
        .add_filter(label, &[extension])
        .blocking_save_file();

    let Some(chosen) = chosen else {
        return Ok(None);
    };
    let path = chosen
        .into_path()
        .map_err(|error| Error::Internal(error.to_string()))?;

    if attachments > 0 {
        let missing = backup::write_archive(&path, &backup, &state.db().attachments())?;
        if !missing.is_empty() {
            log::warn!(
                "some attachments could not be exported: {}",
                missing.join("; ")
            );
        }
    } else {
        let text = if yaml {
            backup::to_yaml(&backup)?
        } else {
            backup::to_json(&backup)?
        };
        std::fs::write(&path, text)?;
    }

    log::info!(
        "exported {} snippets and {attachments} attachment(s) to {}",
        backup.snippets.len(),
        path.display()
    );

    Ok(Some(ExportResult {
        path: path.to_string_lossy().into_owned(),
        snippets: backup.snippets.len(),
        collections: backup.categories.len(),
    }))
}

#[tauri::command]
pub fn import_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    mode: String,
) -> Result<Option<backup::ImportReport>> {
    use tauri_plugin_dialog::DialogExt;

    let chosen = app
        .dialog()
        .file()
        .set_title("Import snippets")
        .add_filter(
            "Ampello backup",
            &["ampellozip", "replazip", "zip", "yaml", "yml", "json"],
        )
        .blocking_pick_file();

    let Some(chosen) = chosen else {
        return Ok(None);
    };
    let path = chosen
        .into_path()
        .map_err(|error| Error::Internal(error.to_string()))?;

    let bytes = std::fs::read(&path)?;
    let store = state.db().attachments();
    let mode = backup::ImportMode::parse(&mode);

    let (parsed, mut file_problems) = if backup::is_archive(&bytes) {
        backup::read_archive(&bytes, &store)?
    } else {
        let text = String::from_utf8(bytes)
            .map_err(|_| Error::invalid("That file is not text, and not a Ampello archive."))?;
        (backup::parse(&text)?, Vec::new())
    };

    let mut report = state
        .db()
        .with(|conn| backup::import(conn, &parsed, mode, &store))?;
    report.problems.append(&mut file_problems);

    state.input.refresh();

    log::info!(
        "imported from {}: {} added, {} replaced, {} skipped",
        path.display(),
        report.added,
        report.replaced,
        report.skipped
    );
    Ok(Some(report))
}

#[tauri::command]
pub fn restart_engine(state: State<'_, AppState>) -> EngineStatus {
    state.input.restart();
    state.input.status()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub log_directory: Option<String>,
}

#[tauri::command]
pub fn diagnostics(app: AppHandle) -> Diagnostics {
    use tauri::Manager;
    Diagnostics {
        log_directory: app
            .path()
            .app_log_dir()
            .ok()
            .map(|path| path.to_string_lossy().into_owned()),
    }
}

fn with_presence(state: &AppState, mut snippet: Snippet) -> Snippet {
    let store = state.db().attachments();
    for attachment in &mut snippet.attachments {
        attachment.present = store.exists(&attachment.digest, &attachment.name);
    }
    snippet
}

#[tauri::command]
pub fn add_attachments(
    state: State<'_, AppState>,
    snippet_id: String,
    paths: Vec<String>,
) -> Result<Snippet> {
    let sources: Vec<std::path::PathBuf> =
        paths.into_iter().map(std::path::PathBuf::from).collect();
    attach_all(&state, &snippet_id, sources)
}

fn attach_all(
    state: &AppState,
    snippet_id: &str,
    sources: Vec<std::path::PathBuf>,
) -> Result<Snippet> {
    if sources.is_empty() {
        return Err(Error::invalid("There are no files to attach."));
    }

    let store = state.db().attachments();
    let mut problems: Vec<String> = Vec::new();

    for path in sources {
        let stored = match store.add_file(&path) {
            Ok(stored) => stored,
            Err(error) => {
                problems.push(format!("{}: {error}", name_of(&path)));
                continue;
            }
        };
        let mime = attachments::mime_for(&stored.name);
        if let Err(error) = state
            .db()
            .with(|conn| db::attachments::add(conn, snippet_id, &stored, mime))
        {
            problems.push(format!("{}: {error}", stored.name));
        }
    }

    if !problems.is_empty() {
        log::warn!("some files could not be attached: {}", problems.join("; "));
        if problems.len() == 1 {
            return Err(Error::invalid(problems.remove(0)));
        }
        return Err(Error::invalid(format!(
            "{} files could not be attached: {}",
            problems.len(),
            problems.join("; ")
        )));
    }

    let snippet = state
        .db()
        .with(|conn| db::snippets::get(conn, snippet_id))?;
    Ok(with_presence(state, snippet))
}

fn name_of(path: &std::path::Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[tauri::command]
pub fn pick_attachments(
    app: AppHandle,
    state: State<'_, AppState>,
    snippet_id: String,
) -> Result<Option<Snippet>> {
    use tauri_plugin_dialog::DialogExt;

    let Some(chosen) = app
        .dialog()
        .file()
        .set_title("Attach files")
        .blocking_pick_files()
    else {
        return Ok(None);
    };

    let mut sources = Vec::with_capacity(chosen.len());
    for file in chosen {
        match file.into_path() {
            Ok(path) => sources.push(path),
            Err(error) => log::warn!("a chosen file had no usable path: {error}"),
        }
    }

    attach_all(&state, &snippet_id, sources).map(Some)
}

#[tauri::command]
pub fn remove_attachment(state: State<'_, AppState>, id: String) -> Result<Snippet> {
    let snippet = state.db().with(|conn| {
        let attachment = db::attachments::get(conn, &id)?;
        db::attachments::remove(conn, &id)?;
        db::snippets::get(conn, &attachment.snippet_id)
    })?;
    collect_garbage(&state);
    Ok(with_presence(&state, snippet))
}

#[tauri::command]
pub fn reorder_attachments(
    state: State<'_, AppState>,
    snippet_id: String,
    ids: Vec<String>,
) -> Result<Snippet> {
    let snippet = state.db().with(|conn| {
        db::attachments::reorder(conn, &snippet_id, &ids)?;
        db::snippets::get(conn, &snippet_id)
    })?;
    Ok(with_presence(&state, snippet))
}

#[tauri::command]
pub fn attachments_size(state: State<'_, AppState>) -> u64 {
    state.db().attachments().size_bytes()
}

fn collect_garbage(state: &AppState) {
    let live = match state.db().with(db::attachments::live_blobs) {
        Ok(live) => live,
        Err(error) => {
            log::warn!("could not work out which attachments are still in use: {error}");
            return;
        }
    };
    match state.db().attachments().gc(&live) {
        Ok(0) => {}
        Ok(count) => log::info!("removed {count} unused attachment file(s)"),
        Err(error) => log::warn!("could not tidy the attachment store: {error}"),
    }
}

#[tauri::command]
pub fn attachment_bytes(state: State<'_, AppState>, id: String) -> Result<tauri::ipc::Response> {
    let attachment = state.db().with(|conn| db::attachments::get(conn, &id))?;

    if !attachments::is_previewable(&attachment.mime) {
        return Err(Error::invalid("That file is not one Ampello can show."));
    }
    if attachment.size_bytes > attachments::MAX_PREVIEW_BYTES {
        return Err(Error::invalid("That file is too large to preview."));
    }

    let bytes = state
        .db()
        .attachments()
        .read(&attachment.digest, &attachment.name)?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInfo {
    pub path: String,
    pub shared: bool,
    pub personal_path: String,
    pub default_shared_path: String,
    pub problem: Option<String>,
}

#[tauri::command]
pub fn library_info(state: State<'_, AppState>) -> LibraryInfo {
    let location = state.library.location();
    LibraryInfo {
        path: location.dir.to_string_lossy().into_owned(),
        shared: location.shared,
        personal_path: location.personal_dir.to_string_lossy().into_owned(),
        default_shared_path: crate::library::default_shared_dir()
            .to_string_lossy()
            .into_owned(),
        problem: location.problem.clone(),
    }
}

/// Pick a folder to keep a shared library in, and point this account at it.
///
/// Returns the chosen path, or `None` when the dialog was dismissed. The change
/// takes effect on the next start: swapping the database under a running
/// expansion engine would mean rebuilding every part of the application that
/// holds a handle to it.
#[tauri::command]
pub fn choose_shared_library(app: AppHandle, state: State<'_, AppState>) -> Result<Option<String>> {
    use tauri_plugin_dialog::DialogExt;

    let start = if state.library.location().shared {
        state.library.location().dir.clone()
    } else {
        crate::library::default_shared_dir()
    };
    if let Some(parent) = start.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let Some(chosen) = app
        .dialog()
        .file()
        .set_title("Choose a shared library folder")
        .set_directory(&start)
        .blocking_pick_folder()
    else {
        return Ok(None);
    };
    let path = chosen
        .into_path()
        .map_err(|error| Error::Internal(error.to_string()))?;

    switch_library(&app, &state, Some(&path))?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

/// Go back to this account's own library. The shared one is left untouched.
#[tauri::command]
pub fn use_personal_library(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    switch_library(&app, &state, None)
}

/// Point this account at a different library and open it immediately.
///
/// The database is exchanged behind a lock rather than requiring a restart:
/// every read goes through `AppState::db`, and the expansion engine resolves
/// the current handle per expansion, so both follow the swap. The engine is
/// then refreshed so its trigger set comes from the new library, and the
/// interface is told to reload.
///
/// The pointer is written only after the new library opens. A directory that
/// turns out to be unusable therefore leaves the account exactly where it was,
/// rather than pointing it at something that will not load on next start.
fn switch_library(
    app: &AppHandle,
    state: &AppState,
    shared: Option<&std::path::Path>,
) -> Result<()> {
    let personal = state.library.location().personal_dir;

    if let Some(dir) = shared {
        crate::library::probe(dir).map_err(|error| Error::invalid(error.to_string()))?;
    }

    let dir = shared
        .map(|d| d.to_path_buf())
        .unwrap_or_else(|| personal.clone());
    let db_path = dir.join("ampello.db");
    let database = std::sync::Arc::new(ampello_core::Database::open(&db_path)?);

    crate::library::set(&personal, shared).map_err(|error| Error::invalid(error.to_string()))?;

    let location = crate::library::Resolved {
        dir,
        personal_dir: personal,
        shared: shared.is_some(),
        problem: None,
    };
    state.library.swap(database, location);

    state.input.refresh();
    let _ = app.emit(crate::LIBRARY_EVENT, ());

    log::info!("library switched to {}", db_path.display());
    Ok(())
}
