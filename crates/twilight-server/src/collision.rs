//! **Headless line of sight** (#142): what a body collides with in the client (terrain, M2
//! doodad hulls, WMO walking faces), baked per loaded tile into one triangle mesh in WoW world
//! coordinates, without the renderer.

use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI};

use benilla_formats::{
    load_m2_collision_hull, load_wmo_collision_tris, Chain, CollisionMesh, TileMesh,
};
use parry3d::glamx::{Mat3, Quat, Vec3};
use parry3d::math::Pose;
use parry3d::query::{intersection_test, Ray, RayCast};
use parry3d::shape::{Capsule, TriMesh};

use crate::terrain::TileKey;

/// How much of each end a sight line leaves out, in yards: the two bodies it joins, as the
/// client's `SIGHT_END_YD`.
const SIGHT_END_YD: f32 = 1.0;

/// How far above a height hint a body may stand, in yards: the hint is where it was, and the
/// stair it climbs onto is a little higher.
const STEP_UP_YD: f32 = 2.0;

/// The flattest a face may be, as its normal's upward share, to be stood on: stairs and ramps
/// count, walls do not.
const WALKABLE_NORMAL_Z: f32 = 0.5;

/// How far over its feet a standing body's solid starts, in yards: a face lower than this (the
/// floor, a low step) is under the body, not in it.
const BODY_FOOT_CLEAR: f32 = 0.3;

/// The loaded tiles' collision, and the model hulls they share.
#[derive(Default)]
pub struct Collision {
    tiles: HashMap<TileKey, TriMesh>,
    /// Each tile's standable surfaces alone: its terrain and WMO faces, without doodad hulls.
    ground: HashMap<TileKey, TriMesh>,
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
        let (mut ground_verts, mut ground_tris) = (verts.clone(), tris.clone());
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
            for (v, t) in [
                (&mut verts, &mut tris),
                (&mut ground_verts, &mut ground_tris),
            ] {
                push(v, t, &faces.positions, &faces.indices, |p| place.apply(p));
            }
        }
        if !ground_tris.is_empty() {
            match TriMesh::new(ground_verts, ground_tris) {
                Ok(mesh) => {
                    self.ground.insert(key, mesh);
                }
                Err(e) => eprintln!("twilight-server: ground for {key:?}: {e}"),
            }
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
        self.ground.remove(key);
    }

    /// The highest surface to stand on at `(x, y)` on `map` that is at or below `z_hint` plus
    /// [`STEP_UP_YD`]: terrain, or a building's floor or stair. None where nothing is loaded.
    pub(crate) fn ground_height(&self, map: u32, x: f32, y: f32, z_hint: f32) -> Option<f32> {
        let down = Vec3::new(0.0, 0.0, -1.0);
        let mut best: Option<f32> = None;
        for (_, mesh) in self.ground.iter().filter(|((m, _, _), _)| *m == map) {
            // A ray down through a stack of faces meets each; the first standable one is this
            // mesh's highest.
            let mut from = z_hint + STEP_UP_YD;
            while let Some(hit) = mesh.cast_local_ray_and_get_normal(
                &Ray::new(Vec3::new(x, y, from), down),
                f32::MAX,
                false,
            ) {
                let z = from - hit.time_of_impact;
                if hit.normal.z.abs() >= WALKABLE_NORMAL_Z {
                    best = Some(best.map_or(z, |b| b.max(z)));
                    break;
                }
                from = z - 1e-3;
            }
        }
        best
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

    /// Whether a body standing with its feet at `foot` on `map` touches anything loaded: a
    /// capsule of `radius` whose solid spans [`BODY_FOOT_CLEAR`] to `height` over `foot`, so the
    /// floor it stands on and a step lower than that never count (ADR 0021). Where nothing is
    /// loaded, nothing is touched.
    pub(crate) fn body_overlaps(&self, map: u32, foot: [f32; 3], radius: f32, height: f32) -> bool {
        let foot = Vec3::from(foot);
        let a = foot + Vec3::Z * (BODY_FOOT_CLEAR + radius);
        let b = foot + Vec3::Z * (height - radius).max(BODY_FOOT_CLEAR + radius);
        let body = Capsule::new(a, b, radius);
        self.tiles
            .iter()
            .filter(|((m, _, _), _)| *m == map)
            .any(|(_, mesh)| {
                intersection_test(&Pose::IDENTITY, &body, &Pose::IDENTITY, mesh).unwrap_or(false)
            })
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

    /// A floor at z 0 and a wall in the plane x = 1, both on map 0.
    fn floor_and_wall() -> Collision {
        let verts = vec![
            Vec3::new(-5.0, -5.0, 0.0),
            Vec3::new(5.0, -5.0, 0.0),
            Vec3::new(5.0, 5.0, 0.0),
            Vec3::new(-5.0, 5.0, 0.0),
            Vec3::new(1.0, -5.0, 0.0),
            Vec3::new(1.0, 5.0, 0.0),
            Vec3::new(1.0, 5.0, 4.0),
            Vec3::new(1.0, -5.0, 4.0),
        ];
        let tris = vec![[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]];
        let mut c = Collision::default();
        c.tiles
            .insert((0, 0, 0), TriMesh::new(verts, tris).unwrap());
        c
    }

    #[test]
    fn a_body_overlaps_a_wall_within_its_radius_but_not_its_floor() {
        let c = floor_and_wall();
        assert!(
            !c.body_overlaps(0, [0.0, 0.0, 0.0], 0.4, 2.0),
            "the floor alone"
        );
        assert!(
            c.body_overlaps(0, [0.8, 0.0, 0.0], 0.4, 2.0),
            "0.2 yd from the wall"
        );
        assert!(
            !c.body_overlaps(0, [-1.0, 0.0, 0.0], 0.4, 2.0),
            "clear of it"
        );
        assert!(
            !c.body_overlaps(1, [0.8, 0.0, 0.0], 0.4, 2.0),
            "another map"
        );
    }

    #[test]
    fn an_unrotated_placement_keeps_wow_axes() {
        let p = Placement::new([100.0, 200.0, 5.0], [0.0, 0.0, 0.0], 2.0);
        let v = p.apply([0.0, 0.0, 1.0]);
        assert!((v.z - 7.0).abs() < 1e-4, "{v:?}");
    }
}
