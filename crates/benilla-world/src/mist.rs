//! The Closed Zones' mist curtain (ADR 0015): an animated fog wall on every edge between an open
//! map chunk and a closed one (`benilla_formats::closed_zones`), so the player never sees the
//! cut. Each segment is five shells, one behind the other into the closed chunk, the front ones
//! thin and the back one solid, each with its own noise and drift. Each loaded ADT tile builds the
//! segments of its open chunks into one mesh, a child of the tile's terrain root that despawns
//! with it. `shaders/mist.wgsl` colours it with the scene fog.
//! A placed mist wall (`benilla_formats::mist_walls`, ADR 0033) stands where there is no closed
//! chunk, as in the Deeprun Tram's tunnel: the same shells on a straight line, and a solid box.
//! Its material is its own: the WMO fog's colour and distance, and a lower band.
//! When a Milestone opens a gated zone during play ([`ZonesOpened`], ADR 0034), the curtain on
//! its old border fades out over [`OPEN_FADE_SECS`] on a copy of its own; placed walls never fade.

use avian3d::prelude::{Collider, RigidBody};
use benilla_assets::coords::wow_to_bevy;
use benilla_assets::{AdtTile, ReachedMilestones};
use benilla_formats::closed_zones::{chunk_closed, Reached};
use benilla_formats::mist_walls::{mist_walls, MistWall};
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

use crate::collision::walk_layers;
use crate::lighting::WowLighting;
use crate::schedule::WorldStage;
use crate::terrain_stream::TerrainTileRoot;
use crate::world_map::CurrentMap;

/// Yards the back shell reaches below the lowest ground under it, so it seals the cut; a segment's
/// `bottom` is this far below the lowest terrain vertex on its side.
pub const BELOW_YD: f32 = 10.0;
/// Yards the back shell rises above the highest ground under it; a segment's `top` is this far
/// above the highest terrain vertex on its side.
pub const ABOVE_YD: f32 = 120.0;
/// Yards each shell stands past the edge line, front (0) to back (4): the closed chunk's outer
/// vertex rows 0, 2, 4, 6 and 8.
pub const SHELL_DEPTHS_YD: [f32; 5] = [
    0.0,
    CHUNK_SIZE / 4.0,
    CHUNK_SIZE / 2.0,
    CHUNK_SIZE * 3.0 / 4.0,
    CHUNK_SIZE,
];
/// Each shell's alpha ceiling: the front ones thin, the back one solid.
pub const SHELL_CAPS: [f32; 5] = [0.25, 0.35, 0.5, 0.7, 1.0];
/// Yards each shell's top stands above the highest ground under it.
pub const SHELL_TOP_YD: [f32; 5] = [35.0, 55.0, 75.0, 95.0, ABOVE_YD];
/// Yards shells 0 to 3 reach under the ground at each column.
pub const SHELL_SINK_YD: f32 = 4.0;
/// Yards back along each side a shell cuts a corner that turns: half a chunk.
pub const CHAMFER_YD: f32 = CHUNK_SIZE / 2.0;
/// [`CHAMFER_YD`] for the front shell where a closed chunk sticks out into open land; its own
/// constant so the shots can set it to 0 and put the front shell back on the realm's wall.
pub const FRONT_CHAMFER_YD: f32 = CHUNK_SIZE / 2.0;
/// Columns in a shell: the middle of the corner cut at each end and nine along the side between.
const COLUMNS: usize = 11;
/// Yards a placed wall's shells stand behind its line, away from its open side.
pub const WALL_SHELL_DEPTHS_YD: [f32; 5] = [0.0, 1.5, 3.0, 4.5, 6.0];
/// The fully opaque band above the ground, in yards, where the noise is middling; above it the
/// mist fades to nothing.
pub const OPAQUE_YD: f32 = 25.0;
/// How far the noise moves the fade's start up or down from [`OPAQUE_YD`], in yards.
pub const RAG_YD: f32 = 20.0;
/// The fade's alpha ceiling where the noise is lowest; it reaches 1 where the noise is highest.
pub const FADE_FLOOR: f32 = 0.3;
/// Yards after which every octave of the shader's noise repeats, along and up: 256 cells of the
/// first octave's 23 yd. The lattice in `mist.wgsl` wraps on this period and must match it.
pub const NOISE_PERIOD_YD: f32 = 5888.0;
/// The noise's drift sideways along the curtain, in yards per second, before each layer's
/// [`LAYER_DRIFT`]. [`DRIFT_UP_YD_S`] must stay half of it, so [`DRIFT_WRAP_SECS`] moves every
/// layer a whole number of noise periods both ways.
pub const DRIFT_YD_S: f32 = 1.2;
/// The noise's drift upward, in yards per second: half of [`DRIFT_YD_S`], which
/// [`DRIFT_WRAP_SECS`] relies on.
pub const DRIFT_UP_YD_S: f32 = 0.6;
/// Each layer's sideways drift as a multiple of [`DRIFT_YD_S`]; its upward drift takes the
/// absolute value. Every entry is a multiple of 0.5, and a layer past the end takes the last.
pub const LAYER_DRIFT: [f32; 5] = [1.0, -0.5, 1.5, 0.5, -1.0];
/// Seconds after which the drift clock wraps: every layer has then moved a whole number of noise
/// periods, so the wrap never jumps.
pub const DRIFT_WRAP_SECS: f64 = 4.0 * NOISE_PERIOD_YD as f64 / DRIFT_YD_S as f64;
/// How far the billow noise lightens or darkens the colour, as a share of it.
pub const BILLOW_LIGHT: f32 = 0.15;
/// The solid band's alpha where the noise is lowest; it reaches 1 where the noise is highest.
pub const BAND_FLOOR: f32 = 0.85;
/// The domain warp: how far a second noise sample pushes the sample point, in yards.
pub const WARP_YD: f32 = 8.0;
/// How far the foot's colour moves from the fog colour towards its own grey (0 keeps the hue).
pub const FOOT_DESAT: f32 = 0.6;
/// The foot's brightness as a share of that grey's.
pub const FOOT_VALUE: f32 = 0.35;
/// Yards above the ground over which the dark foot returns to the pure fog colour.
pub const COLOUR_RAMP_YD: f32 = 60.0;
/// [`OPAQUE_YD`] for a placed wall, whose floor stands far below the outdoor ground's band.
pub const WALL_OPAQUE_YD: f32 = 8.0;
/// [`RAG_YD`] for a placed wall.
pub const WALL_RAG_YD: f32 = 3.0;
/// [`COLOUR_RAMP_YD`] for a placed wall.
pub const WALL_COLOUR_RAMP_YD: f32 = 15.0;
/// Yards over which each sheet fades in above its own foot.
pub const FOOT_RAMP_YD: f32 = 6.0;
/// Yards from the camera within which shells 0 to 3 vanish, so the camera never crosses a hard
/// sheet. The back shell keeps its alpha at every distance.
pub const CAMERA_FADE_YD: f32 = 3.0;
/// Seconds the curtain on a newly opened zone's border takes to fade to nothing.
pub const OPEN_FADE_SECS: f32 = 6.0;

/// The fade factor of a curtain `secs` after its zone opened: 1 at the start, falling evenly to
/// 0 at [`OPEN_FADE_SECS`] and staying there.
pub fn open_fade(secs: f32) -> f32 {
    (1.0 - secs / OPEN_FADE_SECS).clamp(0.0, 1.0)
}

/// The Milestones reached went from `before` to `now`, adding keys (ADR 0034): the land they
/// open loads in and the mist on its border fades. The app writes it from the realm's
/// `SessionEvent::Milestones`, after it sets [`ReachedMilestones`] to `now`.
#[derive(Message, Clone, Debug)]
pub struct ZonesOpened {
    pub before: Reached,
    pub now: Reached,
}

impl ZonesOpened {
    /// The first event's `before` and the last one's `now`, for a frame with several.
    pub fn merged<'a>(events: impl IntoIterator<Item = &'a ZonesOpened>) -> Option<ZonesOpened> {
        events.into_iter().fold(None, |acc, e| {
            Some(ZonesOpened {
                before: acc.map_or_else(|| e.before.clone(), |a: ZonesOpened| a.before),
                now: e.now.clone(),
            })
        })
    }
}

/// Chunks across a map, both ways: 64 tiles of 16.
const CHUNKS: u32 = 64 * 16;

