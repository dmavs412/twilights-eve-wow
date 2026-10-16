//! The Hub's four Tier 1 trainers (`content/hub.toml`, each with a greeting) list their Line's
//! Advancement Quests in their gossip menus (they are quests: `content/quests.toml`). Uzzek, the
//! Vanguard Trainer, also sells the respec through the stock flow
//! (vmangos `Player::SendTalentWipeConfirm`, `Player::ResetTalents`): his gossip's unlearn line
//! closes the menu and asks `MSG_TALENT_WIPE_CONFIRM` with the price; the reset runs when the
//! client sends the same opcode back with his guid.

use benilla_protocol::messages::GossipOption;
use benilla_protocol::SessionEvent;

use crate::fields::values;
use crate::hub::{self, Npc};

/// The gossip line's index, which `CMSG_GOSSIP_SELECT_OPTION` echoes.
const OPTION_UNLEARN: u32 = 0;

/// The trainer who sells the respec, when `guid` is his: a Hub NPC that sells it and has a gossip
/// greeting.
fn trainer(guid: u64) -> Option<&'static Npc> {
    hub::find(guid).filter(|n| n.sells_respec() && n.text_id.is_some())
}

/// The lines a Hub NPC's gossip menu offers: the unlearn line, at the trainer who sells the respec.
pub(crate) fn options(npc: &Npc) -> Vec<GossipOption> {
    if !npc.sells_respec() {
        return Vec::new();
    }
    vec![GossipOption {
        index: OPTION_UNLEARN,
        icon: 0,
        coded: false,
        message: "I wish to unlearn my talents.".into(),
    }]
}

impl crate::Realm {
    /// `CMSG_GOSSIP_SELECT_OPTION`: the unlearn line closes the menu and, when there is something
    /// to unlearn, names the price.
    pub(crate) fn trainer_select(&mut self, guid: u64, option: u32) {
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
}
