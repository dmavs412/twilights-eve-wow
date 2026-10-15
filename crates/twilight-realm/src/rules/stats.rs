//! What a unit's stats come to: a warrior's sheet from its base stats and level (vmangos
//! `StatSystem.cpp`), and a mob's from its unit class and level (`creature_classlevelstats`).

use crate::tables;

/// Unarmed: 1–2 damage every 2 s (`BASE_MINDAMAGE`, `BASE_MAXDAMAGE`, `BASE_ATTACK_TIME`).
const UNARMED: (f32, f32, u32) = (1.0, 2.0, 2000);

/// A warrior's derived stats: what the character sheet shows and the attack table reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WarriorStats {
    pub level: u8,
    /// `[str, agi, sta, int, spi]`.
    pub base: [u32; 5],
    pub base_health: u32,
    pub max_health: u32,
    pub attack_power: i32,
    pub crit_pct: f64,
    pub dodge_pct: f64,
    pub armor: u32,
    /// Main-hand swing damage before armour, attack power included.
    pub min_damage: f32,
    pub max_damage: f32,
    pub attack_time_ms: u32,
    pub weapon_skill: u16,
    pub defense: u16,
}

impl WarriorStats {
    /// A naked warrior of `race` (`ChrRaces.dbc` id) at `level`, on its race's base stats.
    pub fn naked(race: u8, level: u8) -> Self {
        let base = tables::at(tables::levels().warrior.race_stats.of(race), level);
        Self::from_base(level, base)
    }

    /// A naked warrior at `level` on these base stats, `[str, agi, sta, int, spi]`.
    pub fn from_base(level: u8, base: [u32; 5]) -> Self {
        let t = &tables::levels().warrior;
        let [str, agi, sta, _, _] = base;
        let base_health = tables::at(&t.base_health, level);
        // The first 20 stamina give 1 health each, the rest 10.
        let max_health = base_health + sta.min(20) + sta.saturating_sub(20) * 10;
        let attack_power = 3 * i32::from(level) + 2 * str as i32 - 20;
        let skill = 5 * u16::from(level);
        let (min, max, attack_time_ms) = UNARMED;
        let ap_damage = attack_power.max(0) as f32 / 14.0 * attack_time_ms as f32 / 1000.0;
        Self {
            level,
            base,
            base_health,
            max_health,
            attack_power,
            // Skill sits at the maximum, so its `(skill - 5L) × 0.04` terms are zero.
            crit_pct: f64::from(agi) / tables::at(&t.agi_per_crit, level),
            dodge_pct: f64::from(agi) / tables::at(&t.agi_per_dodge, level),
            armor: 2 * agi,
            min_damage: min + ap_damage,
            max_damage: max + ap_damage,
            attack_time_ms,
            weapon_skill: skill,
            defense: skill,
        }
    }
}

/// `creature_classlevelstats`'s unit classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitClass {
    Warrior,
    Paladin,
    Mage,
}

/// A mob's stats at its level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MobStats {
    pub level: u8,
    pub max_health: u32,
    pub armor: u32,
    pub min_damage: f32,
    pub max_damage: f32,
}

/// vmangos's usual `damage_variance`.
const DAMAGE_VARIANCE: f32 = 0.14;

impl MobStats {
    /// A mob of `class` at `level`, with its template's health, damage and armour multipliers
    /// (`Creature::SelectLevel`).
    pub fn new(
        class: UnitClass,
        level: u8,
        health_mul: f32,
        damage_mul: f32,
        armor_mul: f32,
    ) -> Self {
        let all = tables::mob_class_stats();
        let t = match class {
            UnitClass::Warrior => &all.warrior,
            UnitClass::Paladin => &all.paladin,
            UnitClass::Mage => &all.mage,
        };
        let avg = tables::at(&t.melee_damage_avg, level) as f32 * damage_mul;
        Self {
            level,
            max_health: (tables::at(&t.health, level) as f32 * health_mul).round() as u32,
            armor: (tables::at(&t.armor, level) as f32 * armor_mul) as u32,
            min_damage: avg * (1.0 - DAMAGE_VARIANCE),
            max_damage: avg * (1.0 + DAMAGE_VARIANCE),
        }
    }
}
