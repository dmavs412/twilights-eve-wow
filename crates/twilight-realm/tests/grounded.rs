//! Placed units stand on the host's ground (#253): a creature, a Hub NPC and a quest GameObject
//! are created at the ground's height when the host has one, and at their written Z when the host
//! has none or their row says `grounded = false`.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::*;

/// The ground the host answers with, nowhere near any row's written Z.
const SHELF: f32 = 123.5;
/// The hand spawn in `content/mobs.toml` outside the Crossroads inn: guid 9000100, entry 3101.
const CREATURE: u64 = 0xF130 << 48 | 3101 << 24 | 9000100;
const CREATURE_WRITTEN_Z: f32 = 95.9;
/// Barg, the general goods vendor in the inn.
const BARG: u32 = 190001;
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
const GROUNDED_OBJECT: u64 = HIGHGUID_GAMEOBJECT << 48 | 990301;
const PLACED_OBJECT: u64 = HIGHGUID_GAMEOBJECT << 48 | 990302;
const OBJECT_WRITTEN_Z: f32 = 96.0;

/// Two test-only quest GameObjects in the inn: one grounded by default, one that keeps its Z.
const TEST_OBJECTS: &str = r#"
[[gameobject]]
guid = 990301
entry = 990301
display = 1
position = [-420.0, -2640.0, 96.0]

[[gameobject]]
guid = 990302
entry = 990302
display = 1
position = [-422.0, -2640.0, 96.0]
grounded = false
"#;

/// A host whose ground is one height everywhere, or none at all.
struct Shelf(Option<f32>);

impl Host for Shelf {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        self.0
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// A new orc enters the world at the Crossroads over `host`: the creates of the first ticks.
fn enter(host: &dyn Host) -> Vec<SessionEvent> {
    let mut realm = Realm::with_seed(1).with_quests(TEST_OBJECTS);
    assert_eq!(create(&mut realm, "Stander", ORC, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut realm)[0].guid;
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, host);
    let mut events = drain(&mut realm);
    realm.tick(0.1, host);
    events.extend(drain(&mut realm));
    events
}

/// Where `guid`'s create in `events` put it.
fn created_at(events: &[SessionEvent], guid: u64, kind: EntityKind) -> [f32; 3] {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g,
                kind: k,
                position,
                ..
            } if *g == guid && *k == kind => Some(*position),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{guid:#x} is created"))
}

/// A realm with the orc in the world over a host that has no ground yet (#254).
fn enter_without_ground() -> Realm {
    let mut realm = Realm::with_seed(1).with_quests(TEST_OBJECTS);
    assert_eq!(create(&mut realm, "Stander", ORC, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut realm)[0].guid;
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &Shelf(None));
    realm.tick(0.1, &Shelf(None));
    drain(&mut realm);
    realm
}

/// The height `guid`'s moves in `events` end at.
fn lifted_to(events: &[SessionEvent], guid: u64) -> Vec<f32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::MonsterMove { guid: g, path, .. } if *g == guid => {
                path.last().map(|p| p[2])
            }
            _ => None,
        })
        .collect()
}

#[test]
fn units_created_without_ground_are_lifted_once_it_streams_in() {
    let mut realm = enter_without_ground();
    realm.tick(0.1, &Shelf(Some(SHELF)));
    let events = drain(&mut realm);
    assert_eq!(lifted_to(&events, CREATURE), [SHELF]);
    assert_eq!(lifted_to(&events, hub_guid(BARG)), [SHELF]);
    // A GameObject cannot move in place: it is destroyed and created again on the ground.
    let removed = events
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectsRemoved(g) if g.contains(&GROUNDED_OBJECT)));
    assert!(removed, "the object is destroyed");
    assert_eq!(
        created_at(&events, GROUNDED_OBJECT, EntityKind::GameObject)[2],
        SHELF
    );
    // A row that keeps its Z has nothing to settle onto.
    let again = events
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectCreate { guid, .. } if *guid == PLACED_OBJECT));
    assert!(!again, "the placed object is left alone");
}

#[test]
fn a_settled_unit_is_not_asked_again() {
    let mut realm = enter_without_ground();
    realm.tick(0.1, &Shelf(Some(SHELF)));
    drain(&mut realm);
    realm.tick(0.1, &Shelf(Some(SHELF + 10.0)));
    let events = drain(&mut realm);
    assert!(lifted_to(&events, CREATURE).is_empty());
    assert!(lifted_to(&events, hub_guid(BARG)).is_empty());
    let created = events
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectCreate { guid, .. } if *guid == GROUNDED_OBJECT));
    assert!(!created, "the object is not created again");
}

#[test]
fn each_placed_unit_stands_on_the_hosts_ground() {
    let events = enter(&Shelf(Some(SHELF)));
    assert_eq!(created_at(&events, CREATURE, EntityKind::Unit)[2], SHELF);
    assert_eq!(
        created_at(&events, hub_guid(BARG), EntityKind::Unit)[2],
        SHELF
    );
    assert_eq!(
        created_at(&events, GROUNDED_OBJECT, EntityKind::GameObject)[2],
        SHELF
    );
}

#[test]
fn a_row_that_says_grounded_false_keeps_its_written_z() {
    let events = enter(&Shelf(Some(SHELF)));
    assert_eq!(
        created_at(&events, PLACED_OBJECT, EntityKind::GameObject)[2],
        OBJECT_WRITTEN_Z
    );
}

#[test]
fn with_no_ground_loaded_each_placed_unit_keeps_its_written_z() {
    let events = enter(&Shelf(None));
    assert_eq!(
        created_at(&events, CREATURE, EntityKind::Unit)[2],
        CREATURE_WRITTEN_Z
    );
    assert_eq!(
        created_at(&events, hub_guid(BARG), EntityKind::Unit)[2],
        hub_home(hub_guid(BARG))[2]
    );
    assert_eq!(
        created_at(&events, GROUNDED_OBJECT, EntityKind::GameObject)[2],
        OBJECT_WRITTEN_Z
    );
    assert_eq!(
        created_at(&events, PLACED_OBJECT, EntityKind::GameObject)[2],
        OBJECT_WRITTEN_Z
    );
}
