//! The three Northshire Rumours (#289, `content/rumours.toml`), one per Grade, played through on
//! the shipped rows: a dropper's kill rolls the Rumour, its giver stands at one of its points, the
//! steps finish in order with the spawns the rows name, and the last hand-in leaves the Grade's
//! chest. The kills fall outside Northshire, so only the dropper rolls. The chance is the shipped
//! one, so each test kills the dropper until the Rumour comes, which the seed fixes.

mod common;

use benilla_protocol::messages::LootItem;
use benilla_protocol::{EntityKind, SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;
use twilight_realm::rules::loot::band_greens;
use twilight_realm::Audience;

/// The entry of the chest a last hand-in makes.
const CHEST_ENTRY: u32 = 980199;
/// The level of every creature a test spawns for a kill, which is the chest's.
const LEVEL: u8 = 1;
/// Where the dropper dies: in Elwynn, outside the Northshire box, so no other Rumour rolls.
const KILL_SPOT: [f32; 3] = [-9400.0, 0.0, 82.0];
/// The seeds tried, and the kills each may take, to roll a Rumour at the shipped chances.
const SEEDS: u64 = 400;
const KILLS_PER_SEED: usize = 10;
/// The Dungeon pool in `content/rumours.toml`.
const BLUES: [u32; 21] = [
    900500, 900501, 900502, 900503, 900504, 900505, 900506, 900507, 900508, 900509, 900510, 900105,
    900511, 900512, 900513, 900514, 900515, 900516, 900517, 900518, 900519,
];
const STOLEN_LEDGER_PAGE: u32 = 980039;
const UNEARTHED_CACHE: u32 = 980120;
/// The guid of the one cache `quests_elwynn.toml` stands by the mine for everyone.
const STANDING_CACHE: u64 = 790_801;

const OSWIN: u32 = 97010;
const MAREN: u32 = 97011;
const HALE: u32 = 97012;
const BOOKKEEPER: u32 = 97013;
const GRUBHOLLOW: u32 = 97014;
const TIMBER_WOLF: u32 = 69;
const YOUNG_WOLF: u32 = 299;
const DEFIAS_THUG: u32 = 38;
const KOBOLD_WORKER: u32 = 257;

const OSWIN_POINTS: [[f32; 2]; 3] = [[-8975.0, -185.0], [-8829.0, -160.0], [-8870.0, -60.0]];
const MAREN_POINTS: [[f32; 2]; 3] = [[-8985.0, -330.0], [-9020.0, -300.0], [-8960.0, -350.0]];
const HALE_POINTS: [[f32; 2]; 3] = [[-8790.0, -175.0], [-8760.0, -140.0], [-8800.0, -200.0]];
const BOOKKEEPER_POINTS: [[f32; 2]; 3] = [[-8992.0, -325.0], [-9010.0, -300.0], [-8970.0, -345.0]];
const CACHE_POINTS: [[f32; 2]; 3] = [[-8788.0, -170.0], [-8762.0, -140.0], [-8798.0, -198.0]];
const GRUBHOLLOW_POINTS: [[f32; 2]; 2] = [[-8775.0, -160.0], [-8766.0, -136.0]];

struct Scene {
    party: Party,
    map: u32,
    giver: u64,
    spot: [f32; 3],
}

fn at_point(p: [f32; 2]) -> [f32; 3] {
    [p[0], p[1], 0.0]
}

fn near_a_point(at: [f32; 3], points: &[[f32; 2]]) -> bool {
    points.iter().any(|&p| flat_distance(at, at_point(p)) < 1.5)
}

fn creatures(events: &[SessionEvent], entry: u32) -> Vec<(u64, [f32; 3])> {
    unit_creates(events)
        .into_iter()
        .filter(|(_, e, _)| *e == entry)
        .map(|(g, _, at)| (g, at))
        .collect()
}

/// The guid and spot of each cache made for the Audience, leaving the standing one out.
fn caches(events: &[SessionEvent]) -> Vec<(u64, [f32; 3])> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::GameObject,
                fields,
                position,
                ..
            } if fields.object_entry() == Some(UNEARTHED_CACHE)
                && guid & 0xFFFF_FFFF != STANDING_CACHE =>
            {
                Some((*guid, *position))
            }
            _ => None,
        })
        .collect()
}

