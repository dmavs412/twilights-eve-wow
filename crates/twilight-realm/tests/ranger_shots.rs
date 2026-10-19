//! The Ranger's shot mods (#126): armour ignore, per-skill crit damage, Rapid Fire's haste and
//! refund, Point Blank, and Explosive Payload's fiery Auto Shot. The armour and crit mechanics use
//! a test Hunter-Class Calling whose one shot isolates them; the rest play the shipped Ranger at
//! level 10 with a Cracked Shortbow, near the Crossroads or at the scorpid camp south of it.

mod common;
mod gear_common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::{cast, flat, refusal, stand_at, Mob, CAMP};
use common::*;
use gear_common::save_with;
use twilight_realm::{Realm, RealmCommand};

const RANGED: u8 = 17;
const CRACKED_SHORTBOW: u32 = 2773;

const AUTO_SHOT: u32 = 75;
const SNAPSHOT: u32 = 3044;
const PUNCTURING_SHAFT: u32 = 19434;
const BARBED_ARROW: u32 = 1978;
const RAPID_FIRE: u32 = 3045;
/// The active talent's spell, and its hidden shot's realm id.
const EXPLOSIVE_PAYLOAD: u32 = 19503;
const EXPLOSIVE_SHOT: u32 = 1_003_001;
/// `Talent.dbc` ids.
const PIERCING_FORCE: u32 = 1344;
const POINT_BLANK: u32 = 1624;
const EXPLOSIVE_PAYLOAD_TALENT: u32 = 1353;

const SPELL_SCHOOL_FIRE: u8 = 2;
const SPELL_HIT_TYPE_CRIT: u32 = 0x2;
const SPELL_FAILED_TOO_CLOSE: u8 = 0x76;

/// A Hunter-Class Calling with one instant ranged shot of +1000, big enough that a few points of
/// armour show (`ARMOR_IGNORE` stands for its
/// `armor_ignore`), and a talent that adds 5% a rank to that shot's crits. Added Callings live
/// for the process and the first of a key stays, so each variant takes its own `KEY`.
const TEST_CALLINGS: &str = r#"
[[calling]]
key = "KEY"
name = "Test Ranger"
tier = 0
class = 3
cap = 10
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["leather"]

[[calling.skill]]
key = "shot"
spell_id = 3044
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 35.0]
effects = [{ kind = "weapon_strike", weapon = "ranged", bonus = { base = 1000.0 }ARMOR_IGNORE }]
meta = { defense_type = "ranged" }

[[calling.talent_tab]]
id = 909
name = "Test"
background = "HunterMarksmanship"

[[calling.talent_tab.talent]]
key = "piercing_force"
talent_id = 1344
tier = 1
column = 1
rank_spells = [19426, 19427, 19429, 19430, 19431]
spell_name = "Piercing Force"
effects = [{ kind = "skill_crit_damage_pct", skill = "shot", per_rank = [5, 10, 15, 20, 25] }]
"#;

/// A level 10 `calling` with a bow and `talents`, in the world at `seed`, with `callings`
/// replacing the shipped ones when given: the realm, its guid, the units in view.
fn ranger(
    calling: &str,
    callings: Option<&str>,
    talents: &[(u32, u8)],
    seed: u64,
) -> (Realm, u64, Vec<Mob>) {
    let (guid, mut blob) = save_with(10, 0, &[(RANGED, CRACKED_SHORTBOW, 1)]);
    blob = blob.replace(
        "calling = \"basic_warrior\"",
        &format!("calling = \"{calling}\""),
    );
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        let line = format!("money = 0\ntalents = [{}]\n", rows.join(", "));
        blob = blob.replace("money = 0\n", &line);
    }
    let mut realm = Realm::with_seed(seed);
    if let Some(callings) = callings {
        realm = realm.with_callings(callings);
    }
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mobs = units(&drain(&mut realm));
    (realm, guid, mobs)
}

fn units(events: &[SessionEvent]) -> Vec<Mob> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect()
}

/// What `spell_id` dealt in `events`, and whether it was a crit.
fn dealt(events: &[SessionEvent], spell_id: u32) -> Option<(u32, bool)> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id && l.damage > 0 => {
            Some((l.damage, l.hit_info & SPELL_HIT_TYPE_CRIT != 0))
        }
        _ => None,
    })
}

