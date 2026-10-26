//! Fishing (#327, ADR 0018): Lee Brown teaches it; a cast with a pole into water floats a bobber,
//! a fish bites and the client's Reeling Bar reports the catch, which pays Angler XP.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::{FishPage, SatchelPick, SatchelRow, SessionEvent, START_ELWYNN};
use common::*;
use quest_common::{accept, completed, hand_in};
use twilight_realm::{FishingGearSlot, Realm, RealmCommand};

const FISHING: u32 = 7620;
const FISHING_POLE: u32 = 6256;
const SMALLFISH: u32 = 6291;
const CATFISH: u32 = 6308;
const LEE_BROWN: u32 = 1651;
const BROTHER_WENDELL: u32 = 190067;
const ELWYNN: u32 = 12;
const ELWYNN_FISH: [u32; 8] = [6291, 960401, 960402, 6289, 960403, 960404, 960405, 960406];
const MAIN_HAND: u8 = 15;

const SPELL_FAILED_EQUIPPED_ITEM_CLASS: u8 = 0x19;
const SPELL_FAILED_NOT_FISHABLE: u8 = 0x34;
const SPELL_FAILED_NOT_KNOWN: u8 = 0x38;
/// `GAMEOBJECT_TYPE_ID` and `GAMEOBJECT_TYPE_FISHINGNODE`.
const GAMEOBJECT_TYPE_ID: u16 = 21;
const FISHINGNODE: u32 = 17;

/// Test pools, each fished by a circle of water off its own side of the Northshire start: a lone
/// Raw Brilliant Smallfish to the north, a lone level-15 catfish to the west and a lone level-11
/// mud snapper to the south.
const TEST_FISH: &str = r#"
[[pool]]
key = "test_smallfish"
zone = 990001
kind = "lake"
description = "A test pond."
fish = [{ item = 6291, weight = 100.0, level = 1 }]
treasure = []
copper = [0, 0]

[[pool]]
key = "test_level_15"
zone = 990002
kind = "lake"
description = "A test pond."
fish = [{ item = 6308, weight = 100.0, level = 15 }]
treasure = []
copper = [0, 0]

[[pool]]
key = "test_level_11"
zone = 990003
kind = "lake"
description = "A test pond."
fish = [{ item = 6289, weight = 100.0, level = 11 }]
treasure = []
copper = [0, 0]
"#;

/// A Legend Rumour that every catch in the test Rumour Pond hits (note 960206 is a shipped note).
const TEST_LEGEND_RUMOUR: &str = r#"
[[rumour]]
id = 99031201
name = "Test Rumour Fish"
kind = "legend"
legend = 6303
note = 960206
catch_chance = 100.0
"#;

/// Which way the character faces its water from the start, 15 yards off.
#[derive(Clone, Copy)]
enum Side {
    /// Elwynn's own pool, by area.
    East,
    /// The lone smallfish.
    North,
    /// The lone level-15 catfish.
    West,
    /// The lone level-11 mud snapper.
    South,
    /// The pool with the test Legendary Fish (#331).
    NorthWest,
    /// The pool whose catches roll a test Legend Rumour at 100 percent.
    NorthEast,
    /// The pool whose fish carry conditions (#398).
    SouthEast,
}

impl Side {
    fn facing(self) -> f32 {
        match self {
            Side::East => 0.0,
            Side::North => std::f32::consts::FRAC_PI_2,
            Side::West => std::f32::consts::PI,
            Side::South => -std::f32::consts::FRAC_PI_2,
            Side::NorthWest => 3.0 * std::f32::consts::FRAC_PI_4,
            Side::NorthEast => std::f32::consts::FRAC_PI_4,
            Side::SouthEast => -std::f32::consts::FRAC_PI_4,
        }
    }
}

/// The water off `side` of `start`, 15 yards out.
fn pond_centre(start: [f32; 3], side: Side) -> [f32; 2] {
    let f = side.facing();
    [start[0] + 15.0 * f.cos(), start[1] + 15.0 * f.sin()]
}

/// The test pools, with the waters that fish them around `start`.
fn test_fish(start: [f32; 3]) -> String {
    let [nx, ny] = pond_centre(start, Side::North);
    let [wx, wy] = pond_centre(start, Side::West);
    let [sx, sy] = pond_centre(start, Side::South);
    let [lx, ly] = pond_centre(start, Side::NorthWest);
    let [rx, ry] = pond_centre(start, Side::NorthEast);
    let [gx, gy] = pond_centre(start, Side::SouthEast);
    format!(
        "{TEST_FISH}
[[pool]]
key = \"test_legends\"
zone = 990004
kind = \"lake\"
description = \"A test pond.\"
fish = [{{ item = 6291, weight = 100.0, level = 1 }}]
treasure = []
copper = [0, 0]

[[pool]]
key = \"test_legend_rumour\"
zone = 990005
kind = \"lake\"
description = \"A test pond.\"
fish = [{{ item = 6291, weight = 100.0, level = 1 }}]
treasure = []
copper = [0, 0]

[[water]]
key = \"test_legends\"
name = \"Legend Pond\"
circle = [0, {lx}, {ly}, 9.0]
pool = \"test_legends\"

[[water]]
key = \"test_legend_rumour\"
name = \"Rumour Pond\"
circle = [0, {rx}, {ry}, 9.0]
pool = \"test_legend_rumour\"

# The test Legendary Fish borrow existing fish items as their trophies. Table order is the roll
# order: at every hit the Gar comes first.
[[legend]]
item = 6309
name = \"Test Gar\"
pool = \"test_legends\"
level = 1
style = \"dart\"
difficulty = 50
chance = 100
block = \"night\"

[[legend]]
item = 6310
name = \"Test Bream\"
pool = \"test_legends\"
level = 1
style = \"dart\"
difficulty = 50
chance = 100
block = \"dawn\"

[[legend]]
item = 6361
name = \"Test Snapper\"
pool = \"test_legends\"
level = 1
style = \"dart\"
difficulty = 50
chance = 100
bait = 960103

[[legend]]
item = 6358
name = \"Test Gate\"
pool = \"test_legends\"
level = 5
style = \"smooth\"
difficulty = 50
chance = 100

# Never bites (level 99); a catch in its pool rolls its Rumour.
[[legend]]
item = 6303
name = \"Test Rumour Fish\"
pool = \"test_legend_rumour\"
level = 99
style = \"smooth\"
difficulty = 50
chance = 100

# Conditioned fish (#398) borrow four legend trophies as their items. The ungated one keeps the
# pool valid when no condition holds.
[[pool]]
key = \"test_gated\"
zone = 990006
kind = \"lake\"
description = \"A test pond.\"
fish = [
    {{ item = 960003, weight = 100.0, level = 1 }},
    {{ item = 960004, weight = 100.0, level = 1 }},
    {{ item = 960005, weight = 100.0, level = 1 }},
    {{ item = 960006, weight = 0.01, level = 1 }},
]
treasure = []
copper = [0, 0]

[[fish]]
item = 960003
style = \"smooth\"
difficulty = 10
rarity = \"common\"
block = \"dusk\"

[[fish]]
item = 960004
style = \"smooth\"
difficulty = 10
rarity = \"common\"
lure = 6532

[[fish]]
item = 960005
style = \"smooth\"
difficulty = 10
rarity = \"common\"
bait = 960101

[[fish]]
item = 960006
style = \"smooth\"
difficulty = 10
rarity = \"common\"

[[water]]
key = \"test_gated\"
name = \"Gated Pond\"
circle = [0, {gx}, {gy}, 9.0]
pool = \"test_gated\"

[[water]]
key = \"test_smallfish\"
name = \"Smallfish Pond\"
circle = [0, {nx}, {ny}, 9.0]
pool = \"test_smallfish\"

[[water]]
key = \"test_level_15\"
name = \"Catfish Pond\"
circle = [0, {wx}, {wy}, 9.0]
pool = \"test_level_15\"

[[water]]
key = \"test_level_11\"
name = \"Snapper Pond\"
circle = [0, {sx}, {sy}, 9.0]
pool = \"test_level_11\"
"
    )
}

/// A new Elwynn character's save, edited: Angler Level `level`, Fishing known or not, and a
/// Fishing Pole in the Pole slot or nothing carried. Its guid, the save and where it starts.
fn save(level: u8, knows: bool, pole: bool) -> (u64, String, [f32; 3]) {
    save_carrying(level, knows, pole, &[])
}

/// [`save`], also carrying `items`: `(player slot, entry, count)`, the backpack from 23.
fn save_carrying(
    level: u8,
    knows: bool,
    pole: bool,
    items: &[(u8, u32, u32)],
) -> (u64, String, [f32; 3]) {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Lee", HUMAN, 0, WARRIOR) else {
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
    let knows = if knows { "knows_fishing = true\n" } else { "" };
    let mut blob = blob.replace(
        "angler_level = 1\n",
        &format!("angler_level = {level}\n{knows}"),
    );
    if pole {
        blob.push_str(&format!(
            "\n[fishing_pole]\nslot = 0\nentry = {FISHING_POLE}\ncount = 1\n"
        ));
    }
    for (slot, entry, count) in items {
        blob.push_str(&format!(
            "\n[[items]]\nslot = {slot}\nentry = {entry}\ncount = {count}\n"
        ));
    }
    let table: toml::Table = blob.parse().unwrap();
    let p = table["position"].as_array().unwrap();
    let start = std::array::from_fn(|i| p[i].as_float().unwrap() as f32);
    (guid, blob, start)
}

/// A character in the world at the start, facing the water off `side`; the host's pond is there.
struct Angler {
    realm: Realm,
    guid: u64,
    host: Pond,
    events: Vec<SessionEvent>,
}

fn angler(level: u8, knows: bool, pole: bool, side: Side) -> Angler {
    carrying(level, side, knows, pole, &[])
}

