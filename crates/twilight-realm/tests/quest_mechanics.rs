//! Quest mechanics (#230): several prerequisites, talk-to objectives and use-GameObject
//! objectives. The quests here are test-only, added through `Realm::with_quests`; Marshal McBride
//! on the Abbey steps is the talk target and giver.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::*;
use quest_common::*;

const HIGHGUID_UNIT: u64 = 0xF130;
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
/// Marshal McBride on the Abbey steps (Elwynn spawn 79970).
const MCBRIDE: u64 = HIGHGUID_UNIT << 48 | 197 << 24 | 79970;
const MCBRIDE_AT: [f32; 3] = [-8912.0, -140.5, 82.0];

const FIRST: u32 = 99201;
const SECOND: u32 = 99202;
const BOTH: u32 = 99203;
const ASK_THE_ROAD: u32 = 99204;
const GATHER: u32 = 99205;
const ROOT: u64 = HIGHGUID_GAMEOBJECT << 48 | 990001;

const TEST_QUESTS: &str = r#"
[[gameobject]]
guid = 990001
entry = 990001
display = 1
map = 0
position = [-8911.0, -140.5, 82.0]
respawn = 5.0

[[quest]]
id = 99201
kind = "story"
giver = 197
level = 5
title = "First"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0

[[quest]]
id = 99202
kind = "story"
giver = 197
level = 5
title = "Second"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0

[[quest]]
id = 99203
kind = "story"
giver = 197
after = [99201, 99202]
level = 5
title = "Both"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0

[[quest]]
id = 99204
kind = "story"
giver = 197
level = 5
title = "Ask the Road"
objectives = "Ask the Marshal about the road."
details = "-"
progress = "-"
completion = "-"
money = 0
talk = [{ npc = 197, option = "Ask about the road.", reply = "Mind the wolves." }]

[[quest]]
id = 99205
kind = "story"
giver = 197
level = 5
title = "Gather"
objectives = "Dig up two roots."
details = "-"
progress = "-"
completion = "-"
money = 0
use = [[990001, 2]]

[[gameobject]]
guid = 990002
entry = 990002
display = 1
map = 0
position = [-8911.0, -139.5, 82.0]

[[quest]]
id = 99206
kind = "story"
giver_object = 990002
ender = 197
level = 5
title = "Posted"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0

[[quest]]
id = 99207
kind = "story"
giver_object = 990002
ender = 197
after = 99206
level = 5
title = "Posted Again"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
"#;

const NOTICE: u64 = HIGHGUID_GAMEOBJECT << 48 | 990002;

/// The Abbey's floor everywhere.
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

/// A level-10 warrior who began in Northshire, beside McBride, with the test quests.
fn in_northshire() -> Realm {
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
    wait(&mut realm, 0.5);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [MCBRIDE_AT[0] + 2.0, MCBRIDE_AT[1], MCBRIDE_AT[2]],
        orientation: std::f32::consts::PI,
    });
    wait(&mut realm, 0.5);
    realm
}

/// Runs the world for `seconds` in tenth-second ticks; the events on the way.
fn wait(realm: &mut Realm, seconds: f32) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    for _ in 0..(seconds * 10.0).round() as u32 {
        realm.tick(0.1, &AbbeyFloor);
        events.extend(drain(realm));
    }
    events
}

fn take(realm: &mut Realm, quest: u32) {
    realm.handle(RealmCommand::QuestGiverAcceptQuest {
        npc: MCBRIDE,
        quest,
    });
    drain(realm);
}

fn turn_in(realm: &mut Realm, quest: u32) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::QuestGiverCompleteQuest {
        npc: MCBRIDE,
        quest,
    });
    realm.handle(RealmCommand::QuestGiverRequestReward {
        npc: MCBRIDE,
        quest,
    });
    realm.handle(RealmCommand::QuestGiverChooseReward {
        npc: MCBRIDE,
        quest,
        choice: 0,
    });
    drain(realm)
}

fn offered(realm: &mut Realm) -> Vec<u32> {
    realm.handle(RealmCommand::QuestGiverHello { npc: MCBRIDE });
    drain(realm)
        .into_iter()
        .flat_map(|e| match e {
            SessionEvent::QuestGreeting(list) => list.quests.iter().map(|q| q.quest_id).collect(),
            SessionEvent::QuestDetail(d) => vec![d.quest_id],
            _ => Vec::new(),
        })
        .collect()
}

/// McBride's gossip lines now.
fn lines(realm: &mut Realm) -> Vec<(u32, String)> {
    realm.handle(RealmCommand::GossipHello { guid: MCBRIDE });
    drain(realm)
        .into_iter()
        .flat_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => {
                options.into_iter().map(|o| (o.index, o.message)).collect()
            }
            _ => Vec::new(),
        })
        .collect()
}

