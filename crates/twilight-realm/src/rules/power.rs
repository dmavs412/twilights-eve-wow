//! A character's power: which kind its Class uses, how big the bar is, and how it regenerates
//! (vmangos `Player::UpdateMaxPower`, `Player::Regenerate`, `Player::OCTRegenMPPerSpirit`;
//! `docs/reference/vanilla-combat-rules.md` §5.5). Rage's own rules stay in [`super::rage`].

use crate::tables;

/// `ChrClasses.dbc` ids.
pub mod class {
    pub const WARRIOR: u8 = 1;
    pub const PALADIN: u8 = 2;
    pub const HUNTER: u8 = 3;
    pub const ROGUE: u8 = 4;
    pub const PRIEST: u8 = 5;
    pub const SHAMAN: u8 = 7;
    pub const MAGE: u8 = 8;
    pub const WARLOCK: u8 = 9;
    pub const DRUID: u8 = 11;
}

/// The power a Class runs on. Stored as the client field carries it: rage in tenths, energy
/// and mana whole. Named in content as `rage`, `energy` or `mana`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerKind {
    Rage,
    Energy,
    Mana,
}

impl PowerKind {
    /// Warrior rage, Rogue energy, every other Class mana (Druid forms are not modelled).
    pub fn of_class(class: u8) -> Self {
        match class {
            class::WARRIOR => Self::Rage,
            class::ROGUE => Self::Energy,
            _ => Self::Mana,
        }
    }

    /// `Powers`: the client's id, in the unit's bytes, the energize logs and the field offset
    /// (`UNIT_FIELD_POWER1 + id` holds the current value).
    pub fn id(self) -> u32 {
        match self {
            Self::Mana => 0,
            Self::Rage => 1,
            Self::Energy => 3,
        }
    }

    /// `points` as the client field stores them: rage in tenths, energy and mana whole.
    pub fn stored(self, points: u32) -> u32 {
        match self {
            Self::Rage => points * 10,
            Self::Energy | Self::Mana => points,
        }
    }
}

/// A full energy bar.
pub const ENERGY_MAX: u32 = 100;

/// Energy gained every regen tick, in and out of combat (vmangos `Regenerate(POWER_ENERGY)`).
pub const ENERGY_PER_TICK: u32 = 20;

/// Seconds between regen ticks for every kind (`REGEN_TIME_PLAYER_FULL`).
pub const REGEN_TICK_SECS: f32 = 2.0;

/// Seconds after a cast that spent mana during which spirit regenerates none of it
/// (`Player::IsUnderLastManaUseEffect`).
pub const MANA_PAUSE_SECS: f32 = 5.0;

/// The most a bar can hold at `max`: rage and energy have a fixed cap, mana its computed max.
pub fn cap(kind: PowerKind, max: u32) -> u32 {
    match kind {
        PowerKind::Rage => super::rage::MAX,
        PowerKind::Energy => ENERGY_MAX,
        PowerKind::Mana => max,
    }
}

/// Mana from intellect: the first 20 points give 1 each, every point after 15
/// (`Player::GetManaBonusFromIntellect`).
pub fn mana_from_intellect(intellect: u32) -> u32 {
    intellect.min(20) + intellect.saturating_sub(20) * 15
}

/// The mana bar of `class` at `level` with `intellect`: the Class's base mana for the level plus
/// the intellect bonus; 0 for a Class that does not run on mana.
pub fn max_mana(class: u8, level: u8, intellect: u32) -> u32 {
    if PowerKind::of_class(class) != PowerKind::Mana {
        return 0;
    }
    tables::at(&tables::levels().class(class).base_mana, level) + mana_from_intellect(intellect)
}

/// Mana regained from `spirit` every regen tick while not under the five-second pause, by the
/// Class's constants (`Player::OCTRegenMPPerSpirit`); 0 for a Class that does not run on mana.
pub fn mana_per_tick(class: u8, spirit: u32) -> f64 {
    let spirit = f64::from(spirit);
    match class {
        class::MAGE | class::PRIEST => spirit / 4.0 + 12.5,
        class::SHAMAN => spirit / 5.0 + 17.0,
        class::DRUID | class::HUNTER | class::PALADIN | class::WARLOCK => spirit / 5.0 + 15.0,
        _ => 0.0,
    }
}
