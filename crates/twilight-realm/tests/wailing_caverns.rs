//! The Wailing Caverns, the slice's one Dungeon: its real entrance in The Barrens loads a fresh
//! copy of the instance, ten mobs and Lady Anacondra stand along a hand-written route through the
//! tunnels, her kill always pays a blue, a death inside comes back at the entrance with the kills
//! kept, and leaving resets the run.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::*;
use gear_common::{carried, descriptor, save_with, Carried};
use quest_common::{
    accept, completed, givers, hand_in, on_offer, quest_log, save_with_quests, Underway, TOKEN,
};

/// `AreaTrigger.dbc` 228, "Wailing Caverns - Entrance": a 13 yd sphere on Kalimdor.
const ENTRANCE: u32 = 228;
const ENTRANCE_AT: [f32; 3] = [-753.596, -2212.78, 21.54];
/// `AreaTrigger.dbc` 226, "Wailing Caverns - Exit": a 12 yd sphere inside.
const EXIT: u32 = 226;
const EXIT_AT: [f32; 3] = [-172.181, 138.98, -66.647];

/// `Map.dbc` 43.
const WAILING_CAVERNS: u32 = 43;
/// Where the entrance puts the player inside, and the exit outside (`areatrigger_teleport`).
const ARRIVAL: [f32; 3] = [-158.441, 131.601, -74.255];
const ARRIVAL_FACING: f32 = 5.847;
const OUTSIDE: [f32; 3] = [-738.462, -2217.8, 16.919];
const OUTSIDE_FACING: f32 = 6.021;

/// Lady Anacondra's creature entry.
const ANACONDRA: u32 = 3671;

/// The cave seen from the realm: no terrain (a WMO-only map), and walls that block sight.
struct Caverns {
    /// Wall segments in the plane, `[x, y]` to `[x, y]`.
    walls: Vec<([f32; 2], [f32; 2])>,
}

impl Caverns {
    fn open() -> Self {
        Self { walls: Vec::new() }
    }
}

impl Host for Caverns {
    fn ground_height(&self, map: u32, _x: f32, _y: f32, _z_hint: f32) -> Option<f32> {
        // Outside, flat ground; inside, nothing the terrain could say.
        (map != WAILING_CAVERNS).then_some(0.0)
    }

    fn line_of_sight(&self, _map: u32, from: [f32; 3], to: [f32; 3]) -> bool {
        let (a, b) = ([from[0], from[1]], [to[0], to[1]]);
        !self.walls.iter().any(|&(c, d)| crosses(a, b, c, d))
    }
    fn area_at(&self, _map: u32, _x: f32, _y: f32) -> Option<u32> {
        None
    }
}

