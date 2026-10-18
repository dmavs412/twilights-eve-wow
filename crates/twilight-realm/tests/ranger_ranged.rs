//! The Ranger's ranged combat (#124): Auto Shot at the bow's speed, Snapshot and Puncturing Shaft
//! rolling the ranged weapon, the 8 yd minimum range, the bow gate, and Barbed Arrow's bleed.
//! Played through the realm's seam by a level 1 Ranger with a Cracked Shortbow near the
//! Crossroads. A mob runs at whoever shoots it, so the timed tests kite: each tick the Ranger
//! steps back to 20 yd of where the mob's last move puts it.

mod common;
mod gear_common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::{cast, refusal, stand_at, Mob};
use common::*;
use gear_common::{save_with, Carried};
use twilight_realm::{Realm, RealmCommand};

const RANGED: u8 = 17;
/// Bow, 3-6 every 2.3 s.
const CRACKED_SHORTBOW: u32 = 2773;
const BOW_SPEED_SECS: f32 = 2.3;

const AUTO_SHOT: u32 = 75;
const SNAPSHOT: u32 = 3044;
const PUNCTURING_SHAFT: u32 = 19434;
const BARBED_ARROW: u32 = 1978;

const SPELL_FAILED_EQUIPPED_ITEM_CLASS: u8 = 0x19;
const SPELL_FAILED_TOO_CLOSE: u8 = 0x76;

/// A level 1 Ranger carrying `items`, in the world at `seed`, beside a mob near the Crossroads.
fn ranger(items: &[Carried], seed: u64) -> (Realm, Mob) {
    let (guid, blob) = save_with(1, 0, items);
    let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"ranger\"");
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mob = drain(&mut realm)
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
    (realm, mob)
}

fn with_bow(seed: u64) -> (Realm, Mob) {
    ranger(&[(RANGED, CRACKED_SHORTBOW, 1)], seed)
}

/// Ticks `secs`, keeping the Ranger 20 yd from the mob as it runs in: the events, each with the
/// second it came at.
fn kite(realm: &mut Realm, mob: &mut Mob, secs: f32) -> Vec<(f32, SessionEvent)> {
    let mut out = Vec::new();
    let mut now = 0.0;
    while now < secs {
        realm.tick(0.05, &FlatGround);
        now += 0.05;
        for e in drain(realm) {
            if let SessionEvent::MonsterMove { guid, path, .. } = &e {
                if *guid == mob.guid {
                    // Where it heads; standing 20 yd from there keeps it out of melee.
                    mob.position = *path.last().unwrap();
                    stand_at(realm, mob, 20.0);
                }
            }
            out.push((now, e));
        }
    }
    out
}

/// The damage each landed, non-critical hit of `spell_id` dealt.
fn hits(events: &[(f32, SessionEvent)], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|(_, e)| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id && l.damage > 0 => {
                (l.hit_info == 0).then_some(l.damage)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn auto_shot_fires_at_the_bows_speed_from_20_yd() {
    let (mut realm, mut mob) = with_bow(0xA0_0001);
    stand_at(&mut realm, &mob, 20.0);
    drain(&mut realm);
    realm.handle(cast(AUTO_SHOT, Some(mob.guid)));
    let events = kite(&mut realm, &mut mob, 10.0);
    let shots: Vec<f32> = events
        .iter()
        .filter_map(|(t, e)| match e {
            SessionEvent::SpellGo { spell_id, .. } if *spell_id == AUTO_SHOT => Some(*t),
            _ => None,
        })
        .collect();
    assert!(shots.len() >= 4, "shots at {shots:?}");
    for pair in shots.windows(2) {
        let gap = pair[1] - pair[0];
        assert!(
            (gap - BOW_SPEED_SECS).abs() < 0.1,
            "a shot every {BOW_SPEED_SECS} s, got {shots:?}"
        );
    }
    assert!(
        !events
            .iter()
            .any(|(_, e)| matches!(e, SessionEvent::CancelAutoRepeat)),
        "it keeps shooting"
    );
}

#[test]
fn auto_shot_stops_when_the_target_comes_inside_8_yd() {
    let (mut realm, mob) = with_bow(0xA0_0002);
    stand_at(&mut realm, &mob, 20.0);
    realm.handle(cast(AUTO_SHOT, Some(mob.guid)));
    realm.tick(0.05, &FlatGround);
    drain(&mut realm);
    stand_at(&mut realm, &mob, 5.0);
    realm.tick(BOW_SPEED_SECS, &FlatGround);
    let events = drain(&mut realm);
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::CancelAutoRepeat)));
}

