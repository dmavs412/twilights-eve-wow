//! Phased Dungeons (ADR 0006): dungeons on the open world map, each a set of footprint boxes in
//! its own content file. A character standing in a box is in that dungeon's Phase and sees,
//! targets and fights only that Phase's creatures; everyone shares one Run per dungeon, which
//! starts when the first character enters and resets the dungeon when the last one leaves.

use std::sync::OnceLock;

use benilla_protocol::messages::ObjectType;
use benilla_protocol::{EntityKind, ObjectFields, SessionEvent};
use serde::Deserialize;

use crate::fields::{
    float, guid_pairs, OBJECT_FIELD_ENTRY, OBJECT_FIELD_SCALE_X, OBJECT_FIELD_TYPE,
};
use crate::ground::grounded_at;
use crate::space::flat_distance;
use crate::Host;

/// `HIGHGUID_GAMEOBJECT`, in a GameObject guid's top 16 bits.
pub(crate) const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
/// `TYPEMASK_OBJECT | TYPEMASK_GAMEOBJECT`.
const TYPE_GAMEOBJECT: u32 = 0x01 | 0x20;
/// The GameObject fields a portal sets (`UpdateFields.h`, 1.12.1).
const GAMEOBJECT_DISPLAYID: u16 = 8;
const GAMEOBJECT_FLAGS: u16 = 9;
const GAMEOBJECT_ROTATION: u16 = 10;
const GAMEOBJECT_STATE: u16 = 14;
const GAMEOBJECT_POS_X: u16 = 15;
const GAMEOBJECT_TYPE_ID: u16 = 21;
/// `GameObjectDisplayInfo` 672: `InstancePortal.mdx`, the swirl in every dungeon's doorway.
const PORTAL_DISPLAY: u32 = 672;
/// `GAMEOBJECT_TYPE_GENERIC`, flagged `GO_FLAG_NOT_SELECTABLE`, in `GO_STATE_READY`.
pub(crate) const GENERIC: u32 = 5;
const NOT_SELECTABLE: u32 = 0x10;
/// `GAMEOBJECT_TYPE_CHEST`.
pub(crate) const CHEST: u32 = 3;
const READY: u32 = 1;
/// `GAMEOBJECT_TYPE_DOOR`, and `GO_STATE_ACTIVE`, an open door.
const GAMEOBJECT_TYPE_DOOR: u32 = 0;
const ACTIVE: u32 = 0;

/// Which creatures a character or creature shares the world with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(crate) enum Phase {
    #[default]
    OpenWorld,
    /// Inside the Phased Dungeon of this id.
    Dungeon(u32),
}

/// One box of a footprint: x, y and z ranges in the building's own frame.
#[derive(Deserialize)]
pub(crate) struct Footprint {
    x: [f32; 2],
    y: [f32; 2],
    z: [f32; 2],
}

impl Footprint {
    fn holds(&self, [x, y, z]: [f32; 3]) -> bool {
        (self.x[0]..=self.x[1]).contains(&x)
            && (self.y[0]..=self.y[1]).contains(&y)
            && (self.z[0]..=self.z[1]).contains(&z)
    }
}

/// A stop: a piece of a dungeon's Phase far from its building, reached through an inner portal.
/// Its box is in world coordinates on the dungeon's map, not turned with the building.
#[derive(Deserialize)]
pub(crate) struct Stop {
    pub(crate) key: String,
    x: [f32; 2],
    y: [f32; 2],
    z: [f32; 2],
    /// Where a character lands: `[x, y, z, facing]`.
    pub(crate) arrival: [f32; 4],
    /// Where the stop's way back stands, a few yards from the arrival.
    pub(crate) return_portal: [f32; 4],
}

impl Stop {
    pub(crate) fn holds(&self, [x, y, z]: [f32; 3]) -> bool {
        (self.x[0]..=self.x[1]).contains(&x)
            && (self.y[0]..=self.y[1]).contains(&y)
            && (self.z[0]..=self.z[1]).contains(&z)
    }
}

/// The portal inside a dungeon's building that leads to a stop. Its position is in the same
/// frame as the dungeon's other positions (the map's).
#[derive(Deserialize)]
pub(crate) struct InnerPortal {
    pub(crate) stop: String,
    pub(crate) position: [f32; 3],
    pub(crate) orientation: f32,
    /// Where a character comes back to, in front of the portal: `[x, y, z, facing]`.
    pub(crate) back: [f32; 4],
    /// Open when a Run starts.
    #[serde(default)]
    pub(crate) open: bool,
}

/// Which side of a stop's portal pair a GameObject is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StopPortal {
    /// In the building: leads to the stop while it is open.
    Inner,
    /// At the stop: leads back to the building, always open.
    Return,
}

/// A stop portal as the client is shown it.
pub(crate) struct StopObject<'a> {
    pub(crate) object: GameObject,
    /// The stop it belongs to when it shows only while that stop's inner portal is open.
    pub(crate) gate: Option<&'a str>,
}

/// A creature of a Phased Dungeon, there from the Run's start at full health.
#[derive(Deserialize)]
pub(crate) struct Creature {
    pub(crate) guid: u32,
    pub(crate) entry: u32,
    pub(crate) level: u8,
    pub(crate) position: [f32; 3],
    /// It stands on the host's ground (`crate::ground`); `false` keeps its written Z.
    #[serde(default = "crate::ground::yes")]
    pub(crate) grounded: bool,
    pub(crate) orientation: f32,
    /// Calls for help, each made once per life.
    #[serde(default)]
    pub(crate) on_health: Vec<OnHealth>,
    /// Calls for help made on a clock while it fights.
    #[serde(default)]
    pub(crate) every: Vec<Every>,
    #[serde(flatten)]
    pub(crate) appears: Appears,
    /// Items its corpse always holds, on top of the rolled loot.
    #[serde(default)]
    pub(crate) drop: Vec<u32>,
    /// Loot rolls on top of the ordinary one: the first to succeed adds one of its items.
    #[serde(default)]
    pub(crate) rolls: Vec<crate::rules::loot::Roll>,
    /// It starts each life asleep.
    #[serde(default)]
    pub(crate) asleep: bool,
    /// Rows sharing a camp number are a Camp: damage to one wakes the rest.
    #[serde(default)]
    pub(crate) camp: Option<u32>,
    /// Yards added to the distance at which a character wakes it (a light sleeper +, a deep
    /// sleeper -).
    #[serde(default)]
    pub(crate) wake_radius_mod: f32,
    /// How far from its point it strolls while awake and idle, in yards.
    #[serde(default)]
    pub(crate) wander: f32,
}

