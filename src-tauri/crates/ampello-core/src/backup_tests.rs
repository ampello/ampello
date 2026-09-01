// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::db::Database;
use crate::models::NewSnippet;

fn db() -> Database {
    Database::open_in_memory().expect("in-memory database")
}

fn store() -> crate::attachments::Store {
    crate::attachments::Store::new(
        std::env::temp_dir().join(format!("ampello-backup-test-{}", std::process::id())),
    )
}

fn add(conn: &Connection, trigger: &str, content: &str, collection: Option<&str>) {
    let category_id = collection.map(|name| {
        find_category(conn, name)
            .unwrap()
            .unwrap_or_else(|| categories::create(conn, name).unwrap().id)
    });
    snippets::create(
        conn,
        NewSnippet {
            trigger: trigger.into(),
            content: content.into(),
            category_id,
        },
    )
    .unwrap();
}

fn round_trip(source: &Database, yaml: bool) -> (Backup, Backup, String) {
    let exported = source.with(|conn| export(conn, 1_700_000_000_000)).unwrap();
    let text = if yaml {
        to_yaml(&exported).unwrap()
    } else {
        to_json(&exported).unwrap()
    };

    let parsed = parse(&text).unwrap();
    let target = db();
    target
        .with(|conn| import(conn, &parsed, ImportMode::Skip, &store()))
        .unwrap();
    let reexported = target.with(|conn| export(conn, 1_700_000_000_000)).unwrap();
    (exported, reexported, text)
}

#[test]
fn export_is_deterministic() {
    let source = db();
    source
        .with(|conn| {
            add(conn, ":zebra", "z", None);
            add(conn, ":alpha", "a", None);
            add(conn, ":Mid", "m", None);
            Ok(())
        })
        .unwrap();

    let first = to_yaml(&source.with(|conn| export(conn, 7)).unwrap()).unwrap();
    let second = to_yaml(&source.with(|conn| export(conn, 7)).unwrap()).unwrap();
    assert_eq!(first, second);

    let order: Vec<String> = source
        .with(|conn| export(conn, 7))
        .unwrap()
        .snippets
        .into_iter()
        .map(|s| s.trigger)
        .collect();
    assert_eq!(order, [":alpha", ":Mid", ":zebra"]);
}

#[test]
fn a_library_survives_a_yaml_round_trip() {
    let source = db();
    source
        .with(|conn| {
            add(conn, ":email", "Hello,\n\nThank you for reaching out.\n", Some("Work"));
            add(
                conn,
                ":javarepo",
                "public class ItemRepository {\n\tprivate final List<Item> items;\n}\n",
                Some("Programming"),
            );
            add(conn, ":sig", "— Yohann", None);

            let sig = snippets::list_summaries(conn)?
                .into_iter()
                .find(|row| row.trigger == ":sig")
                .unwrap();
            snippets::record_usage(conn, &sig.id)?;
            snippets::record_usage(conn, &sig.id)?;
            Ok(())
        })
        .unwrap();

    let (before, after, text) = round_trip(&source, true);
    assert_eq!(before.snippets, after.snippets);
    assert_eq!(before.categories.len(), after.categories.len());
    let sig = after.snippets.iter().find(|s| s.trigger == ":sig").unwrap();
    assert_eq!(sig.usage_count, 2, "usage counts belong in a restore");

    assert!(text.contains("content: |"), "expected block scalars:\n{text}");
    assert!(text.contains("collection: \"Work\""));
}

#[test]
fn a_library_survives_a_json_round_trip() {
    let source = db();
    source
        .with(|conn| {
            add(conn, ":a", "one", None);
            add(conn, ":b", "two\nlines\n", Some("Notes"));
            Ok(())
        })
        .unwrap();

    let (before, after, text) = round_trip(&source, false);
    assert!(text.trim_start().starts_with('{'));
    assert_eq!(before.snippets, after.snippets);
}

