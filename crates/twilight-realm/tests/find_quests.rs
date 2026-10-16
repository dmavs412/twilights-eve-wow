//! "Find" quests: an explore objective that ticks on walking into its spot, and a quest handed in
//! to a person found out in a Hunting Ground, who offers the next step.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::dialog_status;
use benilla_protocol::SessionEvent;
use common::*;
use quest_common::*;
use twilight_realm::CROSSROADS;

/// Zun'jin's scouting quest, and where its spot lies: Thorn Hill, 30 yd across.
const SCOUTING_THORN_HILL: u32 = 91018;
const THORN_HILL: [f32; 3] = [-200.0, -3325.0, 92.0];
const THORN_HILL_RADIUS: f32 = 30.0;

/// Garosh's find quest, the outrider it sends the player to, where she stands, and the next step
/// she offers.
const THE_KOLKAR_WATCH: u32 = 91019;
const OUTRIDER: u32 = 190010;
const OUTRIDER_AT: [f32; 3] = [-1000.0, -2400.0, 92.0];
const PACK_RUNNERS: u32 = 91020;

/// The Crossroads, where the givers stand.
const TOWN: [f32; 3] = [CROSSROADS[0], CROSSROADS[1], CROSSROADS[2]];

/// The quest's template, as the log asks for it.
fn template(realm: &mut Realm, quest: u32) -> Box<benilla_protocol::messages::QuestTemplate> {
    realm.handle(RealmCommand::QuestQuery { quest });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestTemplate(t) => Some(t),
            _ => None,
        })
        .expect("the quest template")
}

fn objectives_complete(events: &[SessionEvent], quest: u32) -> bool {
    events.iter().any(
        |e| matches!(e, SessionEvent::QuestObjectivesComplete { quest_id } if *quest_id == quest),
    )
}

#[test]
fn walking_into_the_spot_explores_it() {
    let (mut realm, me, mut events) = questing(11, &[], 0, &[]);
    let zunjin = giver(&events, ZUNJIN);
    assert!(on_offer(&mut realm, zunjin).contains(&SCOUTING_THORN_HILL));
    events.extend(accept(&mut realm, zunjin, SCOUTING_THORN_HILL));
    assert_eq!(
        quest_log(&events, me),
        [(0, SCOUTING_THORN_HILL, [0; 4], 0)]
    );
    // The log's exploration line.
    assert_eq!(
        template(&mut realm, SCOUTING_THORN_HILL).end_text,
        "Thorn Hill explored"
    );

    // Just outside the spot: nothing.
    let near = walk_to(
        &mut realm,
        [THORN_HILL[0] + THORN_HILL_RADIUS + 5.0, THORN_HILL[1], 92.0],
    );
    assert!(!objectives_complete(&near, SCOUTING_THORN_HILL));
    events.extend(near);
    assert_eq!(
        quest_log(&events, me),
        [(0, SCOUTING_THORN_HILL, [0; 4], 0)]
    );

    // Inside it: explored, once.
    let inside = walk_to(&mut realm, THORN_HILL);
    assert!(objectives_complete(&inside, SCOUTING_THORN_HILL));
    events.extend(inside);
    assert_eq!(
        quest_log(&events, me),
        [(0, SCOUTING_THORN_HILL, [0; 4], 1)]
    );
    let again = walk_to(&mut realm, [THORN_HILL[0] + 5.0, THORN_HILL[1], 92.0]);
    assert!(!objectives_complete(&again, SCOUTING_THORN_HILL));

    // Back in town, Zun'jin takes it in.
    walk_to(&mut realm, TOWN);
    assert_eq!(status(&mut realm, zunjin), dialog_status::REWARD2);
    let done = completed(&hand_in(&mut realm, zunjin, SCOUTING_THORN_HILL)).expect("handed in");
    assert_eq!(done.items, [(TOKEN, 1)]);
}

#[test]
fn an_explored_spot_stays_explored_through_a_save() {
    let (mut realm, me, _) = questing(11, &[], 0, &[(SCOUTING_THORN_HILL, [0; 4])]);
    walk_to(&mut realm, THORN_HILL);
    let blob = realm.export_character(me).expect("a save");

    let mut back = Realm::with_seed(gear_common::SEED);
    back.import_character(&blob).expect("loads");
    back.handle(RealmCommand::EnterWorld { guid: me });
    back.tick(0.0, &FlatGround);
    let events = drain(&mut back);
    assert_eq!(
        quest_log(&events, me),
        [(0, SCOUTING_THORN_HILL, [0; 4], 1)]
    );
}

#[test]
fn a_find_quest_is_handed_in_to_the_person_found_and_she_offers_the_next_step() {
    let (mut realm, _, events) = questing(14, &[], 0, &[]);
    let garosh = giver(&events, GAROSH);
    assert!(on_offer(&mut realm, garosh).contains(&THE_KOLKAR_WATCH));
    accept(&mut realm, garosh, THE_KOLKAR_WATCH);
    // Garosh does not take it back.
    assert!(completed(&hand_in(&mut realm, garosh, THE_KOLKAR_WATCH)).is_none());

    let there = walk_to(&mut realm, OUTRIDER_AT);
    let outrider = giver(&there, OUTRIDER);
    assert_eq!(status(&mut realm, outrider), dialog_status::REWARD2);
    let done = hand_in(&mut realm, outrider, THE_KOLKAR_WATCH);
    assert_eq!(completed(&done).expect("handed in").items, [(TOKEN, 1)]);
    // Her quest opens at once, as the next in the chain.
    let next = done.iter().find_map(|e| match e {
        SessionEvent::QuestDetail(d) => Some(d.quest_id),
        _ => None,
    });
    assert_eq!(next, Some(PACK_RUNNERS));
    assert_eq!(
        template(&mut realm, THE_KOLKAR_WATCH).next_quest_in_chain,
        PACK_RUNNERS
    );
    assert_eq!(on_offer(&mut realm, outrider), [PACK_RUNNERS]);
    assert_eq!(status(&mut realm, outrider), dialog_status::AVAILABLE);
}

#[test]
fn the_person_found_offers_nothing_before_the_find() {
    let (mut realm, _, _) = questing(15, &[], 0, &[]);
    let there = walk_to(&mut realm, OUTRIDER_AT);
    let outrider = giver(&there, OUTRIDER);
    assert_eq!(status(&mut realm, outrider), dialog_status::NONE);
    assert!(on_offer(&mut realm, outrider).is_empty());
}

#[test]
fn the_giver_lists_no_row_for_a_quest_another_takes_in() {
    let (mut realm, _, events) = questing(14, &[], 0, &[(THE_KOLKAR_WATCH, [0; 4])]);
    let garosh = giver(&events, GAROSH);
    let rows: Vec<u32> = talk(&mut realm, garosh)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestGreeting(list) => {
                Some(list.quests.iter().map(|q| q.quest_id).collect())
            }
            _ => None,
        })
        .expect("his other quests' greeting list");
    assert!(!rows.is_empty());
    assert!(!rows.contains(&THE_KOLKAR_WATCH));
}
