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
pub const ABOVE_YD: f32 = 120.0;
/// The fully opaque band above the ground, in yards, where the noise is middling; above it the
/// mist fades to nothing.
pub const OPAQUE_YD: f32 = 25.0;
/// How far the noise moves the fade's start up or down from [`OPAQUE_YD`], in yards.
pub const RAG_YD: f32 = 20.0;
/// The fade's alpha ceiling where the noise is lowest; it reaches 1 where the noise is highest.
pub const FADE_FLOOR: f32 = 0.3;
/// The noise's drift sideways along the curtain and upward, in yards per second.
pub const DRIFT_YD_S: f32 = 0.4;
pub const DRIFT_UP_YD_S: f32 = 0.3;
/// The domain warp: how far a second noise sample pushes the sample point, in yards.
pub const WARP_YD: f32 = 8.0;
/// How far the foot's colour moves from the fog colour towards its own grey (0 keeps the hue).
pub const FOOT_DESAT: f32 = 0.6;
/// The foot's brightness as a share of that grey's.
pub const FOOT_VALUE: f32 = 0.35;
/// Yards above the ground over which the dark foot returns to the pure fog colour.
pub const COLOUR_RAMP_YD: f32 = 60.0;
/// Yards over which each sheet fades in above its own foot.
pub const FOOT_RAMP_YD: f32 = 6.0;
/// The alpha ceiling of the two wisp sheets on the open chunk's rows 1 and 2.
pub const WISP_CAP: f32 = 0.5;
/// The alpha ceiling of the ground skirt.
pub const SKIRT_CAP: f32 = 0.6;
/// Yards the ground skirt floats above the terrain vertices under it.
pub const SKIRT_RAISE_YD: f32 = 1.5;
/// Yards from the camera within which the wisp sheets and the skirt vanish, so the camera never
/// crosses a hard sheet. The backdrop keeps its alpha at every distance.
pub const CAMERA_FADE_YD: f32 = 3.0;

/// Chunks across a map, both ways: 64 tiles of 16.
const CHUNKS: u32 = 64 * 16;

/// The mist material: the scene fog's colour, alpha from a height band that one noise rags.
pub type MistMaterial = ExtendedMaterial<StandardMaterial, MistExt>;

#[derive(Asset, AsBindGroup, Clone, TypePath)]
pub struct MistExt {
    /// The scene fog colour (raw gamma, as `WowLighting::fog_color`) and, in `w`, the seconds the
    /// noise has drifted for.
    #[uniform(100)]
    pub(crate) fog: Vec4,
    /// [`OPAQUE_YD`], [`DRIFT_YD_S`], [`DRIFT_UP_YD_S`] and [`WARP_YD`], read by the shader.
    #[uniform(101)]
    pub(crate) shape: Vec4,
    /// [`FOOT_DESAT`], [`FOOT_VALUE`], [`COLOUR_RAMP_YD`] and [`FOOT_RAMP_YD`], read by the shader.
    #[uniform(102)]
    pub(crate) foot: Vec4,
    /// [`RAG_YD`], [`FADE_FLOOR`] and [`CAMERA_FADE_YD`], read by the shader.
    #[uniform(103)]
    pub(crate) rag: Vec4,
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
/// from `bottom` up to `top`, over the open chunk's terrain `rows`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Segment {
    pub(crate) from: [f32; 2],
    pub(crate) to: [f32; 2],
    pub(crate) bottom: f32,
    pub(crate) top: f32,
    /// The open chunk's outer vertex rows 0, 1 and 2 counted inward from the side, each ordered
    /// from `from` to `to`. Row 0 lies on the side.
    pub(crate) rows: [[[f32; 3]; 9]; 3],
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
        // Each side: its neighbour, its two corners, its outer-grid vertices (rows of 17, the 9
        // outer ones first; columns run west to east, rows north to south) and the index step to
        // the next outer row or column inward.
        let sides: [(Option<(u32, u32)>, [f32; 2], [f32; 2], [isize; 3]); 4] = [
            (
                gy.checked_sub(1).map(|n| (gx, n)),
                [north, west],
                [north, east],
                [0, 1, 17],
            ),
            (
                Some((gx, gy + 1)),
                [south, west],
                [south, east],
                [136, 1, -17],
            ),
            (
                gx.checked_sub(1).map(|w| (w, gy)),
                [north, west],
                [south, west],
                [0, 17, 1],
            ),
            (
                Some((gx + 1, gy)),
                [north, east],
                [south, east],
                [8, 17, -1],
            ),
        ];
        for (neighbour, from, to, [start, step, inward]) in sides {
            let Some((nx, ny)) = neighbour.filter(|&(nx, ny)| nx < CHUNKS && ny < CHUNKS) else {
                continue;
            };
            if !closed(nx, ny) {
                continue;
            }
            let rows: [[[f32; 3]; 9]; 3] = std::array::from_fn(|j| {
                std::array::from_fn(|k| {
                    chunk.positions[(start + k as isize * step + j as isize * inward) as usize]
                })
            });
            let zs = rows[0].iter().map(|p| p[2]);
            let low = zs.clone().fold(f32::INFINITY, f32::min);
            let high = zs.fold(f32::NEG_INFINITY, f32::max);
            out.push(Segment {
                from,
                to,
                bottom: low - BELOW_YD,
                top: high + ABOVE_YD,
                rows,
            });
        }
    }
    out
}

