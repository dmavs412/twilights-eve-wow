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
    /// maximum, and each aura key names a buff of the sheet that goes on the player.
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
            for key in &proc.auras {
                if let Some(def) = crate::callings::aura(key) {
                    self.grant_aura(def, c.level, events);
                }
            }
        }
    }
}
