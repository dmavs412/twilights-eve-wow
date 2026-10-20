//! Creature Spells (ADR 0008): what a creature may cast in a fight, hand-written in
//! `content/creature_spells.toml`. A template names its spells by key; the mob's tick casts them
//! through the same aura machinery the player's skills use, so nothing here names a spell.

use std::sync::OnceLock;

use serde::Deserialize;

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
    pub(crate) cooldown_ms: u32,
    /// Yards to the player; a self-cast (heal) has no range to check.
    pub(crate) range_max: f32,
    /// Percent per decision while the creature fights and the spell is ready.
    pub(crate) chance: u32,
    /// The spell is cast only while the caster's health is under this percent of its maximum.
    #[serde(default)]
    pub(crate) below_health_pct: Option<u32>,
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

#[cfg(test)]
mod tests {
    use super::{all, healed};

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
