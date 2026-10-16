//! Helpers for the quest scripts: the Hub's givers, their dialogs, the quest log, and saves with
//! quests under way.
#![allow(dead_code)]

use benilla_protocol::messages::dialog_status;
use benilla_protocol::{EntityKind, SessionEvent};
use twilight_realm::{Realm, RealmCommand};

use crate::common::{drain, hub_guid, walk_up_to, FlatGround};
use crate::gear_common::{descriptor, save_with, Carried, SEED};

/// `UNIT_NPC_FLAG_QUESTGIVER`.
pub const NPC_FLAG_QUESTGIVER: u32 = 0x2;

/// The Near Plains' giver, The Far Fields', The Southfury Banks', The Kolkar Lands' and The
/// Southern Barrens'.
pub const GREZZ: u32 = 90101;
pub const TAMAHA: u32 = 90102;
pub const ZUNJIN: u32 = 90103;
pub const GAROSH: u32 = 90104;
pub const MAHU: u32 = 90105;

/// The Near Plains' kill and collect quests (levels 2 and 3) and familiars
/// (level 4), and The Southfury Banks' crocolisk skins (level 10).
pub const THINNING_THE_HERDS: u32 = 91001;
pub const SCORPID_VENOM: u32 = 91002;
pub const THE_FAMILIAR_THREAT: u32 = 91003;
pub const CROCOLISK_SKINS: u32 = 91006;

/// The quest items: the Scorpid Workers' venom sacs and the crocolisks' skins.
pub const VENOM_SAC: u32 = 4886;
pub const CROCOLISK_SKIN: u32 = 2925;

/// The Token's item entry, as the dialogs show it.
pub const TOKEN: u32 = 90001;

/// Every quest giver created in `events`: `(guid, entry, position)`.
pub fn givers(events: &[SessionEvent]) -> Vec<(u64, u32, [f32; 3])> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } if fields.unit_npc_flags() & NPC_FLAG_QUESTGIVER != 0 => {
                Some((*guid, fields.object_entry()?, *position))
            }
            _ => None,
        })
        .collect()
}

/// The giver `entry`'s guid: as created in `events`, or, standing out of view across the town,
/// as `content/hub.toml` places it.
pub fn giver(events: &[SessionEvent], entry: u32) -> u64 {
    givers(events)
        .into_iter()
        .find(|g| g.1 == entry)
        .map(|g| g.0)
        .unwrap_or_else(|| hub_guid(entry))
}

pub fn status(realm: &mut Realm, npc: u64) -> u32 {
    realm.handle(RealmCommand::QuestGiverStatusQuery { npc });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestGiverStatus { npc: n, status } if n == npc => Some(status),
            _ => None,
        })
        .expect("a status answer")
}

/// The player's quest log as the client reads it: `(slot, quest, counters, state)`.
pub fn quest_log(events: &[SessionEvent], me: u64) -> Vec<(u8, u32, [u8; 4], u8)> {
    let d = descriptor(events, me);
    (0..20)
        .filter_map(|slot| {
            let s = d.player_quest_log(slot)?;
            (s.quest_id != 0).then_some((slot, s.quest_id, s.counters, s.state))
        })
        .collect()
}

/// The player walks to `to` on flat ground, returning what the step brought.
pub fn walk_to(realm: &mut Realm, to: [f32; 3]) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::Move {
        position: to,
        orientation: 0.0,
    });
    realm.tick(0.0, &FlatGround);
    drain(realm)
}

/// Walks up to `npc` and says hello.
pub fn talk(realm: &mut Realm, npc: u64) -> Vec<SessionEvent> {
    walk_up_to(realm, npc);
    realm.handle(RealmCommand::QuestGiverHello { npc });
    drain(realm)
}

/// The quests `npc` offers the player to take, as talking to it shows them: the greeting list's
/// rows with the yellow `!`, or the lone quest whose accept panel opens.
pub fn on_offer(realm: &mut Realm, npc: u64) -> Vec<u32> {
    talk(realm, npc)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestGreeting(list) => Some(
                list.quests
                    .iter()
                    .filter(|q| q.icon == dialog_status::AVAILABLE)
                    .map(|q| q.quest_id)
                    .collect(),
            ),
            SessionEvent::QuestDetail(d) => Some(vec![d.quest_id]),
            _ => None,
        })
        .unwrap_or_default()
}

/// Takes `quest` from `npc`, returning the events.
pub fn accept(realm: &mut Realm, npc: u64, quest: u32) -> Vec<SessionEvent> {
    walk_up_to(realm, npc);
    realm.handle(RealmCommand::QuestGiverAcceptQuest { npc, quest });
    drain(realm)
}

/// Hands `quest` in at `npc` the stock way (complete, continue, choose), returning the events.
pub fn hand_in(realm: &mut Realm, npc: u64, quest: u32) -> Vec<SessionEvent> {
    walk_up_to(realm, npc);
    realm.handle(RealmCommand::QuestGiverCompleteQuest { npc, quest });
    realm.handle(RealmCommand::QuestGiverRequestReward { npc, quest });
    realm.handle(RealmCommand::QuestGiverChooseReward {
        npc,
        quest,
        choice: 0,
    });
    drain(realm)
}

/// The turn-in's `SMSG_QUESTGIVER_QUEST_COMPLETE`, if it came.
pub fn completed(events: &[SessionEvent]) -> Option<benilla_protocol::messages::QuestComplete> {
    events.iter().find_map(|e| match e {
        SessionEvent::QuestComplete(c) => Some(c.clone()),
        _ => None,
    })
}

/// A quest under way in a save: `(quest, kills by objective)`, in log slots from 0.
pub type Underway = (u32, [u8; 4]);

/// A warrior's save at `level` carrying `items`, `tokens` (a stack in the last backpack slot) and
/// these quests.
pub fn save_with_quests(
    level: u8,
    items: &[Carried],
    tokens: u32,
    quests: &[Underway],
) -> (u64, String) {
    let mut items = items.to_vec();
    if tokens > 0 {
        items.push((38, TOKEN, tokens));
    }
    let (guid, mut blob) = save_with(level, 0, &items);
    for (slot, (id, kills)) in quests.iter().enumerate() {
        blob.push_str(&format!(
            "\n[[quests]]\nid = {id}\nslot = {slot}\nkills = {kills:?}\n"
        ));
    }
    (guid, blob)
}

/// That warrior in the world at the Hub, with the entry burst.
pub fn questing(
    level: u8,
    items: &[Carried],
    tokens: u32,
    quests: &[Underway],
) -> (Realm, u64, Vec<SessionEvent>) {
    questing_as("basic_warrior", level, items, tokens, quests)
}

/// [`questing`] as a character of another Calling (`warrior` for Tier 1, past level 10).
pub fn questing_as(
    calling: &str,
    level: u8,
    items: &[Carried],
    tokens: u32,
    quests: &[Underway],
) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(level, items, tokens, quests);
    let blob = blob.replace(
        "calling = \"basic_warrior\"",
        &format!("calling = \"{calling}\""),
    );
    let mut realm = Realm::with_seed(SEED);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}
