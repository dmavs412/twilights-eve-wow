//! Vanguard's Talent Rows (#504, ADR 0028): five Skill Choice rows and the Talent row `passives`,
//! opening at 4, 8, 12, 16, 20 and 24. Played through the realm's seam by a shipped Vanguard at
//! the Crossroads.

mod common;

use common::vanguard::*;
use common::*;

const SEED: u64 = 0x5EED_0504;

/// Each row in level order: its key, its level, its plain skill's name ("" for a Talent row) and
/// its three choices.
const ROWS: [(&str, u8, &str, [&str; 3]); 6] = [
    (
        "righteous_smash",
        4,
        "Righteous Smash",
        ["iron_juggernaut", "vengeful_conviction", "penitents_smash"],
    ),
    (
        "shield_of_penitent",
        8,
        "Shield of the Penitent",
        [
            "searing_penance",
            "shield_specialization",
            "steadfast_penitent",
        ],
    ),
    (
        "zealous_cleave",
        12,
        "Zealous Cleave",
        ["sweeping_zeal", "zealous_restoration", "tithe_of_zeal"],
    ),
    (
        "bulwark_charge",
        16,
        "Bulwark Charge",
        ["hammer_of_vanguard", "bulwark_slam", "fervent_charge"],
    ),
    (
        "passives",
        20,
        "",
        ["avenging_light", "bulwark_of_faith", "ceaseless_zeal"],
    ),
    (
        "seal_of_iron",
        24,
        "Seal of Iron",
        ["wrathful_iron", "seal_of_mending", "seal_of_fervor"],
    ),
];

/// The rows the login of a Vanguard of `level` sends, as `(key, level, open, set, skill)`.
fn login_rows(level: u8) -> Vec<(String, u8, bool, u32, String)> {
    let (guid, blob) = vanguard_save(level, true);
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
    assert_eq!(login_rows(24), want);

    let (guid, blob) = vanguard_save(24, true);
    let (_, _, entry) = enter(guid, &blob, SEED);
    for (row, (key, _, _, choices)) in rows_in(&entry).unwrap().iter().zip(ROWS) {
        let sent: Vec<u32> = row.choices.iter().map(|c| c.spell_id).collect();
        let sheet: Vec<u32> = choices.iter().map(|c| choice("vanguard", key, c)).collect();
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
        let plain = spell("vanguard", key);
        for name in choices {
            let (guid, blob) = vanguard_save(24, true);
            let (mut realm, mut view, _) = enter(guid, &blob, SEED);
            let id = choice("vanguard", key, name);
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
        let (guid, blob) = vanguard_save(24, true);
        let (mut realm, mut view, _) = enter(guid, &blob, SEED);
        let id = choice("vanguard", "passives", name);
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
