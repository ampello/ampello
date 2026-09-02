// SPDX-License-Identifier: GPL-3.0-or-later
use serde::{Deserialize, Serialize};

use crate::db::{attachments, categories, snippets};
use crate::error::{Error, Result};
use crate::models::{NewSnippet, SnippetPatch};
use rusqlite::Connection;

#[cfg(test)]
#[path = "backup_tests.rs"]
mod tests;

pub const FORMAT_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub version: u32,
    #[serde(default)]
    pub exported_at: i64,
    #[serde(default)]
    pub categories: Vec<BackupCategory>,
    #[serde(default)]
    pub snippets: Vec<BackupSnippet>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupCategory {
    pub name: String,
    #[serde(default)]
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSnippet {
    pub trigger: String,

    #[serde(default)]
    pub collection: Option<String>,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub usage_count: i64,
    #[serde(default)]
    pub content: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<BackupAttachment>,
    #[serde(default = "yes")]
    pub attachments_first: bool,
    #[serde(default)]
    pub strict_order: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupAttachment {
    pub name: String,
    pub digest: String,
    #[serde(default)]
    pub size_bytes: i64,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportMode {
    Skip,

    Replace,
}

impl ImportMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "replace" => ImportMode::Replace,
            _ => ImportMode::Skip,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub added: usize,
    pub replaced: usize,
    pub skipped: usize,
    pub collections_created: usize,

    pub problems: Vec<String>,
}

pub fn export(conn: &Connection, exported_at: i64) -> Result<Backup> {
    let all_categories = categories::list(conn)?;
    let name_of = |id: &Option<String>| -> Option<String> {
        let id = id.as_deref()?;
        all_categories
            .iter()
            .find(|category| category.id == id)
            .map(|category| category.name.clone())
    };

    let mut out = Vec::new();
    for summary in snippets::list_summaries(conn)? {
        let full = snippets::get(conn, &summary.id)?;
        out.push(BackupSnippet {
            trigger: full.trigger,
            collection: name_of(&full.category_id),
            enabled: full.enabled,
            favorite: full.favorite,
            usage_count: full.usage_count,
            content: full.content,
            attachments: full
                .attachments
                .into_iter()
                .map(|attachment| BackupAttachment {
                    name: attachment.name,
                    digest: attachment.digest,
                    size_bytes: attachment.size_bytes,
                })
                .collect(),
            attachments_first: full.attachments_first,
            strict_order: full.strict_order,
        });
    }

    out.sort_by(|a, b| {
        a.trigger
            .to_lowercase()
            .cmp(&b.trigger.to_lowercase())
            .then_with(|| a.trigger.cmp(&b.trigger))
    });

    Ok(Backup {
        version: FORMAT_VERSION,
        exported_at,
        categories: all_categories
            .into_iter()
            .map(|category| BackupCategory {
                name: category.name,
                position: category.position,
            })
            .collect(),
        snippets: out,
    })
}

pub fn to_json(backup: &Backup) -> Result<String> {
    serde_json::to_string_pretty(backup)
        .map_err(|error| Error::Internal(format!("could not write JSON: {error}")))
}

// Emits readable YAML, then parses it back and compares. If the readable form
// would not survive the round trip, falls back to the library's own emitter:
// legibility is not worth a corrupted backup.
pub fn to_yaml(backup: &Backup) -> Result<String> {
    let pretty = emit_yaml(backup);
    match serde_norway::from_str::<Backup>(&pretty) {
        Ok(parsed) if &parsed == backup => Ok(pretty),
        _ => {
            log::warn!("falling back to plain YAML: the readable form would not round-trip");
            serde_norway::to_string(backup)
                .map_err(|error| Error::Internal(format!("could not write YAML: {error}")))
        }
    }
}

fn emit_yaml(backup: &Backup) -> String {
    let mut out = String::new();
    out.push_str("# Ampello snippet backup\n");
    out.push_str("# Content is stored exactly as written. Edit with care.\n\n");
    out.push_str(&format!("version: {}\n", backup.version));
    out.push_str(&format!("exportedAt: {}\n", backup.exported_at));

    if backup.categories.is_empty() {
        out.push_str("categories: []\n");
    } else {
        out.push_str("categories:\n");
        for category in &backup.categories {
            out.push_str(&format!("  - name: {}\n", quote(&category.name)));
            out.push_str(&format!("    position: {}\n", category.position));
        }
    }

    if backup.snippets.is_empty() {
        out.push_str("snippets: []\n");
        return out;
    }

    out.push_str("snippets:\n");
    for snippet in &backup.snippets {
        out.push_str(&format!("  - trigger: {}\n", quote(&snippet.trigger)));
        match &snippet.collection {
            Some(name) => out.push_str(&format!("    collection: {}\n", quote(name))),
            None => out.push_str("    collection: null\n"),
        }
        out.push_str(&format!("    enabled: {}\n", snippet.enabled));
        out.push_str(&format!("    favorite: {}\n", snippet.favorite));
        out.push_str(&format!("    usageCount: {}\n", snippet.usage_count));
        out.push_str(&emit_content(&snippet.content));
    }
    out
}

const CONTENT_INDENT: &str = "      ";

fn emit_content(content: &str) -> String {
    match block_style(content) {
        Some((indicator, body)) => {
            let mut out = format!("    content: {indicator}\n");
            for line in body.split('\n') {
                if line.is_empty() {
                    out.push('\n');
                } else {
                    out.push_str(CONTENT_INDENT);
                    out.push_str(line);
                    out.push('\n');
                }
            }
            out
        }
        None => format!("    content: {}\n", quote(content)),
    }
}

fn block_style(content: &str) -> Option<(&'static str, &str)> {
    if content.is_empty() || content.contains('\r') {
        return None;
    }
    if content.starts_with(' ') || content.starts_with('\t') {
        return None;
    }
    if content.ends_with("\n\n") {
        return None;
    }

    let body = content.strip_suffix('\n');
    let indicator = if body.is_some() { "|" } else { "|-" };
    let body = body.unwrap_or(content);

    for line in body.split('\n') {
        if line.ends_with(' ') || line.ends_with('\t') {
            return None;
        }
    }
    Some((indicator, body))
}

fn quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub const ARCHIVE_ENTRY: &str = "backup.yaml";

fn archive_path(digest: &str, name: &str) -> String {
    format!("attachments/{digest}/{name}")
}

pub fn is_archive(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04])
}

