//! The Eastvale and Ridgepoint quests (#247), checked on the shipped content: Toren's Pack offers
//! Toren's Trail only once both The Apprentice's Secret and Ridgepoint Falls are done.

mod common;
mod gear_common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::*;

const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
const PACK: u64 = HIGHGUID_GAMEOBJECT << 48 | 790601;
const PACK_AT: [f32; 3] = [-9776.0, -1396.0, 58.0];

const THE_APPRENTICES_SECRET: u32 = 94602;
const RIDGEPOINT_FALLS: u32 = 94703;
const TORENS_TRAIL: u32 = 94704;

struct Floor;

impl Host for Floor {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(58.0)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// A level 23 human warrior in Elwynn who has handed in the quests in `done`.
fn having_done(done: &[u32]) -> Realm {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Scout", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    let me = roster(&mut maker)[0].guid;
    let list: Vec<String> = done.iter().map(u32::to_string).collect();
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", "level = 23\n")
        .replace(
            "xp = 0\n",
            &format!("xp = 0\ncompleted = [{}]\n", list.join(", ")),
        );
    let mut realm = Realm::with_seed(1);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &Floor);
    drain(&mut realm);
    realm
}

/// The quests the Pack offers a player standing two yards east of it.
fn offered_by_the_pack(realm: &mut Realm) -> Vec<u32> {
    realm.handle(RealmCommand::DevTeleport {
        x: PACK_AT[0] + 2.0,
        y: PACK_AT[1],
        facing: None,
    });
    realm.tick(0.1, &Floor);
    drain(realm);
    realm.handle(RealmCommand::UseGameObject { guid: PACK });
    drain(realm)
        .into_iter()
        .flat_map(|e| match e {
            SessionEvent::QuestDetail(d) => vec![d.quest_id],
            SessionEvent::QuestGreeting(list) => list.quests.iter().map(|q| q.quest_id).collect(),
            _ => Vec::new(),
        })
        .collect()
}

#[test]
fn torens_trail_is_offered_only_once_both_the_secret_and_the_falls_are_done() {
    for (done, offered) in [
        (vec![], false),
        (vec![THE_APPRENTICES_SECRET], false),
        (vec![RIDGEPOINT_FALLS], false),
        (vec![THE_APPRENTICES_SECRET, RIDGEPOINT_FALLS], true),
    ] {
        let mut realm = having_done(&done);
        let quests = offered_by_the_pack(&mut realm);
        assert_eq!(
            quests.contains(&TORENS_TRAIL),
            offered,
            "after {done:?}, the Pack offers {quests:?}"
        );
    }
}
