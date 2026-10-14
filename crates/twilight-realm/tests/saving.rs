//! Saving: one versioned save per character, exported by the realm and imported at boot.

mod common;

use common::*;
use twilight_realm::{SaveError, Storage};

/// The blob of the last write for `guid` among the realm's storage requests.
fn last_write(realm: &mut Realm, guid: u64) -> Option<String> {
    realm
        .drain_storage()
        .into_iter()
        .rev()
        .find_map(|op| match op {
            Storage::Write { guid: g, blob } if g == guid => Some(blob),
            _ => None,
        })
}

#[test]
fn an_exported_character_imports_into_a_fresh_realm_unchanged() {
    let mut realm = realm();
    let (guid, _) = enter_new(&mut realm, "Cairne", TAUREN);
    realm.handle(RealmCommand::Move {
        position: [-500.0, -2700.0, 91.5],
        orientation: 2.0,
    });
    let before = roster(&mut realm);
    let blob = realm.export_character(guid).expect("a save");

    let mut fresh = Realm::new();
    assert_eq!(fresh.import_character(&blob), Ok(guid));
    let after = roster(&mut fresh);

    assert_eq!(after.len(), 1);
    let (a, b) = (&before[0], &after[0]);
    assert_eq!((a.guid, &a.name, a.race, a.gender, a.level), (b.guid, &b.name, b.race, b.gender, b.level));
    assert_eq!((a.map, a.zone), (b.map, b.zone));
    assert_eq!(
        (a.position.x, a.position.y, a.position.z),
        (b.position.x, b.position.y, b.position.z)
    );
}

#[test]
fn an_imported_character_enters_the_world_where_it_was_saved() {
    let mut realm = realm();
    let (guid, _) = enter_new(&mut realm, "Baine", TAUREN);
    realm.handle(RealmCommand::Move {
        position: [-321.0, -2500.0, 93.0],
        orientation: 1.0,
    });
    let blob = realm.export_character(guid).unwrap();

    let mut fresh = Realm::new();
    fresh.import_character(&blob).unwrap();
    fresh.handle(RealmCommand::EnterWorld { guid });
    let port = drain(&mut fresh)
        .into_iter()
        .find_map(|e| match e {
            benilla_protocol::SessionEvent::Worldport { position, .. } => Some(position),
            _ => None,
        })
        .expect("a worldport");
    assert_eq!(port, [-321.0, -2500.0, 93.0]);
}

#[test]
fn a_new_character_after_an_import_gets_a_fresh_guid() {
    let mut realm = realm();
    create(&mut realm, "Aa", HUMAN, 0);
    create(&mut realm, "Bb", HUMAN, 0);
    let saved = roster(&mut realm)[1].guid;
    let blob = realm.export_character(saved).unwrap();

    let mut fresh = Realm::new();
    fresh.import_character(&blob).unwrap();
    create(&mut fresh, "Cc", HUMAN, 0);
    let guids: Vec<u64> = roster(&mut fresh).iter().map(|c| c.guid).collect();
    assert_eq!(guids.len(), 2);
    assert_ne!(guids[0], guids[1]);
}

#[test]
fn creating_a_character_writes_its_save() {
    let mut realm = realm();
    create(&mut realm, "Thrall", ORC, 0);
    let guid = roster(&mut realm)[0].guid;
    assert!(last_write(&mut realm, guid).is_some());
}

#[test]
fn logging_out_writes_the_save_with_the_last_position() {
    let mut realm = realm();
    let (guid, _) = enter_new(&mut realm, "Rokhan", ORC);
    realm.drain_storage();
    realm.handle(RealmCommand::Move {
        position: [-10.0, -20.0, 30.0],
        orientation: 0.0,
    });
    realm.handle(RealmCommand::Logout);
    let blob = last_write(&mut realm, guid).expect("a save on logout");

    let mut fresh = Realm::new();
    fresh.import_character(&blob).unwrap();
    let c = &roster(&mut fresh)[0];
    assert_eq!((c.position.x, c.position.y, c.position.z), (-10.0, -20.0, 30.0));
}

#[test]
fn the_world_saves_about_once_a_minute() {
    let mut realm = realm();
    let (guid, _) = enter_new(&mut realm, "Rokhan", ORC);
    realm.drain_storage();
    for _ in 0..50 {
        realm.tick(1.0, &FlatGround);
    }
    assert!(last_write(&mut realm, guid).is_none(), "not before a minute");
    for _ in 0..15 {
        realm.tick(1.0, &FlatGround);
    }
    assert!(last_write(&mut realm, guid).is_some(), "by a minute and a bit");
}

#[test]
fn deleting_a_character_removes_its_save() {
    let mut realm = realm();
    create(&mut realm, "Garrosh", ORC, 0);
    let guid = roster(&mut realm)[0].guid;
    realm.drain_storage();
    realm.handle(RealmCommand::DeleteCharacter { guid });
    assert!(realm
        .drain_storage()
        .iter()
        .any(|op| matches!(op, Storage::Remove { guid: g } if *g == guid)));
}

#[test]
fn garbage_is_unreadable() {
    let mut realm = realm();
    assert_eq!(
        realm.import_character("not a save"),
        Err(SaveError::Unreadable)
    );
    assert!(roster(&mut realm).is_empty());
}

#[test]
fn a_save_from_another_format_version_is_refused() {
    let mut realm = realm();
    let (guid, _) = enter_new(&mut realm, "Vol", TAUREN);
    let blob = realm.export_character(guid).unwrap();
    let future = blob.replacen("version = 1\n", "version = 999\n", 1);
    assert_ne!(blob, future, "the save names its version");

    let mut fresh = Realm::new();
    assert_eq!(
        fresh.import_character(&future),
        Err(SaveError::Version(999))
    );
}

#[test]
fn a_save_whose_name_is_taken_is_refused() {
    let mut realm = realm();
    create(&mut realm, "Zed", HUMAN, 0);
    let guid = roster(&mut realm)[0].guid;
    let blob = realm.export_character(guid).unwrap();
    assert_eq!(realm.import_character(&blob), Err(SaveError::Duplicate));
}
