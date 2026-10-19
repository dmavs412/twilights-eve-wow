//! Quests outside the Hub (#87): a Field Giver, a friendly creature out in Elwynn or inside a
//! Phased Dungeon, offers and takes in quests as a Hub NPC does, and an explore spot may lie on
//! any map. The quests here are test-only, added through `Realm::with_quests`.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::dialog_status;
use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::*;
use quest_common::*;

const HIGHGUID_UNIT: u64 = 0xF130;
/// Marshal McBride on the Abbey steps (Elwynn spawn 79970).
const MCBRIDE: u64 = HIGHGUID_UNIT << 48 | 197 << 24 | 79970;
const MCBRIDE_AT: [f32; 3] = [-8912.0, -140.5, 82.0];
/// Brother Neals inside the Abbey, a Phased Dungeon creature (content/northshire_abbey.toml).
const NEALS: u64 = HIGHGUID_UNIT << 48 | 952 << 24 | 780005;
const NEALS_AT: [f32; 3] = [-8891.8, -186.8, 82.4];
/// An explore spot in Northshire valley, on the Eastern Kingdoms.
const SPOT: [f32; 2] = [-8920.0, -120.0];

const ASK_MCBRIDE: u32 = 99101;
const FIND_NEALS: u32 = 99102;
const WALK_THE_YARD: u32 = 99103;

const TEST_QUESTS: &str = r#"
[[quest]]
id = 99101
kind = "story"
giver = 197
level = 5
title = "Ask the Marshal"
objectives = "Speak with Marshal McBride."
details = "-"
progress = "-"
completion = "-"
money = 0
tokens = 0

[[quest]]
id = 99102
kind = "story"
giver = 197
ender = 952
level = 5
title = "Find the Brother"
objectives = "Find Brother Neals inside the Abbey."
details = "-"
progress = "-"
completion = "-"
money = 0
tokens = 0

[[quest]]
id = 99103
kind = "story"
giver = 197
level = 5
title = "Walk the Yard"
objectives = "Walk the Abbey yard."
details = "-"
progress = "-"
completion = "-"
money = 0
tokens = 0
explore = { place = "The Abbey Yard", position = [-8920.0, -120.0], radius = 5.0, map = 0 }
"#;

/// The Abbey's floor everywhere, so every spot here stands at its height.
struct AbbeyFloor;

impl Host for AbbeyFloor {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(82.0)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// A level-10 warrior who began in Northshire, in the world beside McBride, with the test quests.
fn in_northshire() -> (Realm, u64) {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Pilgrim", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", "level = 10\n");
    let mut realm = Realm::with_seed(1).with_quests(TEST_QUESTS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    go(&mut realm, MCBRIDE_AT);
    (realm, me)
}

/// Moves the player 2 yd east of `at` and lets the world catch up.
fn go(realm: &mut Realm, at: [f32; 3]) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [at[0] + 2.0, at[1], at[2]],
        orientation: std::f32::consts::PI,
    });
    let mut events = Vec::new();
    for _ in 0..5 {
        realm.tick(0.1, &AbbeyFloor);
        events.extend(drain(realm));
    }
    events
}

fn status_of(realm: &mut Realm, npc: u64) -> u32 {
    realm.handle(RealmCommand::QuestGiverStatusQuery { npc });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestGiverStatus { npc: n, status } if n == npc => Some(status),
            _ => None,
        })
        .unwrap_or(dialog_status::NONE)
}

fn offered_by(realm: &mut Realm, npc: u64) -> Vec<u32> {
    realm.handle(RealmCommand::QuestGiverHello { npc });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestGreeting(list) => Some(
                list.quests
                    .iter()
                    .filter(|q| q.icon == dialog_status::AVAILABLE)
                    .map(|q| q.quest_id)
                    .collect(),
            ),
            SessionEvent::QuestDetail(d) => Some(vec![d.quest_id]),
            _ => None,
        })
        .unwrap_or_default()
}

fn take(realm: &mut Realm, npc: u64, quest: u32) {
    realm.handle(RealmCommand::QuestGiverAcceptQuest { npc, quest });
    drain(realm);
}

