//! Several sessions at once: each enters its own character and hears only its own events.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::*;
use twilight_realm::{RealmCommand, SessionId, Storage};

/// The events `session` drained, dropping everyone else's.
fn events_of(realm: &mut Realm, session: SessionId) -> Vec<SessionEvent> {
    realm
        .drain_session_events()
        .into_iter()
        .filter(|(s, _)| *s == session)
        .map(|(_, e)| e)
        .collect()
}

/// Creates a warrior from `session` and returns its guid.
fn create_as(realm: &mut Realm, session: SessionId, name: &str) -> u64 {
    realm.handle_as(session, create_cmd(name, ORC, 0, WARRIOR));
    realm.handle_as(session, RealmCommand::ListCharacters);
    events_of(realm, session)
        .into_iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::CharacterList { characters, .. } => {
                characters.into_iter().find(|c| c.name == name)
            }
            _ => None,
        })
        .expect("created")
        .guid
}

/// `PLAYER_XP`: only a client's own player carries it.
const PLAYER_XP: u16 = 716;

/// The players created as the client's own: with their private fields.
fn player_creates(events: &[SessionEvent]) -> Vec<u64> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Player,
                fields,
                ..
            } if fields.raw_fields().any(|(f, _)| f == PLAYER_XP) => Some(*guid),
            _ => None,
        })
        .collect()
}

#[test]
fn each_session_drains_only_its_own_events() {
    let mut realm = realm();
    let a = realm.connect();
    let b = realm.connect();
    let thrall = create_as(&mut realm, a, "Thrall");
    let rexxar = create_as(&mut realm, b, "Rexxar");

    realm.handle_as(a, RealmCommand::EnterWorld { guid: thrall });
    realm.handle_as(b, RealmCommand::EnterWorld { guid: rexxar });
    realm.tick(0.0, &FlatGround);
    let all = realm.drain_session_events();
    let of = |s| -> Vec<SessionEvent> {
        all.iter()
            .filter(|(id, _)| *id == s)
            .map(|(_, e)| e.clone())
            .collect()
    };
    assert_eq!(player_creates(&of(a)), vec![thrall]);
    assert_eq!(player_creates(&of(b)), vec![rexxar]);
    assert!(of(SessionId::DEFAULT).is_empty());
}

#[test]
fn a_character_in_the_world_cannot_be_entered_again() {
    let mut realm = realm();
    let a = realm.connect();
    let b = realm.connect();
    let thrall = create_as(&mut realm, a, "Thrall");
    realm.handle_as(a, RealmCommand::EnterWorld { guid: thrall });
    realm.drain_session_events();

    realm.handle_as(b, RealmCommand::EnterWorld { guid: thrall });
    realm.tick(0.0, &FlatGround);
    let events = events_of(&mut realm, b);
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::CharacterLoginFailed { result: 2 })));
    assert!(player_creates(&events).is_empty());
}

#[test]
fn disconnecting_saves_that_character_and_leaves_the_other_playing() {
    let mut realm = realm();
    let a = realm.connect();
    let b = realm.connect();
    let thrall = create_as(&mut realm, a, "Thrall");
    let rexxar = create_as(&mut realm, b, "Rexxar");
    realm.handle_as(a, RealmCommand::EnterWorld { guid: thrall });
    realm.handle_as(b, RealmCommand::EnterWorld { guid: rexxar });
    realm.tick(0.0, &FlatGround);
    realm.drain_session_events();
    realm.drain_storage();

    realm.disconnect(a);
    let saved: Vec<u64> = realm
        .drain_storage()
        .into_iter()
        .filter_map(|s| match s {
            Storage::Write { guid, .. } => Some(guid),
            _ => None,
        })
        .collect();
    assert_eq!(saved, vec![thrall]);

    // Rexxar still plays: a move is accepted, and Thrall can be entered again.
    realm.handle_as(b, RealmCommand::AttackStop);
    realm.tick(0.1, &FlatGround);
    let c = realm.connect();
    realm.handle_as(c, RealmCommand::EnterWorld { guid: thrall });
    realm.tick(0.0, &FlatGround);
    assert_eq!(player_creates(&events_of(&mut realm, c)), vec![thrall]);
    realm.handle_as(c, RealmCommand::EnterWorld { guid: rexxar });
    assert!(events_of(&mut realm, c)
        .iter()
        .any(|e| matches!(e, SessionEvent::CharacterLoginFailed { result: 2 })));
}
