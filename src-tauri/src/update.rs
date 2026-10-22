// SPDX-License-Identifier: GPL-3.0-or-later
//! Checking GitHub for a newer release.
//!
//! This is the only network request Ampello makes, and it is made only when
//! the user asks for it. Nothing about the library is sent: it is a plain GET
//! of the public "latest release" record.

use ampello_core::{Error, Result};
use serde::Serialize;
use std::time::Duration;

const LATEST_API: &str = "https://api.github.com/repos/ampello/ampello/releases/latest";
const LATEST_PAGE: &str = "https://github.com/ampello/ampello/releases/latest";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub available: bool,
}

fn parse(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.trim().trim_start_matches('v');
    let core = core.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|part| part.parse::<u64>().ok());
    Some((
        parts.next()??,
        parts.next().flatten().unwrap_or(0),
        parts.next().flatten().unwrap_or(0),
    ))
}

fn is_newer(latest: &str, current: &str) -> bool {
    match (parse(latest), parse(current)) {
        (Some(latest), Some(current)) => latest > current,
        _ => false,
    }
}

fn fetch_latest_tag() -> std::result::Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build();
    let body = agent
        .get(LATEST_API)
        .set("User-Agent", concat!("Ampello/", env!("CARGO_PKG_VERSION")))
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|error| match error {
            ureq::Error::Status(code, _) => format!("GitHub answered with status {code}."),
            ureq::Error::Transport(_) => "Could not reach GitHub. Check your connection.".into(),
        })?
        .into_string()
        .map_err(|error| error.to_string())?;

    let release: serde_json::Value =
        serde_json::from_str(&body).map_err(|_| "GitHub sent an unexpected reply.".to_string())?;
    release["tag_name"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "GitHub sent an unexpected reply.".into())
}

#[tauri::command]
pub async fn check_for_update() -> Result<UpdateInfo> {
    let tag = tauri::async_runtime::spawn_blocking(fetch_latest_tag)
        .await
        .map_err(|error| Error::Internal(error.to_string()))?
        .map_err(Error::invalid)?;

    let current = env!("CARGO_PKG_VERSION").to_string();
    let latest = tag.trim_start_matches('v').to_string();
    log::info!("update check: running {current}, latest is {latest}");
    Ok(UpdateInfo {
        available: is_newer(&latest, &current),
        current,
        latest,
    })
}

#[tauri::command]
pub fn open_download_page() -> Result<()> {
    open::that_detached(LATEST_PAGE).map_err(|error| Error::Internal(error.to_string()))
}

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn compares_versions_numerically() {
        assert!(is_newer("0.4.0", "0.3.0"));
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(is_newer("1.0", "0.99.0"));
        assert!(!is_newer("0.3.0", "0.3.0"));
        assert!(!is_newer("0.2.9", "0.3.0"));
        assert!(!is_newer("nonsense", "0.3.0"));
    }
}
