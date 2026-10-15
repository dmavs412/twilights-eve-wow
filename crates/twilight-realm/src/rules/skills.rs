//! What a skill's numbers come to: the linear amounts every Calling skill is written in
//! (`data/twilights-eve/callings.json`, `formula_conventions`), and the rolls a weapon strike adds
//! them to. No ranks: the level does the growing.

use serde::Deserialize;

use super::melee::{player_vs_mob, HitTable, PlayerOffense};

/// `round(base + per_level × L + per_ap × AP)`, half away from zero, never below `min`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Linear {
    pub base: f64,
    #[serde(default)]
    pub per_level: f64,
    #[serde(default)]
    pub per_ap: f64,
    #[serde(default)]
    pub min: u32,
}

impl Linear {
    /// The amount at `level` (within the Tier) and `attack_power`.
    pub fn at(&self, level: u8, attack_power: i32) -> u32 {
        let v =
            self.base + self.per_level * f64::from(level) + self.per_ap * f64::from(attack_power);
        // `f64::round` rounds half away from zero; a negative amount is none.
        (v.round().max(0.0) as u32).max(self.min)
    }
}

/// vanilla's normalized weapon speeds (`Player::GetAPMultiplier`), in seconds: what a normalized
/// strike (Overpower, Mortal Strike) multiplies attack power by instead of the weapon's own speed.
pub const NORMALIZED_SPEED_ONE_HAND: f32 = 2.4;

/// The two rolls of a yellow (ability) strike at a mob of `mob_level`, the cmangos model that
/// `vanilla-combat-rules.md` §1.5 recommends: the table to roll avoidance on, and then, for what
/// lands, the crit chance in basis points. Abilities never glance, and a block is partial (left
/// out here), so the table holds only miss, dodge and parry; an `unavoidable` strike (Overpower)
/// only misses.
pub fn ability_table(
    p: PlayerOffense,
    mob_level: u8,
    from_front: bool,
    unavoidable: bool,
) -> (HitTable, u32) {
    let white = player_vs_mob(p, mob_level, from_front);
    let table = HitTable {
        miss: white.miss,
        dodge: if unavoidable { 0 } else { white.dodge },
        parry: if unavoidable { 0 } else { white.parry },
        ..HitTable::default()
    };
    (table, white.crit)
}

/// A weapon roll's attack-power part: `AP / 14 × speed`.
pub fn ap_damage(attack_power: i32, speed_secs: f32) -> f32 {
    attack_power.max(0) as f32 / 14.0 * speed_secs
}
