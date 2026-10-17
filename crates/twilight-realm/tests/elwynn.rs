//! Elwynn Forest (spec #71): a second starting zone at Northshire Abbey, whose NPCs are every
//! character's friends while its hostile mobs still fight.

mod common;

use benilla_protocol::messages::AttackSwingError;
use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::*;
use twilight_realm::{CROSSROADS, KALIMDOR};

const EASTERN_KINGDOMS: u32 = 0;
/// `HIGHGUID_UNIT`, in a creature guid's top 16 bits.
const HIGHGUID_UNIT: u64 = 0xF130;

fn create_at(realm: &mut Realm, name: &str, start: u8) -> u64 {
    let RealmCommand::CreateCharacter(mut req) = create_cmd(name, ORC, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = start;
    realm.handle(RealmCommand::CreateCharacter(req));
    assert_eq!(create_result(realm), CHAR_CREATE_SUCCESS);
    roster(realm)
        .into_iter()
        .find(|c| c.name == name)
        .unwrap()
        .guid
}

fn enter(realm: &mut Realm, guid: u64) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    drain(realm)
}

fn worldport(events: &[SessionEvent]) -> (u32, [f32; 3]) {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::Worldport {
                map_id, position, ..
            } => Some((*map_id, *position)),
            _ => None,
        })
        .expect("a worldport")
}

/// The first spawn of `entry` in `content/elwynn.toml`: its unit guid and `[x, y]`.
fn spawn_of(entry: i64) -> (u64, [f32; 2]) {
    let file: toml::Table = include_str!("../content/elwynn.toml").parse().unwrap();
    let row = file["spawns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_array().unwrap())
        .find(|r| r[1].as_integer() == Some(entry))
        .unwrap();
    let guid = row[0].as_integer().unwrap() as u64;
    let unit = HIGHGUID_UNIT << 48 | (entry as u64) << 24 | guid;
    let f = |v: &toml::Value| v.as_float().unwrap() as f32;
    (unit, [f(&row[4]), f(&row[5])])
}

/// An Elwynn character standing at `xy`, and the events of the next `secs`.
fn stand_at(name: &str, xy: [f32; 2], secs: u32) -> (Realm, u64, Vec<SessionEvent>) {
    let mut realm = realm();
    let me = create_at(&mut realm, name, START_ELWYNN);
    enter(&mut realm, me);
    realm.handle(RealmCommand::DevTeleport {
        x: xy[0] + 1.0,
        y: xy[1],
        facing: None,
    });
    let mut events = Vec::new();
    for _ in 0..secs * 10 {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(&mut realm));
    }
    (realm, me, events)
}

fn attacked_by(events: &[SessionEvent], mob: u64, me: u64) -> bool {
    events.iter().any(|e| {
        matches!(e, SessionEvent::AttackStart { attacker, victim } if *attacker == mob && *victim == me)
    })
}

#[test]
fn an_elwynn_start_enters_at_northshire_abbey() {
    let mut realm = realm();
    let me = create_at(&mut realm, "Elwynner", START_ELWYNN);
    let (map, at) = worldport(&enter(&mut realm, me));
    assert_eq!(map, EASTERN_KINGDOMS);
    assert!(
        (at[0] + 8949.95).abs() < 0.1 && (at[1] + 132.493).abs() < 0.1,
        "{at:?}"
    );
}

#[test]
fn a_barrens_start_still_enters_at_the_crossroads() {
    let mut realm = realm();
    let me = create_at(&mut realm, "Barrener", 0);
    let (map, at) = worldport(&enter(&mut realm, me));
    assert_eq!(map, KALIMDOR);
    assert_eq!([at[0], at[1]], [CROSSROADS[0], CROSSROADS[1]]);
}

#[test]
fn a_northshire_guard_never_fights_and_cannot_be_attacked() {
    let (guard, at) = spawn_of(1642);
    let (mut realm, me, events) = stand_at("Friend", at, 3);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SessionEvent::ObjectCreate { guid, .. } if *guid == guard)),
        "the guard streamed in"
    );
    assert!(!attacked_by(&events, guard, me));
    realm.handle(RealmCommand::AttackSwing { guid: guard });
    let refused = drain(&mut realm).into_iter().any(|e| {
        matches!(
            e,
            SessionEvent::AttackSwingError(AttackSwingError::DeadOrUnattackable)
        )
    });
    assert!(refused, "an attack on the guard is refused");
}

#[test]
fn a_defias_thug_still_attacks() {
    let (thug, at) = spawn_of(38);
    let (_, me, events) = stand_at("Victim", at, 3);
    assert!(attacked_by(&events, thug, me), "the thug did not attack");
}
