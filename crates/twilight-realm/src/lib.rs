//! **The Local Realm** (ADR 0001): Twilight's Eve's simulated server. It owns every character and
//! every rule, takes [`RealmCommand`]s, is ticked by its host, and answers with the protocol's
//! [`SessionEvent`]s, the same values a real server's stream decodes to. It knows nothing of Bevy
//! or the client: ground height, line of sight and the area under a point come through [`Host`].
//!
//! ```text
//! handle(RealmCommand) ─┐
//! tick(dt, &dyn Host) ──┼─> Realm ──> drain_events() -> Vec<SessionEvent>
//! ```
//!
//! ## Where things go
//!
//! - **Rules** (`src/rules/`): pure functions of vanilla's formulas (melee table, rage, XP, loot,
//!   durability, talents…), public as [`rules`] so their exact values can be tested.
//! - **Content** (`content/*.toml`): every mob, item, quest, NPC and Calling is data, compiled in.
//!   Each file generated from `data/twilights-eve/*.json` has a generator in `examples/` and a
//!   content test in `tests/` that fails when the two drift.
//! - **Systems** (`src/*.rs`, `src/world/`): state and the event flow. Extend the existing module
//!   for a concern: Hub NPCs (`hub`), merchants (`vendor`), casts (`world/cast`), bags and slots
//!   (`inventory`), the save (`save`, where new fields are optional within [`SAVE_VERSION`]).
//! - **Tests** (`tests/`), two seams (ADR 0002): `rules_*.rs` pin exact values on [`rules`];
//!   every other file is a player script through commands, `tick` and events only. Shared
//!   helpers live in `tests/common/` (`fight` for combat scripts), `gear_common/` and
//!   `quest_common/`.
//!
//! ## A Calling, end to end
//!
//! - **Data**: `content/callings.toml` holds each Calling's kit, Talent Tree, Tier Bonus and armour;
//!   `content/quests.toml` its Advancement Quests. The [`callings`] module loads both.
//! - **Reference**: `data/twilights-eve/callings.json` is the design data and
//!   `tests/content_callings.rs` fails when the two drift. `tools/twilights-eve/extract_spells.py
//!   verify` checks every spell, talent and tab id against the DBCs (the `spell-data` gate).
//! - **Client**: `benilla-app/src/twilight/mod.rs`'s `rewrite_spells` rewrites the spell catalog
//!   once it loads, from [`callings::skill_spell_ids`], [`callings::wip_rank_spell_ids`] and
//!   [`callings::borrowed_spell_rewrites`] (ADR 0003); `twilight/talents.rs` builds the talent
//!   frame's catalog from the current Calling's tree.
//! - **Identity**: the `Calling` enum (`content.rs`) is the save's key; a renamed Calling keeps its
//!   old key as a serde alias. A skill two Callings share has identical data in both.

mod areas;
pub mod callings;
mod characters;
mod clock;
mod combat;
mod content;
mod dev;
mod dungeon;
mod equipment;
mod fields;
mod hub;
mod inventory;
mod items;
mod looting;
mod mobs;
mod population;
mod quests;
mod rng;
pub mod rules;
mod save;
mod space;
mod tables;
mod talents;
mod token_vendor;
mod trainer;
mod vendor;
mod wear;
mod world;

use benilla_protocol::{CharCreateReq, RealmInfo, SessionEvent};

