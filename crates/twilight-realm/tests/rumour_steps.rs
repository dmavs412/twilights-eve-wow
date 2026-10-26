//! Rumour steps (spec #284): accepting a step spawns its creatures and use objects for the
//! Audience, the last hand-in pays the Grade's XP and opens a chest for the one who handed in,
//! and a Calling Rumour is rolled by one Calling at its cap, once. Each test works in an area of
//! its own, because the Rumours the tests add are the process's.

mod common;

use benilla_protocol::messages::LootItem;
use benilla_protocol::{EntityKind, SessionEvent};
use common::party::*;
use common::*;
use twilight_realm::rules::loot::band_greens;
use twilight_realm::{Audience, CROSSROADS};

const WOLF: u32 = 2958;
/// The creature the Rumours spawn, and the dropper the Calling Rumour rolls on.
const YOUNG_WOLF: u32 = WOLF;
/// Brother Danil: any friendly template will do for a giver.
const GIVER: u32 = 152;
/// The Vanguard Trainer, who gives the Advancement into `vanguard`.
const VANGUARD_TRAINER: u32 = 5810;
/// The entry of the chest a last hand-in makes.
const CHEST_ENTRY: u32 = 980199;
/// The killed creature's level, which is the giver's and the chest's. The Wolf at level 6 beats
/// a level 8 warrior; `band_greens` clamps level 1 to the same 5 to 8 band.
const LEVEL: u8 = 1;
/// The Dungeon pool in `content/rumours.toml`.
const BLUES: [u32; 21] = [
    900500, 900501, 900502, 900503, 900504, 900505, 900506, 900507, 900508, 900509, 900510, 900105,
    900511, 900512, 900513, 900514, 900515, 900516, 900517, 900518, 900519,
];

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

/// The ids of area `k`: the Rumour, then its quests from `+1`, its two notes at `+8` and `+9` and
/// its use object at `+20`.
fn id(k: usize, offset: u32) -> u32 {
    5_000_000 + 100 * k as u32 + offset
}

fn quest(k: usize, step: usize) -> u32 {
    id(k, 1 + step as u32)
}

/// The quests and the Rumour of area `k`. `extra[i]` is step `i`'s objective lines and `spawn`
/// the `[[rumour.spawn]]` tables. A calling Rumour (`calling` is its `from` and `to`) is rolled
/// by `droppers` alone and hands its last quest in to the Vanguard Trainer.
fn content(
    k: usize,
    grade: &str,
    extra: &[&str],
    spawn: &str,
    calling: Option<(&str, &str)>,
) -> (String, String) {
    let mut quests = format!(
        r#"
[[gameobject]]
guid = {obj}
entry = {obj}
display = 1
map = 0
position = [-8911.0, -140.5, 82.0]
"#,
        obj = id(k, 20)
    );
    for (note, landmark) in [(id(k, 8), "the north rise"), (id(k, 9), "the south rise")] {
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
description = "Corina Steele waits at {landmark}."
pages = ["Corina Steele waits at {landmark}."]
"#
        );
    }
    let mut ids = Vec::new();
    for (step, lines) in extra.iter().enumerate() {
        let q = quest(k, step);
        let after = if step == 0 {
            String::new()
        } else {
            format!("after = {}", quest(k, step - 1))
        };
        let ender = if calling.is_some() && step + 1 == extra.len() {
            format!("ender = {VANGUARD_TRAINER}")
        } else {
            String::new()
        };
        quests += &format!(
            r#"
[[quest]]
id = {q}
kind = "rumour"
giver = {GIVER}
{ender}
{after}
level = 1
title = "Step {q}"
objectives = "Hear Corina out."
details = "-"
progress = "-"
completion = "-"
money = 0
{lines}
"#
        );
        ids.push(q.to_string());
    }
    let (x, y, _) = (base(k)[0], base(k)[1], 0.0);
    let (north, south) = (at(k, -30.0, 20.0), at(k, 30.0, 20.0));
    let kind = match calling {
        Some((from, to)) => format!(
            "kind = \"calling\"\ncalling = {{ from = \"{from}\", to = \"{to}\" }}\n\
             chance = 0.0\ndroppers = [{{ entry = {YOUNG_WOLF}, chance = 100.0 }}]"
        ),
        None => "chance = 100.0".to_string(),
    };
    let rumours = format!(
        r#"
[[rumour]]
id = {rumour}
name = "Rumour {rumour}"
grade = "{grade}"
{kind}
map = {KALIMDOR}
bounds = [[{}, {}, {}, {}]]
giver = {GIVER}
points = [
  {{ x = {}, y = {}, landmark = "the north rise", note = {} }},
  {{ x = {}, y = {}, landmark = "the south rise", note = {} }},
]
quests = [{}]
{spawn}
"#,
        x - 100.0,
        y - 100.0,
        x + 100.0,
        y + 100.0,
        north[0],
        north[1],
        id(k, 8),
        south[0],
        south[1],
        id(k, 9),
        ids.join(", "),
        rumour = id(k, 0),
    );
    (quests, rumours)
}