fn detail_of(events: &[SessionEvent], quest: u32) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestDetail(d) if d.quest_id == quest))
}

#[test]
fn a_quest_after_two_others_is_offered_when_both_are_done_and_at_the_last_turn_in() {
    let mut realm = in_northshire();
    assert!(!offered(&mut realm).contains(&BOTH));
    take(&mut realm, FIRST);
    take(&mut realm, SECOND);

    let first = turn_in(&mut realm, FIRST);
    assert!(completed(&first).is_some());
    assert!(!detail_of(&first, BOTH), "one prerequisite is not enough");
    assert!(!offered(&mut realm).contains(&BOTH));

    let second = turn_in(&mut realm, SECOND);
    assert!(
        detail_of(&second, BOTH),
        "the last turn-in offers it: {second:?}"
    );
    assert!(offered(&mut realm).contains(&BOTH));
}

#[test]
fn a_quest_after_one_id_still_reads_as_before() {
    let mut realm = in_northshire();
    let toml = r#"
[[quest]]
id = 99290
kind = "story"
giver = 197
after = 99201
level = 5
title = "Single"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
"#;
    realm = realm.with_quests(toml);
    assert!(!offered(&mut realm).contains(&99290));
    take(&mut realm, FIRST);
    turn_in(&mut realm, FIRST);
    assert!(offered(&mut realm).contains(&99290));
}

#[test]
fn a_talk_objective_gives_credit_only_through_its_option_only_while_active_and_once() {
    let mut realm = in_northshire();
    // Not active: no line, and choosing it does nothing.
    assert!(lines(&mut realm).is_empty());
    realm.handle(RealmCommand::GossipSelectOption {
        guid: MCBRIDE,
        option: 100,
    });
    assert!(!drain(&mut realm)
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestObjectiveKill { .. })));

    take(&mut realm, ASK_THE_ROAD);
    // Active but not talked: handing in finds nothing done.
    assert!(completed(&turn_in(&mut realm, ASK_THE_ROAD)).is_none());
    assert_eq!(
        lines(&mut realm),
        vec![(100, "Ask about the road.".to_string())]
    );

    realm.handle(RealmCommand::GossipSelectOption {
        guid: MCBRIDE,
        option: 100,
    });
    let events = drain(&mut realm);
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::QuestObjectiveKill {
            quest_id: ASK_THE_ROAD,
            entry: 197,
            count: 1,
            required: 1
        }
    )));
    let text_id = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu {
                text_id, options, ..
            } if options.is_empty() => Some(*text_id),
            _ => None,
        })
        .expect("the reply menu");
    realm.handle(RealmCommand::NpcTextQuery { text_id });
    let reply = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::NpcGreeting { blocks, .. } => Some(blocks[0].male.clone()),
            _ => None,
        })
        .expect("the reply text");
    assert_eq!(reply, "Mind the wolves.");

    // Once: the line is gone and choosing it again gives nothing.
    assert!(lines(&mut realm).is_empty());
    realm.handle(RealmCommand::GossipSelectOption {
        guid: MCBRIDE,
        option: 100,
    });
    assert!(!drain(&mut realm)
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestObjectiveKill { .. })));

    assert!(completed(&turn_in(&mut realm, ASK_THE_ROAD)).is_some());
}

#[test]
fn a_talk_objective_is_saved_with_the_character() {
    let mut realm = in_northshire();
    take(&mut realm, ASK_THE_ROAD);
    realm.handle(RealmCommand::GossipHello { guid: MCBRIDE });
    realm.handle(RealmCommand::GossipSelectOption {
        guid: MCBRIDE,
        option: 100,
    });
    drain(&mut realm);
    let me = roster(&mut realm)[0].guid;
    let blob = realm.export_character(me).unwrap();
    assert!(blob.contains("kills = [1, 0, 0, 0]"), "{blob}");
}

#[test]
fn a_use_objective_counts_up_to_its_count_and_the_object_returns_after_its_respawn() {
    let mut realm = in_northshire();
    take(&mut realm, GATHER);
    let streamed = wait(&mut realm, 0.5);
    let _ = streamed;
    // The object is in view (it streamed with the walk); clicking it counts one and removes it.
    realm.handle(RealmCommand::UseGameObject { guid: ROOT });
    let first = drain(&mut realm);
    assert!(first.iter().any(|e| matches!(
        e,
        SessionEvent::QuestObjectiveKill {
            quest_id: GATHER,
            count: 1,
            required: 2,
            ..
        }
    )));
    assert!(first
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectsRemoved(g) if g.contains(&ROOT))));
    assert!(completed(&turn_in(&mut realm, GATHER)).is_none());

    // Gone: another click does nothing.
    realm.handle(RealmCommand::UseGameObject { guid: ROOT });
    assert!(!drain(&mut realm)
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestObjectiveKill { .. })));

    // Back after 5 s.
    let early = wait(&mut realm, 3.0);
    assert!(!early
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectCreate { guid, .. } if *guid == ROOT)));
    let later = wait(&mut realm, 3.0);
    assert!(
        later
            .iter()
            .any(|e| matches!(e, SessionEvent::ObjectCreate { guid, .. } if *guid == ROOT)),
        "{:.600}",
        format!("{early:?} {later:?}")
    );

    realm.handle(RealmCommand::UseGameObject { guid: ROOT });
    let second = drain(&mut realm);
    assert!(second.iter().any(|e| matches!(
        e,
        SessionEvent::QuestObjectiveKill {
            count: 2,
            required: 2,
            ..
        }
    )));
    assert!(completed(&turn_in(&mut realm, GATHER)).is_some());
}

