//! **Twilight's Eve**, the glue half (ADR 0001): the boot flag, and the in-process backend that
//! puts the Local Realm ([`twilight_realm`]) on the far ends of the net layer's channels. Once a
//! frame, before the event drain, it hands the realm what the client asked, ticks it on the main
//! thread, and pushes its events where the IO thread would. The rules live in the realm crate;
//! nothing here decides anything.

use benilla_assets::coords::wow_to_bevy;
use benilla_protocol::{SessionEnd, SessionEvent};
use benilla_world::collision::WorldCollision;
use benilla_world::liquid::{water_surface_at, LiquidClaim, WaterChunkInfo};
use benilla_world::schedule::WorldStage;
use benilla_world::world_point::WorldPoint;
use std::collections::BTreeSet;
use std::path::PathBuf;

use bevy::prelude::*;
use twilight_realm::{DevMultipliers, Host, Realm, RealmCommand, Storage};

use crate::net::{CharRequest, ClientCommand, FarEnds, InProcessLink, SelfGuid};

mod goto;
mod remote;
mod talents;
#[cfg(test)]
pub(crate) mod vanilla_player;

pub(crate) use goto::DevGoto;

/// A dev aid for the realm: start Rumour `id` for the player, its giver at `point` (from 0)
/// (`WOW_PROBE_RUMOUR`, `capture::probes::rumour`).
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct DevRumour {
    pub id: u32,
    pub point: usize,
}

/// A dev aid for the realm: start Rumour `id` and make its step quest `quest`'s spawns at each
/// spawn table's point `point` (from 0) (`WOW_PROBE=spawn_audit`, `capture::probes::spawn_audit`).
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct DevRumourStep {
    pub id: u32,
    pub quest: u32,
    pub point: usize,
}

/// A dev aid for the realm: end the player's Rumour `id` (`WOW_PROBE=spawn_audit`).
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct DevRumourEnd {
    pub id: u32,
}

/// Set by [`crate::run_twilight`] before the app builds: the `twilights-eve` launcher boots
/// Twilight's Eve, the plain `benilla` client does not.
pub(crate) static ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn active() -> bool {
    ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
}

/// The dev multipliers, read once at boot from `WOW_DEV_SPEED` / `WOW_DEV_XP`. They live only in
/// this run's realm, never in a save.
fn dev_multipliers() -> DevMultipliers {
    let flag = |name: &str| std::env::var(format!("WOW_DEV_{name}")).ok();
    let dev = DevMultipliers::from_flags(flag("SPEED").as_deref(), flag("XP").as_deref());
    if dev != DevMultipliers::default() {
        info!(
            "twilight: dev multipliers speed ×{} xp ×{}",
            dev.speed, dev.xp
        );
    }
    dev
}

/// Attaches the Local Realm. Added after `NetPlugin`, whose [`InProcessLink`] it takes.
pub(crate) struct TwilightPlugin;

impl Plugin for TwilightPlugin {
    fn build(&self, app: &mut App) {
        let far = app
            .world_mut()
            .get_resource_mut::<InProcessLink>()
            .and_then(|mut link| link.0.take())
            .expect("Twilight's Eve needs the net layer's in-process backend");
        let backing = match remote::server_address() {
            Some(address) => {
                let commit = app
                    .world()
                    .get_resource::<crate::BuildId>()
                    .map_or("", |b| b.sha);
                let hello = remote::hello(twilight_realm::build_id::build_id(commit));
                Backing::Remote(remote::RemoteLink::connect(&address, &hello))
            }
            None => {
                // Fresh dice every session; a test seeds its own.
                let mut realm = Realm::with_seed(rand::random()).with_dev(dev_multipliers());
                let saved = load_saves(&mut realm);
                Backing::Local { realm, saved }
            }
        };
        app.init_resource::<crate::char_select::ClassLabels>()
            .init_resource::<CallingKeys>()
            .init_resource::<BossFightList>()
            .init_resource::<MapMarkList>()
            .init_resource::<SelfCalling>();
        app.add_message::<DevGoto>()
            .add_message::<DevRumour>()
            .add_message::<DevRumourStep>()
            .add_message::<DevRumourEnd>()
            .init_resource::<goto::GotoGround>();
        app.insert_resource(LocalRealm {
            far,
            backing,
            listed: false,
        })
        .add_systems(
            Update,
            (goto::fetch_goto_ground, pump)
                .chain()
                .in_set(WorldStage::Net)
                .before(crate::net::apply_net_updates),
        )
        .add_systems(
            Update,
            only_warriors.run_if(resource_added::<crate::entities::CharCreate>),
        )
        .add_systems(
            Update,
            rewrite_spells.run_if(resource_added::<crate::ui_action::Spells>),
        )
        .add_systems(
            Update,
            (
                talents::follow_calling.after(pump),
                track_self_calling.after(pump),
                crate::ui_twilight::unlock_talent_frame,
            ),
        );
        // Closing the window logs the character out, which saves it.
        crate::shutdown::on_app_exit(app, save_on_exit.into_configs());
    }
}

