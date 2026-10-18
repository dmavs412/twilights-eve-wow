//! **Headless line of sight** (#142): what a body collides with in the client (terrain, M2
//! doodad hulls, WMO walking faces), baked per loaded tile into one triangle mesh in WoW world
//! coordinates, without the renderer.

use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI};

use benilla_formats::{
    load_m2_collision_hull, load_wmo_collision_tris, Chain, CollisionMesh, TileMesh,
};
use parry3d::glamx::{Mat3, Quat, Vec3};
use parry3d::query::{Ray, RayCast};
use parry3d::shape::TriMesh;

use crate::terrain::TileKey;

/// How much of each end a sight line leaves out, in yards: the two bodies it joins, as the
/// client's `SIGHT_END_YD`.
const SIGHT_END_YD: f32 = 1.0;

/// The loaded tiles' collision, and the model hulls they share.
#[derive(Default)]
pub struct Collision {
    tiles: HashMap<TileKey, TriMesh>,
    /// Model-local hulls by model path; an empty one for a model that does not collide.
    models: HashMap<String, CollisionMesh>,
}

impl Collision {
    /// Bakes `tile`'s collision mesh.
    pub(crate) fn load(&mut self, chain: &mut Chain, key: TileKey, tile: &TileMesh) {
        let mut verts: Vec<Vec3> = Vec::new();
        let mut tris: Vec<[u32; 3]> = Vec::new();
        for chunk in &tile.chunks {
            push(
                &mut verts,
                &mut tris,
                &chunk.positions,
                &chunk.indices,
                |p| Vec3::from(p),
            );
        }
        for d in &tile.doodads {
            let hull = self.model(&d.model, |c, p| load_m2_collision_hull(c, p), chain);
            let place = Placement::new(d.position, d.rotation, d.scale);
            push(&mut verts, &mut tris, &hull.positions, &hull.indices, |p| {
                place.apply(p)
            });
        }
        for w in &tile.wmos {
            let faces = self.model(&w.model, |c, p| load_wmo_collision_tris(c, p), chain);
            let place = Placement::new(w.position, w.rotation, 1.0);
            push(
                &mut verts,
                &mut tris,
                &faces.positions,
                &faces.indices,
                |p| place.apply(p),
            );
        }
        if tris.is_empty() {
            return;
        }
        match TriMesh::new(verts, tris) {
            Ok(mesh) => {
                self.tiles.insert(key, mesh);
            }
            Err(e) => eprintln!("twilight-server: collision for {key:?}: {e}"),
        }
    }

    pub(crate) fn drop_tile(&mut self, key: &TileKey) {
        self.tiles.remove(key);
    }

    fn model(
        &mut self,
        path: &str,
        load: impl FnOnce(&mut Chain, &str) -> anyhow::Result<CollisionMesh>,
        chain: &mut Chain,
    ) -> CollisionMesh {
        self.models
            .entry(path.to_ascii_lowercase())
            .or_insert_with(|| load(chain, path).unwrap_or_default())
            .clone()
    }

    /// Whether nothing loaded on `map` lies between `from` and `to`, ends left out. Where nothing
    /// is loaded, nothing is in the way, as in the client.
    pub(crate) fn line_of_sight(&self, map: u32, from: [f32; 3], to: [f32; 3]) -> bool {
        let (a, b) = (Vec3::from(from), Vec3::from(to));
        let span = b - a;
        let length = span.length() - 2.0 * SIGHT_END_YD;
        if length <= 0.0 {
            return true;
        }
        let dir = span / span.length();
        let ray = Ray::new(a + dir * SIGHT_END_YD, dir);
        !self
            .tiles
            .iter()
            .filter(|((m, _, _), _)| *m == map)
            .any(|(_, mesh)| mesh.intersects_local_ray(&ray, length))
    }
}

fn push(
    verts: &mut Vec<Vec3>,
    tris: &mut Vec<[u32; 3]>,
    positions: &[[f32; 3]],
    indices: &[u32],
    to_world: impl Fn([f32; 3]) -> Vec3,
) {
    let base = verts.len() as u32;
    verts.extend(positions.iter().map(|&p| to_world(p)));
    tris.extend(
        indices
            .chunks_exact(3)
            .map(|t| [base + t[0], base + t[1], base + t[2]]),
    );
}

/// A doodad's or building's placement, as the client applies it: its model-local points taken
/// into Bevy's axes, rotated and scaled there by `benilla_assets::coords::placement_rotation`,
/// and back into WoW world coordinates.
struct Placement {
    translation: Vec3,
    rotation: Quat,
    scale: f32,
}

impl Placement {
    fn new(position: [f32; 3], rotation_deg: [f32; 3], scale: f32) -> Self {
        let [rx, ry, rz] = rotation_deg.map(f32::to_radians);
        let in_wow = Quat::from_rotation_x(FRAC_PI_2)
            * Quat::from_rotation_y(ry - PI)
            * Quat::from_rotation_z(-rx)
            * Quat::from_rotation_x(rz - FRAC_PI_2);
        let to_bevy = Quat::from_mat3(&Mat3::from_cols(
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ));
        Self {
            translation: wow_to_bevy(position),
            rotation: to_bevy * in_wow * to_bevy.inverse(),
            scale,
        }
    }

    fn apply(&self, local: [f32; 3]) -> Vec3 {
        let bevy = self.translation + self.rotation * (wow_to_bevy(local) * self.scale);
        Vec3::new(-bevy.z, -bevy.x, bevy.y)
    }
}

fn wow_to_bevy(p: [f32; 3]) -> Vec3 {
    Vec3::new(-p[1], p[2], -p[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A wall across the x axis at x = 10 on map 0.
    fn walled() -> Collision {
        let verts = vec![
            Vec3::new(10.0, -50.0, -50.0),
            Vec3::new(10.0, 50.0, -50.0),
            Vec3::new(10.0, 0.0, 50.0),
        ];
        let mut c = Collision::default();
        c.tiles
            .insert((0, 0, 0), TriMesh::new(verts, vec![[0, 1, 2]]).unwrap());
        c
    }

    #[test]
    fn a_wall_blocks_the_line_and_open_ground_does_not() {
        let c = walled();
        assert!(!c.line_of_sight(0, [0.0, 0.0, 0.0], [20.0, 0.0, 0.0]));
        assert!(c.line_of_sight(0, [0.0, 0.0, 0.0], [8.0, 0.0, 0.0]));
        assert!(c.line_of_sight(1, [0.0, 0.0, 0.0], [20.0, 0.0, 0.0]));
    }

    #[test]
    fn an_unrotated_placement_keeps_wow_axes() {
        let p = Placement::new([100.0, 200.0, 5.0], [0.0, 0.0, 0.0], 2.0);
        let v = p.apply([0.0, 0.0, 1.0]);
        assert!((v.z - 7.0).abs() < 1e-4, "{v:?}");
    }
}
