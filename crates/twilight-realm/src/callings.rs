//! The Callings (`GLOSSARY.md`) as data: each one's Tier, Tier Cap, stance and skills, built once.
//! A Calling the master CSV defines comes from the generated `server_spells.json`
//! (`generated.rs`); the others from `content/callings.toml`. A new Calling or skill is new data,
//! not new code: the world casts any skill from its effects.

use std::sync::{OnceLock, RwLock};

use serde::Deserialize;

use crate::content::Calling;
use crate::rules::power::PowerKind;
use crate::rules::school::School;
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
    /// The power its bar runs on (ADR 0011).
    pub power: PowerKind,
    /// The vanilla class table its per-level stats, attack power and mana regen read, and the
    /// class byte the wire still carries.
    pub stat_profile: StatProfile,
    /// The vanilla class whose `AllowableClass` bit an item it equips must carry; the Warrior's
    /// unless it says.
    #[serde(default = "warrior")]
    pub item_class: StatProfile,
    /// Percent of the profile's base health and base mana it has (100 is the vanilla class's).
    #[serde(default = "full")]
    pub health_pct: u32,
    #[serde(default = "full")]
    pub mana_pct: u32,
    /// Energy gained every regen tick (`callings.csv`); none means the vanilla
    /// [`power::ENERGY_PER_TICK`]. Only an energy Calling sets it.
    #[serde(default)]
    pub energy_per_tick: Option<u32>,
    /// The Tier Bonus: flat base stats on top of the race table, cumulative per Tier.
    pub tier_bonus: TierBonus,
    /// `UNIT_FIELD_BYTES_1` byte 2 while this Calling is current (17, Battle Stance).
    pub shapeshift_form: u8,
    /// The armour it may wear.
    pub armour: Vec<Armour>,
    /// The weapons it may wield, and whether it dual wields; every Warrior weapon unless it says.
    #[serde(default = "WeaponKind::warrior")]
    pub weapons: Vec<WeaponKind>,
    /// Its icon and its spell book tab's name (`callings.csv`); empty for a TOML Calling.
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub spellbook: String,
    #[serde(rename = "skill")]
    pub skills: Vec<Skill>,
    /// The Talent Tree: its tabs, as the stock talent frame shows them.
    #[serde(default, rename = "talent_tab")]
    pub talent_tabs: Vec<TalentTab>,
    /// The buffs its skills and talents grant by key (a `proc`'s `apply_aura`).
    #[serde(default, rename = "aura")]
    pub auras: Vec<AuraDef>,
    /// Hidden abilities (`client_visible=no` rows, found by their realm spell id) and visible skills
    /// kept off the spell book (`spellbook=no`, found by key). Neither is on the bar, in the spell
    /// book or at the trainer.
    #[serde(default)]
    pub hidden_skills: Vec<Skill>,
}

fn full() -> u32 {
    100
}

fn warrior() -> StatProfile {
    StatProfile::Warrior
}

impl CallingDef {
    /// Energy this Calling gains every regen tick.
    pub fn energy_regen(&self) -> u32 {
        self.energy_per_tick
            .unwrap_or(crate::rules::power::ENERGY_PER_TICK)
    }

    /// `ChrClasses.dbc` id of its stat profile: the table the rules read, and the class byte the
    /// wire carries until the client reads the Calling (#207).
    pub fn class(&self) -> u8 {
        self.stat_profile.class()
    }

    /// Whether it wears a one-hander in the off hand too.
    pub fn dual_wields(&self) -> bool {
        self.weapons.contains(&WeaponKind::DualWield)
    }

    /// The skill or hidden ability with this key.
    pub fn skill_by_key(&self, key: &str) -> Option<&Skill> {
        self.skills
            .iter()
            .chain(&self.hidden_skills)
            .find(|s| s.key == key)
    }

    /// The hidden ability with this `realm_spell_id`.
    pub fn hidden_skill(&self, realm_spell_id: u32) -> Option<&Skill> {
        self.hidden_skills
            .iter()
            .find(|s| s.spell_id == realm_spell_id)
    }
}

/// A buff a skill or talent grants: what it does, for how long, and how many uses it has.
#[derive(Debug, Clone, Deserialize)]
pub struct AuraDef {
    pub key: String,
    /// The spell the client shows the buff as.
    pub spell_id: u32,
    pub duration_ms: u32,
    /// Uses before it is spent; 0 for no limit.
    #[serde(default)]
    pub charges: u32,
    pub effects: Vec<AuraEffect>,
    /// The aura-only columns of its row. Loaded, not enforced yet.
    #[serde(default)]
    pub row: AuraRow,
}

/// The aura-only columns of the master CSV (spec #63), as the build resolved them.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct AuraRow {
    /// `Some(false)` for a `client_visible=no` row, whose `spell_id` is its realm spell id.
    pub client_visible: Option<bool>,
    pub school: Option<String>,
    pub prevention: Option<String>,
    pub max_stacks: u32,
    pub stack_rule: Option<String>,
    pub unique_per: Option<String>,
    pub exclusive_group: Option<String>,
    pub group_scope: Option<String>,
    pub dispel_type: Option<String>,
    pub mechanic: Option<String>,
    pub aura_positive: bool,
    pub aura_hidden: bool,
    pub cancelable: bool,
    pub persist_death: bool,
    pub aura_description: Option<String>,
    /// What ends it early: `damage`, `damage_threshold` and the like.
    pub break_on: serde_json::Map<String, serde_json::Value>,
    /// The shapeshift form it grants.
    pub grants_form: Option<u32>,
}

impl AuraRow {
    /// A `client_visible=no` row: the client never sees its spell id.
    pub fn is_hidden(&self) -> bool {
        self.client_visible == Some(false)
    }
}

/// The gate columns of a skill or talent row: what the cast needs. Loaded, not enforced yet.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct SpellGates {
    pub school: Option<String>,
    /// What stops the cast: `pacify`, `silence`.
    pub prevention: Option<String>,
    /// Weapon names as written in `requires_weapon`.
    pub requires_weapon: Vec<String>,
    pub cooldown_category: Option<String>,
    pub category_cooldown_ms: u32,
    pub requires_caster_state: Option<String>,
    pub requires_target_state: Option<String>,
    /// The realm-only `requires` cell, `key` to value (`combat` = `out`).
    pub requires: serde_json::Map<String, serde_json::Value>,
}

/// What a cast does on the way: cost kind and cast shape. The realm runs `cast_time_ms`; the rest
/// is loaded, not enforced yet.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct CastShape {
    /// How long the cast bar runs (`client_cast_time_ms`, for a `cast`); 0 for instant.
    pub cast_time_ms: u32,
    /// How long a `channel` lasts (`client_duration_ms`); its effects run every
    /// `channel_tick_ms` of it.
    pub channel_ms: u32,
    pub cost_kind: Option<String>,
    pub cost_extra_max: u32,
    pub cast_kind: Option<String>,
    pub channel_tick_ms: u32,
    pub interrupt_on: Vec<String>,
    pub defense_type: Option<String>,
    pub on_cast: OnCast,
    /// Which attack outcomes can happen (`dodge` = false for an unavoidable strike).
    pub outcomes: std::collections::BTreeMap<String, bool>,
    pub weapon_hand: Option<String>,
    pub area_shape: Option<String>,
    pub area_center: Option<String>,
    /// How many targets it may land on; `None` for unlimited. Not [`Skill::max_targets`], which
    /// the cast reads.
    pub max_targets: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct OnCast {
    pub breaks_stealth: bool,
    pub starts_auto_attack: bool,
    pub resets_swing: bool,
}

