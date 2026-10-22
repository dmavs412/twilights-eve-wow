//! The Closed Zones' mist curtain (ADR 0015): an animated fog wall on every edge between an open
//! map chunk and a closed one (`benilla_formats::closed_zones`), so the player never sees the
//! cut. Each loaded ADT tile builds the segments of its open chunks into one mesh, a child of the
//! tile's terrain root that despawns with it. `shaders/mist.wgsl` colours it with the scene fog.

use benilla_assets::coords::wow_to_bevy;
use benilla_assets::AdtTile;
use benilla_formats::closed_zones::chunk_closed;
use benilla_formats::{ChunkMesh, CHUNK_SIZE, TILE_SIZE};
use bevy::asset::RenderAssetUsages;
use bevy::camera::primitives::MeshAabb;
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
    MaterialPlugin,
};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

use crate::lighting::WowLighting;
use crate::schedule::WorldStage;
use crate::terrain_stream::TerrainTileRoot;

/// Yards the curtain reaches below the lowest terrain vertex on its side.
pub const BELOW_YD: f32 = 10.0;
/// Yards the curtain rises above the highest terrain vertex on its side.
pub const ABOVE_YD: f32 = 80.0;
/// The fully opaque band from the curtain's foot, in yards; above it the mist fades to nothing.
pub const OPAQUE_YD: f32 = 40.0;
/// The two noise layers' sideways drift, in yards per second.
pub const DRIFT_FAST_YD_S: f32 = 1.5;
pub const DRIFT_SLOW_YD_S: f32 = 0.6;

/// Chunks across a map, both ways: 64 tiles of 16.
const CHUNKS: u32 = 64 * 16;

/// The mist material: the scene fog's colour, alpha from the height band and two noise layers.
pub type MistMaterial = ExtendedMaterial<StandardMaterial, MistExt>;

#[derive(Asset, AsBindGroup, Clone, TypePath)]
pub struct MistExt {
    /// The scene fog colour (raw gamma, as `WowLighting::fog_color`) and, in `w`, the seconds the
    /// noise has drifted for.
    #[uniform(100)]
    pub(crate) fog: Vec4,
    /// [`OPAQUE_YD`], [`DRIFT_FAST_YD_S`] and [`DRIFT_SLOW_YD_S`], read by the shader.
    #[uniform(101)]
    pub(crate) shape: Vec4,
}

impl MaterialExtension for MistExt {
    fn fragment_shader() -> ShaderRef {
        "embedded://benilla_world/shaders/mist.wgsl".into()
    }

    /// Blended over the scene and never written to depth, so the terrain behind still draws.
    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_write_enabled = false;
        }
        Ok(())
    }
}

/// The one mist material every curtain shares.
#[derive(Resource)]
struct Mist {
    material: Handle<MistMaterial>,
}

/// A tile root whose curtain is built (or found to have none).
#[derive(Component)]
struct MistBuilt;

/// One curtain segment, in WoW coordinates: along a chunk side from `from` to `to` (`[x, y]`),
/// from `bottom` up to `top`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Segment {
    pub(crate) from: [f32; 2],
    pub(crate) to: [f32; 2],
    pub(crate) bottom: f32,
    pub(crate) top: f32,
}

/// The chunk-grid line of index `i`, along either axis: grid index `i` starts there.
fn grid_line(i: u32) -> f32 {
    32.0 * TILE_SIZE - i as f32 * CHUNK_SIZE
}

