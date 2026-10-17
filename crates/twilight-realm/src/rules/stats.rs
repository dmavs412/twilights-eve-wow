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
    /// The percent a crit's whole damage grows by (talents): 30 makes a melee crit 200% x 1.3 = 260%.
    pub crit_damage_pct: u32,
    pub dodge_pct: f64,
    /// 0 without the Parry skill, which the slice's warriors do not train; talents add to it.
    pub parry_pct: f64,
    /// 0 without a shield.
    pub block_pct: f64,
    /// What a block stops; 0 without a shield.
    pub block_value: u32,
    /// The worn shield's own block value, which Strength adds to; `None` without a shield.
    pub shield_block: Option<u32>,
    pub armor: u32,
    /// Main-hand swing damage before armour, attack power included.
    pub min_damage: f32,
    pub max_damage: f32,
    pub attack_time_ms: u32,
    pub weapon_skill: u16,
    pub defense: u16,
}

/// The health Stamina gives: the first 20 points 1 each, the rest 10.
fn stamina_health(sta: u32) -> u32 {
    sta.min(20) + sta.saturating_sub(20) * 10
}

impl WarriorStats {
    /// Raises stat `i` (`[str, agi, sta, int, spi]`) by `delta`, and what it feeds with it:
    /// Strength attack power and damage, Agility crit, dodge and armour, Stamina max health.
    pub fn raise_stat(&mut self, i: usize, delta: u32) {
        let old = self.base[i];
        self.base[i] += delta;
        let t = &tables::levels().warrior;
        match i {
            0 => {
                self.attack_power += 2 * delta as i32;
                let ap = 2.0 * delta as f32 / 14.0 * self.attack_time_ms as f32 / 1000.0;
                self.min_damage += ap;
                self.max_damage += ap;
                if let Some(block) = self.shield_block {
                    self.block_value = block_value(block, self.base[0]);
                }
            }
            1 => {
                self.crit_pct += f64::from(delta) / tables::at(&t.agi_per_crit, self.level);
                self.dodge_pct += f64::from(delta) / tables::at(&t.agi_per_dodge, self.level);
                self.armor += 2 * delta;
            }
            2 => self.max_health += stamina_health(old + delta) - stamina_health(old),
            _ => {}
        }
    }

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
            if t.class == crate::rules::gear::CLASS_ARMOR && t.subclass == SUBCLASS_SHIELD {
                g.shield_block = Some(t.block);
            }
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
        let max_health = base_health + stamina_health(sta);
        let attack_power = 3 * i32::from(level) + 2 * str as i32 - 20 + gear.attack_power;
        let skill = 5 * u16::from(level);
        let defense = skill + gear.defense;
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
            crit_damage_pct: 0,
            dodge_pct: f64::from(agi) / tables::at(&t.agi_per_dodge, level),
            parry_pct: 0.0,
            block_pct: gear.shield_block.map_or(0.0, |_| block_pct(defense, level)),
            block_value: gear.shield_block.map_or(0, |block| block_value(block, str)),
            shield_block: gear.shield_block,
            armor: 2 * agi + gear.armor,
            min_damage: min + ap_damage,
            max_damage: max + ap_damage,
            attack_time_ms,
            weapon_skill: skill,
            defense,
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
    /// The shield's block value, when one is worn.
    shield_block: Option<u32>,
}

/// `ITEM_SUBCLASS_ARMOR_SHIELD`.
const SUBCLASS_SHIELD: u32 = 6;

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

/// A warrior's block chance with a shield, in percent: 5, and 0.04 a point of Defense over the
/// level's maximum (vmangos `Player::UpdateBlockPercentage`).
pub fn block_pct(defense: u16, level: u8) -> f64 {
    5.0 + (f64::from(defense) - 5.0 * f64::from(level)) * 0.04
}

/// What a block stops: the shield's block value, strength / 20, less 1, never below 0 (vmangos
/// `Player::GetShieldBlockValue`).
pub fn block_value(shield_block: u32, strength: u32) -> u32 {
    (f64::from(shield_block) + f64::from(strength) / 20.0 - 1.0).max(0.0) as u32
}
