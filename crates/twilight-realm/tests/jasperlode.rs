//! Jasperlode Mine's door (#237): the first opening of the sealed door with the Jasperlode
//! Keystone completes Into Jasperlode (94401), and the keystone stays in the bags.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
const DOOR: u64 = HIGHGUID_GAMEOBJECT << 48 | 98002;
const KEYSTONE: u32 = 980013;
const INTO_JASPERLODE: u32 = 94401;
/// By the door and a few yards up the tunnel from it, in the open world.
const BY_DOOR: [f32; 3] = [-9170.0, -599.2, 62.9];
const OUTSIDE: [f32; 3] = [-9172.1, -599.4, 62.9];

/// `blob` carrying the keystone in its backpack and nothing else of its items, and Into
/// Jasperlode under way.
fn with_keystone(blob: String) -> String {
    let mut blob = blob;
    if let Some(at) = blob.find("[[items]]") {
        blob.truncate(at);
    }
    let mut blob = blob.replace("items = []\n", "");
    blob.push_str(&format!(
        "\n[[items]]\nslot = 23\nentry = {KEYSTONE}\ncount = 1\n\
         \n[[quests]]\nid = {INTO_JASPERLODE}\nslot = 0\nkills = [0, 0, 0, 0]\n"
    ));
    blob
}

#[test]
fn the_first_opening_of_the_door_completes_into_jasperlode_and_keeps_the_keystone() {
    let realm = Realm::with_seed(1);
    let mut party = Party::edited(realm, &[15], START_ELWYNN, Box::new(NoGround), |_, blob| {
        with_keystone(blob)
    });
    party.go(0, OUTSIDE, 0.0);
    party.run(0.5);
    party.go(0, BY_DOOR, 0.0);
    party.cmd(0, RealmCommand::UseGameObject { guid: DOOR });
    party.run(0.2);
    let credited = party
        .seen(0)
        .iter()
        .filter(|e| {
            matches!(
                e,
                SessionEvent::QuestObjectiveKill { quest_id, count: 1, required: 1, .. }
                    if *quest_id == INTO_JASPERLODE
            )
        })
        .count();
    assert_eq!(credited, 1, "the door's first opening counts once");
    let saved = party.realm.export_character(party.guids[0]).unwrap();
    assert!(
        saved.contains(&format!("entry = {KEYSTONE}")),
        "the keystone stayed"
    );
}
