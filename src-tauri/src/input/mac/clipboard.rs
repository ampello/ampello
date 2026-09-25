// SPDX-License-Identifier: GPL-3.0-or-later
//! Clipboard access through the tools every Mac ships with (`pbcopy`,
//! `pbpaste`, `osascript`), so nothing here links against AppKit. Each call
//! costs a process launch, which is invisible next to the paste itself.
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub enum Snapshot {
    Empty,
    Text(String),
    /// Something other than plain text (an image, files, rich text). It cannot
    /// be put back faithfully.
    Other,
}

impl Snapshot {
    pub fn complete(&self) -> bool {
        !matches!(self, Snapshot::Other)
    }
}

// Without a locale the tools fall back to a legacy encoding under a GUI launch
// and mangle anything that is not ASCII.
fn tool(name: &str) -> Command {
    let mut command = Command::new(name);
    command.env("LANG", "en_US.UTF-8").env("LC_ALL", "en_US.UTF-8");
    command
}

fn kinds() -> Result<String, String> {
    let output = tool("osascript")
        .args(["-e", "clipboard info"])
        .output()
        .map_err(|error| format!("could not read the clipboard: {error}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn get_text() -> Result<Option<String>, String> {
    let info = kinds()?;
    if !is_plain_text(&info) {
        return Ok(None);
    }
    let output = tool("pbpaste")
        .output()
        .map_err(|error| format!("could not read the clipboard: {error}"))?;
    if !output.status.success() {
        return Err("could not read the clipboard".into());
    }
    Ok(Some(String::from_utf8_lossy(&output.stdout).into_owned()))
}

fn is_plain_text(info: &str) -> bool {
    let trimmed = info.trim();
    if trimmed.is_empty() || trimmed == "{}" {
        return false;
    }
    // Every kind listed must be a flavour of text.
    trimmed
        .trim_matches(|c: char| c == '{' || c == '}')
        .split("},")
        .map(|entry| entry.trim_matches(|c: char| c == '{' || c == '}' || c.is_whitespace()))
        .all(|entry| {
            let name = entry.split(',').next().unwrap_or("").trim();
            matches!(
                name,
                "string" | "Unicode text" | "«class utf8»" | "«class ut16»"
            )
        })
}

pub fn capture() -> Result<Snapshot, String> {
    let info = kinds()?;
    let trimmed = info.trim();
    if trimmed.is_empty() || trimmed == "{}" {
        return Ok(Snapshot::Empty);
    }
    if !is_plain_text(&info) {
        return Ok(Snapshot::Other);
    }
    match get_text()? {
        Some(text) => Ok(Snapshot::Text(text)),
        None => Ok(Snapshot::Other),
    }
}

pub fn set_text(text: &str) -> Result<(), String> {
    let mut child = tool("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not write to the clipboard: {error}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|error| format!("could not write to the clipboard: {error}"))?;
    }
    let status = child
        .wait()
        .map_err(|error| format!("could not write to the clipboard: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("could not write to the clipboard".into())
    }
}

const SET_FILES: &str = r#"
function run(argv) {
  ObjC.import('AppKit');
  var pasteboard = $.NSPasteboard.generalPasteboard;
  pasteboard.clearContents;
  var urls = $.NSMutableArray.alloc.init;
  for (var i = 0; i < argv.length; i++) {
    urls.addObject($.NSURL.fileURLWithPath(argv[i]));
  }
  return pasteboard.writeObjects(urls) ? "ok" : "failed";
}
"#;

const CLEAR: &str = r#"
ObjC.import('AppKit');
$.NSPasteboard.generalPasteboard.clearContents;
"#;

pub fn set_files(paths: &[&Path]) -> Result<(), String> {
    let output = tool("osascript")
        .args(["-l", "JavaScript", "-e", SET_FILES])
        .args(paths)
        .output()
        .map_err(|error| format!("could not put files on the clipboard: {error}"))?;
    if output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "ok" {
        Ok(())
    } else {
        Err("could not put files on the clipboard".into())
    }
}

pub fn clear() -> Result<(), String> {
    tool("osascript")
        .args(["-l", "JavaScript", "-e", CLEAR])
        .output()
        .map(|_| ())
        .map_err(|error| format!("could not clear the clipboard: {error}"))
}

pub fn restore(snapshot: &Snapshot) -> Result<(), String> {
    match snapshot {
        Snapshot::Text(text) => set_text(text),
        Snapshot::Empty => clear(),
        Snapshot::Other => Ok(()),
    }
}