/// The curtain mesh's vertices and indices as they are built, positions already in Bevy space.
#[derive(Default)]
struct CurtainBuilder {
    positions: Vec<[f32; 3]>,
    uv0: Vec<[f32; 2]>,
    uv1: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl CurtainBuilder {
    fn vertex(&mut self, p: [f32; 3], uv0: [f32; 2], uv1: [f32; 2]) {
        self.positions.push(wow_to_bevy(p).to_array());
        self.uv0.push(uv0);
        self.uv1.push(uv1);
    }

    /// The quads of the `lines` × 9 grid of vertices ending the buffer, 9 to a line.
    fn grid(&mut self, lines: u32) {
        let base = self.positions.len() as u32 - lines * 9;
        for l in 0..lines - 1 {
            for k in 0..8 {
                let a = base + l * 9 + k;
                let (b, c, d) = (a + 1, a + 10, a + 9);
                self.indices.extend_from_slice(&[a, b, c, a, c, d]);
            }
        }
    }

    /// A sheet standing on one terrain `row`, each column from `foot(ground)` up to `top`, under
    /// the alpha ceiling `cap`.
    fn sheet(&mut self, row: &[[f32; 3]; 9], foot: impl Fn(f32) -> f32, top: f32, cap: f32) {
        for &[x, y, ground] in row {
            let f = foot(ground);
            self.vertex([x, y, f], [f - ground, top - ground], [cap, 0.0]);
        }
        for &[x, y, ground] in row {
            let f = foot(ground);
            self.vertex([x, y, top], [top - ground, top - ground], [cap, top - f]);
        }
        self.grid(2);
    }
}

/// One mesh of `segments` in Bevy space, four pieces each: the backdrop on the side, two wisp
/// sheets on the open chunk's next two rows, and a skirt over all three rows on the ground.
/// `UV_0` is `(yards above the ground, the sheet top's yards above the ground)` in the vertex's
/// column, which the shader's height band reads; the skirt's `(0, 1)` keeps it solid. `UV_1` is
/// `(the piece's alpha ceiling, yards above the piece's own foot)`, which fades it in.
fn curtain_mesh(segments: &[Segment]) -> Mesh {
    let mut b = CurtainBuilder::default();
    for s in segments {
        // The backdrop: from below every ground point on the side, so it seals the cut.
        b.sheet(&s.rows[0], |_| s.bottom, s.top, 1.0);
        for row in &s.rows[1..] {
            b.sheet(row, |ground| ground - 1.0, s.top, WISP_CAP);
        }
        // The skirt: strongest at the side, gone by row 2.
        for (row, ramp) in s.rows.iter().zip([FOOT_RAMP_YD, FOOT_RAMP_YD / 2.0, 0.0]) {
            for &[x, y, ground] in row {
                b.vertex(
                    [x, y, ground + SKIRT_RAISE_YD],
                    [0.0, 1.0],
                    [SKIRT_CAP, ramp],
                );
            }
        }
        b.grid(3);
    }
    let n = b.positions.len();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, b.positions);
    // Unlit: the normal only satisfies the pipeline.
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; n]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, b.uv0);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, b.uv1);
    mesh.insert_indices(Indices::U32(b.indices));
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
            shape: Vec4::new(OPAQUE_YD, DRIFT_YD_S, DRIFT_UP_YD_S, WARP_YD),
            foot: Vec4::new(FOOT_DESAT, FOOT_VALUE, COLOUR_RAMP_YD, FOOT_RAMP_YD),
            rag: Vec4::new(RAG_YD, FADE_FLOOR, CAMERA_FADE_YD, 0.0),
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
        let sides: Vec<_> = found
            .iter()
            .map(|s| (s.from, s.to, s.bottom, s.top))
            .collect();
        assert_eq!(
            sides,
            vec![
                // South: the bottom outer row, vertices 136..=144.
                (
                    [south, west],
                    [south, east],
                    136.0 - BELOW_YD,
                    144.0 + ABOVE_YD
                ),
                // East: the last outer column, vertices 8, 25, ..., 144.
                (
                    [north, east],
                    [south, east],
                    8.0 - BELOW_YD,
                    144.0 + ABOVE_YD
                ),
            ]
        );
        let heights = |s: &Segment| s.rows.map(|row| row.map(|p| p[2]));
        let run = |first: usize, step: usize| std::array::from_fn(|k| (first + k * step) as f32);
        assert_eq!(heights(&found[0]), [run(136, 1), run(119, 1), run(102, 1)]);
        assert_eq!(heights(&found[1]), [run(8, 17), run(7, 17), run(6, 17)]);
        // Row 0 is the side itself, from `from` to `to`.
        for s in &found {
            for (p, corner) in [(s.rows[0][0], s.from), (s.rows[0][8], s.to)] {
                let off = Vec2::new(p[0], p[1]).distance(Vec2::from(corner));
                assert!(off < 0.01, "row 0 ends {off} yd from its corner");
            }
        }
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
        assert_eq!(mesh.count_vertices(), 162);
    }

    #[test]
    fn a_segment_mesh_has_a_backdrop_two_wisp_sheets_and_a_skirt_on_the_ground() {
        use bevy::mesh::VertexAttributeValues;
        let found = segments(&[chunk(CX, CY)], |gx, gy| (gx, gy) == (CX, CY + 1));
        let [s] = found.as_slice() else {
            panic!("one segment, got {found:?}");
        };
        let mesh = curtain_mesh(&found);
        assert_eq!(mesh.count_vertices(), 81);
        assert_eq!(mesh.indices().map(|i| i.len()), Some(240));
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("no positions");
        };
        let Some(VertexAttributeValues::Float32x2(uv1)) = mesh.attribute(Mesh::ATTRIBUTE_UV_1)
        else {
            panic!("no UV_1");
        };
        // The backdrop's 18 vertices, then each wisp sheet's 18, then the skirt's 27.
        assert!(uv1[..18].iter().all(|uv| uv[0] == 1.0));
        assert!(uv1[18..54].iter().all(|uv| uv[0] == WISP_CAP));
        for (j, row) in s.rows.iter().enumerate() {
            let ramp = [FOOT_RAMP_YD, FOOT_RAMP_YD / 2.0, 0.0][j];
            for (k, &[x, y, ground]) in row.iter().enumerate() {
                let i = 54 + j * 9 + k;
                let raised = wow_to_bevy([x, y, ground + SKIRT_RAISE_YD]);
                assert!(
                    Vec3::from(positions[i]).abs_diff_eq(raised, 1e-3),
                    "skirt {j}, {k}"
                );
                assert_eq!(uv1[i], [SKIRT_CAP, ramp], "skirt {j}, {k}");
            }
        }
    }
}
