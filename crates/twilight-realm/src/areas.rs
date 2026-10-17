//! **Discovery**: the areas a character can discover (`content/areas.toml`, Kalimdor's rows of
//! `AreaTable.dbc`), and the explore bits that lift their fog on the client's map.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::fields::PLAYER_EXPLORED_ZONES_1;

/// One area of `content/areas.toml`; its `zone` and `name` are there for the reader.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct Area {
    pub(crate) id: u32,
    /// Its bit in `PLAYER_EXPLORED_ZONES`.
    pub(crate) bit: u32,
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

/// The `PLAYER_EXPLORED_ZONES` dwords that `discovered` sets, each with every discovered bit in it.
pub(crate) fn explored_pairs(discovered: &[u32]) -> Vec<(u16, u32)> {
    let mut words: Vec<(u16, u32)> = Vec::new();
    for bit in discovered.iter().filter_map(|&id| get(id)).map(|a| a.bit) {
        let field = PLAYER_EXPLORED_ZONES_1 + (bit / 32) as u16;
        let mask = 1 << (bit % 32);
        match words.iter_mut().find(|(f, _)| *f == field) {
            Some((_, word)) => *word |= mask,
            None => words.push((field, mask)),
        }
    }
    words.sort_unstable();
    words
}

/// The `PLAYER_EXPLORED_ZONES` dword that holds `area`'s bit, as `discovered` sets it.
pub(crate) fn explored_pair(discovered: &[u32], area: &Area) -> (u16, u32) {
    let field = PLAYER_EXPLORED_ZONES_1 + (area.bit / 32) as u16;
    let word = explored_pairs(discovered)
        .into_iter()
        .find(|&(f, _)| f == field)
        .map_or(0, |(_, w)| w);
    (field, word)
}
