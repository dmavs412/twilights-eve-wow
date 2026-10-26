//! Herbalism's realm side (#438, ADR 0024): Brother Ansel teaches it, each character who knows it
//! has Herb Patches of their own, and Find Herbs toggles the minimap's leaves.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::{EntityKind, SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;
use quest_common::{accept, completed, hand_in};
use twilight_realm::{Audience, HerbAction, Realm, RealmCommand};

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
thorns = 0
points = [
  { x = -8930.0, y = -132.0 },
  { x = -8970.0, y = -132.0 },
  { x = -8950.0, y = -110.0 },
  { x = -8950.0, y = -155.0 },
  { x = -8935.0, y = -150.0 },
]

[[plant]]
zone = "test_yard"
item = 961001
rarity = "common"
level = 1
weight = 1000
use = "cooking"

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

const PRAIRIE_WOLF: u32 = 2958;
const DANDELION: u32 = 961_001;

/// A party of one who knows Herbalism, with `picks` pulls of Dandelion Greens behind them.
fn picked_party(picks: u32) -> Party {
    let realm = Realm::with_seed(7).with_herbs(TEST_HERBS);
    Party::edited(
        realm,
        &[1],
        START_ELWYNN,
        Box::new(FlatGround),
        move |_, blob| {
            blob.replacen(
                "angler_level = 1\n",
                &format!(
                    "angler_level = 1\nknows_herbalism = true\nherb_picks = [[{DANDELION}, {picks}]]\n"
                ),
                1,
            )
        },
    )
}

type Grid = (
    u8,
    u8,
    Vec<benilla_protocol::HerbTile>,
    Vec<benilla_protocol::HerbKnown>,
);

/// Each `HerbGrid` in `events`: size, pulls left, tiles and knowledge.
fn grids(events: &[SessionEvent]) -> Vec<Grid> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::HerbGrid {
                size,
                pulls_left,
                tiles,
                known,
                ..
            } => Some((*size, *pulls_left, tiles.clone(), known.clone())),
            _ => None,
        })
        .collect()
}

/// The reason of each `HerbGridClosed` in `events`.
fn closed(events: &[SessionEvent]) -> Vec<u8> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::HerbGridClosed { reason, .. } => Some(*reason),
            _ => None,
        })
        .collect()
}

/// Stands player 0 beside their first patch and clicks it; the patch's guid and position.
fn click_a_patch(p: &mut Party) -> (u64, [f32; 3]) {
    let (guid, at) = patches(p.seen(0))[0];
    p.go(0, [at[0] + 1.0, at[1], at[2]], 0.0);
    p.run(0.5);
    p.cmd(0, RealmCommand::UseGameObject { guid });
    (guid, at)
}

/// Pulls tiles in order until one is a plant; the events of the pulls.
fn pull_a_plant(p: &mut Party) -> Vec<SessionEvent> {
    let before = p.seen(0).len();
    for tile in 0..25u8 {
        p.cmd(0, RealmCommand::Herb(HerbAction::Pull(tile)));
        let got = &p.seen(0)[before..];
        let last = grids(got).pop().expect("a grid after each pull");
        if last.2[usize::from(tile)].state == 1 {
            return got.to_vec();
        }
    }
    panic!("no plant in 25 tiles");
}

#[test]
fn clicking_a_patch_opens_a_grid_of_hidden_tiles() {
    let mut p = party(&[true]);
    let before = p.seen(0).len();
    click_a_patch(&mut p);
    let all = grids(&p.seen(0)[before..]);
    assert_eq!(all.len(), 1);
    let (size, pulls, tiles, _) = &all[0];
    assert_eq!((*size, *pulls, tiles.len()), (5, 6, 25));
    assert!(tiles.iter().all(|t| t.state == 0 && t.stake == -1));
}

#[test]
fn pulling_a_plant_gives_the_item_and_xp_and_spends_a_pull() {
    let mut p = party(&[true]);
    click_a_patch(&mut p);
    let got = pull_a_plant(&mut p);
    let last = grids(&got).pop().unwrap();
    assert!(last.1 < 6, "pulls left {}", last.1);
    let tile = last.2.iter().find(|t| t.state == 1).unwrap();
    assert!(tile.item != 0 && tile.count >= 1);
    assert!(
        got.iter()
            .any(|e| !matches!(e, SessionEvent::HerbGrid { .. })
                && format!("{e:?}").contains(&tile.item.to_string())),
        "the plant reaches the bags"
    );
    let xp = herbalist_events(&got);
    assert!(xp.last().is_some_and(|&(_, _, xp)| xp > 0), "{xp:?}");
}