/// Whether segments `ab` and `cd` cross in the plane.
fn crosses(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2]) -> bool {
    let side = |p: [f32; 2], q: [f32; 2], r: [f32; 2]| {
        (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    };
    let (d1, d2) = (side(c, d, a), side(c, d, b));
    let (d3, d4) = (side(a, b, c), side(a, b, d));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// A player script against the realm, keeping every event the client has seen.
struct Script {
    realm: Realm,
    me: u64,
    host: Caverns,
    events: Vec<SessionEvent>,
}

impl Script {
    /// A warrior of `level` carrying `money` and `items`, standing in the entrance's trigger.
    fn at_the_entrance(level: u8, money: u32, items: &[Carried]) -> Self {
        Self::seeded(7, level, money, items)
    }

    /// The same, with the realm's dice from `seed`.
    fn seeded(seed: u64, level: u8, money: u32, items: &[Carried]) -> Self {
        Self::seeded_as("basic_warrior", seed, level, money, items)
    }

    /// The same, in another Calling.
    fn seeded_as(calling: &str, seed: u64, level: u8, money: u32, items: &[Carried]) -> Self {
        let (me, blob) = save_with(level, money, items);
        Self::from_save(calling, seed, me, &blob, ENTRANCE_AT)
    }

    /// The character `blob` saves, in `calling`, standing at `at` on Kalimdor.
    fn from_save(calling: &str, seed: u64, me: u64, blob: &str, at: [f32; 3]) -> Self {
        let blob = place(blob, at).replace(
            "calling = \"basic_warrior\"",
            &format!("calling = \"{calling}\""),
        );
        let mut realm = Realm::with_seed(seed);
        realm.import_character(&blob).unwrap();
        let mut s = Self {
            realm,
            me,
            host: Caverns::open(),
            events: Vec::new(),
        };
        s.send(RealmCommand::EnterWorld { guid: me });
        s.tick(0.0);
        s
    }

    fn send(&mut self, command: RealmCommand) -> Vec<SessionEvent> {
        self.realm.handle(command);
        let new = drain(&mut self.realm);
        self.events.extend(new.clone());
        new
    }

    fn tick(&mut self, dt: f32) -> Vec<SessionEvent> {
        self.realm.tick(dt, &self.host);
        let new = drain(&mut self.realm);
        self.events.extend(new.clone());
        new
    }

    /// Ticks `secs` in 0.1 s steps.
    fn run(&mut self, secs: f32) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        for _ in 0..(secs * 10.0).round() as u32 {
            events.extend(self.tick(0.1));
        }
        events
    }

    /// Walks into the entrance and loads the cave, as the client does; the events of the ack.
    fn enter(&mut self) -> Vec<SessionEvent> {
        self.send(RealmCommand::AreaTrigger { id: ENTRANCE });
        let mut events = self.send(RealmCommand::WorldportAck);
        events.extend(self.tick(0.0));
        events
    }
}

/// `blob` with the character standing at `at` on Kalimdor.
fn place(blob: &str, at: [f32; 3]) -> String {
    // The character's own keys come before any table, such as the bind point's.
    let mut top = true;
    blob.lines()
        .map(|l| {
            top &= !l.starts_with('[');
            if top && l.starts_with("position = ") {
                format!("position = [{}, {}, {}]", at[0], at[1], at[2])
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The worldport in `events`: `(map, position, orientation, needs_ack)`.
fn worldport(events: &[SessionEvent]) -> Option<(u32, [f32; 3], f32, bool)> {
    events.iter().find_map(|e| match e {
        SessionEvent::Worldport {
            map_id,
            position,
            orientation,
            needs_ack,
        } => Some((*map_id, *position, *orientation, *needs_ack)),
        _ => None,
    })
}

fn transfer_pending(events: &[SessionEvent]) -> Option<u32> {
    events.iter().find_map(|e| match e {
        SessionEvent::TransferPending { map_id, .. } => Some(*map_id),
        _ => None,
    })
}

/// The units created in `events`: `(guid, entry, position)`.
fn unit_creates(events: &[SessionEvent]) -> Vec<(u64, u32, [f32; 3])> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } => Some((*guid, fields.object_entry()?, *position)),
            _ => None,
        })
        .collect()
}

fn creature_name(s: &mut Script, entry: u32) -> Option<String> {
    s.send(RealmCommand::CreatureQuery { entry })
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::CreatureName { name, .. } => name,
            _ => None,
        })
}

fn near(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 0.01)
}

// ── Getting in and out ────────────────────────────────────────────────────────────────────────────

#[test]
fn walking_into_the_real_entrance_loads_the_wailing_caverns_behind_a_loading_screen() {
    let mut s = Script::at_the_entrance(20, 0, &[]);
    let events = s.send(RealmCommand::AreaTrigger { id: ENTRANCE });
    assert_eq!(transfer_pending(&events), Some(WAILING_CAVERNS));
    let (map, at, facing, needs_ack) = worldport(&events).expect("a worldport into the cave");
    assert_eq!(map, WAILING_CAVERNS);
    assert!(near(at, ARRIVAL), "{at:?}");
    assert_eq!(facing, ARRIVAL_FACING);
    assert!(
        needs_ack,
        "the realm waits for the client to have loaded the cave"
    );

    // Until the client says the cave is loaded, nothing of it is sent.
    assert!(unit_creates(&s.run(2.0)).is_empty());

    let events = s.send(RealmCommand::WorldportAck);
    assert!(
        events.iter().any(|e| matches!(
            e,
            SessionEvent::ObjectCreate { guid, kind: EntityKind::Player, .. } if *guid == s.me
        )),
        "the client dropped every object at the worldport: the player is created again"
    );
    let units = unit_creates(&events);
    assert_eq!(units.len(), 11, "ten mobs and the boss");
    assert!(units.iter().any(|(_, entry, _)| *entry == ANACONDRA));
    assert_eq!(
        creature_name(&mut s, ANACONDRA).as_deref(),
        Some("Lady Anacondra")
    );
}

