//! Duskwood's quests for levels 30 to 50 (spec #595), walked on the shipped content. Band 30-35
//! (#603, `content/quests_duskwood.toml`): Lord Ello's round of Darkshire, the Night Watch's
//! chain on the north road and in Tranquil Gardens, and Daltry's log, which leads through Backus
//! and Sirra to the silent house on the hill; and four bounties. Band 35-40 (#608): Jitters'
//! watch on Raven Hill Cemetery and the Grey Robes' crown, the Night Watch on the south road
//! following the Defias and the ogres' crates, Sven Yorgen's revenge on Morbent Fel; and four
//! bounties. Band 40-45 (#613): Sirra asks the Watch about the night the dark came and sends
//! her account to Lord Ello, Jitters follows the Surveyors' stakes to the crypt under Raven Hill,
//! Calor's worgen and Fenros; and four bounties. Band 45-50 (#618): the Grey Robes' camp in
//! Twilight Grove and its Grey Captain, Mor'Ladim and the daughter who lived, Dodds' Rotting
//! Orchard; and four bounties.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;
use twilight_realm::Audience;

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

const RAVEN_HILLS_WATCH: u32 = 98701;
const THE_GRAVE_ROBBERS: u32 = 98702;
const NIGHT_RUNNERS: u32 = 98703;
const OGRES_THE_WRONG_WAY: u32 = 98704;
const SVENS_CAMP: u32 = 98705;
const SVENS_REVENGE: u32 = 98706;

/// Every quest of band 35-40: `(id, giver, after)`; each is handed in to its giver.
const BAND_35_40: [(u32, u32, Option<u32>); 10] = [
    (98701, JITTERS, None),
    (98702, JITTERS, Some(98701)),
    (98703, DODDS, None),
    (98704, PAIGE, Some(98703)),
    (98705, SVEN, None),
    (98706, SVEN, Some(98705)),
    (98711, JITTERS, None),
    (98712, PAIGE, None),
    (98713, 893, None),
    (98714, 273, None),
];

/// Each story quest's choices: greens from the band's Quest Lines (#605), and Sven's Revenge's
/// blues.
const CHOICES_35_40: [(u32, &[u32]); 6] = [
    (98701, &[903220, 903221, 903222, 903223]),
    (98702, &[903231, 903232, 903233]),
    (98703, &[903225, 903227, 903230]),
    (98704, &[903234, 903236, 903238]),
    (98705, &[903224, 903226, 903229]),
    (98706, &[903303, 903304, 903305]),
];

/// Band 35-40's givers (`content/duskwood.toml`): entries and where each stands.
const JITTERS: u32 = 288;
const DODDS: u32 = 888;
const PAIGE: u32 = 499;
const SVEN: u32 = 311;
const JITTERS_AT: [f32; 3] = [-10732.2, 337.197, 38.638];
const DODDS_AT: [f32; 3] = [-10923.2, -384.928, 39.434];
const PAIGE_AT: [f32; 3] = [-10934.5, -381.813, 40.198];
const SVEN_AT: [f32; 3] = [-10329.9, 623.241, 26.874];

/// Raven Hill's Watch's two spots, Grey Robe camps in the cemetery (`content/zones.toml`).
const WEST_GRAVES: [f32; 3] = [-10470.0, 250.0, 31.0];
const NORTH_EARTH: [f32; 3] = [-10500.0, 320.0, 31.0];

const GREY_ROBE_DIGGER: u32 = 96048;
const CROWNED_TOKEN: u32 = 980411;

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

const TWELVE_YEARS_AGO: u32 = 98801;
const A_MACHINE_THAT_WENT_WRONG: u32 = 98802;
const WHAT_THE_DIGGERS_WANT: u32 = 98803;
const INTO_THE_DEEPS: u32 = 98804;
const WORGEN_IN_THE_WOODS: u32 = 98805;
const FENROS: u32 = 98806;

