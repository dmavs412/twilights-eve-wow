//! The Ranger's ranged combat (#124): Auto Shot at the bow's speed, Snapshot and Puncturing Shaft
//! rolling the ranged weapon, the 8 yd minimum range, the bow gate, and Barbed Arrow's bleed.
//! Played through the realm's seam by a level 1 Ranger with a Cracked Shortbow near the
//! Crossroads. A mob runs at whoever shoots it, so the timed tests kite: the Ranger stands while
//! it runs in and steps back to 30 yd once it heads inside 12. A step holds Auto Shot for 0.75 s
//! (#155), so their shot gaps allow for the hold.

mod common;
mod gear_common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::{
    assert_on_time, cast, refusal, stand_at, Mob, CROSSROADS_XY, MOVING_GRACE_SECS,
};
use common::*;
use gear_common::{descriptor, save_with, Carried};
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

/// Ticks `secs`, kiting: the Ranger stands still while the mob runs in, and steps back to 30 yd of
/// it once it is inside 12 yd. The events, each with the second it came at, and the
/// seconds the Ranger stepped at.
fn kite(realm: &mut Realm, mob: &mut Mob, secs: f32) -> (Vec<(f32, SessionEvent)>, Vec<f32>) {
    let (mut out, mut steps) = (Vec::new(), Vec::new());
    let mut me: Option<[f32; 3]> = None;
    // The mob's last path: from, to, when it set off, how long it takes.
    let mut way: Option<([f32; 3], [f32; 3], f32, f32)> = None;
    let mut now = 0.0;
    while now < secs {
        if let Some((from, to, at, secs)) = way {
            let k = if secs > 0.0 {
                ((now - at) / secs).min(1.0)
            } else {
                1.0
            };
            let here: [f32; 3] = std::array::from_fn(|i| from[i] + (to[i] - from[i]) * k);
            if me.is_none_or(|me| common::fight::flat(me, here) < 12.0) {
                mob.position = here;
                stand_at(realm, mob, 30.0);
                me = Some(away(mob, 30.0));
                steps.push(now);
            }
        }
        realm.tick(0.05, &FlatGround);
        now += 0.05;
        for e in drain(realm) {
            if let SessionEvent::MonsterMove {
                guid,
                start,
                path,
                duration_ms,
                ..
            } = &e
            {
                if *guid == mob.guid {
                    way = Some((
                        *start,
                        *path.last().unwrap(),
                        now,
                        *duration_ms as f32 / 1000.0,
                    ));
                }
            }
            out.push((now, e));
        }
    }
    (out, steps)
}

