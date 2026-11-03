//! Trinket Spells (spec #458): what a worn trinket does when it is clicked, and the buffs it puts
//! up, hand-written in `content/trinket_spells.toml`. A Gear row names its spell in `use`; nothing
//! here names a trinket.

use std::ops::Range;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::callings::AuraDef;

/// The ids a Trinket Spell or its buff may take.
const IDS: Range<u32> = 1_960_000..1_970_000;

/// What a click does beyond its cooldown. The trinkets' own tickets add the kinds.
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TrinketEffect {}

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
        let file: File = toml::from_str(include_str!("../content/trinket_spells.toml"))
            .expect("content/trinket_spells.toml parses");
        check(&file);
        file
    })
}

/// Refuses ids outside [`IDS`] and an id two rows share.
fn check(file: &File) {
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
#[allow(dead_code)] // the trinkets' own tickets name their buffs
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
