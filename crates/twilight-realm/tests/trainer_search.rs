//! A Tier 2 Advancement Quest's search and trial (#676), played through the realm's seam on a
//! fixture: the Hidden Tester (`content/hub.toml`, `hidden = true`, in the woods west of the Test
//! Grounds) and two quests only these tests add. The map marks a Hidden Trainer once it is met,
//! Lyria Du Lac gives a hint while the quest is held, and the Advancement waits on its Trial.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::{dialog_status, GossipOption};
use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;
use quest_common::*;

/// The Hidden Tester's entry, and Lyria Du Lac's, a gossiping Hub NPC in Goldshire.
const TESTER: u32 = 199001;
const LYRIA: u32 = 913;
/// The test Advancement Quest, from Vanguard to Zealot, and its Trial's one quest.
const ADVANCEMENT: u32 = 99601;
const TRIAL: u32 = 99602;
const HINT_LINE: &str = "Have you heard of a tester in the woods?";
const HINT_REPLY: &str = "West of the Test Grounds, in the trees. Odd sort.";
const PROGRESS: &str = "Not until the trial is done.";
/// The first talk line's index in a gossip menu (`quests::TALK_OPTION`).
const TALK_OPTION: u32 = 100;

/// The fixture's quests. Only these tests add them; no character in play is offered them.
const FIXTURE: &str = r#"
[[quest]]
id = 99601
kind = "advancement"
giver = 199001
level = 30
title = "The Tester's Vow"
objectives = "Pass the Hidden Tester's trial, then speak with the Hidden Tester."
details = "So you found me. Finding is half of it."
progress = "Not until the trial is done."
completion = "Done, then. Go on."
money = 0
hint = [{ npc = 913, option = "Have you heard of a tester in the woods?", reply = "West of the Test Grounds, in the trees. Odd sort." }]
advancement = { from = "vanguard", to = "zealot", trial = 99602 }

[[quest]]
id = 99602
kind = "story"
giver = 199001
after_taken = 99601
level = 30
min_tier = 1
title = "The Tester's Trial"
objectives = "Tell the Hidden Tester you are ready."
details = "Say the word when you are ready."
progress = "Well?"
completion = "Good."
money = 0
talk = [{ npc = 199001, option = "I am ready.", reply = "Then it is done." }]
"#;

/// A save of `calling` at level 30 standing beside Lyria in Goldshire, with `quests` under way
/// and `done` handed in, entered in a realm that has the fixture's quests.
fn enter(calling: &str, quests: &[Underway], done: &[u32]) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(30, &[], 0, quests);
    let [x, y, z] = hub_home(hub_guid(LYRIA));
    let (head, tail) = blob.split_once("\n[").unwrap_or((&blob, ""));
    let head: Vec<String> = head
        .lines()
        .map(|line| {
            if line.starts_with("map = ") {
                "map = 0".to_string()
            } else if line.starts_with("zone = ") {
                "zone = 12".to_string()
            } else if line.starts_with("position = [") {
                format!("position = [{}, {y:?}, {z:?}]", x + 1.5)
            } else if line == "xp = 0" && !done.is_empty() {
                format!("xp = 0\ncompleted = {done:?}")
            } else {
                line.to_string()
            }
        })
        .collect();
    let mut blob = head.join("\n").replace(
        "calling = \"basic_warrior\"",
        &format!("calling = \"{calling}\""),
    );
    if !tail.is_empty() {
        blob = format!("{blob}\n[{tail}");
    }
    let mut realm = Realm::with_seed(SEED).with_quests(FIXTURE);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

/// Whether the last `MapMarks` in `events` marks `npc`.
fn marked(events: &[SessionEvent], npc: u64) -> bool {
    let last = events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::MapMarks { marks } => Some(marks),
            _ => None,
        })
        .expect("a map mark list");
    last.iter().any(|m| m.guid == npc)
}

/// Walks up to `npc` and opens its gossip menu: its lines.
fn gossip(realm: &mut Realm, npc: u64) -> Vec<GossipOption> {
    walk_up_to(realm, npc);
    realm.handle(RealmCommand::GossipHello { guid: npc });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => Some(options),
            _ => None,
        })
        .expect("a gossip menu")
}

/// Chooses gossip line `option` of `npc`: the text the reply shows.
fn choose(realm: &mut Realm, npc: u64, option: u32) -> String {
    realm.handle(RealmCommand::GossipSelectOption { guid: npc, option });
    let text_id = drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { text_id, .. } => Some(text_id),
            _ => None,
        })
        .expect("the reply");
    realm.handle(RealmCommand::NpcTextQuery { text_id });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::NpcGreeting { blocks, .. } => Some(blocks[0].male.clone()),
            _ => None,
        })
        .expect("the reply's text")
}

