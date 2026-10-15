//! The quest givers in the world: friendly units at the Hub that stream in and out with the
//! player's view, as the mobs do.

use benilla_protocol::messages::ObjectType;
use benilla_protocol::{EntityKind, MoveSpeeds, ObjectFields, SessionEvent};

use super::content::{givers, Giver};
use crate::fields::*;
use crate::Host;

/// `HIGHGUID_UNIT`, in a creature guid's top 16 bits.
const HIGHGUID_UNIT: u64 = 0xF130;
/// `UNIT_NPC_FLAGS`, and its questgiver bit.
const UNIT_NPC_FLAGS: u16 = 147;
const UNIT_NPC_FLAG_QUESTGIVER: u32 = 0x2;
/// `FactionTemplate.dbc` 35: friendly to every player.
const FACTION_FRIENDLY: u32 = 35;
/// A level-30 humanoid's health, for the unit frame.
const HEALTH: u32 = 1_500;

/// How far the player sees a giver, and how far before it leaves the world (as the mobs).
const VIEW_YD: f32 = crate::population::VIEW_YD;
const FORGET_YD: f32 = VIEW_YD + 10.0;

/// How near a giver the player must stand to talk to it (`INTERACTION_DISTANCE`, with slack for
/// the movement reports' lag), measured in the plane.
const TALK_YD: f32 = 10.0;

/// A giver's guid.
pub(crate) fn guid(g: &Giver) -> u64 {
    HIGHGUID_UNIT << 48 | u64::from(g.entry & 0xFF_FFFF) << 24 | u64::from(g.guid & 0xFF_FFFF)
}

/// The giver with this guid.
pub(crate) fn by_guid(npc: u64) -> Option<&'static Giver> {
    givers().iter().find(|g| guid(g) == npc)
}

/// Whether a player at `me` stands near enough to `g` to talk.
pub(crate) fn within_reach(g: &Giver, me: [f32; 3]) -> bool {
    flat_distance(g.position, me) <= TALK_YD
}

/// Which givers are in the world.
#[derive(Default)]
pub(crate) struct Givers {
    present: Vec<bool>,
}

impl Givers {
    /// Brings the givers within view of `me` into the world and lets distant ones go.
    pub(crate) fn update(
        &mut self,
        me: [f32; 3],
        map: u32,
        host: &dyn Host,
        events: &mut Vec<SessionEvent>,
    ) {
        let all = givers();
        self.present.resize(all.len(), false);
        let mut gone = Vec::new();
        for (g, present) in all.iter().zip(&mut self.present) {
            let d = if map == crate::KALIMDOR {
                flat_distance(g.position, me)
            } else {
                f32::INFINITY
            };
            if !*present && d <= VIEW_YD {
                let [x, y, z] = g.position;
                let z = host.ground_height(map, x, y, z + 2.0).unwrap_or(z);
                events.push(create(g, [x, y, z]));
                *present = true;
            } else if *present && d > FORGET_YD {
                gone.push(guid(g));
                *present = false;
            }
        }
        if !gone.is_empty() {
            events.push(SessionEvent::ObjectsRemoved(gone));
        }
    }
}

/// The giver's create: a friendly, standing unit with the questgiver flag.
fn create(g: &Giver, position: [f32; 3]) -> SessionEvent {
    let mut pairs = vec![
        (OBJECT_FIELD_TYPE, TYPE_UNIT),
        (OBJECT_FIELD_ENTRY, g.entry),
        (OBJECT_FIELD_SCALE_X, float(1.0)),
        (UNIT_FIELD_HEALTH, HEALTH),
        (UNIT_FIELD_MAXHEALTH, HEALTH),
        (UNIT_FIELD_LEVEL, u32::from(g.level)),
        (UNIT_FIELD_FACTIONTEMPLATE, FACTION_FRIENDLY),
        // Race 0, warrior, gender 0, rage.
        (UNIT_FIELD_BYTES_0, 1 << 8),
        (UNIT_FIELD_BOUNDINGRADIUS, float(0.389)),
        (UNIT_FIELD_COMBATREACH, float(1.5)),
        (UNIT_FIELD_DISPLAYID, g.display_id),
        (UNIT_FIELD_NATIVEDISPLAYID, g.display_id),
        (UNIT_NPC_FLAGS, UNIT_NPC_FLAG_QUESTGIVER),
    ];
    pairs.extend(guid_pairs(OBJECT_FIELD_GUID, guid(g)));
    SessionEvent::ObjectCreate {
        guid: guid(g),
        kind: EntityKind::Unit,
        display_id: Some(g.display_id),
        position,
        orientation: g.orientation,
        scale: 1.0,
        speeds: Some(MoveSpeeds {
            walk: 2.5,
            run: 7.0,
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

fn flat_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}
