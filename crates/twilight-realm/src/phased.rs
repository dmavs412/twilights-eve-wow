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

/// A creature of a Phased Dungeon, there from the Run's start at full health.
#[derive(Deserialize)]
pub(crate) struct Creature {
    pub(crate) guid: u32,
    pub(crate) entry: u32,
    pub(crate) level: u8,
    pub(crate) position: [f32; 3],
    pub(crate) orientation: f32,
    /// Calls for help, each made once per life.
    #[serde(default)]
    pub(crate) on_health: Vec<OnHealth>,
    /// Items its corpse always holds, on top of the rolled loot.
    #[serde(default)]
    pub(crate) drop: Vec<u32>,
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

    /// Its create, in the closed state: a door anyone can click.
    pub(crate) fn create(&self) -> SessionEvent {
        gameobject_create(&GameObject {
            guid: self.guid(),
            entry: self.entry,
            display_id: self.display,
            type_id: GAMEOBJECT_TYPE_DOOR,
            flags: 0,
            faction: 0,
            position: self.position,
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
    pub(crate) orientation: f32,
    pub(crate) sides: Sides,
    #[serde(default, rename = "use")]
    pub(crate) use_kind: Option<UseKind>,
}

impl DungeonObject {
    pub(crate) fn object_guid(&self) -> u64 {
        HIGHGUID_GAMEOBJECT << 48 | u64::from(self.guid)
    }

    /// Its create: shown only, unless it has a use to click.
    pub(crate) fn create(&self) -> SessionEvent {
        gameobject_create(&GameObject {
            guid: self.object_guid(),
            entry: self.guid,
            display_id: self.display,
            type_id: GENERIC,
            flags: if self.use_kind.is_some() {
                0
            } else {
                NOT_SELECTABLE
            },
            faction: 0,
            position: self.position,
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

    /// The portal's create: a GameObject that only shows, never clicked.
    pub(crate) fn portal_create(&self) -> SessionEvent {
        gameobject_create(&GameObject {
            guid: self.portal_guid(),
            entry: self.id,
            display_id: PORTAL_DISPLAY,
            type_id: GENERIC,
            flags: NOT_SELECTABLE,
            faction: 0,
            position: self.portal.position,
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
    pub(crate) map: u32,
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
        self.footprint.iter().any(|b| b.holds(local))
    }

    /// `position` in the building's frame.
    fn local(&self, [x, y, z]: [f32; 3]) -> [f32; 3] {
        let (sin, cos) = self.yaw.to_radians().sin_cos();
        let [ox, oy, oz] = self.origin;
        let (dx, dy) = (x - ox, y - oy);
        [dx * cos + dy * sin, dy * cos - dx * sin, z - oz]
    }
}

/// Whether a Phased Dungeon's door has this GameObject entry.
pub(crate) fn is_door(entry: u32) -> bool {
    content()
        .iter()
        .any(|d| d.portal.door.as_ref().is_some_and(|o| o.entry == entry))
}

/// The Phased Dungeon of this id.
pub(crate) fn dungeon(id: u32) -> Option<&'static Dungeon> {
    content().iter().find(|d| d.id == id)
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
        ]
        .into_iter()
        .map(|file| toml::from_str(file).expect("a Phased Dungeon's content file parses"))
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

/// The Phase of a character standing at `position` on `map`.
pub(crate) fn at(map: u32, position: [f32; 3]) -> Phase {
    content()
        .iter()
        .find(|d| d.holds(map, position))
        .map_or(Phase::OpenWorld, |d| Phase::Dungeon(d.id))
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
}

impl Run {
    pub(crate) fn new() -> Self {
        Self {
            elapsed: 0.0,
            noise: 0,
            clock_stacks: 0,
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
        assert!(!abbot.drop.is_empty(), "the Abbot drops no blue");
        for &item in &abbot.drop {
            let blue = crate::items::template(item).expect("the Abbot's drop is an item");
            assert_eq!(blue.quality, 3);
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
