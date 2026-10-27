//! Creature Spells (ADR 0008): what a creature may cast in a fight, hand-written in
//! `content/creature_spells.toml`. A template names its spells by key; the mob's tick casts them
//! through the same aura machinery the player's skills use, so nothing here names a spell.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::rng::Rng;
use crate::rules::school::School;
use crate::rules::skills::Linear;

/// What a Creature Spell does, in the skills' own terms where they have one.
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum CreatureEffect {
    /// A curse on the player: `tick` every `tick_ms`, for `duration_ms`.
    PeriodicDamage {
        tick_ms: u32,
        duration_ms: u32,
        tick: Linear,
    },
    /// Restores the caster's health by `pct_max_health` percent of its maximum. The format stays
    /// open to a flat `base` later.
    Heal { pct_max_health: u32 },
    /// The player runs `pct` percent slower for `duration_ms`.
    Snare { pct: u32, duration_ms: u32 },
    /// The caster runs to its target at `speed_yd_s` along the floor, then lands one weapon hit
    /// worth `bonus_pct` percent more than a white swing.
    Charge { speed_yd_s: f32, bonus_pct: u32 },
    /// When the cast lands, one blow at the character the creature fights, rolled on the table a
    /// swing uses, worth `pct_of_swing` percent of the caster's average swing. It takes the place
    /// of the next swing.
    Hit { pct_of_swing: u32 },
    /// When the cast lands, every character in the caster's Phase within `radius` yards takes
    /// `damage` of the spell's school, unless a cover pillar stands in the line to the caster.
    Gaze { damage: u32, radius: f32 },
    /// When the cast lands, every character in the caster's Phase within `radius` yards of the
    /// caster takes `damage` of the spell's school, cover or not.
    Nova { damage: u32, radius: f32 },
    /// The target flees from the caster for `duration_ms` and then gets control back.
    Fear { duration_ms: u32 },
    /// When the cast lands, every character in the caster's Phase within `radius` yards of the
    /// caster can't move for `duration_ms`. They can still cast and swing.
    Root { radius: f32, duration_ms: u32 },
    /// When the cast lands, a patch of `radius` yards appears at the target's position. Every
    /// `tick_ms` for `duration_ms`, each character in the Phase within it takes `tick` damage of
    /// the spell's school.
    GroundPatch {
        radius: f32,
        duration_ms: u32,
        tick_ms: u32,
        tick: Linear,
    },
    /// A self-cast: the caster gains a shield of `amount` that damage comes off first, for
    /// `duration_ms`.
    Absorb { amount: u32, duration_ms: u32 },
    /// The target's armour is `amount` lower for each stack, never below 0. Each landing adds a
    /// stack, up to `max_stacks`, and starts `duration_ms` over; when it runs out every stack
    /// goes.
    ArmorDebuff {
        amount: u32,
        max_stacks: u32,
        duration_ms: u32,
    },
}

impl CreatureEffect {
    /// Whether the spell lands on its caster, so it has no range or line of sight to check.
    pub(crate) fn on_caster(&self) -> bool {
        matches!(self, Self::Heal { .. } | Self::Absorb { .. })
    }
}

/// A Creature Spell's cooldown: `15000`, or `[5000, 9000]`, rolled anew at each cast's end.
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(untagged)]
pub(crate) enum Cooldown {
    Fixed(u32),
    Range([u32; 2]),
}

impl Cooldown {
    /// Milliseconds until the spell is ready again, after a cast that went off.
    pub(crate) fn rolled(self, rng: &mut Rng) -> u32 {
        match self {
            Self::Fixed(ms) => ms,
            Self::Range([lo, hi]) => rng.urand(lo, hi),
        }
    }

    /// Milliseconds until the spell is ready again, after a cast that was cut short.
    pub(crate) fn shortest(self) -> u32 {
        match self {
            Self::Fixed(ms) => ms,
            Self::Range([lo, _]) => lo,
        }
    }
}

/// Whom a Creature Spell is cast at.
#[derive(Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SpellTarget {
    /// The character the creature fights.
    #[default]
    Current,
    /// A random living character in the caster's Phase within `range_max`.
    Random,
}

