//! **The friends server** (ADR 0009): one [`Realm`] on its own thread, ticked on a fixed clock,
//! and a TCP connection per client. A connection is a realm session; its frames are
//! [`RealmCommand`]s in and `SessionEvent`s out ([`twilight_realm::wire`]), after a first
//! [`Hello`] frame that names the account and carries the join password and build id.

use std::collections::HashMap;
use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use benilla_protocol::SessionEnd;
use benilla_protocol::SessionEvent;
use twilight_realm::wire::{read_frame, write_frame, Hello};
use twilight_realm::{Host, Realm, RealmCommand, SessionId, Storage};

/// Where the server listens unless `--listen` says otherwise.
pub const DEFAULT_LISTEN: &str = "0.0.0.0:3725";

/// The realm's tick rate.
const TICK: Duration = Duration::from_millis(50);

/// How long a dropped connection's character stays in the world before it is saved and removed.
const LINGER: Duration = Duration::from_secs(20);

mod accounts;
mod collision;
mod liquid;
mod terrain;

use accounts::Accounts;

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

/// A world with no game data, for tests: no ground is known (a mob keeps its own height; the
/// realm probes 2 yd above it, so echoing the probe would lift it every step), and nothing blocks
/// a sight line.
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
        account: String,
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

/// How the server is run.
pub struct Config {
    /// Where the saves and `accounts.toml` live.
    pub data_dir: PathBuf,
    /// What a client's [`Hello`] must carry to get in.
    pub join_password: String,
    /// This server's [`twilight_realm::build_id::build_id`]; a client of another is refused.
    pub build_id: String,
}

/// Serves `listener` in `world` until `<data-dir>/stop` appears; then everyone is saved and this
/// returns.
pub fn serve(listener: TcpListener, config: Config, world: Box<dyn World>) {
    let (tx, rx) = mpsc::channel();
    let gate = std::sync::Arc::new((config.join_password.clone(), config.build_id.clone()));
    let realm = std::thread::spawn(move || run_realm(rx, &config.data_dir, world));
    std::thread::spawn(move || accept_loop(listener, tx, gate));
    let _ = realm.join();
}

fn accept_loop(listener: TcpListener, tx: Sender<Inbound>, gate: std::sync::Arc<(String, String)>) {
    for (conn, stream) in (1..).zip(listener.incoming()) {
        match stream {
            Ok(stream) => {
                let (tx, gate) = (tx.clone(), gate.clone());
                std::thread::spawn(move || accept(conn, stream, tx, &gate.0, &gate.1));
            }
            Err(e) => eprintln!("twilight-server: accept failed: {e}"),
        }
    }
}

/// Why `hello` may not come in, if it may not.
fn refusal(hello: &Hello, join_password: &str, build_id: &str) -> Option<String> {
    if hello.build_id != build_id {
        return Some(format!(
            "runs build {build_id}, and this client is build {}; get the server's client build",
            hello.build_id
        ));
    }
    if hello.join_password != join_password {
        return Some("refused the join password".to_string());
    }
    if Accounts::key(&hello.account).is_empty() {
        return Some("needs an account name (--account or WOW_ACCOUNT)".to_string());
    }
    None
}

/// Reads a connection's [`Hello`], then reads its commands on this thread and writes its events
/// on another.
fn accept(conn: u64, stream: TcpStream, tx: Sender<Inbound>, join_password: &str, build_id: &str) {
    let peer = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_default();
    eprintln!("twilight-server: connection {conn} from {peer}");
    let _ = stream.set_nodelay(true);
    let Ok(write_half) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(stream);
    let Ok(hello) = read_frame::<_, Hello>(&mut reader) else {
        eprintln!("twilight-server: connection {conn} sent no hello");
        return;
    };
    if let Some(why) = refusal(&hello, join_password, build_id) {
        eprintln!("twilight-server: connection {conn} refused: {why}");
        let mut w = BufWriter::new(write_half);
        let refused = SessionEvent::Disconnected {
            reason: format!("the server {why}"),
            end: SessionEnd::Lost,
        };
        let _ = write_frame(&mut w, &refused).and_then(|()| w.flush());
        return;
    }
    let (events_tx, events_rx) = mpsc::channel();
    if tx
        .send(Inbound::Open {
            conn,
            account: Accounts::key(&hello.account),
            events: events_tx,
        })
        .is_err()
    {
        return;
    }
    std::thread::spawn(move || write_events(write_half, events_rx));
    while let Ok(command) = read_frame::<_, RealmCommand>(&mut reader) {
        if tx.send(Inbound::Command { conn, command }).is_err() {
            return;
        }
    }
    eprintln!("twilight-server: connection {conn} closed");
    let _ = tx.send(Inbound::Closed { conn });
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
    // The realm is gone: the client sees its connection close.
    let _ = w.get_ref().shutdown(std::net::Shutdown::Both);
}

