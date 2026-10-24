//! The Cutthroat's blood droplets: a tick of its own bleed may spill a droplet 3 to 6 yd from the
//! mob, walking over one grants a stack of Bloodlust, and one nobody touches fades after 8 s.
//! Played through the realm's seam by a level 10 Cutthroat at fixed seeds.

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
