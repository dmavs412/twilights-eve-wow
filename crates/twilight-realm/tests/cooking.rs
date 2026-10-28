//! Kitchens in the realm (#491, ADR 0025): Tomas and Brother Hollis open them, recipes are learned,
//! revealed, predicted and cooked.

mod common;

use benilla_protocol::{KitchenRecipe, SessionEvent, START_ELWYNN};
use common::*;
use twilight_realm::{KitchenAction, KitchenMode, Realm, RealmCommand};

const TOMAS: u32 = 1430;
const KITCHEN_LINE: &str = "Let me use your kitchen.";

const GOLDSHIRE_PERCH: u32 = 960_401;
const EVERSTILL_PERCH: u32 = 960_411;
const DANDELION: u32 = 961_001;
const WILD_ONION: u32 = 961_002;
const HAZELNUTS: u32 = 961_005;
const SWEETROOT: u32 = 961_008;
const WILD_GARLIC: u32 = 961_010;
const DUSKROOT: u32 = 961_020;
const GILDED_KOI: u32 = 960_409;

const CHOWDER: u32 = 962_025;
const FISH_STEW: u32 = 962_001;
const HOTPOT: u32 = 962_002;
const MUSH: u32 = 962_400;

/// What a new character holds in the Satchel for these tests.
const PANTRY: [(u32, u32); 8] = [
    (GOLDSHIRE_PERCH, 6),
    (EVERSTILL_PERCH, 2),
    (DANDELION, 6),
    (WILD_ONION, 6),
    (HAZELNUTS, 2),
    (SWEETROOT, 6),
    (WILD_GARLIC, 2),
    (DUSKROOT, 2),
];

/// A new Elwynn character's save, its guid, and where it starts.
fn new_save() -> (u64, String) {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Tam", HUMAN, 0, WARRIOR) else {
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

/// A character standing beside Tomas, holding [`PANTRY`] in the Satchel.
fn at_tomas() -> (Realm, u64) {
    let (guid, blob) = new_save();
    let [x, y, _] = hub_home(hub_guid(TOMAS));
    let mut blob = placed(&blob, [x + 1.5, y, 0.0]);
    for (entry, count) in PANTRY {
        blob.push_str(&format!(
            "\n[[satchel]]\nentry = {entry}\ncount = {count}\n"
        ));
    }
    let mut realm = Realm::with_seed(7);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm);
    (realm, guid)
}

/// Gossip at Tomas: the menu's lines.
fn hello(realm: &mut Realm) -> Vec<(u32, String)> {
    realm.handle(RealmCommand::GossipHello {
        guid: hub_guid(TOMAS),
    });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => {
                Some(options.into_iter().map(|o| (o.index, o.message)).collect())
            }
            _ => None,
        })
        .expect("Tomas has a menu")
}

/// Opens Tomas's kitchen: its events.
fn open(realm: &mut Realm) -> Vec<SessionEvent> {
    let (option, _) = hello(realm)
        .into_iter()
        .find(|(_, m)| m == KITCHEN_LINE)
        .expect("the kitchen line");
    realm.handle(RealmCommand::GossipSelectOption {
        guid: hub_guid(TOMAS),
        option,
    });
    drain(realm)
}

/// The last `Kitchen`'s book in `events`.
fn book(events: &[SessionEvent]) -> Vec<KitchenRecipe> {
    events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::Kitchen { book, .. } => Some(book.clone()),
            _ => None,
        })
        .expect("a Kitchen")
}

fn act(realm: &mut Realm, action: KitchenAction) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::Kitchen(action));
    drain(realm)
}

fn cook(realm: &mut Realm, mode: KitchenMode, recipe: &str, slots: &[u32]) -> Vec<SessionEvent> {
    act(
        realm,
        KitchenAction::Cook {
            mode,
            recipe: recipe.to_owned(),
            slots: slots.to_vec(),
        },
    )
}

/// The `KitchenCooked` in `events`: meal, mush, close, learned, revealed.
#[allow(clippy::type_complexity)]
fn cooked(events: &[SessionEvent]) -> (u32, bool, bool, Vec<String>, Vec<(u32, u32)>) {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::KitchenCooked {
                meal,
                mush,
                close,
                learned,
                revealed,
                ..
            } => Some((*meal, *mush, *close, learned.clone(), revealed.clone())),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a KitchenCooked in {events:?}"))
}

fn predict(realm: &mut Realm, mode: KitchenMode, recipe: &str, slots: &[u32]) -> (bool, u32) {
    let events = act(
        realm,
        KitchenAction::Predict {
            mode,
            recipe: recipe.to_owned(),
            slots: slots.to_vec(),
        },
    );
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::KitchenPrediction { ready, meal, .. } => Some((*ready, *meal)),
            _ => None,
        })
        .expect("a KitchenPrediction")
}