#[test]
fn awkward_whitespace_still_comes_back_exactly() {
    let cases = [
        "trailing spaces   \nand more\n",
        "\r\nwindows\r\nline endings\r\n",
        "  starts with indentation\nsecond line\n",
        "ends with two newlines\n\n",
        "no trailing newline at all",
        "\ttab indented\n",
        "",
        "   ",
        "a\n\n\n\nb\n",
    ];

    for (index, content) in cases.iter().enumerate() {
        let source = db();
        let trigger = format!(":case{index}");
        source
            .with(|conn| {
                add(conn, &trigger, content, None);
                Ok(())
            })
            .unwrap();

        let (before, after, text) = round_trip(&source, true);
        assert_eq!(
            before.snippets[0].content, after.snippets[0].content,
            "case {index} ({content:?}) changed on the way through:\n{text}"
        );
        assert_eq!(after.snippets[0].content, *content, "case {index}");
    }
}

#[test]
fn unicode_and_emoji_survive() {
    let content = "日本語 • Ünïcödé • 🚀🇵🇭 • ﷽\nsecond line\n";
    let source = db();
    source
        .with(|conn| {
            add(conn, "；；jp", content, Some("日本"));
            Ok(())
        })
        .unwrap();

    let (_, after, _) = round_trip(&source, true);
    assert_eq!(after.snippets[0].content, content);
    assert_eq!(after.snippets[0].trigger, "；；jp");
    assert_eq!(after.snippets[0].collection.as_deref(), Some("日本"));
}

#[test]
fn a_very_large_snippet_survives() {
    let content = "The quick brown fox — 素早い茶色の狐 — 🦊\n".repeat(20_000);
    assert!(content.len() > 1_000_000);

    let source = db();
    source
        .with(|conn| {
            add(conn, ":big", &content, None);
            Ok(())
        })
        .unwrap();

    let (_, after, _) = round_trip(&source, true);
    assert_eq!(after.snippets[0].content.len(), content.len());
    assert_eq!(after.snippets[0].content, content);
}

#[test]
fn enabled_and_favorite_survive() {
    let source = db();
    source
        .with(|conn| {
            add(conn, ":off", "x", None);
            add(conn, ":star", "y", None);
            let all = snippets::list_summaries(conn)?;
            for row in all {
                let patch = if row.trigger == ":off" {
                    SnippetPatch {
                        enabled: Some(false),
                        ..Default::default()
                    }
                } else {
                    SnippetPatch {
                        favorite: Some(true),
                        ..Default::default()
                    }
                };
                snippets::update(conn, &row.id, patch)?;
            }
            Ok(())
        })
        .unwrap();

    let (_, after, _) = round_trip(&source, true);
    let off = after.snippets.iter().find(|s| s.trigger == ":off").unwrap();
    let star = after.snippets.iter().find(|s| s.trigger == ":star").unwrap();
    assert!(!off.enabled, "a disabled snippet must import disabled");
    assert!(star.favorite, "a favourite must import as a favourite");
}

#[test]
fn skip_mode_leaves_existing_snippets_alone() {
    let target = db();
    target
        .with(|conn| {
            add(conn, ":email", "MINE", None);
            Ok(())
        })
        .unwrap();

    let backup = Backup {
        version: 1,
        exported_at: 0,
        categories: vec![],
        snippets: vec![BackupSnippet {
            trigger: ":email".into(),
            collection: None,
            enabled: true,
            favorite: false,
            usage_count: 0,
            content: "THEIRS".into(),
            attachments: vec![],
            attachments_first: true,
            strict_order: false,
        }],
    };

    let report = target
        .with(|conn| import(conn, &backup, ImportMode::Skip, &store()))
        .unwrap();
    assert_eq!(report.skipped, 1);
    assert_eq!(report.added, 0);

    let kept = target
        .with(|conn| Ok(export(conn, 0)?.snippets[0].content.clone()))
        .unwrap();
    assert_eq!(kept, "MINE");
}