fn turn_in(realm: &mut Realm, npc: u64, quest: u32) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::QuestGiverCompleteQuest { npc, quest });
    realm.handle(RealmCommand::QuestGiverRequestReward { npc, quest });
    realm.handle(RealmCommand::QuestGiverChooseReward {
        npc,
        quest,
        choice: 0,
    });
    drain(realm)
}

#[test]
fn an_elwynn_friendly_spawn_shows_the_quest_giver_flag_and_offers_its_quests() {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Pilgrim", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    let me = roster(&mut maker)[0].guid;
    let blob = maker.export_character(me).unwrap();
    let mut realm = Realm::with_seed(1).with_quests(TEST_QUESTS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    let events = go(&mut realm, MCBRIDE_AT);
    let flags = events.iter().find_map(|e| match e {
        SessionEvent::ObjectCreate { guid, fields, .. } if *guid == MCBRIDE => {
            Some(fields.unit_npc_flags())
        }
        _ => None,
    });
    assert_eq!(
        flags,
        Some(NPC_FLAG_QUESTGIVER),
        "McBride streams in a quest giver"
    );

    assert_eq!(status_of(&mut realm, MCBRIDE), dialog_status::AVAILABLE);
    let offered = offered_by(&mut realm, MCBRIDE);
    assert!(
        [ASK_MCBRIDE, FIND_NEALS, WALK_THE_YARD]
            .iter()
            .all(|q| offered.contains(q)),
        "{offered:?}"
    );
    take(&mut realm, MCBRIDE, ASK_MCBRIDE);
    let blob = realm.export_character(me).unwrap();
    assert!(blob.contains(&format!("id = {ASK_MCBRIDE}")), "{blob}");
}

#[test]
fn a_field_giver_out_of_reach_offers_nothing() {
    let (mut realm, _) = in_northshire();
    go(
        &mut realm,
        [MCBRIDE_AT[0] + 30.0, MCBRIDE_AT[1], MCBRIDE_AT[2]],
    );
    assert!(offered_by(&mut realm, MCBRIDE).is_empty());
}

#[test]
fn a_quest_whose_ender_is_a_phased_dungeon_creature_is_handed_in_inside() {
    let (mut realm, me) = in_northshire();
    take(&mut realm, MCBRIDE, FIND_NEALS);
    // McBride does not take it in.
    let at_mcbride = turn_in(&mut realm, MCBRIDE, FIND_NEALS);
    assert!(completed(&at_mcbride).is_none());

    go(&mut realm, NEALS_AT);
    assert_eq!(status_of(&mut realm, NEALS), dialog_status::REWARD2);
    let done = turn_in(&mut realm, NEALS, FIND_NEALS);
    let c = completed(&done).expect("Neals takes it in");
    assert_eq!(c.quest_id, FIND_NEALS);
    let blob = realm.export_character(me).unwrap();
    assert!(
        blob.contains(&format!("completed = [{FIND_NEALS}]")),
        "{blob}"
    );
}

#[test]
fn an_explore_spot_on_the_eastern_kingdoms_is_met_there_and_not_on_kalimdor() {
    // On Kalimdor at the same x, y: not met.
    let (guid, blob) = save_with_quests(10, &[], 0, &[(WALK_THE_YARD, [0; 4])]);
    let mut realm = Realm::with_seed(1).with_quests(TEST_QUESTS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [SPOT[0], SPOT[1], 0.0],
        orientation: 0.0,
    });
    realm.tick(0.1, &FlatGround);
    let blob = realm.export_character(guid).unwrap();
    assert!(!blob.contains("explored = true"), "{blob}");

    // In Northshire: met.
    let (mut realm, me) = in_northshire();
    take(&mut realm, MCBRIDE, WALK_THE_YARD);
    go(&mut realm, [SPOT[0] - 2.0, SPOT[1], 82.0]);
    let blob = realm.export_character(me).unwrap();
    assert!(blob.contains("explored = true"), "{blob}");
}

#[test]
#[should_panic(expected = "no friendly spawn anywhere")]
fn a_giver_with_no_spawn_anywhere_fails_the_content_load() {
    let _ = Realm::new().with_quests(
        r#"
        [[quest]]
        id = 99199
        kind = "story"
        giver = 99999
        level = 5
        title = "Nobody"
        objectives = "-"
        details = "-"
        progress = "-"
        completion = "-"
        money = 0
        tokens = 0
        "#,
    );
}
