//! Stonewatch Keep (#647): the quartermaster, the envoy, Gath'Ilzogg and Singe, laid out in
//! `content/stonewatch_keep.toml`.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

const STONEWATCH: &str = include_str!("../content/stonewatch_keep.toml");

/// The Phase Tester, which the keep no longer holds.
const PHASE_TESTER: i64 = 97100;
/// `HIGHGUID_UNIT`, in a guid's top 16 bits.
const HIGHGUID_UNIT: u64 = 0xF130;

/// The two bosses of the top storey, each enraged once the other is dead, and a spot beside them.
const GATH: u64 = 974316;
const GATH_ENTRY: u64 = 97287;
const SINGE: u64 = 974317;
const SINGE_ENTRY: u64 = 97288;
const GATH_AT: [f32; 3] = [-9383.0, -3082.0, 158.07];
const BY_GATH: [f32; 3] = [-9381.5, -3080.5, 158.07];
/// Enraged, `enraged_partner` in `content/creature_spells.toml`, and its warning.
const ENRAGE: u32 = 1900093;
const ENRAGE_WARNING: &str = "A roar of rage shakes the roof!";
/// `UNIT_FIELD_AURA`, the first of a unit's 48 aura slots.
const UNIT_FIELD_AURA: u16 = 47;

/// The Dragonspawn Envoy, the Broodlings he calls, the brood cage's lever and a spot in reach of
/// it beside him.
const ENVOY: u32 = 97285;
const ENVOY_AT: [f32; 3] = [-9366.0, -3062.0, 152.55];
const BROODLING: u32 = 97286;
const LEVER: u64 = 983061;
const BY_LEVER: [f32; 3] = [-9365.0, -3061.0, 153.26];
const WEST_OF_ENVOY: [f32; 3] = [-9368.0, -3061.0, 153.46];
/// The two Broodlings placed beside him, and the warriors that fight him.
const BROODLINGS: [u64; 2] = [974314, 974315];
const PARTY: usize = 4;
const LEVER_USED: &str = "The cage slams shut on the brood.";

/// The first low guid of a creature a timed call has brought in.
const TIMED_LIVE_BASE: u64 = 0xC0_0000;

