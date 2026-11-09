//! Phased Dungeons (ADR 0006): dungeons on the open world map, or on an instance map behind a
//! loading screen (ADR 0032, `crate::instance_map`), each a set of footprint boxes in its own
//! content file. A character standing in a box is in that dungeon's Phase and sees,
//! targets and fights only that Phase's creatures; everyone shares one Run per dungeon, which
//! starts when the first character enters and resets the dungeon when the last one leaves.

use std::sync::OnceLock;

use benilla_protocol::messages::ObjectType;
use benilla_protocol::{EntityKind, ObjectFields, SessionEvent};
use serde::Deserialize;

use crate::creature_spells::CreatureEffect;
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
/// `GameObjectDisplayInfo` 5231, the stock Portal to Inner Maraudon (`gameobject_template` 178404):
/// every stop portal's. The client puts the interact cursor only on a model whose triangles its
/// mouse ray hits, and 672 builds none (#263).
const STOP_PORTAL_DISPLAY: u32 = 5231;
/// `GAMEOBJECT_TYPE_GENERIC`, flagged `GO_FLAG_NOT_SELECTABLE`, in `GO_STATE_READY`: scenery. The
/// client never lets a player use a GENERIC, whatever its flags (`0x5f47f0`).
pub(crate) const GENERIC: u32 = 5;
pub(crate) const NOT_SELECTABLE: u32 = 0x10;
/// `GAMEOBJECT_TYPE_GOOBER`: a GameObject a player clicks. The client shows the interact cursor
/// and sends `CMSG_GAMEOBJ_USE`, and the realm decides what the click does.
pub(crate) const GOOBER: u32 = 10;
/// `GAMEOBJECT_TYPE_CHEST`.
pub(crate) const CHEST: u32 = 3;
/// `GAMEOBJECT_TYPE_FISHINGNODE`: a fishing bobber.
pub(crate) const FISHINGNODE: u32 = 17;
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
///
/// `frozen_until = { guid = 974250, pct = 75 }` keeps it frozen in the grey (#637): it starts
/// each Run frozen, wearing the marker `spell` (a Creature Spell key, `"frozen_in_the_mist"` when
/// left out), and thaws the first time the creature of `guid`, alive, has health at or below
/// `pct`% of its maximum, then turns on the closest living player. If that creature dies first,
/// it stays frozen for the rest of the Run.
///
/// `on_partner_death = { guid = 974350, spell = "enraged_partner" }` links it to its partner
/// (#637): while the creature of `guid` is dead in the Run and this one is alive and engaged, it
/// is in the `stance` Creature Spell `spell`, put up at once with no cast, cooldown or chance,
/// any other stance coming down first. The stance's own rules end it, so one that leashes and is
/// pulled again enrages again. Two rows may name each other.
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
    /// It starts each Run frozen until another creature's health mark.
    #[serde(default)]
    pub(crate) frozen_until: Option<FrozenUntil>,
    /// The stance it takes while its partner is dead.
    #[serde(default)]
    pub(crate) on_partner_death: Option<PartnerDeath>,
}

/// A creature's `on_partner_death`: while the creature of `guid` (the partner) is dead, it is in
/// the stance of the Creature Spell `spell`.
#[derive(Deserialize)]
pub(crate) struct PartnerDeath {
    pub(crate) guid: u32,
    pub(crate) spell: String,
}

/// A creature's `frozen_until`: the creature of `guid` (the boss), alive at or below `pct`% of
/// its maximum health, thaws it.
#[derive(Deserialize)]
pub(crate) struct FrozenUntil {
    pub(crate) guid: u32,
    pub(crate) pct: u32,
    /// The marker aura it wears while frozen, by Creature Spell key.
    #[serde(default = "frozen_marker")]
    pub(crate) spell: String,
}

/// The ice a frozen creature wears unless its row names another: Freezing Trap's look.
fn frozen_marker() -> String {
    "frozen_in_the_mist".into()
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
    /// The guid of the lever whose pull stops the call for the rest of the Run.
    #[serde(default)]
    pub(crate) until_used: Option<u32>,
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
    /// Out of the Run until the lever of this guid has been pulled in it.
    #[serde(default)]
    pub(crate) after_used: Option<u32>,
    /// In the Run until the lever of this guid has been pulled in it.
    #[serde(default)]
    pub(crate) until_used: Option<u32>,
    /// Out of every Run until a Milestone is reached, or in it until one is (ADR 0034).
    #[serde(flatten)]
    pub(crate) milestones: crate::milestones::Gate,
}

impl Appears {
    /// Whether it is in a Run with these stops `pinned`, creatures `dead` and levers `used`, while
    /// the Milestones `reached` are.
    pub(crate) fn met<S: AsRef<str>>(
        &self,
        pinned: &[String],
        dead: &[u32],
        used: &[u32],
        reached: &[S],
    ) -> bool {
        self.after_pinned
            .as_ref()
            .is_none_or(|key| pinned.contains(key))
            && self.after_dead.is_none_or(|guid| dead.contains(&guid))
            && self.until_dead.is_none_or(|guid| !dead.contains(&guid))
            && self.after_used.is_none_or(|guid| used.contains(&guid))
            && self.until_used.is_none_or(|guid| !used.contains(&guid))
            && self.milestones.met(reached)
    }

    /// Whether it comes into the Run after its start, once something has happened in it or a
    /// Milestone is reached.
    pub(crate) fn comes_later(&self) -> bool {
        self.after_pinned.is_some()
            || self.after_dead.is_some()
            || self.after_used.is_some()
            || self.milestones.after_milestone.is_some()
    }

    /// Whether it has left a Run with these creatures `dead` and levers `used`, while the
    /// Milestones `reached` are.
    pub(crate) fn gone<S: AsRef<str>>(&self, dead: &[u32], used: &[u32], reached: &[S]) -> bool {
        self.until_dead.is_some_and(|guid| dead.contains(&guid))
            || self.until_used.is_some_and(|guid| used.contains(&guid))
            || self.milestones.gone(reached)
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
    /// The open-world creature or Hub NPC, by low guid, who says `sealed_text` to a refused
    /// character in place of the notification while that character sees them.
    #[serde(default)]
    pub(crate) speaker: Option<u32>,
}

impl Door {
    pub(crate) fn guid(&self) -> u64 {
        HIGHGUID_GAMEOBJECT << 48 | u64::from(self.entry)
    }

    /// Its create, in the closed state: a door anyone can click, standing on `host`'s ground on
    /// an open-world `map`; on an instance map, with no terrain, at its written Z.
    pub(crate) fn create(&self, host: &dyn Host, map: u32) -> SessionEvent {
        gameobject_create(&GameObject {
            guid: self.guid(),
            entry: self.entry,
            display_id: self.display,
            type_id: GAMEOBJECT_TYPE_DOOR,
            flags: 0,
            faction: 0,
            position: grounded_at(host, map, self.position, crate::content::open_world(map)),
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
    /// A ward stone: see [`Ward`].
    Ward,
    /// A lever: see [`Lever`].
    Lever,
}

/// What a lever does. While the creature `needs_dead` lives in the Run, a click shows
/// `held_text`. After, or from the start with no `needs_dead`, the first click marks it used for
/// the Run and shows the clicker `used_text`; a later click does nothing.
#[derive(Deserialize)]
pub(crate) struct Lever {
    #[serde(default)]
    pub(crate) needs_dead: Option<u32>,
    #[serde(default)]
    pub(crate) held_text: String,
    pub(crate) used_text: String,
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
    /// What a `use = "lever"` does.
    #[serde(default)]
    pub(crate) lever: Option<Lever>,
    /// A quest's `giver_object` names this GameObject's guid as its entry: a click offers it.
    #[serde(default)]
    pub(crate) gives_quests: bool,
    /// It is Cover (#538): a character within this many yards of its position, out of any fight,
    /// is in Cover, and the client walks through it.
    #[serde(default)]
    pub(crate) cover_yd: Option<f32>,
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
            type_id: if self.clicked() { GOOBER } else { GENERIC },
            flags: if self.clicked() { 0 } else { NOT_SELECTABLE },
            faction: 0,
            position: grounded_at(host, map, self.position, self.grounded),
            orientation: self.orientation,
        })
    }

