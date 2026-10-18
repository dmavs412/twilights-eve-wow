//! The roster comes with each character's Calling key, so a client on a friends server labels
//! select and builds the talent frame without a realm of its own.

mod common;

use benilla_protocol::SessionEvent;
use common::*;

#[test]
fn the_roster_names_each_characters_calling() {
    let mut realm = realm();
    assert_eq!(create(&mut realm, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    realm.handle(RealmCommand::ListCharacters);
    let events = drain(&mut realm);
    let guid = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::CharacterList { characters, .. } => Some(characters[0].guid),
            _ => None,
        })
        .expect("the roster");
    let named = events.iter().any(|e| {
        matches!(e, SessionEvent::Callings { by_guid }
            if by_guid.iter().any(|(g, key)| *g == guid && key == "basic_warrior"))
    });
    assert!(named, "and its Calling with it");
}
