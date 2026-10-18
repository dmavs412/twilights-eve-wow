//! **The friends server** (ADR 0009): one [`Realm`] on its own thread, ticked on a fixed clock,
//! and a TCP connection per client. A connection is a realm session; its frames are
//! [`RealmCommand`]s in and `SessionEvent`s out ([`twilight_realm::wire`]).

use std::collections::HashMap;
use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use benilla_protocol::SessionEvent;
use twilight_realm::wire::{read_frame, write_frame};
use twilight_realm::{Host, Realm, RealmCommand, SessionId, Storage};

/// Where the server listens unless `--listen` says otherwise.
pub const DEFAULT_LISTEN: &str = "0.0.0.0:3725";

/// The realm's tick rate.
const TICK: Duration = Duration::from_millis(50);

mod collision;
mod terrain;

pub use terrain::Terrain;

/// The world the realm runs in, as the server keeps it: a [`Host`] that loads what the players
/// are near before each tick.
pub trait World: Host + Send {
    /// Called before each tick with where every character in the world stands.
    fn refresh(&mut self, _players: &[(u32, [f32; 3])]) {}
}

impl World for Terrain {
    fn refresh(&mut self, players: &[(u32, [f32; 3])]) {
        Terrain::refresh(self, players);
    }
}

impl World for FlatHost {}

/// A world with no game data, for tests: no ground is known (a mob keeps its own height; the realm probes 2 yd above it, so
/// echoing the probe would lift it every step), and nothing blocks a sight line.
pub struct FlatHost;

impl Host for FlatHost {
    fn ground_height(&self, _map: u32, _x: f32, _y: f32, _z_hint: f32) -> Option<f32> {
        None
    }
    fn line_of_sight(&self, _map: u32, _from: [f32; 3], _to: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _map: u32, _x: f32, _y: f32) -> Option<u32> {
        None
    }
}

/// What a connection's threads tell the realm thread.
enum Inbound {
    Open {
        conn: u64,
        events: Sender<SessionEvent>,
    },
    Command {
        conn: u64,
        command: RealmCommand,
    },
    Closed {
        conn: u64,
    },
}

/// Serves `listener` forever in `world`, keeping saves under `data_dir`.
pub fn serve(listener: TcpListener, data_dir: PathBuf, world: Box<dyn World>) {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || run_realm(rx, &data_dir, world));
    for (conn, stream) in (1..).zip(listener.incoming()) {
        match stream {
            Ok(stream) => accept(conn, stream, tx.clone()),
            Err(e) => eprintln!("twilight-server: accept failed: {e}"),
        }
    }
}

/// Starts a connection's reader and writer threads.
fn accept(conn: u64, stream: TcpStream, tx: Sender<Inbound>) {
    let peer = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_default();
    eprintln!("twilight-server: connection {conn} from {peer}");
    let _ = stream.set_nodelay(true);
    let Ok(write_half) = stream.try_clone() else {
        return;
    };
    let (events_tx, events_rx) = mpsc::channel();
    if tx
        .send(Inbound::Open {
            conn,
            events: events_tx,
        })
        .is_err()
    {
        return;
    }
    std::thread::spawn(move || write_events(write_half, events_rx));
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stream);
        while let Ok(command) = read_frame::<_, RealmCommand>(&mut reader) {
            if tx.send(Inbound::Command { conn, command }).is_err() {
                return;
            }
        }
        eprintln!("twilight-server: connection {conn} closed");
        let _ = tx.send(Inbound::Closed { conn });
    });
}

/// Writes a connection's events as they come, flushing once the queue runs dry.
fn write_events(stream: TcpStream, events: Receiver<SessionEvent>) {
    let mut w = BufWriter::new(stream);
    while let Ok(event) = events.recv() {
        let mut ok = write_frame(&mut w, &event).is_ok();
        for event in events.try_iter() {
            ok = ok && write_frame(&mut w, &event).is_ok();
        }
        if !ok || w.flush().is_err() {
            let _ = w.get_ref().shutdown(std::net::Shutdown::Both);
            return;
        }
    }
}

/// The realm thread: applies commands as they arrive and ticks at [`TICK`].
fn run_realm(rx: Receiver<Inbound>, data_dir: &Path, mut world: Box<dyn World>) {
    let mut realm = Realm::with_seed(seed());
    load_saves(&mut realm, data_dir);
    let mut sessions: HashMap<u64, (SessionId, Sender<SessionEvent>)> = HashMap::new();
    let mut last = Instant::now();
    let mut next = last + TICK;
    loop {
        // Commands until the next tick is due.
        loop {
            let wait = next.saturating_duration_since(Instant::now());
            match rx.recv_timeout(wait) {
                Ok(Inbound::Open { conn, events }) => {
                    sessions.insert(conn, (realm.connect(), events));
                }
                Ok(Inbound::Command { conn, command }) => {
                    if let Some(&(session, _)) = sessions.get(&conn) {
                        realm.handle_as(session, command);
                    }
                }
                Ok(Inbound::Closed { conn }) => {
                    if let Some((session, _)) = sessions.remove(&conn) {
                        realm.disconnect(session);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
        let now = Instant::now();
        next = now + TICK;
        if let Ok(wall) = SystemTime::now().duration_since(UNIX_EPOCH) {
            realm.set_wall_clock(wall.as_millis() as u64);
        }
        world.refresh(&realm.player_positions());
        realm.tick((now - last).as_secs_f32(), &*world);
        last = now;
        for op in realm.drain_storage() {
            store(data_dir, op);
        }
        let by_session: HashMap<SessionId, &Sender<SessionEvent>> =
            sessions.values().map(|(s, tx)| (*s, tx)).collect();
        for (session, event) in realm.drain_session_events() {
            if let Some(tx) = by_session.get(&session) {
                let _ = tx.send(event);
            }
        }
        for line in realm.drain_log() {
            eprintln!("twilight-server: {line}");
        }
    }
}

fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64)
}

/// `<data-dir>/characters/<guid>.toml`: one character's save, the blob solo writes.
fn save_path(data_dir: &Path, guid: u64) -> PathBuf {
    data_dir.join("characters").join(format!("{guid}.toml"))
}

/// Imports every save under `data_dir`; one that cannot be read keeps its guid reserved.
fn load_saves(realm: &mut Realm, data_dir: &Path) {
    let Ok(dir) = std::fs::read_dir(data_dir.join("characters")) else {
        return;
    };
    for entry in dir.flatten() {
        let path = entry.path();
        let Some(guid) = path
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.parse::<u64>().ok())
        else {
            continue;
        };
        realm.reserve_guid(guid);
        match std::fs::read_to_string(&path).map(|blob| realm.import_character(&blob)) {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => eprintln!("twilight-server: skipping {} ({e:?})", path.display()),
            Err(e) => eprintln!("twilight-server: cannot read {}: {e}", path.display()),
        }
    }
}

/// Carries out one save request; a failure is logged, never fatal.
fn store(data_dir: &Path, op: Storage) {
    let result = match op {
        Storage::Write { guid, blob } => {
            let path = save_path(data_dir, guid);
            let tmp = path.with_extension("toml.tmp");
            std::fs::create_dir_all(data_dir.join("characters"))
                .and_then(|()| std::fs::write(&tmp, blob))
                .and_then(|()| std::fs::rename(&tmp, &path))
        }
        Storage::Remove { guid } => match std::fs::remove_file(save_path(data_dir, guid)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            r => r,
        },
    };
    if let Err(e) = result {
        eprintln!("twilight-server: saving: {e}");
    }
}
