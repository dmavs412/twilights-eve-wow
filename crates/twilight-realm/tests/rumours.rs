//! Rumours (spec #284): a kill rolls one, the looter finds the note, the Audience is told where
//! the giver waits, the giver offers the first step to the Audience only, and the timer, the
//! cap and the hand-ins end it. Each test works in an area of its own, because the Rumours the
//! tests add are the process's.

mod common;

use benilla_protocol::messages::LootItem;
use benilla_protocol::SessionEvent;
use common::party::*;
use common::*;
use twilight_realm::{Audience, CROSSROADS};

const WOLF: u32 = 2958;
/// A second creature, for the Rumour that only a dropper rolls.
const YOUNG_WOLF: u32 = 299;
/// A creature no test kills: the dropper that must not match.
const OTHER_CREATURE: u32 = 69;
/// Corina Steele: any friendly template will do for a giver.
const GIVER: u32 = 54;
const GIVER_NAME: &str = "Corina Steele";

/// Where area `k` is: far out past The Barrens' camps, 4000 yards from the next area.
fn base(k: usize) -> [f32; 3] {
    [
        CROSSROADS[0] + 3000.0,
        CROSSROADS[1] + 4000.0 * k as f32,
        0.0,
    ]
}

fn at(k: usize, dx: f32, dy: f32) -> [f32; 3] {
    let b = base(k);
    [b[0] + dx, b[1] + dy, 0.0]
}

/// The ids of Rumour `j` of area `k`: the Rumour, its quest and its two notes.
fn ids(k: usize, j: usize) -> [u32; 4] {
    let first = 4_000_000 + 100 * k as u32 + 10 * j as u32;
    [first, first + 1, first + 2, first + 3]
}

/// The quests and the Rumours of area `k`: `count` Rumours with the same bounds, each with one
/// quest and a note per point. `chance` and `droppers` are the Rumours'.
fn content(k: usize, count: usize, chance: f32, droppers: &str) -> (String, String) {
    let (mut quests, mut rumours) = (String::new(), String::new());
    let [x, y, _] = base(k);
    let (north, south) = (at(k, -30.0, 20.0), at(k, 30.0, 20.0));
    for j in 0..count {
        let [rumour, quest, note_a, note_b] = ids(k, j);
        for (note, landmark) in [(note_a, "the north rise"), (note_b, "the south rise")] {
            quests += &format!(
                r#"
[[item]]
entry = {note}
name = "Note {note}"
quality = 1
item_level = 5
class = 12
display_id = 6270
max_count = 1
description = "{GIVER_NAME} waits at {landmark}."
"#
            );
        }
        quests += &format!(
            r#"
[[quest]]
id = {quest}
kind = "rumour"
giver = {GIVER}
ender = {GIVER}
level = 1
title = "Rumour {rumour}"
objectives = "Hear {GIVER_NAME} out."
details = "-"
progress = "-"
completion = "-"
money = 0
tokens = 0
"#
        );
        rumours += &format!(
            r#"
[[rumour]]
id = {rumour}
name = "Rumour {rumour}"
grade = "easy"
map = {KALIMDOR}
bounds = [[{}, {}, {}, {}]]
chance = {chance}
droppers = [{droppers}]
giver = {GIVER}
points = [
  {{ x = {}, y = {}, landmark = "the north rise", note = {note_a} }},
  {{ x = {}, y = {}, landmark = "the south rise", note = {note_b} }},
]
quests = [{quest}]
"#,
            x - 100.0,
            y - 100.0,
            x + 100.0,
            y + 100.0,
            north[0],
            north[1],
            south[0],
            south[1],
        );
    }
    (quests, rumours)
}

fn realm_with(k: usize, count: usize, chance: f32, droppers: &str) -> Realm {
    let (quests, rumours) = content(k, count, chance, droppers);
    Realm::with_seed(7)
        .with_quests(&quests)
        .with_rumours(&rumours)
}

