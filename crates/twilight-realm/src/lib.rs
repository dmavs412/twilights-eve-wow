//! **The Local Realm** (ADR 0001): Twilight's Eve's simulated server. It owns every character and
//! every rule, takes [`RealmCommand`]s, is ticked by its host, and answers with the protocol's
//! [`SessionEvent`]s, the same values a real server's stream decodes to. It knows nothing of Bevy
//! or the client: ground height and line of sight come through [`Host`].
//!
//! ```text
//! handle(RealmCommand) ─┐
//! tick(dt, &dyn Host) ──┼─> Realm ──> drain_events() -> Vec<SessionEvent>
//! ```

mod characters;
mod combat;
mod content;
mod fields;
mod items;
mod mobs;
mod rng;
pub mod rules;
mod save;
mod tables;
mod world;

use benilla_protocol::{CharCreateReq, SessionEvent};

pub use content::{CROSSROADS, KALIMDOR, THE_BARRENS};
pub use save::{SaveError, Storage, SAVE_VERSION};

/// What the realm asks of whoever runs it. Coordinates are WoW's (`x` north, `y` west, `z` up).
pub trait Host {
    /// The ground's height under `(x, y)` on `map`, nearest below `z_hint`; `None` where the host
    /// has no ground loaded.
    fn ground_height(&self, map: u32, x: f32, y: f32, z_hint: f32) -> Option<f32>;
    /// Whether nothing solid lies between `from` and `to` on `map`.
    fn line_of_sight(&self, map: u32, from: [f32; 3], to: [f32; 3]) -> bool;
}

/// Everything a client can ask of the realm. The glue maps the client's own commands onto these
/// and drops the rest.
#[derive(Debug, Clone)]
pub enum RealmCommand {
    /// The roster, answered with [`SessionEvent::CharacterList`].
    ListCharacters,
    CreateCharacter(CharCreateReq),
    DeleteCharacter {
        guid: u64,
    },
    /// Log in as this character.
    EnterWorld {
        guid: u64,
    },
    /// The player's own movement report, WoW coordinates.
    Move {
        position: [f32; 3],
        orientation: f32,
    },
    /// The player's target; 0 clears it.
    SetSelection {
        guid: u64,
    },
    /// Start auto-attack on this unit.
    AttackSwing {
        guid: u64,
    },
    /// Stop auto-attack.
    AttackStop,
    /// A creature template's name, answered with [`SessionEvent::CreatureName`].
    CreatureQuery {
        entry: u32,
    },
    /// Back to character select.
    Logout,
}

/// The realm: its characters, the one in the world, and the events not yet drained.
#[derive(Default)]
pub struct Realm {
    characters: characters::Characters,
    world: Option<world::InWorld>,
    events: Vec<SessionEvent>,
    storage: Vec<Storage>,
    rng: rng::Rng,
}

impl Realm {
    /// A realm whose dice start from a fixed seed.
    pub fn new() -> Self {
        Self::default()
    }

    /// A realm whose dice start from `seed`: the same seed and commands replay the same fights.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rng: rng::Rng::new(seed),
            ..Self::default()
        }
    }

    /// Applies one command; its answers queue for [`Self::drain_events`].
    pub fn handle(&mut self, command: RealmCommand) {
        match command {
            RealmCommand::ListCharacters => self.send_roster(),
            RealmCommand::CreateCharacter(req) => {
                let code = self.characters.create(&req);
                // A success changed the roster: the fresh list goes first, as the IO thread sends it.
                if code == benilla_protocol::messages::CHAR_CREATE_SUCCESS {
                    if let Some(guid) = self.characters.newest().map(|c| c.guid) {
                        self.save(guid);
                    }
                    self.send_roster();
                }
                self.events.push(SessionEvent::CharActionResult {
                    action: benilla_protocol::CharAction::Create,
                    code,
                });
            }
            RealmCommand::DeleteCharacter { guid } => {
                let code = self.characters.delete(guid);
                if code == benilla_protocol::messages::CHAR_DELETE_SUCCESS {
                    self.storage.push(Storage::Remove { guid });
                    self.send_roster();
                }
                self.events.push(SessionEvent::CharActionResult {
                    action: benilla_protocol::CharAction::Delete,
                    code,
                });
            }
            RealmCommand::EnterWorld { guid } => self.enter_world(guid),
            RealmCommand::Move {
                position,
                orientation,
            } => {
                if let Some(w) = &self.world {
                    self.characters.set_position(w.guid, position, orientation);
                }
            }
            RealmCommand::SetSelection { guid } => {
                if let Some(w) = &mut self.world {
                    w.select(guid, &mut self.events);
                }
            }
            RealmCommand::AttackSwing { guid } => {
                if let Some(w) = &mut self.world {
                    w.attack(guid, &mut self.events);
                }
            }
            RealmCommand::AttackStop => {
                if let Some(w) = &mut self.world {
                    w.stop_attack(&mut self.events);
                }
            }
            RealmCommand::CreatureQuery { entry } => {
                self.events.push(mobs::creature_name(entry));
            }
            RealmCommand::Logout => self.logout(),
        }
    }

    /// Advances the world by `dt` seconds.
    pub fn tick(&mut self, dt: f32, host: &dyn Host) {
        let Some(w) = &mut self.world else {
            return;
        };
        let Some(c) = self.characters.get_mut(w.guid) else {
            return;
        };
        let ticked = w.tick(dt, host, c, &mut self.rng, &mut self.events);
        w.since_save += dt;
        if ticked.leveled || w.since_save >= save::AUTOSAVE_SECS {
            w.since_save = 0.0;
            let guid = w.guid;
            self.save(guid);
        }
    }

    /// The events since the last drain, oldest first.
    pub fn drain_events(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
    }

    /// The saves to write or remove since the last drain, oldest first.
    pub fn drain_storage(&mut self) -> Vec<Storage> {
        std::mem::take(&mut self.storage)
    }

    /// A character's save, as [`Storage::Write`] carries it.
    pub fn export_character(&self, guid: u64) -> Option<String> {
        self.characters.get(guid).map(save::export)
    }

    /// Adds a character from its save, keeping its guid.
    pub fn import_character(&mut self, blob: &str) -> Result<u64, SaveError> {
        self.characters.insert(save::import(blob)?)
    }

    /// Keeps `guid` from ever being handed out: the host holds a save under it that it could not
    /// import, and a new character must not overwrite it.
    pub fn reserve_guid(&mut self, guid: u64) {
        self.characters.reserve(guid);
    }

    fn save(&mut self, guid: u64) {
        if let Some(blob) = self.export_character(guid) {
            self.storage.push(Storage::Write { guid, blob });
        }
    }

    fn send_roster(&mut self) {
        self.events.push(SessionEvent::CharacterList {
            characters: self.characters.roster(),
            realm: None,
        });
    }

    fn enter_world(&mut self, guid: u64) {
        let Some(c) = self.characters.get(guid) else {
            // `CHAR_LOGIN_NO_CHARACTER`, the reference's fifth refusal.
            self.events
                .push(SessionEvent::CharacterLoginFailed { result: 5 });
            return;
        };
        let entry = world::InWorld::enter(c, &mut self.events);
        self.world = Some(entry);
    }

    fn logout(&mut self) {
        let Some(w) = self.world.take() else {
            return;
        };
        self.save(w.guid);
        self.events.push(SessionEvent::LogoutResponse {
            reason: 0,
            instant: true,
        });
        self.events.push(SessionEvent::LoggedOut);
        self.send_roster();
    }
}