/// One spellmod group of a row: a flat or percent change to an op on the skills it targets.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SpellMod {
    /// `flat` or `pct`.
    #[serde(rename = "type")]
    pub kind: String,
    /// A `SpellModOp` name (`DAMAGE`, `COST`).
    pub op: String,
    /// One skill's `SpellFamilyFlags` bit, or
    pub family_bit: Option<u8>,
    /// several skills' mask.
    pub family_mask: Option<u64>,
    pub value: f64,
    /// The skill keys it targets (server only).
    #[serde(default)]
    pub target: Vec<String>,
}

/// A row's gates, cast shape and spellmods, as `server_spells.json` carries them.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct SpellMeta {
    #[serde(flatten)]
    pub gates: SpellGates,
    #[serde(flatten)]
    pub shape: CastShape,
    pub spellmods: Vec<SpellMod>,
}

/// What a buff does while it lasts.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuraEffect {
    /// The skill's critical strike chance rises by `pct` percent.
    SkillCritPct { skill: String, pct: u32 },
    /// The player's ranged weapon shoots `pct` percent faster.
    ModRangedHastePct { pct: u32 },
    /// The player's ranged attacks crit `pct` percent more often.
    ModRangedCritPct { pct: u32 },
    /// The unit's armour changes by `amount` (a stacking Sunder when negative).
    ModArmor { amount: i32 },
    /// Each cast of `skill` also strikes its targets for `weapon_pct` percent of a `weapon`
    /// roll (Rapid Assault).
    ExtraStrike {
        skill: String,
        weapon: Weapon,
        weapon_pct: u32,
    },
    /// The unit's run speed changes by `pct` percent: faster when positive, a snare when negative.
    ModRunSpeed { pct: i32 },
    /// The player's dodge chance rises by `pct` percent.
    DodgePct { pct: u32 },
    /// The damage the player's non-physical skills deal rises by `pct` percent.
    ModSpellDamagePct { pct: u32 },
    /// The amounts `skill` deals rise by `pct` percent (Arcane Momentum).
    SkillAmountPct { skill: String, pct: u32 },
    /// The unit cannot move, though it still attacks and casts; damage does not break it.
    Root,
    /// `tick` damage in `school` every `tick_ms` from the caster (a ground area's enemy aura).
    PeriodicDamage {
        tick_ms: u32,
        tick: Linear,
        #[serde(default)]
        school: School,
    },
    /// The unit's chance to hit changes by `pct` points (Noxious Vapors: fewer of a mob's
    /// swings land).
    ModHitChance { pct: i32 },
    /// The healing the unit receives rises by `pct` percent.
    ModHealingTakenPct { pct: u32 },
    /// A heal every `tick_ms` for the row's duration: `tick` a tick, and `pct` percent of what
    /// the cast that put it on dealt, healed or cost in health (`pct_of`), spread over the ticks
    /// (Masochism).
    PeriodicHeal {
        tick_ms: u32,
        #[serde(default)]
        tick: Option<Linear>,
        #[serde(default)]
        pct_of: Option<PctOf>,
        #[serde(default)]
        pct: u32,
    },
    /// The unit's casts are `pct` percent faster (Crimson Surge). `family_bits`, resolved by the
    /// build, carry it to the client's cast bars.
    ModCastSpeedPct {
        pct: u32,
        #[serde(default)]
        family_bits: Vec<u8>,
    },
    /// The unit takes `pct` percent more `school` damage from others than the aura's caster
    /// (Festering Wounds). No other player deals damage yet, so it changes nothing today.
    ModDamageTakenPct {
        pct: u32,
        #[serde(default)]
        school: School,
    },
    /// The player's next cast of one of the skills changes: see [`NextCast`].
    NextCast(NextCast),
    /// A count that grows by one each time the aura is put on again; at the row's `max_stacks`
    /// the aura goes and the aura row `becomes` takes its place (Ley-Line Attunement).
    AtMaxStacks { becomes: String },
    /// The player's parry chance rises by `pct` percent.
    ParryPct { pct: u32 },
    /// The player's block chance rises by `pct` percent while a shield is worn.
    BlockPct { pct: u32 },
    /// The player's shield Block Value rises by `pct` percent.
    BlockValuePct { pct: u32 },
    /// The player's armour from items rises by `pct` percent.
    ArmorItemsPct { pct: u32 },
    /// While the aura is up, a proc of its own: see [`AuraProc`].
    Proc(AuraProc),
    /// The skill costs `amount` less of the player's power, used up one `charges` at a time when
    /// the skill lands.
    SkillCost { skill: String, amount: u32 },
    /// The skill's `add_aura_stacks` adds `stacks` (if more than its own), used up one `charges`
    /// at a time when the skill lands.
    SkillStacks { skill: String, stacks: u32 },
    /// The player is stealthed until it breaks; `opener_crit`: the first melee attack or skill
    /// from stealth is a guaranteed critical strike.
    Stealth {
        #[serde(default)]
        opener_crit: bool,
    },
}

/// A proc an aura row carries: while the aura is on the player, when `on` happens, `chance`
/// percent of the time: heal `heal_pct` of the maximum health, give `energize` of the player's
/// power, put up the aura row `apply_aura`, and deal `damage_attacker` (a formula),
/// `block_value_pct` of the shield's Block Value or `weapon_pct` of the average weapon swing in
/// `school` to the event's target.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AuraProc {
    pub on: ProcEvent,
    #[serde(default = "always")]
    pub chance: u32,
    #[serde(default)]
    pub heal_pct: u32,
    #[serde(default)]
    pub energize: u32,
    #[serde(default)]
    pub apply_aura: Option<String>,
    #[serde(default)]
    pub damage_attacker: Option<Linear>,
    #[serde(default)]
    pub block_value_pct: u32,
    #[serde(default)]
    pub weapon_pct: u32,
    #[serde(default)]
    pub school: School,
    /// The skill whose `skill_amount_pct` talents raise the damage.
    #[serde(default)]
    pub amount_skill: Option<String>,
    /// Only for this skill (Rapid Fire's refund on Auto Shot).
    #[serde(default)]
    pub skill: Option<String>,
    /// Give this percent of the player's base mana.
    #[serde(default)]
    pub energize_pct_of_base: u32,
}

