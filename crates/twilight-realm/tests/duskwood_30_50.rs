//! Duskwood's quests for levels 30 to 50 (spec #595), walked on the shipped content. Band 30-35
//! (#603, `content/quests_duskwood.toml`): Lord Ello's round of Darkshire, the Night Watch's
//! chain on the north road and in Tranquil Gardens, and Daltry's log, which leads through Backus
//! and Sirra to the silent house on the hill; and four bounties.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

const DARKSHIRE_WAITS: u32 = 98601;
const THE_NIGHT_WATCH: u32 = 98602;
const A_CLERKS_RECORDS: u32 = 98604;
const THE_TUTORS_NAME: u32 = 98606;
const THE_SILENT_HOUSE: u32 = 98607;

/// Every quest of band 30-35: `(id, giver, ender, after)`, the ender `None` when it is the giver.
const BAND_30_35: [(u32, u32, Option<u32>, Option<u32>); 11] = [
    (98601, ELLO, None, None),
    (98602, ALTHEA, None, None),
    (98603, ALTHEA, None, Some(98602)),
    (98604, DALTRY, Some(BACKUS), Some(98601)),
    (98605, BACKUS, None, Some(98604)),
    (98606, BACKUS, Some(SIRRA), Some(98605)),
    (98607, EVA, None, Some(98606)),
    (98611, 272, None, None),
    (98612, 2142, None, None),
    (98613, 270, None, None),
    (98614, 302, None, None),
];

/// Each story quest's green choices, from the band's Quest Lines (#600).
const CHOICES: [(u32, &[u32]); 7] = [
    (98601, &[903214, 903216, 903218]),
    (98602, &[903204, 903206, 903209]),
    (98603, &[903215, 903217, 903219]),
    (98604, &[903204, 903206, 903208]),
    (98605, &[903214, 903216, 903218]),
    (98606, &[903200, 903201, 903202, 903203]),
    (98607, &[903211, 903212, 903213]),
];

/// Darkshire's people (`content/duskwood.toml`): entries and where each stands.
const ELLO: u32 = 263;
const ALTHEA: u32 = 264;
const EVA: u32 = 265;
const DALTRY: u32 = 267;
const SIRRA: u32 = 268;
const BACKUS: u32 = 840;
const ELLO_AT: [f32; 3] = [-10552.2, -1108.79, 31.511];
const ALTHEA_AT: [f32; 3] = [-10559.5, -1152.25, 28.076];
const EVA_AT: [f32; 3] = [-10532.4, -1213.12, 28.118];
const DALTRY_AT: [f32; 3] = [-10560.3, -1125.19, 30.071];
const SIRRA_AT: [f32; 3] = [-10573.9, -1127.32, 29.167];
const BACKUS_AT: [f32; 3] = [-10330.9, -1122.6, 21.858];

/// The quest items: the log 98604 gives, the note 98606 gives, and the surveyor's record a later
/// Mistmantle quest collects.
const NIGHT_WATCH_LOG: u32 = 980401;
const BACKUS_S_NOTE: u32 = 980402;
const SITE_RECORD: u32 = 980481;

/// Outside Mistmantle Manor's box, a step from its Portal; the Portal; halfway up the yard; and
/// the house's front door (`content/mistmantle_manor.toml`).
const BY_THE_GATE: [f32; 3] = [-10345.0, -1176.0, 40.05];
const PORTAL: [f32; 3] = [-10345.0, -1186.0, 40.05];
const UP_THE_YARD: [f32; 3] = [-10354.0, -1222.0, 37.0];
const MANOR_DOOR: [f32; 3] = [-10362.6, -1257.0, 35.91];

fn duskwood_quests() -> toml::Table {
    include_str!("../content/quests_duskwood.toml")
        .parse()
        .unwrap()
}

