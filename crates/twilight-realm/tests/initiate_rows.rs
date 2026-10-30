//! Initiate's Talent Rows (#506, ADR 0028): five Skill Choice rows and the Talent row `passives`,
//! opening at 4, 8, 12, 16, 20 and 24. Blink has no row. Played through the realm's seam by a
//! shipped Initiate.

mod common;

use common::initiate::*;
use common::vanguard::{enter, learned, login_rows, removed, rows_in, set_row};
use common::*;

const SEED: u64 = 0x5EED_0506;

/// Each row in level order: its key, its level, its plain skill's name ("" for a Talent row) and
/// its three choices.
const ROWS: [(&str, u8, &str, [&str; 3]); 6] = [
    (
        "ley_spark",
        4,
        "Ley-Spark",
        ["ley_line_tap", "arcane_substrate", "warding_spark"],
    ),
    (
        "ley_ward",
        8,
        "Ley Ward",
        ["siphoning_ward", "frost_ward", "volatile_ward"],
    ),
    (
        "searing_ember",
        12,
        "Searing Ember",
        ["thermal_feedback", "smoldering_ember", "kindling_ember"],
    ),
    (
        "biting_cold",
        16,
        "Biting Cold",
        ["combustion_engine", "splintering_cold", "frostbite"],
    ),
    (
        "passives",
        20,
        "",
        ["shattered_blink", "arcane_momentum", "catalytic_resonance"],
    ),
    (
        "catalyst",
        24,
        "Catalyst",
        ["thermal_shock", "deep_freeze", "catalytic_chain"],
    ),
];

#[test]
fn at_level_24_the_login_lists_six_rows_open_and_empty_with_their_choices() {
    let want: Vec<(String, u8, bool, u32, String)> = ROWS
        .iter()
        .map(|(key, level, skill, _)| (key.to_string(), *level, true, 0, skill.to_string()))
        .collect();
    assert_eq!(login_rows(initiate_save(ROWS_OPEN), SEED), want);

    let (guid, blob) = initiate_save(ROWS_OPEN);
    let (_, _, entry) = enter(guid, &blob, SEED);
    for (row, (key, _, _, choices)) in rows_in(&entry).unwrap().iter().zip(ROWS) {
        let sent: Vec<u32> = row.choices.iter().map(|c| c.spell_id).collect();
        let sheet: Vec<u32> = choices.iter().map(|c| choice("initiate", key, c)).collect();
        assert_eq!(sent, sheet, "{key}");
    }
}

#[test]
fn at_level_3_every_row_is_locked() {
    let rows = login_rows(initiate_save(3), SEED);
    assert_eq!(rows.len(), 6);
    assert!(rows.iter().all(|r| !r.2 && r.3 == 0), "{rows:?}");
}

#[test]
fn each_skill_choice_set_on_its_empty_row_replaces_the_plain_skill() {
    for (key, _, _, choices) in ROWS.iter().filter(|r| r.0 != "passives") {
        let plain = spell("initiate", key);
        for name in choices {
            let (guid, blob) = initiate_save(ROWS_OPEN);
            let (mut realm, mut view, _) = enter(guid, &blob, SEED);
            let id = choice("initiate", key, name);
            let events = set_row(&mut realm, &mut view, key, id);
            assert!(learned(&events, id), "{name} is learned: {events:?}");
            assert!(removed(&events, plain), "{name} removes {key}: {events:?}");
        }
    }
}

#[test]
fn each_talent_set_on_the_passives_row_is_learned_and_removes_nothing() {
    let (_, _, _, choices) = ROWS[4];
    for name in choices {
        let (guid, blob) = initiate_save(ROWS_OPEN);
        let (mut realm, mut view, _) = enter(guid, &blob, SEED);
        let id = choice("initiate", "passives", name);
        let events = set_row(&mut realm, &mut view, "passives", id);
        assert!(learned(&events, id), "{name} is learned: {events:?}");
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, benilla_protocol::SessionEvent::SpellRemoved { .. })),
            "{name} removes nothing: {events:?}"
        );
    }
}
