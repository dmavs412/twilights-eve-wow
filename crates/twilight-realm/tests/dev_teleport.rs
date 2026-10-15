//! The dev teleport (`WOW_PROBE_GOTO`): a testing aid that puts the player at a spot on the
//! current map, on the ground there, through the same teleport the Hearthstone sends.

mod common;

use benilla_protocol::SessionEvent;
use common::*;

/// Ground 42 yards up everywhere.
struct Raised;

impl Host for Raised {
    fn ground_height(&self, _map: u32, _x: f32, _y: f32, _z_hint: f32) -> Option<f32> {
        Some(42.0)
    }
    fn line_of_sight(&self, _map: u32, _from: [f32; 3], _to: [f32; 3]) -> bool {
        true
    }
}

/// No ground loaded anywhere.
struct NoGround;

impl Host for NoGround {
    fn ground_height(&self, _map: u32, _x: f32, _y: f32, _z_hint: f32) -> Option<f32> {
        None
    }
    fn line_of_sight(&self, _map: u32, _from: [f32; 3], _to: [f32; 3]) -> bool {
        true
    }
}

/// The Lushwater Oasis.
const OASIS: (f32, f32) = (-848.7, -2050.0);

fn teleports(events: &[SessionEvent]) -> Vec<([f32; 3], f32)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Teleport {
                position,
                orientation,
                ..
            } => Some((*position, *orientation)),
            _ => None,
        })
        .collect()
}

fn notification(events: &[SessionEvent]) -> Option<&str> {
    events.iter().find_map(|e| match e {
        SessionEvent::Notification { text } => Some(text.as_str()),
        _ => None,
    })
}

#[test]
fn the_dev_teleport_puts_the_player_on_the_ground_at_the_spot() {
    let mut realm = realm();
    enter_new(&mut realm, "Thrall", ORC);
    realm.handle(RealmCommand::DevTeleport {
        x: OASIS.0,
        y: OASIS.1,
        facing: Some(3.14),
    });
    realm.tick(0.1, &Raised);
    let events = drain(&mut realm);

    assert_eq!(teleports(&events), vec![([OASIS.0, OASIS.1, 42.0], 3.14)]);
}

#[test]
fn without_a_facing_the_player_keeps_its_own() {
    let mut realm = realm();
    enter_new(&mut realm, "Thrall", ORC);
    realm.handle(RealmCommand::Move {
        position: [0.0, 0.0, 0.0],
        orientation: 1.25,
    });
    realm.handle(RealmCommand::DevTeleport {
        x: OASIS.0,
        y: OASIS.1,
        facing: None,
    });
    realm.tick(0.1, &Raised);
    let events = drain(&mut realm);

    assert_eq!(teleports(&events), vec![([OASIS.0, OASIS.1, 42.0], 1.25)]);
}

#[test]
fn a_spot_with_no_ground_is_refused_and_the_player_stays() {
    let mut realm = realm();
    enter_new(&mut realm, "Thrall", ORC);
    realm.handle(RealmCommand::DevTeleport {
        x: OASIS.0,
        y: OASIS.1,
        facing: None,
    });
    realm.tick(0.1, &NoGround);
    let events = drain(&mut realm);

    assert!(teleports(&events).is_empty(), "{events:?}");
    assert!(
        notification(&events).is_some_and(|t| t.contains("no ground")),
        "the refusal is told: {events:?}"
    );

    // Refused once, it does not come back when ground does.
    realm.tick(0.1, &Raised);
    assert!(teleports(&drain(&mut realm)).is_empty());
}

#[test]
fn the_teleport_lands_once_the_client_acknowledges_it() {
    let mut realm = realm();
    enter_new(&mut realm, "Thrall", ORC);
    realm.handle(RealmCommand::DevTeleport {
        x: OASIS.0,
        y: OASIS.1,
        facing: Some(0.0),
    });
    realm.tick(0.1, &Raised);
    let counter = drain(&mut realm)
        .iter()
        .find_map(|e| match e {
            SessionEvent::Teleport { counter, .. } => Some(*counter),
            _ => None,
        })
        .expect("a teleport");

    // A report from before the ack is from where the player stood, and is ignored.
    realm.handle(RealmCommand::Move {
        position: [0.0, 0.0, 0.0],
        orientation: 0.0,
    });
    realm.handle(RealmCommand::TeleportAck { counter });
    realm.handle(RealmCommand::Logout);
    let saved = realm
        .drain_storage()
        .into_iter()
        .filter_map(|op| match op {
            twilight_realm::Storage::Write { blob, .. } => Some(blob),
            _ => None,
        })
        .last()
        .expect("logout saves");
    assert!(
        saved.contains("-848.7") && saved.contains("-2050"),
        "the save is at the oasis: {saved}"
    );
}
