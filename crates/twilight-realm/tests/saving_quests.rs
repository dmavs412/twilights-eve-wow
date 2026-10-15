//! The quest log and Tokens in the character save.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;
use quest_common::*;
use twilight_realm::Storage;

/// The last save the realm asked to write.
fn last_write(realm: &mut Realm) -> String {
    realm
        .drain_storage()
        .into_iter()
        .rev()
        .find_map(|op| match op {
            Storage::Write { blob, .. } => Some(blob),
            Storage::Remove { .. } => None,
        })
        .expect("a save was written")
}

/// `blob` loaded into a fresh realm and logged in: the entry burst.
fn reload(me: u64, blob: &str) -> Vec<SessionEvent> {
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm)
}

#[test]
fn accepting_a_quest_saves_it_and_it_is_in_the_log_after_a_reload() {
    let (mut realm, me, events) = warrior_with(1, 0, &[]);
    let grezz = giver(&events, GREZZ);
    realm.drain_storage();
    accept(&mut realm, grezz, SCORPID_VENOM);
    accept(&mut realm, grezz, THINNING_THE_HERDS);
    let blob = last_write(&mut realm);

    let events = reload(me, &blob);
    assert_eq!(
        quest_log(&events, me),
        [
            (0, SCORPID_VENOM, [0; 4], 0),
            (1, THINNING_THE_HERDS, [0; 4], 0)
        ]
    );
}

#[test]
fn kill_progress_and_tokens_survive_a_save_round_trip() {
    let (realm, me, events) = questing(5, &[], 7, &[(THINNING_THE_HERDS, [3, 1, 0, 0])]);
    assert_eq!(
        quest_log(&events, me),
        [(0, THINNING_THE_HERDS, [3, 1, 0, 0], 0)]
    );
    let blob = realm.export_character(me).unwrap();
    assert!(blob.contains("tokens = 7"), "{blob}");
    let again = reload(me, &blob);
    assert_eq!(quest_log(&again, me), quest_log(&events, me));
    assert_eq!(realm.export_character(me).unwrap(), blob);
}

#[test]
fn a_save_from_before_quests_loads_with_none() {
    let (me, blob) = save_with(3, 0, &[]);
    let old = blob.replace("tokens = 0\n", "");
    let events = reload(me, &old);
    assert!(quest_log(&events, me).is_empty());
}

#[test]
fn a_quest_the_realm_no_longer_has_is_dropped_from_the_save() {
    let (me, blob) = save_with_quests(3, &[], 0, &[(99999, [0; 4]), (SCORPID_VENOM, [0; 4])]);
    let events = reload(me, &blob);
    assert_eq!(quest_log(&events, me), [(1, SCORPID_VENOM, [0; 4], 0)]);
}

#[test]
fn a_quest_log_slot_named_twice_makes_the_save_unreadable() {
    let (_, blob) = save_with_quests(3, &[], 0, &[(SCORPID_VENOM, [0; 4])]);
    let twice =
        format!("{blob}\n[[quests]]\nid = {THINNING_THE_HERDS}\nslot = 0\nkills = [0, 0, 0, 0]\n");
    assert_eq!(
        Realm::new().import_character(&twice),
        Err(twilight_realm::SaveError::Unreadable)
    );
}
