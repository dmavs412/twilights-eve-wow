//! The Ranger's shot mods (#126): armour ignore, and Rapid Fire's haste and refund. The armour
//! mechanic uses a test Hunter-Class Calling whose one shot isolates it; Rapid Fire plays the
//! shipped Ranger at level 10 with a Cracked Shortbow at the scorpid camp south of the Crossroads.

mod common;
mod gear_common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::{assert_on_time, cast, flat, refusal, stand_at, Mob, CAMP};
use common::*;
use gear_common::save_shooting;
use twilight_realm::{Realm, RealmCommand};

const RANGED: u8 = 17;
const CRACKED_SHORTBOW: u32 = 2773;

static AUTO_SHOT: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("ranger", "auto_shot"));
/// The test Ranger's shot, as `TEST_CALLINGS` numbers it.
const TEST_SNAPSHOT: u32 = 3044;
static BARBED_ARROW: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("ranger", "barbed_arrow"));
static RAPID_FIRE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("ranger", "rapid_fire"));

const SPELL_HIT_TYPE_CRIT: u32 = 0x2;

/// A Hunter-Class Calling with one instant ranged shot of +1000, big enough that a few points of
/// armour show (`ARMOR_IGNORE` stands for its `armor_ignore`). Added Callings live for the
/// process and the first of a key stays, so each variant takes its own `KEY`.
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
"#;

/// A level 10 `calling` with a bow, in the world at `seed`, with `callings` replacing the shipped
/// ones: the realm and the units in view.
fn ranger(calling: &str, callings: &str, seed: u64) -> (Realm, Vec<Mob>) {
    let (guid, blob) = save_shooting(10, &[(RANGED, CRACKED_SHORTBOW, 1)]);
    let blob = blob.replace(
        "calling = \"basic_warrior\"",
        &format!("calling = \"{calling}\""),
    );
    let mut realm = Realm::with_seed(seed).with_callings(callings);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mobs = units(&drain(&mut realm));
    (realm, mobs)
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
fn test_shot(armor_ignore: &str, seed: u64) -> Option<(u32, bool)> {
    let key = format!("test_ranger_{}", armor_ignore.len());
    let callings = TEST_CALLINGS
        .replace("ARMOR_IGNORE", armor_ignore)
        .replace("KEY", &key);
    let (mut realm, mobs) = ranger(&key, &callings, seed);
    stand_at(&mut realm, &mobs[0], 20.0);
    drain(&mut realm);
    realm.handle(cast(TEST_SNAPSHOT, Some(mobs[0].guid)));
    realm.tick(0.05, &FlatGround);
    dealt(&drain(&mut realm), TEST_SNAPSHOT)
}

/// The same shot through the same roll: skipping 8 of the mob's 16 armour deals more than skipping
/// none, and no more than skipping all of it.
#[test]
fn armour_ignore_lands_between_full_armour_and_none() {
    let mut between = 0;
    for seed in 0..20 {
        let plain = test_shot("", 0xC0_0000 + seed);
        let some = test_shot(", armor_ignore = { base = 8.0 }", 0xC0_0000 + seed);
        let all = test_shot(", armor_ignore = { base = 100000.0 }", 0xC0_0000 + seed);
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

/// Rapid Fire, at the scorpid camp: Auto Shot every 2.3 s / 1.4, and 1% of base mana back for each that hits while
/// the bar has room.
#[test]
fn rapid_fire_shoots_40_pct_faster_and_refunds_mana_on_hits() {
    // At the scorpid camp, where there are enemies to spare. (At level 1, 1% of base mana rounds
    // to nothing.)
    let (guid, blob) = save_shooting(10, &[(RANGED, CRACKED_SHORTBOW, 1)]);
    let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"ranger\"");
    let mut realm = Realm::with_seed(0xC2_0001);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
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
            realm.handle(cast(*BARBED_ARROW, Some(m.guid)));
            refusal(&drain(&mut realm), *BARBED_ARROW).is_none()
        })
        .expect("an enemy at the camp");
    realm.tick(1.6, &FlatGround);
    // Rapid Fire, then Auto Shot at another.
    realm.handle(cast(*RAPID_FIRE, None));
    drain(&mut realm);
    let mob = enemies
        .find(|m| {
            m.guid != bled.guid && {
                stand_at(&mut realm, m, 20.0);
                realm.handle(cast(*AUTO_SHOT, Some(m.guid)));
                refusal(&drain(&mut realm), *AUTO_SHOT).is_none()
            }
        })
        .expect("another enemy at the camp");
    let mut shots = Vec::new();
    let (mut hits, mut refunds) = (0, 0);
    // The mob's last path: from, to, when it set off, how long it takes.
    let mut way: Option<([f32; 3], [f32; 3], f32, f32)> = None;
    let mut now = 0.0f32;
    // Where the Ranger stands and when they stepped: a step holds the shot (#155).
    let (mut me, mut steps): (Option<[f32; 3]>, Vec<f32>) = (None, Vec::new());
    while now < 9.0 {
        // Standing while the mob runs in, then 30 yd on from where it is along its way.
        if let Some((from, to, at, secs)) = way {
            let k = if secs > 0.0 {
                ((now - at) / secs).min(1.0)
            } else {
                1.0
            };
            let here: [f32; 3] = std::array::from_fn(|i| from[i] + (to[i] - from[i]) * k);
            let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
            let len = (dx * dx + dy * dy).sqrt();
            if len > 0.01 && me.is_none_or(|me| flat(me, here) < 12.0) {
                let at = [
                    here[0] + dx / len * 30.0,
                    here[1] + dy / len * 30.0,
                    here[2],
                ];
                realm.handle(RealmCommand::Move {
                    motion: Default::default(),
                    position: at,
                    orientation: (-dy).atan2(-dx),
                });
                me = Some(at);
                steps.push(now);
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
                SessionEvent::SpellGo { spell_id, .. } if *spell_id == *AUTO_SHOT => {
                    shots.push(now)
                }
                SessionEvent::SpellDamageLog(l) if l.spell_id == *AUTO_SHOT && l.damage > 0 => {
                    hits += 1
                }
                SessionEvent::SpellEnergizeLog(l) if l.spell_id == *RAPID_FIRE => refunds += 1,
                _ => {}
            }
        }
    }
    // The arrows' DPS (#157) kills the mob by the third shot.
    assert!(shots.len() >= 2, "shots at {shots:?}");
    assert_on_time(&shots, &steps, 2.3 / 1.4);
    // A refund for each hit, until the bar is full again.
    assert!(
        refunds >= 1 && refunds <= hits,
        "{hits} hits, {refunds} refunds"
    );
}
