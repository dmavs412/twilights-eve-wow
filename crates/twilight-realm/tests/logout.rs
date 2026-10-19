//! A logout is instant (#159): one request takes the player out of the world, wherever they stand.

mod common;

use benilla_protocol::SessionEvent;
use common::*;

fn logged_out(events: &[SessionEvent]) -> bool {
    events.iter().any(|e| matches!(e, SessionEvent::LoggedOut))
}

#[test]
fn one_request_logs_out_at_once() {
    let mut realm = realm();
    enter_new(&mut realm, "Cairne", TAUREN);
    realm.handle(RealmCommand::Logout);
    let events = drain(&mut realm);
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::LogoutResponse {
            reason: 0,
            instant: true
        }
    )));
    assert!(logged_out(&events));
}

#[test]
fn away_from_an_inn_the_logout_is_still_instant() {
    let mut realm = realm();
    let (_, events) = enter_new(&mut realm, "Cairne", TAUREN);
    let spawn = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::Worldport { position, .. } => Some(*position),
            _ => None,
        })
        .expect("a login worldport");
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [spawn[0] + 300.0, spawn[1], spawn[2]],
        orientation: 0.0,
    });
    drain(&mut realm);
    realm.handle(RealmCommand::Logout);
    let events = drain(&mut realm);
    assert!(logged_out(&events));
    assert!(!events
        .iter()
        .any(|e| matches!(e, SessionEvent::LogoutResponse { instant: false, .. })));
}

#[test]
fn the_player_is_out_of_the_world_after_one_request() {
    let mut realm = realm();
    enter_new(&mut realm, "Cairne", TAUREN);
    realm.handle(RealmCommand::Logout);
    drain(&mut realm);
    // Out of the world, a second request has nothing left to log out.
    realm.handle(RealmCommand::Logout);
    assert!(!logged_out(&drain(&mut realm)));
}
