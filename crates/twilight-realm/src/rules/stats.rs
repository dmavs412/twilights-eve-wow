//! What a unit's stats come to: a warrior's sheet from its base stats and level (vmangos
//! `StatSystem.cpp`), and a mob's from its unit class and level (`creature_classlevelstats`).

use crate::tables;

/// Unarmed: 1–2 damage every 2 s (`BASE_MINDAMAGE`, `BASE_MAXDAMAGE`, `BASE_ATTACK_TIME`).
const UNARMED: (f32, f32, u32) = (1.0, 2.0, 2000);

/// A warrior's derived stats: what the character sheet shows and the attack table reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WarriorStats {
    pub level: u8,
    /// `[str, agi, sta, int, spi]`, gear included.
    pub base: [u32; 5],
    /// The part of each of `base` that gear gives: the sheet's green number.
    pub bonus: [u32; 5],
    pub base_health: u32,
    pub max_health: u32,
    /// Everything included.
    pub attack_power: i32,
    /// The flat part of `attack_power` from equip spells.
    pub attack_power_bonus: i32,
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
        Self::build(level, base, &Gear::default())
    }

    /// A warrior of `race` at `level` wearing the items `gear` (template entries; unknown ones
    /// add nothing). Item stats add to the base stats, armour to armour, and equip spells' attack
    /// power and Defense to theirs; a main-hand weapon sets the swing (vmangos
    /// `Player::_ApplyItemMods`, `UpdateDamagePhysical`).
    pub fn geared(race: u8, level: u8, gear: &[u32]) -> Self {
        Self::geared_at_tier(race, level, [0; 5], gear)
    }

    /// [`Self::geared`] with a Tier Bonus, `[str, agi, sta, int, spi]`, added to the race's base
    /// stats: it counts as base, not as the gear's green bonus.
    pub fn geared_at_tier(race: u8, level: u8, tier_bonus: [u32; 5], gear: &[u32]) -> Self {
        let race_base = tables::at(tables::levels().warrior.race_stats.of(race), level);
        let base: [u32; 5] = std::array::from_fn(|i| race_base[i] + tier_bonus[i]);
        let mut g = Gear::default();
        for t in gear.iter().filter_map(|e| crate::items::template(*e)) {
            for &[stat, value] in &t.stats {
                // `ItemModType`: 3 agility, 4 strength, 5 intellect, 6 spirit, 7 stamina.
                let i = match stat {
                    4 => 0,
                    3 => 1,
                    7 => 2,
                    5 => 3,
                    6 => 4,
                    _ => continue,
                };
                g.stats[i] += value;
            }
            g.armor += t.armor;
            g.attack_power += t.attack_power;
            g.defense += t.defense;
            let main_hand = crate::rules::gear::equip_slots(t.inventory_type)
                .first()
                .is_some_and(|s| *s == crate::rules::gear::SLOT_MAIN_HAND);
            if main_hand && t.class == crate::rules::gear::CLASS_WEAPON {
                if let Some(&[min, max, _]) = t.damage.first() {
                    g.weapon = Some((min, max, t.speed_ms));
                }
            }
        }
        Self::build(level, base, &g)
    }

    fn build(level: u8, naked: [u32; 5], gear: &Gear) -> Self {
        let t = &tables::levels().warrior;
        let bonus = gear.stats.map(|v| v.max(0) as u32);
        let base: [u32; 5] =
            std::array::from_fn(|i| (naked[i] as i32 + gear.stats[i]).max(0) as u32);
        let [str, agi, sta, _, _] = base;
        let base_health = tables::at(&t.base_health, level);
        // The first 20 stamina give 1 health each, the rest 10.
        let max_health = base_health + sta.min(20) + sta.saturating_sub(20) * 10;
        let attack_power = 3 * i32::from(level) + 2 * str as i32 - 20 + gear.attack_power;
        let skill = 5 * u16::from(level);
        let (min, max, attack_time_ms) = gear.weapon.unwrap_or(UNARMED);
        let ap_damage = attack_power.max(0) as f32 / 14.0 * attack_time_ms as f32 / 1000.0;
        Self {
            level,
            base,
            bonus,
            base_health,
            max_health,
            attack_power,
            attack_power_bonus: gear.attack_power,
            // Skill sits at the maximum, so its `(skill - 5L) × 0.04` terms are zero.
            crit_pct: f64::from(agi) / tables::at(&t.agi_per_crit, level),
            dodge_pct: f64::from(agi) / tables::at(&t.agi_per_dodge, level),
            armor: 2 * agi + gear.armor,
            min_damage: min + ap_damage,
            max_damage: max + ap_damage,
            attack_time_ms,
            weapon_skill: skill,
            defense: skill + gear.defense,
        }
    }
}

/// What a set of items adds up to.
#[derive(Default)]
struct Gear {
    /// `[str, agi, sta, int, spi]`.
    stats: [i32; 5],
    armor: u32,
    attack_power: i32,
    defense: u16,
    /// The main hand's `(min, max, speed ms)`.
    weapon: Option<(f32, f32, u32)>,
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