pub fn write_archive(
    path: &std::path::Path,
    backup: &Backup,
    store: &crate::attachments::Store,
) -> Result<Vec<String>> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let file = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let document = to_yaml(backup)?;
    zip.start_file(ARCHIVE_ENTRY, options)
        .map_err(|error| Error::Internal(format!("could not write the archive: {error}")))?;
    zip.write_all(document.as_bytes())?;

    let mut problems = Vec::new();
    let mut written = std::collections::HashSet::new();

    for snippet in &backup.snippets {
        for attachment in &snippet.attachments {
            if !written.insert((attachment.digest.clone(), attachment.name.clone())) {
                continue;
            }
            let bytes = match store.read(&attachment.digest, &attachment.name) {
                Ok(bytes) => bytes,
                Err(error) => {
                    problems.push(format!("{}: {error}", attachment.name));
                    continue;
                }
            };
            let entry = archive_path(&attachment.digest, &attachment.name);
            zip.start_file(&entry, options).map_err(|error| {
                Error::Internal(format!("could not write {entry} to the archive: {error}"))
            })?;
            zip.write_all(&bytes)?;
        }
    }

    zip.finish()
        .map_err(|error| Error::Internal(format!("could not close the archive: {error}")))?;
    Ok(problems)
}

// Entries are read into memory, hashed, and checked against the digest the
// document declares before anything is written. Nothing is written to a path
// the archive chose, so an archive cannot escape the store or substitute one
// file for another.
pub fn read_archive(
    bytes: &[u8],
    store: &crate::attachments::Store,
) -> Result<(Backup, Vec<String>)> {
    use std::io::Read;

    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor)
        .map_err(|error| Error::invalid(format!("That archive could not be opened: {error}")))?;

    let document = {
        let mut entry = zip.by_name(ARCHIVE_ENTRY).map_err(|_| {
            Error::invalid(format!(
                "That archive has no {ARCHIVE_ENTRY}, so it is not a Ampello backup."
            ))
        })?;
        let mut text = String::new();
        entry.read_to_string(&mut text)?;
        text
    };
    let backup = parse(&document)?;

    let mut wanted: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for snippet in &backup.snippets {
        for attachment in &snippet.attachments {
            wanted.insert(
                archive_path(&attachment.digest, &attachment.name),
                attachment.digest.clone(),
            );
        }
    }

    let mut problems = Vec::new();
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|error| Error::invalid(format!("That archive is damaged: {error}")))?;
        let entry_name = entry.name().to_string();
        let Some(declared) = wanted.get(&entry_name) else {
            continue;
        };
        if entry.size() > crate::attachments::MAX_ATTACHMENT_BYTES {
            problems.push(format!("{entry_name}: larger than Ampello will attach"));
            continue;
        }
        let mut contents = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut contents)?;

        if &crate::attachments::digest_of(&contents) != declared {
            problems.push(format!("{entry_name}: contents do not match the backup"));
            continue;
        }

        let name = entry_name
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string();
        if let Err(error) = store.add_bytes(&name, &contents) {
            problems.push(format!("{name}: {error}"));
        }
    }

    Ok((backup, problems))
}

