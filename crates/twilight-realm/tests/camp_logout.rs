//! The friends server's logout rule (#146): away from an inn a logout takes 20 s, is refused in
//! combat and is cancelled by moving; solo logs out at once.

mod common;

use benilla_protocol::SessionEvent;
use common::*;
use twilight_realm::CAMP_SECS;

/// Enters a new character on a camping realm and walks it out of the inn's bubble.
fn out_of_the_inn() -> (Realm, [f32; 3]) {
    let mut realm = realm().with_camp_logout();
    let (_, events) = enter_new(&mut realm, "Cairne", TAUREN);
    let spawn = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::Worldport { position, .. } => Some(*position),
            _ => None,
        })
        .expect("a login worldport");
    let away = [spawn[0] + 300.0, spawn[1], spawn[2]];
    realm.handle(RealmCommand::Move {
        position: away,
        orientation: 0.0,
    });
    drain(&mut realm);
    (realm, away)
}

fn logged_out(events: &[SessionEvent]) -> bool {
    events.iter().any(|e| matches!(e, SessionEvent::LoggedOut))
}

#[test]
fn away_from_an_inn_the_logout_waits_twenty_seconds() {
    let (mut realm, _) = out_of_the_inn();
    realm.handle(RealmCommand::Logout);
    let asked = drain(&mut realm);
    assert!(asked.iter().any(|e| matches!(
        e,
        SessionEvent::LogoutResponse {
            reason: 0,
            instant: false
        }
    )));
    assert!(!logged_out(&asked));
    let mut waited = 0.0;
    while waited < CAMP_SECS - 1.0 {
        realm.tick(0.5, &FlatGround);
        waited += 0.5;
    }
    assert!(
        !logged_out(&drain(&mut realm)),
        "not before the 20 s are up"
    );
    for _ in 0..4 {
        realm.tick(0.5, &FlatGround);
    }
    assert!(logged_out(&drain(&mut realm)));
}

#[test]
fn moving_cancels_a_pending_logout() {
    let (mut realm, at) = out_of_the_inn();
    realm.handle(RealmCommand::Logout);
    realm.handle(RealmCommand::Move {
        position: [at[0] + 2.0, at[1], at[2]],
        orientation: 0.0,
    });
    for _ in 0..50 {
        realm.tick(0.5, &FlatGround);
    }
    let events = drain(&mut realm);
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::LogoutCancelled)));
    assert!(!logged_out(&events));
}

#[test]
fn solo_logs_out_at_once() {
    let mut realm = realm();
    enter_new(&mut realm, "Cairne", TAUREN);
    realm.handle(RealmCommand::Logout);
    assert!(logged_out(&drain(&mut realm)));
}
