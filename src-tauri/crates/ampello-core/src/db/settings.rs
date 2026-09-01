// SPDX-License-Identifier: GPL-3.0-or-later
use crate::error::{Error, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub appearance: String,
    pub expansion_enabled: bool,
    pub launch_at_startup: bool,
    pub global_shortcut: String,

    pub boundary_mode: String,
    pub preserve_boundary_char: bool,
    pub restore_clipboard: bool,

    pub injection_mode: String,

    pub typing_speed: String,

    pub clipboard_shortcut_enabled: bool,

    pub clipboard_shortcut: String,

    pub clipboard_mode: String,
    pub close_to_tray: bool,

    pub attachment_settle_ms: i64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            appearance: "system".into(),
            expansion_enabled: true,
            launch_at_startup: false,
            global_shortcut: "CommandOrControl+Shift+Space".into(),
            boundary_mode: "word".into(),
            preserve_boundary_char: true,
            restore_clipboard: true,
            injection_mode: "auto".into(),
            typing_speed: "balanced".into(),
            clipboard_shortcut_enabled: true,
            clipboard_shortcut: "CommandOrControl+Shift+V".into(),
            clipboard_mode: "type".into(),
            close_to_tray: true,
            attachment_settle_ms: 500,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    #[serde(default)]
    pub appearance: Option<String>,
    #[serde(default)]
    pub expansion_enabled: Option<bool>,
    #[serde(default)]
    pub launch_at_startup: Option<bool>,
    #[serde(default)]
    pub global_shortcut: Option<String>,
    #[serde(default)]
    pub boundary_mode: Option<String>,
    #[serde(default)]
    pub preserve_boundary_char: Option<bool>,
    #[serde(default)]
    pub restore_clipboard: Option<bool>,
    #[serde(default)]
    pub injection_mode: Option<String>,
    #[serde(default)]
    pub typing_speed: Option<String>,
    #[serde(default)]
    pub clipboard_shortcut_enabled: Option<bool>,
    #[serde(default)]
    pub clipboard_shortcut: Option<String>,
    #[serde(default)]
    pub clipboard_mode: Option<String>,
    #[serde(default)]
    pub close_to_tray: Option<bool>,
    #[serde(default)]
    pub attachment_settle_ms: Option<i64>,
}

const APPEARANCE: &str = "appearance";
const EXPANSION_ENABLED: &str = "expansion_enabled";
const LAUNCH_AT_STARTUP: &str = "launch_at_startup";
const GLOBAL_SHORTCUT: &str = "global_shortcut";
const BOUNDARY_MODE: &str = "boundary_mode";
const PRESERVE_BOUNDARY_CHAR: &str = "preserve_boundary_char";
const RESTORE_CLIPBOARD: &str = "restore_clipboard";
const INJECTION_MODE: &str = "injection_mode";
const TYPING_SPEED: &str = "typing_speed";
const CLIPBOARD_SHORTCUT_ENABLED: &str = "clipboard_shortcut_enabled";
const CLIPBOARD_SHORTCUT: &str = "clipboard_shortcut";
const CLIPBOARD_MODE: &str = "clipboard_mode";
const CLOSE_TO_TRAY: &str = "close_to_tray";
const ATTACHMENT_SETTLE_MS: &str = "attachment_settle_ms";

pub const MIN_SETTLE_MS: i64 = 100;
pub const MAX_SETTLE_MS: i64 = 5_000;

pub fn load(conn: &Connection) -> Result<Settings> {
    let defaults = Settings::default();
    Ok(Settings {
        appearance: read_str(conn, APPEARANCE)?.unwrap_or(defaults.appearance),
        expansion_enabled: read_bool(conn, EXPANSION_ENABLED)?
            .unwrap_or(defaults.expansion_enabled),
        launch_at_startup: read_bool(conn, LAUNCH_AT_STARTUP)?
            .unwrap_or(defaults.launch_at_startup),
        global_shortcut: read_str(conn, GLOBAL_SHORTCUT)?.unwrap_or(defaults.global_shortcut),
        boundary_mode: read_str(conn, BOUNDARY_MODE)?.unwrap_or(defaults.boundary_mode),
        preserve_boundary_char: read_bool(conn, PRESERVE_BOUNDARY_CHAR)?
            .unwrap_or(defaults.preserve_boundary_char),
        restore_clipboard: read_bool(conn, RESTORE_CLIPBOARD)?
            .unwrap_or(defaults.restore_clipboard),
        injection_mode: read_str(conn, INJECTION_MODE)?.unwrap_or(defaults.injection_mode),
        typing_speed: read_str(conn, TYPING_SPEED)?.unwrap_or(defaults.typing_speed),
        clipboard_shortcut_enabled: read_bool(conn, CLIPBOARD_SHORTCUT_ENABLED)?
            .unwrap_or(defaults.clipboard_shortcut_enabled),
        clipboard_shortcut: read_str(conn, CLIPBOARD_SHORTCUT)?
            .unwrap_or(defaults.clipboard_shortcut),
        clipboard_mode: read_str(conn, CLIPBOARD_MODE)?.unwrap_or(defaults.clipboard_mode),
        close_to_tray: read_bool(conn, CLOSE_TO_TRAY)?.unwrap_or(defaults.close_to_tray),
        attachment_settle_ms: read_int(conn, ATTACHMENT_SETTLE_MS)?
            .map(|value| value.clamp(MIN_SETTLE_MS, MAX_SETTLE_MS))
            .unwrap_or(defaults.attachment_settle_ms),
    })
}