fn quest_row(id: u32) -> toml::Value {
    duskwood_quests()["quest"]
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

/// A Zealot (Tier 2, so the band's quests are offered) at level 35 in Darkshire, having handed
/// in `done` and carrying nothing.
fn in_darkshire(done: &[u32]) -> Party {
    let done = done.to_vec();
    let mut party = Party::edited(
        Realm::with_seed(1),
        &[35],
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
    walk(&mut party, ELLO_AT);
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

/// Asks `npc` its gossip option `option`; the events that came.
fn ask(party: &mut Party, npc: u64, option: &str) -> Vec<SessionEvent> {
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::GossipHello { guid: npc });
    party.run(0.1);
    let menu = since(party, from);
    let index = menu
        .iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => options
                .iter()
                .find(|o| o.message == option)
                .map(|o| o.index),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no option {option:?}: {menu:?}"));
    let from = party.seen(0).len();
    party.cmd(
        0,
        RealmCommand::GossipSelectOption {
            guid: npc,
            option: index,
        },
    );
    party.run(0.1);
    since(party, from)
}

fn completed(events: &[SessionEvent], quest: u32) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestComplete(c) if c.quest_id == quest))
}

fn credited(events: &[SessionEvent], quest: u32) -> bool {
    events.iter().any(
        |e| matches!(e, SessionEvent::QuestObjectiveKill { quest_id, .. } if *quest_id == quest),
    )
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
fn band_30_35_s_quests_load_with_their_givers_enders_and_zone() {
    let rows = duskwood_quests();
    let ids: Vec<u32> = rows["quest"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| int(q, "id").unwrap() as u32)
        .filter(|id| (98601..=98614).contains(id))
        .collect();
    assert_eq!(ids, BAND_30_35.map(|q| q.0));
    let mut realm = realm();
    for (id, giver, ender, _) in BAND_30_35 {
        let row = quest_row(id);
        assert_eq!(int(&row, "giver"), Some(giver.into()), "quest {id}'s giver");
        assert_eq!(
            int(&row, "ender"),
            ender.map(i64::from),
            "quest {id}'s ender"
        );
        assert_eq!(int(&row, "zone_or_sort"), Some(10), "quest {id}");
        let kind = row["kind"].as_str().unwrap();
        let want = if id >= 98611 { "bounty" } else { "story" };
        assert_eq!(kind, want, "quest {id}");
        let t = template(&mut realm, id).unwrap_or_else(|| panic!("quest {id} is not loaded"));
        assert_eq!(t.zone_or_sort, 10, "quest {id}");
        assert_eq!(Some(t.title.as_str()), row["title"].as_str(), "quest {id}");
    }
}

#[test]
fn the_chains_hold_as_the_ticket_orders_them() {
    for (id, _, _, after) in BAND_30_35 {
        assert_eq!(
            int(&quest_row(id), "after"),
            after.map(i64::from),
            "quest {id}'s `after`"
        );
    }

    // Lord Ello and Althea offer their first quests to a newcomer; Althea holds the gardens back.
    let mut party = in_darkshire(&[]);
    let ello = up_to(&mut party, ELLO, ELLO_AT);
    assert!(offers(&mut party, ello).contains(&DARKSHIRE_WAITS));
    let althea = up_to(&mut party, ALTHEA, ALTHEA_AT);
    let offered = offers(&mut party, althea);
    assert!(offered.contains(&THE_NIGHT_WATCH), "{offered:?}");
    assert!(!offered.contains(&98603), "{offered:?}");
    let daltry = up_to(&mut party, DALTRY, DALTRY_AT);
    assert!(!offers(&mut party, daltry).contains(&A_CLERKS_RECORDS));
}

#[test]
fn speaking_with_althea_daltry_and_eva_credits_darkshire_waits() {
    let mut party = in_darkshire(&[]);
    let ello = up_to(&mut party, ELLO, ELLO_AT);
    accept(&mut party, ello, DARKSHIRE_WAITS);
    let events = hand_in(&mut party, ello, DARKSHIRE_WAITS);
    assert!(!completed(&events, DARKSHIRE_WAITS), "handed in unasked");

    for (entry, at, option) in [
        (ALTHEA, ALTHEA_AT, "What does the Night Watch need?"),
        (DALTRY, DALTRY_AT, "What do the records say?"),
        (EVA, EVA_AT, "What do you see, Madame Eva?"),
    ] {
        let npc = up_to(&mut party, entry, at);
        let events = ask(&mut party, npc, option);
        assert!(credited(&events, DARKSHIRE_WAITS), "{entry}: {events:?}");
    }

    let ello = up_to(&mut party, ELLO, ELLO_AT);
    let events = hand_in(&mut party, ello, DARKSHIRE_WAITS);
    assert!(completed(&events, DARKSHIRE_WAITS), "{events:?}");
    let daltry = up_to(&mut party, DALTRY, DALTRY_AT);
    assert!(offers(&mut party, daltry).contains(&A_CLERKS_RECORDS));
}

#[test]
fn a_clerk_s_records_puts_the_log_in_the_bag_and_backus_takes_it() {
    let mut party = in_darkshire(&[DARKSHIRE_WAITS]);
    let daltry = up_to(&mut party, DALTRY, DALTRY_AT);
    accept(&mut party, daltry, A_CLERKS_RECORDS);
    assert_eq!(
        carries(&party, NIGHT_WATCH_LOG),
        1,
        "Daltry hands over the log"
    );

    let backus = up_to(&mut party, BACKUS, BACKUS_AT);
    let events = hand_in(&mut party, backus, A_CLERKS_RECORDS);
    assert!(completed(&events, A_CLERKS_RECORDS), "{events:?}");
    assert_eq!(
        carries(&party, NIGHT_WATCH_LOG),
        0,
        "the log stays with Backus"
    );
}

#[test]
fn the_tutor_s_name_puts_backus_s_note_in_the_bag_and_sirra_takes_it() {
    let mut party = in_darkshire(&[DARKSHIRE_WAITS, A_CLERKS_RECORDS, 98605]);
    let backus = up_to(&mut party, BACKUS, BACKUS_AT);
    accept(&mut party, backus, THE_TUTORS_NAME);
    assert_eq!(
        carries(&party, BACKUS_S_NOTE),
        1,
        "Backus hands over his note"
    );

    let sirra = up_to(&mut party, SIRRA, SIRRA_AT);
    let events = hand_in(&mut party, sirra, THE_TUTORS_NAME);
    assert!(completed(&events, THE_TUTORS_NAME), "{events:?}");
    assert_eq!(
        carries(&party, BACKUS_S_NOTE),
        0,
        "the note stays with Sirra"
    );
}

#[test]
fn standing_at_the_manor_s_door_credits_the_silent_house() {
    let done = [DARKSHIRE_WAITS, A_CLERKS_RECORDS, 98605, THE_TUTORS_NAME];
    let mut party = in_darkshire(&done);
    let eva = up_to(&mut party, EVA, EVA_AT);
    accept(&mut party, eva, THE_SILENT_HOUSE);

    let mut events = walk(&mut party, BY_THE_GATE);
    events.extend(walk(&mut party, PORTAL));
    events.extend(walk(&mut party, UP_THE_YARD));
    let complete = |events: &[SessionEvent]| {
        events.iter().any(|e| {
            matches!(e, SessionEvent::QuestObjectivesComplete { quest_id } if *quest_id == THE_SILENT_HOUSE)
        })
    };
    assert!(!complete(&events), "explored short of the door");
    let events = walk(&mut party, MANOR_DOOR);
    assert!(complete(&events), "{events:?}");
}

#[test]
fn the_surveyor_s_site_record_reads_two_pages() {
    let mut realm = realm();
    realm.handle(RealmCommand::ItemQuery { entry: SITE_RECORD });
    let first = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::ItemTemplate {
                info: Some(info), ..
            } => Some(info.page_text),
            _ => None,
        })
        .expect("the record is an item");
    assert_ne!(first, 0, "the record is not readable");
    realm.handle(RealmCommand::PageTextQuery { page_id: first });
    let pages: Vec<u32> = drain(&mut realm)
        .into_iter()
        .filter_map(|e| match e {
            SessionEvent::PageText { next_page_id, .. } => Some(next_page_id),
            _ => None,
        })
        .collect();
    assert_eq!(pages, [first + 1, 0], "two pages");
}

#[test]
fn every_story_quest_offers_the_choices_the_gear_table_names() {
    let mut realm = realm();
    for (id, want) in CHOICES {
        let t = template(&mut realm, id).unwrap_or_else(|| panic!("quest {id} is not loaded"));
        let got: Vec<u32> = t
            .choices
            .iter()
            .map(|c| c.0)
            .filter(|&item| item != 0)
            .collect();
        assert_eq!(got, want, "quest {id}'s choices");
    }
    for id in 98611..=98614 {
        let t = template(&mut realm, id).unwrap_or_else(|| panic!("quest {id} is not loaded"));
        assert!(
            t.choices.iter().all(|c| c.0 == 0),
            "bounty {id} offers a choice"
        );
    }
}
