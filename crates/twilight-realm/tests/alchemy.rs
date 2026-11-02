//! Alchemy tables in the realm (#542, ADR 0025): the three alchemists open them, makes resolve and
//! roll their Toxic, Herb Notes fill, grades are learned or bought as Rumours, sludge is sold and
//! every make goes in the Brew Log.

mod common;

use benilla_protocol::{AlchemyGrade, AlchemyGradePage, SessionEvent, START_ELWYNN};
use common::*;
use twilight_realm::{AlchemyAction, Realm, RealmCommand};

const SISTER_MARLA: u32 = 190_075;
const ALWEN_CARROW: u32 = 190_076;
const TABLE_LINE: &str = "Let me use your alchemy table.";

const PEACEBLOOM: u32 = 2447;
const BRIARTHORN: u32 = 2450;
const SLUDGE: u32 = 963_000;
const MINOR_HEALING: u32 = 963_100;
/// Restorative 10 on the Tincture ladder: Minor x2, clean (963500 + 4 x 8).
const MINOR_RESTORATIVE_TINCTURE: u32 = 963_532;
const PLAIN_FORTIFYING_TINCTURE_TOXIC_7: u32 = 963_747;

/// A new Elwynn character's save, its guid.
fn new_save() -> (u64, String) {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Mara", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    assert_eq!(create_result(&mut maker), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    (guid, maker.export_character(guid).unwrap())
}

/// `blob` with its `position` line set to `at`.
fn placed(blob: &str, at: [f32; 3]) -> String {
    blob.lines()
        .map(|l| {
            if l.starts_with("position = ") {
                format!("position = [{:.3}, {:.3}, {:.3}]", at[0], at[1], at[2])
            } else {
                l.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A character beside the alchemist `npc`, holding `satchel` in the Satchel, with `top` (top-level
/// save keys) and `money` copper, in a realm seeded `seed`.
fn beside(npc: u32, satchel: &[(u32, u32)], top: &str, money: u32, seed: u64) -> (Realm, u64) {
    let (guid, blob) = new_save();
    let [x, y, z] = hub_home(hub_guid(npc));
    let blob = placed(&blob, [x + 1.5, y, z]);
    let mut blob = blob
        .lines()
        .map(|l| {
            if l.starts_with("money = ") {
                format!("money = {money}")
            } else {
                l.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    blob = format!("{top}\n{blob}");
    for (entry, count) in satchel {
        blob.push_str(&format!(
            "\n[[satchel]]\nentry = {entry}\ncount = {count}\n"
        ));
    }
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm);
    (realm, guid)
}

/// Gossip at `npc`: its menu's lines.
fn hello(realm: &mut Realm, npc: u32) -> Vec<(u32, String)> {
    realm.handle(RealmCommand::GossipHello {
        guid: hub_guid(npc),
    });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => {
                Some(options.into_iter().map(|o| (o.index, o.message)).collect())
            }
            _ => None,
        })
        .expect("the alchemist has a menu")
}

/// Opens `npc`'s table: its events.
fn open(realm: &mut Realm, npc: u32) -> Vec<SessionEvent> {
    let (option, _) = hello(realm, npc)
        .into_iter()
        .find(|(_, m)| m == TABLE_LINE)
        .expect("the table line");
    realm.handle(RealmCommand::GossipSelectOption {
        guid: hub_guid(npc),
        option,
    });
    drain(realm)
}

fn act(realm: &mut Realm, action: AlchemyAction) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::Alchemy(action));
    drain(realm)
}

fn make(realm: &mut Realm, make: &str, items: Vec<u32>, target: &str) -> Vec<SessionEvent> {
    act(
        realm,
        AlchemyAction::Make {
            make: make.to_owned(),
            items,
            target: target.to_owned(),
        },
    )
}

/// The chat lines in `events`.
fn lines(events: &[SessionEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Chat(m) => Some(m.text.clone()),
            _ => None,
        })
        .collect()
}

/// The saved character.
fn saved(realm: &Realm, guid: u64) -> toml::Table {
    realm.export_character(guid).unwrap().parse().unwrap()
}

/// How many of `entry` the saved character holds in the Satchel, and in the bags.
fn held(realm: &Realm, guid: u64, entry: u32) -> (i64, i64) {
    let save = saved(realm, guid);
    let count = |key: &str| {
        save.get(key)
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter(|r| r.get("entry").and_then(|e| e.as_integer()) == Some(entry.into()))
            .map(|r| r.get("count").and_then(|c| c.as_integer()).unwrap_or(1))
            .sum::<i64>()
    };
    (count("satchel"), count("items"))
}

/// The last bench's targets in `events`.
fn targets(events: &[SessionEvent]) -> Vec<AlchemyGrade> {
    events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::AlchemyBench { targets, .. } => Some(targets.clone()),
            _ => None,
        })
        .expect("an AlchemyBench")
}