#[test]
fn replace_mode_overwrites_existing_snippets() {
    let target = db();
    target
        .with(|conn| {
            add(conn, ":email", "MINE", None);
            Ok(())
        })
        .unwrap();

    let backup = Backup {
        version: 1,
        exported_at: 0,
        categories: vec![],
        snippets: vec![BackupSnippet {
            trigger: ":email".into(),
            collection: Some("Imported".into()),
            enabled: false,
            favorite: true,
            usage_count: 9,
            content: "THEIRS".into(),
            attachments: vec![],
            attachments_first: true,
            strict_order: false,
        }],
    };

    let report = target
        .with(|conn| import(conn, &backup, ImportMode::Replace, &store()))
        .unwrap();
    assert_eq!(report.replaced, 1);
    assert_eq!(report.collections_created, 1);

    let after = target.with(|conn| export(conn, 0)).unwrap();
    assert_eq!(after.snippets[0].content, "THEIRS");
    assert_eq!(after.snippets[0].collection.as_deref(), Some("Imported"));
    assert!(!after.snippets[0].enabled);
    assert!(after.snippets[0].favorite);
}

#[test]
fn a_hand_written_file_is_accepted() {
    let text = r#"
version: 1
snippets:
  - trigger: ":hello"
    content: |
      Hello, how are you?
  - trigger: ":addr"
    title: Address
    collection: Personal
    content: |
      1 Trigger Lane
      Ampello City
"#;

    let backup = parse(text).unwrap();
    assert_eq!(backup.snippets.len(), 2);

    assert!(backup.snippets[0].enabled);
    assert!(!backup.snippets[0].favorite);

    let target = db();
    let report = target
        .with(|conn| import(conn, &backup, ImportMode::Skip, &store()))
        .unwrap();
    assert_eq!(report.added, 2);
    assert_eq!(report.collections_created, 1);
    assert!(report.problems.is_empty());

    let after = target.with(|conn| export(conn, 0)).unwrap();
    let hello = after.snippets.iter().find(|s| s.trigger == ":hello").unwrap();
    assert_eq!(hello.content, "Hello, how are you?\n");
}

#[test]
fn one_bad_row_does_not_abandon_the_rest() {
    let backup = Backup {
        version: 1,
        exported_at: 0,
        categories: vec![],
        snippets: vec![
            BackupSnippet {
                trigger: "   ".into(),
                collection: None,
                enabled: true,
                favorite: false,
                usage_count: 0,
                content: "x".into(),
                attachments: vec![],
                attachments_first: true,
                strict_order: false,
            },
            BackupSnippet {
                trigger: ":good".into(),
                collection: None,
                enabled: true,
                favorite: false,
                usage_count: 0,
                content: "y".into(),
                attachments: vec![],
                attachments_first: true,
                strict_order: false,
            },
        ],
    };

    let target = db();
    let report = target
        .with(|conn| import(conn, &backup, ImportMode::Skip, &store()))
        .unwrap();
    assert_eq!(report.added, 1);
    assert_eq!(report.problems.len(), 1);
    assert!(report.problems[0].contains("empty"), "{:?}", report.problems);
}

#[test]
fn nonsense_input_is_refused_clearly() {
    assert!(parse("").is_err());
    assert!(parse("   \n  ").is_err());
    assert!(parse("this is just prose, not a backup").is_err());
    assert!(parse("{ \"version\": 1, ").is_err());

    let from_the_future = parse("version: 99\nsnippets: []\n");
    assert!(from_the_future.is_err(), "a newer format must be refused");
}

#[test]
fn an_empty_library_exports_and_imports_cleanly() {
    let source = db();
    let (before, after, text) = round_trip(&source, true);
    assert!(before.snippets.is_empty());
    assert!(after.snippets.is_empty());
    assert!(text.contains("snippets: []"));
}

#[test]
fn block_style_is_only_chosen_when_it_is_safe() {
    assert_eq!(block_style("one line\n"), Some(("|", "one line")));
    assert_eq!(block_style("no newline"), Some(("|-", "no newline")));
    assert_eq!(block_style(""), None);
    assert_eq!(block_style("trailing \n"), None);
    assert_eq!(block_style("has\r\ncrlf\n"), None);
    assert_eq!(block_style(" leading space\n"), None);
    assert_eq!(block_style("two trailing\n\n"), None);
}

struct TempStore {
    store: crate::attachments::Store,
    root: std::path::PathBuf,
}

impl TempStore {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ampello-archive-test-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        Self {
            store: crate::attachments::Store::new(&root),
            root,
        }
    }

    fn file(&self, name: &str) -> std::path::PathBuf {
        self.root.join(name)
    }
}

