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
    walk_up_to(&mut realm, grezz);
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
    let again = reload(me, &blob);
    assert_eq!(carried(&again, me, TOKEN), 7);
    assert_eq!(quest_log(&again, me), quest_log(&events, me));
    assert_eq!(realm.export_character(me).unwrap(), blob);
}

#[test]
fn a_hand_in_saves_the_copper_and_the_emptied_log() {
    let skins: &[Carried] = &[(23, CROCOLISK_SKIN, 8)];
    let (mut realm, me, events) = questing(10, skins, 4, &[(CROCOLISK_SKINS, [0; 4])]);
    realm.drain_storage();
    hand_in(&mut realm, giver(&events, ZUNJIN), CROCOLISK_SKINS);
    let blob = last_write(&mut realm);
    assert!(!blob.contains("[[quests]]"), "{blob}");
    let again = reload(me, &blob);
    assert_eq!(descriptor(&again, me).player_money(), Some(500));
    // The old Tokens in the bags stay as they were: the quest pays none.
    assert_eq!(carried(&again, me, TOKEN), 4);
    assert!(quest_log(&again, me).is_empty());
}

#[test]
fn a_tokens_count_beside_the_money_is_no_part_of_the_save() {
    // Only builds of the integration branch ever wrote one; Tokens are carried in the bags.
    let (me, blob) = save_with(3, 0, &[(23, CROCOLISK_SKIN, 2)]);
    let old = blob.replace(
        "money = 0
",
        "money = 0
tokens = 4
",
    );
    let events = reload(me, &old);
    assert_eq!(carried(&events, me, TOKEN), 0);
    assert_eq!(carried(&events, me, CROCOLISK_SKIN), 2);
}

#[test]
fn abandoning_a_quest_clears_its_slot_and_saves() {
    let (mut realm, me, mut events) = questing(
        3,
        &[],
        0,
        &[(SCORPID_VENOM, [0; 4]), (THINNING_THE_HERDS, [2, 0, 0, 0])],
    );
    realm.drain_storage();
    realm.handle(RealmCommand::QuestLogRemoveQuest { slot: 0 });
    events.extend(drain(&mut realm));
    assert_eq!(
        quest_log(&events, me),
        [(1, THINNING_THE_HERDS, [2, 0, 0, 0], 0)]
    );
    let blob = last_write(&mut realm);
    assert_eq!(
        quest_log(&reload(me, &blob), me),
        [(1, THINNING_THE_HERDS, [2, 0, 0, 0], 0)]
    );
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
    let blob = blob.replace(
        "money = 0
",
        &format!(
            "money = 0
completed = [99998, {THINNING_THE_HERDS}]
"
        ),
    );
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    let log = realm.drain_log();
    assert!(
        log.iter().any(|l| l.contains("99999")) && log.iter().any(|l| l.contains("99998")),
        "each dropped quest is logged: {log:?}"
    );
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    assert_eq!(quest_log(&events, me), [(1, SCORPID_VENOM, [0; 4], 0)]);
    let saved = realm.export_character(me).unwrap();
    assert!(
        saved.contains(&format!(
            "completed = [{THINNING_THE_HERDS}]
"
        )),
        "{saved}"
    );
}

#[test]
fn the_calling_choice_pick_survives_a_save_and_an_old_save_has_not_chosen() {
    let (me, blob) = save_with_quests(10, &[], 0, &[]);
    assert!(!blob.contains("calling_chosen"), "{blob}");
    let chosen = blob.replacen("\nlevel = 10\n", "\nlevel = 10\ncalling_chosen = true\n", 1);
    let mut realm = Realm::new();
    realm.import_character(&chosen).unwrap();
    let saved = realm.export_character(me).unwrap();
    assert!(saved.contains("calling_chosen = true\n"), "{saved}");
    let mut old = Realm::new();
    old.import_character(&blob).unwrap();
    assert!(!old.export_character(me).unwrap().contains("calling_chosen"));
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
