//! Milestones (#627, ADR 0034): the realm reads its state file, reaches a Milestone once, hands
//! the whole file to the host once a tick, and tells every character in the world the reached set.
//! Content that waits on one (#629): quests, quest GameObjects and Phased Dungeon rows with
//! `after_milestone` or `until_milestone`. The Hub NPC and added zone creature filters are tested
//! in `src/hub.rs` and `src/population.rs`, which no test seam reaches.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;
use twilight_realm::{Realm, Storage};

/// 2026-11-20T21:14:03Z.
const BREAK_MS: u64 = 1_795_209_243_000;
const BREAK: &str = "2026-11-20T21:14:03Z";
const ANCHOR: &str = "ilgalar_anchor_broken";
const STEPPES: &str = "steppes_open";

fn at_the_break() -> Realm {
    let mut realm = Realm::new();
    realm.set_wall_clock(BREAK_MS);
    realm
}

/// The Milestones state files among `storage`.
fn writes(storage: Vec<Storage>) -> Vec<String> {
    storage
        .into_iter()
        .filter_map(|s| match s {
            Storage::Milestones { blob } => Some(blob),
            _ => None,
        })
        .collect()
}

/// Whether `events` hold a `SessionEvent::Milestones` naming `key`.
fn told(events: &[SessionEvent], key: &str) -> bool {
    events.iter().any(
        |e| matches!(e, SessionEvent::Milestones { reached } if reached.iter().any(|k| k == key)),
    )
}

#[test]
fn with_no_state_file_nothing_is_reached() {
    let mut realm = at_the_break();
    realm.set_milestones(None);
    assert!(!realm.milestone_reached(ANCHOR));
    assert!(!realm.milestone_reached(STEPPES));
    assert!(realm.reached_milestones().is_empty());
    assert!(realm.drain_log().is_empty());
}

#[test]
fn a_reached_table_reaches_its_milestone_only() {
    let mut realm = at_the_break();
    realm.set_milestones(Some(&format!("[{ANCHOR}]\nreached = \"{BREAK}\"\n")));
    assert!(realm.milestone_reached(ANCHOR));
    assert!(!realm.milestone_reached(STEPPES));
    assert_eq!(realm.reached_milestones(), [ANCHOR]);
    assert!(realm.drain_log().is_empty());
}

#[test]
fn an_unknown_table_or_a_bad_date_warns_once_and_reaches_nothing() {
    for state in [
        "[nope]\nreached = \"2026-11-20T21:14:03Z\"\n".to_string(),
        format!("[{ANCHOR}]\nreached = \"yesterday\"\n"),
        "[nope\n".to_string(),
    ] {
        let mut realm = at_the_break();
        realm.set_milestones(Some(&state));
        assert!(realm.reached_milestones().is_empty(), "{state}");
        let log = realm.drain_log();
        assert_eq!(log.len(), 1, "{state}: {log:?}");
    }
}

#[test]
fn a_milestone_reached_before_its_after_stays_reached_with_a_warning() {
    let mut realm = at_the_break();
    realm.set_milestones(Some(&format!("[{STEPPES}]\nreached = \"{BREAK}\"\n")));
    assert!(realm.milestone_reached(STEPPES));
    let log = realm.drain_log();
    assert_eq!(log.len(), 1, "{log:?}");
    assert!(log[0].contains(ANCHOR), "{log:?}");
}

#[test]
fn reaching_one_writes_the_file_once_with_the_wall_clocks_date() {
    let mut realm = at_the_break();
    realm.set_milestones(None);
    realm.reach_milestone(ANCHOR);
    assert!(realm.milestone_reached(ANCHOR));
    let log = realm.drain_log();
    assert_eq!(
        log,
        ["Milestone The Ilgalar Anchor is broken reached."],
        "{log:?}"
    );

    let blobs = writes(realm.drain_storage());
    assert_eq!(blobs.len(), 1, "{blobs:?}");
    assert!(
        blobs[0].contains(&format!("reached = \"{BREAK}\"")),
        "{}",
        blobs[0]
    );
    let mut again = Realm::new();
    again.set_milestones(Some(&blobs[0]));
    assert!(again.milestone_reached(ANCHOR));
    assert!(!again.milestone_reached(STEPPES));
    assert!(again.drain_log().is_empty());

    realm.reach_milestone(ANCHOR);
    realm.tick(0.1, &FlatGround);
    assert!(writes(realm.drain_storage()).is_empty());
    assert!(realm.drain_log().is_empty());
}

