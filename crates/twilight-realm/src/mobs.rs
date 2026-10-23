//! The mobs: their templates and spawns (`content/mobs.toml` by hand, `content/barrens.toml`
//! and `content/elwynn.toml` generated from the research dumps by `examples/import_barrens.rs`
//! and `examples/import_elwynn.rs`), and each one's state in the world.

use std::collections::HashMap;
use std::sync::OnceLock;

use benilla_protocol::messages::{MonsterMoveFacing, ObjectType};
use benilla_protocol::{EntityKind, MoveSpeeds, ObjectFields, SessionEvent};
use serde::Deserialize;

use crate::clock;
use crate::creature_spells::CreatureSpell;
use crate::fields::*;
use crate::phased::{self, Phase};
use crate::rng::Rng;
use crate::rules::melee::MobDefense;
use crate::rules::school::School;
use crate::rules::stats::{MobStats, UnitClass};
use crate::space::distance;
use crate::Host;

/// `HIGHGUID_UNIT`, in a creature guid's top 16 bits.
const HIGHGUID_UNIT: u64 = 0xF130;

/// `UNIT_NPC_FLAGS`, and its `UNIT_NPC_FLAG_QUESTGIVER` bit.
const UNIT_NPC_FLAGS: u16 = 147;
const UNIT_NPC_FLAG_QUESTGIVER: u32 = 0x2;
/// `UNIT_NPC_FLAG_GOSSIP`.
const UNIT_NPC_FLAG_GOSSIP: u32 = 0x1;

/// vanilla's base movement speeds, yd/s, which a template's multipliers scale.
const BASE_WALK: f32 = 2.5;
const BASE_RUN: f32 = 7.0;

/// A creature's bounding radius and combat reach when its model says nothing else.
const BOUNDING_RADIUS: f32 = 0.5;
pub(crate) const COMBAT_REACH: f32 = 1.5;

/// The first low guid given to a creature a call for help brings in.
const CALLED_GUID_BASE: u32 = 0x90_0000;

/// The first low guid given to a creature an Encounter's wave brings in.
const ENCOUNTER_GUID_BASE: u32 = 0xA0_0000;

/// The first low guid given to a creature row a timed call brings in.
const TIMED_GUID_BASE: u32 = 0xB0_0000;

/// The first low guid of a creature a timed call has brought in: one for every wave, so a wave
/// never shares a guid with the one before.
const TIMED_LIVE_BASE: u32 = 0xC0_0000;

/// The first low guid of a creature made at run time for an Audience.
pub(crate) const AUDIENCE_GUID_BASE: u32 = 0xD0_0000;

/// A creature guid: `entry` and the low guid `low` under `HIGHGUID_UNIT`.
pub(crate) fn unit_guid(entry: u32, low: u32) -> u64 {
    HIGHGUID_UNIT << 48 | u64::from(entry & 0xFF_FFFF) << 24 | u64::from(low & 0xFF_FFFF)
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
pub(crate) struct Template {
    pub(crate) entry: u32,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) subname: String,
    pub(crate) creature_type: u32,
    /// `CreatureEliteType`: 0 normal, 1 elite, 2 rare elite, 3 world boss, 4 rare.
    #[serde(default)]
    pub(crate) rank: u32,
    /// Its fights raise the Boss Bar (#280); independent of `rank`.
    #[serde(default)]
    pub(crate) boss: bool,
    pub(crate) display_id: u32,
    pub(crate) faction_template: u32,
    /// Attacks a player who comes too close; a neutral creature only fights back.
    #[serde(default = "yes")]
    pub(crate) hostile: bool,
    /// Friendly to every player: it never fights, and no attack or spell may target it.
    #[serde(default)]
    pub(crate) friendly: bool,
    /// It may stand in deep water (ADR 0021).
    #[serde(default)]
    pub(crate) swims: bool,
    pub(crate) unit_class: UnitClass,
    #[allow(dead_code)]
    pub(crate) level_min: u8,
    #[allow(dead_code)]
    pub(crate) level_max: u8,
    pub(crate) health_multiplier: f32,
    pub(crate) damage_multiplier: f32,
    pub(crate) armor_multiplier: f32,
    /// What a hand-tuned row was tuned to; its multipliers above are this divided by the creature
    /// difficulty. Read by the content test that fails when the difficulty moves it.
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) tuned: Option<Tuned>,
    pub(crate) base_attack_time: u32,
    pub(crate) speed_walk: f32,
    pub(crate) speed_run: f32,
    pub(crate) detection_range: f32,
    /// vmangos `creature_template.flags_extra`: `NO_PARRY` and `NO_BLOCK` take those slices off
    /// the table.
    #[serde(default)]
    pub(crate) flags_extra: u32,
    /// It carries a shield (`creature_equip_template`): a mob blocks only with one.
    #[serde(default)]
    pub(crate) shield: bool,
    /// Seconds the player must be away before it respawns, in place of its rank's
    /// (`rules::respawn`): a named elite's longer wait.
    #[serde(default)]
    pub(crate) respawn_secs: Option<f32>,
    /// Loot rolls on top of the ordinary one: the first to succeed adds one of its items.
    #[serde(default)]
    pub(crate) rolls: Vec<crate::rules::loot::Roll>,
    /// The Creature Spells it casts in a fight (`content/creature_spells.toml` keys).
    #[serde(default)]
    pub(crate) spells: Vec<String>,
    /// The item entries it shows in its main hand, off hand and ranged slots, 0 for none
    /// (`UNIT_VIRTUAL_ITEM_SLOT_DISPLAY`).
    #[serde(default)]
    pub(crate) equip: Vec<u32>,
    /// A training dummy: it never moves or fights, takes damage and auras normally, and rather
    /// than dying it heals to full [`INERT_RESET_SECS`] after it last took damage.
    #[serde(default)]
    pub(crate) inert: bool,
    /// The stacks its death adds to its Phased Dungeon's meter, in place of the meter's own.
    #[serde(default)]
    pub(crate) noise_on_death: Option<u32>,
    /// It wakes the sleepers within this many yards of it as it passes.
    #[serde(default)]
    pub(crate) wakes_sleepers: Option<f32>,
    /// When its Phased Dungeon's meter is at its maximum it wakes and hunts the nearest character,
    /// waking the sleepers within this many yards of it as it goes.
    #[serde(default)]
    pub(crate) hunts_at_meter_max: Option<f32>,
    /// False for a creature that never swings and never closes to melee, such as one that only
    /// casts from where it stands.
    #[serde(default = "yes")]
    pub(crate) melee: bool,
}

/// The creature entries a test flagged as bosses, for the rest of the process.
static EXTRA_BOSSES: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());

/// Flags creature `entry`s as bosses, as the template's `boss` key does.
pub(crate) fn add_bosses(entries: &[u32]) {
    if let Ok(mut flagged) = EXTRA_BOSSES.lock() {
        flagged.extend_from_slice(entries);
    }
}

/// The number the next fight to begin takes, so fights sort in the order they began.
pub(crate) fn next_fight() -> u64 {
    static FIGHTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    FIGHTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Seconds after an inert creature last took damage before it is whole again.
pub(crate) const INERT_RESET_SECS: f32 = 10.0;

/// `CREATURE_FLAG_EXTRA_NO_PARRY` and `CREATURE_FLAG_EXTRA_NO_BLOCK` (vmangos).
const FLAG_EXTRA_NO_PARRY: u32 = 0x4;
const FLAG_EXTRA_NO_BLOCK: u32 = 0x10;

impl Template {
    /// A Bounty Hunter's template (`crate::bounty`): a hostile humanoid elite named for one
    /// hunter, at whatever level it is made.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn hunter(
        entry: u32,
        name: String,
        subname: String,
        display_id: u32,
        faction_template: u32,
        unit_class: UnitClass,
        base_attack_time: u32,
        speed_run: f32,
        spells: Vec<String>,
        equip: Vec<u32>,
    ) -> Self {
        Self {
            entry,
            name,
            subname,
            // `CREATURE_TYPE_HUMANOID`.
            creature_type: 7,
            rank: 1,
            boss: true,
            display_id,
            faction_template,
            hostile: true,
            friendly: false,
            swims: false,
            unit_class,
            level_min: 1,
            level_max: 63,
            health_multiplier: 1.0,
            damage_multiplier: 1.0,
            armor_multiplier: 1.0,
            base_attack_time,
            speed_walk: 1.0,
            speed_run,
            detection_range: 20.0,
            flags_extra: 0,
            shield: false,
            respawn_secs: None,
            rolls: Vec::new(),
            spells,
            equip,
            inert: false,
            noise_on_death: None,
            wakes_sleepers: None,
            hunts_at_meter_max: None,
            melee: true,
            tuned: None,
        }
    }

    /// Its fights raise the Boss Bar: the template says so, or a test named its entry
    /// ([`add_bosses`]).
    pub(crate) fn raises_boss_bar(&self) -> bool {
        self.boss
            || EXTRA_BOSSES
                .lock()
                .is_ok_and(|entries| entries.contains(&self.entry))
    }

    /// Elite and boss ranks, which kill XP pays double for.
    pub(crate) fn elite(&self) -> bool {
        matches!(self.rank, 1..=3)
    }
}

