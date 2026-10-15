//! Character select through the realm's public interface: create, list and delete.

mod common;

use benilla_protocol::{CharAction, SessionEvent};
use common::*;

#[test]
fn a_fresh_realm_lists_no_characters() {
    let mut realm = realm();
    assert!(roster(&mut realm).is_empty());
}

#[test]
fn a_created_basic_warrior_is_listed_at_level_1_in_the_barrens() {
    let mut realm = realm();
    let code = create(&mut realm, "thrall", ORC, 0);
    assert_eq!(code, CHAR_CREATE_SUCCESS);

    let list = roster(&mut realm);
    assert_eq!(list.len(), 1);
    let c = &list[0];
    // The server normalizes the spelling, as vmangos does.
    assert_eq!(c.name, "Thrall");
    assert_eq!((c.race, c.class, c.gender), (ORC, WARRIOR, 0));
    assert_eq!(c.level, 1);
    assert_eq!(c.map, KALIMDOR);
    assert_eq!(c.zone, THE_BARRENS);
}

#[test]
fn a_success_sends_the_fresh_roster_before_the_result() {
    let mut realm = realm();
    realm.handle(create_cmd("jaina", HUMAN, 1, WARRIOR));
    let events = drain(&mut realm);
    let list_at = events
        .iter()
        .position(|e| matches!(e, SessionEvent::CharacterList { .. }))
        .expect("a fresh roster");
    let result_at = events
        .iter()
        .position(|e| {
            matches!(
                e,
                SessionEvent::CharActionResult {
                    action: CharAction::Create,
                    ..
                }
            )
        })
        .expect("a create result");
    assert!(list_at < result_at);
}

#[test]
fn every_race_and_both_genders_can_be_created() {
    let mut realm = realm();
    let names = ["Aa", "Bb", "Cc", "Dd", "Ee", "Ff", "Gg", "Hh"];
    for (race, name) in (1..=8).zip(names) {
        let gender = race % 2;
        assert_eq!(
            create(&mut realm, name, race, gender),
            CHAR_CREATE_SUCCESS,
            "race {race}"
        );
    }
    assert_eq!(roster(&mut realm).len(), 8);
}

#[test]
fn only_the_warrior_class_can_be_created() {
    let mut realm = realm();
    realm.handle(create_cmd("Merlin", HUMAN, 0, MAGE));
    assert_eq!(create_result(&mut realm), CHAR_CREATE_DISABLED);
    assert!(roster(&mut realm).is_empty());
}

#[test]
fn a_taken_name_is_refused_whatever_its_case() {
    let mut realm = realm();
    assert_eq!(create(&mut realm, "Rexxar", ORC, 0), CHAR_CREATE_SUCCESS);
    assert_eq!(
        create(&mut realm, "REXXAR", TAUREN, 0),
        CHAR_CREATE_NAME_IN_USE
    );
}

#[test]
fn a_name_must_be_two_to_twelve_letters() {
    let mut realm = realm();
    assert_eq!(create(&mut realm, "", HUMAN, 0), CHAR_NAME_NO_NAME);
    assert_eq!(create(&mut realm, "A", HUMAN, 0), CHAR_NAME_TOO_SHORT);
    assert_eq!(
        create(&mut realm, "Abcdefghijklm", HUMAN, 0),
        CHAR_NAME_TOO_LONG
    );
    assert_eq!(
        create(&mut realm, "Ab1", HUMAN, 0),
        CHAR_NAME_INVALID_CHARACTER
    );
}

#[test]
fn the_roster_holds_at_most_ten_characters() {
    let mut realm = realm();
    for name in ["Aa", "Bb", "Cc", "Dd", "Ee", "Ff", "Gg", "Hh", "Ii", "Jj"] {
        assert_eq!(create(&mut realm, name, HUMAN, 0), CHAR_CREATE_SUCCESS);
    }
    assert_eq!(create(&mut realm, "Kk", HUMAN, 0), CHAR_CREATE_SERVER_LIMIT);
}

#[test]
fn a_deleted_character_leaves_the_roster() {
    let mut realm = realm();
    create(&mut realm, "Garrosh", ORC, 0);
    create(&mut realm, "Varian", HUMAN, 0);
    let garrosh = roster(&mut realm)[0].guid;

    realm.handle(RealmCommand::DeleteCharacter { guid: garrosh });
    let events = drain(&mut realm);
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::CharActionResult {
            action: CharAction::Delete,
            code: CHAR_DELETE_SUCCESS
        }
    )));

    let names: Vec<_> = roster(&mut realm).into_iter().map(|c| c.name).collect();
    assert_eq!(names, ["Varian"]);
}

#[test]
fn deleting_an_unknown_guid_fails() {
    let mut realm = realm();
    realm.handle(RealmCommand::DeleteCharacter { guid: 99 });
    assert!(drain(&mut realm).iter().any(|e| matches!(
        e,
        SessionEvent::CharActionResult {
            action: CharAction::Delete,
            code: CHAR_DELETE_FAILED
        }
    )));
}
