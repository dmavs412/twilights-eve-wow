//! Quest Starters (#88): an item that begins a quest, the vanilla 1.12 way. Its item query
//! carries `StartQuest`, using it opens its quest's offer, it stays in the bag until the quest is
//! handed in, and it drops only while its quest is neither taken nor done and none is carried.
//! The item and quest here are test-only, added through `Realm::with_quests`.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;
use quest_common::*;

const STARTER: u32 = 990001;
const QUEST: u32 = 99301;
/// The Vile Familiar near the Crossroads, which drops the starter.
const VILE_FAMILIAR: u32 = 3101;
const BACKPACK: u8 = 255;
const PACK_FIRST: u8 = 23;
/// `INVALIDREASON_QUEST_ALREADY_ON`.
const ALREADY_ON: u32 = 13;

const TEST_CONTENT: &str = r#"
item = [
  { entry = 990001, name = "Test Starter", quality = 1, item_level = 1, class = 12, display_id = 7089, max_count = 1, start_quest = 99301 },
]
starter_drop = [{ item = 990001, from = [3101], chance = 100 }]

[[quest]]
id = 99301
kind = "story"
giver = "item"
ender = 90101
level = 3
title = "A Strange Find"
objectives = "Bring the find to Grezz."
details = "-"
progress = "-"
completion = "-"
money = 0
collect = [{ item = 990001, count = 1 }]
"#;

/// Adds the test item and quest for the rest of the process.
fn seam() {
    let _ = Realm::new().with_quests(TEST_CONTENT);
}

/// A level-60 warrior at the Hub with `items` and `quests` under way, and `completed` done.
fn hero(
    items: &[Carried],
    quests: &[Underway],
    completed: &[u32],
) -> (Realm, u64, Vec<SessionEvent>) {
    seam();
    let (guid, blob) = save_with_quests(60, items, 0, quests);
    let blob = blob.replace("xp = 0\n", &format!("xp = 0\ncompleted = {completed:?}\n"));
    enter_saved(guid, &blob)
}

fn starter_guid(events: &[SessionEvent]) -> u64 {
    item_creates(events)
        .into_iter()
        .find(|(_, f)| f.object_entry() == Some(STARTER))
        .map(|(g, _)| g)
        .expect("the starter is carried")
}

fn use_starter(realm: &mut Realm) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::UseItem {
        bag: BACKPACK,
        slot: PACK_FIRST,
    });
    drain(realm)
}

/// What the familiar's corpse holds once it is killed.
fn familiar_loot(realm: &mut Realm, events: &[SessionEvent]) -> Vec<u32> {
    let mob = the_mob(events);
    assert_eq!(mob.fields.object_entry(), Some(VILE_FAMILIAR));
    kill(realm, &mob);
    realm.handle(RealmCommand::Loot { guid: mob.guid });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::LootResponse { items, .. } => {
                Some(items.into_iter().map(|i| i.item_id).collect())
            }
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn the_item_query_carries_start_quest() {
    seam();
    let mut realm = Realm::new();
    realm.handle(RealmCommand::ItemQuery { entry: STARTER });
    let start = drain(&mut realm).into_iter().find_map(|e| match e {
        SessionEvent::ItemTemplate {
            entry,
            info: Some(info),
        } if entry == STARTER => Some(info.start_quest),
        _ => None,
    });
    assert_eq!(start, Some(QUEST));
}

#[test]
fn using_it_opens_the_offer_and_accepting_keeps_it_in_the_bag() {
    let (mut realm, me, events) = hero(&[(PACK_FIRST, STARTER, 1)], &[], &[]);
    let item = starter_guid(&events);
    let detail = use_starter(&mut realm).into_iter().find_map(|e| match e {
        SessionEvent::QuestDetail(d) => Some(d),
        _ => None,
    });
    let detail = detail.expect("the offer opens");
    assert_eq!(
        (detail.npc, detail.quest_id),
        (item, QUEST),
        "the item is the giver"
    );

    realm.handle(RealmCommand::QuestGiverAcceptQuest {
        npc: item,
        quest: QUEST,
    });
    drain(&mut realm);
    let blob = realm.export_character(me).unwrap();
    assert!(
        blob.contains(&format!("id = {QUEST}")),
        "in the log: {blob}"
    );
    assert!(
        blob.contains(&format!("entry = {STARTER}")),
        "still carried: {blob}"
    );

    // Using it again says the quest is under way.
    let again = use_starter(&mut realm);
    assert!(again
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestGiverInvalid { reason } if *reason == ALREADY_ON)));
}

#[test]
fn it_drops_while_its_quest_is_neither_taken_nor_done() {
    let (mut realm, _, events) = hero(&[], &[], &[]);
    assert!(familiar_loot(&mut realm, &events).contains(&STARTER));
}

#[test]
fn it_does_not_drop_once_the_quest_is_taken() {
    let (mut realm, _, events) = hero(&[], &[(QUEST, [0; 4])], &[]);
    assert!(!familiar_loot(&mut realm, &events).contains(&STARTER));
}

#[test]
fn it_does_not_drop_once_the_quest_is_done() {
    let (mut realm, _, events) = hero(&[], &[], &[QUEST]);
    assert!(!familiar_loot(&mut realm, &events).contains(&STARTER));
}

#[test]
fn it_does_not_drop_while_one_is_carried() {
    let (mut realm, _, events) = hero(&[(PACK_FIRST, STARTER, 1)], &[], &[]);
    assert!(!familiar_loot(&mut realm, &events).contains(&STARTER));
}

#[test]
fn handing_in_to_the_ender_takes_the_item() {
    let (mut realm, me, events) = hero(&[(PACK_FIRST, STARTER, 1)], &[(QUEST, [0; 4])], &[]);
    let done = hand_in(&mut realm, giver(&events, GREZZ), QUEST);
    assert!(completed(&done).is_some(), "Grezz takes it in");
    let blob = realm.export_character(me).unwrap();
    assert!(
        !blob.contains(&format!("entry = {STARTER}")),
        "the item went: {blob}"
    );
}