/// Where a mob stands when it (re)spawns.
#[derive(Default)]
pub(crate) struct Spawn {
    /// The spawn's low guid (24 bits).
    pub(crate) guid: u32,
    pub(crate) entry: u32,
    pub(crate) level_min: u8,
    pub(crate) level_max: u8,
    /// WoW `[x, y, z]`; z the spawn's height in the data, which the host's ground replaces once
    /// it is loaded, while `grounded`.
    pub(crate) position: [f32; 3],
    /// Whether it stands on the host's ground (`crate::ground`); a row says `grounded = false`
    /// to stand exactly at its written Z.
    pub(crate) grounded: bool,
    pub(crate) orientation: f32,
    pub(crate) display_id: u32,
    /// How far from its spawn point it strolls while idle, in yards; 0 stands still.
    pub(crate) wander_radius: f32,
    /// The camp it fights with: pulling one member pulls the rest.
    pub(crate) camp: u32,
    /// The map it stands on: Kalimdor, or a Dungeon's.
    pub(crate) map: u32,
    /// The open world's, or a Phased Dungeon's.
    pub(crate) phase: Phase,
    /// The calls for help it makes in a fight.
    pub(crate) on_health: &'static [phased::OnHealth],
    /// A creature a call for help brings in, not one the Run starts with: the caller's guid and
    /// which of its calls.
    pub(crate) called_by: Option<(u32, usize)>,
    /// The timed calls it makes in a fight.
    pub(crate) every: &'static [phased::Every],
    /// A creature a timed call brings in: the caller's guid and which of its timed calls.
    pub(crate) timed_by: Option<(u32, usize)>,
    /// When a dungeon creature is in the Run; none for any other spawn.
    pub(crate) appears: Option<&'static phased::Appears>,
    /// A creature an Encounter's wave brings in, not one the Population streams: the encounter's
    /// index in `content/encounters.toml` and the wave's.
    pub(crate) encounter: Option<(usize, usize)>,
    /// It starts each life asleep: no aggro, no movement, until something wakes it.
    pub(crate) asleep: bool,
    /// Yards added to the distance at which a character wakes it.
    pub(crate) wake_radius_mod: f32,
    /// A row built from the vmangos spawns in `barrens.toml` or `elwynn.toml`, not one we placed:
    /// the spawn audit (ADR 0021) leaves it out.
    pub(crate) imported: bool,
}

/// The high bits of a Camp number a dungeon file names, apart from every creature guid and from
/// a Bounty Hunter's own camp.
const DUNGEON_CAMP: u32 = 1 << 30;

/// A hand-written spawn in `mobs.toml`, a camp of its own.
#[derive(Deserialize)]
struct HandSpawn {
    guid: u32,
    entry: u32,
    level: u8,
    position: [f32; 3],
    #[serde(default = "crate::ground::yes")]
    grounded: bool,
    orientation: f32,
}

/// A generated spawn row in `barrens.toml` or `elwynn.toml`: guid, entry, level_min, level_max, x, y, z,
/// orientation, display_id, wander_radius, camp.
#[derive(Deserialize)]
struct Row(u32, u32, u8, u8, f32, f32, f32, f32, u32, f32, u32);

/// `[difficulty]` in `mobs.toml`: factors on every non-inert creature's health and melee damage,
/// laid over its template's own multipliers.
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Difficulty {
    pub(crate) health: f32,
    pub(crate) damage: f32,
    /// Each level past 1 multiplies health by `1 + health_per_level` (ADR 0012).
    health_per_level: f32,
    damage_per_level: f32,
    rank: RankFactors,
}

/// `[difficulty.rank]`: `[health, damage]` factors for each rank above normal.
#[derive(Deserialize, Clone, Copy)]
struct RankFactors {
    elite: [f32; 2],
    rare_elite: [f32; 2],
    boss: [f32; 2],
    rare: [f32; 2],
}

impl Difficulty {
    /// The health and damage factors on a non-inert creature of `rank` at `level` in `zone`.
    fn muls(&self, rank: u32, level: u8, zone: ZoneFactor) -> (f32, f32) {
        let [rank_health, rank_damage] = match rank {
            1 => self.rank.elite,
            2 => self.rank.rare_elite,
            3 => self.rank.boss,
            4 => self.rank.rare,
            _ => [1.0, 1.0],
        };
        let steps = i32::from(level.max(1)) - 1;
        (
            self.health * (1.0 + self.health_per_level).powi(steps) * rank_health * zone.health,
            self.damage * (1.0 + self.damage_per_level).powi(steps) * rank_damage * zone.damage,
        )
    }
}

/// A hand-tuned row's `tuned`: its final health and damage multipliers on the vanilla class
/// table at `level_min`, in an area with no Zone Factor, after every creature difficulty factor.
#[derive(Debug, Deserialize, Clone, Copy)]
#[allow(dead_code)]
pub(crate) struct Tuned {
    pub(crate) health: f32,
    pub(crate) damage: f32,
}

/// A creature's **Zone Factor** (ADR 0012): health and damage factors for the area it stands in.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
pub(crate) struct ZoneFactor {
    pub(crate) health: f32,
    pub(crate) damage: f32,
}

impl ZoneFactor {
    /// An area with no row, nor a parent with one.
    pub(crate) const NONE: Self = Self {
        health: 1.0,
        damage: 1.0,
    };
}

/// A `[[zone]]` row of `mobs.toml`: the Zone Factor of the `AreaTable.dbc` area `area`.
#[derive(Deserialize)]
struct ZoneRow {
    area: u32,
    health: f32,
    damage: f32,
}

#[derive(Deserialize)]
struct HandFile {
    difficulty: Difficulty,
    zone: Vec<ZoneRow>,
    template: Vec<Template>,
    spawn: Vec<HandSpawn>,
}

/// A Dungeon's creatures (`content/wailing_caverns.toml`); the rest of the file is
/// [`crate::dungeon`]'s.
#[derive(Deserialize)]
struct DungeonFile {
    map: u32,
    template: Vec<Template>,
    spawn: Vec<DungeonSpawn>,
}

#[derive(Deserialize)]
struct DungeonSpawn {
    guid: u32,
    entry: u32,
    level_min: u8,
    level_max: u8,
    position: [f32; 3],
    #[serde(default = "crate::ground::yes")]
    grounded: bool,
    orientation: f32,
    camp: u32,
}

/// `content/elwynn_overrides.toml`: hand-written changes to the generated Elwynn spawns.
#[derive(Deserialize)]
struct Overrides {
    /// Entries whose spawns are deleted.
    removed: Vec<u32>,
    /// Spawns deleted by guid, where their entry also spawns elsewhere and must stay there.
    #[serde(default)]
    removed_guids: Vec<u32>,
    moved: Vec<Moved>,
    /// Spawns added by hand, each a camp of its own; they keep their own levels, past the Level
    /// Bands.
    #[serde(default)]
    added: Vec<Added>,
}

#[derive(Deserialize)]
struct Added {
    guid: u32,
    entry: u32,
    level_min: u8,
    level_max: u8,
    position: [f32; 3],
    #[serde(default = "crate::ground::yes")]
    grounded: bool,
    orientation: f32,
}

