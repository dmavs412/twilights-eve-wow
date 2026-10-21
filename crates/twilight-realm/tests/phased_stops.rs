//! Stops of a Phased Dungeon (#241): a piece of its Phase far from the building, reached through
//! an inner portal. The Test Grounds has one, "east", with a dummy standing at it.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

/// `HIGHGUID_GAMEOBJECT` and `HIGHGUID_UNIT`, in a guid's top 16 bits.
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
const HIGHGUID_UNIT: u64 = 0xF130;
/// The Test Grounds' inner portal to "east" and the return portal at the stop: the GameObject
/// entries are `0x0F000000 | dungeon << 8 | stop << 1 | side`.
const INNER: u64 = HIGHGUID_GAMEOBJECT << 48 | 0x0F00_0000 | 3 << 8;
const RETURN: u64 = INNER | 1;
/// The dummy at the stop, and one in the building.
const STOP_DUMMY: u64 = HIGHGUID_UNIT << 48 | 97001 << 24 | 790005;
const BUILDING_DUMMY: u64 = HIGHGUID_UNIT << 48 | 97001 << 24 | 790001;
/// By the inner portal in the building, in the stop by its arrival, by the return portal, past the
/// stop's edge, and in Elwynn well clear of both.
const BY_INNER: [f32; 3] = [-9655.0, 265.0, 47.2];
const BY_ARRIVAL: [f32; 3] = [-9550.0, 255.0, 47.2];
const BY_RETURN: [f32; 3] = [-9548.0, 255.0, 47.2];
const PAST_EDGE: [f32; 3] = [-9500.0, 255.0, 47.2];
const OUTSIDE: [f32; 3] = [-9720.0, 255.0, 47.2];
/// Where the inner portal puts a character back, and where the stop lands one.
const BACK: [f32; 3] = [-9658.0, 265.0, 47.2];
const ARRIVAL: [f32; 3] = [-9550.0, 255.0, 47.2];
const TEST_GROUNDS: u32 = 3;

fn party(n: usize) -> Party {
    Party::starting(realm(), &vec![5; n], START_ELWYNN, Box::new(NoGround))
}

/// The teleports player `i` was sent after the first `from` events: position and counter.
fn teleports(p: &Party, i: usize, from: usize) -> Vec<([f32; 3], u32)> {
    p.seen(i)[from..]
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Teleport {
                position, counter, ..
            } => Some((*position, *counter)),
            _ => None,
        })
        .collect()
}

/// Player `i` answers their last teleport, as the client does.
fn ack(p: &mut Party, i: usize) {
    let counter = p
        .seen(i)
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::Teleport { counter, .. } => Some(*counter),
            _ => None,
        })
        .expect("a teleport to answer");
    p.cmd(i, RealmCommand::TeleportAck { counter });
}

/// Whether player `i`'s client holds the unit `guid`.
fn holds(p: &Party, i: usize, guid: u64) -> bool {
    p.units_held(i).contains(&guid)
}

fn click(p: &mut Party, i: usize, guid: u64) -> Vec<([f32; 3], u32)> {
    let from = p.seen(i).len();
    p.cmd(i, RealmCommand::UseGameObject { guid });
    p.run(0.3);
    teleports(p, i, from)
}

#[test]
fn the_open_inner_portal_puts_the_character_at_the_stop_in_the_same_run() {
    let mut p = party(3);
    p.go(0, BY_INNER, 0.0);
    p.go(1, OUTSIDE, 0.0);
    p.go(2, [-9640.0, 250.0, 47.2], 0.0);
    p.run(0.5);
    assert!(!holds(&p, 1, STOP_DUMMY), "the stop's dummy shows outside");
    assert!(
        holds(&p, 0, BUILDING_DUMMY),
        "not in the Phase to begin with"
    );

    let got = click(&mut p, 0, INNER);
    assert_eq!(got.len(), 1, "no teleport: {got:?}");
    assert_eq!(got[0].0, ARRIVAL);
    ack(&mut p, 0);
    p.run(0.5);
    assert!(holds(&p, 0, STOP_DUMMY), "no creature at the stop");
    assert!(
        holds(&p, 0, BUILDING_DUMMY),
        "a different Run: the building's dummies are gone"
    );
    // The character in the building shares the Run, so the stop's dummy is theirs to see too.
    assert!(holds(&p, 2, STOP_DUMMY));
    assert!(holds(&p, 2, BUILDING_DUMMY));
    // And the one in Elwynn sees neither.
    assert!(!holds(&p, 1, STOP_DUMMY));
    assert!(!holds(&p, 1, BUILDING_DUMMY));
}

