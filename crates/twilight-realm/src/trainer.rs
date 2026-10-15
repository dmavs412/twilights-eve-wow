//! The Hub's Warrior trainer (`content/hub.toml`, with his greeting) lists his Advancement Quests in his gossip menu
//! (they are quests: `content/quests.toml`), and sells the respec through the stock flow
//! (vmangos `Player::SendTalentWipeConfirm`, `Player::ResetTalents`): his gossip's unlearn line
//! closes the menu and asks `MSG_TALENT_WIPE_CONFIRM` with the price; the reset runs when the
//! client sends the same opcode back with his guid.

use benilla_protocol::messages::{GossipOption, NpcTextBlock, NPC_TEXT_BLOCKS};
use benilla_protocol::SessionEvent;

use crate::content::KALIMDOR;
use crate::fields::values;
use crate::hub::{self, Npc};

/// The gossip line's index, which `CMSG_GOSSIP_SELECT_OPTION` echoes.
const OPTION_UNLEARN: u32 = 0;

/// The trainer, when `guid` is his: a Hub NPC that trains and has a gossip greeting.
fn trainer(guid: u64) -> Option<&'static Npc> {
    hub::find(guid).filter(|n| n.trains() && n.text_id.is_some())
}

/// A Hub NPC's gossip greeting, for `CMSG_NPC_TEXT_QUERY`; `None` for any other text.
pub(crate) fn greeting(text_id: u32) -> Option<SessionEvent> {
    let npc = hub::npcs().iter().find(|n| n.text_id == Some(text_id))?;
    let mut blocks = vec![NpcTextBlock::default(); NPC_TEXT_BLOCKS];
    blocks[0] = NpcTextBlock {
        probability: 1.0,
        male: npc.greeting.clone(),
        female: npc.greeting.clone(),
    };
    Some(SessionEvent::NpcGreeting { text_id, blocks })
}

impl crate::Realm {
    /// `CMSG_GOSSIP_HELLO` at the trainer: his menu.
    pub(crate) fn gossip_hello(&mut self, guid: u64) {
        let Some(npc) = trainer(guid).filter(|npc| self.near(npc)) else {
            return;
        };
        // His Advancement Quests ride the menu, as vmangos's gossip lists a quest giver's quests.
        let quests = self
            .world
            .as_ref()
            .and_then(|w| self.characters.get(w.guid))
            .map(|c| crate::quests::gossip_rows(c, npc.entry))
            .unwrap_or_default();
        self.events.push(SessionEvent::GossipMenu {
            npc: npc.guid(),
            text_id: npc.text_id.unwrap_or_default(),
            options: vec![GossipOption {
                index: OPTION_UNLEARN,
                icon: 0,
                coded: false,
                message: "I wish to unlearn my talents.".into(),
            }],
            quests,
        });
    }

    /// `CMSG_GOSSIP_SELECT_OPTION`: the unlearn line closes the menu and, when there is something
    /// to unlearn, names the price.
    pub(crate) fn gossip_select(&mut self, guid: u64, option: u32) {
        let Some(npc) = trainer(guid).filter(|npc| self.near(npc)) else {
            return;
        };
        if option != OPTION_UNLEARN {
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
                trainer: npc.guid(),
                cost: crate::talents::wipe_cost(c),
            });
        }
    }

    /// `MSG_TALENT_WIPE_CONFIRM` back from the client: pay, unlearn everything, save. Refused
    /// without a word when the player is out of reach, has nothing learned or cannot pay.
    pub(crate) fn talent_wipe(&mut self, guid: u64) {
        if trainer(guid).filter(|npc| self.near(npc)).is_none() {
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

    /// Whether the character in the world can reach `npc`.
    fn near(&self, npc: &Npc) -> bool {
        self.world
            .as_ref()
            .and_then(|w| self.characters.get(w.guid))
            .is_some_and(|c| c.map == KALIMDOR && hub::within_reach(npc, c.position))
    }
}
