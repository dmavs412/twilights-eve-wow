//! "Know the Land" (spec #28): the scout in the Crossroads watchtower asks the player to discover
//! every Barrens subzone. It counts the discoveries made before it was taken, completes on the
//! last, pays 5 Tokens, and is done once per character: Advancement never offers it again. Played
//! on a host with a scripted map holding the last subzone.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::SEED;
use quest_common::*;

/// The watchtower scout and her quest.
const SCOUT_AHANU: u32 = 90112;
const KNOW_THE_LAND: u32 = 93001;

/// The quest's subzones, as `content/quests.toml` lists them, Razorfen Downs last.
const BARRENS: [u32; 25] = [
    359, 378, 379, 380, 381, 382, 383, 384, 385, 386, 387, 388, 390, 391, 392, 1697, 1698, 1699,
    1700, 1701, 1702, 1703, 1704, 1717, 1316,
];
const RAZORFEN_DOWNS: u32 = 1316;
/// Where it lies on the scripted map, far from the Hub.
const AT_RAZORFEN_DOWNS: [f32; 3] = [-4500.0, -2400.0, 0.0];

/// The Warrior trainer, and the Advancement Quest into Vanguard he gives at the Tier Cap.
const UZZEK: u32 = 5810;
const THE_WARRIORS_PATH: u32 = 64001;

#[test]
fn the_watchtower_scout_offers_know_the_land() {
    let (mut realm, _, events) = in_world(1, &[]);
    let scout = giver(&events, SCOUT_AHANU);
    assert!(on_offer(&mut realm, scout).contains(&KNOW_THE_LAND));
}

#[test]
fn the_last_subzone_completes_it_and_the_hand_in_pays_five_tokens() {
    let (mut realm, _, events) = in_world(5, &BARRENS[..24]);
    let scout = giver(&events, SCOUT_AHANU);
    let taken = accept(&mut realm, scout, KNOW_THE_LAND);
    assert!(!objectives_complete(&taken), "one subzone is still fogged");

    let last = step(&mut realm, AT_RAZORFEN_DOWNS);
    assert!(objectives_complete(&last), "{last:?}");

    let done = hand_in(&mut realm, scout, KNOW_THE_LAND);
    let done = completed(&done).expect("handed in");
    assert_eq!(done.items, [(TOKEN, 5)]);
    assert!(!on_offer(&mut realm, scout).contains(&KNOW_THE_LAND));
}

#[test]
fn after_advancement_it_is_not_offered_again() {
    let (mut realm, me, events) = in_world(10, &BARRENS);
    let scout = giver(&events, SCOUT_AHANU);
    accept(&mut realm, scout, KNOW_THE_LAND);
    assert!(completed(&hand_in(&mut realm, scout, KNOW_THE_LAND)).is_some());

    let uzzek = giver(&events, UZZEK);
    realm.handle(RealmCommand::QuestGiverQueryQuest {
        npc: uzzek,
        quest: THE_WARRIORS_PATH,
    });
    accept(&mut realm, uzzek, THE_WARRIORS_PATH);
    let advanced = hand_in(&mut realm, uzzek, THE_WARRIORS_PATH);
    assert_eq!(
        gear_common::descriptor(&advanced, me).unit_level(),
        Some(1),
        "advanced"
    );
    assert!(!on_offer(&mut realm, scout).contains(&KNOW_THE_LAND));
}

// ── Helpers ─────────────────────────────────────────────────────────────────────────────────────

/// A warrior at `level` who has discovered `discovered`, in the world at the Hub; and the entry
/// burst.
fn in_world(level: u8, discovered: &[u32]) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(level, &[], 0, &[]);
    let blob: String = blob
        .lines()
        .filter(|l| !l.starts_with("discovered"))
        .map(|l| format!("{l}\n"))
        .collect();
    let blob = blob.replacen(
        "money = 0\n",
        &format!("money = 0\ndiscovered = {discovered:?}\n"),
        1,
    );
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

/// Moves the player to `to` and ticks once on the scripted map: the events.
fn step(realm: &mut Realm, to: [f32; 3]) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::Move {
        position: to,
        orientation: 0.0,
    });
    let map = Areas(vec![(
        [AT_RAZORFEN_DOWNS[0], AT_RAZORFEN_DOWNS[1]],
        150.0,
        RAZORFEN_DOWNS,
    )]);
    realm.tick(0.0, &map);
    drain(realm)
}

fn objectives_complete(events: &[SessionEvent]) -> bool {
    events.iter().any(|e| {
        matches!(e, SessionEvent::QuestObjectivesComplete { quest_id } if *quest_id == KNOW_THE_LAND)
    })
}