/// The realm's name, as the select screen's banner shows it.
pub const REALM_NAME: &str = "Twilight's Eve";

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
    /// The `AreaTable.dbc` id of the subzone (or zone) under `(x, y)` on `map`, the one a
    /// character standing there discovers; `None` where the host knows of none.
    fn area_at(&self, map: u32, x: f32, y: f32) -> Option<u32>;
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
    /// Cast a skill; `target` is its unit target, `None` for a self or area skill.
    CastSpell {
        spell_id: u32,
        target: Option<u64>,
    },
    /// Click off one of the player's own buffs.
    CancelAura {
        spell_id: u32,
    },
    /// Put `packed` (`kind << 24 | action`) on action bar `slot`; 0 clears it. No answer.
    SetActionButton {
        slot: u8,
        packed: u32,
    },
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
    /// Use the item at a wire position (bag 255 is the player's own slots): the Hearthstone.
    UseItem {
        bag: u8,
        slot: u8,
    },
    /// The client snapped to the teleport carrying `counter`.
    TeleportAck {
        counter: u32,
    },
    /// Destroy `count` of the item at a wire position, 0 for all of it.
    DestroyItem {
        bag: u8,
        slot: u8,
        count: u8,
    },
    /// Spend Talent Points on a talent of the current Calling's tree, up to the 0-based `rank`.
    LearnTalent {
        talent_id: u32,
        rank: u32,
    },
    /// Talk to an NPC: its gossip menu.
    GossipHello {
        guid: u64,
    },
    /// Pick line `option` of an NPC's gossip menu.
    GossipSelectOption {
        guid: u64,
        option: u32,
    },
    /// A gossip greeting's text, answered with [`SessionEvent::NpcGreeting`].
    NpcTextQuery {
        text_id: u32,
    },
    /// Accept the respec the trainer priced: unlearn every talent for gold.
    TalentWipeConfirm {
        trainer: u64,
    },
    /// Open a vendor's merchant window, answered with [`SessionEvent::VendorInventory`].
    ListInventory {
        guid: u64,
    },
    /// Buy `count` of a vendor row's bundles (`buy_count` items each) by item entry.
    BuyItem {
        vendor: u64,
        entry: u32,
        count: u8,
    },
    /// Sell `count` of an item, 0 for the whole stack.
    SellItem {
        vendor: u64,
        item: u64,
        count: u8,
    },
    /// Buy back what was sold, by its player buyback slot (69-80).
    BuybackItem {
        vendor: u64,
        slot: u32,
    },
    /// Repair one item, or with `item` 0 everything carried.
    RepairItem {
        vendor: u64,
        item: u64,
    },
    /// Release Spirit: the dead player comes back at the respawn point.
    RepopRequest,
    /// The `!` or `?` over a quest giver, answered with [`SessionEvent::QuestGiverStatus`].
    QuestGiverStatusQuery {
        npc: u64,
    },
    /// Talk to a quest giver: its quest list, or its one quest's panel.
    QuestGiverHello {
        npc: u64,
    },
    /// An offered quest's accept panel.
    QuestGiverQueryQuest {
        npc: u64,
        quest: u32,
    },
    /// Take an offered quest into the log.
    QuestGiverAcceptQuest {
        npc: u64,
        quest: u32,
    },
    /// Start handing a quest in: its progress panel, or its reward panel.
    QuestGiverCompleteQuest {
        npc: u64,
        quest: u32,
    },
    /// The progress panel's Continue: the reward panel.
    QuestGiverRequestReward {
        npc: u64,
        quest: u32,
    },
    /// Hand a finished quest in, taking reward `choice`.
    QuestGiverChooseReward {
        npc: u64,
        quest: u32,
        choice: u32,
    },
    /// A quest template, answered with [`SessionEvent::QuestTemplate`].
    QuestQuery {
        quest: u32,
    },
    /// Abandon the quest in a log slot.
    QuestLogRemoveQuest {
        slot: u8,
    },
    /// The client walked into `AreaTrigger.dbc` row `id`: the Dungeon's entrance or exit.
    AreaTrigger {
        id: u32,
    },
    /// The client has loaded the map a worldport sent it to.
    WorldportAck,
    /// A dev aid (`WOW_PROBE_GOTO`): on the next tick, teleport the player to `(x, y)` on its
    /// map, on the ground there, facing `facing` or its own way. A spot with no ground is refused
    /// with a notification.
    DevTeleport {
        x: f32,
        y: f32,
        facing: Option<f32>,
    },
}

/// The realm: its characters, the one in the world, and the events not yet drained.
#[derive(Default)]
pub struct Realm {
    characters: characters::Characters,
    world: Option<world::InWorld>,
    events: Vec<SessionEvent>,
    storage: Vec<Storage>,
    /// Lines for the host's log since the last drain.
    log: Vec<String>,
    rng: rng::Rng,
    dev: DevMultipliers,
    /// The dev teleport asked for, waiting for the tick that can ask the host for ground.
    dev_teleport: Option<dev::DevTeleport>,
    /// The wall clock, in seconds since the Unix epoch: the host's, advanced by every tick.
    wall: f64,
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