/// What a `next_cast` aura does to the next cast of one of `skill`: an instant cast, a cast bar
/// `cast_time_ms` longer (negative: shorter), a cost `cost_pct` percent higher (negative: lower),
/// and `duration_ms` more on the aura the skill applies. One of the aura's charges goes with each
/// such cast; with none, it lasts its duration.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NextCast {
    #[serde(deserialize_with = "one_or_many")]
    pub skill: Vec<String>,
    #[serde(default)]
    pub instant: bool,
    #[serde(default)]
    pub cast_time_ms: i32,
    #[serde(default)]
    pub cost_pct: i32,
    #[serde(default)]
    pub duration_ms: i32,
    /// The next cast goes off as this skill of the Calling instead, hidden or not (Explosive
    /// Payload's shot in place of an Auto Shot).
    #[serde(default)]
    pub becomes: Option<String>,
    /// The skills' `SpellFamilyFlags` bits, resolved by the build, for the client's cast bar.
    #[serde(default)]
    pub family_bits: Vec<u8>,
}

/// Up to four spell ids, the rest 0.
fn up_to_four<'de, D: serde::Deserializer<'de>>(d: D) -> Result<[u32; 4], D::Error> {
    let ids = Vec::<u32>::deserialize(d)?;
    if ids.len() > 4 {
        return Err(serde::de::Error::custom("at most four spell ids"));
    }
    let mut out = [0; 4];
    out[..ids.len()].copy_from_slice(&ids);
    Ok(out)
}

/// One string, or a list of them (`key` or `key|key` in a CSV cell).
fn one_or_many<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }
    Ok(match OneOrMany::deserialize(d)? {
        OneOrMany::One(s) => vec![s],
        OneOrMany::Many(v) => v,
    })
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

/// The vanilla class table a Calling's stats follow, by name (`warrior`, `rogue`, `mage` ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatProfile {
    Warrior,
    Paladin,
    Hunter,
    Rogue,
    Priest,
    Shaman,
    Mage,
    Warlock,
    Druid,
}

impl StatProfile {
    /// Its `ChrClasses.dbc` id (`rules::power::class`).
    pub fn class(self) -> u8 {
        use crate::rules::power::class;
        match self {
            Self::Warrior => class::WARRIOR,
            Self::Paladin => class::PALADIN,
            Self::Hunter => class::HUNTER,
            Self::Rogue => class::ROGUE,
            Self::Priest => class::PRIEST,
            Self::Shaman => class::SHAMAN,
            Self::Mage => class::MAGE,
            Self::Warlock => class::WARLOCK,
            Self::Druid => class::DRUID,
        }
    }

    /// The profile of a `ChrClasses.dbc` id; `None` for one with no table.
    pub fn of_class(class: u8) -> Option<Self> {
        [
            Self::Warrior,
            Self::Paladin,
            Self::Hunter,
            Self::Rogue,
            Self::Priest,
            Self::Shaman,
            Self::Mage,
            Self::Warlock,
            Self::Druid,
        ]
        .into_iter()
        .find(|p| p.class() == class)
    }
}

/// A weapon kind a Calling may wield, or `dual_wield`: a one-hander in the off hand too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponKind {
    OneHandAxe,
    TwoHandAxe,
    Bow,
    Gun,
    OneHandMace,
    TwoHandMace,
    Polearm,
    OneHandSword,
    TwoHandSword,
    Staff,
    Fist,
    Misc,
    Dagger,
    Thrown,
    Crossbow,
    Wand,
    FishingPole,
    DualWield,
}

impl WeaponKind {
    /// Its `ITEM_CLASS_WEAPON` subclass; `None` for `dual_wield`.
    pub fn subclass(self) -> Option<u32> {
        Some(match self {
            Self::OneHandAxe => 0,
            Self::TwoHandAxe => 1,
            Self::Bow => 2,
            Self::Gun => 3,
            Self::OneHandMace => 4,
            Self::TwoHandMace => 5,
            Self::Polearm => 6,
            Self::OneHandSword => 7,
            Self::TwoHandSword => 8,
            Self::Staff => 10,
            Self::Fist => 13,
            Self::Misc => 14,
            Self::Dagger => 15,
            Self::Thrown => 16,
            Self::Crossbow => 18,
            Self::Wand => 19,
            Self::FishingPole => 20,
            Self::DualWield => return None,
        })
    }

