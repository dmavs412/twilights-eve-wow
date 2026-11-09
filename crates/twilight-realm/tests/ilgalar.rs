//! The Tower of Ilgalar (#643): the feed valves, the Blackrock Overseer and Morganth, laid out in
//! `content/ilgalar.toml`.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

const ILGALAR: &str = include_str!("../content/ilgalar.toml");

/// The Phase Tester, which the tower no longer holds.
const PHASE_TESTER: i64 = 97100;
const SERVANT: u32 = 97241;
const MORGANTH: u32 = 97244;
const MORGANTH_AT: [f32; 3] = [-9272.0, -3310.0, 150.04];
const BY_MORGANTH: [f32; 3] = [-9274.0, -3310.0, 150.04];

/// The three feed valves, each a lever, and a spot in reach of valve A.
const VALVES: [i64; 3] = [983041, 983042, 983043];
const VALVE_A: u64 = 983041;
const BY_VALVE_A: [f32; 3] = [-9278.0, -3311.5, 115.67];
const VALVE_USED: &str = "The valve groans shut. Something above stops answering.";
/// Inside the door, in group 1's corridor; and the open hills well away from the tower.
const IN_THE_DOORWAY: [f32; 3] = [-9281.6, -3321.7, 115.67];
const FAR_OUTSIDE: [f32; 3] = [-9150.0, -3420.0, 100.0];

/// The Anchor, its display, and the Steppes spec's two wave points on the top floor.
const ANCHOR: i64 = 794101;
const ANCHOR_DISPLAY: i64 = 1667;
const WAVE_POINTS: [[f32; 3]; 2] = [[-9300.0, -3300.0, 150.09], [-9296.0, -3318.0, 150.09]];

/// The first low guid of a creature a timed call has brought in.
const TIMED_LIVE_BASE: u64 = 0xC0_0000;

fn file() -> toml::Table {
    ILGALAR.parse().unwrap()
}

fn rows<'a>(file: &'a toml::Table, kind: &str) -> &'a [toml::Value] {
    file.get(kind)
        .and_then(|v| v.as_array())
        .map_or(&[][..], |a| a.as_slice())
}

fn point(v: &toml::Value) -> [f32; 3] {
    let a = v.as_array().unwrap();
    let f = |x: &toml::Value| x.as_float().or(x.as_integer().map(|i| i as f64)).unwrap() as f32;
    [f(&a[0]), f(&a[1]), f(&a[2])]
}

fn int(v: &toml::Value, key: &str) -> Option<i64> {
    v.get(key).and_then(|x| x.as_integer())
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f32>().sqrt()
}

/// A box of a footprint, in the building's frame: `[lo, hi]` on x, y and z.
type Box3 = [[f32; 2]; 3];

fn boxes(file: &toml::Table) -> Vec<Box3> {
    let pair = |v: &toml::Value| {
        let a = v.as_array().unwrap();
        [
            a[0].as_float().unwrap() as f32,
            a[1].as_float().unwrap() as f32,
        ]
    };
    rows(file, "footprint")
        .iter()
        .map(|b| [pair(&b["x"]), pair(&b["y"]), pair(&b["z"])])
        .collect()
}

/// Whether the world point `p` lies in a footprint box, turned into the building's frame by the
/// file's `origin` and `yaw` as `phased::Dungeon::local` does.
fn inside(file: &toml::Table, p: [f32; 3]) -> bool {
    let [ox, oy, oz] = point(&file["origin"]);
    let (sin, cos) = (file["yaw"].as_float().unwrap() as f32)
        .to_radians()
        .sin_cos();
    let (dx, dy) = (p[0] - ox, p[1] - oy);
    let local = [dx * cos + dy * sin, dy * cos - dx * sin, p[2] - oz];
    boxes(file)
        .iter()
        .any(|b| (0..3).all(|i| (b[i][0]..=b[i][1]).contains(&local[i])))
}

/// Every placed thing of the file: its label and its point, each timed call's point included.
fn placed(file: &toml::Table) -> Vec<(String, [f32; 3])> {
    let mut out = Vec::new();
    for c in rows(file, "creature") {
        let guid = int(c, "guid").unwrap();
        out.push((format!("creature {guid}"), point(&c["position"])));
        for call in rows(c.as_table().unwrap(), "every") {
            out.push((format!("creature {guid}'s call"), point(&call["position"])));
        }
    }
    for o in rows(file, "gameobject") {
        let guid = int(o, "guid").unwrap();
        out.push((format!("GameObject {guid}"), point(&o["position"])));
    }
    out
}

fn anchor_at(file: &toml::Table) -> [f32; 3] {
    let anchors: Vec<&toml::Value> = rows(file, "gameobject")
        .iter()
        .filter(|o| int(o, "guid") == Some(ANCHOR))
        .collect();
    assert_eq!(anchors.len(), 1, "one Anchor");
    assert_eq!(int(anchors[0], "display"), Some(ANCHOR_DISPLAY));
    point(&anchors[0]["position"])
}

#[test]
fn every_creature_and_gameobject_stands_in_the_tower_s_footprint() {
    let file = file();
    let creatures = rows(&file, "creature");
    assert_eq!(creatures.len(), 14, "{} creatures", creatures.len());
    assert!(
        creatures
            .iter()
            .all(|c| int(c, "entry") != Some(PHASE_TESTER)),
        "the Phase Tester is still in the tower"
    );
    for (label, at) in placed(&file) {
        assert!(inside(&file, at), "{label} at {at:?}");
    }
}

