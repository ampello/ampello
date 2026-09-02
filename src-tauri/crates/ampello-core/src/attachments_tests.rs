// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::collections::HashSet;

struct Temp {
    store: Store,
    root: PathBuf,
}

impl Temp {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ampello-attachment-test-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        Self {
            store: Store::new(&root),
            root,
        }
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn the_same_bytes_are_stored_once_however_often_they_are_added() {
    let temp = Temp::new("dedup");
    let first = temp
        .store
        .add_bytes("invoice.pdf", b"%PDF-1.7 pretend")
        .unwrap();
    let second = temp
        .store
        .add_bytes("invoice.pdf", b"%PDF-1.7 pretend")
        .unwrap();

    assert_eq!(first.digest, second.digest);
    assert_eq!(temp.store.size_bytes(), b"%PDF-1.7 pretend".len() as u64);
}

#[test]
fn different_bytes_under_the_same_name_stay_separate() {
    let temp = Temp::new("distinct");
    let first = temp.store.add_bytes("notes.txt", b"one").unwrap();
    let second = temp.store.add_bytes("notes.txt", b"two").unwrap();

    assert_ne!(first.digest, second.digest);
    assert!(temp.store.exists(&first.digest, "notes.txt"));
    assert!(temp.store.exists(&second.digest, "notes.txt"));
}

#[test]
fn the_stored_path_ends_in_the_real_filename() {
    let temp = Temp::new("name");
    let stored = temp
        .store
        .add_bytes("Q3 Report.docx", b"docx bytes")
        .unwrap();
    let path = temp.store.path_of(&stored.digest, &stored.name);

    assert_eq!(
        path.file_name().unwrap().to_string_lossy(),
        "Q3 Report.docx"
    );
    assert!(path.is_file());
}

#[test]
fn any_kind_of_file_is_the_same_kind_of_thing() {
    let temp = Temp::new("agnostic");
    for (name, bytes) in [
        ("shot.png", &b"\x89PNG\r\n\x1a\n"[..]),
        ("paper.pdf", &b"%PDF-1.7"[..]),
        ("memo.docx", &b"PK\x03\x04"[..]),
        ("archive.zip", &b"PK\x03\x04zip"[..]),
        ("data.csv", &b"a,b,c"[..]),
    ] {
        let stored = temp.store.add_bytes(name, bytes).unwrap();
        assert_eq!(stored.name, name);
        assert_eq!(
            temp.store.read(&stored.digest, &stored.name).unwrap(),
            bytes
        );
    }
}

#[test]
fn a_name_cannot_escape_the_store() {
    assert_eq!(sanitize_name(r"..\..\Startup\evil.exe"), "evil.exe");
    assert_eq!(sanitize_name("../../../etc/passwd"), "passwd");
    assert_eq!(sanitize_name(".."), "attachment");
    assert_eq!(sanitize_name("/"), "attachment");
    assert_eq!(sanitize_name(""), "attachment");

    let temp = Temp::new("escape");
    let stored = temp.store.add_bytes(r"..\..\evil.exe", b"payload").unwrap();
    let path = temp.store.path_of(&stored.digest, &stored.name);
    assert!(path.starts_with(temp.store.root()));
    assert!(path.is_file());
}

#[test]
fn windows_device_names_are_pushed_out_of_the_way() {
    assert_eq!(sanitize_name("CON"), "_CON");
    assert_eq!(sanitize_name("con.txt"), "_con.txt");
    assert_eq!(sanitize_name("LPT1.pdf"), "_LPT1.pdf");
    assert_eq!(sanitize_name("console.txt"), "console.txt");
}

#[test]
fn characters_windows_refuses_are_replaced_not_dropped() {
    assert_eq!(sanitize_name("a:b*c?.txt"), "a_b_c_.txt");

    assert_eq!(sanitize_name("report. "), "report");
}

#[test]
fn an_overlong_name_keeps_its_extension() {
    let long = format!("{}.pdf", "a".repeat(300));
    let cleaned = sanitize_name(&long);
    assert!(cleaned.chars().count() <= 96);
    assert!(cleaned.ends_with(".pdf"));
}

#[test]
fn an_empty_or_oversized_file_is_refused() {
    let temp = Temp::new("limits");
    assert!(temp.store.add_bytes("nothing.txt", b"").is_err());

    let huge = vec![0u8; (MAX_ATTACHMENT_BYTES + 1) as usize];
    let error = temp.store.add_bytes("huge.bin", &huge).unwrap_err();
    assert!(error.to_string().contains("32 MB"), "{error}");
}

#[test]
fn collecting_garbage_removes_orphans_and_spares_live_files() {
    let temp = Temp::new("gc");
    let kept = temp.store.add_bytes("kept.pdf", b"keep me").unwrap();
    let orphan = temp.store.add_bytes("orphan.pdf", b"drop me").unwrap();

    let mut live = HashSet::new();
    live.insert((kept.digest.clone(), kept.name.clone()));

    assert_eq!(temp.store.gc(&live).unwrap(), 1);
    assert!(temp.store.exists(&kept.digest, &kept.name));
    assert!(!temp.store.exists(&orphan.digest, &orphan.name));
}

#[test]
fn a_missing_file_is_a_clear_error_rather_than_a_panic() {
    let temp = Temp::new("missing");
    let error = temp.store.read(&"0".repeat(64), "gone.pdf").unwrap_err();
    assert!(error.to_string().contains("missing"), "{error}");
}
