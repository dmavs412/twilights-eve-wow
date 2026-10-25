//! The map canvas (#410): the map and the `AreaTable` zone ids drawn on one world-map rect, read
//! from `data/twilights-eve/map_canvas.json`. The file order is the draw order, later on top.

use std::sync::OnceLock;

const MAP_CANVAS: &str = include_str!("../../../data/twilights-eve/map_canvas.json");

/// `{"map": 0, "zones": [..]}`: the map and its canvas zone ids in the file's order. Any other
/// key, the `"//"` note among them, is ignored.
fn parse(text: &str) -> (u32, Vec<u32>) {
    let v: serde_json::Value = serde_json::from_str(text).expect("map_canvas.json parses");
    let num = |v: &serde_json::Value| v.as_u64().expect("map_canvas.json holds numbers") as u32;
    let zones: Vec<u32> = v["zones"]
        .as_array()
        .expect("map_canvas.json has zones")
        .iter()
        .map(num)
        .collect();
    (num(&v["map"]), zones)
}

fn loaded() -> &'static (u32, Vec<u32>) {
    static CANVAS: OnceLock<(u32, Vec<u32>)> = OnceLock::new();
    CANVAS.get_or_init(|| parse(MAP_CANVAS))
}

/// The canvas's map and its `AreaTable` zone ids, in draw order.
pub fn canvas() -> (u32, &'static [u32]) {
    let (map, zones) = loaded();
    (*map, zones)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_canvas_is_the_four_zones_on_map_0_in_draw_order() {
        assert_eq!(canvas(), (0, &[10, 44, 12, 1519][..]));
    }
}
