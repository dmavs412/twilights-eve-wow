//! The Hub's four Tier 1 trainers (`content/hub.toml`, each with a greeting) list their Line's
//! Advancement Quests in their gossip menus (they are quests: `content/quests.toml`). Their menus
//! have no lines of their own: Talent Rows change at an innkeeper (ADR 0028).

use benilla_protocol::messages::GossipOption;

use crate::hub::Npc;

/// The lines a trainer's gossip menu offers of its own: none.
pub(crate) fn options(_npc: &Npc) -> Vec<GossipOption> {
    Vec::new()
}

impl crate::Realm {
    /// `CMSG_GOSSIP_SELECT_OPTION` at a trainer: no line of his own answers.
    pub(crate) fn trainer_select(&mut self, _guid: u64, _option: u32) {}
}
