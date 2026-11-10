//! Escorts (#701): a character holding a quest asks a friendly creature to set off, and a private
//! copy of it walks a path for their group. `content/escorts.toml`'s `test_escort` belongs to this
//! file: its start creature is the Erin of spawn 960821 in south Redridge (`content/zones.toml`),
//! its path four points 20 yd apart running 80 yd west from her, `wait_yd` 20, `leash_yd` 60,
//! `leash_s` 10, `reset_s` 5 and `linger_s` 3. Its quest 99821 is added here with
//! `Realm::with_quests`.

mod common;

use benilla_protocol::messages::CHAT_MSG_SYSTEM;
use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;
use twilight_realm::Audience;

const HIGHGUID_UNIT: u64 = 0xF130;
const ERIN: u32 = 850;
/// The start creature: Erin's spawn 960821.
const START: u64 = HIGHGUID_UNIT << 48 | (ERIN as u64) << 24 | 960821;
const QUEST: u32 = 99821;
const ESCORT_OPTION: u32 = 90;
const START_OPTION: &str = "Lead the way.";
const FAILED: &str = "The escort has failed.";
const PRAIRIE_WOLF: u32 = 2958;
/// Erin's spot; the Z is ground_check.py's, and `NoGround` keeps it.
const START_AT: [f32; 3] = [-9805.0, -3463.0, 218.49];
/// The x of the start and of each point, west along y = -3463.
const LEGS: [f32; 5] = [-9805.0, -9825.0, -9845.0, -9865.0, -9885.0];

const QUESTS: &str = r#"
[[quest]]
id = 99821
kind = "story"
giver = 197
level = 25
title = "See Her Home"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
"#;

/// `[x, y]` on the path's line, at Erin's height.
fn at(x: f32, dy: f32) -> [f32; 3] {
    [x, START_AT[1] + dy, START_AT[2]]
}

/// Within talking reach of Erin.
fn beside() -> [f32; 3] {
    at(START_AT[0] + 2.0, 0.0)
}

/// What a character's save holds of quest 99821.
#[derive(Clone, Copy)]
enum Holds {
    Nothing,
    Quest,
    Met,
}

/// Level 30 characters, each holding what `holds` says, each at `spots[i]`.
fn party(holds: &[Holds], spots: &[[f32; 3]]) -> Party {
    let realm = realm().with_quests(QUESTS);
    let levels = vec![30; holds.len()];
    let holds = holds.to_vec();
    let mut p = Party::edited(
        realm,
        &levels,
        START_ELWYNN,
        Box::new(NoGround),
        move |i, blob| {
            let kills = match holds[i] {
                Holds::Nothing => return blob,
                Holds::Quest => "[0, 0, 0, 0]",
                Holds::Met => "[1, 0, 0, 0]",
            };
            format!("{blob}\n[[quests]]\nid = {QUEST}\nslot = 0\nkills = {kills}\n")
        },
    );
    for (i, &spot) in spots.iter().enumerate() {
        p.go(i, spot, 0.0);
    }
    p.run(1.0);
    p
}

/// The options of the gossip menu player `i` is sent on greeting `npc`; none without a menu.
fn options(p: &mut Party, i: usize, npc: u64) -> Vec<(u32, String)> {
    let before = p.seen(i).len();
    p.cmd(i, RealmCommand::GossipHello { guid: npc });
    p.seen(i)[before..]
        .iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu {
                npc: n, options, ..
            } if *n == npc => Some(
                options
                    .iter()
                    .map(|o| (o.index, o.message.clone()))
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

fn offered(p: &mut Party, i: usize, npc: u64) -> bool {
    options(p, i, npc)
        .iter()
        .any(|(index, _)| *index == ESCORT_OPTION)
}

/// Player `i` chooses the start line; the escortee their client is sent within a second.
fn start(p: &mut Party, i: usize) -> u64 {
    let before = p.seen(i).len();
    p.cmd(
        i,
        RealmCommand::GossipSelectOption {
            guid: START,
            option: ESCORT_OPTION,
        },
    );
    p.run(1.0);
    escortee(&p.seen(i)[before..]).expect("an escortee")
}

/// A creature of Erin's entry other than the start creature, created in `events`.
fn escortee(events: &[SessionEvent]) -> Option<u64> {
    unit_creates(events)
        .into_iter()
        .find(|(g, entry, _)| *entry == ERIN && *g != START)
        .map(|(g, _, _)| g)
}

fn removed(events: &[SessionEvent], guid: u64) -> bool {
    events.iter().any(|e| match e {
        SessionEvent::ObjectsRemoved(gone) => gone.contains(&guid),
        SessionEvent::ObjectDestroyed(g) => *g == guid,
        _ => false,
    })
}

fn created(events: &[SessionEvent], guid: u64) -> bool {
    unit_creates(events).iter().any(|(g, _, _)| *g == guid)
}

/// `guid`'s moves in `events`: whether each is a stop, and where it starts.
fn moves(events: &[SessionEvent], guid: u64) -> Vec<(bool, [f32; 3])> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::MonsterMove {
                guid: g,
                stop,
                start,
                ..
            } if *g == guid => Some((*stop, *start)),
            _ => None,
        })
        .collect()
}