/// One test shot at 20 yd on `seed`: what it dealt, and whether it crit.
fn test_shot(armor_ignore: &str, talents: &[(u32, u8)], seed: u64) -> Option<(u32, bool)> {
    let key = format!("test_ranger_{}", armor_ignore.len());
    let callings = TEST_CALLINGS
        .replace("ARMOR_IGNORE", armor_ignore)
        .replace("KEY", &key);
    let (mut realm, _, mobs) = ranger(&key, Some(&callings), talents, seed);
    stand_at(&mut realm, &mobs[0], 20.0);
    drain(&mut realm);
    realm.handle(cast(SNAPSHOT, Some(mobs[0].guid)));
    realm.tick(0.05, &FlatGround);
    dealt(&drain(&mut realm), SNAPSHOT)
}

/// The same shot through the same roll: skipping 8 of the mob's 16 armour deals more than skipping
/// none, and
/// no more than skipping all of it.
#[test]
fn armour_ignore_lands_between_full_armour_and_none() {
    let mut between = 0;
    for seed in 0..20 {
        let plain = test_shot("", &[], 0xC0_0000 + seed);
        let some = test_shot(", armor_ignore = { base = 8.0 }", &[], 0xC0_0000 + seed);
        let all = test_shot(
            ", armor_ignore = { base = 100000.0 }",
            &[],
            0xC0_0000 + seed,
        );
        if let (Some((plain, false)), Some((some, false)), Some((all, false))) = (plain, some, all)
        {
            assert!(
                plain <= some && some <= all,
                "seed {seed}: {plain} {some} {all}"
            );
            if plain < some && some < all {
                between += 1;
            }
        }
    }
    assert!(between >= 3, "{between} seeds between");
}

/// Piercing Force 5/5: a crit of the shot deals 250% where it dealt 200%, the same roll.
#[test]
fn piercing_force_makes_the_shots_crits_250_pct() {
    let mut crits = 0;
    for seed in 0..60 {
        let without = test_shot("", &[], 0xC1_0000 + seed);
        let with = test_shot("", &[(PIERCING_FORCE, 5)], 0xC1_0000 + seed);
        if let (Some((base, true)), Some((more, true))) = (without, with) {
            let want = base as f32 * 250.0 / 200.0;
            assert!(
                (more as f32 - want).abs() <= 1.0,
                "seed {seed}: {base} then {more}"
            );
            crits += 1;
        }
    }
    assert!(crits >= 2, "enough crits: {crits}");
}

/// Rapid Fire, at the scorpid camp: Auto Shot every 2.3 s / 1.4, and 1% of base mana back for each that hits while
/// the bar has room.
#[test]
fn rapid_fire_shoots_40_pct_faster_and_refunds_mana_on_hits() {
    // At the scorpid camp, where there are enemies to spare. (At level 1, 1% of base mana rounds
    // to nothing.)
    let (guid, blob) = save_with(10, 0, &[(RANGED, CRACKED_SHORTBOW, 1)]);
    let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"ranger\"");
    let mut realm = Realm::with_seed(0xC2_0001);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    realm.handle(RealmCommand::Move {
        position: [CAMP[0] + 30.0, CAMP[1], CAMP[2]],
        orientation: std::f32::consts::PI,
    });
    let mut seen = Vec::new();
    for _ in 0..20 {
        realm.tick(0.05, &FlatGround);
        seen.extend(drain(&mut realm));
    }
    let camp: Vec<Mob> = units(&seen)
        .into_iter()
        .filter(|m| flat(m.position, CAMP) < 20.0)
        .collect();
    let mut enemies = camp.into_iter();
    // Some mana spent first, on one enemy, so a refund shows.
    let bled = enemies
        .find(|m| {
            stand_at(&mut realm, m, 20.0);
            realm.handle(cast(BARBED_ARROW, Some(m.guid)));
            refusal(&drain(&mut realm), BARBED_ARROW).is_none()
        })
        .expect("an enemy at the camp");
    realm.tick(1.6, &FlatGround);
    // Rapid Fire, then Auto Shot at another.
    realm.handle(cast(RAPID_FIRE, None));
    drain(&mut realm);
    let mob = enemies
        .find(|m| {
            m.guid != bled.guid && {
                stand_at(&mut realm, m, 20.0);
                realm.handle(cast(AUTO_SHOT, Some(m.guid)));
                refusal(&drain(&mut realm), AUTO_SHOT).is_none()
            }
        })
        .expect("another enemy at the camp");
    let mut shots = Vec::new();
    let (mut hits, mut refunds) = (0, 0);
    // The mob's last path: from, to, when it set off, how long it takes.
    let mut way: Option<([f32; 3], [f32; 3], f32, f32)> = None;
    let mut now = 0.0f32;
    while now < 7.0 {
        // 20 yd on from where the mob is along its way, each tick.
        if let Some((from, to, at, secs)) = way {
            let k = if secs > 0.0 {
                ((now - at) / secs).min(1.0)
            } else {
                1.0
            };
            let here: [f32; 3] = std::array::from_fn(|i| from[i] + (to[i] - from[i]) * k);
            let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
            let len = (dx * dx + dy * dy).sqrt();
            if len > 0.01 {
                realm.handle(RealmCommand::Move {
                    position: [
                        here[0] + dx / len * 20.0,
                        here[1] + dy / len * 20.0,
                        here[2],
                    ],
                    orientation: (-dy).atan2(-dx),
                });
            }
        }
        realm.tick(0.05, &FlatGround);
        now += 0.05;
        for e in drain(&mut realm) {
            match &e {
                SessionEvent::MonsterMove {
                    guid,
                    start,
                    path,
                    duration_ms,
                    ..
                } if *guid == mob.guid => {
                    way = Some((
                        *start,
                        *path.last().unwrap(),
                        now,
                        *duration_ms as f32 / 1000.0,
                    ));
                }
                SessionEvent::SpellGo { spell_id, .. } if *spell_id == AUTO_SHOT => shots.push(now),
                SessionEvent::SpellDamageLog(l) if l.spell_id == AUTO_SHOT && l.damage > 0 => {
                    hits += 1
                }
                SessionEvent::SpellEnergizeLog(l) if l.spell_id == RAPID_FIRE => refunds += 1,
                _ => {}
            }
        }
    }
    assert!(shots.len() >= 3, "shots at {shots:?}");
    for pair in shots.windows(2) {
        assert!(
            (pair[1] - pair[0] - 2.3 / 1.4).abs() < 0.1,
            "a shot every 1.64 s: {shots:?}"
        );
    }
    // A refund for each hit, until the bar is full again.
    assert!(
        refunds >= 1 && refunds <= hits,
        "{hits} hits, {refunds} refunds"
    );
}

