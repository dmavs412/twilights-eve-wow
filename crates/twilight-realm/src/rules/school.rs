//! Damage schools: what a blow's school takes off it, and how the client numbers it
//! (`SpellSchools`, `SPELL_SCHOOL0_CAP`..: 0 physical, 1 holy, 2 fire, 3 nature, 4 frost, 5 shadow, 6 arcane).

use serde::Deserialize;

use super::melee::armor_reduced;
use super::talents::crit_damage;

/// The school a blow deals its damage in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum School {
    #[default]
    Physical,
    /// Not reduced by armour (`Unit::CalcArmorReducedDamage` is for physical damage only).
    Holy,
    /// Magic, like holy: armour does not touch it.
    Fire,
    /// Magic: armour does not touch it.
    Nature,
    /// Magic: armour does not touch it.
    Frost,
    /// Magic: armour does not touch it.
    Shadow,
    /// Magic: armour does not touch it.
    Arcane,
}

impl School {
    /// The school's number in a damage log, which the client turns into its word and colour.
    pub fn index(self) -> u8 {
        match self {
            School::Physical => 0,
            School::Holy => 1,
            School::Fire => 2,
            School::Nature => 3,
            School::Frost => 4,
            School::Shadow => 5,
            School::Arcane => 6,
        }
    }
}

/// `raw` damage of `school` after the victim's `armor`, from an attacker of `attacker_level`:
/// physical loses what armour takes ([`armor_reduced`]); holy loses nothing.
pub fn mitigated(school: School, raw: u32, armor: u32, attacker_level: u8) -> u32 {
    match school {
        School::Physical => armor_reduced(raw, armor, attacker_level),
        School::Holy
        | School::Fire
        | School::Nature
        | School::Frost
        | School::Shadow
        | School::Arcane => raw,
    }
}

impl School {
    /// A blow of this school that avoids nothing by the melee table: dodge, parry and block are
    /// for physical blows only.
    pub fn is_magic(self) -> bool {
        self != School::Physical
    }
}

/// A crit on `damage` (after mitigation) in `school`: physical does 200% ([`crit_damage`]), every
/// other school 150%, and `bonus_pct`% more of the whole crit on either.
pub fn crit_scaled(school: School, damage: u32, bonus_pct: u32) -> u32 {
    if school.is_magic() {
        super::talents::crit_multiplied(damage, 150, bonus_pct)
    } else {
        crit_damage(damage, bonus_pct)
    }
}