#[test]
fn leaving_through_the_exit_returns_the_player_to_the_entrance_outside() {
    let mut s = Script::at_the_entrance(20, 0, &[]);
    s.enter();
    s.send(RealmCommand::Move {
        motion: Default::default(),
        position: EXIT_AT,
        orientation: 0.0,
    });
    let events = s.send(RealmCommand::AreaTrigger { id: EXIT });
    assert_eq!(transfer_pending(&events), Some(KALIMDOR));
    let (map, at, facing, needs_ack) = worldport(&events).expect("a worldport out");
    assert_eq!((map, needs_ack), (KALIMDOR, true));
    assert!(near(at, OUTSIDE), "{at:?}");
    assert_eq!(facing, OUTSIDE_FACING);

    s.send(RealmCommand::WorldportAck);
    let mut events = s.tick(0.0);
    // The oasis round the skull is quiet ground; a little way out, The Barrens stream back in.
    s.send(RealmCommand::Move {
        motion: Default::default(),
        position: [-831.0, -2100.0, OUTSIDE[2]],
        orientation: 0.0,
    });
    events.extend(s.tick(0.0));
    assert!(
        !unit_creates(&events).is_empty(),
        "The Barrens stream back in beyond the entrance"
    );
    assert!(unit_creates(&events)
        .iter()
        .all(|(_, entry, _)| *entry != ANACONDRA));
}

#[test]
fn a_trigger_claimed_from_far_away_does_nothing() {
    let (me, blob) = save_with(20, 0, &[]);
    let mut realm = Realm::new();
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &Caverns::open());
    drain(&mut realm);
    // Standing in the Crossroads, nowhere near the cave.
    realm.handle(RealmCommand::AreaTrigger { id: ENTRANCE });
    assert_eq!(worldport(&drain(&mut realm)), None);
}

// ── The route ─────────────────────────────────────────────────────────────────────────────────────

/// A Deviate Ravager's spawn in the low hall, and its guid.
const RAVAGER_AT: [f32; 3] = [-67.853, 203.787, -93.573];
const RAVAGER: u64 = 0xF130 << 48 | 3636 << 24 | 26216;
/// Further on along the tunnel (`escort_12`), round a bend from the Ravager.
const ROUND_THE_BEND: [f32; 3] = [-34.6, 221.3, -95.8];

/// The paths of every run `guid` was sent on.
fn runs(events: &[SessionEvent], guid: u64) -> Vec<Vec<[f32; 3]>> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::MonsterMove { guid: g, path, .. } if *g == guid => Some(path.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_chasing_mob_follows_the_route_round_a_wall_instead_of_through_it() {
    let mut s = Script::at_the_entrance(30, 0, &[]);
    // The rock between the Ravager and the bend, which the straight line runs through.
    s.host.walls.push(([-55.0, 214.0], [-45.0, 207.0]));
    assert!(!s
        .host
        .line_of_sight(WAILING_CAVERNS, RAVAGER_AT, ROUND_THE_BEND));
    s.enter();
    // The Ravager notices the warrior beside it, who then runs round the bend.
    stand_by(&mut s, RAVAGER_AT);
    s.run(0.5);
    s.send(RealmCommand::Move {
        motion: Default::default(),
        position: ROUND_THE_BEND,
        orientation: 3.6,
    });
    let events = s.run(15.0);

    let runs = runs(&events, RAVAGER);
    assert!(!runs.is_empty(), "the Ravager gives chase");
    for path in &runs {
        for leg in path.windows(2) {
            assert!(
                s.host.line_of_sight(WAILING_CAVERNS, leg[0], leg[1]),
                "a leg through the wall: {leg:?}"
            );
        }
    }
    assert!(
        runs.iter().any(|p| p.len() > 2),
        "the way round goes by the route's nodes: {runs:?}"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            SessionEvent::AttackerState(a) if a.attacker == RAVAGER && a.victim == s.me
        )),
        "and it gets there"
    );
}

// ── The boss ──────────────────────────────────────────────────────────────────────────────────────

const ANACONDRA_AT: [f32; 3] = [-53.984, 271.39, -92.844];
const ANACONDRA_GUID: u64 = 0xF130 << 48 | (ANACONDRA as u64) << 24 | 300999;
/// A two-handed axe, to make short work of her.
const PROSPECTOR_AXE: u32 = 12975;
const MAIN_HAND: u8 = 15;

