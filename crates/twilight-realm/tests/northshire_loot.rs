//! The Northshire world roll (spec #272, ticket #276): `content/northshire_loot.toml` against the
//! gear sheet, and a thousand seeded kills of a level 6 Northshire mob dropping Token whites and
//! World greens at the spec's rates, never a vanilla pool item.

mod common;
mod gear_common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::fight::unit_of_level;
use common::*;
use gear_common::last_health;
use twilight_realm::rules::loot::{northshire_green_rows, northshire_whites};
use twilight_realm::{Realm, RealmCommand};

fn gear_rows() -> Vec<toml::Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("content/gear.toml");
    let gear: toml::Value = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    gear["item"].as_array().unwrap().clone()
}

fn gear_row(rows: &[toml::Value], entry: u32) -> &toml::Value {
    rows.iter()
        .find(|r| r["entry"].as_integer() == Some(i64::from(entry)))
        .unwrap_or_else(|| panic!("{entry} is a row of the gear sheet"))
}

#[test]
fn the_whites_are_the_22_token_whites_and_the_greens_the_21_world_greens_at_their_levels() {
    let rows = gear_rows();
    let whites = northshire_whites();
    assert_eq!(whites.len(), 22);
    for &entry in whites {
        assert_eq!(
            gear_row(&rows, entry)["quality"].as_integer(),
            Some(1),
            "{entry} is Common"
        );
    }
    let greens = northshire_green_rows();
    assert_eq!(greens.len(), 21);
    for (entry, level) in greens {
        let row = gear_row(&rows, entry);
        assert_eq!(row["quality"].as_integer(), Some(2), "{entry} is Uncommon");
        assert_eq!(
            row["item_level"].as_integer(),
            Some(i64::from(level)),
            "{entry}'s level is its item level"
        );
    }
}

/// Kobold Laborer, level 6 to 7 in Echo Ridge Mine at Northshire's far end; the level of each
/// spawn is a hash of its guid, the same in every world.
const KOBOLD_LABORER: u32 = 80;
/// The notes of the three Northshire Rumours' points (`content/rumours.toml`).
const RUMOUR_NOTES: std::ops::RangeInclusive<u32> = 980030..=980038;
/// The mine's mouth, among the laborers.
const MINE_AT: [f32; 2] = [-8630.0, -143.0];
/// Tunneler's Maul, so the fight is short.
const MAUL: u32 = 900304;
const SLOT_MAIN_HAND: u8 = 15;

/// A level 10 Northshire Champion's save (`common::as_champion`), holding the maul: the loot is
/// what this file checks, not the fight.
fn warrior_blob() -> (u64, String) {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Miner", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    assert_eq!(create_result(&mut maker), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker.export_character(guid).unwrap();
    if let Some(at) = blob.find("[[items]]") {
        blob.truncate(at);
    }
    let mut blob = blob
        .replace("level = 1\n", "level = 10\n")
        .replace("items = []\n", "");
    blob.push_str(&format!(
        "\n[[items]]\nslot = {SLOT_MAIN_HAND}\nentry = {MAUL}\ncount = 1\n"
    ));
    (guid, as_champion(&blob))
}

/// One kill of a level 6 laborer in a fresh world at `seed`: what its corpse holds, or `None`
/// when the warrior fell or the fight ran long.
fn kill_a_laborer(seed: u64, guid: u64, blob: &str) -> Option<Vec<u32>> {
    let mut realm = Realm::with_seed(seed).with_callings(CHAMPION_CALLINGS);
    realm.import_character(blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm);
    realm.handle(RealmCommand::DevTeleport {
        x: MINE_AT[0],
        y: MINE_AT[1],
        facing: None,
    });
    let mut events = Vec::new();
    for _ in 0..10 {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(&mut realm));
    }
    let (mob, at) = unit_of_level(&events, KOBOLD_LABORER, 6, MINE_AT)
        .expect("a level 6 laborer at the mine's mouth");
    realm.handle(RealmCommand::DevTeleport {
        x: at[0] + 1.5,
        y: at[1],
        facing: Some(std::f32::consts::PI),
    });
    realm.tick(0.1, &FlatGround);
    events.extend(drain(&mut realm));
    realm.handle(RealmCommand::AttackSwing { guid: mob });
    let mut dead = false;
    for _ in 0..900 {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(&mut realm));
        if last_health(&events, guid) == Some(0) {
            return None;
        }
        if last_health(&events, mob) == Some(0) {
            dead = true;
            break;
        }
    }
    if !dead {
        return None;
    }
    realm.handle(RealmCommand::Loot { guid: mob });
    Some(
        drain(&mut realm)
            .into_iter()
            .find_map(|e| match e {
                SessionEvent::LootResponse { items, .. } => {
                    Some(items.iter().map(|i| i.item_id).collect())
                }
                _ => None,
            })
            .unwrap_or_default(),
    )
}

#[test]
fn a_thousand_kills_of_a_level_6_northshire_mob_drop_8_percent_whites_and_4_percent_greens() {
    let (guid, blob) = warrior_blob();
    let whites = northshire_whites();
    let greens = northshire_green_rows();
    let (mut kills, mut white, mut green) = (0, 0, 0);
    for seed in 0..1500u64 {
        if kills == 1000 {
            break;
        }
        let Some(items) = kill_a_laborer(0x6E00_0000 + seed, guid, &blob) else {
            continue;
        };
        // A kill inside Northshire may also roll a Rumour (0.5%), whose note is no loot row.
        let items: Vec<u32> = items
            .into_iter()
            .filter(|item| !RUMOUR_NOTES.contains(item))
            .collect();
        kills += 1;
        assert!(items.len() <= 1, "one roll a kill: {items:?}");
        for item in items {
            if whites.contains(&item) {
                white += 1;
            } else if let Some((_, level)) = greens.iter().find(|(e, _)| *e == item) {
                green += 1;
                assert!((5..=7).contains(level), "green {item} is level {level}");
            } else {
                panic!("{item} is neither a Token white nor a World green");
            }
        }
    }
    assert_eq!(kills, 1000, "a thousand level 6 kills");
    assert!((60..=100).contains(&white), "{white} whites");
    assert!((25..=55).contains(&green), "{green} greens");
}
