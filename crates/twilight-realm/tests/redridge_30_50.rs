//! Redridge's quests from 30 to 50 (spec #595), walked on the shipped content
//! (`content/quests_redridge.toml`). Band 30-35 (#602): Three Corners, Lakeshire's three problems,
//! the bridge, the lakebed, the Thresher and the tower on the hill, and four bounties.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

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

fn met(events: &[SessionEvent], quest: u32) -> bool {
    events.iter().any(
        |e| matches!(e, SessionEvent::QuestObjectivesComplete { quest_id } if *quest_id == quest),
    )
}
