//! The Callings (`GLOSSARY.md`) as data: each one's Tier, Tier Cap, stance and skills, parsed once
//! from `content/callings.toml`. A new Calling or skill is a new table there, not new code: the
//! world casts any skill from its effects.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::content::Calling;
use crate::rules::skills::Linear;

/// One Calling.
#[derive(Debug, Deserialize)]
pub struct CallingDef {
    /// The save's name for it (`basic_warrior`).
    pub key: String,
    pub tier: u8,
    /// The Tier Cap: the highest level inside the Tier.
    pub cap: u8,
    /// `UNIT_FIELD_BYTES_1` byte 2 while this Calling is current (17, Battle Stance).
    pub shapeshift_form: u8,
    #[serde(rename = "skill")]
    pub skills: Vec<Skill>,
    /// The Talent Tree: its tabs, as the stock talent frame shows them.
    #[serde(default, rename = "talent_tab")]
    pub talent_tabs: Vec<TalentTab>,
}

/// One page of a Talent Tree.
#[derive(Debug, Deserialize)]
pub struct TalentTab {
    /// The client catalog's tab id, unique among every Calling's tabs.
    pub id: u32,
    pub name: String,
    /// The `Interface\TalentFrame\<background>-…` art base.
    pub background: String,
    #[serde(rename = "talent")]
    pub talents: Vec<TalentDef>,
}

/// One talent: a real `Talent.dbc` row re-seated on the Calling's grid, and what it does.
#[derive(Debug, Deserialize)]
pub struct TalentDef {
    pub key: String,
    /// The `Talent.dbc` id the client's `CMSG_LEARN_TALENT` carries.
    pub talent_id: u32,
    /// 1-based; tier `n` needs `5 × (n − 1)` points spent in the tab.
    pub tier: u8,
    /// 1-based, of 4.
    pub column: u8,
    /// The spell each rank teaches; their count is the talent's ranks.
    pub rank_spells: Vec<u32>,
    #[serde(default)]
    pub prereq: Option<Prereq>,
    /// An active talent (the Lua API's `isExceptional`).
    #[serde(default)]
    pub exceptional: bool,
    /// What it does, rank by rank.
    pub effects: Vec<TalentEffect>,
    /// The spell-mod cell that makes the stock client's costs and tooltips agree with the realm.
    #[serde(default)]
    pub client_spellmod: Option<ClientSpellMod>,
}

/// A talent another needs, and the rank (1-based) it needs.
#[derive(Debug, Deserialize)]
pub struct Prereq {
    pub key: String,
    pub rank: u8,
}

/// One thing a talent does; `per_rank[r - 1]` is its size at rank `r`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TalentEffect {
    /// The skill costs this much less rage.
    SkillRageCost { skill: String, per_rank: Vec<u32> },
    /// The skill's damage and buff amounts rise by this percent.
    SkillAmountPct { skill: String, per_rank: Vec<u32> },
    /// The skill's rage gain rises by this many points.
    SkillRage { skill: String, per_rank: Vec<u32> },
    /// The skill's critical strike chance rises by this many percent.
    SkillCritPct { skill: String, per_rank: Vec<u32> },
    /// Every melee strike's critical strike chance rises by this many percent.
    MeleeCritPct { per_rank: Vec<u32> },
    /// A melee critical strike makes the target bleed for this percent of the average swing,
    /// under `aura_spell_id`, a tick every `tick_ms` for `duration_ms`.
    CritBleed {
        aura_spell_id: u32,
        tick_ms: u32,
        duration_ms: u32,
        per_rank: Vec<u32>,
    },
}

impl TalentEffect {
    /// The effect's size at each rank.
    pub fn per_rank(&self) -> &[u32] {
        match self {
            Self::SkillRageCost { per_rank, .. }
            | Self::SkillAmountPct { per_rank, .. }
            | Self::SkillRage { per_rank, .. }
            | Self::SkillCritPct { per_rank, .. }
            | Self::MeleeCritPct { per_rank }
            | Self::CritBleed { per_rank, .. } => per_rank,
        }
    }
}

/// An `SMSG_SET_FLAT_SPELL_MODIFIER` (or `_PCT_`) cell: `op` a vmangos `SpellModOp` (14 cost,
/// 8 all effects), on the skills with SpellFamilyFlags bit `family_bit`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ClientSpellMod {
    pub flat: bool,
    pub op: u8,
    pub family_bit: u8,
    /// The cell's value at each rank: tenths of rage for a cost, percent otherwise.
    pub per_rank: Vec<i32>,
}

