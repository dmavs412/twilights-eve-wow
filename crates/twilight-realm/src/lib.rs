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
//! - **Client**: `benilla-app/src/twilight/mod.rs`'s `rewrite_spells` applies the compiled-in
//!   `client_dbc_patch.json` once the spell catalog loads (ADR 0011), then marks the talents of
//!   [`callings::wip_rank_spell_ids`]; `twilight/talents.rs` builds the talent
//!   frame's catalog from the current Calling's tree.
//! - **Identity**: the `Calling` enum (`content.rs`) is the save's key; a renamed Calling keeps its
//!   old key as a serde alias. A skill two Callings share has identical data in both.

mod ammo;
mod areas;
mod audience;
mod bands;
mod bounty;
pub mod callings;
mod characters;
pub mod chat;
mod clock;
mod combat;
mod content;
mod creature_spells;
mod dev;
mod duel;
mod dungeon;
mod encounters;
mod equipment;
mod fields;
mod generated;
mod ground;
mod group;
mod hub;
mod inn;
mod inventory;
mod items;
mod looting;
mod mobs;
mod objects;
mod party;
mod phased;
mod population;
mod quests;
mod rest;
mod rng;
pub mod rules;
mod rumours;
mod save;
mod shard;
mod space;
mod tables;
mod talents;
mod token_vendor;
mod trade;
mod trainer;
mod vendor;
mod wear;
#[cfg(feature = "wire")]
pub mod wire;
mod world;

use benilla_protocol::{CharCreateReq, RealmInfo, SessionEvent};

/// The realm's name, as the select screen's banner shows it.
pub const REALM_NAME: &str = "Twilight's Eve";

pub mod build_id;

pub use audience::Audience;
pub use content::{CROSSROADS, KALIMDOR, THE_BARRENS};
pub use dev::DevMultipliers;
pub use objects::{ObjectKind, ObjectSpec, CHEST_DISPLAY};
pub use phased::{test_grounds_start, GameObjectUse, UseKind};
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

/// A movement report's state beyond its pose: what an observer animates and extrapolates from.
/// The default is a still body, as a test or a scripted pose reports it.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, Default, PartialEq)]
pub struct Motion {
    /// The opcode's verb: heartbeat, teleport and root are told apart by it, not the flags.
    pub verb: benilla_protocol::messages::RelayVerb,
    pub flags: u32,
    /// The mover's own clock (ms); 0 lets the realm stamp its own.
    pub time: u32,
    pub pitch: f32,
    pub fall_time: u32,
    pub jump: Option<benilla_protocol::messages::JumpInfo>,
    pub transport: Option<benilla_protocol::messages::TransportPose>,
}

