//! Death and the way back (spec #1, "Death"): a killed player pays a tenth of their money and of
//! each worn item's durability, lies dead until Release Spirit, and then stands at the respawn
//! point at full health. There is no ghost run and no corpse to reclaim.

use benilla_protocol::SessionEvent;

use super::power::Power;
use super::InWorld;
use crate::characters::Char;
use crate::content::{CROSSROADS, EASTERN_KINGDOMS, KALIMDOR, NORTHSHIRE};
use crate::equipment;
use crate::fields::*;
use crate::rules::durability;
use crate::rules::power::PowerKind;
use crate::wear;

/// Where a released player comes back. The Hub's is the default; a Dungeon sets its entrance's
/// while the player is inside it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Respawn {
    pub(crate) map: u32,
    pub(crate) position: [f32; 3],
    pub(crate) orientation: f32,
}

impl Respawn {
    /// The Crossroads, where characters first appear.
    pub(crate) const HUB: Self = Self {
        map: KALIMDOR,
        position: [CROSSROADS[0], CROSSROADS[1], CROSSROADS[2]],
        orientation: CROSSROADS[3],
    };

    /// Northshire Abbey, where a released player on Eastern Kingdoms comes back.
    pub(crate) const NORTHSHIRE: Self = Self {
        map: EASTERN_KINGDOMS,
        position: [NORTHSHIRE[0], NORTHSHIRE[1], NORTHSHIRE[2]],
        orientation: NORTHSHIRE[3],
    };

    /// The open world's respawn point on `map`: Northshire on Eastern Kingdoms, the Hub
    /// elsewhere.
    pub(crate) fn home(map: u32) -> Self {
        if map == EASTERN_KINGDOMS {
            Self::NORTHSHIRE
        } else {
            Self::HUB
        }
    }
}

impl InWorld {
    /// Whether the player lies dead, waiting to release.
    pub(crate) fn dead(&self) -> bool {
        self.player.dead
    }

    /// The player was killed: the fight ends and the toll is taken, once
    /// (`Player::DurabilityLossAll`, `SMSG_DURABILITY_DAMAGE_DEATH`; the money is the spec's).
    pub(super) fn die(&mut self, c: &mut Char, events: &mut Vec<SessionEvent>) {
        let p = &mut self.player;
        p.dead = true;
        // Death empties a rage bar; mana and energy keep what they held.
        p.power.take_all(PowerKind::Rage);
        if let Some(victim) = p.attacking.take() {
            events.push(SessionEvent::AttackStop {
                attacker: self.guid,
                victim,
            });
        }
        let lost = durability::death_money_loss(c.inventory.money);
        c.inventory.money -= lost;
        events.push(values(
            self.guid,
            &[equipment::money_pair(&c.inventory), (UNIT_FIELD_POWER2, 0)],
        ));
        // Killed by their own Bounty Hunter, they are Marked.
        self.marked_by_death(c, events);
        // Death takes every aura off the player, a creature's curse with the rest; the Marked
        // debuff comes back after release.
        self.auras_die_with(self.guid, events);
        self.marked_shown = false;
        // Notoriety falls to nothing, so the Heat aura is rightly gone with the rest.
        c.notoriety = 0.0;
        self.heat_shown = 0;
        c.bounty_rank = 0;
        let broke = wear::on_death(c, events);
        events.push(SessionEvent::DurabilityDamageDeath);
        if broke {
            self.refresh_gear(c, events);
        }
    }

    /// `CMSG_REPOP_REQUEST`: the dead player stands up at the respawn point at full health.
    /// Whether they were dead, so the character must be saved where they now stand.
    pub(crate) fn release(&mut self, c: &mut Char, events: &mut Vec<SessionEvent>) -> bool {
        if !self.player.dead {
            return false;
        }
        let to = self.respawn;
        let p = &mut self.player;
        p.dead = false;
        p.health = p.stats.max_health;
        p.power = Power::fresh(&p.stats, crate::callings::of(c.calling).power);
        p.swing_timer = 0.0;
        p.off_swing_timer = 0.0;
        p.refused = None;
        let health = p.health;
        // Another map is a worldport, after which the player is created again at full health.
        self.move_to(
            c,
            crate::dungeon::Place {
                map: to.map,
                position: to.position,
                orientation: to.orientation,
            },
            events,
        );
        if !self.transferring() {
            events.push(values(self.guid, &[(UNIT_FIELD_HEALTH, health)]));
        }
        true
    }
}
