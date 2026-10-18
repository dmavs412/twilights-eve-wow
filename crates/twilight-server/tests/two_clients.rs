//! Two clients join one server over TCP: each creates a character, enters the world, and is sent
//! the other once they stand together.

use std::io::BufReader;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use benilla_protocol::{CharCreateReq, EntityKind, SessionEvent};
use twilight_realm::wire::{read_frame, write_frame};
use twilight_realm::{RealmCommand, CROSSROADS};

const HUMAN: u8 = 1;
const WARRIOR: u8 = 1;
/// Far out past The Barrens' camps: nothing else streams in.
const FAR_AWAY: [f32; 3] = [CROSSROADS[0] + 3000.0, CROSSROADS[1], 0.0];

struct Client {
    stream: TcpStream,
    events: Receiver<SessionEvent>,
}

impl Client {
    fn connect(addr: std::net::SocketAddr) -> Self {
        let stream = TcpStream::connect(addr).expect("the server answers");
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let (tx, events) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(event) = read_frame::<_, SessionEvent>(&mut reader) {
                if tx.send(event).is_err() {
                    return;
                }
            }
        });
        Self { stream, events }
    }

    fn send(&mut self, command: RealmCommand) {
        write_frame(&mut self.stream, &command).unwrap();
    }

    /// The first event `pick` accepts, within a few seconds.
    fn wait_for<T>(&self, what: &str, mut pick: impl FnMut(&SessionEvent) -> Option<T>) -> T {
        let deadline = Instant::now() + Duration::from_secs(10);
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match self.events.recv_timeout(left) {
                Ok(event) => {
                    if let Some(t) = pick(&event) {
                        return t;
                    }
                }
                Err(_) => break,
            }
        }
        panic!("no {what} from the server");
    }

    /// Creates `name` and enters the world as it; returns its guid.
    fn create_and_enter(&mut self, name: &str) -> u64 {
        self.send(RealmCommand::CreateCharacter(CharCreateReq {
            name: name.into(),
            race: HUMAN,
            class: WARRIOR,
            gender: 0,
            skin: 0,
            face: 0,
            hair_style: 0,
            hair_color: 0,
            facial_hair: 0,
            start: 0,
        }));
        self.send(RealmCommand::ListCharacters);
        let guid = self.wait_for("roster", |e| match e {
            SessionEvent::CharacterList { characters, .. } => {
                characters.iter().find(|c| c.name == name).map(|c| c.guid)
            }
            _ => None,
        });
        self.send(RealmCommand::EnterWorld { guid });
        guid
    }
}

#[test]
fn two_clients_see_each_other_over_the_wire() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let data_dir =
        std::env::temp_dir().join(format!("twilight-server-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data_dir);
    let dir = data_dir.clone();
    std::thread::spawn(move || {
        twilight_server::serve(listener, dir, Box::new(twilight_server::FlatHost))
    });

    let mut a = Client::connect(addr);
    let mut b = Client::connect(addr);
    let guid_a = a.create_and_enter("Aldren");
    let guid_b = b.create_and_enter("Brena");
    a.send(RealmCommand::Move {
        position: FAR_AWAY,
        orientation: 0.0,
    });
    b.send(RealmCommand::Move {
        position: [FAR_AWAY[0] + 3.0, FAR_AWAY[1], FAR_AWAY[2]],
        orientation: 0.0,
    });

    let sees = |guid: u64| {
        move |e: &SessionEvent| {
            matches!(e, SessionEvent::ObjectCreate { guid: g, kind: EntityKind::Player, .. } if *g == guid)
                .then_some(())
        }
    };
    a.wait_for("create of the other player", sees(guid_b));
    b.wait_for("create of the other player", sees(guid_a));

    // Both characters were saved under the data dir when they were created.
    let saves = std::fs::read_dir(data_dir.join("characters")).map_or(0, |d| d.count());
    assert_eq!(saves, 2);
    let _ = std::fs::remove_dir_all(&data_dir);
}
