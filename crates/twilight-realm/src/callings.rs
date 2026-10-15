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
    /// What the select screen calls it (`Basic Warrior`).
    pub name: String,
    pub tier: u8,
    /// The Tier Cap: the highest level inside the Tier.
    pub cap: u8,
    /// The Tier Bonus: flat base stats on top of the race table, cumulative per Tier.
    pub tier_bonus: TierBonus,
    /// `UNIT_FIELD_BYTES_1` byte 2 while this Calling is current (17, Battle Stance).
    pub shapeshift_form: u8,
    /// The armour it may wear. Every Calling of the Warrior line wields every Warrior weapon.
    pub armour: Vec<Armour>,
    #[serde(rename = "skill")]
    pub skills: Vec<Skill>,
    /// The Talent Tree: its tabs, as the stock talent frame shows them.
    #[serde(default, rename = "talent_tab")]
    pub talent_tabs: Vec<TalentTab>,
}

/// An armour kind a Calling may wear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Armour {
    Cloth,
    Leather,
    Mail,
    Plate,
    /// Shields and bucklers.
    Shield,
}

impl Armour {
    /// The `ITEM_CLASS_ARMOR` subclasses it covers.
    pub fn subclasses(self) -> &'static [u32] {
        match self {
            Armour::Cloth => &[1],
            Armour::Leather => &[2],
            Armour::Mail => &[3],
            Armour::Plate => &[4],
            Armour::Shield => &[5, 6],
        }
    }

    /// Its `SkillLine.dbc` id, the line the skill frame lists.
    pub fn skill_line(self) -> u32 {
        match self {
            Armour::Cloth => 415,
            Armour::Leather => 414,
            Armour::Mail => 413,
            Armour::Plate => 293,
            Armour::Shield => 433,
        }
    }
}

/// A Tier Bonus: what a Calling adds to each base stat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct TierBonus {
    pub strength: u32,
    pub agility: u32,
    pub stamina: u32,
    pub intellect: u32,
    pub spirit: u32,
}

impl TierBonus {
    /// `[str, agi, sta, int, spi]`, the order of `UNIT_FIELD_STAT0..4`.
    pub fn stats(&self) -> [u32; 5] {
        [
            self.strength,
            self.agility,
            self.stamina,
            self.intellect,
            self.spirit,
        ]
    }
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
    /// An active talent's skill, which its one rank teaches: cast as any Calling skill once
    /// learned.
    #[serde(default)]
    pub skill: Option<Skill>,
    /// The spell-mod cells that make the stock client's costs and tooltips agree with the realm:
    /// one per family bit, since a talent can touch several skills (Benediction).
    #[serde(default)]
    pub client_spellmods: Vec<ClientSpellMod>,
}

