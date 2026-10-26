//! Herbalism's realm side (#438, ADR 0024): Brother Ansel teaches it, each character who knows it
//! has Herb Patches of their own, and Find Herbs toggles the minimap's leaves.

mod common;

use benilla_protocol::{EntityKind, SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const PATCH_ENTRY: u32 = 981_001;
const BROTHER_ANSEL: u32 = 190_072;
const HERB_GATHERING: u32 = 2366;
const FIND_HERBS: u32 = 2383;
const SICKLE: u32 = 961_101;
const RESPAWN_SECS: f32 = 600.0;

/// A test zone around the Northshire start, with five spots within 40 yards of it and three kept
/// live. Its one legendary is Moonpetal, as a zone needs.
const TEST_HERBS: &str = r#"
[[zone]]
key = "test_yard"
zone = 990101
live = 3
points = [
  { x = -8930.0, y = -132.0 },
  { x = -8970.0, y = -132.0 },
  { x = -8950.0, y = -110.0 },
  { x = -8950.0, y = -155.0 },
  { x = -8935.0, y = -150.0 },
]

[[plant]]
zone = "test_yard"
item = 961009
rarity = "legendary"
level = 14
bloom = "night"
timed = true
weight = 0
use = "alchemy"
"#;

const POINTS: [[f32; 2]; 5] = [
    [-8930.0, -132.0],
    [-8970.0, -132.0],
    [-8950.0, -110.0],
    [-8950.0, -155.0],
    [-8935.0, -150.0],
];

/// A party of `knows.len()` warriors at the Northshire start, each knowing Herbalism or not.
fn party(knows: &[bool]) -> Party {
    let realm = Realm::with_seed(7).with_herbs(TEST_HERBS);
    let knows = knows.to_vec();
    Party::edited(
        realm,
        &vec![1; knows.len()],
        START_ELWYNN,
        Box::new(FlatGround),
        move |i, blob| {
            if knows[i] {
                blob.replacen(
                    "angler_level = 1\n",
                    "angler_level = 1\nknows_herbalism = true\n",
                    1,
                )
            } else {
                blob
            }
        },
    )
}

/// Each patch made in `events`: guid and position.
fn patches(events: &[SessionEvent]) -> Vec<(u64, [f32; 3])> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::GameObject,
                fields,
                position,
                ..
            } if fields.object_entry() == Some(PATCH_ENTRY) && on_a_test_point(*position) => {
                Some((*guid, *position))
            }
            _ => None,
        })
        .collect()
}

/// The shipped zones' patches may stand in view too; the scripts count only the test zone's.
fn on_a_test_point(at: [f32; 3]) -> bool {
    POINTS
        .iter()
        .any(|p| flat_distance(at, [p[0], p[1], 0.0]) < 1.0)
}

/// The spot of `POINTS` that `at` stands on.
fn spot_of(at: [f32; 3]) -> usize {
    POINTS
        .iter()
        .position(|p| flat_distance(at, [p[0], p[1], 0.0]) < 1.0)
        .unwrap_or_else(|| panic!("{at:?} is no test point"))
}

/// Ticks the party `secs` in 5 s steps; player `i`'s events.
fn skip(p: &mut Party, i: usize, secs: f32) -> Vec<SessionEvent> {
    let mut got = Vec::new();
    let mut t = 0.0;
    while t < secs {
        p.realm.tick(5.0, p.host.as_ref());
        for (s, e) in p.realm.drain_session_events() {
            if s == p.sessions[i] {
                got.push(e);
            }
        }
        t += 5.0;
    }
    got
}

fn herbalist_events(events: &[SessionEvent]) -> Vec<(bool, u8, u32)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Herbalist {
                knows, level, xp, ..
            } => Some((*knows, *level, *xp)),
            _ => None,
        })
        .collect()
}

fn tracking(events: &[SessionEvent]) -> Vec<(u32, bool)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::HerbTracking { patch_entry, on } => Some((*patch_entry, *on)),
            _ => None,
        })
        .collect()
}