#[test]
fn two_reached_in_one_tick_give_one_write() {
    let mut realm = at_the_break();
    realm.reach_milestone(ANCHOR);
    realm.reach_milestone(STEPPES);
    realm.tick(0.1, &FlatGround);
    let blobs = writes(realm.drain_storage());
    assert_eq!(blobs.len(), 1, "{blobs:?}");
    assert!(blobs[0].contains(ANCHOR) && blobs[0].contains(STEPPES));
}

#[test]
fn an_unknown_key_warns_and_reaches_nothing() {
    let mut realm = at_the_break();
    realm.reach_milestone("nope");
    assert!(realm.reached_milestones().is_empty());
    assert_eq!(realm.drain_log().len(), 1);
    assert!(writes(realm.drain_storage()).is_empty());
}

#[test]
fn every_character_in_the_world_is_told() {
    let mut p = Party::new(at_the_break(), &[1, 1]);
    for i in 0..2 {
        assert!(p
            .seen(i)
            .iter()
            .any(|e| matches!(e, SessionEvent::Milestones { reached } if reached.is_empty())));
    }
    p.realm.reach_milestone(ANCHOR);
    let got = p.run(0.0);
    for events in &got {
        assert!(told(events, ANCHOR), "{events:?}");
    }
}

#[test]
fn a_character_entering_afterwards_is_told_before_its_world() {
    let mut realm = at_the_break();
    realm.reach_milestone(ANCHOR);
    let (_, events) = enter_new(&mut realm, "Mara", HUMAN);
    let told_at = events
        .iter()
        .position(|e| matches!(e, SessionEvent::Milestones { reached } if reached == &[ANCHOR]))
        .expect("told at login");
    let world_at = events
        .iter()
        .position(|e| {
            matches!(
                e,
                SessionEvent::Teleport { .. } | SessionEvent::ObjectCreate { .. }
            )
        })
        .expect("the world loads");
    assert!(told_at < world_at, "{told_at} before {world_at}");
}

const HIGHGUID_UNIT: u64 = 0xF130;
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
/// Marshal McBride on the Abbey steps (Elwynn spawn 79970), who gives the test quests.
const MCBRIDE: u64 = HIGHGUID_UNIT << 48 | 197 << 24 | 79970;
const BY_MCBRIDE: [f32; 3] = [-8910.0, -140.5, 82.0];
const UNTIL_BREAK: u32 = 99631;
const AFTER_BREAK: u32 = 99632;
/// A notice on the steps that is posted once the Anchor breaks.
const POSTED: u64 = HIGHGUID_GAMEOBJECT << 48 | 990631;

const WAITING: &str = r#"
[[gameobject]]
guid = 990631
entry = 990631
display = 1
map = 0
position = [-8911.0, -138.5, 82.0]
after_milestone = "ilgalar_anchor_broken"

[[quest]]
id = 99631
kind = "story"
giver = 197
level = 5
title = "Before the Break"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
until_milestone = "ilgalar_anchor_broken"

[[quest]]
id = 99632
kind = "story"
giver = 197
level = 5
title = "After the Break"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
after_milestone = "ilgalar_anchor_broken"
"#;

/// A level 10 warrior from Northshire beside McBride, with the waiting test content.
fn on_the_steps() -> Party {
    let realm = Realm::with_seed(1).with_quests(WAITING);
    let mut p = Party::starting(realm, &[10], START_ELWYNN, Box::new(NoGround));
    p.go(0, BY_MCBRIDE, std::f32::consts::PI);
    p.run(0.5);
    p
}

/// The quests McBride offers player 0 now.
fn offered(p: &mut Party) -> Vec<u32> {
    let from = p.seen(0).len();
    p.cmd(0, RealmCommand::QuestGiverHello { npc: MCBRIDE });
    p.seen(0)[from..]
        .iter()
        .flat_map(|e| match e {
            SessionEvent::QuestGreeting(list) => list.quests.iter().map(|q| q.quest_id).collect(),
            SessionEvent::QuestDetail(d) => vec![d.quest_id],
            _ => Vec::new(),
        })
        .collect()
}

