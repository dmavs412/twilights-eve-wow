//! Stormwind's quests from 30 to 50 (spec #595), walked on the shipped content
//! (`content/quests_stormwind.toml`). Band 30-35 (#604): Commander Wray's two sets of orders, the
//! poster beside the Writ Board, and Magistrate Solomon's request to the city gate. Band 35-40
//! (#609): Darkshire's thirteenth letter to the General and his answer.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

/// `zone_or_sort` of a Stormwind City quest.
const STORMWIND: i32 = 1519;

const ORDERS_FOR_LAKESHIRE: u32 = 97401;
const ORDERS_FOR_DARKSHIRE: u32 = 97402;
const HANDS_WANTED_LAKESHIRE: u32 = 97403;
const HANDS_WANTED_NIGHT_WATCH: u32 = 97404;
const A_MAGISTRATES_REQUEST: u32 = 97405;
const BLACKROCK_MENACE: u32 = 98108;
const DARKSHIRES_THIRTEENTH_LETTER: u32 = 97406;
const AN_ANSWER_FOR_DARKSHIRE: u32 = 97407;
const DARKSHIRE_WAITS: u32 = 98601;

/// Each quest: `(id, level, giver, giver_object, ender, after)`.
const BANDS: [(u32, u32, Option<u32>, Option<u32>, u32, Option<u32>); 7] = [
    (97401, 30, Some(WRAY), None, SOLOMON, None),
    (97402, 30, Some(WRAY), None, ELLO, None),
    (97403, 30, None, Some(POSTER_ENTRY), PARKER, None),
    (97404, 30, None, Some(POSTER_ENTRY), ALTHEA, None),
    (
        97405,
        34,
        Some(SOLOMON),
        None,
        MARCUS,
        Some(BLACKROCK_MENACE),
    ),
    (97406, 37, Some(ELLO), None, MARCUS, Some(DARKSHIRE_WAITS)),
    (
        97407,
        37,
        Some(MARCUS),
        None,
        ELLO,
        Some(DARKSHIRES_THIRTEENTH_LETTER),
    ),
];

/// Each story quest's reward choices.
const CHOICES: [(u32, &[u32]); 7] = [
    (97401, &[903204, 903206, 903208]),
    (97402, &[903214, 903216, 903218]),
    (97403, &[903200, 903201, 903202, 903203]),
    (97404, &[903211, 903212, 903213]),
    (97405, &[903205, 903207, 903210]),
    (97406, &[903224, 903226, 903228]),
    (97407, &[903234, 903236, 903238]),
];

/// The zones' first quests, which the way in must not gate.
const FIRST_QUESTS: [u32; 4] = [98101, 98103, 98601, 98602];

/// Commander Aldous Wray, a Hub NPC in the barracks (`content/hub.toml`).
const WRAY: u32 = 190071;
const WRAY_AT: [f32; 3] = [-8687.0, 328.0, 109.44];
/// General Marcus Jonathan at the city gate (`content/stormwind.toml`).
const MARCUS: u32 = 466;
const MARCUS_AT: [f32; 3] = [-8953.95, 521.019, 96.54];
/// The Redridge and Duskwood spawns that take these quests.
const SOLOMON: u32 = 344;
const SOLOMON_AT: [f32; 3] = [-9218.61, -2221.8, 67.623];
const PARKER: u32 = 464;
const PARKER_AT: [f32; 3] = [-9609.64, -1902.4, 59.886];
const ELLO: u32 = 263;
const ELLO_AT: [f32; 3] = [-10552.2, -1108.79, 31.511];
const ALTHEA: u32 = 264;
const ALTHEA_AT: [f32; 3] = [-10559.5, -1152.25, 28.076];

/// The poster beside the Writ Board.
const POSTER_ENTRY: u32 = 980202;
const POSTER: u64 = 0xF110 << 48 | 791002;
const POSTER_AT: [f32; 3] = [-8823.0, 667.0, 95.43];

const WRAYS_ORDERS: u32 = 980151;
const WRAYS_LETTER: u32 = 980152;
const SOLOMONS_REQUEST: u32 = 980153;
const EBONLOCKES_LETTER: u32 = 980154;
const JONATHANS_REPLY: u32 = 980155;

fn quest_file(text: &str) -> Vec<toml::Value> {
    let file: toml::Table = text.parse().unwrap();
    file["quest"].as_array().unwrap().clone()
}

