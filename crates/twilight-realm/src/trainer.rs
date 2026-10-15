//! The Hub's Warrior trainer: a friendly NPC by the Crossroads' inn who sells the respec through
//! the stock flow (vmangos `Player::SendTalentWipeConfirm`, `Player::ResetTalents`): his gossip's
//! unlearn line closes the menu and asks `MSG_TALENT_WIPE_CONFIRM` with the price; the reset runs
//! when the client sends the same opcode back with his guid.

use benilla_protocol::messages::{GossipOption, NpcTextBlock, ObjectType, NPC_TEXT_BLOCKS};
use benilla_protocol::{EntityKind, MoveSpeeds, ObjectFields, SessionEvent};

use crate::content::{CROSSROADS, KALIMDOR};
use crate::fields::*;
use crate::Host;

/// `creature_template` 5810, Uzzek, the Barrens' warrior quest giver: his name and model.
const ENTRY: u32 = 5810;
const DISPLAY_ID: u32 = 4391;
/// `HIGHGUID_UNIT`, the creature's entry, and a low guid no spawn uses.
const GUID: u64 = 0xF130 << 48 | (ENTRY as u64) << 24 | 0xFF_FFFF;
/// Five yards in front of where a character starts, facing them.
const POSITION: [f32; 2] = [-441.95, -2643.6];
const ORIENTATION: f32 = CROSSROADS[3] + std::f32::consts::PI;
const LEVEL: u32 = 40;
/// `FactionTemplate.dbc` 35: friendly to every race.
const FACTION_FRIENDLY: u32 = 35;
/// `UNIT_NPC_FLAG_GOSSIP | UNIT_NPC_FLAG_TRAINER`.
const NPC_FLAGS: u32 = 0x01 | 0x10;
const UNIT_NPC_FLAGS: u16 = 147;
/// Humanoid (`CreatureType.dbc`).
const HUMANOID: u32 = 7;
/// The greeting's `npc_text` id, the realm's own.
const TEXT_ID: u32 = 64_014;
const GREETING: &str = "Strength is earned, $C, and so is every scar. If the path you chose \
    no longer serves you, I can help you forget it - for a price.";
/// The gossip line's index, which `CMSG_GOSSIP_SELECT_OPTION` echoes.
const OPTION_UNLEARN: u32 = 0;
/// How close the player must stand to deal with him (vmangos `INTERACTION_DISTANCE` and
/// his reach, with room for the client's own gate).
const REACH_YD: f32 = 10.0;

/// Whether the client has been shown the trainer this session.
#[derive(Default)]
pub(crate) struct Presence {
    shown: bool,
}

impl Presence {
    /// Shows the trainer once, on the ground the host gives, when the player is on his map.
    pub(crate) fn show(&mut self, map: u32, host: &dyn Host, events: &mut Vec<SessionEvent>) {
        if self.shown || map != KALIMDOR {
            return;
        }
        self.shown = true;
        let [x, y] = POSITION;
        let z = host
            .ground_height(map, x, y, CROSSROADS[2] + 5.0)
            .unwrap_or(CROSSROADS[2]);
        events.push(create([x, y, z]));
    }
}

