//! Fishing (#327, ADR 0018): Lee Brown teaches it; a cast with a pole into water floats a bobber,
//! a fish bites and the client's Reeling Bar reports the catch, which pays Angler XP.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::*;
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
/// Raw Brilliant Smallfish to the north and a lone level-15 catfish to the west.
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
}

impl Side {
    fn facing(self) -> f32 {
        match self {
            Side::East => 0.0,
            Side::North => std::f32::consts::FRAC_PI_2,
            Side::West => std::f32::consts::PI,
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
    format!(
        "{TEST_FISH}
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
"
    )
}

/// A new Elwynn character's save, edited: Angler Level `level`, Fishing known or not, and a
/// Fishing Pole in the main hand or nothing carried. Its guid, the save and where it starts.
fn save(level: u8, knows: bool, pole: bool) -> (u64, String, [f32; 3]) {
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
    let (guid, blob, start) = save(level, knows, pole);
    let host = Pond {
        centre: pond_centre(start, side),
        radius: 8.0,
        area: Some(ELWYNN),
    };
    let mut realm = Realm::with_seed(7).with_fish(&test_fish(start));
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