/// [`angler`], with a pole and Fishing known, carrying `items` in the backpack.
fn angler_carrying(level: u8, side: Side, items: &[(u8, u32, u32)]) -> Angler {
    carrying(level, side, true, true, items)
}

fn carrying(level: u8, side: Side, knows: bool, pole: bool, items: &[(u8, u32, u32)]) -> Angler {
    let (guid, blob, start) = save_carrying(level, knows, pole, items);
    from_save(guid, &blob, start, side)
}

/// The character of `blob` in the world at `start`, facing the water off `side`.
fn from_save(guid: u64, blob: &str, start: [f32; 3], side: Side) -> Angler {
    let host = Pond {
        centre: pond_centre(start, side),
        radius: 8.0,
        area: Some(ELWYNN),
    };
    let mut realm = Realm::with_seed(7)
        .with_fish(&test_fish(start))
        .with_rumours(TEST_LEGEND_RUMOUR);
    realm.import_character(blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &host);
    realm.handle(RealmCommand::Move {
        position: [start[0], start[1], 0.0],
        orientation: side.facing(),
        motion: Default::default(),
    });
    realm.tick(0.0, &host);
    let events = drain(&mut realm);
    Angler {
        realm,
        guid,
        host,
        events,
    }
}

impl Angler {
    fn cast(&mut self) -> Vec<SessionEvent> {
        self.realm.handle(RealmCommand::CastSpell {
            spell_id: FISHING,
            target: None,
        });
        self.realm.tick(0.05, &self.host);
        drain(&mut self.realm)
    }

    /// Ticks until a fish bites, at most `secs`; every event on the way.
    fn wait_for_bite(&mut self, secs: f32) -> Vec<SessionEvent> {
        let mut all = Vec::new();
        let mut t = 0.0;
        while t < secs
            && !all
                .iter()
                .any(|e| matches!(e, SessionEvent::FishingBite { .. }))
        {
            self.realm.tick(0.1, &self.host);
            all.extend(drain(&mut self.realm));
            t += 0.1;
        }
        all
    }

    fn result(&mut self, won: bool, perfect: bool, secs: f32) -> Vec<SessionEvent> {
        self.realm.handle(RealmCommand::FishingResult {
            won,
            perfect,
            treasure: false,
            secs,
        });
        self.realm.tick(0.0, &self.host);
        drain(&mut self.realm)
    }

    /// Cast, wait for the bite and return it.
    fn hook(&mut self) -> (Vec<SessionEvent>, SessionEvent) {
        let mut events = self.cast();
        events.extend(self.wait_for_bite(12.5));
        let bite = events
            .iter()
            .find(|e| matches!(e, SessionEvent::FishingBite { .. }))
            .cloned()
            .expect("a fish bites within 12 s");
        (events, bite)
    }
}

fn refusal(events: &[SessionEvent]) -> Option<u8> {
    events.iter().find_map(|e| match e {
        SessionEvent::CastResult {
            spell_id: FISHING,
            success: false,
            reason,
            ..
        } => *reason,
        _ => None,
    })
}

fn fishing_end(events: &[SessionEvent]) -> Option<(Option<u32>, bool)> {
    events.iter().find_map(|e| match e {
        SessionEvent::FishingEnd {
            caught, perfect, ..
        } => Some((*caught, *perfect)),
        _ => None,
    })
}

fn angler_event(events: &[SessionEvent]) -> Option<(u8, u32)> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::Angler { level, xp, .. } => Some((*level, *xp)),
        _ => None,
    })
}

fn gossip_lines(events: &[SessionEvent]) -> Vec<String> {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => {
                Some(options.iter().map(|o| o.message.clone()).collect())
            }
            _ => None,
        })
        .expect("a gossip menu")
}

#[test]
fn fishing_unknown_is_refused_until_lee_brown_teaches_it() {
    let mut a = angler(1, false, true, Side::East);
    assert_eq!(angler_event(&a.events), Some((1, 0)), "login sends Angler");
    assert_eq!(refusal(&a.cast()), Some(SPELL_FAILED_NOT_KNOWN));

    let lee = hub_guid(LEE_BROWN);
    walk_up_to(&mut a.realm, lee);
    a.realm.handle(RealmCommand::GossipHello { guid: lee });
    let lines = gossip_lines(&drain(&mut a.realm));
    let teach = lines.iter().position(|l| l == "Teach me to fish.");
    assert!(teach.is_some(), "Lee Brown offers the lesson: {lines:?}");
    a.realm.handle(RealmCommand::GossipSelectOption {
        guid: lee,
        option: 2,
    });
    let taught = drain(&mut a.realm);
    assert!(taught
        .iter()
        .any(|e| matches!(e, SessionEvent::SpellLearned { spell_id: FISHING })));
    assert!(taught
        .iter()
        .any(|e| matches!(e, SessionEvent::GossipComplete)));

    a.realm.handle(RealmCommand::GossipHello { guid: lee });
    let lines = gossip_lines(&drain(&mut a.realm));
    assert!(
        !lines.iter().any(|l| l == "Teach me to fish."),
        "the lesson is gone once learned: {lines:?}"
    );
}

#[test]
fn fishing_unknown_is_refused_until_brother_wendell_teaches_it() {
    let mut a = angler(1, false, true, Side::East);
    assert_eq!(refusal(&a.cast()), Some(SPELL_FAILED_NOT_KNOWN));

    let wendell = hub_guid(BROTHER_WENDELL);
    walk_up_to(&mut a.realm, wendell);
    a.realm.handle(RealmCommand::GossipHello { guid: wendell });
    let events = drain(&mut a.realm);
    let teach = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => options
                .iter()
                .find(|o| o.message == "Teach me to fish.")
                .map(|o| o.index),
            _ => None,
        })
        .expect("Brother Wendell offers the lesson");
    a.realm.handle(RealmCommand::GossipSelectOption {
        guid: wendell,
        option: teach,
    });
    let taught = drain(&mut a.realm);
    assert!(taught
        .iter()
        .any(|e| matches!(e, SessionEvent::SpellLearned { spell_id: FISHING })));

    a.realm.handle(RealmCommand::GossipHello { guid: wendell });
    let lines = gossip_lines(&drain(&mut a.realm));
    assert!(
        !lines.iter().any(|l| l == "Teach me to fish."),
        "the lesson is gone once learned: {lines:?}"
    );
}

#[test]
fn a_cast_without_a_pole_or_into_dry_land_is_refused() {
    let mut no_pole = angler(1, true, false, Side::East);
    assert_eq!(
        refusal(&no_pole.cast()),
        Some(SPELL_FAILED_EQUIPPED_ITEM_CLASS)
    );

    let mut dry = angler(1, true, true, Side::East);
    // Turned away from the pond.
    let [x, y] = dry.host.centre;
    dry.host.centre = [x - 30.0, y];
    assert_eq!(refusal(&dry.cast()), Some(SPELL_FAILED_NOT_FISHABLE));
}

#[test]
fn a_cast_into_the_pond_floats_a_bobber_and_an_elwynn_fish_bites() {
    let mut a = angler(1, true, true, Side::East);
    let (events, bite) = a.hook();
    let bobber = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate { guid, fields, .. }
                if fields
                    .raw_fields()
                    .any(|f| f == (GAMEOBJECT_TYPE_ID, FISHINGNODE)) =>
            {
                Some(*guid)
            }
            _ => None,
        })
        .expect("a bobber GameObject of type 17");
    let channel = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectValues { guid, fields } if *guid == a.guid => {
                fields.unit_channel_object()
            }
            _ => None,
        })
        .next();
    assert_eq!(channel, Some(bobber), "the channel names the bobber");
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::ChannelStart {
            spell_id: FISHING,
            ..
        }
    )));
    let SessionEvent::FishingBite {
        bobber: bit,
        fish,
        zone_height,
        ..
    } = bite
    else {
        unreachable!()
    };
    assert_eq!(bit, bobber);
    assert!(ELWYNN_FISH.contains(&fish), "{fish} is in the Elwynn pool");
    assert!((zone_height - 0.17).abs() < 1e-6, "zone {zone_height}");
}

#[test]
fn a_win_lands_the_fish_and_pays_angler_xp() {
    let mut a = angler(1, true, true, Side::North);
    a.hook();
    let landed = a.result(true, false, 6.0);
    assert_eq!(fishing_end(&landed), Some((Some(SMALLFISH), false)));
    assert!(landed.iter().any(|e| matches!(
        e,
        SessionEvent::ItemPushResult(p) if p.item_entry == SMALLFISH
    )));
    assert_eq!(angler_event(&landed), Some((1, 7)));

    a.hook();
    let perfect = a.result(true, true, 6.0);
    assert_eq!(fishing_end(&perfect), Some((Some(SMALLFISH), true)));
    assert_eq!(angler_event(&perfect), Some((1, 7 + 14)));
}

#[test]
fn a_win_faster_than_the_meter_fills_is_a_loss() {
    let mut a = angler(1, true, true, Side::North);
    a.hook();
    let events = a.result(true, false, 2.0);
    assert_eq!(fishing_end(&events), Some((None, false)));
    assert_eq!(angler_event(&events), None);
}

#[test]
fn stepping_during_the_bar_loses_the_fish_and_a_later_result_is_ignored() {
    let mut a = angler(1, true, true, Side::North);
    a.hook();
    a.realm.handle(RealmCommand::Move {
        position: [0.0, 0.0, 0.0],
        orientation: 0.0,
        motion: Default::default(),
    });
    let stepped = drain(&mut a.realm);
    assert_eq!(fishing_end(&stepped), Some((None, false)));
    let later = a.result(true, false, 6.0);
    assert_eq!(fishing_end(&later), None);
}