/// Every quest of band 40-45: `(id, giver, ender, after)`, the ender `None` when it is the giver.
const BAND_40_45: [(u32, u32, Option<u32>, Option<u32>); 10] = [
    (98801, SIRRA, None, Some(98606)),
    (98802, SIRRA, Some(ELLO), Some(98801)),
    (98803, JITTERS, None, Some(98702)),
    (98804, JITTERS, None, Some(98803)),
    (98805, CALOR, None, None),
    (98806, CALOR, None, Some(98805)),
    (98811, ROYCE, None, None),
    (98812, 273, None, None),
    (98813, JITTERS, None, None),
    (98814, KELLER, None, None),
];

/// Each band 40-45 story quest's green choices, from the band's Quest Lines (#610).
const CHOICES_40_45: [(u32, &[u32]); 6] = [
    (98801, &[903244, 903246, 903248]),
    (98802, &[903255, 903257, 903259]),
    (98803, &[903240, 903241, 903242, 903243]),
    (98804, &[903251, 903252, 903253]),
    (98805, &[903245, 903247, 903250]),
    (98806, &[903254, 903256, 903258]),
];

/// Band 40-45's Darkshire people (`content/duskwood.toml`): entries and where each stands.
const ROYCE: u32 = 999;
const KELLER: u32 = 885;
const HARTIN: u32 = 886;
const CALOR: u32 = 663;
const ROYCE_AT: [f32; 3] = [-10547.6, -1312.39, 41.695];
const KELLER_AT: [f32; 3] = [-10520.7, -1279.64, 39.307];
const HARTIN_AT: [f32; 3] = [-10523.4, -1197.27, 27.192];
const CALOR_AT: [f32; 3] = [-10572.9, -1211.98, 26.219];

/// The account 98802 gives, and the two items the Deeps' quest of a later ticket uses.
const SIRRAS_ACCOUNT: u32 = 980421;
const CROWNED_ORDERS: u32 = 980486;
const MORGANTHS_NOTES: u32 = 980487;

/// Raven Hill Deeps' Portal on the mausoleum floor (`content/raven_hill_deeps.toml`), and a
/// spot by the mausoleum, outside the spot's radius.
const DEEPS_PORTAL: [f32; 3] = [-10418.6, 410.8, 47.8];
const BY_THE_MAUSOLEUM: [f32; 3] = [-10440.0, 410.8, 47.8];

const THE_AGENTS_CAMP: u32 = 98901;
const THE_GREY_CAPTAIN: u32 = 98902;
const MOR_LADIM: u32 = 98903;
const THE_DAUGHTER_WHO_LIVED: u32 = 98904;
const A_DAUGHTERS_LOVE: u32 = 98905;

/// Every quest of band 45-50: `(id, giver, ender, after)`, the ender `None` when it is the giver.
const BAND_45_50: [(u32, u32, Option<u32>, Option<u32>); 10] = [
    (98901, ALTHEA, None, Some(98602)),
    (98902, ALTHEA, None, Some(98901)),
    (98903, SIRRA, None, Some(98801)),
    (98904, SIRRA, Some(LADIMORE), Some(98903)),
    (98905, LADIMORE, None, Some(98904)),
    (98906, DODDS, None, None),
    (98911, ALTHEA, None, None),
    (98912, PAIGE, None, None),
    (98913, 272, None, None),
    (98914, JITTERS, None, None),
];

/// Each band 45-50 story quest's choices: greens from the band's Quest Lines (#615), and the
/// named elites' item quests' blues.
const CHOICES_45_50: [(u32, &[u32]); 6] = [
    (98901, &[903275, 903277, 903279]),
    (98902, &[903310, 903311, 903312]),
    (98903, &[903316, 903317, 903318]),
    (98904, &[903260, 903261, 903262, 903263]),
    (98905, &[903271, 903272, 903273]),
    (98906, &[903265, 903267, 903270]),
];

/// Watcher Ladimore (`content/duskwood.toml`), and where she stands.
const LADIMORE: u32 = 576;
const LADIMORE_AT: [f32; 3] = [-10581.6, -1172.89, 28.494];

const GREY_CAPTAIN: u32 = 96053;
const CROWNED_COMMISSION: u32 = 980432;
const MOR_LADIMS_SKULL: u32 = 980433;

/// The agents' camp's centre in Twilight Grove (`content/zones.toml`), and its three objects.
const CAMP_AT: [f32; 2] = [-10460.0, -470.0];
const CAMP_OBJECTS: [u32; 3] = [793011, 793012, 793013];

