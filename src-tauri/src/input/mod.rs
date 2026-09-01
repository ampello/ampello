// SPDX-License-Identifier: GPL-3.0-or-later
use serde::Serialize;

#[cfg(windows)]
#[path = "win/mod.rs"]
mod platform;

#[cfg(not(windows))]
#[path = "stub.rs"]
mod platform;

pub use platform::InputService;

pub type ExpandedCallback = Box<dyn Fn(&str) + Send + Sync + 'static>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub running: bool,

    pub enabled: bool,
    pub trigger_count: usize,

    pub error: Option<String>,
    pub platform: String,

    pub keystrokes_seen: u64,
    pub expansions: u64,

    pub last_expansion_error: Option<String>,
}