fn realm_with(content: &(String, String)) -> Realm {
    Realm::with_seed(7)
        .with_quests(&content.0)
        .with_rumours(&content.1)
}

struct Scene {
    party: Party,
    map: u32,
    k: usize,
}

/// Three level 8 warriors, A at the area's base, B 10 yards from it and C 90; with `alone`, B
/// and C stand out of reach.
fn scene(k: usize, content: &(String, String), levels: &[u8], alone: bool) -> Scene {
    let mut party = Party::new(realm_with(content), levels);
    party.go(0, at(k, 0.0, 0.0), 0.0);
    if levels.len() > 1 {
        let far = if alone { 500.0 } else { 10.0 };
        party.go(1, at(k, far, 0.0), 0.0);
    }
    if levels.len() > 2 {
        let far = if alone { 600.0 } else { 90.0 };
        party.go(2, at(k, far, 0.0), 0.0);
    }
    party.run(0.5);
    let map = party.realm.player_positions()[0].0;
    Scene { party, map, k }
}

impl Scene {
    /// Player `i` kills a level `LEVEL` `entry` creature 3 yards from where they stand.
    fn kill(&mut self, i: usize, entry: u32) -> u64 {
        let spot = at(self.k, 0.0, 0.0);
        let audience = Audience::of(&[self.party.guids[i]]);
        let prey = [spot[0] + 3.0, spot[1], 0.0];
        let wolf = self
            .party
            .realm
            .spawn_for(
                self.party.host.as_ref(),
                entry,
                LEVEL,
                prey,
                self.map,
                audience,
            )
            .expect("a creature");
        self.slay(i, wolf, spot, prey);
        self.party.run(0.5);
        wolf
    }

    fn slay(&mut self, i: usize, wolf: u64, from: [f32; 3], at: [f32; 3]) {
        self.party.go_facing(i, from, at);
        self.party.run(0.3);
        self.party.cmd(i, RealmCommand::AttackSwing { guid: wolf });
        let dead = self.party.run_until(600.0, |got| died(&got[i], wolf));
        let swings = swung_at(self.party.seen(i), self.party.guids[i]).len();
        assert!(dead, "the creature never died ({swings} swings by A)");
        self.party.run(0.5);
    }

    /// The giver's guid and spot, as player `i` saw it made.
    fn giver(&self, i: usize) -> (u64, [f32; 3]) {
        unit_creates(self.party.seen(i))
            .into_iter()
            .find(|(_, entry, _)| *entry == GIVER)
            .map(|(g, _, at)| (g, at))
            .expect("the giver")
    }

    fn accept(&mut self, i: usize, giver: u64, spot: [f32; 3], quest: u32) {
        self.party.go(i, spot, 0.0);
        self.party.run(0.2);
        let before = self.party.seen(i).len();
        self.party
            .cmd(i, RealmCommand::QuestGiverHello { npc: giver });
        let offered = self.party.seen(i)[before..].iter().any(|e| match e {
            SessionEvent::QuestDetail(d) => d.quest_id == quest,
            SessionEvent::QuestGreeting(list) => list.quests.iter().any(|q| q.quest_id == quest),
            _ => false,
        });
        assert!(offered, "quest {quest} is offered");
        self.party
            .cmd(i, RealmCommand::QuestGiverAcceptQuest { npc: giver, quest });
        self.party.run(0.5);
    }

    /// Player `i` hands in `quest` at `npc`; the XP paid, when it went through.
    fn hand_in(&mut self, i: usize, npc: u64, quest: u32) -> Option<u32> {
        let before = self.party.seen(i).len();
        for command in [
            RealmCommand::QuestGiverCompleteQuest { npc, quest },
            RealmCommand::QuestGiverRequestReward { npc, quest },
            RealmCommand::QuestGiverChooseReward {
                npc,
                quest,
                choice: 0,
            },
        ] {
            self.party.cmd(i, command);
        }
        self.party.seen(i)[before..].iter().find_map(|e| match e {
            SessionEvent::QuestComplete(q) if q.quest_id == quest => Some(q.xp),
            _ => None,
        })
    }
}

