//! The world map's marks (spec #383): quest, service and Phased Dungeon marks, sent whole
//! whenever they change.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::{MapIcon, MapMark, MarkedQuest, SessionEvent, START_ELWYNN};
use common::*;
use quest_common::*;
use twilight_realm::{Realm, RealmCommand};

fn all_marks(events: &[SessionEvent]) -> Vec<Vec<MapMark>> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::MapMarks { marks } => Some(marks.clone()),
            _ => None,
        })
        .collect()
}

fn mark<'a>(marks: &'a [MapMark], name: &str) -> &'a MapMark {
    marks
        .iter()
        .find(|m| m.name == name)
        .unwrap_or_else(|| panic!("no mark named {name}"))
}

fn wolves() -> MarkedQuest {
    MarkedQuest {
        title: "Wolves at the Fold".into(),
        kind: "Bounty".into(),
        level: 3,
        monsters: Some([1, 4]),
        ready: false,
    }
}

/// A new character, entered and ticked once more, with every `MapMarks` it got.
fn new_character() -> (Realm, Vec<Vec<MapMark>>) {
    let mut realm = Realm::with_seed(1);
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Marker", 1, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    realm.handle(RealmCommand::CreateCharacter(req));
    let me = roster(&mut realm)[0].guid;
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let mut events = drain(&mut realm);
    realm.tick(0.0, &FlatGround);
    events.extend(drain(&mut realm));
    (realm, all_marks(&events))
}

#[test]
fn entering_sends_one_list_and_a_quiet_tick_sends_none() {
    let (mut realm, sent) = new_character();
    assert_eq!(sent.len(), 1);
    let eagan = mark(&sent[0], "Eagan Peltskinner");
    assert_eq!(eagan.icon, MapIcon::QuestAvailable);
    assert_eq!(eagan.map, 0);
    assert!(eagan.quests.contains(&wolves()), "{:?}", eagan.quests);

    realm.tick(0.0, &FlatGround);
    assert!(all_marks(&drain(&mut realm)).is_empty());
}

#[test]
fn an_accepted_quest_leaves_the_giver_s_list() {
    let (mut realm, sent) = new_character();
    let eagan = mark(&sent[0], "Eagan Peltskinner").clone();
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [eagan.position[0] + 1.5, eagan.position[1], 82.0],
        orientation: 0.0,
    });
    // The Abbey lawn's floor, where no monster sees the character; let the view stream him in.
    let floor = Unseen(82.0);
    for _ in 0..5 {
        realm.tick(0.1, &floor);
    }
    drain(&mut realm);
    realm.handle(RealmCommand::QuestGiverAcceptQuest {
        npc: eagan.guid,
        quest: 94005,
    });
    realm.tick(0.0, &floor);
    let sent = all_marks(&drain(&mut realm));
    let latest = sent.last().expect("the list changed");
    let still = latest
        .iter()
        .find(|m| m.name == "Eagan Peltskinner")
        .map(|m| m.quests.contains(&wolves()));
    assert_ne!(still, Some(true));
}

#[test]
fn a_quest_ready_to_hand_in_marks_its_ender_first() {
    let (_, _, events) = questing(4, &[], 0, &[(THE_FAMILIAR_THREAT, [5, 0, 0, 0])]);
    let marks = all_marks(&events).pop().expect("marks");
    let ready: Vec<&MapMark> = marks
        .iter()
        .filter(|m| m.icon == MapIcon::QuestReady)
        .collect();
    assert!(!ready.is_empty());
    let first = &ready[0].quests[0];
    assert!(first.ready);
}

#[test]
fn a_new_character_sees_the_dungeons_and_vendors_but_not_the_test_ones() {
    let (_, sent) = new_character();
    let marks = &sent[0];
    let abbey = mark(marks, "Northshire Abbey");
    assert_eq!(abbey.icon, MapIcon::Dungeon);
    assert_eq!(abbey.subtitle, "Phased Dungeon, levels 8-10");
    for name in ["Jasperlode Mine", "Azora", "Forest's Edge"] {
        assert_eq!(mark(marks, name).icon, MapIcon::Dungeon);
    }
    for name in [
        "the Test Grounds",
        "the Gaze Hall",
        "the Sealed Yard",
        "the Ward Test",
    ] {
        assert!(marks.iter().all(|m| m.name != name), "{name} is marked");
    }
    let hollis = mark(marks, "Quartermaster Hollis");
    assert_eq!(hollis.icon, MapIcon::Vendor);
    assert_eq!(hollis.subtitle, "Token Vendor");
    assert!(marks.iter().all(|m| m.name != "Brother Neals"));
    assert!(marks.iter().all(|m| m.name != "Commander Aldous Wray"));
}