/// Morgan Ladimore's grave at Raven Hill Cemetery's north edge, and a spot 10 yd short of it.
const LADIMORES_GRAVE: [f32; 3] = [-10495.0, 431.0, 47.0];
const SHORT_OF_THE_GRAVE: [f32; 3] = [-10505.0, 431.0, 47.0];

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

/// Item `entry`'s first page and each page's next page, as the client reads them; `(0, [])` for
/// an item that is not readable. Panics when the realm has no such item.
fn pages(realm: &mut Realm, entry: u32) -> (u32, Vec<u32>) {
    realm.handle(RealmCommand::ItemQuery { entry });
    let first = drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::ItemTemplate {
                info: Some(info), ..
            } => Some(info.page_text),
            _ => None,
        })
        .unwrap_or_else(|| panic!("item {entry} is not an item"));
    if first == 0 {
        return (0, Vec::new());
    }
    realm.handle(RealmCommand::PageTextQuery { page_id: first });
    let next = drain(realm)
        .into_iter()
        .filter_map(|e| match e {
            SessionEvent::PageText { next_page_id, .. } => Some(next_page_id),
            _ => None,
        })
        .collect();
    (first, next)
}

#[test]
fn the_surveyor_s_site_record_reads_two_pages() {
    let (first, next) = pages(&mut realm(), SITE_RECORD);
    assert_ne!(first, 0, "the record is not readable");
    assert_eq!(next, [first + 1, 0], "two pages");
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

#[test]
fn band_35_40_s_quests_load_with_their_givers_and_zone() {
    let rows = duskwood_quests();
    let ids: Vec<u32> = rows["quest"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| int(q, "id").unwrap() as u32)
        .filter(|id| (98701..=98714).contains(id))
        .collect();
    assert_eq!(ids, BAND_35_40.map(|q| q.0));
    let mut realm = realm();
    for (id, giver, _) in BAND_35_40 {
        let row = quest_row(id);
        assert_eq!(int(&row, "giver"), Some(giver.into()), "quest {id}'s giver");
        assert_eq!(int(&row, "ender"), None, "quest {id}'s ender");
        assert_eq!(int(&row, "zone_or_sort"), Some(10), "quest {id}");
        let kind = row["kind"].as_str().unwrap();
        let want = if id >= 98711 { "bounty" } else { "story" };
        assert_eq!(kind, want, "quest {id}");
        let t = template(&mut realm, id).unwrap_or_else(|| panic!("quest {id} is not loaded"));
        assert_eq!(t.zone_or_sort, 10, "quest {id}");
        assert_eq!(Some(t.title.as_str()), row["title"].as_str(), "quest {id}");
    }
}

#[test]
fn band_35_40_s_chains_hold() {
    for (id, _, after) in BAND_35_40 {
        assert_eq!(
            int(&quest_row(id), "after"),
            after.map(i64::from),
            "quest {id}'s `after`"
        );
    }

    // Each chain's first step is on offer to a newcomer, its second held back.
    let mut party = in_darkshire(&[]);
    for (entry, at, first, second) in [
        (JITTERS, JITTERS_AT, RAVEN_HILLS_WATCH, THE_GRAVE_ROBBERS),
        (DODDS, DODDS_AT, NIGHT_RUNNERS, 0),
        (PAIGE, PAIGE_AT, 0, OGRES_THE_WRONG_WAY),
        (SVEN, SVEN_AT, SVENS_CAMP, SVENS_REVENGE),
    ] {
        let npc = up_to(&mut party, entry, at);
        let offered = offers(&mut party, npc);
        assert!(
            first == 0 || offered.contains(&first),
            "{entry}: {offered:?}"
        );
        assert!(!offered.contains(&second), "{entry}: {offered:?}");
    }

    // With each first step handed in, the second is on offer.
    let mut party = in_darkshire(&[RAVEN_HILLS_WATCH, NIGHT_RUNNERS, SVENS_CAMP]);
    for (entry, at, second) in [
        (JITTERS, JITTERS_AT, THE_GRAVE_ROBBERS),
        (PAIGE, PAIGE_AT, OGRES_THE_WRONG_WAY),
        (SVEN, SVEN_AT, SVENS_REVENGE),
    ] {
        let npc = up_to(&mut party, entry, at);
        let offered = offers(&mut party, npc);
        assert!(offered.contains(&second), "{entry}: {offered:?}");
    }
}

#[test]
fn reaching_both_cemetery_spots_credits_raven_hill_s_watch_each_on_its_own_row() {
    let mut party = in_darkshire(&[]);
    let jitters = up_to(&mut party, JITTERS, JITTERS_AT);
    accept(&mut party, jitters, RAVEN_HILLS_WATCH);

    let spot_credits = |events: &[SessionEvent]| -> Vec<u32> {
        events
            .iter()
            .filter_map(|e| match e {
                SessionEvent::QuestObjectiveKill {
                    quest_id, entry, ..
                } if *quest_id == RAVEN_HILLS_WATCH => Some(*entry),
                _ => None,
            })
            .collect()
    };
    let complete = |events: &[SessionEvent]| {
        events.iter().any(|e| {
            matches!(e, SessionEvent::QuestObjectivesComplete { quest_id } if *quest_id == RAVEN_HILLS_WATCH)
        })
    };

    let west = walk(&mut party, WEST_GRAVES);
    let first = spot_credits(&west);
    assert_eq!(first.len(), 1, "{west:?}");
    assert!(!complete(&west), "one spot completed the quest");
    let north = walk(&mut party, NORTH_EARTH);
    let second = spot_credits(&north);
    assert_eq!(second.len(), 1, "{north:?}");
    assert_ne!(first, second, "both spots credit one log row");
    assert!(complete(&north), "{north:?}");
}

#[test]
fn killing_a_grey_robe_digger_credits_the_grave_robbers_and_can_drop_a_crowned_token() {
    let mut party = in_darkshire(&[RAVEN_HILLS_WATCH]);
    let jitters = up_to(&mut party, JITTERS, JITTERS_AT);
    accept(&mut party, jitters, THE_GRAVE_ROBBERS);

    let me = party.guids[0];
    let from = [JITTERS_AT[0] - 10.0, JITTERS_AT[1], JITTERS_AT[2]];
    let prey = [from[0] - 3.0, from[1], from[2]];
    let mut credits = 0;
    for _ in 0..12 {
        let digger = party
            .realm
            .spawn_for(
                party.host.as_ref(),
                GREY_ROBE_DIGGER,
                30,
                prey,
                0,
                Audience::of(&[me]),
            )
            .expect("a Grey Robe Digger");
        let before = party.seen(0).len();
        party.go_facing(0, from, prey);
        party.run(0.3);
        party.cmd(0, RealmCommand::AttackSwing { guid: digger });
        assert!(
            party.run_until(600.0, |got| died(&got[0], digger)),
            "the Digger never died"
        );
        party.run(0.5);
        if credited(&since(&party, before), THE_GRAVE_ROBBERS) {
            credits += 1;
        }
        party.cmd(0, RealmCommand::Loot { guid: digger });
        let items: Vec<u32> = since(&party, before)
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
        party.cmd(0, RealmCommand::LootRelease { guid: digger });
        if items.contains(&CROWNED_TOKEN) {
            break;
        }
    }
    assert!(credits >= 1, "no Digger credited the quest");
    assert_eq!(
        carries(&party, CROWNED_TOKEN),
        1,
        "no Crowned Token dropped"
    );
}

#[test]
fn every_band_35_40_story_quest_offers_the_choices_the_gear_table_names() {
    let mut realm = realm();
    for (id, want) in CHOICES_35_40 {
        let t = template(&mut realm, id).unwrap_or_else(|| panic!("quest {id} is not loaded"));
        let got: Vec<u32> = t
            .choices
            .iter()
            .map(|c| c.0)
            .filter(|&item| item != 0)
            .collect();
        assert_eq!(got, want, "quest {id}'s choices");
    }
    for id in 98711..=98714 {
        let t = template(&mut realm, id).unwrap_or_else(|| panic!("quest {id} is not loaded"));
        assert!(
            t.choices.iter().all(|c| c.0 == 0),
            "bounty {id} offers a choice"
        );
    }
}

#[test]
fn band_40_45_s_quests_load_with_their_givers_enders_and_zone() {
    let rows = duskwood_quests();
    let ids: Vec<u32> = rows["quest"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| int(q, "id").unwrap() as u32)
        .filter(|id| (98801..=98814).contains(id))
        .collect();
    assert_eq!(ids, BAND_40_45.map(|q| q.0));
    let mut realm = realm();
    for (id, giver, ender, _) in BAND_40_45 {
        let row = quest_row(id);
        assert_eq!(int(&row, "giver"), Some(giver.into()), "quest {id}'s giver");
        assert_eq!(
            int(&row, "ender"),
            ender.map(i64::from),
            "quest {id}'s ender"
        );
        assert_eq!(int(&row, "zone_or_sort"), Some(10), "quest {id}");
        let kind = row["kind"].as_str().unwrap();
        let want = if id >= 98811 { "bounty" } else { "story" };
        assert_eq!(kind, want, "quest {id}");
        let t = template(&mut realm, id).unwrap_or_else(|| panic!("quest {id} is not loaded"));
        assert_eq!(t.zone_or_sort, 10, "quest {id}");
        assert_eq!(Some(t.title.as_str()), row["title"].as_str(), "quest {id}");
    }
}

#[test]
fn band_40_45_s_chains_hold() {
    for (id, _, _, after) in BAND_40_45 {
        assert_eq!(
            int(&quest_row(id), "after"),
            after.map(i64::from),
            "quest {id}'s `after`"
        );
    }

    // A newcomer gets Calor's worgen and nothing else of the band's chains.
    let mut party = in_darkshire(&[]);
    let sirra = up_to(&mut party, SIRRA, SIRRA_AT);
    assert!(!offers(&mut party, sirra).contains(&TWELVE_YEARS_AGO));
    let calor = up_to(&mut party, CALOR, CALOR_AT);
    let offered = offers(&mut party, calor);
    assert!(offered.contains(&WORGEN_IN_THE_WOODS), "{offered:?}");
    assert!(!offered.contains(&FENROS), "{offered:?}");
    let jitters = up_to(&mut party, JITTERS, JITTERS_AT);
    let offered = offers(&mut party, jitters);
    assert!(!offered.contains(&WHAT_THE_DIGGERS_WANT), "{offered:?}");

    // Each chain's next step opens once the one before it is handed in.
    let done = [
        DARKSHIRE_WAITS,
        A_CLERKS_RECORDS,
        98605,
        THE_TUTORS_NAME,
        RAVEN_HILLS_WATCH,
        THE_GRAVE_ROBBERS,
        WORGEN_IN_THE_WOODS,
    ];
    let mut party = in_darkshire(&done);
    let sirra = up_to(&mut party, SIRRA, SIRRA_AT);
    let offered = offers(&mut party, sirra);
    assert!(offered.contains(&TWELVE_YEARS_AGO), "{offered:?}");
    assert!(!offered.contains(&A_MACHINE_THAT_WENT_WRONG), "{offered:?}");
    let calor = up_to(&mut party, CALOR, CALOR_AT);
    assert!(offers(&mut party, calor).contains(&FENROS));
    let jitters = up_to(&mut party, JITTERS, JITTERS_AT);
    let offered = offers(&mut party, jitters);
    assert!(offered.contains(&WHAT_THE_DIGGERS_WANT), "{offered:?}");
    assert!(!offered.contains(&INTO_THE_DEEPS), "{offered:?}");

    let mut party = in_darkshire(&[RAVEN_HILLS_WATCH, THE_GRAVE_ROBBERS, WHAT_THE_DIGGERS_WANT]);
    let jitters = up_to(&mut party, JITTERS, JITTERS_AT);
    assert!(offers(&mut party, jitters).contains(&INTO_THE_DEEPS));
}

#[test]
fn speaking_with_the_three_watchers_credits_twelve_years_ago() {
    let done = [DARKSHIRE_WAITS, A_CLERKS_RECORDS, 98605, THE_TUTORS_NAME];
    let mut party = in_darkshire(&done);
    let sirra = up_to(&mut party, SIRRA, SIRRA_AT);
    accept(&mut party, sirra, TWELVE_YEARS_AGO);
    let events = hand_in(&mut party, sirra, TWELVE_YEARS_AGO);
    assert!(!completed(&events, TWELVE_YEARS_AGO), "handed in unasked");

    let option = "What do you remember of the night the dark came?";
    for (entry, at) in [(ROYCE, ROYCE_AT), (KELLER, KELLER_AT), (HARTIN, HARTIN_AT)] {
        let npc = up_to(&mut party, entry, at);
        let events = ask(&mut party, npc, option);
        assert!(credited(&events, TWELVE_YEARS_AGO), "{entry}: {events:?}");
    }

    let sirra = up_to(&mut party, SIRRA, SIRRA_AT);
    let events = hand_in(&mut party, sirra, TWELVE_YEARS_AGO);
    assert!(completed(&events, TWELVE_YEARS_AGO), "{events:?}");
}

#[test]
fn a_machine_that_went_wrong_puts_sirra_s_account_in_the_bag_and_ebonlocke_takes_it() {
    let done = [
        DARKSHIRE_WAITS,
        A_CLERKS_RECORDS,
        98605,
        THE_TUTORS_NAME,
        TWELVE_YEARS_AGO,
    ];
    let mut party = in_darkshire(&done);
    let sirra = up_to(&mut party, SIRRA, SIRRA_AT);
    accept(&mut party, sirra, A_MACHINE_THAT_WENT_WRONG);
    assert_eq!(
        carries(&party, SIRRAS_ACCOUNT),
        1,
        "Sirra hands over her account"
    );

    let ello = up_to(&mut party, ELLO, ELLO_AT);
    let events = hand_in(&mut party, ello, A_MACHINE_THAT_WENT_WRONG);
    assert!(completed(&events, A_MACHINE_THAT_WENT_WRONG), "{events:?}");
    assert_eq!(
        carries(&party, SIRRAS_ACCOUNT),
        0,
        "the account stays with Ebonlocke"
    );
}

#[test]
fn standing_at_the_crypt_s_portal_credits_into_the_deeps() {
    let done = [RAVEN_HILLS_WATCH, THE_GRAVE_ROBBERS, WHAT_THE_DIGGERS_WANT];
    let mut party = in_darkshire(&done);
    let jitters = up_to(&mut party, JITTERS, JITTERS_AT);
    accept(&mut party, jitters, INTO_THE_DEEPS);

    let complete = |events: &[SessionEvent]| {
        events.iter().any(|e| {
            matches!(e, SessionEvent::QuestObjectivesComplete { quest_id } if *quest_id == INTO_THE_DEEPS)
        })
    };
    let events = walk(&mut party, BY_THE_MAUSOLEUM);
    assert!(!complete(&events), "explored short of the portal");
    let events = walk(&mut party, DEEPS_PORTAL);
    assert!(complete(&events), "{events:?}");
}

#[test]
fn the_crowned_orders_read_two_pages_and_morganth_s_notes_exist() {
    let mut realm = realm();
    let (first, next) = pages(&mut realm, CROWNED_ORDERS);
    assert_ne!(first, 0, "the orders are not readable");
    assert_eq!(next, [first + 1, 0], "two pages");
    pages(&mut realm, MORGANTHS_NOTES);
}

#[test]
fn every_band_40_45_story_quest_offers_the_choices_the_gear_table_names() {
    let mut realm = realm();
    for (id, want) in CHOICES_40_45 {
        let t = template(&mut realm, id).unwrap_or_else(|| panic!("quest {id} is not loaded"));
        let got: Vec<u32> = t
            .choices
            .iter()
            .map(|c| c.0)
            .filter(|&item| item != 0)
            .collect();
        assert_eq!(got, want, "quest {id}'s choices");
    }
    for id in 98811..=98814 {
        let t = template(&mut realm, id).unwrap_or_else(|| panic!("quest {id} is not loaded"));
        assert!(
            t.choices.iter().all(|c| c.0 == 0),
            "bounty {id} offers a choice"
        );
    }
}

#[test]
fn band_45_50_s_quests_load_with_their_givers_enders_and_zone() {
    let rows = duskwood_quests();
    let ids: Vec<u32> = rows["quest"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| int(q, "id").unwrap() as u32)
        .filter(|id| (98901..=98914).contains(id))
        .collect();
    assert_eq!(ids, BAND_45_50.map(|q| q.0));
    let mut realm = realm();
    for (id, giver, ender, _) in BAND_45_50 {
        let row = quest_row(id);
        assert_eq!(int(&row, "giver"), Some(giver.into()), "quest {id}'s giver");
        assert_eq!(
            int(&row, "ender"),
            ender.map(i64::from),
            "quest {id}'s ender"
        );
        assert_eq!(int(&row, "zone_or_sort"), Some(10), "quest {id}");
        let kind = row["kind"].as_str().unwrap();
        let want = if id >= 98911 { "bounty" } else { "story" };
        assert_eq!(kind, want, "quest {id}");
        let t = template(&mut realm, id).unwrap_or_else(|| panic!("quest {id} is not loaded"));
        assert_eq!(t.zone_or_sort, 10, "quest {id}");
        assert_eq!(Some(t.title.as_str()), row["title"].as_str(), "quest {id}");
    }
}

#[test]
fn band_45_50_s_chains_hold() {
    for (id, _, _, after) in BAND_45_50 {
        assert_eq!(
            int(&quest_row(id), "after"),
            after.map(i64::from),
            "quest {id}'s `after`"
        );
    }

    // A newcomer gets Dodds' orchard, but neither the camp nor Mor'Ladim.
    let mut party = in_darkshire(&[]);
    let althea = up_to(&mut party, ALTHEA, ALTHEA_AT);
    assert!(!offers(&mut party, althea).contains(&THE_AGENTS_CAMP));
    let sirra = up_to(&mut party, SIRRA, SIRRA_AT);
    assert!(!offers(&mut party, sirra).contains(&MOR_LADIM));
    let dodds = up_to(&mut party, DODDS, DODDS_AT);
    assert!(offers(&mut party, dodds).contains(&98906));

    let mut party = in_darkshire(&[THE_NIGHT_WATCH, TWELVE_YEARS_AGO]);
    let althea = up_to(&mut party, ALTHEA, ALTHEA_AT);
    let offered = offers(&mut party, althea);
    assert!(offered.contains(&THE_AGENTS_CAMP), "{offered:?}");
    assert!(!offered.contains(&THE_GREY_CAPTAIN), "{offered:?}");
    let sirra = up_to(&mut party, SIRRA, SIRRA_AT);
    let offered = offers(&mut party, sirra);
    assert!(offered.contains(&MOR_LADIM), "{offered:?}");
    assert!(!offered.contains(&THE_DAUGHTER_WHO_LIVED), "{offered:?}");
}

#[test]
fn the_grey_captain_s_corpse_gives_the_commission_while_the_grey_captain_wants_it() {
    let mut party = in_darkshire(&[THE_AGENTS_CAMP]);
    let althea = up_to(&mut party, ALTHEA, ALTHEA_AT);
    assert!(offers(&mut party, althea).contains(&THE_GREY_CAPTAIN));
    accept(&mut party, althea, THE_GREY_CAPTAIN);

    let from = [ALTHEA_AT[0] + 10.0, ALTHEA_AT[1], ALTHEA_AT[2]];
    let (events, loot) = slay(&mut party, GREY_CAPTAIN, from);
    assert!(credited(&events, THE_GREY_CAPTAIN), "{events:?}");
    assert!(
        loot.contains(&CROWNED_COMMISSION),
        "his corpse held {loot:?}"
    );
    assert_eq!(carries(&party, CROWNED_COMMISSION), 1);

    let (_, again) = slay(&mut party, GREY_CAPTAIN, from);
    assert!(
        !again.contains(&CROWNED_COMMISSION),
        "a second copy once the quest has the commission: {again:?}"
    );

    let althea = up_to(&mut party, ALTHEA, ALTHEA_AT);
    let events = hand_in(&mut party, althea, THE_GREY_CAPTAIN);
    assert!(completed(&events, THE_GREY_CAPTAIN), "{events:?}");
    assert_eq!(
        carries(&party, CROWNED_COMMISSION),
        0,
        "Althea keeps the commission"
    );
}

#[test]
fn the_crowned_commission_reads_one_page() {
    let (first, next) = pages(&mut realm(), CROWNED_COMMISSION);
    assert_ne!(first, 0, "the commission is not readable");
    assert_eq!(next, [0], "one page");
}

#[test]
fn the_daughter_who_lived_puts_the_skull_in_the_bag_and_ladimore_takes_it() {
    let mut party = in_darkshire(&[MOR_LADIM]);
    let sirra = up_to(&mut party, SIRRA, SIRRA_AT);
    accept(&mut party, sirra, THE_DAUGHTER_WHO_LIVED);
    assert_eq!(
        carries(&party, MOR_LADIMS_SKULL),
        1,
        "Sirra hands over the skull"
    );

    let ladimore = up_to(&mut party, LADIMORE, LADIMORE_AT);
    let events = hand_in(&mut party, ladimore, THE_DAUGHTER_WHO_LIVED);
    assert!(completed(&events, THE_DAUGHTER_WHO_LIVED), "{events:?}");
    assert_eq!(
        carries(&party, MOR_LADIMS_SKULL),
        0,
        "the skull stays with Ladimore"
    );
}

#[test]
fn reaching_the_grave_credits_a_daughter_s_love() {
    let mut party = in_darkshire(&[THE_DAUGHTER_WHO_LIVED]);
    let ladimore = up_to(&mut party, LADIMORE, LADIMORE_AT);
    accept(&mut party, ladimore, A_DAUGHTERS_LOVE);

    let complete = |events: &[SessionEvent]| {
        events.iter().any(|e| {
            matches!(e, SessionEvent::QuestObjectivesComplete { quest_id } if *quest_id == A_DAUGHTERS_LOVE)
        })
    };
    let events = walk(&mut party, SHORT_OF_THE_GRAVE);
    assert!(!complete(&events), "explored short of the grave");
    let events = walk(&mut party, LADIMORES_GRAVE);
    assert!(complete(&events), "{events:?}");
}

#[test]
fn every_band_45_50_story_quest_offers_the_choices_the_gear_table_names() {
    let mut realm = realm();
    for (id, want) in CHOICES_45_50 {
        let t = template(&mut realm, id).unwrap_or_else(|| panic!("quest {id} is not loaded"));
        let got: Vec<u32> = t
            .choices
            .iter()
            .map(|c| c.0)
            .filter(|&item| item != 0)
            .collect();
        assert_eq!(got, want, "quest {id}'s choices");
    }
    for id in 98911..=98914 {
        let t = template(&mut realm, id).unwrap_or_else(|| panic!("quest {id} is not loaded"));
        assert!(
            t.choices.iter().all(|c| c.0 == 0),
            "bounty {id} offers a choice"
        );
    }
}

#[test]
fn the_agents_camp_has_two_tents_and_a_fire_round_the_captain() {
    let rows = duskwood_quests();
    let objects = rows["gameobject"].as_array().unwrap();
    for guid in CAMP_OBJECTS {
        let row = objects
            .iter()
            .find(|o| int(o, "guid") == Some(guid.into()))
            .unwrap_or_else(|| panic!("gameobject {guid}"));
        assert_eq!(int(row, "map"), Some(0), "gameobject {guid}'s map");
        let at = row["position"].as_array().unwrap();
        let (x, y) = (at[0].as_float().unwrap(), at[1].as_float().unwrap());
        let off = (x - CAMP_AT[0] as f64).hypot(y - CAMP_AT[1] as f64);
        assert!(
            off < 25.0,
            "gameobject {guid} stands {off} yd from the camp"
        );
    }
    let displays: Vec<i64> = CAMP_OBJECTS
        .iter()
        .map(|&g| {
            let row = objects
                .iter()
                .find(|o| int(o, "guid") == Some(g.into()))
                .unwrap();
            int(row, "display").unwrap()
        })
        .collect();
    assert_eq!(displays, [6738, 6738, 192], "two tents and a campfire");
}

/// Spawns a level 1 `creature` 3 yd west of `from`, for this character alone, and fights it from
/// `from` until it dies, then loots it into the bags: the events from the spawn on, and the items
/// its corpse held.
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