#[test]
fn brother_ansel_teaches_herbalism_and_gives_the_sickle() {
    let mut p = party(&[false]);
    assert_eq!(
        herbalist_events(p.seen(0)),
        [(false, 1, 0)],
        "login sends Herbalist"
    );
    let ansel = hub_guid(BROTHER_ANSEL);
    let [x, y, _] = hub_home(ansel);
    p.go(0, [x + 1.5, y, 0.0], std::f32::consts::PI);
    p.cmd(0, RealmCommand::GossipHello { guid: ansel });
    let teach = p
        .seen(0)
        .iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => options
                .iter()
                .find(|o| o.message == "Teach me Herbalism.")
                .map(|o| o.index),
            _ => None,
        })
        .expect("Brother Ansel offers the lesson");
    let before = p.seen(0).len();
    p.cmd(
        0,
        RealmCommand::GossipSelectOption {
            guid: ansel,
            option: teach,
        },
    );
    let taught = p.seen(0)[before..].to_vec();
    for spell_id in [HERB_GATHERING, FIND_HERBS] {
        assert!(
            taught
                .iter()
                .any(|e| matches!(e, SessionEvent::SpellLearned { spell_id: s } if *s == spell_id)),
            "{spell_id} is learned"
        );
    }
    assert!(
        taught
            .iter()
            .any(|e| format!("{e:?}").contains(&SICKLE.to_string())),
        "the sickle comes into the bags"
    );
    assert_eq!(herbalist_events(&taught), [(true, 1, 0)]);
}

#[test]
fn entering_the_world_stands_the_live_patches_at_distinct_points() {
    let p = party(&[true]);
    let made = patches(p.seen(0));
    assert_eq!(made.len(), 3, "{made:?}");
    let mut spots: Vec<usize> = made.iter().map(|&(_, at)| spot_of(at)).collect();
    spots.sort_unstable();
    spots.dedup();
    assert_eq!(spots.len(), 3, "three distinct points");
}

#[test]
fn a_second_character_sees_none_of_the_first_ones_patches() {
    let p = party(&[true, false]);
    assert_eq!(patches(p.seen(0)).len(), 3);
    assert!(patches(p.seen(1)).is_empty());
}

#[test]
fn a_finished_patch_comes_back_after_the_respawn_delay_at_a_free_point() {
    let mut p = party(&[true]);
    let made = patches(p.seen(0));
    let (gone, gone_at) = made[0];
    let standing: Vec<usize> = made[1..].iter().map(|&(_, at)| spot_of(at)).collect();
    p.realm.finish_herb_patch(gone);
    let early = skip(&mut p, 0, RESPAWN_SECS - 50.0);
    assert!(patches(&early).is_empty(), "nothing before the delay");
    let late = skip(&mut p, 0, 60.0);
    let back = patches(&late);
    assert_eq!(back.len(), 1, "one replacement: {back:?}");
    let spot = spot_of(back[0].1);
    assert!(!standing.contains(&spot), "not on a standing patch");
    let _ = gone_at;
}

#[test]
fn find_herbs_toggles_tracking() {
    let mut p = party(&[true]);
    assert_eq!(
        tracking(p.seen(0)),
        [(PATCH_ENTRY, false)],
        "login sends it off"
    );
    let cast = RealmCommand::CastSpell {
        spell_id: FIND_HERBS,
        target: None,
    };
    let before = p.seen(0).len();
    p.cmd(0, cast.clone());
    assert_eq!(tracking(&p.seen(0)[before..]), [(PATCH_ENTRY, true)]);
    let before = p.seen(0).len();
    p.cmd(0, cast);
    assert_eq!(tracking(&p.seen(0)[before..]), [(PATCH_ENTRY, false)]);
}

#[test]
fn the_herbalist_fields_survive_a_save_round_trip() {
    let mut p = party(&[true]);
    let guid = p.guids[0];
    let blob = p.realm.export_character(guid).unwrap();
    assert!(blob.contains("knows_herbalism = true"), "{blob}");
    let edited = blob.replacen(
        "herbalist_level = 1\n",
        "herbalist_level = 3\nherbalist_xp = 10\nherb_tracking = true\n\
         herb_picks = [[961001, 4]]\nherb_perks = [\"nightowl\"]\nperk_resets = 2\n",
        1,
    );
    let mut realm = Realm::with_seed(9).with_herbs(TEST_HERBS);
    realm.import_character(&edited).unwrap();
    let again = realm.export_character(guid).unwrap();
    for line in [
        "herbalist_level = 3",
        "herbalist_xp = 10",
        "herb_tracking = true",
        "herb_picks = [[961001, 4]]",
        "herb_perks = [\"nightowl\"]",
        "perk_resets = 2",
    ] {
        assert!(again.contains(line), "{line} in {again}");
    }
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    assert_eq!(herbalist_events(&events), [(true, 3, 10)]);
    assert_eq!(tracking(&events), [(PATCH_ENTRY, true)]);
    let _ = &mut p;
}
