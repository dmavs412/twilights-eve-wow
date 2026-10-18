//! The mobs: their templates and spawns (`content/mobs.toml` by hand, `content/barrens.toml`
//! and `content/elwynn.toml` generated from the research dumps by `examples/import_barrens.rs`
//! and `examples/import_elwynn.rs`), and each one's state in the world.

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
use crate::rules::stats::{MobStats, UnitClass};
use crate::space::distance;
use crate::Host;

/// `HIGHGUID_UNIT`, in a creature guid's top 16 bits.
const HIGHGUID_UNIT: u64 = 0xF130;

/// `UNIT_NPC_FLAGS`, and its `UNIT_NPC_FLAG_QUESTGIVER` bit.
const UNIT_NPC_FLAGS: u16 = 147;
const UNIT_NPC_FLAG_QUESTGIVER: u32 = 0x2;

/// vanilla's base movement speeds, yd/s, which a template's multipliers scale.
const BASE_WALK: f32 = 2.5;
const BASE_RUN: f32 = 7.0;

/// A creature's bounding radius and combat reach when its model says nothing else.
const BOUNDING_RADIUS: f32 = 0.5;
pub(crate) const COMBAT_REACH: f32 = 1.5;

/// The first low guid given to a creature a call for help brings in.
const CALLED_GUID_BASE: u32 = 0x90_0000;

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
    pub(crate) display_id: u32,
    pub(crate) faction_template: u32,
    /// Attacks a player who comes too close; a neutral creature only fights back.
    #[serde(default = "yes")]
    pub(crate) hostile: bool,
    /// Friendly to every player: it never fights, and no attack or spell may target it.
    #[serde(default)]
    pub(crate) friendly: bool,
    pub(crate) unit_class: UnitClass,
    #[allow(dead_code)]
    pub(crate) level_min: u8,
    #[allow(dead_code)]
    pub(crate) level_max: u8,
    pub(crate) health_multiplier: f32,
    pub(crate) damage_multiplier: f32,
    pub(crate) armor_multiplier: f32,
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
    /// The Creature Spells it casts in a fight (`content/creature_spells.toml` keys).
    #[serde(default)]
    pub(crate) spells: Vec<String>,
}

/// `CREATURE_FLAG_EXTRA_NO_PARRY` and `CREATURE_FLAG_EXTRA_NO_BLOCK` (vmangos).
const FLAG_EXTRA_NO_PARRY: u32 = 0x4;
const FLAG_EXTRA_NO_BLOCK: u32 = 0x10;

impl Template {
    /// Elite and boss ranks, which kill XP pays double for.
    pub(crate) fn elite(&self) -> bool {
        matches!(self.rank, 1..=3)
    }
}

/// Where a mob stands when it (re)spawns.
pub(crate) struct Spawn {
    /// The spawn's low guid (24 bits).
    pub(crate) guid: u32,
    pub(crate) entry: u32,
    pub(crate) level_min: u8,
    pub(crate) level_max: u8,
    /// WoW `[x, y, z]`; z the spawn's height in the data, which the host's ground replaces once
    /// it is loaded.
    pub(crate) position: [f32; 3],
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
}

/// A hand-written spawn in `mobs.toml`, a camp of its own.
#[derive(Deserialize)]
struct HandSpawn {
    guid: u32,
    entry: u32,
    level: u8,
    position: [f32; 3],
    orientation: f32,
}

/// A generated spawn row in `barrens.toml` or `elwynn.toml`: guid, entry, level_min, level_max, x, y, z,
/// orientation, display_id, wander_radius, camp.
#[derive(Deserialize)]
struct Row(u32, u32, u8, u8, f32, f32, f32, f32, u32, f32, u32);

#[derive(Deserialize)]
struct HandFile {
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
    orientation: f32,
    camp: u32,
}

/// `content/elwynn_overrides.toml`: hand-written changes to the generated Elwynn spawns.
#[derive(Deserialize)]
struct Overrides {
    /// Entries whose spawns are deleted.
    removed: Vec<u32>,
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
    orientation: f32,
}