/// The client is going down: log out, and write the save that makes.
/// On a server, the logout is sent and the server saves.
fn save_on_exit(mut local: ResMut<LocalRealm>) {
    local.backing.handle(RealmCommand::Logout);
    if let Backing::Local { realm, saved } = &mut local.backing {
        for op in realm.drain_storage() {
            store(op, saved);
        }
    }
}

/// The realm and the channel ends it serves.
#[derive(Resource)]
struct LocalRealm {
    far: FarEnds,
    backing: Backing,
    /// The first roster went out: there is no login, so the realm answers at once.
    listed: bool,
}

/// Where the realm runs: in this process, or on a friends server.
enum Backing {
    Local {
        realm: Realm,
        /// The guids with a save on disk, which the index file lists.
        saved: BTreeSet<u64>,
    },
    Remote(remote::RemoteLink),
}

impl Backing {
    fn handle(&mut self, command: RealmCommand) {
        match self {
            Backing::Local { realm, .. } => realm.handle(command),
            Backing::Remote(link) => link.send(command),
        }
    }
}

// ── Saves ─────────────────────────────────────────────────────────────────────────────────────────
//
// One file per character under `benilla-config/twilight/` (`localStorage` keys on the web), and an
// index naming them, since a browser's store cannot list a folder.

/// `benilla-config/twilight/characters.txt`: one saved guid per line.
fn index_path() -> Option<PathBuf> {
    crate::local_state::home().map(|h| h.join("twilight/characters.txt"))
}

/// `benilla-config/twilight/character-<guid>.toml`: one character's save.
fn save_path(guid: u64) -> Option<PathBuf> {
    crate::local_state::home().map(|h| h.join(format!("twilight/character-{guid}.toml")))
}

/// Imports every indexed save; one that cannot be read is skipped with a warning and left on
/// disk, so a newer build can still read it.
fn load_saves(realm: &mut Realm) -> BTreeSet<u64> {
    let mut saved = BTreeSet::new();
    let Some(index) = index_path().and_then(|p| crate::local_state::read_to_string(&p).ok()) else {
        return saved;
    };
    for guid in index.lines().filter_map(|l| l.trim().parse::<u64>().ok()) {
        let Some(path) = save_path(guid) else {
            continue;
        };
        // Indexed either way: an unreadable save stays on disk and in the index, and its guid is
        // never handed to a new character, whose save would overwrite it.
        saved.insert(guid);
        realm.reserve_guid(guid);
        match crate::local_state::read_to_string(&path) {
            Ok(blob) => match realm.import_character(&blob) {
                Ok(_) => {
                    for line in realm.drain_log() {
                        warn!("twilight: {line}");
                    }
                }
                Err(e) => warn!("twilight: skipping the save {} ({e:?})", path.display()),
            },
            Err(e) => warn!("twilight: cannot read the save {}: {e}", path.display()),
        }
    }
    info!("twilight: {} character save(s) loaded", saved.len());
    saved
}