impl TalentDef {
    /// A talent the realm does not model yet: no effect and no skill. It is still learnable and
    /// still counts toward its tab's tier gates.
    pub fn is_wip(&self) -> bool {
        self.effects.is_empty() && self.skill.is_none()
    }
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
    /// The Defense skill rises by this much.
    Defense { per_rank: Vec<u32> },
    /// The armour from items rises by this many percent.
    ItemArmorPct { per_rank: Vec<u32> },
    /// A melee critical strike makes the target bleed for this percent of the average swing,
    /// under `aura_spell_id`, a tick every `tick_ms` for `duration_ms`.
    CritBleed {
        aura_spell_id: u32,
        tick_ms: u32,
        duration_ms: u32,
        per_rank: Vec<u32>,
    },
    /// The physical damage the character deals rises by this percent while it wields `weapon`.
    WeaponDamagePct { weapon: Wielded, per_rank: Vec<u32> },
    /// The melee critical strike chance rises by this many percent while it wields `weapon`.
    WeaponCritPct { weapon: Wielded, per_rank: Vec<u32> },
    /// A skill's critical strike adds this percent of the crit's bonus damage (Impale).
    SkillCritDamagePct { per_rank: Vec<u32> },
    /// The skill's auras last this percent longer.
    SkillDurationPct { skill: String, per_rank: Vec<u32> },
    /// The parry chance rises by this many percent.
    ParryPct { per_rank: Vec<u32> },
    /// The block chance with a shield rises by this many percent.
    BlockPct { per_rank: Vec<u32> },
    /// A block gives 1 rage this percent of the time, logged under `spell_id`.
    BlockRagePct { spell_id: u32, per_rank: Vec<u32> },
    /// A white swing that lands gives 1 rage this percent of the time, logged under `spell_id`.
    HitRagePct { spell_id: u32, per_rank: Vec<u32> },
    /// A white swing that lands with `weapon` gives one more swing this percent of the time; the
    /// extra swing never gives another.
    ExtraSwingPct { weapon: Wielded, per_rank: Vec<u32> },
    /// A white swing that lands with `weapon` stuns the target this percent of the time, under
    /// `aura_spell_id` for `duration_ms`.
    HitStunPct {
        weapon: Wielded,
        aura_spell_id: u32,
        duration_ms: u32,
        per_rank: Vec<u32>,
    },
    /// A critical strike taken heals this percent of the maximum health over `duration_ms`, a
    /// tick every `tick_ms`, under the rank's aura of `aura_spell_ids`.
    CritTakenHealPct {
        aura_spell_ids: Vec<u32>,
        tick_ms: u32,
        duration_ms: u32,
        per_rank: Vec<u32>,
    },
    /// A critical strike taken raises the melee damage dealt by this percent, under the rank's aura
    /// of `aura_spell_ids`, for `duration_ms` or `swings` white swings.
    CritTakenDamagePct {
        aura_spell_ids: Vec<u32>,
        duration_ms: u32,
        swings: u32,
        per_rank: Vec<u32>,
    },
}

/// What a talent asks to be wielded in the main hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Wielded {
    /// Any one-handed melee weapon.
    OneHand,
    /// Any two-handed melee weapon.
    TwoHand,
    /// An axe, one- or two-handed.
    Axe,
    /// A mace, one- or two-handed.
    Mace,
    /// A sword, one- or two-handed.
    Sword,
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
            | Self::Defense { per_rank }
            | Self::ItemArmorPct { per_rank }
            | Self::CritBleed { per_rank, .. }
            | Self::WeaponDamagePct { per_rank, .. }
            | Self::WeaponCritPct { per_rank, .. }
            | Self::SkillCritDamagePct { per_rank }
            | Self::SkillDurationPct { per_rank, .. }
            | Self::ParryPct { per_rank }
            | Self::BlockPct { per_rank }
            | Self::BlockRagePct { per_rank, .. }
            | Self::HitRagePct { per_rank, .. }
            | Self::ExtraSwingPct { per_rank, .. }
            | Self::HitStunPct { per_rank, .. }
            | Self::CritTakenHealPct { per_rank, .. }
            | Self::CritTakenDamagePct { per_rank, .. } => per_rank,
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
    /// Only on a target at or below this percent of its health (Execute); 0 for any.
    #[serde(default)]
    pub requires_target_health_pct: u32,
    /// More enemies a strike also lands on, the nearest within `extra_target_radius_yd` of the
    /// target (Cleave).
    #[serde(default)]
    pub extra_targets: u32,
    #[serde(default)]
    pub extra_target_radius_yd: f32,
    /// Set on a spell borrowed from another Class: the client rewrites its row to cost
    /// `rage_cost` rage in any stance, as a Warrior spell (ADR 0003).
    #[serde(default)]
    pub borrowed: Option<Borrowed>,
    pub effects: Vec<Effect>,
}