    /// Every weapon a vanilla Warrior wields, the default of a Calling that names none.
    pub fn warrior() -> Vec<Self> {
        vec![
            Self::OneHandAxe,
            Self::TwoHandAxe,
            Self::Bow,
            Self::Gun,
            Self::OneHandMace,
            Self::TwoHandMace,
            Self::Polearm,
            Self::OneHandSword,
            Self::TwoHandSword,
            Self::Staff,
            Self::Fist,
            Self::Misc,
            Self::Dagger,
            Self::Thrown,
            Self::Crossbow,
            Self::FishingPole,
        ]
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
    /// Each rank row's spellmod groups from the master CSV, by rank. Loaded, not enforced yet;
    /// the client cells above still come from the TOML.
    #[serde(default)]
    pub rank_spellmods: Vec<Vec<SpellMod>>,
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
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TalentEffect {
    /// The skill costs this many points less of the Calling's power.
    SkillCost {
        skill: String,
        per_rank: Vec<u32>,
        /// A flat amount taken off (`amount=`); with `pct=`, the cost rises by that percent
        /// instead (Volatile Embers).
        #[serde(default = "yes")]
        flat: bool,
    },
    /// The skill has this percent chance to land on one more enemy in melee reach in front of
    /// the caster, for the same blow and no combo point (Flurry of Blades).
    SkillExtraTarget { skill: String, per_rank: Vec<u32> },
    /// The skill's combo point gains this percent chance of one more; with `target_state`
    /// `"bleeding"`, only on a target the caster has a bleed on (Serrated Strikes).
    SkillBonusCombo {
        skill: String,
        #[serde(default)]
        target_state: Option<String>,
        per_rank: Vec<u32>,
    },
    /// The skill's damage and buff amounts rise by this percent.
    SkillAmountPct {
        skill: String,
        per_rank: Vec<u32>,
        /// Which of the skill's amounts it raises: its hit, its ticks, or both (and its other
        /// amounts).
        #[serde(default)]
        applies_to: AppliesTo,
        /// Only while the caster's aura of this aura row is on the target as the blow lands.
        #[serde(default)]
        requires_target_aura: Option<String>,
        /// Only on a target within this many yards of the caster (Point Blank); 0 for any.
        #[serde(default)]
        max_distance_yd: f32,
    },
    /// The skill's strikes skip this much more of the target's armour.
    SkillArmorIgnore { skill: String, per_rank: Vec<u32> },
    /// The character's `school` damage rises by this percent on a target wearing one of the
    /// `aura` rows (Shattered Defenses).
    DamagePctVsAura {
        #[serde(deserialize_with = "one_or_many")]
        aura: Vec<String>,
        #[serde(default)]
        school: School,
        per_rank: Vec<u32>,
    },
    /// The skill (`ranged`: every `defense_type=ranged` skill) has no minimum range.
    NoMinRange { skill: String, per_rank: Vec<u32> },
    /// The skill's rage gain rises by this many points.
    SkillRage { skill: String, per_rank: Vec<u32> },
    /// The skill's critical strike chance rises by this many percent.
    SkillCritPct { skill: String, per_rank: Vec<u32> },
    /// A skill that consumes aura stacks heals the caster for this percent of the bonus damage
    /// the stacks dealt.
    ConsumeHealPct { skill: String, per_rank: Vec<u32> },
    /// The skill's `consume_aura` outcomes, damage and power alike, are this percent larger.
    ConsumeAmountPct { skill: String, per_rank: Vec<u32> },
    /// The patches the skill leaves deal this percent more damage a tick.
    GroundAmountPct { skill: String, per_rank: Vec<u32> },
    /// The patches the skill leaves last this many milliseconds longer.
    GroundDurationMs { skill: String, per_rank: Vec<u32> },
    /// Every melee strike's critical strike chance rises by this many percent.
    MeleeCritPct { per_rank: Vec<u32> },
    /// The Defense skill rises by this much.
    Defense { per_rank: Vec<u32> },
    /// A stat rises by this percent of its total (base, Tier Bonus and gear), or, with `flat`,
    /// by this many points.
    ModStat {
        stat: Stat,
        #[serde(default)]
        flat: bool,
        per_rank: Vec<u32>,
    },
    /// The maximum health rises by this percent, after the stat talents (Vitality).
    MaxHealthPct { per_rank: Vec<u32> },
    /// The healing the character receives rises by this percent while their health is below
    /// `below_health_pct` percent (Will to Survive); at any health with none.
    HealingTakenPct {
        per_rank: Vec<u32>,
        #[serde(default)]
        below_health_pct: u32,
    },
    /// Every critical strike chance, melee and spell, rises by this many percent.
    ModCritChance { per_rank: Vec<u32> },
    /// The dodge chance rises by this many percent.
    DodgePct { per_rank: Vec<u32> },
    /// The character's physical attacks ignore this much of the target's armour (never below 0).
    ArmorPen { per_rank: Vec<u32> },
    /// The maximum of the character's own power (Energy for a Cutthroat) rises by this much; what
    /// is held stays.
    MaxPower { per_rank: Vec<u32> },
    /// The character runs this many percent faster, always.
    ModRunSpeed { per_rank: Vec<u32> },
    /// The off hand's swings deal this many percent more damage.
    OffhandDamagePct { per_rank: Vec<u32> },
    /// Every critical strike does this percent more of its damage (a physical crit's bonus is
    /// its whole damage, a magic crit's the half over 100%).
    ModCritDamage { per_rank: Vec<u32> },
    /// Out-of-combat health regeneration rises by this percent.
    ModHealthRegenOutOfCombat { per_rank: Vec<u32> },
    /// When `on` happens (and the filters hold), `chance` percent of the time: heal `per_rank`
    /// percent of the maximum health (0 for none), apply the rank's `auras` by key, give the
    /// rank's `energize` of the character's power, and add the rank's `combo_points` on the
    /// event's target.
    Proc {
        on: ProcEvent,
        /// How long `moving_for` or `still_for` waits.
        #[serde(default)]
        ms: u32,
        /// The triggering skill goes off again at once at the same target, free, off the global
        /// cooldown; a repeat never repeats (Twin Arrows).
        #[serde(default)]
        repeat_cast: bool,
        /// The skill whose cooldown each rank's `cooldown_ms` changes (Adrenaline Rush).
        #[serde(default)]
        cooldown_skill: Option<String>,
        #[serde(default)]
        cooldown_ms: Vec<i32>,
        /// Each rank's percent of the damage done dealt again to every other enemy within
        /// `radius_yd` of the target (Shrapnel Payload).
        #[serde(default)]
        splash_pct: Vec<u32>,
        /// Only for a cast of this skill.
        #[serde(default)]
        skill: Option<String>,
        /// Only for a killing blow on a target that yields experience.
        #[serde(default)]
        yields_xp: bool,
        #[serde(default = "always")]
        chance: u32,
        /// The chance a rank has, when it differs from `chance`.
        #[serde(default)]
        chances: Vec<u32>,
        per_rank: Vec<u32>,
        #[serde(default)]
        auras: Vec<Vec<String>>,
        #[serde(default)]
        energize: Vec<u32>,
        #[serde(default)]
        combo_points: Vec<u32>,
        /// Deal this formula's damage in `school` to the event's target (the attacker, for a
        /// defensive event).
        #[serde(default)]
        damage_attacker: Option<Linear>,
        /// Deal the rank's percent of the shield's Block Value in `school` to the event's target.
        #[serde(default)]
        block_value_pct: Vec<u32>,
        #[serde(default)]
        school: School,
        /// The skill whose `skill_amount_pct` talents raise the damage.
        #[serde(default)]
        amount_skill: Option<String>,
        /// Deal this percent of the average weapon swing in `school` to the event's target.
        #[serde(default)]
        weapon_pct: u32,
        /// Only while the aura row of this key is on the character.
        #[serde(default)]
        while_aura: Option<String>,
        /// On a consume: a fresh copy of the consumed aura goes on one other enemy within this
        /// many yards of the target.
        #[serde(default)]
        spread: f32,
        /// Each rank's aura keys put on every living enemy within `radius_yd` of where the
        /// caster stood as they cast.
        #[serde(default)]
        area_auras: Vec<Vec<String>>,
        #[serde(default)]
        radius_yd: f32,
        /// Each rank's percent of the triggering cast's cost given back.
        #[serde(default)]
        refund_cost_pct: Vec<u32>,
        /// A shield on the caster soaking this percent of the triggering cast's cost, for
        /// `absorb_duration_ms`.
        #[serde(default)]
        absorb_cost_pct: u32,
        #[serde(default)]
        absorb_duration_ms: u32,
        /// `absorb`: when the caster's bar was full before the cast, the shield goes up and
        /// nothing else; otherwise everything but the shield.
        #[serde(default)]
        at_full_power: Option<String>,
        /// Only when the target wore this aura row's aura before the cast landed; `slowed` for
        /// any snare.
        #[serde(default)]
        requires_target_aura: Option<String>,
        /// Each rank's percent of the character's base mana given.
        #[serde(default)]
        energize_pct_of_base: Vec<u32>,
        /// Each rank's aura keys put on the event's target.
        #[serde(default)]
        target_auras: Vec<Vec<String>>,
        /// Only for damage in this school (`damage_school=`).
        #[serde(default)]
        damage_school: Option<School>,
        /// The ticking aura's copy, its time left and one stack, goes on the nearest other enemy
        /// within this many yards of the target that lacks it (Epidemic).
        #[serde(default)]
        spread_aura: f32,
        /// Each rank's percent of the damage done that heals the friendly players within
        /// `radius_yd` of the character, split evenly (Symbiotic Rot).
        #[serde(default)]
        heal_allies_pct: Vec<u32>,
        /// `health`: only a cast that cost health.
        #[serde(default)]
        cost_kind: Option<String>,
        /// What the shield is a share of: the cast's cost (default), its heal, its hit or its
        /// health cost; and the one school it soaks, if any.
        #[serde(default)]
        absorb_of: Option<PctOf>,
        #[serde(default)]
        absorb_school: Option<School>,
    },
    /// The armour from items rises by this many percent.
    #[serde(alias = "armor_items_pct")]
    ItemArmorPct { per_rank: Vec<u32> },
    /// A melee critical strike makes the target bleed for this percent of `of` (the average
    /// swing, or the crit's own damage), under `aura_spell_id`, a tick every `tick_ms` for
    /// `duration_ms`.
    CritBleed {
        aura_spell_id: u32,
        tick_ms: u32,
        duration_ms: u32,
        #[serde(default)]
        of: CritBleedOf,
        per_rank: Vec<u32>,
    },
    /// The physical damage the character deals rises by this percent while it wields `weapon`.
    WeaponDamagePct { weapon: Wielded, per_rank: Vec<u32> },
    /// The melee critical strike chance rises by this many percent while it wields `weapon`.
    WeaponCritPct { weapon: Wielded, per_rank: Vec<u32> },
    /// A skill's critical strike adds this percent of the whole crit's damage (Impale). With
    /// `builders`, only the skills that add combo points (Lethality).
    SkillCritDamagePct {
        #[serde(default)]
        builders: bool,
        /// Only this skill's crits; every skill's when unset.
        #[serde(default)]
        skill: Option<String>,
        per_rank: Vec<u32>,
    },
    /// The skill's auras last this percent longer.
    SkillDurationPct { skill: String, per_rank: Vec<u32> },
    /// The skill's auras last this many milliseconds longer (Bloodletting).
    SkillDurationMs { skill: String, per_rank: Vec<u32> },
    /// The parry chance rises by this many percent.
    ParryPct { per_rank: Vec<u32> },
    /// The block chance with a shield rises by this many percent.
    BlockPct { per_rank: Vec<u32> },
    /// The shield's Block Value rises by this many percent.
    BlockValuePct { per_rank: Vec<u32> },
    /// The skill's strike deals this percent of the shield's Block Value more (Iron Juggernaut).
    SkillBlockValuePct { skill: String, per_rank: Vec<u32> },
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

fn yes() -> bool {
    true
}

/// What a share is of: the damage the cast dealt, or what it healed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PctOf {
    Hit,
    Heal,
    /// The health the cast cost.
    HealthCost,
    /// The caster's maximum mana at cast time.
    MaxMana,
    /// The caster's maximum health at cast time.
    MaxHealth,
}

/// How an effect picks its friendly player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllyPick {
    /// The one with the smallest share of their health left.
    LowestHealth,
}

/// Where an ally pick's radius is measured from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Center {
    /// The cast's target.
    #[default]
    Target,
    /// The caster.
    Caster,
}