fn quest_row(id: u32) -> toml::Value {
    [
        include_str!("../content/quests_stormwind.toml"),
        include_str!("../content/quests_redridge.toml"),
        include_str!("../content/quests_duskwood.toml"),
    ]
    .into_iter()
    .flat_map(quest_file)
    .find(|q| entry(q, "id") == Some(id))
    .unwrap_or_else(|| panic!("quest {id}"))
}

fn entry(row: &toml::Value, key: &str) -> Option<u32> {
    row.get(key).and_then(|v| v.as_integer()).map(|v| v as u32)
}

/// What the realm sends a client asking for `quest`.
fn template(realm: &mut Realm, quest: u32) -> Box<benilla_protocol::messages::QuestTemplate> {
    realm.handle(RealmCommand::QuestQuery { quest });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestTemplate(t) if t.quest_id == quest => Some(t),
            _ => None,
        })
        .unwrap_or_else(|| panic!("quest {quest} has no template"))
}

/// A character of `calling` at `level` in the barracks, having handed in `done` and carrying
/// nothing.
fn questing_party(calling: &str, level: u8, done: &[u32]) -> Party {
    let done = done.to_vec();
    let calling = format!("calling = \"{calling}\"");
    let mut party = Party::edited(
        Realm::with_seed(1),
        &[level],
        START_ELWYNN,
        Box::new(NoGround),
        move |_, blob| {
            let mut blob = blob.replace("calling = \"basic_warrior\"", &calling);
            if !done.is_empty() {
                blob = blob.replace("xp = 0\n", &format!("xp = 0\ncompleted = {done:?}\n"));
            }
            blob
        },
    );
    walk(&mut party, WRAY_AT);
    party
}

/// What the client has been sent since its `from`th event.
fn since(party: &Party, from: usize) -> Vec<SessionEvent> {
    party.seen(0)[from..].to_vec()
}

/// Walks to `at` and lets the realm tick; the events that came.
fn walk(party: &mut Party, at: [f32; 3]) -> Vec<SessionEvent> {
    let from = party.seen(0).len();
    party.go(0, at, 0.0);
    party.run(0.5);
    since(party, from)
}

/// Walks up to the creature `npc` standing at `at`; its guid.
fn meet(party: &mut Party, npc: u32, at: [f32; 3]) -> u64 {
    walk(party, [at[0] + 1.5, at[1], at[2]]);
    let (guid, there) = party.unit(0, npc, at);
    walk(party, [there[0] + 1.5, there[1], there[2]]);
    guid
}

/// The quests offered to take in what the client was sent.
fn available(events: Vec<SessionEvent>) -> Vec<u32> {
    events
        .into_iter()
        .flat_map(|e| match e {
            SessionEvent::QuestGreeting(list) => list
                .quests
                .iter()
                .filter(|q| q.icon == benilla_protocol::messages::dialog_status::AVAILABLE)
                .map(|q| q.quest_id)
                .collect(),
            SessionEvent::QuestDetail(d) => vec![d.quest_id],
            _ => Vec::new(),
        })
        .collect()
}

/// The quests `npc` offers to take, as talking to it shows them.
fn offers(party: &mut Party, npc: u64) -> Vec<u32> {
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::QuestGiverHello { npc });
    party.run(0.2);
    available(since(party, from))
}

/// The quests the poster offers, as using it shows them.
fn poster_offers(party: &mut Party) -> Vec<u32> {
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::UseGameObject { guid: POSTER });
    party.run(0.2);
    available(since(party, from))
}

/// Takes `quest` from `npc` where the player stands; the events that came.
fn accept(party: &mut Party, npc: u64, quest: u32) -> Vec<SessionEvent> {
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::QuestGiverAcceptQuest { npc, quest });
    party.run(0.2);
    since(party, from)
}

/// Hands `quest` in at `npc` the stock way; the events that came.
fn hand_in(party: &mut Party, npc: u64, quest: u32) -> Vec<SessionEvent> {
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::QuestGiverCompleteQuest { npc, quest });
    party.cmd(0, RealmCommand::QuestGiverRequestReward { npc, quest });
    party.cmd(
        0,
        RealmCommand::QuestGiverChooseReward {
            npc,
            quest,
            choice: 0,
        },
    );
    party.run(0.2);
    since(party, from)
}

fn completed(events: &[SessionEvent], quest: u32) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestComplete(c) if c.quest_id == quest))
}