/// The item reference the realm's item tables are generated from.
fn loot_items() -> serde_json::Value {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/twilights-eve/loot_items.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// An item's vanilla required level, from the reference: the item query reports 0 for every item
/// (gear follows the Calling), but the loot bands still go by it.
fn required_level(entry: u32) -> u32 {
    fn find(v: &serde_json::Value, entry: u32) -> Option<u32> {
        match v {
            serde_json::Value::Object(o) => {
                if o.get("entry").and_then(|e| e.as_u64()) == Some(u64::from(entry)) {
                    if let Some(l) = o.get("required_level").and_then(|l| l.as_u64()) {
                        return Some(l as u32);
                    }
                }
                o.values().find_map(|v| find(v, entry))
            }
            serde_json::Value::Array(a) => a.iter().find_map(|v| find(v, entry)),
            _ => None,
        }
    }
    find(&loot_items(), entry).unwrap_or_else(|| panic!("{entry} is not in the reference"))
}

/// The reference's boss pool: the run's level band of real Wailing Caverns blues.
fn boss_pool() -> Vec<u32> {
    loot_items()["dungeon_blues"]["proposed_boss_pool"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_u64().unwrap() as u32)
        .collect()
}

fn last_health(events: &[SessionEvent], guid: u64) -> Option<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectValues { guid: g, fields } if *g == guid => fields.unit_health(),
            _ => None,
        })
        .next_back()
}

/// Stands beside `at`, facing it, a yard and a half toward the entrance side.
fn stand_by(s: &mut Script, at: [f32; 3]) {
    let here = [at[0] - 1.5, at[1] - 1.5, at[2]];
    s.send(RealmCommand::Move {
        motion: Default::default(),
        position: here,
        orientation: (at[1] - here[1]).atan2(at[0] - here[0]),
    });
}

/// Fights `guid` at `at` until it dies (at most five minutes).
fn kill(s: &mut Script, guid: u64, at: [f32; 3]) {
    let mark = s.events.len();
    stand_by(s, at);
    s.send(RealmCommand::AttackSwing { guid });
    for _ in 0..3000 {
        if last_health(&s.events[mark..], guid) == Some(0) {
            return;
        }
        assert_ne!(
            last_health(&s.events[mark..], s.me),
            Some(0),
            "the warrior died fighting {guid:#x}"
        );
        s.tick(0.1);
    }
    panic!("{guid:#x} outlived five minutes of fighting");
}

/// What `entry`'s template says its quality is.
fn quality(s: &mut Script, entry: u32) -> u32 {
    s.send(RealmCommand::ItemQuery { entry })
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::ItemTemplate {
                info: Some(info), ..
            } => Some(info.quality),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no template for {entry}"))
}

/// The Gear a warrior of 20 or more wears into the run: `(slot, entry)`, the Wolfpack Medallion,
/// Jouster's Chestplate, the Ring of Defense, the Thunderbrow Ring, Headsplitter and Kresh's
/// Back. Only a Chest carries armour now (ADR 0007), so the shield makes up the rest.
const RUN_GEAR: [(u8, u32); 6] = [
    (1, 5754),
    (4, 8157),
    (10, 12985),
    (11, 13097),
    (MAIN_HAND, 7786),
    (16, 13245),
];

/// The blues on Lady Anacondra's corpse after a geared warrior of `level` kills her, with their
/// required levels.
fn boss_blues(seed: u64, level: u8) -> Vec<(u32, u32)> {
    let items: Vec<Carried> = RUN_GEAR.iter().map(|&(slot, e)| (slot, e, 1)).collect();
    // A Zealot: past the Basic Warrior's Tier Cap, with a Tier Bonus that makes up for the set
    // only a Chest now stands in for (ADR 0007).
    let mut s = Script::seeded_as("zealot", seed, level, 0, &items);
    s.enter();
    kill(&mut s, ANACONDRA_GUID, ANACONDRA_AT);
    let loot = s.send(RealmCommand::Loot {
        guid: ANACONDRA_GUID,
    });
    let items = loot
        .iter()
        .find_map(|e| match e {
            SessionEvent::LootResponse { items, .. } => Some(items.clone()),
            _ => None,
        })
        .expect("her corpse has loot");
    items
        .iter()
        .filter(|i| quality(&mut s, i.item_id) == 3)
        .map(|i| (i.item_id, required_level(i.item_id)))
        .collect()
}

