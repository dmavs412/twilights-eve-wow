//! **Twilight's Eve**, the glue half (ADR 0001): the boot flag, and the in-process backend that
//! puts the Local Realm ([`twilight_realm`]) on the far ends of the net layer's channels. Once a
//! frame, before the event drain, it hands the realm what the client asked, ticks it on the main
//! thread, and pushes its events where the IO thread would. The rules live in the realm crate;
//! nothing here decides anything.

use benilla_assets::coords::wow_to_bevy;
use benilla_assets::AdtTile;
use benilla_protocol::{SessionEnd, SessionEvent};
use benilla_world::schedule::WorldStage;
use benilla_world::terrain_stream::{terrain_height_under, TerrainStreamer};
use bevy::prelude::*;
use twilight_realm::{Host, Realm, RealmCommand};

use crate::net::{CharRequest, ClientCommand, FarEnds, InProcessLink};

/// `WOW_TWILIGHT` natively, `?twilight=1` in the browser: boot Twilight's Eve.
pub(crate) fn active() -> bool {
    crate::webenv::var("WOW_TWILIGHT").is_some_and(|v| v != "0")
}

/// Attaches the Local Realm. Added after `NetPlugin`, whose [`InProcessLink`] it takes.
pub(crate) struct TwilightPlugin;

impl Plugin for TwilightPlugin {
    fn build(&self, app: &mut App) {
        let far = app
            .world_mut()
            .get_resource_mut::<InProcessLink>()
            .and_then(|mut link| link.0.take())
            .expect("Twilight's Eve needs the net layer's in-process backend");
        app.insert_resource(LocalRealm {
            far,
            realm: Realm::new(),
            listed: false,
        })
        .add_systems(
            Update,
            pump.in_set(WorldStage::Net)
                .before(crate::net::apply_net_updates),
        )
        .add_systems(
            Update,
            only_warriors.run_if(resource_added::<crate::entities::CharCreate>),
        );
    }
}

/// The realm and the channel ends it serves.
#[derive(Resource)]
struct LocalRealm {
    far: FarEnds,
    realm: Realm,
    /// The first roster went out: there is no login, so the realm answers at once.
    listed: bool,
}

/// Commands in, one tick, events out.
fn pump(
    mut local: ResMut<LocalRealm>,
    time: Res<Time>,
    streamer: Option<Res<TerrainStreamer>>,
    tiles: Res<Assets<AdtTile>>,
) {
    let LocalRealm { far, realm, listed } = &mut *local;
    if !*listed {
        *listed = true;
        realm.handle(RealmCommand::ListCharacters);
    }
    while let Ok(pick) = far.pick.try_recv() {
        realm.handle(match pick {
            CharRequest::Enter(guid) => RealmCommand::EnterWorld { guid },
            CharRequest::Create(req) => RealmCommand::CreateCharacter(req),
            CharRequest::Delete(guid) => RealmCommand::DeleteCharacter { guid },
            // Select's Back has no login screen to go to; the roster brings the screen back.
            CharRequest::Abandon => RealmCommand::ListCharacters,
        });
    }
    // One realm and no login: the realm list and credentials have nothing to answer.
    while far.realm.try_recv().is_ok() {}
    while far.login.try_recv().is_ok() {}
    while let Ok(command) = far.commands.try_recv() {
        if let Some(command) = realm_command(command) {
            realm.handle(command);
        }
    }

    let host = TerrainHost {
        streamer: streamer.as_deref(),
        tiles: &tiles,
    };
    realm.tick(time.delta_secs(), &host);

    for event in realm.drain_events() {
        let logged_out = matches!(event, SessionEvent::LoggedOut);
        let _ = far.events.send(event);
        // The IO thread ends the session after a logout, which tears the streamed world down.
        if logged_out {
            let _ = far.events.send(SessionEvent::Disconnected {
                reason: "logged out".into(),
                end: SessionEnd::LoggedOut,
            });
        }
    }
}

/// The client's commands the realm understands; the rest are dropped.
fn realm_command(command: ClientCommand) -> Option<RealmCommand> {
    match command {
        ClientCommand::Move {
            pos, orientation, ..
        } => Some(RealmCommand::Move {
            position: pos,
            orientation,
        }),
        ClientCommand::Logout => Some(RealmCommand::Logout),
        _ => None,
    }
}

/// The realm's view of the world, answered from the streamed terrain.
struct TerrainHost<'a> {
    streamer: Option<&'a TerrainStreamer>,
    tiles: &'a Assets<AdtTile>,
}

impl Host for TerrainHost<'_> {
    fn ground_height(&self, _map: u32, x: f32, y: f32, z_hint: f32) -> Option<f32> {
        terrain_height_under(self.streamer?, self.tiles, wow_to_bevy([x, y, z_hint]))
    }

    // Nothing asks yet: the first reader is mob aggro, which brings the collision query with it.
    fn line_of_sight(&self, _map: u32, _from: [f32; 3], _to: [f32; 3]) -> bool {
        true
    }
}

/// Basic Warrior is the only Calling a character starts as, so create offers only Warrior.
fn only_warriors(mut catalog: ResMut<crate::entities::CharCreate>) {
    catalog
        .0
        .retain_classes(|class| class == benilla_protocol::messages::CLASS_WARRIOR);
}