#[test]
fn morganth_is_the_one_boss_bar_and_each_valve_stops_one_of_his_calls() {
    let file = file();
    let bosses: Vec<&toml::Value> = rows(&file, "creature")
        .iter()
        .filter(|c| {
            mob_template(int(c, "entry").unwrap() as u32)
                .get("boss")
                .and_then(|b| b.as_bool())
                == Some(true)
        })
        .collect();
    assert_eq!(bosses.len(), 1, "one Boss Bar creature");
    assert_eq!(int(bosses[0], "entry"), Some(MORGANTH.into()));

    let calls = rows(bosses[0].as_table().unwrap(), "every");
    for valve in VALVES {
        let stopping = calls
            .iter()
            .filter(|c| int(c, "until_used") == Some(valve))
            .count();
        assert_eq!(stopping, 1, "valve {valve}");
    }
    for call in calls {
        assert_eq!(int(call, "entry"), Some(SERVANT.into()));
    }
    let levers: Vec<i64> = rows(&file, "gameobject")
        .iter()
        .filter(|o| o.get("use").and_then(|u| u.as_str()) == Some("lever"))
        .map(|o| int(o, "guid").unwrap())
        .collect();
    assert_eq!(levers, VALVES);
}

#[test]
fn the_three_cover_pylons_stand_within_10_yd_of_the_anchor() {
    let file = file();
    let anchor = anchor_at(&file);
    let cover = rows(&file, "cover");
    assert_eq!(cover.len(), 3);
    for c in cover {
        let at = point(&c["position"]);
        assert!(distance(at, anchor) <= 10.0, "cover at {at:?}");
        let shown = rows(&file, "gameobject")
            .iter()
            .any(|o| point(&o["position"]) == at && int(o, "display") == Some(3080));
        assert!(shown, "no pylon shows the cover at {at:?}");
    }
}

#[test]
fn nothing_stands_on_the_wave_points_or_against_the_anchor() {
    let file = file();
    let anchor = anchor_at(&file);
    for (label, at) in placed(&file) {
        for wave in WAVE_POINTS {
            assert!(
                flat_distance(at, wave) >= 3.0,
                "{label} at {at:?} is within 3 yd of {wave:?}"
            );
        }
        if label != format!("GameObject {ANCHOR}") {
            assert!(
                distance(at, anchor) >= 4.0,
                "{label} at {at:?} is within 4 yd of the Anchor"
            );
        }
    }
}

#[test]
fn the_quest_text_no_longer_puts_a_ward_at_the_tower_s_foot() {
    let quests = include_str!("../content/quests_elwynn.toml");
    assert!(!quests.contains("foot of the Ilgalar tower"));
}

/// `n` level-60 warriors in Elwynn, ready to walk into the tower.
fn party(n: usize) -> Party {
    let mut p = Party::edited(
        Realm::with_seed(1),
        &vec![60; n],
        START_ELWYNN,
        Box::new(NoGround),
        |_, blob| blob,
    );
    p.run(1.0);
    p
}

/// Pulls Morganth and lets him fight for 21 s, past his first calls; the Servants they brought.
fn morganth_s_calls(p: &mut Party) -> Vec<[f32; 3]> {
    p.go_facing(0, BY_MORGANTH, MORGANTH_AT);
    p.run(0.5);
    let (morganth, _) = p.unit(0, MORGANTH, MORGANTH_AT);
    let from = p.seen(0).len();
    p.cmd(0, RealmCommand::AttackSwing { guid: morganth });
    p.run(0.2);
    p.cmd(0, RealmCommand::AttackStop);
    p.run(21.0);
    unit_creates(&p.seen(0)[from..])
        .into_iter()
        .filter(|u| u.1 == SERVANT && u.0 & 0xFF_FFFF >= TIMED_LIVE_BASE)
        .map(|u| u.2)
        .collect()
}

#[test]
fn with_every_valve_open_morganth_calls_a_servant_for_each() {
    let mut p = party(1);
    let calls = morganth_s_calls(&mut p);
    assert_eq!(calls.len(), 3, "{calls:?}");
}

#[test]
fn with_valve_a_pulled_morganth_calls_servants_only_for_valves_b_and_c() {
    // The first warrior waits in the doorway, holding the Run; the second turns the valve and
    // leaves the tower with its guards, out of reach of the Tuning Pulse and the Head Count, so
    // the first fights Morganth alone.
    let mut p = party(2);
    p.go(0, IN_THE_DOORWAY, 0.0);
    p.run(0.5);
    p.go(1, BY_VALVE_A, 0.0);
    p.run(1.0);
    let from = p.seen(1).len();
    p.cmd(
        1,
        RealmCommand::UseGameObject {
            guid: 0xF110 << 48 | VALVE_A,
        },
    );
    p.run(0.5);
    assert!(
        p.seen(1)[from..]
            .iter()
            .any(|e| matches!(e, SessionEvent::Notification { text } if text == VALVE_USED)),
        "valve A did not turn"
    );
    p.go(1, FAR_OUTSIDE, 0.0);
    p.run(1.0);

    let calls = morganth_s_calls(&mut p);
    assert_eq!(calls.len(), 2, "{calls:?}");
}