#[derive(Deserialize)]
struct Moved {
    entry: u32,
    position: [f32; 3],
    #[serde(default = "crate::ground::yes")]
    grounded: bool,
    orientation: f32,
}

#[derive(Deserialize)]
struct ZoneFile {
    spawns: Vec<Row>,
    template: Vec<Template>,
}

/// `content/level_overrides.toml`: the Level Bands laid over the generated spawns.
#[derive(Deserialize)]
struct BandFile {
    area: Vec<BandArea>,
}

#[derive(Deserialize)]
struct BandArea {
    map: u32,
    x: [f32; 2],
    y: [f32; 2],
    creature: Vec<BandCreature>,
}

#[derive(Deserialize)]
struct BandCreature {
    entry: u32,
    levels: [u8; 2],
}

pub(crate) struct Mobs {
    pub(crate) difficulty: Difficulty,
    /// Each `[[zone]]` row's factor, by area.
    zones: HashMap<u32, ZoneFactor>,
    pub(crate) template: Vec<Template>,
    pub(crate) spawn: Vec<Spawn>,
    /// Entries whose template comes from `elwynn.toml` and no hand-written one overrides.
    vanilla: Vec<u32>,
}

/// Whether `entry`'s template is vanilla Elwynn's, not one `mobs.toml` defines.
pub(crate) fn is_vanilla(entry: u32) -> bool {
    content().vanilla.contains(&entry)
}

/// The Level Band range for a creature of `entry` standing at `position` on `map`, when its
/// template is vanilla and the point is in a band. For a creature spawned at runtime; the
/// static spawns take theirs when the content loads.
pub(crate) fn banded(entry: u32, map: u32, position: [f32; 3]) -> Option<[u8; 2]> {
    if map != crate::content::EASTERN_KINGDOMS || !is_vanilla(entry) {
        return None;
    }
    crate::bands::range_at(position[0], position[1])
}

pub(crate) fn content() -> &'static Mobs {
    static MOBS: OnceLock<Mobs> = OnceLock::new();
    MOBS.get_or_init(|| {
        let hand: HandFile =
            toml::from_str(include_str!("../content/mobs.toml")).expect("content/mobs.toml parses");
        let barrens: ZoneFile = toml::from_str(include_str!("../content/barrens.toml"))
            .expect("content/barrens.toml parses");
        let mut elwynn: ZoneFile = toml::from_str(include_str!("../content/elwynn.toml"))
            .expect("content/elwynn.toml parses");
        let overrides: Overrides = toml::from_str(include_str!("../content/elwynn_overrides.toml"))
            .expect("content/elwynn_overrides.toml parses");
        elwynn.spawns.retain(|r| {
            !overrides.removed.contains(&r.1) && !overrides.removed_guids.contains(&r.0)
        });
        let difficulty = hand.difficulty;
        let zone_factors = hand
            .zone
            .iter()
            .map(|z| {
                let factor = ZoneFactor {
                    health: z.health,
                    damage: z.damage,
                };
                (z.area, factor)
            })
            .collect();
        let mut template = hand.template;
        template.extend(barrens.template);
        let vanilla: Vec<u32> = elwynn
            .template
            .iter()
            .map(|t| t.entry)
            .filter(|e| !template.iter().any(|t| t.entry == *e))
            .collect();
        template.extend(elwynn.template);
        let mut spawn: Vec<Spawn> = hand
            .spawn
            .into_iter()
            .filter_map(|s| {
                let display_id = template.iter().find(|t| t.entry == s.entry)?.display_id;
                Some(Spawn {
                    guid: s.guid,
                    entry: s.entry,
                    level_min: s.level,
                    level_max: s.level,
                    position: s.position,
                    grounded: s.grounded,
                    orientation: s.orientation,
                    display_id,
                    wander_radius: 0.0,
                    camp: s.guid,
                    map: crate::content::KALIMDOR,
                    phase: Phase::OpenWorld,
                    on_health: &[],
                    called_by: None,
                    ..Default::default()
                })
            })
            .collect();
        let zones = [
            (barrens.spawns, crate::content::KALIMDOR),
            (elwynn.spawns, crate::content::EASTERN_KINGDOMS),
        ];
        for (rows, map) in zones {
            spawn.extend(rows.into_iter().map(
                |Row(guid, entry, lmin, lmax, x, y, z, o, display_id, wander, camp)| Spawn {
                    guid,
                    entry,
                    level_min: lmin,
                    level_max: lmax.max(lmin),
                    position: [x, y, z],
                    grounded: true,
                    orientation: o,
                    display_id,
                    wander_radius: wander,
                    camp,
                    map,
                    phase: Phase::OpenWorld,
                    on_health: &[],
                    called_by: None,
                    imported: true,
                    ..Default::default()
                },
            ));
        }
        // The hand-written moves of generated Elwynn spawns, before the Level Bands read where
        // each stands. A moved spawn stands where we put it, so the audit counts it as ours.
        for m in &overrides.moved {
            for s in spawn
                .iter_mut()
                .filter(|s| s.map == crate::content::EASTERN_KINGDOMS && s.entry == m.entry)
            {
                s.position = m.position;
                s.grounded = m.grounded;
                s.orientation = m.orientation;
                s.imported = false;
            }
        }
        let bands: BandFile = toml::from_str(include_str!("../content/level_overrides.toml"))
            .expect("content/level_overrides.toml parses");
        for s in &mut spawn {
            let [x, y, _] = s.position;
            let levels = bands
                .area
                .iter()
                .filter(|a| a.map == s.map && (a.x[0]..=a.x[1]).contains(&x))
                .filter(|a| (a.y[0]..=a.y[1]).contains(&y))
                .flat_map(|a| &a.creature)
                .find(|c| c.entry == s.entry);
            if let Some(c) = levels {
                [s.level_min, s.level_max] = c.levels;
            }
        }
        // The Level Bands: a vanilla Elwynn spawn takes its sub-zone's levels.
        for s in &mut spawn {
            if s.map != crate::content::EASTERN_KINGDOMS || !vanilla.contains(&s.entry) {
                continue;
            }
            if let Some(range) = crate::bands::range_at(s.position[0], s.position[1]) {
                [s.level_min, s.level_max] = range;
            }
        }
        for a in overrides.added {
            let t = template
                .iter()
                .find(|t| t.entry == a.entry)
                .unwrap_or_else(|| panic!("added spawn {} names no template", a.guid));
            spawn.push(Spawn {
                guid: a.guid,
                entry: a.entry,
                level_min: a.level_min,
                level_max: a.level_max.max(a.level_min),
                position: a.position,
                grounded: a.grounded,
                orientation: a.orientation,
                display_id: t.display_id,
                wander_radius: 0.0,
                camp: a.guid,
                map: crate::content::EASTERN_KINGDOMS,
                phase: Phase::OpenWorld,
                on_health: &[],
                called_by: None,
                ..Default::default()
            });
        }
        let dungeon: DungeonFile = toml::from_str(include_str!("../content/wailing_caverns.toml"))
            .expect("content/wailing_caverns.toml parses");
        template.extend(dungeon.template);
        for t in &template {
            for roll in &t.rolls {
                roll.check(&t.name);
            }
        }
        for s in dungeon.spawn {
            let Some(t) = template.iter().find(|t| t.entry == s.entry) else {
                continue;
            };
            spawn.push(Spawn {
                guid: s.guid,
                entry: s.entry,
                level_min: s.level_min,
                level_max: s.level_max.max(s.level_min),
                position: s.position,
                grounded: s.grounded,
                orientation: s.orientation,
                display_id: t.display_id,
                wander_radius: 0.0,
                camp: s.camp,
                map: dungeon.map,
                phase: Phase::OpenWorld,
                on_health: &[],
                called_by: None,
                ..Default::default()
            });
        }
        let mut called_guids = 0;
        let mut timed_guids = 0;
        for d in phased::content() {
            for c in &d.creature {
                let Some(t) = template.iter().find(|t| t.entry == c.entry) else {
                    continue;
                };
                spawn.push(Spawn {
                    guid: c.guid,
                    entry: c.entry,
                    level_min: c.level,
                    level_max: c.level,
                    position: c.position,
                    grounded: c.grounded,
                    orientation: c.orientation,
                    display_id: t.display_id,
                    wander_radius: c.wander,
                    camp: c.camp.map_or(c.guid, |n| DUNGEON_CAMP | d.id << 16 | n),
                    map: d.map,
                    phase: Phase::Dungeon(d.id),
                    on_health: &c.on_health,
                    called_by: None,
                    every: &c.every,
                    timed_by: None,
                    appears: Some(&c.appears),
                    encounter: None,
                    asleep: c.asleep,
                    wake_radius_mod: c.wake_radius_mod,
                    imported: false,
                });
                // The creatures its calls bring in: rows the Run does not start with, each
                // with a guid of its own above every authored one.
                for (k, call) in c.on_health.iter().enumerate() {
                    let t = template
                        .iter()
                        .find(|t| t.entry == call.entry)
                        .unwrap_or_else(|| panic!("creature {}'s call names no template", c.guid));
                    for _ in 0..call.count {
                        spawn.push(Spawn {
                            guid: CALLED_GUID_BASE + called_guids,
                            entry: call.entry,
                            level_min: call.level,
                            level_max: call.level,
                            position: call.position,
                            grounded: true,
                            orientation: c.orientation,
                            display_id: t.display_id,
                            wander_radius: 0.0,
                            camp: c.guid,
                            map: d.map,
                            phase: Phase::Dungeon(d.id),
                            on_health: &[],
                            called_by: Some((c.guid, k)),
                            ..Default::default()
                        });
                        called_guids += 1;
                    }
                }
                // And those its timed calls bring in, one row per creature of a wave.
                for (k, call) in c.every.iter().enumerate() {
                    let t = template
                        .iter()
                        .find(|t| t.entry == call.entry)
                        .unwrap_or_else(|| {
                            panic!("creature {}'s timed call names no template", c.guid)
                        });
                    for _ in 0..call.count {
                        spawn.push(Spawn {
                            guid: TIMED_GUID_BASE + timed_guids,
                            entry: call.entry,
                            level_min: call.level,
                            level_max: call.level,
                            position: call.position,
                            grounded: true,
                            orientation: c.orientation,
                            display_id: t.display_id,
                            camp: c.guid,
                            map: d.map,
                            phase: Phase::Dungeon(d.id),
                            timed_by: Some((c.guid, k)),
                            ..Default::default()
                        });
                        timed_guids += 1;
                    }
                }
            }
        }
        // The creatures an Encounter's waves bring in: rows the Population never streams, each
        // with a guid of its own above every called one.
        let mut encounter_guids = 0;
        for (e, enc) in crate::encounters::content().iter().enumerate() {
            for (w, wave) in enc.wave.iter().enumerate() {
                for c in &wave.creatures {
                    let t = template
                        .iter()
                        .find(|t| t.entry == c.entry)
                        .unwrap_or_else(|| panic!("encounter {} names no template", enc.key));
                    for _ in 0..c.count {
                        spawn.push(Spawn {
                            guid: ENCOUNTER_GUID_BASE + encounter_guids,
                            entry: c.entry,
                            level_min: c.level,
                            level_max: c.level,
                            position: c.position,
                            grounded: true,
                            display_id: t.display_id,
                            camp: ENCOUNTER_GUID_BASE + encounter_guids,
                            map: crate::content::EASTERN_KINGDOMS,
                            phase: Phase::OpenWorld,
                            encounter: Some((e, w)),
                            ..Default::default()
                        });
                        encounter_guids += 1;
                    }
                }
            }
        }
        Mobs {
            difficulty,
            zones: zone_factors,
            template,
            spawn,
            vanilla,
        }
    })
}