#[test]
fn six_pulls_close_the_grid_and_finish_the_patch() {
    let mut p = party(&[true]);
    let (patch, _) = click_a_patch(&mut p);
    let before = p.seen(0).len();
    for tile in 0..6u8 {
        p.cmd(0, RealmCommand::Herb(HerbAction::Pull(tile)));
    }
    assert_eq!(closed(&p.seen(0)[before..]), [0]);
    let later = skip(&mut p, 0, 5.0);
    assert!(
        later.iter().any(|e| match e {
            SessionEvent::ObjectsRemoved(gone) => gone.contains(&patch),
            SessionEvent::ObjectDestroyed(g) => *g == patch,
            _ => false,
        }),
        "the patch is removed"
    );
}

#[test]
fn leaving_closes_the_grid() {
    let mut p = party(&[true]);
    click_a_patch(&mut p);
    let before = p.seen(0).len();
    p.cmd(0, RealmCommand::Herb(HerbAction::Leave));
    assert_eq!(closed(&p.seen(0)[before..]), [2]);
}

#[test]
fn damage_closes_the_grid() {
    let mut p = party(&[true]);
    let (_, at) = click_a_patch(&mut p);
    let map = p.realm.player_positions()[0].0;
    let audience = Audience::of(&[p.guids[0]]);
    p.realm
        .spawn_for(
            p.host.as_ref(),
            PRAIRIE_WOLF,
            5,
            [at[0] + 3.0, at[1], at[2]],
            map,
            audience,
        )
        .expect("a wolf");
    let before = p.seen(0).len();
    p.run(15.0);
    assert_eq!(closed(&p.seen(0)[before..]), [3]);
}

#[test]
fn walking_away_closes_the_grid() {
    let mut p = party(&[true]);
    let (_, at) = click_a_patch(&mut p);
    let before = p.seen(0).len();
    p.go(0, [at[0] + 21.0, at[1], at[2]], 0.0);
    p.run(1.0);
    assert_eq!(closed(&p.seen(0)[before..]), [4]);
}

#[test]
fn five_picks_list_the_plant_in_the_journal_and_ten_show_its_rows() {
    for (picks, rows) in [(0, None), (5, Some(false)), (10, Some(true))] {
        let mut p = picked_party(picks);
        let before = p.seen(0).len();
        click_a_patch(&mut p);
        let (_, _, _, known) = grids(&p.seen(0)[before..]).remove(0);
        let entry = known.iter().find(|k| k.item == DANDELION);
        match rows {
            None => assert!(entry.is_none(), "{known:?}"),
            Some(shown) => {
                let k = entry.unwrap_or_else(|| panic!("{picks} picks list it: {known:?}"));
                assert!(k.present);
                assert_eq!(!k.rows.is_empty(), shown, "{k:?}");
            }
        }
    }
}

const PROBE: u32 = 961_102;
const SNIP: u32 = 961_103;
const THYME_TEA: u32 = 961_201;
const CHARM: u32 = 961_206;

/// A second test zone whose every tile is a thorn, with its own spots near the Northshire start.
const THORN_HERBS: &str = r#"
[[zone]]
key = "thorn_yard"
zone = 990102
live = 3
thorns = 25
points = [
  { x = -8960.0, y = -120.0 },
  { x = -8940.0, y = -120.0 },
  { x = -8960.0, y = -145.0 },
  { x = -8945.0, y = -142.0 },
  { x = -8955.0, y = -132.0 },
]

[[plant]]
zone = "thorn_yard"
item = 961001
rarity = "common"
level = 1
weight = 1000
use = "cooking"

[[plant]]
zone = "thorn_yard"
item = 961009
rarity = "legendary"
level = 14
bloom = "night"
timed = true
weight = 0
use = "alchemy"
"#;

