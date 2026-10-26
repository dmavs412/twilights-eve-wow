//! Several explore spots in one quest: each listed spot is a log row of its own with a credit of
//! one, saved with the character, and the quest completes once every spot is reached. The quests
//! here are test-only, added through `Realm::with_quests`.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::SessionEvent;
use common::*;
use quest_common::*;

const THREE_SPOTS: u32 = 99301;
const ONE_SPOT: u32 = 99302;

/// Where the three spots lie, on Kalimdor, and the one lone spot.
const NORTH: [f32; 3] = [-5000.0, -3000.0, 92.0];
const EAST: [f32; 3] = [-4800.0, -3200.0, 92.0];
const SOUTH: [f32; 3] = [-5000.0, -3400.0, 92.0];
const LONE: [f32; 3] = [-4000.0, -3000.0, 92.0];

const QUESTS: &str = r#"
[[quest]]
id = 99301
kind = "story"
giver = 197
level = 5
title = "Three Overlooks"
objectives = "Look from three overlooks."
details = "-"
progress = "-"
completion = "-"
money = 0
explore = [
    { place = "North Overlook", position = [-5000.0, -3000.0], radius = 10.0 },
    { place = "East Overlook", position = [-4800.0, -3200.0], radius = 10.0 },
    { place = "South Overlook", position = [-5000.0, -3400.0], radius = 10.0 },
]

[[quest]]
id = 99302
kind = "story"
giver = 197
level = 5
title = "One Overlook"
objectives = "Look from one overlook."
details = "-"
progress = "-"
completion = "-"
money = 0
explore = { place = "Lone Overlook", position = [-4000.0, -3000.0], radius = 10.0 }
"#;

/// A realm holding the test quests with the player in the world, the quests under way.
fn playing(quests: &[(u32, [u8; 4])]) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(10, &[], 0, quests);
    let mut realm = Realm::with_seed(gear_common::SEED).with_quests(QUESTS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

fn objectives_complete(events: &[SessionEvent], quest: u32) -> bool {
    events.iter().any(
        |e| matches!(e, SessionEvent::QuestObjectivesComplete { quest_id } if *quest_id == quest),
    )
}

fn template(realm: &mut Realm, quest: u32) -> Box<benilla_protocol::messages::QuestTemplate> {
    realm.handle(RealmCommand::QuestQuery { quest });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestTemplate(t) => Some(t),
            _ => None,
        })
        .expect("the quest template")
}

#[test]
fn three_listed_spots_complete_the_quest_in_any_order_and_credit_once_each() {
    let (mut realm, me, mut events) = playing(&[(THREE_SPOTS, [0; 4])]);
    assert_eq!(quest_log(&events, me), [(0, THREE_SPOTS, [0; 4], 0)]);

    let south = walk_to(&mut realm, SOUTH);
    assert!(!objectives_complete(&south, THREE_SPOTS));
    events.extend(south);
    assert_eq!(quest_log(&events, me), [(0, THREE_SPOTS, [0, 0, 1, 0], 0)]);

    // Standing in it again gives nothing more.
    events.extend(walk_to(&mut realm, [SOUTH[0] + 2.0, SOUTH[1], 92.0]));
    assert_eq!(quest_log(&events, me), [(0, THREE_SPOTS, [0, 0, 1, 0], 0)]);

    let north = walk_to(&mut realm, NORTH);
    assert!(!objectives_complete(&north, THREE_SPOTS));
    events.extend(north);
    assert_eq!(quest_log(&events, me), [(0, THREE_SPOTS, [1, 0, 1, 0], 0)]);

    let east = walk_to(&mut realm, EAST);
    assert!(objectives_complete(&east, THREE_SPOTS));
    events.extend(east);
    assert_eq!(quest_log(&events, me), [(0, THREE_SPOTS, [1, 1, 1, 0], 1)]);
}

#[test]
fn each_listed_spot_names_its_place_in_the_quest_template() {
    let (mut realm, _, _) = playing(&[]);
    let t = template(&mut realm, THREE_SPOTS);
    let texts: Vec<&str> = t.objectives.iter().map(|o| o.text.as_str()).collect();
    assert_eq!(
        texts,
        ["North Overlook", "East Overlook", "South Overlook", ""]
    );
    let counts: Vec<u32> = t.objectives.iter().map(|o| o.required_count).collect();
    assert_eq!(counts, [1, 1, 1, 0]);
    assert!(t.objectives[..3].iter().all(|o| o.creature_or_go != 0));
    assert_eq!(t.end_text, "");
}

#[test]
fn a_single_spot_behaves_as_before() {
    let (mut realm, me, mut events) = playing(&[(ONE_SPOT, [0; 4])]);
    assert_eq!(
        template(&mut realm, ONE_SPOT).end_text,
        "Lone Overlook explored"
    );
    let inside = walk_to(&mut realm, LONE);
    assert!(objectives_complete(&inside, ONE_SPOT));
    events.extend(inside);
    assert_eq!(quest_log(&events, me), [(0, ONE_SPOT, [0; 4], 1)]);
}

#[test]
fn listed_spot_credit_stays_through_a_save() {
    let (mut realm, me, _) = playing(&[(THREE_SPOTS, [0; 4])]);
    walk_to(&mut realm, NORTH);
    walk_to(&mut realm, EAST);
    let blob = realm.export_character(me).expect("a save");

    let mut back = Realm::with_seed(gear_common::SEED).with_quests(QUESTS);
    back.import_character(&blob).expect("loads");
    back.handle(RealmCommand::EnterWorld { guid: me });
    back.tick(0.0, &FlatGround);
    let events = drain(&mut back);
    assert_eq!(quest_log(&events, me), [(0, THREE_SPOTS, [1, 1, 0, 0], 0)]);

    // The last spot, after the restore, finishes it.
    let last = walk_to(&mut back, SOUTH);
    assert!(objectives_complete(&last, THREE_SPOTS));
}

#[test]
#[should_panic(expected = "listed explore")]
fn three_listed_spots_and_two_kills_fail_validation() {
    let _ = Realm::new().with_quests(
        r#"
        [[quest]]
        id = 99399
        kind = "story"
        giver = 197
        level = 5
        title = "Too Many"
        objectives = "-"
        details = "-"
        progress = "-"
        completion = "-"
        money = 0
        kill = [[3, 1], [4, 1]]
        explore = [
            { place = "A", position = [0.0, 0.0], radius = 5.0 },
            { place = "B", position = [10.0, 0.0], radius = 5.0 },
            { place = "C", position = [20.0, 0.0], radius = 5.0 },
        ]
        "#,
    );
}