/// How the client files a borrowed spell among the Warrior's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Borrowed {
    /// Its own `SpellFamilyFlags` bit, one no Warrior spell uses (33..=63), so a talent's
    /// [`ClientSpellMod`] can name it alone.
    pub family_bit: u8,
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
    /// The target's healing is cut by `pct`%, under the skill's spell id (Mortal Strike).
    HealingReduction { pct: u32, duration_ms: u32 },
    /// Physical damage of `base` plus `per_extra_rage` for every rage point over the cost, all of
    /// which the cast spends (Execute).
    Execute {
        base: Linear,
        per_extra_rage: Linear,
    },
    /// The caster's maximum health rises by `pct`% under `aura_spell_id`, and the health with it;
    /// what is over the old maximum goes when it ends (Last Stand).
    AuraMaxHealthPct {
        aura_spell_id: u32,
        pct: u32,
        duration_ms: u32,
    },
    /// The caster deals `pct`% more physical damage and has `armor_loss_pct`% less armour, under
    /// the skill's spell id (Death Wish).
    AuraDamagePct {
        pct: u32,
        armor_loss_pct: u32,
        duration_ms: u32,
    },
    /// The caster's next `strikes` melee strikes that land, within `duration_ms`, also strike the
    /// nearest other enemy within `radius_yd` for as much, logged under `strike_spell_id`
    /// (Sweeping Strikes).
    SweepingStrikes {
        strikes: u32,
        duration_ms: u32,
        radius_yd: f32,
        strike_spell_id: u32,
    },
    /// A Seal on the caster for `duration_ms`, under the skill's spell id. Only one Seal is ever
    /// active: a new one replaces the old.
    Seal { duration_ms: u32, seal: Seal },
    /// Consumes the caster's active Seal for its judgement on the target. With no Seal active the
    /// cast fails and costs nothing.
    Judgement,
}

/// What a Seal does while it lasts, and what judging it does.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Seal {
    /// Each landed white swing has a `procs_per_minute` chance of a holy hit of `weapon_pct`% of
    /// a weapon roll, logged under `proc_spell_id`. Judged: a holy hit of `judgement`, logged
    /// under `judgement_spell_id`.
    Command {
        procs_per_minute: f32,
        weapon_pct: u32,
        proc_spell_id: u32,
        judgement_spell_id: u32,
        judgement: Linear,
    },
    /// While it lasts: `attack_power` more (sized by level when cast), swings `haste_pct`% faster
    /// and each hit cut to match. Judged: the target takes `holy_taken_pct`% more holy damage for
    /// `judgement_duration_ms`, under the debuff `judgement_spell_id`.
    Crusader {
        attack_power: Linear,
        haste_pct: u32,
        judgement_spell_id: u32,
        judgement_duration_ms: u32,
        holy_taken_pct: u32,
    },
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
    self::calling(calling.key()).expect("every Calling has a table in content/callings.toml")
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

/// Every skill of a Calling: its own, then its active talents'.
fn every_skill(c: &'static CallingDef) -> impl Iterator<Item = &'static Skill> {
    c.skills.iter().chain(
        c.talent_tabs
            .iter()
            .flat_map(|tab| &tab.talents)
            .filter_map(|t| t.skill.as_ref()),
    )
}

/// A skill of any Calling, an active talent's included, by spell id.
pub fn skill(spell_id: u32) -> Option<&'static Skill> {
    all()
        .iter()
        .flat_map(every_skill)
        .find(|s| s.spell_id == spell_id)
}

/// Every Calling skill's spell id, active talents' included: the spells whose rank text the
/// client blanks.
pub fn skill_spell_ids() -> impl Iterator<Item = u32> {
    all().iter().flat_map(every_skill).map(|s| s.spell_id)
}

/// Every skill borrowed from another Class, once each: the spells the client rewrites to cost
/// rage (ADR 0003).
pub fn borrowed_skills() -> impl Iterator<Item = &'static Skill> {
    let mut seen = std::collections::HashSet::new();
    all()
        .iter()
        .flat_map(every_skill)
        .filter(|s| s.borrowed.is_some())
        .filter(move |s| seen.insert(s.spell_id))
}

/// Every WIP talent's rank spells, of every Calling: the spells the client names
/// "<Name> (WIP)" (ADR 0003).
pub fn wip_rank_spell_ids() -> impl Iterator<Item = u32> {
    all()
        .iter()
        .flat_map(|c| &c.talent_tabs)
        .flat_map(|tab| &tab.talents)
        .filter(|t| t.is_wip())
        .flat_map(|t| t.rank_spells.iter().copied())
}
