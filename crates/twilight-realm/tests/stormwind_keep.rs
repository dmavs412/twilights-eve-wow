//! Stormwind Keep's sealed gate (#559): a Writ of Audience opens it once and for good, the Royal
//! Guard at the gate turns away a character without one, and Lady Katrana Prestor and Anduin
//! Wrynn stand inside the Run. The Keep is dungeon 13 on map 0 (`content/stormwind_keep.toml`).

mod common;

use benilla_protocol::messages::CHAT_MSG_MONSTER_SAY;
use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
const DOOR: u64 = HIGHGUID_GAMEOBJECT << 48 | 98003;
const DOOR_ENTRY: u32 = 98003;
const WRIT: u32 = 980122;
const SEALED: &str =
    "Halt. The Keep is closed by order of the House. No one passes without a writ of audience.";
/// The Stormwind Royal Guard at the gate, spawn 79747 (`content/stormwind.toml`).
const GUARD: u64 = 0xF130 << 48 | 1756 << 24 | 79747;
const OPEN_SECS: f32 = 15.0;
/// The gate, inside the Keep's footprint; the open-world side the refused are put on, in reach
/// of the gate; and the throne room.
const GATE: [f32; 3] = [-8535.1, 451.0, 104.92];
const OUTSIDE: [f32; 3] = [-8539.0, 456.0, 104.25];
const THRONE: [f32; 3] = [-8445.0, 338.0, 122.58];
/// The creatures of the Keep's Run: Lady Katrana Prestor, Anduin Wrynn and the Phase Tester.
const PRESTOR: u32 = 1749;
const ANDUIN: u32 = 1747;
const PHASE_TESTER: u32 = 97100;
const BACKPACK: u8 = 255;
const PACK_1: u8 = 23;

/// `blob` carrying a Writ of Audience in its backpack and nothing else of its items.
fn with_writ(blob: String) -> String {
    let mut blob = blob;
    if let Some(at) = blob.find("[[items]]") {
        blob.truncate(at);
    }
    let mut blob = blob.replace("items = []\n", "");
    blob.push_str(&format!(
        "\n[[items]]\nslot = {PACK_1}\nentry = {WRIT}\ncount = 1\n"
    ));
    blob
}

/// Level-60 players outside the gate; the first carries the writ.
fn keep(n: usize) -> Party {
    let realm = Realm::with_seed(1);
    let mut party = Party::edited(
        realm,
        &vec![60; n],
        START_ELWYNN,
        Box::new(NoGround),
        |i, blob| if i == 0 { with_writ(blob) } else { blob },
    );
    for i in 0..n {
        party.go(i, OUTSIDE, 0.0);
    }
    party.run(0.5);
    party
}

fn click(party: &mut Party, i: usize) {
    party.go(i, OUTSIDE, 0.0);
    party.cmd(i, RealmCommand::UseGameObject { guid: DOOR });
    party.run(0.2);
}

fn walk_in(party: &mut Party, i: usize) {
    party.go(i, GATE, 0.0);
    party.run(0.3);
}

fn in_phase(party: &Party, i: usize) -> bool {
    party.realm.run_elapsed_as(party.sessions[i]).is_some()
}

/// How many of player `i`'s events satisfy `f`.
fn count(party: &Party, i: usize, f: impl Fn(&SessionEvent) -> bool) -> usize {
    party.seen(i).iter().filter(|e| f(e)).count()
}

/// The guard's refusals player `i` heard.
fn guard_lines(party: &Party, i: usize) -> usize {
    count(party, i, |e| {
        matches!(
            e,
            SessionEvent::Chat(c)
                if c.chat_type == CHAT_MSG_MONSTER_SAY
                    && c.sender_guid == GUARD
                    && c.sender_name.as_deref() == Some("Stormwind Royal Guard")
                    && c.text == SEALED
        )
    })
}

fn sealed_notes(party: &Party, i: usize) -> usize {
    count(
        party,
        i,
        |e| matches!(e, SessionEvent::Notification { text } if text == SEALED),
    )
}

