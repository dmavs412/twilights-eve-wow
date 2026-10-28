//! The legendary and plant meals' specials (#493, ADR 0025): each a generic rule keyed by its
//! `unique.key` or `specials[].key` in `content/cooking.toml`, read from the Well Fed buff on.
//! The numbers come from the row.

use benilla_protocol::SessionEvent;

use super::{cast, InWorld, Ticked};
use crate::characters::Char;
use crate::cooking;
use crate::fields::*;
use crate::rng::Rng;
use crate::rules::school::School;

impl InWorld {
    /// Number `param` of the Well Fed special `key`, when the buff on has that special.
    fn fed_param(&self, key: &str, param: &str) -> Option<u32> {
        self.fed.as_ref()?.special(key).map(|s| s.param(param))
    }

    /// Whether the Well Fed special `key` goes off: a roll under its `chance_pct`.
    fn fed_rolls(&self, key: &str, rng: &mut Rng) -> bool {
        self.fed_param(key, "chance_pct")
            .is_some_and(|pct| rng.below(100) < pct)
    }

    /// A white swing of the player's landed on mob `i`: `extra_strike` may swing again at once
    /// (never from a swing that may not give another), and `fire_proc` may burn the target.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fed_swing_landed(
        &mut self,
        i: usize,
        from_front: bool,
        may_swing_again: bool,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        if self.mobs[i].alive() && self.fed_rolls("fire_proc", rng) {
            let target = self.mobs[i].guid;
            let damage = self.fed_param("fire_proc", "damage").unwrap_or(0);
            let spell_id = cooking::eat().buff_spell;
            events.push(SessionEvent::SpellDamageLog(cast::damage_log(
                self.guid,
                target,
                spell_id,
                damage,
                School::Fire,
                false,
            )));
            self.hurt(target, damage, c, rng, events, ticked);
        }
        if may_swing_again && self.mobs[i].alive() && self.fed_rolls("extra_strike", rng) {
            self.white_swing(i, from_front, false, false, c, rng, events, ticked);
        }
    }

    /// A melee blow of the player's landed: a `damage_stack` grows by its `pct`, up to its
    /// `max_pct`.
    pub(super) fn fed_stack_hit(&mut self) {
        if let (Some(pct), Some(max)) = (
            self.fed_param("damage_stack", "pct"),
            self.fed_param("damage_stack", "max_pct"),
        ) {
            self.damage_stack = (self.damage_stack + pct).min(max);
        }
    }

    /// The player killed a mob: `kill_heal` heals its `pct` of their max health.
    pub(super) fn fed_kill_heal(&mut self, events: &mut Vec<SessionEvent>) {
        let Some(pct) = self.fed_param("kill_heal", "pct") else {
            return;
        };
        let max = cast::buffed(&self.player.stats, &self.skills).max_health;
        let p = &mut self.player;
        if !p.alive() || p.health >= max {
            return;
        }
        p.health = (p.health + max * pct / 100).min(max);
        events.push(values(self.guid, &[(UNIT_FIELD_HEALTH, p.health)]));
    }

    /// `damage` of `school` from `from` (of `level`, or as this world's mob says, else the
    /// player's own), after `shadow_resist`: shadow damage falls by
    /// `amount / (amount + 5 x the attacker's level)` of itself, rounded down.
    pub(super) fn fed_damage_taken(
        &self,
        from: u64,
        level: Option<u8>,
        damage: u32,
        school: School,
    ) -> u32 {
        let Some(amount) = self
            .fed_param("shadow_resist", "amount")
            .filter(|_| school == School::Shadow)
        else {
            return damage;
        };
        let level = level
            .or_else(|| self.mobs.iter().find(|m| m.guid == from).map(|m| m.level()))
            .unwrap_or(self.player.stats.level);
        let resisted =
            u64::from(damage) * u64::from(amount) / (u64::from(amount) + 5 * u64::from(level));
        damage - resisted as u32
    }

    /// The player took `damage` of `school` from `from`: of a mob's physical blow,
    /// `melee_reflect` owes its `pct` back to the mob as nature damage.
    pub(super) fn fed_reflect(&mut self, from: u64, damage: u32, school: School) {
        let Some(pct) = self.fed_param("melee_reflect", "pct") else {
            return;
        };
        let back = damage * pct / 100;
        if school == School::Physical && back > 0 && self.mobs.iter().any(|m| m.guid == from) {
            self.reflected.push((from, back));
        }
    }

    /// The `melee_reflect` returns owed since the last tick land on their mobs.
    pub(super) fn pay_reflected(
        &mut self,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        for (mob, damage) in std::mem::take(&mut self.reflected) {
            if !self.mobs.iter().any(|m| m.guid == mob && m.alive()) {
                continue;
            }
            events.push(SessionEvent::SpellDamageLog(cast::damage_log(
                self.guid,
                mob,
                cooking::eat().buff_spell,
                damage,
                School::Nature,
                false,
            )));
            self.hurt(mob, damage, c, rng, events, ticked);
        }
    }

    /// `duration_ms` of a fear (`fear`) or a root on the player, shortened to `cc_half`'s
    /// percent and, for a fear, `fear_half`'s; the two multiply.
    pub(super) fn fed_cc_ms(&self, duration_ms: u32, fear: bool) -> u32 {
        let mut ms = u64::from(duration_ms);
        if let Some(pct) = self.fed_param("cc_half", "pct") {
            ms = ms * u64::from(pct) / 100;
        }
        if let Some(pct) = self.fed_param("fear_half", "pct").filter(|_| fear) {
            ms = ms * u64::from(pct) / 100;
        }
        ms as u32
    }

    /// What an idle mob's notice radius is multiplied by: `night_aggro_half`'s percent at
    /// night, else 1.
    pub(super) fn night_notice_mul(&self) -> f32 {
        match self.fed_param("night_aggro_half", "pct") {
            Some(pct) if self.night => pct as f32 / 100.0,
            _ => 1.0,
        }
    }

    /// The Well Fed out-of-combat regeneration percent: the buff's own, and `night_regen`'s
    /// `pct` more at night.
    pub(super) fn fed_regen_pct(&self) -> u32 {
        let own = self.fed.as_ref().map_or(0, |f| f.regen_pct);
        let night = self
            .fed_param("night_regen", "pct")
            .filter(|_| self.night)
            .unwrap_or(0);
        own + night
    }

    /// The player died: `revive` cancels it once. Health comes back to its `health_pct` of
    /// max, a system line names the legend whose meal it was, and the buff ends. Whether it did.
    pub(super) fn fed_revive(&mut self, c: &mut Char, events: &mut Vec<SessionEvent>) -> bool {
        let Some(pct) = self.fed_param("revive", "health_pct") else {
            return false;
        };
        let max = cast::buffed(&self.player.stats, &self.skills).max_health;
        let p = &mut self.player;
        p.health = (max * pct / 100).max(1);
        events.push(values(self.guid, &[(UNIT_FIELD_HEALTH, p.health)]));
        let legend = c
            .well_fed
            .and_then(|w| cooking::recipe_of_meal(w.meal))
            .and_then(|r| r.teacher)
            .and_then(crate::items::template)
            .map_or_else(String::new, |t| t.name.clone());
        events.push(crate::quests::system_line(format!(
            "{legend}'s meal pulls you back from death."
        )));
        c.well_fed = None;
        self.well_fed_changed(c, events);
        true
    }
}