/// How many of `item` the saved character holds in the Satchel, and in the bags.
fn held(realm: &Realm, guid: u64, item: u32) -> (i64, i64) {
    let blob = realm.export_character(guid).unwrap();
    let table: toml::Table = blob.parse().unwrap();
    let sum = |key: &str| {
        table
            .get(key)
            .and_then(|s| s.as_array())
            .into_iter()
            .flatten()
            .filter(|r| r["entry"].as_integer() == Some(i64::from(item)))
            .map(|r| r["count"].as_integer().unwrap())
            .sum()
    };
    (sum("satchel"), sum("items"))
}

const FIRST_POT: [u32; 3] = [GOLDSHIRE_PERCH, DANDELION, WILD_ONION];

#[test]
fn tomas_opens_a_kitchen_of_44_unknown_tier_recipes() {
    let (mut realm, _) = at_tomas();
    assert!(hello(&mut realm).iter().any(|(_, m)| m == KITCHEN_LINE));
    let events = open(&mut realm);
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::GossipComplete)));
    let book = book(&events);
    assert_eq!(book.len(), 44);
    assert!(book.iter().all(|r| r.kind == 0 && !r.known));
    assert!(book.iter().all(|r| r.slots.iter().all(|&s| s == 0)));
}

#[test]
fn the_first_experiment_teaches_the_starters_and_cooks_the_chowder() {
    let (mut realm, guid) = at_tomas();
    open(&mut realm);
    let events = cook(&mut realm, KitchenMode::Experiment, "", &FIRST_POT);
    let (meal, mush, _, learned, revealed) = cooked(&events);
    assert_eq!(meal, CHOWDER);
    assert!(!mush);
    assert_eq!(learned, ["stamina_1", "xp_1", "regen_1", "pull_1"]);
    assert_eq!(held(&realm, guid, GOLDSHIRE_PERCH).0, 5);
    assert_eq!(held(&realm, guid, DANDELION).0, 5);
    assert_eq!(held(&realm, guid, WILD_ONION).0, 5);
    assert_eq!(
        held(&realm, guid, CHOWDER),
        (0, 1),
        "the meal is in the bags"
    );
    assert!(
        revealed
            .iter()
            .any(|&(item, count)| item == GOLDSHIRE_PERCH && count > 0),
        "{revealed:?}"
    );

    // Only the tried items show in unknown recipes.
    let book = book(&events);
    let hotpot = book.iter().find(|r| r.key == "stamina_10").unwrap();
    assert!(!hotpot.known);
    assert_eq!(
        hotpot.slots,
        [0, 0, 0],
        "Everstill Perch and Wild Garlic stay hidden"
    );
    // Every unknown tier recipe holding the perch shows it: as many as the reveal counted.
    let perch_pages = book
        .iter()
        .filter(|r| !r.known && r.slots.contains(&GOLDSHIRE_PERCH))
        .count() as u32;
    let perch_count = revealed
        .iter()
        .find(|&&(item, _)| item == GOLDSHIRE_PERCH)
        .map(|&(_, n)| n);
    assert_eq!(Some(perch_pages), perch_count);
    let saved = realm.export_character(guid).unwrap();
    assert!(saved.contains("cook_visited = [\"goldshire\"]"), "{saved}");
}

#[test]
fn a_recipe_prediction_needs_its_own_items() {
    let (mut realm, _) = at_tomas();
    open(&mut realm);
    cook(&mut realm, KitchenMode::Experiment, "", &FIRST_POT);
    let stew = [GOLDSHIRE_PERCH, WILD_ONION, SWEETROOT];
    assert_eq!(
        predict(&mut realm, KitchenMode::Recipe, "stamina_1", &stew),
        (true, FISH_STEW)
    );
    let swapped = [GOLDSHIRE_PERCH, WILD_ONION, DANDELION];
    assert!(!predict(&mut realm, KitchenMode::Recipe, "stamina_1", &swapped).0);
}

#[test]
fn a_pot_that_matches_nothing_is_hearty_mush_and_one_item_off_is_close() {
    let (mut realm, _) = at_tomas();
    open(&mut realm);
    let (meal, mush, close, ..) = cooked(&cook(
        &mut realm,
        KitchenMode::Experiment,
        "",
        &[SWEETROOT, SWEETROOT, SWEETROOT],
    ));
    assert_eq!((meal, mush, close), (MUSH, true, false));
    // Hunter's Fish Stew with Hazelnuts for its Sweetroot.
    let (meal, mush, close, ..) = cooked(&cook(
        &mut realm,
        KitchenMode::Experiment,
        "",
        &[GOLDSHIRE_PERCH, WILD_ONION, HAZELNUTS],
    ));
    assert_eq!((meal, mush, close), (MUSH, true, true));
}

#[test]
fn a_discover_cook_of_the_exact_items_teaches_the_recipe() {
    let (mut realm, _) = at_tomas();
    open(&mut realm);
    let events = cook(
        &mut realm,
        KitchenMode::Discover,
        "stamina_10",
        &[EVERSTILL_PERCH, WILD_GARLIC, DUSKROOT],
    );
    let (meal, _, _, learned, _) = cooked(&events);
    assert_eq!(meal, HOTPOT);
    assert!(learned.contains(&"stamina_10".to_owned()), "{learned:?}");
    let hotpot = book(&events)
        .into_iter()
        .find(|r| r.key == "stamina_10")
        .unwrap();
    assert!(hotpot.known);
}