fn chest_guids(events: &[SessionEvent]) -> Vec<u64> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::GameObject,
                fields,
                ..
            } if fields.object_entry() == Some(CHEST_ENTRY) => Some(*guid),
            _ => None,
        })
        .collect()
}

impl Scene {
    /// A level 10 warrior in Elwynn who kills `dropper` a few times, under each seed in turn until
    /// one rolls the Rumour, then walks to its points until the giver `giver_entry` shows. A short
    /// run keeps the wolves from wearing the warrior down, and the seed fixes the search.
    fn rolled(dropper: u32, giver_entry: u32, points: &[[f32; 2]]) -> Scene {
        (1..=SEEDS)
            .find_map(|seed| Scene::attempt(seed, dropper, giver_entry, points))
            .unwrap_or_else(|| panic!("no seed to {SEEDS} put the giver {giver_entry} at a point"))
    }

    fn attempt(seed: u64, dropper: u32, giver_entry: u32, points: &[[f32; 2]]) -> Option<Scene> {
        let mut party = Party::starting(
            Realm::with_seed(seed),
            &[10],
            START_ELWYNN,
            Box::new(NoGround),
        );
        party.go(0, KILL_SPOT, 0.0);
        party.run(0.5);
        let map = party.realm.player_positions()[0].0;
        let mut s = Scene {
            party,
            map,
            giver: 0,
            spot: KILL_SPOT,
        };
        for _ in 0..KILLS_PER_SEED {
            s.kill(dropper);
            if !s.party.realm.rumour_givers().is_empty() {
                break;
            }
        }
        if s.party.realm.rumour_givers().len() != 1 {
            return None;
        }
        for &p in points {
            s.party.go(0, at_point(p), 0.0);
            s.party.run(1.0);
            if let Some(&(guid, at)) = creatures(s.party.seen(0), giver_entry).last() {
                assert!(near_a_point(at, points), "the giver stands at a point");
                s.giver = guid;
                s.spot = at;
                return Some(s);
            }
        }
        None
    }

    /// Kills a level `LEVEL` `entry` creature 3 yards from `spot` and returns its corpse.
    fn kill(&mut self, entry: u32) -> u64 {
        let guid = self.party.guids[0];
        let prey = [self.spot[0] + 3.0, self.spot[1], self.spot[2]];
        let creature = self
            .party
            .realm
            .spawn_for(
                self.party.host.as_ref(),
                entry,
                LEVEL,
                prey,
                self.map,
                Audience::of(&[guid]),
            )
            .expect("a creature");
        self.slay(creature, self.spot, prey);
        creature
    }

    fn slay(&mut self, creature: u64, from: [f32; 3], at: [f32; 3]) {
        self.party.go_facing(0, from, at);
        self.party.run(0.3);
        self.party
            .cmd(0, RealmCommand::AttackSwing { guid: creature });
        let dead = self.party.run_until(600.0, |got| died(&got[0], creature));
        let swings = swung_at(self.party.seen(0), self.party.guids[0]).len();
        let me_dead = died(self.party.seen(0), self.party.guids[0]);
        assert!(
            dead,
            "the creature never died ({swings} swings, player dead: {me_dead})"
        );
        self.party.run(0.5);
    }

    /// Everything on `corpse`, taken; the item ids.
    fn loot(&mut self, corpse: u64) -> Vec<u32> {
        let before = self.party.seen(0).len();
        self.party.cmd(0, RealmCommand::Loot { guid: corpse });
        let items: Vec<u32> = self.party.seen(0)[before..]
            .iter()
            .find_map(|e| match e {
                SessionEvent::LootResponse { items, .. } => {
                    Some(items.iter().map(|l| l.item_id).collect())
                }
                _ => None,
            })
            .unwrap_or_default();
        for slot in 0..items.len() {
            self.party
                .cmd(0, RealmCommand::AutostoreLootItem { slot: slot as _ });
        }
        self.party
            .cmd(0, RealmCommand::LootRelease { guid: corpse });
        items
    }

