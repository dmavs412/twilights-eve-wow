//! The Farmlands' story quests (#232), checked on the shipped content: Mend the Fence waits for
//! both The Collector's Letter and Princess.

mod common;
mod gear_common;

use benilla_protocol::messages::dialog_status;
use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::*;

const HIGHGUID_UNIT: u64 = 0xF130;
const fn spawn(entry: u64, guid: u64) -> u64 {
    HIGHGUID_UNIT << 48 | entry << 24 | guid
}
const THOMAS: u64 = spawn(261, 80880);
const THOMAS_AT: [f32; 3] = [-9610.23, -1032.05, 41.3];

const THE_COLLECTORS_LETTER: u32 = 94201;
const PRINCESS: u32 = 94202;
const MEND_THE_FENCE: u32 = 94203;

struct Floor;

impl Host for Floor {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(41.0)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// A level 13 human warrior of Tier 1 in Elwynn who has handed in the quests in `done`.
fn having_done(done: &[u32]) -> Realm {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Farmhand", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    let me = roster(&mut maker)[0].guid;
    let list: Vec<String> = done.iter().map(u32::to_string).collect();
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", "level = 13\n")
        .replace("calling = \"basic_warrior\"", "calling = \"warrior\"")
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

/// The quests `npc` offers a player standing two yards east of `at`.
fn offered_by(realm: &mut Realm, at: [f32; 3], npc: u64) -> Vec<u32> {
    realm.handle(RealmCommand::DevTeleport {
        x: at[0] + 2.0,
        y: at[1],
        facing: None,
    });
    realm.tick(0.1, &Floor);
    drain(realm);
    realm.handle(RealmCommand::QuestGiverHello { npc });
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
fn mend_the_fence_is_offered_only_once_both_the_letter_and_princess_are_done() {
    let before = [94001, 94002, 94004, 94101, 94102, 94103];
    let with = |extra: &[u32]| [&before[..], extra].concat();

    for (done, offered) in [
        (with(&[]), false),
        (with(&[THE_COLLECTORS_LETTER]), false),
        (with(&[PRINCESS]), false),
        (with(&[THE_COLLECTORS_LETTER, PRINCESS]), true),
    ] {
        let mut realm = having_done(&done);
        let quests = offered_by(&mut realm, THOMAS_AT, THOMAS);
        assert_eq!(
            quests.contains(&MEND_THE_FENCE),
            offered,
            "after {done:?}, Thomas offers {quests:?}"
        );
    }
}
