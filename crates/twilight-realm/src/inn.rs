//! The innkeeper (`content/hub.toml`, role `innkeeper`): the innkeeper's gossip binds the
//! Hearthstone and opens the innkeeper's merchant window. The bind runs the stock flow (vmangos `Player::SetBindPoint`,
//! `Spell::EffectBind`): the gossip's bind line closes the menu and asks `SMSG_BINDER_CONFIRM`;
//! the client's `CMSG_BINDER_ACTIVATE` binds the character where they stand and answers with
//! `SMSG_BINDPOINTUPDATE` and `SMSG_PLAYERBOUND`, whose "is now your home" line the client shows.

use benilla_protocol::messages::GossipOption;
use benilla_protocol::SessionEvent;

use crate::content::{CROSSROADS, KALIMDOR};
use crate::dungeon::Place;
use crate::hub::{self, Npc};

/// `AreaTable.dbc`'s The Crossroads.
const THE_CROSSROADS: u32 = 380;

// The menu's lines, by the index `CMSG_GOSSIP_SELECT_OPTION` echoes.
const OPTION_BIND: u32 = 0;
const OPTION_BROWSE: u32 = 1;
/// `GOSSIP_ICON_INTERACT_2`, the binder's, and `GOSSIP_ICON_VENDOR`.
const ICON_BINDER: u8 = 5;
const ICON_VENDOR: u8 = 1;

/// Where the Hearthstone takes a character, and the `AreaTable` id that names it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Bind {
    pub(crate) place: Place,
    pub(crate) area: u32,
}

impl Bind {
    /// The Crossroads inn's hall, where characters start: a new character's bind, and a save's
    /// that names none.
    pub(crate) fn crossroads_inn() -> Self {
        let [x, y, z, orientation] = CROSSROADS;
        Self {
            place: Place {
                map: KALIMDOR,
                position: [x, y, z],
                orientation,
            },
            area: THE_CROSSROADS,
        }
    }

    /// Northshire Abbey, a character's bind when they chose to start in Elwynn Forest.
    pub(crate) fn northshire() -> Self {
        let [x, y, z, orientation] = crate::content::NORTHSHIRE;
        Self {
            place: Place {
                map: crate::content::EASTERN_KINGDOMS,
                position: [x, y, z],
                orientation,
            },
            area: crate::content::NORTHSHIRE_VALLEY,
        }
    }

    /// `SMSG_BINDPOINTUPDATE`, which a login sends and a bind sends again.
    pub(crate) fn update(&self) -> SessionEvent {
        SessionEvent::BindPoint { area: self.area }
    }
}

/// The lines an innkeeper's gossip menu offers.
pub(crate) fn options() -> Vec<GossipOption> {
    vec![
        GossipOption {
            index: OPTION_BIND,
            icon: ICON_BINDER,
            coded: false,
            message: "Make this inn your home.".into(),
        },
        GossipOption {
            index: OPTION_BROWSE,
            icon: ICON_VENDOR,
            coded: false,
            message: "Let me browse your goods.".into(),
        },
    ]
}

/// The innkeeper, when `guid` is one.
fn innkeeper(guid: u64) -> Option<&'static Npc> {
    hub::find(guid).filter(|n| n.is_innkeeper() && n.area.is_some())
}

impl crate::Realm {
    /// A line of the innkeeper's menu: the bind line asks the stock confirm; the browse line
    /// opens his merchant window. Both close the menu.
    pub(crate) fn inn_select(&mut self, npc: &Npc, option: u32) {
        if !self.near(npc) || !matches!(option, OPTION_BIND | OPTION_BROWSE) {
            return;
        }
        self.events.push(SessionEvent::GossipComplete);
        if option == OPTION_BIND {
            self.events
                .push(SessionEvent::BinderConfirm { binder: npc.guid() });
            return;
        }
        let Some(c) = self
            .world
            .as_ref()
            .and_then(|w| self.characters.get(w.guid))
        else {
            return;
        };
        crate::vendor::list(npc.guid(), c, &mut self.events);
    }

    /// `CMSG_BINDER_ACTIVATE`: binds the character where they stand, at an innkeeper within
    /// reach, and saves. Refused without a word otherwise, as vmangos refuses it.
    pub(crate) fn binder_activate(&mut self, guid: u64) {
        let Some(npc) = innkeeper(guid).filter(|n| self.near(n)) else {
            return;
        };
        let Some(w) = &self.world else {
            return;
        };
        let me = w.guid;
        let Some(c) = self.characters.get_mut(me) else {
            return;
        };
        let bind = Bind {
            place: Place {
                map: c.map,
                position: c.position,
                orientation: c.orientation,
            },
            area: npc.area.unwrap_or(THE_CROSSROADS),
        };
        c.bind = bind;
        self.events.push(bind.update());
        self.events.push(SessionEvent::PlayerBound {
            binder: npc.guid(),
            area: bind.area,
        });
        self.save(me);
    }
}
