// SPDX-License-Identifier: GPL-3.0-or-later
use std::time::Instant;

use crate::db::{snippets, Database};
use crate::engine::{Engine, Key, Trigger};
use crate::models::NewSnippet;

fn triggers(count: usize) -> Vec<Trigger> {
    (0..count)
        .map(|i| Trigger {
            snippet_id: format!("id{i}"),

            text: match i % 4 {
                0 => format!(":t{i}"),
                1 => format!(";;{i}"),
                2 => format!("::abbrev{i}"),
                _ => format!(":x{i}y"),
            },
        })
        .collect()
}

#[test]
fn matching_keeps_up_with_typing() {
    let mut engine = Engine::new();
    engine.set_triggers(triggers(10_000));

    let text: Vec<char> = "the quick brown fox jumps over the lazy dog. "
        .repeat(1_200)
        .chars()
        .collect();
    assert!(text.len() > 50_000);

    let start = Instant::now();
    let mut fired = 0usize;
    for &c in &text {
        if engine.on_key(Key::Char(c)).is_some() {
            fired += 1;
        }
    }
    let elapsed = start.elapsed();
    let per_key = elapsed.as_nanos() / text.len() as u128;

    println!(
        "matcher: {} keystrokes against 10,000 triggers in {:?} ({} ns/keystroke)",
        text.len(),
        elapsed,
        per_key
    );
    assert_eq!(fired, 0);

    assert!(
        per_key < 1_000_000,
        "matching took {per_key} ns per keystroke"
    );
}

#[test]
fn rebuilding_the_trigger_set_is_cheap() {
    let mut engine = Engine::new();
    let set = triggers(10_000);

    let start = Instant::now();
    for _ in 0..10 {
        engine.set_triggers(set.clone());
    }
    let elapsed = start.elapsed() / 10;
    println!("trigger set rebuild (10,000 triggers): {elapsed:?}");
    assert!(elapsed.as_millis() < 500, "rebuild took {elapsed:?}");
}

fn library(count: usize) -> Database {
    let db = Database::open_in_memory().unwrap();
    db.with_mut(|conn| {
        let tx = conn.transaction()?;
        for i in 0..count {
            snippets::create(
                &tx,
                NewSnippet {
                    trigger: format!(":snip{i}"),

                    content: format!("body {i}\n").repeat(40),
                    category_id: None,
                },
            )?;
        }
        tx.commit()?;
        Ok(())
    })
    .unwrap();
    db
}

#[test]
fn a_large_library_lists_quickly() {
    let db = library(10_000);

    let start = Instant::now();
    let rows = db.with(snippets::list_summaries).unwrap();
    let elapsed = start.elapsed();
    println!("list_summaries over {} snippets: {elapsed:?}", rows.len());

    assert_eq!(rows.len(), 10_000);

    assert!(rows.iter().all(|row| row.preview.chars().count() <= 160));
    assert!(elapsed.as_secs() < 3, "listing took {elapsed:?}");
}

#[test]
fn a_large_library_searches_quickly() {
    let db = library(10_000);

    let start = Instant::now();
    let hits = db.with(|conn| snippets::search(conn, "body 9999")).unwrap();
    let elapsed = start.elapsed();
    println!("full-content search over 10,000 snippets: {elapsed:?}");

    assert_eq!(hits.len(), 1);
    assert!(elapsed.as_secs() < 3, "search took {elapsed:?}");
}

#[test]
fn loading_triggers_for_the_engine_is_quick() {
    let db = library(10_000);

    let start = Instant::now();
    let loaded = db.with(snippets::enabled_triggers).unwrap();
    let elapsed = start.elapsed();
    println!("enabled_triggers over 10,000 snippets: {elapsed:?}");

    assert_eq!(loaded.len(), 10_000);
    assert!(elapsed.as_millis() < 2_000, "took {elapsed:?}");
}

#[test]
fn fetching_one_body_does_not_depend_on_library_size() {
    let db = library(10_000);
    let id = db
        .with(|conn| Ok(snippets::list_summaries(conn)?[0].id.clone()))
        .unwrap();

    let start = Instant::now();
    for _ in 0..100 {
        let _ = db.with(|conn| snippets::content_of(conn, &id)).unwrap();
    }
    let elapsed = start.elapsed() / 100;
    println!("content_of on the expansion path: {elapsed:?}");

    assert!(elapsed.as_millis() < 50, "content_of took {elapsed:?}");
}