/// A creature's call for help: the first time its health is at or below `pct`% of its maximum
/// in a life, `count` creatures of `entry` appear at `position` and fall on its target.
#[derive(Deserialize)]
pub(crate) struct OnHealth {
    pub(crate) pct: u32,
    pub(crate) entry: u32,
    pub(crate) level: u8,
    pub(crate) count: u32,
    pub(crate) position: [f32; 3],
    /// A cast the client shows as the call goes out.
    #[serde(default)]
    pub(crate) client_spell_id: Option<u32>,
}

impl OnHealth {
    /// Whether `health` of `max` has fallen to the threshold.
    pub(crate) fn reached(&self, health: u32, max: u32) -> bool {
        u64::from(health) * 100 <= u64::from(max) * u64::from(self.pct)
    }
}

/// A creature's timed call for help: while it is in combat, every `every_s` seconds counted from
/// the pull, `count` creatures of `entry` appear at `position` and fall on its target.
#[derive(Deserialize)]
pub(crate) struct Every {
    pub(crate) every_s: f32,
    pub(crate) entry: u32,
    pub(crate) level: u8,
    pub(crate) count: u32,
    pub(crate) position: [f32; 3],
    /// A cast the client shows as the call goes out.
    #[serde(default)]
    pub(crate) client_spell_id: Option<u32>,
}

/// When a creature or GameObject of a Phased Dungeon is in the Run: from the start, unless one of
/// these says otherwise.
#[derive(Deserialize, Default, Debug)]
pub(crate) struct Appears {
    /// Out of the Run until this stop is pinned.
    #[serde(default)]
    pub(crate) after_pinned: Option<String>,
    /// Out of the Run until the creature of this guid has died in it.
    #[serde(default)]
    pub(crate) after_dead: Option<u32>,
    /// In the Run until the creature of this guid has died in it.
    #[serde(default)]
    pub(crate) until_dead: Option<u32>,
}

impl Appears {
    /// Whether it is in a Run with these stops `pinned` and creatures `dead`.
    pub(crate) fn met(&self, pinned: &[String], dead: &[u32]) -> bool {
        self.after_pinned
            .as_ref()
            .is_none_or(|key| pinned.contains(key))
            && self.after_dead.is_none_or(|guid| dead.contains(&guid))
            && self.until_dead.is_none_or(|guid| !dead.contains(&guid))
    }
}

/// The instance portal a Phased Dungeon shows in its doorway.
#[derive(Deserialize)]
pub(crate) struct Portal {
    pub(crate) position: [f32; 3],
    pub(crate) orientation: f32,
    /// A sealed door in front of the portal.
    #[serde(default)]
    pub(crate) door: Option<Door>,
}

/// A sealed door in front of a Phased Dungeon's portal. It is shut by default, opens for a
/// character with the key (or one who opened it before), and shuts again `open_secs` later.
#[derive(Deserialize)]
pub(crate) struct Door {
    pub(crate) entry: u32,
    pub(crate) display: u32,
    pub(crate) position: [f32; 3],
    pub(crate) orientation: f32,
    /// Where a refused character is put, on the open-world side.
    pub(crate) outside: [f32; 3],
    pub(crate) open_secs: f32,
    /// The item a character needs in their bags for their first opening.
    pub(crate) opens_with: u32,
    pub(crate) sealed_text: String,
}

impl Door {
    pub(crate) fn guid(&self) -> u64 {
        HIGHGUID_GAMEOBJECT << 48 | u64::from(self.entry)
    }

    /// Its create, in the closed state: a door anyone can click, standing on `host`'s ground on
    /// `map`.
    pub(crate) fn create(&self, host: &dyn Host, map: u32) -> SessionEvent {
        gameobject_create(&GameObject {
            guid: self.guid(),
            entry: self.entry,
            display_id: self.display,
            type_id: GAMEOBJECT_TYPE_DOOR,
            flags: 0,
            faction: 0,
            position: grounded_at(host, map, self.position, true),
            orientation: self.orientation,
        })
    }

    /// The update that puts the door in its open or closed state.
    pub(crate) fn state(&self, open: bool) -> SessionEvent {
        let state = if open { ACTIVE } else { READY };
        crate::fields::values(self.guid(), &[(GAMEOBJECT_STATE, state)])
    }
}

/// The open doors of the Phased Dungeons: world state, the same for every character and apart
/// from any Run. Each open door is listed with the seconds until it shuts.
#[derive(Default)]
pub(crate) struct Doors(std::collections::BTreeMap<u32, f32>);

impl Doors {
    /// Opens the door of dungeon `id` for `secs`, or restarts the time it has left.
    pub(crate) fn open(&mut self, id: u32, secs: f32) {
        self.0.insert(id, secs);
    }

    /// Time passes: doors whose time is up shut.
    pub(crate) fn tick(&mut self, dt: f32) {
        self.0.values_mut().for_each(|left| *left -= dt);
        self.0.retain(|_, left| *left > 0.0);
    }

    /// The ids of the dungeons whose door is open.
    pub(crate) fn ids(&self) -> Vec<u32> {
        self.0.keys().copied().collect()
    }
}

/// Who sees a dungeon GameObject.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Sides {
    /// From the open world and from the Phase.
    Both,
    /// Only from the Phase.
    Inside,
}

/// What clicking a dungeon GameObject asks for.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum UseKind {
    Hide,
    /// A ward stone: see [`Ward`].
    Ward,
}

/// What a ward stone does. While the creature `needs_dead` lives in the Run, a click shows
/// `held_text`. After, the first click pins stop `stop`: its inner portal goes dark, everyone in
/// its box is sent back, and the inner portal of stop `opens` opens.
#[derive(Deserialize)]
pub(crate) struct Ward {
    pub(crate) stop: String,
    pub(crate) needs_dead: u32,
    pub(crate) held_text: String,
    #[serde(default)]
    pub(crate) opens: Option<String>,
}

