//! Cutthroat's Talent Rows (#505, ADR 0028): five Skill Choice rows and the Talent row
//! `passives`, opening at 4, 8, 12, 16, 20 and 24. Played through the realm's seam by a shipped
//! Cutthroat.

mod common;

use common::cutthroat::*;
use common::vanguard::{enter, learned, removed, rows_in, set_row};
use common::*;

const SEED: u64 = 0x5EED_0505;

/// Each row in level order: its key, its level, its plain skill's name ("" for a Talent row) and
/// its three choices.
const ROWS: [(&str, u8, &str, [&str; 3]); 6] = [
    (
        "sinister_strike",
        4,
        "Sinister Strike",
        ["serrated_strikes", "flurry_of_blades", "practiced_strike"],
    ),
    (
        "evasion",
        8,
        "Evasion",
        ["phantom_guile", "elusiveness", "second_wind"],
    ),
    (
        "eviscerate",
        12,
        "Eviscerate",
        ["open_wounds", "executioners_toll", "kidney_shot"],
    ),
    (
        "rupture",
        16,
        "Rupture",
        ["bloodletting", "open_veins", "sanguine_rupture"],
    ),
    ("passives", 20, "", ["blood_frenzy", "thirst", "blood_rush"]),
    (
        "garrote",
        24,
        "Garrote",
        ["brazen_garrote", "choking_garrote", "initiative"],
    ),
];

/// The rows the login of a Cutthroat of `level` sends, as `(key, level, open, set, skill)`.
fn login_rows(level: u8) -> Vec<(String, u8, bool, u32, String)> {
    let (guid, blob) = cutthroat_save(level, false);
    let (_, _, entry) = enter(guid, &blob, SEED);
    rows_in(&entry)
        .expect("the login sends the rows")
        .iter()
        .map(|r| (r.key.clone(), r.level, r.open, r.set, r.skill.clone()))
        .collect()
}

#[test]
fn at_level_24_the_login_lists_six_rows_open_and_empty_with_their_choices() {
    let want: Vec<(String, u8, bool, u32, String)> = ROWS
        .iter()
        .map(|(key, level, skill, _)| (key.to_string(), *level, true, 0, skill.to_string()))
        .collect();
    assert_eq!(login_rows(ROWS_OPEN), want);

    let (guid, blob) = cutthroat_save(ROWS_OPEN, false);
    let (_, _, entry) = enter(guid, &blob, SEED);
    for (row, (key, _, _, choices)) in rows_in(&entry).unwrap().iter().zip(ROWS) {
        let sent: Vec<u32> = row.choices.iter().map(|c| c.spell_id).collect();
        let sheet: Vec<u32> = choices
            .iter()
            .map(|c| choice("cutthroat", key, c))
            .collect();
        assert_eq!(sent, sheet, "{key}");
    }
}

#[test]
fn at_level_3_every_row_is_locked() {
    let rows = login_rows(3);
    assert_eq!(rows.len(), 6);
    assert!(rows.iter().all(|r| !r.2 && r.3 == 0), "{rows:?}");
}

#[test]
fn each_skill_choice_set_on_its_empty_row_replaces_the_plain_skill() {
    for (key, _, _, choices) in ROWS.iter().filter(|r| r.0 != "passives") {
        let plain = spell("cutthroat", key);
        for name in choices {
            let (guid, blob) = cutthroat_save(ROWS_OPEN, false);
            let (mut realm, mut view, _) = enter(guid, &blob, SEED);
            let id = choice("cutthroat", key, name);
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
        let (guid, blob) = cutthroat_save(ROWS_OPEN, false);
        let (mut realm, mut view, _) = enter(guid, &blob, SEED);
        let id = choice("cutthroat", "passives", name);
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
