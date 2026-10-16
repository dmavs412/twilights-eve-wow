//! `content/quests.toml` against the rest of the realm's content: every quest's creatures and
//! items are ones the realm knows, so no objective can never be met.

mod common;

use benilla_protocol::SessionEvent;
use common::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct Collect {
    item: u32,
    count: u32,
    chance: u32,
    from: Vec<u32>,
}

#[derive(Deserialize)]
struct Quest {
    id: u32,
    /// "hunting", "repeatable" or "advancement".
    kind: String,
    /// A Hub NPC's entry (`content/hub.toml`).
    giver: u32,
    level: u8,
    title: String,
    details: String,
    money: u32,
    tokens: u32,
    #[serde(default)]
    kill: Vec<(u32, u32)>,
    #[serde(default)]
    collect: Vec<Collect>,
    /// An Advancement Quest asks for nothing but a word with its giver.
    #[serde(default)]
    advancement: Option<toml::Value>,
}

#[derive(Deserialize)]
struct File {
    quest: Vec<Quest>,
}

fn content() -> File {
    toml::from_str(include_str!("../content/quests.toml")).expect("quests.toml parses")
}

fn creature_named(realm: &mut Realm, entry: u32) -> bool {
    realm.handle(RealmCommand::CreatureQuery { entry });
    drain(realm).into_iter().any(
        |e| matches!(e, SessionEvent::CreatureName { entry: e, name: Some(_), .. } if e == entry),
    )
}

fn item_known(realm: &mut Realm, entry: u32) -> bool {
    realm.handle(RealmCommand::ItemQuery { entry });
    drain(realm)
        .into_iter()
        .any(|e| matches!(e, SessionEvent::ItemTemplate { entry: e, info: Some(_) } if e == entry))
}

#[test]
fn every_objective_names_a_creature_and_an_item_the_realm_has() {
    let mut realm = realm();
    for q in content().quest {
        assert!(
            !q.kill.is_empty() || !q.collect.is_empty() || q.advancement.is_some(),
            "quest {}",
            q.id
        );
        for (entry, count) in &q.kill {
            assert!(
                creature_named(&mut realm, *entry),
                "quest {}: creature {entry}",
                q.id
            );
            // The log's kill counters are 6 bits wide.
            assert!((1..64).contains(count), "quest {}", q.id);
        }
        for c in &q.collect {
            assert!(
                item_known(&mut realm, c.item),
                "quest {}: item {}",
                q.id,
                c.item
            );
            assert!(
                (1..=100).contains(&c.chance) && c.count > 0,
                "quest {}",
                q.id
            );
            for entry in &c.from {
                assert!(
                    creature_named(&mut realm, *entry),
                    "quest {}: creature {entry}",
                    q.id
                );
            }
        }
    }
}

#[test]
fn every_quest_names_its_kind_and_pays_its_kinds_tokens() {
    for q in content().quest {
        let tokens = match q.kind.as_str() {
            "hunting" => 1,
            "repeatable" => 3,
            "advancement" => 0,
            other => panic!("quest {}: kind {other:?}", q.id),
        };
        assert_eq!(q.tokens, tokens, "quest {} ({})", q.id, q.kind);
        assert_eq!(
            q.kind == "advancement",
            q.advancement.is_some(),
            "quest {}",
            q.id
        );
    }
}

#[test]
fn every_quest_has_its_template_and_every_giver_its_name() {
    let mut realm = realm();
    let file = content();
    for q in &file.quest {
        realm.handle(RealmCommand::QuestQuery { quest: q.id });
        let t = drain(&mut realm)
            .into_iter()
            .find_map(|e| match e {
                SessionEvent::QuestTemplate(t) => Some(t),
                _ => None,
            })
            .unwrap_or_else(|| panic!("quest {}'s template", q.id));
        assert_eq!(
            (t.title.as_str(), t.level),
            (q.title.as_str(), u32::from(q.level))
        );
        assert_eq!(t.money, q.money as i32);
        assert!(creature_named(&mut realm, q.giver), "giver {}", q.giver);
    }
}

#[test]
fn the_quests_cover_levels_one_to_twenty_four() {
    let levels: Vec<u8> = content().quest.iter().map(|q| q.level).collect();
    assert_eq!(levels.iter().min(), Some(&2));
    assert!(levels.iter().max().is_some_and(|&l| l >= 24));
}

#[test]
fn the_token_is_a_known_item() {
    let mut realm = realm();
    assert!(item_known(&mut realm, 90001));
}

/// The compass words a quest's details may use, as bearings: degrees from north (+x) toward west
/// (+y), the way WoW's map coordinates run.
const COMPASS: [(&str, f32); 8] = [
    ("north", 0.0),
    ("north-west", 45.0),
    ("west", 90.0),
    ("south-west", 135.0),
    ("south", 180.0),
    ("south-east", 225.0),
    ("east", 270.0),
    ("north-east", 315.0),
];

/// How far off a compass word may be and still point the way: half a quarter turn either side.
const POINTS_THE_WAY_DEG: f32 = 45.0;

#[test]
fn the_low_quests_point_the_way_to_their_camps() {
    let low: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/twilights-eve/crossroads_low_camps.json"
    ))
    .unwrap();
    let camps = low["camps"].as_array().unwrap();
    let town = twilight_realm::CROSSROADS;
    let off = |a: f32, b: f32| {
        let d = (a - b).rem_euclid(360.0);
        d.min(360.0 - d)
    };
    let low_quests: Vec<Quest> = content()
        .quest
        .into_iter()
        .filter(|q| q.level <= 6)
        .collect();
    assert!(!low_quests.is_empty());
    for q in low_quests {
        let targets: Vec<u32> = q
            .kill
            .iter()
            .map(|k| k.0)
            .chain(q.collect.iter().flat_map(|c| c.from.iter().copied()))
            .collect();
        let bearings: Vec<f32> = camps
            .iter()
            .filter(|c| {
                c["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|m| targets.contains(&(m["entry"].as_u64().unwrap() as u32)))
            })
            .map(|c| {
                let dx = c["center"][0].as_f64().unwrap() as f32 - town[0];
                let dy = c["center"][1].as_f64().unwrap() as f32 - town[1];
                dy.atan2(dx).to_degrees().rem_euclid(360.0)
            })
            .collect();
        assert!(
            !bearings.is_empty(),
            "quest {}: no low camp of its targets",
            q.id
        );
        let details = q.details.to_lowercase();
        let words: Vec<f32> = details
            .split(|ch: char| !(ch.is_ascii_alphabetic() || ch == '-'))
            .filter_map(|w| COMPASS.iter().find(|(name, _)| *name == w).map(|c| c.1))
            .collect();
        for b in &bearings {
            assert!(
                words.iter().any(|w| off(*w, *b) <= POINTS_THE_WAY_DEG),
                "quest {}: a camp lies at {b:.0} deg, and \"{}\" does not say so",
                q.id,
                q.details
            );
        }
        for w in &words {
            assert!(
                bearings.iter().any(|b| off(*w, *b) <= POINTS_THE_WAY_DEG),
                "quest {}: \"{}\" points at {w:.0} deg, where none of its camps is",
                q.id,
                q.details
            );
        }
    }
}
