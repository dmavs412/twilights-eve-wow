//! The Writ Board's rules (#573, spec #557): which Writ each pool shows in each two-day window,
//! once per showing for a character, and the save that remembers it. The Writs and their board are
//! test-only, added through `Realm::with_quests`; Marshal McBride on the Abbey steps takes them in.
//! The board itself is the shipped `content/writs.toml`'s: window 0 starts 2026-11-02 at midnight
//! UTC, a window is two days, and Duskwood's (10) draw skips the band Redridge (44) drew.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::*;
use quest_common::*;

const HIGHGUID_UNIT: u64 = 0xF130;
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
/// Marshal McBride on the Abbey steps (Elwynn spawn 79970).
const MCBRIDE: u64 = HIGHGUID_UNIT << 48 | 197 << 24 | 79970;
const MCBRIDE_AT: [f32; 3] = [-8912.0, -140.5, 82.0];
const BOARD: u64 = HIGHGUID_GAMEOBJECT << 48 | 990101;

/// 2026-11-02T00:00Z, in Unix ms: window 0's start.
const ANCHOR_MS: u64 = 1_793_577_600_000;
const MINUTE_MS: u64 = 60_000;
const HOUR_MS: u64 = 60 * MINUTE_MS;
const WINDOW_MS: u64 = 48 * HOUR_MS;

/// Redridge's low, mid and high Writs, then Duskwood's.
const REDRIDGE: [u32; 3] = [99301, 99302, 99303];
const DUSKWOOD: [u32; 3] = [99304, 99305, 99306];

/// One Writ of the test board, offered from Tier 0 so the level-10 pilgrim may take it.
fn writ(id: u32, zone: i32, level: u8, levels: &str) -> String {
    format!(
        r#"
[[quest]]
id = {id}
kind = "writ"
giver_object = 990101
ender = 197
zone_or_sort = {zone}
level = {level}
levels = {levels}
min_tier = 0
title = "Writ {id}"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
"#
    )
}

fn writs() -> String {
    let mut toml = String::from(
        r#"
[[gameobject]]
guid = 990101
entry = 990101
display = 1
map = 0
position = [-8911.0, -139.5, 82.0]
"#,
    );
    for (pool, zone) in [(REDRIDGE, 44), (DUSKWOOD, 10)] {
        for (id, (level, levels)) in pool.into_iter().zip([(33, "1"), (40, "1.5"), (47, "2")]) {
            toml.push_str(&writ(id, zone, level, levels));
        }
    }
    toml
}

/// The Abbey's floor everywhere.
struct AbbeyFloor;

impl Host for AbbeyFloor {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(82.0)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// A level-10 warrior's save, begun in Northshire.
fn pilgrim() -> String {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Pilgrim", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    let me = roster(&mut maker)[0].guid;
    maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
}

/// A realm with the test Writs where the character `blob` saves stands beside McBride and the
/// board, at the start of window 0.
fn entered(blob: &str) -> Realm {
    let mut realm = Realm::with_seed(1).with_quests(&writs());
    let me = realm.import_character(blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    wait(&mut realm, 0.5);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [MCBRIDE_AT[0] + 2.0, MCBRIDE_AT[1], MCBRIDE_AT[2]],
        orientation: std::f32::consts::PI,
    });
    wait(&mut realm, 0.5);
    realm.set_wall_clock(ANCHOR_MS);
    realm
}

fn in_northshire() -> Realm {
    entered(&pilgrim())
}

/// Runs the world for `seconds` in tenth-second ticks.
fn wait(realm: &mut Realm, seconds: f32) {
    for _ in 0..(seconds * 10.0).round() as u32 {
        realm.tick(0.1, &AbbeyFloor);
        drain(realm);
    }
}

/// Sets the wall clock an hour into window `window`.
fn at_window(realm: &mut Realm, window: u64) {
    realm.set_wall_clock(ANCHOR_MS + window * WINDOW_MS + HOUR_MS);
}