pub fn parse(text: &str) -> Result<Backup> {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return Err(Error::invalid("That file is empty."));
    }

    let parsed: std::result::Result<Backup, String> = if trimmed.starts_with('{') {
        serde_json::from_str(text).map_err(|error| error.to_string())
    } else {
        serde_norway::from_str(text).map_err(|error| error.to_string())
    };

    let backup = parsed.map_err(|error| {
        Error::invalid(format!("That does not look like a Ampello backup: {error}"))
    })?;

    if backup.version > FORMAT_VERSION {
        return Err(Error::invalid(format!(
            "That backup was written by a newer version of Ampello (format {}).",
            backup.version
        )));
    }
    Ok(backup)
}

pub fn import(
    conn: &Connection,
    backup: &Backup,
    mode: ImportMode,
    store: &crate::attachments::Store,
) -> Result<ImportReport> {
    let mut report = ImportReport::default();

    for category in &backup.categories {
        if find_category(conn, &category.name)?.is_none() {
            match categories::create(conn, &category.name) {
                Ok(_) => report.collections_created += 1,
                Err(error) => report
                    .problems
                    .push(format!("Collection “{}”: {error}", category.name)),
            }
        }
    }

    for snippet in &backup.snippets {
        let category_id = match &snippet.collection {
            Some(name) if !name.trim().is_empty() => match find_category(conn, name)? {
                Some(id) => Some(id),
                None => match categories::create(conn, name) {
                    Ok(created) => {
                        report.collections_created += 1;
                        Some(created.id)
                    }
                    Err(error) => {
                        report
                            .problems
                            .push(format!("Collection “{name}”: {error}"));
                        None
                    }
                },
            },
            _ => None,
        };

        let trigger = match snippets::normalize_trigger(&snippet.trigger) {
            Ok(trigger) => trigger,
            Err(error) => {
                report
                    .problems
                    .push(format!("“{}”: {error}", snippet.trigger));
                continue;
            }
        };

        let existing = find_snippet(conn, &trigger)?;
        match (existing, mode) {
            (Some(_), ImportMode::Skip) => report.skipped += 1,
            (Some(id), ImportMode::Replace) => {
                let patch = SnippetPatch {
                    content: Some(snippet.content.clone()),
                    enabled: Some(snippet.enabled),
                    favorite: Some(snippet.favorite),
                    category_id: Some(category_id),
                    ..Default::default()
                };
                match snippets::update(conn, &id, patch) {
                    Ok(_) => {
                        if snippet.usage_count > 0 {
                            let _ = snippets::restore_usage(conn, &id, snippet.usage_count);
                        }

                        if let Err(error) = clear_attachments(conn, &id) {
                            report
                                .problems
                                .push(format!("\u{201c}{trigger}\u{201d}: {error}"));
                        }
                        restore_attachments(conn, &id, snippet, store, &mut report);
                        report.replaced += 1;
                    }
                    Err(error) => report.problems.push(format!("“{trigger}”: {error}")),
                }
            }
            (None, _) => {
                let created = snippets::create(
                    conn,
                    NewSnippet {
                        trigger: trigger.clone(),
                        content: snippet.content.clone(),
                        category_id,
                    },
                );
                match created {
                    Ok(created) => {
                        if !snippet.enabled || snippet.favorite {
                            let _ = snippets::update(
                                conn,
                                &created.id,
                                SnippetPatch {
                                    enabled: Some(snippet.enabled),
                                    favorite: Some(snippet.favorite),
                                    ..Default::default()
                                },
                            );
                        }
                        if snippet.usage_count > 0 {
                            let _ = snippets::restore_usage(conn, &created.id, snippet.usage_count);
                        }
                        restore_attachments(conn, &created.id, snippet, store, &mut report);
                        report.added += 1;
                    }
                    Err(error) => report.problems.push(format!("“{trigger}”: {error}")),
                }
            }
        }
    }

    Ok(report)
}