/// Where [`stand_at`] puts the Ranger `yards` from `mob`.
fn away(mob: &Mob, yards: f32) -> [f32; 3] {
    let (dx, dy) = (
        CROSSROADS_XY[0] - mob.position[0],
        CROSSROADS_XY[1] - mob.position[1],
    );
    let len = (dx * dx + dy * dy).sqrt();
    [
        mob.position[0] + dx / len * yards,
        mob.position[1] + dy / len * yards,
        mob.position[2],
    ]
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
    let (events, steps) = kite(&mut realm, &mut mob, 12.0);
    let shots: Vec<f32> = events
        .iter()
        .filter_map(|(t, e)| match e {
            SessionEvent::SpellGo { spell_id, .. } if *spell_id == AUTO_SHOT => Some(*t),
            _ => None,
        })
        .collect();
    assert!(shots.len() >= 4, "shots at {shots:?}");
    assert_on_time(&shots, &steps, BOW_SPEED_SECS);
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
    // The step holds the shot a grace; the next tick tries it.
    realm.tick(BOW_SPEED_SECS, &FlatGround);
    realm.tick(0.05, &FlatGround);
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
/// roll plus their bonus: every hit of either lands above Auto Shot's best roll (3-6 and ranged
/// attack power / 14 x 2.3 s, #156) and no higher than the bow's top roll plus the bonus.
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
        events.extend(kite(&mut realm, &mut mob, 3.0).0);
        auto.extend(hits(&events, AUTO_SHOT));
        snap.extend(hits(&events, SNAPSHOT));
        shaft.extend(hits(&events, PUNCTURING_SHAFT));
    }
    assert!(
        !auto.is_empty() && !snap.is_empty() && !shaft.is_empty(),
        "{auto:?} {snap:?} {shaft:?}"
    );
    let top = (6.0 + ranged_ap_damage()).ceil() as u32;
    let best_auto = *auto.iter().max().unwrap();
    assert!(best_auto <= top, "Auto Shot: {auto:?} over {top}");
    assert!(
        snap.iter().all(|&d| d > best_auto && d <= top + 15),
        "Snapshot {snap:?} against Auto Shot {auto:?}"
    );
    assert!(
        shaft.iter().all(|&d| d > best_auto && d <= top + 24),
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
        let (events, _) = kite(&mut realm, &mut mob, 9.0);
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

/// Auto Shot holds while the Ranger moves (#155): steps every 0.1 s for 5 s let no shot go, and
/// the timer runs on, so the first comes within a tick of the grace once they stop.
#[test]
fn auto_shot_holds_while_moving() {
    let (mut realm, mob) = with_bow(0xA0_0004);
    stand_at(&mut realm, &mob, 25.0);
    realm.tick(1.0, &FlatGround);
    drain(&mut realm);
    realm.handle(cast(AUTO_SHOT, Some(mob.guid)));
    let mut events = drain(&mut realm);
    let shot = |e: &SessionEvent| matches!(e, SessionEvent::SpellGo { spell_id, .. } if *spell_id == AUTO_SHOT);
    for i in 0..50 {
        let yards = if i % 2 == 0 { 25.5 } else { 25.0 };
        stand_at(&mut realm, &mob, yards);
        realm.tick(0.1, &FlatGround);
        events.extend(drain(&mut realm));
    }
    assert!(!events.iter().any(shot), "no shot while moving");
    assert!(!events
        .iter()
        .any(|e| matches!(e, SessionEvent::CancelAutoRepeat)));
    let mut now = 0.0;
    let fired = loop {
        realm.tick(0.05, &FlatGround);
        now += 0.05;
        if drain(&mut realm).iter().any(shot) {
            break now;
        }
        assert!(now < 2.0, "the shot never came");
    };
    assert!(
        fired <= MOVING_GRACE_SECS + 0.05 + 0.1 + 1e-3,
        "the shot came due long ago, so it goes as the hold lifts: {fired} s"
    );
}

/// The level 1 Ranger's sheet as the client gets it on entering the world.
fn sheet() -> benilla_protocol::ObjectFields {
    let (guid, blob) = save_with(1, 0, &[(RANGED, CRACKED_SHORTBOW, 1)]);
    let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"ranger\"");
    let mut realm = Realm::with_seed(0xA5_0001);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    descriptor(&drain(&mut realm), guid)
}

/// What ranged attack power adds to each shot: `AP / 14 x speed`.
fn ranged_ap_damage() -> f32 {
    sheet().unit_ranged_attack_power().unwrap() as f32 / 14.0 * BOW_SPEED_SECS
}

/// The sheet's ranged block (#156): a level 1 Ranger's ranged attack power is the Hunter's
/// `2L + 2Agi - 10`, and the bow's damage and speed ride beside it.
#[test]
fn the_sheet_shows_ranged_attack_power() {
    let sheet = sheet();
    let agi = sheet.unit_stat(1).expect("agility") as i32;
    let rap = sheet
        .unit_ranged_attack_power()
        .expect("ranged attack power");
    assert_eq!(rap, 2 + 2 * agi - 10);
    assert_eq!(
        sheet.unit_ranged_attack_time(),
        Some((BOW_SPEED_SECS * 1000.0) as u32)
    );
    let ap = rap as f32 / 14.0 * BOW_SPEED_SECS;
    // The Cracked Shortbow rolls 3-6.
    let (min, max) = (
        sheet.unit_min_ranged_damage().unwrap(),
        sheet.unit_max_ranged_damage().unwrap(),
    );
    assert!(
        (min - (3.0 + ap)).abs() < 0.01 && (max - (6.0 + ap)).abs() < 0.01,
        "{min}-{max}"
    );
}
