//! Redridge's quests from 30 to 50 (spec #595), walked on the shipped content
//! (`content/quests_redridge.toml`). Band 30-35 (#602): Three Corners, Lakeshire's three problems,
//! the bridge, the lakebed, the Thresher and the tower on the hill, and four bounties. Band 35-40
//! (#607): Toren in Lakeshire's inn, the tower's hill, the ringing stones, the Rethban diggers,
//! the north farms' gnolls, and four bounties. Band 40-45 (#612): the Blackrock badges, Render's
//! Rock, Kazon and Tharil'zun's orders, the Ranthal farm, and four bounties.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;
use twilight_realm::Audience;

/// `zone_or_sort` of a Redridge quest.
const REDRIDGE: i32 = 44;

const GNOLLS_AT_THREE_CORNERS: u32 = 98101;
const REPORT_TO_LAKESHIRE: u32 = 98102;
const WORSE_PROBLEMS: u32 = 98103;
const BLACKROCK_MENACE: u32 = 98108;
const THE_TOWER_ON_THE_HILL: u32 = 98109;

/// Band 30-35's quests: `(id, giver, ender)`, the ender when it is not the giver.
const BAND_30_35: [(u32, u32, Option<u32>); 13] = [
    (98101, PARKER, None),
    (98102, PARKER, Some(FELDON)),
    (98103, SOLOMON, None),
    (98104, OSLOW, None),
    (98105, OSLOW, None),
    (98106, BAREN, None),
    (98107, BAREN, None),
    (98108, MARRIS, None),
    (98109, MARRIS, None),
    (98111, PARKER, None),
    (98112, 343, None),
    (98113, 415, None),
    (98114, FELDON, None),
];

/// `(quest, the quest it follows)`; every other quest of the band has no `after`.
const CHAINED: [(u32, u32); 7] = [
    (98102, 98101),
    (98104, 98103),
    (98105, 98104),
    (98106, 98103),
    (98107, 98106),
    (98108, 98103),
    (98109, 98108),
];

/// Each story quest's reward choices; the bounties offer none.
const CHOICES: [(u32, &[u32]); 9] = [
    (98101, &[903215, 903217, 903219]),
    (98102, &[903204, 903206, 903209]),
    (98103, &[903214, 903216, 903218]),
    (98104, &[903204, 903206, 903208]),
    (98105, &[903215, 903217, 903219]),
    (98106, &[903200, 903201, 903202, 903203]),
    (98107, &[903300, 903301, 903302]),
    (98108, &[903211, 903212, 903213]),
    (98109, &[903205, 903207, 903210]),
];

/// The vanilla Redridge spawns that give and take these quests (`content/redridge.toml`): entry
/// and where each stands.
const PARKER: u32 = 464;
const PARKER_AT: [f32; 3] = [-9609.64, -1902.4, 59.886];
const FELDON: u32 = 1070;
const FELDON_AT: [f32; 3] = [-9443.69, -2238.0, 70.69];
const SOLOMON: u32 = 344;
const SOLOMON_AT: [f32; 3] = [-9218.61, -2221.8, 67.623];
const MARRIS: u32 = 382;
const MARRIS_AT: [f32; 3] = [-9284.03, -2298.16, 67.577];
const OSLOW: u32 = 341;
const OSLOW_AT: [f32; 3] = [-9279.22, -2268.53, 67.631];
const BAREN: u32 = 381;
const BAREN_AT: [f32; 3] = [-9261.03, -2172.66, 64.203];

/// The Tower of Ilgalar's door (`content/ilgalar.toml`'s portal).
const TOWER_DOOR: [f32; 3] = [-9282.2, -3327.6, 0.0];

const PARKERS_REPORT: u32 = 980301;

const TOREN_AT_THE_INN: u32 = 98201;

/// Band 35-40's quests: `(id, giver, ender)`, the ender when it is not the giver.
const BAND_35_40: [(u32, u32, Option<u32>); 10] = [
    (98201, BRIANNA, Some(TOREN)),
    (98202, MARRIS, None),
    (98203, MARRIS, None),
    (98204, OSLOW, None),
    (98205, CONACHER, None),
    (98206, CONACHER, None),
    (98211, 903, None),
    (98212, 415, None),
    (98213, 342, None),
    (98214, FELDON, None),
];