/// Point Blank: without it a shot at 5 yd is too close; with it the Shaft goes, rank 2 dealing
/// 120/110 of rank 1's through the same roll.
#[test]
fn point_blank_shoots_at_5_yards_with_its_bonus() {
    let (mut realm, _, mobs) = ranger("ranger", None, &[], 0xC3_0000);
    stand_at(&mut realm, &mobs[0], 5.0);
    drain(&mut realm);
    realm.handle(cast(SNAPSHOT, Some(mobs[0].guid)));
    assert_eq!(
        refusal(&drain(&mut realm), SNAPSHOT),
        Some(SPELL_FAILED_TOO_CLOSE)
    );

    let mut compared = 0;
    for seed in 0..20 {
        let mut shafts = Vec::new();
        for rank in [1, 2] {
            let (mut realm, _, mobs) =
                ranger("ranger", None, &[(POINT_BLANK, rank)], 0xC3_0000 + seed);
            stand_at(&mut realm, &mobs[0], 5.0);
            drain(&mut realm);
            realm.handle(cast(PUNCTURING_SHAFT, Some(mobs[0].guid)));
            let mut events = drain(&mut realm);
            assert_eq!(refusal(&events, PUNCTURING_SHAFT), None, "seed {seed}");
            realm.tick(3.1, &FlatGround);
            events.extend(drain(&mut realm));
            shafts.push(dealt(&events, PUNCTURING_SHAFT));
        }
        if let [Some((one, false)), Some((two, false))] = shafts[..] {
            let want = one as f32 * 120.0 / 110.0;
            assert!(
                (two as f32 - want).abs() <= 1.5,
                "seed {seed}: {one} then {two}"
            );
            compared += 1;
        }
    }
    assert!(compared >= 5, "enough plain hits: {compared}");
}