#[test]
fn a_fish_ten_levels_above_the_angler_never_bites() {
    let mut low = angler(1, true, true, Side::West);
    let events = low.cast();
    assert!(
        events.iter().any(|e| matches!(
            e,
            SessionEvent::Notification { text } if text == "Nothing is biting"
        )),
        "{events:?}"
    );

    let mut six = angler(6, true, true, Side::West);
    let (_, bite) = six.hook();
    let SessionEvent::FishingBite {
        fish, difficulty, ..
    } = bite
    else {
        unreachable!()
    };
    assert_eq!(fish, CATFISH);
    // The catfish's 35, and 5 for each of the nine levels above.
    assert!((difficulty - 80.0).abs() < 1e-4, "difficulty {difficulty}");
}

#[test]
fn a_gray_fish_pays_no_angler_xp() {
    let mut a = angler(30, true, true, Side::North);
    a.hook();
    let events = a.result(true, true, 6.0);
    assert_eq!(fishing_end(&events), Some((Some(SMALLFISH), true)));
    assert_eq!(angler_event(&events), Some((30, 0)));
}

#[test]
fn the_save_keeps_fishing_and_an_old_save_loads_at_the_defaults() {
    let (guid, blob, _) = save(6, true, false);
    let blob = blob.replace("angler_level = 6\n", "angler_level = 6\nangler_xp = 42\n");
    let mut realm = realm();
    realm.import_character(&blob).unwrap();
    let back = realm.export_character(guid).unwrap();
    for line in [
        "knows_fishing = true\n",
        "angler_level = 6\n",
        "angler_xp = 42\n",
    ] {
        assert!(back.contains(line), "{line:?} in\n{back}");
    }

    let old: String = back
        .lines()
        .filter(|l| !l.starts_with("knows_fishing") && !l.starts_with("angler_"))
        .map(|l| format!("{l}\n"))
        .collect();
    let mut realm = Realm::new();
    realm.import_character(&old).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    assert_eq!(angler_event(&drain(&mut realm)), Some((1, 0)));
    let back = realm.export_character(guid).unwrap();
    assert!(back.contains("angler_level = 1\n") && !back.contains("knows_fishing"));
}

/// The last `FishingJournal`'s total casts.
fn casts_in(events: &[SessionEvent]) -> Option<u32> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::FishingJournal { casts, .. } => Some(*casts),
        _ => None,
    })
}

/// The last `Angler` event's title.
fn title_in(events: &[SessionEvent]) -> Option<String> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::Angler { title, .. } => Some(title.clone()),
        _ => None,
    })
}

#[test]
fn a_landed_cast_counts_and_a_refused_one_does_not() {
    let (_, _, start) = save(1, true, true);
    let mut a = angler(1, true, true, Side::North);
    assert!(landed(&a.cast()));
    assert!(landed(&a.cast()));
    let back = from_save(a.guid, &saved(&a), start, Side::North);
    assert_eq!(casts_in(&back.events), Some(2));

    let mut bare = angler(1, true, false, Side::North);
    assert!(!landed(&bare.cast()));
    let back = from_save(bare.guid, &saved(&bare), start, Side::North);
    assert_eq!(casts_in(&back.events), Some(0));
}

#[test]
fn the_save_keeps_the_cast_count() {
    let (guid, blob, _) = save(6, true, false);
    let blob = blob.replace("fishing_casts = 0\n", "fishing_casts = 17\n");
    let mut realm = realm();
    realm.import_character(&blob).unwrap();
    let back = realm.export_character(guid).unwrap();
    assert!(back.contains("fishing_casts = 17\n"), "{back}");

    let old: String = back
        .lines()
        .filter(|l| !l.starts_with("fishing_casts"))
        .map(|l| format!("{l}\n"))
        .collect();
    let mut realm = Realm::new();
    realm.import_character(&old).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    assert_eq!(casts_in(&drain(&mut realm)), Some(0));
}

#[test]
fn the_angler_event_names_the_title() {
    let one = angler(1, true, true, Side::East);
    assert_eq!(title_in(&one.events).as_deref(), Some("Novice Fisher"));
    let twenty = angler(20, true, true, Side::East);
    assert_eq!(
        title_in(&twenty.events).as_deref(),
        Some("Journeyman Fisher")
    );
}

// ── Bait, lures and the Journal (#329) ──────────────────────────────────────────────────────────

const WRIGGLING_WORMS: u32 = 960101;
const SHINY_BAUBLE: u32 = 6529;
const NIGHTCRAWLERS: u32 = 6530;
const BRIGHT_BAUBLES: u32 = 6532;
const MUD_SNAPPER: u32 = 6289;
/// The backpack's first two slots, and the bag the backpack is.
const PACK_1: u8 = 23;
const PACK_2: u8 = 24;
const PLAYER_BAG: u8 = 255;

/// The last `FishingJournal` in `events`: its bait and its zones.
fn journal(events: &[SessionEvent]) -> Option<(Option<u32>, Vec<(u32, Vec<(u32, u8)>)>)> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::FishingJournal { bait, zones, .. } => Some((*bait, zones.clone())),
        _ => None,
    })
}

/// The Journal's best catch of `fish` in pool zone `zone`.
fn best(zones: &[(u32, Vec<(u32, u8)>)], zone: u32, fish: u32) -> Option<u8> {
    let (_, fish_of) = zones.iter().find(|(z, _)| *z == zone)?;
    fish_of.iter().find(|(f, _)| *f == fish).map(|&(_, b)| b)
}

fn zone_height(bite: &SessionEvent) -> f32 {
    match bite {
        SessionEvent::FishingBite { zone_height, .. } => *zone_height,
        _ => unreachable!(),
    }
}

/// The bar a `FishingBite` opens: `(start_fill, fill_per_sec, drain_per_sec)`.
fn bar_of(bite: &SessionEvent) -> (f32, f32, f32) {
    match bite {
        SessionEvent::FishingBite {
            start_fill,
            fill_per_sec,
            drain_per_sec,
            ..
        } => (*start_fill, *fill_per_sec, *drain_per_sec),
        _ => unreachable!(),
    }
}

impl Angler {
    fn set_bait(&mut self, item: Option<u32>) -> Vec<SessionEvent> {
        self.realm.handle(RealmCommand::SetBait { item });
        drain(&mut self.realm)
    }

    fn use_item(&mut self, slot: u8) -> Vec<SessionEvent> {
        self.realm.handle(RealmCommand::UseItem {
            bag: PLAYER_BAG,
            slot,
        });
        self.realm.tick(0.0, &self.host);
        drain(&mut self.realm)
    }
}

#[test]
fn login_sends_the_journal_with_every_pool_and_its_fish() {
    let a = angler(1, true, true, Side::East);
    let (bait, zones) = journal(&a.events).expect("a Journal at login");
    assert_eq!(bait, None);
    assert_eq!(
        best(&zones, ELWYNN, SMALLFISH),
        Some(0),
        "uncaught fish are listed"
    );
    for fish in ELWYNN_FISH {
        assert!(
            best(&zones, ELWYNN, fish).is_some(),
            "{fish} in the Elwynn page"
        );
    }
    let zone_text = a
        .events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::FishingJournal { zone_text, .. } => Some(zone_text.clone()),
            _ => None,
        })
        .expect("a Journal at login");
    for zone in [12, 1519, 44, 10] {
        assert!(zone_text.iter().any(|(z, _)| *z == zone), "zone {zone}");
    }
    assert!(
        zone_text.iter().all(|(_, t)| !t.is_empty()),
        "{zone_text:?}"
    );
}

#[test]
fn each_cast_that_lands_takes_one_bait_and_the_last_leaves_the_choice() {
    let (mut a, start) = holding(Side::East, &[(WRIGGLING_WORMS, 1, None)], &[], rich);
    let chose = a.set_bait(Some(WRIGGLING_WORMS));
    assert_eq!(journal(&chose).map(|j| j.0), Some(Some(WRIGGLING_WORMS)));

    let events = a.cast();
    assert!(landed(&events), "the cast landed");
    assert_eq!(
        journal(&events).map(|j| j.0),
        Some(Some(WRIGGLING_WORMS)),
        "the choice stays with the last bait gone"
    );
    assert_eq!(satchel_of(&events), Some(vec![]), "the Satchel has none");

    let bare = a.cast();
    assert!(landed(&bare), "a cast with no bait still lands");
    assert_eq!(journal(&bare), None, "nothing to take, nothing to send");
    assert!(saved(&a).contains("bait = 960101"), "{}", saved(&a));

    // Buying more puts them on the next cast without a SetBait.
    let bought = buy(&mut a, WRIGGLING_WORMS, 1);
    assert_eq!(satchel_of(&bought).map(|r| r.len()), Some(1));
    a.realm.handle(RealmCommand::Move {
        position: [start[0], start[1], 0.0],
        orientation: Side::East.facing(),
        motion: Default::default(),
    });
    a.realm.tick(0.0, &a.host);
    drain(&mut a.realm);
    let again = a.cast();
    assert!(landed(&again), "the cast landed");
    assert_eq!(satchel_of(&again), Some(vec![]), "the new worms were taken");
}

#[test]
fn a_refused_cast_takes_no_bait() {
    let mut a = angler_carrying(1, Side::East, &[(PACK_1, WRIGGLING_WORMS, 1)]);
    a.set_bait(Some(WRIGGLING_WORMS));
    let [x, y] = a.host.centre;
    a.host.centre = [x - 30.0, y];
    let events = a.cast();
    assert_eq!(refusal(&events), Some(SPELL_FAILED_NOT_FISHABLE));
    assert_eq!(journal(&events), None);
    let saved = a.realm.export_character(a.guid).unwrap();
    assert!(saved.contains("bait = 960101"), "{saved}");
}

#[test]
fn set_bait_ignores_an_item_not_carried_or_not_a_bait() {
    let mut a = angler_carrying(1, Side::East, &[(PACK_1, SMALLFISH, 1)]);
    assert_eq!(
        journal(&a.set_bait(Some(WRIGGLING_WORMS))),
        None,
        "not carried"
    );
    assert_eq!(journal(&a.set_bait(Some(SMALLFISH))), None, "not a bait");
    let saved = a.realm.export_character(a.guid).unwrap();
    assert!(!saved.contains("bait ="), "{saved}");
}