/// The quests a click on the board offers, sorted.
fn board(realm: &mut Realm) -> Vec<u32> {
    realm.handle(RealmCommand::UseGameObject { guid: BOARD });
    let mut ids: Vec<u32> = drain(realm)
        .into_iter()
        .flat_map(|e| match e {
            SessionEvent::QuestGreeting(list) => list.quests.iter().map(|q| q.quest_id).collect(),
            SessionEvent::QuestDetail(d) => vec![d.quest_id],
            _ => Vec::new(),
        })
        .collect();
    ids.sort();
    ids
}

fn take(realm: &mut Realm, quest: u32) {
    realm.handle(RealmCommand::QuestGiverAcceptQuest { npc: BOARD, quest });
    drain(realm);
}

fn turn_in(realm: &mut Realm, quest: u32) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::QuestGiverCompleteQuest {
        npc: MCBRIDE,
        quest,
    });
    realm.handle(RealmCommand::QuestGiverRequestReward {
        npc: MCBRIDE,
        quest,
    });
    realm.handle(RealmCommand::QuestGiverChooseReward {
        npc: MCBRIDE,
        quest,
        choice: 0,
    });
    drain(realm)
}

fn in_log(realm: &mut Realm, quest: u32) -> bool {
    let me = roster(realm)[0].guid;
    let blob = realm.export_character(me).unwrap();
    blob.contains(&format!("id = {quest}\n"))
}

/// The pool's band of `writ`: 0 low, 1 mid, 2 high.
fn band(writ: u32) -> usize {
    REDRIDGE
        .iter()
        .chain(&DUSKWOOD)
        .position(|&w| w == writ)
        .unwrap()
        % 3
}

/// The board's two Writs in `window`: Redridge's, then Duskwood's.
fn showing(realm: &mut Realm, window: u64) -> (u32, u32) {
    at_window(realm, window);
    let offered = board(realm);
    let red: Vec<u32> = offered
        .iter()
        .copied()
        .filter(|w| REDRIDGE.contains(w))
        .collect();
    let dusk: Vec<u32> = offered
        .iter()
        .copied()
        .filter(|w| DUSKWOOD.contains(w))
        .collect();
    assert_eq!(
        (red.len(), dusk.len(), offered.len()),
        (1, 1, 2),
        "window {window} offers {offered:?}"
    );
    (red[0], dusk[0])
}

#[test]
fn a_window_is_two_days_from_the_anchors_midnight_and_window_0_before_it() {
    let mut realm = in_northshire();
    realm.set_wall_clock(ANCHOR_MS);
    let first = board(&mut realm);
    realm.set_wall_clock(ANCHOR_MS + 47 * HOUR_MS + 59 * MINUTE_MS);
    assert_eq!(board(&mut realm), first, "47 h 59 min in is still window 0");
    realm.set_wall_clock(ANCHOR_MS - 24 * HOUR_MS);
    assert_eq!(
        board(&mut realm),
        first,
        "a day before the anchor is window 0"
    );
    realm.set_wall_clock(ANCHOR_MS + 48 * HOUR_MS);
    let second = board(&mut realm);
    assert!(
        second.iter().all(|w| !first.contains(w)),
        "48 h in is window 1, where neither pool repeats: {first:?} then {second:?}"
    );
}

#[test]
fn no_pool_repeats_a_writ_and_the_two_pools_never_share_a_band() {
    let mut realm = in_northshire();
    let mut last = showing(&mut realm, 0);
    assert_ne!(band(last.0), band(last.1), "window 0");
    for window in 1..200 {
        let now = showing(&mut realm, window);
        assert_ne!(now.0, last.0, "Redridge repeats in window {window}");
        assert_ne!(now.1, last.1, "Duskwood repeats in window {window}");
        assert_ne!(
            band(now.0),
            band(now.1),
            "one band twice in window {window}"
        );
        last = now;
    }
}

#[test]
fn clicking_the_board_offers_exactly_the_two_showing_writs() {
    let mut realm = in_northshire();
    let (red, dusk) = showing(&mut realm, 3);
    let mut both = vec![red, dusk];
    both.sort();
    assert_eq!(board(&mut realm), both);
}

