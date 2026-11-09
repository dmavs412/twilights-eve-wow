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

// ── The Blackrock Pass Watch camp (#632) ─────────────────────────────────────────────────────────

/// Captain Sela Marrow (`hub.toml` 9300091) and the camp's Writ board (`writs.toml` 794001).
const CAPTAIN: u64 = HIGHGUID_UNIT << 48 | 190090 << 24 | 9300091;
const CAMP_BOARD: u64 = HIGHGUID_GAMEOBJECT << 48 | 794001;
/// Stormwind's Writ Board in the Trade District (`writs.toml` 791001), and a spot beside it.
const CITY_BOARD: u64 = HIGHGUID_GAMEOBJECT << 48 | 791001;
const BY_CITY_BOARD: [f32; 3] = [-8820.0, 662.0, 95.43];
/// In the camp, a couple of yards from the captain, and a yard from the board.
const IN_CAMP: [f32; 3] = [-8745.0, -2600.0, 133.0];
const BY_CAPTAIN: [f32; 3] = [-8736.0, -2592.0, 132.83];
const BY_CAMP_BOARD: [f32; 3] = [-8731.0, -2586.0, 132.61];
/// The camp's eight creature templates (`mobs.toml`).
const CAMP_CREATURES: std::ops::RangeInclusive<u32> = 97501..=97508;
const BOUNTIES: [u32; 3] = [99511, 99512, 99513];
const CAMP_WRITS: [u32; 3] = [99521, 99522, 99523];
const STEPPES: &str = "steppes_open";
/// An hour into the Writ Board's window 0 (2026-11-02T00:00Z), in Unix ms.
const WRIT_WINDOW_0: u64 = 1_793_577_600_000 + 3_600_000;
const WRIT_WINDOW_MS: u64 = 48 * 3_600_000;

/// `levels.len()` level 50 Zealots from Northshire, each holding `held`: a quest and its kill
/// counts, every objective met.
fn camp_party(levels: &[u8], held: &'static [(u32, [u32; 4])]) -> Party {
    let mut p = Party::edited(
        Realm::with_seed(1),
        levels,
        START_ELWYNN,
        Box::new(NoGround),
        move |_, blob| {
            let mut blob = blob.replace("calling = \"basic_warrior\"", "calling = \"zealot\"");
            for (slot, (id, kills)) in held.iter().enumerate() {
                blob.push_str(&format!(
                    "\n[[quests]]\nid = {id}\nslot = {slot}\nkills = {kills:?}\n"
                ));
            }
            blob
        },
    );
    p.realm.set_wall_clock(WRIT_WINDOW_0);
    p.run(0.5);
    p
}

/// The quests player `i` is offered in answer to `command`.
fn listed(p: &mut Party, i: usize, command: RealmCommand) -> Vec<u32> {
    let from = p.seen(i).len();
    p.cmd(i, command);
    p.seen(i)[from..]
        .iter()
        .flat_map(|e| match e {
            SessionEvent::QuestGreeting(list) => list.quests.iter().map(|q| q.quest_id).collect(),
            SessionEvent::QuestDetail(d) => vec![d.quest_id],
            _ => Vec::new(),
        })
        .collect()
}