/// Three warriors: the first at the area's base, the second 10 yards from it, the third 90.
fn party(realm: Realm, k: usize) -> (Party, u32) {
    let mut party = Party::new(realm, &[5, 5, 5]);
    party.go(0, at(k, 0.0, 0.0), 0.0);
    party.go(1, at(k, 10.0, 0.0), 0.0);
    party.go(2, at(k, 90.0, 0.0), 0.0);
    // A tick for the characters to come out on the map their position is on.
    party.run(0.5);
    let map = party.realm.player_positions()[0].0;
    (party, map)
}

/// Player `i` kills a level 1 `entry` creature standing 3 yards from them and 3 yards short of
/// where player `i` stands. Its corpse guid.
fn kill(party: &mut Party, map: u32, k: usize, i: usize, entry: u32, dy: f32) -> u64 {
    let spot = at(k, 0.0, dy);
    let audience = Audience::of(&[party.guids[i]]);
    let wolf = party
        .realm
        .spawn_for(
            party.host.as_ref(),
            entry,
            1,
            [spot[0] + 3.0, spot[1], 0.0],
            map,
            audience,
        )
        .expect("a creature");
    party.go_facing(i, spot, [spot[0] + 3.0, spot[1], 0.0]);
    party.run(0.3);
    party.cmd(i, RealmCommand::AttackSwing { guid: wolf });
    let dead = party.run_until(30.0, |got| died(&got[i], wolf));
    assert!(dead, "the creature never died");
    party.run(0.5);
    wolf
}

/// What player `i`'s loot window on `corpse` shows.
fn loot(party: &mut Party, i: usize, corpse: u64) -> Vec<LootItem> {
    for _ in 0..3 {
        let before = party.seen(i).len();
        party.cmd(i, RealmCommand::Loot { guid: corpse });
        let window = party.seen(i)[before..].iter().find_map(|e| match e {
            SessionEvent::LootResponse { items, .. } => Some(items.clone()),
            _ => None,
        });
        if let Some(items) = window {
            // Closed again, so the next corpse's window is the one that opens.
            party.cmd(i, RealmCommand::LootRelease { guid: corpse });
            return items;
        }
        party.run(0.2);
    }
    Vec::new()
}

fn notes_of(items: &[LootItem], k: usize, j: usize) -> usize {
    let [_, _, a, b] = ids(k, j);
    items
        .iter()
        .filter(|l| l.item_id == a || l.item_id == b)
        .count()
}

/// The chat lines player `i` has seen.
fn chat(party: &Party, i: usize) -> Vec<String> {
    party
        .seen(i)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Chat(m) => Some(m.text.clone()),
            _ => None,
        })
        .collect()
}

/// The giver's creates player `i` has seen: guid and position.
fn giver_creates(party: &Party, i: usize) -> Vec<(u64, [f32; 3])> {
    unit_creates(party.seen(i))
        .into_iter()
        .filter(|(_, entry, _)| *entry == GIVER)
        .map(|(g, _, at)| (g, at))
        .collect()
}

fn offered(party: &mut Party, i: usize, giver: u64, spot: [f32; 3], quest: u32) -> bool {
    party.go(i, spot, 0.0);
    party.run(0.2);
    let before = party.seen(i).len();
    party.cmd(i, RealmCommand::QuestGiverHello { npc: giver });
    party.seen(i)[before..].iter().any(|e| match e {
        SessionEvent::QuestDetail(d) => d.quest_id == quest,
        SessionEvent::QuestGreeting(list) => list.quests.iter().any(|q| q.quest_id == quest),
        _ => false,
    })
}

fn saved(party: &Party, i: usize) -> String {
    party.realm.export_character(party.guids[i]).unwrap()
}