/// `(quest, the quest it follows)`; every other quest of band 35-40 has no `after`.
const CHAINED_35_40: [(u32, u32); 4] = [
    (98202, 98109),
    (98203, 98108),
    (98204, 98105),
    (98206, 98205),
];

/// Each band 35-40 story quest's reward choices (#605's rows); the bounties offer none.
const CHOICES_35_40: [(u32, &[u32]); 6] = [
    (98201, &[903220, 903221, 903222, 903223]),
    (98202, &[903231, 903232, 903233]),
    (98203, &[903225, 903227, 903230]),
    (98204, &[903235, 903237, 903239]),
    (98205, &[903224, 903226, 903229]),
    (98206, &[903234, 903236, 903238]),
];

const BRIANNA: u32 = 6727;
const BRIANNA_AT: [f32; 3] = [-9223.98, -2157.12, 64.017];
const CONACHER: u32 = 900;
/// Toren in Lakeshire's inn (`hub.toml` 9300083), on the inn's ground floor.
const TOREN: u32 = 190083;
const TOREN_AT: [f32; 3] = [-9228.0, -2162.0, 63.73];

/// Ilgalar's three items, defined for the dungeon ticket's quests.
const ILGALAR_ITEMS: [u32; 3] = [980381, 980382, 980383];
const LETTERS_UNDER_A_CROWNED_SEAL: u32 = 980382;

const INTO_ILGALAR: u32 = 98202;
const RENDERS_ROCK: u32 = 98302;
const KAZONS_CAMP: u32 = 98303;
const THARILZUN: u32 = 98304;

/// Band 40-45's quests: `(id, giver)`; each giver takes its own quest in.
const BAND_40_45: [(u32, u32); 9] = [
    (98301, 903),
    (98302, MARRIS),
    (98303, MARRIS),
    (98304, MARRIS),
    (98305, CLYDE_RANTHAL),
    (98311, 903),
    (98312, FELDON),
    (98313, CLYDE_RANTHAL),
    (98314, 6295),
];

/// `(quest, the quest it follows)`; every other quest of band 40-45 has no `after`.
const CHAINED_40_45: [(u32, u32); 3] = [(98302, 98202), (98303, 98302), (98304, 98303)];

/// Each band 40-45 story quest's reward choices (#610's rows); the bounties offer none.
const CHOICES_40_45: [(u32, &[u32]); 5] = [
    (98301, &[903251, 903252, 903253]),
    (98302, &[903245, 903247, 903250]),
    (98303, &[903255, 903257, 903259]),
    (98304, &[903306, 903307, 903308, 903309]),
    (98305, &[903244, 903246, 903249]),
];

const CLYDE_RANTHAL: u32 = 2697;
const BLACKROCK_TRACKER: u32 = 615;
const THARILZUN_ENTRY: u32 = 486;
const THARILZUNS_ORDERS: u32 = 980322;

/// Render's Rock, the spot 98302 sends the player to, on the ground the test host gives (none).
const RENDERS_ROCK_AT: [f32; 3] = [-8667.0, -2283.0, 0.0];
/// Tharil'zun's spawn in Stonewatch's yard (`content/redridge.toml`).
const THARILZUN_AT: [f32; 3] = [-9439.3, -3080.77, 0.0];

fn redridge_quests() -> toml::Table {
    include_str!("../content/quests_redridge.toml")
        .parse()
        .unwrap()
}

fn quest_row(id: u32) -> toml::Value {
    redridge_quests()["quest"]
        .as_array()
        .unwrap()
        .iter()
        .find(|q| q["id"].as_integer() == Some(id.into()))
        .unwrap_or_else(|| panic!("quest {id}"))
        .clone()
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
            SessionEvent::QuestTemplate(t) => Some(t),
            _ => None,
        })
        .unwrap_or_else(|| panic!("quest {quest} has no template"))
}