/// The curtain segments of a tile's `chunks`: one per side of an open chunk whose neighbour on the
/// 1024 × 1024 grid is closed by `closed(gx, gy)`. The open chunk owns the side, so an edge on a
/// tile border is built once, by the tile holding the open chunk.
pub(crate) fn segments(chunks: &[ChunkMesh], closed: impl Fn(u32, u32) -> bool) -> Vec<Segment> {
    let mut out = Vec::new();
    for chunk in chunks {
        // `positions[0]` is the north-west corner, on the grid lines of the chunk's own indices.
        let Some(&[x, y, _]) = chunk.positions.first() else {
            continue;
        };
        if chunk.positions.len() < 145 {
            continue;
        }
        let index = |v: f32| ((32.0 * TILE_SIZE - v) / CHUNK_SIZE).round() as i64;
        let (gx, gy) = (index(y), index(x));
        if !(0..CHUNKS as i64).contains(&gx) || !(0..CHUNKS as i64).contains(&gy) {
            continue;
        }
        let (gx, gy) = (gx as u32, gy as u32);
        if closed(gx, gy) {
            continue;
        }
        let (north, south) = (grid_line(gy), grid_line(gy + 1));
        let (west, east) = (grid_line(gx), grid_line(gx + 1));
        // Each side: its neighbour, its two corners, and its outer-grid vertices (rows of 17, the
        // 9 outer ones first; columns run west to east, rows north to south).
        let sides: [(Option<(u32, u32)>, [f32; 2], [f32; 2], [usize; 2]); 4] = [
            (
                gy.checked_sub(1).map(|n| (gx, n)),
                [north, west],
                [north, east],
                [0, 1],
            ),
            (Some((gx, gy + 1)), [south, west], [south, east], [136, 1]),
            (
                gx.checked_sub(1).map(|w| (w, gy)),
                [north, west],
                [south, west],
                [0, 17],
            ),
            (Some((gx + 1, gy)), [north, east], [south, east], [8, 17]),
        ];
        for (neighbour, from, to, [start, step]) in sides {
            let Some((nx, ny)) = neighbour.filter(|&(nx, ny)| nx < CHUNKS && ny < CHUNKS) else {
                continue;
            };
            if !closed(nx, ny) {
                continue;
            }
            let zs = (0..9).map(|k| chunk.positions[start + k * step][2]);
            let low = zs.clone().fold(f32::INFINITY, f32::min);
            let high = zs.fold(f32::NEG_INFINITY, f32::max);
            out.push(Segment {
                from,
                to,
                bottom: low - BELOW_YD,
                top: high + ABOVE_YD,
            });
        }
    }
    out
}