/// Carries out the realm's save requests.
fn store(op: Storage, saved: &mut BTreeSet<u64>) {
    let changed = match op {
        Storage::Write { guid, blob } => {
            // `None`: persistence is off (a capture run).
            let Some(path) = save_path(guid) else {
                return;
            };
            if let Err(e) = crate::local_state::write_atomic(&path, &blob) {
                warn!("twilight: saving character {guid}: {e}");
                return;
            }
            saved.insert(guid)
        }
        Storage::Remove { guid } => {
            let Some(path) = save_path(guid) else {
                return;
            };
            match crate::local_state::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => warn!("twilight: removing the save of character {guid}: {e}"),
            }
            saved.remove(&guid)
        }
    };
    if changed {
        let index: String = saved.iter().map(|g| format!("{g}\n")).collect();
        if let Some(Err(e)) = index_path().map(|p| crate::local_state::write_atomic(&p, &index)) {
            warn!("twilight: writing the save index: {e}");
        }
    }
}

/// Commands in, one tick, events out.
fn pump(
    mut local: ResMut<LocalRealm>,
    time: Res<Time>,
    world: WorldPoint,
    collision: WorldCollision,
    mut labels: ResMut<crate::char_select::ClassLabels>,
    mut keys: ResMut<CallingKeys>,
    mut boss: ResMut<BossFightList>,
    mut marks: ResMut<MapMarkList>,
    mut goto: ResMut<goto::GotoGround>,
    mut rumours: (
        MessageReader<DevRumour>,
        MessageReader<DevRumourStep>,
        MessageReader<DevRumourEnd>,
    ),
    adts: Res<Assets<benilla_assets::AdtTile>>,
    water: Query<&'static WaterChunkInfo>,
    mut reel: (
        Option<ResMut<crate::fishing_reel::ReelInbox>>,
        Option<ResMut<crate::fishing_reel::ReelOutbox>>,
        Option<ResMut<crate::calling_choice::CallingChoiceInbox>>,
    ),
) {
    let LocalRealm {
        far,
        backing,
        listed,
    } = &mut *local;
    if !*listed {
        *listed = true;
        backing.handle(RealmCommand::ListCharacters);
    }
    while let Ok(pick) = far.pick.try_recv() {
        backing.handle(match pick {
            CharRequest::Enter(guid) => RealmCommand::EnterWorld { guid },
            CharRequest::Create(req) => RealmCommand::CreateCharacter(req),
            CharRequest::Delete(guid) => RealmCommand::DeleteCharacter { guid },
            // Select's Back has no login screen to go to; the roster brings the screen back.
            CharRequest::Abandon => RealmCommand::ListCharacters,
        });
    }
    // One realm and no login: the realm list and credentials have nothing to answer.
    while far.realm.try_recv().is_ok() {}
    while far.login.try_recv().is_ok() {}
    while let Ok(command) = far.commands.try_recv() {
        if let Some(command) = realm_command(command) {
            backing.handle(command);
        }
    }
    for DevGoto { x, y, facing } in goto.take_ready() {
        backing.handle(RealmCommand::DevTeleport { x, y, facing });
    }
    for &DevRumour { id, point } in rumours.0.read() {
        backing.handle(RealmCommand::DevRumour { id, point });
    }
    for &DevRumourStep { id, quest, point } in rumours.1.read() {
        backing.handle(RealmCommand::DevRumourStep { id, quest, point });
    }
    for &DevRumourEnd { id } in rumours.2.read() {
        backing.handle(RealmCommand::DevRumourEnd { id });
    }
    // The Reeling Bar's results (ADR 0018).
    if let Some(outbox) = reel.1.as_deref_mut() {
        for command in outbox.0.drain(..) {
            backing.handle(command);
        }
    }

    let events: Vec<SessionEvent> = match backing {
        Backing::Local { realm, saved } => {
            // The wall clock an item's cooldown runs on, so time between sessions counts.
            // `web-time`: std's `SystemTime` panics in a browser.
            if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                realm.set_wall_clock(now.as_millis() as u64);
            }
            let host = TerrainHost {
                world: &world,
                collision: &collision,
                goto: &goto,
                adts: &adts,
                water: &water,
            };
            realm.tick(time.delta_secs(), &host);
            for op in realm.drain_storage() {
                store(op, saved);
            }
            realm.drain_events()
        }
        Backing::Remote(link) => link.drain().collect(),
    };

    for event in events {
        // The realm's word on each character's Calling, the same in-process or on a server.
        if let SessionEvent::Callings { by_guid } = event {
            keys.0.extend(by_guid);
            label_callings(&keys, &mut labels);
            continue;
        }
        // The Boss Bar's list, whole on every change; an empty one clears it.
        if let SessionEvent::BossFight { guids } = event {
            boss.set_if_neq(BossFightList(guids));
            continue;
        }
        // The world map's quest, service and dungeon marks, whole on every change (spec #383).
        if let SessionEvent::MapMarks { marks: m } = event {
            marks.set_if_neq(MapMarkList(m));
            continue;
        }
        // The Calling Choice popup's options go to its own feed (spec #414).
        if matches!(event, SessionEvent::CallingChoice { .. }) {
            if let Some(inbox) = reel.2.as_deref_mut() {
                inbox.0.push(event);
            }
            continue;
        }
        // Fishing's bite, end and Angler Level go to the Reeling Bar (ADR 0018).
        if matches!(
            event,
            SessionEvent::FishingBite { .. }
                | SessionEvent::FishingEnd { .. }
                | SessionEvent::Angler { .. }
                | SessionEvent::FishingJournal { .. }
        ) {
            if let Some(inbox) = reel.0.as_deref_mut() {
                inbox.0.push(event);
            }
            continue;
        }
        let logged_out = matches!(event, SessionEvent::LoggedOut);
        if logged_out {
            boss.set_if_neq(BossFightList::default());
            marks.set_if_neq(MapMarkList::default());
            if let Some(inbox) = reel.0.as_deref_mut() {
                inbox.0.push(SessionEvent::LoggedOut);
            }
            if let Some(inbox) = reel.2.as_deref_mut() {
                inbox.0.push(SessionEvent::LoggedOut);
            }
        }
        let _ = far.events.send(event);
        // The IO thread ends the session after a logout, which tears the streamed world down.
        if logged_out {
            let _ = far.events.send(SessionEvent::Disconnected {
                reason: "logged out".into(),
                end: SessionEnd::LoggedOut,
            });
        }
    }
}