const THORN_POINTS: [[f32; 2]; 5] = [
    [-8960.0, -120.0],
    [-8940.0, -120.0],
    [-8960.0, -145.0],
    [-8945.0, -142.0],
    [-8955.0, -132.0],
];

/// Stands player 0 beside a patch of the thorn zone and clicks it.
fn click_a_thorn_patch(p: &mut Party) {
    let (guid, at) = p
        .seen(0)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::GameObject,
                fields,
                position,
                ..
            } if fields.object_entry() == Some(PATCH_ENTRY)
                && THORN_POINTS
                    .iter()
                    .any(|t| flat_distance(*position, [t[0], t[1], 0.0]) < 1.0) =>
            {
                Some((*guid, *position))
            }
            _ => None,
        })
        .expect("a thorn patch");
    p.go(0, [at[0] + 1.0, at[1], at[2]], 0.0);
    p.run(0.5);
    p.cmd(0, RealmCommand::UseGameObject { guid });
}

/// A party of one who knows Herbalism, with `plain` extra top-level save keys and `satchel`
/// `(entry, count)` rows in the Satchel. With `thorny`, the thorn zone is added too.
fn kitted_party(plain: &str, satchel: &[(u32, u32)], thorny: bool) -> Party {
    let mut realm = Realm::with_seed(7).with_herbs(TEST_HERBS);
    if thorny {
        realm = realm.with_herbs(THORN_HERBS);
    }
    let plain = plain.to_owned();
    let satchel = satchel.to_vec();
    Party::edited(
        realm,
        &[1],
        START_ELWYNN,
        Box::new(FlatGround),
        move |_, blob| {
            let blob = if plain.contains("herbalist_level") {
                blob.replacen("herbalist_level = 1\n", "", 1)
            } else {
                blob
            };
            let mut blob = blob.replacen(
                "angler_level = 1\n",
                &format!("angler_level = 1\nknows_herbalism = true\n{plain}\n"),
                1,
            );
            for (entry, count) in &satchel {
                blob.push_str(&format!(
                    "\n[[satchel]]\nentry = {entry}\ncount = {count}\n"
                ));
            }
            blob
        },
    )
}

/// The last `HerbGrid`'s pulls left, tiles, tool, skill cost, consumable and readiness.
fn last_grid(events: &[SessionEvent]) -> (u8, Vec<benilla_protocol::HerbTile>) {
    events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::HerbGrid {
                pulls_left, tiles, ..
            } => Some((*pulls_left, tiles.clone())),
            _ => None,
        })
        .expect("a grid")
}

fn journals(events: &[SessionEvent]) -> Vec<&SessionEvent> {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::HerbalismJournal { .. }))
        .collect()
}

/// The count the last `HerbalismJournal` shows for Satchel entry `item`.
fn satchel_count(events: &[SessionEvent], item: u32) -> u32 {
    events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::HerbalismJournal { satchel, .. } => Some(
                satchel
                    .iter()
                    .filter(|r| r.item == item)
                    .map(|r| r.count)
                    .sum(),
            ),
            _ => None,
        })
        .expect("a Herbalism Journal")
}