#[test]
fn a_shiny_bauble_starts_the_bar_fuller_and_gives_no_angler_level() {
    let mut a = angler_carrying(1, Side::East, &[(PACK_1, SHINY_BAUBLE, 1)]);
    let used = a.slot_lure(PACK_1, SHINY_BAUBLE);
    assert_eq!(
        lure_of(&used),
        Some(Some((SHINY_BAUBLE, 20, 20))),
        "using the lure slots it: {used:?}"
    );
    let (_, bite) = a.hook();
    assert!((bar_of(&bite).0 - 0.45).abs() < 1e-6, "{bite:?}");
    assert!((zone_height(&bite) - 0.17).abs() < 1e-6, "{bite:?}");

    // The L11 fish stays out of reach: the bauble adds no Angler level.
    let mut south = angler_carrying(1, Side::South, &[(PACK_1, SHINY_BAUBLE, 1)]);
    south.slot_lure(PACK_1, SHINY_BAUBLE);
    assert!(south.cast().iter().any(|e| matches!(
        e,
        SessionEvent::Notification { text } if text == "Nothing is biting"
    )));
}

#[test]
fn a_shiny_bauble_shortens_the_fastest_win() {
    // (1 - 0.45) / 0.12 x 0.95 = 4.354 s.
    let mut fast = angler_carrying(1, Side::East, &[(PACK_1, SHINY_BAUBLE, 1)]);
    fast.slot_lure(PACK_1, SHINY_BAUBLE);
    fast.hook();
    let landed = fast.result(true, false, 4.5);
    assert_eq!(fishing_end(&landed).map(|e| e.0.is_some()), Some(true));

    let mut early = angler_carrying(1, Side::East, &[(PACK_1, SHINY_BAUBLE, 1)]);
    early.slot_lure(PACK_1, SHINY_BAUBLE);
    early.hook();
    let lost = early.result(true, false, 4.2);
    assert_eq!(fishing_end(&lost), Some((None, false)));
}

#[test]
fn bright_baubles_fill_the_bar_faster_and_keep_the_zone() {
    let mut a = angler_carrying(1, Side::East, &[(PACK_1, BRIGHT_BAUBLES, 1)]);
    a.slot_lure(PACK_1, BRIGHT_BAUBLES);
    let (_, bite) = a.hook();
    assert!((bar_of(&bite).1 - 0.15).abs() < 1e-6, "{bite:?}");
    assert!((zone_height(&bite) - 0.17).abs() < 1e-6, "{bite:?}");
}

#[test]
fn nightcrawlers_slow_the_drain() {
    let mut a = angler_carrying(1, Side::East, &[(PACK_1, NIGHTCRAWLERS, 1)]);
    a.slot_lure(PACK_1, NIGHTCRAWLERS);
    let (_, bite) = a.hook();
    let (start, _, drain) = bar_of(&bite);
    assert!((drain - 0.126).abs() < 1e-6, "{bite:?}");
    assert!((start - 0.30).abs() < 1e-6, "{bite:?}");
}

#[test]
fn wriggling_worms_bring_the_bite_within_nine_seconds() {
    let mut a = angler_carrying(1, Side::East, &[(PACK_1, WRIGGLING_WORMS, 1)]);
    a.set_bait(Some(WRIGGLING_WORMS));
    let mut events = a.cast();
    // 12 x 0.75 = 9.0 s at the longest.
    events.extend(a.wait_for_bite(9.05));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SessionEvent::FishingBite { .. })),
        "no bite within 9.05 s"
    );
}

#[test]
fn the_journal_keeps_the_best_catch_and_the_save_keeps_it_and_the_bait() {
    let mut a = angler_carrying(1, Side::North, &[(PACK_1, WRIGGLING_WORMS, 5)]);
    a.hook();
    let first = a.result(true, false, 6.0);
    let (_, zones) = journal(&first).expect("a Journal on the first catch");
    assert_eq!(best(&zones, 990001, SMALLFISH), Some(1));

    a.hook();
    let perfect = a.result(true, true, 6.0);
    let (_, zones) = journal(&perfect).expect("a Journal on the perfect catch");
    assert_eq!(best(&zones, 990001, SMALLFISH), Some(2));

    a.hook();
    let again = a.result(true, false, 6.0);
    let (_, zones) = journal(&again).expect("every catch sends the Journal");
    assert_eq!(
        best(&zones, 990001, SMALLFISH),
        Some(2),
        "a plain catch after a perfect one keeps the best"
    );

    a.set_bait(Some(WRIGGLING_WORMS));
    let blob = a.realm.export_character(a.guid).unwrap();
    let mut back = Realm::new().with_fish(TEST_FISH);
    back.import_character(&blob).unwrap();
    back.handle(RealmCommand::EnterWorld { guid: a.guid });
    back.tick(0.0, &FlatGround);
    let (bait, zones) = journal(&drain(&mut back)).expect("a Journal at login");
    assert_eq!(bait, Some(WRIGGLING_WORMS));
    assert_eq!(best(&zones, 990001, SMALLFISH), Some(2));
}

// The pole and the lure in the Journal (#371).

const DARKWOOD_POLE: u32 = 6366;
const WORN_SHORTSWORD: u32 = 25;

/// The last `FishingJournal`'s Lure slot in `events`: `(entry, durability, max)`.
fn lure_of(events: &[SessionEvent]) -> Option<Option<(u32, u32, u32)>> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::FishingJournal { lure, .. } => Some(*lure),
        _ => None,
    })
}

/// The last `FishingJournal`'s Pole slot in `events`.
fn pole_of(events: &[SessionEvent]) -> Option<Option<u32>> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::FishingJournal { pole, .. } => Some(*pole),
        _ => None,
    })
}

/// How many of `entry` the saved character carries in the bags and worn slots.
fn carried(a: &Angler, entry: u32) -> usize {
    let blob = a.realm.export_character(a.guid).unwrap();
    let table: toml::Table = blob.parse().unwrap();
    table
        .get("items")
        .and_then(|i| i.as_array())
        .map_or(0, |items| {
            items
                .iter()
                .filter(|i| i["entry"].as_integer() == Some(i64::from(entry)))
                .count()
        })
}

/// A character with `extra` (a TOML table) added to its save, carrying `items`.
fn with_gear(side: Side, extra: &str, items: &[(u8, u32, u32)]) -> Angler {
    let (guid, mut blob, start) = save_carrying(1, true, false, items);
    blob.push_str(extra);
    from_save(guid, &blob, start, side)
}

fn lure_table(entry: u32, durability: u32) -> String {
    format!("\n[fishing_lure]\nslot = 0\nentry = {entry}\ncount = 1\ndurability = {durability}\n")
}

impl Angler {
    fn set_gear(&mut self, slot: FishingGearSlot, from: Option<SatchelPick>) -> Vec<SessionEvent> {
        self.realm
            .handle(RealmCommand::SetFishingGear { slot, from });
        drain(&mut self.realm)
    }

    /// Deposits the lure of `entry` at backpack slot `slot`, then puts it in the Lure slot.
    fn slot_lure(&mut self, slot: u8, entry: u32) -> Vec<SessionEvent> {
        self.realm.handle(RealmCommand::DepositToJournal {
            from: (PLAYER_BAG, slot),
        });
        self.set_gear(FishingGearSlot::Lure, pick(entry))
    }

    /// Ticks the bar out unanswered.
    fn let_it_go(&mut self) -> Vec<SessionEvent> {
        let mut all = Vec::new();
        for _ in 0..300 {
            self.realm.tick(0.1, &self.host);
            all.extend(drain(&mut self.realm));
        }
        all
    }
}

