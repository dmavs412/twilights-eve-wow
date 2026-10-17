//! **Discovery**, in the world: the first time the character stands in an area, its explore bit
//! lifts the fog on the map, the client announces it, and it pays an eighth of a level's XP times
//! the Tier XP Rate (`Player::CheckAreaExploreAndOutdoor`). Once per character, never again.

use benilla_protocol::messages::{ExplorationXp, XpGain};
use benilla_protocol::SessionEvent;

use super::{InWorld, Ticked};
use crate::characters::Char;
use crate::fields::values;
use crate::rules::xp;
use crate::Host;

impl InWorld {
    /// Discovers the area under the player, if it is one the character has never stood in.
    pub(super) fn discover(
        &mut self,
        host: &dyn Host,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let [x, y, _] = c.position;
        let Some(area) = host.area_at(self.map, x, y).and_then(crate::areas::get) else {
            return;
        };
        let Err(at) = c.discovered.binary_search(&area.id) else {
            return;
        };
        c.discovered.insert(at, area.id);
        ticked.discovered = true;
        crate::quests::refresh(c, events);
        events.push(values(
            self.guid,
            &[crate::areas::explored_pair(&c.discovered, area)],
        ));
        let gained = self.scaled_xp(c, xp::discovery_xp(c.level));
        if gained > 0 {
            events.push(SessionEvent::XpGain(XpGain {
                victim: 0,
                total: gained,
                base: gained,
                kill: false,
            }));
            ticked.leveled |= self.give_xp(c, gained, events);
        }
        // Sent even when it paid nothing, at the Tier Cap.
        events.push(SessionEvent::ExplorationXp(ExplorationXp {
            area_id: area.id,
            xp: gained,
        }));
    }
}