#[test]
fn a_kill_in_the_bounds_drops_the_note_and_shows_the_giver_to_the_audience() {
    let k = 0;
    let (mut party, map) = party(realm_with(k, 1, 100.0, ""), k);
    let corpse = kill(&mut party, map, k, 0, WOLF, 0.0);
    party.run(1.0);

    let items = loot(&mut party, 0, corpse);
    assert_eq!(notes_of(&items, k, 0), 1, "A's loot holds a note");
    assert_eq!(
        notes_of(&loot(&mut party, 1, corpse), k, 0),
        0,
        "B's does not"
    );

    // The giver stands at one of the two points, for A and B and not for C.
    let [_, quest, ..] = ids(k, 0);
    let points = [at(k, -30.0, 20.0), at(k, 30.0, 20.0)];
    let a = giver_creates(&party, 0);
    assert_eq!(a.len(), 1, "A sees one giver");
    let (giver, spot) = a[0];
    assert!(points.iter().any(|p| flat_distance(*p, spot) < 1.0));
    assert_eq!(giver_creates(&party, 1).len(), 1);
    assert!(giver_creates(&party, 2).is_empty());
    assert_eq!(party.realm.rumour_givers(), vec![giver]);

    // B hears of it; A, who found it, and C do not.
    let line = chat(&party, 1)
        .into_iter()
        .find(|l| l.contains("found a rumour"))
        .expect("B's chat line");
    assert!(line.contains(GIVER_NAME) && line.contains("rise"), "{line}");
    assert!(line.starts_with("Varian"), "{line}");
    assert!(!chat(&party, 0).iter().any(|l| l.contains("found a rumour")));
    assert!(!chat(&party, 2).iter().any(|l| l.contains("found a rumour")));

    // The giver offers the first step to A and B; C is too far to see it.
    assert!(offered(&mut party, 0, giver, spot, quest));
    assert!(offered(&mut party, 1, giver, spot, quest));
    assert!(!offered(&mut party, 2, giver, spot, quest));
    party.cmd(0, RealmCommand::QuestGiverAcceptQuest { npc: giver, quest });
    assert!(saved(&party, 0).contains(&quest.to_string()));
}

#[test]
fn a_dropper_rolls_at_its_own_chance_and_only_the_bounds_roll_the_rest() {
    let k = 1;
    let dropper = format!("{{ entry = {YOUNG_WOLF}, chance = 100.0 }}");
    let (mut party, map) = party(realm_with(k, 1, 0.0, &dropper), k);
    // Outside the bounds, a creature the Rumour does not name rolls nothing.
    for dy in [500.0, 600.0, 700.0] {
        let corpse = kill(&mut party, map, k, 0, WOLF, dy);
        assert_eq!(notes_of(&loot(&mut party, 0, corpse), k, 0), 0);
    }
    assert!(party.realm.rumour_givers().is_empty());
    // The named creature rolls it anywhere, at 100 percent.
    let corpse = kill(&mut party, map, k, 0, YOUNG_WOLF, 500.0);
    assert_eq!(notes_of(&loot(&mut party, 0, corpse), k, 0), 1);
}

#[test]
fn a_chance_of_zero_in_the_bounds_never_drops() {
    let k = 2;
    let dropper = format!("{{ entry = {OTHER_CREATURE}, chance = 100.0 }}");
    let (mut party, map) = party(realm_with(k, 1, 0.0, &dropper), k);
    for _ in 0..3 {
        let corpse = kill(&mut party, map, k, 0, WOLF, 0.0);
        assert_eq!(notes_of(&loot(&mut party, 0, corpse), k, 0), 0);
    }
}

#[test]
fn the_timer_takes_the_giver_the_quest_and_the_note_away() {
    let k = 3;
    let (mut party, map) = party(realm_with(k, 1, 100.0, ""), k);
    let corpse = kill(&mut party, map, k, 0, WOLF, 0.0);
    party.run(1.0);
    let items = loot(&mut party, 0, corpse);
    let note = items.iter().find(|l| notes_of(&[(*l).clone()], k, 0) == 1);
    let slot = note.expect("the note").slot;
    party.cmd(0, RealmCommand::Loot { guid: corpse });
    party.cmd(0, RealmCommand::AutostoreLootItem { slot });
    let [_, quest, ..] = ids(k, 0);
    let (giver, spot) = giver_creates(&party, 0)[0];
    assert!(offered(&mut party, 0, giver, spot, quest));
    party.cmd(0, RealmCommand::QuestGiverAcceptQuest { npc: giver, quest });
    let held = saved(&party, 0);
    let (note_id, quest_id) = (note.unwrap().item_id.to_string(), quest.to_string());
    assert!(held.contains(&note_id) && held.contains(&quest_id));

    // 30 minutes pass.
    party.realm.set_wall_clock(10_000_000_000);
    let got = party.run(1.0);
    assert!(got[0].iter().any(|e| matches!(
        e,
        SessionEvent::ObjectsRemoved(gone) if gone.contains(&giver)
    ) || matches!(e, SessionEvent::ObjectDestroyed(g) if *g == giver)));
    assert!(got[0]
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestFailed { quest_id, .. } if *quest_id == quest)));
    let held = saved(&party, 0);
    assert!(!held.contains(&note_id) && !held.contains(&quest_id));
    assert!(party.realm.rumour_givers().is_empty());
}