#[test]
fn a_pole_from_the_satchel_fills_the_slot_and_the_sword_stays_in_hand() {
    let (mut a, _) = holding_pole(
        false,
        Side::East,
        &[(FISHING_POLE, 1, Some(16))],
        &[(MAIN_HAND, WORN_SHORTSWORD, 1)],
        |b| b,
    );
    let set = a.set_gear(FishingGearSlot::Pole, pick(FISHING_POLE));
    assert_eq!(pole_of(&set), Some(Some(FISHING_POLE)));
    assert_eq!(satchel_of(&set), Some(vec![]), "the Satchel holds no pole");
    let events = a.cast();
    assert!(landed(&events), "the cast landed");
    assert_eq!(carried(&a, WORN_SHORTSWORD), 1, "the sword is still worn");
    let blob = a.realm.export_character(a.guid).unwrap();
    let table: toml::Table = blob.parse().unwrap();
    let sword = table["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["entry"].as_integer() == Some(i64::from(WORN_SHORTSWORD)))
        .unwrap();
    assert_eq!(sword["slot"].as_integer(), Some(i64::from(MAIN_HAND)));
}

#[test]
fn the_slots_take_only_their_own_kind_and_none_puts_the_item_back() {
    let (mut a, _) = holding_pole(
        false,
        Side::East,
        &[(SHINY_BAUBLE, 1, Some(20)), (FISHING_POLE, 1, Some(16))],
        &full_bags(),
        |b| b,
    );
    assert!(a
        .set_gear(FishingGearSlot::Pole, pick(SHINY_BAUBLE))
        .is_empty());
    assert!(a
        .set_gear(FishingGearSlot::Lure, pick(FISHING_POLE))
        .is_empty());
    assert!(
        a.set_gear(FishingGearSlot::Pole, pick(SHINY_BAUBLE + 1))
            .is_empty(),
        "a pick the Satchel does not hold changes nothing"
    );
    let set = a.set_gear(FishingGearSlot::Pole, pick(FISHING_POLE));
    assert_eq!(pole_of(&set), Some(Some(FISHING_POLE)));
    let cleared = a.set_gear(FishingGearSlot::Pole, None);
    assert_eq!(pole_of(&cleared), Some(None));
    let rows = satchel_of(&cleared).expect("the Journal follows");
    assert!(
        rows.iter().any(|r| r.item == FISHING_POLE),
        "back in the Satchel: {rows:?}"
    );
    assert_eq!(carried(&a, TROPHY_FILL), 16, "every bag slot still full");
}

#[test]
fn a_main_hand_pole_does_not_cast_without_one_in_the_slot() {
    let mut a = carrying(1, Side::East, true, false, &[(MAIN_HAND, FISHING_POLE, 1)]);
    assert_eq!(refusal(&a.cast()), Some(SPELL_FAILED_EQUIPPED_ITEM_CLASS));
}

#[test]
fn a_darkwood_pole_adds_three_angler_levels_to_the_zone() {
    let extra = format!("\n[fishing_pole]\nslot = 0\nentry = {DARKWOOD_POLE}\ncount = 1\n");
    let mut a = with_gear(Side::East, &extra, &[]);
    let (_, bite) = a.hook();
    assert!((zone_height(&bite) - 0.1805).abs() < 1e-6, "{bite:?}");
}

#[test]
fn a_lure_at_one_point_breaks_at_the_bite_and_the_next_one_takes_its_place() {
    let (mut a, _) = holding(
        Side::East,
        &[(SHINY_BAUBLE, 1, Some(20)), (SHINY_BAUBLE, 1, Some(20))],
        &[],
        |b| b + &lure_table(SHINY_BAUBLE, 1),
    );
    let (events, _) = a.hook();
    assert_eq!(lure_of(&events), Some(Some((SHINY_BAUBLE, 20, 20))));
    assert!(
        format!("{events:?}").contains("Your Shiny Bauble broke. Another one is on the line."),
        "the system line"
    );
    // The slot holds one of the two; one is left in the Satchel.
    let rows = satchel_of(&events).expect("the Journal follows");
    assert_eq!(rows.iter().filter(|r| r.item == SHINY_BAUBLE).count(), 1);
}

#[test]
fn a_last_lure_empties_the_slot_and_only_its_own_kind_refills_it() {
    let (mut a, _) = holding(Side::East, &[(BRIGHT_BAUBLES, 1, Some(20))], &[], |b| {
        rich(b) + &lure_table(SHINY_BAUBLE, 1)
    });
    let (events, _) = a.hook();
    assert_eq!(lure_of(&events), Some(None));
    assert_eq!(waiting_of(&events), Some(Some(SHINY_BAUBLE)));
    assert!(format!("{events:?}").contains("Your Shiny Bauble broke."));
    let rows = satchel_of(&events).expect("the Journal follows");
    assert!(rows.iter().any(|r| r.item == BRIGHT_BAUBLES), "{rows:?}");

    a.let_it_go();
    let bought = buy(&mut a, SHINY_BAUBLE, 1);
    assert_eq!(lure_of(&bought), Some(Some((SHINY_BAUBLE, 20, 20))));
    assert_eq!(waiting_of(&bought), Some(None));
    let rows = satchel_of(&bought).expect("the Journal follows");
    assert!(rows.iter().any(|r| r.item == BRIGHT_BAUBLES), "{rows:?}");
}

#[test]
fn clearing_the_lure_slot_forgets_what_it_waited_for() {
    let (mut a, _) = holding(
        Side::East,
        &[],
        &[(PACK_1, SHINY_BAUBLE, 1)],
        waiting_for(SHINY_BAUBLE),
    );
    let cleared = a.set_gear(FishingGearSlot::Lure, None);
    assert_eq!(waiting_of(&cleared), Some(None));
    a.realm.handle(RealmCommand::DepositToJournal {
        from: (PLAYER_BAG, PACK_1),
    });
    let events = drain(&mut a.realm);
    assert_eq!(lure_of(&events), Some(None), "the slot stays empty");
    let rows = satchel_of(&events).expect("the Journal follows");
    assert!(rows.iter().any(|r| r.item == SHINY_BAUBLE), "{rows:?}");
}

#[test]
fn using_a_bag_lure_deposits_it_and_leaves_the_lure_slot_as_it_was() {
    let (mut a, _) = holding(Side::East, &[], &[(PACK_1, SHINY_BAUBLE, 1)], |b| {
        b + &lure_table(BRIGHT_BAUBLES, 9)
    });
    let used = a.use_item(PACK_1);
    assert_eq!(lure_of(&used), Some(Some((BRIGHT_BAUBLES, 9, 20))));
    let rows = satchel_of(&used).expect("the Journal follows");
    assert!(rows.iter().any(|r| r.item == SHINY_BAUBLE), "{rows:?}");
}

#[test]
fn the_save_keeps_what_the_lure_slot_waits_for() {
    let (a, _) = holding(Side::East, &[], &[], waiting_for(SHINY_BAUBLE));
    let blob = saved(&a);
    assert!(blob.contains("lure_waiting = 6529"), "{blob}");
    let mut back = Realm::new().with_fish(TEST_FISH);
    back.import_character(&blob).unwrap();
    back.handle(RealmCommand::EnterWorld { guid: a.guid });
    back.tick(0.0, &FlatGround);
    assert_eq!(waiting_of(&drain(&mut back)), Some(Some(SHINY_BAUBLE)));
}

#[test]
fn a_lost_fish_still_costs_the_lure_a_point() {
    let mut a = with_gear(
        Side::East,
        &format!(
            "{}\n[fishing_pole]\nslot = 0\nentry = {FISHING_POLE}\ncount = 1\n",
            lure_table(SHINY_BAUBLE, 5)
        ),
        &[],
    );
    a.hook();
    a.let_it_go();
    let blob = a.realm.export_character(a.guid).unwrap();
    let table: toml::Table = blob.parse().unwrap();
    assert_eq!(table["fishing_lure"]["durability"].as_integer(), Some(4));
}

#[test]
fn a_lure_is_never_repaired_or_charged_for() {
    use twilight_realm::rules::durability::repair_cost;
    assert_eq!(repair_cost(SHINY_BAUBLE, 12), 0);
}

#[test]
fn the_save_round_trip_keeps_the_pole_and_the_lure_with_its_durability() {
    let a = with_gear(
        Side::East,
        &format!(
            "{}\n[fishing_pole]\nslot = 0\nentry = {DARKWOOD_POLE}\ncount = 1\n",
            lure_table(SHINY_BAUBLE, 12)
        ),
        &[],
    );
    let blob = a.realm.export_character(a.guid).unwrap();
    let mut back = Realm::new().with_fish(TEST_FISH);
    back.import_character(&blob).unwrap();
    back.handle(RealmCommand::EnterWorld { guid: a.guid });
    back.tick(0.0, &FlatGround);
    let events = drain(&mut back);
    assert_eq!(pole_of(&events), Some(Some(DARKWOOD_POLE)));
    assert_eq!(lure_of(&events), Some(Some((SHINY_BAUBLE, 12, 20))));
}

// Legendary Fish (#331). The test pool's Legendary Fish all bite at 100 percent when their
// criteria hold, in this order: Gar (the night block), Bream (the dawn block),
// Snapper (Gnoll Ear as bait) and Gate (Angler Level 5). The game clock runs 5 wall seconds to
// a game minute.

const GAR: u32 = 6309;
const BREAM: u32 = 6310;
const SNAPPER: u32 = 6361;
const GATE: u32 = 6358;
const RUMOUR_FISH: u32 = 6303;
const GNOLL_EAR: u32 = 960103;
const AQUADYNAMIC: u32 = 6533;
const LEGEND_NOTE: u32 = 960206;
const LEE_BROWN_HINT: &str = "Heard anything about the fish around here?";
const OLD_GOLDFIN: u32 = 960001;

/// Sets the game clock to `hour`:`minute`.
fn at_game(a: &mut Angler, hour: u64, minute: u64) {
    a.realm.set_wall_clock((hour * 60 + minute) * 5 * 1000);
}

/// The Legendary Fish a bite is, if it is one.
fn legend_of(bite: &SessionEvent) -> Option<u32> {
    match bite {
        SessionEvent::FishingBite {
            fish,
            legendary: true,
            ..
        } => Some(*fish),
        _ => None,
    }
}

/// The last Journal's Legendary Fish: `(item, zone, hint found, caught)`.
fn legends_of(events: &[SessionEvent]) -> Option<Vec<(u32, u32, bool, bool)>> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::FishingJournal { legends, .. } => Some(legends.clone()),
        _ => None,
    })
}

#[test]
fn the_gar_bites_by_night_and_never_at_noon() {
    let mut night = angler(1, true, true, Side::NorthWest);
    at_game(&mut night, 22, 0);
    let (_, bite) = night.hook();
    assert_eq!(legend_of(&bite), Some(GAR));

    let mut noon = angler(1, true, true, Side::NorthWest);
    at_game(&mut noon, 12, 0);
    let (_, bite) = noon.hook();
    assert_ne!(legend_of(&bite), Some(GAR), "{bite:?}");
}

#[test]
fn the_bream_bites_at_dawn_only() {
    let mut early = angler(1, true, true, Side::NorthWest);
    at_game(&mut early, 5, 0);
    let (_, bite) = early.hook();
    assert_eq!(legend_of(&bite), Some(BREAM));

    let mut late = angler(1, true, true, Side::NorthWest);
    at_game(&mut late, 12, 0);
    let (_, bite) = late.hook();
    assert_eq!(legend_of(&bite), None, "{bite:?}");
}