fn perk_slots(events: &[SessionEvent]) -> Vec<Vec<String>> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Herbalist { perks, .. } => Some(perks.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn sweep_across_from_the_first_tile_reveals_three_for_two_pulls() {
    let mut p = kitted_party("herb_tool = 961101", &[(SICKLE, 1)], false);
    click_a_patch(&mut p);
    let before = p.seen(0).len();
    p.cmd(
        0,
        RealmCommand::Herb(HerbAction::Sweep {
            tile: 0,
            across: true,
        }),
    );
    let (pulls, tiles) = last_grid(&p.seen(0)[before..]);
    assert_eq!(pulls, 4);
    let pulled: Vec<usize> = (0..25).filter(|&i| tiles[i].state != 0).collect();
    assert_eq!(pulled, [0, 1, 2]);
}

#[test]
fn sweep_from_the_last_column_shifts_to_fit() {
    let mut p = kitted_party("herb_tool = 961101", &[(SICKLE, 1)], false);
    click_a_patch(&mut p);
    let before = p.seen(0).len();
    p.cmd(
        0,
        RealmCommand::Herb(HerbAction::Sweep {
            tile: 4,
            across: true,
        }),
    );
    let (_, tiles) = last_grid(&p.seen(0)[before..]);
    let pulled: Vec<usize> = (0..25).filter(|&i| tiles[i].state != 0).collect();
    assert_eq!(pulled, [2, 3, 4]);
}

#[test]
fn a_skill_of_another_tool_is_refused() {
    let mut p = kitted_party("herb_tool = 961101", &[(SICKLE, 1)], false);
    click_a_patch(&mut p);
    let before = p.seen(0).len();
    p.cmd(0, RealmCommand::Herb(HerbAction::Probe(12)));
    p.cmd(0, RealmCommand::Herb(HerbAction::Snip(12)));
    assert!(grids(&p.seen(0)[before..]).is_empty(), "nothing changed");
}

#[test]
fn probe_on_the_centre_marks_nine_tiles_for_one_pull() {
    let mut p = kitted_party("herb_tool = 961102", &[(PROBE, 1)], false);
    click_a_patch(&mut p);
    let before = p.seen(0).len();
    p.cmd(0, RealmCommand::Herb(HerbAction::Probe(12)));
    let (pulls, tiles) = last_grid(&p.seen(0)[before..]);
    assert_eq!(pulls, 5);
    let marked: Vec<usize> = (0..25).filter(|&i| tiles[i].known != 0).collect();
    assert_eq!(marked, [6, 7, 8, 11, 12, 13, 16, 17, 18]);
}

#[test]
fn snip_on_a_thorn_costs_two_pulls_and_nothing_more() {
    let mut p = kitted_party("herb_tool = 961103", &[(SNIP, 1)], true);
    click_a_thorn_patch(&mut p);
    let before = p.seen(0).len();
    p.cmd(0, RealmCommand::Herb(HerbAction::Snip(0)));
    let (pulls, tiles) = last_grid(&p.seen(0)[before..]);
    assert_eq!((pulls, tiles[0].state), (4, 3));
}

#[test]
fn thyme_tea_adds_a_pull_and_leaves_one_fewer_in_the_satchel() {
    let mut p = kitted_party("herb_consumable = 961201", &[(THYME_TEA, 2)], false);
    let before = p.seen(0).len();
    click_a_patch(&mut p);
    let got = &p.seen(0)[before..];
    assert_eq!(last_grid(got).0, 7);
    assert_eq!(satchel_count(got, THYME_TEA), 1);
}

#[test]
fn an_armed_charm_is_spent_by_the_next_pull() {
    let mut p = kitted_party("herb_consumable = 961206", &[(CHARM, 1)], false);
    click_a_patch(&mut p);
    p.cmd(0, RealmCommand::Herb(HerbAction::ArmCharm));
    let armed = p.seen(0).len();
    p.cmd(0, RealmCommand::Herb(HerbAction::Pull(0)));
    let (pulls, tiles) = last_grid(&p.seen(0)[armed..]);
    match tiles[0].state {
        1 => assert_eq!((tiles[0].count, pulls), (2, 5), "a plant doubles"),
        _ => assert_eq!(pulls, 4, "a weed costs one more"),
    }
    let ready = p.seen(0)[armed..].iter().rev().find_map(|e| match e {
        SessionEvent::HerbGrid {
            consumable_ready,
            charm_armed,
            ..
        } => Some((*consumable_ready, *charm_armed)),
        _ => None,
    });
    assert_eq!(ready, Some((false, false)));
}

#[test]
fn night_owl_gives_seven_pulls_at_night() {
    let mut p = kitted_party("herb_perks = [\"nightowl\"]", &[], false);
    p.realm.set_wall_clock(23 * 60 * 5 * 1000);
    let before = p.seen(0).len();
    click_a_patch(&mut p);
    assert_eq!(last_grid(&p.seen(0)[before..]).0, 7);
}

#[test]
fn thornwise_shows_every_thorn_and_gives_five_pulls() {
    let mut p = kitted_party("herb_perks = [\"thornwise\"]", &[], true);
    let before = p.seen(0).len();
    click_a_thorn_patch(&mut p);
    let got = &p.seen(0)[before..];
    let (pulls, tiles) = last_grid(got);
    assert_eq!(pulls, 5);
    assert!(tiles.iter().all(|t| t.known == 3));
}

#[test]
fn a_perk_slot_opens_at_level_ten_and_fills_once() {
    let mut p = kitted_party("herbalist_level = 1", &[], false);
    let before = p.seen(0).len();
    p.cmd(
        0,
        RealmCommand::PickPerk {
            slot: 0,
            perk: "gloves".into(),
        },
    );
    assert!(
        perk_slots(&p.seen(0)[before..]).is_empty(),
        "slot 0 is locked"
    );

    let mut p = kitted_party("herbalist_level = 10", &[], false);
    let before = p.seen(0).len();
    p.cmd(
        0,
        RealmCommand::PickPerk {
            slot: 0,
            perk: "gloves".into(),
        },
    );
    assert_eq!(
        perk_slots(&p.seen(0)[before..]),
        [vec!["gloves".to_owned()]]
    );
    let again = p.seen(0).len();
    p.cmd(
        0,
        RealmCommand::PickPerk {
            slot: 0,
            perk: "keeneye".into(),
        },
    );
    assert!(
        perk_slots(&p.seen(0)[again..]).is_empty(),
        "a filled slot stays"
    );
}

// The tool quests and the perk reset (#441): Brother Ansel's and Hedda Thornwick's.

const HEDDA: u32 = 190_073;
const TROWEL: u32 = 961_102;
const SHEARS: u32 = 961_103;
const ROOT_CELLAR: u32 = 96_201;
const CAREFUL_CUT: u32 = 96_202;
const EARTHROOT: u32 = 2449;
const PEACEBLOOM: u32 = 2447;
const SILVERLEAF: u32 = 765;
const GOLD: u32 = 10_000;

/// A lone Herbalist in the world at the Northshire start, driven through the default session.
/// `plain` is extra top-level save keys (it may set `herbalist_level` and `money`).
fn lone_herbalist(plain: &str, satchel: &[(u32, u32)]) -> (Realm, u64) {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Lee", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    assert_eq!(create_result(&mut maker), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker.export_character(guid).unwrap();
    if plain.contains("herbalist_level") {
        blob = blob.replacen("herbalist_level = 1\n", "", 1);
    }
    if plain.contains("money") {
        blob = blob.replacen("money = 0\n", "", 1);
    }
    let mut blob = blob.replacen(
        "angler_level = 1\n",
        &format!("angler_level = 1\nknows_herbalism = true\n{plain}\n"),
        1,
    );
    for (entry, count) in satchel {
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

/// How many of `item` the saved character holds in the Satchel.
fn saved_satchel(realm: &Realm, guid: u64, item: u32) -> i64 {
    let blob = realm.export_character(guid).unwrap();
    let table: toml::Table = blob.parse().unwrap();
    table
        .get("satchel")
        .and_then(|s| s.as_array())
        .into_iter()
        .flatten()
        .filter(|r| r["entry"].as_integer() == Some(i64::from(item)))
        .map(|r| r["count"].as_integer().unwrap())
        .sum()
}

fn offered_by(realm: &mut Realm, entry: u32) -> Vec<u32> {
    quest_common::on_offer(realm, hub_guid(entry))
}

#[test]
fn the_root_cellar_quest_waits_for_herbalist_level_ten_and_pays_a_trowel() {
    let herbs = [(EARTHROOT, 20), (PEACEBLOOM, 20), (SILVERLEAF, 10)];
    let (mut low, _) = lone_herbalist("herbalist_level = 9", &herbs);
    assert!(!offered_by(&mut low, BROTHER_ANSEL).contains(&ROOT_CELLAR));
    let (mut realm, guid) = lone_herbalist("herbalist_level = 10", &herbs);
    assert!(offered_by(&mut realm, BROTHER_ANSEL).contains(&ROOT_CELLAR));
    let ansel = hub_guid(BROTHER_ANSEL);
    accept(&mut realm, ansel, ROOT_CELLAR);
    let done = completed(&hand_in(&mut realm, ansel, ROOT_CELLAR)).expect("the quest completes");
    assert_eq!(done.quest_id, ROOT_CELLAR);
    assert_eq!(saved_satchel(&realm, guid, EARTHROOT), 0);
    assert_eq!(saved_satchel(&realm, guid, PEACEBLOOM), 0);
    assert_eq!(saved_satchel(&realm, guid, SILVERLEAF), 0);
    assert_eq!(saved_satchel(&realm, guid, TROWEL), 1);
}

#[test]
fn the_careful_cut_quest_is_offered_only_at_herbalist_level_twenty() {
    for (level, offered) in [(10, false), (19, false), (20, true)] {
        let (mut realm, _) = lone_herbalist(&format!("herbalist_level = {level}"), &[]);
        assert_eq!(
            offered_by(&mut realm, HEDDA).contains(&CAREFUL_CUT),
            offered,
            "Herbalist Level {level}"
        );
    }
    let _ = SHEARS;
}

/// The lines of the menu `npc` opens, and the index of the one that begins with `prefix`.
fn menu_line(realm: &mut Realm, npc: u64, prefix: &str) -> Option<(u32, String)> {
    quest_common::talk(realm, npc);
    realm.handle(RealmCommand::GossipHello { guid: npc });
    drain(realm).into_iter().find_map(|e| match e {
        SessionEvent::GossipMenu { options, .. } => options
            .into_iter()
            .find(|o| o.message.starts_with(prefix))
            .map(|o| (o.index, o.message)),
        _ => None,
    })
}

fn money_of(realm: &Realm, guid: u64) -> i64 {
    let blob = realm.export_character(guid).unwrap();
    let table: toml::Table = blob.parse().unwrap();
    table["money"].as_integer().unwrap()
}

#[test]
fn hedda_forgets_the_perks_for_money_that_doubles_each_time() {
    let hedda = hub_guid(HEDDA);
    let perks = "herbalist_level = 10\nherb_perks = [\"nightowl\"]";
    // One perk and 1 gold: the reset clears it and takes the gold.
    let (mut realm, guid) = lone_herbalist(&format!("{perks}\nmoney = {GOLD}"), &[]);
    let (option, line) = menu_line(&mut realm, hedda, "Forget my perks").expect("the line");
    assert_eq!(line, "Forget my perks (1 gold).");
    realm.handle(RealmCommand::GossipSelectOption {
        guid: hedda,
        option,
    });
    let events = drain(&mut realm);
    assert_eq!(perk_slots(&events), [vec![String::new()]]);
    assert_eq!(money_of(&realm, guid), 0);
    let blob = realm.export_character(guid).unwrap();
    assert!(blob.contains("perk_resets = 1"), "{blob}");
    assert!(!blob.contains("nightowl"), "{blob}");
    // No perk, no line.
    assert!(menu_line(&mut realm, hedda, "Forget my perks").is_none());

    // The second reset asks 2 gold; short of it, nothing changes.
    let second = format!("{perks}\nperk_resets = 1\nmoney = {}", 2 * GOLD - 1);
    let (mut realm, guid) = lone_herbalist(&second, &[]);
    let (option, line) = menu_line(&mut realm, hedda, "Forget my perks").expect("the line");
    assert_eq!(line, "Forget my perks (2 gold).");
    realm.handle(RealmCommand::GossipSelectOption {
        guid: hedda,
        option,
    });
    let events = drain(&mut realm);
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::VendorBuyFailed { .. })));
    assert!(perk_slots(&events).is_empty());
    assert_eq!(money_of(&realm, guid), i64::from(2 * GOLD - 1));
    let blob = realm.export_character(guid).unwrap();
    assert!(blob.contains("nightowl"), "{blob}");
}

#[test]
fn brother_ansel_does_not_sell_the_perk_reset() {
    let (mut realm, _) = lone_herbalist("herb_perks = [\"nightowl\"]", &[]);
    assert!(menu_line(&mut realm, hub_guid(BROTHER_ANSEL), "Forget my perks").is_none());
}

#[test]
fn receiving_a_herb_sends_the_herbalism_journal_not_the_fishing_one() {
    let mut p = kitted_party("", &[], false);
    click_a_patch(&mut p);
    let got = pull_a_plant(&mut p);
    assert!(!journals(&got).is_empty(), "the Herbalism Journal follows");
    assert!(
        !got.iter()
            .any(|e| matches!(e, SessionEvent::FishingJournal { .. })),
        "no Fishing Journal"
    );
}
