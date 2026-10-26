//! The Loot Band roll (spec #448, ticket #452): `content/loot_bands.toml` against the gear sheet,
//! and a thousand seeded kills each of a level 6 Northshire mob and a level 16 Elwynn mob dropping
//! their band's whites and greens at the spec's rates, never a vanilla pool item.

mod common;
mod gear_common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::fight::unit_of_level;
use common::*;
use gear_common::last_health;
use twilight_realm::rules::loot::{band_greens, band_of, bands};
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

/// Each band after Northshire: its levels and the item level its rows sit at.
const LATER_BANDS: [([u8; 2], i64); 5] = [
    ([11, 14], 12),
    ([15, 18], 15),
    ([19, 22], 19),
    ([23, 26], 23),
    ([27, 30], 28),
];

#[test]
fn each_band_lists_its_whites_and_greens_from_the_gear_sheet() {
    let rows = gear_rows();
    let bands = bands();
    assert_eq!(bands.len(), 1 + LATER_BANDS.len());

    // Band 1, Northshire: the 22 Token whites and the 21 World greens at their levels.
    assert_eq!(bands[0].levels, [1, 10]);
    let whites = bands[0].whites();
    assert_eq!(whites.len(), 22);
    for &entry in whites {
        assert_eq!(
            gear_row(&rows, entry)["quality"].as_integer(),
            Some(1),
            "{entry} is Common"
        );
    }
    let greens = bands[0].green_rows();
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

    // Every later band: 21 Common and 20 Uncommon rows at the band's set level, wearable from
    // its first level.
    for (band, (levels, item_level)) in bands[1..].iter().zip(LATER_BANDS) {
        assert_eq!(band.levels, levels);
        let greens = band.green_rows();
        assert_eq!(band.whites().len(), 21, "band {levels:?}");
        assert_eq!(greens.len(), 20, "band {levels:?}");
        let listed = band
            .whites()
            .iter()
            .map(|&e| (e, 1, item_level))
            .chain(greens.iter().map(|&(e, l)| (e, 2, i64::from(l))));
        for (entry, quality, level) in listed {
            let row = gear_row(&rows, entry);
            assert_eq!(
                row["quality"].as_integer(),
                Some(quality),
                "{entry}'s quality"
            );
            assert_eq!(row["item_level"].as_integer(), Some(item_level), "{entry}");
            assert_eq!(level, item_level, "{entry}'s listed level");
            assert_eq!(
                row["required_level"].as_integer(),
                Some(i64::from(levels[0])),
                "{entry}'s required level"
            );
        }
    }
}

#[test]
fn a_level_35_mob_rolls_the_last_band() {
    let last = bands().last().unwrap();
    assert_eq!(last.levels, [27, 30]);
    assert!(std::ptr::eq(band_of(35), last));
    assert_eq!(
        band_greens(35),
        last.green_rows()
            .iter()
            .map(|&(e, _)| e)
            .collect::<Vec<_>>()
    );
}

/// Kobold Laborer, level 6 to 7 in Echo Ridge Mine at Northshire's far end; the level of each
/// spawn is a hash of its guid, the same in every world.
const KOBOLD_LABORER: u32 = 80;
/// Kobold Miner, one of them level 16 alone at Jasperlode's mouth (Elwynn Level Band 15 to 18).
const KOBOLD_MINER: u32 = 40;
/// The notes of the three Northshire Rumours' points (`content/rumours.toml`).
const RUMOUR_NOTES: std::ops::RangeInclusive<u32> = 980030..=980038;
/// The mine's mouth, among the laborers.
const MINE_AT: [f32; 2] = [-8630.0, -143.0];
/// Jasperlode's mouth, by the level 16 miner.
const JASPERLODE_AT: [f32; 2] = [-9187.0, -649.0];
/// Tunneler's Maul, so the fight is short.
const MAUL: u32 = 900304;
const SLOT_MAIN_HAND: u8 = 15;

/// A Champion's save at `level` (`common::as_champion`), holding the maul: the loot is what this
/// file checks, not the fight.
fn warrior_blob(level: u8) -> (u64, String) {
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
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("items = []\n", "");
    blob.push_str(&format!(
        "\n[[items]]\nslot = {SLOT_MAIN_HAND}\nentry = {MAUL}\ncount = 1\n"
    ));
    (guid, as_champion(&blob))
}

/// One kill of the `entry` of `level` nearest `place` in a fresh world at `seed`: what its corpse
/// holds, or `None` when the warrior fell or the fight ran long.
fn kill(
    seed: u64,
    guid: u64,
    blob: &str,
    entry: u32,
    level: u32,
    place: [f32; 2],
) -> Option<Vec<u32>> {
    let mut realm = Realm::with_seed(seed).with_callings(CHAMPION_CALLINGS);
    realm.import_character(blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm);
    realm.handle(RealmCommand::DevTeleport {
        x: place[0],
        y: place[1],
        facing: None,
    });
    let mut events = Vec::new();
    for _ in 0..10 {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(&mut realm));
    }
    let (mob, at) = unit_of_level(&events, entry, level, place)
        .unwrap_or_else(|| panic!("a level {level} {entry} by {place:?}"));
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

/// A thousand kills of the `entry` of `level` nearest `place`: how many dropped a white of
/// `band` and how many a green, panicking on any other drop. `green_levels` holds every level a
/// dropped green may have.
fn a_thousand_kills(
    seed: u64,
    warrior_level: u8,
    (entry, level, place): (u32, u32, [f32; 2]),
    band: u8,
    green_levels: std::ops::RangeInclusive<u8>,
) -> (u32, u32) {
    let (guid, blob) = warrior_blob(warrior_level);
    let band = band_of(band);
    let whites = band.whites();
    let greens = band.green_rows();
    let (mut kills, mut white, mut green) = (0, 0, 0);
    for seed in seed..seed + 1500 {
        if kills == 1000 {
            break;
        }
        let Some(items) = kill(seed, guid, &blob, entry, level, place) else {
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
                assert!(
                    green_levels.contains(level),
                    "green {item} is level {level}"
                );
            } else {
                panic!("{item} is neither a white nor a green of the band");
            }
        }
    }
    assert_eq!(kills, 1000, "a thousand level {level} kills");
    (white, green)
}

#[test]
fn a_thousand_kills_of_a_level_6_northshire_mob_drop_8_percent_whites_and_4_percent_greens() {
    let (white, green) = a_thousand_kills(0x6E00_0000, 10, (KOBOLD_LABORER, 6, MINE_AT), 6, 5..=7);
    assert!((60..=100).contains(&white), "{white} whites");
    assert!((25..=55).contains(&green), "{green} greens");
}

#[test]
fn a_thousand_kills_of_a_level_16_mob_drop_8_percent_band_2_whites_and_4_percent_band_2_greens() {
    let (white, green) = a_thousand_kills(
        0x6F00_0000,
        18,
        (KOBOLD_MINER, 16, JASPERLODE_AT),
        16,
        15..=15,
    );
    assert!((60..=100).contains(&white), "{white} whites");
    assert!((25..=55).contains(&green), "{green} greens");
}
