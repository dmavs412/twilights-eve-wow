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

// Ranger's Talent Rows (#507): a shipped Ranger of 24, its row set, 25 yd west of the sturdy
// scorpid south of the Crossroads.

use common::initiate::{cast_bar, cast_through};
use common::ranger::{face_from, gone_off, leap, mana, ranger_choice, ranger_scene, shots};
use common::vanguard::STURDY;

/// Twin Arrows: a landed shot goes off again at once about 30% of the time, the repeat costing
/// nothing, and a repeat never repeats. Plain Snapshot never goes off twice.
#[test]
fn twin_arrows_fires_a_second_snapshot_on_a_winning_roll_and_never_a_third() {
    let twin = ranger_choice("snapshot", "twin_arrows");
    let (mut twice, mut casts, mut cost) = (0, 0, None);
    for seed in 0..40 {
        let mut s = ranger_scene(0x507_2000 + seed, &[("snapshot", twin)], STURDY);
        let before = mana(&s.view, s.me);
        let events = s.cast_at_mob(twin);
        let spent = before - mana(&s.view, s.me);
        assert_eq!(*cost.get_or_insert(spent), spent, "seed {seed}: one cost");
        let gone = gone_off(&events, twin);
        assert!((1..=2).contains(&gone), "seed {seed}: {gone} shots");
        if gone == 2 {
            assert!(
                !shots(&events, twin).is_empty(),
                "seed {seed}: only a landed shot repeats"
            );
        }
        casts += 1;
        twice += usize::from(gone == 2);
    }
    assert!((4..=20).contains(&twice), "{twice} of {casts} repeated");

    let snapshot = spell("ranger", "snapshot");
    for seed in 0..10 {
        let mut s = ranger_scene(0x507_2000 + seed, &[], STURDY);
        let events = s.cast_at_mob(snapshot);
        assert_eq!(gone_off(&events, snapshot), 1, "seed {seed}");
    }
}

/// What `events` set Vault's cooldown to, in ms.
fn vault_cooldown(events: &[SessionEvent], vault: u32) -> Option<u32> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellCooldowns { cooldowns, .. } => cooldowns
            .iter()
            .find(|(id, _)| *id == vault)
            .map(|(_, ms)| *ms),
        _ => None,
    })
}

/// Adrenaline Rush: a crit takes 6 s off Vault's cooldown, and a hit that does not crit takes
/// none. Flurry, on the Rapid Fire row, makes the first two Snapshots crit.
#[test]
fn adrenaline_rush_crits_take_six_seconds_off_vaults_cooldown() {
    let rush = ranger_choice("snapshot", "adrenaline_rush");
    let flurry = ranger_choice("rapid_fire", "flurry");
    let vault = spell("ranger", "vault");
    let mut s = ranger_scene(
        0x507_3000,
        &[("snapshot", rush), ("rapid_fire", flurry)],
        STURDY,
    );
    leap(&mut s.realm, &mut s.view, vault);
    let mob = s.mob.clone();
    face_from(&mut s.realm, &mut s.view, &mob, 25.0);
    s.realm.handle(cast(flurry, None));
    s.view.drain(&mut s.realm);
    // 15 s, less the 1 s gone (the leap's 0.9, the turn's 0.1), less 6 s a crit.
    let mut left = 15_000 - 1_000;
    let (mut crits, mut plain) = (0, 0);
    for shot in 0..8 {
        let events = s.cast_at_mob(rush);
        let landed = shots(&events, rush);
        let crit = landed.iter().any(|&(_, _, crit)| crit);
        let cut = vault_cooldown(&events, vault);
        if crit {
            left -= 6_000;
            let ms = cut.unwrap_or_else(|| panic!("shot {shot}: a crit cuts Vault: {events:?}"));
            assert!(
                ms.abs_diff(left) <= 150,
                "shot {shot}: {ms} ms left, want {left}"
            );
            crits += 1;
        } else {
            assert_eq!(cut, None, "shot {shot}: only a crit cuts it");
            plain += usize::from(!landed.is_empty());
        }
        if crits >= 2 && plain >= 1 {
            break;
        }
    }
    assert!(
        crits >= 2 && plain >= 1,
        "{crits} crits, {plain} plain hits"
    );
}

/// One Puncturing Shaft, or the Shaft choice `key`, at the sturdy scorpid on `seed`: what it
/// dealt and whether it crit, or `None` for a miss.
fn shaft(key: Option<&str>, seed: u64) -> Option<(u32, bool)> {
    let (id, rows) = match key {
        Some(key) => {
            let id = ranger_choice("puncturing_shaft", key);
            (id, vec![("puncturing_shaft", id)])
        }
        None => (spell("ranger", "puncturing_shaft"), vec![]),
    };
    let mut s = ranger_scene(seed, &rows, STURDY);
    let mob = s.mob.guid;
    let events = cast_through(&mut s.realm, &mut s.view, id, mob);
    shots(&events, id)
        .first()
        .map(|&(_, dealt, crit)| (dealt, crit))
}

/// Piercing Force: on the same dice a crit deals 1.25 times plain Puncturing Shaft's (250% where
/// it dealt 200%). The scorpid's armour is under what either Shaft ignores at 24.
#[test]
fn piercing_force_crits_deal_a_quarter_more_than_puncturing_shafts() {
    let mut crits = 0;
    for seed in 0..400 {
        let seed = 0x507_4000 + seed;
        let (Some((plain, true)), Some((force, true))) =
            (shaft(None, seed), shaft(Some("piercing_force"), seed))
        else {
            continue;
        };
        let want = plain as f32 * 1.25;
        assert!(
            (force as f32 - want).abs() <= 1.0,
            "seed {seed}: {plain} then {force}"
        );
        crits += 1;
    }
    assert!(crits >= 3, "{crits} crits on both");
}

