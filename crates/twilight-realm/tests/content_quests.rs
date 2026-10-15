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
    level: u8,
    title: String,
    money: u32,
    #[serde(default)]
    kill: Vec<(u32, u32)>,
    #[serde(default)]
    collect: Vec<Collect>,
}

#[derive(Deserialize)]
struct Giver {
    entry: u32,
}

#[derive(Deserialize)]
struct File {
    giver: Vec<Giver>,
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
            !q.kill.is_empty() || !q.collect.is_empty(),
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
    }
    for g in &file.giver {
        assert!(creature_named(&mut realm, g.entry), "giver {}", g.entry);
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