/// The mist material: the scene fog's colour, alpha from a height band that one noise rags.
pub type MistMaterial = ExtendedMaterial<StandardMaterial, MistExt>;

#[derive(Asset, AsBindGroup, Clone, TypePath)]
pub struct MistExt {
    /// The scene fog colour (raw gamma, as `WowLighting::fog_color`, or `wmo_fog_color` on the
    /// placed walls) and, in `w`, the seconds the noise has drifted for.
    #[uniform(100)]
    pub(crate) fog: Vec4,
    /// [`OPAQUE_YD`], [`DRIFT_YD_S`], [`DRIFT_UP_YD_S`] and [`WARP_YD`], read by the shader.
    #[uniform(101)]
    pub(crate) shape: Vec4,
    /// [`FOOT_DESAT`], [`FOOT_VALUE`], [`COLOUR_RAMP_YD`] and [`FOOT_RAMP_YD`], read by the shader.
    #[uniform(102)]
    pub(crate) foot: Vec4,
    /// [`RAG_YD`], [`FADE_FLOOR`], [`CAMERA_FADE_YD`] and, in `w`, the fade factor the shader
    /// multiplies into the alpha: 1 on every curtain but one fading out ([`open_fade`]).
    #[uniform(103)]
    pub(crate) rag: Vec4,
    /// The scene's linear fog start and end in yards (`WowLighting::fog_start`, `fog_end`, or
    /// `wmo_fog_start`, `wmo_fog_end` on the placed walls), [`BILLOW_LIGHT`] and [`BAND_FLOOR`].
    #[uniform(104)]
    pub(crate) scene: Vec4,
    /// [`LAYER_DRIFT`]: entries 0 to 3 in the first vector, entry 4 in the second's `x`.
    #[uniform(105)]
    pub(crate) drift: [Vec4; 2],
}

/// [`MistExt::scene`] for the scene fog from `start` to `end` yards.
fn scene_lanes(start: f32, end: f32) -> Vec4 {
    Vec4::new(start, end, BILLOW_LIGHT, BAND_FLOOR)
}

/// [`MistExt::drift`]: [`LAYER_DRIFT`] packed four to a vector.
fn drift_lanes() -> [Vec4; 2] {
    let d = LAYER_DRIFT;
    [
        Vec4::new(d[0], d[1], d[2], d[3]),
        Vec4::new(d[4], 0.0, 0.0, 0.0),
    ]
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

/// The two shared mist materials: `material` for every closed-zone curtain on a tile, `wall` for
/// the placed walls, which take the WMO fog and their own band.
#[derive(Resource)]
struct Mist {
    material: Handle<MistMaterial>,
    wall: Handle<MistMaterial>,
}

/// The closed-zone curtains' extension and the placed walls', in that order: the same lanes but
/// for the placed walls' [`WALL_OPAQUE_YD`], [`WALL_RAG_YD`] and [`WALL_COLOUR_RAMP_YD`]. Both
/// start with no colour, no fog and a fade factor of 1.
fn mist_exts() -> [MistExt; 2] {
    let ext = |opaque: f32, rag: f32, ramp: f32| MistExt {
        fog: Vec4::ZERO,
        shape: Vec4::new(opaque, DRIFT_YD_S, DRIFT_UP_YD_S, WARP_YD),
        foot: Vec4::new(FOOT_DESAT, FOOT_VALUE, ramp, FOOT_RAMP_YD),
        rag: Vec4::new(rag, FADE_FLOOR, CAMERA_FADE_YD, 1.0),
        scene: scene_lanes(0.0, 0.0),
        drift: drift_lanes(),
    };
    [
        ext(OPAQUE_YD, RAG_YD, COLOUR_RAMP_YD),
        ext(WALL_OPAQUE_YD, WALL_RAG_YD, WALL_COLOUR_RAMP_YD),
    ]
}

/// The `fog` and `scene` lanes of the closed-zone material and of the placed-wall one, in that
/// order, after `t` seconds of drift: the scene fog triple for the curtains and the interior one
/// (`wmo_fog_*`, the scene fog crossfaded toward the claimed WMO's MFOG) for the walls.
fn fog_lanes(lighting: &WowLighting, t: f32) -> [(Vec4, Vec4); 2] {
    let lanes = |[r, g, b]: [f32; 3], start: f32, end: f32| {
        (Vec4::new(r, g, b, t), scene_lanes(start, end))
    };
    [
        lanes(lighting.fog_color, lighting.fog_start, lighting.fog_end),
        lanes(
            lighting.wmo_fog_color,
            lighting.wmo_fog_start,
            lighting.wmo_fog_end,
        ),
    ]
}

/// A tile root whose curtain is built (or found to have none).
#[derive(Component)]
struct MistBuilt;

/// A tile's curtain, a child of its terrain root.
#[derive(Component)]
struct MistCurtain;

/// A curtain on a newly opened zone's border, detached from every tile, with its own material
/// instance: it fades out and despawns [`OPEN_FADE_SECS`] after the zone opened.
#[derive(Component)]
pub(crate) struct FadingMist {
    pub(crate) secs: f32,
}

/// The segments a closed-zone curtain (tile or fading) stands on, which its foot's puffs
/// (`mist_puffs`) spawn along. Placed walls keep none.
#[derive(Component)]
pub(crate) struct MistSegments(pub(crate) Vec<Segment>);

/// How the edge goes on past one end of a segment, looking outward from that end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Corner {
    /// The edge runs on in line.
    Straight,
    /// The closed chunk ends here: the edge turns into the closed side, and the shells stop
    /// inside the closed chunk.
    ClosedOut,
    /// The edge turns toward the open side, and the shells run on to the mitre point.
    OpenOut,
}

impl Corner {
    /// Which way a shell's joint moves along the segment, in units of its depth: back at a
    /// [`Corner::ClosedOut`], on at a [`Corner::OpenOut`]. Also the way, along `inward`, the
    /// neighbouring shell leaves the joint.
    fn pull(self) -> f32 {
        match self {
            Corner::ClosedOut => 1.0,
            Corner::Straight => 0.0,
            Corner::OpenOut => -1.0,
        }
    }
}

/// One curtain segment, in WoW coordinates: along a chunk side from `from` to `to` (`[x, y]`),
/// from `bottom` up to `top`, over the open chunk's terrain `rows`. It carries everything its
/// five shells need, so a curtain built from any subset of segments gets them whole.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Segment {
    pub(crate) from: [f32; 2],
    pub(crate) to: [f32; 2],
    pub(crate) bottom: f32,
    pub(crate) top: f32,
    /// The open chunk's outer vertex rows 0, 1 and 2 counted inward from the side, each ordered
    /// from `from` to `to`. Row 0 lies on the side.
    pub(crate) rows: [[[f32; 3]; 9]; 3],
    /// The unit vector from the edge line into the closed side.
    pub(crate) inward: [f32; 2],
    /// The corner at `from` and at `to`.
    pub(crate) ends: [Corner; 2],
    /// The far corner of the segment that meets this one at `from` and at `to`, so the two work
    /// out the same corner cut from their two shell lengths.
    pub(crate) beside: [Corner; 2],
    /// The closed neighbour's 9 outer vertex rows, row 0 on the shared side and row 8 on its far
    /// side, each ordered from `from` to `to`; `None` when that chunk is in another tile.
    pub(crate) closed: Option<[[[f32; 3]; 9]; 9]>,
    /// Yards each shell stands past the edge line.
    pub(crate) depths: [f32; 5],
    /// A placed wall's segment: every shell runs from `bottom` to `top`.
    pub(crate) wall: bool,
}

/// The chunk-grid line of index `i`, along either axis: grid index `i` starts there.
fn grid_line(i: u32) -> f32 {
    32.0 * TILE_SIZE - i as f32 * CHUNK_SIZE
}

/// The curtain segments of a tile's `chunks`: one per side of an open chunk whose neighbour on the
/// 1024 × 1024 grid is closed by `closed(gx, gy)`. The open chunk owns the side, so an edge on a
/// tile border is built once, by the tile holding the open chunk.
pub(crate) fn segments(chunks: &[ChunkMesh], closed: impl Fn(u32, u32) -> bool) -> Vec<Segment> {
    sides(chunks, &closed, &closed)
}