    /// Sets the wall clock to `unix_ms`, milliseconds since the Unix epoch. Ticks advance it in
    /// between; a host without one leaves it running from 0. An item's cooldown runs on it, so
    /// the time between two sessions counts.
    pub fn set_wall_clock(&mut self, unix_ms: u64) {
        self.wall = unix_ms as f64 / 1000.0;
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
                if let Some(w) = &mut self.world {
                    let from = self.characters.get(w.guid).map(|c| c.position);
                    if from.is_none_or(|from| w.moved(from, position, &mut self.events)) {
                        self.characters.set_position(w.guid, position, orientation);
                    }
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
            RealmCommand::CastSpell { spell_id, target } => self.cast(spell_id, target),
            RealmCommand::UseItem { bag, slot } => {
                if let Some(w) = &mut self.world {
                    if let Some(c) = self.characters.get_mut(w.guid) {
                        let slot = inventory::Pos { bag, slot };
                        // Food and drink are eaten at once; anything else casts its spell.
                        match w.eat_or_drink(slot, c, &mut self.events) {
                            Some(true) => {
                                let guid = w.guid;
                                self.save(guid);
                            }
                            Some(false) => {}
                            None => w.use_item(slot, c, self.wall, &mut self.events),
                        }
                    }
                }
            }
            RealmCommand::TeleportAck { counter } => {
                if let Some(w) = &mut self.world {
                    w.teleport_ack(counter);
                }
            }
            RealmCommand::CancelAura { spell_id } => {
                if let Some(w) = &mut self.world {
                    w.cancel_aura(spell_id, &mut self.events);
                }
            }
            RealmCommand::SetActionButton { slot, packed } => {
                if let Some(w) = &self.world {
                    self.characters.set_action_button(w.guid, slot, packed);
                }
            }
            RealmCommand::CreatureQuery { entry } => {
                let answer =
                    hub::creature_name(entry).unwrap_or_else(|| mobs::creature_name(entry));
                self.events.push(answer);
            }
            RealmCommand::Logout => {
                self.dev_teleport = None;
                self.logout();
            }
            RealmCommand::DevTeleport { x, y, facing } => {
                if self.world.is_some() {
                    self.dev_teleport = Some(dev::DevTeleport { x, y, facing });
                }
            }
            RealmCommand::AreaTrigger { id } => self.area_trigger(id),
            RealmCommand::WorldportAck => {
                if let Some(w) = &mut self.world {
                    if let Some(c) = self.characters.get(w.guid) {
                        w.worldport_ack(c, &mut self.events);
                    }
                }
            }
            RealmCommand::ItemQuery { entry } => self.events.push(items::query_response(entry)),
            RealmCommand::LearnTalent { talent_id, rank } => self.learn_talent(talent_id, rank),
            RealmCommand::GossipHello { guid } => self.gossip_hello(guid),
            RealmCommand::GossipSelectOption { guid, option } => self.gossip_select(guid, option),
            RealmCommand::NpcTextQuery { text_id } => {
                self.events.extend(trainer::greeting(text_id));
            }
            RealmCommand::TalentWipeConfirm { trainer } => self.talent_wipe(trainer),
            RealmCommand::QuestQuery { quest } => {
                self.events.extend(quests::template_response(quest));
            }
            other if other.is_quest() => self.handle_quest(other),
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
        let mut traded = vendor::Traded::default();
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
            RealmCommand::ListInventory { guid } => vendor::list(guid, c, events),
            RealmCommand::BuyItem {
                vendor,
                entry,
                count,
            } => traded = vendor::buy(vendor, entry, count, c, events),
            RealmCommand::SellItem {
                vendor,
                item,
                count,
            } => traded = vendor::sell(vendor, item, count, c, &mut w.buyback, events),
            RealmCommand::BuybackItem { vendor, slot } => {
                traded = vendor::buy_back(vendor, slot, c, &mut w.buyback, events);
            }
            RealmCommand::RepairItem { vendor, item } => {
                traded = vendor::repair(vendor, item, c, events);
            }
            RealmCommand::RepopRequest => traded.changed = w.release(c, events),
            _ => return,
        }
        moved.gear_changed |= traded.gear_changed;
        looted.took |= traded.changed;
        if moved.gear_changed {
            w.refresh_gear(c, events);
        }
        if let Some((corpse, lootable)) = looted.flag {
            events.push(fields::values(corpse, &[looting::lootable_pair(lootable)]));
        }
        // A quest may want what came into the bags, or what left them.
        quests::refresh(c, events);
        if moved.gear_changed || moved.bags_changed || looted.took {
            let guid = w.guid;
            self.save(guid);
        }
    }