/// The cast bar Killshot starts with at the sturdy scorpid once Snapshots have brought it inside
/// `health` percent of its health. Point Blank keeps the Ranger shooting once it reaches them.
fn killshot_bar_at(health: std::ops::RangeInclusive<u32>) -> Option<u32> {
    let killshot = ranger_choice("puncturing_shaft", "killshot");
    let point_blank = ranger_choice("talents", "point_blank");
    let snapshot = spell("ranger", "snapshot");
    let mut s = ranger_scene(
        0x507_5000,
        &[("puncturing_shaft", killshot), ("talents", point_blank)],
        STURDY,
    );
    let mob = s.mob.guid;
    for _ in 0..80 {
        let f = s.view.fields(mob);
        let now = f.unit_health().unwrap() * 100 / f.unit_max_health().unwrap();
        assert!(now > 0, "the scorpid died before {health:?}%");
        if health.contains(&now) {
            s.realm.handle(cast(killshot, Some(mob)));
            let events = s.view.drain(&mut s.realm);
            assert_eq!(refusal(&events, killshot), None, "{events:?}");
            return cast_bar(&events, killshot);
        }
        s.realm.handle(cast(snapshot, Some(mob)));
        s.view.drain(&mut s.realm);
        s.view.run(&mut s.realm, 0.3);
    }
    panic!("the scorpid never came inside {health:?}%");
}

/// Killshot: no cast bar against a target under 30% of its health, Puncturing Shaft's 3 s at
/// half health.
#[test]
fn killshot_is_instant_under_30_pct_health_and_a_3_second_cast_at_half() {
    assert_eq!(killshot_bar_at(1..=29), Some(0), "under 30%");
    assert_eq!(killshot_bar_at(35..=60), Some(3000), "at half health");
}

/// Point Blank: a shot 6 yd from the scorpid is too close without it and goes with it, and its
/// Puncturing Shaft there deals 20% more than the same roll from 10 yd.
#[test]
fn point_blank_shoots_inside_8_yards_with_a_fifth_more_on_the_shaft() {
    const SPELL_FAILED_TOO_CLOSE: u8 = 0x76;
    let point_blank = ranger_choice("talents", "point_blank");
    let snapshot = spell("ranger", "snapshot");
    let shaft = spell("ranger", "puncturing_shaft");
    for (rows, refused) in [
        (vec![], Some(SPELL_FAILED_TOO_CLOSE)),
        (vec![("talents", point_blank)], None),
    ] {
        let mut s = ranger_scene(0x507_6100, &rows, STURDY);
        let mob = s.mob.clone();
        face_from(&mut s.realm, &mut s.view, &mob, 6.0);
        s.realm.handle(cast(snapshot, Some(mob.guid)));
        let events = s.view.drain(&mut s.realm);
        assert_eq!(refusal(&events, snapshot), refused, "{rows:?}");
    }

    let mut compared = 0;
    for seed in 0..20 {
        let mut dealt = Vec::new();
        for yards in [10.0, 6.0] {
            let mut s = ranger_scene(0x507_6200 + seed, &[("talents", point_blank)], STURDY);
            let mob = s.mob.clone();
            face_from(&mut s.realm, &mut s.view, &mob, yards);
            let events = cast_through(&mut s.realm, &mut s.view, shaft, mob.guid);
            dealt.push(
                shots(&events, shaft)
                    .first()
                    .filter(|&&(_, _, crit)| !crit)
                    .map(|&(_, d, _)| d),
            );
        }
        if let [Some(far), Some(near)] = dealt[..] {
            let want = far as f32 * 1.2;
            assert!(
                (near as f32 - want).abs() <= 1.5,
                "seed {seed}: {far} then {near}"
            );
            compared += 1;
        }
    }
    assert!(compared >= 5, "{compared} plain hits compared");
}

/// Explosive Payload, set on the Talent row: its buff turns the next Auto Shot into Explosive
/// Shot, a Fire blast in the Auto Shot's place.
#[test]
fn explosive_payload_turns_the_next_auto_shot_into_a_fire_blast() {
    const SPELL_SCHOOL_FIRE: u8 = 2;
    let payload = ranger_choice("talents", "explosive_payload");
    let blast = spell("ranger", "explosive_shot");
    let mut s = ranger_scene(0x507_6300, &[("talents", payload)], STURDY);
    let mob = s.mob.guid;
    s.realm.handle(cast(payload, None));
    assert_eq!(refusal(&s.view.drain(&mut s.realm), payload), None);
    s.realm.handle(cast(*AUTO_SHOT, Some(mob)));
    let events = s.view.run(&mut s.realm, 0.5);
    let hits = events.iter().find_map(|e| match e {
        SessionEvent::SpellGo {
            spell_id,
            hits,
            misses,
            ..
        } if *spell_id == blast => Some(hits.len() + misses.len()),
        _ => None,
    });
    assert!(
        hits.is_some_and(|n| n >= 1),
        "the blast goes off: {events:?}"
    );
    assert!(events.iter().all(|e| match e {
        SessionEvent::SpellDamageLog(l) if l.spell_id == blast => l.school == SPELL_SCHOOL_FIRE,
        _ => true,
    }));
    assert_eq!(gone_off(&events, *AUTO_SHOT), 0, "in the Auto Shot's place");
    assert!(
        s.view
            .aura(s.me, spell("ranger", "aura_explosive_payload"))
            .is_none(),
        "the buff is spent"
    );
}
