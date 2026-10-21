//! Quest mechanics (#246): a quest with `fail_if_seen` fails when a creature enters combat with
//! the character. The quests here are test-only, added through `Realm::with_quests`, and given by
//! a Hub giver.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::{stand_at, CROSSROADS_XY};
use common::party::Party;
use common::*;
use gear_common::SEED;
use quest_common::*;
use twilight_realm::{Realm, RealmCommand};

const UNSEEN: u32 = 99301;
const SPOTTED: u32 = 99302;
const FREE: u32 = 99303;
const SEEN_TEXT: &str = "You were seen.";
/// The Monster faction template, hostile to every player race.
const MONSTER: u32 = 14;

const TEST_QUESTS: &str = r#"
[[quest]]
id = 99301
kind = "story"
giver = 90101
level = 3
title = "Unseen"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
tokens = 0
fail_if_seen = "You were seen."

[[quest]]
id = 99302
kind = "story"
giver = 90101
level = 3
title = "Spotted"
objectives = "Reach the spot."
details = "-"
progress = "-"
completion = "-"
money = 0
tokens = 0
fail_if_seen = "You were seen."
explore = [{ place = "The Test Spot", position = [-441.8, -2540.0], radius = 5.0 }]

[[quest]]
id = 99303
kind = "story"
giver = 90101
level = 3
title = "Free"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
tokens = 0
"#;

const FAILED: u8 = 2;

fn save_with(quests: &[u32]) -> (u64, String) {
    let rows: Vec<(u32, [u8; 4])> = quests.iter().map(|&q| (q, [0; 4])).collect();
    save_with_quests(10, &[], 0, &rows)
}

/// A level 10 warrior at the Hub with these test quests under way.
fn underway(quests: &[u32]) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with(quests);
    let mut realm = Realm::with_seed(SEED).with_quests(TEST_QUESTS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

/// The first hostile creature a burst creates: its guid and position.
fn hostile(events: &[SessionEvent]) -> (u64, [f32; 3]) {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } if fields.unit_faction_template() == Some(MONSTER)
                && fields.unit_npc_flags() & NPC_FLAG_QUESTGIVER == 0 =>
            {
                Some((*guid, *position))
            }
            _ => None,
        })
        .expect("a hostile creature streams in")
}

fn run(realm: &mut Realm, secs: f32) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    for _ in 0..(secs * 10.0).round() as u32 {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(realm));
    }
    events
}

/// The player stands beside `mob`, and the mob notices them.
fn get_seen(realm: &mut Realm, mob: (u64, [f32; 3])) -> Vec<SessionEvent> {
    stand_at(
        realm,
        &common::fight::Mob {
            guid: mob.0,
            position: mob.1,
        },
        2.0,
    );
    run(realm, 1.0)
}

fn failures(events: &[SessionEvent], quest: u32) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::QuestFailed { quest_id, .. } if *quest_id == quest))
        .count()
}

fn texts(events: &[SessionEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::Chat(m) if m.text == SEEN_TEXT))
        .count()
}

fn state_of(events: &[SessionEvent], me: u64, quest: u32) -> Option<u8> {
    quest_log(events, me)
        .into_iter()
        .rev()
        .find(|q| q.1 == quest)
        .map(|q| q.3)
}

#[test]
fn a_creature_aggroing_on_the_character_fails_the_quest_and_sends_the_text_once() {
    let (mut realm, me, entry) = underway(&[UNSEEN]);
    let mob = hostile(&entry);
    let mut events = get_seen(&mut realm, mob);
    events.extend(run(&mut realm, 3.0));

    assert_eq!(failures(&events, UNSEEN), 1, "{events:?}");
    assert_eq!(texts(&events), 1);
    assert_eq!(state_of(&events, me, UNSEEN), Some(FAILED));
}

#[test]
fn the_character_attacking_a_creature_fails_it_too() {
    let (mut realm, me, entry) = underway(&[UNSEEN]);
    let (guid, at) = hostile(&entry);
    // Beside the mob, then a swing in the same instant, before it can notice.
    stand_at(&mut realm, &common::fight::Mob { guid, position: at }, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid });
    let events = run(&mut realm, 0.2);

    assert_eq!(failures(&events, UNSEEN), 1, "{events:?}");
    assert_eq!(texts(&events), 1);
    assert_eq!(state_of(&events, me, UNSEEN), Some(FAILED));
}

