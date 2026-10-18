//! Creature Spells (ADR 0008): what a creature may cast in a fight, hand-written in
//! `content/creature_spells.toml`. A template names its spells by key; the mob's tick casts them
//! through the same aura machinery the player's skills use, so nothing here names a spell.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::callings::Effect;
use crate::rules::school::School;

/// One Creature Spell.
#[derive(Deserialize, Debug)]
pub(crate) struct CreatureSpell {
    pub(crate) key: String,
    /// What the client shows: the cast bar, the icon, the debuff.
    pub(crate) client_spell_id: u32,
    pub(crate) school: School,
    pub(crate) cast_time_ms: u32,
    pub(crate) cooldown_ms: u32,
    pub(crate) range_max: f32,
    /// Percent per decision while the creature fights and the spell is ready.
    pub(crate) chance: u32,
    /// What it does to the target, in the skills' own terms.
    pub(crate) effect: Effect,
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
        for s in &file.spell {
            assert!(
                matches!(s.effect, Effect::PeriodicDamage { .. }),
                "creature spell {}: only periodic_damage is cast by creatures yet",
                s.key
            );
        }
        file.spell
    })
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
