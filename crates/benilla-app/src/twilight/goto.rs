//! The dev teleport's ground (`WOW_PROBE_GOTO`): the realm grounds a [`DevGoto`] through its
//! host, and the host answers from the streamed terrain, which reaches about a tile from the
//! player. For a spot beyond it, this loads that one ADT tile first and holds the request until
//! the tile is in (or fails to load), so [`GotoGround::height`] can answer from its heightfield.

use benilla_assets::coords::wow_to_bevy;
use benilla_assets::AdtTile;
use benilla_formats::{terrain_height_at, world_to_tile};
use benilla_world::terrain_stream::TerrainStreamer;
use benilla_world::world_point::WorldPoint;
use bevy::asset::LoadState;
use bevy::prelude::*;

/// A dev aid for the realm: teleport the player to `(x, y)` on its map, on the ground there
/// (`WOW_PROBE_GOTO`, `capture::probes::goto`). `facing` `None` keeps the player's own.
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct DevGoto {
    pub x: f32,
    pub y: f32,
    pub facing: Option<f32>,
}

/// How long a request waits for its tile before it goes to the realm anyway, to be refused.
const TILE_WAIT_SECS: f32 = 30.0;

/// The requests waiting for ground, those ready for the realm, and the tile fetched for them.
#[derive(Resource, Default)]
pub(super) struct GotoGround {
    waiting: Option<(DevGoto, f32)>,
    ready: Vec<DevGoto>,
    /// The ADT tile fetched for the last spot beyond the stream window, kept while it may be asked.
    tile: Option<((u32, u32), Handle<AdtTile>)>,
}

impl GotoGround {
    /// The requests whose ground is in, for the realm.
    pub(super) fn take_ready(&mut self) -> Vec<DevGoto> {
        std::mem::take(&mut self.ready)
    }

    /// The fetched tile's terrain height under `(x, y)`; `None` off it or before it loads.
    pub(super) fn height(&self, adts: &Assets<AdtTile>, x: f32, y: f32) -> Option<f32> {
        let (key, handle) = self.tile.as_ref()?;
        if *key != world_to_tile(x, y) {
            return None;
        }
        terrain_height_at(&adts.get(handle)?.chunks, [x, y, 0.0])
    }
}

/// Takes each [`DevGoto`]: ready at once where the streamed terrain has ground, otherwise after
/// its tile loads.
pub(super) fn fetch_goto_ground(
    mut requests: MessageReader<DevGoto>,
    mut ground: ResMut<GotoGround>,
    world: WorldPoint,
    streamer: Res<TerrainStreamer>,
    asset_server: Res<AssetServer>,
    time: Res<Time<bevy::time::Real>>,
) {
    let now = time.elapsed_secs();
    for &goto in requests.read() {
        if world
            .terrain_height_under(wow_to_bevy([goto.x, goto.y, 0.0]))
            .is_some()
        {
            ground.ready.push(goto);
            continue;
        }
        let Some(dir) = streamer.map_dir() else {
            ground.ready.push(goto);
            continue;
        };
        let (tx, ty) = world_to_tile(goto.x, goto.y);
        info!("twilight: dev teleport waits for tile {dir}_{tx}_{ty}");
        let handle = asset_server.load(format!("mpq://World/Maps/{dir}/{dir}_{tx}_{ty}.adt"));
        ground.tile = Some(((tx, ty), handle));
        ground.waiting = Some((goto, now));
    }
    let Some((goto, since)) = ground.waiting else {
        return;
    };
    let settled = ground.tile.as_ref().is_some_and(|(_, handle)| {
        matches!(
            asset_server.load_state(handle),
            LoadState::Loaded | LoadState::Failed(_)
        )
    });
    if settled || now >= since + TILE_WAIT_SECS {
        ground.waiting = None;
        ground.ready.push(goto);
    }
}
