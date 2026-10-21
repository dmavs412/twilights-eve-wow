//! Goldshire's story quests (#231), walked on the shipped content: 94004 The Abbot's Fall leads to
//! McBride's Report, then The Marshal's Doubt, then The Wrecked Cart.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::dialog_status;
use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::*;
use quest_common::*;

const HIGHGUID_UNIT: u64 = 0xF130;
const fn spawn(entry: u64, guid: u64) -> u64 {
    HIGHGUID_UNIT << 48 | entry << 24 | guid
}
const MCBRIDE: u64 = spawn(197, 79970);
const DUGHAN: u64 = spawn(240, 80334);
const FARLEY: u64 = spawn(295, 80346);
const ARGUS: u64 = spawn(514, 80336);
const REMY: u64 = spawn(241, 80326);

const MCBRIDE_AT: [f32; 3] = [-8912.0, -140.5, 82.0];
const DUGHAN_AT: [f32; 3] = [-9465.5, 74.0, 57.0];
const FARLEY_AT: [f32; 3] = [-9462.7, 16.2, 57.0];
const ARGUS_AT: [f32; 3] = [-9456.7, 87.9, 58.0];
const REMY_AT: [f32; 3] = [-9496.3, 72.8, 57.0];

const MCBRIDES_REPORT: u32 = 94101;
const THE_MARSHALS_DOUBT: u32 = 94102;
const THE_WRECKED_CART: u32 = 94103;

struct Floor;

impl Host for Floor {
    fn ground_height(&self, _: u32, x: f32, _: f32, _: f32) -> Option<f32> {
        Some(if x > -9100.0 { 82.0 } else { 57.0 })
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// A level 9 human warrior in Elwynn who has handed in The Abbot's Fall and the quests before it.
/// Level 9, because a level 10 character is at the Tier Cap and earns no XP.
fn after_the_abbot() -> Realm {
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
        .replace("level = 1\n", "level = 9\n")
        .replace("xp = 0\n", "xp = 0\ncompleted = [94001, 94002, 94004]\n");
    let mut realm = Realm::with_seed(1);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &Floor);
    drain(&mut realm);
    realm
}

/// Puts the player two yards east of `at`.
fn go(realm: &mut Realm, at: [f32; 3]) {
    realm.handle(RealmCommand::DevTeleport {
        x: at[0] + 2.0,
        y: at[1],
        facing: None,
    });
    realm.tick(0.1, &Floor);
    drain(realm);
}

fn take(realm: &mut Realm, at: [f32; 3], npc: u64, quest: u32) -> Vec<SessionEvent> {
    go(realm, at);
    realm.handle(RealmCommand::QuestGiverAcceptQuest { npc, quest });
    drain(realm)
}

fn hand_in_at(realm: &mut Realm, at: [f32; 3], npc: u64, quest: u32) -> Vec<SessionEvent> {
    go(realm, at);
    realm.handle(RealmCommand::QuestGiverCompleteQuest { npc, quest });
    realm.handle(RealmCommand::QuestGiverRequestReward { npc, quest });
    realm.handle(RealmCommand::QuestGiverChooseReward {
        npc,
        quest,
        choice: 0,
    });
    drain(realm)
}

fn offered_by(realm: &mut Realm, at: [f32; 3], npc: u64) -> Vec<u32> {
    go(realm, at);
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

/// Asks `npc` the quest's talk line: the gossip option whose text starts with `ask`.
fn ask(realm: &mut Realm, at: [f32; 3], npc: u64, ask: &str) -> Vec<SessionEvent> {
    go(realm, at);
    realm.handle(RealmCommand::GossipHello { guid: npc });
    let menu = drain(realm);
    let option = menu
        .iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => options
                .iter()
                .find(|o| o.message.starts_with(ask))
                .map(|o| o.index),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no option starting {ask:?}: {menu:?}"));
    realm.handle(RealmCommand::GossipSelectOption { guid: npc, option });
    drain(realm)
}

fn credited(events: &[SessionEvent], quest: u32) -> bool {
    events.iter().any(
        |e| matches!(e, SessionEvent::QuestObjectiveKill { quest_id, .. } if *quest_id == quest),
    )
}

#[test]
fn the_goldshire_chain_runs_from_the_abbots_fall_to_the_wrecked_cart() {
    let mut realm = after_the_abbot();

    // McBride offers the report only after The Abbot's Fall, hands it over on taking it, and
    // Dughan takes it in for XP alone.
    assert!(offered_by(&mut realm, MCBRIDE_AT, MCBRIDE).contains(&MCBRIDES_REPORT));
    let events = take(&mut realm, MCBRIDE_AT, MCBRIDE, MCBRIDES_REPORT);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SessionEvent::QuestLogFull)),
        "{events:?}"
    );
    let done = completed(&hand_in_at(&mut realm, DUGHAN_AT, DUGHAN, MCBRIDES_REPORT))
        .expect("Dughan takes the Sealed Report");
    assert_eq!(done.money, 0);
    assert!(done.xp > 0);
    assert!(!offered_by(&mut realm, MCBRIDE_AT, MCBRIDE).contains(&MCBRIDES_REPORT));

    // Dughan offers the doubt next. It is done by asking three people, not by killing.
    assert!(offered_by(&mut realm, DUGHAN_AT, DUGHAN).contains(&THE_MARSHALS_DOUBT));
    take(&mut realm, DUGHAN_AT, DUGHAN, THE_MARSHALS_DOUBT);
    assert!(completed(&hand_in_at(
        &mut realm,
        DUGHAN_AT,
        DUGHAN,
        THE_MARSHALS_DOUBT
    ))
    .is_none());
    let clue = |realm: &mut Realm, at, npc, line: &str| {
        let events = ask(realm, at, npc, line);
        assert!(credited(&events, THE_MARSHALS_DOUBT), "{events:?}");
    };
    clue(&mut realm, FARLEY_AT, FARLEY, "Have you seen a hammer");
    clue(&mut realm, ARGUS_AT, ARGUS, "Has anyone been buying ore");
    clue(&mut realm, REMY_AT, REMY, "Have you seen a hammer");
    let done = completed(&hand_in_at(
        &mut realm,
        DUGHAN_AT,
        DUGHAN,
        THE_MARSHALS_DOUBT,
    ))
    .expect("all three asked");
    assert_eq!(done.money, 0);

    // Farley offers the cart once the doubt is handed in.
    assert!(offered_by(&mut realm, FARLEY_AT, FARLEY).contains(&THE_WRECKED_CART));
}