fn clear_attachments(conn: &Connection, snippet_id: &str) -> Result<()> {
    for attachment in attachments::list(conn, snippet_id)? {
        attachments::remove(conn, &attachment.id)?;
    }
    Ok(())
}

fn restore_attachments(
    conn: &Connection,
    snippet_id: &str,
    snippet: &BackupSnippet,
    store: &crate::attachments::Store,
    report: &mut ImportReport,
) {
    if snippet.attachments.is_empty() {
        return;
    }

    for attachment in &snippet.attachments {
        let stored = crate::attachments::Stored {
            digest: attachment.digest.clone(),
            name: crate::attachments::sanitize_name(&attachment.name),
            size_bytes: attachment.size_bytes,
        };
        if !store.exists(&stored.digest, &stored.name) {
            report.problems.push(format!(
                "\u{201c}{}\u{201d}: the file {} was not in that backup",
                snippet.trigger, stored.name
            ));
        }
        let mime = crate::attachments::mime_for(&stored.name);
        if let Err(error) = attachments::add(conn, snippet_id, &stored, mime) {
            report
                .problems
                .push(format!("\u{201c}{}\u{201d}: {error}", snippet.trigger));
        }
    }

    let _ = snippets::update(
        conn,
        snippet_id,
        SnippetPatch {
            attachments_first: Some(snippet.attachments_first),
            strict_order: Some(snippet.strict_order),
            ..Default::default()
        },
    );
}

fn find_category(conn: &Connection, name: &str) -> Result<Option<String>> {
    Ok(categories::list(conn)?
        .into_iter()
        .find(|category| category.name.eq_ignore_ascii_case(name.trim()))
        .map(|category| category.id))
}

fn find_snippet(conn: &Connection, trigger: &str) -> Result<Option<String>> {
    use rusqlite::{params, OptionalExtension};
    Ok(conn
        .query_row(
            r#"SELECT id FROM snippets WHERE "trigger" = ?1"#,
            params![trigger],
            |row| row.get(0),
        )
        .optional()?)
}
