//! Repeatable Quests: the Hub's givers, accepting, progress, hand-in and retake, through the stock
//! quest dialogs and the `PLAYER_QUEST_LOG` slots.

mod common;
mod gear_common;

use benilla_protocol::messages::dialog_status;
use benilla_protocol::{EntityKind, SessionEvent};
use common::*;
use gear_common::*;
use twilight_realm::CROSSROADS;

/// `UNIT_NPC_FLAG_QUESTGIVER`.
const NPC_FLAG_QUESTGIVER: u32 = 0x2;

/// The Near Plains' giver and The Southern Barrens'.
const GREZZ: u32 = 90101;
const MAHU: u32 = 90105;

/// Every quest giver created in `events`: `(guid, entry, position)`.
fn givers(events: &[SessionEvent]) -> Vec<(u64, u32, [f32; 3])> {
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

fn giver(events: &[SessionEvent], entry: u32) -> u64 {
    givers(events)
        .into_iter()
        .find(|g| g.1 == entry)
        .unwrap_or_else(|| panic!("giver {entry} stands at the Hub"))
        .0
}

fn status(realm: &mut Realm, npc: u64) -> u32 {
    realm.handle(RealmCommand::QuestGiverStatusQuery { npc });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestGiverStatus { npc: n, status } if n == npc => Some(status),
            _ => None,
        })
        .expect("a status answer")
}

#[test]
fn quest_givers_stand_at_the_hub_with_names_and_marks() {
    let (mut realm, _, events) = warrior_with(1, 0, &[]);
    let all = givers(&events);
    assert_eq!(all.len(), 6, "one giver per Hunting Ground");
    for &(_, _, at) in &all {
        let d = ((at[0] - CROSSROADS[0]).powi(2) + (at[1] - CROSSROADS[1]).powi(2)).sqrt();
        assert!(d < 15.0, "a giver stands {d} yd from the Hub");
    }

    realm.handle(RealmCommand::CreatureQuery { entry: GREZZ });
    let name = drain(&mut realm).into_iter().find_map(|e| match e {
        SessionEvent::CreatureName { entry, name, .. } if entry == GREZZ => name,
        _ => None,
    });
    assert_eq!(name.as_deref(), Some("Grezz Plainswatcher"));

    // A new warrior sees a yellow `!` over the plains giver, and a grey one far south.
    assert_eq!(
        status(&mut realm, giver(&events, GREZZ)),
        dialog_status::AVAILABLE
    );
    assert_eq!(
        status(&mut realm, giver(&events, MAHU)),
        dialog_status::UNAVAILABLE
    );
}

/// The Near Plains' kill and collect quests.
const THINNING_THE_HERDS: u32 = 91001;
const SCORPID_VENOM: u32 = 91002;

/// The player's quest log as the client reads it: `(slot, quest, counters, state)`.
fn quest_log(events: &[SessionEvent], me: u64) -> Vec<(u8, u32, [u8; 4], u8)> {
    let d = descriptor(events, me);
    (0..20)
        .filter_map(|slot| {
            let s = d.player_quest_log(slot)?;
            (s.quest_id != 0).then_some((slot, s.quest_id, s.counters, s.state))
        })
        .collect()
}

fn talk(realm: &mut Realm, npc: u64) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::QuestGiverHello { npc });
    drain(realm)
}

/// Takes `quest` from `npc`, returning the events.
fn accept(realm: &mut Realm, npc: u64, quest: u32) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::QuestGiverAcceptQuest { npc, quest });
    drain(realm)
}

#[test]
fn a_giver_lists_its_quests_and_the_accepted_one_goes_in_the_log() {
    let (mut realm, me, mut events) = warrior_with(1, 0, &[]);
    let grezz = giver(&events, GREZZ);

    // Two quests on offer: the greeting list, both under "Available".
    let greeting = talk(&mut realm, grezz)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestGreeting(list) => Some(list),
            _ => None,
        })
        .expect("the greeting list");
    assert_eq!(greeting.npc, grezz);
    let rows: Vec<(u32, u32)> = greeting
        .quests
        .iter()
        .map(|q| (q.quest_id, q.level))
        .collect();
    assert_eq!(rows, [(THINNING_THE_HERDS, 2), (SCORPID_VENOM, 3)]);
    assert!(greeting
        .quests
        .iter()
        .all(|q| q.icon == dialog_status::AVAILABLE));

    // Picking one opens its accept panel.
    realm.handle(RealmCommand::QuestGiverQueryQuest {
        npc: grezz,
        quest: THINNING_THE_HERDS,
    });
    let detail = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestDetail(d) => Some(d),
            _ => None,
        })
        .expect("the accept panel");
    assert_eq!(detail.title, "Thinning the Herds");
    assert_eq!(detail.money, 60);

    // Accepting closes the dialog and fills the first log slot.
    let accepted = accept(&mut realm, grezz, THINNING_THE_HERDS);
    assert!(accepted
        .iter()
        .any(|e| matches!(e, SessionEvent::GossipComplete)));
    events.extend(accepted);
    assert_eq!(quest_log(&events, me), [(0, THINNING_THE_HERDS, [0; 4], 0)]);
    // A yellow `!` for the other quest outranks the grey `?` of the one under way.
    assert_eq!(status(&mut realm, grezz), dialog_status::AVAILABLE);
    accept(&mut realm, grezz, SCORPID_VENOM);
    assert_eq!(status(&mut realm, grezz), dialog_status::INCOMPLETE);

    // The log asks for the template: kill five of each, in The Barrens.
    realm.handle(RealmCommand::QuestQuery {
        quest: THINNING_THE_HERDS,
    });
    let t = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestTemplate(t) => Some(t),
            _ => None,
        })
        .expect("the quest template");
    assert_eq!(
        (t.quest_id, t.level, t.zone_or_sort),
        (THINNING_THE_HERDS, 2, 17)
    );
    let kills: Vec<(u32, u32)> = t
        .objectives
        .iter()
        .map(|o| (o.creature_or_go, o.required_count))
        .collect();
    assert_eq!(kills, [(2955, 5), (3098, 5), (0, 0), (0, 0)]);
}

#[test]
fn a_giver_far_away_does_not_talk() {
    let (mut realm, _, events) = warrior_with(1, 0, &[]);
    let grezz = giver(&events, GREZZ);
    realm.handle(RealmCommand::Move {
        position: [CROSSROADS[0] + 40.0, CROSSROADS[1], 0.0],
        orientation: 0.0,
    });
    assert!(talk(&mut realm, grezz).is_empty());
    assert!(accept(&mut realm, grezz, THINNING_THE_HERDS).is_empty());
}
