// SPDX-License-Identifier: GPL-3.0-or-later
use std::sync::Arc;

use ampello_core::Database;

use super::{EngineStatus, ExpandedCallback};

pub struct InputService {
    platform: String,
}

impl InputService {
    pub fn start(_db: Arc<Database>, _on_expanded: ExpandedCallback) -> Self {
        Self {
            platform: std::env::consts::OS.to_string(),
        }
    }

    pub fn refresh(&self) {}

    pub fn restart(&self) {}

    pub fn insert_clipboard(&self) {}

    pub fn shutdown(&self) {}

    pub fn status(&self) -> EngineStatus {
        EngineStatus {
            running: false,
            enabled: false,
            trigger_count: 0,
            error: Some(format!(
                "Ampello's expansion engine does not support {} yet.",
                self.platform
            )),
            platform: self.platform.clone(),
            keystrokes_seen: 0,
            expansions: 0,
            last_expansion_error: None,
        }
    }
}
