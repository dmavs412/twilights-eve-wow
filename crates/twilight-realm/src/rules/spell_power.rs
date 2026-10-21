//! Worn Spell Power's share in a spell, by vanilla's cast-time coefficient (vmangos
//! `SpellMgr::CalculateDefaultCoefficient`, patch 1.12, spec #272). A direct spell takes its
//! base cast time over 3.5 s, never under 1.5 s (an instant counts as 1.5 s) nor over 3.5 s. A
//! periodic effect takes its duration over 15 s, capped there, spread evenly over its ticks.
//! Healing uses the same shares. Everything rounds down at each step.

/// The cast time a direct spell's share is at least measured by: an instant's 1.5 s.
pub const DIRECT_MIN_MS: u32 = 1500;
/// The cast time a direct spell's share is at most measured by: 3.5 s takes it all.
pub const DIRECT_MAX_MS: u32 = 3500;
/// The duration a periodic effect's share is at most measured by: 15 s takes it all.
pub const PERIODIC_MAX_MS: u32 = 15000;

/// What `spell_power` adds to one direct hit or heal of a spell whose base cast time is
/// `base_cast_ms` (0 for an instant).
pub fn spell_power_bonus(spell_power: u32, base_cast_ms: u32) -> u32 {
    spell_power * base_cast_ms.clamp(DIRECT_MIN_MS, DIRECT_MAX_MS) / DIRECT_MAX_MS
}

/// What `spell_power` adds to each of the `ticks` ticks of a periodic effect lasting
/// `duration_ms`; nothing when it never ticks.
pub fn periodic_spell_power_bonus(spell_power: u32, duration_ms: u32, ticks: u32) -> u32 {
    if ticks == 0 {
        return 0;
    }
    spell_power * duration_ms.min(PERIODIC_MAX_MS) / PERIODIC_MAX_MS / ticks
}