/// Everything a client can ask of the realm. The glue maps the client's own commands onto these
/// and drops the rest.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
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
        /// The rest of the report, relayed as sent to the others who see the mover.
        motion: Motion,
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
    /// Stop an auto-repeat shot (`CMSG_CANCEL_AUTO_REPEAT_SPELL`).
    CancelAutoRepeat,
    /// Cast a skill; `target` is its unit target, `None` for a self or area skill.
    CastSpell {
        spell_id: u32,
        target: Option<u64>,
    },
    /// Cast a ground-targeted skill at `dest`, WoW coordinates (`TARGET_FLAG_DEST_LOCATION`).
    CastSpellAtDest {
        spell_id: u32,
        dest: [f32; 3],
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
    /// Click a GameObject (`CMSG_GAMEOBJ_USE`). A Phased Dungeon's GameObject with a use queues a
    /// [`GameObjectUse`] for [`Realm::drain_uses`]; any other guid does nothing.
    UseGameObject {
        guid: u64,
    },
    /// A creature template's name, answered with [`SessionEvent::CreatureName`].
    CreatureQuery {
        entry: u32,
    },
    /// Back to character select.
    Logout,
    /// Stand up from a pending logout (`CMSG_LOGOUT_CANCEL`).
    LogoutCancel,
    /// A chat line (`CMSG_MESSAGECHAT`): its `CHAT_TYPE_*`, the whisper's addressee or the
    /// channel, and the text.
    Chat {
        chat_type: u32,
        target: Option<String>,
        text: String,
    },
    /// A text emote (`CMSG_TEXT_EMOTE`): an `EmotesText.dbc` id at `target` (0 for none).
    TextEmote {
        text_id: u32,
        target: u64,
    },
    /// A player's name, answered with [`SessionEvent::PlayerName`].
    NameQuery {
        guid: u64,
    },
    /// An item template, answered with [`SessionEvent::ItemTemplate`].
    ItemQuery {
        entry: u32,
    },
    /// A readable item's pages from `page_id` on, answered with one [`SessionEvent::PageText`] per
    /// page.
    PageTextQuery {
        page_id: u32,
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
    /// `CMSG_SET_AMMO`: shoot the arrows or bullets of this entry, wherever the bags hold them.
    SetAmmo {
        entry: u32,
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
    /// Accept the innkeeper's bind confirm: bind the Hearthstone where the player stands.
    BinderActivate {
        binder: u64,
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
    /// Accept the duel whose flag is `arbiter` (`CMSG_DUEL_ACCEPTED`).
    DuelAccept {
        arbiter: u64,
    },
    /// Decline, call off or yield the duel whose flag is `arbiter` (`CMSG_DUEL_CANCELLED`).
    DuelCancel {
        arbiter: u64,
    },
    /// Ask the player `target` to trade (`CMSG_INITIATE_TRADE`).
    InitiateTrade {
        target: u64,
    },
    /// Accept the trade request made of the player: both windows open.
    BeginTrade,
    /// Turn a trade request down as busy.
    BusyTrade,
    /// Turn a trade request down as ignored.
    IgnoreTrade,
    /// The Trade button.
    AcceptTrade,
    /// Take back the accept, staying in the trade.
    UnacceptTrade,
    /// Close the trade window, or take back a request.
    CancelTrade,
    /// Offer `copper` in the trade.
    SetTradeGold {
        copper: u32,
    },
    /// Offer the item at the wire's (`bag`, `slot`) in trade slot `trade_slot`.
    SetTradeItem {
        trade_slot: u8,
        bag: u8,
        slot: u8,
    },
    /// Take the item out of trade slot `trade_slot`.
    ClearTradeItem {
        trade_slot: u8,
    },
    /// A dev aid (`WOW_PROBE_GOTO`): on the next tick, teleport the player to `(x, y)` on its
    /// map, on the ground there, facing `facing` or its own way. A spot with no ground is refused
    /// with a notification.
    DevTeleport {
        x: f32,
        y: f32,
        facing: Option<f32>,
    },
}

/// One client the realm serves: a handle from [`Realm::connect`]. The realm starts with
/// [`SessionId::DEFAULT`], which the session-less calls ([`Realm::handle`], [`Realm::drain_events`])
/// act on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(u32);

impl SessionId {
    /// The session every realm has from [`Realm::new`].
    pub const DEFAULT: Self = Self(0);
}

/// What one session owns: its character in the world, its undrained events, its dev teleport.
#[derive(Default)]
struct Session {
    world: Option<world::InWorld>,
    events: Vec<SessionEvent>,
    dev_teleport: Option<dev::DevTeleport>,
}

/// The realm: its characters, the sessions and who of theirs is in the world, and the events not
/// yet drained.
#[derive(Default)]
pub struct Realm {
    characters: characters::Characters,
    /// Everyone in the world, as one party.
    party: party::Party,
    /// The trades under way between players.
    trades: trade::Trades,
    /// The duels under way between players.
    duels: duel::Duels,
    /// The open world's Encounters under way.
    encounters: encounters::Encounters,
    /// The served session's character in the world: `active`'s, swapped in by [`Self::serve`].
    world: Option<world::InWorld>,
    /// The served session's events.
    events: Vec<SessionEvent>,
    /// The session `world`, `events` and `dev_teleport` belong to.
    active: SessionId,
    /// Every other session, parked while another is served.
    parked: std::collections::BTreeMap<SessionId, Session>,
    /// The last session id handed out.
    last_session: u32,
    /// The open-world maps someone stands on, each shared by every player there.
    shards: std::collections::BTreeMap<shard::Place, shard::Shard>,
    /// The shard whose mobs the served session holds, lent.
    lent_from: Option<shard::Place>,
    /// How many of the served session's events came before the lend: what follows is shown to
    /// the other players who see the units in it.
    lent_mark: usize,
    storage: Vec<Storage>,
    /// Lines for the host's log since the last drain.
    log: Vec<String>,
    rng: rng::Rng,
    dev: DevMultipliers,
    /// The dev teleport asked for, waiting for the tick that can ask the host for ground.
    dev_teleport: Option<dev::DevTeleport>,
    /// The wall clock, in seconds since the Unix epoch: the host's, advanced by every tick.
    wall: f64,
    /// The Creature Spells a test gave creatures beyond their templates'.
    casters: creature_spells::Casters,
    /// The dungeon GameObject clicks since the last drain.
    uses: Vec<GameObjectUse>,
    /// The Phased Dungeon doors that are open: world state, in no Run.
    doors: phased::Doors,
    /// The GameObjects made at run time for an Audience.
    objects: objects::Objects,
    /// The Rumours under way: not saved.
    rumours: rumours::Rumours,
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

    /// This realm with the Callings in `toml` (the shape of `content/callings.toml`) added
    /// alongside the shipped ones, for the rest of the process: a test seam, so a player script
    /// can import a save on a Calling of a Class no shipped Calling sits on.
    pub fn with_callings(self, toml: &str) -> Self {
        callings::add(toml);
        self
    }

    /// This realm with the quests in `toml` (the shape of `content/quests.toml`) added alongside
    /// the shipped ones, for the rest of the process: a test seam, so a player script can try a
    /// quest the content does not ship.
    pub fn with_quests(self, toml: &str) -> Self {
        quests::content::add(toml);
        self
    }

    /// This realm with the quest items in `toml` (`[[item]]` rows, as `content/quest_items.toml`
    /// writes them) added, for the rest of the process: a test seam beside [`Self::with_quests`].
    pub fn with_items(self, toml: &str) -> Self {
        quests::content::add(toml);
        self
    }

    /// This realm with the Rumours in `toml` (the shape of `content/rumours.toml`) added alongside
    /// the shipped ones, for the rest of the process: a test seam. Add the quests and notes they
    /// name with [`Self::with_quests`] first.
    pub fn with_rumours(self, toml: &str) -> Self {
        rumours::add(toml);
        self
    }

    /// The guids of the Rumour givers standing in the world.
    pub fn rumour_givers(&self) -> Vec<u64> {
        self.rumours.givers()
    }

    /// This realm with creature `entry` casting the Creature Spells `keys`
    /// (`content/creature_spells.toml`) in place of its template's: a test seam, so a player
    /// script can fight a caster where the content has none.
    pub fn with_creature_spells(mut self, entry: u32, keys: &[&str]) -> Self {
        self.casters.add(entry, keys);
        self
    }

    /// This realm with creatures `entries` flagged as bosses (the template's `boss` key), for the
    /// rest of the process: a test seam, so a player script can fight a boss where the content
    /// flags none.
    pub fn with_bosses(self, entries: &[u32]) -> Self {
        mobs::add_bosses(entries);
        self
    }

    /// Sets the wall clock to `unix_ms`, milliseconds since the Unix epoch. Ticks advance it in
    /// between; a host without one leaves it running from 0. An item's cooldown runs on it, so
    /// the time between two sessions counts.
    pub fn set_wall_clock(&mut self, unix_ms: u64) {
        self.wall = unix_ms as f64 / 1000.0;
    }

    /// A new session, with no character in the world.
    pub fn connect(&mut self) -> SessionId {
        self.last_session += 1;
        let id = SessionId(self.last_session);
        self.parked.insert(id, Session::default());
        id
    }

    /// Ends `session`: its character logs out and is saved, as [`RealmCommand::Logout`] does, and
    /// its undrained events are dropped. The default session is emptied, never removed.
    pub fn disconnect(&mut self, session: SessionId) {
        if session != self.active && !self.parked.contains_key(&session) {
            return;
        }
        self.serve(session);
        self.dev_teleport = None;
        self.logout();
        self.events.clear();
        if session != SessionId::DEFAULT {
            self.serve(SessionId::DEFAULT);
            self.parked.remove(&session);
        }
    }

    /// Where every character in the world stands: `(map, position)`, for a host that loads the
    /// world around its players.
    pub fn player_positions(&self) -> Vec<(u32, [f32; 3])> {
        let worlds = self
            .world
            .iter()
            .chain(self.parked.values().filter_map(|s| s.world.as_ref()));
        worlds
            .filter_map(|w| self.characters.get(w.guid))
            .map(|c| (c.map, c.position))
            .collect()
    }

    /// Applies one command from `session`; its answers queue for that session. An unknown session
    /// is ignored.
    pub fn handle_as(&mut self, session: SessionId, command: RealmCommand) {
        if session != self.active && !self.parked.contains_key(&session) {
            return;
        }
        self.serve(session);
        self.handle_served(command);
    }

    /// Every session's events since the last drain, by session then oldest first.
    pub fn drain_session_events(&mut self) -> Vec<(SessionId, SessionEvent)> {
        self.give_back();
        self.lend();
        self.lent_mark = 0;
        let mut out: Vec<_> = std::mem::take(&mut self.events)
            .into_iter()
            .map(|e| (self.active, e))
            .collect();
        for (&id, s) in &mut self.parked {
            out.extend(std::mem::take(&mut s.events).into_iter().map(|e| (id, e)));
        }
        out.sort_by_key(|(id, _)| *id);
        out
    }

    /// Makes `session` the served one: its state moves into `world`, `events` and
    /// `dev_teleport`, and the one served before is parked.
    fn serve(&mut self, session: SessionId) {
        if session != self.active {
            self.give_back();
            let incoming = self.parked.remove(&session).unwrap_or_default();
            let outgoing = Session {
                world: std::mem::replace(&mut self.world, incoming.world),
                events: std::mem::replace(&mut self.events, incoming.events),
                dev_teleport: std::mem::replace(&mut self.dev_teleport, incoming.dev_teleport),
            };
            self.parked.insert(self.active, outgoing);
            self.active = session;
        }
        self.lend();
    }

    /// Lends the served session its shard's mobs, when its player stands in the open world, and
    /// seats its duel foe among them.
    fn lend(&mut self) {
        if let Some(w) = &mut self.world {
            w.doors_open = self.doors.ids();
        }
        self.lend_shard();
        self.seat_foes();
    }

    fn lend_shard(&mut self) {
        if self.lent_from.is_some() {
            return;
        }
        let Some(w) = &mut self.world else {
            return;
        };
        let Some(map) = w.shard_place() else {
            return;
        };
        let shard = self
            .shards
            .entry(map)
            .or_insert_with(|| shard::Shard::new(map));
        w.mobs = std::mem::take(&mut shard.mobs);
        w.lent = true;
        self.lent_from = Some(map);
        self.lent_mark = self.events.len();
    }

    /// Takes the lent mobs back into their shard, with the deaths the served session saw, and
    /// shows the other players there what it did to the units they see.
    fn give_back(&mut self) {
        self.unseat_foes();
        let Some(map) = self.lent_from.take() else {
            // A player who came out into the open world left their own mobs behind.
            if let Some(w) = &mut self.world {
                if w.shard_place().is_some() {
                    w.mobs.clear();
                }
                w.died.clear();
            }
            return;
        };
        let new: Vec<SessionEvent> = self.events[self.lent_mark.min(self.events.len())..].to_vec();
        let shard = self
            .shards
            .entry(map)
            .or_insert_with(|| shard::Shard::new(map));
        if let Some(w) = &mut self.world {
            shard.mobs = if w.lent {
                w.lent = false;
                std::mem::take(&mut w.mobs)
            } else {
                w.handed_back.take().unwrap_or_default()
            };
            for (spawn, wait) in w.died.drain(..) {
                shard.died(spawn, wait);
            }
            // What the session itself created or removed (a call for help's adds).
            if w.shard_place() == Some(map) {
                for e in &new {
                    match e {
                        // A GameObject, or the player's own patch on the ground, is the
                        // session's own to stream, not the shard's view.
                        SessionEvent::ObjectCreate { guid, kind, .. }
                            if *guid != w.guid
                                && !matches!(
                                    kind,
                                    benilla_protocol::EntityKind::GameObject
                                        | benilla_protocol::EntityKind::DynamicObject
                                ) =>
                        {
                            w.seen.insert(*guid);
                        }
                        SessionEvent::ObjectDestroyed(guid) => {
                            w.seen.remove(guid);
                        }
                        SessionEvent::ObjectsRemoved(guids) => {
                            guids.iter().for_each(|g| {
                                w.seen.remove(g);
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
        if let Some(w) = &self.world {
            shard::note_threat(&mut shard.mobs, w.guid, &new);
        }
        self.show_others(map, &new);
        let kills = self
            .world
            .as_mut()
            .map(|w| std::mem::take(&mut w.kills))
            .unwrap_or_default();
        for kill in kills {
            self.share_kill(map, kill);
        }
        self.deliver_helps();
    }

    /// Hands the served session's friendly spells on other players to those players' sessions.
    fn deliver_helps(&mut self) {
        let Some(w) = &mut self.world else {
            return;
        };
        let from = w.guid;
        for help in w.take_helps() {
            let to = help.to();
            if let world::Help::Aura { aura, .. } = &help {
                self.duels.note_worn(from, to, aura.spell_id());
            }
            let target = self
                .parked
                .values_mut()
                .filter_map(|s| s.world.as_mut())
                .find(|w| w.guid == to);
            if let Some(t) = target {
                t.helped.push(help);
            }
        }
    }

    /// Shares `kill` among every player on `map` within the credit range of it, alive or dead,
    /// fought or not (#148): its money split evenly with the remainder the killer's, and the rest
    /// each their own.
    fn share_kill(&mut self, map: shard::Place, kill: world::Kill) {
        // A Marked player's victor fell, at anyone's hand: their mark ends, logged in or not.
        if let Some(b) = kill.bounty.filter(|b| b.victor) {
            if let Some(owner) = self.characters.get_mut(b.owner) {
                owner.marked = None;
                self.save(b.owner);
            }
        }
        let range = group::get().credit_range_yd;
        let killer = self.active;
        let characters = &self.characters;
        let sessions = std::iter::once((self.active, &mut self.world))
            .chain(self.parked.iter_mut().map(|(id, s)| (*id, &mut s.world)));
        let mut near = Vec::new();
        for (id, w) in sessions {
            let Some(w) = w.as_mut() else { continue };
            let Some(c) = characters.get(w.guid) else {
                continue;
            };
            if w.shard_place() == Some(map)
                && space::flat_distance(c.position, kill.position) <= range
            {
                near.push((id, w, c.level));
            }
        }
        let levels: Vec<u8> = near.iter().map(|n| n.2).collect();
        let n = near.len().max(1) as u32;
        // The kill may roll a Rumour for the players it credits (spec #284).
        let credited: Vec<&characters::Char> = near
            .iter()
            .filter_map(|n| characters.get(n.1.guid))
            .collect();
        let killer_guid = near.iter().find(|n| n.0 == killer).map_or(0, |n| n.1.guid);
        let found = self.rumours.roll(
            &credited,
            killer_guid,
            &kill,
            map.map,
            self.wall,
            &mut self.rng,
        );
        let mut own = None;
        for (id, w, _) in near {
            let remainder = if id == killer { kill.money % n } else { 0 };
            let credit = world::Credit {
                kill: kill.clone(),
                money: Some(kill.money / n + remainder),
                levels: levels.clone(),
                note: found
                    .as_ref()
                    .filter(|f| f.looter == w.guid)
                    .map(|f| f.note),
            };
            if id == killer {
                own = Some(credit);
            } else {
                w.credits.push(credit);
            }
        }
        // The rest of the Audience is told where the giver waits.
        if let Some(f) = &found {
            let sessions = std::iter::once((&mut self.world, &mut self.events)).chain(
                self.parked
                    .values_mut()
                    .map(|s| (&mut s.world, &mut s.events)),
            );
            for (w, events) in sessions {
                if w.as_ref()
                    .is_some_and(|w| w.guid != f.looter && f.audience.contains(&w.guid))
                {
                    events.push(quests::system_line(f.line.clone()));
                }
            }
        }
        // The killer's part is paid now, so the corpse sparkles for them as it falls; the others'
        // on their next tick.
        let (Some(credit), Some(w)) = (own, self.world.as_mut()) else {
            return;
        };
        let guid = w.guid;
        let Some(c) = self.characters.get_mut(guid) else {
            return;
        };
        let mut ticked = world::Ticked::default();
        w.take_credit(credit, c, &mut self.rng, &mut self.events, &mut ticked);
        if ticked.leveled {
            self.save(guid);
        }
    }

    /// Shows `events` to every parked session on `map` whose client sees a unit in them.
    fn show_others(&mut self, map: shard::Place, events: &[SessionEvent]) {
        for s in self.parked.values_mut() {
            let Some(w) = &s.world else {
                continue;
            };
            if w.shard_place() != Some(map) {
                continue;
            }
            for e in events {
                if shard::shown_units(e).iter().any(|u| w.seen.contains(u)) {
                    s.events.extend(shard::as_seen_by_others(e));
                }
            }
        }
    }

    /// The shards' own tick: each streams around its players, and every client there is sent
    /// what came into its view and what left it; then each mob is given to the player who runs
    /// it this tick. `docs/realm_view.md` walks through what reaches which client.
    fn tick_shards(&mut self, dt: f32, host: &dyn Host) {
        self.give_back();
        let mut on: std::collections::BTreeMap<shard::Place, Vec<shard::OnShard<SessionId>>> =
            Default::default();
        let worlds = std::iter::once((self.active, &self.world))
            .chain(self.parked.iter().map(|(id, s)| (*id, &s.world)));
        for (id, w) in worlds {
            let Some(w) = w else { continue };
            let (Some(map), Some(c)) = (w.shard_place(), self.characters.get(w.guid)) else {
                continue;
            };
            on.entry(map).or_default().push(shard::OnShard {
                session: id,
                guid: w.guid,
                position: c.position,
                alive: w.alive(),
                health: w.health_share(),
                create: w.remote_create(c),
                calling: callings::of(c.calling).key.clone(),
                hidden: w.hidden(),
                patches: w.patches(),
            });
        }
        self.shards.retain(|map, _| on.contains_key(map));
        for (map, players) in &on {
            let shard = self
                .shards
                .entry(*map)
                .or_insert_with(|| shard::Shard::new(*map));
            let positions: Vec<[f32; 3]> = players.iter().map(|p| p.position).collect();
            let present: Vec<u64> = players.iter().map(|p| p.guid).collect();
            shard.drop_hunts(&present);
            let made = shard.update(dt, &positions, host);
            let mut owners = shard.owners(players);
            let shard = &self.shards[map];
            let sessions = std::iter::once((self.active, &mut self.world, &mut self.events)).chain(
                self.parked
                    .iter_mut()
                    .map(|(id, s)| (*id, &mut s.world, &mut s.events)),
            );
            for (id, w, events) in sessions {
                let Some(p) = players.iter().find(|p| p.session == id) else {
                    continue;
                };
                let Some(w) = w else { continue };
                // What streamed out or came back, and the Hub's wanderers' walks.
                for e in &made {
                    match e {
                        SessionEvent::ObjectDestroyed(g) => {
                            if w.seen.remove(g) {
                                events.push(e.clone());
                            }
                        }
                        SessionEvent::ObjectsRemoved(gs) => {
                            let gone: Vec<u64> =
                                gs.iter().copied().filter(|g| w.seen.remove(g)).collect();
                            if !gone.is_empty() {
                                events.push(SessionEvent::ObjectsRemoved(gone));
                            }
                        }
                        other if shard::shown_units(other).iter().any(|u| w.seen.contains(u)) => {
                            events.push(other.clone());
                        }
                        _ => {}
                    }
                }
                // The view: what came within it is created, what left it removed.
                let mut view = shard.in_view(p.guid, p.position); // The other players in view.
                view.extend(
                    players
                        .iter()
                        .filter(|o| o.guid != p.guid && !o.hidden)
                        .filter(|o| {
                            space::flat_distance(o.position, p.position) <= population::VIEW_YD
                        })
                        .map(|o| (o.guid, o.create.clone())),
                );
                // The other players' patches on the ground; a player's own are their session's.
                view.extend(
                    players
                        .iter()
                        .filter(|o| o.guid != p.guid)
                        .flat_map(|o| &o.patches)
                        .filter(|(_, centre, _)| {
                            space::flat_distance(*centre, p.position) <= population::VIEW_YD
                        })
                        .map(|(guid, _, create)| (*guid, create.clone())),
                );
                let gone: Vec<u64> = w
                    .seen
                    .iter()
                    .copied()
                    .filter(|g| !view.iter().any(|(v, _)| v == g) && !shard.keeps(*g))
                    .collect();
                if !gone.is_empty() {
                    gone.iter().for_each(|g| {
                        w.seen.remove(g);
                    });
                    events.push(SessionEvent::ObjectsRemoved(gone));
                }
                for (guid, create) in view {
                    if w.seen.insert(guid) {
                        // A player first seen: the client names them by their Calling.
                        if let Some(o) = players.iter().find(|o| o.guid == guid) {
                            events.push(SessionEvent::Callings {
                                by_guid: vec![(guid, o.calling.clone())],
                            });
                        }
                        events.push(create);
                    }
                }
                w.runs = owners.remove(&id).unwrap_or_default();
                w.inner_open = shard.portals_open();
                (w.run_pinned, w.run_dead) = shard.run_facts();
                w.others = players
                    .iter()
                    .filter(|o| o.alive && o.guid != w.guid)
                    .map(|o| (o.guid, o.position, o.health))
                    .collect();
            }
        }
    }

    /// Each Phased Dungeon's meter hears what its Run's characters and creatures did this tick
    /// (stacks from casts, stacks drained by hiding, the creatures first hurt or dead and the
    /// clock), and every character in the Phase is shown the stacks it now holds.
    fn tick_noise(&mut self) {
        let mut heard: std::collections::BTreeMap<shard::Place, (u32, u32)> = Default::default();
        let worlds = std::iter::once(&mut self.world)
            .chain(self.parked.values_mut().map(|s| &mut s.world))
            .flatten();
        for w in worlds {
            let (gain, drain) = w.take_noise();
            let Some(place) = w.shard_place() else {
                continue;
            };
            let total = heard.entry(place).or_default();
            total.0 += gain;
            total.1 += drain;
        }
        for (place, (gain, drain)) in heard {
            let (phased::Phase::Dungeon(id), Some(shard)) =
                (place.phase, self.shards.get_mut(&place))
            else {
                continue;
            };
            let Some(meter) = phased::dungeon(id).and_then(|d| d.meter.as_ref()) else {
                continue;
            };
            shard.hear_noise(meter, gain, drain);
            let stacks = shard.noise();
            let sessions = std::iter::once((&mut self.world, &mut self.events)).chain(
                self.parked
                    .values_mut()
                    .map(|s| (&mut s.world, &mut s.events)),
            );
            for (w, events) in sessions {
                if let Some(w) = w.as_mut().filter(|w| w.shard_place() == Some(place)) {
                    w.show_noise(stacks, events);
                }
            }
        }
    }

    /// Each gaze that went off this tick lands on every character in its caster's Phase, and each
    /// cast that began with a warning sends it to them as a raid warning.
    fn tick_gazes(&mut self) {
        let mut cast = Vec::new();
        let worlds = std::iter::once(&mut self.world)
            .chain(self.parked.values_mut().map(|s| &mut s.world))
            .flatten();
        for w in worlds {
            let (gazes, warnings, foreign, scenery) = w.take_gazes();
            let nothing =
                gazes.is_empty() && warnings.is_empty() && foreign.is_empty() && scenery.is_empty();
            if let (Some(place), false) = (w.shard_place(), nothing) {
                cast.push((place, gazes, warnings, foreign, scenery));
            }
        }
        let characters = &self.characters;
        for (place, gazes, warnings, foreign, scenery) in cast {
            let cover = match place.phase {
                phased::Phase::Dungeon(id) => phased::dungeon(id).map_or(&[][..], |d| &d.cover[..]),
                phased::Phase::OpenWorld => &[][..],
            };
            let sessions = std::iter::once((&mut self.world, &mut self.events)).chain(
                self.parked
                    .values_mut()
                    .map(|s| (&mut s.world, &mut s.events)),
            );
            for (w, events) in sessions {
                let Some(w) = w.as_mut().filter(|w| w.shard_place() == Some(place)) else {
                    continue;
                };
                for text in &warnings {
                    events.push(SessionEvent::Chat(
                        benilla_protocol::messages::ChatMessage {
                            chat_type: benilla_protocol::messages::CHAT_MSG_RAID_WARNING,
                            language: 0,
                            sender_guid: 0,
                            target_guid: 0,
                            sender_name: None,
                            channel: None,
                            text: text.clone(),
                            chat_tag: 0,
                        },
                    ));
                }
                for s in &scenery {
                    events.push(match s {
                        world::Scenery::Show(create) => create.clone(),
                        world::Scenery::Hide(guid) => SessionEvent::ObjectDestroyed(*guid),
                    });
                }
                let at = characters.get(w.guid).map(|c| c.position);
                for gaze in &gazes {
                    w.gazed(gaze, at, cover, events);
                }
                let me = w.guid;
                for cast in foreign.iter().filter(|f| f.target == me) {
                    w.spell_from_afar(cast, events);
                }
            }
        }
    }

    /// Tells each session who else stands in its player's Phase and where, so a creature's spell
    /// with a random target can pick among them.
    fn refresh_phase_mates(&mut self) {
        let worlds = std::iter::once(&self.world)
            .chain(self.parked.values().map(|s| &s.world))
            .flatten();
        let mut standing = Vec::new();
        for w in worlds.filter(|w| w.alive()) {
            if let (Some(place), Some(c)) = (w.shard_place(), self.characters.get(w.guid)) {
                standing.push((place, w.guid, c.position));
            }
        }
        let worlds = std::iter::once(&mut self.world)
            .chain(self.parked.values_mut().map(|s| &mut s.world))
            .flatten();
        for w in worlds {
            let mine = w.shard_place();
            w.phase_mates = standing
                .iter()
                .filter(|(place, guid, _)| Some(*place) == mine && *guid != w.guid)
                .map(|&(_, guid, at)| (guid, at))
                .collect();
        }
    }

    /// The Encounters' tick: waves come at their points among the characters on the map, and
    /// what they say is sent to the characters it concerns.
    fn tick_encounters(&mut self, dt: f32, host: &dyn Host) {
        let place = shard::Place {
            map: content::EASTERN_KINGDOMS,
            phase: phased::Phase::OpenWorld,
        };
        let mut players = Vec::new();
        let worlds = std::iter::once(&self.world).chain(self.parked.values().map(|s| &s.world));
        for w in worlds.flatten() {
            let (true, Some(c)) = (w.shard_place() == Some(place), self.characters.get(w.guid))
            else {
                continue;
            };
            players.push(encounters::Player {
                guid: w.guid,
                position: c.position,
                alive: w.alive(),
                quests: c
                    .quests
                    .active
                    .iter()
                    .map(|a| (a.id, quests::uses_done(a)))
                    .collect(),
            });
        }
        let mut none = Vec::new();
        let mobs = self
            .shards
            .get_mut(&place)
            .map_or(&mut none, |s| &mut s.mobs);
        let told = self.encounters.tick(dt, &players, mobs, host);
        let sessions = std::iter::once((&self.world, &mut self.events))
            .chain(self.parked.values_mut().map(|s| (&s.world, &mut s.events)));
        for (w, events) in sessions {
            let Some(w) = w else { continue };
            events.extend(
                told.iter()
                    .filter(|(g, _)| *g == w.guid)
                    .map(|(_, e)| e.clone()),
            );
        }
    }

    /// Applies one command from the default session; its answers queue for
    /// [`Self::drain_events`].
    pub fn handle(&mut self, command: RealmCommand) {
        self.handle_as(SessionId::DEFAULT, command);
    }

    fn handle_served(&mut self, command: RealmCommand) {
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
                motion,
            } => {
                if let Some(w) = &mut self.world {
                    let from = self.characters.get(w.guid).map(|c| c.position);
                    let walled = from.is_some_and(|from| {
                        w.walled_off(from, position, orientation, &mut self.events)
                    });
                    if !walled && from.is_none_or(|from| w.moved(from, position, &mut self.events))
                    {
                        let guid = w.guid;
                        self.characters.set_position(guid, position, orientation);
                        // Trusted, as reported (ADR 0009): the others who see the player see it.
                        if let Some(place) = w.shard_place() {
                            let moved = SessionEvent::UnitMove {
                                guid,
                                position,
                                orientation,
                                flags: motion.flags,
                                pitch: motion.pitch,
                                time: match motion.time {
                                    0 => (self.wall * 1000.0) as u32,
                                    t => t,
                                },
                                verb: motion.verb,
                                fall_time: motion.fall_time,
                                jump: motion.jump,
                                transport: motion.transport,
                            };
                            self.show_others(place, &[moved]);
                        }
                        let explored = self
                            .characters
                            .get_mut(guid)
                            .is_some_and(|c| quests::explore(c, &mut self.events));
                        if explored {
                            self.save(guid);
                        }
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
            RealmCommand::CancelAutoRepeat => {
                if let Some(w) = &mut self.world {
                    w.stop_shooting(&mut self.events);
                }
            }
            RealmCommand::CastSpell { spell_id, target } => self.cast(spell_id, target, None),
            RealmCommand::CastSpellAtDest { spell_id, dest } => {
                self.cast(spell_id, None, Some(dest))
            }
            RealmCommand::UseItem { bag, slot } => {
                if let Some(w) = &mut self.world {
                    if let Some(c) = self.characters.get_mut(w.guid) {
                        let slot = inventory::Pos { bag, slot };
                        // A Quest Starter opens its quest; food and drink are eaten at once;
                        // anything else casts its spell.
                        if !quests::use_starter(c, slot, &mut self.events) {
                            match w.eat_or_drink(slot, c, self.wall, &mut self.events) {
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
            RealmCommand::UseGameObject { guid } if self.objects.get(guid).is_some() => {
                self.use_runtime_object(guid);
            }
            RealmCommand::UseGameObject { guid } => {
                let mut credited = None;
                let mut ward = None;
                if let Some(w) = &mut self.world {
                    let player = w.guid;
                    let giver = self
                        .characters
                        .get(player)
                        .and_then(|c| Some((c, w.giver_object_within_reach(guid, c.position)?)));
                    if let Some((c, entry)) = giver {
                        quests::object_hello(c, entry, guid, &mut self.events);
                    } else {
                        let quest_object = self
                            .characters
                            .get_mut(player)
                            .is_some_and(|c| w.use_quest_object(guid, c, &mut self.events));
                        let door = match self.characters.get_mut(player) {
                            Some(c) if !quest_object => w.click_door(guid, c, &mut self.events),
                            _ => world::DoorClick::NotADoor,
                        };
                        if let world::DoorClick::Opened {
                            dungeon,
                            secs,
                            first,
                        } = door
                        {
                            self.doors.open(dungeon, secs);
                            w.doors_open = self.doors.ids();
                            if first {
                                credited = Some(player);
                            }
                        } else if !matches!(door, world::DoorClick::NotADoor) {
                            // A door that stays shut.
                        } else if quest_object {
                            credited = Some(player);
                        } else if self
                            .characters
                            .get_mut(player)
                            .is_some_and(|c| w.click_stop_portal(guid, c, &mut self.events))
                        {
                            // A stop's portal, taken or dark.
                        } else if let Some(used) = w.use_gameobject(guid) {
                            if let Some(c) = self.characters.get(w.guid) {
                                w.begin_hide(used, c);
                                if w.ward_click(used, c) {
                                    ward = Some(used);
                                }
                            }
                            self.uses.push(used);
                        }
                    }
                }
                if let Some(used) = ward {
                    self.use_ward(used);
                }
                if let Some(player) = credited {
                    self.save(player);
                }
            }
            RealmCommand::CreatureQuery { entry } => {
                let answer =
                    hub::creature_name(entry).unwrap_or_else(|| mobs::creature_name(entry));
                self.events.push(answer);
            }
            RealmCommand::Chat {
                chat_type,
                target,
                text,
            } => self.chat(chat_type, target, text),
            RealmCommand::TextEmote { text_id, target } => self.text_emote(text_id, target),
            RealmCommand::NameQuery { guid } => self.name_query(guid),
            RealmCommand::Logout => self.logout(),
            // A logout is instant, so there is never one to cancel.
            RealmCommand::LogoutCancel => {}
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
            RealmCommand::PageTextQuery { page_id } => {
                self.events.extend(items::page_chain(page_id));
            }
            RealmCommand::LearnTalent { talent_id, rank } => self.learn_talent(talent_id, rank),
            RealmCommand::GossipHello { guid } => self.gossip_hello(guid),
            RealmCommand::GossipSelectOption { guid, option } => self.gossip_select(guid, option),
            RealmCommand::NpcTextQuery { text_id } => {
                self.events.extend(hub::greeting(text_id));
            }
            RealmCommand::TalentWipeConfirm { trainer } => self.talent_wipe(trainer),
            RealmCommand::BinderActivate { binder } => self.binder_activate(binder),
            RealmCommand::QuestQuery { quest } => {
                self.events.extend(quests::template_response(quest));
            }
            other if other.is_quest() => self.handle_quest(other),
            other if other.is_trade() => self.handle_trade(other),
            other if other.is_duel() => self.handle_duel(other),
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
        let open_chest = w
            .looting
            .open_guid()
            .filter(|&g| self.objects.get(g).is_some());
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
            RealmCommand::SetAmmo { entry } => moved.gear_changed = ammo::set(c, entry, events),
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
        // A chest keeps what is left in it; the last item taken removes it.
        if let Some(chest) = open_chest {
            if let Some(loot) = w.looting.loot_of(chest).cloned() {
                if loot.is_empty() {
                    w.looting.release(events);
                    self.objects.remove(chest);
                } else {
                    self.objects.set_loot(chest, loot);
                }
            }
            looted.flag = None;
        }
        moved.gear_changed |= traded.gear_changed;
        looted.took |= traded.changed;
        // A quiver in a bag slot moves the ranged speed (#158).
        if moved.gear_changed || moved.bags_changed {
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

    /// Advances the world by `dt` seconds: every session's, in session order.
    pub fn tick(&mut self, dt: f32, host: &dyn Host) {
        self.wall += f64::from(dt);
        self.doors.tick(dt);
        let mut ids: Vec<SessionId> = self.parked.keys().copied().collect();
        ids.push(self.active);
        ids.sort();
        // A dev teleport lands, and each player steps into the Phase where they stand, before
        // the shards stream in around them.
        for &id in &ids {
            self.serve(id);
            self.dev_teleport_served(host);
            if let Some(w) = &mut self.world {
                if let Some(c) = self.characters.get_mut(w.guid) {
                    w.step_into_phase(c, &mut self.events);
                }
            }
        }
        self.tick_shards(dt, host);
        self.refresh_phase_mates();
        for id in ids {
            self.serve(id);
            self.tick_served(dt, host);
        }
        self.give_back();
        self.tick_rumours(host);
        self.tick_noise();
        self.tick_gazes();
        self.tick_encounters(dt, host);
        self.party_tick(dt);
        self.tick_trades();
        self.tick_duels(dt);
    }

    /// The Rumours' tick: a giver is made for each new one, and each that ran out of time or was
    /// handed in by everyone ends.
    fn tick_rumours(&mut self, host: &dyn Host) {
        for i in self.rumours.unspawned() {
            let r = self.rumours.get(i);
            let Some(d) = rumours::def(r.id) else {
                continue;
            };
            let p = &d.points[r.point];
            let (level, audience) = (r.level, r.audience.clone());
            let guid = self.spawn_for(
                host,
                d.giver,
                level,
                [p.x, p.y, 0.0],
                d.map,
                audience.clone(),
            );
            // Every instance is offered afresh: the steps an earlier one finished are forgotten.
            if d.kind == rumours::Class::Ordinary {
                for &g in &audience.0 {
                    if let Some(c) = self.characters.get_mut(g) {
                        c.quests.completed.retain(|q| !d.quests.contains(q));
                    }
                }
            }
            // A giver the realm cannot make leaves the Rumour with nobody to talk to; it still
            // times out.
            self.rumours.set_giver(i, guid.unwrap_or(u64::MAX));
        }
        for order in self.rumours.take_orders() {
            self.fill_order(host, order);
        }
        let objects = &self.objects;
        let holds = |guid| {
            objects
                .get(guid)
                .and_then(|o| o.loot.as_ref())
                .is_some_and(|l| l.items.iter().any(Option::is_some))
        };
        for r in self.rumours.take_ended(self.wall, holds) {
            let (quests, notes) = rumours::leftovers(r.id);
            self.despawn_audience(&r.audience);
            for &chest in r.chests() {
                self.objects.remove(chest);
            }
            let sessions = std::iter::once((&mut self.world, &mut self.events)).chain(
                self.parked
                    .values_mut()
                    .map(|s| (&mut s.world, &mut s.events)),
            );
            let mut changed = Vec::new();
            for (w, events) in sessions {
                let Some(w) = w else { continue };
                let Some(c) = self
                    .characters
                    .get_mut(w.guid)
                    .filter(|_| r.audience.admits(w.guid))
                else {
                    continue;
                };
                quests::end_rumour(c, &quests, &notes, events);
                changed.push(w.guid);
            }
            for guid in changed {
                self.save(guid);
            }
        }
    }

    /// Makes what a step accept or a last-step hand-in ordered: the step's spawns for the whole
    /// Audience, or a chest for the one who handed in.
    fn fill_order(&mut self, host: &dyn Host, order: rumours::Order) {
        match order {
            rumours::Order::Step { rumour, quest } => {
                let r = self.rumours.get(rumour);
                let Some(d) = rumours::def(r.id) else {
                    return;
                };
                let (level, audience) = (r.level, r.audience.clone());
                for s in d.spawn.iter().filter(|s| s.quest == quest) {
                    let at = &s.points[self.rng.below(s.points.len() as u32) as usize];
                    let position = [at.x, at.y, 0.0];
                    for _ in 0..s.count {
                        if let Some(entry) = s.creature {
                            let level = s.level.unwrap_or(level);
                            self.spawn_for(host, entry, level, position, d.map, audience.clone());
                        } else if let Some(o) = &s.object {
                            let spec = ObjectSpec {
                                entry: o.entry,
                                display: o.display,
                                kind: ObjectKind::Use,
                                map: d.map,
                                position,
                                orientation: 0.0,
                            };
                            self.spawn_object(host, spec, audience.clone());
                        }
                    }
                }
            }
            rumours::Order::Chest { rumour, guid } => {
                let r = self.rumours.get(rumour);
                let Some(d) = rumours::def(r.id) else {
                    return;
                };
                let items = rumours::chest_items(d, r.level, &mut self.rng);
                // Two yards from the giver's point, which the ground then settles.
                let p = &d.points[r.point];
                let spec = ObjectSpec {
                    entry: rumours::CHEST_ENTRY,
                    display: CHEST_DISPLAY,
                    kind: ObjectKind::Chest(items),
                    map: d.map,
                    position: [p.x + 2.0, p.y, 0.0],
                    orientation: 0.0,
                };
                let chest = self.spawn_object(host, spec, Audience::of(&[guid]));
                self.rumours.add_chest(rumour, chest);
            }
        }
    }

    fn dev_teleport_served(&mut self, host: &dyn Host) {
        let (Some(w), Some(to)) = (&mut self.world, self.dev_teleport.take()) else {
            return;
        };
        if let Some(c) = self.characters.get_mut(w.guid) {
            w.dev_teleport(to, host, c, &mut self.events);
        }
    }

    fn tick_served(&mut self, dt: f32, host: &dyn Host) {
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
        w.show_runtime_objects(&self.objects, c, &mut self.events);
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
    fn cast(&mut self, spell_id: u32, target: Option<u64>, dest: Option<[f32; 3]>) {
        if spell_id == duel::content().challenge_spell {
            return self.challenge(target);
        }
        let Some(w) = &mut self.world else {
            return;
        };
        let Some(c) = self.characters.get_mut(w.guid) else {
            return;
        };
        w.set_dest(dest);
        let cast = w.cast(spell_id, target, c, &mut self.rng, &mut self.events);
        if cast.leveled {
            let guid = w.guid;
            self.save(guid);
        }
    }

    /// The default session's events since the last drain, oldest first.
    pub fn drain_events(&mut self) -> Vec<SessionEvent> {
        self.give_back();
        self.serve(SessionId::DEFAULT);
        self.lent_mark = 0;
        std::mem::take(&mut self.events)
    }

    /// Opens or closes the inner portal to stop `stop` of Phased Dungeon `dungeon` in its Run.
    /// Nothing happens while no one is in the dungeon, since its next Run opens its portals as
    /// the content says.
    pub fn set_inner_portal(&mut self, dungeon: u32, stop: &str, open: bool) {
        let Some(d) = phased::dungeon(dungeon) else {
            return;
        };
        let place = shard::Place {
            map: d.map,
            phase: phased::Phase::Dungeon(dungeon),
        };
        if let Some(shard) = self.shards.get_mut(&place) {
            shard.set_portal(stop, open);
        }
    }

    /// Teleports every character standing in stop `stop` of Phased Dungeon `dungeon` to its
    /// inner portal's `back`, still in the Run.
    pub fn recall_from_stop(&mut self, dungeon: u32, stop: &str) {
        self.recall_stop(dungeon, stop, None);
    }

    /// Sends everyone in stop `stop` of dungeon `dungeon` back, and gives each one use credit
    /// for the GameObject of entry `credit`, if any.
    fn recall_stop(&mut self, dungeon: u32, stop: &str, credit: Option<u32>) {
        let Some(d) = phased::dungeon(dungeon) else {
            return;
        };
        let Some(i) = d.stop.iter().position(|s| s.key == stop) else {
            return;
        };
        let sessions = std::iter::once((&mut self.world, &mut self.events)).chain(
            self.parked
                .values_mut()
                .map(|s| (&mut s.world, &mut s.events)),
        );
        let mut sent = Vec::new();
        for (w, events) in sessions {
            let Some(w) = w.as_mut() else { continue };
            if let Some(c) = self.characters.get_mut(w.guid) {
                if w.recall_from_stop(d, i, c, events) {
                    if let Some(entry) = credit {
                        quests::credit_use(c, entry, events);
                    }
                    sent.push(w.guid);
                }
            }
        }
        if credit.is_some() {
            sent.into_iter().for_each(|guid| self.save(guid));
        }
    }

    /// The active session clicks the ward stone `used`, in reach. While the creature it needs
    /// lives, they are told it holds; once it is dead, the first click pins the stone's stop in
    /// the Run, sends everyone in the stop back, and credits each of them with a use of it.
    fn use_ward(&mut self, used: GameObjectUse) {
        let Some((d, (stone, _))) =
            phased::dungeon(used.dungeon).and_then(|d| Some((d, d.usable(used.guid)?)))
        else {
            return;
        };
        let Some(ward) = &stone.ward else {
            return;
        };
        let place = shard::Place {
            map: d.map,
            phase: phased::Phase::Dungeon(d.id),
        };
        let Some(shard) = self.shards.get_mut(&place) else {
            return;
        };
        if !shard.is_dead(ward.needs_dead) {
            self.events.push(SessionEvent::Notification {
                text: ward.held_text.clone(),
            });
            return;
        }
        if shard.pin(&ward.stop, ward.opens.as_deref()) {
            self.recall_stop(d.id, &ward.stop, Some(stone.guid));
        }
    }

    /// The dungeon GameObject clicks since the last drain, oldest first.
    pub fn drain_uses(&mut self) -> Vec<GameObjectUse> {
        std::mem::take(&mut self.uses)
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

    /// Seconds into the Run of the Phased Dungeon the character in the world stands in; `None`
    /// outside every one.
    pub fn run_elapsed(&self) -> Option<f32> {
        self.run_elapsed_as(SessionId::DEFAULT)
    }

    /// Seconds into the Run of the Phased Dungeon `session`'s character stands in; `None`
    /// outside every one.
    pub fn run_elapsed_as(&self, session: SessionId) -> Option<f32> {
        let world = if self.active == session {
            self.world.as_ref()
        } else {
            self.parked.get(&session).and_then(|s| s.world.as_ref())
        };
        let place = world.and_then(|w| w.shard_place())?;
        self.shards.get(&place)?.run_elapsed()
    }

    /// Keeps `guid` from ever being handed out: the host holds a save under it that it could not
    /// import, and a new character must not overwrite it.
    pub fn reserve_guid(&mut self, guid: u64) {
        self.characters.reserve(guid);
    }

    /// Spawns the creature of mob template `entry` at `level` on `map`'s open world, at
    /// `position` grounded on `host`'s ground, for `audience` alone (ADR 0014). No spawn timer
    /// revives it. Its guid, or `None` when `entry` is no mob template.
    pub fn spawn_for(
        &mut self,
        host: &dyn Host,
        entry: u32,
        level: u8,
        position: [f32; 3],
        map: u32,
        audience: Audience,
    ) -> Option<u64> {
        let template = mobs::template(entry)?;
        let place = shard::Place {
            map,
            phase: phased::Phase::OpenWorld,
        };
        // The served session may hold the shard's mobs on loan; they come back first.
        self.give_back();
        let shard = self
            .shards
            .entry(place)
            .or_insert_with(|| shard::Shard::new(place));
        Some(shard.spawn_for(host, template, level, position, audience))
    }

    /// Removes every creature made for exactly `audience` on any shard; each admitted client
    /// that had one in view is sent its destroy on the next tick.
    pub fn despawn_audience(&mut self, audience: &Audience) {
        self.give_back();
        for shard in self.shards.values_mut() {
            shard.despawn_audience(audience);
        }
        self.objects.despawn_audience(audience);
    }

    /// Spawns a GameObject for `audience` alone (ADR 0014), at `spec.position` grounded on
    /// `host`'s ground. Its guid. A click from anyone else does nothing and they never see it.
    pub fn spawn_object(
        &mut self,
        host: &dyn Host,
        mut spec: ObjectSpec,
        audience: Audience,
    ) -> u64 {
        spec.position = ground::grounded_at(host, spec.map, spec.position, true);
        self.objects.spawn(spec, audience)
    }

    /// The served session clicks the run-time GameObject `guid`, which `Self::objects` holds.
    fn use_runtime_object(&mut self, guid: u64) {
        let Some(w) = &mut self.world else {
            return;
        };
        let Some(c) = self.characters.get_mut(w.guid) else {
            return;
        };
        let Some(o) = self.objects.get(guid) else {
            return;
        };
        if !o.audience.admits(w.guid)
            || !w.sees_runtime_object(guid)
            || space::flat_distance(o.spec.position, c.position) > space::INTERACTION_YD
        {
            return;
        }
        match &o.loot {
            None => {
                if quests::credit_use(c, o.spec.entry, &mut self.events) {
                    self.objects.remove(guid);
                }
            }
            Some(loot) => w.looting.offer(guid, loot.clone(), &mut self.events),
        }
    }

    fn save(&mut self, guid: u64) {
        // A save from the world stamps the leaving: a session that ends without a logout banks
        // its rest from the last save.
        if self.world.as_ref().is_some_and(|w| w.guid == guid) {
            if let Some(c) = self.characters.get_mut(guid) {
                rest::leave(c, self.wall);
            }
        }
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
        let by_guid = characters
            .iter()
            .filter_map(|c| Some((c.guid, self.characters.get(c.guid)?)))
            .map(|(guid, c)| (guid, callings::of(c.calling).key.clone()))
            .collect();
        self.events.push(SessionEvent::CharacterList {
            characters,
            realm: Some(realm),
        });
        self.events.push(SessionEvent::Callings { by_guid });
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
        self.give_back();
        // A character another session has in the world stays with it.
        if self
            .parked
            .values()
            .any(|s| s.world.as_ref().is_some_and(|w| w.guid == guid))
        {
            // `CHAR_LOGIN_DUPLICATE_CHARACTER`, the reference's second refusal.
            self.events
                .push(SessionEvent::CharacterLoginFailed { result: 2 });
            return;
        }
        // A save from inside the Dungeon comes back outside it: a run ends with its session.
        if let Some(c) = self.characters.get_mut(guid) {
            world::come_out(c);
            rest::come_back(c, self.wall);
        }
        let Some(c) = self.characters.get(guid) else {
            // `CHAR_LOGIN_NO_CHARACTER`, the reference's fifth refusal.
            self.events
                .push(SessionEvent::CharacterLoginFailed { result: 5 });
            return;
        };
        let mut entry = world::InWorld::enter(c, self.dev, self.wall, &mut self.events);
        entry.casters = self.casters.clone();
        self.world = Some(entry);
        self.events.push(chat::joined_world_channel());
    }

    /// A logout is always instant: no timer, no rest-area rule, no refusal in combat.
    fn logout(&mut self) {
        self.dev_teleport = None;
        self.give_back();
        let Some(mut w) = self.world.take() else {
            return;
        };
        // Leaving dead is a release: the save puts the character where it comes back.
        if let Some(c) = self.characters.get_mut(w.guid) {
            w.release(c, &mut Vec::new());
            // Leaving inside the Dungeon leaves it.
            world::come_out(c);
            rest::leave(c, self.wall);
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