/// The global health and damage factors from `mobs.toml`'s `[difficulty]`.
pub(crate) fn difficulty_muls() -> (f32, f32) {
    let d = content().difficulty;
    (d.health, d.damage)
}

/// The Zone Factor of `area`: its own `[[zone]]` row, else its parent zone's
/// (`content/area_parents.toml`), else none.
pub(crate) fn zone_factor(area: Option<u32>) -> ZoneFactor {
    let zones = &content().zones;
    area.and_then(|a| zones.get(&a).or_else(|| zones.get(area_parent(a)?)))
        .copied()
        .unwrap_or(ZoneFactor::NONE)
}

/// The Zone Factor of a creature made at `position` on `map` in `phase`: a Phased Dungeon's
/// comes from its file's `zone`, the open world's from the host's area under the point.
pub(crate) fn zone_factor_at(
    host: &dyn crate::Host,
    phase: Phase,
    map: u32,
    position: [f32; 3],
) -> ZoneFactor {
    zone_factor(match phase {
        Phase::OpenWorld => host.area_at(map, position[0], position[1]),
        Phase::Dungeon(_) => phased::zone(phase),
    })
}

/// The zone `area` is a subzone of, from `content/area_parents.toml`.
pub(crate) fn area_parent(area: u32) -> Option<&'static u32> {
    #[derive(Deserialize)]
    struct Row {
        id: u32,
        zone: u32,
    }
    #[derive(Deserialize)]
    struct File {
        area: Vec<Row>,
    }
    static PARENTS: OnceLock<HashMap<u32, u32>> = OnceLock::new();
    PARENTS
        .get_or_init(|| {
            let file: File = toml::from_str(include_str!("../content/area_parents.toml"))
                .expect("content/area_parents.toml parses");
            file.area.into_iter().map(|r| (r.id, r.zone)).collect()
        })
        .get(&area)
}

pub(crate) fn template(entry: u32) -> Option<&'static Template> {
    content()
        .template
        .iter()
        .find(|t| t.entry == entry)
        .or_else(|| crate::bounty::template(entry))
}

/// `SMSG_CREATURE_QUERY_RESPONSE` for `entry`; an unknown entry answers with no name.
pub(crate) fn creature_name(entry: u32) -> SessionEvent {
    let t = template(entry);
    SessionEvent::CreatureName {
        entry,
        name: t.map(|t| t.name.clone()),
        subname: t.map(|t| t.subname.clone()),
        creature_type: t.map(|t| t.creature_type),
        pet_family: 0,
        rank: t.map_or(0, |t| t.rank),
        type_flags: 0,
        display_id: t.map_or(0, |t| t.display_id),
        civilian: false,
        racial_leader: false,
    }
}

/// What a mob is doing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum State {
    /// Standing at its spawn point.
    Idle,
    /// Fighting the player.
    Engaged,
    /// Running home after the fight was lost to it; ignores everything on the way.
    Evading,
    Dead,
}

/// A walk along `points` (straight legs, at one speed) over `duration` seconds, `elapsed` of it
/// done.
#[derive(Debug, Clone)]
pub(crate) struct Walk {
    pub(crate) points: Vec<[f32; 3]>,
    pub(crate) duration: f32,
    pub(crate) elapsed: f32,
    /// Where the player stood when this chase leg was laid.
    pub(crate) aimed_at: [f32; 3],
}