/// A Zealot (Tier 2, so the level-30 quests are offered) at `level` in Lakeshire, having handed
/// in `done` and carrying nothing.
fn questing_party(level: u8, done: &[u32]) -> Party {
    let done = done.to_vec();
    let mut party = Party::edited(
        Realm::with_seed(1),
        &[level],
        START_ELWYNN,
        Box::new(NoGround),
        move |_, blob| {
            let mut blob = blob.replace("calling = \"basic_warrior\"", "calling = \"zealot\"");
            if !done.is_empty() {
                blob = blob.replace("xp = 0\n", &format!("xp = 0\ncompleted = {done:?}\n"));
            }
            blob
        },
    );
    party.go(0, SOLOMON_AT, 0.0);
    party.run(0.5);
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

/// Walks up to the spawn of `npc` standing at `at`; its guid.
fn meet(party: &mut Party, npc: u32, at: [f32; 3]) -> u64 {
    walk(party, [at[0] + 1.5, at[1], at[2]]);
    let (guid, there) = party.unit(0, npc, at);
    walk(party, [there[0] + 1.5, there[1], there[2]]);
    guid
}

fn offered(party: &mut Party, npc: u64, quest: u32) -> bool {
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::QuestGiverHello { npc });
    party.run(0.2);
    since(party, from).iter().any(|e| match e {
        SessionEvent::QuestDetail(d) => d.quest_id == quest,
        SessionEvent::QuestGreeting(list) => list.quests.iter().any(|q| q.quest_id == quest),
        _ => false,
    })
}

fn accept(party: &mut Party, npc: u64, quest: u32) -> Vec<SessionEvent> {
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::QuestGiverAcceptQuest { npc, quest });
    party.run(0.2);
    since(party, from)
}

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

/// Asks `npc` the gossip option `ask`; the events that came.
fn ask(party: &mut Party, npc: u64, ask: &str) -> Vec<SessionEvent> {
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::GossipHello { guid: npc });
    party.run(0.1);
    let menu = since(party, from);
    let option = menu
        .iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => {
                options.iter().find(|o| o.message == ask).map(|o| o.index)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no option {ask:?}: {menu:?}"));
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::GossipSelectOption { guid: npc, option });
    party.run(0.1);
    since(party, from)
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
fn band_30_35_loads_with_its_givers_enders_and_zone() {
    let mut realm = realm();
    for (id, giver, ender) in BAND_30_35 {
        let row = quest_row(id);
        assert_eq!(entry(&row, "giver"), Some(giver), "quest {id}'s giver");
        assert_eq!(entry(&row, "ender"), ender, "quest {id}'s ender");
        let want = if id >= 98111 { "bounty" } else { "story" };
        assert_eq!(row["kind"].as_str(), Some(want), "quest {id}'s kind");
        let t = template(&mut realm, id);
        assert_eq!(Some(t.title.as_str()), row["title"].as_str(), "quest {id}");
        assert_eq!(t.zone_or_sort, REDRIDGE, "quest {id}'s zone");
    }
}

#[test]
fn band_30_35_chains_as_the_ticket_names() {
    for (id, _, _) in BAND_30_35 {
        let want = CHAINED.iter().find(|c| c.0 == id).map(|c| c.1);
        assert_eq!(entry(&quest_row(id), "after"), want, "quest {id}'s `after`");
    }
}

