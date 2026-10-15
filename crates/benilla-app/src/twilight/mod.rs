//! **Twilight's Eve**, the glue half (ADR 0001): the boot flag, and the in-process backend that
//! puts the Local Realm ([`twilight_realm`]) on the far ends of the net layer's channels. Once a
//! frame, before the event drain, it hands the realm what the client asked, ticks it on the main
//! thread, and pushes its events where the IO thread would. The rules live in the realm crate;
//! nothing here decides anything.

use benilla_assets::coords::wow_to_bevy;
use benilla_protocol::{SessionEnd, SessionEvent};
use benilla_world::schedule::WorldStage;
use benilla_world::world_point::WorldPoint;
use std::collections::BTreeSet;
use std::path::PathBuf;

use bevy::prelude::*;
use twilight_realm::{DevMultipliers, Host, Realm, RealmCommand, Storage};

use crate::net::{CharRequest, ClientCommand, FarEnds, InProcessLink};

mod talents;

/// `WOW_TWILIGHT` natively, `?twilight=1` in the browser: boot Twilight's Eve.
pub(crate) fn active() -> bool {
    crate::webenv::var("WOW_TWILIGHT").is_some_and(|v| v != "0")
}

/// The dev multipliers, read once at boot: `?speed=` / `?xp=` in the browser, `WOW_TWILIGHT_SPEED`
/// / `WOW_TWILIGHT_XP` natively. They live only in this run's realm, never in a save.
fn dev_multipliers() -> DevMultipliers {
    let flag = |name: &str| {
        // The browser's short form first; `webenv` maps `WOW_<NAME>` to `?<name>=` there.
        let short = cfg!(target_arch = "wasm32")
            .then(|| crate::webenv::var(&format!("WOW_{name}")))
            .flatten();
        short.or_else(|| crate::webenv::var(&format!("WOW_TWILIGHT_{name}")))
    };
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
        // Fresh dice every session; a test seeds its own.
        let mut realm = Realm::with_seed(rand::random()).with_dev(dev_multipliers());
        let saved = load_saves(&mut realm);
        app.insert_resource(LocalRealm {
            far,
            realm,
            listed: false,
            saved,
        })
        .add_systems(
            Update,
            pump.in_set(WorldStage::Net)
                .before(crate::net::apply_net_updates),
        )
        .add_systems(
            Update,
            only_warriors.run_if(resource_added::<crate::entities::CharCreate>),
        )
        .add_systems(
            Update,
            blank_skill_ranks.run_if(resource_added::<crate::ui_action::Spells>),
        )
        .add_systems(
            Update,
            (
                talents::follow_calling.after(pump),
                talents::unlock_talent_frame,
            ),
        );
        // Closing the window logs the character out, which saves it.
        crate::shutdown::on_app_exit(app, save_on_exit.into_configs());
    }
}

/// The client is going down: log out, and write the save that makes.
fn save_on_exit(mut local: ResMut<LocalRealm>) {
    let LocalRealm { realm, saved, .. } = &mut *local;
    realm.handle(RealmCommand::Logout);
    for op in realm.drain_storage() {
        store(op, saved);
    }
}

/// The realm and the channel ends it serves.
#[derive(Resource)]
struct LocalRealm {
    far: FarEnds,
    realm: Realm,
    /// The first roster went out: there is no login, so the realm answers at once.
    listed: bool,
    /// The guids with a save on disk, which the index file lists.
    saved: BTreeSet<u64>,
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
                Ok(_) => {}
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
fn pump(mut local: ResMut<LocalRealm>, time: Res<Time>, world: WorldPoint) {
    let LocalRealm {
        far,
        realm,
        listed,
        saved,
    } = &mut *local;
    if !*listed {
        *listed = true;
        realm.handle(RealmCommand::ListCharacters);
    }
    while let Ok(pick) = far.pick.try_recv() {
        realm.handle(match pick {
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
            realm.handle(command);
        }
    }

    let host = TerrainHost { world: &world };
    realm.tick(time.delta_secs(), &host);

    for op in realm.drain_storage() {
        store(op, saved);
    }

    for event in realm.drain_events() {
        let logged_out = matches!(event, SessionEvent::LoggedOut);
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

/// The client's commands the realm understands; the rest are dropped.
fn realm_command(command: ClientCommand) -> Option<RealmCommand> {
    match command {
        ClientCommand::Move {
            pos, orientation, ..
        } => Some(RealmCommand::Move {
            position: pos,
            orientation,
        }),
        ClientCommand::SetSelection { guid } => Some(RealmCommand::SetSelection { guid }),
        ClientCommand::AttackSwing { guid } => Some(RealmCommand::AttackSwing { guid }),
        ClientCommand::AttackStop => Some(RealmCommand::AttackStop),
        ClientCommand::CreatureQuery { entry, .. } => Some(RealmCommand::CreatureQuery { entry }),
        ClientCommand::CastSpell { spell_id, target } => {
            Some(RealmCommand::CastSpell { spell_id, target })
        }
        ClientCommand::CancelAura { spell_id } => Some(RealmCommand::CancelAura { spell_id }),
        ClientCommand::SetActionButton { button, packed } => Some(RealmCommand::SetActionButton {
            slot: button,
            packed,
        }),
        ClientCommand::Logout => Some(RealmCommand::Logout),
        ClientCommand::ItemQuery { entry, .. } => Some(RealmCommand::ItemQuery { entry }),
        ClientCommand::Loot { guid } => Some(RealmCommand::Loot { guid }),
        ClientCommand::LootMoney => Some(RealmCommand::LootMoney),
        ClientCommand::AutostoreLootItem { slot } => Some(RealmCommand::AutostoreLootItem { slot }),
        ClientCommand::LootRelease { guid } => Some(RealmCommand::LootRelease { guid }),
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
        _ => None,
    }
}

/// The realm's view of the world, answered from the streamed terrain.
struct TerrainHost<'a, 'w, 's> {
    world: &'a WorldPoint<'w, 's>,
}

impl Host for TerrainHost<'_, '_, '_> {
    fn ground_height(&self, _map: u32, x: f32, y: f32, z_hint: f32) -> Option<f32> {
        self.world.terrain_height_under(wow_to_bevy([x, y, z_hint]))
    }

    // Nothing asks yet: the first reader is mob aggro, which brings the collision query with it.
    fn line_of_sight(&self, _map: u32, _from: [f32; 3], _to: [f32; 3]) -> bool {
        true
    }
}

/// A Calling skill has no ranks: the spellbook, tooltip and learn line show none, even though
/// each borrows a ranked 1.12 spell.
fn blank_skill_ranks(mut spells: ResMut<crate::ui_action::Spells>) {
    spells
        .catalog
        .clear_ranks(twilight_realm::callings::skill_spell_ids());
}

/// Basic Warrior is the only Calling a character starts as, so create offers only Warrior.
fn only_warriors(mut catalog: ResMut<crate::entities::CharCreate>) {
    catalog
        .0
        .retain_classes(|class| class == benilla_protocol::messages::CLASS_WARRIOR);
}
