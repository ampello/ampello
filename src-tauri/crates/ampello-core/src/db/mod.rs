// SPDX-License-Identifier: GPL-3.0-or-later
pub mod attachments;
pub mod categories;
mod migrations;

pub mod settings;
pub mod snippets;

#[cfg(test)]
mod tests;

pub use crate::models::*;
pub use settings::{Settings, SettingsPatch};

use crate::error::{Error, Result};
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub struct Database {
    conn: Mutex<Connection>,
    path: PathBuf,

    recovered_from: Option<PathBuf>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        match Self::open_verified(path) {
            Ok(database) => Ok(database),
            Err(error) => {
                if !path.exists() {
                    return Err(error);
                }
                log::error!("the database at {} is unusable: {error}", path.display());
                let moved = quarantine(path)?;
                log::error!("moved it to {}", moved.display());

                let mut database = Self::open_verified(path)?;
                database.recovered_from = Some(moved);
                Ok(database)
            }
        }
    }

    fn open_verified(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        configure(&conn)?;
        integrity_check(&conn)?;
        migrations::apply(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: path.to_path_buf(),
            recovered_from: None,
        })
    }

    pub fn recovered_from(&self) -> Option<&Path> {
        self.recovered_from.as_deref()
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        configure(&conn)?;
        migrations::apply(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: PathBuf::from(":memory:"),
            recovered_from: None,
        })
    }

    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let guard = self.conn.lock();
        f(&guard)
    }

    pub fn with_mut<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        let mut guard = self.conn.lock();
        f(&mut guard)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn attachments(&self) -> crate::attachments::Store {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        crate::attachments::Store::new(parent.join("attachments"))
    }

    pub fn info(&self) -> Result<DatabaseInfo> {
        self.with(|conn| {
            let snippet_count: i64 =
                conn.query_row("SELECT COUNT(*) FROM snippets", [], |r| r.get(0))?;
            let category_count: i64 =
                conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))?;
            let schema_version: i64 =
                conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
            let page_count: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
            let page_size: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;
            Ok(DatabaseInfo {
                path: self.path.to_string_lossy().into_owned(),
                recovered_from: self
                    .recovered_from
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned()),
                snippet_count,
                category_count,
                schema_version,
                size_bytes: page_count * page_size,
            })
        })
    }
}

fn integrity_check(conn: &Connection) -> Result<()> {
    let verdict: String = conn.query_row("PRAGMA quick_check(1)", [], |row| row.get(0))?;
    if verdict.eq_ignore_ascii_case("ok") {
        Ok(())
    } else {
        Err(Error::Internal(format!(
            "the database failed its integrity check: {verdict}"
        )))
    }
}

// Moves a damaged database aside, along with its write-ahead log. Nothing is
// deleted: a corrupt file can often still be salvaged.
fn quarantine(path: &Path) -> Result<PathBuf> {
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "ampello".to_string());
    let parent = path.parent().unwrap_or_else(|| Path::new("."));

    let mut destination = parent.join(format!("{stem}-damaged-{}.db", now_ms()));
    let mut attempt = 1;
    while destination.exists() {
        destination = parent.join(format!("{stem}-damaged-{}-{attempt}.db", now_ms()));
        attempt += 1;
    }

    std::fs::rename(path, &destination)?;

    for suffix in ["-wal", "-shm"] {
        let companion = PathBuf::from(format!("{}{suffix}", path.display()));
        if companion.exists() {
            let _ = std::fs::rename(
                &companion,
                PathBuf::from(format!("{}{suffix}", destination.display())),
            );
        }
    }
    Ok(destination)
}

fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(())
}