    fn at_giver(&mut self) {
        self.party.go(0, self.spot, 0.0);
        self.party.run(0.3);
    }

    fn accept(&mut self, quest: u32) {
        self.at_giver();
        let before = self.party.seen(0).len();
        let npc = self.giver;
        self.party.cmd(0, RealmCommand::QuestGiverHello { npc });
        let offered = self.party.seen(0)[before..].iter().any(|e| match e {
            SessionEvent::QuestDetail(d) => d.quest_id == quest,
            SessionEvent::QuestGreeting(list) => list.quests.iter().any(|q| q.quest_id == quest),
            _ => false,
        });
        assert!(offered, "quest {quest} is offered");
        self.party
            .cmd(0, RealmCommand::QuestGiverAcceptQuest { npc, quest });
        self.party.run(0.5);
    }

    /// Hands `quest` in at the giver; the XP paid, when it went through.
    fn hand_in(&mut self, quest: u32) -> Option<u32> {
        self.at_giver();
        let npc = self.giver;
        let before = self.party.seen(0).len();
        self.party.cmd(0, RealmCommand::QuestGiverHello { npc });
        for command in [
            RealmCommand::QuestGiverCompleteQuest { npc, quest },
            RealmCommand::QuestGiverRequestReward { npc, quest },
            RealmCommand::QuestGiverChooseReward {
                npc,
                quest,
                choice: 0,
            },
        ] {
            self.party.cmd(0, command);
        }
        self.party.seen(0)[before..].iter().find_map(|e| match e {
            SessionEvent::QuestComplete(q) if q.quest_id == quest => Some(q.xp),
            _ => None,
        })
    }

    /// Kills `n` of `entry`, then hands `quest` in.
    fn kill_and_hand_in(&mut self, quest: u32, entry: u32, n: usize) {
        self.accept(quest);
        for _ in 0..n {
            self.kill(entry);
        }
        self.party.run(0.5);
        assert!(self.hand_in(quest).is_some(), "quest {quest} is handed in");
    }

    /// The items in the chest the last hand-in left, opened by the one who handed in.
    fn chest(&mut self) -> Vec<u32> {
        self.party.run(1.0);
        let chests = chest_guids(self.party.seen(0));
        assert_eq!(chests.len(), 1, "one chest");
        let before = self.party.seen(0).len();
        self.party
            .cmd(0, RealmCommand::UseGameObject { guid: chests[0] });
        let items: Vec<LootItem> = self.party.seen(0)[before..]
            .iter()
            .find_map(|e| match e {
                SessionEvent::LootResponse { items, .. } => Some(items.clone()),
                _ => None,
            })
            .expect("the chest's loot window");
        items.iter().map(|l| l.item_id).collect()
    }
}

#[test]
fn a_hermits_request_is_rolled_by_a_wolf_and_leaves_a_green() {
    let mut s = Scene::rolled(YOUNG_WOLF, OSWIN, &OSWIN_POINTS);
    s.kill_and_hand_in(94901, TIMBER_WOLF, 8);
    let items = s.chest();
    assert_eq!(items.len(), 1, "an easy chest holds one row");
    assert!(band_greens(LEVEL).contains(&items[0]), "a World green");
}