/// The Hidden Tester's quest list: `(quest, icon)` of each row.
fn quest_list(realm: &mut Realm, npc: u64) -> Vec<(u32, u32)> {
    talk(realm, npc)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestGreeting(list) => {
                Some(list.quests.iter().map(|q| (q.quest_id, q.icon)).collect())
            }
            _ => None,
        })
        .expect("a quest list")
}

#[test]
fn the_hidden_trainer_is_marked_once_met_and_the_save_keeps_the_meeting() {
    let (mut realm, me, events) = enter("vanguard", &[(ADVANCEMENT, [0; 4])], &[]);
    let tester = hub_guid(TESTER);
    assert!(!marked(&events, tester), "not met yet");
    assert!(!realm.export_character(me).unwrap().contains("met = "));

    gossip(&mut realm, tester);
    realm.tick(0.0, &FlatGround);
    assert!(marked(&drain(&mut realm), tester), "met at its gossip menu");
    let blob = realm.export_character(me).unwrap();
    assert!(blob.contains(&format!("met = [{TESTER}]\n")), "{blob}");

    let mut reloaded = Realm::with_seed(SEED).with_quests(FIXTURE);
    reloaded.import_character(&blob).unwrap();
    reloaded.handle(RealmCommand::EnterWorld { guid: me });
    reloaded.tick(0.0, &FlatGround);
    assert!(marked(&drain(&mut reloaded), tester), "still met");
    assert!(reloaded
        .export_character(me)
        .unwrap()
        .contains(&format!("met = [{TESTER}]\n")));
}

#[test]
fn opening_the_hidden_trainers_quest_list_meets_it_too() {
    let (mut realm, me, _) = enter("vanguard", &[(ADVANCEMENT, [0; 4])], &[]);
    let tester = hub_guid(TESTER);
    talk(&mut realm, tester);
    realm.tick(0.0, &FlatGround);
    assert!(marked(&drain(&mut realm), tester));
    assert!(realm
        .export_character(me)
        .unwrap()
        .contains(&format!("met = [{TESTER}]\n")));
}

#[test]
fn lyria_gives_the_hint_only_while_the_quest_is_held() {
    let lyria = hub_guid(LYRIA);
    let hint = |options: &[GossipOption]| options.iter().find(|o| o.message == HINT_LINE).cloned();

    let (mut realm, _, _) = enter("vanguard", &[], &[]);
    assert!(hint(&gossip(&mut realm, lyria)).is_none(), "not taken yet");

    let (mut realm, me, _) = enter("vanguard", &[(ADVANCEMENT, [0; 4])], &[]);
    let line = hint(&gossip(&mut realm, lyria)).expect("the hint line");
    let before = realm.export_character(me).unwrap();
    assert_eq!(choose(&mut realm, lyria, line.index), HINT_REPLY);
    assert_eq!(
        realm.export_character(me).unwrap(),
        before,
        "the hint changes nothing"
    );
    let again = hint(&gossip(&mut realm, lyria)).expect("asked again");
    assert_eq!(choose(&mut realm, lyria, again.index), HINT_REPLY);

    let (mut realm, _, _) = enter("zealot", &[], &[ADVANCEMENT, TRIAL]);
    assert!(hint(&gossip(&mut realm, lyria)).is_none(), "handed in");
}

#[test]
fn the_advancement_waits_on_its_trial() {
    let (mut realm, _, _) = enter("vanguard", &[(ADVANCEMENT, [0; 4])], &[]);
    let tester = hub_guid(TESTER);
    let list = quest_list(&mut realm, tester);
    assert!(list.contains(&(ADVANCEMENT, 3)), "incomplete: {list:?}");

    walk_up_to(&mut realm, tester);
    realm.handle(RealmCommand::QuestGiverCompleteQuest {
        npc: tester,
        quest: ADVANCEMENT,
    });
    let progress = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestProgress(p) => Some(p),
            _ => None,
        })
        .expect("the progress panel");
    assert_eq!(progress.request_text, PROGRESS);
    assert!(!progress.is_complete);
    assert!(completed(&hand_in(&mut realm, tester, ADVANCEMENT)).is_none());
    assert_eq!(realm.current_calling().unwrap().key, "vanguard");

    accept(&mut realm, tester, TRIAL);
    let ready = gossip(&mut realm, tester)
        .into_iter()
        .find(|o| o.message == "I am ready.")
        .expect("the trial's talk line");
    assert_eq!(ready.index, TALK_OPTION);
    assert_eq!(choose(&mut realm, tester, ready.index), "Then it is done.");
    assert!(completed(&hand_in(&mut realm, tester, TRIAL)).is_some());

    assert_eq!(
        status(&mut realm, tester),
        dialog_status::REWARD2,
        "a yellow ? now"
    );
    assert!(completed(&hand_in(&mut realm, tester, ADVANCEMENT)).is_some());
    assert_eq!(realm.current_calling().unwrap().key, "zealot");
}