#[test]
fn auto_shot_stops_on_cancel() {
    let (mut realm, mob) = with_bow(0xA0_0003);
    stand_at(&mut realm, &mob, 20.0);
    realm.handle(cast(AUTO_SHOT, Some(mob.guid)));
    realm.tick(0.05, &FlatGround);
    realm.handle(RealmCommand::CancelAutoRepeat);
    drain(&mut realm);
    let mut later = Vec::new();
    for _ in 0..100 {
        realm.tick(0.05, &FlatGround);
        later.extend(drain(&mut realm));
    }
    assert!(!later
        .iter()
        .any(|e| matches!(e, SessionEvent::SpellGo { spell_id, .. } if *spell_id == AUTO_SHOT)));
}

/// Snapshot (+13, +1.3 a level) and Puncturing Shaft (+20, +3.5 a level) at level 1 deal the bow
/// roll plus their bonus: every hit of either lands above Auto Shot's best roll (3-6 and agility
/// / 14 x 2.3 s) and no higher than the bow's top roll plus the bonus.
#[test]
fn snapshot_and_puncturing_shaft_add_their_bonus_to_the_bow() {
    let (mut auto, mut snap, mut shaft) = (Vec::new(), Vec::new(), Vec::new());
    for seed in 0..30 {
        let (mut realm, mut mob) = with_bow(0xA1_0000 + seed);
        // The Shaft's bar breaks on a step, so it goes first, standing still before the pull.
        stand_at(&mut realm, &mob, 30.0);
        drain(&mut realm);
        realm.handle(cast(PUNCTURING_SHAFT, Some(mob.guid)));
        let mut events = Vec::new();
        for _ in 0..62 {
            realm.tick(0.05, &FlatGround);
            events.extend(drain(&mut realm).into_iter().map(|e| (0.0, e)));
        }
        stand_at(&mut realm, &mob, 20.0);
        realm.handle(cast(SNAPSHOT, Some(mob.guid)));
        realm.handle(cast(AUTO_SHOT, Some(mob.guid)));
        events.extend(kite(&mut realm, &mut mob, 3.0));
        auto.extend(hits(&events, AUTO_SHOT));
        snap.extend(hits(&events, SNAPSHOT));
        shaft.extend(hits(&events, PUNCTURING_SHAFT));
    }
    assert!(
        !auto.is_empty() && !snap.is_empty() && !shaft.is_empty(),
        "{auto:?} {snap:?} {shaft:?}"
    );
    let best_auto = *auto.iter().max().unwrap();
    assert!(best_auto <= 12, "Auto Shot: {auto:?}");
    assert!(
        snap.iter().all(|&d| d > best_auto && d <= 6 + 6 + 15),
        "Snapshot {snap:?} against Auto Shot {auto:?}"
    );
    assert!(
        shaft.iter().all(|&d| d > best_auto && d <= 6 + 6 + 24),
        "Puncturing Shaft {shaft:?} against Auto Shot {auto:?}"
    );
}

#[test]
fn the_shots_are_refused_inside_8_yd() {
    for shot in [SNAPSHOT, PUNCTURING_SHAFT, BARBED_ARROW, AUTO_SHOT] {
        let (mut realm, mob) = with_bow(0xA2_0000);
        stand_at(&mut realm, &mob, 5.0);
        drain(&mut realm);
        realm.handle(cast(shot, Some(mob.guid)));
        let events = drain(&mut realm);
        assert_eq!(
            refusal(&events, shot),
            Some(SPELL_FAILED_TOO_CLOSE),
            "{shot}"
        );
    }
}

#[test]
fn the_shots_are_refused_without_a_bow() {
    for shot in [SNAPSHOT, PUNCTURING_SHAFT, BARBED_ARROW, AUTO_SHOT] {
        let (mut realm, mob) = ranger(&[], 0xA3_0000);
        stand_at(&mut realm, &mob, 20.0);
        drain(&mut realm);
        realm.handle(cast(shot, Some(mob.guid)));
        assert_eq!(
            refusal(&drain(&mut realm), shot),
            Some(SPELL_FAILED_EQUIPPED_ITEM_CLASS),
            "{shot}"
        );
    }
}

#[test]
fn barbed_arrow_bleeds_4_times_over_8_seconds() {
    for seed in 0..10 {
        let (mut realm, mut mob) = with_bow(0xA4_0000 + seed);
        stand_at(&mut realm, &mob, 20.0);
        drain(&mut realm);
        realm.handle(cast(BARBED_ARROW, Some(mob.guid)));
        let events = kite(&mut realm, &mut mob, 9.0);
        // Barbed Arrow rolls no hit table, so it always lands.
        let ticks = events
            .iter()
            .filter(|(_, e)| match e {
                SessionEvent::PeriodicAuraLog(l) => {
                    l.spell_id == BARBED_ARROW
                        && matches!(l.ticks[0], PeriodicTick::Damage { amount, .. } if amount > 0)
                }
                _ => false,
            })
            .count();
        assert_eq!(ticks, 4, "seed {seed}");
    }
}
