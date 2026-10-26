//! `content/herbs.toml` against the roster of spec #435: the plants of each zone, one timed
//! legendary per zone, an item row for everything it hands out, and the nine perks.

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize)]
struct Plant {
    zone: String,
    item: u32,
    rarity: String,
    level: u32,
    #[serde(default)]
    timed: bool,
    #[serde(rename = "use")]
    use_: Option<String>,
}

#[derive(Deserialize)]
struct Ref {
    item: u32,
}

#[derive(Deserialize)]
struct Perk {
    kind: String,
}

#[derive(Deserialize)]
struct Item {
    entry: u32,
    #[serde(default)]
    journal: String,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize)]
struct Herbs {
    plant: Vec<Plant>,
    tool: Vec<Ref>,
    consumable: Vec<Ref>,
    perk: Vec<Perk>,
    item: Vec<Item>,
}

fn herbs() -> Herbs {
    toml::from_str(include_str!("../content/herbs.toml")).unwrap()
}

#[test]
fn the_roster_has_44_rows_across_four_zones() {
    let mut per_zone: HashMap<String, usize> = HashMap::new();
    for p in herbs().plant {
        *per_zone.entry(p.zone).or_default() += 1;
    }
    assert_eq!(per_zone["elwynn"], 15);
    assert_eq!(per_zone["redridge"], 13);
    assert_eq!(per_zone["duskwood"], 12);
    assert_eq!(per_zone["any"], 4);
    assert_eq!(per_zone.len(), 4);
}

#[test]
fn each_zone_has_one_legendary_and_it_is_timed() {
    let herbs = herbs();
    for zone in ["elwynn", "redridge", "duskwood"] {
        let legendaries: Vec<_> = herbs
            .plant
            .iter()
            .filter(|p| p.zone == zone && p.rarity == "legendary")
            .collect();
        assert_eq!(legendaries.len(), 1, "{zone}");
        assert!(legendaries[0].timed, "{zone}'s legendary");
    }
}

#[test]
fn no_plant_needs_more_than_level_25() {
    for p in herbs().plant {
        assert!(p.level <= 25, "plant {} needs level {}", p.item, p.level);
    }
}

#[test]
fn everything_it_hands_out_has_a_herbalism_item_row() {
    let herbs = herbs();
    let rows: HashMap<u32, &Item> = herbs.item.iter().map(|i| (i.entry, i)).collect();
    let items = herbs
        .plant
        .iter()
        .map(|p| p.item)
        .chain(herbs.tool.iter().map(|t| t.item))
        .chain(herbs.consumable.iter().map(|c| c.item));
    for item in items {
        let row = rows
            .get(&item)
            .unwrap_or_else(|| panic!("item {item} has no row"));
        assert_eq!(row.journal, "herbalism", "item {item}");
    }
}

#[test]
fn a_plants_row_says_what_it_is_for() {
    let herbs = herbs();
    let rows: HashMap<u32, &Item> = herbs.item.iter().map(|i| (i.entry, i)).collect();
    for p in &herbs.plant {
        let want = match p.use_.as_deref() {
            Some("cooking") => "Cooking ingredient.",
            Some("alchemy") => "Alchemy reagent.",
            None => "A purse someone dropped in the brush.",
            Some(other) => panic!("plant {} has use {other}", p.item),
        };
        assert_eq!(rows[&p.item].description, want, "item {}", p.item);
    }
}

#[test]
fn there_are_three_boons_and_six_pacts() {
    let perks = herbs().perk;
    assert_eq!(perks.len(), 9);
    assert_eq!(perks.iter().filter(|p| p.kind == "boon").count(), 3);
    assert_eq!(perks.iter().filter(|p| p.kind == "pact").count(), 6);
}
