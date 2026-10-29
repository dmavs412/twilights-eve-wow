//! The Cutthroat's blood droplets: a tick of its own bleed may spill a droplet 3 to 6 yd from the
//! mob, walking over one grants a stack of Bloodlust, and one nobody touches fades after 8 s.
//! Played through the realm's seam by a level 10 Cutthroat at fixed seeds. A test Calling that
//! spills a droplet on every tick shows Bloodlust stops at five stacks and a droplet 5 yd away
//! grants none.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

static SINISTER_STRIKE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "sinister_strike"));
static RUPTURE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "rupture"));
static DROPLET: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "aura_blood_droplet"));
static BLOODLUST: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "aura_bloodlust"));

/// A droplet's guid, spell id and centre, as its create shows it.
type Droplet = (u64, [f32; 3]);

/// A level 10 Cutthroat, fists only, in the world at `seed`; its guid.
fn cutthroat(seed: u64) -> (Realm, View, u64) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"cutthroat\"");
    let blob = drop_slot(&drop_slot(&blob, 15), 16);
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    (realm, view, me)
}

fn camp_mob(events: &[SessionEvent]) -> Mob {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } if flat(*position, CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("a camp mob streams in")
}

/// The droplets `events` create.
fn droplets(events: &[SessionEvent]) -> Vec<Droplet> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::DynamicObject,
                position,
                fields,
                ..
            } if fields.dynamicobject_spell_id() == Some(*DROPLET) => Some((*guid, *position)),
            _ => None,
        })
        .collect()
}

/// Sinister Strike for a combo point, then a Rupture, and the 10 s after it: the mob and every
/// event since the Rupture.
fn rupture_a_mob(realm: &mut Realm, view: &mut View, secs: f32) -> (Mob, Vec<SessionEvent>) {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [CAMP[0] + 40.0, CAMP[1], 0.0],
        orientation: std::f32::consts::PI,
    });
    let mob = camp_mob(&view.run(realm, 1.0));
    stand_at(realm, &mob, 2.0);
    realm.handle(cast(*SINISTER_STRIKE, Some(mob.guid)));
    view.run(realm, 1.6);
    realm.handle(cast(*RUPTURE, Some(mob.guid)));
    let mut events = view.drain(realm);
    events.extend(view.run(realm, secs));
    (mob, events)
}

#[test]
fn rupture_ticks_spill_droplets_three_to_six_yards_from_the_mob() {
    let mut count = 0;
    for seed in 0..20 {
        let (mut realm, mut view, _) = cutthroat(seed);
        let (mob, events) = rupture_a_mob(&mut realm, &mut view, 9.0);
        for (_, at) in droplets(&events) {
            let away = flat(at, mob.position);
            assert!((3.0..=6.0).contains(&away), "seed {seed}: {away} yd");
            count += 1;
        }
    }
    assert!((25..=55).contains(&count), "{count} droplets in 80 ticks");
}

/// Rupture on a camp mob, stepping 0.1 s at a time for up to `secs`; the events up to the step
/// that spilled the first droplet, and that droplet.
fn until_droplet(
    realm: &mut Realm,
    view: &mut View,
    secs: f32,
) -> Option<(Vec<SessionEvent>, Droplet)> {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [CAMP[0] + 40.0, CAMP[1], 0.0],
        orientation: std::f32::consts::PI,
    });
    let mob = camp_mob(&view.run(realm, 1.0));
    stand_at(realm, &mob, 2.0);
    realm.handle(cast(*SINISTER_STRIKE, Some(mob.guid)));
    view.run(realm, 1.6);
    realm.handle(cast(*RUPTURE, Some(mob.guid)));
    let mut events = view.drain(realm);
    for _ in 0..(secs / 0.1).round() as u32 {
        realm.tick(0.1, &FlatGround);
        let step = view.drain(realm);
        let found = droplets(&step).first().copied();
        events.extend(step);
        if let Some(d) = found {
            return Some((events, d));
        }
    }
    None
}

#[test]
fn walking_over_a_droplet_grants_a_stack_of_bloodlust_and_takes_it_away() {
    for seed in 0..20 {
        let (mut realm, mut view, me) = cutthroat(seed);
        let Some((_, (object, at))) = until_droplet(&mut realm, &mut view, 9.0) else {
            continue;
        };
        if view.aura(me, *BLOODLUST).is_some() {
            continue;
        }
        realm.handle(RealmCommand::Move {
            motion: Default::default(),
            position: at,
            orientation: 0.0,
        });
        let events = view.run(&mut realm, 0.2);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == object)),
            "seed {seed}: the droplet is destroyed"
        );
        let lust = view
            .aura(me, *BLOODLUST)
            .expect("Bloodlust is on the player");
        assert_eq!(lust.stacks, 1, "seed {seed}");
        return;
    }
    panic!("no seed spilled a droplet");
}