impl Drop for TempStore {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn attach(
    conn: &Connection,
    store: &crate::attachments::Store,
    snippet_id: &str,
    name: &str,
    bytes: &[u8],
) {
    let stored = store.add_bytes(name, bytes).unwrap();
    let mime = crate::attachments::mime_for(&stored.name);
    attachments::add(conn, snippet_id, &stored, mime).unwrap();
}

#[test]
fn an_archive_carries_every_kind_of_file_back_intact() {
    let source = TempStore::new("roundtrip-source");
    let destination = TempStore::new("roundtrip-destination");
    let from = db();

    let files: [(&str, &[u8]); 4] = [
        ("contract.pdf", b"%PDF-1.7 pretend contract"),
        ("notes.docx", b"PK\x03\x04 pretend docx"),
        ("screenshot.png", b"\x89PNG\r\n\x1a\n pretend png"),
        ("bundle.zip", b"PK\x03\x04 pretend zip"),
    ];

    from.with(|conn| {
        let snippet = snippets::create(
            conn,
            NewSnippet {
                trigger: ":report".into(),
                content: "See attached.".into(),
                category_id: None,
            },
        )?;
        for (name, bytes) in files {
            attach(conn, &source.store, &snippet.id, name, bytes);
        }
        Ok(())
    })
    .unwrap();

    let exported = from.with(|conn| export(conn, 1_700_000_000_000)).unwrap();
    let archive = source.file("backup.ampellozip");
    let missing = write_archive(&archive, &exported, &source.store).unwrap();
    assert!(missing.is_empty(), "{missing:?}");

    let bytes = std::fs::read(&archive).unwrap();
    assert!(is_archive(&bytes));

    let (parsed, problems) = read_archive(&bytes, &destination.store).unwrap();
    assert!(problems.is_empty(), "{problems:?}");

    let to = db();
    let report = to
        .with(|conn| import(conn, &parsed, ImportMode::Skip, &destination.store))
        .unwrap();
    assert_eq!(report.added, 1);
    assert!(report.problems.is_empty(), "{:?}", report.problems);

    let restored = to
        .with(|conn| {
            let id = find_snippet(conn, ":report")?.expect("the snippet came back");
            snippets::get(conn, &id)
        })
        .unwrap();
    let names: Vec<&str> = restored
        .attachments
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    assert_eq!(names, files.iter().map(|(n, _)| *n).collect::<Vec<_>>());

    for (attachment, (_, expected)) in restored.attachments.iter().zip(files) {
        let bytes = destination
            .store
            .read(&attachment.digest, &attachment.name)
            .unwrap();
        assert_eq!(bytes, expected);
    }
}

#[test]
fn the_delivery_order_survives_the_round_trip() {
    let source = TempStore::new("order-source");
    let destination = TempStore::new("order-destination");
    let from = db();

    from.with(|conn| {
        let snippet = snippets::create(
            conn,
            NewSnippet {
                trigger: ":deck".into(),
                content: "Slides attached.".into(),
                category_id: None,
            },
        )?;
        for name in ["third.pdf", "first.pdf", "second.pdf"] {
            attach(conn, &source.store, &snippet.id, name, name.as_bytes());
        }

        snippets::update(
            conn,
            &snippet.id,
            SnippetPatch {
                attachments_first: Some(false),
                strict_order: Some(true),
                ..Default::default()
            },
        )?;
        Ok(())
    })
    .unwrap();

    let exported = from.with(|conn| export(conn, 0)).unwrap();
    let archive = source.file("order.ampellozip");
    write_archive(&archive, &exported, &source.store).unwrap();

    let (parsed, _) = read_archive(&std::fs::read(&archive).unwrap(), &destination.store).unwrap();
    let to = db();
    to.with(|conn| import(conn, &parsed, ImportMode::Skip, &destination.store))
        .unwrap();

    let restored = to
        .with(|conn| {
            let id = find_snippet(conn, ":deck")?.expect("the snippet came back");
            snippets::get(conn, &id)
        })
        .unwrap();

    assert_eq!(
        restored
            .attachments
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["third.pdf", "first.pdf", "second.pdf"]
    );
    assert!(!restored.attachments_first);
    assert!(restored.strict_order);
}

#[test]
fn one_file_on_two_snippets_is_stored_once_in_the_archive() {
    let source = TempStore::new("dedup-source");
    let from = db();

    from.with(|conn| {
        for trigger in [":one", ":two"] {
            let snippet = snippets::create(
                conn,
                NewSnippet {
                    trigger: trigger.into(),
                    content: String::new(),
                    category_id: None,
                },
            )?;
            attach(conn, &source.store, &snippet.id, "shared.pdf", b"the same bytes");
        }
        Ok(())
    })
    .unwrap();

    let exported = from.with(|conn| export(conn, 0)).unwrap();
    let archive = source.file("dedup.ampellozip");
    write_archive(&archive, &exported, &source.store).unwrap();

    let file = std::fs::File::open(&archive).unwrap();
    let zip = zip::ZipArchive::new(file).unwrap();

    assert_eq!(zip.len(), 2);
}

#[test]
fn a_snippet_with_no_files_still_exports_as_a_plain_document() {
    let store = TempStore::new("plain");
    let from = db();
    from.with(|conn| {
        add(conn, ":email", "Hello,", None);
        Ok(())
    })
    .unwrap();

    let exported = from.with(|conn| export(conn, 0)).unwrap();
    assert!(exported.snippets.iter().all(|s| s.attachments.is_empty()));

    let text = to_yaml(&exported).unwrap();
    assert!(!text.contains("attachments:"), "{text}");
    let back = parse(&text).unwrap();
    assert_eq!(back, exported);

    drop(store);
}

#[test]
fn an_archive_whose_contents_do_not_match_is_refused_not_written() {
    use std::io::Write;

    let destination = TempStore::new("tampered");
    let source = TempStore::new("tampered-source");
    let from = db();

    from.with(|conn| {
        let snippet = snippets::create(
            conn,
            NewSnippet {
                trigger: ":doc".into(),
                content: String::new(),
                category_id: None,
            },
        )?;
        attach(conn, &source.store, &snippet.id, "real.pdf", b"the real contents");
        Ok(())
    })
    .unwrap();

    let exported = from.with(|conn| export(conn, 0)).unwrap();
    let declared = exported.snippets[0].attachments[0].clone();

    let path = source.file("tampered.ampellozip");
    {
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file(ARCHIVE_ENTRY, options).unwrap();
        zip.write_all(to_yaml(&exported).unwrap().as_bytes()).unwrap();
        zip.start_file(
            format!("attachments/{}/{}", declared.digest, declared.name),
            options,
        )
        .unwrap();
        zip.write_all(b"something else entirely").unwrap();
        zip.finish().unwrap();
    }

    let (_, problems) =
        read_archive(&std::fs::read(&path).unwrap(), &destination.store).unwrap();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("do not match"), "{problems:?}");
    assert!(!destination.store.exists(&declared.digest, &declared.name));
}

#[test]
fn a_restore_without_the_files_says_so_rather_than_dropping_them() {
    let source = TempStore::new("document-only-source");
    let destination = TempStore::new("document-only");
    let from = db();

    from.with(|conn| {
        let snippet = snippets::create(
            conn,
            NewSnippet {
                trigger: ":report".into(),
                content: "See attached.".into(),
                category_id: None,
            },
        )?;
        attach(conn, &source.store, &snippet.id, "missing.pdf", b"gone");
        Ok(())
    })
    .unwrap();

    let exported = from.with(|conn| export(conn, 0)).unwrap();
    let parsed = parse(&to_yaml(&exported).unwrap()).unwrap();

    let to = db();
    let report = to
        .with(|conn| import(conn, &parsed, ImportMode::Skip, &destination.store))
        .unwrap();

    assert_eq!(report.added, 1);
    assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
    assert!(report.problems[0].contains("missing.pdf"), "{:?}", report.problems);

    let restored = to
        .with(|conn| {
            let id = find_snippet(conn, ":report")?.expect("the snippet came back");
            snippets::get(conn, &id)
        })
        .unwrap();
    assert_eq!(restored.attachments.len(), 1);
    assert_eq!(restored.attachments[0].name, "missing.pdf");
}