#[test]
fn the_vintners_ledger_takes_five_pages_then_the_bookkeeper_and_leaves_a_blue() {
    let mut s = Scene::rolled(DEFIAS_THUG, MAREN, &MAREN_POINTS);
    s.accept(94902);
    let mut pages = 0;
    for _ in 0..80 {
        if pages >= 5 {
            break;
        }
        let corpse = s.kill(DEFIAS_THUG);
        pages += s
            .loot(corpse)
            .iter()
            .filter(|&&i| i == STOLEN_LEDGER_PAGE)
            .count();
    }
    assert!(pages >= 5, "five pages drop from Defias Thugs");
    assert!(s.hand_in(94902).is_some(), "step 1 is handed in");

    assert!(
        creatures(s.party.seen(0), BOOKKEEPER).is_empty(),
        "step 1 spawns no bookkeeper"
    );
    s.accept(94903);
    s.party.run(1.5);
    let (bookkeeper, at) = *creatures(s.party.seen(0), BOOKKEEPER)
        .last()
        .expect("the Bookkeeper");
    assert!(near_a_point(at, &BOOKKEEPER_POINTS), "at one of its points");
    s.slay(bookkeeper, [at[0] - 3.0, at[1], at[2]], at);
    assert!(s.hand_in(94903).is_some(), "step 2 is handed in");
    let items = s.chest();
    assert_eq!(items.len(), 1, "a hard chest holds one row");
    assert!(BLUES.contains(&items[0]), "a Dungeon blue");
}

#[test]
fn what_the_kobolds_dug_up_runs_three_steps_and_leaves_two_blues() {
    let mut s = Scene::rolled(KOBOLD_WORKER, HALE, &HALE_POINTS);
    s.kill_and_hand_in(94904, KOBOLD_WORKER, 8);

    s.accept(94905);
    s.party.run(1.5);
    let made = caches(s.party.seen(0));
    assert_eq!(made.len(), 3, "three caches for the Audience");
    for (guid, at) in made {
        assert!(near_a_point(at, &CACHE_POINTS), "at one of its points");
        s.party.go(0, at, 0.0);
        s.party.run(0.2);
        s.party.cmd(0, RealmCommand::UseGameObject { guid });
        s.party.run(0.3);
    }
    assert!(s.hand_in(94905).is_some(), "step 2 is handed in");

    s.accept(94906);
    s.party.run(1.5);
    let (foreman, at) = *creatures(s.party.seen(0), GRUBHOLLOW)
        .last()
        .expect("Foreman Grubhollow");
    assert!(near_a_point(at, &GRUBHOLLOW_POINTS), "at one of his points");
    s.slay(foreman, [at[0] - 3.0, at[1], at[2]], at);
    assert!(s.hand_in(94906).is_some(), "step 3 is handed in");
    let items = s.chest();
    assert_eq!(items.len(), 2, "a deadly chest holds two rows");
    assert_ne!(items[0], items[1], "two distinct blues");
    assert!(items.iter().all(|i| BLUES.contains(i)), "Dungeon blues");
}

/// The dev aid `WOW_PROBE_RUMOUR` starts What the Kobolds Dug Up with Prospector Hale at the
/// point it names and puts that point's Claim Marker in the backpack, once.
#[test]
fn a_dev_rumour_stands_hale_at_the_named_point_with_its_marker() {
    let mut party = Party::starting(Realm::with_seed(1), &[10], START_ELWYNN, Box::new(NoGround));
    let point = at_point(HALE_POINTS[1]);
    party.go(0, point, 0.0);
    party.run(0.5);
    party.cmd(0, RealmCommand::DevRumour { id: 3, point: 1 });
    party.run(1.0);
    assert!(
        party.seen(0).iter().any(|e| matches!(
            e,
            SessionEvent::ItemCreate { fields, .. }
                if fields.object_entry() == Some(980037)
        )),
        "the marker for the diggings is in the backpack"
    );
    let hale = creatures(party.seen(0), HALE);
    assert_eq!(hale.len(), 1, "Hale stands for the player");
    assert!(flat_distance(hale[0].1, point) < 1.5, "at the second point");
    assert_eq!(party.realm.rumour_givers(), [hale[0].0]);

    party.cmd(0, RealmCommand::DevRumour { id: 3, point: 0 });
    party.run(1.0);
    assert_eq!(
        party.realm.rumour_givers().len(),
        1,
        "a held Rumour is not started twice"
    );
}