#[test]
fn three_active_rumours_keep_a_fourth_from_a_character() {
    let k = 4;
    // Four Rumours over the same ground: each kill rolls the first one a character lacks.
    let (mut party, map) = party(realm_with(k, 4, 100.0, ""), k);
    // B stands out of range for the first four kills, and so does C.
    party.go(1, at(k, 500.0, 0.0), 0.0);
    party.run(0.5);
    for j in 0..3 {
        let corpse = kill(&mut party, map, k, 0, WOLF, 0.0);
        assert_eq!(
            notes_of(&loot(&mut party, 0, corpse), k, j),
            1,
            "Rumour {j}"
        );
    }
    // A holds three: a fourth kill drops nothing for A alone.
    let corpse = kill(&mut party, map, k, 0, WOLF, 0.0);
    let all: usize = (0..4)
        .map(|j| notes_of(&loot(&mut party, 0, corpse), k, j))
        .sum();
    assert_eq!(all, 0, "A is at the cap");
    assert_eq!(party.realm.rumour_givers().len(), 3);

    // With B beside A, B alone is the Audience and finds the first.
    party.go(1, at(k, 10.0, 0.0), 0.0);
    party.run(0.5);
    let corpse = kill(&mut party, map, k, 0, WOLF, 0.0);
    assert_eq!(notes_of(&loot(&mut party, 0, corpse), k, 0), 0);
    assert_eq!(notes_of(&loot(&mut party, 1, corpse), k, 0), 1);
}

#[test]
fn every_audience_member_handing_in_ends_a_one_step_rumour() {
    let k = 5;
    let (mut party, map) = party(realm_with(k, 1, 100.0, ""), k);
    kill(&mut party, map, k, 0, WOLF, 0.0);
    party.run(1.0);
    let [_, quest, ..] = ids(k, 0);
    let (giver, spot) = giver_creates(&party, 0)[0];
    for i in [0, 1] {
        assert!(offered(&mut party, i, giver, spot, quest));
        party.cmd(i, RealmCommand::QuestGiverAcceptQuest { npc: giver, quest });
    }
    let hand_in = |party: &mut Party, i: usize| {
        party.cmd(
            i,
            RealmCommand::QuestGiverCompleteQuest { npc: giver, quest },
        );
        party.cmd(
            i,
            RealmCommand::QuestGiverRequestReward { npc: giver, quest },
        );
        party.cmd(
            i,
            RealmCommand::QuestGiverChooseReward {
                npc: giver,
                quest,
                choice: 0,
            },
        );
    };
    hand_in(&mut party, 0);
    party.run(1.0);
    assert_eq!(
        party.realm.rumour_givers(),
        vec![giver],
        "B has not handed in"
    );
    hand_in(&mut party, 1);
    party.run(1.0);
    // Each hand-in left a chest, and the Rumour stays until both are emptied (#288).
    assert_eq!(party.realm.rumour_givers(), vec![giver], "chests are full");
    for i in [0, 1] {
        let chest = party
            .seen(i)
            .iter()
            .find_map(|e| match e {
                SessionEvent::ObjectCreate {
                    guid,
                    kind: benilla_protocol::EntityKind::GameObject,
                    ..
                } => Some(*guid),
                _ => None,
            })
            .expect("a chest");
        party.cmd(i, RealmCommand::UseGameObject { guid: chest });
        party.cmd(i, RealmCommand::AutostoreLootItem { slot: 0 });
    }
    party.run(1.0);
    assert!(party.realm.rumour_givers().is_empty(), "both handed in");
}