/// One Creature Spell.
#[derive(Deserialize, Debug)]
pub(crate) struct CreatureSpell {
    pub(crate) key: String,
    /// Its own spell id (ADR 0011), whose client row `build_spells.py` writes from the same
    /// file's `name`, `icon`, `visual` and `aura_description`: the cast bar, the icon, the debuff.
    pub(crate) spell_id: u32,
    pub(crate) school: School,
    pub(crate) cast_time_ms: u32,
    pub(crate) cooldown_ms: Cooldown,
    /// Yards to the player; a self-cast (heal) has no range to check.
    pub(crate) range_max: f32,
    /// Yards the player must be beyond: a charge is not cast from melee reach.
    #[serde(default)]
    pub(crate) range_min: f32,
    /// The spell is cast only while a dead creature lies within this many yards of the caster.
    #[serde(default)]
    pub(crate) corpse_within: Option<f32>,
    /// Percent per decision while the creature fights and the spell is ready.
    pub(crate) chance: u32,
    /// The spell is cast only while the caster's health is under this percent of its maximum.
    #[serde(default)]
    pub(crate) below_health_pct: Option<u32>,
    /// The stacks each cast adds to its Phased Dungeon's meter.
    #[serde(default)]
    pub(crate) noise_on_cast: u32,
    /// The spell can't be cast until this long after the creature enters combat.
    #[serde(default)]
    pub(crate) first_cast_ms: Option<u32>,
    /// Sent as a raid warning to every character in the caster's Phase when the cast starts.
    #[serde(default)]
    pub(crate) warning: Option<String>,
    /// Whom the cast goes on; the creature keeps fighting the same character either way.
    #[serde(default)]
    pub(crate) target: SpellTarget,
    pub(crate) effect: CreatureEffect,
}

#[derive(Deserialize)]
struct File {
    spell: Vec<CreatureSpell>,
}

fn all() -> &'static [CreatureSpell] {
    static SPELLS: OnceLock<Vec<CreatureSpell>> = OnceLock::new();
    SPELLS.get_or_init(|| {
        let file: File = toml::from_str(include_str!("../content/creature_spells.toml"))
            .expect("content/creature_spells.toml parses");
        file.spell
    })
}

/// Health a heal of `pct`% of `max` restores to a unit at `health`: never past `max`.
pub(crate) fn healed(health: u32, max: u32, pct: u32) -> u32 {
    (max * pct / 100).min(max.saturating_sub(health))
}

/// Where a charge from `from` at `to` ends: `stop` yards short of its target, or where it began
/// when it starts closer than that.
pub(crate) fn charge_dest(from: [f32; 3], to: [f32; 3], stop: f32) -> [f32; 3] {
    let d = crate::space::distance(from, to);
    if d <= stop {
        return from;
    }
    let t = (d - stop) / d;
    std::array::from_fn(|i| from[i] + (to[i] - from[i]) * t)
}

#[cfg(test)]
mod tests {
    use super::{all, charge_dest, healed};

    #[test]
    fn a_charge_ends_short_of_its_target_and_never_backs_away() {
        let end = charge_dest([0.0; 3], [20.0, 0.0, 0.0], 5.0);
        assert!((end[0] - 15.0).abs() < 1e-4, "{end:?}");
        assert_eq!(charge_dest([0.0; 3], [3.0, 0.0, 0.0], 5.0), [0.0; 3]);
    }

    #[test]
    fn every_creature_spell_is_a_spell_of_its_own() {
        let mut ids: Vec<u32> = all().iter().map(|s| s.spell_id).collect();
        assert!(
            ids.iter().all(|&id| id >= 1_000_000),
            "from-scratch ids: {ids:?}"
        );
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all().len(), "one id per spell");
    }

    #[test]
    fn a_heal_restores_its_percent_and_never_overheals() {
        assert_eq!(healed(50, 100, 15), 15);
        assert_eq!(healed(95, 100, 15), 5);
        assert_eq!(healed(100, 100, 15), 0);
    }
}

/// The Creature Spell `key`.
pub(crate) fn get(key: &str) -> Option<&'static CreatureSpell> {
    all().iter().find(|s| s.key == key)
}

/// The spells of `keys`; an unknown key fails loudly, as content should.
pub(crate) fn resolve(keys: &[String]) -> Vec<&'static CreatureSpell> {
    keys.iter()
        .map(|k| get(k).unwrap_or_else(|| panic!("unknown creature spell {k}")))
        .collect()
}

/// Which spells each creature entry casts beyond its template's: a test seam
/// (`Realm::with_creature_spells`), held per realm.
#[derive(Default, Clone)]
pub(crate) struct Casters(Vec<(u32, Vec<&'static CreatureSpell>)>);

impl Casters {
    pub(crate) fn add(&mut self, entry: u32, keys: &[&str]) {
        let keys: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
        self.0.push((entry, resolve(&keys)));
    }

    /// What creature `entry` casts: what this realm gave it, else its template's.
    pub(crate) fn of(&self, entry: u32) -> Vec<&'static CreatureSpell> {
        if let Some((_, s)) = self.0.iter().find(|(e, _)| *e == entry) {
            return s.clone();
        }
        crate::mobs::template(entry).map_or_else(Vec::new, |t| resolve(&t.spells))
    }
}
