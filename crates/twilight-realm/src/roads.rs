//! Roads (spec #560): `content/roads.toml`, the terrain textures that are a road or a street on
//! each map, and the run speed Open Road adds on them.

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize)]
struct File {
    open_road: OpenRoad,
    map: Vec<Map>,
}

#[derive(Deserialize)]
struct OpenRoad {
    speed_pct: u32,
}

#[derive(Deserialize)]
struct Map {
    id: u32,
    textures: Vec<String>,
}

/// The road textures by map, and Open Road's speed.
#[derive(Clone, Debug)]
pub(crate) struct Roads {
    /// Each map's road names, lowercase and without the extension.
    maps: HashMap<u32, Vec<String>>,
    /// The run speed percent a road adds out of combat.
    pub(crate) speed_pct: u32,
}

impl Default for Roads {
    /// The shipped file.
    fn default() -> Self {
        Self::parse(include_str!("../content/roads.toml"))
    }
}

impl Roads {
    /// The roads in `text`, the shape of `content/roads.toml`.
    pub(crate) fn parse(text: &str) -> Self {
        let file: File = toml::from_str(text).expect("content/roads.toml parses");
        let mut maps: HashMap<u32, Vec<String>> = HashMap::new();
        for map in file.map {
            maps.entry(map.id)
                .or_default()
                .extend(map.textures.iter().map(|t| bare(t)));
        }
        Self {
            maps,
            speed_pct: file.open_road.speed_pct,
        }
    }

    /// Whether `texture`, an ADT texture name, is a road on `map`: case-insensitive, with or
    /// without its extension.
    pub(crate) fn is_road(&self, map: u32, texture: &str) -> bool {
        let texture = bare(texture);
        self.maps
            .get(&map)
            .is_some_and(|roads| roads.iter().any(|r| *r == texture))
    }
}

/// `texture` lowercase, its extension stripped.
fn bare(texture: &str) -> String {
    let name_at = texture.rfind(['\\', '/']).map_or(0, |i| i + 1);
    let stem = match texture[name_at..].rfind('.') {
        Some(dot) => &texture[..name_at + dot],
        None => texture,
    };
    stem.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::Roads;

    #[test]
    fn a_road_matches_in_any_case_with_or_without_its_extension() {
        let roads = Roads::default();
        assert!(roads.is_road(0, "Tileset\\Elwynn\\ElwynnDirtBase2"));
        assert!(roads.is_road(0, "TILESET\\ELWYNN\\elwynndirtbase2.blp"));
        assert!(!roads.is_road(0, "Tileset\\Elwynn\\ElwynnGrassBase"));
        assert!(
            !roads.is_road(1, "Tileset\\Elwynn\\ElwynnDirtBase2"),
            "map 1 lists none"
        );
        assert_eq!(roads.speed_pct, 30);
    }
}
