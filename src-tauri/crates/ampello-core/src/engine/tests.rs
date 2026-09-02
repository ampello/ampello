// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn engine_with(triggers: &[(&str, &str)]) -> Engine {
    let mut engine = Engine::new();
    engine.set_triggers(
        triggers
            .iter()
            .map(|(id, text)| Trigger {
                snippet_id: (*id).to_string(),
                text: (*text).to_string(),
            })
            .collect(),
    );
    engine
}

fn type_text(engine: &mut Engine, text: &str) -> Vec<Expansion> {
    let mut fired = Vec::new();
    for c in text.chars() {
        if let Some(expansion) = engine.on_key(Key::Char(c)) {
            engine.note_injected_terminator(expansion.terminator);
            fired.push(expansion);
        }
    }
    fired
}

fn ids(fired: &[Expansion]) -> Vec<&str> {
    fired.iter().map(|e| e.snippet_id.as_str()).collect()
}

#[test]
fn a_trigger_followed_by_a_space_expands() {
    let mut engine = engine_with(&[("email", ":email")]);
    let fired = type_text(&mut engine, ":email ");
    assert_eq!(ids(&fired), ["email"]);
    assert_eq!(fired[0].trigger, ":email");
    assert_eq!(fired[0].terminator, ' ');
}

#[test]
fn a_trigger_on_its_own_does_not_expand_until_it_is_terminated() {
    let mut engine = engine_with(&[("email", ":email")]);
    assert!(type_text(&mut engine, ":email").is_empty());

    assert_eq!(ids(&type_text(&mut engine, " ")), ["email"]);
}

#[test]
fn every_kind_of_terminator_fires() {
    for terminator in [' ', '\n', '\t', '.', ',', '!', '?', ')', '-', '/', ';'] {
        let mut engine = engine_with(&[("email", ":email")]);
        let text = format!(":email{terminator}");
        let fired = type_text(&mut engine, &text);
        assert_eq!(
            ids(&fired),
            ["email"],
            "terminator {terminator:?} should fire"
        );
        assert_eq!(fired[0].terminator, terminator);
    }
}

#[test]
fn the_terminator_is_reported_so_it_can_be_preserved() {
    let mut engine = engine_with(&[("email", ":email")]);
    let fired = type_text(&mut engine, ":email.");
    assert_eq!(fired[0].terminator, '.', "the full stop must survive");
}

#[test]
fn a_partial_trigger_never_fires() {
    let mut engine = engine_with(&[("email", ":email")]);
    assert!(type_text(&mut engine, ":emai ").is_empty());
    assert!(type_text(&mut engine, ":emailx ").is_empty());
}

#[test]
fn a_trigger_mid_word_does_not_fire_in_word_mode() {
    let mut engine = engine_with(&[("email", ":email")]);
    assert!(
        type_text(&mut engine, "something:email ").is_empty(),
        "`something:email` must not expand"
    );
}

#[test]
fn a_trigger_mid_word_does_fire_in_anywhere_mode() {
    let mut engine = engine_with(&[("email", ":email")]);
    engine.set_mode(BoundaryMode::Anywhere);
    assert_eq!(ids(&type_text(&mut engine, "something:email ")), ["email"]);
}

#[test]
fn a_trigger_after_a_space_or_punctuation_fires() {
    for prefix in ["hello ", "(", "hello\n", "— ", "…"] {
        let mut engine = engine_with(&[("sig", ":sig")]);
        let text = format!("{prefix}:sig ");
        assert_eq!(
            ids(&type_text(&mut engine, &text)),
            ["sig"],
            "prefix {prefix:?} should allow the trigger"
        );
    }
}

#[test]
fn a_word_trigger_is_still_protected_from_its_own_suffix() {
    let mut engine = engine_with(&[("sig", "sig"), ("mysig", "mysig")]);
    let fired = type_text(&mut engine, "mysig ");
    assert_eq!(
        ids(&fired),
        ["mysig"],
        "the longer, boundary-respecting trigger wins"
    );
}

#[test]
fn the_longest_trigger_wins() {
    let mut engine = engine_with(&[("short", ":sig"), ("long", ":signature")]);
    assert_eq!(ids(&type_text(&mut engine, ":signature ")), ["long"]);

    let mut engine = engine_with(&[("short", ":sig"), ("long", ":signature")]);
    assert_eq!(ids(&type_text(&mut engine, ":sig ")), ["short"]);
}

#[test]
fn overlapping_prefixes_stay_distinct() {
    let mut engine = engine_with(&[("a", ":a"), ("ab", ":ab"), ("abc", ":abc")]);
    assert_eq!(ids(&type_text(&mut engine, ":a ")), ["a"]);
    assert_eq!(ids(&type_text(&mut engine, ":ab ")), ["ab"]);
    assert_eq!(ids(&type_text(&mut engine, ":abc ")), ["abc"]);
}

#[test]
fn a_trigger_containing_a_terminator_still_completes() {
    let mut engine = engine_with(&[("dash", ":a-b")]);
    assert_eq!(ids(&type_text(&mut engine, ":a-b ")), ["dash"]);
}

#[test]
fn backspacing_a_typo_still_leads_to_an_expansion() {
    let mut engine = engine_with(&[("email", ":email")]);
    type_text(&mut engine, ":emaik");
    engine.on_key(Key::Backspace);
    assert_eq!(ids(&type_text(&mut engine, "l ")), ["email"]);
}