/// A connection the realm serves.
struct Conn {
    session: SessionId,
    account: String,
    events: Sender<SessionEvent>,
}

/// The realm thread: applies commands as they arrive and ticks at [`TICK`].
fn run_realm(rx: Receiver<Inbound>, data_dir: &Path, mut world: Box<dyn World>) {
    let mut realm = Realm::with_seed(seed());
    load_saves(&mut realm, data_dir);
    let mut accounts = Accounts::load(data_dir);
    let mut conns: HashMap<u64, Conn> = HashMap::new();
    // Dropped connections whose characters stay in the world until their time is up.
    let mut lingering: Vec<(Instant, String, SessionId)> = Vec::new();
    let mut last = Instant::now();
    let mut next = last + TICK;
    loop {
        // Commands until the next tick is due.
        loop {
            let wait = next.saturating_duration_since(Instant::now());
            match rx.recv_timeout(wait) {
                Ok(Inbound::Open {
                    conn,
                    account,
                    events,
                }) => {
                    accounts.register(&account);
                    // Back before the linger ran out: the old character leaves now, saved.
                    lingering.retain(|(_, a, session)| {
                        let back = *a == account;
                        if back {
                            realm.disconnect(*session);
                        }
                        !back
                    });
                    let session = realm.connect();
                    conns.insert(
                        conn,
                        Conn {
                            session,
                            account,
                            events,
                        },
                    );
                }
                Ok(Inbound::Command { conn, command }) => {
                    let Some(c) = conns.get(&conn) else {
                        continue;
                    };
                    if !accounts.allows(&c.account, &command) {
                        continue;
                    }
                    let creates = matches!(command, RealmCommand::CreateCharacter(_));
                    realm.handle_as(c.session, command);
                    if creates {
                        // The new character's first save names its guid, which no one owns yet.
                        for op in realm.drain_storage() {
                            if let Storage::Write { guid, .. } = &op {
                                accounts.claim(&c.account, *guid);
                            }
                            store(data_dir, op);
                        }
                    }
                }
                Ok(Inbound::Closed { conn }) => {
                    if let Some(c) = conns.remove(&conn) {
                        lingering.push((Instant::now() + LINGER, c.account, c.session));
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
        let stop = data_dir.join("stop");
        if stop.exists() {
            for (_, c) in conns.drain() {
                realm.disconnect(c.session);
            }
            for (_, _, session) in lingering.drain(..) {
                realm.disconnect(session);
            }
            let mut saved = 0;
            for op in realm.drain_storage() {
                if matches!(op, Storage::Write { .. }) {
                    saved += 1;
                }
                store(data_dir, op);
            }
            if let Err(e) = std::fs::remove_file(&stop) {
                eprintln!("twilight-server: cannot delete the stop file: {e}");
            }
            eprintln!("twilight-server: stop file seen, saved {saved} characters, exiting");
            return;
        }
        let now = Instant::now();
        next = now + TICK;
        lingering.retain(|&(until, _, session)| {
            let due = now >= until;
            if due {
                realm.disconnect(session);
            }
            !due
        });
        if let Ok(wall) = SystemTime::now().duration_since(UNIX_EPOCH) {
            realm.set_wall_clock(wall.as_millis() as u64);
        }
        world.refresh(&realm.player_positions());
        realm.tick((now - last).as_secs_f32(), &*world);
        last = now;
        for op in realm.drain_storage() {
            if let Storage::Remove { guid } = op {
                accounts.release(guid);
            }
            store(data_dir, op);
        }
        let by_session: HashMap<SessionId, &Conn> =
            conns.values().map(|c| (c.session, c)).collect();
        // A lingering session's events have no one to go to.
        for (session, event) in realm.drain_session_events() {
            if let Some(c) = by_session.get(&session) {
                let _ = c.events.send(accounts.filter(&c.account, event));
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
