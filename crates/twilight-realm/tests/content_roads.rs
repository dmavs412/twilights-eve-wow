//! `content/roads.toml`, the terrain textures that are a road or a street per map (spec #560).

use std::path::PathBuf;

#[derive(serde::Deserialize)]
struct File {
    map: Vec<Map>,
}

#[derive(serde::Deserialize)]
struct Map {
    id: u32,
    textures: Vec<String>,
}

#[test]
fn map_0_lists_road_textures_without_extensions() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("content/roads.toml");
    let text = std::fs::read_to_string(&path).expect("content/roads.toml");
    let file = toml::from_str::<File>(&text).expect("it parses");
    let map = file.map.iter().find(|m| m.id == 0).expect("map 0");
    assert!(!map.textures.is_empty(), "map 0 lists a road texture");
    for t in &map.textures {
        let name = t.rsplit(['\\', '/']).next().unwrap_or(t);
        assert!(!name.contains('.'), "{t} carries an extension");
    }
}
