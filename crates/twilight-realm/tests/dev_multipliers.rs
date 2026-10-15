//! The dev multipliers (`?speed=` / `?xp=`): testing aids read at boot, never saved.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::*;
use twilight_realm::{DevMultipliers, CROSSROADS};

#[test]
fn the_xp_multiplier_multiplies_kill_xp() {
    let dev = DevMultipliers {
        xp: 10.0,
        ..DevMultipliers::default()
    };
    let (mut realm, me, mob) = warrior_at(5, dev);
    let events = kill(&mut realm, mob);

    // Level 5 kills level 3: 70 × 3/5 = 42, ten times over.
    assert_eq!(kill_xp(&events), Some(420));
    assert_eq!(xp_shown(&events, me), Some(420));
}

#[test]
fn without_the_flags_xp_is_vanillas() {
    assert_eq!(
        DevMultipliers::default(),
        DevMultipliers { speed: 1.0, xp: 1.0 }
    );
    let (mut realm, me, mob) = warrior_at(5, DevMultipliers::default());
    let events = kill(&mut realm, mob);
    assert_eq!(kill_xp(&events), Some(42));
    assert_eq!(xp_shown(&events, me), Some(42));
}

#[test]
fn multiplied_xp_levels_the_character_up() {
    // 42 × 100 = 4200 XP: a level-5 warrior's 2800 to level 6, and 1400 of level 6's 3600.
    let dev = DevMultipliers {
        xp: 100.0,
        ..DevMultipliers::default()
    };
    let (mut realm, me, mob) = warrior_at(5, dev);
    let events = kill(&mut realm, mob);
    let levels: Vec<u32> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::LevelUp(l) => Some(l.level),
            _ => None,
        })
        .collect();
    assert_eq!(levels, [6]);
    assert_eq!(xp_shown(&events, me), Some(1400));
}

#[test]
fn no_save_carries_a_multiplier() {
    let dev = DevMultipliers {
        speed: 3.0,
        xp: 10.0,
    };
    let (mut realm, me, mob) = warrior_at(5, dev);
    kill(&mut realm, mob);
    realm.handle(RealmCommand::Logout);
    let blob = realm.export_character(me).unwrap();
    assert!(!blob.contains("speed"), "{blob}");
    for op in realm.drain_storage() {
        if let twilight_realm::Storage::Write { blob, .. } = op {
            assert!(!blob.contains("speed"), "{blob}");
        }
    }

    // The same character in a realm booted without the flags earns vanilla XP and runs at 7 yd/s.
    let mut plain = Realm::with_seed(SEED);
    plain.import_character(&blob).unwrap();
    plain.handle(RealmCommand::EnterWorld { guid: me });
    plain.tick(0.0, &FlatGround);
    let events = drain(&mut plain);
    assert_eq!(own_speeds(&events, me).run, 7.0);
    let before = own_xp(&events, me);
    let mob = the_mob(&events);
    let events = kill(&mut plain, mob);
    assert_eq!(kill_xp(&events), Some(42));
    assert_eq!(xp_shown(&events, me), Some(before + 42));
}

#[test]
fn the_speed_multiplier_scales_the_players_movement_speeds() {
    let dev = DevMultipliers {
        speed: 3.0,
        ..DevMultipliers::default()
    };
    let (_, events) = enter_with(dev);
    let me = own_guid(&events);
    let s = own_speeds(&events, me);
    assert_eq!(
        (s.walk, s.run, s.run_back, s.swim, s.swim_back),
        (7.5, 21.0, 13.5, 4.722_222 * 3.0, 7.5)
    );
    assert_eq!(s.turn_rate, std::f32::consts::PI, "turning is not movement");
}

#[test]
fn without_the_flags_the_player_runs_at_vanillas_7_yards_a_second() {
    let (_, events) = enter_with(DevMultipliers::default());
    let me = own_guid(&events);
    let s = own_speeds(&events, me);
    assert_eq!((s.walk, s.run, s.run_back), (2.5, 7.0, 4.5));
}

