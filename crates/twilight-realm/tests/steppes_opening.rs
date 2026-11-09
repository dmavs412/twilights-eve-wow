//! The break (#631, spec #626): Toren's two quests in Lakeshire's inn, and the encounter at the
//! Ilgalar Anchor that runs in the tower's Phase and reaches `ilgalar_anchor_broken` when its last
//! wave is dead. The load check on an encounter's `records` is in `src/encounters.rs`, which no
//! test seam reaches.

mod common;

use std::collections::HashSet;

use benilla_protocol::messages::CHAT_MSG_SYSTEM;
use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;
use twilight_realm::Storage;

const HIGHGUID_UNIT: u64 = 0xF130;
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
/// Toren in Lakeshire's inn (`hub.toml` 9300083), and a spot a couple of yards east of him.
const TOREN: u64 = HIGHGUID_UNIT << 48 | 190083 << 24 | 9300083;
const BY_TOREN: [f32; 3] = [-9226.0, -2162.0, 63.73];
const TRANSLATION: u32 = 99501;
const BRING_IT_DOWN: u32 = 99502;
const BROKEN: &str = "ilgalar_anchor_broken";

/// The whole Anchor and the broken one, on Ilgalar's top floor.
const WHOLE: u64 = HIGHGUID_GAMEOBJECT << 48 | 794101;
const SPLIT: u64 = HIGHGUID_GAMEOBJECT << 48 | 794102;
/// 3 yd west of the Anchor, inside the start radius of 10; and 4 yd east of it.
const BY_ANCHOR: [f32; 3] = [-9291.0, -3308.0, 150.1];
const BESIDE_ANCHOR: [f32; 3] = [-9285.0, -3306.0, 150.1];
/// On the ground south of the tower, outside its footprint and its Phase.
const OUTSIDE: [f32; 3] = [-9288.0, -3340.0, 113.0];
/// Far from the tower in the hills.
const FAR_OUTSIDE: [f32; 3] = [-9150.0, -3420.0, 100.0];

const RETUNER: u32 = 97511;
const SERVANT: u32 = 97512;
const WARDEN: u32 = 97513;
const ABANDONED: &str = "You have abandoned the defense.";

/// `levels.len()` Zealots (Tier 2, so quests past level 30 are offered) from Northshire; those
/// listed in `holders` hold Bring It Down.
fn party(levels: &[u8], holders: &'static [usize]) -> Party {
    let mut p = Party::edited(
        Realm::with_seed(1),
        levels,
        START_ELWYNN,
        Box::new(NoGround),
        move |i, blob| {
            let mut blob = blob.replace("calling = \"basic_warrior\"", "calling = \"zealot\"");
            if holders.contains(&i) {
                blob.push_str(&format!(
                    "\n[[quests]]\nid = {BRING_IT_DOWN}\nslot = 0\nkills = [0, 0, 0, 0]\n"
                ));
            }
            blob
        },
    );
    p.run(0.5);
    p
}

/// The quests Toren offers player `i` now.
fn offered(p: &mut Party, i: usize) -> Vec<u32> {
    let from = p.seen(i).len();
    p.cmd(i, RealmCommand::QuestGiverHello { npc: TOREN });
    p.seen(i)[from..]
        .iter()
        .flat_map(|e| match e {
            SessionEvent::QuestGreeting(list) => list.quests.iter().map(|q| q.quest_id).collect(),
            SessionEvent::QuestDetail(d) => vec![d.quest_id],
            _ => Vec::new(),
        })
        .collect()
}

/// Player `i` hands `quest` in to Toren; panics if it did not go through.
fn hand_in(p: &mut Party, i: usize, quest: u32) {
    let from = p.seen(i).len();
    for command in [
        RealmCommand::QuestGiverCompleteQuest { npc: TOREN, quest },
        RealmCommand::QuestGiverRequestReward { npc: TOREN, quest },
        RealmCommand::QuestGiverChooseReward {
            npc: TOREN,
            quest,
            choice: 0,
        },
    ] {
        p.cmd(i, command);
    }
    let done = p.seen(i)[from..]
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestComplete(c) if c.quest_id == quest));
    assert!(done, "{quest}: {:?}", &p.seen(i)[from..]);
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

