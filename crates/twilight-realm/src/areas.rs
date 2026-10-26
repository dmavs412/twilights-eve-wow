//! **Discovery**: the areas a character can discover (`content/areas.toml`, Kalimdor's rows of
//! `AreaTable.dbc`), and the explore bits of the client's map, which are all set.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::fields::PLAYER_EXPLORED_ZONES_1;

/// One area of `content/areas.toml`; its `zone`, `bit` and `name` are there for the reader.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct Area {
    pub(crate) id: u32,
}

#[derive(serde::Deserialize)]
struct File {
    area: Vec<Area>,
}

fn areas() -> &'static HashMap<u32, Area> {
    static AREAS: OnceLock<HashMap<u32, Area>> = OnceLock::new();
    AREAS.get_or_init(|| {
        let file: File = toml::from_str(include_str!("../content/areas.toml"))
            .expect("content/areas.toml parses");
        file.area.into_iter().map(|a| (a.id, a)).collect()
    })
}

/// The area with `AreaTable.dbc` id `id`, if a character can discover it.
pub(crate) fn get(id: u32) -> Option<&'static Area> {
    areas().get(&id)
}

/// A save's discoveries: the ids the content knows, ascending, each once.
pub(crate) fn known(mut ids: Vec<u32>) -> Vec<u32> {
    ids.retain(|&id| get(id).is_some());
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// The `PLAYER_EXPLORED_ZONES` dwords, every bit set: the whole world map shows explored from the
/// first login (Davis, 2026-10-26). Discovery still pays its XP and meets discover objectives.
pub(crate) fn explored_pairs() -> Vec<(u16, u32)> {
    (0..64)
        .map(|i| (PLAYER_EXPLORED_ZONES_1 + i, u32::MAX))
        .collect()
}
