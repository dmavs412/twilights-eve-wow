//! Fishing (#327, ADR 0018): Lee Brown teaches it; a cast with a pole into water floats a bobber,
//! a fish bites and the client's Reeling Bar reports the catch, which pays Angler XP.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::*;
use quest_common::{accept, completed, hand_in};
use twilight_realm::{Realm, RealmCommand};

const FISHING: u32 = 7620;
const FISHING_POLE: u32 = 6256;
const SMALLFISH: u32 = 6291;
const CATFISH: u32 = 6308;
const LEE_BROWN: u32 = 1651;
const ELWYNN: u32 = 12;
const ELWYNN_FISH: [u32; 4] = [6291, 6289, 6292, 6294];
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
fish = [{ item = 6291, weight = 100.0, level = 1 }]
treasure = []
copper = [0, 0]

[[pool]]
key = "test_level_15"
zone = 990002
fish = [{ item = 6308, weight = 100.0, level = 15 }]
treasure = []
copper = [0, 0]

[[pool]]
key = "test_level_11"
zone = 990003
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
    format!(
        "{TEST_FISH}
[[pool]]
key = \"test_legends\"
zone = 990004
fish = [{{ item = 6291, weight = 100.0, level = 1 }}]
treasure = []
copper = [0, 0]

[[pool]]
key = \"test_legend_rumour\"
zone = 990005
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
hours = [21, 3]

[[legend]]
item = 6310
name = \"Test Bream\"
pool = \"test_legends\"
level = 1
style = \"dart\"
difficulty = 50
chance = 100
minutes = [0, 10]

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
/// Fishing Pole in the main hand or nothing carried. Its guid, the save and where it starts.
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
            "\n[[items]]\nslot = {MAIN_HAND}\nentry = {FISHING_POLE}\ncount = 1\n"
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
    let host = Pond {
        centre: pond_centre(start, side),
        radius: 8.0,
        area: Some(ELWYNN),
    };
    let mut realm = Realm::with_seed(7)
        .with_fish(&test_fish(start))
        .with_rumours(TEST_LEGEND_RUMOUR);
    realm.import_character(&blob).unwrap();
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
}

#[test]
fn each_cast_that_lands_takes_one_bait_and_the_last_clears_the_slot() {
    let mut a = angler_carrying(1, Side::East, &[(PACK_1, WRIGGLING_WORMS, 3)]);
    let chose = a.set_bait(Some(WRIGGLING_WORMS));
    assert_eq!(journal(&chose).map(|j| j.0), Some(Some(WRIGGLING_WORMS)));

    for n in 1..=3 {
        let events = a.cast();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SessionEvent::ChannelStart { .. })),
            "cast {n} landed"
        );
        let cleared = journal(&events).map(|j| j.0);
        if n < 3 {
            assert_eq!(cleared, None, "cast {n} still has bait");
        } else {
            assert_eq!(cleared, Some(None), "the last bait clears the slot");
        }
    }
    let saved = a.realm.export_character(a.guid).unwrap();
    assert!(
        !saved.contains("960101"),
        "the worms are all gone:\n{saved}"
    );
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
fn a_shiny_bauble_counts_as_one_angler_level_more_but_not_for_xp() {
    let mut a = angler_carrying(1, Side::East, &[(PACK_1, SHINY_BAUBLE, 1)]);
    let used = a.use_item(PACK_1);
    assert!(
        used.iter().any(|e| matches!(
            e,
            SessionEvent::AuraDuration {
                slot: 30,
                remaining_ms: 600_000
            }
        )),
        "the lure's ten-minute aura: {used:?}"
    );
    let (_, bite) = a.hook();
    assert!((zone_height(&bite) - 0.1735).abs() < 1e-6, "{bite:?}");

    // An L11 fish bites at Angler 1 with the lure (11 < 2 + 10).
    let mut south = angler_carrying(1, Side::South, &[(PACK_1, SHINY_BAUBLE, 1)]);
    south.use_item(PACK_1);
    let (_, bite) = south.hook();
    assert!(matches!(
        bite,
        SessionEvent::FishingBite {
            fish: MUD_SNAPPER,
            ..
        }
    ));
    let landed = south.result(true, false, 6.0);
    // Angler 1's own XP: 5 + 2 x 11, the lure adding nothing.
    assert_eq!(angler_event(&landed), Some((1, 27)));

    // Without it the same water has nothing that bites.
    let mut bare = angler(1, true, true, Side::South);
    assert!(bare.cast().iter().any(|e| matches!(
        e,
        SessionEvent::Notification { text } if text == "Nothing is biting"
    )));
}

#[test]
fn bright_baubles_widen_the_zone_after_the_cap() {
    let mut a = angler_carrying(1, Side::East, &[(PACK_1, BRIGHT_BAUBLES, 1)]);
    a.use_item(PACK_1);
    let (_, bite) = a.hook();
    assert!((zone_height(&bite) - 0.17 * 1.15).abs() < 1e-6, "{bite:?}");
}

#[test]
fn nightcrawlers_and_worms_bring_the_bite_within_five_and_a_half_seconds() {
    let mut a = angler_carrying(
        1,
        Side::East,
        &[(PACK_1, NIGHTCRAWLERS, 1), (PACK_2, WRIGGLING_WORMS, 1)],
    );
    a.use_item(PACK_1);
    a.set_bait(Some(WRIGGLING_WORMS));
    let mut events = a.cast();
    // 12 x (1 - 0.30 - 0.25) = 5.4 s at the longest.
    events.extend(a.wait_for_bite(5.45));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SessionEvent::FishingBite { .. })),
        "no bite within 5.4 s"
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
    assert_eq!(
        journal(&again),
        None,
        "a plain catch after a perfect one changes nothing"
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

// Legendary Fish (#331). The test pool's Legendary Fish all bite at 100 percent when their
// criteria hold, in this order: Gar (game hours 21 to 3), Bream (minutes 0 to 10 of every hour),
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
fn the_bream_bites_in_the_first_minutes_of_an_hour_only() {
    let mut early = angler(1, true, true, Side::NorthWest);
    at_game(&mut early, 12, 3);
    let (_, bite) = early.hook();
    assert_eq!(legend_of(&bite), Some(BREAM));

    let mut late = angler(1, true, true, Side::NorthWest);
    at_game(&mut late, 12, 30);
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
fn a_legendary_above_the_effective_angler_level_waits_for_a_lure() {
    let mut plain = angler(1, true, true, Side::NorthWest);
    at_game(&mut plain, 12, 30);
    let (_, bite) = plain.hook();
    assert_eq!(legend_of(&bite), None, "{bite:?}");

    // The Aquadynamic Fish Attractor counts as four levels more: 1 + 4 reaches the Gate's 5.
    let mut lured = angler_carrying(1, Side::NorthWest, &[(PACK_1, AQUADYNAMIC, 1)]);
    lured.use_item(PACK_1);
    at_game(&mut lured, 12, 30);
    let (_, bite) = lured.hook();
    assert_eq!(legend_of(&bite), Some(GATE));
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
    assert_eq!(legends_of(&second), None, "no second roll: {second:?}");
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
