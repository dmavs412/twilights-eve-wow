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
//!   once it loads, from [`callings::skill_spell_ids`] and [`callings::wip_rank_spell_ids`], then applies the
//!   compiled-in `client_dbc_patch.json` (ADR 0003); `twilight/talents.rs` builds the talent
//!   frame's catalog from the current Calling's tree.
//! - **Identity**: the `Calling` enum (`content.rs`) is the save's key; a renamed Calling keeps its
//!   old key as a serde alias. A skill two Callings share has identical data in both.

mod ammo;
mod areas;
pub mod callings;
mod characters;
pub mod chat;
mod clock;
mod combat;
mod content;
mod creature_spells;
mod dev;
mod dungeon;
mod equipment;
mod fields;
mod generated;
mod group;
mod hub;
mod inn;
mod inventory;
mod items;
mod looting;
mod mobs;
mod party;
mod phased;
mod population;
mod quests;
mod rest;
mod rng;
pub mod rules;
mod save;
mod shard;
mod space;
mod tables;
mod talents;
mod token_vendor;
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
    /// A logout away from an inn waits [`CAMP_SECS`], as on a server (#146); solo is instant.
    camp_logout: bool,
}

/// How long a logout away from an inn takes when [`Realm::with_camp_logout`] is on.
pub const CAMP_SECS: f32 = 20.0;

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

    /// This realm with creature `entry` casting the Creature Spells `keys`
    /// (`content/creature_spells.toml`) in place of its template's: a test seam, so a player
    /// script can fight a caster where the content has none.
    pub fn with_creature_spells(mut self, entry: u32, keys: &[&str]) -> Self {
        self.casters.add(entry, keys);
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

    /// A realm whose logouts away from an inn take [`CAMP_SECS`], refused in combat and cancelled
    /// by moving: the friends server's rule.
    pub fn with_camp_logout(mut self) -> Self {
        self.camp_logout = true;
        self
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

    /// Lends the served session its shard's mobs, when its player stands in the open world.
    fn lend(&mut self) {
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
                        SessionEvent::ObjectCreate { guid, .. } if *guid != w.guid => {
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
        for help in w.take_helps() {
            let to = help.to();
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
        let mut own = None;
        for (id, w, _) in near {
            let remainder = if id == killer { kill.money % n } else { 0 };
            let credit = world::Credit {
                kill: kill.clone(),
                money: Some(kill.money / n + remainder),
                levels: levels.clone(),
            };
            if id == killer {
                own = Some(credit);
            } else {
                w.credits.push(credit);
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
    /// it this tick.
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
                hidden: w.hidden(),
            });
        }
        self.shards.retain(|map, _| on.contains_key(map));
        for (map, players) in &on {
            let shard = self
                .shards
                .entry(*map)
                .or_insert_with(|| shard::Shard::new(*map));
            let positions: Vec<[f32; 3]> = players.iter().map(|p| p.position).collect();
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
                let mut view = shard.in_view(p.position);
                // The other players in view.
                view.extend(
                    players
                        .iter()
                        .filter(|o| o.guid != p.guid && !o.hidden)
                        .filter(|o| {
                            space::flat_distance(o.position, p.position) <= population::VIEW_YD
                        })
                        .map(|o| (o.guid, o.create.clone())),
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
                        events.push(create);
                    }
                }
                w.runs = owners.remove(&id).unwrap_or_default();
                w.others = players
                    .iter()
                    .filter(|o| o.alive && o.guid != w.guid)
                    .map(|o| (o.guid, o.position, o.health))
                    .collect();
            }
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
            } => {
                if let Some(w) = &mut self.world {
                    let from = self.characters.get(w.guid).map(|c| c.position);
                    // Walking off stands up from a pending logout.
                    if from.is_some_and(|f| f != position) && w.logout_in.take().is_some() {
                        self.events.push(SessionEvent::LogoutCancelled);
                    }
                    if from.is_none_or(|from| w.moved(from, position, &mut self.events)) {
                        let guid = w.guid;
                        self.characters.set_position(guid, position, orientation);
                        // Trusted, as reported (ADR 0009): the others who see the player see it.
                        if let Some(place) = w.shard_place() {
                            let moved = SessionEvent::UnitMove {
                                guid,
                                position,
                                orientation,
                                flags: 0,
                                pitch: 0.0,
                                time: (self.wall * 1000.0) as u32,
                                verb: benilla_protocol::messages::RelayVerb::Heartbeat,
                                fall_time: 0,
                                jump: None,
                                transport: None,
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
            RealmCommand::Chat {
                chat_type,
                target,
                text,
            } => self.chat(chat_type, target, text),
            RealmCommand::TextEmote { text_id, target } => self.text_emote(text_id, target),
            RealmCommand::NameQuery { guid } => self.name_query(guid),
            RealmCommand::Logout => self.ask_logout(),
            RealmCommand::LogoutCancel => {
                if let Some(w) = &mut self.world {
                    if w.logout_in.take().is_some() {
                        self.events.push(SessionEvent::LogoutCancelled);
                    }
                }
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
                self.events.extend(hub::greeting(text_id));
            }
            RealmCommand::TalentWipeConfirm { trainer } => self.talent_wipe(trainer),
            RealmCommand::BinderActivate { binder } => self.binder_activate(binder),
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

    /// Advances the world by `dt` seconds: every session's, in session order.
    pub fn tick(&mut self, dt: f32, host: &dyn Host) {
        self.wall += f64::from(dt);
        let mut ids: Vec<SessionId> = self.parked.keys().copied().collect();
        ids.push(self.active);
        ids.sort();
        // A dev teleport lands, and each player steps into the Phase where they stand, before
        // the shards stream in around them.
        for &id in &ids {
            self.serve(id);
            self.dev_teleport_served(host);
            if let Some(w) = &mut self.world {
                if let Some(c) = self.characters.get(w.guid) {
                    w.step_into_phase(c, &mut self.events);
                }
            }
        }
        self.tick_shards(dt, host);
        for id in ids {
            self.serve(id);
            self.tick_served(dt, host);
        }
        self.give_back();
        self.party_tick(dt);
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
        if let Some(left) = &mut w.logout_in {
            *left -= dt;
            if *left <= 0.0 {
                self.dev_teleport = None;
                self.logout();
                return;
            }
        }
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
    fn cast(&mut self, spell_id: u32, target: Option<u64>, dest: Option<[f32; 3]>) {
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

    /// A logout asked for: at once in an inn, solo, or out of the world; refused in combat;
    /// otherwise after [`CAMP_SECS`].
    fn ask_logout(&mut self) {
        let camps = self.camp_logout
            && self.world.as_ref().is_some_and(|w| {
                self.characters
                    .get(w.guid)
                    .is_some_and(|c| !rest::in_inn(c.map, c.position))
            });
        let Some(w) = self.world.as_mut().filter(|_| camps) else {
            self.dev_teleport = None;
            self.logout();
            return;
        };
        if w.in_combat() {
            // vmangos `LogoutRequest`: 1 is "You can't log out now." in combat.
            self.events.push(SessionEvent::LogoutResponse {
                reason: 1,
                instant: false,
            });
            return;
        }
        if w.logout_in.is_none() {
            w.logout_in = Some(CAMP_SECS);
            self.events.push(SessionEvent::LogoutResponse {
                reason: 0,
                instant: false,
            });
        }
    }

    fn logout(&mut self) {
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
