// SPDX-License-Identifier: GPL-3.0-or-later
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use ampello_core::Database;
use parking_lot::{Mutex, RwLock};

use crate::input::InputService;
use crate::library::Resolved;

/// The library currently open, behind a lock so it can be exchanged while the
/// application runs.
///
/// Switching between a personal and a shared library used to require a
/// restart, which meant the change appeared to do nothing until the user
/// worked out why. Everything that reads snippets goes through `db()` and
/// therefore picks up the exchange on its next call, including the expansion
/// engine's worker thread.
pub struct Library {
    db: RwLock<Arc<Database>>,
    location: RwLock<Resolved>,
}

impl Library {
    pub fn new(db: Arc<Database>, location: Resolved) -> Self {
        Self {
            db: RwLock::new(db),
            location: RwLock::new(location),
        }
    }

    pub fn db(&self) -> Arc<Database> {
        Arc::clone(&self.db.read())
    }

    pub fn location(&self) -> Resolved {
        self.location.read().clone()
    }

    pub fn swap(&self, db: Arc<Database>, location: Resolved) {
        *self.db.write() = db;
        *self.location.write() = location;
    }
}

pub struct AppState {
    pub library: Arc<Library>,
    pub input: InputService,

    pub shortcut_error: Mutex<Option<String>>,

    pub start_hidden: AtomicBool,
}

impl AppState {
    pub fn new(library: Arc<Library>, input: InputService, start_hidden: bool) -> Self {
        Self {
            library,
            input,
            shortcut_error: Mutex::new(None),
            start_hidden: AtomicBool::new(start_hidden),
        }
    }

    pub fn db(&self) -> Arc<Database> {
        self.library.db()
    }
}