#[test]
fn sister_marla_opens_a_bench_with_only_the_potion_open_and_says_her_line_once() {
    let (mut realm, _) = beside(SISTER_MARLA, &[], "", 0, 1);
    let events = open(&mut realm, SISTER_MARLA);
    let open_makes: Vec<(String, bool)> = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::AlchemyBench { makes, .. } => {
                Some(makes.iter().map(|m| (m.key.clone(), m.open)).collect())
            }
            _ => None,
        })
        .expect("an AlchemyBench");
    assert_eq!(open_makes.len(), 5);
    assert!(
        open_makes.iter().all(|(k, o)| *o == (k == "potion")),
        "{open_makes:?}"
    );
    let said = |events: &[SessionEvent]| {
        events
            .iter()
            .filter(|e| matches!(e, SessionEvent::Chat(m) if m.text.contains("Two to four herbs in the Mortar")))
            .count()
    };
    assert_eq!(said(&events), 1);
    assert_eq!(said(&open(&mut realm, SISTER_MARLA)), 0);
}

#[test]
fn two_peacebloom_brew_a_minor_healing_potion_and_open_the_still() {
    let (mut realm, guid) = beside(SISTER_MARLA, &[(PEACEBLOOM, 4)], "", 0, 1);
    open(&mut realm, SISTER_MARLA);
    let events = make(&mut realm, "potion", vec![PEACEBLOOM; 2], "");
    assert_eq!(held(&realm, guid, MINOR_HEALING), (0, 1));
    let save = saved(&realm, guid);
    let notes = save["alchemy_notes"].as_array().unwrap();
    assert_eq!(notes[0].as_array().unwrap()[1].as_integer(), Some(2));
    assert!(save["alchemy_makes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m.as_str() == Some("tincture")));
    let grades = save["alchemy_grades"].as_array().unwrap();
    assert!(grades.iter().any(|g| g["family"].as_str() == Some("potion")
        && g["key"].as_str() == Some("Restorative")
        && g["grade"].as_integer() == Some(0)
        && g["learned"].as_bool() == Some(true)));
    let (pages, log): (Vec<AlchemyGradePage>, usize) = events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::AlchemyJournal { grades, log, .. } => Some((grades.clone(), log.len())),
            _ => None,
        })
        .expect("an AlchemyJournal");
    let page = pages
        .iter()
        .find(|p| p.family == "potion" && p.key == "Restorative" && p.grade == 0)
        .unwrap();
    assert!(page.learned);
    assert_eq!(page.threshold, 4);
    assert_eq!(log, 1);
    assert!(lines(&events)
        .iter()
        .any(|l| l == "Tinctures are open: 5 herbs at the Still."));
}