/// One mesh of `segments`' quads in Bevy space. UV is `(yards above the foot, the segment's
/// height)`, which the shader's height band reads.
fn curtain_mesh(segments: &[Segment]) -> Mesh {
    let mut positions = Vec::with_capacity(segments.len() * 4);
    let mut uvs = Vec::with_capacity(segments.len() * 4);
    let mut indices = Vec::with_capacity(segments.len() * 6);
    for s in segments {
        let base = positions.len() as u32;
        let h = s.top - s.bottom;
        for (p, z, up) in [
            (s.from, s.bottom, 0.0),
            (s.to, s.bottom, 0.0),
            (s.to, s.top, h),
            (s.from, s.top, h),
        ] {
            positions.push(wow_to_bevy([p[0], p[1], z]).to_array());
            uvs.push([up, h]);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    let n = positions.len();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    // Unlit: the normal only satisfies the pipeline.
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; n]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn setup_mist(mut commands: Commands, mut materials: ResMut<Assets<MistMaterial>>) {
    let material = materials.add(MistMaterial {
        base: StandardMaterial {
            unlit: true,
            cull_mode: None, // both faces: the curtain is seen from either side
            alpha_mode: AlphaMode::Blend,
            ..default()
        },
        extension: MistExt {
            fog: Vec4::ZERO,
            shape: Vec4::new(OPAQUE_YD, DRIFT_FAST_YD_S, DRIFT_SLOW_YD_S, 0.0),
        },
    });
    commands.insert_resource(Mist { material });
}

/// Builds each newly spawned tile's curtain, once, as a child of its terrain root.
fn build_mist_curtains(
    mut commands: Commands,
    roots: Query<(Entity, &TerrainTileRoot), Without<MistBuilt>>,
    tiles: Res<Assets<AdtTile>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mist: Res<Mist>,
) {
    for (root, tile) in &roots {
        let Some(adt) = tiles.get(&tile.adt) else {
            continue;
        };
        let map = tile.map;
        let found = segments(&adt.chunks, |gx, gy| chunk_closed(map, gx, gy));
        commands.entity(root).insert(MistBuilt);
        if found.is_empty() {
            continue;
        }
        let mesh = curtain_mesh(&found);
        let aabb = mesh.compute_aabb();
        let handle = meshes.add(mesh);
        commands.entity(root).with_children(|c| {
            let mut curtain = c.spawn((
                Mesh3d(handle),
                MeshMaterial3d(mist.material.clone()),
                Transform::IDENTITY, // the mesh is in absolute world coords
                // Outdoors like the terrain it stands on: hidden from inside a sealed WMO room.
                crate::exterior_cull::ExteriorScene,
            ));
            if let Some(aabb) = aabb {
                curtain.insert(aabb);
            }
        });
    }
}

/// Keeps the mist the scene fog's colour, through day, night and weather, and drifts its noise.
fn tint_mist(
    lighting: Option<Res<WowLighting>>,
    time: Res<Time>,
    mist: Res<Mist>,
    mut materials: ResMut<Assets<MistMaterial>>,
) {
    let Some(lighting) = lighting else {
        return;
    };
    let [r, g, b] = lighting.fog_color;
    if let Some(m) = materials.get_mut(&mist.material) {
        m.extension.fog = Vec4::new(r, g, b, time.elapsed_secs_wrapped());
    }
}

pub struct MistPlugin;

impl Plugin for MistPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<MistMaterial>::default())
            .add_systems(Startup, setup_mist)
            .add_systems(
                Update,
                (
                    build_mist_curtains.in_set(WorldStage::Stream),
                    tint_mist.in_set(crate::lighting::LightingConsumeSet),
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chunk `(gx, gy)`'s mesh, each of its 145 vertices at the height of its own index.
    fn chunk(gx: u32, gy: u32) -> ChunkMesh {
        let (x0, y0) = (grid_line(gy), grid_line(gx));
        let positions = (0..145)
            .map(|i| {
                // Outer rows of 9 then inner rows of 8, 17 to a row pair.
                let (row, col, inner) = (i / 17, i % 17, i % 17 >= 9);
                let (r, c) = if inner {
                    (row as f32 + 0.5, (col - 9) as f32 + 0.5)
                } else {
                    (row as f32, col as f32)
                };
                let step = CHUNK_SIZE / 8.0;
                [x0 - r * step, y0 - c * step, i as f32]
            })
            .collect();
        ChunkMesh {
            positions,
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
            holes: 0,
            base_texture: None,
            layer_textures: Vec::new(),
            layer_effect_ids: Vec::new(),
            alpha_map: None,
            shadow: None,
            pred_tex: [0; 64],
            no_effect_doodad: [false; 64],
            index_x: gx % 16,
            index_y: gy % 16,
            area_id: 0,
            impassable: false,
            liquids: Vec::new(),
        }
    }

    /// The centre of the 3 × 3 grid: chunk 5 of tile (32, 48).
    const CX: u32 = 32 * 16 + 5;
    const CY: u32 = 48 * 16 + 5;

    #[test]
    fn closed_east_and_south_neighbours_give_two_segments_with_their_heights() {
        let closed = |gx, gy| (gx, gy) == (CX + 1, CY) || (gx, gy) == (CX, CY + 1);
        let found = segments(&[chunk(CX, CY)], closed);
        let (north, south) = (grid_line(CY), grid_line(CY + 1));
        let (west, east) = (grid_line(CX), grid_line(CX + 1));
        assert_eq!(
            found,
            vec![
                // South: the bottom outer row, vertices 136..=144.
                Segment {
                    from: [south, west],
                    to: [south, east],
                    bottom: 136.0 - BELOW_YD,
                    top: 144.0 + ABOVE_YD,
                },
                // East: the last outer column, vertices 8, 25, ..., 144.
                Segment {
                    from: [north, east],
                    to: [south, east],
                    bottom: 8.0 - BELOW_YD,
                    top: 144.0 + ABOVE_YD,
                },
            ]
        );
    }

    #[test]
    fn an_all_open_grid_gives_none() {
        let tile: Vec<ChunkMesh> = (0..3)
            .flat_map(|dx| (0..3).map(move |dy| chunk(CX - 1 + dx, CY - 1 + dy)))
            .collect();
        assert!(segments(&tile, |_, _| false).is_empty());
    }

    #[test]
    fn a_closed_neighbour_in_the_next_tile_gives_the_border_chunk_one_segment() {
        // Chunk 15 of its tile's row, and chunk 0 of the tile to the east.
        let edge = 32 * 16 + 15;
        let found = segments(&[chunk(edge, CY)], |gx, gy| (gx, gy) == (edge + 1, CY));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].from[1], grid_line(edge + 1));
        assert_eq!(found[0].to[1], grid_line(edge + 1));
    }

    #[test]
    fn a_closed_chunk_builds_nothing_of_its_own() {
        assert!(segments(&[chunk(CX, CY)], |_, _| true).is_empty());
    }

    #[test]
    fn neighbouring_segments_share_their_corners() {
        let closed = |gx, gy| (gx, gy) == (CX + 1, CY) || (gx, gy) == (CX, CY + 1);
        let found = segments(&[chunk(CX, CY)], closed);
        assert_eq!(found[0].to, found[1].to, "the south-east corner");
        let mesh = curtain_mesh(&found);
        assert_eq!(mesh.count_vertices(), 8);
    }
}
