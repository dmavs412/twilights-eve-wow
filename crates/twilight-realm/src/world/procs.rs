//! Procs: a talent's "when X happens, do Y" ([`crate::callings::TalentEffect::Proc`]), rolled
//! and paid here for every event the world reports.

use benilla_protocol::SessionEvent;

use super::{cast, InWorld};
use crate::characters::Char;
use crate::fields::*;
use crate::rng::Rng;
use crate::rules::procs::Happened;

impl InWorld {
    /// Runs every learned proc that `happened` sets off. A heal moves health by a percent of the
    /// maximum. Aura keys name auras the realm applies once it has them (spec #50, ticket #60).
    pub(super) fn proc(
        &mut self,
        happened: Happened,
        c: &Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) {
        for proc in crate::talents::procs(c, &happened) {
            if proc.chance < 100 && rng.below(100) >= proc.chance {
                continue;
            }
            if proc.heal_pct > 0 {
                let max = cast::buffed(&self.player.stats, &self.skills).max_health;
                let p = &mut self.player;
                if p.alive() {
                    p.health = (p.health + max * proc.heal_pct / 100).min(max);
                    events.push(values(self.guid, &[(UNIT_FIELD_HEALTH, p.health)]));
                }
            }
        }
    }
}