    /// Whether a click does something: a use, or quests to offer.
    fn clicked(&self) -> bool {
        self.use_kind.is_some() || self.gives_quests
    }

    /// Whether a character in `phase` sees it.
    pub(crate) fn seen_from(&self, phase: Phase, dungeon: u32) -> bool {
        self.sides == Sides::Both || phase == Phase::Dungeon(dungeon)
    }
}

impl Dungeon {
    /// The guids of its Cover GameObjects (`cover_yd`), which the client walks through.
    pub(crate) fn walk_through(&self) -> Vec<u64> {
        self.gameobject
            .iter()
            .filter(|o| o.cover_yd.is_some())
            .map(DungeonObject::object_guid)
            .collect()
    }

    /// Whether `at` is within reach of one of its Cover GameObjects. Not the `[[cover]]` pillars
    /// that stop a gaze.
    pub(crate) fn in_cover(&self, at: [f32; 3]) -> bool {
        self.gameobject.iter().any(|o| {
            o.cover_yd
                .is_some_and(|yd| flat_distance(o.position, at) <= yd)
        })
    }

    /// Refuses a lever table on a GameObject without `use = "lever"` and the use without the
    /// table, a lever's `needs_dead` that names no creature of the dungeon, an `after_used`,
    /// `until_used` or timed call's `until_used` that names no lever of it, and an
    /// `after_milestone` or `until_milestone` that names no Milestone.
    fn check_levers(&self) {
        let levers: Vec<u32> = self
            .gameobject
            .iter()
            .filter(|o| o.use_kind == Some(UseKind::Lever))
            .map(|o| o.guid)
            .collect();
        for o in &self.gameobject {
            assert_eq!(
                o.lever.is_some(),
                o.use_kind == Some(UseKind::Lever),
                "{}: GameObject {} has a lever table or use = \"lever\" without the other",
                self.name,
                o.guid
            );
            if let Some(guid) = o.lever.as_ref().and_then(|l| l.needs_dead) {
                assert!(
                    self.creature.iter().any(|c| c.guid == guid),
                    "{}: lever {} needs_dead {guid}, no creature of it",
                    self.name,
                    o.guid
                );
            }
        }
        let appears = self
            .creature
            .iter()
            .map(|c| (c.guid, &c.appears))
            .chain(self.gameobject.iter().map(|o| (o.guid, &o.appears)));
        for (guid, a) in appears {
            a.milestones.check(&format!("{}: {guid}", self.name));
            for lever in [a.after_used, a.until_used].into_iter().flatten() {
                assert!(
                    levers.contains(&lever),
                    "{}: {guid} waits on {lever}, no lever of it",
                    self.name
                );
            }
        }
        for c in &self.creature {
            for lever in c.every.iter().filter_map(|e| e.until_used) {
                assert!(
                    levers.contains(&lever),
                    "{}: creature {}'s timed call stops on {lever}, no lever of it",
                    self.name,
                    c.guid
                );
            }
        }
    }

    /// Refuses a Bonus Chest whose `boss` or `together_s` guid is no creature of the dungeon, whose
    /// `unused` lever is no lever of it, whose `together_s` leaves out the boss, or whose `one_of`
    /// is empty or names no item.
    fn check_bonus_chest(&self) {
        let Some(chest) = &self.bonus_chest else {
            return;
        };
        let creature = |guid: u32| self.creature.iter().any(|c| c.guid == guid);
        let lever = |guid: u32| {
            self.gameobject
                .iter()
                .any(|o| o.guid == guid && o.use_kind == Some(UseKind::Lever))
        };
        let row = format!("{}: the bonus chest", self.name);
        assert!(
            creature(chest.boss),
            "{row}'s boss {} is no creature of it",
            chest.boss
        );
        match &chest.challenge {
            Challenge::WithinS { .. } => {}
            Challenge::Unused { levers } => {
                for &l in levers {
                    assert!(lever(l), "{row}'s unused {l} is no lever of it");
                }
            }
            Challenge::TogetherS { guids, .. } => {
                for &g in guids {
                    assert!(creature(g), "{row}'s together_s {g} is no creature of it");
                }
                assert!(
                    guids.contains(&chest.boss),
                    "{row}'s together_s leaves out its boss {}",
                    chest.boss
                );
            }
        }
        assert!(!chest.one_of.is_empty(), "{row} holds no items");
        for &item in &chest.one_of {
            assert!(
                crate::items::template(item).is_some(),
                "{row} names item {item}, which the content does not have"
            );
        }
    }

    /// Refuses a `frozen_until` whose `guid` names no creature of the dungeon, whose `pct` is
    /// outside 1 to 99 or whose `spell` is no Creature Spell, and one on a creature that starts
    /// asleep.
    fn check_frozen(&self) {
        for c in &self.creature {
            let Some(f) = &c.frozen_until else {
                continue;
            };
            let row = format!("{}: creature {}'s frozen_until", self.name, c.guid);
            assert!(
                self.creature.iter().any(|b| b.guid == f.guid),
                "{row} waits on {}, no creature of it",
                f.guid
            );
            assert!(
                (1..=99).contains(&f.pct),
                "{row} pct {} is not 1 to 99",
                f.pct
            );
            assert!(
                crate::creature_spells::get(&f.spell).is_some(),
                "{row} wears {}, no Creature Spell",
                f.spell
            );
            assert!(!c.asleep, "{row} on a creature that starts asleep");
        }
    }

