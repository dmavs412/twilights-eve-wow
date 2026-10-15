//! The mobs: their templates and spawns (`content/mobs.toml` by hand, `content/barrens.toml`
//! generated from the research dumps by `examples/import_barrens.rs`), and each one's state in
//! the world.

use std::collections::HashSet;
use std::sync::OnceLock;

use benilla_protocol::messages::{MonsterMoveFacing, ObjectType};
use benilla_protocol::{EntityKind, MoveSpeeds, ObjectFields, SessionEvent};
use serde::Deserialize;

use crate::clock;
use crate::fields::*;
use crate::rules::melee::MobDefense;
use crate::rules::stats::{MobStats, UnitClass};
use crate::space::distance;

/// `HIGHGUID_UNIT`, in a creature guid's top 16 bits.
const HIGHGUID_UNIT: u64 = 0xF130;

/// vanilla's base movement speeds, yd/s, which a template's multipliers scale.
const BASE_WALK: f32 = 2.5;
const BASE_RUN: f32 = 7.0;

/// A creature's bounding radius and combat reach when its model says nothing else.
const BOUNDING_RADIUS: f32 = 0.5;
pub(crate) const COMBAT_REACH: f32 = 1.5;

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
    #[allow(dead_code)]
    pub(crate) wander_radius: f32,
    /// The camp it fights with: pulling one member pulls the rest.
    pub(crate) camp: u32,
    /// The map it stands on: Kalimdor, or a Dungeon's.
    pub(crate) map: u32,
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

/// A generated spawn row in `barrens.toml`: guid, entry, level_min, level_max, x, y, z,
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

#[derive(Deserialize)]
struct BarrensFile {
    suppress: Vec<u32>,
    spawns: Vec<Row>,
    template: Vec<Template>,
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
        let barrens: BarrensFile = toml::from_str(include_str!("../content/barrens.toml"))
            .expect("content/barrens.toml parses");
        let mut template = hand.template;
        template.extend(barrens.template);
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
                })
            })
            .collect();
        let suppress: HashSet<u32> = barrens.suppress.into_iter().collect();
        spawn.extend(
            barrens
                .spawns
                .into_iter()
                .filter(|r| !suppress.contains(&r.0))
                .map(
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
                        map: crate::content::KALIMDOR,
                    },
                ),
        );
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
            });
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
    display_id: u32,
}

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
            display_id: spawn.display_id,
        })
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
