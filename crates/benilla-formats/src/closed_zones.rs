//! Closed Zones (ADR 0015): the map chunks the world leaves out, read from
//! `data/twilights-eve/closed_chunks.json`. The realm reads the same file with its own reader
//! (`twilight-realm`'s `world/closed.rs`); the math below matches it line for line.

use std::collections::HashSet;
use std::sync::OnceLock;

use crate::{CHUNK_SIZE, TILE_SIZE};

/// Chunks across a map, both ways: 64 tiles of 16.
const CHUNKS: i64 = 64 * 16;

const CLOSED_CHUNKS: &str = include_str!("../../../data/twilights-eve/closed_chunks.json");

struct Closed {
    map: u32,
    chunks: HashSet<(u32, u32)>,
}

/// `{"map": 0, "zones": [..], "chunks": [[gx, gy], ..]}`: the map, and the chunks in the
/// file's order.
fn parse(text: &str) -> (u32, Vec<(u32, u32)>) {
    let v: serde_json::Value = serde_json::from_str(text).expect("closed_chunks.json parses");
    let num = |v: &serde_json::Value| v.as_u64().expect("closed_chunks.json holds numbers") as u32;
    let list: Vec<(u32, u32)> = v["chunks"]
        .as_array()
        .expect("closed_chunks.json has chunks")
        .iter()
        .map(|c| (num(&c[0]), num(&c[1])))
        .collect();
    (num(&v["map"]), list)
}

fn closed() -> &'static Closed {
    static CLOSED: OnceLock<Closed> = OnceLock::new();
    CLOSED.get_or_init(|| {
        let (map, list) = parse(CLOSED_CHUNKS);
        Closed {
            map,
            chunks: list.into_iter().collect(),
        }
    })
}

/// Whether `(x, y)` on `map` lies on a closed chunk. A point off the map's grid, or on another
/// map than the file's, is open.
pub fn closed_at(map: u32, x: f32, y: f32) -> bool {
    let c = closed();
    if map != c.map {
        return false;
    }
    let index = |v: f32| ((32.0 * TILE_SIZE - v) / CHUNK_SIZE).floor() as i64;
    let (gx, gy) = (index(y), index(x));
    (0..CHUNKS).contains(&gx)
        && (0..CHUNKS).contains(&gy)
        && c.chunks.contains(&(gx as u32, gy as u32))
}

/// Whether chunk `(gx, gy)` of the map's 1024 × 1024 grid (`tile_x · 16 + ix`,
/// `tile_y · 16 + iy`) on `map` is closed.
pub fn chunk_closed(map: u32, gx: u32, gy: u32) -> bool {
    let c = closed();
    map == c.map && c.chunks.contains(&(gx, gy))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinel_hill_is_closed_and_goldshire_and_northshire_are_open() {
        assert!(closed_at(0, -10508.8, 1045.23), "Gryan Stoutmantle");
        assert!(!closed_at(0, -9465.52, 74.0069), "Marshal Dughan");
        assert!(!closed_at(0, -8933.54, -136.523), "Deputy Willem");
        assert!(!closed_at(1, -10508.8, 1045.23), "Kalimdor is all open");
        assert!(!closed_at(1, -417.0, -2646.0), "the Crossroads");
    }

    #[test]
    fn chunk_closed_agrees_with_closed_at_at_the_first_and_last_chunks_centres() {
        let (map, list) = parse(CLOSED_CHUNKS);
        let ends = [list[0], *list.last().unwrap()];
        for (gx, gy) in ends {
            // Chunk `gx` runs west to east along y, `gy` north to south along x.
            let x = 32.0 * TILE_SIZE - (gy as f32 + 0.5) * CHUNK_SIZE;
            let y = 32.0 * TILE_SIZE - (gx as f32 + 0.5) * CHUNK_SIZE;
            assert!(chunk_closed(map, gx, gy), "({gx}, {gy})");
            assert!(closed_at(map, x, y), "({gx}, {gy}) at ({x}, {y})");
        }
        assert!(!chunk_closed(1, ends[0].0, ends[0].1), "another map");
    }
}