    /// Advances the world by `dt` seconds.
    pub fn tick(&mut self, dt: f32, host: &dyn Host) {
        self.wall += f64::from(dt);
        let Some(w) = &mut self.world else {
            return;
        };
        let Some(c) = self.characters.get_mut(w.guid) else {
            return;
        };
        let map = c.map;
        if let Some(to) = self.dev_teleport.take() {
            w.dev_teleport(to, host, c, &mut self.events);
        }
        let ticked = w.tick(dt, self.wall, host, c, &mut self.rng, &mut self.events);
        w.since_save += dt;
        // Zoning saves (a hearth out of the Dungeon); the dead are saved once they release.
        let zoned = c.map != map;
        if ticked.leveled
            || ticked.discovered
            || zoned
            || (w.since_save >= save::AUTOSAVE_SECS && !w.dead())
        {
            w.since_save = 0.0;
            let guid = w.guid;
            self.save(guid);
        }
    }

    /// A skill cast: answered at once, as an instant is.
    fn cast(&mut self, spell_id: u32, target: Option<u64>) {
        let Some(w) = &mut self.world else {
            return;
        };
        let Some(c) = self.characters.get_mut(w.guid) else {
            return;
        };
        let cast = w.cast(spell_id, target, c, &mut self.rng, &mut self.events);
        if cast.leveled {
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

    /// What the realm has to report since the last drain, for the host to log as warnings: what
    /// a save named that the realm no longer has.
    pub fn drain_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.log)
    }

    /// A character's save, as [`Storage::Write`] carries it.
    pub fn export_character(&self, guid: u64) -> Option<String> {
        self.characters.get(guid).map(save::export)
    }

    /// Adds a character from its save, keeping its guid.
    pub fn import_character(&mut self, blob: &str) -> Result<u64, SaveError> {
        let (c, notes) = save::import(blob)?;
        let guid = self.characters.insert(c)?;
        self.log.extend(notes);
        Ok(guid)
    }

    /// The name of a character's Calling, as the select screen shows it (`Basic Warrior`).
    pub fn calling_name(&self, guid: u64) -> Option<&'static str> {
        self.characters
            .get(guid)
            .map(|c| callings::of(c.calling).name.as_str())
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

    /// The roster, naming the realm: the select screen shows its banner and the Create New
    /// Character button only for a list that names one.
    fn send_roster(&mut self) {
        let characters = self.characters.roster();
        let realm = RealmInfo {
            name: REALM_NAME.into(),
            address: String::new(),
            population: 0.0,
            characters: u8::try_from(characters.len()).unwrap_or(u8::MAX),
            // A normal realm: the banner carries no PvP or RP suffix.
            realm_type: 0,
            flags: 0,
            category: 0,
            id: 1,
        };
        self.events.push(SessionEvent::CharacterList {
            characters,
            realm: Some(realm),
        });
    }

    /// `CMSG_AREATRIGGER`: a trigger that moved the player saves where they went.
    fn area_trigger(&mut self, id: u32) {
        let Some(w) = &mut self.world else {
            return;
        };
        let Some(c) = self.characters.get_mut(w.guid) else {
            return;
        };
        if w.area_trigger(id, c, &mut self.events) {
            let guid = w.guid;
            self.save(guid);
        }
    }

    fn enter_world(&mut self, guid: u64) {
        // A save from inside the Dungeon comes back outside it: a run ends with its session.
        if let Some(c) = self.characters.get_mut(guid) {
            world::come_out(c);
        }
        let Some(c) = self.characters.get(guid) else {
            // `CHAR_LOGIN_NO_CHARACTER`, the reference's fifth refusal.
            self.events
                .push(SessionEvent::CharacterLoginFailed { result: 5 });
            return;
        };
        let entry = world::InWorld::enter(c, self.dev, self.wall, &mut self.events);
        self.world = Some(entry);
    }

    fn logout(&mut self) {
        let Some(mut w) = self.world.take() else {
            return;
        };
        // Leaving dead is a release: the save puts the character where it comes back.
        if let Some(c) = self.characters.get_mut(w.guid) {
            w.release(c, &mut Vec::new());
            // Leaving inside the Dungeon leaves it.
            world::come_out(c);
        }
        self.save(w.guid);
        self.events.push(SessionEvent::LogoutResponse {
            reason: 0,
            instant: true,
        });
        self.events.push(SessionEvent::LoggedOut);
        self.send_roster();
    }
}