#[test]
fn the_snapper_needs_the_gnoll_ear_chosen_as_bait() {
    let mut bare = angler_carrying(1, Side::NorthWest, &[(PACK_1, GNOLL_EAR, 1)]);
    at_game(&mut bare, 12, 30);
    let (_, bite) = bare.hook();
    assert_eq!(legend_of(&bite), None, "carried but not chosen: {bite:?}");

    let mut baited = angler_carrying(1, Side::NorthWest, &[(PACK_1, GNOLL_EAR, 1)]);
    baited.set_bait(Some(GNOLL_EAR));
    at_game(&mut baited, 12, 30);
    let (_, bite) = baited.hook();
    assert_eq!(legend_of(&bite), Some(SNAPPER));
}

#[test]
fn a_legendary_above_the_effective_angler_level_waits_for_a_pole() {
    let mut plain = angler(2, true, true, Side::NorthWest);
    at_game(&mut plain, 12, 30);
    let (_, bite) = plain.hook();
    assert_eq!(legend_of(&bite), None, "{bite:?}");

    // The Darkwood pole counts as three levels more: 2 + 3 reaches the Gate's 5.
    let (guid, mut blob, start) = save_carrying(2, true, false, &[]);
    blob.push_str(&format!(
        "\n[fishing_pole]\nslot = 0\nentry = {DARKWOOD_POLE}\ncount = 1\n"
    ));
    let mut poled = from_save(guid, &blob, start, Side::NorthWest);
    at_game(&mut poled, 12, 30);
    let (_, bite) = poled.hook();
    assert_eq!(legend_of(&bite), Some(GATE));

    // The Aquadynamic Fish Attractor adds no level.
    let mut lured = angler_carrying(2, Side::NorthWest, &[(PACK_1, AQUADYNAMIC, 1)]);
    lured.slot_lure(PACK_1, AQUADYNAMIC);
    at_game(&mut lured, 12, 30);
    let (_, bite) = lured.hook();
    assert_eq!(legend_of(&bite), None, "{bite:?}");
}

#[test]
fn a_caught_legendary_pays_once_shows_in_the_journal_and_never_bites_again() {
    let mut a = angler(1, true, true, Side::NorthWest);
    at_game(&mut a, 23, 30);
    let (_, bite) = a.hook();
    assert_eq!(legend_of(&bite), Some(GAR));
    let landed = a.result(true, false, 6.0);
    assert_eq!(fishing_end(&landed).map(|e| e.0), Some(Some(GAR)));
    // The 500 bonus XP carries Angler 1 well past its first level.
    assert!(angler_event(&landed).is_some_and(|(level, _)| level > 1));
    let legends = legends_of(&landed).expect("a Journal on the catch");
    assert!(legends.iter().any(|&(i, _, _, caught)| i == GAR && caught));
    assert!(legends
        .iter()
        .any(|&(i, _, _, caught)| i == BREAM && !caught));
    let saved = a.realm.export_character(a.guid).unwrap();
    assert!(saved.contains("legends_caught"), "{saved}");

    at_game(&mut a, 23, 30);
    let (_, again) = a.hook();
    assert_eq!(legend_of(&again), None, "{again:?}");
}

#[test]
fn a_catch_in_the_pool_hits_its_legend_rumour_once() {
    let mut a = angler(1, true, true, Side::NorthEast);
    a.hook();
    let first = a.result(true, false, 6.0);
    let legends = legends_of(&first).expect("the Journal follows the hit");
    assert!(
        legends
            .iter()
            .any(|&(i, _, hint, caught)| i == RUMOUR_FISH && hint && !caught),
        "{legends:?}"
    );
    let saved = a.realm.export_character(a.guid).unwrap();
    assert!(saved.contains(&format!("entry = {LEGEND_NOTE}")), "{saved}");

    a.hook();
    let second = a.result(true, false, 6.0);
    assert!(
        !second
            .iter()
            .any(|e| matches!(e, SessionEvent::ItemPushResult(p) if p.item_entry == LEGEND_NOTE)),
        "no second roll: {second:?}"
    );
    let saved = a.realm.export_character(a.guid).unwrap();
    let note = format!("entry = {LEGEND_NOTE}");
    assert_eq!(saved.matches(&note).count(), 1, "{saved}");
}

#[test]
fn lee_browns_hint_line_shows_at_angler_five_and_records_the_hint() {
    let lee = hub_guid(LEE_BROWN);
    let mut four = angler(4, true, true, Side::East);
    walk_up_to(&mut four.realm, lee);
    four.realm.handle(RealmCommand::GossipHello { guid: lee });
    let lines = gossip_lines(&drain(&mut four.realm));
    assert!(!lines.iter().any(|l| l == LEE_BROWN_HINT), "{lines:?}");

    let mut five = angler(5, true, true, Side::East);
    walk_up_to(&mut five.realm, lee);
    five.realm.handle(RealmCommand::GossipHello { guid: lee });
    let events = drain(&mut five.realm);
    let hint = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => options
                .iter()
                .find(|o| o.message == LEE_BROWN_HINT)
                .map(|o| o.index),
            _ => None,
        })
        .expect("the hint line at Angler 5");
    five.realm.handle(RealmCommand::GossipSelectOption {
        guid: lee,
        option: hint,
    });
    let picked = drain(&mut five.realm);
    let legends = legends_of(&picked).expect("the Journal follows the hint");
    assert!(
        legends
            .iter()
            .any(|&(i, _, hinted, caught)| i == OLD_GOLDFIN && hinted && !caught),
        "{legends:?}"
    );
}

#[test]
fn brother_wendells_hint_line_shows_at_angler_three_and_records_the_hint() {
    const BROTHER_BASS: u32 = 960002;
    let wendell = hub_guid(BROTHER_WENDELL);
    let mut two = angler(2, true, true, Side::East);
    walk_up_to(&mut two.realm, wendell);
    two.realm
        .handle(RealmCommand::GossipHello { guid: wendell });
    let lines = gossip_lines(&drain(&mut two.realm));
    assert!(!lines.iter().any(|l| l == LEE_BROWN_HINT), "{lines:?}");

    let mut three = angler(3, true, true, Side::East);
    walk_up_to(&mut three.realm, wendell);
    three
        .realm
        .handle(RealmCommand::GossipHello { guid: wendell });
    let events = drain(&mut three.realm);
    let hint = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => options
                .iter()
                .find(|o| o.message == LEE_BROWN_HINT)
                .map(|o| o.index),
            _ => None,
        })
        .expect("the hint line at Angler 3");
    three.realm.handle(RealmCommand::GossipSelectOption {
        guid: wendell,
        option: hint,
    });
    let picked = drain(&mut three.realm);
    let legends = legends_of(&picked).expect("the Journal follows the hint");
    assert!(
        legends
            .iter()
            .any(|&(i, _, hinted, caught)| i == BROTHER_BASS && hinted && !caught),
        "{legends:?}"
    );
}

#[test]
fn lee_browns_smallfry_quest_is_handed_in_twice_and_paid_twice() {
    const SMALLFRY: u32 = 96101;
    // Twenty smallfish in the backpack: two lots of ten.
    let mut a = angler_carrying(1, Side::East, &[(23, SMALLFISH, 20)]);
    let lee = hub_guid(LEE_BROWN);
    let mut paid = Vec::new();
    for _ in 0..2 {
        accept(&mut a.realm, lee, SMALLFRY);
        let done = completed(&hand_in(&mut a.realm, lee, SMALLFRY)).expect("the quest completes");
        assert_eq!(done.quest_id, SMALLFRY);
        paid.push((done.money, done.xp));
    }
    // The first lot's XP can level the character, so the second lot's XP is not the first's.
    for (money, xp) in paid {
        assert_eq!(money, 50, "50 copper");
        assert!(xp > 0, "quest XP");
    }
}

// Water Kinds (#370, spec #368): a fish bites only in the kinds it lists.

const STORMWIND: u32 = 1519;
const MACKEREL: u32 = 6303;
const ALBACORE: u32 = 6361;
const COAST_BARRED: [u32; 4] = [6289, 6308, 6309, 6310];
const CANAL_BARRED: [u32; 2] = [6303, 6361];

/// Every fish `n` bites at Angler 60 bring in, a lost cast each time.
fn bitten(a: &mut Angler, n: usize) -> std::collections::HashSet<u32> {
    let mut seen = std::collections::HashSet::new();
    for _ in 0..n {
        let (_, bite) = a.hook();
        let SessionEvent::FishingBite { fish, .. } = bite else {
            unreachable!()
        };
        seen.insert(fish);
        a.result(false, false, 1.0);
    }
    seen
}

#[test]
fn stormwinds_coast_never_bites_river_fish_and_the_canal_never_bites_coast_fish() {
    let mut coast = angler(60, true, true, Side::East);
    coast.host.area = Some(STORMWIND);
    let seen = bitten(&mut coast, 2000);
    for fish in COAST_BARRED {
        assert!(!seen.contains(&fish), "{fish} bit on the coast: {seen:?}");
    }
    assert!(seen.contains(&SMALLFISH) && seen.contains(&MACKEREL));

    // The Cathedral Canal's circle is a river, inside the Stormwind pool.
    let mut canal = angler(60, true, true, Side::East);
    canal.host.area = Some(STORMWIND);
    canal.host.centre = [-8560.0, 820.0];
    canal.realm.handle(RealmCommand::Move {
        position: [-8575.0, 820.0, 0.0],
        orientation: 0.0,
        motion: Default::default(),
    });
    canal.realm.tick(0.0, &canal.host);
    drain(&mut canal.realm);
    let seen = bitten(&mut canal, 2000);
    for fish in CANAL_BARRED {
        assert!(!seen.contains(&fish), "{fish} bit in the canal: {seen:?}");
    }
    assert!(seen.contains(&SMALLFISH) && seen.contains(&CATFISH));
    assert!(!seen.contains(&ALBACORE));
}

#[test]
fn a_fish_with_no_kinds_bites_in_lake_water_and_a_lake_fish_does_too() {
    let mut elwynn = angler(60, true, true, Side::East);
    let seen = bitten(&mut elwynn, 500);
    assert!(seen.contains(&SMALLFISH) && seen.contains(&MUD_SNAPPER));
}