#[test]
fn lady_anacondra_always_drops_one_blue_from_the_runs_level_band() {
    // At level 25: a blue of levels 21-25, which the warrior can wear now. Some of hers are.
    let pool = boss_pool();
    let mut hers = 0;
    for seed in [1, 2, 3, 4, 5, 6] {
        let blues = boss_blues(seed, 25);
        assert_eq!(blues.len(), 1, "seed {seed}: {blues:?}");
        let (entry, required) = blues[0];
        assert!(
            (21..=25).contains(&required),
            "seed {seed}: needs {required}"
        );
        hers += usize::from(pool.contains(&entry));
    }
    assert!(hers > 0, "her own blues drop in their band");
}

#[test]
fn the_boss_blue_follows_the_runners_level() {
    // At level 30 her own blues are all below the band: a blue of levels 26-30 drops instead.
    for seed in [1, 2, 3] {
        let blues = boss_blues(seed, 30);
        assert_eq!(blues.len(), 1, "seed {seed}: {blues:?}");
        let (_, required) = blues[0];
        assert!(
            (26..=30).contains(&required),
            "seed {seed}: needs {required}"
        );
    }
}

// ── Death, and the run's kills ────────────────────────────────────────────────────────────────────

/// The Deviate Guardian alone on the first bend, and its guid.
const GUARDIAN_AT: [f32; 3] = [-100.639, 180.884, -78.428];
const GUARDIAN: u64 = 0xF130 << 48 | 3637 << 24 | 26202;

/// Whether `guid` was created in `events`.
fn created(events: &[SessionEvent], guid: u64) -> bool {
    unit_creates(events).iter().any(|(g, _, _)| *g == guid)
}

/// Stands among the boss and her druids without fighting back until the warrior dies (at most
/// ten minutes).
fn die_at_the_boss(s: &mut Script) {
    stand_by(s, ANACONDRA_AT);
    for _ in 0..6000 {
        if last_health(&s.events, s.me) == Some(0) {
            return;
        }
        s.tick(0.1);
    }
    panic!("the warrior outlived ten minutes among the boss's pack");
}

#[test]
fn dying_inside_comes_back_at_the_entrance_with_the_toll_taken_and_the_kills_kept() {
    // Level 60: the Guardian has twice the health and hits half again as hard, and a level 25
    // warrior with an axe no longer outlasts him.
    let mut s = Script::at_the_entrance(60, 1000, &[(MAIN_HAND, PROSPECTOR_AXE, 1)]);
    s.enter();
    kill(&mut s, GUARDIAN, GUARDIAN_AT);
    die_at_the_boss(&mut s);
    assert_eq!(descriptor(&s.events, s.me).player_money(), Some(900));

    let events = s.send(RealmCommand::RepopRequest);
    let (position, orientation) = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::Teleport {
                guid,
                position,
                orientation,
                ..
            } if *guid == s.me => Some((*position, *orientation)),
            _ => None,
        })
        .expect("a teleport to the entrance, inside");
    assert!(near(position, ARRIVAL), "{position:?}");
    assert_eq!(orientation, ARRIVAL_FACING);
    assert_eq!(worldport(&events), None, "the same map: no loading screen");
    let me = descriptor(&s.events, s.me);
    assert_eq!(me.unit_health(), me.unit_max_health());

    // Walking back in, the Guardian still lies dead; the rest still stand.
    let mark = s.events.len();
    s.send(RealmCommand::Move {
        motion: Default::default(),
        position: GUARDIAN_AT,
        orientation: 0.0,
    });
    s.run(5.0);
    let since = &s.events[mark..];
    assert!(!created(since, GUARDIAN), "the Guardian came back");
    assert_eq!(last_health(&s.events, GUARDIAN), Some(0));
    assert!(
        !since
            .iter()
            .any(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == GUARDIAN)),
        "its corpse stays"
    );
}

