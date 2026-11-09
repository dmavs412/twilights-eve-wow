//! Duskwood's dungeon quests (spec #595, `content/quests_duskwood_dungeons.toml`), walked on the
//! shipped content. Mistmantle Manor (#621): Madame Eva sends the character into the house to put
//! Stalvan to rest and bring back what he guards, then sends the surveyor's site record on to
//! Sirra Von'Indi.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

const THE_SILENT_HOUSE: u32 = 98607;
const STALVAN_QUEST: u32 = 99001;
const THE_SITE_RECORD: u32 = 99002;

/// Every Mistmantle Manor quest: `(id, giver, ender, after)`, the ender `None` when it is the
/// giver.
const MISTMANTLE: [(u32, u32, Option<u32>, Option<u32>); 2] = [
    (STALVAN_QUEST, EVA, None, Some(THE_SILENT_HOUSE)),
    (THE_SITE_RECORD, EVA, Some(SIRRA), Some(STALVAN_QUEST)),
];

/// Each story quest's green choices, from the band's Quest Lines (#600).
const CHOICES: [(u32, &[u32]); 1] = [(THE_SITE_RECORD, &[903205, 903207, 903210])];

/// The Mistmantle Manor set's four Chests, cloth, leather, mail and plate (#642, `gear.csv`).
const MISTMANTLE_CHESTS: [u32; 4] = [905140, 905141, 905142, 905143];

/// Darkshire's people (`content/duskwood.toml`): entries and where each stands.
const EVA: u32 = 265;
const SIRRA: u32 = 268;
const EVA_AT: [f32; 3] = [-10532.4, -1213.12, 28.118];
const SIRRA_AT: [f32; 3] = [-10573.9, -1127.32, 29.167];

/// Stalvan Mistmantle, the manor's master, and his creature row (`content/mistmantle_manor.toml`).
const STALVAN: u32 = 97229;
const STALVAN_GUID: i64 = 974014;

const SITE_RECORD: u32 = 980481;

/// The quests done before 98607, which leads to the manor.
const BEFORE_THE_HOUSE: [u32; 4] = [98601, 98604, 98605, 98606];

fn dungeon_quests() -> toml::Table {
    include_str!("../content/quests_duskwood_dungeons.toml")
        .parse()
        .unwrap()
}

fn manor() -> toml::Table {
    include_str!("../content/mistmantle_manor.toml")
        .parse()
        .unwrap()
}

fn quest_row(id: u32) -> toml::Value {
    dungeon_quests()["quest"]
        .as_array()
        .unwrap()
        .iter()
        .find(|q| int(q, "id") == Some(id.into()))
        .unwrap_or_else(|| panic!("quest {id}"))
        .clone()
}

fn int(v: &toml::Value, key: &str) -> Option<i64> {
    v.get(key).and_then(|x| x.as_integer())
}

/// The quest's template as the client asks for it, if the realm has the quest.
fn template(
    realm: &mut Realm,
    quest: u32,
) -> Option<Box<benilla_protocol::messages::QuestTemplate>> {
    realm.handle(RealmCommand::QuestQuery { quest });
    drain(realm).into_iter().find_map(|e| match e {
        SessionEvent::QuestTemplate(t) if t.quest_id == quest => Some(t),
        _ => None,
    })
}

/// The non-empty choices of `quest`'s template.
fn choices(realm: &mut Realm, quest: u32) -> Vec<u32> {
    template(realm, quest)
        .unwrap_or_else(|| panic!("quest {quest} is not loaded"))
        .choices
        .iter()
        .map(|c| c.0)
        .filter(|&item| item != 0)
        .collect()
}

