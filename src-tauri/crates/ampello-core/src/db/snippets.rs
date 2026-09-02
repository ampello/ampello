// SPDX-License-Identifier: GPL-3.0-or-later
use super::{new_id, now_ms};
use crate::error::{Error, Result};
use crate::models::*;
use rusqlite::{params, Connection, OptionalExtension, Row};

const MAX_TRIGGER_CHARS: usize = 64;

pub fn normalize_trigger(raw: &str) -> Result<String> {
    let trigger = raw.trim();
    if trigger.is_empty() {
        return Err(Error::invalid("A trigger cannot be empty."));
    }
    if trigger.chars().any(|c| c == '\n' || c == '\r' || c == '\t') {
        return Err(Error::invalid(
            "A trigger cannot contain line breaks or tabs.",
        ));
    }
    let len = trigger.chars().count();
    if len > MAX_TRIGGER_CHARS {
        return Err(Error::invalid(format!(
            "A trigger cannot be longer than {MAX_TRIGGER_CHARS} characters."
        )));
    }
    Ok(trigger.to_string())
}

const SUMMARY_COLUMNS: &str = r#"
    id,
    "trigger",
    substr(content, 1, 400) AS head,
    length(content)         AS content_length,
    enabled,
    favorite,
    category_id,
    usage_count,
    last_used_at,
    created_at,
    updated_at,
    (SELECT COUNT(*) FROM attachments a WHERE a.snippet_id = snippets.id)
        AS attachment_count
"#;

const FULL_COLUMNS: &str = r#"
    id, "trigger", content, enabled, favorite,
    category_id, usage_count, last_used_at, created_at, updated_at,
    attachments_first, strict_order
"#;

fn map_summary(row: &Row<'_>) -> rusqlite::Result<SnippetSummary> {
    let head: String = row.get("head")?;
    Ok(SnippetSummary {
        id: row.get("id")?,
        trigger: row.get("trigger")?,
        preview: preview_of(&head),
        content_length: row.get("content_length")?,
        enabled: row.get::<_, i64>("enabled")? != 0,
        favorite: row.get::<_, i64>("favorite")? != 0,
        category_id: row.get("category_id")?,
        usage_count: row.get("usage_count")?,
        last_used_at: row.get("last_used_at")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        attachment_count: row.get("attachment_count")?,
    })
}

fn map_snippet(row: &Row<'_>) -> rusqlite::Result<Snippet> {
    Ok(Snippet {
        id: row.get("id")?,
        trigger: row.get("trigger")?,
        content: row.get("content")?,
        enabled: row.get::<_, i64>("enabled")? != 0,
        favorite: row.get::<_, i64>("favorite")? != 0,
        category_id: row.get("category_id")?,
        usage_count: row.get("usage_count")?,
        last_used_at: row.get("last_used_at")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        attachments_first: row.get::<_, i64>("attachments_first")? != 0,
        strict_order: row.get::<_, i64>("strict_order")? != 0,

        attachments: Vec::new(),
    })
}

