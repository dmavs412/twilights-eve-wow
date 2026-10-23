//! **The headless terrain host** (#141): ADT tiles loaded from game data around every player,
//! answering the realm's ground height and area as the client's streamed terrain does: the
//! terrain's own triangles, a hole's column none (a cave mouth goes to the WMO below). Line of
//! sight runs through the same tiles' [`Collision`].

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::Result;
use benilla_formats::{area_id_at, load_tile_mesh, terrain_height_at, MapTiles, TileMesh};
use benilla_formats::{load_map_catalog, Chain, MapCatalog};
use twilight_realm::Host;

use crate::collision::Collision;

/// How many tiles out from a player's own tile stay loaded: the 3x3 around it.
const RADIUS: u32 = 1;

/// How long a tile no player is near stays loaded, so walking along a tile edge does not reload.
const LINGER: Duration = Duration::from_secs(60);

/// A tile on a map: `(map, tile_x, tile_y)`.
pub type TileKey = (u32, u32, u32);

struct LoadedTile {
    mesh: TileMesh,
    wanted: Instant,
}

/// The game data and the tiles loaded from it.
pub struct Terrain {
    chain: Chain,
    maps: MapCatalog,
    /// Each map's tile index, `None` for a map with no WDT (or no tiles: a WMO-only Dungeon).
    wdts: HashMap<u32, Option<MapTiles>>,
    tiles: HashMap<TileKey, LoadedTile>,
    collision: Collision,
}

impl Terrain {
    /// Opens the `Data` folder's archives.
    pub fn open(data: &Path) -> Result<Self> {
        let mut chain = Chain::open(data)?;
        let maps = load_map_catalog(&mut chain)?;
        Ok(Self {
            chain,
            maps,
            wdts: HashMap::new(),
            tiles: HashMap::new(),
            collision: Collision::default(),
        })
    }

    /// Loads the tiles around each of `players` (`(map, position)`) and drops the ones no player
    /// has been near for [`LINGER`], collision with them.
    pub fn refresh(&mut self, players: &[(u32, [f32; 3])]) {
        let now = Instant::now();
        for &(map, [x, y, _]) in players {
            for (tx, ty) in self.tiles_around(map, x, y) {
                let key = (map, tx, ty);
                if let Some(tile) = self.tiles.get_mut(&key) {
                    tile.wanted = now;
                    continue;
                }
                let Some(dir) = self.maps.directory(map) else {
                    continue;
                };
                match load_tile_mesh(&mut self.chain, dir, tx, ty) {
                    Ok(mesh) => {
                        self.collision.load(&mut self.chain, key, &mesh);
                        self.tiles.insert(key, LoadedTile { mesh, wanted: now });
                    }
                    Err(e) => eprintln!("twilight-server: tile {key:?}: {e:#}"),
                }
            }
        }
        let dropped: Vec<TileKey> = self
            .tiles
            .iter()
            .filter(|(_, t)| now.duration_since(t.wanted) > LINGER)
            .map(|(k, _)| *k)
            .collect();
        for key in &dropped {
            self.tiles.remove(key);
            self.collision.drop_tile(key);
        }
    }

    /// The existing tiles within [`RADIUS`] of `(x, y)` on `map`.
    fn tiles_around(&mut self, map: u32, x: f32, y: f32) -> Vec<(u32, u32)> {
        let (chain, maps) = (&mut self.chain, &self.maps);
        let wdt = self.wdts.entry(map).or_insert_with(|| {
            let dir = maps.directory(map)?;
            MapTiles::load(chain, dir).ok()
        });
        wdt.as_ref()
            .map_or_else(Vec::new, |w| w.existing_in_radius(x, y, RADIUS))
    }

    fn tile_under(&self, map: u32, x: f32, y: f32) -> Option<&TileMesh> {
        let (tx, ty) = benilla_wdt::world_to_tile(x, y);
        self.tiles.get(&(map, tx, ty)).map(|t| &t.mesh)
    }
}

impl Host for Terrain {
    fn ground_height(&self, map: u32, x: f32, y: f32, z_hint: f32) -> Option<f32> {
        let terrain = || terrain_height_at(&self.tile_under(map, x, y)?.chunks, [x, y, z_hint]);
        // The highest surface at or just below the hint, a building's stairs included; a hole
        // in the terrain with nothing under it stays none.
        self.collision
            .ground_height(map, x, y, z_hint)
            .or_else(terrain)
    }
    fn line_of_sight(&self, map: u32, from: [f32; 3], to: [f32; 3]) -> bool {
        self.collision.line_of_sight(map, from, to)
    }
    fn area_at(&self, map: u32, x: f32, y: f32) -> Option<u32> {
        area_id_at(&self.tile_under(map, x, y)?.chunks, [x, y, 0.0])
    }
    /// The tile's ADT water alone: no WMO liquid is loaded here (`crate::liquid`).
    fn liquid_at(&self, map: u32, x: f32, y: f32) -> Option<f32> {
        let chunks = &self.tile_under(map, x, y)?.chunks;
        crate::liquid::water_surface_at(chunks.iter().flat_map(|c| &c.liquids), x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Northshire Abbey's courtyard, on game data when `WOW_DATA` names it; skipped otherwise.
    #[test]
    fn northshire_has_ground_an_area_and_abbey_walls() {
        let Some(data) = std::env::var_os("WOW_DATA") else {
            return;
        };
        let mut t = Terrain::open(Path::new(&data)).unwrap();
        let (x, y) = (-8913.0, -137.0);
        t.refresh(&[(0, [x, y, 83.0])]);
        let z = t.ground_height(0, x, y, 120.0).expect("ground");
        assert!((70.0..100.0).contains(&z), "{z}");
        assert!(t.area_at(0, x, y).is_some());
        // Across the Abbey's footprint at head height, a wall is in the way.
        assert!(!t.line_of_sight(0, [-8930.0, -190.0, z + 2.0], [-8890.0, -150.0, z + 2.0]));
        // High above it, nothing is.
        assert!(t.line_of_sight(0, [-8930.0, -190.0, z + 80.0], [-8890.0, -150.0, z + 80.0]));
    }

    /// Marshal McBride's spawn (-8902.59, -162.606, 82.022) is on the Abbey's stairs, a WMO
    /// surface above the terrain under it.
    #[test]
    fn mcbride_stands_on_the_abbey_stairs_not_the_terrain_below() {
        let Some(data) = std::env::var_os("WOW_DATA") else {
            return;
        };
        let mut t = Terrain::open(Path::new(&data)).unwrap();
        let (x, y, spawn_z) = (-8902.59, -162.606, 82.022);
        t.refresh(&[(0, [x, y, spawn_z])]);
        let z = t.ground_height(0, x, y, spawn_z).expect("ground");
        assert!((z - spawn_z).abs() < 0.5, "{z}");
        let terrain = terrain_height_at(&t.tile_under(0, x, y).unwrap().chunks, [x, y, spawn_z])
            .expect("terrain");
        assert!(z > terrain + 0.5, "stairs {z} vs terrain {terrain}");
    }
}
