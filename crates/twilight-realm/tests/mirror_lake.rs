//! The Mirror Lake and Jerod's Landing quests (#248), checked on the shipped content: Marshal
//! Dughan offers The Smugglers' Ledger only once both Remy's Last Debtor and Ridgepoint Falls are
//! done.

mod common;
mod gear_common;

use benilla_protocol::messages::dialog_status;
use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::*;

const HIGHGUID_UNIT: u64 = 0xF130;
const DUGHAN: u64 = HIGHGUID_UNIT << 48 | 240 << 24 | 80334;
const DUGHAN_AT: [f32; 3] = [-9465.5, 74.0, 57.0];

const REMYS_LAST_DEBTOR: u32 = 94801;
const THE_SMUGGLERS_LEDGER: u32 = 94802;
const RIDGEPOINT_FALLS: u32 = 94703;

struct Floor;

impl Host for Floor {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(57.0)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// A level 24 human warrior in Elwynn who has handed in the quests in `done`.
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
        .replace("level = 1\n", "level = 24\n")
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

/// The quests Dughan offers a player standing two yards east of him.
fn offered_by_dughan(realm: &mut Realm) -> Vec<u32> {
    realm.handle(RealmCommand::DevTeleport {
        x: DUGHAN_AT[0] + 2.0,
        y: DUGHAN_AT[1],
        facing: None,
    });
    realm.tick(0.1, &Floor);
    drain(realm);
    realm.handle(RealmCommand::QuestGiverHello { npc: DUGHAN });
    drain(realm)
        .into_iter()
        .flat_map(|e| match e {
            SessionEvent::QuestGreeting(list) => list
                .quests
                .iter()
                .filter(|q| q.icon == dialog_status::AVAILABLE)
                .map(|q| q.quest_id)
                .collect(),
            SessionEvent::QuestDetail(d) => vec![d.quest_id],
            _ => Vec::new(),
        })
        .collect()
}

#[test]
fn the_smugglers_ledger_is_offered_only_once_the_debtor_and_the_falls_are_done() {
    // 94801 itself needs The Marshal's Doubt (94102), which needs the quests before it.
    let before = [94001, 94002, 94004, 94101, 94102];
    for (extra, offered) in [
        (vec![], false),
        (vec![REMYS_LAST_DEBTOR], false),
        (vec![RIDGEPOINT_FALLS], false),
        (vec![REMYS_LAST_DEBTOR, RIDGEPOINT_FALLS], true),
    ] {
        let done: Vec<u32> = before
            .iter()
            .copied()
            .chain(extra.iter().copied())
            .collect();
        let mut realm = having_done(&done);
        let quests = offered_by_dughan(&mut realm);
        assert_eq!(
            quests.contains(&THE_SMUGGLERS_LEDGER),
            offered,
            "after {extra:?}, Dughan offers {quests:?}"
        );
    }
}