pub fn list_summaries(conn: &Connection) -> Result<Vec<SnippetSummary>> {
    let sql = format!("SELECT {SUMMARY_COLUMNS} FROM snippets ORDER BY updated_at DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_summary)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

const SUMMARY_COLUMNS_PREFIXED: &str = r#"
    s.id                        AS id,
    s."trigger"                 AS "trigger",
    substr(s.content, 1, 400)   AS head,
    length(s.content)           AS content_length,
    s.enabled                   AS enabled,
    s.favorite                  AS favorite,
    s.category_id               AS category_id,
    s.usage_count               AS usage_count,
    s.last_used_at              AS last_used_at,
    s.created_at                AS created_at,
    s.updated_at                AS updated_at,
    (SELECT COUNT(*) FROM attachments a WHERE a.snippet_id = s.id)
        AS attachment_count
"#;

fn escape_like(query: &str) -> String {
    let mut out = String::with_capacity(query.len() + 8);
    for ch in query.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

pub fn search(conn: &Connection, query: &str) -> Result<Vec<SnippetSummary>> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return list_summaries(conn);
    }
    let pattern = format!("%{}%", escape_like(trimmed));

    let sql = format!(
        r#"SELECT {SUMMARY_COLUMNS_PREFIXED}
             FROM snippets s
             LEFT JOIN categories c ON c.id = s.category_id
            WHERE s."trigger" LIKE ?1 ESCAPE '\'
               OR s.content   LIKE ?1 ESCAPE '\'
               OR c.name      LIKE ?1 ESCAPE '\'
            ORDER BY
              CASE
                WHEN s."trigger" LIKE ?1 ESCAPE '\' THEN 0
                WHEN c.name      LIKE ?1 ESCAPE '\' THEN 1
                ELSE 2
              END,
              s.updated_at DESC"#
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![pattern], map_summary)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> Result<Snippet> {
    let sql = format!("SELECT {FULL_COLUMNS} FROM snippets WHERE id = ?1");
    let mut snippet = conn
        .query_row(&sql, params![id], map_snippet)
        .optional()?
        .ok_or_else(|| Error::not_found("That snippet no longer exists."))?;
    snippet.attachments = super::attachments::list(conn, id)?;
    Ok(snippet)
}

pub fn trigger_available(
    conn: &Connection,
    trigger: &str,
    except_id: Option<&str>,
) -> Result<bool> {
    let trigger = normalize_trigger(trigger)?;
    let existing: Option<String> = conn
        .query_row(
            r#"SELECT id FROM snippets WHERE "trigger" = ?1"#,
            params![trigger],
            |r| r.get(0),
        )
        .optional()?;
    Ok(match (existing, except_id) {
        (None, _) => true,
        (Some(found), Some(skip)) => found == skip,
        (Some(_), None) => false,
    })
}

pub fn create(conn: &Connection, input: NewSnippet) -> Result<Snippet> {
    let trigger = normalize_trigger(&input.trigger)?;
    if !trigger_available(conn, &trigger, None)? {
        return Err(Error::conflict(format!(
            "Another snippet already uses the trigger \"{trigger}\"."
        )));
    }
    if let Some(category_id) = input.category_id.as_deref() {
        ensure_category_exists(conn, category_id)?;
    }

    let id = new_id();
    let now = now_ms();

    conn.execute(
        r#"INSERT INTO snippets
             (id, "trigger", content, enabled, favorite, category_id,
              usage_count, created_at, updated_at)
           VALUES (?1, ?2, ?3, 1, 0, ?4, 0, ?5, ?5)"#,
        params![id, trigger, input.content, input.category_id, now],
    )?;

    get(conn, &id)
}

pub fn update(conn: &Connection, id: &str, patch: SnippetPatch) -> Result<Snippet> {
    let current = get(conn, id)?;

    let trigger = match patch.trigger.as_deref() {
        Some(raw) => {
            let normalized = normalize_trigger(raw)?;
            if normalized != current.trigger && !trigger_available(conn, &normalized, Some(id))? {
                return Err(Error::conflict(format!(
                    "Another snippet already uses the trigger \"{normalized}\"."
                )));
            }
            normalized
        }
        None => current.trigger.clone(),
    };

    let content = patch.content.unwrap_or(current.content);
    let enabled = patch.enabled.unwrap_or(current.enabled);
    let favorite = patch.favorite.unwrap_or(current.favorite);
    let category_id = match patch.category_id {
        Some(next) => {
            if let Some(cid) = next.as_deref() {
                ensure_category_exists(conn, cid)?;
            }
            next
        }
        None => current.category_id,
    };
    let attachments_first = patch.attachments_first.unwrap_or(current.attachments_first);
    let strict_order = patch.strict_order.unwrap_or(current.strict_order);

    conn.execute(
        r#"UPDATE snippets
              SET "trigger" = ?2, content = ?3, enabled = ?4,
                  favorite = ?5, category_id = ?6, updated_at = ?7,
                  attachments_first = ?8, strict_order = ?9
            WHERE id = ?1"#,
        params![
            id,
            trigger,
            content,
            enabled as i64,
            favorite as i64,
            category_id,
            now_ms(),
            attachments_first as i64,
            strict_order as i64
        ],
    )?;

    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM snippets WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(Error::not_found("That snippet no longer exists."));
    }
    Ok(())
}

pub fn restore_usage(conn: &Connection, id: &str, count: i64) -> Result<()> {
    conn.execute(
        "UPDATE snippets SET usage_count = ?2 WHERE id = ?1",
        params![id, count.max(0)],
    )?;
    Ok(())
}

pub fn enabled_triggers(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(r#"SELECT id, "trigger" FROM snippets WHERE enabled = 1"#)?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn content_of(conn: &Connection, id: &str) -> Result<String> {
    conn.query_row(
        "SELECT content FROM snippets WHERE id = ?1 AND enabled = 1",
        params![id],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| Error::not_found("That snippet is no longer available."))
}

pub fn record_usage(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE snippets SET usage_count = usage_count + 1, last_used_at = ?2 WHERE id = ?1",
        params![id, now_ms()],
    )?;
    Ok(())
}

fn ensure_category_exists(conn: &Connection, id: &str) -> Result<()> {
    let exists: Option<String> = conn
        .query_row(
            "SELECT id FROM categories WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .optional()?;
    exists
        .map(|_| ())
        .ok_or_else(|| Error::not_found("That collection no longer exists."))
}
