//! The mobs: their templates and spawns (`content/mobs.toml`), and each one's state in the world.

use std::sync::OnceLock;

use benilla_protocol::messages::{MonsterMoveFacing, ObjectType};
use benilla_protocol::{EntityKind, MoveSpeeds, ObjectFields, SessionEvent};
use serde::Deserialize;

use crate::fields::*;
use crate::rules::stats::{MobStats, UnitClass};

/// `HIGHGUID_UNIT`, in a creature guid's top 16 bits.
const HIGHGUID_UNIT: u64 = 0xF130;

/// vanilla's base movement speeds, yd/s, which a template's multipliers scale.
const BASE_WALK: f32 = 2.5;
const BASE_RUN: f32 = 7.0;

/// A creature's bounding radius and combat reach when its model says nothing else.
const BOUNDING_RADIUS: f32 = 0.5;
pub(crate) const COMBAT_REACH: f32 = 1.5;

#[derive(Deserialize)]
pub(crate) struct Template {
    pub(crate) entry: u32,
    pub(crate) name: String,
    pub(crate) creature_type: u32,
    pub(crate) display_id: u32,
    pub(crate) faction_template: u32,
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
}

#[derive(Deserialize)]
pub(crate) struct Spawn {
    pub(crate) guid: u32,
    pub(crate) entry: u32,
    pub(crate) level: u8,
    pub(crate) position: [f32; 3],
    pub(crate) orientation: f32,
}

#[derive(Deserialize)]
pub(crate) struct Mobs {
    pub(crate) template: Vec<Template>,
    pub(crate) spawn: Vec<Spawn>,
}

pub(crate) fn content() -> &'static Mobs {
    static MOBS: OnceLock<Mobs> = OnceLock::new();
    MOBS.get_or_init(|| {
        toml::from_str(include_str!("../content/mobs.toml")).expect("content/mobs.toml parses")
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
        subname: t.map(|_| String::new()),
        creature_type: t.map(|t| t.creature_type),
        pet_family: 0,
        rank: 0,
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

/// A straight walk from `from` to `to` over `duration` seconds, `elapsed` of it done.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Walk {
    pub(crate) from: [f32; 3],
    pub(crate) to: [f32; 3],
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
    next_spline_id: u32,
}

impl Mob {
    /// The mob of `spawn`, standing at `z` (the ground under its spawn point).
    pub(crate) fn spawn(spawn: &Spawn, z: f32) -> Option<Self> {
        let template = template(spawn.entry)?;
        let stats = MobStats::new(
            template.unit_class,
            spawn.level,
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
            next_spline_id: 1,
        })
    }

    pub(crate) fn level(&self) -> u8 {
        self.stats.level
    }

    pub(crate) fn alive(&self) -> bool {
        self.state != State::Dead
    }

    pub(crate) fn run_speed(&self) -> f32 {
        BASE_RUN * self.template.speed_run
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
            (UNIT_FIELD_DISPLAYID, t.display_id),
            (UNIT_FIELD_NATIVEDISPLAYID, t.display_id),
            (UNIT_FIELD_MINDAMAGE, float(self.stats.min_damage)),
            (UNIT_FIELD_MAXDAMAGE, float(self.stats.max_damage)),
            (UNIT_FIELD_RESISTANCES, self.stats.armor),
        ];
        pairs.extend(guid_pairs(OBJECT_FIELD_GUID, self.guid));
        SessionEvent::ObjectCreate {
            guid: self.guid,
            kind: EntityKind::Unit,
            display_id: Some(t.display_id),
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
        let duration = distance(self.position, to) / self.run_speed();
        self.walk = Some(Walk {
            from: self.position,
            to,
            duration,
            elapsed: 0.0,
            aimed_at,
        });
        let spline_id = self.next_spline_id;
        self.next_spline_id += 1;
        SessionEvent::MonsterMove {
            guid: self.guid,
            transport: None,
            start: self.position,
            spline_id,
            path: vec![self.position, to],
            facing: MonsterMoveFacing::None,
            stop: false,
            duration_ms: (duration * 1000.0) as u32,
            flying: false,
            run_mode: true,
        }
    }

    /// Walks on along the current leg by `dt` seconds.
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
        let p = std::array::from_fn(|i| w.from[i] + (w.to[i] - w.from[i]) * t);
        let (dx, dy) = (w.to[0] - w.from[0], w.to[1] - w.from[1]);
        if dx != 0.0 || dy != 0.0 {
            self.orientation = dy.atan2(dx).rem_euclid(std::f32::consts::TAU);
        }
        self.position = p;
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

pub(crate) fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}