/// A click on a dungeon GameObject with a use, for whatever listens for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameObjectUse {
    /// The id of the Phased Dungeon it stands in.
    pub dungeon: u32,
    pub guid: u64,
    pub kind: UseKind,
}

/// A GameObject of a Phased Dungeon: scenery, or something to click when it has a `use`.
#[derive(Deserialize)]
pub(crate) struct DungeonObject {
    pub(crate) guid: u32,
    pub(crate) display: u32,
    pub(crate) position: [f32; 3],
    /// It stands on the host's ground (`crate::ground`); `false` keeps its written Z.
    #[serde(default = "crate::ground::yes")]
    pub(crate) grounded: bool,
    pub(crate) orientation: f32,
    pub(crate) sides: Sides,
    #[serde(default, rename = "use")]
    pub(crate) use_kind: Option<UseKind>,
    /// What a `use = "ward"` does.
    #[serde(default)]
    pub(crate) ward: Option<Ward>,
    /// A quest's `giver_object` names this GameObject's guid as its entry: a click offers it.
    #[serde(default)]
    pub(crate) gives_quests: bool,
    #[serde(flatten)]
    pub(crate) appears: Appears,
}

impl DungeonObject {
    pub(crate) fn object_guid(&self) -> u64 {
        HIGHGUID_GAMEOBJECT << 48 | u64::from(self.guid)
    }

    /// Its create, standing on `host`'s ground on `map`: shown only, unless it has a use to
    /// click.
    pub(crate) fn create(&self, host: &dyn Host, map: u32) -> SessionEvent {
        gameobject_create(&GameObject {
            guid: self.object_guid(),
            entry: self.guid,
            display_id: self.display,
            type_id: GENERIC,
            flags: if self.use_kind.is_some() || self.gives_quests {
                0
            } else {
                NOT_SELECTABLE
            },
            faction: 0,
            position: grounded_at(host, map, self.position, self.grounded),
            orientation: self.orientation,
        })
    }

    /// Whether a character in `phase` sees it.
    pub(crate) fn seen_from(&self, phase: Phase, dungeon: u32) -> bool {
        self.sides == Sides::Both || phase == Phase::Dungeon(dungeon)
    }
}

impl Dungeon {
    /// The dungeon GameObject of this guid that has a use, if any.
    pub(crate) fn usable(&self, guid: u64) -> Option<(&DungeonObject, UseKind)> {
        self.gameobject
            .iter()
            .find(|o| o.object_guid() == guid)
            .and_then(|o| o.use_kind.map(|k| (o, k)))
    }

    /// The dungeon whose door has this guid, in the content.
    pub(crate) fn with_door(guid: u64) -> Option<(&'static Dungeon, &'static Door)> {
        content().iter().find_map(|d| {
            d.portal
                .door
                .as_ref()
                .filter(|o| o.guid() == guid)
                .map(|o| (d, o))
        })
    }

    /// The portal's guid: one per dungeon.
    pub(crate) fn portal_guid(&self) -> u64 {
        HIGHGUID_GAMEOBJECT << 48 | u64::from(self.id)
    }

    /// The portal's create: a GameObject that only shows, never clicked, standing on `host`'s
    /// ground.
    pub(crate) fn portal_create(&self, host: &dyn Host) -> SessionEvent {
        gameobject_create(&GameObject {
            guid: self.portal_guid(),
            entry: self.id,
            display_id: PORTAL_DISPLAY,
            type_id: GENERIC,
            flags: NOT_SELECTABLE,
            faction: 0,
            position: grounded_at(host, self.map, self.portal.position, true),
            orientation: self.portal.orientation,
        })
    }
}

/// A GameObject as its create shows it, standing in `GO_STATE_READY`.
pub(crate) struct GameObject {
    pub(crate) guid: u64,
    pub(crate) entry: u32,
    pub(crate) display_id: u32,
    /// `GameobjectTypes`.
    pub(crate) type_id: u32,
    pub(crate) flags: u32,
    pub(crate) faction: u32,
    pub(crate) position: [f32; 3],
    pub(crate) orientation: f32,
}

/// `GAMEOBJECT_FACTION`.
const GAMEOBJECT_FACTION: u16 = 20;

/// `object`'s create.
pub(crate) fn gameobject_create(object: &GameObject) -> SessionEvent {
    let [x, y, z] = object.position;
    let orientation = object.orientation;
    let (sin, cos) = (orientation / 2.0).sin_cos();
    let mut pairs = vec![
        (OBJECT_FIELD_TYPE, TYPE_GAMEOBJECT),
        (OBJECT_FIELD_ENTRY, object.entry),
        (OBJECT_FIELD_SCALE_X, float(1.0)),
        (GAMEOBJECT_DISPLAYID, object.display_id),
        (GAMEOBJECT_FLAGS, object.flags),
        (GAMEOBJECT_ROTATION + 2, float(sin)),
        (GAMEOBJECT_ROTATION + 3, float(cos)),
        (GAMEOBJECT_STATE, READY),
        (GAMEOBJECT_POS_X, float(x)),
        (GAMEOBJECT_POS_X + 1, float(y)),
        (GAMEOBJECT_POS_X + 2, float(z)),
        (GAMEOBJECT_POS_X + 3, float(orientation)),
        (GAMEOBJECT_FACTION, object.faction),
        (GAMEOBJECT_TYPE_ID, object.type_id),
    ];
    pairs.extend(guid_pairs(crate::fields::OBJECT_FIELD_GUID, object.guid));
    SessionEvent::ObjectCreate {
        guid: object.guid,
        kind: EntityKind::GameObject,
        display_id: Some(object.display_id),
        position: object.position,
        orientation,
        scale: 1.0,
        speeds: None,
        mover: None,
        transport_progress: None,
        transport: None,
        spline: None,
        fields: ObjectFields::from_pairs(&pairs).into_created(ObjectType::GameObject),
    }
}

