// SPDX-License-Identifier: GPL-3.0-or-later
use crate::error::{Error, Result};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

#[cfg(test)]
#[path = "attachments_tests.rs"]
mod tests;

pub const MAX_ATTACHMENT_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_SNIPPET_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_PER_SNIPPET: usize = 16;

const MAX_NAME_LEN: usize = 96;

#[derive(Debug, Clone)]
pub struct Stored {
    pub digest: String,

    pub name: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path_of(&self, digest: &str, name: &str) -> PathBuf {
        self.root
            .join(&digest[..2.min(digest.len())])
            .join(digest)
            .join(name)
    }

    pub fn exists(&self, digest: &str, name: &str) -> bool {
        self.path_of(digest, name).is_file()
    }

    pub fn add_file(&self, source: &Path) -> Result<Stored> {
        let metadata = std::fs::metadata(source)?;
        if !metadata.is_file() {
            return Err(Error::invalid(format!(
                "{} is not a file.",
                source.display()
            )));
        }
        check_size(metadata.len())?;

        let name = source
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let bytes = std::fs::read(source)?;
        self.add_bytes(&name, &bytes)
    }

    pub fn add_bytes(&self, name: &str, bytes: &[u8]) -> Result<Stored> {
        check_size(bytes.len() as u64)?;

        let name = sanitize_name(name);
        let digest = digest_of(bytes);
        let destination = self.path_of(&digest, &name);

        if destination.is_file() {
            return Ok(Stored {
                digest,
                name,
                size_bytes: bytes.len() as i64,
            });
        }

        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let temporary =
            destination.with_file_name(format!(".{}.{}.part", name, std::process::id()));
        std::fs::write(&temporary, bytes)?;
        if let Err(error) = std::fs::rename(&temporary, &destination) {
            let _ = std::fs::remove_file(&temporary);

            if !destination.is_file() {
                return Err(error.into());
            }
        }

        Ok(Stored {
            digest,
            name,
            size_bytes: bytes.len() as i64,
        })
    }

    pub fn read(&self, digest: &str, name: &str) -> Result<Vec<u8>> {
        let path = self.path_of(digest, name);
        if !path.is_file() {
            return Err(Error::not_found(format!(
                "The attachment file for {name} is missing from the store."
            )));
        }
        Ok(std::fs::read(path)?)
    }

    pub fn gc(&self, live: &HashSet<(String, String)>) -> Result<usize> {
        if !self.root.is_dir() {
            return Ok(0);
        }
        let mut removed = 0usize;

        for shard in read_dir(&self.root)? {
            let Ok(blobs) = read_dir(&shard) else {
                continue;
            };
            for blob in blobs {
                let Some(digest) = blob.file_name().map(|n| n.to_string_lossy().into_owned())
                else {
                    continue;
                };
                let Ok(files) = read_dir(&blob) else { continue };
                let mut orphaned = true;
                for file in files {
                    let Some(name) = file.file_name().map(|n| n.to_string_lossy().into_owned())
                    else {
                        continue;
                    };
                    if live.contains(&(digest.clone(), name)) {
                        orphaned = false;
                        continue;
                    }
                    match std::fs::remove_file(&file) {
                        Ok(()) => removed += 1,
                        Err(error) => {
                            log::warn!("could not remove {}: {error}", file.display());
                            orphaned = false;
                        }
                    }
                }
                if orphaned {
                    let _ = std::fs::remove_dir(&blob);
                }
            }
            let _ = std::fs::remove_dir(&shard);
        }
        Ok(removed)
    }

    pub fn size_bytes(&self) -> u64 {
        let mut total = 0u64;
        let Ok(shards) = read_dir(&self.root) else {
            return 0;
        };
        for shard in shards {
            let Ok(blobs) = read_dir(&shard) else {
                continue;
            };
            for blob in blobs {
                let Ok(files) = read_dir(&blob) else { continue };
                for file in files {
                    if let Ok(metadata) = std::fs::metadata(&file) {
                        total = total.saturating_add(metadata.len());
                    }
                }
            }
        }
        total
    }
}

fn read_dir(path: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(path)? {
        out.push(entry?.path());
    }
    Ok(out)
}

fn check_size(len: u64) -> Result<()> {
    if len == 0 {
        return Err(Error::invalid("That file is empty."));
    }
    if len > MAX_ATTACHMENT_BYTES {
        return Err(Error::invalid(format!(
            "That file is {:.1} MB. Ampello will not attach anything larger than {} MB.",
            len as f64 / (1024.0 * 1024.0),
            MAX_ATTACHMENT_BYTES / (1024 * 1024)
        )));
    }
    Ok(())
}

// Reduces a name from anywhere - a picker, a hand-edited backup, another
// operating system - to one component safe to create inside the store. The
// name is used as a path, so `..\\..\\Startup\\evil.exe` must not escape it.
pub fn sanitize_name(raw: &str) -> String {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or(raw).trim();

    let mut cleaned: String = base
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '|' | '?' | '*' | '/' | '\\' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        })
        .collect();

    while cleaned.ends_with('.') || cleaned.ends_with(' ') {
        cleaned.pop();
    }

    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        cleaned = "attachment".to_string();
    }

    if is_reserved_device(&cleaned) {
        cleaned.insert(0, '_');
    }

    truncate_keeping_extension(&cleaned)
}

fn is_reserved_device(name: &str) -> bool {
    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let stem = name.split('.').next().unwrap_or(name);
    RESERVED
        .iter()
        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
}

fn truncate_keeping_extension(name: &str) -> String {
    if name.chars().count() <= MAX_NAME_LEN {
        return name.to_string();
    }
    let (stem, extension) = match name.rfind('.') {
        Some(index) if index > 0 && name.len() - index <= 12 => (&name[..index], &name[index..]),
        _ => (name, ""),
    };
    let room = MAX_NAME_LEN.saturating_sub(extension.chars().count());
    let short: String = stem.chars().take(room).collect();
    format!("{short}{extension}")
}

pub fn mime_for(name: &str) -> &'static str {
    let extension = name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "tif" | "tiff" => "image/tiff",
        "heic" => "image/heic",
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "txt" | "log" => "text/plain",
        "md" => "text/markdown",
        "csv" => "text/csv",
        "json" => "application/json",
        "xml" => "application/xml",
        "html" | "htm" => "text/html",
        "zip" => "application/zip",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        _ => "application/octet-stream",
    }
}

pub fn has_bitmap_route(mime: &str) -> bool {
    matches!(
        mime,
        "image/png" | "image/jpeg" | "image/gif" | "image/webp" | "image/bmp" | "image/tiff"
    )
}

pub const MAX_PREVIEW_BYTES: i64 = 8 * 1024 * 1024;

pub fn is_previewable(mime: &str) -> bool {
    matches!(
        mime,
        "image/png" | "image/jpeg" | "image/gif" | "image/webp" | "image/bmp" | "image/svg+xml"
    )
}

pub fn digest_of(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}