/// Each character's Calling key, as the realm last said (`SessionEvent::Callings`): its own
/// characters', and every other player's as they come into view.
#[derive(Resource, Default)]
pub(crate) struct CallingKeys(pub(crate) std::collections::HashMap<u64, String>);

impl CallingKeys {
    /// The Calling of the character `guid`, if the realm named one the client patch knows: what
    /// the client reads in place of the unit's class byte (ADR 0011, #207).
    pub(crate) fn of(&self, guid: u64) -> Option<&'static benilla_formats::CallingInfo> {
        benilla_formats::twilight_calling(self.0.get(&guid)?)
    }
}

/// The Boss Bar's fights, oldest first, as the realm last sent them (`SessionEvent::BossFight`).
/// Only the first three show as `boss1` to `boss3`; the rest wait.
#[derive(Resource, Default, Clone, PartialEq)]
pub(crate) struct BossFightList(pub(crate) Vec<u64>);

/// The world map's marks as the realm last sent them (`SessionEvent::MapMarks`).
#[derive(Resource, Default, Clone, PartialEq)]
pub(crate) struct MapMarkList(pub(crate) Vec<benilla_protocol::MapMark>);

/// The local player's Calling, or `None` until the realm names it.
#[derive(Resource, Default, Clone, Copy, PartialEq)]
pub(crate) struct SelfCalling(pub(crate) Option<&'static benilla_formats::CallingInfo>);

impl SelfCalling {
    /// The Calling of `calling`, a system's `Option<Res<SelfCalling>>` (`None` in an app built
    /// without this plugin).
    pub(crate) fn get(
        calling: Option<&SelfCalling>,
    ) -> Option<&'static benilla_formats::CallingInfo> {
        calling.and_then(|c| c.0)
    }
}

/// The self Calling for the pure helpers that read the self descriptor deep in a call (the
/// reaction colours' reputation base); systems read [`SelfCalling`].
static SELF_CALLING: std::sync::RwLock<Option<&'static benilla_formats::CallingInfo>> =
    std::sync::RwLock::new(None);

/// The local player's Calling, as [`track_self_calling`] last saw it.
pub(crate) fn self_calling() -> Option<&'static benilla_formats::CallingInfo> {
    *SELF_CALLING.read().unwrap_or_else(|e| e.into_inner())
}

