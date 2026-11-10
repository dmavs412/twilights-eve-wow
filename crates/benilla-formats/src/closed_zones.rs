//! Closed Zones (ADR 0015): the map chunks the world leaves out, read from
//! `data/twilights-eve/closed_chunks.json`. The realm reads the same file with its own reader
//! (`twilight-realm`'s `world/closed.rs`); the math below matches it line for line. Unlike the
//! realm, which holds the Milestones itself, it answers for the [`Reached`] set its caller holds.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::OnceLock;

use crate::{CHUNK_SIZE, TILE_SIZE};

/// Chunks across a map, both ways: 64 tiles of 16.
const CHUNKS: i64 = 64 * 16;

const CLOSED_CHUNKS: &str = include_str!("../../../data/twilights-eve/closed_chunks.json");

/// The Milestones reached (ADR 0034), by key: a gated chunk is open once its key is among them.
/// The realm sends the whole set (`SessionEvent::Milestones`); the client holds the last one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reached(BTreeSet<String>);

impl Reached {
    pub fn new<S: Into<String>>(keys: impl IntoIterator<Item = S>) -> Self {
        Self(keys.into_iter().map(Into::into).collect())
    }

    pub fn contains(&self, key: &str) -> bool {
        self.0.contains(key)
    }

    /// The keys in `self` that `before` lacks, in key order.
    pub fn added_since(&self, before: &Reached) -> Vec<String> {
        self.0.difference(&before.0).cloned().collect()
    }
}

/// A zone closed until its Milestone `key` is reached.
struct GatedZone {
    key: String,
    zone: u32,
}

struct Closed {
    map: u32,
    /// The `AreaTable` zones closed for good.
    zones: Vec<u32>,
    /// The chunks closed for good.
    chunks: HashSet<(u32, u32)>,
    gated_zones: Vec<GatedZone>,
    /// The chunks closed until a Milestone is reached, each with the key that opens it.
    gated: HashMap<(u32, u32), String>,
}

impl Closed {
    /// Whether chunk `(gx, gy)` of the file's map is closed while `reached` are.
    fn chunk(&self, chunk: (u32, u32), reached: &Reached) -> bool {
        self.chunks.contains(&chunk) || self.gated.get(&chunk).is_some_and(|k| !reached.contains(k))
    }
}

/// `{"map": 0, "zones": [..], "chunks": [[gx, gy], ..], "gated": [{"key", "zone", "chunks"}, ..]}`:
/// the map, its `AreaTable` zone ids and chunks closed for good, the latter in the file's order,
/// and the zones a Milestone opens (ADR 0034) with their chunks.
fn parse(
    text: &str,
) -> (
    u32,
    Vec<u32>,
    Vec<(u32, u32)>,
    Vec<(GatedZone, Vec<(u32, u32)>)>,
) {
    let v: serde_json::Value = serde_json::from_str(text).expect("closed_chunks.json parses");
    let num = |v: &serde_json::Value| v.as_u64().expect("closed_chunks.json holds numbers") as u32;
    let chunks = |v: &serde_json::Value| -> Vec<(u32, u32)> {
        v.as_array()
            .expect("closed_chunks.json has chunks")
            .iter()
            .map(|c| (num(&c[0]), num(&c[1])))
            .collect()
    };
    let zones: Vec<u32> = v["zones"]
        .as_array()
        .expect("closed_chunks.json has zones")
        .iter()
        .map(num)
        .collect();
    let gated = v["gated"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|g| {
            let zone = GatedZone {
                key: g["key"]
                    .as_str()
                    .expect("closed_chunks.json names each gated zone's key")
                    .to_string(),
                zone: num(&g["zone"]),
            };
            (zone, chunks(&g["chunks"]))
        })
        .collect();
    (num(&v["map"]), zones, chunks(&v["chunks"]), gated)
}

fn closed() -> &'static Closed {
    static CLOSED: OnceLock<Closed> = OnceLock::new();
    CLOSED.get_or_init(|| {
        let (map, zones, list, gated_list) = parse(CLOSED_CHUNKS);
        let mut gated = HashMap::new();
        let mut gated_zones = Vec::new();
        for (zone, chunks) in gated_list {
            for chunk in chunks {
                gated.insert(chunk, zone.key.clone());
            }
            gated_zones.push(zone);
        }
        Closed {
            map,
            zones,
            chunks: list.into_iter().collect(),
            gated_zones,
            gated,
        }
    })
}

/// Whether the `AreaTable` zone `area_id` on `map` is a Closed Zone while `reached` are: misted
/// on the world map.
pub fn zone_closed(map: u32, area_id: u32, reached: &Reached) -> bool {
    let c = closed();
    map == c.map
        && (c.zones.contains(&area_id)
            || c.gated_zones
                .iter()
                .any(|g| g.zone == area_id && !reached.contains(&g.key)))
}