#[derive(Deserialize)]
struct Moved {
    entry: u32,
    position: [f32; 3],
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
    pub(crate) template: Vec<Template>,
    pub(crate) spawn: Vec<Spawn>,
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
        elwynn.spawns.retain(|r| !overrides.removed.contains(&r.1));
        for m in &overrides.moved {
            for r in elwynn.spawns.iter_mut().filter(|r| r.1 == m.entry) {
                [r.4, r.5, r.6] = m.position;
                r.7 = m.orientation;
            }
        }
        let mut template = hand.template;
        template.extend(barrens.template);
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
                    orientation: s.orientation,
                    display_id,
                    wander_radius: 0.0,
                    camp: s.guid,
                    map: crate::content::KALIMDOR,
                    phase: Phase::OpenWorld,
                    on_health: &[],
                    called_by: None,
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
                    orientation: o,
                    display_id,
                    wander_radius: wander,
                    camp,
                    map,
                    phase: Phase::OpenWorld,
                    on_health: &[],
                    called_by: None,
                },
            ));
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
                orientation: a.orientation,
                display_id: t.display_id,
                wander_radius: 0.0,
                camp: a.guid,
                map: crate::content::EASTERN_KINGDOMS,
                phase: Phase::OpenWorld,
                on_health: &[],
                called_by: None,
            });
        }
        let dungeon: DungeonFile = toml::from_str(include_str!("../content/wailing_caverns.toml"))
            .expect("content/wailing_caverns.toml parses");
        template.extend(dungeon.template);
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
                orientation: s.orientation,
                display_id: t.display_id,
                wander_radius: 0.0,
                camp: s.camp,
                map: dungeon.map,
                phase: Phase::OpenWorld,
                on_health: &[],
                called_by: None,
            });
        }
        let mut called_guids = 0;
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
                    orientation: c.orientation,
                    display_id: t.display_id,
                    wander_radius: 0.0,
                    camp: c.guid,
                    map: d.map,
                    phase: Phase::Dungeon(d.id),
                    on_health: &c.on_health,
                    called_by: None,
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
                            orientation: c.orientation,
                            display_id: t.display_id,
                            wander_radius: 0.0,
                            camp: c.guid,
                            map: d.map,
                            phase: Phase::Dungeon(d.id),
                            on_health: &[],
                            called_by: Some((c.guid, k)),
                        });
                        called_guids += 1;
                    }
                }
            }
        }
        Mobs { template, spawn }
    })
}

pub(crate) fn template(entry: u32) -> Option<&'static Template> {
    content().template.iter().find(|t| t.entry == entry)
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
    /// Held out of the fight by an incapacitate that dropped combat; when the hold ends it
    /// notices the player as an idle mob does, or runs home.
    Dazed,
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
    /// Seconds until it next decides whether to cast.
    pub(crate) think_in: f32,
    /// Which of its spawn's calls for help it has made, this life.
    pub(crate) called: Vec<bool>,
}

/// The pause between one stroll and the next, in seconds.
const WANDER_PAUSE: (f32, f32) = (6.0, 16.0);

impl Mob {
    /// The mob of `content().spawn[index]` at `level`, standing at `z` (the ground under its
    /// spawn point).
    pub(crate) fn spawn(index: u32, level: u8, z: f32) -> Option<Self> {
        let spawn = content().spawn.get(index as usize)?;
        let template = template(spawn.entry)?;
        let stats = MobStats::new(
            template.unit_class,
            level,
            template.health_multiplier,
            template.damage_multiplier,
            template.armor_multiplier,
        );
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
            think_in: 0.0,
            called: vec![false; spawn.on_health.len()],
        })
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
            level: self.level(),
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
        // A Field Giver shows the quest giver flag, so the client asks for its `!` and `?`.
        if t.friendly && crate::quests::content::field_giver(t.entry) {
            pairs.push((UNIT_NPC_FLAGS, UNIT_NPC_FLAG_QUESTGIVER));
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
        // Northshire Guard, Innkeeper Farley, Marshal McBride.
        for entry in [1642, 295, 197] {
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
        assert!(outside.iter().all(|s| (s.level_min, s.level_max) == (1, 1)));
    }

    #[test]
    fn a_friendly_npc_is_never_attackable() {
        let i = content()
            .spawn
            .iter()
            .position(|s| s.entry == 1642)
            .unwrap();
        let guard = Mob::spawn(i as u32, 10, 0.0).unwrap();
        assert!(guard.alive() && !guard.attackable());
        let i = content().spawn.iter().position(|s| s.entry == 6).unwrap();
        assert!(Mob::spawn(i as u32, 1, 0.0).unwrap().attackable());
    }
}