#[test]
fn leaving_and_coming_back_starts_a_fresh_run() {
    let mut s = Script::at_the_entrance(60, 0, &[(MAIN_HAND, PROSPECTOR_AXE, 1)]);
    s.enter();
    kill(&mut s, GUARDIAN, GUARDIAN_AT);

    s.send(RealmCommand::Move {
        motion: Default::default(),
        position: EXIT_AT,
        orientation: 0.0,
    });
    s.send(RealmCommand::AreaTrigger { id: EXIT });
    s.send(RealmCommand::WorldportAck);
    s.tick(0.0);
    s.send(RealmCommand::Move {
        motion: Default::default(),
        position: ENTRANCE_AT,
        orientation: 0.0,
    });
    let events = s.enter();

    let units = unit_creates(&events);
    assert_eq!(units.len(), 11, "every mob back");
    for guid in [GUARDIAN, ANACONDRA_GUID] {
        assert!(created(&events, guid), "{guid:#x}");
        let f = descriptor(&events, guid);
        assert_eq!(
            f.unit_health(),
            f.unit_max_health(),
            "{guid:#x} at full health"
        );
    }
    // And the boss pays out again.
    kill(&mut s, ANACONDRA_GUID, ANACONDRA_AT);
    let loot = s.send(RealmCommand::Loot {
        guid: ANACONDRA_GUID,
    });
    assert!(loot
        .iter()
        .any(|e| matches!(e, SessionEvent::LootResponse { items, .. } if items.len() == 1)));
}

// ── Leaving otherwise ─────────────────────────────────────────────────────────────────────────────

const HEARTHSTONE: u32 = 6948;
const FIRST_PACK_SLOT: u8 = 23;

#[test]
fn the_hearthstone_takes_the_player_out_of_the_cave_to_the_crossroads() {
    let mut s = Script::at_the_entrance(20, 0, &[(FIRST_PACK_SLOT, HEARTHSTONE, 1)]);
    s.enter();
    s.send(RealmCommand::UseItem {
        bag: 255,
        slot: FIRST_PACK_SLOT,
    });
    let events = s.run(10.5);
    assert_eq!(transfer_pending(&events), Some(KALIMDOR));
    let (map, at, _, needs_ack) = worldport(&events).expect("a worldport to the Hub");
    assert_eq!((map, needs_ack), (KALIMDOR, true));
    let [x, y, z, _] = twilight_realm::CROSSROADS;
    assert!(near(at, [x, y, z]), "{at:?}");

    s.send(RealmCommand::WorldportAck);
    let events = s.tick(0.0);
    assert!(
        !unit_creates(&events).is_empty(),
        "the Crossroads and its camps come back"
    );
    // Zoning saved the character where it now stands.
    let saved = s
        .realm
        .drain_storage()
        .into_iter()
        .filter_map(|op| match op {
            twilight_realm::Storage::Write { blob, .. } => Some(blob),
            _ => None,
        })
        .next_back()
        .expect("a save on zoning");
    assert!(saved.contains("map = 1\n"), "{saved}");
}

#[test]
fn logging_out_inside_comes_back_outside_the_entrance() {
    let mut s = Script::at_the_entrance(20, 0, &[]);
    s.enter();
    s.send(RealmCommand::Logout);
    let me = s.me;
    let events = s.send(RealmCommand::EnterWorld { guid: me });
    let (map, at, facing, needs_ack) = worldport(&events).expect("the login's worldport");
    assert_eq!((map, needs_ack), (KALIMDOR, false));
    assert!(near(at, OUTSIDE), "{at:?}");
    assert_eq!(facing, OUTSIDE_FACING);
}

// ── The Disciples' quests ─────────────────────────────────────────────────────────────────────────

/// The Disciples of Naralex on the skull: Nalpak and Ebru.
const NALPAK: u32 = 5767;
const EBRU: u32 = 5768;
/// Their Repeatable Quests: kill the Deviates (Nalpak), bring their hides (Ebru), slay Lady
/// Anacondra (Nalpak).
const THE_NIGHTMARES_BROOD: u32 = 92001;
const DEVIATE_HIDES: u32 = 92002;
const THE_FANGLORD: u32 = 92003;
/// The Deviates' quest drop.
const DEVIATE_HIDE: u32 = 6443;
/// The skull's crown: the ridge above the Caverns' mouth in the Lushwater Oasis, over its teeth
/// (pinned in the client). The tunnels from the mouth lead down to the entrance's trigger.
const SKULL: [f32; 3] = [-780.0, -2034.0, 100.0];
/// How far from the crown's middle a Disciple may stand, in yards.
const ON_THE_SKULL_YD: f32 = 15.0;

impl Script {
    /// Walks up to the Disciple `entry` on the skull; its guid. The player must be on Kalimdor.
    fn go_to_disciple(&mut self, entry: u32) -> u64 {
        let (guid, _, at) = givers(&self.events)
            .into_iter()
            .find(|g| g.1 == entry)
            .unwrap_or_else(|| panic!("Disciple {entry} stands on the skull"));
        self.send(RealmCommand::Move {
            motion: Default::default(),
            position: at,
            orientation: 0.0,
        });
        self.tick(0.0);
        guid
    }