/// Whether `events` show `player`'s log slot 0 in the complete state.
fn log_complete(events: &[SessionEvent], player: u64) -> bool {
    /// `PLAYER_QUEST_LOG_1_1` + 1: slot 0's counters and state.
    const SLOT_0_STATE: u16 = 199;
    events.iter().any(|e| match e {
        SessionEvent::ObjectValues { guid, fields } if *guid == player => fields
            .raw_fields()
            .any(|(f, v)| f == SLOT_0_STATE && v >> 24 & 1 == 1),
        _ => false,
    })
}

fn system_lines(events: &[SessionEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Chat(m) if m.chat_type == CHAT_MSG_SYSTEM => Some(m.text.clone()),
            _ => None,
        })
        .collect()
}

fn quest_failed(events: &[SessionEvent]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestFailed { quest_id, .. } if *quest_id == QUEST))
}

/// Players `who` keep up with the escortee leg by leg, each standing 10 yd along the leg he
/// walks, until the first of them is sent the quest complete. Whether it came.
fn follow(p: &mut Party, who: &[usize]) -> bool {
    let lead = p.guids[who[0]];
    for k in 0..4 {
        let mid = at((LEGS[k] + LEGS[k + 1]) / 2.0, 3.0);
        for &i in who {
            p.go(i, mid, 0.0);
        }
        if p.run_until(15.0, |got| log_complete(&got[who[0]], lead)) {
            return true;
        }
    }
    false
}

#[test]
fn start_to_finish() {
    let mut p = party(&[Holds::Quest], &[beside()]);
    assert!(
        options(&mut p, 0, START).contains(&(ESCORT_OPTION, START_OPTION.to_string())),
        "the start line"
    );
    let before = p.seen(0).len();
    let him = start(&mut p, 0);
    let new = p.seen(0)[before..].to_vec();
    assert!(removed(&new, START), "the start creature leaves their view");
    p.run(1.0);
    assert!(
        !moves(&p.seen(0)[before..], him).is_empty(),
        "he is sent walking"
    );

    assert!(follow(&mut p, &[0]), "he reaches the last point");
    let arrived = p.seen(0).len();
    p.run(2.5);
    assert!(p.units_held(0).contains(&him), "he lingers 3 s");
    p.run(1.0);
    assert!(!p.units_held(0).contains(&him), "then he goes");
    assert!(!created(&p.seen(0)[arrived..], START));
    p.run(2.0);
    assert!(
        created(&p.seen(0)[arrived..], START),
        "the start creature is back 5 s after the arrival"
    );
    assert!(!quest_failed(p.seen(0)));
}

#[test]
fn the_group_shares_him_and_another_character_starts_their_own() {
    let mut p = party(
        &[Holds::Quest, Holds::Quest, Holds::Quest],
        &[
            beside(),
            at(START_AT[0] + 30.0, 0.0),
            at(START_AT[0] + 150.0, 0.0),
        ],
    );
    let him = start(&mut p, 0);
    assert!(created(p.seen(1), him), "the second is in the group");
    p.go(2, beside(), 0.0);
    p.run(1.0);
    assert!(
        p.units_held(2).contains(&START),
        "the third, come up, still sees the start creature"
    );
    assert!(offered(&mut p, 2, START), "and is offered the line");
    let theirs = start(&mut p, 2);
    assert_ne!(theirs, him, "an escortee of their own");

    assert!(follow(&mut p, &[0]), "the first escort arrives");
    p.run(0.5);
    assert!(
        log_complete(p.seen(1), p.guids[1]),
        "the second is credited at the start"
    );
    assert!(!created(p.seen(2), him), "the third never sees the first");
    assert!(!created(p.seen(0), theirs), "nor the first the third's");
}