#[test]
fn the_realm_keeps_up_with_a_faster_player() {
    let dev = DevMultipliers {
        speed: 3.0,
        ..DevMultipliers::default()
    };
    let (mut realm, events) = enter_with(dev);
    let me = own_guid(&events);
    // One second of running at 21 yd/s, reported as a client reports it.
    let [x, y, z, _] = CROSSROADS;
    let there = [x + 21.0, y, z];
    realm.handle(RealmCommand::Move {
        position: there,
        orientation: 0.0,
    });
    realm.tick(1.0, &FlatGround);
    realm.handle(RealmCommand::Logout);
    let blob = realm.export_character(me).unwrap();
    let save: toml::Value = toml::from_str(&blob).unwrap();
    let position: Vec<f64> = save["position"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_float().unwrap())
        .collect();
    assert_eq!(position, there.map(f64::from));
}

// ── Helpers ─────────────────────────────────────────────────────────────────────────────────────

const SEED: u64 = 4;

/// A human warrior of `level`, in the world of a realm with `dev`, and the mob near it.
fn warrior_at(level: u8, dev: DevMultipliers) -> (Realm, u64, Mob) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"));
    let mut realm = Realm::with_seed(SEED).with_dev(dev);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    let mob = the_mob(&events);
    (realm, guid, mob)
}

/// A new human warrior entering the world of a realm with `dev`, and the entry's events.
fn enter_with(dev: DevMultipliers) -> (Realm, Vec<SessionEvent>) {
    let mut realm = Realm::with_seed(SEED).with_dev(dev);
    let (_, events) = enter_new(&mut realm, "Varian", HUMAN);
    (realm, events)
}

fn own_guid(events: &[SessionEvent]) -> u64 {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::Connected { self_guid, .. } => Some(*self_guid),
            _ => None,
        })
        .expect("connected")
}

/// Stands by `mob` and auto-attacks it until it dies, collecting the events.
fn kill(realm: &mut Realm, mob: Mob) -> Vec<SessionEvent> {
    let [hx, hy, ..] = CROSSROADS;
    let (dx, dy) = (hx - mob.position[0], hy - mob.position[1]);
    let len = (dx * dx + dy * dy).sqrt();
    let at = [
        mob.position[0] + dx / len * 2.0,
        mob.position[1] + dy / len * 2.0,
        mob.position[2],
    ];
    realm.handle(RealmCommand::Move {
        position: at,
        orientation: (-dy).atan2(-dx),
    });
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut events = Vec::new();
    for _ in 0..1200 {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(realm));
        let dead = events.iter().any(|e| matches!(e,
            SessionEvent::ObjectValues { guid, fields } if *guid == mob.guid && fields.unit_health() == Some(0)));
        if dead {
            return events;
        }
    }
    panic!("the mob outlived two minutes of fighting");
}

/// The kill's XP, as the client's combat log reports it.
fn kill_xp(events: &[SessionEvent]) -> Option<u32> {
    events.iter().find_map(|e| match e {
        SessionEvent::XpGain(x) if x.kill => Some(x.total),
        _ => None,
    })
}

/// The XP bar's last value.
fn xp_shown(events: &[SessionEvent], me: u64) -> Option<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectValues { guid, fields } if *guid == me => fields.player_xp(),
            _ => None,
        })
        .last()
}

/// The movement speeds on the player's own create.
fn own_speeds(events: &[SessionEvent], me: u64) -> benilla_protocol::MoveSpeeds {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate { guid, speeds, .. } if *guid == me => *speeds,
            _ => None,
        })
        .expect("our own create carries speeds")
}

/// The XP on the player's own create.
fn own_xp(events: &[SessionEvent], me: u64) -> u32 {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate { guid, fields, .. } if *guid == me => fields.player_xp(),
            _ => None,
        })
        .expect("our own create carries XP")
}

struct Mob {
    guid: u64,
    position: [f32; 3],
}

fn the_mob(events: &[SessionEvent]) -> Mob {
    events
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
        .expect("a mob streams in with the world")
}
