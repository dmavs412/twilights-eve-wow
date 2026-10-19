//! Notoriety and its Heat Levels (#176): what raises it, the Hub's drain, death's reset, the XP
//! multiplier on top of the Tier XP Rate, the Heat aura, and the save.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;
use twilight_realm::rules::notoriety::*;
use twilight_realm::rules::xp::Elite;

// ── The rules ───────────────────────────────────────────────────────────────────────────────────

#[test]
fn the_heat_level_steps_up_at_each_threshold() {
    assert_eq!(heat_levels(), 4);
    for level in 1..=heat_levels() {
        let at = threshold(level);
        assert_eq!(heat_level(at), level, "at {at}");
        assert_eq!(heat_level(at - 0.01), level - 1, "just under {at}");
    }
    assert_eq!(heat_level(0.0), 0);
    assert_eq!(heat_level(1.0e6), heat_levels());
}

#[test]
fn each_heat_level_multiplies_xp_and_level_0_does_not() {
    assert_eq!(with_heat(100, 0), 100);
    let mut last = 100;
    for level in 1..=heat_levels() {
        let xp = with_heat(100, level);
        assert!(xp > last, "level {level}: {xp}");
        last = xp;
    }
}

#[test]
fn each_heat_level_has_an_aura_and_level_0_none() {
    assert_eq!(heat_aura(0), None);
    let auras: Vec<u32> = (1..=heat_levels()).filter_map(heat_aura).collect();
    assert_eq!(auras.len(), usize::from(heat_levels()));
}

#[test]
fn a_gray_kill_is_worth_nothing() {
    assert_eq!(kill_gain(20, 3, Elite::No, false), 0.0);
    assert!(kill_gain(5, 3, Elite::No, false) > 0.0);
}

#[test]
fn higher_elite_and_boss_kills_are_worth_more() {
    let even = kill_gain(10, 10, Elite::No, false);
    assert!(kill_gain(10, 12, Elite::No, false) > even);
    assert!(kill_gain(10, 8, Elite::No, false) < even);
    assert!(kill_gain(10, 10, Elite::OpenWorld, false) > even);
    assert!(
        kill_gain(10, 10, Elite::Dungeon, true) > kill_gain(10, 10, Elite::Dungeon, false),
        "a boss is the Dungeon's clear"
    );
}

#[test]
fn only_rare_drops_and_better_are_worth_notoriety() {
    assert_eq!(drop_gain(2), 0.0);
    assert!(drop_gain(3) > 0.0 && drop_gain(4) > 0.0);
}

#[test]
fn advancement_halves_rounding_down_and_the_drain_stops_at_0() {
    assert_eq!(advanced(301.0), 150.0);
    assert_eq!(drained(1.0, 1000.0), 0.0);
    assert!(drained(100.0, 1.0) < 100.0);
}

// ── In the world ────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_kill_raises_notoriety_and_a_gray_one_does_not() {
    let (mut realm, me, events) = warrior_at(5, 0.0);
    kill(&mut realm, &the_mob(&events));
    assert!(notoriety(&realm, me) > 0.0, "level 5 kills a level 3");

    let (mut realm, me, events) = warrior_at(20, 0.0);
    kill(&mut realm, &the_mob(&events));
    assert_eq!(notoriety(&realm, me), 0.0, "level 3 is gray to level 20");
}

#[test]
fn kill_xp_is_base_times_the_tier_rate_times_the_heat_multiplier() {
    // Level 5 kills the level 3 scorpid: 42 at ×1, ×3 for a Vanguard. 650 stays at Heat 3
    // through the fight's drain.
    let heat = heat_level(650.0);
    assert_eq!(heat, 3);
    for (notoriety, want) in [(0.0, 3 * 42), (650.0, with_heat(3 * 42, heat))] {
        let (guid, blob) = with_notoriety(save_with(5, 0, &[]), notoriety);
        let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"vanguard\"");
        let (mut realm, _, events) = enter_saved(guid, &blob);
        let events = kill(&mut realm, &the_mob(&events));
        let paid = events
            .iter()
            .find_map(|e| match e {
                SessionEvent::XpGain(x) if x.kill => Some(x.total),
                _ => None,
            })
            .expect("the kill pays XP");
        assert_eq!(paid, want, "at {notoriety} Notoriety");
    }
}

#[test]
fn death_drops_notoriety_to_0_and_takes_the_heat_aura() {
    let (mut realm, me, events) = warrior_at(1, 650.0);
    let mut view = common::fight::View::default();
    view.see(&events);
    view.see(&run(&mut realm, 0.1));
    assert!(
        view.aura(me, heat_aura(3).unwrap()).is_some(),
        "Heat 3 shows"
    );

    let mob = the_mob(&events);
    stand_by(&mut realm, &mob);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    realm.handle(RealmCommand::AttackStop);
    for _ in 0..12_000 {
        view.see(&run(&mut realm, 0.1));
        if view.health(me) == 0 {
            break;
        }
    }
    assert_eq!(view.health(me), 0, "the mob killed the warrior");
    assert_eq!(notoriety(&realm, me), 0.0);
    assert!(view.aura(me, heat_aura(3).unwrap()).is_none());
}

#[test]
fn the_hub_drains_notoriety_and_stops_at_0() {
    // A new character stands in the Crossroads, the Hub.
    let (mut realm, me, _) = warrior_at(5, 3.0);
    run(&mut realm, 2.0);
    let left = notoriety(&realm, me);
    assert!(left > 0.0 && left < 3.0, "{left}");
    run(&mut realm, 60.0);
    assert_eq!(notoriety(&realm, me), 0.0);
}

#[test]
fn the_heat_aura_comes_back_on_login() {
    let (realm, me, _) = warrior_at(5, 350.0);
    let blob = realm.export_character(me).unwrap();
    let (mut realm, _, events) = enter_saved(me, &blob);
    let mut view = common::fight::View::default();
    view.see(&events);
    view.see(&run(&mut realm, 0.1));
    assert!(view.aura(me, heat_aura(2).unwrap()).is_some());
    assert!(
        view.aura(me, heat_aura(1).unwrap()).is_none(),
        "one aura, the level's"
    );
}

#[test]
fn notoriety_survives_the_save_and_an_old_save_has_none() {
    let (realm, me, _) = warrior_at(5, 250.0);
    let blob = realm.export_character(me).unwrap();
    assert!(blob.contains("notoriety = 250"), "{blob}");

    let (guid, old) = save_with(5, 0, &[]);
    assert!(!old.contains("notoriety"));
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&old).unwrap();
    assert_eq!(notoriety(&realm, guid), 0.0);
}

// ── Helpers ─────────────────────────────────────────────────────────────────────────────────────

/// `save` with `notoriety` written in.
fn with_notoriety((guid, blob): (u64, String), notoriety: f32) -> (u64, String) {
    let blob = blob.replacen(
        "xp = 0\n",
        &format!("xp = 0\nnotoriety = {notoriety:?}\n"),
        1,
    );
    (guid, blob)
}

/// A warrior of `level` with `notoriety`, in the world at the Crossroads.
fn warrior_at(level: u8, notoriety: f32) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = with_notoriety(save_with(level, 0, &[]), notoriety);
    enter_saved(guid, &blob)
}

/// The Notoriety `guid`'s save holds.
fn notoriety(realm: &Realm, guid: u64) -> f32 {
    realm
        .export_character(guid)
        .unwrap()
        .lines()
        .find_map(|l| l.strip_prefix("notoriety = "))
        .map_or(0.0, |v| v.parse().unwrap())
}