// The catch log and the Fish Page (#372, spec #368).

/// The Fish Page of `item` in the last `FishingJournal` of `events`.
fn page_of(events: &[SessionEvent], item: u32) -> Option<FishPage> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::FishingJournal { pages, .. } => {
            pages.iter().find(|p| p.item == item).cloned()
        }
        _ => None,
    })
}

/// Casts until `fish` bites and lands it, `n` times: another fish that bites is let go. The events
/// of the last landing.
fn land_fish(a: &mut Angler, fish: u32, n: usize) -> Vec<SessionEvent> {
    let mut last = Vec::new();
    for _ in 0..n {
        loop {
            let (_, bite) = a.hook();
            let SessionEvent::FishingBite { fish: bit, .. } = bite else {
                unreachable!()
            };
            if bit == fish {
                last = a.result(true, false, 6.0);
                break;
            }
            a.result(false, false, 1.0);
        }
    }
    last
}

/// A page's `lives_in` without the test pools, which hold the snapper too.
fn shipped_lives_in(page: &FishPage) -> Vec<(u32, String)> {
    page.lives_in
        .iter()
        .filter(|(zone, _)| *zone < 990000)
        .cloned()
        .collect()
}

#[test]
fn a_fish_page_unlocks_its_facts_at_five_ten_and_twenty_five_catches() {
    let mut a = angler(20, true, true, Side::East);
    let events = land_fish(&mut a, MUD_SNAPPER, 4);
    let page = page_of(&events, MUD_SNAPPER).expect("a page after the first catch");
    assert_eq!((page.caught, page.perfect), (4, 0));
    assert_eq!((page.revealed, page.next_reveal), (0, 5));
    assert!(page.lives_in.is_empty() && page.kinds.is_empty());
    assert!(page.peak_hours.is_none() && page.favourite_bait.is_none());

    let events = land_fish(&mut a, MUD_SNAPPER, 1);
    let page = page_of(&events, MUD_SNAPPER).unwrap();
    assert_eq!((page.revealed, page.next_reveal), (1, 10));
    let expected: Vec<(u32, String)> = [
        (12, ""),
        (44, ""),
        (10, ""),
        (12, "Northshire River"),
        (1519, "Cathedral Canal"),
        (44, "Alther's Mill"),
        (44, "Lake Everstill"),
        (12, "Crystal Lake"),
        (10, "The Hushed Bank"),
        (10, "The Darkened Bank"),
        (10, "Twilight Grove"),
    ]
    .map(|(zone, name)| (zone, name.to_owned()))
    .to_vec();
    assert_eq!(shipped_lives_in(&page), expected);
    assert!(page.kinds.is_empty() && page.favourite_bait.is_none());

    let events = land_fish(&mut a, MUD_SNAPPER, 5);
    let page = page_of(&events, MUD_SNAPPER).unwrap();
    assert_eq!((page.revealed, page.next_reveal), (2, 25));
    assert_eq!(page.kinds, ["river", "lake", "pond"]);
    assert_eq!(page.peak_hours, Some((3, 9)));
    assert!(page.favourite_bait.is_none());

    let events = land_fish(&mut a, MUD_SNAPPER, 15);
    let page = page_of(&events, MUD_SNAPPER).unwrap();
    assert_eq!((page.caught, page.revealed, page.next_reveal), (25, 3, 0));
    assert_eq!(page.favourite_bait, Some(WRIGGLING_WORMS));
}

#[test]
fn catches_in_the_river_and_in_plain_water_are_two_places_the_larger_first() {
    let (guid, blob, start) = save(20, true, true);
    let mut a = from_save(guid, &blob, start, Side::East);
    a.host.centre = [-8830.0, -330.0];
    a.realm.handle(RealmCommand::Move {
        position: [-8845.0, -330.0, 0.0],
        orientation: 0.0,
        motion: Default::default(),
    });
    a.realm.tick(0.0, &a.host);
    drain(&mut a.realm);
    land_fish(&mut a, MUD_SNAPPER, 2);

    a.host.centre = pond_centre(start, Side::East);
    a.realm.handle(RealmCommand::Move {
        position: [start[0], start[1], 0.0],
        orientation: Side::East.facing(),
        motion: Default::default(),
    });
    a.realm.tick(0.0, &a.host);
    drain(&mut a.realm);
    let events = land_fish(&mut a, MUD_SNAPPER, 1);
    let page = page_of(&events, MUD_SNAPPER).unwrap();
    assert_eq!(
        page.places,
        [
            (ELWYNN, "Northshire River".to_owned(), 2),
            (ELWYNN, String::new(), 1)
        ]
    );
}

#[test]
fn an_old_catches_save_loads_as_a_log_and_saves_back_as_one() {
    let (guid, blob, _) = save(1, true, false);
    let blob = blob.replace(
        "angler_level = 1\n",
        "angler_level = 1\ncatches = [[6291, 2]]\n",
    );
    let mut realm = realm();
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let page = page_of(&drain(&mut realm), SMALLFISH).expect("a page for the old catch");
    assert_eq!((page.caught, page.perfect), (1, 1));
    assert!(page.places.is_empty());
    let back = realm.export_character(guid).unwrap();
    assert!(back.contains("fish_log"), "{back}");
    assert!(!back.contains("catches"), "{back}");
}

// The Satchel (#391, ADR 0023): Journal Items never take a bag slot.

const TROPHY_FILL: u32 = WORN_SHORTSWORD;

fn satchel_of(events: &[SessionEvent]) -> Option<Vec<SatchelRow>> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::FishingJournal { satchel, .. } => Some(satchel.clone()),
        _ => None,
    })
}

fn row(item: u32, count: u32) -> SatchelRow {
    SatchelRow {
        item,
        count,
        wear: None,
        tackle: false,
    }
}

/// A fisher who holds `satchel` (`(entry, count, durability)`) and carries `items`, in a version
/// 5 save; `blob` lets a test edit the save first.
fn holding(
    side: Side,
    satchel: &[(u32, u32, Option<u32>)],
    items: &[(u8, u32, u32)],
    blob: impl FnOnce(String) -> String,
) -> (Angler, [f32; 3]) {
    holding_pole(true, side, satchel, items, blob)
}

/// [`holding`], with a pole in the Pole slot or not.
fn holding_pole(
    pole: bool,
    side: Side,
    satchel: &[(u32, u32, Option<u32>)],
    items: &[(u8, u32, u32)],
    blob: impl FnOnce(String) -> String,
) -> (Angler, [f32; 3]) {
    let (guid, mut text, start) = save_carrying(1, true, pole, items);
    for (entry, count, durability) in satchel {
        text.push_str(&format!(
            "\n[[satchel]]\nentry = {entry}\ncount = {count}\n"
        ));
        if let Some(d) = durability {
            text.push_str(&format!("durability = {d}\n"));
        }
    }
    (from_save(guid, &blob(text), start, side), start)
}

/// A save with copper enough for the shop.
fn rich(blob: String) -> String {
    let (head, tail) = blob.split_once("money = ").expect("a save keeps money");
    let rest = tail.split_once('\n').expect("a line").1;
    format!("{head}money = 100000\n{rest}")
}

/// A gear request for the Satchel piece of `item`.
fn pick(item: u32) -> Option<SatchelPick> {
    Some(SatchelPick {
        item,
        durability: None,
    })
}

/// Whether a cast landed its bobber.
fn landed(events: &[SessionEvent]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::ChannelStart { .. }))
}

/// The last Journal's kept lure entry.
fn waiting_of(events: &[SessionEvent]) -> Option<Option<u32>> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::FishingJournal { lure_waiting, .. } => Some(*lure_waiting),
        _ => None,
    })
}

/// A save with the Lure slot waiting to refill with `entry`.
fn waiting_for(entry: u32) -> impl FnOnce(String) -> String {
    move |b| {
        b.replacen(
            "version = 5\n",
            &format!("version = 5\nlure_waiting = {entry}\n"),
            1,
        )
    }
}

fn saved(a: &Angler) -> String {
    a.realm.export_character(a.guid).unwrap()
}

/// Every backpack slot full of something else.
fn full_bags() -> Vec<(u8, u32, u32)> {
    (23..39).map(|slot| (slot, TROPHY_FILL, 1)).collect()
}

fn buy(a: &mut Angler, entry: u32, count: u8) -> Vec<SessionEvent> {
    let lee = hub_guid(LEE_BROWN);
    walk_up_to(&mut a.realm, lee);
    a.realm.handle(RealmCommand::ListInventory { guid: lee });
    a.realm.handle(RealmCommand::BuyItem {
        vendor: lee,
        entry,
        count,
    });
    drain(&mut a.realm)
}

#[test]
fn a_won_catch_with_full_bags_lands_in_the_satchel() {
    let (mut a, _) = holding(Side::North, &[], &full_bags(), |b| b);
    a.hook();
    let events = a.result(true, false, 6.0);
    assert!(
        events.iter().any(|e| matches!(
            e,
            SessionEvent::FishingEnd {
                caught: Some(SMALLFISH),
                ..
            }
        )),
        "{events:?}"
    );
    assert_eq!(satchel_of(&events), Some(vec![row(SMALLFISH, 1)]));
    let blob = saved(&a);
    assert_eq!(blob.matches("[[items]]").count(), 16, "{blob}");
    assert!(!blob.contains(&format!("entry = {SMALLFISH}\ncount = 1\ndurability")));
}

#[test]
fn buying_worms_with_full_bags_succeeds_and_the_satchel_holds_them() {
    let (mut a, _) = holding(Side::East, &[], &full_bags(), rich);
    let events = buy(&mut a, WRIGGLING_WORMS, 5);
    let worms = SatchelRow {
        tackle: true,
        ..row(WRIGGLING_WORMS, 5)
    };
    assert_eq!(satchel_of(&events), Some(vec![worms]));
}