#[test]
fn a_dark_inner_portal_and_one_out_of_reach_do_nothing() {
    let mut p = party(1);
    p.go(0, BY_INNER, 0.0);
    p.run(0.5);
    p.realm.set_inner_portal(TEST_GROUNDS, "east", false);
    p.run(0.3);
    assert!(
        click(&mut p, 0, INNER).is_empty(),
        "a dark portal took them"
    );

    p.realm.set_inner_portal(TEST_GROUNDS, "east", true);
    p.go(0, [-9640.0, 246.0, 47.2], 0.0);
    p.run(0.3);
    assert!(
        click(&mut p, 0, INNER).is_empty(),
        "a portal out of reach took them"
    );

    p.go(0, BY_INNER, 0.0);
    p.run(0.3);
    assert_eq!(click(&mut p, 0, INNER).len(), 1, "the reopened portal");
}

#[test]
fn the_inner_portal_shows_only_while_open() {
    let mut p = party(1);
    p.go(0, BY_INNER, 0.0);
    p.run(0.5);
    let created = |p: &Party| {
        p.seen(0)
            .iter()
            .any(|e| matches!(e, SessionEvent::ObjectCreate { guid, .. } if *guid == INNER))
    };
    assert!(created(&p), "an open portal is not shown");
    let n = p.seen(0).len();
    p.realm.set_inner_portal(TEST_GROUNDS, "east", false);
    p.run(0.3);
    let removed = p.seen(0)[n..]
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectsRemoved(gone) if gone.contains(&INNER)));
    assert!(removed, "a dark portal is still shown");
}

#[test]
fn the_return_portal_puts_the_character_at_back_and_still_in_the_run() {
    let mut p = party(1);
    p.go(0, BY_INNER, 0.0);
    p.run(0.5);
    click(&mut p, 0, INNER);
    ack(&mut p, 0);
    p.go(0, BY_RETURN, 0.0);
    p.run(0.3);
    let got = click(&mut p, 0, RETURN);
    assert_eq!(got.len(), 1, "no teleport: {got:?}");
    assert_eq!(got[0].0, BACK);
    ack(&mut p, 0);
    p.run(0.5);
    assert!(
        holds(&p, 0, BUILDING_DUMMY) && holds(&p, 0, STOP_DUMMY),
        "left the Phase"
    );
}

#[test]
fn walking_off_the_stops_edge_drags_the_character_back_to_the_building() {
    let mut p = party(1);
    p.go(0, BY_ARRIVAL, 0.0);
    p.run(0.5);
    assert!(holds(&p, 0, STOP_DUMMY));
    let n = p.seen(0).len();
    p.go(0, PAST_EDGE, 0.0);
    p.run(0.3);
    let got = teleports(&p, 0, n);
    assert_eq!(got.len(), 1, "no pull: {got:?}");
    assert_eq!(got[0].0, BACK);
    let told = p.seen(0)[n..].iter().any(|e| {
        matches!(e, SessionEvent::Notification { text }
            if text == "The portal's pull drags you back to the Test Grounds.")
    });
    assert!(told, "no notification");
    ack(&mut p, 0);
    p.run(0.5);
    assert!(
        holds(&p, 0, BUILDING_DUMMY) && holds(&p, 0, STOP_DUMMY),
        "left the Phase"
    );
}

#[test]
fn a_creature_at_the_stop_is_unseen_from_outside_the_phase() {
    let mut p = party(2);
    p.go(0, BY_ARRIVAL, 0.0);
    p.go(1, [-9540.0, 300.0, 47.2], 0.0);
    p.run(0.5);
    assert!(holds(&p, 0, STOP_DUMMY));
    assert!(!holds(&p, 1, STOP_DUMMY), "seen from just outside the box");
}

#[test]
fn recalling_a_stop_sends_everyone_in_it_to_back_and_no_one_else() {
    let mut p = party(2);
    p.go(0, BY_ARRIVAL, 0.0);
    p.go(1, BY_INNER, 0.0);
    p.run(0.5);
    let from = [p.seen(0).len(), p.seen(1).len()];
    p.realm.recall_from_stop(TEST_GROUNDS, "east");
    p.run(0.3);
    let got = teleports(&p, 0, from[0]);
    assert_eq!(got.len(), 1, "no recall: {got:?}");
    assert_eq!(got[0].0, BACK);
    assert!(
        teleports(&p, 1, from[1]).is_empty(),
        "the building's one moved"
    );
}