    /// Walks out through the exit, back to The Barrens.
    fn leave(&mut self) {
        self.send(RealmCommand::Move {
            motion: Default::default(),
            position: EXIT_AT,
            orientation: 0.0,
        });
        self.send(RealmCommand::AreaTrigger { id: EXIT });
        self.send(RealmCommand::WorldportAck);
        self.tick(0.0);
    }

    /// Keeps `events`, drained by a helper outside the script, as seen by the client.
    fn keep(&mut self, events: Vec<SessionEvent>) -> Vec<SessionEvent> {
        self.events.extend(events.clone());
        events
    }

    /// The quest's state byte in the log, if it is there: 1 once complete.
    fn quest_state(&self, quest: u32) -> Option<u8> {
        quest_log(&self.events, self.me)
            .into_iter()
            .find(|q| q.1 == quest)
            .map(|q| q.3)
    }
}

/// A Vanguard of `level` on the skull, carrying `items`, with these quests under way.
fn questing_at_the_skull(seed: u64, level: u8, items: &[Carried], quests: &[Underway]) -> Script {
    questing_at_the_skull_as("vanguard", seed, level, items, quests)
}

/// [`questing_at_the_skull`], in `calling`.
fn questing_at_the_skull_as(
    calling: &str,
    seed: u64,
    level: u8,
    items: &[Carried],
    quests: &[Underway],
) -> Script {
    let (me, blob) = save_with_quests(level, items, 0, quests);
    Script::from_save(calling, seed, me, &blob, SKULL)
}

impl Script {
    /// Goes down the tunnels to the entrance and into the Caverns.
    fn go_in(&mut self) {
        self.send(RealmCommand::Move {
            motion: Default::default(),
            position: ENTRANCE_AT,
            orientation: 0.0,
        });
        self.enter();
    }
}

