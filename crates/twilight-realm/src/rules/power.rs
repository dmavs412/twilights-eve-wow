//! A character's power: the kinds a Calling's bar can run on, how big the bar is, and how it
//! regenerates
//! (vmangos `Player::UpdateMaxPower`, `Player::Regenerate`, `Player::OCTRegenMPPerSpirit`;
//! `docs/reference/vanilla-combat-rules.md` §5.5). Rage's own rules stay in [`super::rage`].

use crate::tables;

/// `ChrClasses.dbc` ids: the vanilla class tables a Calling's `stat_profile` names.
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

/// The power a Calling's bar runs on, its `power` (ADR 0011). Stored as the client field carries
/// it: rage in tenths, energy, focus and mana whole. Named in content as `rage`, `energy`,
/// `focus` or `mana`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerKind {
    Rage,
    Energy,
    Focus,
    Mana,
}

impl PowerKind {
    /// `Powers`: the client's id, in the unit's bytes, the energize logs and the field offset
    /// (`UNIT_FIELD_POWER1 + id` holds the current value).
    pub fn id(self) -> u32 {
        match self {
            Self::Mana => 0,
            Self::Rage => 1,
            Self::Focus => 2,
            Self::Energy => 3,
        }
    }

    /// `points` as the client field stores them: rage in tenths, the rest whole.
    pub fn stored(self, points: u32) -> u32 {
        match self {
            Self::Rage => points * 10,
            Self::Energy | Self::Focus | Self::Mana => points,
        }
    }
}

/// A full energy bar.
pub const ENERGY_MAX: u32 = 100;

/// Energy gained every regen tick, in and out of combat (vmangos `Regenerate(POWER_ENERGY)`).
pub const ENERGY_PER_TICK: u32 = 20;

/// A full focus bar, as a vanilla pet's.
pub const FOCUS_MAX: u32 = 100;

/// Focus gained every regen tick, in and out of combat: a vanilla pet's five a second.
pub const FOCUS_PER_TICK: u32 = 10;

/// Seconds between regen ticks for every kind (`REGEN_TIME_PLAYER_FULL`).
pub const REGEN_TICK_SECS: f32 = 2.0;

/// Seconds after a cast that spent mana during which spirit regenerates none of it
/// (`Player::IsUnderLastManaUseEffect`).
pub const MANA_PAUSE_SECS: f32 = 5.0;

/// The most a bar can hold at `max`: rage has a fixed cap, energy its 100 and what talents add,
/// focus its 100, mana its computed max.
pub fn cap(kind: PowerKind, max: u32, energy_bonus: u32) -> u32 {
    match kind {
        PowerKind::Rage => super::rage::MAX,
        PowerKind::Energy => ENERGY_MAX + energy_bonus,
        PowerKind::Focus => FOCUS_MAX,
        PowerKind::Mana => max,
    }
}

/// Mana from intellect: the first 20 points give 1 each, every point after 15
/// (`Player::GetManaBonusFromIntellect`).
pub fn mana_from_intellect(intellect: u32) -> u32 {
    intellect.min(20) + intellect.saturating_sub(20) * 15
}

/// The base mana of the `class` table at `level`, before intellect: what a `pct_base` cost is a
/// share of; 0 for a table with no mana (Warrior, Rogue).
pub fn base_mana(class: u8, level: u8) -> u32 {
    tables::at(&tables::levels().class(class).base_mana, level)
}

/// The mana bar of the `class` table at `level` with `intellect`: its base mana for the level
/// plus the intellect bonus; 0 for a table with no mana.
pub fn max_mana(class: u8, level: u8, intellect: u32) -> u32 {
    match base_mana(class, level) {
        0 => 0,
        base => base + mana_from_intellect(intellect),
    }
}

/// Mana regained from `spirit` every regen tick while not under the five-second pause, by the
/// `class` table's constants (`Player::OCTRegenMPPerSpirit`); 0 for a table with no mana.
pub fn mana_per_tick(class: u8, spirit: u32) -> f64 {
    let spirit = f64::from(spirit);
    match class {
        class::MAGE | class::PRIEST => spirit / 4.0 + 12.5,
        class::SHAMAN => spirit / 5.0 + 17.0,
        class::DRUID | class::HUNTER | class::PALADIN | class::WARLOCK => spirit / 5.0 + 15.0,
        _ => 0.0,
    }
}
