//! Redridge's dungeon quests (spec #595, `content/quests_redridge_dungeons.toml`), walked on the
//! shipped content. The Tower of Ilgalar (#622): Marshal Marris sends the character up the tower
//! to kill Morganth and bring back his papers, Toren reads the cipher, and Toren's translation of
//! page one goes to Sirra Von'Indi in Darkshire.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

const INTO_ILGALAR: u32 = 98202;
const MORGANTH_QUEST: u32 = 98501;
const THE_CIPHER: u32 = 98502;
const PAGE_ONE: u32 = 98503;

/// Every Tower of Ilgalar quest: `(id, level, giver, ender, after, money)`, the ender `None` when
/// it is the giver.
const ILGALAR: [(u32, i64, u32, Option<u32>, u32, i64); 3] = [
    (MORGANTH_QUEST, 38, MARRIS, None, INTO_ILGALAR, 1710),
    (THE_CIPHER, 39, MARRIS, Some(TOREN), MORGANTH_QUEST, 0),
    (PAGE_ONE, 40, TOREN, Some(SIRRA), THE_CIPHER, 0),
];

/// Each story quest's green choices, from the band's Quest Lines (#605).
const CHOICES: [(u32, &[u32]); 2] = [
    (THE_CIPHER, &[903224, 903226, 903228]),
    (PAGE_ONE, &[903235, 903237, 903239]),
];

/// The Tower of Ilgalar set's four Chests, cloth, leather, mail and plate (#644, `gear.csv`).
const ILGALAR_CHESTS: [u32; 4] = [905160, 905161, 905162, 905163];

/// The people the quests pass through: Marris and Toren in Lakeshire (`content/redridge.toml`,
/// `content/hub.toml`), Sirra in Darkshire (`content/duskwood.toml`).
const MARRIS: u32 = 382;
const MARRIS_AT: [f32; 3] = [-9284.03, -2298.16, 67.577];
const TOREN: u32 = 190083;
const TOREN_AT: [f32; 3] = [-9228.0, -2162.0, 63.73];
const SIRRA: u32 = 268;
const SIRRA_AT: [f32; 3] = [-10573.9, -1127.32, 29.167];

/// Morganth, the tower's last boss, and his creature row (`content/ilgalar.toml`).
const MORGANTH: u32 = 97244;
const MORGANTH_GUID: i64 = 974114;

const FAILURE_NOTES: u32 = 980381;
const LETTERS: u32 = 980382;
const TRANSLATION: u32 = 980383;

/// The quest 98202 follows.
const BEFORE_INTO_ILGALAR: u32 = 98109;

fn dungeon_quests() -> toml::Table {
    include_str!("../content/quests_redridge_dungeons.toml")
        .parse()
        .unwrap()
}