/// A Phased Dungeon's content file.
#[derive(Deserialize)]
pub(crate) struct Dungeon {
    pub(crate) id: u32,
    /// What the dungeon is called in the text players read.
    pub(crate) name: String,
    pub(crate) map: u32,
    /// The `AreaTable.dbc` area its creatures' Zone Factor comes from; none takes 1.0.
    #[serde(default)]
    pub(crate) zone: Option<u32>,
    /// Where the building stands: the footprint's frame starts here...
    origin: [f32; 3],
    /// ...turned this many degrees about +Z.
    yaw: f32,
    footprint: Vec<Footprint>,
    pub(crate) portal: Portal,
    /// Where a probe drops a character into the dungeon: `[x, y, facing]`.
    #[serde(default)]
    start: Option<[f32; 3]>,
    #[serde(default)]
    pub(crate) creature: Vec<Creature>,
    #[serde(default)]
    pub(crate) gameobject: Vec<DungeonObject>,
    /// The Run-wide stacking debuff its fights build and its hiding spots drain.
    #[serde(default)]
    pub(crate) meter: Option<Meter>,
    /// Pillars a gaze cannot see past.
    #[serde(default)]
    pub(crate) cover: Vec<Cover>,
    /// Pieces of the Phase far from the building.
    #[serde(default)]
    pub(crate) stop: Vec<Stop>,
    /// The portals in the building that lead to the stops.
    #[serde(default)]
    pub(crate) inner_portal: Vec<InnerPortal>,
    /// A wall along the footprint's edge, open at the Portal.
    #[serde(default)]
    wall: Option<Wall>,
}

/// An invisible wall that keeps walkers from crossing the footprint's edge in either direction,
/// except within `gap_yd` of the dungeon's Portal.
#[derive(Deserialize)]
struct Wall {
    gap_yd: f32,
}

/// A cover pillar: a vertical cylinder standing on `position`, in the dungeon file's frame like
/// its creatures'.
#[derive(Deserialize)]
pub(crate) struct Cover {
    pub(crate) position: [f32; 3],
    pub(crate) radius: f32,
    pub(crate) height: f32,
}

impl Cover {
    /// Whether the straight line from `from` to `to` passes through the cylinder.
    pub(crate) fn blocks(&self, from: [f32; 3], to: [f32; 3]) -> bool {
        let [cx, cy, base] = self.position;
        let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        let (fx, fy) = (from[0] - cx, from[1] - cy);
        // The parameters t of the segment (0 at `from`, 1 at `to`) inside the cylinder's disc.
        let a = d[0] * d[0] + d[1] * d[1];
        let c = fx * fx + fy * fy - self.radius * self.radius;
        let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
        if a < 1e-6 {
            if c > 0.0 {
                return false;
            }
        } else {
            let b = 2.0 * (fx * d[0] + fy * d[1]);
            let disc = b * b - 4.0 * a * c;
            if disc < 0.0 {
                return false;
            }
            let root = disc.sqrt();
            lo = lo.max((-b - root) / (2.0 * a));
            hi = hi.min((-b + root) / (2.0 * a));
        }
        // And between its base and its top.
        let top = base + self.height;
        if d[2].abs() < 1e-6 {
            if from[2] < base || from[2] > top {
                return false;
            }
        } else {
            let (t0, t1) = ((base - from[2]) / d[2], (top - from[2]) / d[2]);
            lo = lo.max(t0.min(t1));
            hi = hi.min(t0.max(t1));
        }
        lo <= hi
    }
}

/// Whether any of `cover` blocks the line from `from` to `to`.
pub(crate) fn covered(cover: &[Cover], from: [f32; 3], to: [f32; 3]) -> bool {
    cover.iter().any(|c| c.blocks(from, to))
}

/// A Phased Dungeon's meter (spec #169): stacks on the Run, shown as a debuff on every character
/// in the Phase, fed by combat and the Run's clock and drained by hiding.
#[derive(Deserialize)]
pub(crate) struct Meter {
    #[allow(dead_code)]
    pub(crate) name: String,
    /// The most stacks it holds.
    pub(crate) max: u32,
    /// The Spell.dbc row whose icon and name the debuff borrows.
    pub(crate) client_spell: u32,
    /// A stack comes with every this many seconds of the Run.
    pub(crate) clock_stack_every_s: f32,
    /// Stacks the first damage a player does to a creature in its life adds.
    pub(crate) on_first_damage: u32,
    /// Stacks a creature's death adds, unless its template says its own.
    pub(crate) on_death: u32,
    /// A hiding character takes a stack off every this many seconds.
    pub(crate) hide_drain_every_s: f32,
    /// A sleeper wakes when a character comes within this many yards, plus
    /// `wake_per_stack_yd` for every stack, plus its own `wake_radius_mod`.
    #[serde(default)]
    pub(crate) wake_base_yd: f32,
    #[serde(default)]
    pub(crate) wake_per_stack_yd: f32,
}

impl Meter {
    /// How near a character wakes a sleeper with this `wake_radius_mod` at `stacks`.
    pub(crate) fn wake_radius(&self, stacks: u32, wake_radius_mod: f32) -> f32 {
        (self.wake_base_yd + self.wake_per_stack_yd * stacks as f32 + wake_radius_mod).max(0.0)
    }
}

impl Dungeon {
    /// Whether `position` on `map` is inside the footprint.
    pub(crate) fn holds(&self, map: u32, position: [f32; 3]) -> bool {
        if map != self.map {
            return false;
        }
        let local = self.local(position);
        self.footprint.iter().any(|b| b.holds(local)) || self.stop_at(map, position).is_some()
    }

    /// Whether a step from `from` to `to` on `map` crosses the footprint's walled edge, outside
    /// the gap the Portal leaves.
    pub(crate) fn wall_blocks(&self, map: u32, from: [f32; 3], to: [f32; 3]) -> bool {
        let Some(wall) = &self.wall else {
            return false;
        };
        let in_box = |p: [f32; 3]| self.footprint.iter().any(|b| b.holds(self.local(p)));
        map == self.map
            && in_box(from) != in_box(to)
            && [from, to]
                .iter()
                .all(|&p| flat_distance(self.portal.position, p) > wall.gap_yd)
    }

    /// The index of the stop whose box holds `position` on `map`.
    pub(crate) fn stop_at(&self, map: u32, position: [f32; 3]) -> Option<usize> {
        if map != self.map {
            return None;
        }
        self.stop.iter().position(|s| s.holds(position))
    }

    /// The inner portal that leads to the stop `key`.
    pub(crate) fn inner_portal_of(&self, key: &str) -> Option<&InnerPortal> {
        self.inner_portal.iter().find(|p| p.stop == key)
    }

