//! A Cutthroat's off-hand auto-attack and Dual Wield Specialization: the off hand swings on its
//! own timer at half damage, two worn weapons add 19% to every white swing's miss chance, and the
//! talent raises the off hand's damage. Played through the realm's seam by a level 10 Cutthroat
//! near the Crossroads, read off the `AttackerState` packets (`HITINFO_LEFTSWING` marks the off
//! hand).

mod common;
mod gear_common;

use benilla_protocol::messages::AttackerState;
use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::{stand_at, Mob};
use common::*;
use gear_common::{save_with, Carried};
use twilight_realm::{Realm, RealmCommand};

const LEVEL: u8 = 10;
const MAIN_HAND: u8 = 15;
const OFF_HAND: u8 = 16;
const BACKPACK: u8 = 255;
const PACK_1: u8 = 23;
/// `Talent.dbc` id of Dual Wield Specialization, 5 ranks.
static DUAL_WIELD_SPECIALIZATION: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("cutthroat", "dual_wield_specialization"));
/// 1H dagger, 6-12 every 1.5 s.
const SMALL_HAND_BLADE: u32 = 816;
/// 1H dagger, 7-14 every 1.4 s.
const GIANT_TARANTULA_FANG: u32 = 1287;

const HITINFO_AFFECTS_VICTIM: u32 = 0x2;
const HITINFO_LEFTSWING: u32 = 0x4;
const HITINFO_MISS: u32 = 0x10;

/// A Cutthroat's save carrying `items`, with `talents` learned.
fn cutthroat_save(items: &[Carried], talents: &[(u32, u8)]) -> (u64, String) {
    let (guid, mut blob) = save_with(LEVEL, 0, items);
    blob = blob.replace("calling = \"basic_warrior\"", "calling = \"cutthroat\"");
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        let line = format!("money = 0\ntalents = [{}]\n", rows.join(", "));
        blob = blob.replace("money = 0\n", &line);
    }
    (guid, blob)
}

/// A Cutthroat with a dagger in each hand.
fn two_daggers(talents: &[(u32, u8)]) -> (u64, String) {
    let items = [
        (MAIN_HAND, SMALL_HAND_BLADE, 1),
        (OFF_HAND, SMALL_HAND_BLADE, 1),
    ];
    cutthroat_save(&items, talents)
}