/// The first window after `after` whose board shows `writ`.
fn next_showing(realm: &mut Realm, writ: u32, after: u64) -> u64 {
    (after + 1..after + 100)
        .find(|&w| {
            let (red, dusk) = showing(realm, w);
            red == writ || dusk == writ
        })
        .expect("the Writ shows again")
}

#[test]
fn a_handed_in_writ_is_not_offered_again_until_a_later_window_shows_it() {
    let mut realm = in_northshire();
    let (red, _) = showing(&mut realm, 0);
    take(&mut realm, red);
    assert!(completed(&turn_in(&mut realm, red)).is_some());
    assert!(
        !board(&mut realm).contains(&red),
        "handed in this window, so not offered"
    );

    let later = next_showing(&mut realm, red, 0);
    at_window(&mut realm, later);
    assert!(board(&mut realm).contains(&red), "window {later} shows it");
    take(&mut realm, red);
    assert!(completed(&turn_in(&mut realm, red)).is_some());
}

#[test]
fn a_writ_taken_stays_in_the_log_when_the_board_changes_and_the_ender_takes_it() {
    let mut realm = in_northshire();
    let (_, dusk) = showing(&mut realm, 0);
    take(&mut realm, dusk);
    assert!(in_log(&mut realm, dusk));

    // Window 1 never shows the Writ window 0 showed.
    at_window(&mut realm, 1);
    assert!(!board(&mut realm).contains(&dusk));
    assert!(in_log(&mut realm, dusk), "still in the log in window 1");
    assert!(completed(&turn_in(&mut realm, dusk)).is_some());
}

#[test]
fn a_save_keeps_the_window_of_each_writs_last_hand_in() {
    let mut realm = in_northshire();
    let (red, _) = showing(&mut realm, 0);
    take(&mut realm, red);
    assert!(completed(&turn_in(&mut realm, red)).is_some());
    let me = roster(&mut realm)[0].guid;
    let blob = realm.export_character(me).unwrap();
    assert!(blob.contains("writs_done"), "{blob}");

    let mut back = entered(&blob);
    at_window(&mut back, 0);
    assert!(
        !board(&mut back).contains(&red),
        "the save remembers the hand-in this window"
    );
    let later = next_showing(&mut back, red, 0);
    at_window(&mut back, later);
    assert!(board(&mut back).contains(&red));
}

/// A Writ of the test board but for the lines `extra` adds and the ones `without` names.
fn bad_writ(id: u32, extra: &str, without: &str) -> String {
    let lines: String = writ(id, 44, 33, "1")
        .lines()
        .filter(|l| without.is_empty() || !l.starts_with(without))
        .map(|l| format!("{l}\n"))
        .collect();
    format!("{}{lines}{extra}", writs())
}

#[test]
#[should_panic(expected = "`levels` from 1 to 2")]
fn a_writ_without_levels_fails_validation() {
    let _ = Realm::new().with_quests(&bad_writ(99391, "", "levels"));
}

#[test]
#[should_panic(expected = "`levels` from 1 to 2")]
fn a_writ_with_levels_above_2_fails_validation() {
    let _ = Realm::new().with_quests(
        &bad_writ(99392, "", "levels").replace("id = 99392\n", "id = 99392\nlevels = 2.5\n"),
    );
}

#[test]
#[should_panic(expected = "needs a `giver_object`")]
fn a_writ_without_a_giver_object_fails_validation() {
    let _ = Realm::new().with_quests(&bad_writ(99393, "giver = 197\n", "giver_object"));
}

#[test]
#[should_panic(expected = "only a Writ names `levels`")]
fn a_hunting_quest_with_levels_fails_validation() {
    let _ = Realm::new().with_quests(
        r#"
[[quest]]
id = 99394
kind = "hunting"
giver = 197
level = 5
levels = 1
title = "Not a Writ"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
"#,
    );
}
