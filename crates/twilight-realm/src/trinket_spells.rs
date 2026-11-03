//! Trinket Spells (spec #458): what a worn trinket does when it is clicked, and the buffs it puts
//! up, hand-written in `content/trinket_spells.toml`. A Gear row names its spell in `use`; nothing
//! here names a trinket.

use std::ops::Range;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::callings::{AuraDef, AuraEffect, BORROWED_FAMILY_BITS};

/// The ids a Trinket Spell or its buff may take.
const IDS: Range<u32> = 1_960_000..1_970_000;

/// What a click does beyond its cooldown. Every kind is generic: no kind names a trinket.
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum TrinketEffect {
    /// One of `faces` drawn evenly, shown in chat as a `/random` of 1 to their count.
    Roll { faces: Vec<TrinketEffect> },
    /// The wearer loses `pct` percent of their current health, rounded down, and never drops
    /// below 1.
    LoseCurrentHealthPct { pct: u32 },
    /// The wearer heals `pct` percent of their maximum health.
    HealMaxHealthPct { pct: u32 },
    /// The wearer's power bar (rage, energy or mana) fills by `pct` percent of its maximum.
    EnergizeMaxPowerPct { pct: u32 },
    /// The `[[aura]]` row `key` goes on the wearer.
    Aura { key: String },
}

/// One Trinket Spell: a worn trinket's click.
#[derive(Deserialize, Debug)]
pub(crate) struct TrinketSpell {
    pub(crate) key: String,
    /// Its own id, whose client row `build_spells.py` writes from the same file's name, icon and
    /// description.
    pub(crate) spell_id: u32,
    /// From the click until the item can be clicked again.
    pub(crate) cooldown_ms: u32,
    /// What the click does; with none it only starts the cooldown.
    #[serde(default)]
    pub(crate) effect: Option<TrinketEffect>,
}

#[derive(Deserialize)]
struct File {
    spell: Vec<TrinketSpell>,
    #[serde(default)]
    aura: Vec<AuraDef>,
}

fn file() -> &'static File {
    static FILE: OnceLock<File> = OnceLock::new();
    FILE.get_or_init(|| {
        let mut file: File = toml::from_str(include_str!("../content/trinket_spells.toml"))
            .expect("content/trinket_spells.toml parses");
        check(&file);
        // A trinket's faster casts reach every Calling skill's cast bar: each takes its
        // SpellFamilyFlags bit from 33 up.
        for effect in file.aura.iter_mut().flat_map(|a| &mut a.effects) {
            if let AuraEffect::ModCastSpeedPct { family_bits, .. } = effect {
                if family_bits.is_empty() {
                    *family_bits = BORROWED_FAMILY_BITS.collect();
                }
            }
        }
        file
    })
}

/// Refuses ids outside [`IDS`], an id two rows share, a roll with no faces, and an effect naming
/// an aura the file does not have.
fn check(file: &File) {
    fn effects(effect: &TrinketEffect) -> Vec<&TrinketEffect> {
        match effect {
            TrinketEffect::Roll { faces } => std::iter::once(effect)
                .chain(faces.iter().flat_map(effects))
                .collect(),
            _ => vec![effect],
        }
    }
    for spell in &file.spell {
        for effect in spell.effect.iter().flat_map(effects) {
            match effect {
                TrinketEffect::Roll { faces } => {
                    assert!(
                        !faces.is_empty(),
                        "trinket spell {}: a roll has no faces",
                        spell.key
                    );
                }
                TrinketEffect::Aura { key } => assert!(
                    file.aura.iter().any(|a| &a.key == key),
                    "trinket spell {}: no [[aura]] row {key}",
                    spell.key
                ),
                _ => {}
            }
        }
    }
    let mut ids: Vec<(u32, &str)> = file
        .spell
        .iter()
        .map(|s| (s.spell_id, s.key.as_str()))
        .chain(file.aura.iter().map(|a| (a.spell_id, a.key.as_str())))
        .collect();
    for (id, key) in &ids {
        assert!(
            IDS.contains(id),
            "trinket spell {key}: id {id} is outside {IDS:?}"
        );
    }
    ids.sort_unstable();
    for pair in ids.windows(2) {
        assert_ne!(
            pair[0].0, pair[1].0,
            "{} and {} share an id",
            pair[0].1, pair[1].1
        );
    }
}

/// The Trinket Spell `spell_id`, if it is one.
pub(crate) fn spell(spell_id: u32) -> Option<&'static TrinketSpell> {
    file().spell.iter().find(|s| s.spell_id == spell_id)
}

/// The trinket buff `key`, which an effect puts up through `put_aura`.
pub(crate) fn aura(key: &str) -> Option<&'static AuraDef> {
    file().aura.iter().find(|a| a.key == key)
}

#[cfg(test)]
mod tests {
    use super::{file, spell};

    #[test]
    fn the_file_loads_with_its_ids_in_range_and_unique() {
        assert!(!file().spell.is_empty());
        let first = &file().spell[0];
        assert_eq!(spell(first.spell_id).map(|s| &s.key), Some(&first.key));
        assert!(spell(0).is_none());
    }
}