#[test]
fn he_waits_while_the_group_is_behind() {
    let mut p = party(&[Holds::Quest], &[beside()]);
    let him = start(&mut p, 0);
    let mark = p.seen(0).len();
    assert!(
        p.run_until(20.0, |got| moves(&got[0], him).iter().any(|m| m.0)),
        "he stops once 20 yd ahead"
    );
    let stopped = moves(&p.seen(0)[mark..], him)
        .into_iter()
        .find(|m| m.0)
        .unwrap()
        .1;
    p.go(0, at(stopped[0] + 30.0, 0.0), 0.0);
    let held = p.run(3.0);
    assert!(
        moves(&held[0], him).iter().all(|m| m.0),
        "30 yd behind: he stays put"
    );
    assert!(!system_lines(&held[0]).contains(&FAILED.to_string()));
    p.go(0, at(stopped[0] + 15.0, 0.0), 0.0);
    let on = p.run(1.0);
    assert!(
        moves(&on[0], him).iter().any(|m| !m.0),
        "within 20 yd: he walks on"
    );
}

#[test]
fn left_behind_it_fails_and_can_start_again() {
    let mut p = party(&[Holds::Quest], &[beside()]);
    let him = start(&mut p, 0);
    p.go(0, at(START_AT[0] + 100.0, 0.0), 0.0);
    let early = p.run(9.0);
    assert!(system_lines(&early[0]).is_empty(), "not before 10 s");
    let failed = p.run(1.5);
    assert_eq!(system_lines(&failed[0]), [FAILED]);
    assert!(!p.units_held(0).contains(&him));
    assert!(!quest_failed(p.seen(0)), "the quest is not failed");

    let mark = p.seen(0).len();
    p.go(0, beside(), 0.0);
    p.run(2.0);
    assert!(
        !created(&p.seen(0)[mark..], START),
        "hidden until the reset"
    );
    p.run(3.5);
    assert!(created(&p.seen(0)[mark..], START), "back after 5 s");
    assert!(offered(&mut p, 0, START), "the quest is still in the log");
    let again = start(&mut p, 0);
    assert_ne!(again, him);
}

#[test]
fn killed_it_fails_and_can_start_again() {
    let mut p = party(&[Holds::Quest], &[beside()]);
    let him = start(&mut p, 0);
    let mark = p.seen(0).len();
    assert!(
        p.run_until(20.0, |got| moves(&got[0], him).iter().any(|m| m.0)),
        "he waits 20 yd out"
    );
    let stopped = moves(&p.seen(0)[mark..], him)
        .into_iter()
        .find(|m| m.0)
        .unwrap()
        .1;
    // Out of the wolf's reach, inside the leash.
    p.go(0, at(stopped[0] + 35.0, 0.0), 0.0);
    p.run(0.5);
    let group = Audience::of(&[p.guids[0]]);
    let map = p.realm.player_positions()[0].0;
    p.realm
        .spawn_for(
            p.host.as_ref(),
            PRAIRIE_WOLF,
            60,
            at(stopped[0] - 2.0, 0.0),
            map,
            group.clone(),
        )
        .expect("a wolf");
    assert!(
        p.run_until(60.0, |got| system_lines(&got[0])
            .contains(&FAILED.to_string())),
        "the wolf kills him and the escort fails"
    );
    assert!(!quest_failed(p.seen(0)), "the quest is not failed");
    // The wolf goes, so the next try starts clear of it; the escort made nothing for the group.
    p.realm.despawn_audience(&group);
    p.go(0, beside(), 0.0);
    p.run(5.5);
    assert!(offered(&mut p, 0, START), "the quest is still in the log");
    let again = start(&mut p, 0);
    assert_ne!(again, him);
}

#[test]
fn only_a_holder_with_the_escort_to_do_outside_a_group_is_offered_the_line() {
    let here = beside();
    let mut p = party(
        &[Holds::Nothing, Holds::Met, Holds::Quest, Holds::Quest],
        &[here, here, here, here],
    );
    assert!(!offered(&mut p, 0, START), "without the quest");
    assert!(!offered(&mut p, 1, START), "with the escort met");
    assert!(offered(&mut p, 3, START));
    let him = start(&mut p, 2);
    assert!(created(p.seen(3), him), "the fourth is in the group");
    assert!(!offered(&mut p, 3, START), "a group member, at the start");
    assert!(!offered(&mut p, 3, him), "a group member, at the escortee");
}
