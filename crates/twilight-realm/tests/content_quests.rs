//! `content/quests.toml` against the rest of the realm's content: every quest's creatures and
//! items are ones the realm knows, every explore spot lies in The Barrens and every ender stands
//! somewhere, so no objective can never be met.

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
    /// The Hub NPC who takes it in, when not the giver.
    #[serde(default)]
    ender: Option<u32>,
    level: u8,
    title: String,
    details: String,
    money: u32,
    tokens: u32,
    #[serde(default)]
    kill: Vec<(u32, u32)>,
    #[serde(default)]
    collect: Vec<Collect>,
    #[serde(default)]
    explore: Option<Explore>,
    #[serde(default)]
    discover: Vec<u32>,
    /// An Advancement Quest asks for nothing but a word with its giver.
    #[serde(default)]
    advancement: Option<toml::Value>,
    /// Listed in the Calling Choice popup.
    #[serde(default)]
    calling_choice: bool,
}

#[derive(Deserialize)]
struct Explore {
    place: String,
    position: [f32; 2],
    radius: f32,
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
            !q.kill.is_empty()
                || !q.collect.is_empty()
                || q.explore.is_some()
                || !q.discover.is_empty()
                || q.ender.is_some_and(|e| e != q.giver)
                || q.advancement.is_some(),
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
            // A once-per-character quest pays what it is worth, never nothing.
            "once" if q.tokens > 0 => q.tokens,
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
fn the_quests_cover_levels_one_to_thirty() {
    let levels: Vec<u8> = content().quest.iter().map(|q| q.level).collect();
    assert_eq!(levels.iter().min(), Some(&2));
    assert!(levels.iter().max().is_some_and(|&l| l >= 30));
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
        .filter(|q| [90101, 90102].contains(&q.giver))
        .filter(|q| !q.kill.is_empty() || !q.collect.is_empty())
        .collect();
    assert!(!low_quests.is_empty());
    for q in low_quests {
        let targets: Vec<u32> = q
            .kill
            .iter()
            .map(|k| k.0)
            .chain(q.collect.iter().flat_map(|c| c.from.iter().copied()))
            .collect();
        // A camp is where its quest says when its own creatures (the first member's kind) are the
        // quest's targets: the wild camps blend a few of another kind in, which no quest chases.
        // A target that leads no camp is found where it stands among the others.
        let has = |c: &&serde_json::Value, any: bool| {
            let mut kinds = c["members"]
                .as_array()
                .unwrap()
                .iter()
                .map(|m| m["entry"].as_u64().unwrap() as u32);
            if any {
                kinds.any(|e| targets.contains(&e))
            } else {
                kinds.next().is_some_and(|e| targets.contains(&e))
            }
        };
        let led = camps.iter().any(|c| has(&c, false));
        let bearings: Vec<f32> = camps
            .iter()
            .filter(|c| has(c, !led))
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

#[derive(Deserialize)]
struct Npc {
    entry: u32,
    npc_flags: u32,
}

#[derive(Deserialize)]
struct Hub {
    npc: Vec<Npc>,
}

#[test]
fn every_ender_is_a_quest_giver_with_a_name() {
    let hub: Hub = toml::from_str(include_str!("../content/hub.toml")).expect("hub.toml parses");
    let mut realm = realm();
    for q in content().quest {
        let ender = q.ender.unwrap_or(q.giver);
        assert!(
            hub.npc
                .iter()
                .any(|n| n.entry == ender && n.npc_flags & 0x2 != 0),
            "quest {}: ender {ender} is no quest giver in hub.toml",
            q.id
        );
        assert!(creature_named(&mut realm, ender), "ender {ender}");
    }
}

#[test]
fn every_explore_spot_is_a_named_place_in_the_barrens() {
    // The Barrens as its spawns span it.
    let barrens: toml::Value =
        toml::from_str(include_str!("../content/barrens.toml")).expect("barrens.toml parses");
    let at = |s: &toml::Value, i: usize| s[i].as_float().expect("a coordinate") as f32;
    let spawns = barrens["spawns"].as_array().expect("spawns");
    let span = |i: usize| {
        let all = spawns.iter().map(|s| at(s, i));
        let lo = all.clone().fold(f32::INFINITY, f32::min);
        (lo, all.fold(f32::NEG_INFINITY, f32::max))
    };
    let ((x0, x1), (y0, y1)) = (span(4), span(5));
    let spots: Vec<(u32, Explore)> = content()
        .quest
        .into_iter()
        .filter_map(|q| Some((q.id, q.explore?)))
        .collect();
    assert!(!spots.is_empty());
    for (id, e) in spots {
        let [x, y] = e.position;
        assert!(
            (x0..=x1).contains(&x) && (y0..=y1).contains(&y),
            "quest {id}: {} lies outside The Barrens",
            e.place
        );
        assert!(!e.place.trim().is_empty(), "quest {id}");
        assert!(
            (5.0..=100.0).contains(&e.radius),
            "quest {id}: radius {}",
            e.radius
        );
    }
}

#[test]
fn exactly_the_five_northshire_advancement_quests_are_calling_choices() {
    let chosen: Vec<u32> = content()
        .quest
        .into_iter()
        .filter(|q| q.calling_choice)
        .map(|q| q.id)
        .collect();
    assert_eq!(chosen, [64009, 64010, 64011, 64012, 64013]);
}

/// Every Elwynn story quest (#448) offers a choice of sheet rows the character may wear by the
/// time the quest is done, and pays no Token.
#[test]
fn every_elwynn_story_quest_offers_gear_it_can_be_worn_at_and_pays_no_token() {
    let quests: toml::Value =
        toml::from_str(include_str!("../content/quests_elwynn.toml")).expect("parses");
    let gear: toml::Value = toml::from_str(include_str!("../content/gear.toml")).expect("parses");
    let required: std::collections::HashMap<i64, i64> = gear["item"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["entry"].as_integer().unwrap(),
                i.get("required_level")
                    .and_then(|l| l.as_integer())
                    .unwrap_or(0),
            )
        })
        .collect();
    let mut checked = 0;
    for q in quests["quest"].as_array().unwrap() {
        let id = q["id"].as_integer().unwrap();
        if q["kind"].as_str() != Some("story") {
            continue;
        }
        assert_eq!(q["tokens"].as_integer(), Some(0), "quest {id} pays a Token");
        if id < 94101 {
            continue;
        }
        let level = q["level"].as_integer().unwrap();
        let choices = q
            .get("choices")
            .and_then(|c| c.as_array())
            .unwrap_or_else(|| panic!("quest {id} offers no choice"));
        assert!(!choices.is_empty(), "quest {id}");
        for c in choices {
            let entry = c.as_integer().unwrap();
            let need = required
                .get(&entry)
                .unwrap_or_else(|| panic!("quest {id}: choice {entry} is no sheet row"));
            assert!(
                *need <= level,
                "quest {id}: choice {entry} needs level {need}"
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 27);
}
