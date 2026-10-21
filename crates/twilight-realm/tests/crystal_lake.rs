//! Crystal Lake and Fargodeep's story quests (#236), checked on the shipped content: handing in
//! 94303 The Keystone leaves the Jasperlode Keystone in the bags.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::START_ELWYNN;
use common::*;
use quest_common::*;

const HIGHGUID_UNIT: u64 = 0xF130;
const fn spawn(entry: u64, guid: u64) -> u64 {
    HIGHGUID_UNIT << 48 | entry << 24 | guid
}
const GORN: u64 = spawn(96013, 960203);
const GORN_AT: [f32; 3] = [-9800.0, 165.0, 40.0];

const THE_KEYSTONE: u32 = 94303;
const RUNE_MARKED_KEYSTONE: u32 = 980014;
const JASPERLODE_KEYSTONE: u32 = 980013;

struct Floor;

impl Host for Floor {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(40.0)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// A level 15 human warrior in Elwynn with The Keystone under way: Gllrg killed and the
/// Rune-marked Keystone in the backpack.
fn holding_the_rune_marked_keystone() -> Realm {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Tidebreaker", HUMAN, 0, WARRIOR)
    else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    let me = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", "level = 15\n")
        .replace("xp = 0\n", "xp = 0\ncompleted = [94301]\n");
    if let Some(at) = blob.find("[[items]]") {
        blob.truncate(at);
    }
    let mut blob = blob.replace("items = []\n", "");
    blob.push_str(&format!(
        "\n[[items]]\nslot = 23\nentry = {RUNE_MARKED_KEYSTONE}\ncount = 1\n\
         \n[[quests]]\nid = {THE_KEYSTONE}\nslot = 0\nkills = [1, 0, 0, 0]\n"
    ));
    let mut realm = Realm::with_seed(1);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &Floor);
    drain(&mut realm);
    realm
}

#[test]
fn turning_in_the_keystone_leaves_the_jasperlode_keystone_in_the_bags() {
    let mut realm = holding_the_rune_marked_keystone();
    realm.handle(RealmCommand::DevTeleport {
        x: GORN_AT[0] + 2.0,
        y: GORN_AT[1],
        facing: None,
    });
    realm.tick(0.1, &Floor);
    drain(&mut realm);

    realm.handle(RealmCommand::QuestGiverCompleteQuest {
        npc: GORN,
        quest: THE_KEYSTONE,
    });
    realm.handle(RealmCommand::QuestGiverRequestReward {
        npc: GORN,
        quest: THE_KEYSTONE,
    });
    realm.handle(RealmCommand::QuestGiverChooseReward {
        npc: GORN,
        quest: THE_KEYSTONE,
        choice: 0,
    });
    let done = completed(&drain(&mut realm)).expect("Gorn takes the Rune-marked Keystone");
    assert_eq!(done.money, 0);

    let me = roster(&mut realm)[0].guid;
    let saved = realm.export_character(me).unwrap();
    assert!(
        saved.contains(&format!("entry = {JASPERLODE_KEYSTONE}")),
        "the Jasperlode Keystone is in the bags: {saved}"
    );
    assert!(
        !saved.contains(&format!("entry = {RUNE_MARKED_KEYSTONE}")),
        "Gorn took the Rune-marked Keystone: {saved}"
    );
}