fn units(events: &[SessionEvent], entry: u32) -> Vec<(u64, [f32; 3])> {
    unit_creates(events)
        .into_iter()
        .filter(|(_, e, _)| *e == entry)
        .map(|(g, _, at)| (g, at))
        .collect()
}

fn objects(events: &[SessionEvent], entry: u32) -> Vec<u64> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::GameObject,
                fields,
                ..
            } if fields.object_entry() == Some(entry) => Some(*guid),
            _ => None,
        })
        .collect()
}

fn removed(events: &[SessionEvent], guid: u64) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectsRemoved(gone) if gone.contains(&guid)))
}

/// What player `i`'s loot window shows after a click on `guid`, if one opened.
fn window(party: &mut Party, i: usize, guid: u64) -> Option<Vec<LootItem>> {
    let before = party.seen(i).len();
    party.cmd(i, RealmCommand::UseGameObject { guid });
    party.seen(i)[before..].iter().find_map(|e| match e {
        SessionEvent::LootResponse { items, .. } => Some(items.clone()),
        _ => None,
    })
}

fn flat(a: [f32; 3], b: [f32; 3]) -> f32 {
    flat_distance(a, b)
}

#[test]
fn a_step_spawns_its_creature_for_the_audience_once_and_it_counts_for_the_kill() {
    let k = 0;
    let spawn = format!(
        r#"
[[rumour.spawn]]
quest = {}
creature = {YOUNG_WOLF}
points = [{{ x = {}, y = {} }}, {{ x = {}, y = {} }}]
"#,
        quest(k, 1),
        at(k, 20.0, 30.0)[0],
        at(k, 20.0, 30.0)[1],
        at(k, -20.0, 30.0)[0],
        at(k, -20.0, 30.0)[1],
    );
    let kills = format!("kill = [[{YOUNG_WOLF}, 1]]");
    let c = content(k, "easy", &["", &kills], &spawn, None);
    let mut s = scene(k, &c, &[8, 8, 8], false);
    s.kill(0, WOLF);
    s.party.run(1.0);
    let (giver, spot) = s.giver(0);
    let (q1, q2) = (quest(k, 0), quest(k, 1));
    // The kill that rolled the Rumour is the one creature each of A and B has seen so far.
    let (seen_a, seen_b) = (
        units(s.party.seen(0), YOUNG_WOLF).len(),
        units(s.party.seen(1), YOUNG_WOLF).len(),
    );
    s.accept(0, giver, spot, q1);
    assert!(s.hand_in(0, giver, q1).is_some(), "step 1 is handed in");
    assert_eq!(
        units(s.party.seen(0), YOUNG_WOLF).len(),
        seen_a,
        "step 1 spawns nothing"
    );
    // C stands among the points and still does not see them.
    s.party.go(2, at(k, 0.0, 30.0), 0.0);
    s.party.run(0.5);
    s.accept(0, giver, spot, q2);
    s.party.run(1.5);

    let made = units(s.party.seen(0), YOUNG_WOLF);
    assert_eq!(made.len(), seen_a + 1, "one creature more for A");
    let (wolf, at_) = made[seen_a];
    let points = [at(k, 20.0, 30.0), at(k, -20.0, 30.0)];
    assert!(points.iter().any(|&p| flat(p, at_) < 1.0), "at a point");
    assert_eq!(
        units(s.party.seen(1), YOUNG_WOLF).len(),
        seen_b + 1,
        "and for B"
    );
    assert!(units(s.party.seen(2), YOUNG_WOLF).is_empty(), "not for C");

    // B taking the step spawns nothing more.
    s.accept(1, giver, spot, q1);
    assert!(s.hand_in(1, giver, q1).is_some());
    s.accept(1, giver, spot, q2);
    s.party.run(1.0);
    assert_eq!(units(s.party.seen(0), YOUNG_WOLF).len(), seen_a + 1);
    assert_eq!(units(s.party.seen(1), YOUNG_WOLF).len(), seen_b + 1);

    s.slay(0, wolf, [at_[0] - 3.0, at_[1], 0.0], at_);
    s.party.run(70.0);
    assert_eq!(
        units(s.party.seen(0), YOUNG_WOLF).len(),
        seen_a + 1,
        "no respawn"
    );
    s.party.go(0, spot, 0.0);
    s.party.run(0.2);
    assert!(s.hand_in(0, giver, q2).is_some(), "the kill counted");
}