/// The segments of the curtain that opening a zone takes down: each side of a chunk open under
/// `before` whose neighbour `before` closes and `now` opens.
pub(crate) fn opened_segments(
    chunks: &[ChunkMesh],
    before: impl Fn(u32, u32) -> bool,
    now: impl Fn(u32, u32) -> bool,
) -> Vec<Segment> {
    sides(chunks, &before, &|gx, gy| before(gx, gy) && !now(gx, gy))
}

/// A chunk's indices on the 1024 × 1024 grid, from its north-west corner (`positions[0]`, on the
/// grid lines of its own indices); `None` for a chunk short of its 145 vertices or off the grid.
fn grid_index(chunk: &ChunkMesh) -> Option<(u32, u32)> {
    let &[x, y, _] = chunk.positions.first()?;
    if chunk.positions.len() < 145 {
        return None;
    }
    let index = |v: f32| ((32.0 * TILE_SIZE - v) / CHUNK_SIZE).round() as i64;
    let (gx, gy) = (index(y), index(x));
    let on = |i: i64| (0..CHUNKS as i64).contains(&i);
    (on(gx) && on(gy)).then_some((gx as u32, gy as u32))
}

/// `N` rows of 9 outer-grid vertices of a chunk (rows of 17, the 9 outer ones first; columns run
/// west to east, rows north to south) from index `start`, `step` apart, the rows `inward` apart.
fn outer_rows<const N: usize>(
    chunk: &ChunkMesh,
    [start, step, inward]: [isize; 3],
) -> [[[f32; 3]; 9]; N] {
    std::array::from_fn(|j| {
        std::array::from_fn(|k| {
            chunk.positions[(start + k as isize * step + j as isize * inward) as usize]
        })
    })
}

/// One segment per side of a chunk `closed` leaves open whose neighbour `across` picks.
fn sides(
    chunks: &[ChunkMesh],
    closed: &dyn Fn(u32, u32) -> bool,
    across: &dyn Fn(u32, u32) -> bool,
) -> Vec<Segment> {
    // Off the grid counts as open.
    let shut = |gx: i64, gy: i64| {
        let on = |i: i64| (0..CHUNKS as i64).contains(&i);
        on(gx) && on(gy) && closed(gx as u32, gy as u32)
    };
    let mut out = Vec::new();
    for chunk in chunks {
        let Some((gx, gy)) = grid_index(chunk) else {
            continue;
        };
        if closed(gx, gy) {
            continue;
        }
        let (north, south) = (grid_line(gy), grid_line(gy + 1));
        let (west, east) = (grid_line(gx), grid_line(gx + 1));
        // Each side: its neighbour, its two corners, the open chunk's outer rows from the side
        // inward and the closed neighbour's from the side outward (start, step along, step to the
        // next row), and the unit vector into the neighbour. WoW `x` runs north, `y` west.
        let sides: [(
            Option<(u32, u32)>,
            [f32; 2],
            [f32; 2],
            [isize; 3],
            [isize; 3],
            [f32; 2],
        ); 4] = [
            (
                gy.checked_sub(1).map(|n| (gx, n)),
                [north, west],
                [north, east],
                [0, 1, 17],
                [136, 1, -17],
                [1.0, 0.0],
            ),
            (
                Some((gx, gy + 1)),
                [south, west],
                [south, east],
                [136, 1, -17],
                [0, 1, 17],
                [-1.0, 0.0],
            ),
            (
                gx.checked_sub(1).map(|w| (w, gy)),
                [north, west],
                [south, west],
                [0, 17, 1],
                [8, 17, -1],
                [0.0, 1.0],
            ),
            (
                Some((gx + 1, gy)),
                [north, east],
                [south, east],
                [8, 17, -1],
                [0, 17, 1],
                [0.0, -1.0],
            ),
        ];
        for (neighbour, from, to, open_rows, closed_rows, inward) in sides {
            let Some((nx, ny)) = neighbour.filter(|&(nx, ny)| nx < CHUNKS && ny < CHUNKS) else {
                continue;
            };
            if !across(nx, ny) {
                continue;
            }
            let rows = outer_rows::<3>(chunk, open_rows);
            let zs = rows[0].iter().map(|p| p[2]);
            let low = zs.clone().fold(f32::INFINITY, f32::min);
            let high = zs.fold(f32::NEG_INFINITY, f32::max);
            // The grid step from `from` toward `to`: east along a north or south side, south
            // along a west or east one.
            let (dx, dy): (i64, i64) = if ny != gy { (1, 0) } else { (0, 1) };
            let add = |(x, y): (i64, i64), (sx, sy): (i64, i64)| (x + sx, y + sy);
            // The corner past the end of the side between `open` and `closed`, one `step` on: the
            // chunks beyond it on the open side and on the closed side.
            let turn = |open: (i64, i64), closed: (i64, i64), step: (i64, i64)| {
                let (o, c) = (add(open, step), add(closed, step));
                match (shut(o.0, o.1), shut(c.0, c.1)) {
                    (false, true) => Corner::Straight,
                    (true, true) => Corner::OpenOut,
                    _ => Corner::ClosedOut,
                }
            };
            let (open, shut_in) = ((gx as i64, gy as i64), (nx as i64, ny as i64));
            let inward_step = (shut_in.0 - open.0, shut_in.1 - open.1);
            // The corner at one end (`sign` -1 at `from`, +1 at `to`).
            let corner = |sign: i64| turn(open, shut_in, (sign * dx, sign * dy));
            // The far corner of the segment that meets this one at that end.
            let beside = |sign: i64| {
                let on = (sign * dx, sign * dy);
                let back = (-inward_step.0, -inward_step.1);
                match corner(sign) {
                    // Along the closed chunk's end, leaving the corner inward.
                    Corner::ClosedOut => turn(add(shut_in, on), shut_in, inward_step),
                    // The open chunk's own side across the corner, leaving it outward.
                    Corner::OpenOut => turn(open, add(open, on), back),
                    // The next side in line.
                    Corner::Straight => turn(add(open, on), add(shut_in, on), on),
                }
            };
            let closed = chunks
                .iter()
                .find(|c| grid_index(c) == Some((nx, ny)))
                .map(|c| outer_rows::<9>(c, closed_rows));
            out.push(Segment {
                from,
                to,
                bottom: low - BELOW_YD,
                top: high + ABOVE_YD,
                rows,
                inward,
                ends: [corner(-1), corner(1)],
                beside: [beside(-1), beside(1)],
                closed,
                depths: SHELL_DEPTHS_YD,
                wall: false,
            });
        }
    }
    out
}

/// Yards a placed wall's solid box is thick.
const WALL_THICK_YD: f32 = 0.5;

/// A placed wall's curtain segment: from `from` to `to`, from `bottom` to `top`, with row 0 nine
/// points evenly along the wall at Z `floor` and rows 1 and 2 the same points moved 1 and 2 yards
/// along the wall's normal toward `open_side`. Its shells stand [`WALL_SHELL_DEPTHS_YD`] behind the
/// line, away from `open_side`, with straight ends.
pub(crate) fn wall_segment(wall: &MistWall) -> Segment {
    let (from, to) = (Vec2::from(wall.from), Vec2::from(wall.to));
    let along = (to - from).normalize_or_zero();
    let mut normal = along.perp();
    if normal.dot(Vec2::from(wall.open_side) - from) < 0.0 {
        normal = -normal;
    }
    let rows = std::array::from_fn(|j| {
        std::array::from_fn(|k| {
            let p = from.lerp(to, k as f32 / 8.0) + normal * j as f32;
            [p.x, p.y, wall.floor]
        })
    });
    Segment {
        from: wall.from,
        to: wall.to,
        bottom: wall.bottom,
        top: wall.top,
        rows,
        inward: (-normal).to_array(),
        ends: [Corner::Straight; 2],
        beside: [Corner::Straight; 2],
        closed: None,
        depths: WALL_SHELL_DEPTHS_YD,
        wall: true,
    }
}

