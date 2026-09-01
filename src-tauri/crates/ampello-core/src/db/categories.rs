// SPDX-License-Identifier: GPL-3.0-or-later
use crate::models::Category;
use super::{new_id, now_ms};
use crate::error::{Error, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};

fn map(row: &Row<'_>) -> rusqlite::Result<Category> {
    Ok(Category {
        id: row.get("id")?,
        name: row.get("name")?,
        position: row.get("position")?,
        created_at: row.get("created_at")?,
    })
}

pub fn list(conn: &Connection) -> Result<Vec<Category>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, position, created_at FROM categories \
         ORDER BY position ASC, name COLLATE NOCASE ASC",
    )?;
    let rows = stmt.query_map([], map)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn normalize_name(raw: &str) -> Result<String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(Error::invalid("A collection needs a name."));
    }
    if name.chars().count() > 48 {
        return Err(Error::invalid(
            "A collection name cannot be longer than 48 characters.",
        ));
    }
    Ok(name.to_string())
}

pub fn create(conn: &Connection, name: &str) -> Result<Category> {
    let name = normalize_name(name)?;
    if name_taken(conn, &name, None)? {
        return Err(Error::conflict(format!("\"{name}\" already exists.")));
    }
    let next_position: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM categories",
        [],
        |r| r.get(0),
    )?;
    let id = new_id();
    conn.execute(
        "INSERT INTO categories (id, name, position, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, name, next_position, now_ms()],
    )?;
    get(conn, &id)
}

pub fn rename(conn: &Connection, id: &str, name: &str) -> Result<Category> {
    let name = normalize_name(name)?;
    if name_taken(conn, &name, Some(id))? {
        return Err(Error::conflict(format!("\"{name}\" already exists.")));
    }
    let affected = conn.execute(
        "UPDATE categories SET name = ?2 WHERE id = ?1",
        params![id, name],
    )?;
    if affected == 0 {
        return Err(Error::not_found("That collection no longer exists."));
    }
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM categories WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(Error::not_found("That collection no longer exists."));
    }
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> Result<Category> {
    conn.query_row(
        "SELECT id, name, position, created_at FROM categories WHERE id = ?1",
        params![id],
        map,
    )
    .optional()?
    .ok_or_else(|| Error::not_found("That collection no longer exists."))
}

fn name_taken(conn: &Connection, name: &str, except_id: Option<&str>) -> Result<bool> {
    let found: Option<String> = conn
        .query_row(
            "SELECT id FROM categories WHERE name = ?1 COLLATE NOCASE",
            params![name],
            |r| r.get(0),
        )
        .optional()?;
    Ok(match (found, except_id) {
        (None, _) => false,
        (Some(id), Some(skip)) => id != skip,
        (Some(_), None) => true,
    })
}