/// Whether `(x, y)` on `map` lies on a closed chunk while `reached` are. A point off the map's
/// grid, or on another map than the file's, is open.
pub fn closed_at(map: u32, x: f32, y: f32, reached: &Reached) -> bool {
    let c = closed();
    if map != c.map {
        return false;
    }
    let index = |v: f32| ((32.0 * TILE_SIZE - v) / CHUNK_SIZE).floor() as i64;
    let (gx, gy) = (index(y), index(x));
    (0..CHUNKS).contains(&gx)
        && (0..CHUNKS).contains(&gy)
        && c.chunk((gx as u32, gy as u32), reached)
}

/// Whether chunk `(gx, gy)` of the map's 1024 × 1024 grid (`tile_x · 16 + ix`,
/// `tile_y · 16 + iy`) on `map` is closed while `reached` are.
pub fn chunk_closed(map: u32, gx: u32, gy: u32, reached: &Reached) -> bool {
    let c = closed();
    map == c.map && c.chunk((gx, gy), reached)
}

/// The chunks `(gx, gy)` on `map` that `now` opens and `before` kept closed: the gated chunks
/// whose key `now` holds and `before` lacks.
pub fn opened_chunks(map: u32, before: &Reached, now: &Reached) -> Vec<(u32, u32)> {
    let c = closed();
    if map != c.map {
        return Vec::new();
    }
    c.gated
        .iter()
        .filter(|(_, key)| now.contains(key) && !before.contains(key))
        .map(|(&chunk, _)| chunk)
        .collect()
}

/// The north-west corner `(north x, west y)` of chunk `(gx, gy)` in WoW coordinates. The chunk
/// spans [`CHUNK_SIZE`] yards south (down x) and east (down y) of it.
pub fn chunk_corner(gx: u32, gy: u32) -> (f32, f32) {
    (
        32.0 * TILE_SIZE - gy as f32 * CHUNK_SIZE,
        32.0 * TILE_SIZE - gx as f32 * CHUNK_SIZE,
    )
}

/// Every chunk `(gx, gy)` closed while `reached` are whose square overlaps the square of
/// half-side `radius_yd` around `(x, y)` on `map`, in the file's grid convention (`gx` from y,
/// `gy` from x).
pub fn closed_chunks_near(
    map: u32,
    x: f32,
    y: f32,
    radius_yd: f32,
    reached: &Reached,
) -> Vec<(u32, u32)> {
    let c = closed();
    if map != c.map {
        return Vec::new();
    }
    let index = |v: f32| ((32.0 * TILE_SIZE - v) / CHUNK_SIZE).floor() as i64;
    let last = CHUNKS - 1;
    // World coordinates shrink as the index grows, so the square's max edge gives the low index.
    let (gx_lo, gx_hi) = (
        index(y + radius_yd).clamp(0, last),
        index(y - radius_yd).clamp(0, last),
    );
    let (gy_lo, gy_hi) = (
        index(x + radius_yd).clamp(0, last),
        index(x - radius_yd).clamp(0, last),
    );
    let mut out = Vec::new();
    for gx in gx_lo..=gx_hi {
        for gy in gy_lo..=gy_hi {
            if c.chunk((gx as u32, gy as u32), reached) {
                out.push((gx as u32, gy as u32));
            }
        }
    }
    out
}

