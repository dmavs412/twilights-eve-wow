//! **Where the net layer's channel ends lead.** The app holds one end of each channel as a
//! resource ([`NetEvents`], [`NetCommands`], [`CharPick`], [`RealmChoice`], [`LoginSubmit`]); a
//! backend serves the other ends ([`FarEnds`]). The IO thread is one backend and the default; an
//! in-process backend (the Local Realm, ADR 0001) takes the same far ends from [`InProcessLink`],
//! so the game reads the same resources whichever one feeds them.

use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use benilla_protocol::SessionEvent;
use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};

use super::io::{LoginRequest, PingClock};
use super::{CharRequest, ClientCommand, RealmRequest};

/// Which backend serves the far ends of the net layer's channels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum NetBackend {
    /// The IO thread (a browser task on the web): logon, realm and world sockets.
    #[default]
    Io,
    /// Capture mode ([`crate::capture`]): the channels exist, sends are no-ops, and no IO thread
    /// runs.
    Offline,
    /// The far ends wait in [`InProcessLink`] for an in-process backend to take them.
    // Nothing boots into this yet; the Twilight's Eve glue will.
    #[allow(dead_code)]
    InProcess,
}

/// The app's ends, which `NetPlugin` inserts as resources.
pub(super) struct AppEnds {
    pub(super) events: Receiver<SessionEvent>,
    pub(super) commands: Sender<ClientCommand>,
    pub(super) pick: async_channel::Sender<CharRequest>,
    pub(super) realm: async_channel::Sender<RealmRequest>,
    pub(super) login: async_channel::Sender<LoginRequest>,
    pub(super) login_abandon: Arc<AtomicU64>,
    pub(super) ping: Arc<Mutex<PingClock>>,
}

/// The ends a backend serves: it pushes [`SessionEvent`]s and answers commands and park requests.
///
/// The park channels are `async_channel` because the IO sequencer waits on them; an in-process
/// backend polls them with `try_recv`.
pub(crate) struct FarEnds {
    pub(crate) events: Sender<SessionEvent>,
    pub(crate) commands: Receiver<ClientCommand>,
    pub(crate) pick: async_channel::Receiver<CharRequest>,
    pub(crate) realm: async_channel::Receiver<RealmRequest>,
    pub(crate) login: async_channel::Receiver<LoginRequest>,
}

/// [`NetBackend::InProcess`]'s far ends, waiting for the in-process backend to take them.
#[derive(Resource)]
pub(crate) struct InProcessLink(#[allow(dead_code)] pub(crate) Option<FarEnds>);

/// One pair of every channel the net layer uses.
pub(super) fn channels() -> (AppEnds, FarEnds) {
    let (events_tx, events_rx) = crossbeam_channel::unbounded();
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (pick_tx, pick_rx) = async_channel::unbounded();
    let (realm_tx, realm_rx) = async_channel::unbounded();
    let (login_tx, login_rx) = async_channel::unbounded();
    (
        AppEnds {
            events: events_rx,
            commands: cmd_tx,
            pick: pick_tx,
            realm: realm_tx,
            login: login_tx,
            login_abandon: Arc::new(AtomicU64::new(0)),
            ping: Arc::new(Mutex::new(PingClock::default())),
        },
        FarEnds {
            events: events_tx,
            commands: cmd_rx,
            pick: pick_rx,
            realm: realm_rx,
            login: login_rx,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::{CharPick, NetCommands, NetEvents, NetOffline, NetPlugin};

    fn app(backend: NetBackend) -> App {
        let mut app = App::new();
        app.add_plugins(NetPlugin { backend });
        app
    }

    #[test]
    fn an_in_process_backend_feeds_the_resources_the_game_reads() {
        let mut app = app(NetBackend::InProcess);
        assert!(app.world().get_resource::<NetOffline>().is_none());
        let far = app
            .world_mut()
            .resource_mut::<InProcessLink>()
            .0
            .take()
            .expect("far ends");

        // An event pushed by the backend lands where `apply_net_updates` drains it.
        far.events
            .send(SessionEvent::CharacterList {
                characters: Vec::new(),
                realm: None,
            })
            .unwrap();
        assert!(matches!(
            app.world().resource::<NetEvents>().0.try_recv(),
            Ok(SessionEvent::CharacterList { .. })
        ));

        // A command the game sends reaches the backend.
        app.world()
            .resource::<NetCommands>()
            .0
            .send(ClientCommand::SetSelection { guid: 7 })
            .unwrap();
        match far.commands.try_recv() {
            Ok(ClientCommand::SetSelection { guid }) => assert_eq!(guid, 7),
            _ => panic!("the command did not reach the backend"),
        }

        // So does a pick at character select.
        app.world()
            .resource::<CharPick>()
            .0
            .try_send(CharRequest::Enter(9))
            .unwrap();
        assert!(matches!(far.pick.try_recv(), Ok(CharRequest::Enter(9))));
    }

    #[test]
    fn offline_has_no_link_and_says_so() {
        let app = app(NetBackend::Offline);
        assert!(app.world().get_resource::<NetOffline>().is_some());
        assert!(app.world().get_resource::<InProcessLink>().is_none());
    }
}