/// Which of a skill's amounts a `skill_amount_pct` raises.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppliesTo {
    /// Every amount: the hit, the ticks, and the rest.
    #[default]
    Both,
    /// The direct hit only.
    Hit,
    /// The damage-over-time ticks only.
    Tick,
}

fn always() -> u32 {
    100
}

/// What a [`TalentEffect::Proc`] waits for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcEvent {
    /// A skill is cast.
    Cast,
    /// The character lands a killing blow.
    Kill,
    /// The character dodges a melee attack.
    Dodge,
    /// The character lands a skill that spends combo points.
    Finisher,
    /// The character blocks a melee attack.
    Block,
    /// The character parries a melee attack.
    Parry,
    /// A melee attack crits the character.
    CritTaken,
    /// The character lands a melee hit, a white swing or a weapon skill's.
    MeleeHit,
    /// The character's `consume_aura` takes its aura off a target.
    Consume,
    /// A damage-over-time the character put on a target ticks.
    PeriodicTick,
    /// The character's cast or damage-over-time tick damages a target (once a cast or tick).
    DamageDone,
    /// The character has moved for the proc's `ms` without stopping, and again each further
    /// `ms`.
    MovingFor,
    /// The character has stood still for the proc's `ms`; once, until they move again.
    StillFor,
    /// The character's cast lands a critical strike (once a cast).
    Crit,
}

/// One of the five stats, in the order of [`crate::rules::stats::CharacterStats::base`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stat {
    Strength,
    Agility,
    Stamina,
    Intellect,
    Spirit,
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

/// What a `crit_bleed`'s percent is a share of.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CritBleedOf {
    /// The main hand's average swing, spread over four ticks (Deep Wounds).
    #[default]
    AverageSwing,
    /// The damage of the crit that caused it (Scent of Blood).
    CritDamage,
}

