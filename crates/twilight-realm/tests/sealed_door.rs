//! A sealed door in front of a Phased Dungeon's Portal (#234), tried on the Sealed Yard
//! (`content/sealed_yard.toml`), a test dungeon whose door opens with a Crocolisk Skin.

mod common;

use benilla_protocol::{EntityKind, SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
const DOOR: u64 = HIGHGUID_GAMEOBJECT << 48 | 98001;
const DOOR_ENTRY: u32 = 98001;
const SEALED: &str = "The door is sealed. A rune-marked keystone would fit it.";
const KEY: u32 = 2925;
const OPEN_SECS: f32 = 15.0;
/// By the door, out in the open world, and inside the Yard's footprint.
const BY_DOOR: [f32; 3] = [-9667.0, 150.0, 47.2];
const OUTSIDE: [f32; 3] = [-9668.0, 150.0, 47.2];
const INSIDE: [f32; 3] = [-9650.0, 150.0, 47.2];
/// A quest that wants one use of the door.
const OPEN_THE_DOOR: u32 = 99301;

const TEST_QUESTS: &str = r#"
[[quest]]
id = 99301
kind = "story"
giver = 197
level = 5
title = "Open the Door"
objectives = "Open the sealed door."
details = "-"
progress = "-"
completion = "-"
money = 0
use = [[98001, 1]]
"#;

/// `blob` carrying a Crocolisk Skin in its backpack and nothing else of its items, and the
/// quest to open the door under way.
fn with_key(blob: String) -> String {
    let mut blob = blob;
    if let Some(at) = blob.find("[[items]]") {
        blob.truncate(at);
    }
    let mut blob = blob.replace("items = []\n", "");
    blob.push_str(&format!(
        "\n[[items]]\nslot = 23\nentry = {KEY}\ncount = 1\n\
         \n[[quests]]\nid = {OPEN_THE_DOOR}\nslot = 0\nkills = [0, 0, 0, 0]\n"
    ));
    blob
}

/// `blob` with no items.
fn without_items(blob: String) -> String {
    let mut blob = blob;
    if let Some(at) = blob.find("[[items]]") {
        blob.truncate(at);
    }
    blob.replace("items = []\n", "")
}

/// Level-5 players in the open world; the first carries the key.
fn yard(n: usize) -> Party {
    let realm = Realm::with_seed(1).with_quests(TEST_QUESTS);
    let mut party = Party::edited(
        realm,
        &vec![5; n],
        START_ELWYNN,
        Box::new(NoGround),
        |i, blob| if i == 0 { with_key(blob) } else { blob },
    );
    for i in 0..n {
        party.go(i, OUTSIDE, 0.0);
    }
    party.run(0.5);
    party
}

fn click(party: &mut Party, i: usize) {
    party.go(i, BY_DOOR, 0.0);
    party.cmd(i, RealmCommand::UseGameObject { guid: DOOR });
    party.run(0.2);
}

fn in_phase(party: &Party, i: usize) -> bool {
    party.realm.run_elapsed_as(party.sessions[i]).is_some()
}

/// How many of player `i`'s events satisfy `f`.
fn count(party: &Party, i: usize, f: impl Fn(&SessionEvent) -> bool) -> usize {
    party.seen(i).iter().filter(|e| f(e)).count()
}

fn sealed_notes(party: &Party, i: usize) -> usize {
    count(
        party,
        i,
        |e| matches!(e, SessionEvent::Notification { text } if text == SEALED),
    )
}

fn teleports(party: &Party, i: usize) -> usize {
    count(party, i, |e| matches!(e, SessionEvent::Teleport { .. }))
}

fn door_updates(party: &Party, i: usize) -> usize {
    count(
        party,
        i,
        |e| matches!(e, SessionEvent::ObjectValues { guid, .. } if *guid == DOOR),
    )
}

fn walk_in(party: &mut Party, i: usize) {
    party.go(i, INSIDE, 0.0);
    party.run(0.3);
}

#[test]
fn the_door_shows_shut_from_the_open_world() {
    let party = yard(1);
    let creates = count(&party, 0, |e| {
        matches!(
            e,
            SessionEvent::ObjectCreate { guid, kind: EntityKind::GameObject, .. } if *guid == DOOR
        )
    });
    assert_eq!(creates, 1);
    assert_eq!(door_updates(&party, 0), 0, "shut is how it is created");
}

#[test]
fn a_first_click_without_the_key_is_refused_and_nothing_opens() {
    let mut party = yard(2);
    click(&mut party, 1);
    assert_eq!(sealed_notes(&party, 1), 1);
    assert_eq!(door_updates(&party, 0), 0, "the door stayed shut");
    walk_in(&mut party, 1);
    assert!(!in_phase(&party, 1));
}

#[test]
fn a_first_click_with_the_key_opens_it_counts_the_use_and_keeps_the_key() {
    let mut party = yard(2);
    click(&mut party, 0);
    assert_eq!(sealed_notes(&party, 0), 0);
    assert_eq!(door_updates(&party, 0), 1, "the door shows open");
    assert_eq!(door_updates(&party, 1), 1, "and so to everyone who sees it");
    let credited = count(&party, 0, |e| {
        matches!(
            e,
            SessionEvent::QuestObjectiveKill { quest_id, count: 1, required: 1, .. }
                if *quest_id == OPEN_THE_DOOR
        )
    });
    assert_eq!(credited, 1);
    let saved = party.realm.export_character(party.guids[0]).unwrap();
    assert!(saved.contains(&format!("entry = {KEY}")), "the key stayed");
}

#[test]
fn while_it_is_open_anyone_walks_in_and_after_the_time_it_shuts() {
    let mut party = yard(3);
    click(&mut party, 0);
    // A stranger without the key walks in; the opener's party does not matter.
    walk_in(&mut party, 1);
    assert!(in_phase(&party, 1));
    assert_eq!(teleports(&party, 1), 0);

    party.run(OPEN_SECS);
    assert_eq!(door_updates(&party, 2), 2, "open, then shut again");
    walk_in(&mut party, 2);
    assert!(!in_phase(&party, 2), "the door is shut");
    assert_eq!(sealed_notes(&party, 2), 1);
    let sent = party
        .seen(2)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Teleport { position, .. } => Some(*position),
            _ => None,
        })
        .last();
    assert_eq!(sent, Some(OUTSIDE), "put back outside");

    // The one already inside stays, and leaves though the door is shut.
    assert!(in_phase(&party, 1));
    party.go(1, OUTSIDE, 0.0);
    party.run(0.3);
    assert!(!in_phase(&party, 1));
    assert_eq!(sealed_notes(&party, 1), 0);
}