/// A placed wall's solid box in Bevy space: its centre, its turn about the vertical, and its
/// length, thickness and height.
fn wall_box(wall: &MistWall) -> (Vec3, Quat, Vec3) {
    let (from, to) = (Vec2::from(wall.from), Vec2::from(wall.to));
    let mid = (from + to) / 2.0;
    let centre = wow_to_bevy([mid.x, mid.y, (wall.bottom + wall.top) / 2.0]);
    let d = to - from;
    let line = wow_to_bevy([d.x, d.y, 0.0]).normalize_or_zero();
    let turn = Quat::from_rotation_arc(Vec3::X, line);
    let size = Vec3::new(d.length(), wall.top - wall.bottom, WALL_THICK_YD);
    (centre, turn, size)
}

/// A placed wall's curtain or its solid box; both despawn when the map changes.
#[derive(Component)]
struct PlacedMistWall;

/// Spawns the placed walls of the current map, each a curtain and a box on the walk layers, and
/// despawns the last map's when it changes.
fn build_placed_walls(
    mut commands: Commands,
    map: Option<Res<CurrentMap>>,
    mut built_for: Local<Option<u32>>,
    placed: Query<Entity, With<PlacedMistWall>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mist: Res<Mist>,
) {
    let Some(map) = map.map(|m| m.0) else {
        return;
    };
    if *built_for == Some(map) {
        return;
    }
    *built_for = Some(map);
    for e in &placed {
        commands.entity(e).despawn();
    }
    for wall in mist_walls().iter().filter(|w| w.map == map) {
        let mesh = curtain_mesh(&[wall_segment(wall)]);
        let aabb = mesh.compute_aabb();
        // No `ExteriorScene`: the wall stands inside a WMO, whose sealed rooms would hide it.
        let mut curtain = commands.spawn((
            PlacedMistWall,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(mist.wall.clone()),
            Transform::IDENTITY, // the mesh is in absolute world coords
        ));
        if let Some(aabb) = aabb {
            curtain.insert(aabb);
        }
        let (centre, turn, size) = wall_box(wall);
        commands.spawn((
            PlacedMistWall,
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
            walk_layers(),
            Transform::from_translation(centre).with_rotation(turn),
        ));
    }
}

/// The curtain mesh's vertices and indices as they are built, positions already in Bevy space.
#[derive(Default)]
struct CurtainBuilder {
    positions: Vec<[f32; 3]>,
    uv0: Vec<[f32; 2]>,
    uv1: Vec<[f32; 2]>,
    /// `x` is the vertex's layer number; `y`, `z` and `w` are 0.
    colour: Vec<[f32; 4]>,
    /// One index list per layer, joined from the last layer to the first at the end.
    layers: Vec<Vec<u32>>,
}

impl CurtainBuilder {
    fn vertex(&mut self, p: [f32; 3], uv0: [f32; 2], uv1: [f32; 2], layer: u32) {
        self.positions.push(wow_to_bevy(p).to_array());
        self.uv0.push(uv0);
        self.uv1.push(uv1);
        self.colour.push([layer as f32, 0.0, 0.0, 0.0]);
    }

    /// The quads of the `lines` × [`COLUMNS`] grid of vertices ending the buffer, [`COLUMNS`] to
    /// a line, into `layer`'s index list.
    fn grid(&mut self, lines: u32, layer: u32) {
        let n = COLUMNS as u32;
        let base = self.positions.len() as u32 - lines * n;
        let layer = layer as usize;
        if self.layers.len() <= layer {
            self.layers.resize(layer + 1, Vec::new());
        }
        for l in 0..lines - 1 {
            for k in 0..n - 1 {
                let a = base + l * n + k;
                let (b, c, d) = (a + 1, a + n + 1, a + n);
                self.layers[layer].extend_from_slice(&[a, b, c, a, c, d]);
            }
        }
    }

    /// A sheet of `layer` standing on one terrain `row`, each column from `foot(ground)` up to
    /// `top`, under the alpha ceiling `cap`.
    fn sheet(
        &mut self,
        row: &[[f32; 3]; COLUMNS],
        foot: impl Fn(f32) -> f32,
        top: f32,
        cap: f32,
        layer: u32,
    ) {
        for &[x, y, ground] in row {
            let f = foot(ground);
            self.vertex([x, y, f], [f - ground, top - ground], [cap, 0.0], layer);
        }
        for &[x, y, ground] in row {
            let f = foot(ground);
            self.vertex(
                [x, y, top],
                [top - ground, top - ground],
                [cap, top - f],
                layer,
            );
        }
        self.grid(2, layer);
    }
}

/// Shell `k`'s two ends on `s` (`[x, y]`): the segment moved its depth `d` inward, each end moved
/// `d` along it, inward at a [`Corner::ClosedOut`] and outward at a [`Corner::OpenOut`]. `None`
/// when that leaves it shorter than a yard.
pub(crate) fn shell_line(s: &Segment, k: usize) -> Option<(Vec2, Vec2)> {
    let (from, to) = (Vec2::from(s.from), Vec2::from(s.to));
    let u = (to - from).normalize_or_zero();
    let n = Vec2::from(s.inward);
    let d = s.depths[k];
    let start = from + n * d + u * (s.ends[0].pull() * d);
    let end = to + n * d - u * (s.ends[1].pull() * d);
    ((end - start).dot(u) >= 1.0).then_some((start, end))
}

/// The yards `L` shell `k` of `s`, `length` long, cuts back along each side at end `e` (0 at
/// `from`, 1 at `to`): [`FRONT_CHAMFER_YD`] for the front shell and [`CHAMFER_YD`] for the others
/// at a [`Corner::ClosedOut`]; at a [`Corner::OpenOut`] at most twice the depth, which keeps the
/// cut off the open chunk; none at a [`Corner::Straight`]. At most half this shell's length and
/// half the meeting shell's, so the cuts at its two ends never cross and both shells cut alike.
fn chamfer(s: &Segment, k: usize, e: usize, length: f32) -> f32 {
    let d = s.depths[k];
    let rule = match s.ends[e] {
        Corner::Straight => return 0.0,
        Corner::ClosedOut if k == 0 => FRONT_CHAMFER_YD,
        Corner::ClosedOut => CHAMFER_YD,
        Corner::OpenOut => CHAMFER_YD.min(2.0 * d),
    };
    let side = Vec2::from(s.from).distance(Vec2::from(s.to));
    let beside = side - d * (s.ends[e].pull() + s.beside[e].pull());
    rule.min(length / 2.0).min(beside / 2.0).max(0.0)
}

/// Shell `k`'s columns on `s` (`[x, y]`). Each end's joint `J` from [`shell_line`] is cut by
/// [`chamfer`]'s `L`: the shell starts at `M = J + (u + w) * L / 2`, the middle of the diagonal,
/// with `u` along the shell away from `J` and `w` the way the meeting shell leaves `J`, then runs
/// nine columns evenly from `J + u * L` to the far end's matching point and ends at its `M`. Both
/// segments at a corner work `M` out alike, so their shells meet there. `None` with no shell.
pub(crate) fn shell_columns(s: &Segment, k: usize) -> Option<[Vec2; COLUMNS]> {
    let (start, end) = shell_line(s, k)?;
    let u = (Vec2::from(s.to) - Vec2::from(s.from)).normalize_or_zero();
    let n = Vec2::from(s.inward);
    let length = (end - start).dot(u);
    let [a, b] = [0, 1].map(|e| chamfer(s, k, e, length));
    let middle = [
        start + (u + n * s.ends[0].pull()) * a / 2.0,
        end + (-u + n * s.ends[1].pull()) * b / 2.0,
    ];
    let (first, last) = (start + u * a, end - u * b);
    Some(std::array::from_fn(|i| match i {
        0 => middle[0],
        i if i == COLUMNS - 1 => middle[1],
        i => first.lerp(last, (i - 1) as f32 / (COLUMNS - 3) as f32),
    }))
}

/// The ground at `p` behind `s`: bilinear in its closed neighbour's grid, `p` clamped into that
/// chunk, or with no neighbour the height of `rows[0]`'s nearest column along the side.
pub(crate) fn shell_ground(s: &Segment, p: Vec2) -> f32 {
    let (from, to) = (Vec2::from(s.from), Vec2::from(s.to));
    let step = from.distance(to) / 8.0;
    if step <= 0.0 {
        return s.rows[0][0][2];
    }
    // Grid steps along the side and inward from it.
    let u = (to - from) / (8.0 * step);
    let along = (p - from).dot(u) / step;
    let Some(grid) = &s.closed else {
        return s.rows[0][along.round().clamp(0.0, 8.0) as usize][2];
    };
    let (a, b) = (
        along.clamp(0.0, 8.0),
        ((p - from).dot(Vec2::from(s.inward)) / step).clamp(0.0, 8.0),
    );
    let (i, j) = (a.floor().min(7.0) as usize, b.floor().min(7.0) as usize);
    let (fa, fb) = (a - i as f32, b - j as f32);
    let z = |j: usize, i: usize| grid[j][i][2];
    let near = z(j, i) + (z(j, i + 1) - z(j, i)) * fa;
    let far = z(j + 1, i) + (z(j + 1, i + 1) - z(j + 1, i)) * fa;
    near + (far - near) * fb
}