#[test]
fn a_quest_without_the_field_never_fails_this_way() {
    let (mut realm, me, entry) = underway(&[FREE]);
    let mob = hostile(&entry);
    let events = get_seen(&mut realm, mob);

    assert_eq!(failures(&events, FREE), 0);
    assert_eq!(texts(&events), 0);
    assert_ne!(state_of(&events, me, FREE), Some(FAILED));
}

#[test]
fn a_groupmates_combat_does_not_fail_the_quest() {
    let realm = Realm::with_seed(SEED).with_quests(TEST_QUESTS);
    let mut party = Party::edited(realm, &[10, 10], 0, Box::new(FlatGround), |i, mut blob| {
        if i == 0 {
            blob.push_str(&format!(
                "\n[[quests]]\nid = {UNSEEN}\nslot = 0\nkills = [0, 0, 0, 0]\n"
            ));
        }
        blob
    });
    let creates: Vec<SessionEvent> = party.seen(1).to_vec();
    let mob = hostile(&creates);
    // Player 0 stays at the Crossroads; player 1 walks up to the creature.
    party.go(0, [CROSSROADS_XY[0], CROSSROADS_XY[1], 0.0], 0.0);
    let to = [mob.1[0] + 2.0, mob.1[1], mob.1[2]];
    party.go_facing(1, to, mob.1);
    let got = party.run(2.0);

    assert!(
        got[1].iter().any(
            |e| matches!(e, SessionEvent::AttackStart { victim, .. } if *victim == party.guids[1])
        ),
        "the creature turned on the groupmate"
    );
    assert_eq!(failures(&got[0], UNSEEN), 0);
    assert_eq!(texts(&got[0]), 0);
}

#[test]
fn a_failed_quest_gains_no_credit_and_cannot_be_handed_in() {
    let (mut realm, me, entry) = underway(&[UNSEEN, SPOTTED]);
    let mob = hostile(&entry);
    get_seen(&mut realm, mob);

    // Walking into the spot after the failure gives no credit.
    let events = walk_to(&mut realm, [-441.8, -2540.0, 0.0]);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SessionEvent::QuestObjectiveKill { quest_id, .. } if *quest_id == SPOTTED)),
        "{events:?}"
    );
    let giver = giver(&entry, GREZZ);
    let handed = hand_in(&mut realm, giver, UNSEEN);
    assert!(completed(&handed).is_none(), "{handed:?}");
    let handed = hand_in(&mut realm, giver, SPOTTED);
    assert!(completed(&handed).is_none(), "{handed:?}");
    let _ = me;
}

#[test]
fn an_abandoned_failed_quest_is_offered_again_with_no_credit() {
    let (mut realm, me, entry) = underway(&[SPOTTED]);
    let mob = hostile(&entry);
    get_seen(&mut realm, mob);
    let giver = giver(&entry, GREZZ);
    assert!(
        !on_offer(&mut realm, giver).contains(&SPOTTED),
        "still in the log"
    );

    realm.handle(RealmCommand::QuestLogRemoveQuest { slot: 0 });
    drain(&mut realm);
    assert!(on_offer(&mut realm, giver).contains(&SPOTTED));

    let taken = accept(&mut realm, giver, SPOTTED);
    assert_eq!(
        state_of(&taken, me, SPOTTED),
        Some(0),
        "taken afresh: {taken:?}"
    );
    assert_eq!(quest_log(&taken, me).last().map(|q| q.2), Some([0; 4]));
}

#[test]
fn the_failed_state_survives_a_save_and_restore() {
    let (mut realm, me, entry) = underway(&[UNSEEN]);
    let mob = hostile(&entry);
    get_seen(&mut realm, mob);
    let blob = realm.export_character(me).unwrap();

    let mut again = Realm::with_seed(SEED).with_quests(TEST_QUESTS);
    again.import_character(&blob).unwrap();
    again.handle(RealmCommand::EnterWorld { guid: me });
    again.tick(0.0, &FlatGround);
    let events = drain(&mut again);
    assert_eq!(state_of(&events, me, UNSEEN), Some(FAILED));
}
