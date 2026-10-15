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
mod dev;
mod equipment;
mod fields;
mod inventory;
mod items;
mod looting;
mod mobs;
mod population;
mod rng;
pub mod rules;
mod save;
mod tables;
mod world;

use benilla_protocol::{CharCreateReq, SessionEvent};

pub use content::{CROSSROADS, KALIMDOR, THE_BARRENS};
pub use dev::DevMultipliers;
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
    /// An item template, answered with [`SessionEvent::ItemTemplate`].
    ItemQuery {
        entry: u32,
    },
    /// Open a corpse's loot window.
    Loot {
        guid: u64,
    },
    /// Take the open loot's money.
    LootMoney,
    /// Take the open loot's row `slot` into the bags.
    AutostoreLootItem {
        slot: u8,
    },
    /// Close the loot window.
    LootRelease {
        guid: u64,
    },
    /// Wear the item at a wire position (bag 255 is the player's own slots).
    AutoEquipItem {
        bag: u8,
        slot: u8,
    },
    /// Move an item between two of the player's own slots, swapping with what is there.
    SwapInvItem {
        src: u8,
        dst: u8,
    },
    /// Move an item between two wire positions.
    SwapItem {
        src_bag: u8,
        src_slot: u8,
        dst_bag: u8,
        dst_slot: u8,
    },
    /// Put the item at a wire position into the first free slot of `dst_bag`.
    AutoStoreBagItem {
        src_bag: u8,
        src_slot: u8,
        dst_bag: u8,
    },
    /// Destroy `count` of the item at a wire position, 0 for all of it.
    DestroyItem {
        bag: u8,
        slot: u8,
        count: u8,
    },
}

/// The realm: its characters, the one in the world, and the events not yet drained.
#[derive(Default)]
pub struct Realm {
    characters: characters::Characters,
    world: Option<world::InWorld>,
    events: Vec<SessionEvent>,
    storage: Vec<Storage>,
    rng: rng::Rng,
    dev: DevMultipliers,
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

    /// This realm with the dev multipliers set: the host's boot flags, for the whole run.
    pub fn with_dev(self, dev: DevMultipliers) -> Self {
        Self { dev, ..self }
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
            RealmCommand::ItemQuery { entry } => self.events.push(items::query_response(entry)),
            other => self.handle_items(other),
        }
    }

    /// The loot and inventory commands: they need the character in the world.
    fn handle_items(&mut self, command: RealmCommand) {
        let Some(w) = &mut self.world else {
            return;
        };
        let Some(c) = self.characters.get_mut(w.guid) else {
            return;
        };
        let events = &mut self.events;
        let mut moved = equipment::Moved::default();
        let mut looted = looting::Looted::default();
        match command {
            RealmCommand::Loot { guid } => w.open_loot(guid, c, events),
            RealmCommand::LootMoney => looted = w.looting.take_money(c, events),
            RealmCommand::AutostoreLootItem { slot } => {
                looted = w.looting.take_item(c, slot, events);
            }
            RealmCommand::LootRelease { .. } => looted = w.looting.release(events),
            RealmCommand::AutoEquipItem { bag, slot } => {
                moved = equipment::auto_equip(c, bag, slot, events);
            }
            RealmCommand::SwapInvItem { src, dst } => moved = equipment::swap(c, src, dst, events),
            RealmCommand::SwapItem {
                src_bag,
                src_slot,
                dst_bag,
                dst_slot,
            } => {
                moved =
                    equipment::swap_positions(c, (src_bag, src_slot), (dst_bag, dst_slot), events);
            }
            RealmCommand::AutoStoreBagItem {
                src_bag,
                src_slot,
                dst_bag,
            } => moved = equipment::auto_store(c, src_bag, src_slot, dst_bag, events),
            RealmCommand::DestroyItem { bag, slot, count } => {
                moved = equipment::destroy(c, bag, slot, count, events);
            }
            _ => return,
        }
        if moved.gear_changed {
            w.refresh_gear(c, events);
        }
        if let Some((corpse, lootable)) = looted.flag {
            events.push(fields::values(corpse, &[looting::lootable_pair(lootable)]));
        }
        if moved.gear_changed || looted.took {
            let guid = w.guid;
            self.save(guid);
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
        let entry = world::InWorld::enter(c, self.dev, &mut self.events);
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