    /// The Run's inner portals that are open as it starts: the keys of their stops.
    pub(crate) fn portals_open_at_start(&self) -> Vec<String> {
        self.inner_portal
            .iter()
            .filter(|p| p.open)
            .map(|p| p.stop.clone())
            .collect()
    }

    /// The guid of stop `stop`'s inner portal (`Inner`) or return portal (`Return`). The entry
    /// is above every authored GameObject's.
    fn stop_portal_guid(&self, stop: usize, kind: StopPortal) -> u64 {
        let side = u32::from(kind == StopPortal::Return);
        let entry = 0x0F00_0000 | self.id << 8 | (stop as u32) << 1 | side;
        HIGHGUID_GAMEOBJECT << 48 | u64::from(entry)
    }

    /// The dungeon with a stop portal of this guid, which stop, and which side of the pair.
    pub(crate) fn with_stop_portal(guid: u64) -> Option<(&'static Dungeon, usize, StopPortal)> {
        content().iter().find_map(|d| {
            (0..d.stop.len()).find_map(|i| {
                [StopPortal::Inner, StopPortal::Return]
                    .into_iter()
                    .find(|&k| d.stop_portal_guid(i, k) == guid)
                    .map(|k| (d, i, k))
            })
        })
    }

    /// The GameObjects the stops put in the Phase: each stop's return portal, and the inner
    /// portal that leads to it, shown only while open. An open portal is the instance swirl; a
    /// shut one is not shown, as no stock unlit ring display is known.
    pub(crate) fn stop_objects(&self) -> Vec<StopObject<'_>> {
        let mut out = Vec::new();
        for (i, stop) in self.stop.iter().enumerate() {
            let [x, y, z, facing] = stop.return_portal;
            out.push(StopObject {
                object: self.stop_portal_object(i, StopPortal::Return, [x, y, z], facing),
                gate: None,
            });
            if let Some(p) = self.inner_portal_of(&stop.key) {
                out.push(StopObject {
                    object: self.stop_portal_object(
                        i,
                        StopPortal::Inner,
                        p.position,
                        p.orientation,
                    ),
                    gate: Some(&stop.key),
                });
            }
        }
        out
    }

    fn stop_portal_object(
        &self,
        stop: usize,
        kind: StopPortal,
        position: [f32; 3],
        orientation: f32,
    ) -> GameObject {
        let guid = self.stop_portal_guid(stop, kind);
        GameObject {
            guid,
            entry: guid as u32,
            display_id: PORTAL_DISPLAY,
            type_id: GENERIC,
            flags: 0,
            faction: 0,
            position,
            orientation,
        }
    }

    /// `position` in the building's frame.
    fn local(&self, [x, y, z]: [f32; 3]) -> [f32; 3] {
        let (sin, cos) = self.yaw.to_radians().sin_cos();
        let [ox, oy, oz] = self.origin;
        let (dx, dy) = (x - ox, y - oy);
        [dx * cos + dy * sin, dy * cos - dx * sin, z - oz]
    }
}

/// Whether a Phased Dungeon's door or ward stone has this GameObject entry: a use objective may
/// name either.
pub(crate) fn is_door(entry: u32) -> bool {
    content().iter().any(|d| {
        d.portal.door.as_ref().is_some_and(|o| o.entry == entry)
            || d.gameobject
                .iter()
                .any(|o| o.guid == entry && (o.use_kind == Some(UseKind::Ward) || o.gives_quests))
    })
}

/// The Phased Dungeon of this id.
pub(crate) fn dungeon(id: u32) -> Option<&'static Dungeon> {
    content().iter().find(|d| d.id == id)
}

/// The area `phase`'s creatures take their Zone Factor from: its dungeon file's `zone`.
pub(crate) fn zone(phase: Phase) -> Option<u32> {
    match phase {
        Phase::OpenWorld => None,
        Phase::Dungeon(id) => dungeon(id)?.zone,
    }
}

/// Every Phased Dungeon: one content file each.
pub(crate) fn content() -> &'static [Dungeon] {
    static DUNGEONS: OnceLock<Vec<Dungeon>> = OnceLock::new();
    DUNGEONS.get_or_init(|| {
        [
            include_str!("../content/northshire_abbey.toml"),
            include_str!("../content/test_grounds.toml"),
            include_str!("../content/forest_edge.toml"),
            include_str!("../content/sealed_yard.toml"),
            include_str!("../content/gaze_hall.toml"),
            include_str!("../content/jasperlode.toml"),
            include_str!("../content/ward_test.toml"),
            include_str!("../content/azora.toml"),
        ]
        .into_iter()
        .map(|file| toml::from_str(file).expect("a Phased Dungeon's content file parses"))
        .inspect(|d: &Dungeon| {
            for c in &d.creature {
                for roll in &c.rolls {
                    roll.check(&format!("creature {}", c.guid));
                }
            }
        })
        .collect()
    })
}

/// The id of the Test Grounds, the Phased Dungeon of training dummies.
pub(crate) const TEST_GROUNDS: u32 = 3;

/// Where the Test Grounds probe puts a character: `[x, y, facing]`, inside the Phase and
/// facing the dummies.
pub fn test_grounds_start() -> [f32; 3] {
    content()
        .iter()
        .find(|d| d.id == TEST_GROUNDS)
        .and_then(|d| d.start)
        .expect("the Test Grounds has a start")
}

/// The items a Phased Dungeon creature of `guid` on `map` always drops; none for any other.
pub(crate) fn drops(map: u32, guid: u32) -> &'static [u32] {
    content()
        .iter()
        .filter(|d| d.map == map)
        .flat_map(|d| &d.creature)
        .find(|c| c.guid == guid)
        .map_or(&[], |c| &c.drop)
}

/// The loot rolls a Phased Dungeon creature of `guid` on `map` makes; none for any other.
pub(crate) fn rolls(map: u32, guid: u32) -> &'static [crate::rules::loot::Roll] {
    content()
        .iter()
        .filter(|d| d.map == map)
        .flat_map(|d| &d.creature)
        .find(|c| c.guid == guid)
        .map_or(&[], |c| &c.rolls)
}

/// The Phase of a character standing at `position` on `map`.
pub(crate) fn at(map: u32, position: [f32; 3]) -> Phase {
    content()
        .iter()
        .find(|d| d.holds(map, position))
        .map_or(Phase::OpenWorld, |d| Phase::Dungeon(d.id))
}