pub fn apply(conn: &Connection, patch: SettingsPatch) -> Result<Settings> {
    if let Some(value) = patch.appearance.as_deref() {
        if !matches!(value, "light" | "dark" | "system") {
            return Err(Error::invalid("Appearance must be light, dark or system."));
        }
        write_str(conn, APPEARANCE, value)?;
    }
    if let Some(value) = patch.boundary_mode.as_deref() {
        if !matches!(value, "word" | "anywhere") {
            return Err(Error::invalid("Boundary mode must be word or anywhere."));
        }
        write_str(conn, BOUNDARY_MODE, value)?;
    }
    if let Some(value) = patch.global_shortcut.as_deref() {
        let value = value.trim();
        if value.is_empty() {
            return Err(Error::invalid("A global shortcut cannot be empty."));
        }
        write_str(conn, GLOBAL_SHORTCUT, value)?;
    }
    if let Some(value) = patch.expansion_enabled {
        write_bool(conn, EXPANSION_ENABLED, value)?;
    }
    if let Some(value) = patch.launch_at_startup {
        write_bool(conn, LAUNCH_AT_STARTUP, value)?;
    }
    if let Some(value) = patch.preserve_boundary_char {
        write_bool(conn, PRESERVE_BOUNDARY_CHAR, value)?;
    }
    if let Some(value) = patch.restore_clipboard {
        write_bool(conn, RESTORE_CLIPBOARD, value)?;
    }
    if let Some(value) = patch.injection_mode.as_deref() {
        if !matches!(value, "auto" | "paste" | "type") {
            return Err(Error::invalid("Injection mode must be auto, paste or type."));
        }
        write_str(conn, INJECTION_MODE, value)?;
    }
    if let Some(value) = patch.typing_speed.as_deref() {
        if !matches!(value, "fast" | "balanced" | "careful") {
            return Err(Error::invalid(
                "Typing speed must be fast, balanced or careful.",
            ));
        }
        write_str(conn, TYPING_SPEED, value)?;
    }
    if let Some(value) = patch.clipboard_shortcut_enabled {
        write_bool(conn, CLIPBOARD_SHORTCUT_ENABLED, value)?;
    }
    if let Some(value) = patch.clipboard_shortcut.as_deref() {
        let value = value.trim();
        if value.is_empty() {
            return Err(Error::invalid("A clipboard shortcut cannot be empty."));
        }
        write_str(conn, CLIPBOARD_SHORTCUT, value)?;
    }
    if let Some(value) = patch.clipboard_mode.as_deref() {
        if !matches!(value, "paste" | "type") {
            return Err(Error::invalid("Clipboard mode must be paste or type."));
        }
        write_str(conn, CLIPBOARD_MODE, value)?;
    }
    if let Some(value) = patch.close_to_tray {
        write_bool(conn, CLOSE_TO_TRAY, value)?;
    }
    if let Some(value) = patch.attachment_settle_ms {
        if !(MIN_SETTLE_MS..=MAX_SETTLE_MS).contains(&value) {
            return Err(Error::invalid(format!(
                "The attachment delay must be between {MIN_SETTLE_MS} and {MAX_SETTLE_MS} milliseconds."
            )));
        }
        write_str(conn, ATTACHMENT_SETTLE_MS, &value.to_string())?;
    }
    load(conn)
}

fn read_str(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| {
            r.get(0)
        })
        .optional()?)
}

fn read_bool(conn: &Connection, key: &str) -> Result<Option<bool>> {
    Ok(read_str(conn, key)?.map(|value| value == "1" || value.eq_ignore_ascii_case("true")))
}

fn read_int(conn: &Connection, key: &str) -> Result<Option<i64>> {
    Ok(read_str(conn, key)?.and_then(|value| value.trim().parse().ok()))
}

fn write_str(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn write_bool(conn: &Connection, key: &str, value: bool) -> Result<()> {
    write_str(conn, key, if value { "1" } else { "0" })
}
