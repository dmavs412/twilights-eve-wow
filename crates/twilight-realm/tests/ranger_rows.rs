//! Ranger's Talent Rows (#507, ADR 0028): the Talent row `talents` and five Skill Choice rows,
//! opening at 4, 8, 12, 16, 20 and 24. Field Dressing has no row, and Explosive Shot is fired
//! only by Explosive Payload. Played through the realm's seam by a shipped Ranger.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::{cast, refusal};
use common::ranger::*;
use common::vanguard::{enter, learned, login_rows, removed, rows_in, set_row};
use common::*;

const SEED: u64 = 0x5EED_0507;

/// The client's spell-mod op for a shot's minimum range (ADR 0010).
const SPELLMOD_MIN_RANGE: u8 = 29;

/// Each row in level order: its key, its level, its plain skill's name ("" for a Talent row) and
/// its three choices.
const ROWS: [(&str, u8, &str, [&str; 3]); 6] = [
    (
        "talents",
        4,
        "",
        ["explosive_payload", "guerilla_tactics", "point_blank"],
    ),
    (
        "snapshot",
        8,
        "Snapshot",
        ["twin_arrows", "rhythmic_draw", "adrenaline_rush"],
    ),
    (
        "vault",
        12,
        "Vault",
        ["parting_gift", "parting_shot", "unbound_leap"],
    ),
    (
        "barbed_arrow",
        16,
        "Barbed Arrow",
        ["jagged_barbs", "pinning_barb", "bloodscent"],
    ),
    (
        "puncturing_shaft",
        20,
        "Puncturing Shaft",
        ["piercing_force", "staggering_shaft", "killshot"],
    ),
    (
        "rapid_fire",
        24,
        "Rapid Fire",
        ["flurry", "rapid_assault", "running_fire"],
    ),
];

#[test]
fn at_level_24_the_login_lists_six_rows_open_and_empty_with_their_choices() {
    let want: Vec<(String, u8, bool, u32, String)> = ROWS
        .iter()
        .map(|(key, level, skill, _)| (key.to_string(), *level, true, 0, skill.to_string()))
        .collect();
    assert_eq!(login_rows(ranger_save(ROWS_OPEN), SEED), want);

    let (guid, blob) = ranger_save(ROWS_OPEN);
    let (_, _, entry) = enter(guid, &blob, SEED);
    for (row, (key, _, _, choices)) in rows_in(&entry).unwrap().iter().zip(ROWS) {
        let sent: Vec<u32> = row.choices.iter().map(|c| c.spell_id).collect();
        let sheet: Vec<u32> = choices.iter().map(|c| ranger_choice(key, c)).collect();
        assert_eq!(sent, sheet, "{key}");
    }
}

#[test]
fn at_level_3_every_row_is_locked() {
    let rows = login_rows(ranger_save(3), SEED);
    assert_eq!(rows.len(), 6);
    assert!(rows.iter().all(|r| !r.2 && r.3 == 0), "{rows:?}");
}

#[test]
fn each_skill_choice_set_on_its_empty_row_replaces_the_plain_skill() {
    for (key, _, _, choices) in ROWS.iter().filter(|r| r.0 != "talents") {
        let plain = ranger(key);
        for name in choices {
            let (guid, blob) = ranger_save(ROWS_OPEN);
            let (mut realm, mut view, _) = enter(guid, &blob, SEED);
            let id = ranger_choice(key, name);
            let events = set_row(&mut realm, &mut view, key, id);
            assert!(learned(&events, id), "{name} is learned: {events:?}");
            assert!(removed(&events, plain), "{name} removes {key}: {events:?}");
        }
    }
}

#[test]
fn each_talent_set_on_the_talents_row_is_learned_and_removes_nothing() {
    let (_, _, _, choices) = ROWS[0];
    for name in choices {
        let (guid, blob) = ranger_save(ROWS_OPEN);
        let (mut realm, mut view, _) = enter(guid, &blob, SEED);
        let id = ranger_choice("talents", name);
        let events = set_row(&mut realm, &mut view, "talents", id);
        assert!(learned(&events, id), "{name} is learned: {events:?}");
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, SessionEvent::SpellRemoved { .. })),
            "{name} removes nothing: {events:?}"
        );
    }
}

#[test]
fn explosive_payload_set_on_its_row_casts_and_buffs_the_next_auto_shot() {
    let payload = ranger_choice("talents", "explosive_payload");
    let (mut realm, mut view, me) = ranger_world(SEED, &[("talents", payload)]);
    realm.handle(cast(payload, None));
    let events = view.run(&mut realm, 0.1);
    assert_eq!(refusal(&events, payload), None, "{events:?}");
    assert!(
        view.aura(me, ranger("aura_explosive_payload")).is_some(),
        "the next Auto Shot is Explosive Shot"
    );
}

/// The `MIN_RANGE` cells in `events`, `(family bit, value)`.
fn min_range_cells(events: &[SessionEvent]) -> Vec<(u8, i32)> {
    events
        .iter()
        .filter_map(|e| match *e {
            SessionEvent::SpellModifier {
                flat: false,
                mask_bit,
                op: SPELLMOD_MIN_RANGE,
                value,
            } => Some((mask_bit, value)),
            _ => None,
        })
        .collect()
}

#[test]
fn point_blank_sends_its_min_range_cells_and_another_talent_takes_them_away() {
    let point_blank = ranger_choice("talents", "point_blank");
    let (guid, blob) = ranger_save(ROWS_OPEN);
    let (mut realm, mut view, _) = enter(guid, &blob, SEED);
    let events = set_row(&mut realm, &mut view, "talents", point_blank);
    let on = min_range_cells(&events);
    // Auto Shot, Snapshot, Puncturing Shaft, Barbed Arrow and Explosive Shot: one bit each.
    assert_eq!(on.len(), 5, "{events:?}");
    assert!(on.iter().all(|&(_, v)| v == -100), "{on:?}");

    walk_up_to(&mut realm, hub_guid(INNKEEPER));
    view.drain(&mut realm);
    let tactics = ranger_choice("talents", "guerilla_tactics");
    let events = set_row(&mut realm, &mut view, "talents", tactics);
    assert!(learned(&events, tactics), "{events:?}");
    let off = min_range_cells(&events);
    assert_eq!(
        off.iter().map(|&(b, _)| b).collect::<Vec<_>>(),
        on.iter().map(|&(b, _)| b).collect::<Vec<_>>()
    );
    assert!(off.iter().all(|&(_, v)| v == 0), "{off:?}");
}

/// Innkeeper Boorand Plainswind, in the Crossroads inn.
const INNKEEPER: u32 = 3934;