#[test]
fn a_step_with_a_count_spawns_that_many_use_objects_and_each_click_counts() {
    let k = 1;
    let object = id(k, 20);
    let spawn = format!(
        r#"
[[rumour.spawn]]
quest = {}
object = {{ entry = {object}, display = 1 }}
count = 3
points = [{{ x = {}, y = {} }}]
"#,
        quest(k, 0),
        at(k, 0.0, 20.0)[0],
        at(k, 0.0, 20.0)[1],
    );
    let c = content(
        k,
        "easy",
        &[&format!("use = [[{object}, 3]]")],
        &spawn,
        None,
    );
    let mut s = scene(k, &c, &[8], true);
    s.kill(0, WOLF);
    s.party.run(1.0);
    let (giver, spot) = s.giver(0);
    let q = quest(k, 0);
    s.accept(0, giver, spot, q);
    s.party.go(0, at(k, 0.0, 19.0), 0.0);
    s.party.run(1.5);

    let made = objects(s.party.seen(0), object);
    assert_eq!(made.len(), 3, "three use objects");
    for (n, &guid) in made.iter().enumerate() {
        s.party.go(0, at(k, 0.0, 19.0), 0.0);
        s.party.run(0.2);
        let before = s.party.seen(0).len();
        s.party.cmd(0, RealmCommand::UseGameObject { guid });
        s.party.run(0.3);
        assert!(removed(&s.party.seen(0)[before..], guid), "click {n}");
        s.party.go(0, spot, 0.0);
        s.party.run(0.2);
        let done = s.hand_in(0, giver, q);
        assert_eq!(done.is_some(), n == 2, "complete only after the third");
    }
}

/// A one-step Rumour of `grade` at giver level `LEVEL`, handed in by A alone: the XP it paid
/// and the chest's contents.
fn finish(k: usize, grade: &str) -> (u32, Vec<u32>, bool) {
    let c = content(k, grade, &[""], "", None);
    let mut s = scene(k, &c, &[8, 8], true);
    s.kill(0, WOLF);
    s.party.run(1.0);
    let (giver, spot) = s.giver(0);
    let q = quest(k, 0);
    s.accept(0, giver, spot, q);
    let xp = s.hand_in(0, giver, q).expect("handed in");
    s.party.run(1.0);
    let chests = objects(s.party.seen(0), CHEST_ENTRY);
    assert_eq!(chests.len(), 1, "one chest");
    assert!(
        flat(spot, unit_or_object_position(&s.party, chests[0])) <= 5.0,
        "within 5 yards of the giver"
    );
    // B stands out of the Audience: the chest is not theirs.
    s.party.go(1, spot, 0.0);
    s.party.run(0.5);
    let theirs = window(&mut s.party, 1, chests[0]).is_some();
    let items = window(&mut s.party, 0, chests[0]).expect("A's loot window");
    (xp, items.iter().map(|l| l.item_id).collect(), theirs)
}

fn unit_or_object_position(party: &Party, guid: u64) -> [f32; 3] {
    party
        .seen(0)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g, position, ..
            } if *g == guid => Some(*position),
            _ => None,
        })
        .expect("the chest's create")
}

#[test]
fn an_easy_hand_in_pays_once_and_leaves_a_green_for_the_one_who_handed_in() {
    let (xp, items, theirs) = finish(2, "easy");
    assert!(xp > 0);
    assert_eq!(items.len(), 1);
    assert!(band_greens(LEVEL).contains(&items[0]), "a World green");
    assert!(!theirs, "B's click does nothing");
}

#[test]
fn a_deadly_hand_in_triples_the_xp_and_leaves_two_distinct_blues() {
    let (easy, ..) = finish(3, "easy");
    let (xp, items, theirs) = finish(4, "deadly");
    assert_eq!(xp, 3 * easy);
    assert_eq!(items.len(), 2);
    assert_ne!(items[0], items[1]);
    assert!(items.iter().all(|i| BLUES.contains(i)));
    assert!(!theirs);
}

#[test]
fn a_hard_hand_in_leaves_one_blue_and_doubles_the_xp() {
    let (easy, ..) = finish(5, "easy");
    let (xp, items, _) = finish(6, "hard");
    assert_eq!(xp, 2 * easy);
    assert_eq!(items.len(), 1);
    assert!(BLUES.contains(&items[0]));
}

#[test]
fn the_instance_outlives_the_last_hand_in_until_the_chest_is_empty() {
    let k = 7;
    let c = content(k, "easy", &[""], "", None);
    let mut s = scene(k, &c, &[8], true);
    s.kill(0, WOLF);
    s.party.run(1.0);
    let (giver, spot) = s.giver(0);
    let q = quest(k, 0);
    s.accept(0, giver, spot, q);
    assert!(s.hand_in(0, giver, q).is_some());
    s.party.run(2.0);
    assert_eq!(s.party.realm.rumour_givers(), vec![giver], "chest unopened");

    let chest = objects(s.party.seen(0), CHEST_ENTRY)[0];
    assert!(window(&mut s.party, 0, chest).is_some());
    s.party.cmd(0, RealmCommand::AutostoreLootItem { slot: 0 });
    s.party.run(1.0);
    assert!(s.party.realm.rumour_givers().is_empty(), "chest emptied");
}