/// The distance in yards from `(x, y)` on `map` to the nearest square of the chunks
/// [`closed_chunks_near`] finds for `radius_yd` while `reached` are: 0 on a closed chunk, `None`
/// when no closed chunk lies within `radius_yd`.
pub fn closed_distance(map: u32, x: f32, y: f32, radius_yd: f32, reached: &Reached) -> Option<f32> {
    closed_chunks_near(map, x, y, radius_yd, reached)
        .into_iter()
        .map(|(gx, gy)| {
            let (north, west) = chunk_corner(gx, gy);
            let dx = (north - CHUNK_SIZE - x).max(x - north).max(0.0);
            let dy = (west - CHUNK_SIZE - y).max(y - west).max(0.0);
            dx.hypot(dy)
        })
        .filter(|&d| d <= radius_yd)
        .min_by(f32::total_cmp)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nothing reached.
    fn none() -> Reached {
        Reached::default()
    }

    /// The Burning Steppes' key reached.
    fn steppes() -> Reached {
        Reached::new(["steppes_open"])
    }

    const MORGANS_VIGIL: (f32, f32) = (-8378.5, -2748.9);

    #[test]
    fn closed_chunks_near_finds_westfall_and_not_northshire() {
        let near = closed_chunks_near(0, -10508.8, 1045.23, 100.0, &none());
        assert!(!near.is_empty(), "Gryan Stoutmantle");
        for (gx, gy) in near {
            assert!(chunk_closed(0, gx, gy, &none()), "({gx}, {gy})");
        }
        assert!(closed_chunks_near(0, -8933.54, -136.523, 300.0, &none()).is_empty());
        assert!(closed_chunks_near(1, -10508.8, 1045.23, 100.0, &none()).is_empty());
    }

    #[test]
    fn morgans_vigil_opens_with_its_key_and_searing_gorge_and_westfall_stay_closed() {
        let (x, y) = MORGANS_VIGIL;
        assert!(closed_at(0, x, y, &none()), "Morgan's Vigil, no keys");
        assert!(
            !closed_at(0, x, y, &steppes()),
            "Morgan's Vigil, steppes_open"
        );
        assert!(closed_at(0, -6679.9, -1194.4, &steppes()), "Searing Gorge");
        assert!(closed_at(0, -10508.8, 1045.23, &steppes()), "Westfall");
        assert!(zone_closed(0, 46, &none()));
        assert!(!zone_closed(0, 46, &steppes()));
        assert!(zone_closed(0, 40, &steppes()), "Westfall stays misted");
        assert!(!closed_chunks_near(0, x, y, 50.0, &none()).is_empty());
        assert!(closed_chunks_near(0, x, y, 50.0, &steppes()).is_empty());
    }

    #[test]
    fn closed_distance_measures_to_the_westfall_edge() {
        // The edge runs along y 866.67 at x -9700.
        let d = closed_distance(0, -9700.0, 856.67, 60.0, &none()).expect("10 yd off");
        assert!((d - 10.0).abs() < 0.01, "{d}");
        assert_eq!(closed_distance(0, -9700.0, 880.0, 60.0, &none()), Some(0.0));
        assert_eq!(closed_distance(0, -9700.0, 760.0, 60.0, &none()), None);
        assert_eq!(closed_distance(1, -9700.0, 856.67, 60.0, &none()), None);
    }

    #[test]
    fn closed_distance_skips_a_reached_gated_chunk() {
        let (x, y) = MORGANS_VIGIL;
        assert_eq!(closed_distance(0, x, y, 50.0, &none()), Some(0.0));
        assert_eq!(closed_distance(0, x, y, 50.0, &steppes()), None);
    }

    #[test]
    fn the_steppes_key_opens_the_steppes_chunks_once() {
        let opened = opened_chunks(0, &none(), &steppes());
        assert!(!opened.is_empty());
        for &(gx, gy) in &opened {
            assert!(chunk_closed(0, gx, gy, &none()), "({gx}, {gy})");
            assert!(!chunk_closed(0, gx, gy, &steppes()), "({gx}, {gy})");
        }
        assert!(opened_chunks(0, &steppes(), &steppes()).is_empty());
        assert!(opened_chunks(1, &none(), &steppes()).is_empty());
        assert_eq!(steppes().added_since(&none()), vec!["steppes_open"]);
        assert!(steppes().added_since(&steppes()).is_empty());
    }

    #[test]
    fn chunk_corner_puts_a_point_within_a_chunk_south_and_east_of_it() {
        let (x, y) = (-10508.8_f32, 1045.23_f32);
        let index = |v: f32| ((32.0 * TILE_SIZE - v) / CHUNK_SIZE).floor() as u32;
        let (north, west) = chunk_corner(index(y), index(x));
        assert!((0.0..=CHUNK_SIZE).contains(&(north - x)), "south of north");
        assert!((0.0..=CHUNK_SIZE).contains(&(west - y)), "east of west");
    }

    #[test]
    fn sentinel_hill_is_closed_and_goldshire_and_northshire_are_open() {
        assert!(
            closed_at(0, -10508.8, 1045.23, &none()),
            "Gryan Stoutmantle"
        );
        assert!(!closed_at(0, -9465.52, 74.0069, &none()), "Marshal Dughan");
        assert!(!closed_at(0, -8933.54, -136.523, &none()), "Deputy Willem");
        assert!(
            !closed_at(1, -10508.8, 1045.23, &none()),
            "Kalimdor is all open"
        );
        assert!(!closed_at(1, -417.0, -2646.0, &none()), "the Crossroads");
    }

    #[test]
    fn the_gated_steppes_count_as_closed_and_lakeshire_is_open() {
        assert!(closed_at(0, -8378.5, -2748.9, &none()), "Morgan's Vigil");
        assert!(!closed_at(0, -9219.0, -2222.0, &none()), "Lakeshire");
        assert!(zone_closed(0, 46, &none()), "the Burning Steppes");
        assert!(!closed_chunks_near(0, -8378.5, -2748.9, 50.0, &none()).is_empty());
    }

    #[test]
    fn zone_closed_holds_the_files_zone_on_the_files_map_only() {
        assert!(zone_closed(0, 40, &none()));
        assert!(!zone_closed(0, 12, &none()));
        assert!(!zone_closed(1, 40, &none()));
    }

    #[test]
    fn chunk_closed_agrees_with_closed_at_at_the_first_and_last_chunks_centres() {
        let (map, _, list, _) = parse(CLOSED_CHUNKS);
        let ends = [list[0], *list.last().unwrap()];
        for (gx, gy) in ends {
            // Chunk `gx` runs west to east along y, `gy` north to south along x.
            let x = 32.0 * TILE_SIZE - (gy as f32 + 0.5) * CHUNK_SIZE;
            let y = 32.0 * TILE_SIZE - (gx as f32 + 0.5) * CHUNK_SIZE;
            assert!(chunk_closed(map, gx, gy, &none()), "({gx}, {gy})");
            assert!(closed_at(map, x, y, &none()), "({gx}, {gy}) at ({x}, {y})");
        }
        assert!(
            !chunk_closed(1, ends[0].0, ends[0].1, &none()),
            "another map"
        );
    }
}