/// The dungeon and stop whose box holds `position` on `map`.
pub(crate) fn stop_at(map: u32, position: [f32; 3]) -> Option<(u32, usize)> {
    content()
        .iter()
        .find_map(|d| d.stop_at(map, position).map(|i| (d.id, i)))
}

/// Where a creature of `phase` chasing toward `to` may run: on the dungeon's own floor, not
/// the terrain the ground lookup finds under it, and nowhere past its walls.
pub(crate) fn chase_within(phase: Phase, map: u32, to: [f32; 3], me: [f32; 3]) -> Option<[f32; 3]> {
    match phase {
        Phase::OpenWorld => Some(to),
        Phase::Dungeon(_) => {
            let to = [to[0], to[1], me[2]];
            (at(map, to) == phase).then_some(to)
        }
    }
}

/// A dungeon's Run: from the first character in to the last one out.
pub(crate) struct Run {
    /// Seconds since it started, for the timed events to come.
    pub(crate) elapsed: f32,
    /// The stacks on its dungeon's meter; they never fall but by hiding.
    pub(crate) noise: u32,
    /// How many of the meter's clock stacks the Run has had.
    clock_stacks: u32,
    /// The stops whose inner portal is open.
    portals_open: Vec<String>,
    /// The stops a ward stone has pinned.
    pub(crate) pinned: Vec<String>,
    /// The guids of the dungeon's creatures that have died in it.
    pub(crate) dead: Vec<u32>,
    /// The guids of the creatures that came into the Run after its start.
    pub(crate) appeared: Vec<u32>,
    /// Whether the creatures it started with stand on the host's ground: the shard makes them
    /// before it has a host, and grounds them on its first tick, before any client is shown them.
    pub(crate) grounded: bool,
}

impl Run {
    /// A Run of dungeon `dungeon`, its inner portals as the content opens them.
    pub(crate) fn new(dungeon: u32) -> Self {
        Self {
            elapsed: 0.0,
            noise: 0,
            clock_stacks: 0,
            portals_open: self::dungeon(dungeon).map_or(Vec::new(), |d| d.portals_open_at_start()),
            pinned: Vec::new(),
            dead: Vec::new(),
            appeared: Vec::new(),
            grounded: false,
        }
    }

    /// The stops whose inner portal is open.
    pub(crate) fn portals_open(&self) -> &[String] {
        &self.portals_open
    }

    /// Pins stop `key` for the rest of the Run. Whether it was not pinned already.
    pub(crate) fn pin(&mut self, key: &str) -> bool {
        let new = !self.pinned.iter().any(|k| k == key);
        if new {
            self.pinned.push(key.to_string());
        }
        new
    }

    /// Opens or closes the inner portal of stop `key`.
    pub(crate) fn set_portal(&mut self, key: &str, open: bool) {
        self.portals_open.retain(|k| k != key);
        if open {
            self.portals_open.push(key.to_string());
        }
    }