#[test]
fn a_droplet_nobody_touches_is_destroyed_eight_seconds_after_its_create() {
    for seed in 0..20 {
        let (mut realm, mut view, me) = cutthroat(seed);
        let Some((_, (object, _))) = until_droplet(&mut realm, &mut view, 9.0) else {
            continue;
        };
        let mut steps = 0;
        let mut lust = view.aura(me, *BLOODLUST).is_some();
        let mut gone = false;
        while !gone && steps < 120 {
            realm.tick(0.1, &FlatGround);
            let events = view.drain(&mut realm);
            lust |= view.aura(me, *BLOODLUST).is_some();
            gone = events
                .iter()
                .any(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == object));
            steps += 1;
        }
        // A later droplet may have landed under the player; that seed proves nothing.
        if lust {
            continue;
        }
        assert!(gone, "seed {seed}: the droplet is still down after 12 s");
        assert!(
            (79..=81).contains(&steps),
            "seed {seed}: gone after {steps} steps of 0.1 s"
        );
        return;
    }
    panic!("no seed spilled an untouched droplet");
}

/// A Rogue-Class Calling whose Rupture bleeds every second for 12 s and spills a droplet on every
/// tick.
const BLOOD_CALLINGS: &str = r#"
[[calling]]
key = "test_rogue"
name = "Test Rogue"
tier = 0
class = 4
cap = 10
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth", "leather"]
bleed_drop = "droplet"
bleed_drop_chance = 100

[[calling.skill]]
key = "builder"
spell_id = 1752
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 5.0]
effects = [{ kind = "add_combo_points", amount = 1 }]

[[calling.skill]]
key = "rupture"
spell_id = 1943
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 5.0]
effects = [{ kind = "periodic_damage", tick_ms = 1000, duration_ms = 12000, tick = { base = 0.0 }, per_combo = { base = 0.0 }, duration_per_combo = { base = 0.0 } }]

[[calling.aura]]
key = "bloodlust"
spell_id = 1000411
duration_ms = 10000
row = { max_stacks = 5, stack_rule = "stack" }
effects = [{ kind = "mod_damage_done_pct", pct = 3 }]

[[calling.aura]]
key = "droplet"
spell_id = 1000412
duration_ms = 8000
effects = [{ kind = "pickup", grants = "bloodlust", radius_yd = 1.0, scatter_min_yd = 3.0, scatter_max_yd = 6.0 }]
"#;

const TEST_BUILDER: u32 = 1752;
const TEST_RUPTURE: u32 = 1943;

/// The test Rogue, fists only, with a Rupture on a camp mob that spills a droplet a second: the
/// realm, its view and guid.
fn bleeder() -> (Realm, View, u64) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_rogue\"");
    let blob = drop_slot(&drop_slot(&blob, 15), 16);
    let mut realm = Realm::with_seed(1).with_callings(BLOOD_CALLINGS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [CAMP[0] + 40.0, CAMP[1], 0.0],
        orientation: std::f32::consts::PI,
    });
    let mob = camp_mob(&view.run(&mut realm, 1.0));
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(TEST_BUILDER, Some(mob.guid)));
    view.run(&mut realm, 0.1);
    realm.handle(cast(TEST_RUPTURE, Some(mob.guid)));
    view.drain(&mut realm);
    (realm, view, me)
}

/// Steps 0.1 s at a time for up to 12 s, standing `away` yd from each droplet as it appears, until
/// `want` have spilled, then 0.3 s more.
fn stand_near_droplets(realm: &mut Realm, view: &mut View, away: f32, want: usize) {
    let mut spilled = 0;
    for _ in 0..120 {
        realm.tick(0.1, &FlatGround);
        let step = view.drain(realm);
        for (_, at) in droplets(&step) {
            spilled += 1;
            realm.handle(RealmCommand::Move {
                motion: Default::default(),
                position: [at[0] + away, at[1], at[2]],
                orientation: 0.0,
            });
        }
        if spilled >= want {
            view.run(realm, 0.3);
            break;
        }
    }
}

fn bloodlust_stacks(view: &View, me: u64) -> u32 {
    view.aura(me, *BLOODLUST).map_or(0, |a| a.stacks.into())
}

#[test]
fn bloodlust_stops_at_five_stacks() {
    let (mut realm, mut view, me) = bleeder();
    stand_near_droplets(&mut realm, &mut view, 0.0, 8);
    assert_eq!(bloodlust_stacks(&view, me), 5, "eight pickups");
}

#[test]
fn standing_five_yards_from_a_droplet_grants_no_bloodlust() {
    let (mut realm, mut view, me) = bleeder();
    stand_near_droplets(&mut realm, &mut view, 5.0, 1);
    assert_eq!(bloodlust_stacks(&view, me), 0);
}