impl TalentEffect {
    /// The effect's size at each rank.
    pub fn per_rank(&self) -> &[u32] {
        match self {
            Self::SkillCost { per_rank, .. }
            | Self::SkillExtraTarget { per_rank, .. }
            | Self::SkillBonusCombo { per_rank, .. }
            | Self::SkillAmountPct { per_rank, .. }
            | Self::SkillArmorIgnore { per_rank, .. }
            | Self::DamagePctVsAura { per_rank, .. }
            | Self::NoMinRange { per_rank, .. }
            | Self::SkillRage { per_rank, .. }
            | Self::SkillCritPct { per_rank, .. }
            | Self::ConsumeHealPct { per_rank, .. }
            | Self::ConsumeAmountPct { per_rank, .. }
            | Self::GroundAmountPct { per_rank, .. }
            | Self::GroundDurationMs { per_rank, .. }
            | Self::MeleeCritPct { per_rank }
            | Self::Defense { per_rank }
            | Self::ModStat { per_rank, .. }
            | Self::MaxHealthPct { per_rank }
            | Self::HealingTakenPct { per_rank, .. }
            | Self::ModCritChance { per_rank }
            | Self::DodgePct { per_rank }
            | Self::ArmorPen { per_rank }
            | Self::MaxPower { per_rank }
            | Self::ModRunSpeed { per_rank }
            | Self::OffhandDamagePct { per_rank }
            | Self::ModCritDamage { per_rank }
            | Self::ModHealthRegenOutOfCombat { per_rank }
            | Self::Proc { per_rank, .. }
            | Self::ItemArmorPct { per_rank }
            | Self::CritBleed { per_rank, .. }
            | Self::WeaponDamagePct { per_rank, .. }
            | Self::WeaponCritPct { per_rank, .. }
            | Self::SkillCritDamagePct { per_rank, .. }
            | Self::SkillDurationPct { per_rank, .. }
            | Self::SkillDurationMs { per_rank, .. }
            | Self::ParryPct { per_rank }
            | Self::BlockPct { per_rank }
            | Self::BlockValuePct { per_rank }
            | Self::SkillBlockValuePct { per_rank, .. }
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

/// A `SpellModOp` name's number: vmangos's for vanilla's ops (13 and 25 to 28 have none), and
/// the Twilight ops past them (ADR 0010), which only benilla reads.
pub fn spell_mod_op(name: &str) -> Option<u8> {
    const VANILLA: [&str; 25] = [
        "DAMAGE",
        "DURATION",
        "THREAT",
        "EFFECT1",
        "CHARGES",
        "RANGE",
        "RADIUS",
        "CRITICAL_CHANCE",
        "ALL_EFFECTS",
        "NOT_LOSE_CASTING_TIME",
        "CASTING_TIME",
        "COOLDOWN",
        "EFFECT2",
        "",
        "COST",
        "CRIT_DAMAGE_BONUS",
        "RESIST_MISS_CHANCE",
        "JUMP_TARGETS",
        "CHANCE_OF_SUCCESS",
        "ACTIVATION_TIME",
        "EFFECT_PAST_FIRST",
        "CASTING_TIME_OLD",
        "DOT",
        "EFFECT3",
        "SPELL_BONUS_DAMAGE",
    ];
    match name {
        "" => None,
        "MIN_RANGE" => Some(29),
        _ => VANILLA.iter().position(|n| *n == name).map(|i| i as u8),
    }
}

/// One skill: a 1.12.1 spell id, what it costs, what it needs and what it does.
#[derive(Debug, Deserialize)]
pub struct Skill {
    pub key: String,
    pub spell_id: u32,
    /// The level the Calling learns it at; 1 from the start.
    #[serde(default = "one_u8")]
    pub learn_level: u8,
    /// Whole points of the Calling's power (`realm_cost` in the master CSV).
    pub power_cost: u32,
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
    /// `power_cost` rage in any stance, as a Warrior spell (ADR 0003).
    #[serde(default)]
    pub borrowed: Option<BorrowedFamilyBit>,
    pub effects: Vec<Effect>,
    /// Gates, cost kind, cast shape and spellmods from the master CSV. Loaded, not enforced yet.
    #[serde(default)]
    pub meta: SpellMeta,
}

/// The `SpellFamilyFlags` bits the client gives borrowed spells: no Warrior spell uses a bit
/// from 33 up, so a talent cell on one of them reaches that borrowed spell alone.
pub const BORROWED_FAMILY_BITS: std::ops::RangeInclusive<u8> = 33..=63;

/// Where the client files a borrowed spell among the Warrior's: its own family bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct BorrowedFamilyBit {
    /// Its own `SpellFamilyFlags` bit, in [`BORROWED_FAMILY_BITS`], so a talent's
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
    /// A friendly player: the caster, the only one until other players exist.
    Ally,
    /// A point on the ground the client picks. No ground area plays yet, so the cast is refused.
    #[serde(rename = "dest")]
    Ground,
}

/// Which roll a weapon strike starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weapon {
    /// A white swing's roll, attack power at the weapon's speed.
    Normal,
    /// Attack power at the normalized speed.
    Normalized,
    /// The ranged weapon's shot, ranged attack power at its speed; nothing without one.
    Ranged,
}

/// Where a target stands for a strike to be a guaranteed critical strike.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CritIf {
    /// Inside a patch the caster left on the ground (Consecration).
    GroundEffect,
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
        /// Added to `bonus` once per combo point on the target: a finisher.
        #[serde(default)]
        per_combo: Option<Linear>,
        /// The percent of the weapon roll the strike deals.
        #[serde(default = "always")]
        weapon_pct: u32,
        /// The school of its damage; a magic one ignores armour.
        #[serde(default)]
        school: School,
        /// A guaranteed critical strike on a target standing in this of the caster's.
        #[serde(default)]
        crit_if_target_in: Option<CritIf>,
        /// This much of the target's armour is skipped for this strike (Puncturing Shaft).
        #[serde(default)]
        armor_ignore: Option<Linear>,
    },
    /// Flat damage of a school. Physical goes against armour, and through the melee table when
    /// `melee_hit_table`; any other school ignores armour, rolls only the level-based miss, and
    /// crits for 150%.
    SchoolDamage {
        #[serde(default)]
        school: School,
        amount: Linear,
        #[serde(default)]
        melee_hit_table: bool,
        /// Added to `amount` once per combo point on the target: a finisher.
        #[serde(default)]
        per_combo: Option<Linear>,
        /// Extra crit chance, in percent, at a full mana bar, scaled by how full it is when the
        /// blow rolls (after the cast's own cost). A caster on another bar gets none.
        #[serde(default)]
        crit_pct_at_full_mana: Option<f32>,
    },
    /// A bleed on the target under the skill's own spell id: `tick` every `tick_ms`.
    PeriodicDamage {
        tick_ms: u32,
        duration_ms: u32,
        tick: Linear,
        /// Added to `tick` once per combo point on the target: a finisher.
        #[serde(default)]
        per_combo: Option<Linear>,
        /// Milliseconds added to `duration_ms` once per combo point on the target: a finisher.
        #[serde(default)]
        duration_per_combo: Option<Linear>,
        /// A new application adds a stack up to this many and starts the duration over; each
        /// tick deals `tick` once per stack. From the aura row's `max_stacks` under `stack_rule`
        /// `stack`.
        #[serde(default = "one")]
        max_stacks: u32,
        /// The aura row's spell id when `apply_aura` names one, so another skill applying it
        /// (Elemental Convergence) puts on the same aura; the skill's own otherwise.
        #[serde(default)]
        aura_spell_id: Option<u32>,
        /// The school its ticks are logged in (the row's `school`).
        #[serde(default)]
        school: School,
        /// Also `pct` percent of what this cast dealt or healed (`pct_of`), spread over the ticks
        /// (Dark Mending's toll).
        #[serde(default)]
        pct_of: Option<PctOf>,
        #[serde(default)]
        pct: u32,
        /// Each tick heals the caster this percent of what it dealt (Creeping Blight).
        #[serde(default)]
        leech_pct: u32,
    },
    /// Heals the target `amount`, up to its maximum health.
    Heal { amount: Linear },
    /// Deals `amount` in `school` to the target and heals the caster `heal_pct` percent of what
    /// it dealt (Siphon Life's tick).
    Drain {
        amount: Linear,
        #[serde(default)]
        heal_pct: u32,
        #[serde(default)]
        school: School,
    },
    /// A heal every `tick_ms` for `duration_ms`: `tick` a tick, and `pct` percent of what this
    /// cast dealt or healed (`pct_of`) spread over the ticks. With `ally`, it goes on the friendly
    /// player that rule picks within `radius_yd` of `center` instead of the cast's target.
    PeriodicHeal {
        tick_ms: u32,
        duration_ms: u32,
        #[serde(default)]
        tick: Option<Linear>,
        #[serde(default)]
        pct_of: Option<PctOf>,
        #[serde(default)]
        pct: u32,
        #[serde(default)]
        aura_spell_id: Option<u32>,
        #[serde(default)]
        ally: Option<AllyPick>,
        #[serde(default)]
        radius_yd: f32,
        #[serde(default)]
        center: Center,
    },
    /// A Kick: stops the Creature Spell the target is casting and locks its school for
    /// `lockout_ms`. The effects after it in the row run only on a target whose cast it stopped.
    /// With `bosses` false it leaves a boss alone.
    Interrupt {
        lockout_ms: u32,
        #[serde(default = "yes")]
        bosses: bool,
    },
    /// `amount` combo points on the target, five at most; points on another target are lost.
    AddComboPoints { amount: u32 },
    /// The caster runs to the target.
    Charge { speed_yd_s: f32 },
    /// `amount` of `power` now, logged as an energize; nothing at all when the target's bar is
    /// of another kind.
    Energize {
        power: PowerKind,
        amount: Linear,
        /// Also this percent of the bar's maximum (Mana Surge).
        #[serde(default)]
        pct_of_max: u32,
    },
    /// The caster's cooldowns on these spells end now (Mana Surge). The build resolves
    /// `skill=key|key` to the skills' spell ids; four at most.
    ResetCooldowns {
        #[serde(deserialize_with = "up_to_four")]
        spell_ids: [u32; 4],
    },
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
        /// Milliseconds added to `duration_ms` once per combo point on the target: a finisher.
        #[serde(default)]
        per_combo: Option<Linear>,
    },
    /// A stun under the skill's spell id. With `break_on_damage`, any damage after the cast ends
    /// it. The target stays in combat: no player mechanic takes a mob out of it (#165). No
    /// diminishing returns.
    Incapacitate {
        duration_ms: u32,
        #[serde(default)]
        break_on_damage: bool,
    },
    /// The target swings `pct`% slower, under the skill's spell id.
    AttackSpeedSlow { pct: u32, duration_ms: u32 },
    /// The target moves `pct`% slower, under the skill's spell id.
    MovementSlow { pct: u32, duration_ms: u32 },
    /// A shield of `pct` percent of what this cast dealt, healed or cost in health (`pct_of`) on
    /// every friendly player within `radius_yd` of the caster, for `duration_ms`; with a
    /// `school`, it soaks only that (Sanguine Shell).
    Absorb {
        pct: u32,
        pct_of: PctOf,
        duration_ms: u32,
        #[serde(default)]
        radius_yd: f32,
        #[serde(default)]
        school: Option<School>,
    },
    /// A ground area at the cast's destination: `radius_yd` across, for `duration_ms`. Every
    /// enemy inside holds the aura row `enemy_aura_spell_id`, every friendly player inside
    /// `ally_aura_spell_id`; leaving or the area's end takes them off. A new one from the same
    /// caster replaces the old.
    GroundArea {
        radius_yd: f32,
        duration_ms: u32,
        #[serde(default)]
        enemy_aura_spell_id: Option<u32>,
        #[serde(default)]
        ally_aura_spell_id: Option<u32>,
    },
    /// The caster teleports `distance_yd` along their facing, as far as the ground lets them, and
    /// sheds their stuns and roots first.
    Blink {
        distance_yd: f32,
        /// Straight back from the facing instead (Vault).
        #[serde(default)]
        backward: bool,
    },
    /// Takes the caster's aura `aura_spell_id` off the target, then pays its outcome: damage of
    /// `amount_pct` percent of the hit `amount` of the skill `amount_of` names, in its `school`, and
    /// `energize_pct_of_max` percent of the caster's power bar. Nothing when the aura is absent.
    /// `duration_ms` is the aura row's, for a fresh copy a proc spreads.
    ConsumeAura {
        aura_spell_id: u32,
        #[serde(default)]
        duration_ms: u32,
        /// The named skill's hit, resolved by the build from `amount_of=skill`.
        #[serde(default)]
        amount: Option<Linear>,
        #[serde(default)]
        school: School,
        #[serde(default = "always")]
        amount_pct: u32,
        #[serde(default)]
        energize_pct_of_max: u32,
        /// The damage is `amount` once per stack the aura held (Necrotic Detonation).
        #[serde(default)]
        per_stack: bool,
        /// Then `heal_allies_pct` percent of the damage dealt heals the friendly players within
        /// `radius_yd` of the target, split evenly.
        #[serde(default)]
        heal_allies_pct: u32,
        #[serde(default)]
        radius_yd: f32,
    },
    /// The target's run speed changes by `pct` percent for `duration_ms`: a snare on an enemy
    /// when negative, a sprint on the caster when positive.
    ModRunSpeed {
        pct: i32,
        duration_ms: u32,
        /// As `PeriodicDamage`'s: the aura row's spell id when `apply_aura` names one.
        #[serde(default)]
        aura_spell_id: Option<u32>,
    },
    /// The caster's dodge chance rises by `pct` percent for `duration_ms`, under the skill's
    /// spell id (Evasion).
    DodgePct { pct: u32, duration_ms: u32 },
    /// The caster takes its Calling's aura row of `aura_spell_id`, every effect of it, for the
    /// row's duration.
    GrantAura { aura_spell_id: u32 },
    /// The caster's parry chance rises by `pct` percent for `duration_ms`.
    ParryPct { pct: u32, duration_ms: u32 },
    /// The caster's block chance rises by `pct` percent for `duration_ms`, with a shield worn
    /// (Shield of the Penitent).
    BlockPct { pct: u32, duration_ms: u32 },
    /// The caster's shield Block Value rises by `pct` percent for `duration_ms`.
    BlockValuePct { pct: u32, duration_ms: u32 },
    /// The caster's armour from items rises by `pct` percent for `duration_ms`.
    ArmorItemsPct { pct: u32, duration_ms: u32 },
    /// The caster is stealthed (Stealth) until it breaks, or for `duration_ms` when set, under the
    /// skill's spell id; see [`AuraEffect::Stealth`].
    Stealth {
        #[serde(default)]
        duration_ms: u32,
        #[serde(default)]
        opener_crit: bool,
    },
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
    /// `stacks` more of the caster's aura `aura_spell_id` when the hit lands, held to the aura
    /// row's `max_stacks` under its `stack_rule`, each application putting it back to
    /// `duration_ms`.
    AddAuraStacks {
        aura_spell_id: u32,
        #[serde(default = "one")]
        stacks: u32,
        duration_ms: u32,
        #[serde(default = "one")]
        max_stacks: u32,
        #[serde(default)]
        stack_rule: StackRule,
    },
    /// Takes every stack of the caster's aura `aura_spell_id`, and strikes each target the cast
    /// hit for `per_stack` of `school` damage a stack. No stacks, no bonus.
    ConsumeAuraStacks {
        aura_spell_id: u32,
        per_stack: Linear,
        #[serde(default)]
        school: School,
    },
    /// A patch on the ground where the cast ends (the caster's spot, after a Charge), `radius_yd`
    /// wide: every `tick_ms` for `duration_ms`, each enemy inside takes `amount` of `school`
    /// damage. The caster has one patch at a time; a new one replaces the old.
    GroundEffect {
        radius_yd: f32,
        tick_ms: u32,
        duration_ms: u32,
        #[serde(default)]
        school: School,
        amount: Linear,
        /// The row the patch is named after (`ground_spell=key`): its DynamicObject and its tick
        /// logs carry this id. `None` names the skill that cast it.
        #[serde(default)]
        ground_spell_id: Option<u32>,
    },
}