/// One mob in the world.
pub(crate) struct Mob {
    pub(crate) guid: u64,
    pub(crate) template: &'static Template,
    pub(crate) stats: MobStats,
    pub(crate) health: u32,
    pub(crate) position: [f32; 3],
    pub(crate) orientation: f32,
    pub(crate) home: [f32; 3],
    pub(crate) home_orientation: f32,
    pub(crate) state: State,
    pub(crate) walk: Option<Walk>,
    /// Seconds until the next swing; 0 is ready.
    pub(crate) swing_timer: f32,
    /// Seconds until the chase may lay a new leg.
    pub(crate) repath_timer: f32,
    /// What slows on it leave of its run speed: 1.0 unslowed.
    pub(crate) speed_mul: f32,
    next_spline_id: u32,
    /// Its index in [`Mobs::spawn`].
    pub(crate) spawn: u32,
    pub(crate) camp: u32,
    /// Seconds until it answers a campmate's cry for help; `None` when no one has called.
    pub(crate) assist_in: Option<f32>,
    /// How far from home it strolls while idle.
    wander_radius: f32,
    /// Seconds until an idle mob sets off on its next stroll.
    wander_in: f32,
    display_id: u32,
    /// The Creature Spell it is casting, and seconds until it goes off.
    pub(crate) casting: Option<(&'static CreatureSpell, f32)>,
    /// Its Creature Spells' cooldowns: `(key, seconds left)`.
    pub(crate) spell_cooldowns: Vec<(&'static str, f32)>,
    /// Schools a Kick locked it out of: `(school, seconds left)`.
    pub(crate) lockouts: Vec<(School, f32)>,
    /// A Taunt on it: the taunter and the seconds left. It fights the taunter while it can.
    pub(crate) taunted: Option<(u64, f32)>,
    /// Seconds of a knockback slide left: it does not swing, cast or chase while above 0.
    pub(crate) knocked: f32,
    /// Seconds it has been in combat; 0 out of it.
    pub(crate) engaged_secs: f32,
    /// The character of another session its cast under way goes on; none for the one it fights.
    pub(crate) cast_at: Option<u64>,
    /// Seconds until it next decides whether to cast.
    pub(crate) think_in: f32,
    /// Which of its spawn's calls for help it has made, this life.
    pub(crate) called: Vec<bool>,
    /// The player it fights, or last fought; 0 before any.
    pub(crate) target: u64,
    /// Its threat per player this life (#147): whom it fights.
    pub(crate) threat: crate::rules::threat::Table,
    /// The player it hunts (a Bounty Hunter's owner): it notices no one else, though it fights
    /// back whoever attacks it.
    pub(crate) hunts: Option<u64>,
    /// It gives up a chase that strays past its leash (`rules::leash`).
    pub(crate) leashes: bool,
    /// Another session's player, stood in for (a duel opponent): it never acts and never dies,
    /// its health stopping at 1, and what lands on it is delivered to that session.
    pub(crate) remote: bool,
    /// The Bounty it is: whose hunter, and at what Bounty Rank. Its kill pays Blood Coin.
    pub(crate) bounty: Option<Bounty>,
    /// What its swing time is multiplied by: below 1 swings faster.
    pub(crate) attack_time_mul: f32,
    /// Creature Spells it casts besides its template's.
    pub(crate) extra_spells: Vec<&'static CreatureSpell>,
    /// A Bounty Hunter's affixes, by key.
    pub(crate) affixes: Vec<&'static str>,
    /// An inert creature's seconds since it last took damage; `None` while it is whole.
    pub(crate) since_hurt: Option<f32>,
    /// A player has hurt it this life, and its dungeon's meter has heard.
    pub(crate) noised: bool,
    /// Its death has been heard by its dungeon's meter.
    pub(crate) death_noised: bool,
    /// Asleep: no aggro, no threat, no movement, lying down on the client, until woken for the
    /// rest of its life.
    pub(crate) asleep: bool,
    /// Yards added to the distance at which a character wakes it.
    pub(crate) wake_radius_mod: f32,
    /// It has woken to hunt at its dungeon's meter maximum, and wakes the sleepers it passes.
    pub(crate) hunting: bool,
    /// How many times it has made each of its spawn's timed calls since the pull.
    pub(crate) timed_made: Vec<u32>,
    /// The creature whose timed call brought it in.
    pub(crate) brought_by: Option<u64>,
    /// The number of the fight its threat table began (`next_fight`); none while the table is
    /// empty. The Boss Bar lists fights in this order.
    pub(crate) fight_no: Option<u64>,
    /// The characters that see it and that it acts on; none for every mob a spawn row or a
    /// Phased Dungeon places. Fixed at spawn.
    pub(crate) audience: Option<crate::audience::Audience>,
}

/// A Bounty Hunter's tag: the player it was sent after, and their Bounty Rank when it came.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Bounty {
    pub(crate) owner: u64,
    pub(crate) rank: u32,
    /// It killed its owner, who is **Marked** until it falls (#179): it roams where it won, and
    /// its kill lifts the mark instead of raising their rank.
    pub(crate) victor: bool,
}

/// The spawn index of a mob that no spawn placed.
pub(crate) const NO_SPAWN: u32 = u32::MAX;

/// The pause between one stroll and the next, in seconds.
const WANDER_PAUSE: (f32, f32) = (6.0, 16.0);

impl Mob {
    /// The mob of `content().spawn[index]` at `level`, standing at `z` (the ground under its
    /// spawn point), under the Zone Factor `zone`.
    pub(crate) fn spawn(index: u32, level: u8, z: f32, zone: ZoneFactor) -> Option<Self> {
        Self::from_spawn(content().spawn.get(index as usize)?, index, level, z, zone)
    }

    /// Stands it, and its home, at height `z`: for one made before its ground was known.
    pub(crate) fn ground_to(&mut self, z: f32) {
        self.position[2] = z;
        self.home[2] = z;
    }

    /// `template`'s stats at `level`, under the creature difficulty: its level, its rank and
    /// the Zone Factor `zone`.
    pub(crate) fn stats_for(template: &Template, level: u8, zone: ZoneFactor) -> MobStats {
        let (health_mul, damage_mul) = content().difficulty.muls(template.rank, level, zone);
        if template.inert {
            MobStats::new(
                template.unit_class,
                level,
                template.health_multiplier,
                template.damage_multiplier,
                template.armor_multiplier,
            )
        } else {
            MobStats::new(
                template.unit_class,
                level,
                template.health_multiplier * health_mul,
                template.damage_multiplier * damage_mul,
                template.armor_multiplier,
            )
        }
    }

    /// A mob no spawn placed, made at run time for `audience` (ADR 0014): it is seen by and acts
    /// on those characters only, and nothing respawns it. Built as a hunter is, then freed of
    /// the hunt.
    pub(crate) fn for_audience(
        guid: u64,
        template: &'static Template,
        level: u8,
        position: [f32; 3],
        audience: crate::audience::Audience,
        zone: ZoneFactor,
    ) -> Self {
        let stats = Self::stats_for(template, level, zone);
        let mut mob = Self::hunter(guid, template, stats, position, 0.0, 0);
        mob.hunts = None;
        mob.leashes = true;
        mob.audience = Some(audience);
        mob
    }

    /// The mob of `spawn`, which is `content().spawn[index]`, at `level` under the Zone Factor
    /// `zone`.
    pub(crate) fn from_spawn(
        spawn: &'static Spawn,
        index: u32,
        level: u8,
        z: f32,
        zone: ZoneFactor,
    ) -> Option<Self> {
        let template = template(spawn.entry)?;
        let stats = Self::stats_for(template, level, zone);
        let guid = HIGHGUID_UNIT << 48
            | u64::from(spawn.entry & 0xFF_FFFF) << 24
            | u64::from(spawn.guid & 0xFF_FFFF);
        let [x, y, _] = spawn.position;
        Some(Self {
            guid,
            template,
            stats,
            health: stats.max_health,
            position: [x, y, z],
            orientation: spawn.orientation,
            home: [x, y, z],
            home_orientation: spawn.orientation,
            state: State::Idle,
            walk: None,
            swing_timer: 0.0,
            repath_timer: 0.0,
            speed_mul: 1.0,
            next_spline_id: 1,
            spawn: index,
            camp: spawn.camp,
            assist_in: None,
            wander_radius: spawn.wander_radius,
            // A stagger, so a camp's mobs don't all set off together.
            wander_in: WANDER_PAUSE.0 + (spawn.guid % 97) as f32 / 97.0 * WANDER_PAUSE.1,
            display_id: spawn.display_id,
            casting: None,
            spell_cooldowns: Vec::new(),
            lockouts: Vec::new(),
            taunted: None,
            knocked: 0.0,
            engaged_secs: 0.0,
            cast_at: None,
            think_in: 0.0,
            called: vec![false; spawn.on_health.len()],
            asleep: spawn.asleep,
            wake_radius_mod: spawn.wake_radius_mod,
            hunting: false,
            timed_made: vec![0; spawn.every.len()],
            brought_by: None,
            fight_no: None,
            audience: None,
            target: 0,
            threat: Default::default(),
            hunts: None,
            leashes: true,
            remote: false,
            bounty: None,
            attack_time_mul: 1.0,
            extra_spells: Vec::new(),
            affixes: Vec::new(),
            since_hurt: None,
            noised: false,
            death_noised: false,
        })
    }