/// The wave creatures of `entry` player `i`'s client has been sent: guid and where it came.
fn wave(p: &Party, i: usize, entry: u32) -> Vec<(u64, [f32; 3])> {
    unit_creates(p.seen(i))
        .into_iter()
        .filter(|(_, e, _)| *e == entry)
        .map(|(g, _, at)| (g, at))
        .collect()
}

/// The texts of the system lines player `i` has been sent.
fn system_lines(p: &Party, i: usize) -> Vec<String> {
    p.seen(i)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Chat(m) if m.chat_type == CHAT_MSG_SYSTEM => Some(m.text.clone()),
            _ => None,
        })
        .collect()
}

/// The last max health player `i`'s client was sent for `guid`.
fn max_health(p: &Party, i: usize, guid: u64) -> u32 {
    p.seen(i)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g, fields, ..
            }
            | SessionEvent::ObjectValues { guid: g, fields }
                if *g == guid =>
            {
                fields.unit_max_health()
            }
            _ => None,
        })
        .last()
        .expect("a max health")
}

/// The Milestones state files the realm handed the host since the last drain.
fn milestone_writes(p: &mut Party) -> usize {
    p.realm
        .drain_storage()
        .into_iter()
        .filter(|s| matches!(s, Storage::Milestones { .. }))
        .count()
}

/// Kills every wave creature but the Anchorwarden as it comes, until the Anchorwarden comes;
/// its guid and where it came.
fn clear_to_the_warden(p: &mut Party) -> (u64, [f32; 3]) {
    let mut killed = HashSet::new();
    for _ in 0..1500 {
        p.run(0.1);
        let mut warden = None;
        for (guid, entry, at) in unit_creates(p.seen(0)) {
            if entry == WARDEN {
                warden = Some((guid, at));
            } else if (entry == RETUNER || entry == SERVANT) && killed.insert(guid) {
                p.realm.kill_creature(guid);
            }
        }
        if let Some(w) = warden {
            return w;
        }
    }
    panic!("the Anchorwarden never came");
}

/// Player 0 kills the Anchorwarden at `at`, brought to its last sliver of health.
fn kill_the_warden(p: &mut Party, warden: u64, at: [f32; 3]) {
    p.realm.set_creature_health_pct(warden, 1);
    p.realm.place_creature(warden, at);
    p.go_facing(0, [at[0] + 1.5, at[1], at[2]], at);
    p.cmd(0, RealmCommand::AttackSwing { guid: warden });
    assert!(
        p.run_until(20.0, |got| died(&got[0], warden)),
        "the Anchorwarden died"
    );
}

/// Whether player `i` was sent a kill credit for Bring It Down.
fn credited(p: &Party, i: usize) -> bool {
    p.seen(i).iter().any(|e| {
        matches!(e, SessionEvent::QuestObjectiveKill { quest_id, entry, .. }
            if *quest_id == BRING_IT_DOWN && *entry == WARDEN)
    })
}

#[test]
fn toren_offers_the_translation_then_the_break_until_the_anchor_is_broken() {
    let mut p = party(&[48], &[]);
    p.go(0, BY_TOREN, std::f32::consts::PI);
    p.run(0.5);
    let first = offered(&mut p, 0);
    assert!(first.contains(&TRANSLATION), "{first:?}");
    assert!(!first.contains(&BRING_IT_DOWN), "{first:?}");

    p.cmd(
        0,
        RealmCommand::QuestGiverAcceptQuest {
            npc: TOREN,
            quest: TRANSLATION,
        },
    );
    hand_in(&mut p, 0, TRANSLATION);
    assert!(offered(&mut p, 0).contains(&BRING_IT_DOWN));

    p.realm.reach_milestone(BROKEN);
    p.run(0.1);
    assert!(!offered(&mut p, 0).contains(&BRING_IT_DOWN));
}

