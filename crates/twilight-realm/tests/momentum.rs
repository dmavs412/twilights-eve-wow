//! Cutthroat's Momentum, the Recruit tree's Charge talent: Charge grants a crit aura (the next
//! Heroic Strike has more crit chance, used up when it lands) and a run speed aura (the client is
//! told to move faster for the aura's duration). Played through the realm's seam by a level 10
//! Basic Warrior.

mod common;

use benilla_protocol::messages::SpeedKind;
use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const MOMENTUM: u32 = 222;
static HEROIC_STRIKE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("basic_warrior", "heroic_strike"));
static CHARGE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("basic_warrior", "charge"));
/// The borrowed spell the crit aura shows as (Cold Blood), and the speed aura (Sprint).
static CRIT_AURA: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("basic_warrior", "momentum_crit_1"));
static SPEED_AURA: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("basic_warrior", "momentum_speed_1"));
/// A player's run speed, yd/s.
const RUN: f32 = 7.0;

/// A level 10 human warrior with `talents` in its save, in the world, the mob near it.
fn warrior(talents: &[(u32, u8)], seed: u64) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n");
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
    }
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let mob = view
        .drain(&mut realm)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("a mob near the Crossroads");
    (realm, guid, mob, view)
}

/// The run speeds the realm told the client to move at, in order.
fn run_speeds(events: &[SessionEvent], me: u64) -> Vec<f32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ForceSpeedChange {
                guid,
                kind: SpeedKind::Run,
                speed,
                ..
            } if *guid == me => Some(*speed),
            _ => None,
        })
        .collect()
}

/// Charges the mob, then fights it with white swings until the warrior has the rage for Heroic
/// Strike. The aura's 10 s are not yet up.
fn charge_and_build_rage(realm: &mut Realm, view: &mut View, me: u64, mob: &Mob) {
    stand_at(realm, mob, 15.0);
    realm.handle(cast(*CHARGE, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(realm), *CHARGE), None, "the charge");
    stand_at(realm, mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    for _ in 0..80 {
        if view.rage(me) >= 15 {
            return;
        }
        view.run(realm, 0.1);
    }
    panic!("no rage for Heroic Strike in eight seconds");
}

/// Whether the Heroic Strike thrown after Charge crit, and whether the crit aura was still on
/// the warrior after it; `None` if the strike did not land.
fn strike_after_charge(talents: &[(u32, u8)], seed: u64) -> Option<(bool, bool)> {
    let (mut realm, me, mob, mut view) = warrior(talents, seed);
    charge_and_build_rage(&mut realm, &mut view, me, &mob);
    realm.handle(cast(*HEROIC_STRIKE, Some(mob.guid)));
    let events = view.run(&mut realm, 3.0);
    let crit = events.iter().find_map(|e| match e {
        SessionEvent::SpellDamageLog(l) if l.spell_id == *HEROIC_STRIKE => Some(l.hit_info != 0),
        _ => None,
    })?;
    Some((crit, view.aura(me, *CRIT_AURA).is_some()))
}

#[test]
fn the_next_heroic_strike_after_charge_crits_and_uses_the_charge_up() {
    let mut landed = 0;
    for seed in 0..12 {
        let Some((crit, aura_left)) = strike_after_charge(&[(MOMENTUM, 2)], seed) else {
            continue;
        };
        landed += 1;
        assert!(crit, "seed {seed}: rank 2 adds 100% crit chance");
        assert!(!aura_left, "seed {seed}: the charge is consumed");
    }
    assert!(landed >= 6, "only {landed} strikes landed");
}

#[test]
fn without_the_talent_heroic_strike_after_charge_does_not_always_crit() {
    let crits: Vec<bool> = (0..12)
        .filter_map(|seed| strike_after_charge(&[], seed))
        .map(|(crit, _)| crit)
        .collect();
    assert!(crits.iter().any(|c| !c), "{crits:?}");
}

#[test]
fn white_swings_do_not_use_the_crit_charge() {
    let (mut realm, me, mob, mut view) = warrior(&[(MOMENTUM, 2)], 3);
    charge_and_build_rage(&mut realm, &mut view, me, &mob);
    assert!(view.aura(me, *CRIT_AURA).is_some());
}

#[test]
fn charge_speeds_the_warrior_up_for_the_auras_duration_then_back() {
    let (mut realm, me, mob, mut view) = warrior(&[(MOMENTUM, 2)], 3);
    stand_at(&mut realm, &mob, 15.0);
    realm.handle(cast(*CHARGE, Some(mob.guid)));
    let at_cast = run_speeds(&view.drain(&mut realm), me);
    assert_eq!(at_cast.len(), 1, "{at_cast:?}");
    assert!((at_cast[0] - RUN * 1.3).abs() < 1e-3, "{at_cast:?}");
    assert!(view.aura(me, *SPEED_AURA).is_some());

    let during = run_speeds(&view.run(&mut realm, 3.0), me);
    assert!(during.is_empty(), "still fast: {during:?}");
    let after = run_speeds(&view.run(&mut realm, 1.5), me);
    assert_eq!(after, vec![RUN]);
    assert_eq!(view.aura(me, *SPEED_AURA), None);
}

#[test]
fn rank_one_grants_its_smaller_speed() {
    let (mut realm, me, mob, mut view) = warrior(&[(MOMENTUM, 1)], 3);
    stand_at(&mut realm, &mob, 15.0);
    realm.handle(cast(*CHARGE, Some(mob.guid)));
    let at_cast = run_speeds(&view.drain(&mut realm), me);
    assert!((at_cast[0] - RUN * 1.15).abs() < 1e-3, "{at_cast:?}");
}

#[test]
fn without_the_talent_charge_changes_no_speed() {
    let (mut realm, me, mob, mut view) = warrior(&[], 3);
    stand_at(&mut realm, &mob, 15.0);
    realm.handle(cast(*CHARGE, Some(mob.guid)));
    assert!(run_speeds(&view.drain(&mut realm), me).is_empty());
    assert_eq!(view.aura(me, *CRIT_AURA), None);
}
