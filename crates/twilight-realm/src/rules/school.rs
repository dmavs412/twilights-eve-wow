//! Damage schools: what a blow's school takes off it, and how the client numbers it
//! (`SpellSchools`, `SPELL_SCHOOL0_CAP`..: 0 physical, 1 holy).

use super::melee::armor_reduced;

/// The school a blow deals its damage in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum School {
    Physical,
    /// Not reduced by armour (`Unit::CalcArmorReducedDamage` is for physical damage only).
    Holy,
}

impl School {
    /// The school's number in a damage log, which the client turns into its word and colour.
    pub fn index(self) -> u8 {
        match self {
            School::Physical => 0,
            School::Holy => 1,
        }
    }
}

/// `raw` damage of `school` after the victim's `armor`, from an attacker of `attacker_level`:
/// physical loses what armour takes ([`armor_reduced`]); holy loses nothing.
pub fn mitigated(school: School, raw: u32, armor: u32, attacker_level: u8) -> u32 {
    match school {
        School::Physical => armor_reduced(raw, armor, attacker_level),
        School::Holy => raw,
    }
}