/// Whether player `i`'s client holds `guid`, by what it has been sent.
fn shows(p: &Party, i: usize, guid: u64) -> bool {
    let mut shown = false;
    for e in p.seen(i) {
        match e {
            SessionEvent::ObjectCreate { guid: g, .. } if *g == guid => shown = true,
            SessionEvent::ObjectsRemoved(gone) if gone.contains(&guid) => shown = false,
            SessionEvent::ObjectDestroyed(g) if *g == guid => shown = false,
            _ => {}
        }
    }
    shown
}

#[test]
fn a_quest_until_the_break_is_offered_before_it_and_handed_in_after() {
    let mut p = on_the_steps();
    assert!(offered(&mut p).contains(&UNTIL_BREAK));
    p.cmd(
        0,
        RealmCommand::QuestGiverAcceptQuest {
            npc: MCBRIDE,
            quest: UNTIL_BREAK,
        },
    );
    p.realm.reach_milestone(ANCHOR);
    p.run(0.0);
    let mut fresh = on_the_steps();
    fresh.realm.reach_milestone(ANCHOR);
    assert!(!offered(&mut fresh).contains(&UNTIL_BREAK));

    // The one who took it before keeps it and hands it in.
    let from = p.seen(0).len();
    for command in [
        RealmCommand::QuestGiverCompleteQuest {
            npc: MCBRIDE,
            quest: UNTIL_BREAK,
        },
        RealmCommand::QuestGiverRequestReward {
            npc: MCBRIDE,
            quest: UNTIL_BREAK,
        },
        RealmCommand::QuestGiverChooseReward {
            npc: MCBRIDE,
            quest: UNTIL_BREAK,
            choice: 0,
        },
    ] {
        p.cmd(0, command);
    }
    let done = p.seen(0)[from..]
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestComplete(c) if c.quest_id == UNTIL_BREAK));
    assert!(done, "{:?}", &p.seen(0)[from..]);
}

#[test]
fn a_quest_after_the_break_is_offered_only_after_it() {
    let mut p = on_the_steps();
    assert!(!offered(&mut p).contains(&AFTER_BREAK));
    p.realm.reach_milestone(ANCHOR);
    assert!(offered(&mut p).contains(&AFTER_BREAK));
}

#[test]
fn a_quest_gameobject_after_the_break_is_created_within_a_tick_of_it() {
    let mut p = on_the_steps();
    p.run(1.0);
    assert!(!shows(&p, 0, POSTED), "posted before the break");
    p.realm.reach_milestone(ANCHOR);
    p.run(0.0);
    assert!(shows(&p, 0, POSTED), "not posted a tick after the break");
}

/// The Ward Test's scenery that stands until the Anchor breaks, and its twin after.
const UNTIL_SCENERY: u64 = HIGHGUID_GAMEOBJECT << 48 | 810008;
const AFTER_SCENERY: u64 = HIGHGUID_GAMEOBJECT << 48 | 810009;
/// In the Ward Test's building, by its inner portal to "water".
const IN_THE_WARD_TEST: [f32; 3] = [-9790.0, 565.0, 47.2];

#[test]
fn a_phased_dungeons_twins_swap_in_a_run_under_way_when_the_milestone_is_reached() {
    let mut p = Party::starting(Realm::with_seed(1), &[60], START_ELWYNN, Box::new(NoGround));
    p.go(0, IN_THE_WARD_TEST, 0.0);
    p.run(1.0);
    assert!(shows(&p, 0, UNTIL_SCENERY), "until_milestone scenery");
    assert!(!shows(&p, 0, AFTER_SCENERY), "after_milestone scenery");
    p.realm.reach_milestone(ANCHOR);
    p.run(0.0);
    assert!(
        !shows(&p, 0, UNTIL_SCENERY),
        "until_milestone scenery stayed"
    );
    assert!(
        shows(&p, 0, AFTER_SCENERY),
        "after_milestone scenery did not come"
    );
}

#[test]
#[should_panic(expected = "quest 99639 names Milestone nope")]
fn a_quest_waiting_on_no_milestone_fails_the_content_load() {
    let _ = Realm::new().with_quests(
        r#"
[[quest]]
id = 99639
kind = "story"
giver = 197
level = 5
title = "Nope"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
after_milestone = "nope"
"#,
    );
}