/// Keeps [`SelfCalling`] (and [`self_calling`]) on the self guid's Calling.
fn track_self_calling(keys: Res<CallingKeys>, me: Res<SelfGuid>, mut current: ResMut<SelfCalling>) {
    let calling = me.0.and_then(|guid| keys.of(guid));
    if current.set_if_neq(SelfCalling(calling)) {
        *SELF_CALLING.write().unwrap_or_else(|e| e.into_inner()) = calling;
    }
}

/// The select screen names each character's Calling (`Level 7 Basic Warrior`), not its class.
fn label_callings(keys: &CallingKeys, labels: &mut ResMut<crate::char_select::ClassLabels>) {
    let named = crate::char_select::ClassLabels(
        keys.0
            .keys()
            .filter_map(|&guid| Some((guid, keys.of(guid)?.name.clone())))
            .collect(),
    );
    labels.set_if_neq(named);
}

/// The client's commands the realm understands; the rest are dropped.
fn realm_command(command: ClientCommand) -> Option<RealmCommand> {
    match command {
        ClientCommand::Move {
            kind,
            flags,
            pos,
            orientation,
            pitch,
            fall_time,
            jump,
            transport,
        } => Some(RealmCommand::Move {
            position: pos,
            orientation,
            motion: twilight_realm::Motion {
                verb: benilla_protocol::messages::RelayVerb::of(kind.opcode()),
                flags,
                time: benilla_protocol::world::client_uptime_ms(),
                pitch,
                fall_time,
                jump,
                transport,
            },
        }),
        ClientCommand::SetSelection { guid } => Some(RealmCommand::SetSelection { guid }),
        ClientCommand::AttackSwing { guid } => Some(RealmCommand::AttackSwing { guid }),
        ClientCommand::AttackStop => Some(RealmCommand::AttackStop),
        ClientCommand::CancelAutoRepeat => Some(RealmCommand::CancelAutoRepeat),
        ClientCommand::GameObjUse { guid } => Some(RealmCommand::UseGameObject { guid }),
        ClientCommand::CreatureQuery { entry, .. } => Some(RealmCommand::CreatureQuery { entry }),
        ClientCommand::CastSpell { spell_id, target } => {
            Some(RealmCommand::CastSpell { spell_id, target })
        }
        ClientCommand::CastSpellAtDest { spell_id, dest } => {
            Some(RealmCommand::CastSpellAtDest { spell_id, dest })
        }
        ClientCommand::CancelAura { spell_id } => Some(RealmCommand::CancelAura { spell_id }),
        ClientCommand::SetActionButton { button, packed } => Some(RealmCommand::SetActionButton {
            slot: button,
            packed,
        }),
        ClientCommand::Logout => Some(RealmCommand::Logout),
        ClientCommand::LogoutCancel => Some(RealmCommand::LogoutCancel),
        ClientCommand::Chat {
            kind, target, text, ..
        } => Some(RealmCommand::Chat {
            chat_type: kind.chat_type(),
            target,
            text,
        }),
        ClientCommand::TextEmote { text_id, target } => {
            Some(RealmCommand::TextEmote { text_id, target })
        }
        ClientCommand::NameQuery { guid } => Some(RealmCommand::NameQuery { guid }),
        ClientCommand::ItemQuery { entry, .. } => Some(RealmCommand::ItemQuery { entry }),
        ClientCommand::PageTextQuery { page_id, .. } => {
            Some(RealmCommand::PageTextQuery { page_id })
        }
        ClientCommand::Loot { guid } => Some(RealmCommand::Loot { guid }),
        ClientCommand::LootMoney => Some(RealmCommand::LootMoney),
        ClientCommand::AutostoreLootItem { slot } => Some(RealmCommand::AutostoreLootItem { slot }),
        ClientCommand::LootRelease { guid } => Some(RealmCommand::LootRelease { guid }),
        ClientCommand::SetAmmo { entry } => Some(RealmCommand::SetAmmo { entry }),
        ClientCommand::AutoEquipItem { bag_index, slot } => Some(RealmCommand::AutoEquipItem {
            bag: bag_index,
            slot,
        }),
        ClientCommand::SwapInvItem { src_slot, dst_slot } => Some(RealmCommand::SwapInvItem {
            src: src_slot,
            dst: dst_slot,
        }),
        ClientCommand::SwapItem {
            dst_bag,
            dst_slot,
            src_bag,
            src_slot,
        } => Some(RealmCommand::SwapItem {
            src_bag,
            src_slot,
            dst_bag,
            dst_slot,
        }),
        ClientCommand::AutoStoreBagItem {
            src_bag,
            src_slot,
            dst_bag,
        } => Some(RealmCommand::AutoStoreBagItem {
            src_bag,
            src_slot,
            dst_bag,
        }),
        ClientCommand::DestroyItem {
            bag_index,
            slot,
            count,
        } => Some(RealmCommand::DestroyItem {
            bag: bag_index,
            slot,
            count,
        }),
        ClientCommand::LearnTalent { talent_id, rank } => {
            Some(RealmCommand::LearnTalent { talent_id, rank })
        }
        ClientCommand::GossipHello { guid } => Some(RealmCommand::GossipHello { guid }),
        ClientCommand::GossipSelectOption { guid, option } => {
            Some(RealmCommand::GossipSelectOption { guid, option })
        }
        ClientCommand::NpcTextQuery { text_id, .. } => Some(RealmCommand::NpcTextQuery { text_id }),
        ClientCommand::TalentWipeConfirm { trainer } => {
            Some(RealmCommand::TalentWipeConfirm { trainer })
        }
        ClientCommand::BinderActivate { binder } => Some(RealmCommand::BinderActivate { binder }),
        ClientCommand::ListInventory { guid } => Some(RealmCommand::ListInventory { guid }),
        ClientCommand::BuyItem {
            vendor,
            entry,
            count,
        } => Some(RealmCommand::BuyItem {
            vendor,
            entry,
            count,
        }),
        // A row dropped on a slot: bought into the first free one, as the realm stores loot.
        ClientCommand::BuyItemInSlot {
            vendor,
            entry,
            count,
            ..
        } => Some(RealmCommand::BuyItem {
            vendor,
            entry,
            count,
        }),
        ClientCommand::SellItem {
            vendor,
            item_guid,
            count,
        } => Some(RealmCommand::SellItem {
            vendor,
            item: item_guid,
            count,
        }),
        ClientCommand::BuybackItem { vendor, slot } => {
            Some(RealmCommand::BuybackItem { vendor, slot })
        }
        ClientCommand::RepairItem { vendor, item_guid } => Some(RealmCommand::RepairItem {
            vendor,
            item: item_guid,
        }),
        ClientCommand::UseItem {
            bag_index, slot, ..
        } => Some(RealmCommand::UseItem {
            bag: bag_index,
            slot,
        }),
        ClientCommand::RepopRequest => Some(RealmCommand::RepopRequest),
        ClientCommand::TeleportAck { counter, .. } => Some(RealmCommand::TeleportAck { counter }),
        ClientCommand::QuestgiverStatusQuery { npc } => {
            Some(RealmCommand::QuestGiverStatusQuery { npc })
        }
        ClientCommand::QuestgiverHello { npc } => Some(RealmCommand::QuestGiverHello { npc }),
        ClientCommand::QuestgiverQuery { npc, quest } => {
            Some(RealmCommand::QuestGiverQueryQuest { npc, quest })
        }
        ClientCommand::QuestgiverAccept { npc, quest } => {
            Some(RealmCommand::QuestGiverAcceptQuest { npc, quest })
        }
        ClientCommand::QuestgiverComplete { npc, quest } => {
            Some(RealmCommand::QuestGiverCompleteQuest { npc, quest })
        }
        ClientCommand::QuestgiverRequestReward { npc, quest } => {
            Some(RealmCommand::QuestGiverRequestReward { npc, quest })
        }
        ClientCommand::QuestgiverChooseReward { npc, quest, choice } => {
            Some(RealmCommand::QuestGiverChooseReward { npc, quest, choice })
        }
        ClientCommand::QuestQuery { quest } => Some(RealmCommand::QuestQuery { quest }),
        ClientCommand::QuestlogRemove { slot } => Some(RealmCommand::QuestLogRemoveQuest { slot }),
        ClientCommand::AreaTrigger { trigger_id } => {
            Some(RealmCommand::AreaTrigger { id: trigger_id })
        }
        ClientCommand::WorldportAck => Some(RealmCommand::WorldportAck),
        ClientCommand::DuelAccepted { arbiter } => Some(RealmCommand::DuelAccept { arbiter }),
        ClientCommand::DuelCancelled { arbiter } => Some(RealmCommand::DuelCancel { arbiter }),
        ClientCommand::InitiateTrade { target } => Some(RealmCommand::InitiateTrade { target }),
        ClientCommand::BeginTrade => Some(RealmCommand::BeginTrade),
        ClientCommand::BusyTrade => Some(RealmCommand::BusyTrade),
        ClientCommand::IgnoreTrade => Some(RealmCommand::IgnoreTrade),
        ClientCommand::AcceptTrade => Some(RealmCommand::AcceptTrade),
        ClientCommand::UnacceptTrade => Some(RealmCommand::UnacceptTrade),
        ClientCommand::CancelTrade => Some(RealmCommand::CancelTrade),
        ClientCommand::SetTradeGold { copper } => Some(RealmCommand::SetTradeGold { copper }),
        ClientCommand::SetTradeItem {
            trade_slot,
            bag,
            slot,
        } => Some(RealmCommand::SetTradeItem {
            trade_slot,
            bag,
            slot,
        }),
        ClientCommand::ClearTradeItem { trade_slot } => {
            Some(RealmCommand::ClearTradeItem { trade_slot })
        }
        _ => None,
    }
}