fn one() -> u32 {
    1
}

fn one_u8() -> u8 {
    1
}

/// What a new application of an aura does to the stacks it already holds (the `stack_rule`
/// column).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StackRule {
    /// The stacks add up to the maximum, and the duration starts over.
    #[default]
    Refresh,
    /// The new application's stacks replace the old, and the duration starts over.
    Replace,
    /// The old aura stays exactly as it is.
    Ignore,
    /// Each application is its own aura. Not modelled apart yet: it stacks like `Refresh`.
    Independent,
    /// One more stack each application, up to the maximum, and the duration starts over (a
    /// stacking damage over time).
    Stack,
}

impl Effect {
    /// The block added once per combo point held on the target, when the effect carries one.
    pub fn per_combo(&self) -> Option<Linear> {
        match *self {
            Effect::WeaponStrike { per_combo, .. }
            | Effect::SchoolDamage { per_combo, .. }
            | Effect::PeriodicDamage { per_combo, .. }
            | Effect::Stun { per_combo, .. } => per_combo,
            _ => None,
        }
    }
}

impl Skill {
    /// A shot: it needs a bow, gun, crossbow or thrown weapon in the ranged slot
    /// (`requires_weapon=any_ranged`), and ammo for one that shoots it.
    pub fn wants_ranged(&self) -> bool {
        self.meta
            .gates
            .requires_weapon
            .iter()
            .any(|w| w == "any_ranged")
    }