/// One mesh of `segments` in Bevy space, five shells each (layers 0 at the front to 4 at the
/// back), [`shell_line`] apart, on their [`shell_columns`]. The back shell's feet stand [`BELOW_YD`] under
/// the lowest ground under it, so it seals the cut; the others' sink [`SHELL_SINK_YD`] under their
/// own ground. Each top is [`SHELL_TOP_YD`] over the highest ground under the shell; a placed
/// wall's shells all run from its `bottom` to its `top`. `UV_0` is `(yards above the ground, the
/// shell top's yards above the ground)` in the vertex's column, which the shader's height band
/// reads. `UV_1` is `(the shell's alpha ceiling, yards above its own foot)`, which fades it in.
/// `COLOR`'s `x` is the layer, which picks the shell's own noise and drift. The triangles draw
/// shell by shell across every segment, the back shell first, so back to front from the open side.
fn curtain_mesh(segments: &[Segment]) -> Mesh {
    let mut b = CurtainBuilder::default();
    for s in segments {
        for k in 0..SHELL_CAPS.len() {
            let Some(columns) = shell_columns(s, k) else {
                continue;
            };
            let row = columns.map(|p| [p.x, p.y, shell_ground(s, p)]);
            let zs = row.iter().map(|p| p[2]);
            let low = zs.clone().fold(f32::INFINITY, f32::min);
            let top = zs.fold(f32::NEG_INFINITY, f32::max) + SHELL_TOP_YD[k];
            let (cap, layer) = (SHELL_CAPS[k], k as u32);
            if s.wall {
                b.sheet(&row, |_| s.bottom, s.top, cap, layer);
            } else if k == SHELL_CAPS.len() - 1 {
                b.sheet(&row, |_| low - BELOW_YD, top, cap, layer);
            } else {
                b.sheet(&row, |ground| ground - SHELL_SINK_YD, top, cap, layer);
            }
        }
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
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, b.colour);
    mesh.insert_indices(Indices::U32(b.layers.into_iter().rev().flatten().collect()));
    mesh
}

fn setup_mist(mut commands: Commands, mut materials: ResMut<Assets<MistMaterial>>) {
    let [material, wall] = mist_exts().map(|extension| {
        materials.add(MistMaterial {
            base: StandardMaterial {
                unlit: true,
                cull_mode: None, // both faces: the curtain is seen from either side
                alpha_mode: AlphaMode::Blend,
                ..default()
            },
            extension,
        })
    });
    commands.insert_resource(Mist { material, wall });
}

