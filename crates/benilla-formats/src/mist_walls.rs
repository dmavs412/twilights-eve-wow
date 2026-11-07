//! Placed mist walls (ADR 0033): a straight wall of mist standing where the world has no closed
//! chunk to build one from, read from `data/twilights-eve/mist_walls.json`. The realm reads the
//! same file with its own reader (`twilight-realm`'s `world/closed.rs`) and refuses a step across
//! a wall; the client draws the wall and makes it solid.

use std::sync::OnceLock;

const MIST_WALLS: &str = include_str!("../../../data/twilights-eve/mist_walls.json");

/// One wall, in WoW coordinates: along the ground from `from` to `to` (`[x, y]`), from Z `bottom`
/// up to Z `top`, standing on a floor at Z `floor`. `open_side` is a point on the side players
/// stand on.
#[derive(Debug, Clone, PartialEq)]
pub struct MistWall {
    pub key: String,
    pub map: u32,
    pub from: [f32; 2],
    pub to: [f32; 2],
    pub bottom: f32,
    pub top: f32,
    pub floor: f32,
    pub open_side: [f32; 2],
}

/// `{"walls": [{"key", "map", "from": [x, y], "to": [x, y], "bottom", "top", "floor",
/// "open_side": [x, y]}, ..]}`.
fn parse(text: &str) -> Vec<MistWall> {
    let v: serde_json::Value = serde_json::from_str(text).expect("mist_walls.json parses");
    let num = |v: &serde_json::Value| v.as_f64().expect("mist_walls.json holds numbers") as f32;
    let pair = |v: &serde_json::Value| [num(&v[0]), num(&v[1])];
    v["walls"]
        .as_array()
        .expect("mist_walls.json has walls")
        .iter()
        .map(|w| MistWall {
            key: w["key"]
                .as_str()
                .expect("a mist wall has a key")
                .to_string(),
            map: w["map"].as_u64().expect("a mist wall has a map") as u32,
            from: pair(&w["from"]),
            to: pair(&w["to"]),
            bottom: num(&w["bottom"]),
            top: num(&w["top"]),
            floor: num(&w["floor"]),
            open_side: pair(&w["open_side"]),
        })
        .collect()
}

/// Every placed mist wall, on every map, in the file's order.
pub fn mist_walls() -> &'static [MistWall] {
    static WALLS: OnceLock<Vec<MistWall>> = OnceLock::new();
    WALLS.get_or_init(|| parse(MIST_WALLS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mist_walls_holds_the_deeprun_tunnel_wall() {
        let wall = mist_walls()
            .iter()
            .find(|w| w.key == "deeprun_tunnel")
            .expect("deeprun_tunnel");
        assert_eq!(wall.map, 369);
        assert_eq!(wall.from, [-75.0, 2400.0]);
        assert_eq!(wall.to, [35.0, 2400.0]);
        assert_eq!(wall.floor, -4.3);
    }
}