    /// The mob of the timed-call row `index` for wave `wave` of `brought_by`: its own guid, since
    /// the row's repeats as the fight goes on.
    pub(crate) fn timed(
        index: u32,
        level: u8,
        z: f32,
        zone: ZoneFactor,
        wave: u32,
        brought_by: u64,
    ) -> Option<Self> {
        let mut mob = Self::spawn(index, level, z, zone)?;
        let low = TIMED_LIVE_BASE + (wave & 0x3F_FFFF);
        mob.guid = unit_guid(mob.template.entry, low);
        mob.brought_by = Some(brought_by);
        Some(mob)
    }

    /// A mob no spawn placed, of `template` with `stats`, standing at `position`: it hunts
    /// `hunts` and never leashes. Its home is where it stands.
    pub(crate) fn hunter(
        guid: u64,
        template: &'static Template,
        stats: MobStats,
        position: [f32; 3],
        orientation: f32,
        hunts: u64,
    ) -> Self {
        Self {
            guid,
            template,
            stats,
            health: stats.max_health,
            position,
            orientation,
            home: position,
            home_orientation: orientation,
            state: State::Idle,
            walk: None,
            swing_timer: 0.0,
            repath_timer: 0.0,
            speed_mul: 1.0,
            next_spline_id: 1,
            spawn: NO_SPAWN,
            // A camp of its own: no other mob evades with it.
            camp: guid as u32 | 1 << 31,
            assist_in: None,
            wander_radius: 0.0,
            wander_in: WANDER_PAUSE.0,
            display_id: template.display_id,
            casting: None,
            spell_cooldowns: Vec::new(),
            lockouts: Vec::new(),
            taunted: None,
            knocked: 0.0,
            engaged_secs: 0.0,
            cast_at: None,
            think_in: 0.0,
            called: Vec::new(),
            asleep: false,
            wake_radius_mod: 0.0,
            hunting: false,
            timed_made: Vec::new(),
            brought_by: None,
            fight_no: None,
            audience: None,
            target: 0,
            threat: Default::default(),
            hunts: Some(hunts),
            leashes: false,
            remote: false,
            bounty: None,
            attack_time_mul: 1.0,
            extra_spells: Vec::new(),
            affixes: Vec::new(),
            since_hurt: None,
            noised: false,
            death_noised: false,
        }
    }

    /// A Bounty Hunter that killed its owner (#179): it stops hunting and roams `roam` yards
    /// around where it stands, giving up a chase that strays from there.
    pub(crate) fn settle(&mut self, roam: f32) {
        self.hunts = None;
        self.leashes = true;
        self.home = self.position;
        self.home_orientation = self.orientation;
        self.wander_radius = roam;
        if let Some(b) = &mut self.bounty {
            b.victor = true;
        }
    }

    /// The stand-in for another session's player `guid` in a fight with `foe`: their level,
    /// health and armour where they stand. It is in the fight from the start.
    pub(crate) fn stand_in(
        guid: u64,
        stats: MobStats,
        health: u32,
        position: [f32; 3],
        orientation: f32,
        foe: u64,
    ) -> Self {
        static TEMPLATE: OnceLock<Template> = OnceLock::new();
        let template = TEMPLATE.get_or_init(|| {
            let mut t = Template::hunter(
                0,
                String::new(),
                String::new(),
                0,
                0,
                UnitClass::Warrior,
                2000,
                7.0,
                Vec::new(),
                Vec::new(),
            );
            t.rank = 0;
            t
        });
        let mut mob = Self::hunter(guid, template, stats, position, orientation, foe);
        mob.hunts = None;
        mob.remote = true;
        mob.health = health.max(1);
        mob.state = State::Engaged;
        mob.target = foe;
        mob
    }

    /// The spawn that placed it; `None` for one no spawn placed.
    pub(crate) fn spawn_row(&self) -> Option<&'static Spawn> {
        content().spawn.get(self.spawn as usize)
    }

    /// An idle mob's stroll: after a pause it walks to a spot within its wander of home, and
    /// never through something solid. Does nothing while it walks, fights or flees.
    pub(crate) fn stroll(
        &mut self,
        dt: f32,
        map: u32,
        host: &dyn Host,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) {
        if self.wander_radius <= 0.0 || self.state != State::Idle || self.walk.is_some() {
            return;
        }
        self.wander_in -= dt;
        if self.wander_in > 0.0 {
            return;
        }
        self.wander_in = rng.frand(WANDER_PAUSE.0, WANDER_PAUSE.1);
        // A spot spread evenly over the wander's disc.
        let angle = rng.frand(0.0, std::f32::consts::TAU);
        let r = self.wander_radius * rng.frand(0.0, 1.0).sqrt();
        let [hx, hy, hz] = self.home;
        let (x, y) = (hx + r * angle.cos(), hy + r * angle.sin());
        let to = [x, y, host.ground_height(map, x, y, hz + 2.0).unwrap_or(hz)];
        let eye = |p: [f32; 3]| [p[0], p[1], p[2] + 2.0];
        if !host.line_of_sight(map, eye(self.position), eye(to)) {
            return;
        }
        let duration = distance(self.position, to) / (BASE_WALK * self.template.speed_walk);
        if duration <= 0.0 {
            return;
        }
        let path = vec![self.position, to];
        let spline_id = self.next_spline_id;
        self.next_spline_id += 1;
        // Aimed nowhere near a player, so a chase that begins mid-stroll lays its own leg.
        self.walk = Some(Walk {
            points: path.clone(),
            duration,
            elapsed: 0.0,
            aimed_at: [f32::MAX; 3],
        });
        events.push(SessionEvent::MonsterMove {
            guid: self.guid,
            transport: None,
            start: self.position,
            spline_id,
            path,
            facing: MonsterMoveFacing::None,
            stop: false,
            duration_ms: clock::ms(duration),
            flying: false,
            run_mode: false,
        });
    }

    pub(crate) fn level(&self) -> u8 {
        self.stats.level
    }

    /// What it defends with against a swing.
    pub(crate) fn defense(&self) -> MobDefense {
        let flags = self.template.flags_extra;
        MobDefense {
            parries: flags & FLAG_EXTRA_NO_PARRY == 0,
            blocks: self.template.shield && flags & FLAG_EXTRA_NO_BLOCK == 0,
        }
    }

    pub(crate) fn alive(&self) -> bool {
        self.state != State::Dead
    }

    /// Alive and not friendly: what an attack or a spell may hit.
    pub(crate) fn attackable(&self) -> bool {
        self.alive() && !self.template.friendly
    }

    /// Whether `viewer`'s character is an Audience of it (or it has none).
    pub(crate) fn admits(&self, viewer: u64) -> bool {
        crate::audience::admits(&self.audience, viewer)
    }

    /// [`Self::attackable`], and `viewer` can see it.
    pub(crate) fn attackable_by(&self, viewer: u64) -> bool {
        self.attackable() && self.admits(viewer)
    }

    pub(crate) fn run_speed(&self) -> f32 {
        BASE_RUN * self.template.speed_run * self.speed_mul
    }