/// How many of item `entry` the character's save carries.
fn carries(party: &Party, entry: u32) -> u32 {
    let saved = party.realm.export_character(party.guids[0]).unwrap();
    let rows: toml::Table = saved.parse().unwrap();
    rows.get("items")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter(|i| i["entry"].as_integer() == Some(entry.into()))
        .map(|i| i.get("count").and_then(|c| c.as_integer()).unwrap_or(1) as u32)
        .sum()
}

#[test]
fn every_band_loads_with_its_givers_enders_and_zone() {
    let mut realm = realm();
    for (id, level, giver, giver_object, ender, after) in BANDS {
        let row = quest_row(id);
        assert_eq!(row["kind"].as_str(), Some("story"), "quest {id}'s kind");
        assert_eq!(entry(&row, "level"), Some(level), "quest {id}'s level");
        assert_eq!(entry(&row, "giver"), giver, "quest {id}'s giver");
        assert_eq!(
            entry(&row, "giver_object"),
            giver_object,
            "quest {id}'s giver_object"
        );
        assert_eq!(entry(&row, "ender"), Some(ender), "quest {id}'s ender");
        assert_eq!(entry(&row, "after"), after, "quest {id}'s `after`");
        let t = template(&mut realm, id);
        assert_eq!(Some(t.title.as_str()), row["title"].as_str(), "quest {id}");
        assert_eq!(t.zone_or_sort, STORMWIND, "quest {id}'s zone");
    }
}

#[test]
fn no_zone_waits_on_the_way_in() {
    for id in FIRST_QUESTS {
        let after = entry(&quest_row(id), "after");
        assert!(
            !after.is_some_and(|a| (97400..97500).contains(&a)),
            "quest {id} follows {after:?}"
        );
    }
}

#[test]
fn every_story_quest_offers_the_choices_its_gear_row_names() {
    let mut realm = realm();
    for (id, want) in CHOICES {
        let got: Vec<u32> = template(&mut realm, id)
            .choices
            .iter()
            .map(|c| c.0)
            .filter(|&item| item != 0)
            .collect();
        assert_eq!(got, want, "quest {id}'s choices");
    }
}

#[test]
fn wray_sends_a_tier_2_character_to_both_zones_and_not_a_tier_1() {
    let mut zealot = questing_party("zealot", 30, &[]);
    let wray = meet(&mut zealot, WRAY, WRAY_AT);
    let offered = offers(&mut zealot, wray);
    for quest in [ORDERS_FOR_LAKESHIRE, ORDERS_FOR_DARKSHIRE] {
        assert!(offered.contains(&quest), "{quest} in {offered:?}");
    }

    let mut vanguard = questing_party("vanguard", 30, &[]);
    let wray = meet(&mut vanguard, WRAY, WRAY_AT);
    let offered = offers(&mut vanguard, wray);
    for quest in [ORDERS_FOR_LAKESHIRE, ORDERS_FOR_DARKSHIRE] {
        assert!(!offered.contains(&quest), "{quest} in {offered:?}");
    }
}

#[test]
fn wrays_orders_and_letter_go_in_the_bag_and_their_enders_keep_them() {
    let mut party = questing_party("zealot", 30, &[]);
    let wray = meet(&mut party, WRAY, WRAY_AT);
    accept(&mut party, wray, ORDERS_FOR_LAKESHIRE);
    accept(&mut party, wray, ORDERS_FOR_DARKSHIRE);
    assert_eq!(
        carries(&party, WRAYS_ORDERS),
        1,
        "Wray hands over the orders"
    );
    assert_eq!(
        carries(&party, WRAYS_LETTER),
        1,
        "Wray hands over the letter"
    );

    let solomon = meet(&mut party, SOLOMON, SOLOMON_AT);
    let events = hand_in(&mut party, solomon, ORDERS_FOR_LAKESHIRE);
    assert!(completed(&events, ORDERS_FOR_LAKESHIRE), "{events:?}");
    assert_eq!(carries(&party, WRAYS_ORDERS), 0, "Solomon keeps the orders");

    let ello = meet(&mut party, ELLO, ELLO_AT);
    let events = hand_in(&mut party, ello, ORDERS_FOR_DARKSHIRE);
    assert!(completed(&events, ORDERS_FOR_DARKSHIRE), "{events:?}");
    assert_eq!(
        carries(&party, WRAYS_LETTER),
        0,
        "Ebonlocke keeps the letter"
    );
}

