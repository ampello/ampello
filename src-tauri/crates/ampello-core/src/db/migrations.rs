// SPDX-License-Identifier: GPL-3.0-or-later
use crate::error::Result;
use rusqlite::Connection;

// Ordered and append-only. Never edit a migration that has shipped; add another.
const MIGRATIONS: &[&str] = &[
    r#"
    CREATE TABLE categories (
        id          TEXT PRIMARY KEY,
        name        TEXT NOT NULL,
        position    INTEGER NOT NULL DEFAULT 0,
        created_at  INTEGER NOT NULL
    );
    CREATE UNIQUE INDEX idx_categories_name ON categories(name COLLATE NOCASE);

    CREATE TABLE snippets (
        id          TEXT PRIMARY KEY,
        "trigger"   TEXT NOT NULL,
        title       TEXT NOT NULL DEFAULT '',
        content     TEXT NOT NULL DEFAULT '',
        enabled     INTEGER NOT NULL DEFAULT 1,
        favorite    INTEGER NOT NULL DEFAULT 0,
        category_id TEXT REFERENCES categories(id) ON DELETE SET NULL,
        usage_count INTEGER NOT NULL DEFAULT 0,
        created_at  INTEGER NOT NULL,
        updated_at  INTEGER NOT NULL
    );
    CREATE UNIQUE INDEX idx_snippets_trigger ON snippets("trigger");
    CREATE INDEX idx_snippets_category ON snippets(category_id);
    CREATE INDEX idx_snippets_enabled ON snippets(enabled);
    CREATE INDEX idx_snippets_updated ON snippets(updated_at DESC);

    CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    "#,
    r#"
    ALTER TABLE snippets ADD COLUMN last_used_at INTEGER;
    CREATE INDEX idx_snippets_last_used ON snippets(last_used_at DESC);
    "#,
    r#"
    ALTER TABLE snippets DROP COLUMN title;
    "#,
    r#"
    CREATE TABLE attachments (
        id          TEXT PRIMARY KEY,
        snippet_id  TEXT NOT NULL REFERENCES snippets(id) ON DELETE CASCADE,
        position    INTEGER NOT NULL DEFAULT 0,
        name        TEXT NOT NULL,
        mime        TEXT NOT NULL DEFAULT '',
        digest      TEXT NOT NULL,
        size_bytes  INTEGER NOT NULL DEFAULT 0,
        created_at  INTEGER NOT NULL
    );
    CREATE INDEX idx_attachments_snippet ON attachments(snippet_id, position);
    CREATE INDEX idx_attachments_digest ON attachments(digest);

    ALTER TABLE snippets ADD COLUMN attachments_first INTEGER NOT NULL DEFAULT 1;
    ALTER TABLE snippets ADD COLUMN strict_order INTEGER NOT NULL DEFAULT 0;
    "#,
];

pub fn apply(conn: &Connection) -> Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let target = MIGRATIONS.len() as i64;

    if current > target {
        log::warn!(
            "database schema version {current} is newer than this build understands ({target})"
        );
        return Ok(());
    }

    for (index, sql) in MIGRATIONS.iter().enumerate() {
        let version = index as i64 + 1;
        if version <= current {
            continue;
        }
        conn.execute_batch(&format!(
            "BEGIN; {sql} PRAGMA user_version = {version}; COMMIT;"
        ))?;
        log::info!("applied database migration {version}");
    }
    Ok(())
}