fn door_updates(party: &Party, i: usize) -> usize {
    count(
        party,
        i,
        |e| matches!(e, SessionEvent::ObjectValues { guid, .. } if *guid == DOOR),
    )
}

/// The entries of the units player `i`'s client holds now.
fn held_entries(party: &Party, i: usize) -> Vec<u32> {
    let held = party.units_held(i);
    unit_creates(party.seen(i))
        .into_iter()
        .filter(|(guid, _, _)| held.contains(guid))
        .map(|(_, entry, _)| entry)
        .collect()
}

fn has_writ(party: &Party, i: usize) -> bool {
    let saved = party.realm.export_character(party.guids[i]).unwrap();
    saved.contains(&format!("entry = {WRIT}"))
}

#[test]
fn a_walker_without_the_writ_is_put_outside_and_the_guard_tells_them_why() {
    let mut party = keep(2);
    assert!(party.units_held(1).contains(&GUARD), "the guard is in view");
    walk_in(&mut party, 1);
    assert!(!in_phase(&party, 1));
    let sent = party
        .seen(1)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Teleport { position, .. } => Some(*position),
            _ => None,
        })
        .last();
    assert_eq!(sent, Some(OUTSIDE), "put back outside");
    assert_eq!(guard_lines(&party, 1), 1);
    assert_eq!(
        sealed_notes(&party, 1),
        0,
        "the guard's line, not a notification"
    );
    assert_eq!(
        guard_lines(&party, 0),
        0,
        "only the refused character hears it"
    );
}

#[test]
fn a_click_without_the_writ_gets_the_same_line_and_the_gate_stays_shut() {
    let mut party = keep(2);
    click(&mut party, 1);
    assert_eq!(guard_lines(&party, 1), 1);
    assert_eq!(sealed_notes(&party, 1), 0);
    assert_eq!(door_updates(&party, 0), 0, "the gate stayed shut");
    walk_in(&mut party, 1);
    assert!(!in_phase(&party, 1));
}

#[test]
fn the_writ_opens_the_gate_and_prestor_and_anduin_stand_inside() {
    let mut party = keep(1);
    click(&mut party, 0);
    assert_eq!(guard_lines(&party, 0), 0);
    assert_eq!(door_updates(&party, 0), 1, "the gate shows open");
    walk_in(&mut party, 0);
    assert!(in_phase(&party, 0));
    party.go(0, THRONE, 0.0);
    party.run(0.5);
    assert!(in_phase(&party, 0));
    let held = held_entries(&party, 0);
    for entry in [PRESTOR, ANDUIN, PHASE_TESTER] {
        assert!(held.contains(&entry), "entry {entry} in view: {held:?}");
    }
    assert!(has_writ(&party, 0), "the writ is never taken");
}

#[test]
fn after_one_opening_the_gate_opens_again_without_the_writ() {
    let mut party = keep(1);
    click(&mut party, 0);
    walk_in(&mut party, 0);
    assert!(in_phase(&party, 0));
    party.go(0, OUTSIDE, 0.0);
    party.run(0.3);
    assert!(!in_phase(&party, 0));
    party.run(OPEN_SECS);
    assert_eq!(door_updates(&party, 0), 2, "open, then shut again");

    party.cmd(
        0,
        RealmCommand::DestroyItem {
            bag: BACKPACK,
            slot: PACK_1,
            count: 0,
        },
    );
    party.run(0.2);
    assert!(!has_writ(&party, 0), "the writ is gone");
    let saved = party.realm.export_character(party.guids[0]).unwrap();
    assert!(
        saved.contains(&format!("doors_opened = [{DOOR_ENTRY}]")),
        "{saved}"
    );

    click(&mut party, 0);
    assert_eq!(guard_lines(&party, 0), 0, "no writ needed");
    assert_eq!(door_updates(&party, 0), 3, "open again");
    walk_in(&mut party, 0);
    assert!(in_phase(&party, 0));
}