fn file() -> toml::Table {
    STONEWATCH.parse().unwrap()
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
    let yaw = file["yaw"]
        .as_float()
        .or(file["yaw"].as_integer().map(|i| i as f64))
        .unwrap() as f32;
    let (sin, cos) = yaw.to_radians().sin_cos();
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

fn is_boss(c: &toml::Value) -> bool {
    mob_template(int(c, "entry").unwrap() as u32)
        .get("boss")
        .and_then(|b| b.as_bool())
        == Some(true)
}

#[test]
fn every_creature_and_gameobject_stands_in_the_keep_s_footprint() {
    let file = file();
    let creatures = rows(&file, "creature");
    assert_eq!(creatures.len(), 17, "{} creatures", creatures.len());
    assert!(
        creatures
            .iter()
            .all(|c| int(c, "entry") != Some(PHASE_TESTER)),
        "the Phase Tester is still in the keep"
    );
    for (label, at) in placed(&file) {
        assert!(inside(&file, at), "{label} at {at:?}");
    }
}

#[test]
fn gath_ilzogg_and_singe_are_the_two_boss_bars_one_camp_each_enraged_by_the_other_s_death() {
    let file = file();
    let bosses: Vec<&toml::Value> = rows(&file, "creature")
        .iter()
        .filter(|c| is_boss(c))
        .collect();
    assert_eq!(bosses.len(), 2, "two Boss Bar creatures");
    let entries: Vec<i64> = bosses.iter().map(|c| int(c, "entry").unwrap()).collect();
    assert_eq!(entries, [GATH_ENTRY as i64, SINGE_ENTRY as i64]);
    assert_eq!(int(bosses[0], "camp"), int(bosses[1], "camp"), "one camp");
    assert!(int(bosses[0], "camp").is_some(), "no camp");
    for (me, other) in [(bosses[0], bosses[1]), (bosses[1], bosses[0])] {
        let link = &me["on_partner_death"];
        assert_eq!(int(link, "guid"), int(other, "guid"));
        assert_eq!(link["spell"].as_str(), Some("enraged_partner"));
    }
}

/// Singe's Scale (#648), the one proof of the dragons the player carries out.
const SINGE_S_SCALE: u32 = 980701;

#[test]
fn singe_always_drops_her_scale_a_known_item() {
    let file = file();
    let singe = rows(&file, "creature")
        .iter()
        .find(|c| int(c, "guid") == Some(SINGE as i64))
        .expect("Singe's row");
    let drop: Vec<i64> = singe["drop"]
        .as_array()
        .expect("Singe has a drop list")
        .iter()
        .map(|v| v.as_integer().unwrap())
        .collect();
    assert_eq!(drop, [SINGE_S_SCALE as i64]);

    let mut realm = realm();
    realm.handle(RealmCommand::ItemQuery {
        entry: SINGE_S_SCALE,
    });
    let name = drain(&mut realm).into_iter().find_map(|e| match e {
        SessionEvent::ItemTemplate { info, .. } => info.map(|i| i.name),
        _ => None,
    });
    assert_eq!(name.as_deref(), Some("Singe's Scale"));
}

#[test]
fn the_envoy_s_call_stops_at_the_brood_cage_lever() {
    let file = file();
    let envoys: Vec<&toml::Value> = rows(&file, "creature")
        .iter()
        .filter(|c| int(c, "entry") == Some(ENVOY.into()))
        .collect();
    assert_eq!(envoys.len(), 1, "one Envoy");
    let calls = rows(envoys[0].as_table().unwrap(), "every");
    assert_eq!(calls.len(), 1, "one call");
    assert_eq!(int(&calls[0], "until_used"), Some(LEVER as i64));
    assert_eq!(int(&calls[0], "entry"), Some(BROODLING.into()));
    let levers: Vec<i64> = rows(&file, "gameobject")
        .iter()
        .filter(|o| o.get("use").and_then(|u| u.as_str()) == Some("lever"))
        .map(|o| int(o, "guid").unwrap())
        .collect();
    assert_eq!(levers, [LEVER as i64]);
}

/// `n` level-60 warriors in Elwynn, ready to walk into the keep.
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

fn unit(entry: u64, low: u64) -> u64 {
    HIGHGUID_UNIT << 48 | entry << 24 | low
}

/// Whether player `i`'s client last saw `guid` wearing Enraged in one of its aura slots.
fn enraged(p: &Party, i: usize, guid: u64) -> bool {
    let mut slots = [0u32; 48];
    for e in p.seen(i) {
        let fields = match e {
            SessionEvent::ObjectCreate {
                guid: g, fields, ..
            } if *g == guid => {
                // A create sends every slot afresh.
                slots = [0; 48];
                fields
            }
            SessionEvent::ObjectValues { guid: g, fields } if *g == guid => fields,
            _ => continue,
        };
        for (f, v) in fields.raw_fields() {
            if (UNIT_FIELD_AURA..UNIT_FIELD_AURA + 48).contains(&f) {
                slots[usize::from(f - UNIT_FIELD_AURA)] = v;
            }
        }
    }
    slots.contains(&ENRAGE)
}

fn warnings(p: &Party, i: usize, text: &str) -> usize {
    p.seen(i)
        .iter()
        .filter(|e| matches!(e, SessionEvent::Chat(m) if m.text == text))
        .count()
}

/// Player 0 pulls the top storey's camp with one swing at `target`, then stops swinging, so both
/// bosses fight on; then `dies` is killed.
fn pull_and_kill(p: &mut Party, target: u64, dies: u64) {
    p.go_facing(0, BY_GATH, GATH_AT);
    p.run(0.5);
    p.cmd(0, RealmCommand::AttackSwing { guid: target });
    p.run(0.2);
    p.cmd(0, RealmCommand::AttackStop);
    p.run(1.0);
    p.realm.kill_creature(dies);
    p.run(0.2);
}

#[test]
fn killing_singe_while_gath_ilzogg_fights_enrages_him() {
    let mut p = party(1);
    let (gath, singe) = (unit(GATH_ENTRY, GATH), unit(SINGE_ENTRY, SINGE));
    p.go_facing(0, BY_GATH, GATH_AT);
    p.run(0.5);
    assert!(!enraged(&p, 0, gath), "enraged before the fight");
    pull_and_kill(&mut p, gath, singe);
    assert!(enraged(&p, 0, gath), "Gath'Ilzogg is not enraged");
    assert_eq!(warnings(&p, 0, ENRAGE_WARNING), 1, "the warning");
}

#[test]
fn killing_gath_ilzogg_while_singe_fights_enrages_her() {
    let mut p = party(1);
    let (gath, singe) = (unit(GATH_ENTRY, GATH), unit(SINGE_ENTRY, SINGE));
    pull_and_kill(&mut p, singe, gath);
    assert!(enraged(&p, 0, singe), "Singe is not enraged");
}

/// Four warriors stand beside the Envoy, the first by the lever, with his two Broodlings cleared,
/// and pull him with one swing each; the first pulls the lever when `pull` says so, and he fights
/// 21 s, past his first call. His Fear sends one of them running out of the keep, and a bare
/// level-60 warrior lasts about 15 s against him, so four keep him fighting. The Broodlings the
/// call brought, as any of them saw them.
fn envoy_s_calls(pull: bool) -> Vec<u64> {
    let mut p = party(PARTY);
    p.go_facing(0, BY_LEVER, ENVOY_AT);
    for i in 1..PARTY {
        p.go_facing(i, WEST_OF_ENVOY, ENVOY_AT);
    }
    p.run(0.5);
    for low in BROODLINGS {
        p.realm.kill_creature(unit(BROODLING.into(), low));
    }
    let (envoy, _) = p.unit(0, ENVOY, ENVOY_AT);
    let from: Vec<usize> = (0..PARTY).map(|i| p.seen(i).len()).collect();
    for i in 0..PARTY {
        p.cmd(i, RealmCommand::AttackSwing { guid: envoy });
    }
    p.run(0.2);
    for i in 0..PARTY {
        p.cmd(i, RealmCommand::AttackStop);
    }
    if pull {
        p.cmd(
            0,
            RealmCommand::UseGameObject {
                guid: 0xF110 << 48 | LEVER,
            },
        );
        p.run(0.5);
        assert!(
            p.seen(0)[from[0]..]
                .iter()
                .any(|e| matches!(e, SessionEvent::Notification { text } if text == LEVER_USED)),
            "the lever did not move"
        );
    }
    p.run(21.0);
    let mut brought: Vec<u64> = (0..PARTY)
        .flat_map(|i| unit_creates(&p.seen(i)[from[i]..]))
        .filter(|u| u.1 == BROODLING && u.0 & 0xFF_FFFF >= TIMED_LIVE_BASE)
        .map(|u| u.0)
        .collect();
    brought.sort_unstable();
    brought.dedup();
    brought
}

#[test]
fn with_the_lever_up_the_envoy_calls_two_broodlings() {
    let calls = envoy_s_calls(false);
    assert_eq!(calls.len(), 2, "{calls:?}");
}

#[test]
fn a_run_with_the_lever_pulled_brings_no_broodlings_from_the_envoy_s_call() {
    let calls = envoy_s_calls(true);
    assert!(calls.is_empty(), "{calls:?}");
}
