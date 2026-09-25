// SPDX-License-Identifier: GPL-3.0-or-later
//! Clipboard access through `xclip` (or `xsel` for plain text). An X selection
//! only exists while a process owns it, so a helper that stays alive to serve
//! it is unavoidable; `xclip` forks into the background for exactly that.
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::process::{Command, Stdio};

pub enum Snapshot {
    Empty,
    Text(String),
    /// Something other than plain text; it cannot be put back faithfully.
    Other,
}

impl Snapshot {
    pub fn complete(&self) -> bool {
        !matches!(self, Snapshot::Other)
    }
}

const MISSING: &str = "Ampello needs the xclip package to use the clipboard. \
    Install it (for example: sudo apt install xclip) and try again.";

fn xclip(args: &[&str]) -> Command {
    let mut command = Command::new("xclip");
    command.args(["-selection", "clipboard"]).args(args);
    command
}

fn run(mut command: Command) -> Result<Option<Vec<u8>>, String> {
    match command.stderr(Stdio::null()).output() {
        Ok(output) if output.status.success() => Ok(Some(output.stdout)),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == ErrorKind::NotFound => Err(MISSING.into()),
        Err(error) => Err(format!("could not read the clipboard: {error}")),
    }
}

fn targets() -> Result<Vec<String>, String> {
    let output = run(xclip(&["-o", "-t", "TARGETS"]))?;
    Ok(output
        .map(|bytes| {
            String::from_utf8_lossy(&bytes)
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect()
        })
        .unwrap_or_default())
}

fn is_plain_text(targets: &[String]) -> bool {
    !targets.is_empty()
        && targets.iter().all(|target| {
            matches!(
                target.as_str(),
                "UTF8_STRING"
                    | "STRING"
                    | "TEXT"
                    | "COMPOUND_TEXT"
                    | "text/plain"
                    | "text/plain;charset=utf-8"
                    | "TARGETS"
                    | "TIMESTAMP"
                    | "MULTIPLE"
                    | "SAVE_TARGETS"
            )
        })
}

pub fn get_text() -> Result<Option<String>, String> {
    if !is_plain_text(&targets()?) {
        return Ok(None);
    }
    Ok(run(xclip(&["-o"]))?.map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
}

pub fn capture() -> Result<Snapshot, String> {
    let targets = targets()?;
    if targets.is_empty() {
        return Ok(Snapshot::Empty);
    }
    if !is_plain_text(&targets) {
        return Ok(Snapshot::Other);
    }
    Ok(match get_text()? {
        Some(text) => Snapshot::Text(text),
        None => Snapshot::Other,
    })
}

fn feed(args: &[&str], body: &[u8]) -> Result<(), String> {
    let mut child = xclip(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => MISSING.to_string(),
            _ => format!("could not write to the clipboard: {error}"),
        })?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(body)
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

pub fn set_text(text: &str) -> Result<(), String> {
    feed(&["-i"], text.as_bytes())
}

pub fn set_files(paths: &[&Path]) -> Result<(), String> {
    let mut list = String::new();
    for path in paths {
        list.push_str("file://");
        for byte in path.to_string_lossy().bytes() {
            if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
                list.push(byte as char);
            } else {
                list.push_str(&format!("%{byte:02X}"));
            }
        }
        list.push_str("\r\n");
    }
    feed(&["-t", "text/uri-list", "-i"], list.as_bytes())
}

pub fn clear() -> Result<(), String> {
    feed(&["-i"], b"")
}

pub fn restore(snapshot: &Snapshot) -> Result<(), String> {
    match snapshot {
        Snapshot::Text(text) => set_text(text),
        Snapshot::Empty => clear(),
        Snapshot::Other => Ok(()),
    }
}
