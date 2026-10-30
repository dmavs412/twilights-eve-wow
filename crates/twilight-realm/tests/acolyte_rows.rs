//! Acolyte's Talent Rows (#508, ADR 0028): five Skill Choice rows and the Talent row `talents`,
//! opening at 4, 8, 12, 16, 20 and 24. Hush and Grasping Shadows have no row. Played through the
//! realm's seam by a shipped Acolyte.

mod common;

use benilla_protocol::SessionEvent;
use common::acolyte::*;
use common::cutthroat::{damage_of, healed};
use common::vanguard::{enter, learned, removed, rows_in, set_row, STURDY};
use common::*;

const SEED: u64 = 0x5EED_0508;

/// Innkeeper Boorand Plainswind, in the Crossroads inn.
const INNKEEPER: u32 = 3934;

/// Each row in level order: its key, its level, its plain skill's name ("" for a Talent row) and
/// its three choices.
const ROWS: [(&str, u8, &str, [&str; 3]); 6] = [
    (
        "creeping_blight",
        4,
        "Creeping Blight",
        ["epidemic", "virulence", "withering_blight"],
    ),
    (
        "blood_lance",
        8,
        "Blood Lance",
        ["exsanguinate", "splintering_lance", "sanguine_barrier"],
    ),
    (
        "talents",
        12,
        "",
        ["necrotic_detonation", "masochism", "will_to_survive"],
    ),
    (
        "siphon_life",
        16,
        "Siphon Life",
        ["feeding_siphon", "gorging_siphon", "unbroken_siphon"],
    ),
    (
        "dark_mending",
        20,
        "Dark Mending",
        ["coagulation", "dark_surplus", "sanguine_flow"],
    ),
    (
        "miasma",
        24,
        "Miasma",
        ["noxious_vapors", "breeding_ground", "shroud_of_rot"],
    ),
];