/// Player `i` hands `quest` in to the captain; the money it paid. Panics if it did not go
/// through.
fn hand_in_to_captain(p: &mut Party, i: usize, quest: u32) -> u32 {
    let from = p.seen(i).len();
    for command in [
        RealmCommand::QuestGiverCompleteQuest {
            npc: CAPTAIN,
            quest,
        },
        RealmCommand::QuestGiverRequestReward {
            npc: CAPTAIN,
            quest,
        },
        RealmCommand::QuestGiverChooseReward {
            npc: CAPTAIN,
            quest,
            choice: 0,
        },
    ] {
        p.cmd(i, command);
    }
    p.seen(i)[from..]
        .iter()
        .find_map(|e| match e {
            SessionEvent::QuestComplete(c) if c.quest_id == quest => Some(c.money),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{quest}: {:?}", &p.seen(i)[from..]))
}

/// The Milestones state files the realm handed the host since the last drain.
fn milestone_blobs(p: &mut Party) -> Vec<String> {
    p.realm
        .drain_storage()
        .into_iter()
        .filter_map(|s| match s {
            Storage::Milestones { blob } => Some(blob),
            _ => None,
        })
        .collect()
}

#[test]
fn the_camp_comes_when_the_anchor_breaks() {
    let mut p = camp_party(&[50], &[]);
    p.go(0, IN_CAMP, 0.0);
    p.run(2.0);
    let camp = |p: &Party| {
        unit_creates(p.seen(0))
            .into_iter()
            .filter(|(_, e, _)| CAMP_CREATURES.contains(e))
            .map(|(_, e, _)| e)
            .collect::<HashSet<u32>>()
    };
    assert!(!shows(&p, 0, CAPTAIN), "the captain before the break");
    assert!(!shows(&p, 0, CAMP_BOARD), "the board before the break");
    assert!(camp(&p).is_empty(), "{:?} before the break", camp(&p));

    p.realm.reach_milestone(BROKEN);
    p.run(0.1);
    assert!(shows(&p, 0, CAPTAIN), "the captain did not come");
    assert!(shows(&p, 0, CAMP_BOARD), "the board did not come");
    p.run(2.0);
    // Every template has a spawn within sight (100 yd) of the camp.
    assert_eq!(camp(&p), CAMP_CREATURES.collect::<HashSet<u32>>());
}

#[test]
fn the_captain_offers_the_bounties_and_the_camp_board_one_writ() {
    let mut p = camp_party(&[50], &[]);
    p.realm.reach_milestone(BROKEN);
    p.go(0, BY_CAPTAIN, 0.0);
    p.run(0.5);
    let bounties = listed(&mut p, 0, RealmCommand::QuestGiverHello { npc: CAPTAIN });
    for id in BOUNTIES {
        assert!(bounties.contains(&id), "{id} in {bounties:?}");
    }

    p.go(0, BY_CAMP_BOARD, 0.0);
    p.run(0.5);
    let mut seen = HashSet::new();
    for window in 0..12 {
        p.realm
            .set_wall_clock(WRIT_WINDOW_0 + window * WRIT_WINDOW_MS);
        let board = listed(&mut p, 0, RealmCommand::UseGameObject { guid: CAMP_BOARD });
        assert_eq!(board.len(), 1, "window {window}: {board:?}");
        assert!(CAMP_WRITS.contains(&board[0]), "window {window}: {board:?}");
        seen.insert(board[0]);
    }
    assert_eq!(seen, HashSet::from(CAMP_WRITS), "in 12 windows");

    p.go(0, BY_CITY_BOARD, 0.0);
    p.run(0.5);
    let city = listed(&mut p, 0, RealmCommand::UseGameObject { guid: CITY_BOARD });
    assert!(!city.is_empty(), "the city board offered nothing");
    assert!(city.iter().all(|id| !CAMP_WRITS.contains(id)), "{city:?}");
}

#[test]
fn camp_hand_ins_feed_the_tally_until_the_steppes_open() {
    const HELD: &[(u32, [u32; 4])] = &[(99511, [12, 0, 0, 0]), (99521, [50, 50, 1, 0])];
    let mut p = camp_party(&[50, 50], HELD);
    p.realm.reach_milestone(BROKEN);
    for i in 0..2 {
        p.go(i, BY_CAPTAIN, 0.0);
    }
    p.run(0.5);
    milestone_blobs(&mut p);

    let paid = hand_in_to_captain(&mut p, 0, 99511);
    p.run(0.1);
    let blobs = milestone_blobs(&mut p);
    assert_eq!(blobs.len(), 1, "{blobs:?}");
    assert!(
        blobs[0].contains(&format!("[{STEPPES}]\ncount = 15\n")),
        "{}",
        blobs[0]
    );
    hand_in_to_captain(&mut p, 0, 99521);
    p.run(0.1);
    let blobs = milestone_blobs(&mut p);
    assert!(
        blobs.len() == 1 && blobs[0].contains(&format!("[{STEPPES}]\ncount = 135\n")),
        "{blobs:?}"
    );

    p.realm.reach_milestone(STEPPES);
    p.run(0.1);
    milestone_blobs(&mut p);
    assert_eq!(hand_in_to_captain(&mut p, 1, 99511), paid);
    hand_in_to_captain(&mut p, 1, 99521);
    p.run(0.1);
    assert!(milestone_blobs(&mut p).is_empty(), "the Tally counted on");
}

#[test]
fn toren_s_quest_text_keeps_to_the_bible() {
    let text = include_str!("../content/quests_blackrock_pass.toml");
    assert!(!text.contains('\u{2014}'), "an em dash");
    for name in ["Prestor", "Cedric", "Sareth", "Deathwing", "Lich King"] {
        assert!(!text.contains(name), "{name}");
    }
}