#[test]
fn a_use_without_a_quest_that_wants_it_leaves_the_object_standing() {
    let mut realm = in_northshire();
    realm.handle(RealmCommand::UseGameObject { guid: ROOT });
    assert!(!drain(&mut realm)
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectsRemoved(_))));
}

#[test]
#[should_panic(expected = "more than 4 kill, talk and use objectives")]
fn more_than_four_kill_talk_and_use_objectives_fail_validation() {
    let _ = Realm::new().with_quests(
        r#"
[[gameobject]]
guid = 990001
entry = 990001
display = 1
map = 0
position = [-8911.0, -140.5, 82.0]

[[quest]]
id = 99299
kind = "story"
giver = 197
level = 5
title = "Too many"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
kill = [[6, 1], [38, 1]]
talk = [{ npc = 197, option = "a", reply = "b" }]
use = [[990001, 1], [990001, 2]]
"#,
    );
}

fn click_notice(realm: &mut Realm) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::UseGameObject { guid: NOTICE });
    drain(realm)
}

fn accept_from_notice(realm: &mut Realm, quest: u32) {
    realm.handle(RealmCommand::QuestGiverAcceptQuest { npc: NOTICE, quest });
    drain(realm);
}

#[test]
fn clicking_a_giver_object_offers_its_quest_and_accepting_puts_it_in_the_log() {
    let mut realm = in_northshire();
    let clicked = click_notice(&mut realm);
    assert!(
        clicked.iter().any(
            |e| matches!(e, SessionEvent::QuestDetail(d) if d.quest_id == 99206 && d.npc == NOTICE)
        ),
        "{clicked:?}"
    );
    // The quest after it is not offered yet.
    assert!(!detail_of(&clicked, 99207));
    accept_from_notice(&mut realm, 99206);
    let me = roster(&mut realm)[0].guid;
    let blob = realm.export_character(me).unwrap();
    assert!(blob.contains("99206"), "{blob}");
    // The object stays, and a turn-in at the ender works.
    assert!(completed(&turn_in(&mut realm, 99206)).is_some());
}

#[test]
fn a_giver_object_offers_nothing_for_a_quest_in_the_log_or_done_or_before_its_after() {
    let mut realm = in_northshire();
    // 99207 waits for 99206, so accepting it does nothing.
    accept_from_notice(&mut realm, 99207);
    let me = roster(&mut realm)[0].guid;
    assert!(!realm.export_character(me).unwrap().contains("99207"));

    accept_from_notice(&mut realm, 99206);
    // In the log: nothing to offer, and the click does nothing.
    assert!(click_notice(&mut realm).is_empty());

    // Done: the follow-up is offered, and then nothing is left.
    assert!(completed(&turn_in(&mut realm, 99206)).is_some());
    let after = click_notice(&mut realm);
    assert!(detail_of(&after, 99207), "{after:?}");
    accept_from_notice(&mut realm, 99207);
    assert!(completed(&turn_in(&mut realm, 99207)).is_some());
    assert!(click_notice(&mut realm).is_empty());
}

const BAD_QUEST: &str = r#"
[[quest]]
id = 99298
kind = "story"
level = 5
title = "Bad"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
"#;

#[test]
#[should_panic(expected = "exactly one of `giver` and `giver_object`")]
fn a_quest_with_both_giver_and_giver_object_fails_validation() {
    let _ = Realm::new().with_quests(&format!(
        "{BAD_QUEST}giver = 197\ngiver_object = 990002\nender = 197\n"
    ));
}

#[test]
#[should_panic(expected = "exactly one of `giver` and `giver_object`")]
fn a_quest_with_neither_giver_nor_giver_object_fails_validation() {
    let _ = Realm::new().with_quests(BAD_QUEST);
}

#[test]
#[should_panic(expected = "no `gameobject` row declares")]
fn a_giver_object_that_no_gameobject_row_declares_fails_validation() {
    let _ = Realm::new().with_quests(&format!("{BAD_QUEST}giver_object = 990777\nender = 197\n"));
}