/// Builds each newly spawned tile's curtain, once, as a child of its terrain root, for the
/// Milestones reached.
fn build_mist_curtains(
    mut commands: Commands,
    roots: Query<(Entity, &TerrainTileRoot), Without<MistBuilt>>,
    tiles: Res<Assets<AdtTile>>,
    reached: Option<Res<ReachedMilestones>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mist: Res<Mist>,
) {
    if roots.is_empty() {
        return;
    }
    let reached = reached.map(|r| r.get()).unwrap_or_default();
    for (root, tile) in &roots {
        let Some(adt) = tiles.get(&tile.adt) else {
            continue;
        };
        let map = tile.map;
        let found = segments(&adt.chunks, |gx, gy| chunk_closed(map, gx, gy, &reached));
        commands.entity(root).insert(MistBuilt);
        if found.is_empty() {
            continue;
        }
        let mesh = curtain_mesh(&found);
        let aabb = mesh.compute_aabb();
        let handle = meshes.add(mesh);
        commands.entity(root).with_children(|c| {
            let mut curtain = c.spawn((
                MistCurtain,
                MistSegments(found),
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

/// On a zone opening, takes down every built curtain segment between an open chunk and a newly
/// opened one: a copy of those segments, on no tile and with a material of its own, starts to
/// fade, and the tile's curtain is rebuilt for the new set, so a segment on the new border with a
/// zone still closed comes in at once.
#[allow(clippy::too_many_arguments)]
fn fade_opened_curtains(
    mut commands: Commands,
    mut opened: MessageReader<ZonesOpened>,
    roots: Query<(Entity, &TerrainTileRoot, Option<&Children>), With<MistBuilt>>,
    curtains: Query<(), With<MistCurtain>>,
    tiles: Res<Assets<AdtTile>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MistMaterial>>,
    mist: Res<Mist>,
) {
    let Some(ZonesOpened { before, now }) = ZonesOpened::merged(opened.read()) else {
        return;
    };
    for (root, tile, children) in &roots {
        let Some(adt) = tiles.get(&tile.adt) else {
            continue;
        };
        let map = tile.map;
        let fading = opened_segments(
            &adt.chunks,
            |gx, gy| chunk_closed(map, gx, gy, &before),
            |gx, gy| chunk_closed(map, gx, gy, &now),
        );
        if fading.is_empty() {
            continue;
        }
        for &child in children.into_iter().flatten() {
            if curtains.contains(child) {
                commands.entity(child).despawn();
            }
        }
        commands.entity(root).remove::<MistBuilt>();
        let Some(mut material) = materials.get(&mist.material).cloned() else {
            continue;
        };
        material.extension.rag.w = open_fade(0.0);
        let mesh = curtain_mesh(&fading);
        let aabb = mesh.compute_aabb();
        let mut curtain = commands.spawn((
            FadingMist { secs: 0.0 },
            MistSegments(fading),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
            Transform::IDENTITY,
            crate::exterior_cull::ExteriorScene,
        ));
        if let Some(aabb) = aabb {
            curtain.insert(aabb);
        }
    }
}

/// Fades each [`FadingMist`] by [`open_fade`], in the shared curtain's colour and drift, and
/// despawns it once it is gone.
fn tick_fading_mist(
    mut commands: Commands,
    time: Res<Time>,
    mist: Res<Mist>,
    mut materials: ResMut<Assets<MistMaterial>>,
    mut fading: Query<(Entity, &mut FadingMist, &MeshMaterial3d<MistMaterial>)>,
) {
    let shared = materials
        .get(&mist.material)
        .map(|m| (m.extension.fog, m.extension.scene));
    for (e, mut f, material) in &mut fading {
        f.secs += time.delta_secs();
        if f.secs >= OPEN_FADE_SECS {
            commands.entity(e).despawn();
            continue;
        }
        if let Some(m) = materials.get_mut(&material.0) {
            if let Some((fog, scene)) = shared {
                m.extension.fog = fog;
                m.extension.scene = scene;
            }
            m.extension.rag.w = open_fade(f.secs);
        }
    }
}

/// Keeps the curtains the scene fog's colour and distance and the placed walls the WMO fog's
/// ([`fog_lanes`]), through day, night and weather, and drifts their noise on a clock that wraps
/// every [`DRIFT_WRAP_SECS`].
fn tint_mist(
    lighting: Option<Res<WowLighting>>,
    time: Res<Time>,
    mist: Res<Mist>,
    mut materials: ResMut<Assets<MistMaterial>>,
) {
    let Some(lighting) = lighting else {
        return;
    };
    let t = (time.elapsed_secs_f64() % DRIFT_WRAP_SECS) as f32;
    let handles = [&mist.material, &mist.wall];
    for (handle, (fog, scene)) in handles.into_iter().zip(fog_lanes(&lighting, t)) {
        if let Some(m) = materials.get_mut(handle) {
            m.extension.fog = fog;
            m.extension.scene = scene;
        }
    }
}

pub struct MistPlugin;

impl Plugin for MistPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<MistMaterial>::default())
            .add_message::<ZonesOpened>()
            .add_systems(Startup, (setup_mist, crate::mist_puffs::setup_mist_puffs))
            .add_systems(
                Update,
                (
                    (fade_opened_curtains, build_mist_curtains)
                        .chain()
                        .in_set(WorldStage::Stream),
                    // After both, so a fading copy takes over its segments' puffs the frame it
                    // spawns.
                    crate::mist_puffs::simulate_mist_puffs
                        .after(build_mist_curtains)
                        .after(tick_fading_mist),
                    build_placed_walls.in_set(WorldStage::Stream),
                    (tint_mist, tick_fading_mist)
                        .chain()
                        .in_set(crate::lighting::LightingConsumeSet),
                ),
            )
            // The stream push: PostUpdate after the frame's clear, as the weather's.
            .add_systems(
                PostUpdate,
                crate::mist_puffs::push_mist_puffs
                    .after(crate::particles::buffer::begin_effect_frame),
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
    fn a_border_chunk_beside_a_gated_one_has_a_segment_until_its_key_is_reached() {
        // Morgan's Vigil's chunk, in the gated Burning Steppes, and its neighbour to the west.
        let index = |v: f32| ((32.0 * TILE_SIZE - v) / CHUNK_SIZE).floor() as u32;
        let (gx, gy) = (index(-2748.9), index(-8378.5));
        let none = Reached::default();
        let steppes = Reached::new(["steppes_open"]);
        assert!(chunk_closed(0, gx, gy, &none));
        let west = chunk(gx - 1, gy);
        // Only the gated chunk counts, so the test holds wherever the Steppes' edge runs.
        let gated = |reached: &Reached| {
            let reached = reached.clone();
            move |x, y| (x, y) == (gx, gy) && chunk_closed(0, x, y, &reached)
        };
        let shut = segments(std::slice::from_ref(&west), gated(&none));
        assert_eq!(shut.len(), 1, "the east side, against the Steppes");
        assert!(segments(std::slice::from_ref(&west), gated(&steppes)).is_empty());
        let fading = opened_segments(std::slice::from_ref(&west), gated(&none), gated(&steppes));
        assert_eq!(fading, shut, "the segment the key takes down fades");
    }

    #[test]
    fn a_side_against_a_zone_closed_for_good_never_fades() {
        // The east neighbour stays closed; the south one opens.
        let before = |gx, gy| (gx, gy) == (CX + 1, CY) || (gx, gy) == (CX, CY + 1);
        let now = |gx, gy| (gx, gy) == (CX + 1, CY);
        let fading = opened_segments(&[chunk(CX, CY)], before, now);
        assert_eq!(fading.len(), 1);
        assert_eq!(fading[0].from, [grid_line(CY + 1), grid_line(CX)]);
        // A chunk closed before, opened now, owns no side that fades.
        assert!(opened_segments(&[chunk(CX, CY)], |_, _| true, |_, _| false).is_empty());
    }

    #[test]
    fn the_open_fade_runs_from_one_to_nothing_over_six_seconds() {
        assert_eq!(open_fade(0.0), 1.0);
        assert_eq!(open_fade(3.0), 0.5);
        assert_eq!(open_fade(6.0), 0.0);
        assert_eq!(open_fade(9.0), 0.0);
        assert_eq!(OPEN_FADE_SECS, 6.0);
    }

    #[test]
    fn merged_events_keep_the_first_before_and_the_last_now() {
        let (a, b, c) = (
            Reached::default(),
            Reached::new(["x"]),
            Reached::new(["x", "y"]),
        );
        let events = [
            ZonesOpened {
                before: a.clone(),
                now: b.clone(),
            },
            ZonesOpened {
                before: b,
                now: c.clone(),
            },
        ];
        let merged = ZonesOpened::merged(&events).unwrap();
        assert_eq!((merged.before, merged.now), (a, c));
        assert!(ZonesOpened::merged(&[]).is_none());
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
        // Both ends of each turn into the closed side, so only shells 0 and 1 are a yard long.
        let mesh = curtain_mesh(&found);
        assert_eq!(mesh.count_vertices(), 2 * 2 * 22);
    }

    /// The mesh's positions back in WoW coordinates, its `UV_0`, `UV_1` and layer numbers.
    #[allow(clippy::type_complexity)]
    fn mesh_parts(mesh: &Mesh) -> (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<[f32; 2]>, Vec<f32>) {
        use benilla_assets::coords::bevy_to_wow;
        use bevy::mesh::VertexAttributeValues as V;
        let Some(V::Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
            panic!("no positions");
        };
        let Some(V::Float32x2(uv0)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) else {
            panic!("no UV_0");
        };
        let Some(V::Float32x2(uv1)) = mesh.attribute(Mesh::ATTRIBUTE_UV_1) else {
            panic!("no UV_1");
        };
        let Some(V::Float32x4(colour)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR) else {
            panic!("no COLOR");
        };
        (
            positions
                .iter()
                .map(|&p| bevy_to_wow(Vec3::from(p)))
                .collect(),
            uv0.clone(),
            uv1.clone(),
            colour.iter().map(|c| c[0]).collect(),
        )
    }

    /// The open chunk at the centre with its south, south-west and south-east neighbours closed:
    /// one south segment with straight ends.
    fn closed_south_row(gx: u32, gy: u32) -> bool {
        gy == CY + 1 && (CX - 1..=CX + 1).contains(&gx)
    }

    #[test]
    fn shells_stand_at_their_depths_with_their_caps() {
        let found = segments(&[chunk(CX, CY), chunk(CX, CY + 1)], closed_south_row);
        let [s] = found.as_slice() else {
            panic!("one segment, got {found:?}");
        };
        assert_eq!(s.ends, [Corner::Straight; 2]);
        assert_eq!(s.inward, [-1.0, 0.0]);
        let mesh = curtain_mesh(&found);
        assert_eq!(mesh.count_vertices(), 5 * 22);
        let (positions, _, uv1, layers) = mesh_parts(&mesh);
        let south = grid_line(CY + 1);
        let depths = [0.0, 8.333, 16.667, 25.0, 33.333];
        for k in 0..5 {
            for i in k * 22..(k + 1) * 22 {
                let past = south - positions[i][0];
                assert!((past - depths[k]).abs() < 0.01, "shell {k}: {past} yd past");
                assert_eq!(uv1[i][0], SHELL_CAPS[k], "shell {k}'s cap");
                assert_eq!(layers[i], k as f32, "shell {k}'s layer");
            }
        }
        let indices: Vec<usize> = mesh.indices().expect("indices").iter().collect();
        assert_eq!(indices.len(), 5 * 60);
        assert!(
            indices[..60].iter().all(|&v| layers[v] == 4.0),
            "shell 4 first"
        );
        // Back to front: the layers never rise along the index buffer.
        let order: Vec<f32> = indices.iter().map(|&v| layers[v]).collect();
        assert!(order.windows(2).all(|w| w[0] >= w[1]));
    }

    #[test]
    fn shells_of_a_straight_edge_meet_end_to_end() {
        let closed = |gx, gy| gy == CY + 1 && (CX..=CX + 1).contains(&gx);
        let found = segments(&[chunk(CX, CY), chunk(CX + 1, CY)], closed);
        let [a, b] = found.as_slice() else {
            panic!("two segments, got {found:?}");
        };
        assert_eq!((a.ends[1], b.ends[0]), (Corner::Straight, Corner::Straight));
        let mut met = 0;
        for k in 0..5 {
            match (shell_line(a, k), shell_line(b, k)) {
                (Some((_, end)), Some((start, _))) => {
                    assert!(end.distance(start) < 0.01, "shell {k}: {end} to {start}");
                    met += 1;
                }
                (None, None) => {}
                other => panic!("shell {k} on one side only: {other:?}"),
            }
        }
        // The outer ends turn into the closed side, so the back shell is too short on both.
        assert_eq!(met, 4);
    }

    #[test]
    fn closed_out_shells_stop_inside_the_closed_chunk() {
        let found = segments(&[chunk(CX, CY)], |gx, gy| (gx, gy) == (CX + 1, CY));
        let [s] = found.as_slice() else {
            panic!("one segment, got {found:?}");
        };
        assert_eq!(s.ends, [Corner::ClosedOut; 2]);
        let from = Vec2::from(s.from);
        let u = (Vec2::from(s.to) - from).normalize();
        let n = Vec2::from(s.inward);
        for k in 0..2 {
            let d = SHELL_DEPTHS_YD[k];
            let (start, end) = shell_line(s, k).expect("shells 0 and 1");
            for (p, along) in [(start, d), (end, CHUNK_SIZE - d)] {
                assert!(((p - from).dot(u) - along).abs() < 0.01, "shell {k} along");
                assert!(((p - from).dot(n) - d).abs() < 0.01, "shell {k} inward");
            }
        }
        for k in 2..5 {
            assert_eq!(shell_line(s, k), None, "shell {k} is left out");
        }
        assert_eq!(curtain_mesh(&found).count_vertices(), 2 * 22);
    }

    /// The open chunk at the centre with its east, north and north-east neighbours closed.
    fn closed_north_east(gx: u32, gy: u32) -> bool {
        [(CX + 1, CY), (CX, CY - 1), (CX + 1, CY - 1)].contains(&(gx, gy))
    }

    #[test]
    fn open_out_shells_meet_at_the_mitre() {
        let found = segments(&[chunk(CX, CY)], closed_north_east);
        let [north, east] = found.as_slice() else {
            panic!("north and east segments, got {found:?}");
        };
        assert_eq!(north.to, east.from, "the north-east corner");
        assert_eq!(
            (north.ends[1], east.ends[0]),
            (Corner::OpenOut, Corner::OpenOut)
        );
        let corner = Vec2::from(north.to);
        for k in 0..5 {
            let d = SHELL_DEPTHS_YD[k];
            let mitre = corner + Vec2::from(north.inward) * d + Vec2::from(east.inward) * d;
            let (_, north_end) = shell_line(north, k).expect("north shell");
            let (east_start, _) = shell_line(east, k).expect("east shell");
            assert!(
                north_end.distance(mitre) < 0.01,
                "shell {k}: {north_end} vs {mitre}"
            );
            assert!(
                east_start.distance(mitre) < 0.01,
                "shell {k}: {east_start} vs {mitre}"
            );
        }
    }

    /// Only the chunk south-east of the centre closed: its corner sticks out into open land.
    fn closed_south_east(gx: u32, gy: u32) -> bool {
        (gx, gy) == (CX + 1, CY + 1)
    }

    /// The two open chunks that border [`closed_south_east`]'s closed one: east and south.
    fn south_east_borders() -> Vec<ChunkMesh> {
        vec![chunk(CX + 1, CY), chunk(CX, CY + 1)]
    }

    /// A one-chunk staircase: closed where `gx + gy` is past the centre's.
    fn staircase(gx: u32, gy: u32) -> bool {
        gx as i64 - CX as i64 + gy as i64 - CY as i64 > 0
    }

    /// The 5 × 5 chunks around the centre.
    fn five_by_five() -> Vec<ChunkMesh> {
        (0..5)
            .flat_map(|dx| (0..5).map(move |dy| chunk(CX - 2 + dx, CY - 2 + dy)))
            .collect()
    }

    #[test]
    fn a_closed_out_corner_is_cut_on_the_closed_chunk() {
        let found = segments(&south_east_borders(), closed_south_east);
        let [south, east] = found.as_slice() else {
            panic!("the east chunk's south side and the south chunk's east side, got {found:?}");
        };
        assert_eq!(
            south.from, east.from,
            "the closed chunk's north-west corner"
        );
        assert_eq!(
            (south.ends[0], east.ends[0]),
            (Corner::ClosedOut, Corner::ClosedOut)
        );
        // Each shell is a chunk less twice its depth long, so shell 1's cut is half its 16.7 yd.
        for (k, cut) in [(0, FRONT_CHAMFER_YD), (1, 8.333)] {
            let (joint, _) = shell_line(south, k).expect("south shell");
            let (other, _) = shell_line(east, k).expect("east shell");
            assert!(joint.distance(other) < 0.01, "shell {k}: one joint");
            let a = shell_columns(south, k).expect("south shell")[0];
            let b = shell_columns(east, k).expect("east shell")[0];
            assert!(a.distance(b) < 0.01, "shell {k}: {a} vs {b}");
            for s in [south, east] {
                let u = (Vec2::from(s.to) - Vec2::from(s.from)).normalize();
                let n = Vec2::from(s.inward);
                let m = a - joint;
                assert!((m.dot(u) - cut / 2.0).abs() < 0.01, "shell {k}: {m} along");
                assert!((m.dot(n) - cut / 2.0).abs() < 0.01, "shell {k}: {m} inward");
            }
        }
        for k in 2..5 {
            assert_eq!(shell_columns(south, k), None, "shell {k} is left out");
        }
    }

    #[test]
    fn an_open_out_corner_keeps_its_front_and_cuts_its_back() {
        let found = segments(&[chunk(CX, CY)], closed_north_east);
        let [north, east] = found.as_slice() else {
            panic!("north and east segments, got {found:?}");
        };
        let corner = Vec2::from(north.to);
        // Back from the mitre toward the open chunk, along both sides.
        let back = -(Vec2::from(north.inward) + Vec2::from(east.inward));
        for (k, cut) in [(0, 0.0), (1, 16.667), (4, CHAMFER_YD)] {
            let (_, joint) = shell_line(north, k).expect("north shell");
            let m = joint + back * cut / 2.0;
            let a = shell_columns(north, k).expect("north shell")[10];
            let b = shell_columns(east, k).expect("east shell")[0];
            assert!(a.distance(m) < 0.01, "shell {k}: {a} vs {m}");
            assert!(b.distance(m) < 0.01, "shell {k}: {b} vs {m}");
        }
        let front = shell_columns(north, 0).expect("front shell")[10];
        assert!(
            front.distance(corner) < 0.01,
            "the front keeps the grid corner"
        );
    }

    #[test]
    fn a_one_chunk_staircase_turns_no_sharper_than_135_degrees() {
        let found = segments(&five_by_five(), staircase);
        for k in 1..5 {
            // Every piece of the shell between two distinct columns, across all segments.
            let pieces: Vec<(Vec2, Vec2)> = found
                .iter()
                .filter_map(|s| shell_columns(s, k))
                .flat_map(|c| c.windows(2).map(|w| (w[0], w[1])).collect::<Vec<_>>())
                .filter(|(a, b)| a.distance(*b) > 0.01)
                .collect();
            // The angle between two pieces that share an end, each pointing away from it.
            let mut turns = 0;
            for (i, &(a0, a1)) in pieces.iter().enumerate() {
                for &(b0, b1) in &pieces[i + 1..] {
                    for (p, a, q, b) in [
                        (a0, a1, b0, b1),
                        (a0, a1, b1, b0),
                        (a1, a0, b0, b1),
                        (a1, a0, b1, b0),
                    ] {
                        if p.distance(q) >= 0.01 {
                            continue;
                        }
                        let cos = (a - p).normalize().dot((b - q).normalize());
                        let angle = cos.clamp(-1.0, 1.0).acos().to_degrees();
                        assert!(angle >= 135.0 - 0.01, "shell {k}: {angle} degrees at {p}");
                        turns += 1;
                    }
                }
            }
            assert!(turns > 4, "shell {k}: only {turns} joins");
        }
    }

    #[test]
    fn a_straight_end_is_not_cut() {
        let closed = |gx, gy| gy == CY + 1 && (CX..=CX + 1).contains(&gx);
        let found = segments(&[chunk(CX, CY), chunk(CX + 1, CY)], closed);
        let [a, b] = found.as_slice() else {
            panic!("two segments, got {found:?}");
        };
        for k in 0..4 {
            let (_, joint) = shell_line(a, k).expect("west shell");
            let ca = shell_columns(a, k).expect("west shell");
            let cb = shell_columns(b, k).expect("east shell");
            for p in [ca[9], ca[10], cb[0], cb[1]] {
                assert!(p.distance(joint) < 0.01, "shell {k}: {p} vs {joint}");
            }
        }
    }

    #[test]
    fn no_chamfer_stands_on_open_land() {
        let straight = |gx, gy| gy == CY + 1 && (CX..=CX + 1).contains(&gx);
        let east = |gx, gy| (gx, gy) == (CX + 1, CY);
        let cases: [(&str, Vec<ChunkMesh>, &dyn Fn(u32, u32) -> bool); 5] = [
            (
                "straight",
                vec![chunk(CX, CY), chunk(CX + 1, CY)],
                &straight,
            ),
            ("closed out", vec![chunk(CX, CY)], &east),
            ("closed corner", south_east_borders(), &closed_south_east),
            ("open out", vec![chunk(CX, CY)], &closed_north_east),
            ("staircase", five_by_five(), &staircase),
        ];
        let index = |v: f32| (32.0 * TILE_SIZE - v) / CHUNK_SIZE;
        let on_line = |i: f32| (i - i.round()).abs() * CHUNK_SIZE < 0.01;
        for (name, chunks, closed) in cases {
            let found = segments(&chunks, closed);
            assert!(!found.is_empty(), "{name}");
            let (positions, ..) = mesh_parts(&curtain_mesh(&found));
            for [x, y, _] in positions {
                let (gx, gy) = (index(y), index(x));
                if on_line(gx) || on_line(gy) {
                    continue;
                }
                let (gx, gy) = (gx.floor() as u32, gy.floor() as u32);
                assert!(
                    closed(gx, gy),
                    "{name}: ({x}, {y}) on open chunk ({gx}, {gy})"
                );
            }
        }
    }

    #[test]
    fn shell_heights_come_from_the_closed_chunk() {
        // The closed chunk's heights sit 1000 yd over the open one's, to tell them apart.
        let mut south = chunk(CX, CY + 1);
        for p in &mut south.positions {
            p[2] += 1000.0;
        }
        // Shell 4's column grounds: each top vertex's height less its yards above the ground,
        // skipping the first column, which repeats the second at a straight end.
        let back_grounds = |chunks: &[ChunkMesh]| {
            let found = segments(chunks, closed_south_row);
            let (positions, uv0, _, _) = mesh_parts(&curtain_mesh(&found));
            (0..9)
                .map(|k| positions[4 * 22 + 12 + k][2] - uv0[4 * 22 + 12 + k][0])
                .collect::<Vec<f32>>()
        };
        // Row 8 of the closed chunk to the south is its south row, vertices 136..=144; the open
        // chunk's row 0 is its own south row, the same indices.
        let with = back_grounds(&[chunk(CX, CY), south]);
        let without = back_grounds(&[chunk(CX, CY)]);
        for k in 0..9 {
            let row = 136.0 + k as f32;
            assert!(
                (with[k] - (1000.0 + row)).abs() < 0.01,
                "with, column {k}: {}",
                with[k]
            );
            assert!(
                (without[k] - row).abs() < 0.01,
                "without, column {k}: {}",
                without[k]
            );
        }
    }

    #[test]
    fn the_drift_wraps_on_whole_noise_periods() {
        let whole = |v: f64| (v - v.round()).abs() < 1e-3;
        let period = NOISE_PERIOD_YD as f64;
        for m in LAYER_DRIFT {
            let m = m as f64;
            let along = m * DRIFT_YD_S as f64 * DRIFT_WRAP_SECS / period;
            let up = m.abs() * DRIFT_UP_YD_S as f64 * DRIFT_WRAP_SECS / period;
            assert!(whole(along), "{m}: {along} periods along");
            assert!(whole(up), "{m}: {up} periods up");
        }
        assert!(DRIFT_WRAP_SECS > 3600.0);
    }

    fn test_wall() -> MistWall {
        MistWall {
            key: "test".to_string(),
            map: 369,
            from: [0.0, 0.0],
            to: [8.0, 0.0],
            bottom: -20.0,
            top: 30.0,
            floor: -4.3,
            open_side: [4.0, 5.0],
        }
    }

    #[test]
    fn a_placed_walls_segment_steps_its_rows_toward_the_open_side() {
        let wall = test_wall();
        let s = wall_segment(&wall);
        assert_eq!((s.from, s.to), (wall.from, wall.to));
        assert_eq!((s.bottom, s.top), (wall.bottom, wall.top));
        for (j, row) in s.rows.iter().enumerate() {
            for (k, p) in row.iter().enumerate() {
                assert_eq!(*p, [k as f32, j as f32, wall.floor], "row {j}, point {k}");
            }
        }
        let mesh = curtain_mesh(&[s]);
        assert_eq!(mesh.indices().map(|i| i.len()), Some(5 * 60));
    }

    #[test]
    fn a_placed_walls_shells_step_away_from_its_open_side() {
        let wall = test_wall();
        let s = wall_segment(&wall);
        // The open side is north of the wall in `y`, so the shells step to negative `y`.
        assert_eq!(s.inward, [0.0, -1.0]);
        assert_eq!((s.ends, s.closed), ([Corner::Straight; 2], None));
        let (positions, ..) = mesh_parts(&curtain_mesh(&[s]));
        assert_eq!(positions.len(), 5 * 22);
        for (k, depth) in [0.0, 1.5, 3.0, 4.5, 6.0].into_iter().enumerate() {
            for (i, &[x, y, z]) in positions[k * 22..(k + 1) * 22].iter().enumerate() {
                assert!((y + depth).abs() < 1e-4, "shell {k}: y {y}");
                // Straight ends: the first and last columns repeat their neighbours.
                let along = (i % 11).clamp(1, 9) - 1;
                assert!((x - along as f32).abs() < 1e-4, "shell {k}: x {x}");
                let want = if i < 11 { wall.bottom } else { wall.top };
                assert!((z - want).abs() < 1e-4, "shell {k}: z {z}");
            }
        }
    }

    #[test]
    fn placed_walls_take_their_own_material() {
        let [zone, wall] = mist_exts();
        assert_eq!(
            (zone.shape.x, zone.rag.x, zone.foot.z),
            (OPAQUE_YD, RAG_YD, COLOUR_RAMP_YD)
        );
        assert_eq!(
            (wall.shape.x, wall.rag.x, wall.foot.z),
            (WALL_OPAQUE_YD, WALL_RAG_YD, WALL_COLOUR_RAMP_YD)
        );
        // Every other lane alike, the fade factor 1 on both.
        assert_eq!(zone.shape.yzw(), wall.shape.yzw());
        assert_eq!(zone.rag.yzw(), wall.rag.yzw());
        assert_eq!(wall.rag.w, 1.0);
        assert_eq!((zone.foot.x, zone.foot.y), (wall.foot.x, wall.foot.y));
        assert_eq!(zone.foot.w, wall.foot.w);
        assert_eq!(
            (zone.fog, zone.scene, zone.drift),
            (wall.fog, wall.scene, wall.drift)
        );
    }

    #[test]
    fn the_placed_wall_reads_the_wmo_fog() {
        let lighting = WowLighting {
            fog_color: [0.5, 0.6, 0.7],
            fog_start: 100.0,
            fog_end: 400.0,
            wmo_fog_color: [30.0 / 255.0, 53.0 / 255.0, 100.0 / 255.0],
            wmo_fog_start: 50.0,
            wmo_fog_end: 236.1,
            ..default()
        };
        let [zone, wall] = fog_lanes(&lighting, 12.0);
        assert_eq!(zone.0, Vec4::new(0.5, 0.6, 0.7, 12.0));
        assert_eq!(zone.1, scene_lanes(100.0, 400.0));
        assert_eq!(
            wall.0,
            Vec4::new(30.0 / 255.0, 53.0 / 255.0, 100.0 / 255.0, 12.0)
        );
        assert_eq!(wall.1, scene_lanes(50.0, 236.1));
    }

    #[test]
    fn a_placed_walls_shells_span_the_wall() {
        let (positions, ..) = mesh_parts(&curtain_mesh(&[wall_segment(&test_wall())]));
        assert_eq!(positions.len(), 5 * 22);
        for (k, shell) in positions.chunks(22).enumerate() {
            let (feet, tops) = shell.split_at(11);
            assert!(feet.iter().all(|p| (p[2] + 20.0).abs() < 1e-4), "shell {k}");
            assert!(tops.iter().all(|p| (p[2] - 30.0).abs() < 1e-4), "shell {k}");
        }
    }

    #[test]
    fn a_placed_walls_box_spans_the_wall_at_its_middle() {
        let wall = test_wall();
        let (centre, turn, size) = wall_box(&wall);
        assert!(centre.abs_diff_eq(wow_to_bevy([4.0, 0.0, 5.0]), 1e-4));
        assert_eq!(size, Vec3::new(8.0, 50.0, WALL_THICK_YD));
        let along = turn * Vec3::X;
        assert!(along.abs_diff_eq(wow_to_bevy([1.0, 0.0, 0.0]), 1e-4));
    }
}
