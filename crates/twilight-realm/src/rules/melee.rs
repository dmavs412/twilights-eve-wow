//! White swings: the one-roll attack table and armour. The table is flat (ADR 0012): the chances
//! come from `content/combat.toml` and neither the level gap nor a weapon skill moves them. Chances
//! are basis points (1/100 of a percent), rolled as `0..10000`.

use std::sync::OnceLock;

use serde::Deserialize;

/// `content/combat.toml`, in percent.
#[derive(Deserialize)]
struct Combat {
    hit_table: Chances,
}

#[derive(Deserialize)]
struct Chances {
    miss: f64,
    mob_dodge: f64,
    mob_parry: f64,
    mob_block: f64,
    mob_crit: f64,
    dual_wield_miss: f64,
}

fn chances() -> &'static Chances {
    static COMBAT: OnceLock<Combat> = OnceLock::new();
    &COMBAT
        .get_or_init(|| {
            toml::from_str(include_str!("../../content/combat.toml"))
                .expect("content/combat.toml parses")
        })
        .hit_table
}

/// What an attacking player brings to the table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerOffense {
    /// The character sheet's melee crit, in percent.
    pub crit_pct: f64,
}

/// What a defending player brings to the table: the character sheet's chances, in percent. A
/// zero parry or block chance means the player cannot (no Parry skill, no shield).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerDefense {
    pub dodge_pct: f64,
    pub parry_pct: f64,
    pub block_pct: f64,
}

/// What a defending mob brings to the table: whether it can parry (unless
/// `CREATURE_FLAG_EXTRA_NO_PARRY`) and block (only with a shield, and not with
/// `CREATURE_FLAG_EXTRA_NO_BLOCK`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobDefense {
    pub parries: bool,
    pub blocks: bool,
}

/// The slices of one swing's roll, in order; a normal hit is what is left of 10000.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HitTable {
    pub miss: u32,
    pub dodge: u32,
    pub parry: u32,
    pub block: u32,
    pub crit: u32,
}

/// How a swing landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Miss,
    Dodge,
    Parry,
    Block,
    Crit,
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
            (self.block, Outcome::Block),
            (self.crit, Outcome::Crit),
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

/// A percentage to basis points, truncated. The nudge keeps an exact value from truncating one
/// low through floating-point error.
fn bp(pct: f64) -> u32 {
    ((pct * 100.0 + 1e-6).floor() as i64).clamp(0, i64::from(ROLL)) as u32
}

/// What two weapons add to the miss of every white swing, in basis points.
pub fn dual_wield_miss_bp() -> u32 {
    bp(chances().dual_wield_miss)
}

/// The table a player's white swing rolls on against a mob; `from_front` when the player stands in
/// the mob's front arc, which parry and block need.
pub fn player_vs_mob(p: PlayerOffense, mob: MobDefense, from_front: bool) -> HitTable {
    let c = chances();
    HitTable {
        miss: bp(c.miss),
        dodge: bp(c.mob_dodge),
        parry: if from_front && mob.parries {
            bp(c.mob_parry)
        } else {
            0
        },
        block: if from_front && mob.blocks {
            bp(c.mob_block)
        } else {
            0
        },
        crit: bp(p.crit_pct),
    }
}

/// The table a mob's swing rolls on against a player; `from_front` when the mob stands in the
/// player's front arc: from behind a player can neither dodge, parry nor block.
pub fn mob_vs_player(p: PlayerDefense, from_front: bool) -> HitTable {
    let c = chances();
    let avoid = |pct: f64| if from_front { bp(pct) } else { 0 };
    HitTable {
        miss: bp(c.miss),
        dodge: avoid(p.dodge_pct),
        parry: avoid(p.parry_pct),
        block: avoid(p.block_pct),
        crit: bp(c.mob_crit),
    }
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