#[test]
fn the_cooking_fields_survive_a_save_round_trip() {
    let (mut realm, guid) = at_tomas();
    open(&mut realm);
    cook(&mut realm, KitchenMode::Experiment, "", &FIRST_POT);
    let blob = realm.export_character(guid).unwrap();
    let mut back = Realm::with_seed(9);
    back.import_character(&blob).unwrap();
    let again = back.export_character(guid).unwrap();
    for key in [
        "cook_known",
        "cook_tried",
        "cook_visited",
        "cook_lessons",
        "[[dish_log]]",
    ] {
        let line = |b: &str| {
            b.lines()
                .filter(|l| l.starts_with(key))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(blob.contains(key), "{key} in {blob}");
        assert_eq!(line(&blob), line(&again), "{key}");
    }
    assert!(again.contains("recipe = \"regen_1\""), "{again}");
    assert!(again.contains("cooked = 1"), "{again}");
    assert!(again.contains("Lion's Pride kitchen"), "{again}");
}

#[test]
fn walking_ten_yards_from_tomas_closes_the_kitchen() {
    let (mut realm, _) = at_tomas();
    open(&mut realm);
    let [x, y, _] = hub_home(hub_guid(TOMAS));
    realm.handle(RealmCommand::Move {
        position: [x + 11.5, y, 0.0],
        orientation: 0.0,
        motion: Default::default(),
    });
    realm.tick(0.1, &FlatGround);
    let events = drain(&mut realm);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SessionEvent::KitchenClosed)),
        "{events:?}"
    );
}

/// A river off the character's east side, 15 yards out, where only Gilded Koi bite. The koi keeps
/// its shipped level 14: `cooking.toml`'s load check reads every pool, a test's too.
fn koi_river(start: [f32; 3]) -> String {
    let (x, y) = (start[0] + 15.0, start[1]);
    format!(
        "
[[pool]]
key = \"test_koi\"
zone = 990091
kind = \"river\"
description = \"A test river.\"
fish = [{{ item = {GILDED_KOI}, weight = 100.0, level = 14 }}]
treasure = []
copper = [0, 0]

[[water]]
key = \"test_koi\"
name = \"Koi Stream\"
circle = [0, {x}, {y}, 9.0]
pool = \"test_koi\"
"
    )
}

#[test]
fn the_first_gilded_koi_teaches_its_dish() {
    const FISHING: u32 = 7620;
    const FISHING_POLE: u32 = 6256;
    const KOI_LURE: u32 = 6532;
    let (guid, blob) = new_save();
    let table: toml::Table = blob.parse().unwrap();
    let p = table["position"].as_array().unwrap();
    let start: [f32; 3] = std::array::from_fn(|i| p[i].as_float().unwrap() as f32);
    let mut blob = blob.replace(
        "angler_level = 1\n",
        "angler_level = 60\nknows_fishing = true\n",
    );
    blob.push_str(&format!(
        "\n[fishing_pole]\nslot = 0\nentry = {FISHING_POLE}\ncount = 1\n\
         \n[fishing_lure]\nslot = 0\nentry = {KOI_LURE}\ncount = 1\ndurability = 50\n"
    ));
    let host = Pond {
        centre: [start[0] + 15.0, start[1]],
        radius: 8.0,
        area: Some(12),
    };
    let mut realm = Realm::with_seed(7).with_fish(&koi_river(start));
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &host);
    realm.set_wall_clock(12 * 60 * 5 * 1000);
    realm.handle(RealmCommand::Move {
        position: [start[0], start[1], 0.0],
        orientation: 0.0,
        motion: Default::default(),
    });
    realm.tick(0.0, &host);
    drain(&mut realm);

    realm.handle(RealmCommand::CastSpell {
        spell_id: FISHING,
        target: None,
    });
    let mut bit = None;
    for _ in 0..130 {
        realm.tick(0.1, &host);
        bit = drain(&mut realm).into_iter().find_map(|e| match e {
            SessionEvent::FishingBite { fish, .. } => Some(fish),
            _ => None,
        });
        if bit.is_some() {
            break;
        }
    }
    assert_eq!(bit, Some(GILDED_KOI));
    realm.handle(RealmCommand::FishingResult {
        won: true,
        perfect: false,
        treasure: false,
        secs: 6.0,
    });
    realm.tick(0.0, &host);
    let events = drain(&mut realm);
    let lines = format!("{events:?}");
    assert!(
        lines.contains("Your first catch teaches you: Gilded Koi Cakes."),
        "{lines}"
    );
    let saved = realm.export_character(guid).unwrap();
    assert!(saved.contains("dish_gilded_koi"), "{saved}");
}