    /// The mob's create: `SMSG_UPDATE_OBJECT`'s create block as a living unit.
    pub(crate) fn create(&self) -> SessionEvent {
        let t = self.template;
        let mut pairs = vec![
            (OBJECT_FIELD_TYPE, TYPE_UNIT),
            (OBJECT_FIELD_ENTRY, t.entry),
            (OBJECT_FIELD_SCALE_X, float(1.0)),
            (UNIT_FIELD_HEALTH, self.health),
            (UNIT_FIELD_MAXHEALTH, self.stats.max_health),
            (UNIT_FIELD_LEVEL, u32::from(self.level())),
            (UNIT_FIELD_FACTIONTEMPLATE, t.faction_template),
            // Race 0, the unit class, gender 0, mana.
            (UNIT_FIELD_BYTES_0, unit_class_id(t.unit_class) << 8),
            (UNIT_FIELD_BASEATTACKTIME, t.base_attack_time),
            (UNIT_FIELD_BASEATTACKTIME + 1, t.base_attack_time),
            (UNIT_FIELD_BOUNDINGRADIUS, float(BOUNDING_RADIUS)),
            (UNIT_FIELD_COMBATREACH, float(COMBAT_REACH)),
            (UNIT_FIELD_DISPLAYID, self.display_id),
            (UNIT_FIELD_NATIVEDISPLAYID, self.display_id),
            (UNIT_FIELD_MINDAMAGE, float(self.stats.min_damage)),
            (UNIT_FIELD_MAXDAMAGE, float(self.stats.max_damage)),
            (UNIT_FIELD_RESISTANCES, self.stats.armor),
        ];
        pairs.extend(equip_pairs(&t.equip));
        // A Field Giver shows the quest giver flag, so the client asks for its `!` and `?`.
        // A talk-to target shows the gossip flag, so the client opens its menu.
        let flags = if t.friendly {
            u32::from(crate::quests::content::talk_target(t.entry)) * UNIT_NPC_FLAG_GOSSIP
                | u32::from(crate::quests::content::field_giver(t.entry)) * UNIT_NPC_FLAG_QUESTGIVER
        } else {
            0
        };
        if flags != 0 {
            pairs.push((UNIT_NPC_FLAGS, flags));
        }
        if self.asleep {
            pairs.push((UNIT_NPC_EMOTESTATE, EMOTE_STATE_SLEEP));
        }
        pairs.extend(guid_pairs(OBJECT_FIELD_GUID, self.guid));
        SessionEvent::ObjectCreate {
            guid: self.guid,
            kind: EntityKind::Unit,
            display_id: Some(self.display_id),
            position: self.position,
            orientation: self.orientation,
            scale: 1.0,
            speeds: Some(MoveSpeeds {
                walk: BASE_WALK * t.speed_walk,
                run: self.run_speed(),
                run_back: 4.5,
                swim: 4.722_222,
                swim_back: 2.5,
                turn_rate: std::f32::consts::PI,
            }),
            mover: None,
            transport_progress: None,
            transport: None,
            spline: None,
            fields: ObjectFields::from_pairs(&pairs).into_created(ObjectType::Unit),
        }
    }

    /// Starts a straight run to `to` (ground walk: the client takes Z from its terrain).
    pub(crate) fn run_to(&mut self, to: [f32; 3], aimed_at: [f32; 3]) -> SessionEvent {
        self.run_path(vec![self.position, to], aimed_at)
    }

    /// Charges to `to` at `speed` yards a second, facing `target`: it stands there at once, as a
    /// player's charge does, and the client sees the run.
    pub(crate) fn charge_to(&mut self, to: [f32; 3], speed: f32, target: u64) -> SessionEvent {
        let from = self.position;
        self.position = to;
        self.walk = None;
        let spline_id = self.next_spline_id;
        self.next_spline_id += 1;
        SessionEvent::MonsterMove {
            guid: self.guid,
            transport: None,
            start: from,
            spline_id,
            path: vec![from, to],
            facing: MonsterMoveFacing::Target(target),
            stop: false,
            duration_ms: clock::ms(distance(from, to) / speed),
            flying: false,
            run_mode: true,
        }
    }

    /// Starts a run along `path`, which begins where the mob stands: one spline through every
    /// point, which the client walks leg by leg.
    pub(crate) fn run_path(&mut self, mut path: Vec<[f32; 3]>, aimed_at: [f32; 3]) -> SessionEvent {
        if path.first().is_none_or(|p| *p != self.position) {
            path.insert(0, self.position);
        }
        if path.len() < 2 {
            path.push(self.position);
        }
        let duration = length(&path) / self.run_speed();
        let spline_id = self.next_spline_id;
        self.next_spline_id += 1;
        self.walk = Some(Walk {
            points: path.clone(),
            duration,
            elapsed: 0.0,
            aimed_at,
        });
        SessionEvent::MonsterMove {
            guid: self.guid,
            transport: None,
            start: self.position,
            spline_id,
            path,
            facing: MonsterMoveFacing::None,
            stop: false,
            duration_ms: clock::ms(duration),
            flying: false,
            run_mode: true,
        }
    }

    /// Stops the mob where it stands, if it was moving: the stop the client needs.
    pub(crate) fn halt(&mut self) -> Option<SessionEvent> {
        self.walk.take()?;
        Some(SessionEvent::MonsterMove {
            guid: self.guid,
            transport: None,
            start: self.position,
            spline_id: 0,
            path: vec![self.position],
            facing: MonsterMoveFacing::None,
            stop: true,
            duration_ms: 0,
            flying: false,
            run_mode: false,
        })
    }

    /// Walks on along the current path by `dt` seconds.
    pub(crate) fn advance(&mut self, dt: f32) {
        let Some(w) = &mut self.walk else {
            return;
        };
        w.elapsed += dt;
        let t = if w.duration > 0.0 {
            (w.elapsed / w.duration).min(1.0)
        } else {
            1.0
        };
        // The leg `t` of the way along, by distance.
        let mut left = t * length(&w.points);
        let mut at = *w.points.last().expect("a path has points");
        let mut heading = None;
        for leg in w.points.windows(2) {
            let (from, to) = (leg[0], leg[1]);
            let d = distance(from, to);
            if d > 0.0 {
                heading = Some((to[0] - from[0], to[1] - from[1]));
            }
            if left <= d && d > 0.0 {
                let u = left / d;
                at = std::array::from_fn(|i| from[i] + (to[i] - from[i]) * u);
                break;
            }
            left -= d;
        }
        if let Some((dx, dy)) = heading {
            self.orientation = dy.atan2(dx).rem_euclid(std::f32::consts::TAU);
        }
        self.position = at;
        if t >= 1.0 {
            self.walk = None;
        }
    }
}

/// `creature_classlevelstats`'s unit class as `ChrClasses.dbc` numbers it.
fn unit_class_id(class: UnitClass) -> u32 {
    match class {
        UnitClass::Warrior => 1,
        UnitClass::Paladin => 2,
        UnitClass::Mage => 8,
    }
}

