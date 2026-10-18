//! Procs: a talent's "when X happens, do Y" ([`crate::callings::TalentEffect::Proc`]), and a
//! buff's while it is up ([`crate::callings::AuraProc`]), rolled and paid here for every event
//! the world reports.

use benilla_protocol::messages::{SpellEnergizeLog, SpellHealLog};
use benilla_protocol::SessionEvent;

use super::{cast, InWorld, Ticked};
use crate::characters::Char;
use crate::fields::*;
use crate::rng::Rng;
use crate::rules::procs::Happened;
use crate::rules::talents::scaled;

impl InWorld {
    /// Runs every learned proc, and every proc of the player's buffs, that `happened` sets off.
    /// A heal moves health by a percent of the maximum, each aura key names a buff of the sheet
    /// that goes on the player, and damage lands on the event's target.
    pub(super) fn proc(
        &mut self,
        happened: Happened,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let mut procs = crate::talents::procs(c, &happened);
        procs.extend(
            self.skills
                .aura_procs()
                .into_iter()
                .filter_map(|(spell_id, p)| crate::talents::aura_proc(spell_id, p, &happened)),
        );
        for proc in procs {
            if !proc.while_aura.is_none_or(|key| self.wears_aura(c, key)) {
                continue;
            }
            if proc.chance < 100 && rng.below(100) >= proc.chance {
                continue;
            }
            if proc.heal_pct > 0 {
                let max = cast::buffed(&self.player.stats, &self.skills).max_health;
                let p = &mut self.player;
                if p.alive() {
                    let amount = max * proc.heal_pct / 100;
                    p.health = (p.health + amount).min(max);
                    events.push(SessionEvent::SpellHealLog(SpellHealLog {
                        target: self.guid,
                        healer: self.guid,
                        spell_id: proc.spell_id,
                        amount,
                        critical: false,
                    }));
                    events.push(values(self.guid, &[(UNIT_FIELD_HEALTH, p.health)]));
                }
            }
            let p = &mut self.player;
            if proc.energize > 0 && p.alive() {
                let kind = p.power.kind;
                if p.power.gain(kind, proc.energize) {
                    events.push(SessionEvent::SpellEnergizeLog(SpellEnergizeLog {
                        target: self.guid,
                        caster: self.guid,
                        spell_id: proc.spell_id,
                        power: kind.id(),
                        amount: proc.energize,
                    }));
                    events.push(values(self.guid, &[p.power.field()]));
                }
            }
            if let (true, Some(target)) = (proc.combo_points > 0, happened.target) {
                self.skills.combo.add(target, proc.combo_points);
                self.skills.send_combo(events);
            }
            for key in &proc.auras {
                if let Some(def) = crate::callings::aura(key) {
                    self.grant_aura(def, c.level, events);
                }
            }
            if let (true, Some(target)) = (proc.spread > 0.0, happened.target) {
                self.spread_consumed(target, proc.spread, events);
            }
            if let Some(target) = happened.target {
                let stats = cast::buffed(&self.player.stats, &self.skills);
                let swing = cast::white(&self.player.stats, &self.skills);
                let average = (swing.min_damage + swing.max_damage) / 2.0;
                let raw = proc.damage.map_or(0, |l| l.at(c.level, stats.attack_power))
                    + stats.block_value * proc.block_value_pct / 100
                    + (average * proc.weapon_pct as f32 / 100.0).round() as u32;
                let pct = proc
                    .amount_skill
                    .map_or(0, |skill| crate::talents::amount_pct(c, skill));
                let raw = scaled(raw, pct);
                // The target may have died to the blow that set the proc off.
                if raw > 0 && self.mobs.iter().any(|m| m.guid == target && m.alive()) {
                    self.bonus_hit(
                        proc.spell_id,
                        target,
                        raw,
                        proc.school,
                        c,
                        rng,
                        events,
                        ticked,
                    );
                }
            }
        }
    }

    /// Whether the player wears the aura row `key` of their Calling.
    fn wears_aura(&self, c: &Char, key: &str) -> bool {
        crate::callings::of(c.calling)
            .auras
            .iter()
            .find(|a| a.key == key)
            .is_some_and(|a| self.skills.wears(a.spell_id))
    }
}