/// The realm's view of the world, answered from the streamed terrain and its collision.
struct TerrainHost<'a, 'w, 's, 'cw, 'cs> {
    world: &'a WorldPoint<'w, 's>,
    collision: &'a WorldCollision<'cw, 'cs>,
    /// The tile a dev teleport fetched beyond the stream window.
    goto: &'a goto::GotoGround,
    adts: &'a Assets<benilla_assets::AdtTile>,
    /// The streamed liquid surfaces, ADT and WMO alike.
    water: &'a Query<'w, 's, &'static WaterChunkInfo>,
}

/// How much of each end a sight line leaves out, in yards: the two bodies it joins.
const SIGHT_END_YD: f32 = 1.0;

/// How far below a spawn's own height its floor is looked for, in yards.
const GROUND_RAY_YD: f32 = 40.0;

/// A standing body's radius and height for [`Host::body_fits`], in yards (ADR 0021).
const BODY_RADIUS_YD: f32 = 0.4;
const BODY_HEIGHT_YD: f32 = 2.0;

impl Host for TerrainHost<'_, '_, '_, '_, '_> {
    /// The first floor below `z_hint`: a WMO's (a mine's tunnels, a building's storeys) before
    /// the terrain's. Terrain above the hint is either the hill over a tunnel or the ground a
    /// unit was written under. The ray tells them apart: a tunnel has its WMO floor within
    /// [`GROUND_RAY_YD`] below the hint, so the ray hits it and it wins. Only where nothing solid
    /// lies below does the terrain above the hint count, as ground the unit sank under.
    fn ground_height(&self, _map: u32, x: f32, y: f32, z_hint: f32) -> Option<f32> {
        let from = wow_to_bevy([x, y, z_hint]);
        if let Some(hit) = self.collision.ray_body(from, Dir3::NEG_Y, GROUND_RAY_YD) {
            return Some(z_hint - hit.distance);
        }
        self.world
            .terrain_height_under(from)
            .or_else(|| self.goto.height(self.adts, x, y))
    }

    /// A ray through what a body collides with: terrain, doodads and WMO walls (a Dungeon's
    /// tunnels). Where nothing is streamed in, nothing is in the way.
    fn line_of_sight(&self, _map: u32, from: [f32; 3], to: [f32; 3]) -> bool {
        let (a, b) = (wow_to_bevy(from), wow_to_bevy(to));
        let span = b - a;
        let length = span.length() - 2.0 * SIGHT_END_YD;
        let Ok(dir) = Dir3::new(span) else {
            return true;
        };
        length <= 0.0
            || self
                .collision
                .ray_body(a + dir * SIGHT_END_YD, dir, length)
                .is_none()
    }

    /// Whether a standing body at `at` overlaps nothing a body collides with: terrain, doodad
    /// hulls and WMO walls. Where nothing is streamed in, it fits.
    fn body_fits(&self, _map: u32, at: [f32; 3]) -> bool {
        !self
            .collision
            .body_overlaps(wow_to_bevy(at), BODY_RADIUS_YD, BODY_HEIGHT_YD)
    }

    /// The streamed terrain's MCNK `areaId` under the point, the area the client's own zone
    /// text names; where nothing is streamed in (or inside a Dungeon, which has no terrain), none.
    fn area_at(&self, _map: u32, x: f32, y: f32) -> Option<u32> {
        self.world.area_id_under(wow_to_bevy([x, y, 0.0]))
    }

    /// The streamed water under the point, ADT and WMO pools both (the Stormwind canals), the
    /// lowest where they stack. The question has no height and no room, so the claim admits every
    /// surface and the height clears every pool's floor.
    fn liquid_at(&self, _map: u32, x: f32, y: f32) -> Option<f32> {
        water_surface_at(
            self.water.iter(),
            [x, y, f32::INFINITY],
            LiquidClaim::Unknown,
        )
    }
}

