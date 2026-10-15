//! White swings: the one-roll attack table and armour (vmangos `SpellCaster::RollMeleeOutcomeAgainst`,
//! `Unit::GetUnit*Chance`, `SpellCaster::CalcArmorReducedDamage`). Chances are basis points
//! (1/100 of a percent), rolled as `0..10000`. Skill is weapon skill or defence, `5 × level` at
//! the level's maximum.

/// What an attacking player brings to the table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerOffense {
    pub level: u8,
    pub weapon_skill: u16,
    /// The character sheet's melee crit, in percent.
    pub crit_pct: f64,
}

/// What a defending player brings to the table: the character sheet's chances, in percent. A
/// zero parry or block chance means the player cannot (no Parry skill, no shield).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerDefense {
    pub level: u8,
    pub defense: u16,
    pub dodge_pct: f64,
    pub parry_pct: f64,
    pub block_pct: f64,
}

/// What a defending mob brings to the table besides its level: whether it can parry (vmangos: unless
/// `CREATURE_FLAG_EXTRA_NO_PARRY`) and block (only with a shield, and not with
/// `CREATURE_FLAG_EXTRA_NO_BLOCK`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobDefense {
    pub level: u8,
    pub parries: bool,
    pub blocks: bool,
}

/// The slices of one swing's roll, in vmangos's order; a normal hit is what is left of 10000.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HitTable {
    pub miss: u32,
    pub dodge: u32,
    pub parry: u32,
    pub glancing: u32,
    pub block: u32,
    pub crit: u32,
    pub crushing: u32,
}

/// How a swing landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Miss,
    Dodge,
    Parry,
    Glancing,
    Block,
    Crit,
    Crushing,
    Hit,
}

/// The size of the roll: `0..ROLL`.
pub const ROLL: u32 = 10_000;

impl HitTable {
    /// The outcome of `roll` (`0..10000`): the first slice whose running sum exceeds it.
    pub fn outcome(&self, roll: u32) -> Outcome {
        let slices = [
            (self.miss, Outcome::Miss),
            (self.dodge, Outcome::Dodge),
            (self.parry, Outcome::Parry),
            (self.glancing, Outcome::Glancing),
            (self.block, Outcome::Block),
            (self.crit, Outcome::Crit),
            (self.crushing, Outcome::Crushing),
        ];
        let mut sum = 0;
        for (size, outcome) in slices {
            sum += size;
            if roll < sum {
                return outcome;
            }
        }
        Outcome::Hit
    }
}

/// vmangos shrinks a mob's avoidance below level 10 by `level / 10`.
fn low_level(bp: i64, mob_level: u8) -> i64 {
    if mob_level < 10 {
        bp * i64::from(mob_level) / 10
    } else {
        bp
    }
}

/// A percentage to basis points, truncated as vmangos's `int32(chance * 100)`. The nudge keeps an
/// exact value from truncating one low through floating-point error.
fn bp(pct: f64) -> i64 {
    (pct * 100.0 + 1e-6).floor() as i64
}

fn clamp_bp(v: i64, max: i64) -> u32 {
    v.clamp(0, max) as u32
}