/// The save in a realm on `seed`, in the world, attacking a mob near it.
fn enter(guid: u64, blob: &str, seed: u64) -> Realm {
    let mut realm = Realm::with_seed(seed);
    realm.import_character(blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    let mob = events
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
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    realm
}

/// The player's swings in the `secs` after attacking, each with the second it landed at.
fn swings(realm: &mut Realm, me: u64, secs: f32) -> Vec<(f32, AttackerState)> {
    let mut out = Vec::new();
    let mut now = 0.0;
    while now < secs {
        realm.tick(0.05, &FlatGround);
        now += 0.05;
        for e in drain(realm) {
            if let SessionEvent::AttackerState(s) = e {
                if s.attacker == me {
                    out.push((now, s));
                }
            }
        }
    }
    out
}

fn off_hand(s: &AttackerState) -> bool {
    s.hit_info & HITINFO_LEFTSWING != 0
}

/// A plain hit: not a miss, a crit, a glancing blow or a crush.
fn plain_hit(s: &AttackerState) -> bool {
    s.hit_info & !HITINFO_LEFTSWING == HITINFO_AFFECTS_VICTIM && s.damage > 0
}

/// When the swings of one hand landed: the off hand's if `off`, else the main hand's.
fn times(all: &[(f32, AttackerState)], off: bool) -> Vec<f32> {
    let mut out = Vec::new();
    for (t, s) in all {
        if off_hand(s) == off {
            out.push(*t);
        }
    }
    out
}

fn assert_spaced(times: &[f32], gap: f32) {
    assert!(times.len() >= 3, "swings at {times:?}");
    for pair in times.windows(2) {
        assert!((pair[1] - pair[0] - gap).abs() < 0.11, "{times:?}");
    }
}

#[test]
fn the_off_hand_swings_at_its_own_weapons_speed() {
    let items = [
        (MAIN_HAND, SMALL_HAND_BLADE, 1),
        (OFF_HAND, GIANT_TARANTULA_FANG, 1),
    ];
    let (guid, blob) = cutthroat_save(&items, &[]);
    let mut realm = enter(guid, &blob, 0x5EED_0108);
    let all = swings(&mut realm, guid, 6.0);
    assert_spaced(&times(&all, true), 1.4);
    assert_spaced(&times(&all, false), 1.5);
}

#[test]
fn with_one_dagger_there_are_no_off_hand_swings() {
    let (guid, blob) = cutthroat_save(&[(MAIN_HAND, SMALL_HAND_BLADE, 1)], &[]);
    let mut realm = enter(guid, &blob, 0x5EED_0108);
    let all = swings(&mut realm, guid, 6.0);
    assert!(all.len() >= 3, "{} swings", all.len());
    assert!(times(&all, true).is_empty());
}

#[test]
fn a_second_dagger_goes_in_the_off_hand_for_a_cutthroat_only() {
    let items = [
        (MAIN_HAND, SMALL_HAND_BLADE, 1),
        (PACK_1, GIANT_TARANTULA_FANG, 1),
    ];
    let equip = RealmCommand::AutoEquipItem {
        bag: BACKPACK,
        slot: PACK_1,
    };

    let (guid, blob) = cutthroat_save(&items, &[]);
    let mut realm = enter(guid, &blob, 0x5EED_0108);
    realm.handle(equip.clone());
    let all = swings(&mut realm, guid, 4.0);
    assert!(!times(&all, true).is_empty(), "a Cutthroat dual wields");

    // Every other Calling keeps one-handers in the main hand: the new dagger replaces the old.
    let (guid, blob) = save_with(LEVEL, 0, &items);
    let mut realm = enter(guid, &blob, 0x5EED_0108);
    realm.handle(equip);
    let all = swings(&mut realm, guid, 4.0);
    assert!(!all.is_empty());
    assert!(times(&all, true).is_empty(), "a Warrior does not");
}

/// Every swing of the first tick the player attacks in, over `seeds` fights: both hands swing at
/// once, because both timers start together.
fn opening_swings(save: &(u64, String), seeds: u64) -> Vec<AttackerState> {
    let (guid, blob) = save;
    let mut out = Vec::new();
    for seed in 0..seeds {
        let mut realm = enter(*guid, blob, 0x5EED_1000 + seed);
        for (_, s) in swings(&mut realm, *guid, 0.1) {
            out.push(s);
        }
    }
    out
}

/// The mean damage of the plain hits of one hand.
fn mean_plain_hit(swings: &[AttackerState], off: bool) -> f64 {
    let mut hits = Vec::new();
    for s in swings {
        if off_hand(s) == off && plain_hit(s) {
            hits.push(f64::from(s.damage));
        }
    }
    assert!(hits.len() > 20, "only {} plain hits", hits.len());
    hits.iter().sum::<f64>() / hits.len() as f64
}

/// How many of the main hand's opening swings missed.
fn main_hand_misses(swings: &[AttackerState]) -> usize {
    let missed = |s: &&AttackerState| !off_hand(s) && s.hit_info & HITINFO_MISS != 0;
    swings.iter().filter(missed).count()
}

#[test]
fn two_weapons_add_to_the_miss_chance() {
    let one = cutthroat_save(&[(MAIN_HAND, SMALL_HAND_BLADE, 1)], &[]);
    let two = main_hand_misses(&opening_swings(&two_daggers(&[]), 300));
    let single = main_hand_misses(&opening_swings(&one, 300));
    // 19 points more of 300 fights is about 57; the base chance is under 1%.
    assert!(two > single + 30, "two weapons {two}, one {single}");
}

#[test]
fn the_off_hand_deals_about_half_the_main_hands_damage() {
    let swings = opening_swings(&two_daggers(&[]), 300);
    let main = mean_plain_hit(&swings, false);
    let off = mean_plain_hit(&swings, true);
    let ratio = off / main;
    assert!(ratio > 0.4 && ratio < 0.6, "off {off:.2}, main {main:.2}");
}

#[test]
fn dual_wield_specialization_raises_the_off_hands_damage() {
    let bare = opening_swings(&two_daggers(&[]), 300);
    let talents = [(*DUAL_WIELD_SPECIALIZATION, 5)];
    let trained = opening_swings(&two_daggers(&talents), 300);
    let ratio = mean_plain_hit(&trained, true) / mean_plain_hit(&bare, true);
    // A Power Level body's attack power is a larger part of each hit, which the talent does not raise.
    assert!(ratio > 1.4 && ratio < 1.7, "5/5 deals {ratio:.2}x of 0/5");
}