fn create(position: [f32; 3]) -> SessionEvent {
    let mut pairs = vec![
        (OBJECT_FIELD_TYPE, TYPE_UNIT),
        (OBJECT_FIELD_ENTRY, ENTRY),
        (OBJECT_FIELD_SCALE_X, float(1.0)),
        (UNIT_FIELD_HEALTH, 1_600),
        (UNIT_FIELD_MAXHEALTH, 1_600),
        (UNIT_FIELD_LEVEL, LEVEL),
        (UNIT_FIELD_FACTIONTEMPLATE, FACTION_FRIENDLY),
        // Race 0, warrior, male, rage.
        (UNIT_FIELD_BYTES_0, 1 << 8 | 1 << 24),
        (UNIT_FIELD_BOUNDINGRADIUS, float(0.5)),
        (UNIT_FIELD_COMBATREACH, float(1.5)),
        (UNIT_FIELD_DISPLAYID, DISPLAY_ID),
        (UNIT_FIELD_NATIVEDISPLAYID, DISPLAY_ID),
        (UNIT_NPC_FLAGS, NPC_FLAGS),
    ];
    pairs.extend(guid_pairs(OBJECT_FIELD_GUID, GUID));
    SessionEvent::ObjectCreate {
        guid: GUID,
        kind: EntityKind::Unit,
        display_id: Some(DISPLAY_ID),
        position,
        orientation: ORIENTATION,
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

/// The trainer's name, for `CMSG_CREATURE_QUERY`; `None` for any other entry.
pub(crate) fn creature_name(entry: u32) -> Option<SessionEvent> {
    (entry == ENTRY).then(|| SessionEvent::CreatureName {
        entry,
        name: Some("Uzzek".into()),
        subname: Some("Warrior Trainer".into()),
        creature_type: Some(HUMANOID),
        pet_family: 0,
        rank: 0,
        type_flags: 0,
        display_id: DISPLAY_ID,
        civilian: false,
        racial_leader: false,
    })
}

/// The greeting, for `CMSG_NPC_TEXT_QUERY`; `None` for any other text.
pub(crate) fn greeting(text_id: u32) -> Option<SessionEvent> {
    (text_id == TEXT_ID).then(|| {
        let mut blocks = vec![NpcTextBlock::default(); NPC_TEXT_BLOCKS];
        blocks[0] = NpcTextBlock {
            probability: 1.0,
            male: GREETING.into(),
            female: GREETING.into(),
        };
        SessionEvent::NpcGreeting { text_id, blocks }
    })
}

fn within_reach(at: [f32; 3]) -> bool {
    let [x, y] = POSITION;
    (at[0] - x).hypot(at[1] - y) <= REACH_YD
}

impl crate::Realm {
    /// `CMSG_GOSSIP_HELLO` at the trainer: his menu.
    pub(crate) fn gossip_hello(&mut self, guid: u64) {
        if guid != GUID || !self.near_trainer() {
            return;
        }
        self.events.push(SessionEvent::GossipMenu {
            npc: GUID,
            text_id: TEXT_ID,
            options: vec![GossipOption {
                index: OPTION_UNLEARN,
                icon: 0,
                coded: false,
                message: "I wish to unlearn my talents.".into(),
            }],
            quests: Vec::new(),
        });
    }

    /// `CMSG_GOSSIP_SELECT_OPTION`: the unlearn line closes the menu and, when there is something
    /// to unlearn, names the price.
    pub(crate) fn gossip_select(&mut self, guid: u64, option: u32) {
        if guid != GUID || option != OPTION_UNLEARN || !self.near_trainer() {
            return;
        }
        self.events.push(SessionEvent::GossipComplete);
        let Some(c) = self
            .world
            .as_ref()
            .and_then(|w| self.characters.get(w.guid))
        else {
            return;
        };
        if crate::talents::any(c) {
            self.events.push(SessionEvent::TalentWipeConfirm {
                trainer: GUID,
                cost: crate::talents::wipe_cost(c),
            });
        }
    }

    /// `MSG_TALENT_WIPE_CONFIRM` back from the client: pay, unlearn everything, save. Refused
    /// without a word when the player is out of reach, has nothing learned or cannot pay.
    pub(crate) fn talent_wipe(&mut self, trainer: u64) {
        if trainer != GUID || !self.near_trainer() {
            return;
        }
        let Some(w) = &mut self.world else {
            return;
        };
        let Some(c) = self.characters.get_mut(w.guid) else {
            return;
        };
        let cost = crate::talents::wipe_cost(c);
        if !crate::talents::any(c) || c.inventory.money < cost {
            return;
        }
        c.inventory.money -= cost;
        c.talent_wipes += 1;
        crate::talents::wipe(c, &mut self.events);
        self.events.push(values(
            c.guid,
            &[crate::equipment::money_pair(&c.inventory)],
        ));
        w.refresh_gear(c, &mut self.events);
        let guid = w.guid;
        self.save(guid);
    }

    fn near_trainer(&self) -> bool {
        self.world
            .as_ref()
            .and_then(|w| self.characters.get(w.guid))
            .is_some_and(|c| c.map == KALIMDOR && within_reach(c.position))
    }
}