#[test]
fn a_calling_rumour_is_rolled_at_the_cap_and_finished_once() {
    let k = 8;
    let c = content(k, "easy", &[""], "", Some(("basic_warrior", "vanguard")));
    let mut s = scene(k, &c, &[10, 9], false);
    let corpse = s.kill(0, YOUNG_WOLF);
    s.party.run(1.0);
    let items = |s: &mut Scene, i: usize, corpse: u64| -> Vec<u32> {
        let before = s.party.seen(i).len();
        s.party.cmd(i, RealmCommand::Loot { guid: corpse });
        let got = s.party.seen(i)[before..].iter().find_map(|e| match e {
            SessionEvent::LootResponse { items, .. } => {
                Some(items.iter().map(|l| l.item_id).collect())
            }
            _ => None,
        });
        s.party.cmd(i, RealmCommand::LootRelease { guid: corpse });
        got.unwrap_or_default()
    };
    let notes = [id(k, 8), id(k, 9)];
    assert!(
        items(&mut s, 0, corpse).iter().any(|i| notes.contains(i)),
        "the level 10 warrior finds the note"
    );
    assert!(
        !items(&mut s, 1, corpse).iter().any(|i| notes.contains(i)),
        "the level 9 one does not"
    );
    assert_eq!(s.party.realm.rumour_givers().len(), 1);

    let (giver, spot) = s.giver(0);
    let q = quest(k, 0);
    s.accept(0, giver, spot, q);
    let trainer = hub_guid(VANGUARD_TRAINER);
    s.party.go(0, hub_home(trainer), 0.0);
    s.party.run(0.5);
    assert!(
        s.hand_in(0, trainer, q).is_some(),
        "handed in at the trainer"
    );
    s.party.run(1.0);
    assert!(s.party.realm.rumour_givers().is_empty());

    let blob = s.party.realm.export_character(s.party.guids[0]).unwrap();
    let finished = format!("finished_rumours = [{}]", id(k, 0));
    assert!(blob.contains(&finished), "{blob}");
    let mut again = Realm::with_seed(1).with_quests(&c.0).with_rumours(&c.1);
    let guid = again.import_character(&blob).unwrap();
    assert!(again.export_character(guid).unwrap().contains(&finished));

    // A second kill never rolls it again.
    let corpse = s.kill(0, YOUNG_WOLF);
    s.party.run(1.0);
    assert!(!items(&mut s, 0, corpse).iter().any(|i| notes.contains(i)));
    assert!(s.party.realm.rumour_givers().is_empty());
}

/// The spawn audit's dev aids (#356): `DevRumourStep` starts a Rumour and makes its step's
/// spawns at the point it names, as an accept would, and `DevRumourEnd` ends it, taking them
/// away.
#[test]
fn a_dev_step_makes_the_steps_spawns_and_a_dev_end_takes_them_away() {
    let k = 9;
    let spawn = format!(
        r#"
[[rumour.spawn]]
quest = {}
creature = {YOUNG_WOLF}
points = [{{ x = {}, y = {} }}, {{ x = {}, y = {} }}]
"#,
        quest(k, 1),
        at(k, 20.0, 30.0)[0],
        at(k, 20.0, 30.0)[1],
        at(k, -20.0, 30.0)[0],
        at(k, -20.0, 30.0)[1],
    );
    let c = content(k, "easy", &["", ""], &spawn, None);
    let mut s = scene(k, &c, &[8], true);
    s.party.cmd(
        0,
        RealmCommand::DevRumourStep {
            id: id(k, 0),
            quest: quest(k, 1),
            point: 1,
        },
    );
    s.party.run(1.0);
    let wolves = units(s.party.seen(0), YOUNG_WOLF);
    assert_eq!(wolves.len(), 1, "the step's one creature");
    assert!(
        flat(wolves[0].1, at(k, -20.0, 30.0)) < 0.01,
        "at the point asked for"
    );
    assert_eq!(units(s.party.seen(0), GIVER).len(), 1, "and the giver");

    s.party.cmd(0, RealmCommand::DevRumourEnd { id: id(k, 0) });
    s.party.run(1.0);
    assert!(
        removed(s.party.seen(0), wolves[0].0),
        "the end takes it away"
    );
}