/// The rows the login of an Acolyte of `level` sends, as `(key, level, open, set, skill)`.
fn login_rows(level: u8) -> Vec<(String, u8, bool, u32, String)> {
    let (guid, blob) = acolyte_save(level);
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

    let (guid, blob) = acolyte_save(ROWS_OPEN);
    let (_, _, entry) = enter(guid, &blob, SEED);
    for (row, (key, _, _, choices)) in rows_in(&entry).unwrap().iter().zip(ROWS) {
        let sent: Vec<u32> = row.choices.iter().map(|c| c.spell_id).collect();
        let sheet: Vec<u32> = choices.iter().map(|c| row_choice(key, c)).collect();
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
    for (key, _, _, choices) in ROWS.iter().filter(|r| r.0 != "talents") {
        let plain = acolyte(key);
        for name in choices {
            let (guid, blob) = acolyte_save(ROWS_OPEN);
            let (mut realm, mut view, _) = enter(guid, &blob, SEED);
            let id = row_choice(key, name);
            let events = set_row(&mut realm, &mut view, key, id);
            assert!(learned(&events, id), "{name} is learned: {events:?}");
            assert!(removed(&events, plain), "{name} removes {key}: {events:?}");
        }
    }
}

#[test]
fn each_talent_set_on_the_talents_row_is_learned_and_removes_nothing() {
    let (_, _, _, choices) = ROWS[2];
    for name in choices {
        let (guid, blob) = acolyte_save(ROWS_OPEN);
        let (mut realm, mut view, _) = enter(guid, &blob, SEED);
        let id = row_choice("talents", name);
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

/// Necrotic Detonation, learned from the Talent row, casts at a Blighted mob: it consumes the
/// Blight for damage under its own spell and heals the Acolyte.
#[test]
fn necrotic_detonation_set_on_the_talents_row_casts_and_consumes_the_blight() {
    let detonation = row_choice("talents", "necrotic_detonation");
    let mut s = acolyte_scene(SEED, &[("talents", detonation)], STURDY);
    let mob = s.mob.guid;
    cast_through(&mut s.realm, &mut s.view, acolyte("creeping_blight"), mob);
    s.view.run(&mut s.realm, 3.1);
    assert!(blight_stacks(&s.view, mob) >= 2, "the Blight grew");
    let events = cast_through(&mut s.realm, &mut s.view, detonation, mob);
    let hits = damage_of(&events, detonation);
    assert!(
        matches!(hits[..], [(t, d, _)] if t == mob && d > 0),
        "{hits:?}"
    );
    assert_eq!(blight_stacks(&s.view, mob), 0, "the Blight is consumed");
    assert_eq!(healed(&events, detonation).len(), 1, "it heals the Acolyte");
}

/// Under Masochism each Blood Lance, which costs health, puts up Masochism's heal over time; with
/// the Talent row empty the Lance puts up nothing.
#[test]
fn masochisms_heal_follows_a_blood_lance() {
    let masochism = row_choice("talents", "masochism");
    let heal = acolyte("aura_masochism_5");
    let lance = acolyte("blood_lance");
    let mut s = acolyte_scene(SEED, &[("talents", masochism)], STURDY);
    let me = s.me;
    let full = s.view.health(me);
    cast_through(&mut s.realm, &mut s.view, lance, s.mob.guid);
    let cost = full - s.view.health(me);
    assert!(cost > 0, "the Lance cost health");
    assert!(s.view.aura(me, heal).is_some(), "Masochism's heal is up");
    let ticks = heal_ticks(&s.view.run(&mut s.realm, 6.0), heal);
    assert_eq!(ticks.len(), 5, "{ticks:?}");
    let total: u32 = ticks.iter().sum();
    assert!(
        total.abs_diff(cost * 30 / 100) <= 5,
        "{ticks:?} make 30% of {cost}"
    );

    let mut s = acolyte_scene(SEED, &[], STURDY);
    let me = s.me;
    cast_through(&mut s.realm, &mut s.view, lance, s.mob.guid);
    assert!(s.view.aura(me, heal).is_none());
    assert!(heal_ticks(&s.view.run(&mut s.realm, 6.0), heal).is_empty());
}

/// Changing the Talent row from Masochism to Will to Survive at the innkeeper takes Masochism's
/// proc away: a Blood Lance after it puts up no heal.
#[test]
fn changing_masochism_to_will_to_survive_at_the_innkeeper_takes_the_proc_away() {
    let masochism = row_choice("talents", "masochism");
    let will = row_choice("talents", "will_to_survive");
    let heal = acolyte("aura_masochism_5");
    let (mut realm, mut view, me) = acolyte_world(SEED, &[("talents", masochism)]);
    walk_up_to(&mut realm, hub_guid(INNKEEPER));
    view.drain(&mut realm);
    let events = set_row(&mut realm, &mut view, "talents", will);
    assert!(removed(&events, masochism), "{events:?}");
    assert!(learned(&events, will), "{events:?}");
    let mob = walk_west_of(&mut realm, &mut view, STURDY);
    cast_through(&mut realm, &mut view, acolyte("blood_lance"), mob.guid);
    assert!(view.aura(me, heal).is_none(), "no Masochism heal");
    assert!(heal_ticks(&view.run(&mut realm, 6.0), heal).is_empty());
}

/// For each Siphon Life tick at dummy 0 in the Test Grounds after the Acolyte has bled below 45%
/// of their health on Blood Lances at the far dummy: their health share before it, in percent,
/// what it dealt and what it healed.
fn siphon_heals(rows: &[(&str, u32)]) -> Vec<(u32, u32, u32)> {
    let mut y = yard(SEED, rows, [-9655.0, 255.0]);
    let me = y.me;
    let max = max_health(&y.view, me);
    while y.view.health(me) * 100 >= max * 45 {
        y.cast_through(acolyte("blood_lance"), dummy(3));
    }
    let siphon = acolyte("siphon_life");
    y.cast_through(siphon, dummy(0));
    let mut ticks = Vec::new();
    for _ in 0..55 {
        let share = y.view.health(me) * 100 / max;
        let events = y.run(0.1);
        if let ([(_, dealt, _)], [healed]) = (
            &damage_of(&events, siphon)[..],
            &common::cutthroat::healed(&events, siphon)[..],
        ) {
            ticks.push((share, *dealt, *healed));
        }
    }
    ticks
}

/// Under Will to Survive a Siphon Life tick that lands below half health heals 30% more than it
/// dealt, and one at half health or above heals what it dealt; with Masochism set instead, every
/// tick heals what it dealt.
#[test]
fn will_to_survive_raises_a_heal_below_half_health_and_not_above() {
    let will = row_choice("talents", "will_to_survive");
    let ticks = siphon_heals(&[("talents", will)]);
    assert!(ticks.iter().any(|t| t.0 < 50), "{ticks:?}");
    assert!(ticks.iter().any(|t| t.0 >= 50), "{ticks:?}");
    for &(share, dealt, healed) in &ticks {
        let want = if share < 50 {
            (dealt * 130 + 50) / 100
        } else {
            dealt
        };
        assert_eq!(healed, want, "at {share}%: {ticks:?}");
    }
    let masochism = row_choice("talents", "masochism");
    let ticks = siphon_heals(&[("talents", masochism)]);
    assert!(ticks.iter().any(|t| t.0 < 50), "{ticks:?}");
    assert!(ticks.iter().all(|&(_, d, h)| h == d), "{ticks:?}");
}