#[test]
fn every_story_quest_offers_the_choices_its_gear_row_names() {
    let mut realm = realm();
    for (id, _, _) in BAND_30_35 {
        let want: Vec<u32> = CHOICES
            .iter()
            .find(|c| c.0 == id)
            .map_or(Vec::new(), |c| c.1.to_vec());
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
fn worse_problems_is_offered_cold_and_met_by_speaking_with_three() {
    let mut party = questing_party(31, &[]);
    let solomon = meet(&mut party, SOLOMON, SOLOMON_AT);
    assert!(offered(&mut party, solomon, WORSE_PROBLEMS));
    accept(&mut party, solomon, WORSE_PROBLEMS);
    let early = hand_in(&mut party, solomon, WORSE_PROBLEMS);
    assert!(!completed(&early, WORSE_PROBLEMS), "nobody asked yet");

    for (npc, at, line) in [
        (MARRIS, MARRIS_AT, "What does the Watch need?"),
        (OSLOW, OSLOW_AT, "What happened to the bridge?"),
        (BAREN, BAREN_AT, "What's wrong with the barges?"),
    ] {
        let guid = meet(&mut party, npc, at);
        let replied = ask(&mut party, guid, line);
        let credited = replied.iter().any(|e| {
            matches!(e, SessionEvent::QuestObjectiveKill { quest_id, entry, .. }
                if *quest_id == WORSE_PROBLEMS && *entry == npc)
        });
        assert!(credited, "{npc} gave no credit: {replied:?}");
    }

    let solomon = meet(&mut party, SOLOMON, SOLOMON_AT);
    let events = hand_in(&mut party, solomon, WORSE_PROBLEMS);
    assert!(completed(&events, WORSE_PROBLEMS), "{events:?}");
}

#[test]
fn parker_hands_over_his_report_and_feldon_keeps_it() {
    let mut party = questing_party(31, &[GNOLLS_AT_THREE_CORNERS]);
    let parker = meet(&mut party, PARKER, PARKER_AT);
    assert!(offered(&mut party, parker, REPORT_TO_LAKESHIRE));
    accept(&mut party, parker, REPORT_TO_LAKESHIRE);
    assert_eq!(carries(&party, PARKERS_REPORT), 1, "Parker hands it over");

    let feldon = meet(&mut party, FELDON, FELDON_AT);
    let events = hand_in(&mut party, feldon, REPORT_TO_LAKESHIRE);
    assert!(completed(&events, REPORT_TO_LAKESHIRE), "{events:?}");
    assert_eq!(carries(&party, PARKERS_REPORT), 0, "Feldon keeps it");
}

#[test]
fn walking_to_the_tower_door_credits_the_tower_on_the_hill() {
    let mut party = questing_party(34, &[WORSE_PROBLEMS, BLACKROCK_MENACE]);
    let marris = meet(&mut party, MARRIS, MARRIS_AT);
    assert!(offered(&mut party, marris, THE_TOWER_ON_THE_HILL));
    accept(&mut party, marris, THE_TOWER_ON_THE_HILL);

    let short = walk(&mut party, [TOWER_DOOR[0] + 40.0, TOWER_DOOR[1], 0.0]);
    assert!(!met(&short, THE_TOWER_ON_THE_HILL), "met short of the door");
    let there = walk(&mut party, TOWER_DOOR);
    assert!(met(&there, THE_TOWER_ON_THE_HILL), "{there:?}");
}

#[test]
fn band_35_40_loads_with_its_givers_enders_and_zone() {
    let mut realm = realm();
    for (id, giver, ender) in BAND_35_40 {
        let row = quest_row(id);
        assert_eq!(entry(&row, "giver"), Some(giver), "quest {id}'s giver");
        assert_eq!(entry(&row, "ender"), ender, "quest {id}'s ender");
        let want = if id >= 98211 { "bounty" } else { "story" };
        assert_eq!(row["kind"].as_str(), Some(want), "quest {id}'s kind");
        let t = template(&mut realm, id);
        assert_eq!(Some(t.title.as_str()), row["title"].as_str(), "quest {id}");
        assert_eq!(t.zone_or_sort, REDRIDGE, "quest {id}'s zone");
    }
}

#[test]
fn band_35_40_chains_as_the_ticket_names() {
    for (id, _, _) in BAND_35_40 {
        let want = CHAINED_35_40.iter().find(|c| c.0 == id).map(|c| c.1);
        assert_eq!(entry(&quest_row(id), "after"), want, "quest {id}'s `after`");
    }
}

#[test]
fn every_band_35_40_story_quest_offers_the_choices_its_gear_row_names() {
    let mut realm = realm();
    for (id, _, _) in BAND_35_40 {
        let want: Vec<u32> = CHOICES_35_40
            .iter()
            .find(|c| c.0 == id)
            .map_or(Vec::new(), |c| c.1.to_vec());
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
fn toren_is_a_quest_giver_on_the_inn_s_ground_floor() {
    let hub: toml::Table = include_str!("../content/hub.toml").parse().unwrap();
    let row = hub["npc"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["entry"].as_integer() == Some(TOREN.into()))
        .expect("Toren's hub row");
    assert_eq!(row["map"].as_integer(), Some(0));
    assert_eq!(row["role"].as_str(), Some("giver"));
    assert_eq!(row["npc_flags"].as_integer(), Some(0x2), "a quest giver");

    let mut party = questing_party(36, &[]);
    walk(&mut party, [TOREN_AT[0] + 1.5, TOREN_AT[1], TOREN_AT[2]]);
    let (_, at) = party.unit(0, TOREN, TOREN_AT);
    assert!(
        (0..3).all(|i| (at[i] - TOREN_AT[i]).abs() < 0.01),
        "Toren shown at {at:?}"
    );
}

#[test]
fn brianna_offers_toren_at_the_inn_cold_and_toren_takes_it_with_nothing_to_collect() {
    let row = quest_row(TOREN_AT_THE_INN);
    for key in ["after", "kill", "collect", "talk", "explore", "gives"] {
        assert!(row.get(key).is_none(), "98201 has no `{key}`");
    }

    let mut party = questing_party(36, &[]);
    let brianna = meet(&mut party, BRIANNA, BRIANNA_AT);
    assert!(offered(&mut party, brianna, TOREN_AT_THE_INN));
    accept(&mut party, brianna, TOREN_AT_THE_INN);

    let toren = meet(&mut party, TOREN, TOREN_AT);
    let events = hand_in(&mut party, toren, TOREN_AT_THE_INN);
    assert!(completed(&events, TOREN_AT_THE_INN), "{events:?}");
}

#[test]
fn ilgalar_s_three_items_exist_and_the_letters_have_two_pages() {
    let items: toml::Table = include_str!("../content/quest_items_redridge.toml")
        .parse()
        .unwrap();
    let rows = items["item"].as_array().unwrap();
    for id in ILGALAR_ITEMS {
        let row = rows
            .iter()
            .find(|i| i["entry"].as_integer() == Some(id.into()))
            .unwrap_or_else(|| panic!("item {id}"));
        let pages = row.get("pages").and_then(|p| p.as_array()).map(Vec::len);
        let want = (id == LETTERS_UNDER_A_CROWNED_SEAL).then_some(2);
        assert_eq!(pages, want, "item {id}'s pages");
    }

    let mut realm = realm();
    for id in ILGALAR_ITEMS {
        realm.handle(RealmCommand::ItemQuery { entry: id });
        let known = drain(&mut realm)
            .iter()
            .any(|e| matches!(e, SessionEvent::ItemTemplate { info: Some(_), .. }));
        assert!(known, "the realm knows item {id}");
    }
}

#[test]
fn band_40_45_loads_with_its_givers_and_zone() {
    let mut realm = realm();
    for (id, giver) in BAND_40_45 {
        let row = quest_row(id);
        assert_eq!(entry(&row, "giver"), Some(giver), "quest {id}'s giver");
        assert_eq!(entry(&row, "ender"), None, "quest {id}'s ender");
        let want = if id >= 98311 { "bounty" } else { "story" };
        assert_eq!(row["kind"].as_str(), Some(want), "quest {id}'s kind");
        let t = template(&mut realm, id);
        assert_eq!(Some(t.title.as_str()), row["title"].as_str(), "quest {id}");
        assert_eq!(t.zone_or_sort, REDRIDGE, "quest {id}'s zone");
    }
}

#[test]
fn band_40_45_chains_as_the_ticket_names() {
    for (id, _) in BAND_40_45 {
        let want = CHAINED_40_45.iter().find(|c| c.0 == id).map(|c| c.1);
        assert_eq!(entry(&quest_row(id), "after"), want, "quest {id}'s `after`");
    }
}

#[test]
fn every_band_40_45_story_quest_offers_the_choices_its_gear_row_names() {
    let mut realm = realm();
    for (id, _) in BAND_40_45 {
        let want: Vec<u32> = CHOICES_40_45
            .iter()
            .find(|c| c.0 == id)
            .map_or(Vec::new(), |c| c.1.to_vec());
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
fn reaching_render_s_rock_and_killing_8_trackers_meets_render_s_rock() {
    let mut party = questing_party(42, &[INTO_ILGALAR]);
    let marris = meet(&mut party, MARRIS, MARRIS_AT);
    assert!(offered(&mut party, marris, RENDERS_ROCK));
    accept(&mut party, marris, RENDERS_ROCK);

    // The trackers fall 40 yd short of the rock, so the kills alone do not meet it.
    let short = [RENDERS_ROCK_AT[0] + 40.0, RENDERS_ROCK_AT[1], 0.0];
    walk(&mut party, short);
    let mut credits = 0;
    for _ in 0..8 {
        let (events, _) = slay(&mut party, BLACKROCK_TRACKER, short);
        if credited(&events, RENDERS_ROCK) {
            credits += 1;
        }
        assert!(!met(&events, RENDERS_ROCK), "met short of the rock");
    }
    assert_eq!(credits, 8, "each tracker counts");

    let there = walk(&mut party, RENDERS_ROCK_AT);
    assert!(met(&there, RENDERS_ROCK), "{there:?}");
}

#[test]
fn tharil_zun_drops_his_orders_while_the_quest_wants_them() {
    let mut party = questing_party(44, &[KAZONS_CAMP]);
    let marris = meet(&mut party, MARRIS, MARRIS_AT);
    assert!(offered(&mut party, marris, THARILZUN));
    accept(&mut party, marris, THARILZUN);

    walk(&mut party, THARILZUN_AT);
    let (events, loot) = slay(&mut party, THARILZUN_ENTRY, THARILZUN_AT);
    assert!(credited(&events, THARILZUN), "{events:?}");
    assert!(
        loot.contains(&THARILZUNS_ORDERS),
        "his corpse held {loot:?}"
    );
    assert_eq!(carries(&party, THARILZUNS_ORDERS), 1);

    let (_, again) = slay(&mut party, THARILZUN_ENTRY, THARILZUN_AT);
    assert!(
        !again.contains(&THARILZUNS_ORDERS),
        "a second copy once the quest has its orders: {again:?}"
    );
}

#[test]
fn tharil_zun_s_orders_read_one_page() {
    let items: toml::Table = include_str!("../content/quest_items_redridge.toml")
        .parse()
        .unwrap();
    let row = items["item"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["entry"].as_integer() == Some(THARILZUNS_ORDERS.into()))
        .expect("Tharil'zun's Orders");
    let pages = row.get("pages").and_then(|p| p.as_array()).map(Vec::len);
    assert_eq!(pages, Some(1));
    assert_eq!(row["max_count"].as_integer(), Some(1));
}

/// Spawns a level 1 `entry` 3 yd west of `from`, for this character alone, and fights it from
/// `from` until it dies, then loots it into the bags: the events from the spawn on, and the
/// items its corpse held. Level 1 keeps the fight short; a kill's quest credit and quest drops
/// do not look at the creature's level.
fn slay(party: &mut Party, creature: u32, from: [f32; 3]) -> (Vec<SessionEvent>, Vec<u32>) {
    let me = party.guids[0];
    let prey = [from[0] - 3.0, from[1], from[2]];
    let before = party.seen(0).len();
    let guid = party
        .realm
        .spawn_for(
            party.host.as_ref(),
            creature,
            1,
            prey,
            0,
            Audience::of(&[me]),
        )
        .unwrap_or_else(|| panic!("creature {creature}"));
    party.go_facing(0, from, prey);
    party.run(0.3);
    party.cmd(0, RealmCommand::AttackSwing { guid });
    assert!(
        party.run_until(600.0, |got| died(&got[0], guid)),
        "creature {creature} never died"
    );
    party.run(0.5);
    party.cmd(0, RealmCommand::Loot { guid });
    let items: Vec<u32> = since(party, before)
        .iter()
        .find_map(|e| match e {
            SessionEvent::LootResponse { items, .. } => {
                Some(items.iter().map(|l| l.item_id).collect())
            }
            _ => None,
        })
        .unwrap_or_default();
    for slot in 0..items.len() {
        party.cmd(0, RealmCommand::AutostoreLootItem { slot: slot as _ });
    }
    party.cmd(0, RealmCommand::LootRelease { guid });
    (since(party, before), items)
}

fn credited(events: &[SessionEvent], quest: u32) -> bool {
    events.iter().any(
        |e| matches!(e, SessionEvent::QuestObjectiveKill { quest_id, .. } if *quest_id == quest),
    )
}

fn met(events: &[SessionEvent], quest: u32) -> bool {
    events.iter().any(
        |e| matches!(e, SessionEvent::QuestObjectivesComplete { quest_id } if *quest_id == quest),
    )
}
