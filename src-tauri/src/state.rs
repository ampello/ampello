// SPDX-License-Identifier: GPL-3.0-or-later
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use parking_lot::Mutex;
use ampello_core::Database;

use crate::input::InputService;
use crate::library::Resolved;

pub struct AppState {
    pub db: Arc<Database>,
    pub input: InputService,

    pub shortcut_error: Mutex<Option<String>>,

    pub start_hidden: AtomicBool,

    pub library: Resolved,
}

impl AppState {
    pub fn new(
        db: Arc<Database>,
        input: InputService,
        start_hidden: bool,
        library: Resolved,
    ) -> Self {
        Self {
            db,
            input,
            shortcut_error: Mutex::new(None),
            start_hidden: AtomicBool::new(start_hidden),
            library,
        }
    }
}
