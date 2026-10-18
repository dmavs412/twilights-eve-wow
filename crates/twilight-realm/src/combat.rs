//! One white swing, either way: roll the damage, take armour off, roll the attack table and
//! apply its outcome (vmangos `Unit::CalculateMeleeDamage`), as the report a client reads.

use benilla_protocol::messages::AttackerState;

use crate::mobs::Mob;
use crate::rng::Rng;
use crate::rules::melee::{
    armor_reduced, glancing_range, mob_vs_player, player_vs_mob, Outcome, PlayerDefense,
    PlayerOffense, ROLL,
};
use crate::rules::stats::{CharacterStats, DUAL_WIELD_MISS_BP};

// `HitInfo` bits (vmangos `Unit.h`), as the client reads them.
const HITINFO_AFFECTS_VICTIM: u32 = 0x2;
const HITINFO_LEFTSWING: u32 = 0x4;
const HITINFO_MISS: u32 = 0x10;
const HITINFO_CRITICALHIT: u32 = 0x80;
const HITINFO_GLANCING: u32 = 0x4000;
const HITINFO_CRUSHING: u32 = 0x8000;

// `VictimState`.
const VICTIMSTATE_UNAFFECTED: u32 = 0;
const VICTIMSTATE_NORMAL: u32 = 1;
const VICTIMSTATE_DODGE: u32 = 2;
const VICTIMSTATE_PARRY: u32 = 3;
const VICTIMSTATE_BLOCKS: u32 = 5;

/// How one swing went.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Swing {
    pub(crate) outcome: Outcome,
    /// What the victim loses.
    pub(crate) damage: u32,
    pub(crate) blocked: u32,
    /// The damage after armour had the swing landed: a dodged or parried swing still earns
    /// rage from it.
    pub(crate) would_be: u32,
    /// Struck with the off-hand weapon (`HITINFO_LEFTSWING`).
    pub(crate) off_hand: bool,
}

impl Swing {
    pub(crate) fn report(&self, attacker: u64, victim: u64) -> AttackerState {
        let (hit_info, victim_state) = match self.outcome {
            Outcome::Miss => (HITINFO_MISS, VICTIMSTATE_UNAFFECTED),
            Outcome::Dodge => (0, VICTIMSTATE_DODGE),
            Outcome::Parry => (0, VICTIMSTATE_PARRY),
            Outcome::Block if self.damage == 0 => (0, VICTIMSTATE_BLOCKS),
            Outcome::Block | Outcome::Hit => (HITINFO_AFFECTS_VICTIM, VICTIMSTATE_NORMAL),
            Outcome::Glancing => (
                HITINFO_AFFECTS_VICTIM | HITINFO_GLANCING,
                VICTIMSTATE_NORMAL,
            ),
            Outcome::Crit => (
                HITINFO_AFFECTS_VICTIM | HITINFO_CRITICALHIT,
                VICTIMSTATE_NORMAL,
            ),
            Outcome::Crushing => (
                HITINFO_AFFECTS_VICTIM | HITINFO_CRUSHING,
                VICTIMSTATE_NORMAL,
            ),
        };
        AttackerState {
            attacker,
            victim,
            hit_info: if self.off_hand {
                hit_info | HITINFO_LEFTSWING
            } else {
                hit_info
            },
            damage: self.damage,
            victim_state,
            absorb: 0,
            resist: 0,
            blocked: self.blocked,
            melee_spell_id: 0,
            school: 0,
        }
    }
}

/// The player's swing at `mob`; `from_front` when the player stands in the mob's front arc;
/// `opener`: the first blow from stealth, which skips the attack table and is a critical strike.
pub(crate) fn player_swing(
    p: &CharacterStats,
    mob: &Mob,
    from_front: bool,
    opener: bool,
    rng: &mut Rng,
) -> Swing {
    let raw = rng.urand(p.min_damage as u32, p.max_damage as u32);
    let armored = armor_reduced(raw, mob.stats.armor.saturating_sub(p.armor_pen), p.level);
    let table = player_vs_mob(
        PlayerOffense {
            level: p.level,
            weapon_skill: p.weapon_skill,
            crit_pct: p.crit_pct,
        },
        mob.defense(),
        from_front,
    );
    let mut table = table;
    // Two weapons worn: every white swing, either hand, misses 19% more.
    if p.off_hand.is_some() {
        table.miss = (table.miss + DUAL_WIELD_MISS_BP).min(6000);
    }
    let outcome = if opener {
        Outcome::Crit
    } else {
        table.outcome(rng.below(ROLL))
    };
    let mob_defense = 5 * u16::from(mob.level());
    // `Creature::GetShieldBlockValue`: level / 2 + str / 20, the strength term 1 at these levels.
    let block_value = u32::from(mob.level()) / 2 + 1;
    land(
        outcome,
        armored,
        p.crit_damage_pct,
        block_value,
        rng,
        || glancing_range(mob_defense, p.weapon_skill),
    )
}

/// `mob`'s swing at the player; `from_front` when the mob stands in the player's front arc.
/// `miss_pct` more points of misses (a hit-chance debuff on the mob, Noxious Vapors) come
/// first, rolled only when there are any.
pub(crate) fn mob_swing(
    mob: &Mob,
    p: &CharacterStats,
    from_front: bool,
    miss_pct: u32,
    rng: &mut Rng,
) -> Swing {
    let raw = rng.urand(mob.stats.min_damage as u32, mob.stats.max_damage as u32);
    let armored = armor_reduced(raw, p.armor, mob.level());
    let table = mob_vs_player(
        mob.level(),
        PlayerDefense {
            level: p.level,
            defense: p.defense,
            dodge_pct: p.dodge_pct,
            parry_pct: p.parry_pct,
            block_pct: p.block_pct,
        },
        from_front,
    );
    let outcome = if miss_pct > 0 && rng.below(ROLL) < miss_pct * 100 {
        Outcome::Miss
    } else {
        table.outcome(rng.below(ROLL))
    };
    land(outcome, armored, 0, p.block_value, rng, || (1.0, 1.0))
}

fn land(
    outcome: Outcome,
    armored: u32,
    crit_damage_pct: u32,
    block_value: u32,
    rng: &mut Rng,
    glancing: impl FnOnce() -> (f32, f32),
) -> Swing {
    let (damage, blocked) = match outcome {
        Outcome::Miss | Outcome::Dodge | Outcome::Parry => (0, 0),
        Outcome::Hit => (armored, 0),
        Outcome::Crit => (
            crate::rules::talents::crit_damage(armored, crit_damage_pct),
            0,
        ),
        Outcome::Crushing => (armored + armored / 2, 0),
        Outcome::Glancing => {
            let (lo, hi) = glancing();
            ((armored as f32 * rng.frand(lo, hi)) as u32, 0)
        }
        Outcome::Block => {
            let blocked = block_value.min(armored);
            (armored - blocked, blocked)
        }
    };
    Swing {
        outcome,
        damage,
        blocked,
        would_be: armored,
        off_hand: false,
    }
}
