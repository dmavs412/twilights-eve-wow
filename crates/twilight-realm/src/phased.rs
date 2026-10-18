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
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
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
const GENERIC: u32 = 5;
const NOT_SELECTABLE: u32 = 0x10;
const READY: u32 = 1;

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
}

impl Dungeon {
    /// The portal's guid: one per dungeon.
    pub(crate) fn portal_guid(&self) -> u64 {
        HIGHGUID_GAMEOBJECT << 48 | u64::from(self.id)
    }

    /// The portal's create: a GameObject that only shows, never clicked.
    pub(crate) fn portal_create(&self) -> SessionEvent {
        let Portal {
            position: [x, y, z],
            orientation,
        } = self.portal;
        let (sin, cos) = (orientation / 2.0).sin_cos();
        let mut pairs = vec![
            (OBJECT_FIELD_TYPE, TYPE_GAMEOBJECT),
            (OBJECT_FIELD_ENTRY, self.id),
            (OBJECT_FIELD_SCALE_X, float(1.0)),
            (GAMEOBJECT_DISPLAYID, PORTAL_DISPLAY),
            (GAMEOBJECT_FLAGS, NOT_SELECTABLE),
            (GAMEOBJECT_ROTATION + 2, float(sin)),
            (GAMEOBJECT_ROTATION + 3, float(cos)),
            (GAMEOBJECT_STATE, READY),
            (GAMEOBJECT_POS_X, float(x)),
            (GAMEOBJECT_POS_X + 1, float(y)),
            (GAMEOBJECT_POS_X + 2, float(z)),
            (GAMEOBJECT_POS_X + 3, float(orientation)),
            (GAMEOBJECT_TYPE_ID, GENERIC),
        ];
        pairs.extend(guid_pairs(
            crate::fields::OBJECT_FIELD_GUID,
            self.portal_guid(),
        ));
        SessionEvent::ObjectCreate {
            guid: self.portal_guid(),
            kind: EntityKind::GameObject,
            display_id: Some(PORTAL_DISPLAY),
            position: self.portal.position,
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
    pub(crate) creature: Vec<Creature>,
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

/// Every Phased Dungeon: one content file each.
pub(crate) fn content() -> &'static [Dungeon] {
    static DUNGEONS: OnceLock<Vec<Dungeon>> = OnceLock::new();
    DUNGEONS.get_or_init(|| {
        [include_str!("../content/northshire_abbey.toml")]
            .into_iter()
            .map(|file| toml::from_str(file).expect("a Phased Dungeon's content file parses"))
            .collect()
    })
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
    fn marshal_mcbride_stands_outside_the_abbey() {
        let mcbride = crate::mobs::content()
            .spawn
            .iter()
            .find(|s| s.entry == 197)
            .unwrap();
        assert_eq!(at(mcbride.map, mcbride.position), Phase::OpenWorld);
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