    /// Refuses an `on_partner_death` whose `guid` names no creature of the dungeon or the row
    /// itself, and one whose `spell` is no `stance` Creature Spell.
    fn check_partners(&self) {
        for c in &self.creature {
            let Some(p) = &c.on_partner_death else {
                continue;
            };
            let row = format!("{}: creature {}'s on_partner_death", self.name, c.guid);
            assert_ne!(p.guid, c.guid, "{row} names itself");
            assert!(
                self.creature.iter().any(|b| b.guid == p.guid),
                "{row} names {}, no creature of it",
                p.guid
            );
            assert!(
                crate::creature_spells::get(&p.spell)
                    .is_some_and(|s| matches!(s.effect, CreatureEffect::Stance { .. })),
                "{row} takes {}, no stance Creature Spell",
                p.spell
            );
        }
    }

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
    /// ground on an open-world map; an instance map has no terrain, so there it keeps its
    /// written Z.
    pub(crate) fn portal_create(&self, host: &dyn Host) -> SessionEvent {
        gameobject_create(&GameObject {
            guid: self.portal_guid(),
            entry: self.id,
            display_id: PORTAL_DISPLAY,
            type_id: GENERIC,
            flags: NOT_SELECTABLE,
            faction: 0,
            position: grounded_at(
                host,
                self.map,
                self.portal.position,
                crate::content::open_world(self.map),
            ),
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
    /// The level range of its hostile creatures, shown on the world map; none for a test dungeon.
    #[serde(default)]
    pub(crate) levels: Option<[u8; 2]>,
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
    /// Doors inside the building that a Run's key unlocks.
    #[serde(default)]
    pub(crate) locked_door: Vec<LockedDoor>,
    /// The key its locked doors take, and the creatures that may carry it.
    #[serde(default)]
    pub(crate) key: Option<Key>,
    /// The Deeprun Tram's line (ADR 0033): trams that come once its bell is rung.
    #[serde(default)]
    pub(crate) tram: Option<crate::tram::Tram>,
    /// The chest its boss leaves when the Run met its optional challenge (spec #650).
    #[serde(default)]
    pub(crate) bonus_chest: Option<BonusChest>,
}

/// A Phased Dungeon's Bonus Chest (spec #650): when `boss` dies and the Run met `challenge`, a
/// chest of `entry` stands `east_yd` east of its corpse, at the corpse's Z, holding `money` copper
/// and one of `one_of` drawn evenly.
#[derive(Deserialize)]
pub(crate) struct BonusChest {
    pub(crate) boss: u32,
    /// A run-time GameObject entry with no template, as the tram's chests.
    pub(crate) entry: u32,
    pub(crate) display: u32,
    pub(crate) east_yd: f32,
    pub(crate) money: u32,
    pub(crate) one_of: Vec<u32>,
    /// Shown to every character in the Phase when the chest appears.
    #[serde(default)]
    pub(crate) found_text: Option<String>,
    pub(crate) challenge: Challenge,
}

/// What a Run must do for its dungeon's Bonus Chest, judged once, when the chest would appear.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum Challenge {
    /// The boss dies at most `seconds` into the Run.
    WithinS { seconds: f32 },
    /// The boss dies with none of `levers` pulled in the Run.
    Unused { levers: Vec<u32> },
    /// The first and the last deaths of `guids` are at most `seconds` apart; judged at the last.
    TogetherS { guids: Vec<u32>, seconds: f32 },
}

impl Challenge {
    /// Whether the Run met it, with `deaths` the `(guid, elapsed)` of each watched creature that
    /// died, `used` the levers pulled and `boss_died_at` the Run's clock at the boss's death. A
    /// `together_s` with a guid still alive fails.
    pub(crate) fn holds(&self, deaths: &[(u32, f32)], used: &[u32], boss_died_at: f32) -> bool {
        match self {
            Self::WithinS { seconds } => boss_died_at <= *seconds,
            Self::Unused { levers } => !levers.iter().any(|l| used.contains(l)),
            Self::TogetherS { guids, seconds } => {
                let at: Vec<f32> = deaths
                    .iter()
                    .filter(|(g, _)| guids.contains(g))
                    .map(|&(_, t)| t)
                    .collect();
                let first = at.iter().copied().fold(f32::INFINITY, f32::min);
                let last = at.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                at.len() == guids.len() && last - first <= *seconds
            }
        }
    }
}

impl BonusChest {
    /// The creatures whose deaths it watches: the boss, and a `together_s`'s guids.
    pub(crate) fn watched(&self) -> Vec<u32> {
        let mut out = vec![self.boss];
        if let Challenge::TogetherS { guids, .. } = &self.challenge {
            out.extend(guids.iter().filter(|&&g| g != self.boss));
        }
        out
    }

    /// Whether the Run met the challenge, once the creature that judges it has died: the boss, or
    /// the last of a `together_s`'s guids. `None` before then.
    pub(crate) fn judge(&self, deaths: &[(u32, f32)], used: &[u32]) -> Option<bool> {
        let died = |guid: u32| deaths.iter().find(|(g, _)| *g == guid).map(|&(_, t)| t);
        let boss_died_at = died(self.boss)?;
        if let Challenge::TogetherS { guids, .. } = &self.challenge {
            if guids.iter().any(|&g| died(g).is_none()) {
                return None;
            }
        }
        Some(self.challenge.holds(deaths, used, boss_died_at))
    }
}

/// A door inside a Phased Dungeon, locked at the start of every Run until a character with the
/// dungeon's key clicks it.
#[derive(Deserialize)]
pub(crate) struct LockedDoor {
    pub(crate) guid: u32,
    pub(crate) display: u32,
    pub(crate) position: [f32; 3],
    pub(crate) orientation: f32,
    /// The doorway in the building's frame.
    pub(crate) doorway: Doorway,
    /// The item that unlocks it: the dungeon's `[key]` item.
    pub(crate) opens_with: u32,
    pub(crate) locked_text: String,
}

/// A doorway in the building's frame: the plane `x` between these `y` and `z`.
#[derive(Deserialize)]
pub(crate) struct Doorway {
    pub(crate) x: f32,
    pub(crate) y: [f32; 2],
    pub(crate) z: [f32; 2],
}

impl LockedDoor {
    pub(crate) fn object_guid(&self) -> u64 {
        HIGHGUID_GAMEOBJECT << 48 | u64::from(self.guid)
    }

    /// Its create, in the closed state, standing at the written Z.
    pub(crate) fn create(&self) -> SessionEvent {
        gameobject_create(&GameObject {
            guid: self.object_guid(),
            entry: self.guid,
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
        crate::fields::values(self.object_guid(), &[(GAMEOBJECT_STATE, state)])
    }
}

/// The key of a Phased Dungeon's locked doors: one of the `holders` carries it in each Run, and
/// the `hint_npc` says which.
#[derive(Deserialize)]
pub(crate) struct Key {
    pub(crate) item: u32,
    pub(crate) hint_npc: u32,
    pub(crate) holders: Vec<Holder>,
}

/// A creature that may carry the key, and what the hint says when it does.
#[derive(Deserialize)]
pub(crate) struct Holder {
    pub(crate) guid: u32,
    pub(crate) hint: String,
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

/// A Phased Dungeon's meter (spec #169): stacks on the Run, shown on the Noise Bar of every
/// character in the Phase (#533), fed by combat and the Run's clock and drained by hiding.
#[derive(Deserialize)]
pub(crate) struct Meter {
    pub(crate) name: String,
    /// The most stacks it holds.
    pub(crate) max: u32,
    /// The icon at the bar's full end.
    pub(crate) end_icon: String,
    /// The line under the bar when each source raises the stacks.
    pub(crate) lines: MeterLines,
    /// A stack comes with every this many seconds of the Run.
    pub(crate) clock_stack_every_s: f32,
    /// Stacks the first damage a player does to a creature in its life adds.
    pub(crate) on_first_damage: u32,
    /// Stacks a creature's death adds, unless its template says its own.
    pub(crate) on_death: u32,
    /// A character in Cover takes a stack off every this many seconds.
    pub(crate) hide_drain_every_s: f32,
    /// The Creature Spell id of the aura a character in Cover wears.
    #[serde(default)]
    pub(crate) cover_aura: Option<u32>,
    /// A sleeper wakes when a character comes within this many yards, plus
    /// `wake_per_stack_yd` for every stack, plus its own `wake_radius_mod`.
    #[serde(default)]
    pub(crate) wake_base_yd: f32,
    #[serde(default)]
    pub(crate) wake_per_stack_yd: f32,
    /// The Creature Spell id of the aura every asleep creature of the Run wears until it wakes.
    #[serde(default)]
    pub(crate) sleep_aura: Option<u32>,
}

impl Meter {
    /// How near a character wakes a sleeper with this `wake_radius_mod` at `stacks`.
    pub(crate) fn wake_radius(&self, stacks: u32, wake_radius_mod: f32) -> f32 {
        (self.wake_base_yd + self.wake_per_stack_yd * stacks as f32 + wake_radius_mod).max(0.0)
    }
}

/// A meter's cause lines (`[meter.lines]`), one for each source of its stacks.
#[derive(Deserialize)]
pub(crate) struct MeterLines {
    pub(crate) first_damage: String,
    pub(crate) death: String,
    pub(crate) cast: String,
    pub(crate) clock: String,
}

/// The stacks each source added to a Run's meter in one hearing, before the meter's max.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Heard {
    /// From creatures a player hurt for the first time in their lives.
    pub(crate) first_damage: u32,
    /// From creatures' deaths.
    pub(crate) death: u32,
    /// From creatures' casts.
    pub(crate) cast: u32,
    /// From the Run's clock.
    pub(crate) clock: u32,
}

impl Heard {
    /// The line of the source that added the most stacks, a tie going to the first of first
    /// damage, death, cast and clock; `None` when none added any.
    pub(crate) fn line<'a>(&self, lines: &'a MeterLines) -> Option<&'a str> {
        let sources = [
            (self.first_damage, &lines.first_damage),
            (self.death, &lines.death),
            (self.cast, &lines.cast),
            (self.clock, &lines.clock),
        ];
        let mut most: Option<(u32, &String)> = None;
        for (stacks, line) in sources {
            if stacks > most.map_or(0, |m| m.0) {
                most = Some((stacks, line));
            }
        }
        most.map(|m| m.1.as_str())
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

    /// The locked door of this dungeon that a step from `from` to `to` on `map` walks through:
    /// the step crosses the doorway's plane inside its rectangle, in either direction.
    pub(crate) fn locked_door_crossed(
        &self,
        map: u32,
        from: [f32; 3],
        to: [f32; 3],
        unlocked: &[u32],
    ) -> Option<&LockedDoor> {
        if map != self.map {
            return None;
        }
        let (a, b) = (self.local(from), self.local(to));
        self.locked_door.iter().find(|door| {
            let w = &door.doorway;
            if unlocked.contains(&door.guid) || (a[0] - w.x) * (b[0] - w.x) >= 0.0 {
                return false;
            }
            let t = (w.x - a[0]) / (b[0] - a[0]);
            let (y, z) = (a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t);
            (w.y[0]..=w.y[1]).contains(&y) && (w.z[0]..=w.z[1]).contains(&z)
        })
    }

    /// The hint the key's `hint_npc` gives when `holder` carries the key.
    pub(crate) fn hint_for(&self, holder: u32) -> Option<&str> {
        let key = self.key.as_ref()?;
        let h = key.holders.iter().find(|h| h.guid == holder)?;
        Some(&h.hint)
    }

    /// Fails loading when its door names a `speaker` that is no open-world creature of `spawns`
    /// and no Hub NPC on its map.
    pub(crate) fn check_speaker(&self, spawns: &[crate::mobs::Spawn]) {
        let Some(speaker) = self.portal.door.as_ref().and_then(|door| door.speaker) else {
            return;
        };
        let spawned = spawns
            .iter()
            .any(|s| s.guid == speaker && s.map == self.map && s.phase == Phase::OpenWorld)
            || crate::hub::spawned(speaker, self.map).is_some();
        assert!(
            spawned,
            "{}: door speaker {speaker} is no open-world creature or Hub NPC",
            self.name
        );
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
            display_id: STOP_PORTAL_DISPLAY,
            type_id: GOOBER,
            flags: 0,
            faction: 0,
            position,
            orientation,
        }
    }

    /// World `[x, y]` points at most `step` yards apart over the footprint's boxes and its
    /// stops' boxes, edges included.
    pub(crate) fn footprint_points(&self, step: f32) -> Vec<[f32; 2]> {
        use crate::world::closed::steps;
        let (sin, cos) = self.yaw.to_radians().sin_cos();
        let [ox, oy, _] = self.origin;
        let mut out = Vec::new();
        for b in &self.footprint {
            for lx in steps(b.x, step) {
                for ly in steps(b.y, step) {
                    out.push([ox + lx * cos - ly * sin, oy + lx * sin + ly * cos]);
                }
            }
        }
        for s in &self.stop {
            for x in steps(s.x, step) {
                out.extend(steps(s.y, step).into_iter().map(|y| [x, y]));
            }
        }
        out
    }

    /// `position` in the building's frame.
    fn local(&self, [x, y, z]: [f32; 3]) -> [f32; 3] {
        let (sin, cos) = self.yaw.to_radians().sin_cos();
        let [ox, oy, oz] = self.origin;
        let (dx, dy) = (x - ox, y - oy);
        [dx * cos + dy * sin, dy * cos - dx * sin, z - oz]
    }
}

/// Whether a Phased Dungeon's door, ward stone or lever has this GameObject entry: a use
/// objective may name any of them.
pub(crate) fn is_door(entry: u32) -> bool {
    content().iter().any(|d| {
        d.portal.door.as_ref().is_some_and(|o| o.entry == entry)
            || d.gameobject
                .iter()
                .any(|o| o.guid == entry && (o.use_kind.is_some() || o.gives_quests))
    })
}

/// The dungeon whose locked door has this GameObject guid.
pub(crate) fn locked_door_with(guid: u64) -> Option<(&'static Dungeon, &'static LockedDoor)> {
    content().iter().find_map(|d| {
        d.locked_door
            .iter()
            .find(|o| o.object_guid() == guid)
            .map(|o| (d, o))
    })
}

/// The hint a dungeon's `hint_npc` gives when creature `holder` carries its key.
pub(crate) fn hint_text(holder: u32) -> Option<&'static str> {
    content().iter().find_map(|d| d.hint_for(holder))
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

/// Every Phased Dungeon's content file: (file name, text). A file may also hold `[[template]]`
/// rows, which `mobs::content` reads; [`Dungeon`] ignores them.
pub fn sources() -> &'static [(&'static str, &'static str)] {
    macro_rules! file {
        ($name:literal) => {
            ($name, include_str!(concat!("../content/", $name)))
        };
    }
    &[
        file!("northshire_abbey.toml"),
        file!("test_grounds.toml"),
        file!("forest_edge.toml"),
        file!("sealed_yard.toml"),
        file!("gaze_hall.toml"),
        file!("jasperlode.toml"),
        file!("ward_test.toml"),
        file!("azora.toml"),
        file!("ilgalar.toml"),
        file!("stonewatch_keep.toml"),
        file!("mistmantle_manor.toml"),
        file!("raven_hill_deeps.toml"),
        file!("cathedral_of_light.toml"),
        file!("stormwind_keep.toml"),
        file!("blackrock_stronghold.toml"),
        file!("dreadmaul_rock.toml"),
        file!("stockade_riot.toml"),
        file!("stockade_quiet_wing.toml"),
        file!("deeprun_tram.toml"),
    ]
}

/// Every Phased Dungeon: one content file each.
pub(crate) fn content() -> &'static [Dungeon] {
    static DUNGEONS: OnceLock<Vec<Dungeon>> = OnceLock::new();
    DUNGEONS.get_or_init(|| {
        sources()
            .iter()
            .map(|(name, file)| {
                toml::from_str(file).unwrap_or_else(|e| panic!("content/{name} parses: {e}"))
            })
            .inspect(|d: &Dungeon| {
                // A dungeon stands on Eastern Kingdoms, walked into, or on an instance map, whose
                // own doors worldport a character in and out (ADR 0032).
                assert!(
                    d.map == crate::content::EASTERN_KINGDOMS
                        || crate::instance_map::of_map(d.map).is_some(),
                    "{}: map {} is not Eastern Kingdoms and no instance map",
                    d.name,
                    d.map
                );
                for c in &d.creature {
                    for roll in &c.rolls {
                        roll.check(&format!("creature {}", c.guid));
                    }
                }
                if let Some(key) = &d.key {
                    let has = |guid: u32| d.creature.iter().any(|c| c.guid == guid);
                    assert!(has(key.hint_npc), "{}: key hint_npc is no creature", d.name);
                    assert!(!key.holders.is_empty(), "{}: a key with no holders", d.name);
                    for h in &key.holders {
                        assert!(
                            has(h.guid),
                            "{}: key holder {} is no creature",
                            d.name,
                            h.guid
                        );
                    }
                    for door in &d.locked_door {
                        assert_eq!(door.opens_with, key.item, "{}: door {}", d.name, door.guid);
                    }
                }
                d.check_levers();
                d.check_frozen();
                d.check_partners();
                d.check_bonus_chest();
                if let Some(tram) = &d.tram {
                    tram.check(&d.name);
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

/// The loot rolls a tram line's creature of `entry` makes in Phased Dungeon `dungeon`: the
/// Conductor's `conductor_rolls` (#592); none for any other.
pub(crate) fn tram_rolls(dungeon: u32, entry: u32) -> &'static [crate::rules::loot::Roll] {
    self::dungeon(dungeon)
        .and_then(|d| d.tram.as_ref())
        .filter(|t| t.conductor == entry)
        .map_or(&[], |t| &t.conductor_rolls)
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
    /// The guids of the levers pulled in it.
    pub(crate) used: Vec<u32>,
    /// The guids of the creatures that came into the Run after its start.
    pub(crate) appeared: Vec<u32>,
    /// Whether the creatures it started with stand on the host's ground: the shard makes them
    /// before it has a host, and grounds them on its first tick, before any client is shown them.
    pub(crate) grounded: bool,
    /// The guid of the creature that carries the dungeon's key, rolled as the Run starts.
    pub(crate) key_holder: Option<u32>,
    /// The guids of the locked doors a character has unlocked in the Run.
    pub(crate) unlocked: Vec<u32>,
    /// The Head Count last counted (ADR 0029); 0 before the first count, when none has been
    /// announced.
    pub(crate) head_count: u32,
    /// The dungeon's tram line (ADR 0033), from the tick after its bell is rung.
    pub(crate) tram: Option<crate::tram::Line>,
    /// The seed it started with, which rolls its tram's creatures.
    pub(crate) seed: u64,
    /// The creatures its Bonus Chest watches that have died, each with `elapsed` at its death.
    pub(crate) chest_deaths: Vec<(u32, f32)>,
    /// Where the Bonus Chest's boss died.
    pub(crate) chest_corpse: Option<[f32; 3]>,
    /// Whether its Bonus Chest's challenge was judged.
    pub(crate) chest_judged: bool,
    /// Where the Bonus Chest stands, once earned and until the realm makes it.
    pub(crate) chest_due: Option<[f32; 3]>,
    /// The run-time GameObject that holds its Bonus Chest, which goes with the Run.
    pub(crate) chest: Option<u64>,
}

impl Run {
    /// A Run of dungeon `dungeon`, its inner portals as the content opens them and its key on
    /// the holder `seed` picks; `seed` also rolls its tram's creatures.
    pub(crate) fn new(dungeon: u32, seed: u64) -> Self {
        Self {
            elapsed: 0.0,
            noise: 0,
            clock_stacks: 0,
            portals_open: self::dungeon(dungeon).map_or(Vec::new(), |d| d.portals_open_at_start()),
            pinned: Vec::new(),
            dead: Vec::new(),
            used: Vec::new(),
            appeared: Vec::new(),
            grounded: false,
            key_holder: self::dungeon(dungeon)
                .and_then(|d| d.key.as_ref())
                .map(|k| k.holders[((seed >> 32) * k.holders.len() as u64 >> 32) as usize].guid),
            unlocked: Vec::new(),
            head_count: 0,
            tram: None,
            seed,
            chest_deaths: Vec::new(),
            chest_corpse: None,
            chest_judged: false,
            chest_due: None,
            chest: None,
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

    /// Marks the lever `guid` used for the rest of the Run. Whether it was not used already.
    pub(crate) fn pull(&mut self, guid: u32) -> bool {
        let new = !self.used.contains(&guid);
        if new {
            self.used.push(guid);
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

    /// Adds the stacks `heard` from the creatures and the clock's due, then takes `drain` off,
    /// within `0..=max`. What each source added, the clock's among them.
    pub(crate) fn hear(&mut self, meter: &Meter, heard: Heard, drain: u32) -> Heard {
        let mut heard = heard;
        while meter.clock_stack_every_s > 0.0
            && (self.clock_stacks + 1) as f32 * meter.clock_stack_every_s <= self.elapsed
        {
            self.clock_stacks += 1;
            heard.clock += 1;
        }
        let gain = heard.first_damage + heard.death + heard.cast + heard.clock;
        self.noise = (self.noise + gain).min(meter.max);
        self.noise = self.noise.saturating_sub(drain);
        heard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_waiting_on_a_milestone_comes_later_or_goes() {
        let row = |text: &str| -> Appears { toml::from_str(text).unwrap() };
        let after = row("after_milestone = \"ilgalar_anchor_broken\"");
        let until = row("until_milestone = \"ilgalar_anchor_broken\"");
        let (none, anchor): ([&str; 0], _) = ([], ["ilgalar_anchor_broken"]);
        assert!(after.comes_later() && !until.comes_later());
        assert!(!after.met(&[], &[], &[], &none) && after.met(&[], &[], &[], &anchor));
        assert!(until.met(&[], &[], &[], &none) && !until.met(&[], &[], &[], &anchor));
        assert!(!until.gone(&[], &[], &none) && until.gone(&[], &[], &anchor));
        assert!(!after.gone(&[], &[], &anchor));
    }

    #[test]
    fn the_abbeys_tower_door_blocks_a_crossing_of_its_doorway_and_no_other_step() {
        let abbey = dungeon(1).unwrap();
        let door = &abbey.locked_door[0];
        let at = |x: f32, y: f32, z: f32| {
            let (sin, cos) = abbey.yaw.to_radians().sin_cos();
            let [ox, oy, oz] = abbey.origin;
            [ox + x * cos - y * sin, oy + x * sin + y * cos, oz + z]
        };
        let across = |a: [f32; 3], b: [f32; 3], open: &[u32]| {
            abbey.locked_door_crossed(0, a, b, open).is_some()
        };
        let (west, east) = (at(-5.0, -8.3, 17.6), at(-3.0, -8.3, 17.6));
        assert!(across(west, east, &[]), "west to east");
        assert!(across(east, west, &[]), "east to west");
        assert!(!across(west, east, &[door.guid]), "an unlocked door");
        // Beside the doorway, above its top and on the same side.
        assert!(!across(at(-5.0, -5.0, 17.6), at(-3.0, -5.0, 17.6), &[]));
        assert!(!across(at(-5.0, -8.3, 24.0), at(-3.0, -8.3, 24.0), &[]));
        assert!(!across(west, at(-4.5, -8.3, 17.6), &[]));
    }

    #[test]
    fn the_abbeys_key_is_a_known_item_its_holders_are_creatures_and_the_door_takes_it() {
        let abbey = dungeon(1).unwrap();
        let key = abbey.key.as_ref().unwrap();
        assert!(crate::items::template(key.item).is_some());
        assert_eq!(abbey.locked_door[0].opens_with, key.item);
    }

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

    /// A dungeon file of one creature (guid 1) and a lever (guid 2), with `more` after them.
    fn with_lever(more: &str) -> Dungeon {
        let file = format!(
            r#"
id = 99
name = "the Lever Check"
map = 0
origin = [0.0, 0.0, 0.0]
yaw = 0.0
footprint = []
portal = {{ position = [0.0, 0.0, 0.0], orientation = 0.0 }}

[[creature]]
guid = 1
entry = 96003
level = 1
position = [0.0, 0.0, 0.0]
orientation = 0.0

[[gameobject]]
guid = 2
display = 461
position = [0.0, 0.0, 0.0]
orientation = 0.0
sides = "inside"
use = "lever"
lever = {{ needs_dead = 1, held_text = "held", used_text = "used" }}
{more}"#
        );
        toml::from_str(&file).expect("the dungeon parses")
    }

    #[test]
    fn a_lever_and_what_waits_on_it_pass_the_check() {
        with_lever(
            r#"
[[gameobject]]
guid = 3
display = 461
position = [0.0, 0.0, 0.0]
orientation = 0.0
sides = "inside"
after_used = 2
until_used = 2
"#,
        )
        .check_levers();
    }

    #[test]
    #[should_panic(expected = "a lever table or use")]
    fn a_lever_table_without_the_use_is_refused() {
        with_lever(
            r#"
[[gameobject]]
guid = 3
display = 461
position = [0.0, 0.0, 0.0]
orientation = 0.0
sides = "inside"
lever = { used_text = "used" }
"#,
        )
        .check_levers();
    }

    #[test]
    #[should_panic(expected = "needs_dead 7, no creature")]
    fn a_lever_that_needs_no_creature_of_the_dungeon_is_refused() {
        with_lever(
            r#"
[[gameobject]]
guid = 3
display = 461
position = [0.0, 0.0, 0.0]
orientation = 0.0
sides = "inside"
use = "lever"
lever = { needs_dead = 7, used_text = "used" }
"#,
        )
        .check_levers();
    }

    #[test]
    #[should_panic(expected = "waits on 1, no lever")]
    fn an_after_used_that_names_no_lever_is_refused() {
        with_lever(
            r#"
[[gameobject]]
guid = 3
display = 461
position = [0.0, 0.0, 0.0]
orientation = 0.0
sides = "inside"
after_used = 1
"#,
        )
        .check_levers();
    }

    #[test]
    #[should_panic(expected = "stops on 3, no lever")]
    fn a_timed_call_until_used_that_names_no_lever_is_refused() {
        with_lever(
            r#"
[[creature]]
guid = 4
entry = 96003
level = 1
position = [0.0, 0.0, 0.0]
orientation = 0.0

[[creature.every]]
every_s = 20
entry = 96003
level = 1
count = 2
position = [0.0, 0.0, 0.0]
until_used = 3
"#,
        )
        .check_levers();
    }

    /// `challenge` as its content row writes it.
    fn challenge(text: &str) -> Challenge {
        #[derive(Deserialize)]
        struct Row {
            challenge: Challenge,
        }
        toml::from_str::<Row>(&format!("challenge = {text}"))
            .expect("the challenge parses")
            .challenge
    }

    #[test]
    fn within_s_holds_at_its_seconds_and_fails_a_second_later() {
        let c = challenge(r#"{ kind = "within_s", seconds = 300 }"#);
        assert_eq!(c, Challenge::WithinS { seconds: 300.0 });
        assert!(c.holds(&[(1, 300.0)], &[], 300.0));
        assert!(!c.holds(&[(1, 301.0)], &[], 301.0));
    }

    #[test]
    fn unused_fails_on_a_listed_lever_and_holds_on_an_unlisted_one() {
        let c = challenge(r#"{ kind = "unused", levers = [2, 3] }"#);
        assert!(c.holds(&[(1, 10.0)], &[], 10.0));
        assert!(!c.holds(&[(1, 10.0)], &[3], 10.0), "a listed lever");
        assert!(c.holds(&[(1, 10.0)], &[5], 10.0), "an unlisted lever");
    }

    #[test]
    fn together_s_holds_for_deaths_its_seconds_apart_and_waits_for_the_last() {
        let c = challenge(r#"{ kind = "together_s", guids = [1, 4], seconds = 10 }"#);
        assert!(c.holds(&[(4, 5.0), (1, 15.0)], &[], 15.0));
        assert!(!c.holds(&[(4, 5.0), (1, 16.0)], &[], 16.0));
        assert!(
            c.holds(&[(1, 20.0), (4, 30.0)], &[], 20.0),
            "the boss first"
        );
        let chest: BonusChest = toml::from_str(
            r#"
boss = 1
entry = 980799
display = 259
east_yd = 2.0
money = 1
one_of = [905040]
challenge = { kind = "together_s", guids = [1, 4], seconds = 10 }
"#,
        )
        .unwrap();
        assert_eq!(chest.judge(&[(1, 5.0)], &[]), None, "4 lives");
        assert_eq!(chest.judge(&[(4, 5.0)], &[]), None, "the boss lives");
        assert_eq!(chest.judge(&[(1, 5.0), (4, 9.0)], &[]), Some(true));
        assert_eq!(chest.judge(&[(1, 5.0), (4, 16.0)], &[]), Some(false));
        let mut watched = chest.watched();
        watched.sort_unstable();
        assert_eq!(watched, [1, 4]);
    }

    #[test]
    fn the_boss_s_death_judges_within_s_and_unused() {
        let unused: BonusChest = toml::from_str(
            r#"
boss = 1
entry = 980799
display = 259
east_yd = 2.0
money = 1
one_of = [905040]
challenge = { kind = "unused", levers = [2] }
"#,
        )
        .unwrap();
        assert_eq!(unused.judge(&[], &[]), None, "the boss lives");
        assert_eq!(unused.judge(&[(1, 3.0)], &[]), Some(true));
        assert_eq!(unused.judge(&[(1, 3.0)], &[2]), Some(false));
    }

    /// The Lever Check with a Bonus Chest of `boss`, `one_of` and `challenge`, and a creature 4.
    fn with_bonus_chest(boss: u32, one_of: &str, challenge: &str) -> Dungeon {
        with_lever(&format!(
            r#"
[[creature]]
guid = 4
entry = 96003
level = 1
position = [0.0, 0.0, 0.0]
orientation = 0.0

[bonus_chest]
boss = {boss}
entry = 980799
display = 259
east_yd = 2.0
money = 100
one_of = {one_of}
challenge = {challenge}
"#
        ))
    }

    #[test]
    fn a_bonus_chest_of_its_own_creatures_levers_and_items_passes_the_check() {
        let items = "[905040, 905041]";
        with_bonus_chest(1, items, r#"{ kind = "unused", levers = [2] }"#).check_bonus_chest();
        with_bonus_chest(1, items, r#"{ kind = "within_s", seconds = 60 }"#).check_bonus_chest();
        with_bonus_chest(
            4,
            items,
            r#"{ kind = "together_s", guids = [1, 4], seconds = 10 }"#,
        )
        .check_bonus_chest();
    }

    #[test]
    #[should_panic(expected = "the Lever Check: the bonus chest's boss 7 is no creature")]
    fn a_bonus_chest_whose_boss_is_no_creature_is_refused() {
        with_bonus_chest(7, "[905040]", r#"{ kind = "within_s", seconds = 60 }"#)
            .check_bonus_chest();
    }

    #[test]
    #[should_panic(expected = "together_s 7 is no creature")]
    fn a_together_s_guid_that_is_no_creature_is_refused() {
        with_bonus_chest(
            1,
            "[905040]",
            r#"{ kind = "together_s", guids = [1, 7], seconds = 10 }"#,
        )
        .check_bonus_chest();
    }

    #[test]
    #[should_panic(expected = "unused 1 is no lever")]
    fn an_unused_lever_that_is_no_lever_is_refused() {
        with_bonus_chest(1, "[905040]", r#"{ kind = "unused", levers = [1] }"#).check_bonus_chest();
    }

    #[test]
    #[should_panic(expected = "together_s leaves out its boss 1")]
    fn a_together_s_without_the_boss_is_refused() {
        with_bonus_chest(
            1,
            "[905040]",
            r#"{ kind = "together_s", guids = [4], seconds = 10 }"#,
        )
        .check_bonus_chest();
    }

    #[test]
    #[should_panic(expected = "the bonus chest holds no items")]
    fn a_bonus_chest_of_no_items_is_refused() {
        with_bonus_chest(1, "[]", r#"{ kind = "within_s", seconds = 60 }"#).check_bonus_chest();
    }

    #[test]
    #[should_panic(expected = "names item 4242424, which the content does not have")]
    fn a_bonus_chest_of_an_unknown_item_is_refused() {
        with_bonus_chest(1, "[4242424]", r#"{ kind = "within_s", seconds = 60 }"#)
            .check_bonus_chest();
    }

    /// The Lever Check with creature 4 frozen until `frozen_until`, starting asleep or not.
    fn with_frozen(frozen_until: &str, asleep: bool) -> Dungeon {
        with_lever(&format!(
            r#"
[[creature]]
guid = 4
entry = 96003
level = 1
position = [0.0, 0.0, 0.0]
orientation = 0.0
asleep = {asleep}
frozen_until = {frozen_until}
"#
        ))
    }

    #[test]
    fn a_frozen_until_on_a_creature_of_the_dungeon_passes_the_check() {
        let d = with_frozen("{ guid = 1, pct = 75 }", false);
        d.check_frozen();
        let f = d.creature[1].frozen_until.as_ref().unwrap();
        assert_eq!(f.spell, "frozen_in_the_mist", "the marker's default");
        with_frozen(
            r#"{ guid = 1, pct = 1, spell = "frozen_in_the_mist" }"#,
            false,
        )
        .check_frozen();
    }

    #[test]
    #[should_panic(expected = "waits on 7, no creature")]
    fn a_frozen_until_that_names_no_creature_of_the_dungeon_is_refused() {
        with_frozen("{ guid = 7, pct = 75 }", false).check_frozen();
    }

    #[test]
    #[should_panic(expected = "pct 100 is not 1 to 99")]
    fn a_frozen_until_past_99_percent_is_refused() {
        with_frozen("{ guid = 1, pct = 100 }", false).check_frozen();
    }

    #[test]
    #[should_panic(expected = "pct 0 is not 1 to 99")]
    fn a_frozen_until_at_0_percent_is_refused() {
        with_frozen("{ guid = 1, pct = 0 }", false).check_frozen();
    }

    #[test]
    #[should_panic(expected = "no Creature Spell")]
    fn a_frozen_until_with_an_unknown_marker_is_refused() {
        with_frozen(r#"{ guid = 1, pct = 50, spell = "no_such_ice" }"#, false).check_frozen();
    }

    #[test]
    #[should_panic(expected = "starts asleep")]
    fn a_frozen_creature_that_starts_asleep_is_refused() {
        with_frozen("{ guid = 1, pct = 50 }", true).check_frozen();
    }

    /// The Lever Check with creature 4 linked to a partner by `on_partner_death`.
    fn with_partner(on_partner_death: &str) -> Dungeon {
        with_lever(&format!(
            r#"
[[creature]]
guid = 4
entry = 96003
level = 1
position = [0.0, 0.0, 0.0]
orientation = 0.0
on_partner_death = {on_partner_death}
"#
        ))
    }

    #[test]
    fn an_on_partner_death_naming_a_creature_of_the_dungeon_and_a_stance_passes_the_check() {
        let d = with_partner(r#"{ guid = 1, spell = "test_enrage" }"#);
        d.check_partners();
        let p = d.creature[1].on_partner_death.as_ref().unwrap();
        assert_eq!((p.guid, p.spell.as_str()), (1, "test_enrage"));
    }

    #[test]
    #[should_panic(expected = "names 7, no creature")]
    fn an_on_partner_death_that_names_no_creature_of_the_dungeon_is_refused() {
        with_partner(r#"{ guid = 7, spell = "test_enrage" }"#).check_partners();
    }

    #[test]
    #[should_panic(expected = "names itself")]
    fn an_on_partner_death_naming_its_own_row_is_refused() {
        with_partner(r#"{ guid = 4, spell = "test_enrage" }"#).check_partners();
    }

    #[test]
    #[should_panic(expected = "no stance Creature Spell")]
    fn an_on_partner_death_whose_spell_is_no_stance_is_refused() {
        with_partner(r#"{ guid = 1, spell = "frozen_in_the_mist" }"#).check_partners();
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
        assert!(
            abbot.on_health.is_empty(),
            "the Abbot calls no adds (2026-10-25)"
        );
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
        assert_eq!(q.kill, [crate::quests::content::Kill::one(96005, 1)]);
    }

    #[test]
    fn the_idol_the_abbey_and_the_abbots_fall_each_offer_their_choices() {
        let choices = |id| crate::quests::content::quest(id).unwrap().choices.clone();
        assert_eq!(choices(94001), [900414, 900415, 900416]);
        assert_eq!(choices(94002), [900400, 900401, 900402, 900403]);
        assert_eq!(choices(94004), [900520, 900521, 900522]);
    }

    #[test]
    fn jasperlodes_portal_stands_open_with_no_door() {
        let d = content().iter().find(|d| d.id == 6).expect("Jasperlode");
        assert!(d.portal.door.is_none());
        assert!(!d.footprint.is_empty() && !d.creature.is_empty());
    }

    #[test]
    #[should_panic(expected = "door speaker 971006 is no open-world creature")]
    fn a_door_speaker_that_is_no_open_world_creature_is_refused() {
        // The Keep's Phase Tester stands in a Phase, not the open world.
        let text = include_str!("../content/sealed_yard.toml").replacen(
            "sealed_text =",
            "speaker = 971006\nsealed_text =",
            1,
        );
        let d: Dungeon = toml::from_str(&text).expect("the Sealed Yard parses");
        d.check_speaker(&crate::mobs::content().spawn);
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
        assert!(!eye.rolls.is_empty());
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
            .filter(|o| o.cover_yd.is_some())
            .count();
        assert!(hides >= 5, "{hides} hiding spots");
        for o in edge.gameobject.iter().filter(|o| o.cover_yd.is_some()) {
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
            .find(|o| o.cover_yd.is_some())
            .unwrap();
        let fence = edge
            .gameobject
            .iter()
            .find(|o| o.cover_yd.is_none())
            .unwrap();
        assert!(!spot.seen_from(Phase::OpenWorld, 2));
        assert!(spot.seen_from(Phase::Dungeon(2), 2));
        assert!(fence.seen_from(Phase::OpenWorld, 2));
        // Cover is walked into, never clicked, and the client walks through it alone.
        assert!(edge.gameobject.iter().all(|o| !o.clicked()));
        assert!(edge.usable(spot.object_guid()).is_none());
        assert!(edge.walk_through().contains(&spot.object_guid()));
        assert!(!edge.walk_through().contains(&fence.object_guid()));
    }

    #[test]
    fn forest_edge_holds_the_riverpaw_camp_in_nine_camps_and_three_patrols() {
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
            (33..=40).contains(&sleepers.len()),
            "{} sleepers",
            sleepers.len()
        );
        let mut camps: Vec<_> = sleepers.iter().filter_map(|c| c.camp).collect();
        assert!(sleepers.iter().all(|c| c.camp.is_some()), "a sleeper alone");
        camps.sort_unstable();
        camps.dedup();
        assert_eq!(camps.len(), 9, "camps {camps:?}");
        for camp in camps {
            let size = sleepers.iter().filter(|c| c.camp == Some(camp)).count();
            assert!((3..=5).contains(&size), "camp {camp} has {size}");
        }
        let awake: Vec<_> = edge.creature.iter().filter(|c| !c.asleep).collect();
        assert_eq!(awake.len(), 3);
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
                .filter(|o| o.cover_yd.is_some())
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
    fn forest_edges_noise_runs_to_40_at_half_a_yard_a_stack() {
        let meter = dungeon(2).unwrap().meter.as_ref().unwrap();
        assert_eq!(meter.max, 40);
        assert_eq!(meter.wake_radius(0, 0.0), 3.0);
        assert_eq!(meter.wake_radius(40, 0.0), 23.0);
        // Hogger (-15) stays asleep a step from his tent at 39 stacks.
        assert_eq!(meter.wake_radius(39, -15.0), 7.5);
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

#[cfg(test)]
mod lore_bible_dungeons {
    use super::*;

    /// The lore bible's eight dungeons on the open world, ids 8 to 15.
    const IDS: std::ops::RangeInclusive<u32> = 8..=15;

    #[test]
    fn no_open_world_creature_or_hub_npc_stands_inside_them() {
        let dungeons: Vec<_> = IDS.map(|id| dungeon(id).expect("a dungeon")).collect();
        let inside = |map: u32, p: [f32; 3]| dungeons.iter().find(|d| d.holds(map, p));
        let mut found = Vec::new();
        for s in &crate::mobs::content().spawn {
            if s.phase != Phase::OpenWorld {
                continue;
            }
            if let Some(d) = inside(s.map, s.position) {
                found.push(format!(
                    "{}: creature {} (entry {})",
                    d.name, s.guid, s.entry
                ));
            }
        }
        for npc in crate::hub::npcs() {
            if let Some(d) = inside(npc.map, npc.position) {
                found.push(format!("{}: Hub NPC {}", d.name, npc.entry));
            }
        }
        assert!(found.is_empty(), "{}", found.join("\n"));
    }

    /// Stormwind Keep, whose Prestor and Anduin stand beside its Phase Tester (#559).
    const STORMWIND_KEEP: u32 = 13;

    /// The Phase Tester, the one creature of a shell.
    const PHASE_TESTER: u32 = 97100;

    /// Why `d` is neither a shell (every creature friendly, one of them, three in Stormwind Keep)
    /// nor a filled dungeon (no Phase Tester, a Boss Bar creature, every creature inside its
    /// footprint); `None` when it is one of them.
    fn not_shell_or_filled(d: &Dungeon) -> Option<String> {
        if let Some(c) = d.creature.iter().find(|c| !d.holds(d.map, c.position)) {
            return Some(format!("{}: creature {} stands outside", d.name, c.guid));
        }
        let template = |c: &Creature| crate::mobs::template(c.entry);
        let want = if d.id == STORMWIND_KEEP { 3 } else { 1 };
        let friendly = |c: &Creature| template(c).is_some_and(|t| t.friendly);
        if d.creature.len() == want && d.creature.iter().all(friendly) {
            return None;
        }
        if let Some(c) = d.creature.iter().find(|c| c.entry == PHASE_TESTER) {
            return Some(format!(
                "{}: a Phase Tester ({}) beside creatures a shell has not",
                d.name, c.guid
            ));
        }
        let boss = |c: &Creature| template(c).is_some_and(|t| t.boss);
        (!d.creature.iter().any(boss)).then(|| format!("{}: no Boss Bar creature", d.name))
    }

    #[test]
    fn each_is_a_shell_or_filled_and_its_portal_stands_at_the_door() {
        for id in IDS {
            let d = dungeon(id).expect("a dungeon");
            if let Some(why) = not_shell_or_filled(d) {
                panic!("{why}");
            }
            // A dungeon makes a mark when it has levels and stands on no instance map, whose
            // own mark stands at its door (ADR 0032).
            assert!(
                d.levels.is_some() && crate::instance_map::of_map(d.map).is_none(),
                "{}: a map mark",
                d.name
            );
            let near = d
                .footprint_points(1.0)
                .into_iter()
                .any(|[x, y]| flat_distance(d.portal.position, [x, y, 0.0]) <= 5.0);
            assert!(near, "{}: its portal is off the footprint", d.name);
        }
    }

    /// A shell as Mistmantle Manor's was before #641: the house's one room and its Phase Tester.
    const SHELL: &str = "id = 10\nname = \"A Shell\"\nlevels = [32, 36]\nmap = 0\n\
        origin = [-10368.6, -1255.5, 31.8]\nyaw = 90.5\n\n\
        [[footprint]]\nx = [-7.0, 7.0]\ny = [-7.0, 9.0]\nz = [2.0, 15.0]\n\n\
        [portal]\nposition = [-10362.6, -1257.0, 35.91]\norientation = 3.15\n\n\
        [[creature]]\nguid = 971003\nentry = 97100\nlevel = 32\n\
        position = [-10370.6, -1255.0, 35.91]\norientation = 3.15\n";

    /// [`SHELL`] with a creature of `entry` beside its Phase Tester, or in its place when
    /// `tester` is false.
    fn shell_with(entry: u32, tester: bool) -> Dungeon {
        let text = if tester {
            SHELL.to_string()
        } else {
            SHELL.replacen("entry = 97100", "entry = 3122", 1)
        };
        let row = format!(
            "\n[[creature]]\nguid = 971004\nentry = {entry}\nlevel = 32\n\
             position = [-10370.6, -1254.0, 35.91]\norientation = 3.15\n"
        );
        toml::from_str(&(text + &row)).unwrap()
    }

    #[test]
    fn a_phase_tester_beside_a_hostile_creature_is_neither() {
        // Bloodtalon Taillasher: hostile, no Boss Bar.
        let why = not_shell_or_filled(&shell_with(3122, true));
        assert!(why.is_some_and(|w| w.contains("Phase Tester")));
    }

    #[test]
    fn a_filled_dungeon_with_no_boss_bar_creature_is_neither() {
        let why = not_shell_or_filled(&shell_with(3122, false));
        assert!(why.is_some_and(|w| w.contains("no Boss Bar creature")));
    }

    #[test]
    fn a_filled_dungeon_with_a_boss_bar_creature_passes() {
        // Bloodtalon Scythemaw has the Boss Bar.
        assert_eq!(not_shell_or_filled(&shell_with(3123, false)), None);
    }

    /// Raven Hill Deeps and Stonewatch Keep take the lore bible's bands, Raven Hill 40-44 and
    /// Stonewatch 45-48 (#595 decision 9).
    #[test]
    fn raven_hill_and_stonewatch_take_the_bibles_levels() {
        assert_eq!(
            dungeon(11).expect("Raven Hill Deeps").levels,
            Some([40, 44])
        );
        assert_eq!(dungeon(9).expect("Stonewatch Keep").levels, Some([45, 48]));
    }
}