#[test]
fn a_tincture_goes_to_the_alchemy_satchel() {
    let top = "alchemy_makes = [\"tincture\"]";
    let (mut realm, guid) = beside(SISTER_MARLA, &[(PEACEBLOOM, 5)], top, 0, 1);
    open(&mut realm, SISTER_MARLA);
    make(&mut realm, "tincture", vec![PEACEBLOOM; 5], "");
    assert_eq!(held(&realm, guid, MINOR_RESTORATIVE_TINCTURE), (1, 0));
}

#[test]
fn briarthorn_at_the_still_sometimes_spoils_and_sometimes_carries_toxic_7() {
    let top = "alchemy_makes = [\"tincture\"]";
    let (mut sludge, mut tincture) = (0, 0);
    for seed in 1..=10 {
        let (mut realm, guid) = beside(SISTER_MARLA, &[(BRIARTHORN, 5)], top, 0, seed);
        open(&mut realm, SISTER_MARLA);
        make(&mut realm, "tincture", vec![BRIARTHORN; 5], "");
        sludge += held(&realm, guid, SLUDGE).0;
        tincture += held(&realm, guid, PLAIN_FORTIFYING_TINCTURE_TOXIC_7).0;
    }
    assert!(sludge >= 1, "no sludge in ten seeds");
    assert!(tincture >= 1, "no Tincture carrying Toxic 7 in ten seeds");
}

#[test]
fn rumours_are_sold_by_the_tables_whose_tiers_hold_them() {
    let buy = |realm: &mut Realm| {
        act(
            realm,
            AlchemyAction::BuyRumour {
                family: "potion".to_owned(),
                key: "Restorative".to_owned(),
                grade: 1,
            },
        )
    };
    let (mut realm, guid) = beside(SISTER_MARLA, &[], "", 10_000, 1);
    open(&mut realm, SISTER_MARLA);
    buy(&mut realm);
    assert_eq!(saved(&realm, guid)["money"].as_integer(), Some(10_000));
    let (mut realm, guid) = beside(ALWEN_CARROW, &[], "", 10_000, 1);
    open(&mut realm, ALWEN_CARROW);
    let events = buy(&mut realm);
    assert_eq!(saved(&realm, guid)["money"].as_integer(), Some(4_000));
    assert!(lines(&events)
        .iter()
        .any(|l| l == "Rumour: Lesser Healing Potion needs Restorative 12."));
    let t = targets(&events)
        .into_iter()
        .find(|t| t.family == "potion" && t.key == "Restorative" && t.grade == 1)
        .unwrap();
    assert!(t.rumoured && !t.learned);
    assert_eq!(t.threshold, 12);
}

#[test]
fn three_murky_sludge_sell_for_15_copper() {
    let (mut realm, guid) = beside(SISTER_MARLA, &[(SLUDGE, 3)], "", 0, 1);
    open(&mut realm, SISTER_MARLA);
    let events = act(&mut realm, AlchemyAction::SellSludge);
    assert_eq!(saved(&realm, guid)["money"].as_integer(), Some(15));
    assert_eq!(held(&realm, guid, SLUDGE), (0, 0));
    assert!(lines(&events)
        .iter()
        .any(|l| l == "Sold 3 Murky Sludge for 15c."));
}

#[test]
fn the_alchemy_fields_survive_a_save_round_trip() {
    let (mut realm, guid) = beside(SISTER_MARLA, &[(PEACEBLOOM, 2)], "", 0, 1);
    open(&mut realm, SISTER_MARLA);
    make(&mut realm, "potion", vec![PEACEBLOOM; 2], "");
    let blob = realm.export_character(guid).unwrap();
    let mut again = Realm::with_seed(1);
    again.import_character(&blob).unwrap();
    let (before, after) = (saved(&realm, guid), saved(&again, guid));
    for key in [
        "alchemy_notes",
        "alchemy_grades",
        "alchemy_makes",
        "alchemy_tables",
        "alchemy_last",
        "brew_log",
    ] {
        assert!(before.contains_key(key), "{key} is saved");
        assert_eq!(before.get(key), after.get(key), "{key}");
    }
}