/// Explosive Payload: the next Auto Shot is a Fire blast on the target and the mobs packed
/// within 8 yd of it, in place of the shot. Three of the scorpid camp, pulled with Barbed Arrow,
/// bunch up on the Ranger, who steps 20 yd back and shoots.
#[test]
fn explosive_payload_turns_the_next_auto_shot_into_a_fire_blast() {
    let (mut realm, _, _) = ranger("ranger", None, &[(EXPLOSIVE_PAYLOAD_TALENT, 1)], 0xC4_0001);
    let stand = [CAMP[0] + 20.0, CAMP[1], CAMP[2]];
    realm.handle(RealmCommand::Move {
        position: stand,
        orientation: std::f32::consts::PI,
    });
    let mut seen = Vec::new();
    for _ in 0..20 {
        realm.tick(0.05, &FlatGround);
        seen.extend(drain(&mut realm));
    }
    let camp: Vec<Mob> = units(&seen)
        .into_iter()
        .filter(|m| flat(m.position, CAMP) < 20.0)
        .collect();
    assert!(camp.len() >= 3, "the camp in view");
    // Barbed Arrow on each brings them running; where each heads last.
    let mut heading: Vec<(u64, [f32; 3])> = Vec::new();
    let mut follow = |events: Vec<SessionEvent>| {
        for e in events {
            if let SessionEvent::MonsterMove { guid, path, .. } = e {
                heading.retain(|(g, _)| *g != guid);
                heading.push((guid, *path.last().unwrap()));
            }
        }
    };
    for mob in &camp[..3] {
        realm.handle(RealmCommand::Move {
            position: stand,
            orientation: (mob.position[1] - stand[1]).atan2(mob.position[0] - stand[0]),
        });
        realm.handle(cast(BARBED_ARROW, Some(mob.guid)));
        for _ in 0..32 {
            realm.tick(0.05, &FlatGround);
            follow(drain(&mut realm));
        }
    }
    for _ in 0..40 {
        realm.tick(0.05, &FlatGround);
        follow(drain(&mut realm));
    }
    let target = heading[0];
    let packed = heading
        .iter()
        .filter(|(_, at)| flat(*at, target.1) <= 8.0)
        .count();
    assert!(packed >= 3, "three mobs packed on the Ranger: {heading:?}");

    realm.handle(RealmCommand::Move {
        position: [stand[0] + 20.0, stand[1], stand[2]],
        orientation: std::f32::consts::PI,
    });
    realm.handle(cast(EXPLOSIVE_PAYLOAD, None));
    assert_eq!(refusal(&drain(&mut realm), EXPLOSIVE_PAYLOAD), None);
    realm.handle(cast(AUTO_SHOT, Some(target.0)));
    realm.tick(0.05, &FlatGround);
    let events = drain(&mut realm);
    let blast = events.iter().find_map(|e| match e {
        SessionEvent::SpellGo {
            spell_id,
            hits,
            misses,
            ..
        } if *spell_id == EXPLOSIVE_SHOT => Some(hits.len() + misses.len()),
        _ => None,
    });
    assert!(
        blast.is_some_and(|n| n >= 3),
        "the target and its packmates: {blast:?}"
    );
    assert!(events.iter().all(|e| match e {
        SessionEvent::SpellDamageLog(l) if l.spell_id == EXPLOSIVE_SHOT => {
            l.school == SPELL_SCHOOL_FIRE
        }
        _ => true,
    }));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SessionEvent::SpellGo { spell_id, .. } if *spell_id == AUTO_SHOT)),
        "the blast took the Auto Shot's place"
    );
}

/// Point Blank tells the client: the Twilight op 29 (ADR 0010) at -100 on each ranged skill's
/// family bit, so its range check lets the shot go inside 8 yd. Its tier needs ten points first,
/// so the Ranger enters the world with it learned, the burst that sends every cell.
#[test]
fn point_blank_sends_the_min_range_cell() {
    const SPELLMOD_MIN_RANGE: u8 = 29;
    let cells = |talents: &[(u32, u8)]| {
        let (guid, mut blob) = save_with(60, 0, &[(RANGED, CRACKED_SHORTBOW, 1)]);
        blob = blob.replace("calling = \"basic_warrior\"", "calling = \"ranger\"");
        if let [(t, r)] = talents {
            blob = blob.replace(
                "money = 0
",
                &format!(
                    "money = 0
talents = [[{t}, {r}]]
"
                ),
            );
        }
        let mut realm = Realm::with_seed(0xC3_1000);
        realm.import_character(&blob).unwrap();
        realm.handle(RealmCommand::EnterWorld { guid });
        realm.tick(0.0, &FlatGround);
        let mut bits: Vec<(u8, i32)> = drain(&mut realm)
            .iter()
            .filter_map(|e| match *e {
                SessionEvent::SpellModifier {
                    flat: false,
                    mask_bit,
                    op: SPELLMOD_MIN_RANGE,
                    value,
                } => Some((mask_bit, value)),
                _ => None,
            })
            .collect();
        bits.sort_unstable();
        bits
    };
    let learned = cells(&[(POINT_BLANK, 1)]);
    assert_eq!(learned.len(), 4, "a cell per ranged skill: {learned:?}");
    assert!(learned.iter().all(|&(_, v)| v == -100), "{learned:?}");
    assert!(cells(&[]).iter().all(|&(_, v)| v == 0));
}