#[test]
fn two_shiny_baubles_are_two_pieces_after_the_counted_rows() {
    let (mut a, _) = holding(Side::East, &[(SMALLFISH, 2, None)], &full_bags(), rich);
    let events = buy(&mut a, SHINY_BAUBLE, 2);
    let piece = SatchelRow {
        item: SHINY_BAUBLE,
        count: 1,
        wear: Some((20, 20)),
        tackle: true,
    };
    assert_eq!(
        satchel_of(&events),
        Some(vec![row(SMALLFISH, 2), piece.clone(), piece])
    );
}

#[test]
fn withdrawing_moves_to_the_bags_and_a_later_catch_stays_in_the_satchel() {
    let (mut a, _) = holding(Side::North, &[(SMALLFISH, 10, None)], &[], |b| b);
    a.realm.handle(RealmCommand::WithdrawFromJournal {
        pick: SatchelPick {
            item: SMALLFISH,
            durability: None,
        },
        count: 3,
    });
    let events = drain(&mut a.realm);
    assert_eq!(satchel_of(&events), Some(vec![row(SMALLFISH, 7)]));
    assert!(saved(&a).contains(&format!("entry = {SMALLFISH}\ncount = 3\n")));

    a.hook();
    let events = a.result(true, false, 6.0);
    assert_eq!(satchel_of(&events), Some(vec![row(SMALLFISH, 8)]));
    assert!(saved(&a).contains(&format!("entry = {SMALLFISH}\ncount = 3\n")));

    // Depositing the bag stack puts it back.
    a.realm.handle(RealmCommand::DepositToJournal {
        from: (PLAYER_BAG, PACK_1),
    });
    let events = drain(&mut a.realm);
    assert_eq!(satchel_of(&events), Some(vec![row(SMALLFISH, 11)]));
    assert!(!saved(&a).contains("[[items]]"), "{}", saved(&a));
}

#[test]
fn a_withdrawn_bauble_deposits_on_use_and_a_withdrawn_fish_is_eaten() {
    let (mut a, _) = holding(
        Side::East,
        &[(SHINY_BAUBLE, 1, Some(14)), (SMALLFISH, 2, None)],
        &[],
        |b| b,
    );
    a.realm.handle(RealmCommand::WithdrawFromJournal {
        pick: SatchelPick {
            item: SHINY_BAUBLE,
            durability: Some(14),
        },
        count: 1,
    });
    a.realm.handle(RealmCommand::WithdrawFromJournal {
        pick: SatchelPick {
            item: SMALLFISH,
            durability: None,
        },
        count: 1,
    });
    drain(&mut a.realm);
    let bag = saved(&a);
    assert!(bag.contains(&format!(
        "entry = {SHINY_BAUBLE}\ncount = 1\ndurability = 14"
    )));
    let items_from = bag.find("[[items]]").expect("bag items");
    let slot_of = |entry: u32| {
        let at = items_from
            + bag[items_from..]
                .find(&format!("entry = {entry}\n"))
                .unwrap();
        let head = &bag[..at];
        let s = head.rfind("slot = ").unwrap() + 7;
        head[s..].lines().next().unwrap().parse::<u8>().unwrap()
    };
    let (bauble, fish) = (slot_of(SHINY_BAUBLE), slot_of(SMALLFISH));
    let used = a.use_item(bauble);
    let rows = satchel_of(&used).expect("the Journal follows");
    assert!(rows.iter().any(|r| r.wear == Some((14, 20))), "{rows:?}");
    a.use_item(fish);
    let blob = saved(&a);
    // The fish was eaten, not deposited: the Satchel still holds the one it kept.
    assert!(
        blob.contains(&format!("[[satchel]]\nentry = {SMALLFISH}\ncount = 1\n")),
        "{blob}"
    );
}

#[test]
fn withdrawing_into_full_bags_moves_nothing_and_says_so() {
    let (mut a, _) = holding(Side::East, &[(SMALLFISH, 4, None)], &full_bags(), |b| b);
    a.realm.handle(RealmCommand::WithdrawFromJournal {
        pick: SatchelPick {
            item: SMALLFISH,
            durability: None,
        },
        count: 4,
    });
    let events = drain(&mut a.realm);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SessionEvent::InventoryFailure { .. })),
        "{events:?}"
    );
    assert!(satchel_of(&events).is_none());
}

#[test]
fn a_collect_quest_counts_the_satchel_and_the_bags_and_takes_the_satchel_first() {
    const SMALLFRY: u32 = 96101;
    let lee = hub_guid(LEE_BROWN);
    for (satchel, bags, left) in [(7, 3, (0, 0)), (12, 3, (2, 3))] {
        let (mut a, _) = holding(
            Side::East,
            &[(SMALLFISH, satchel, None)],
            &[(PACK_1, SMALLFISH, bags)],
            |b| b,
        );
        accept(&mut a.realm, lee, SMALLFRY);
        let done = completed(&hand_in(&mut a.realm, lee, SMALLFRY)).expect("the quest completes");
        assert_eq!(done.quest_id, SMALLFRY);
        let blob = saved(&a);
        let in_satchel = blob.contains("[[satchel]]");
        assert_eq!(in_satchel, left.0 > 0, "{blob}");
        if left.0 > 0 {
            assert!(blob.contains(&format!("entry = {SMALLFISH}\ncount = {}\n", left.0)));
        }
        assert_eq!(blob.contains("[[items]]"), left.1 > 0, "{blob}");
    }
}

#[test]
fn the_tenth_catch_completes_a_collect_quest_through_the_satchel() {
    const SMALLFRY: u32 = 96101;
    let lee = hub_guid(LEE_BROWN);
    let (mut a, start) = holding(Side::North, &[(SMALLFISH, 9, None)], &[], |b| b);
    accept(&mut a.realm, lee, SMALLFRY);
    a.realm.handle(RealmCommand::Move {
        position: [start[0], start[1], 0.0],
        orientation: Side::North.facing(),
        motion: Default::default(),
    });
    a.realm.tick(0.0, &a.host);
    drain(&mut a.realm);
    a.hook();
    let events = a.result(true, false, 6.0);
    assert!(
        events.iter().any(|e| matches!(
            e,
            SessionEvent::QuestObjectivesComplete { quest_id: SMALLFRY }
        )),
        "{events:?}"
    );
}

#[test]
fn a_version_4_save_moves_its_bag_journal_items_and_a_version_5_save_keeps_them() {
    let items = [
        (PACK_1, SMALLFISH, 4),
        (PACK_2, SHINY_BAUBLE, 1),
        (25, WORN_SHORTSWORD, 1),
    ];
    let old = |b: String| {
        b.replace("version = 5", "version = 4").replace(
            &format!("entry = {SHINY_BAUBLE}\ncount = 1\n"),
            &format!("entry = {SHINY_BAUBLE}\ncount = 1\ndurability = 14\n"),
        )
    };
    let (a, _) = holding(Side::East, &[], &items, old);
    let blob = saved(&a);
    assert!(
        blob.contains(&format!("[[satchel]]\nentry = {SMALLFISH}\ncount = 4\n")),
        "{blob}"
    );
    assert!(
        blob.contains(&format!(
            "[[satchel]]\nentry = {SHINY_BAUBLE}\ncount = 1\ndurability = 14\n"
        )),
        "{blob}"
    );
    assert!(blob.contains("version = 5"));
    assert_eq!(blob.matches("[[items]]").count(), 1, "{blob}");

    let (b, _) = holding(Side::East, &[], &items, |b| b);
    let blob = saved(&b);
    assert!(!blob.contains("[[satchel]]"), "{blob}");
    assert_eq!(blob.matches("[[items]]").count(), 3, "{blob}");
}

// Fish conditions (#398): a fish whose block, bait or lure does not hold is left out of the roll.

const DUSK_FISH: u32 = 960003;
const LURE_FISH: u32 = 960004;
const BAIT_FISH: u32 = 960005;
const UNGATED_FISH: u32 = 960006;
const WRIGGLING_WORMS_ITEM: u32 = 960101;

#[test]
fn a_dusk_fish_bites_at_dusk_and_never_at_noon() {
    let mut dusk = angler(60, true, true, Side::SouthEast);
    at_game(&mut dusk, 16, 0);
    assert!(bitten(&mut dusk, 3).contains(&DUSK_FISH));

    let mut noon = angler(60, true, true, Side::SouthEast);
    at_game(&mut noon, 12, 0);
    assert_eq!(bitten(&mut noon, 3), [UNGATED_FISH].into());
}

#[test]
fn a_lure_fish_bites_only_with_that_lure_slotted() {
    let items = [(PACK_1, BRIGHT_BAUBLES, 1), (PACK_2, SHINY_BAUBLE, 1)];
    let mut bright = angler_carrying(60, Side::SouthEast, &items);
    bright.slot_lure(PACK_1, BRIGHT_BAUBLES);
    at_game(&mut bright, 12, 0);
    assert!(bitten(&mut bright, 3).contains(&LURE_FISH));

    let mut shiny = angler_carrying(60, Side::SouthEast, &items);
    shiny.slot_lure(PACK_2, SHINY_BAUBLE);
    at_game(&mut shiny, 12, 0);
    assert_eq!(bitten(&mut shiny, 3), [UNGATED_FISH].into());
}

#[test]
fn a_bait_fish_bites_only_with_that_bait_chosen() {
    let items = [(PACK_1, WRIGGLING_WORMS_ITEM, 20)];
    let mut chosen = angler_carrying(60, Side::SouthEast, &items);
    chosen.set_bait(Some(WRIGGLING_WORMS_ITEM));
    at_game(&mut chosen, 12, 0);
    assert!(bitten(&mut chosen, 3).contains(&BAIT_FISH));

    let mut carried = angler_carrying(60, Side::SouthEast, &items);
    at_game(&mut carried, 12, 0);
    assert_eq!(bitten(&mut carried, 3), [UNGATED_FISH].into());
}