#[test]
fn holding_the_quest_at_the_anchor_brings_the_first_wave_into_the_run_only() {
    let mut p = party(&[50, 50], &[0]);
    p.go(1, OUTSIDE, 0.0);
    p.go(0, BY_ANCHOR, 0.0);
    p.run(8.0);
    assert!(
        wave(&p, 0, RETUNER).is_empty(),
        "the first wave is 10 s out"
    );
    p.run(3.0);
    assert_eq!(wave(&p, 0, RETUNER).len(), 3);
    assert!(wave(&p, 1, RETUNER).is_empty(), "seen from the open world");
}

#[test]
fn a_second_character_in_the_phase_scales_the_wave_by_the_head_count() {
    let alone = {
        let mut p = party(&[50], &[0]);
        p.go(0, BY_ANCHOR, 0.0);
        p.run(12.0);
        let (guid, _) = wave(&p, 0, RETUNER)[0];
        max_health(&p, 0, guid)
    };
    let mut p = party(&[50, 50], &[0]);
    p.go(1, BESIDE_ANCHOR, 0.0);
    p.go(0, BY_ANCHOR, 0.0);
    p.run(12.0);
    let (guid, _) = wave(&p, 0, RETUNER)[0];
    let two = max_health(&p, 0, guid);
    assert_eq!(two, (alone as f32 * 1.5).round() as u32, "{alone} alone");
}

#[test]
fn killing_every_wave_breaks_the_anchor_and_credits_everyone_near() {
    let mut p = party(&[50, 50], &[0, 1]);
    p.go(1, BESIDE_ANCHOR, 0.0);
    p.go(0, BY_ANCHOR, 0.0);
    p.run(1.0);
    assert!(shows(&p, 0, WHOLE) && !shows(&p, 0, SPLIT));
    milestone_writes(&mut p);

    let (warden, at) = clear_to_the_warden(&mut p);
    assert!(
        !p.realm.milestone_reached(BROKEN),
        "broken before the last wave died"
    );
    kill_the_warden(&mut p, warden, at);
    p.run(0.2);
    assert!(p.realm.milestone_reached(BROKEN));
    assert_eq!(milestone_writes(&mut p), 1);
    assert!(!shows(&p, 0, WHOLE), "the whole Anchor stayed");
    assert!(shows(&p, 0, SPLIT), "the broken Anchor did not come");
    assert!(credited(&p, 0) && credited(&p, 1));
}

#[test]
fn after_the_break_a_holder_still_runs_it_and_finishes_the_quest() {
    let mut p = party(&[50], &[0]);
    p.realm.reach_milestone(BROKEN);
    p.run(0.1);
    milestone_writes(&mut p);

    p.go(0, BY_ANCHOR, 0.0);
    let (warden, at) = clear_to_the_warden(&mut p);
    kill_the_warden(&mut p, warden, at);
    p.run(0.2);
    assert!(credited(&p, 0));
    assert_eq!(
        milestone_writes(&mut p),
        0,
        "the Milestone was written again"
    );

    p.go(0, BY_TOREN, std::f32::consts::PI);
    p.run(0.5);
    hand_in(&mut p, 0, BRING_IT_DOWN);
}

#[test]
fn leaving_the_phase_with_the_waves_under_way_abandons_it() {
    let mut p = party(&[50, 50], &[0]);
    p.go(1, BESIDE_ANCHOR, 0.0);
    p.go(0, BY_ANCHOR, 0.0);
    p.run(11.0);
    let first: Vec<u64> = wave(&p, 1, RETUNER).into_iter().map(|(g, _)| g).collect();
    assert_eq!(first.len(), 3);

    p.go(0, FAR_OUTSIDE, 0.0);
    p.run(1.0);
    let lines = system_lines(&p, 1);
    assert!(lines.iter().any(|l| l == ABANDONED), "{lines:?}");
    let held = p.units_held(1);
    assert!(
        first.iter().all(|g| !held.contains(g)),
        "the creatures stayed"
    );
}

#[test]
fn toren_s_quest_text_keeps_to_the_bible() {
    let text = include_str!("../content/quests_blackrock_pass.toml");
    assert!(!text.contains('\u{2014}'), "an em dash");
    for name in ["Prestor", "Cedric", "Sareth", "Deathwing", "Lich King"] {
        assert!(!text.contains(name), "{name}");
    }
}