/// The table a player's white swing rolls on against `mob`; `from_front` when the player stands in
/// the mob's front arc, which parry and block need.
pub fn player_vs_mob(p: PlayerOffense, mob: MobDefense, from_front: bool) -> HitTable {
    let mob_level = mob.level;
    let skill = i64::from(p.weapon_skill);
    let max_skill = 5 * i64::from(p.level);
    let defense = 5 * i64::from(mob_level);
    let full_diff = skill - defense;
    let capped = skill.min(max_skill) - defense;

    let miss = if full_diff >= -10 {
        500 - 10 * full_diff
    } else {
        500 - 20 * full_diff
    };
    let dodge = 500 - 10 * full_diff;
    let parry = if !from_front || !mob.parries {
        0
    } else if capped < -10 {
        500 - 60 * capped
    } else {
        500 - 20 * capped
    };
    let glancing = (10 + (defense - skill.min(max_skill)) * 2) * 100;
    let block = if from_front && mob.blocks {
        (500 - 10 * full_diff).min(500)
    } else {
        0
    };
    let crit = if full_diff > 0 {
        bp(p.crit_pct + full_diff as f64 * 0.04)
    } else {
        bp(p.crit_pct + capped as f64 * 0.2)
    };
    HitTable {
        miss: clamp_bp(low_level(miss, mob_level), 6000),
        dodge: clamp_bp(low_level(dodge, mob_level), ROLL as i64),
        parry: clamp_bp(low_level(parry, mob_level), ROLL as i64),
        glancing: clamp_bp(glancing, 4000),
        block: clamp_bp(low_level(block, mob_level), ROLL as i64),
        crit: clamp_bp(crit, ROLL as i64),
        crushing: 0,
    }
}

/// The table a mob's swing rolls on against a player; `from_front` when the mob stands in the
/// player's front arc: from behind a player can neither dodge, parry nor block.
pub fn mob_vs_player(mob_level: u8, p: PlayerDefense, from_front: bool) -> HitTable {
    let skill = 5 * i64::from(mob_level);
    let max_defense = 5 * i64::from(p.level);
    let defense = i64::from(p.defense);
    let full_diff = skill - defense;
    let skill_diff = skill - max_defense;

    let avoid = |pct: f64| {
        if from_front && pct > 0.0 {
            bp(pct) - 4 * skill_diff
        } else {
            0
        }
    };
    let crush_margin = skill - defense.min(max_defense);
    let crushing = if crush_margin >= 15 {
        (2 * crush_margin - 15) * 100
    } else {
        0
    };
    HitTable {
        miss: clamp_bp(500 - 4 * full_diff, 6000),
        dodge: clamp_bp(avoid(p.dodge_pct), ROLL as i64),
        parry: clamp_bp(avoid(p.parry_pct), ROLL as i64),
        glancing: 0,
        block: clamp_bp(avoid(p.block_pct), ROLL as i64),
        crit: clamp_bp(500 + 4 * full_diff, ROLL as i64),
        crushing: clamp_bp(crushing, ROLL as i64),
    }
}

/// A glancing blow's damage multiplier range `[low, high]`, by how far the defence outstrips the
/// weapon skill (`Unit::GetGlancingBlowDamageMultiplier`).
pub fn glancing_range(defense: u16, weapon_skill: u16) -> (f32, f32) {
    let d = f32::from(defense) - f32::from(weapon_skill);
    (
        (1.3 - 0.05 * d).clamp(0.01, 0.91),
        (1.2 - 0.03 * d).clamp(0.2, 0.99),
    )
}

/// `damage` after the victim's `armor`, against an attacker of `attacker_level`: at most 75% off,
/// truncated, never below 1.
pub fn armor_reduced(damage: u32, armor: u32, attacker_level: u8) -> u32 {
    if damage == 0 {
        return 0;
    }
    let armor = f64::from(armor);
    let dr = (armor / (armor + 85.0 * f64::from(attacker_level) + 400.0)).clamp(0.0, 0.75);
    let d = f64::from(damage);
    ((d - d * dr) as u32).max(1)
}

/// `NOMINAL_MELEE_RANGE`: the shortest melee reach, in yards.
pub const NOMINAL_MELEE_RANGE: f32 = 5.0;

/// How far apart two units of these combat reaches can strike each other in melee
/// (`Unit::GetCombatReachToTarget` plus vmangos's 4/3 yd lean): both reaches and 4/3, never
/// under [`NOMINAL_MELEE_RANGE`].
pub fn reach(attacker_reach: f32, target_reach: f32) -> f32 {
    (attacker_reach + target_reach + 4.0 / 3.0).max(NOMINAL_MELEE_RANGE)
}
