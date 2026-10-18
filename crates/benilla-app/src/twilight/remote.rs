//! **A friends server** (ADR 0009) in place of the in-process realm: `--server host:port` (or
//! `WOW_SERVER`) sends the realm's commands over TCP and reads its events back, framed by
//! [`twilight_realm::wire`]. The server keeps the saves; nothing is written here.

use crossbeam_channel::{Receiver, Sender};
use std::io::{BufReader, BufWriter, Write};
use std::net::TcpStream;

use benilla_protocol::{SessionEnd, SessionEvent};
use twilight_realm::wire::{read_frame, write_frame};
use twilight_realm::RealmCommand;

/// The server address this run joins: `--server host:port`, else `WOW_SERVER`; `None` plays solo.
pub(super) fn server_address() -> Option<String> {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "--server" {
            return args.next();
        }
        if let Some(addr) = arg.strip_prefix("--server=") {
            return Some(addr.to_string());
        }
    }
    std::env::var("WOW_SERVER").ok().filter(|v| !v.is_empty())
}

/// The connection: commands go to a writer thread, events come from a reader thread. When the
/// stream dies, the reader's last event is a lost [`SessionEvent::Disconnected`] naming the server.
pub(super) struct RemoteLink {
    commands: Sender<RealmCommand>,
    events: Receiver<SessionEvent>,
}

impl RemoteLink {
    /// Dials `address`; an unreachable server is reported as the first event, never a panic.
    pub(super) fn connect(address: &str) -> Self {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<RealmCommand>();
        let (event_tx, events) = crossbeam_channel::unbounded();
        let lost = {
            let address = address.to_string();
            move |why: String| SessionEvent::Disconnected {
                reason: format!("the server {address} {why}"),
                end: SessionEnd::Lost,
            }
        };
        match TcpStream::connect(address) {
            Err(e) => {
                let _ = event_tx.send(lost(format!("cannot be reached: {e}")));
            }
            Ok(stream) => {
                bevy::log::info!("twilight: joined the server {address}");
                let _ = stream.set_nodelay(true);
                if let Ok(write_half) = stream.try_clone() {
                    std::thread::spawn(move || write_commands(write_half, cmd_rx));
                }
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream);
                    let why = loop {
                        match read_frame::<_, SessionEvent>(&mut reader) {
                            Ok(event) => {
                                if event_tx.send(event).is_err() {
                                    return;
                                }
                            }
                            Err(e) => break format!("closed the connection: {e}"),
                        }
                    };
                    let _ = event_tx.send(lost(why));
                });
            }
        }
        Self {
            commands: cmd_tx,
            events,
        }
    }

    pub(super) fn send(&self, command: RealmCommand) {
        let _ = self.commands.send(command);
    }

    /// The events that arrived since the last call.
    pub(super) fn drain(&self) -> impl Iterator<Item = SessionEvent> + '_ {
        self.events.try_iter()
    }
}

/// Writes commands as they come, flushing once the queue runs dry.
fn write_commands(stream: TcpStream, commands: Receiver<RealmCommand>) {
    let mut w = BufWriter::new(stream);
    while let Ok(command) = commands.recv() {
        let mut ok = write_frame(&mut w, &command).is_ok();
        for command in commands.try_iter() {
            ok = ok && write_frame(&mut w, &command).is_ok();
        }
        if !ok || w.flush().is_err() {
            return;
        }
    }
}