#[test]
fn the_poster_sends_a_tier_2_character_to_both_watches() {
    let mut party = questing_party("zealot", 30, &[]);
    walk(&mut party, [POSTER_AT[0] + 1.5, POSTER_AT[1], POSTER_AT[2]]);
    let offered = poster_offers(&mut party);
    for quest in [HANDS_WANTED_LAKESHIRE, HANDS_WANTED_NIGHT_WATCH] {
        assert!(offered.contains(&quest), "{quest} in {offered:?}");
        accept(&mut party, POSTER, quest);
    }

    let parker = meet(&mut party, PARKER, PARKER_AT);
    let events = hand_in(&mut party, parker, HANDS_WANTED_LAKESHIRE);
    assert!(completed(&events, HANDS_WANTED_LAKESHIRE), "{events:?}");

    let althea = meet(&mut party, ALTHEA, ALTHEA_AT);
    let events = hand_in(&mut party, althea, HANDS_WANTED_NIGHT_WATCH);
    assert!(completed(&events, HANDS_WANTED_NIGHT_WATCH), "{events:?}");
}

#[test]
fn solomon_asks_only_after_the_blackrock_menace() {
    let mut early = questing_party("zealot", 34, &[]);
    let solomon = meet(&mut early, SOLOMON, SOLOMON_AT);
    let offered = offers(&mut early, solomon);
    assert!(!offered.contains(&A_MAGISTRATES_REQUEST), "{offered:?}");

    let mut party = questing_party("zealot", 34, &[BLACKROCK_MENACE]);
    let solomon = meet(&mut party, SOLOMON, SOLOMON_AT);
    let offered = offers(&mut party, solomon);
    assert!(offered.contains(&A_MAGISTRATES_REQUEST), "{offered:?}");
    accept(&mut party, solomon, A_MAGISTRATES_REQUEST);
    assert_eq!(
        carries(&party, SOLOMONS_REQUEST),
        1,
        "Solomon hands it over"
    );

    let marcus = meet(&mut party, MARCUS, MARCUS_AT);
    let events = hand_in(&mut party, marcus, A_MAGISTRATES_REQUEST);
    assert!(completed(&events, A_MAGISTRATES_REQUEST), "{events:?}");
    assert_eq!(carries(&party, SOLOMONS_REQUEST), 0, "the General keeps it");
}

#[test]
fn ebonlocke_writes_only_after_darkshire_waits_and_the_general_answers_his_letter() {
    let mut early = questing_party("zealot", 37, &[]);
    let ello = meet(&mut early, ELLO, ELLO_AT);
    let offered = offers(&mut early, ello);
    assert!(
        !offered.contains(&DARKSHIRES_THIRTEENTH_LETTER),
        "{offered:?}"
    );

    let mut party = questing_party("zealot", 37, &[DARKSHIRE_WAITS]);
    let marcus = meet(&mut party, MARCUS, MARCUS_AT);
    let offered = offers(&mut party, marcus);
    assert!(!offered.contains(&AN_ANSWER_FOR_DARKSHIRE), "{offered:?}");

    let ello = meet(&mut party, ELLO, ELLO_AT);
    let offered = offers(&mut party, ello);
    assert!(
        offered.contains(&DARKSHIRES_THIRTEENTH_LETTER),
        "{offered:?}"
    );
    accept(&mut party, ello, DARKSHIRES_THIRTEENTH_LETTER);
    assert_eq!(
        carries(&party, EBONLOCKES_LETTER),
        1,
        "Ebonlocke hands over his letter"
    );

    let marcus = meet(&mut party, MARCUS, MARCUS_AT);
    let events = hand_in(&mut party, marcus, DARKSHIRES_THIRTEENTH_LETTER);
    assert!(
        completed(&events, DARKSHIRES_THIRTEENTH_LETTER),
        "{events:?}"
    );
    assert_eq!(
        carries(&party, EBONLOCKES_LETTER),
        0,
        "the General keeps the letter"
    );
    let offered = offers(&mut party, marcus);
    assert!(offered.contains(&AN_ANSWER_FOR_DARKSHIRE), "{offered:?}");
    accept(&mut party, marcus, AN_ANSWER_FOR_DARKSHIRE);
    assert_eq!(
        carries(&party, JONATHANS_REPLY),
        1,
        "the General hands over his reply"
    );

    let ello = meet(&mut party, ELLO, ELLO_AT);
    let events = hand_in(&mut party, ello, AN_ANSWER_FOR_DARKSHIRE);
    assert!(completed(&events, AN_ANSWER_FOR_DARKSHIRE), "{events:?}");
    assert_eq!(
        carries(&party, JONATHANS_REPLY),
        0,
        "Ebonlocke keeps the reply"
    );
}
