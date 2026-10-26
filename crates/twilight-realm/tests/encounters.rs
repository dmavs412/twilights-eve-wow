//! Encounters (#238): waves of creatures at a point in the open world, started by a quest step.
//! `content/encounters.toml` ships two test encounters. `test_defense` (quest 99801, started
//! within 30 yd of its anchor at (-9720, 20)) has a wave of two Prairie Wolves at 10 s and one
//! more 60 s after, or sooner when the first two are dead. `test_rite` (quest 99802, started by
//! the last use objective) has one wolf at 5 s. The quests are added here with
//! `Realm::with_quests`.

mod common;

use std::collections::HashSet;

use benilla_protocol::messages::{CHAT_MSG_RAID_WARNING, CHAT_MSG_SYSTEM};
use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

const WOLF: u32 = 2958;
const DEFENSE: u32 = 99801;
const RITE: u32 = 99802;
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
const STONE: u64 = HIGHGUID_GAMEOBJECT << 48 | 990801;
const ANCHOR: [f32; 3] = [-9720.0, 20.0, 47.2];
/// 10 yd from the anchor, inside the start radius of 30.
const NEAR: [f32; 3] = [-9710.0, 20.0, 47.2];
/// Far past the start radius and the leash of 60.
const FAR: [f32; 3] = [-9720.0, 220.0, 47.2];
const WARNING: &str = "The test is coming!";
const ABANDONED: &str = "You have abandoned the defense.";

const QUESTS: &str = r#"
[[quest]]
id = 99801
kind = "story"
giver = 197
level = 5
title = "Hold the Line"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0

[[gameobject]]
guid = 990801
entry = 990801
display = 1
map = 0
position = [-9720.0, 119.0, 47.2]
respawn = 5.0

[[quest]]
id = 99802
kind = "story"
giver = 197
level = 5
title = "The Rite"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
use = [[990801, 1]]
"#;

/// Two level 60 characters, the first holding `quest`, both far from the anchor.
fn party(quest: u32) -> Party {
    let realm = realm().with_quests(QUESTS);
    Party::edited(
        realm,
        &[60, 60],
        START_ELWYNN,
        Box::new(NoGround),
        move |i, mut blob| {
            if i == 0 {
                blob.push_str(&format!(
                    "\n[[quests]]\nid = {quest}\nslot = 0\nkills = [0, 0, 0, 0]\n"
                ));
            }
            blob
        },
    )
}

/// The wolves player `i`'s client has been sent.
fn wolves(p: &Party, i: usize) -> HashSet<u64> {
    unit_creates(p.seen(i))
        .into_iter()
        .filter(|(_, entry, _)| *entry == WOLF)
        .map(|(guid, _, _)| guid)
        .collect()
}

/// The texts of the chat lines of `chat_type` player `i` has been sent.
fn said(p: &Party, i: usize, chat_type: u8) -> Vec<String> {
    p.seen(i)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Chat(m) if m.chat_type == chat_type => Some(m.text.clone()),
            _ => None,
        })
        .collect()
}

/// Ticks until player 0's client holds `n` wolves in all, at most `secs`; the seconds it took.
fn until_wolves(p: &mut Party, n: usize, secs: f32) -> Option<f32> {
    let mut t = 0.0;
    while t < secs {
        p.run(0.1);
        t += 0.1;
        if wolves(p, 0).len() >= n {
            return Some(t);
        }
    }
    None
}

/// Player 0 kills the wolf `guid`, which must come to them at `ANCHOR`.
fn kill(p: &mut Party, guid: u64, from: [f32; 3]) {
    p.go_facing(0, ANCHOR, from);
    p.cmd(0, RealmCommand::AttackSwing { guid });
    assert!(
        p.run_until(20.0, |got| died(&got[0], guid)),
        "the wolf died"
    );
}

#[test]
fn accepting_near_the_anchor_starts_it_and_far_away_does_not_until_they_walk_up() {
    let mut p = party(DEFENSE);
    p.go(1, NEAR, 0.0);
    p.run(15.0);
    assert!(said(&p, 0, CHAT_MSG_RAID_WARNING).is_empty());
    assert!(wolves(&p, 0).is_empty() && wolves(&p, 1).is_empty());

    p.go(0, NEAR, 0.0);
    p.run(1.0);
    assert_eq!(said(&p, 0, CHAT_MSG_RAID_WARNING), [WARNING]);
    assert_eq!(
        said(&p, 1, CHAT_MSG_RAID_WARNING),
        [WARNING],
        "everyone within the leash hears it"
    );
    p.run(7.0);
    assert!(wolves(&p, 0).is_empty(), "the first wave is 10 s out");
    p.run(4.0);
    assert_eq!(wolves(&p, 0).len(), 2);
    assert_eq!(wolves(&p, 1), wolves(&p, 0), "visible to everyone");
}

#[test]
fn wave_two_comes_when_wave_one_is_dead() {
    let mut p = party(DEFENSE);
    p.go(0, ANCHOR, 0.0);
    until_wolves(&mut p, 2, 20.0).expect("wave one");
    let first = wolves(&p, 0);
    p.run(1.0);
    for guid in first {
        let from = unit_creates(p.seen(0))
            .into_iter()
            .find(|(g, _, _)| *g == guid)
            .map(|(_, _, at)| at)
            .unwrap();
        kill(&mut p, guid, from);
    }
    let secs = until_wolves(&mut p, 3, 30.0).expect("wave two");
    assert!(secs < 5.0, "wave two came {secs} s after the kills");
}

#[test]
fn wave_two_comes_after_the_timer_when_wave_one_lives() {
    let mut p = party(DEFENSE);
    p.go(0, ANCHOR, 0.0);
    until_wolves(&mut p, 2, 20.0).expect("wave one");
    p.run(50.0);
    assert_eq!(wolves(&p, 0).len(), 2, "60 s have not passed");
    until_wolves(&mut p, 3, 20.0).expect("wave two after 60 s");
}

#[test]
fn with_every_questing_character_past_the_leash_the_creatures_go_and_it_starts_again() {
    let mut p = party(DEFENSE);
    p.go(1, NEAR, 0.0);
    p.go(0, ANCHOR, 0.0);
    until_wolves(&mut p, 2, 20.0).expect("wave one");
    let first = wolves(&p, 1);
    assert_eq!(first.len(), 2);

    p.go(0, FAR, 0.0);
    p.run(2.0);
    assert_eq!(said(&p, 1, CHAT_MSG_SYSTEM), [ABANDONED]);
    assert!(
        first.iter().all(|g| !p.units_held(1).contains(g)),
        "the creatures despawned"
    );

    p.go(0, NEAR, 0.0);
    p.run(1.0);
    assert_eq!(
        said(&p, 0, CHAT_MSG_RAID_WARNING).len(),
        2,
        "it starts again at once"
    );
    until_wolves(&mut p, 4, 20.0).expect("a new first wave");
}

#[test]
fn a_use_done_encounter_starts_on_the_last_use_credit() {
    let mut p = party(RITE);
    p.go(0, [-9720.0, 118.0, 47.2], 0.0);
    p.run(15.0);
    assert!(wolves(&p, 0).is_empty(), "nothing starts before the use");
    p.cmd(0, RealmCommand::UseGameObject { guid: STONE });
    p.run(3.0);
    assert!(wolves(&p, 0).is_empty(), "the wave is 5 s out");
    p.run(4.0);
    assert_eq!(wolves(&p, 0).len(), 1);
}