/// A Zealot (Tier 2, so the quests are offered) at level 36 by Madame Eva in Darkshire, having
/// handed in `done` and carrying nothing.
fn in_darkshire(done: &[u32]) -> Party {
    let done = done.to_vec();
    let mut party = Party::edited(
        Realm::with_seed(1),
        &[36],
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
    walk(&mut party, EVA_AT);
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

/// Walks up to the creature `entry` standing at `at`; its guid.
fn up_to(party: &mut Party, entry: u32, at: [f32; 3]) -> u64 {
    walk(party, [at[0] + 1.5, at[1], at[2]]);
    party.unit(0, entry, at).0
}

/// The quests `npc` offers to take, as talking to it shows them.
fn offers(party: &mut Party, npc: u64) -> Vec<u32> {
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::QuestGiverHello { npc });
    party.run(0.2);
    since(party, from)
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
        .filter(|i| int(i, "entry") == Some(entry.into()))
        .map(|i| int(i, "count").unwrap_or(1) as u32)
        .sum()
}

#[test]
fn mistmantle_s_quests_load_with_their_givers_enders_and_zone() {
    let rows = dungeon_quests();
    let ids: Vec<u32> = rows["quest"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| int(q, "id").unwrap() as u32)
        .filter(|id| (99001..=99099).contains(id))
        .collect();
    assert_eq!(ids, MISTMANTLE.map(|q| q.0));
    let mut realm = realm();
    for (id, giver, ender, after) in MISTMANTLE {
        let row = quest_row(id);
        assert_eq!(int(&row, "giver"), Some(giver.into()), "quest {id}'s giver");
        assert_eq!(
            int(&row, "ender"),
            ender.map(i64::from),
            "quest {id}'s ender"
        );
        assert_eq!(
            int(&row, "after"),
            after.map(i64::from),
            "quest {id}'s `after`"
        );
        assert_eq!(row["kind"].as_str(), Some("story"), "quest {id}");
        assert_eq!(int(&row, "level"), Some(36), "quest {id}");
        assert_eq!(int(&row, "zone_or_sort"), Some(10), "quest {id}");
        let t = template(&mut realm, id).unwrap_or_else(|| panic!("quest {id} is not loaded"));
        assert_eq!(t.zone_or_sort, 10, "quest {id}");
        assert_eq!(Some(t.title.as_str()), row["title"].as_str(), "quest {id}");
    }
    assert_eq!(int(&quest_row(STALVAN_QUEST), "money"), Some(1620));
    assert_eq!(int(&quest_row(THE_SITE_RECORD), "money"), Some(0));
}

#[test]
fn eva_offers_stalvan_only_once_the_silent_house_is_handed_in() {
    let mut party = in_darkshire(&BEFORE_THE_HOUSE);
    let eva = up_to(&mut party, EVA, EVA_AT);
    let offered = offers(&mut party, eva);
    assert!(offered.contains(&THE_SILENT_HOUSE), "{offered:?}");
    assert!(!offered.contains(&STALVAN_QUEST), "{offered:?}");

    let mut done = BEFORE_THE_HOUSE.to_vec();
    done.push(THE_SILENT_HOUSE);
    let mut party = in_darkshire(&done);
    let eva = up_to(&mut party, EVA, EVA_AT);
    let offered = offers(&mut party, eva);
    assert!(offered.contains(&STALVAN_QUEST), "{offered:?}");
    assert!(!offered.contains(&THE_SITE_RECORD), "{offered:?}");
}

#[test]
fn stalvan_s_corpse_holds_the_site_record_that_his_quest_wants() {
    let manor = manor();
    let row = manor["creature"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| int(c, "guid") == Some(STALVAN_GUID))
        .expect("Stalvan's creature row");
    assert_eq!(int(row, "entry"), Some(STALVAN.into()));
    let drop: Vec<i64> = row["drop"]
        .as_array()
        .expect("Stalvan's drop")
        .iter()
        .map(|i| i.as_integer().unwrap())
        .collect();
    assert_eq!(drop, [i64::from(SITE_RECORD)]);
    let others = manor["creature"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| int(c, "guid") != Some(STALVAN_GUID))
        .filter(|c| c.get("drop").is_some())
        .count();
    assert_eq!(others, 0, "only Stalvan holds the record");

    let t = template(&mut realm(), STALVAN_QUEST).expect("99001 is loaded");
    let wants: Vec<(u32, u32, u32, u32)> = t
        .objectives
        .iter()
        .map(|o| (o.creature_or_go, o.required_count, o.item_id, o.item_count))
        .filter(|o| *o != (0, 0, 0, 0))
        .collect();
    assert_eq!(wants, [(STALVAN, 1, SITE_RECORD, 1)]);
}

#[test]
fn stalvan_offers_exactly_the_mistmantle_set_s_four_chests() {
    assert_eq!(
        choices(&mut realm(), STALVAN_QUEST),
        MISTMANTLE_CHESTS,
        "99001's choices"
    );
}

#[test]
fn the_site_record_puts_the_record_back_in_the_bag_and_sirra_takes_it() {
    let mut done = BEFORE_THE_HOUSE.to_vec();
    done.extend([THE_SILENT_HOUSE, STALVAN_QUEST]);
    let mut party = in_darkshire(&done);
    let eva = up_to(&mut party, EVA, EVA_AT);
    accept(&mut party, eva, THE_SITE_RECORD);
    assert_eq!(carries(&party, SITE_RECORD), 1, "Eva hands back the record");

    let sirra = up_to(&mut party, SIRRA, SIRRA_AT);
    let events = hand_in(&mut party, sirra, THE_SITE_RECORD);
    assert!(completed(&events, THE_SITE_RECORD), "{events:?}");
    assert_eq!(
        carries(&party, SITE_RECORD),
        0,
        "the record stays with Sirra"
    );
}

#[test]
fn every_story_quest_offers_the_choices_the_gear_table_names() {
    let mut realm = realm();
    for (id, want) in CHOICES {
        assert_eq!(choices(&mut realm, id), want, "quest {id}'s choices");
    }
}
