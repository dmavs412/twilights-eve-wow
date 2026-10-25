//! The map canvas (#410): the map and the `AreaTable` zone ids drawn on one world-map rect, read
//! from `data/twilights-eve/map_canvas.json`. The file order is the draw order, later on top.

use std::sync::OnceLock;

const MAP_CANVAS: &str = include_str!("../../../data/twilights-eve/map_canvas.json");

/// The parsed file: the map, its canvas zones in draw order, and the held zones.
struct MapCanvas {
    map: u32,
    zones: Vec<u32>,
    held: Vec<u32>,
}

/// `{"map": 0, "zones": [..], "held": [..]}`; `held` may be absent. Any other key, the `"//"`
/// note among them, is ignored.
fn parse(text: &str) -> MapCanvas {
    let v: serde_json::Value = serde_json::from_str(text).expect("map_canvas.json parses");
    let num = |v: &serde_json::Value| v.as_u64().expect("map_canvas.json holds numbers") as u32;
    let ids = |v: &serde_json::Value| -> Vec<u32> {
        v.as_array()
            .map(|a| a.iter().map(num).collect())
            .unwrap_or_default()
    };
    assert!(v["zones"].is_array(), "map_canvas.json has zones");
    MapCanvas {
        map: num(&v["map"]),
        zones: ids(&v["zones"]),
        held: ids(&v["held"]),
    }
}

fn loaded() -> &'static MapCanvas {
    static CANVAS: OnceLock<MapCanvas> = OnceLock::new();
    CANVAS.get_or_init(|| parse(MAP_CANVAS))
}

/// The canvas's map and its `AreaTable` zone ids, in draw order.
pub fn canvas() -> (u32, &'static [u32]) {
    let c = loaded();
    (c.map, &c.zones)
}

/// The zones the canvas rect holds in full while they stay misted (#421), so the painted map
/// already shows them whole when one joins `zones`.
pub fn held() -> &'static [u32] {
    &loaded().held
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_canvas_is_the_four_zones_on_map_0_in_draw_order() {
        assert_eq!(canvas(), (0, &[10, 44, 12, 1519][..]));
    }

    #[test]
    fn the_canvas_holds_burning_steppes() {
        assert_eq!(held(), &[46]);
    }

    #[test]
    fn held_may_be_absent() {
        assert!(parse(r#"{"map": 0, "zones": [12]}"#).held.is_empty());
    }
}