    /// Adds `gain` stacks and the clock's due, then takes `drain` off, within `0..=max`.
    pub(crate) fn hear(&mut self, meter: &Meter, gain: u32, drain: u32) {
        let mut gain = gain;
        while meter.clock_stack_every_s > 0.0
            && (self.clock_stacks + 1) as f32 * meter.clock_stack_every_s <= self.elapsed
        {
            self.clock_stacks += 1;
            gain += 1;
        }
        self.noise = (self.noise + gain).min(meter.max);
        self.noise = self.noise.saturating_sub(drain);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pillar_blocks_a_line_through_it_and_no_other() {
        let pillar = Cover {
            position: [5.0, 0.0, 0.0],
            radius: 2.5,
            height: 8.0,
        };
        assert!(
            pillar.blocks([0.0; 3], [12.0, 0.0, 0.0]),
            "straight through"
        );
        assert!(
            pillar.blocks([0.0; 3], [12.0, 1.0, 0.0]),
            "through its edge"
        );
        assert!(!pillar.blocks([0.0; 3], [12.0, 12.0, 0.0]), "wide of it");
        assert!(
            !pillar.blocks([0.0; 3], [2.0, 0.0, 0.0]),
            "stops short of it"
        );
        assert!(!pillar.blocks([0.0; 3], [0.0, 9.0, 0.0]), "the other way");
        assert!(
            !pillar.blocks([0.0, 0.0, 9.0], [12.0, 0.0, 9.0]),
            "over its top"
        );
        assert!(
            pillar.blocks([5.0, 0.0, 1.0], [5.0, 0.0, 3.0]),
            "a line inside it"
        );
    }

    #[test]
    fn a_call_at_half_health_is_reached_at_half_and_not_above() {
        let call = OnHealth {
            pct: 50,
            entry: 38,
            level: 9,
            count: 2,
            position: [0.0; 3],
            client_spell_id: None,
        };
        assert!(!call.reached(51, 100));
        assert!(call.reached(50, 100));
        assert!(call.reached(1, 100));
    }

    #[test]
    fn the_abbey_holds_its_brothers_lieutenants_and_abbot() {
        let abbey = &content()[0];
        let template = |entry| {
            crate::mobs::content()
                .template
                .iter()
                .find(|t| t.entry == entry)
                .unwrap()
        };
        assert!(
            abbey.creature.iter().all(|c| c.entry != 38),
            "a Thug remains"
        );
        let named = |name| {
            abbey
                .creature
                .iter()
                .filter(move |c| template(c.entry).name == name)
        };
        let brothers = named("Corrupted Brother").count();
        assert!((5..=7).contains(&brothers), "{brothers} brothers");
        let lieutenants: Vec<_> = named("Twilight Disciple").collect();
        assert_eq!(lieutenants.len(), 3);
        assert!(lieutenants.iter().all(|c| template(c.entry).rank == 1));

        let abbots: Vec<_> = named("Abbot Ashworth").collect();
        let [abbot] = abbots[..] else {
            panic!("{} abbots", abbots.len())
        };
        assert_eq!((abbot.level, template(abbot.entry).rank), (10, 1));
        assert!(template(abbot.entry)
            .spells
            .iter()
            .any(|s| s == "abbot_heal"));
        assert!(abbot.on_health.iter().any(|call| call.pct == 50));
        assert!(abbot.drop.is_empty() && !abbot.rolls.is_empty());
        for &item in abbot.rolls.iter().flat_map(|r| &r.one_of) {
            let gear = crate::items::template(item).expect("the Abbot's roll names an item");
            assert!(
                matches!(gear.quality, 3 | 4),
                "item {item} is not Rare or Epic"
            );
        }
    }

    #[test]
    fn neals_offers_the_abbots_fall_after_into_the_abbey() {
        let q = crate::quests::content::quest(94004).unwrap();
        assert!(matches!(q.giver, crate::quests::content::Giver::Npc(952)));
        assert_eq!(q.after.ids(), [94002]);
        assert_eq!(q.kill, [(96005, 1)]);
    }

    #[test]
    fn the_idol_the_abbey_and_the_abbots_fall_each_offer_their_choices() {
        let choices = |id| crate::quests::content::quest(id).unwrap().choices.clone();
        assert_eq!(choices(94001), [900414, 900415, 900416]);
        assert_eq!(choices(94002), [900400, 900401, 900402, 900403]);
        assert_eq!(choices(94004), [900520, 900521, 900522]);
    }

    #[test]
    fn jasperlodes_portal_stands_behind_a_door_the_keystone_opens() {
        let d = content().iter().find(|d| d.id == 6).expect("Jasperlode");
        let door = d.portal.door.as_ref().expect("a door");
        assert_eq!(door.opens_with, 980013, "the Jasperlode Keystone");
        assert_eq!(door.open_secs, 15.0);
        assert!(!d.footprint.is_empty() && !d.creature.is_empty());
    }

    #[test]
    fn the_eye_below_never_moves_or_swings_and_calls_two_tentacles_at_66_and_33() {
        let template = |entry| crate::mobs::template(entry).expect("a template");
        let d = content().iter().find(|d| d.id == 6).expect("Jasperlode");
        let eyes: Vec<_> = d
            .creature
            .iter()
            .filter(|c| template(c.entry).name == "The Eye Below")
            .collect();
        let [eye] = eyes[..] else {
            panic!("{} eyes", eyes.len())
        };
        let t = template(eye.entry);
        assert_eq!((t.speed_walk, t.speed_run, t.melee), (0.0, 0.0, false));
        assert_eq!(eye.level, 18);
        let calls: Vec<_> = eye.on_health.iter().map(|c| (c.pct, c.count)).collect();
        assert_eq!(calls, [(66, 2), (33, 2)]);
        assert!(eye
            .on_health
            .iter()
            .all(|c| template(c.entry).name == "Eye Tentacle"));
        assert!(!eye.drop.is_empty());
        let covers = d.cover.len();
        assert_eq!(covers, 4);
        let boulders = d.gameobject.len();
        assert_eq!(boulders, covers, "a boulder marks each pillar");
        assert!(d
            .cover
            .iter()
            .zip(&d.gameobject)
            .all(|(c, o)| c.position == o.position));
    }

    #[test]
    fn jasperlodes_mines_hold_kobolds_and_diggers_led_by_delvers() {
        let template = |entry| crate::mobs::template(entry).expect("a template");
        let d = content().iter().find(|d| d.id == 6).expect("Jasperlode");
        let named = |name: &'static str| {
            d.creature
                .iter()
                .filter(move |c| template(c.entry).name == name)
                .count()
        };
        assert_eq!(named("Chieftain Grik"), 1);
        assert!((3..=4).contains(&named("Frightened Kobold")));
        assert!(named("Deep Kobold Miner") > 0 && named("Deep Kobold Geomancer") > 0);
        assert_eq!(named("Twilight Delver") * 3, named("Twilight Digger"));
        assert!(template(d.creature[0].entry).friendly);
    }

    #[test]
    fn every_stop_has_one_inner_portal_and_lands_inside_its_own_box() {
        for d in content() {
            for (i, s) in d.stop.iter().enumerate() {
                let portals = d.inner_portal.iter().filter(|p| p.stop == s.key).count();
                assert_eq!(
                    portals, 1,
                    "{} stop {} has {portals} portals",
                    d.name, s.key
                );
                let at = |p: [f32; 4]| [p[0], p[1], p[2]];
                assert!(
                    s.holds(at(s.arrival)),
                    "{} arrives outside {}",
                    d.name,
                    s.key
                );
                assert!(s.holds(at(s.return_portal)), "{} return outside", d.name);
                let back = at(d.inner_portal_of(&s.key).unwrap().back);
                assert!(d.holds(d.map, back) && d.stop_at(d.map, back).is_none());
                // Round trip through the guids the client clicks.
                for k in [StopPortal::Inner, StopPortal::Return] {
                    let guid = d.stop_portal_guid(i, k);
                    let (found, stop, kind) = Dungeon::with_stop_portal(guid).unwrap();
                    assert_eq!((found.id, stop, kind), (d.id, i, k));
                }
            }
        }
    }

    #[test]
    fn marshal_mcbride_stands_outside_the_abbey() {
        let mcbride = crate::mobs::content()
            .spawn
            .iter()
            .find(|s| s.entry == 197)
            .unwrap();
        assert_eq!(at(mcbride.map, mcbride.position), Phase::OpenWorld);
    }

    #[test]
    fn forest_edge_holds_hoggers_spawn_and_its_portal_and_hides_a_palisade_and_spots() {
        let edge = content().iter().find(|d| d.id == 2).unwrap();
        assert_eq!(at(0, [-9946.0, 604.27, 38.3]), Phase::Dungeon(2));
        assert!(edge.holds(0, edge.portal.position), "portal outside");
        // A step inside the portal, and a step out onto the approach.
        let [x, y, z] = edge.portal.position;
        let (sin, cos) = edge.portal.orientation.sin_cos();
        assert!(edge.holds(0, [x - 2.0 * cos, y - 2.0 * sin, z]));
        assert!(!edge.holds(0, [x + 2.0 * cos, y + 2.0 * sin, z]));
        let hides = edge
            .gameobject
            .iter()
            .filter(|o| o.use_kind == Some(UseKind::Hide))
            .count();
        assert!(hides >= 5, "{hides} hiding spots");
        for o in edge.gameobject.iter().filter(|o| o.use_kind.is_some()) {
            assert_eq!(o.sides, Sides::Inside, "spot {} shows outside", o.guid);
            assert!(edge.holds(0, o.position), "spot {} is outside", o.guid);
        }
        let mut guids: Vec<_> = edge.gameobject.iter().map(|o| o.guid).collect();
        guids.sort_unstable();
        guids.dedup();
        assert_eq!(guids.len(), edge.gameobject.len(), "a guid repeats");
        // Only a Phase character sees the hiding spots; the palisade shows from both sides.
        let spot = edge
            .gameobject
            .iter()
            .find(|o| o.use_kind.is_some())
            .unwrap();
        let fence = edge
            .gameobject
            .iter()
            .find(|o| o.use_kind.is_none())
            .unwrap();
        assert!(!spot.seen_from(Phase::OpenWorld, 2));
        assert!(spot.seen_from(Phase::Dungeon(2), 2));
        assert!(fence.seen_from(Phase::OpenWorld, 2));
        // Only a spot with a use is clickable.
        assert!(edge.usable(spot.object_guid()).is_some());
        assert!(edge.usable(fence.object_guid()).is_none());
    }

