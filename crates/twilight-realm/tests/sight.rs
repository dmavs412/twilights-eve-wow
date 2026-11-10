//! Sight by quest state (#596, ADR 0035), on the test rows that belong to this file: Rachel
//! (`content/zones.toml`, Redridge guid 960811) is seen only before test quest 99811 is handed
//! in, and the Sight Tester (`content/hub.toml`, entry 199811) 10 yd from her only after. Quest
//! 99811, given and ended by Rachel, and 99812, which she also gives, are added here with
//! `Realm::with_quests`.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

const HIGHGUID_UNIT: u64 = 0xF130;
const fn unit(entry: u64, low: u64) -> u64 {
    HIGHGUID_UNIT << 48 | entry << 24 | low
}
const RACHEL: u64 = unit(849, 960811);
const TESTER: u64 = unit(199811, 9399811);
const QUEST: u32 = 99811;
/// Between the two, within talking reach of both.
const BETWEEN: [f32; 3] = [-8845.0, -2960.0, 278.0];

const QUESTS: &str = r#"
[[quest]]
id = 99811
kind = "story"
giver = 849
level = 5
title = "Seen and Unseen"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0

[[quest]]
id = 99812
kind = "story"
giver = 849
level = 5
title = "Still Here"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
"#;

/// Two level 10 characters between Rachel and the Sight Tester: the first as `first` leaves
/// their save, the second with 99811 handed in.
fn party(first: impl Fn(String) -> String) -> Party {
    let realm = realm().with_quests(QUESTS);
    let mut p = Party::edited(
        realm,
        &[10, 10],
        START_ELWYNN,
        Box::new(NoGround),
        move |i, blob| {
            if i == 0 {
                first(blob)
            } else {
                blob.replace("xp = 0\n", &format!("xp = 0\ncompleted = [{QUEST}]\n"))
            }
        },
    );
    p.go(0, BETWEEN, 0.0);
    p.go(1, BETWEEN, 0.0);
    p.run(2.0);
    p
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

/// Whether player `i` is sent a gossip menu for `npc` when they greet it.
fn gossip(p: &mut Party, i: usize, npc: u64) -> bool {
    let before = p.seen(i).len();
    p.cmd(i, RealmCommand::GossipHello { guid: npc });
    p.seen(i)[before..]
        .iter()
        .any(|e| matches!(e, SessionEvent::GossipMenu { npc: n, .. } if *n == npc))
}

#[test]
fn the_test_rows_stand_on_open_land() {
    assert!(!twilight_realm::closed_at(0, BETWEEN[0], BETWEEN[1], &[]));
}

#[test]
fn each_character_is_sent_only_what_their_hand_ins_show() {
    let p = party(|blob| blob);
    assert!(p.units_held(0).contains(&RACHEL), "99811 not handed in");
    assert!(!p.units_held(0).contains(&TESTER));
    assert!(p.units_held(1).contains(&TESTER), "99811 handed in");
    assert!(!p.units_held(1).contains(&RACHEL));
    assert!(!created(p.seen(0), TESTER) && !created(p.seen(1), RACHEL));
}

#[test]
fn a_hand_in_takes_rachel_away_and_brings_the_tester() {
    let mut p = party(|mut blob| {
        blob.push_str(&format!(
            "\n[[quests]]\nid = {QUEST}\nslot = 0\nkills = [0, 0, 0, 0]\n"
        ));
        blob
    });
    assert!(p.units_held(0).contains(&RACHEL));
    let other = p.units_held(1);
    let before = p.seen(1).len();
    p.cmd(
        0,
        RealmCommand::QuestGiverChooseReward {
            npc: RACHEL,
            quest: QUEST,
            choice: 0,
        },
    );
    let mut got = Vec::new();
    let swapped = p.run_until(2.0, |events| {
        got.extend_from_slice(&events[0]);
        removed(&got, RACHEL) && created(&got, TESTER)
    });
    assert!(swapped, "Rachel removed and the Tester created: {got:?}");
    assert!(!p.units_held(0).contains(&RACHEL) && p.units_held(0).contains(&TESTER));
    p.run(1.0);
    assert_eq!(p.units_held(1), other, "the other's view is as it was");
    let after = &p.seen(1)[before..];
    assert!(!removed(after, TESTER) && !created(after, RACHEL));
}

#[test]
fn neither_serves_a_character_who_does_not_see_it() {
    let mut p = party(|blob| blob);
    assert!(!gossip(&mut p, 0, TESTER), "the Tester, before 99811");
    assert!(gossip(&mut p, 1, TESTER), "the Tester, after 99811");
    assert!(!gossip(&mut p, 1, RACHEL), "Rachel, after 99811");
    assert!(gossip(&mut p, 0, RACHEL), "Rachel, before 99811");
}
