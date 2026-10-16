//! The Hub's friendly NPCs (`content/hub.toml`): units that stand in the Crossroads, or out in a
//! Hunting Ground for a quest to find them, each opening the stock window its NPC flags name, and
//! stream in and out with the player's view as the mobs do.

use std::sync::OnceLock;

use benilla_protocol::messages::ObjectType;
use benilla_protocol::{EntityKind, MoveSpeeds, ObjectFields, SessionEvent};
use serde::Deserialize;

use crate::fields::*;
use crate::space::flat_distance;
use crate::Host;

/// `HIGHGUID_UNIT`, in a creature guid's top 16 bits.
const HIGHGUID_UNIT: u64 = 0xF130;
/// `UNIT_NPC_FLAGS`.
const UNIT_NPC_FLAGS: u16 = 147;
/// A Hub NPC's health: it is friendly, so nothing ever takes any.
const HEALTH: u32 = 1500;

#[derive(Deserialize)]
pub(crate) struct Npc {
    guid: u32,
    pub(crate) entry: u32,
    name: String,
    #[serde(default)]
    subname: String,
    level: u8,
    display_id: u32,
    faction_template: u32,
    npc_flags: u32,
    pub(crate) position: [f32; 3],
    orientation: f32,
    /// A vendor's list, in the merchant window's order.
    #[serde(default)]
    pub(crate) stock: Vec<u32>,
    /// The Token vendor: it sells the buyer's band of greens for Tokens (`crate::token_vendor`).
    #[serde(default)]
    pub(crate) tokens: bool,
    /// What its quest list, or its gossip menu, opens with.
    #[serde(default)]
    pub(crate) greeting: String,
    /// A gossip NPC's greeting's `npc_text` id, which `CMSG_NPC_TEXT_QUERY` asks for.
    #[serde(default)]
    pub(crate) text_id: Option<u32>,
}

#[derive(Deserialize)]
struct File {
    npc: Vec<Npc>,
}

pub(crate) fn npcs() -> &'static [Npc] {
    static NPCS: OnceLock<Vec<Npc>> = OnceLock::new();
    NPCS.get_or_init(|| {
        let file: File =
            toml::from_str(include_str!("../content/hub.toml")).expect("content/hub.toml parses");
        file.npc
    })
}

impl Npc {
    pub(crate) fn guid(&self) -> u64 {
        HIGHGUID_UNIT << 48
            | u64::from(self.entry & 0xFF_FFFF) << 24
            | u64::from(self.guid & 0xFF_FFFF)
    }

    /// Whether it trades through the merchant window (`UNIT_NPC_FLAG_VENDOR`).
    pub(crate) fn is_vendor(&self) -> bool {
        self.npc_flags & 0x4 != 0
    }

    /// Whether it gives quests (`UNIT_NPC_FLAG_QUESTGIVER`).
    pub(crate) fn gives_quests(&self) -> bool {
        self.npc_flags & 0x2 != 0
    }

    /// Whether it trains (`UNIT_NPC_FLAG_TRAINER`): the Warrior trainer, who sells the respec.
    pub(crate) fn trains(&self) -> bool {
        self.npc_flags & 0x10 != 0
    }

    /// Whether it repairs gear (`UNIT_NPC_FLAG_REPAIR`).
    pub(crate) fn repairs(&self) -> bool {
        self.npc_flags & 0x4000 != 0
    }

    /// The NPC's create at `position`: a friendly unit with its NPC flags.
    fn create(&self, position: [f32; 3]) -> SessionEvent {
        let mut pairs = vec![
            (OBJECT_FIELD_TYPE, TYPE_UNIT),
            (OBJECT_FIELD_ENTRY, self.entry),
            (OBJECT_FIELD_SCALE_X, float(1.0)),
            (UNIT_FIELD_HEALTH, HEALTH),
            (UNIT_FIELD_MAXHEALTH, HEALTH),
            (UNIT_FIELD_LEVEL, u32::from(self.level)),
            (UNIT_FIELD_FACTIONTEMPLATE, self.faction_template),
            // Race 0, warrior, male, mana.
            (UNIT_FIELD_BYTES_0, 1 << 8),
            (UNIT_FIELD_BOUNDINGRADIUS, float(0.389)),
            (UNIT_FIELD_COMBATREACH, float(1.5)),
            (UNIT_FIELD_DISPLAYID, self.display_id),
            (UNIT_FIELD_NATIVEDISPLAYID, self.display_id),
            (UNIT_NPC_FLAGS, self.npc_flags),
        ];
        pairs.extend(guid_pairs(OBJECT_FIELD_GUID, self.guid()));
        SessionEvent::ObjectCreate {
            guid: self.guid(),
            kind: EntityKind::Unit,
            display_id: Some(self.display_id),
            position,
            orientation: self.orientation,
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
}

/// The NPC with object guid `guid`.
pub(crate) fn find(guid: u64) -> Option<&'static Npc> {
    npcs().iter().find(|n| n.guid() == guid)
}

/// `SMSG_CREATURE_QUERY_RESPONSE` for a Hub NPC's `entry`; `None` for any other entry.
pub(crate) fn creature_name(entry: u32) -> Option<SessionEvent> {
    let npc = npcs().iter().find(|n| n.entry == entry)?;
    Some(SessionEvent::CreatureName {
        entry,
        name: Some(npc.name.clone()),
        subname: Some(npc.subname.clone()),
        // `CREATURE_TYPE_HUMANOID`.
        creature_type: Some(7),
        pet_family: 0,
        rank: 0,
        type_flags: 0,
        display_id: npc.display_id,
        civilian: false,
        racial_leader: false,
    })
}

/// Whether the player at `at` can reach `npc`.
pub(crate) fn within_reach(npc: &Npc, at: [f32; 3]) -> bool {
    crate::space::within_interaction(npc.position, at)
}

/// How far the player sees a Hub NPC, and how far before it leaves the world (as the mobs).
const VIEW_YD: f32 = crate::population::VIEW_YD;
const FORGET_YD: f32 = VIEW_YD + 10.0;

/// Which Hub NPCs are in the world.
#[derive(Default)]
pub(crate) struct InView {
    present: Vec<bool>,
}

impl InView {
    /// Brings the NPCs within view of `me` into the world, standing on the host's ground, and
    /// lets distant ones go. They stand on Kalimdor only.
    pub(crate) fn update(
        &mut self,
        me: [f32; 3],
        map: u32,
        host: &dyn Host,
        events: &mut Vec<SessionEvent>,
    ) {
        let all = npcs();
        self.present.resize(all.len(), false);
        let mut gone = Vec::new();
        for (npc, present) in all.iter().zip(&mut self.present) {
            let d = if map == crate::KALIMDOR {
                flat_distance(npc.position, me)
            } else {
                f32::INFINITY
            };
            if !*present && d <= VIEW_YD {
                let [x, y, z] = npc.position;
                let z = host.ground_height(map, x, y, z + 2.0).unwrap_or(z);
                events.push(npc.create([x, y, z]));
                *present = true;
            } else if *present && d > FORGET_YD {
                gone.push(npc.guid());
                *present = false;
            }
        }
        if !gone.is_empty() {
            events.push(SessionEvent::ObjectsRemoved(gone));
        }
    }
}