    /// A finisher: any effect carries a `per_combo` block. It needs combo points on the target
    /// to cast, and a cast that lands spends them all.
    pub fn is_finisher(&self) -> bool {
        self.effects.iter().any(|e| {
            e.per_combo().is_some()
                || matches!(
                    e,
                    Effect::PeriodicDamage {
                        duration_per_combo: Some(_),
                        ..
                    }
                )
        })
    }

    /// A builder: a cast that lands puts combo points on the target.
    pub fn adds_combo_points(&self) -> bool {
        self.effects
            .iter()
            .any(|e| matches!(e, Effect::AddComboPoints { .. }))
    }
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
    /// and each white swing cut to match (strikes keep their full weapon roll). Judged: the target takes `holy_taken_pct`% more holy damage for
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

/// The shipped Callings: the ones the master sheet defines (`generated.rs`), then the tables in
/// `content/callings.toml`.
fn shipped() -> &'static [CallingDef] {
    static CALLINGS: OnceLock<Vec<CallingDef>> = OnceLock::new();
    CALLINGS.get_or_init(|| {
        let mut all = crate::generated::callings();
        let tables = parse_toml(include_str!("../content/callings.toml"))
            .expect("content/callings.toml parses");
        all.extend(tables.calling);
        all
    })
}

/// Callings in the shape of `content/callings.toml`. A TOML Calling may still write `class = N`
/// for its `stat_profile` and that vanilla class's power (Warrior rage, Rogue energy, the rest
/// mana): the test seam's shorthand. `power` or `stat_profile` written beside it win.
fn parse_toml(text: &str) -> Result<Callings, toml::de::Error> {
    let mut value: toml::Value = toml::from_str(text)?;
    for calling in value
        .get_mut("calling")
        .and_then(toml::Value::as_array_mut)
        .into_iter()
        .flatten()
    {
        let Some(table) = calling.as_table_mut() else {
            continue;
        };
        let Some(class) = table.remove("class").and_then(|c| c.as_integer()) else {
            continue;
        };
        let class = u8::try_from(class).unwrap_or(0);
        let profile = StatProfile::of_class(class).unwrap_or(StatProfile::Warrior);
        let power = match profile {
            StatProfile::Warrior => "rage",
            StatProfile::Rogue => "energy",
            _ => "mana",
        };
        let name = format!("{profile:?}").to_lowercase();
        table
            .entry("stat_profile")
            .or_insert(toml::Value::String(name));
        table
            .entry("power")
            .or_insert(toml::Value::String(power.into()));
    }
    value.try_into()
}

/// The Callings tests added ([`add`]), for the rest of the process.
fn added() -> &'static RwLock<Vec<&'static CallingDef>> {
    static ADDED: RwLock<Vec<&'static CallingDef>> = RwLock::new(Vec::new());
    &ADDED
}

/// Adds the Callings in `toml` (the shape of `content/callings.toml`) alongside the shipped
/// ones, for the rest of the process; a key already present is left as it is. A test seam
/// (`Realm::with_callings`), so a player script can run on a power or profile no shipped Calling has.
pub(crate) fn add(toml: &str) {
    let parsed = parse_toml(toml).expect("the added Callings parse");
    let mut added = added().write().unwrap();
    for def in parsed.calling {
        if shipped()
            .iter()
            .chain(added.iter().copied())
            .all(|c| c.key != def.key)
        {
            added.push(Box::leak(Box::new(def)));
        }
    }
}

/// Every Calling: the shipped ones, then any a test added.
fn all() -> Vec<&'static CallingDef> {
    shipped()
        .iter()
        .chain(added().read().unwrap().iter().copied())
        .collect()
}

/// The Tier Cap of `tier`: the highest Cap among its Callings; 0 when no Calling is in it.
pub(crate) fn tier_cap(tier: u8) -> u8 {
    all()
        .into_iter()
        .filter(|c| c.tier == tier)
        .map(|c| c.cap)
        .max()
        .unwrap_or(0)
}

/// A Calling by its save key (`basic_warrior`).
pub fn calling(key: &str) -> Option<&'static CallingDef> {
    all().into_iter().find(|c| c.key == key)
}

/// A buff of any Calling, by its key in the sheet.
pub fn aura(key: &str) -> Option<&'static AuraDef> {
    all().iter().flat_map(|c| &c.auras).find(|a| a.key == key)
}

/// A current Calling's data.
pub(crate) fn of(calling: Calling) -> &'static CallingDef {
    self::calling(calling.key()).expect("every Calling has a table in content/callings.toml")
}

/// The first slot of the bar Battle Stance pages the main bar to (`GetBonusBarOffset` 1: Lua
/// actions 73-84).
const STANCE_BAR: u8 = 72;

/// A Calling's bar before the player arranges it: its skills learned from level 1, in order on
/// the stance bar, as `(slot, kind << 24 | spell)` with kind 0, a spell.
pub(crate) fn default_bar(calling: Calling) -> Vec<(u8, u32)> {
    (STANCE_BAR..)
        .zip(
            of(calling)
                .skills
                .iter()
                .filter(|s| s.learn_level <= 1)
                .map(|s| s.spell_id),
        )
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
        .into_iter()
        .flat_map(every_skill)
        .find(|s| s.spell_id == spell_id)
}

/// Whether a dodge opens a skill of `calling`'s (Overpower), so the client needs the marker.
pub(crate) fn opens_on_dodge(calling: Calling) -> bool {
    every_skill(of(calling)).any(|s| s.requires_target_dodged)
}

/// Every skill borrowed from another Class, once each: the spells the client rewrites to cost
/// rage (ADR 0003).
pub fn borrowed_skills() -> impl Iterator<Item = &'static Skill> {
    let mut seen = std::collections::HashSet::new();
    all()
        .into_iter()
        .flat_map(every_skill)
        .filter(|s| s.borrowed.is_some())
        .filter(move |s| seen.insert(s.spell_id))
}

/// Every WIP talent's rank spells, of every Calling: the spells the client names
/// "<Name> (WIP)" (ADR 0003).
pub fn wip_rank_spell_ids() -> impl Iterator<Item = u32> {
    all()
        .into_iter()
        .flat_map(|c| &c.talent_tabs)
        .flat_map(|tab| &tab.talents)
        .filter(|t| t.is_wip())
        .flat_map(|t| t.rank_spells.iter().copied())
}