    #[test]
    fn forest_edge_holds_the_riverpaw_camp_in_six_camps_and_two_patrols() {
        let edge = content().iter().find(|d| d.id == 2).unwrap();
        let meter = edge.meter.as_ref().unwrap();
        let template = |entry| {
            crate::mobs::content()
                .template
                .iter()
                .find(|t| t.entry == entry)
                .unwrap_or_else(|| panic!("no template {entry}"))
        };
        for c in &edge.creature {
            assert!(edge.holds(0, c.position), "creature {} is outside", c.guid);
            template(c.entry);
        }
        // Hogger (#174) sleeps alone in his tent, apart from the Camps.
        let sleepers: Vec<_> = edge
            .creature
            .iter()
            .filter(|c| c.asleep && c.entry != 96010)
            .collect();
        assert!(
            (21..=27).contains(&sleepers.len()),
            "{} sleepers",
            sleepers.len()
        );
        let mut camps: Vec<_> = sleepers.iter().filter_map(|c| c.camp).collect();
        assert!(sleepers.iter().all(|c| c.camp.is_some()), "a sleeper alone");
        camps.sort_unstable();
        camps.dedup();
        assert_eq!(camps.len(), 6, "camps {camps:?}");
        for camp in camps {
            let size = sleepers.iter().filter(|c| c.camp == Some(camp)).count();
            assert!((3..=5).contains(&size), "camp {camp} has {size}");
        }
        let awake: Vec<_> = edge.creature.iter().filter(|c| !c.asleep).collect();
        assert_eq!(awake.len(), 2);
        assert!(awake.iter().all(|c| c.wander == 8.0));
        // The four roles, in the levels the ticket sets.
        for (entry, name, levels) in [
            (96006, "Riverpaw Outrunner", 27..=27),
            (96007, "Riverpaw Mystic", 28..=28),
            (96008, "Riverpaw Brute", 28..=28),
            (96009, "Riverpaw Runt", 27..=27),
        ] {
            assert_eq!(template(entry).name, name);
            assert!(edge.creature.iter().any(|c| c.entry == entry), "no {name}");
            for c in edge.creature.iter().filter(|c| c.entry == entry) {
                assert!(levels.contains(&c.level), "{name} at {}", c.level);
            }
        }
        // The Mystic's spell resolves, and is heard.
        let spells = crate::creature_spells::resolve(&template(96007).spells);
        assert_eq!(spells.len(), 1);
        assert_eq!(spells[0].noise_on_cast, 2);
        assert_eq!(template(96009).noise_on_death, Some(2));
        // No straight line from the Portal to Hogger's tent at the back misses every wake radius at
        // 0 stacks.
        let (from, to) = (edge.portal.position, [-10050.0f32, 655.0, 38.0]);
        let radius = |c: &Creature| meter.wake_radius(0, c.wake_radius_mod);
        let blocked = |t: f32| {
            let p = [
                from[0] + (to[0] - from[0]) * t,
                from[1] + (to[1] - from[1]) * t,
            ];
            sleepers
                .iter()
                .any(|c| (c.position[0] - p[0]).hypot(c.position[1] - p[1]) <= radius(c))
        };
        let free = (0..=200).filter(|i| !blocked(*i as f32 / 200.0)).count();
        assert!(free < 201, "the straight path is clear");
        // But the Portal reaches the last hiding spot by legs from spot to spot, each clear of every
        // radius at 0 stacks.
        let mut nodes = vec![from];
        nodes.extend(
            edge.gameobject
                .iter()
                .filter(|o| o.use_kind == Some(UseKind::Hide))
                .map(|o| o.position),
        );
        let clear = |a: [f32; 3], b: [f32; 3]| {
            (0..=40).all(|i| {
                let t = i as f32 / 40.0;
                let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
                sleepers
                    .iter()
                    .all(|c| (c.position[0] - p[0]).hypot(c.position[1] - p[1]) > radius(c))
            })
        };
        let mut reached = vec![false; nodes.len()];
        reached[0] = true;
        for _ in 0..nodes.len() {
            for i in 0..nodes.len() {
                for j in 0..nodes.len() {
                    if reached[i] && !reached[j] && clear(nodes[i], nodes[j]) {
                        reached[j] = true;
                    }
                }
            }
        }
        assert!(reached.last().unwrap(), "no hiding route {reached:?}");
    }

    #[test]
    fn the_abbey_holds_its_creatures_and_its_doorway_but_not_its_lawn() {
        let abbey = &content()[0];
        for c in &abbey.creature {
            assert!(abbey.holds(0, c.position), "creature {} is outside", c.guid);
        }
        // A step inside the portal, and a step out onto the steps.
        let [x, y, z] = abbey.portal.position;
        let (sin, cos) = abbey.portal.orientation.sin_cos();
        assert!(abbey.holds(0, [x - 2.0 * cos, y - 2.0 * sin, z]));
        assert!(!abbey.holds(0, [x + 2.0 * cos, y + 2.0 * sin, z]));
        // The lawn by the west wall, where Drusilla La Salle stood, and the stables'
        // Jorik Kerridan (vmangos).
        assert!(!abbey.holds(0, [-8926.7, -195.6, 80.8]));
        assert!(!abbey.holds(0, [-8863.5, -210.9, 80.8]));
    }
}