fn tower() -> toml::Table {
    include_str!("../content/ilgalar.toml").parse().unwrap()
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

/// A Zealot (Tier 2, so the quests are offered) at level 40 by Marshal Marris in Lakeshire,
/// having handed in `done` and carrying nothing.
fn in_lakeshire(done: &[u32]) -> Party {
    let done = done.to_vec();
    let mut party = Party::edited(
        Realm::with_seed(1),
        &[40],
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
    walk(&mut party, MARRIS_AT);
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
fn ilgalar_s_quests_load_with_their_givers_enders_and_zone() {
    let rows = dungeon_quests();
    let ids: Vec<u32> = rows["quest"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| int(q, "id").unwrap() as u32)
        .filter(|id| (98501..=98503).contains(id))
        .collect();
    assert_eq!(ids, ILGALAR.map(|q| q.0));
    let mut realm = realm();
    for (id, level, giver, ender, after, money) in ILGALAR {
        let row = quest_row(id);
        assert_eq!(int(&row, "giver"), Some(giver.into()), "quest {id}'s giver");
        assert_eq!(
            int(&row, "ender"),
            ender.map(i64::from),
            "quest {id}'s ender"
        );
        assert_eq!(
            int(&row, "after"),
            Some(after.into()),
            "quest {id}'s `after`"
        );
        assert_eq!(row["kind"].as_str(), Some("story"), "quest {id}");
        assert_eq!(int(&row, "level"), Some(level), "quest {id}");
        assert_eq!(int(&row, "money"), Some(money), "quest {id}");
        assert_eq!(int(&row, "zone_or_sort"), Some(44), "quest {id}");
        let t = template(&mut realm, id).unwrap_or_else(|| panic!("quest {id} is not loaded"));
        assert_eq!(t.zone_or_sort, 44, "quest {id}");
        assert_eq!(Some(t.title.as_str()), row["title"].as_str(), "quest {id}");
    }
}

#[test]
fn marris_offers_morganth_only_once_into_ilgalar_is_handed_in() {
    let mut party = in_lakeshire(&[BEFORE_INTO_ILGALAR]);
    let marris = up_to(&mut party, MARRIS, MARRIS_AT);
    let offered = offers(&mut party, marris);
    assert!(offered.contains(&INTO_ILGALAR), "{offered:?}");
    assert!(!offered.contains(&MORGANTH_QUEST), "{offered:?}");

    let mut party = in_lakeshire(&[BEFORE_INTO_ILGALAR, INTO_ILGALAR]);
    let marris = up_to(&mut party, MARRIS, MARRIS_AT);
    let offered = offers(&mut party, marris);
    assert!(offered.contains(&MORGANTH_QUEST), "{offered:?}");
    assert!(!offered.contains(&THE_CIPHER), "{offered:?}");
}

#[test]
fn morganth_s_corpse_holds_the_notes_and_letters_that_his_quest_wants() {
    let tower = tower();
    let row = tower["creature"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| int(c, "guid") == Some(MORGANTH_GUID))
        .expect("Morganth's creature row");
    assert_eq!(int(row, "entry"), Some(MORGANTH.into()));
    let drop: Vec<i64> = row["drop"]
        .as_array()
        .expect("Morganth's drop")
        .iter()
        .map(|i| i.as_integer().unwrap())
        .collect();
    assert_eq!(drop, [i64::from(FAILURE_NOTES), i64::from(LETTERS)]);
    let others = tower["creature"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| int(c, "guid") != Some(MORGANTH_GUID))
        .filter(|c| c.get("drop").is_some())
        .count();
    assert_eq!(others, 0, "only Morganth holds the papers");

    let t = template(&mut realm(), MORGANTH_QUEST).expect("98501 is loaded");
    let wants: Vec<(u32, u32, u32, u32)> = t
        .objectives
        .iter()
        .map(|o| (o.creature_or_go, o.required_count, o.item_id, o.item_count))
        .filter(|o| *o != (0, 0, 0, 0))
        .collect();
    assert_eq!(wants, [(MORGANTH, 1, FAILURE_NOTES, 1), (0, 0, LETTERS, 1)]);
}

#[test]
fn morganth_offers_exactly_the_tower_of_ilgalar_set_s_four_chests() {
    assert_eq!(
        choices(&mut realm(), MORGANTH_QUEST),
        ILGALAR_CHESTS,
        "98501's choices"
    );
}

#[test]
fn the_cipher_puts_both_papers_back_in_the_bag_and_toren_takes_them() {
    let mut party = in_lakeshire(&[BEFORE_INTO_ILGALAR, INTO_ILGALAR, MORGANTH_QUEST]);
    let marris = up_to(&mut party, MARRIS, MARRIS_AT);
    accept(&mut party, marris, THE_CIPHER);
    assert_eq!(
        carries(&party, FAILURE_NOTES),
        1,
        "Marris hands back the notes"
    );
    assert_eq!(carries(&party, LETTERS), 1, "Marris hands back the letters");

    let toren = up_to(&mut party, TOREN, TOREN_AT);
    let events = hand_in(&mut party, toren, THE_CIPHER);
    assert!(completed(&events, THE_CIPHER), "{events:?}");
    assert_eq!(carries(&party, FAILURE_NOTES), 0, "Toren keeps the notes");
    assert_eq!(carries(&party, LETTERS), 0, "Toren keeps the letters");
}

#[test]
fn toren_offers_page_one_once_the_cipher_is_handed_in_and_sirra_takes_it() {
    let mut party = in_lakeshire(&[BEFORE_INTO_ILGALAR, INTO_ILGALAR, MORGANTH_QUEST]);
    let toren = up_to(&mut party, TOREN, TOREN_AT);
    let offered = offers(&mut party, toren);
    assert!(!offered.contains(&PAGE_ONE), "{offered:?}");

    let mut party = in_lakeshire(&[
        BEFORE_INTO_ILGALAR,
        INTO_ILGALAR,
        MORGANTH_QUEST,
        THE_CIPHER,
    ]);
    let toren = up_to(&mut party, TOREN, TOREN_AT);
    let offered = offers(&mut party, toren);
    assert!(offered.contains(&PAGE_ONE), "{offered:?}");
    accept(&mut party, toren, PAGE_ONE);
    assert_eq!(
        carries(&party, TRANSLATION),
        1,
        "Toren gives his translation"
    );

    let sirra = up_to(&mut party, SIRRA, SIRRA_AT);
    let events = hand_in(&mut party, sirra, PAGE_ONE);
    assert!(completed(&events, PAGE_ONE), "{events:?}");
    assert_eq!(
        carries(&party, TRANSLATION),
        0,
        "the translation stays with Sirra"
    );
}

#[test]
fn every_story_quest_offers_the_choices_the_gear_table_names() {
    let mut realm = realm();
    for (id, want) in CHOICES {
        assert_eq!(choices(&mut realm, id), want, "quest {id}'s choices");
    }
}
