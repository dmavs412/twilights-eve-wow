//! Milestones (#627, ADR 0034): the realm reads its state file, reaches a Milestone once, hands
//! the whole file to the host once a tick, and tells every character in the world the reached set.

mod common;

use benilla_protocol::SessionEvent;
use common::party::*;
use common::*;
use twilight_realm::{Realm, Storage};

/// 2026-11-20T21:14:03Z.
const BREAK_MS: u64 = 1_795_209_243_000;
const BREAK: &str = "2026-11-20T21:14:03Z";
const ANCHOR: &str = "ilgalar_anchor_broken";
const STEPPES: &str = "steppes_open";

fn at_the_break() -> Realm {
    let mut realm = Realm::new();
    realm.set_wall_clock(BREAK_MS);
    realm
}

/// The Milestones state files among `storage`.
fn writes(storage: Vec<Storage>) -> Vec<String> {
    storage
        .into_iter()
        .filter_map(|s| match s {
            Storage::Milestones { blob } => Some(blob),
            _ => None,
        })
        .collect()
}

/// Whether `events` hold a `SessionEvent::Milestones` naming `key`.
fn told(events: &[SessionEvent], key: &str) -> bool {
    events.iter().any(
        |e| matches!(e, SessionEvent::Milestones { reached } if reached.iter().any(|k| k == key)),
    )
}

#[test]
fn with_no_state_file_nothing_is_reached() {
    let mut realm = at_the_break();
    realm.set_milestones(None);
    assert!(!realm.milestone_reached(ANCHOR));
    assert!(!realm.milestone_reached(STEPPES));
    assert!(realm.reached_milestones().is_empty());
    assert!(realm.drain_log().is_empty());
}

#[test]
fn a_reached_table_reaches_its_milestone_only() {
    let mut realm = at_the_break();
    realm.set_milestones(Some(&format!("[{ANCHOR}]\nreached = \"{BREAK}\"\n")));
    assert!(realm.milestone_reached(ANCHOR));
    assert!(!realm.milestone_reached(STEPPES));
    assert_eq!(realm.reached_milestones(), [ANCHOR]);
    assert!(realm.drain_log().is_empty());
}

#[test]
fn an_unknown_table_or_a_bad_date_warns_once_and_reaches_nothing() {
    for state in [
        "[nope]\nreached = \"2026-11-20T21:14:03Z\"\n".to_string(),
        format!("[{ANCHOR}]\nreached = \"yesterday\"\n"),
        "[nope\n".to_string(),
    ] {
        let mut realm = at_the_break();
        realm.set_milestones(Some(&state));
        assert!(realm.reached_milestones().is_empty(), "{state}");
        let log = realm.drain_log();
        assert_eq!(log.len(), 1, "{state}: {log:?}");
    }
}

#[test]
fn a_milestone_reached_before_its_after_stays_reached_with_a_warning() {
    let mut realm = at_the_break();
    realm.set_milestones(Some(&format!("[{STEPPES}]\nreached = \"{BREAK}\"\n")));
    assert!(realm.milestone_reached(STEPPES));
    let log = realm.drain_log();
    assert_eq!(log.len(), 1, "{log:?}");
    assert!(log[0].contains(ANCHOR), "{log:?}");
}

#[test]
fn reaching_one_writes_the_file_once_with_the_wall_clocks_date() {
    let mut realm = at_the_break();
    realm.set_milestones(None);
    realm.reach_milestone(ANCHOR);
    assert!(realm.milestone_reached(ANCHOR));
    let log = realm.drain_log();
    assert_eq!(
        log,
        ["Milestone The Ilgalar Anchor is broken reached."],
        "{log:?}"
    );

    let blobs = writes(realm.drain_storage());
    assert_eq!(blobs.len(), 1, "{blobs:?}");
    assert!(
        blobs[0].contains(&format!("reached = \"{BREAK}\"")),
        "{}",
        blobs[0]
    );
    let mut again = Realm::new();
    again.set_milestones(Some(&blobs[0]));
    assert!(again.milestone_reached(ANCHOR));
    assert!(!again.milestone_reached(STEPPES));
    assert!(again.drain_log().is_empty());

    realm.reach_milestone(ANCHOR);
    realm.tick(0.1, &FlatGround);
    assert!(writes(realm.drain_storage()).is_empty());
    assert!(realm.drain_log().is_empty());
}

#[test]
fn two_reached_in_one_tick_give_one_write() {
    let mut realm = at_the_break();
    realm.reach_milestone(ANCHOR);
    realm.reach_milestone(STEPPES);
    realm.tick(0.1, &FlatGround);
    let blobs = writes(realm.drain_storage());
    assert_eq!(blobs.len(), 1, "{blobs:?}");
    assert!(blobs[0].contains(ANCHOR) && blobs[0].contains(STEPPES));
}

#[test]
fn an_unknown_key_warns_and_reaches_nothing() {
    let mut realm = at_the_break();
    realm.reach_milestone("nope");
    assert!(realm.reached_milestones().is_empty());
    assert_eq!(realm.drain_log().len(), 1);
    assert!(writes(realm.drain_storage()).is_empty());
}

#[test]
fn every_character_in_the_world_is_told() {
    let mut p = Party::new(at_the_break(), &[1, 1]);
    for i in 0..2 {
        assert!(p
            .seen(i)
            .iter()
            .any(|e| matches!(e, SessionEvent::Milestones { reached } if reached.is_empty())));
    }
    p.realm.reach_milestone(ANCHOR);
    let got = p.run(0.0);
    for events in &got {
        assert!(told(events, ANCHOR), "{events:?}");
    }
}

#[test]
fn a_character_entering_afterwards_is_told_before_its_world() {
    let mut realm = at_the_break();
    realm.reach_milestone(ANCHOR);
    let (_, events) = enter_new(&mut realm, "Mara", HUMAN);
    let told_at = events
        .iter()
        .position(|e| matches!(e, SessionEvent::Milestones { reached } if reached == &[ANCHOR]))
        .expect("told at login");
    let world_at = events
        .iter()
        .position(|e| {
            matches!(
                e,
                SessionEvent::Teleport { .. } | SessionEvent::ObjectCreate { .. }
            )
        })
        .expect("the world loads");
    assert!(told_at < world_at, "{told_at} before {world_at}");
}