/// The length of a path through `points`.
fn length(points: &[[f32; 3]]) -> f32 {
    points.windows(2).map(|l| distance(l[0], l[1])).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elwynn_npcs_are_everyones_friends_and_its_beasts_still_fight() {
        // Northshire Guard, Marshal McBride. Innkeeper Farley is a Hub NPC now (#331).
        for entry in [1642, 197] {
            let t = template(entry).unwrap();
            assert!(t.friendly && !t.hostile, "{} fights", t.name);
            assert_eq!(t.faction_template, 35, "{} shows its own faction", t.name);
        }
        // Defias Thug.
        let thug = template(38).unwrap();
        assert!(thug.hostile && !thug.friendly);
    }

    #[test]
    fn northshire_valley_spawns_at_its_level_band_and_the_rest_keeps_vanilla() {
        // Defias Thug, in the valley; the Abbey's are its own.
        let thugs: Vec<_> = content()
            .spawn
            .iter()
            .filter(|s| s.entry == 38 && s.phase == Phase::OpenWorld)
            .filter(|s| s.position[0] > -9160.0 && s.position[1] > -480.0)
            .collect();
        assert!(thugs.len() > 50);
        for s in thugs {
            assert_eq!((s.level_min, s.level_max), (6, 8), "thug {}", s.guid);
        }
        // Kobold Laborer, all of them in Echo Ridge Mine at the valley's far end.
        let (added, laborers): (Vec<_>, Vec<_>) = content()
            .spawn
            .iter()
            .filter(|s| s.entry == 80)
            .partition(|s| s.guid >= 960_000);
        assert_eq!(laborers.len(), 25);
        assert!(laborers
            .iter()
            .all(|s| (s.level_min, s.level_max) == (6, 7)));
        // The deep tunnels' hand-added ones (#89) keep their own levels, past the band.
        assert!(!added.is_empty());
        assert!(added.iter().all(|s| s.level_min >= 4 && s.level_max <= 6));
        // Young Wolf, which also roams outside the valley toward Goldshire.
        let wolves: Vec<_> = content().spawn.iter().filter(|s| s.entry == 299).collect();
        let (inside, outside): (Vec<_>, Vec<_>) =
            wolves.into_iter().partition(|s| s.position[0] > -9160.0);
        assert!(!inside.is_empty() && !outside.is_empty());
        assert!(inside.iter().all(|s| (s.level_min, s.level_max) == (1, 2)));
        // Outside it they take their sub-zone's Elwynn Level Band (#228).
        assert!(outside.iter().all(|s| s.level_min >= 10));
    }

    #[test]
    fn a_friendly_npc_is_never_attackable() {
        let i = content()
            .spawn
            .iter()
            .position(|s| s.entry == 1642)
            .unwrap();
        let guard = Mob::spawn(i as u32, 10, 0.0, ZoneFactor::NONE).unwrap();
        assert!(guard.alive() && !guard.attackable());
        let i = content().spawn.iter().position(|s| s.entry == 6).unwrap();
        assert!(Mob::spawn(i as u32, 1, 0.0, ZoneFactor::NONE)
            .unwrap()
            .attackable());
    }

    /// The mob of `entry` at `level` in `zone` and its template's stats before the global
    /// difficulty.
    fn mob_and_vanilla(entry: u32, level: u8, zone: ZoneFactor) -> (MobStats, MobStats) {
        let spawn: &'static Spawn = Box::leak(Box::new(Spawn {
            entry,
            ..Default::default()
        }));
        let t = template(entry).unwrap();
        let vanilla = MobStats::new(
            t.unit_class,
            level,
            t.health_multiplier,
            t.damage_multiplier,
            t.armor_multiplier,
        );
        (
            Mob::from_spawn(spawn, 0, level, 0.0, zone).unwrap().stats,
            vanilla,
        )
    }

    #[test]
    fn difficulty_scales_health_and_damage_by_level_except_for_dummies() {
        // Defias Thug, a normal mob: x2 health and x1.5 damage, and 5% and 4% a level past 1.
        let (hard, vanilla) = mob_and_vanilla(38, 8, ZoneFactor::NONE);
        let (health, damage) = (2.0 * 1.05f32.powi(7), 1.5 * 1.04f32.powi(7));
        let want = (vanilla.max_health as f32 * health).round() as u32;
        assert!(
            hard.max_health.abs_diff(want) <= 1,
            "{} vs {want}",
            hard.max_health
        );
        assert!((hard.min_damage - vanilla.min_damage * damage).abs() < 1e-3);
        assert!((hard.max_damage - vanilla.max_damage * damage).abs() < 1e-3);
        assert_eq!(hard.armor, vanilla.armor);
        // Training Dummy, even in Elwynn Forest.
        let (dummy, vanilla) = mob_and_vanilla(97001, 10, zone(ELWYNN_FOREST));
        assert_eq!(dummy, vanilla);
    }

    const ELWYNN_FOREST: u32 = 12;
    const NORTHSHIRE_VALLEY: u32 = 9;

    fn zone(area: u32) -> ZoneFactor {
        zone_factor(Some(area))
    }

    /// A warrior-class template of `rank` with no multipliers of its own.
    fn warrior(rank: u32) -> Template {
        let mut t = Template::hunter(
            0,
            String::new(),
            String::new(),
            0,
            0,
            UnitClass::Warrior,
            2000,
            7.0,
            Vec::new(),
            Vec::new(),
        );
        t.rank = rank;
        t
    }

    #[test]
    fn a_level_12_warrior_in_elwynn_forest_has_half_again_northshires_health() {
        let t = warrior(0);
        let northshire = Mob::stats_for(&t, 12, zone(NORTHSHIRE_VALLEY));
        let elwynn = Mob::stats_for(&t, 12, zone(ELWYNN_FOREST));
        let vanilla = MobStats::new(UnitClass::Warrior, 12, 1.0, 1.0, 1.0);
        let want = (vanilla.max_health as f32 * 2.0 * 1.05f32.powi(11)).round() as u32;
        assert_eq!(northshire.max_health, want);
        let half_again = (northshire.max_health as f32 * 1.5).round() as u32;
        assert!(elwynn.max_health.abs_diff(half_again) <= 1);
        // About 1,270 health and a 59 swing on average.
        assert_eq!(elwynn.max_health, 1267);
        let swing = (elwynn.min_damage + elwynn.max_damage) / 2.0;
        assert!((swing - 59.05).abs() < 0.05, "{swing}");
    }

    #[test]
    fn rank_multiplies_health_and_damage_over_a_normal_mob() {
        let normal = Mob::stats_for(&warrior(0), 12, ZoneFactor::NONE);
        for (rank, health, damage) in [(1, 2.0, 1.5), (2, 2.5, 1.75), (3, 3.0, 2.0), (4, 1.5, 1.25)]
        {
            let ranked = Mob::stats_for(&warrior(rank), 12, ZoneFactor::NONE);
            let want = (normal.max_health as f32 * health).round() as u32;
            assert!(ranked.max_health.abs_diff(want) <= 1, "rank {rank}");
            assert!((ranked.max_damage - normal.max_damage * damage).abs() < 1e-3);
        }
    }

    #[test]
    fn every_tuned_row_plays_as_it_was_tuned() {
        let mut checked = 0;
        for t in &content().template {
            let Some(tuned) = t.tuned else {
                continue;
            };
            let got = Mob::stats_for(t, t.level_min, ZoneFactor::NONE);
            let want = MobStats::new(
                t.unit_class,
                t.level_min,
                tuned.health,
                tuned.damage,
                t.armor_multiplier,
            );
            assert!(
                got.max_health.abs_diff(want.max_health) <= 1
                    && (got.max_damage / want.max_damage - 1.0).abs() < 1e-3,
                "{} ({}) has {} health and {:.2} max damage at level {}, tuned to {} and {:.2}: \
                 divide its multipliers by the creature difficulty again",
                t.name,
                t.entry,
                got.max_health,
                got.max_damage,
                t.level_min,
                want.max_health,
                want.max_damage,
            );
            checked += 1;
        }
        // Geomancer Vurx and the Wailing Caverns' four.
        assert!(checked >= 5, "{checked} tuned rows");
    }

    #[test]
    fn a_subzone_with_no_row_takes_its_parent_zones_factor() {
        // Goldshire, a subzone of Elwynn Forest.
        assert_eq!(zone(87), zone(ELWYNN_FOREST));
        assert_eq!(zone(ELWYNN_FOREST).health, 1.5);
        // Northshire Abbey's DBC parent is Elwynn Forest; its own row keeps it at Valley strength.
        assert_eq!(zone(24), ZoneFactor::NONE);
        // The Wailing Caverns: no row, no parent.
        assert_eq!(zone(718), ZoneFactor::NONE);
        assert_eq!(zone_factor(None), ZoneFactor::NONE);
        // Forest's Edge's creatures stand in its file's zone.
        assert_eq!(phased::zone(Phase::Dungeon(2)), Some(ELWYNN_FOREST));
    }
}

/// `UNIT_VIRTUAL_ITEM_SLOT_DISPLAY` and `UNIT_VIRTUAL_ITEM_INFO`: what a creature shows in its
/// hands, three slots.
const UNIT_VIRTUAL_ITEM_SLOT_DISPLAY: u16 = 37;
const UNIT_VIRTUAL_ITEM_INFO: u16 = 40;

/// The fields that show `equip`'s items in a creature's hands: each slot's display, and its
/// class, subclass, material, inventory type and sheath.
fn equip_pairs(equip: &[u32]) -> Vec<(u16, u32)> {
    let mut pairs = Vec::new();
    for (slot, &entry) in equip.iter().take(3).enumerate() {
        let Some(item) = crate::items::template(entry) else {
            continue;
        };
        let slot = slot as u16;
        pairs.push((UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + slot, item.display_id));
        pairs.push((
            UNIT_VIRTUAL_ITEM_INFO + 2 * slot,
            item.class
                | item.subclass << 8
                | (item.material.max(0) as u32 & 0xFF) << 16
                | item.inventory_type << 24,
        ));
        pairs.push((UNIT_VIRTUAL_ITEM_INFO + 2 * slot + 1, item.sheath));
    }
    pairs
}