/// One skill: a 1.12.1 spell id, what it costs, what it needs and what it does.
#[derive(Debug, Deserialize)]
pub struct Skill {
    pub key: String,
    pub spell_id: u32,
    /// Whole rage points.
    pub rage_cost: u32,
    /// Percent of base health a cast costs (Bloodrage).
    #[serde(default)]
    pub health_cost_pct_of_base_health: u32,
    pub cooldown_ms: u32,
    /// The global cooldown a cast starts; 0 starts none.
    pub gcd_ms: u32,
    pub target: Target,
    /// `[min, max]` yards to an enemy target.
    #[serde(default)]
    pub range_yd: [f32; 2],
    /// A `self_area` skill's reach.
    #[serde(default)]
    pub radius_yd: f32,
    #[serde(default)]
    pub max_targets: u32,
    /// Only out of combat (Charge).
    #[serde(default)]
    pub requires_out_of_combat: bool,
    /// Only on a target that dodged the caster in the last few seconds (Overpower).
    #[serde(default)]
    pub requires_target_dodged: bool,
    /// Holds the caster in combat this long (Bloodrage), so rage does not decay meanwhile.
    #[serde(default)]
    pub puts_in_combat_ms: u32,
    pub effects: Vec<Effect>,
}

/// Whom a skill lands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    /// The cast's unit target, a living enemy.
    Enemy,
    /// The caster.
    #[serde(rename = "self")]
    Caster,
    /// Every living enemy around the caster, within `radius_yd`, at most `max_targets`.
    SelfArea,
}

/// Which roll a weapon strike starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weapon {
    /// A white swing's roll, attack power at the weapon's speed.
    Normal,
    /// Attack power at the normalized speed.
    Normalized,
}

/// One thing a skill does.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Effect {
    /// A weapon roll plus `bonus`, against armour and the melee table.
    WeaponStrike {
        weapon: Weapon,
        bonus: Linear,
        /// Replaces the next white swing instead of striking now (Heroic Strike).
        #[serde(default)]
        on_next_swing: bool,
        /// Cannot be dodged, parried or blocked (Overpower).
        #[serde(default)]
        unavoidable: bool,
    },
    /// Flat physical damage, against armour; through the melee table when `melee_hit_table`.
    SchoolDamage {
        amount: Linear,
        #[serde(default)]
        melee_hit_table: bool,
    },
    /// A bleed on the target under the skill's own spell id: `tick` every `tick_ms`.
    PeriodicDamage {
        tick_ms: u32,
        duration_ms: u32,
        tick: Linear,
    },
    /// The caster runs to the target.
    Charge { speed_yd_s: f32 },
    /// Rage now.
    EnergizeRage { amount: Linear },
    /// Rage over time, under `aura_spell_id`.
    PeriodicRage {
        aura_spell_id: u32,
        per_tick: u32,
        tick_ms: u32,
        duration_ms: u32,
    },
    /// The target cannot act, under `aura_spell_id`.
    Stun {
        aura_spell_id: u32,
        duration_ms: u32,
    },
    /// The target swings `pct`% slower, under the skill's spell id.
    AttackSpeedSlow { pct: u32, duration_ms: u32 },
    /// The target moves `pct`% slower, under the skill's spell id.
    MovementSlow { pct: u32, duration_ms: u32 },
    /// The caster's attack power rises by `amount`, under the skill's spell id.
    AuraAttackPower { duration_ms: u32, amount: Linear },
}

#[derive(Deserialize)]
struct Callings {
    calling: Vec<CallingDef>,
}

fn all() -> &'static [CallingDef] {
    static CALLINGS: OnceLock<Callings> = OnceLock::new();
    &CALLINGS
        .get_or_init(|| {
            toml::from_str(include_str!("../content/callings.toml"))
                .expect("content/callings.toml parses")
        })
        .calling
}

/// A Calling by its save key (`basic_warrior`).
pub fn calling(key: &str) -> Option<&'static CallingDef> {
    all().iter().find(|c| c.key == key)
}

/// A current Calling's data.
pub(crate) fn of(calling: Calling) -> &'static CallingDef {
    let key = match calling {
        Calling::BasicWarrior => "basic_warrior",
    };
    self::calling(key).expect("every Calling has a table in content/callings.toml")
}

/// The first slot of the bar Battle Stance pages the main bar to (`GetBonusBarOffset` 1: Lua
/// actions 73-84).
const STANCE_BAR: u8 = 72;

/// A Calling's bar before the player arranges it: its skills in order on the stance bar, as
/// `(slot, kind << 24 | spell)` with kind 0, a spell.
pub(crate) fn default_bar(calling: Calling) -> Vec<(u8, u32)> {
    (STANCE_BAR..)
        .zip(of(calling).skills.iter().map(|s| s.spell_id))
        .collect()
}

/// A skill of any Calling, by spell id.
pub fn skill(spell_id: u32) -> Option<&'static Skill> {
    all()
        .iter()
        .flat_map(|c| &c.skills)
        .find(|s| s.spell_id == spell_id)
}

/// Every Calling skill's spell id: the spells whose rank text the client blanks.
pub fn skill_spell_ids() -> impl Iterator<Item = u32> {
    all().iter().flat_map(|c| &c.skills).map(|s| s.spell_id)
}