/// The spell rewrites of ADR 0011, made once the client has loaded its spell data:
/// - the compiled-in patch inserts every Calling spell as a row of its own;
/// - then a WIP talent's rank spells read "<Name> (WIP)" in the talent frame and its tooltips.
fn rewrite_spells(mut spells: ResMut<crate::ui_action::Spells>) {
    let catalog = &mut spells.catalog;
    if let Err(err) = catalog.apply_dbc_patch(benilla_formats::CLIENT_DBC_PATCH) {
        error!("client_dbc_patch.json does not parse: {err}");
    }
    catalog.mark_wip(twilight_realm::callings::wip_rank_spell_ids());
}

/// Basic Warrior is the only Calling a character starts as, so create offers only Warrior.
fn only_warriors(mut catalog: ResMut<crate::entities::CharCreate>) {
    catalog
        .0
        .retain_classes(|class| class == benilla_protocol::messages::CLASS_WARRIOR);
}

#[cfg(test)]
mod tests {
    use benilla_formats::{area_id_at, load_area_table_catalog, load_tile_mesh, world_to_tile};

    #[test]
    fn a_page_text_query_maps_to_the_realm_command_with_the_same_page_id() {
        use crate::net::ClientCommand;
        use twilight_realm::RealmCommand;
        let mapped = super::realm_command(ClientCommand::PageTextQuery {
            page_id: 15840,
            guid: 7,
        });
        assert!(matches!(
            mapped,
            Some(RealmCommand::PageTextQuery { page_id: 15840 })
        ));
    }