#[test]
fn a_click_while_open_restarts_the_time() {
    let mut party = yard(2);
    click(&mut party, 0);
    party.run(10.0);
    click(&mut party, 0);
    party.run(10.0);
    walk_in(&mut party, 1);
    assert!(in_phase(&party, 1), "still open 20 s after the first click");
    assert_eq!(door_updates(&party, 1), 1, "never shut in between");
}

#[test]
fn the_door_is_world_state_and_a_run_ending_does_not_shut_it() {
    let mut party = yard(2);
    click(&mut party, 0);
    walk_in(&mut party, 0);
    assert!(in_phase(&party, 0));
    party.go(0, OUTSIDE, 0.0);
    party.run(0.3);
    // The Run reset with its last visitor; the door has its own clock.
    walk_in(&mut party, 1);
    assert!(in_phase(&party, 1));
}

#[test]
fn an_opener_reopens_without_the_key_after_a_save_and_load() {
    let mut party = yard(1);
    click(&mut party, 0);
    let saved = party.realm.export_character(party.guids[0]).unwrap();
    assert!(
        saved.contains(&format!("doors_opened = [{DOOR_ENTRY}]")),
        "{saved}"
    );

    let keyless = without_items(saved);
    let realm = Realm::with_seed(1).with_quests(TEST_QUESTS);
    let mut again = Party::edited(realm, &[5], START_ELWYNN, Box::new(NoGround), |_, _| {
        keyless.clone()
    });
    again.go(0, OUTSIDE, 0.0);
    again.run(0.5);
    click(&mut again, 0);
    assert_eq!(sealed_notes(&again, 0), 0, "no key needed");
    assert_eq!(door_updates(&again, 0), 1);
    walk_in(&mut again, 0);
    assert!(in_phase(&again, 0));
}