#[test]
fn backspacing_into_a_trigger_does_not_fire_it() {
    let mut engine = engine_with(&[("email", ":email")]);
    type_text(&mut engine, ":emailx");
    engine.on_key(Key::Backspace);

    assert!(type_text(&mut engine, "y ").is_empty());
}

#[test]
fn a_reset_forgets_everything_before_it() {
    let mut engine = engine_with(&[("email", ":email")]);
    type_text(&mut engine, ":emai");

    engine.on_key(Key::Reset);
    assert!(
        type_text(&mut engine, "l ").is_empty(),
        "context before the reset must not complete a trigger"
    );
}

#[test]
fn after_an_expansion_the_buffer_starts_clean() {
    let mut engine = engine_with(&[("email", ":email")]);
    let fired = type_text(&mut engine, ":email :email ");
    assert_eq!(
        ids(&fired),
        ["email", "email"],
        "back-to-back expansions both fire"
    );
}

#[test]
fn an_expansion_cannot_be_completed_by_leftover_trigger_text() {
    let mut engine = engine_with(&[("a", ":ab")]);
    type_text(&mut engine, ":ab ");
    assert_eq!(
        engine.buffer_contents(),
        " ",
        "only the re-sent terminator remains"
    );
}

#[test]
fn nothing_fires_while_expansion_is_disabled() {
    let mut engine = engine_with(&[("email", ":email")]);
    engine.set_enabled(false);
    assert!(type_text(&mut engine, ":email ").is_empty());

    engine.set_enabled(true);
    assert_eq!(ids(&type_text(&mut engine, ":email ")), ["email"]);
}

#[test]
fn re_enabling_does_not_fire_on_stale_keystrokes() {
    let mut engine = engine_with(&[("email", ":email")]);
    engine.set_enabled(false);
    type_text(&mut engine, ":emai");
    engine.set_enabled(true);
    assert!(type_text(&mut engine, "l ").is_empty());
}

#[test]
fn changing_the_trigger_set_forgets_the_buffer() {
    let mut engine = engine_with(&[("email", ":email")]);
    type_text(&mut engine, ":emai");
    engine.set_triggers(vec![Trigger {
        snippet_id: "email".into(),
        text: ":email".into(),
    }]);
    assert!(type_text(&mut engine, "l ").is_empty());
}

#[test]
fn a_disabled_snippet_is_simply_absent_from_the_trigger_set() {
    let mut engine = engine_with(&[("on", ":on"), ("off", ":off")]);
    assert_eq!(engine.trigger_count(), 2);
    engine.set_triggers(vec![Trigger {
        snippet_id: "on".into(),
        text: ":on".into(),
    }]);
    assert_eq!(engine.trigger_count(), 1);
    assert!(type_text(&mut engine, ":off ").is_empty());
    assert_eq!(ids(&type_text(&mut engine, ":on ")), ["on"]);
}

#[test]
fn unicode_triggers_work() {
    let mut engine = engine_with(&[("jp", "；；jp"), ("arrow", "→→"), ("rocket", "🚀🚀")]);
    assert_eq!(ids(&type_text(&mut engine, "；；jp ")), ["jp"]);
    assert_eq!(ids(&type_text(&mut engine, "→→ ")), ["arrow"]);
    assert_eq!(ids(&type_text(&mut engine, "🚀🚀 ")), ["rocket"]);
}

#[test]
fn a_unicode_word_character_still_blocks_a_mid_word_trigger() {
    let mut engine = engine_with(&[("sig", "sig")]);
    assert!(
        type_text(&mut engine, "日本sig ").is_empty(),
        "CJK text is word text; the trigger is mid-word"
    );
}

#[test]
fn a_trigger_can_be_terminated_by_a_unicode_terminator() {
    let mut engine = engine_with(&[("jp", ":jp")]);
    assert_eq!(ids(&type_text(&mut engine, ":jp。")), ["jp"]);
}

#[test]
fn thousands_of_triggers_still_match_the_right_one() {
    let mut triggers: Vec<Trigger> = (0..5_000)
        .map(|i| Trigger {
            snippet_id: format!("id{i}"),
            text: format!(":t{i}"),
        })
        .collect();
    triggers.push(Trigger {
        snippet_id: "needle".into(),
        text: ":a-very-long-trigger-indeed".into(),
    });

    let mut engine = Engine::new();
    engine.set_triggers(triggers);

    assert_eq!(ids(&type_text(&mut engine, ":t4999 ")), ["id4999"]);
    assert_eq!(
        ids(&type_text(&mut engine, ":a-very-long-trigger-indeed ")),
        ["needle"]
    );
    assert!(type_text(&mut engine, ":t50000 ").is_empty());
}

#[test]
fn a_long_line_of_typing_before_a_trigger_is_fine() {
    let mut engine = engine_with(&[("email", ":email")]);
    let mut text = "the quick brown fox jumps over the lazy dog ".repeat(20);
    text.push_str(":email ");
    assert_eq!(ids(&type_text(&mut engine, &text)), ["email"]);
}

#[test]
fn an_empty_trigger_set_never_fires() {
    let mut engine = Engine::new();
    assert!(type_text(&mut engine, ":anything at all. ").is_empty());
}