fn flat_yd(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

#[test]
fn the_disciples_stand_on_the_skull_and_offer_the_caverns_quests_from_tier_one() {
    let mut s = questing_at_the_skull(7, 16, &[], &[]);
    let disciples: Vec<_> = givers(&s.events)
        .into_iter()
        .filter(|g| [NALPAK, EBRU].contains(&g.1))
        .collect();
    assert_eq!(disciples.len(), 2, "Nalpak and Ebru stand there");
    for (_, entry, at) in &disciples {
        let yd = flat_yd(*at, SKULL);
        assert!(
            yd < ON_THE_SKULL_YD,
            "{entry} stands {yd} yd from the crown"
        );
    }
    assert_eq!(creature_name(&mut s, NALPAK).as_deref(), Some("Nalpak"));
    assert_eq!(creature_name(&mut s, EBRU).as_deref(), Some("Ebru"));

    let mut offered = Vec::new();
    for entry in [NALPAK, EBRU] {
        let npc = s.go_to_disciple(entry);
        offered.extend(on_offer(&mut s.realm, npc));
    }
    offered.sort_unstable();
    assert_eq!(offered, [THE_NIGHTMARES_BROOD, DEVIATE_HIDES, THE_FANGLORD]);

    // A Tier 0 character at its cap has nothing from them yet.
    let mut s = questing_at_the_skull_as("basic_warrior", 7, 10, &[], &[]);
    for entry in [NALPAK, EBRU] {
        let npc = s.go_to_disciple(entry);
        assert!(on_offer(&mut s.realm, npc).is_empty(), "{entry} at Tier 0");
    }
}

#[test]
fn slaying_lady_anacondra_pays_a_full_level_and_three_tokens_and_is_offered_again_at_once() {
    let items: Vec<Carried> = RUN_GEAR.iter().map(|&(slot, e)| (slot, e, 1)).collect();
    // A Zealot: only the Chest carries armour (ADR 0007), and its Tier Bonus makes up for the
    // rest of the set a level 25 warrior used to wear against her.
    let mut s = questing_at_the_skull_as("zealot", 1, 25, &items, &[]);
    let nalpak = s.go_to_disciple(NALPAK);
    let taken = accept(&mut s.realm, nalpak, THE_FANGLORD);
    s.keep(taken);
    assert_eq!(s.quest_state(THE_FANGLORD), Some(0), "under way");

    s.go_in();
    kill(&mut s, ANACONDRA_GUID, ANACONDRA_AT);
    assert_eq!(
        s.quest_state(THE_FANGLORD),
        Some(1),
        "her kill completes it"
    );
    s.leave();

    let nalpak = s.go_to_disciple(NALPAK);
    assert_eq!(descriptor(&s.events, s.me).unit_level(), Some(25));
    let turned_in = hand_in(&mut s.realm, nalpak, THE_FANGLORD);
    let turned_in = s.keep(turned_in);
    let done = completed(&turned_in).expect("the quest completes");
    // Level 25's whole 131900 XP, times the Calling's Tier XP Rate of 5: a Repeatable Quest pays
    // the player's next level.
    assert_eq!((done.quest_id, done.xp), (THE_FANGLORD, 659500));
    assert_eq!(done.items, [(TOKEN, 3)]);
    assert_eq!(carried(&s.events, s.me, TOKEN), 3);
    assert_eq!(s.quest_state(THE_FANGLORD), None, "the slot clears");

    // Repeatable: Nalpak offers it again at once, and it can be taken again.
    assert!(on_offer(&mut s.realm, nalpak).contains(&THE_FANGLORD));
    let taken = accept(&mut s.realm, nalpak, THE_FANGLORD);
    s.keep(taken);
    assert_eq!(s.quest_state(THE_FANGLORD), Some(0), "under way again");
}

#[test]
fn a_deviate_killed_inside_counts_toward_nalpaks_quest() {
    // Five Ravagers and both Guardians down already: the sixth Ravager finishes it.
    let mut s = questing_at_the_skull(
        7,
        60,
        &[(MAIN_HAND, PROSPECTOR_AXE, 1)],
        &[(THE_NIGHTMARES_BROOD, [5, 2, 0, 0])],
    );
    assert_eq!(s.quest_state(THE_NIGHTMARES_BROOD), Some(0));
    s.go_in();
    let mark = s.events.len();
    kill(&mut s, RAVAGER, RAVAGER_AT);
    assert!(s.events[mark..].iter().any(|e| matches!(
        e,
        SessionEvent::QuestObjectiveKill { quest_id, entry: 3636, count: 6, required: 6 }
            if *quest_id == THE_NIGHTMARES_BROOD
    )));
    assert_eq!(s.quest_state(THE_NIGHTMARES_BROOD), Some(1));

    s.leave();
    let nalpak = s.go_to_disciple(NALPAK);
    let done = completed(&hand_in(&mut s.realm, nalpak, THE_NIGHTMARES_BROOD))
        .expect("the quest completes");
    assert_eq!(done.items, [(TOKEN, 3)]);
}

#[test]
fn the_deviates_inside_drop_their_hides_for_ebrus_quest() {
    // At 80% a hide falls from most Guardians; over three runs, one does. The warrior wears the
    // run's chest and shield: the flat table (ADR 0012) gives a Guardian no level-gap miss.
    let dropped = [1, 2, 3].into_iter().any(|seed| {
        let mut s = questing_at_the_skull(
            seed,
            25,
            &[
                (4, 8157, 1),
                (MAIN_HAND, PROSPECTOR_AXE, 1),
                (16, 13245, 1),
                (23, DEVIATE_HIDE, 4),
            ],
            &[(DEVIATE_HIDES, [0; 4])],
        );
        s.go_in();
        kill(&mut s, GUARDIAN, GUARDIAN_AT);
        s.send(RealmCommand::Loot { guid: GUARDIAN })
            .iter()
            .any(|e| {
                matches!(
                    e,
                    SessionEvent::LootResponse { items, .. }
                        if items.iter().any(|i| i.item_id == DEVIATE_HIDE)
                )
            })
    });
    assert!(dropped, "no Guardian dropped a hide");

    // Five hides in the bags: Ebru takes them for a full level and three Tokens.
    let mut s = questing_at_the_skull(7, 20, &[(23, DEVIATE_HIDE, 5)], &[(DEVIATE_HIDES, [0; 4])]);
    assert_eq!(
        s.quest_state(DEVIATE_HIDES),
        Some(1),
        "complete from the bags"
    );
    let ebru = s.go_to_disciple(EBRU);
    let done = completed(&hand_in(&mut s.realm, ebru, DEVIATE_HIDES)).expect("it completes");
    // Level 20's whole 57300 XP, times the Tier XP Rate of 3.
    assert_eq!(done.xp, 171900);
    assert_eq!(done.items, [(TOKEN, 3)]);
}
