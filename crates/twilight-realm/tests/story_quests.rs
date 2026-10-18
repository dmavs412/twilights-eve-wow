//! Story Quests (#87): once per character, paid in XP by the Hunting Quest formula, with no
//! Token, and never forgotten by Advancement. The quest here is test-only, from a Hub giver.

mod common;
mod gear_common;
mod quest_common;

use common::*;
use quest_common::*;
use twilight_realm::rules::quest::hunting_reward_xp;

const TALE: u32 = 99201;
/// The Warrior trainer, and the Basic Warrior's Advancement Quest into Warrior.
const UZZEK: u32 = 5810;
const THE_WARRIORS_PATH: u32 = 64001;

const TEST_QUEST: &str = r#"
[[quest]]
id = 99201
kind = "story"
giver = 90101
level = 5
title = "A Tale Told Once"
objectives = "Hear Grezz out."
details = "-"
progress = "-"
completion = "-"
money = 0
tokens = 0
"#;

fn at_the_hub(level: u8, completed: &[u32]) -> (Realm, u64, Vec<benilla_protocol::SessionEvent>) {
    let (guid, blob) = save_with_quests(level, &[], 0, &[]);
    let blob = blob.replace("xp = 0\n", &format!("xp = 0\ncompleted = {completed:?}\n"));
    let mut realm = Realm::with_seed(SEED_STORY).with_quests(TEST_QUEST);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

const SEED_STORY: u64 = 3;

#[test]
fn a_story_quest_pays_the_hunting_xp_and_no_token_and_is_not_offered_again() {
    let (mut realm, _, events) = at_the_hub(5, &[]);
    let grezz = giver(&events, GREZZ);
    assert!(on_offer(&mut realm, grezz).contains(&TALE));
    accept(&mut realm, grezz, TALE);
    let done = hand_in(&mut realm, grezz, TALE);
    let c = completed(&done).expect("handed in");
    assert_eq!(c.xp, hunting_reward_xp(5, 5));
    assert_eq!(c.money, 0);
    assert!(c.items.is_empty(), "no Token: {:?}", c.items);
    assert!(!on_offer(&mut realm, grezz).contains(&TALE));
}

#[test]
fn advancement_never_forgets_a_story_quest() {
    let (mut realm, me, events) = at_the_hub(10, &[TALE]);
    let grezz = giver(&events, GREZZ);
    assert!(!on_offer(&mut realm, grezz).contains(&TALE));

    let uzzek = hub_guid(UZZEK);
    walk_up_to(&mut realm, uzzek);
    realm.handle(RealmCommand::QuestGiverQueryQuest {
        npc: uzzek,
        quest: THE_WARRIORS_PATH,
    });
    accept(&mut realm, uzzek, THE_WARRIORS_PATH);
    let advanced = hand_in(&mut realm, uzzek, THE_WARRIORS_PATH);
    assert!(completed(&advanced).is_some(), "advanced");

    assert!(!on_offer(&mut realm, grezz).contains(&TALE));
    let blob = realm.export_character(me).unwrap();
    assert!(blob.contains(&TALE.to_string()), "{blob}");
}
