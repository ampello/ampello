// SPDX-License-Identifier: GPL-3.0-or-later
use super::{new_id, now_ms};
use crate::error::{Error, Result};
use crate::models::*;
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::collections::HashSet;

const COLUMNS: &str = r#"
    id, snippet_id, position, name, mime, digest, size_bytes, created_at
"#;

fn map(row: &Row<'_>) -> rusqlite::Result<Attachment> {
    Ok(Attachment {
        id: row.get("id")?,
        snippet_id: row.get("snippet_id")?,
        position: row.get("position")?,
        name: row.get("name")?,
        mime: row.get("mime")?,
        digest: row.get("digest")?,
        size_bytes: row.get("size_bytes")?,
        created_at: row.get("created_at")?,

        present: false,
    })
}

pub fn list(conn: &Connection, snippet_id: &str) -> Result<Vec<Attachment>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM attachments WHERE snippet_id = ?1 ORDER BY position, created_at"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![snippet_id], map)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> Result<Attachment> {
    let sql = format!("SELECT {COLUMNS} FROM attachments WHERE id = ?1");
    conn.query_row(&sql, params![id], map)
        .optional()?
        .ok_or_else(|| Error::not_found("That attachment no longer exists."))
}

pub fn count(conn: &Connection, snippet_id: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM attachments WHERE snippet_id = ?1",
        params![snippet_id],
        |row| row.get(0),
    )?)
}

pub fn total_bytes(conn: &Connection, snippet_id: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(size_bytes), 0) FROM attachments WHERE snippet_id = ?1",
        params![snippet_id],
        |row| row.get(0),
    )?)
}

pub fn add(
    conn: &Connection,
    snippet_id: &str,
    stored: &crate::attachments::Stored,
    mime: &str,
) -> Result<Attachment> {
    let existing = count(conn, snippet_id)?;
    if existing as usize >= crate::attachments::MAX_PER_SNIPPET {
        return Err(Error::invalid(format!(
            "A snippet can carry at most {} files.",
            crate::attachments::MAX_PER_SNIPPET
        )));
    }
    let total = total_bytes(conn, snippet_id)? as u64 + stored.size_bytes as u64;
    if total > crate::attachments::MAX_SNIPPET_BYTES {
        return Err(Error::invalid(format!(
            "That would put this snippet over {} MB of attachments.",
            crate::attachments::MAX_SNIPPET_BYTES / (1024 * 1024)
        )));
    }

    let next: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM attachments WHERE snippet_id = ?1",
        params![snippet_id],
        |row| row.get(0),
    )?;

    let id = new_id();
    conn.execute(
        r#"INSERT INTO attachments
             (id, snippet_id, position, name, mime, digest, size_bytes, created_at)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
        params![
            id,
            snippet_id,
            next,
            stored.name,
            mime,
            stored.digest,
            stored.size_bytes,
            now_ms()
        ],
    )?;

    touch_snippet(conn, snippet_id)?;
    get(conn, &id)
}

pub fn remove(conn: &Connection, id: &str) -> Result<()> {
    let attachment = get(conn, id)?;
    conn.execute("DELETE FROM attachments WHERE id = ?1", params![id])?;
    compact(conn, &attachment.snippet_id)?;
    touch_snippet(conn, &attachment.snippet_id)?;
    Ok(())
}

// Attachments the caller left out keep their relative place at the end rather
// than being dropped, so a stale list from the interface cannot lose a file.
pub fn reorder(conn: &Connection, snippet_id: &str, ids: &[String]) -> Result<Vec<Attachment>> {
    let current = list(conn, snippet_id)?;
    let known: HashSet<&str> = current.iter().map(|a| a.id.as_str()).collect();

    let mut order: Vec<String> = Vec::with_capacity(current.len());
    let mut seen: HashSet<&str> = HashSet::new();
    for id in ids {
        if known.contains(id.as_str()) && seen.insert(id.as_str()) {
            order.push(id.clone());
        }
    }
    for attachment in &current {
        if !seen.contains(attachment.id.as_str()) {
            order.push(attachment.id.clone());
        }
    }

    for (position, id) in order.iter().enumerate() {
        conn.execute(
            "UPDATE attachments SET position = ?1 WHERE id = ?2 AND snippet_id = ?3",
            params![position as i64, id, snippet_id],
        )?;
    }

    touch_snippet(conn, snippet_id)?;
    list(conn, snippet_id)
}

fn compact(conn: &Connection, snippet_id: &str) -> Result<()> {
    let remaining = list(conn, snippet_id)?;
    for (position, attachment) in remaining.iter().enumerate() {
        if attachment.position != position as i64 {
            conn.execute(
                "UPDATE attachments SET position = ?1 WHERE id = ?2",
                params![position as i64, attachment.id],
            )?;
        }
    }
    Ok(())
}

fn touch_snippet(conn: &Connection, snippet_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE snippets SET updated_at = ?1 WHERE id = ?2",
        params![now_ms(), snippet_id],
    )?;
    Ok(())
}

pub fn live_blobs(conn: &Connection) -> Result<HashSet<(String, String)>> {
    let mut stmt = conn.prepare("SELECT digest, name FROM attachments")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let mut out = HashSet::new();
    for row in rows {
        out.insert(row?);
    }
    Ok(out)
}