    /// The host's `area_at` reads the streamed tile's MCNK `areaId` through
    /// [`benilla_formats::area_id_at`]: on the game data, it names the Barrens subzones the realm's
    /// discovery and "Know the Land" rest on, at known points. Skipped without the data.
    #[test]
    fn the_terrain_names_the_crossroads_thorn_hill_and_the_lushwater_oasis() {
        let Some(data) = benilla_formats::wow_data() else {
            eprintln!("skipped: no WoW data (set WOW_DATA)");
            return;
        };
        let mut chain = benilla_formats::open_chain(&data).expect("the MPQs open");
        let areas = load_area_table_catalog(&mut chain).expect("AreaTable.dbc");
        let [x, y, ..] = twilight_realm::CROSSROADS;
        for ((x, y), name) in [
            ((x, y), "The Crossroads"),
            ((-183.3, -3183.3), "Thorn Hill"),
            ((-1049.9, -2116.6), "Lushwater Oasis"),
        ] {
            let (tx, ty) = world_to_tile(x, y);
            let tile = load_tile_mesh(&mut chain, "Kalimdor", tx, ty).expect("the tile loads");
            let id = area_id_at(&tile.chunks, [x, y, 0.0]).expect("an area under the point");
            assert_eq!(areas.name(id), Some(name), "({x}, {y}) is area {id}");
        }
    }
}
