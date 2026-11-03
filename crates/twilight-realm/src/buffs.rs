//! Buffs (ADR 0011): `content/buffs.toml`, the realm's own auras that are no Calling's and no
//! creature's. Each is a spell of its own; the realm reads its id, and the build script writes the
//! client row.

use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Deserialize)]
struct File {
    spell: Vec<Buff>,
}

/// The realm's side of a row: the client reads the name, icon and text from its patch.
#[derive(Deserialize)]
struct Buff {
    key: String,
    spell_id: u32,
}

fn shipped() -> &'static [Buff] {
    static FILE: OnceLock<Vec<Buff>> = OnceLock::new();
    FILE.get_or_init(|| {
        let file: File = toml::from_str(include_str!("../content/buffs.toml"))
            .expect("content/buffs.toml parses");
        file.spell
    })
}

/// The spell id of buff `key`; a key the file does not list panics.
pub(crate) fn spell_id(key: &str) -> u32 {
    shipped()
        .iter()
        .find(|b| b.key == key)
        .unwrap_or_else(|| panic!("content/buffs.toml lists no buff {key}"))
        .spell_id
}

#[cfg(test)]
mod tests {
    #[test]
    fn open_road_has_its_own_spell_id() {
        assert!(super::spell_id("open_road") >= 1_950_000);
    }
}
