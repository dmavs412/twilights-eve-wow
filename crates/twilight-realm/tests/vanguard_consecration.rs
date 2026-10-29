//! Consecration, the ground effect Bulwark Charge leaves where the charge ends: every second for
//! eight, each enemy inside the patch takes Holy damage, and the charger standing in it wears
//! Consecrated Ground, 10% less damage taken (#509). Played through the realm's seam by a
//! level 4 Vanguard charging the first mob of the scorpid camp south of the Crossroads, from
//! the west. The charge ends two yards short of it; the camp's next mob stands ten yards from
//! that spot and, being out of social range of the first, is never pulled.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings;
use twilight_realm::{Realm, RealmCommand};

const SEED: u64 = 0x5EED_0103;
const LEVEL: u8 = 4;
/// Seconds to watch: the patch's eight, with two to spare.
const WATCH_SECS: f32 = 10.0;

fn vanguard() -> &'static callings::CallingDef {
    callings::calling("vanguard").expect("the Vanguard is a shipped Calling")
}

/// Bulwark Charge's spell id, from the Vanguard's rows.
fn charge_id() -> u32 {
    vanguard()
        .skills
        .iter()
        .find(|s| s.key == "bulwark_charge")
        .expect("the Vanguard has Bulwark Charge")
        .spell_id
}

/// Consecration's spell id: the row the patch is named after, which its object and ticks carry.
fn consecration_id() -> u32 {
    spell("vanguard", "consecration")
}

/// Consecrated Ground's spell id: the ward the charger wears while standing in their own patch.
fn ward_id() -> u32 {
    spell("vanguard", "aura_consecration_ward")
}

/// The DynamicObjects `events` create: guid, spell id, centre and radius.
fn patch_objects(events: &[SessionEvent]) -> Vec<(u64, u32, [f32; 3], f32)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::DynamicObject,
                position,
                fields,
                ..
            } => Some((
                *guid,
                fields.dynamicobject_spell_id()?,
                *position,
                fields.dynamicobject_radius()?,
            )),
            _ => None,
        })
        .collect()
}

fn destroyed(events: &[SessionEvent], guid: u64) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == guid))
}

/// The level 8 or 9 mob at the edge of the camp south of the Crossroads, sturdy enough to outlast
/// the patch (`content/barrens.toml`, 9000034).
const STURDY: [f32; 3] = [-452.0, -3014.0, 0.0];

/// A level 4 Vanguard 25 yd west of the mob at `spot` and facing it: that mob, the charge's
/// target, and the one ten yards from where the charge ends, if any.
fn scene(spot: [f32; 3]) -> (Realm, View, u64, Mob, Option<Mob>) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", &format!("level = {LEVEL}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"vanguard\"");
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [spot[0] - 25.0, spot[1], 0.0],
        orientation: 0.0,
    });
    let streamed = view.run(&mut realm, 1.0);
    let camp: Vec<Mob> = streamed
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: benilla_protocol::EntityKind::Unit,
                position,
                ..
            } if flat(*position, spot) < 30.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect();
    let at = |p: [f32; 3]| camp.iter().find(|m| flat(m.position, p) < 1.0).cloned();
    let near = at(spot).expect("the mob the charge goes for streams in");
    let far = at([spot[0] - 10.0, spot[1] + 6.0, 0.0]);
    (realm, view, me, near, far)
}

/// What a scene's mobs took from the patch, each tick's damage in order.
struct Ticks {
    near: Vec<u32>,
    /// The mob ten yards from where the charge ends, if the scene has one.
    far: Option<Vec<u32>>,
}

/// Charges at the mob at `spot`, then watches the patch run out.
fn charge(spot: [f32; 3]) -> Ticks {
    let (mut realm, mut view, _, near, far) = scene(spot);
    let id = charge_id();
    realm.handle(cast(id, Some(near.guid)));
    let mut events = view.drain(&mut realm);
    assert_eq!(refusal(&events, id), None, "Bulwark Charge goes off");
    // The blows of the swing would add to the patch's.
    realm.handle(RealmCommand::AttackStop);
    events.extend(view.run(&mut realm, WATCH_SECS));
    let patch = |guid: u64| -> Vec<u32> {
        events
            .iter()
            .filter_map(|e| match e {
                SessionEvent::SpellDamageLog(l)
                    if l.spell_id == consecration_id() && l.target == guid && l.school == 1 =>
                {
                    Some(l.damage)
                }
                _ => None,
            })
            .collect()
    };
    assert!(
        view.health(near.guid) > 0,
        "the near mob outlived the patch"
    );
    Ticks {
        near: patch(near.guid),
        far: far.map(|m| patch(m.guid)),
    }
}

#[test]
fn an_enemy_at_the_destination_takes_eight_ticks_of_holy_damage() {
    let ticks = charge(CAMP);
    assert_eq!(ticks.near.len(), 8, "{:?}", ticks.near);
    assert!(ticks.near.iter().all(|d| *d > 0 && *d == ticks.near[0]));
}

#[test]
fn an_enemy_ten_yards_away_takes_none() {
    let ticks = charge(CAMP);
    assert_eq!(
        ticks.far,
        Some(Vec::new()),
        "the far mob is in view and untouched"
    );
}

/// The patch is a DynamicObject named Consecration, 8 yd wide where the charge ended, two yards
/// short of the mob; it is destroyed as the patch ends, eight seconds on.
#[test]
fn the_patch_is_a_consecration_object_at_the_destination_until_it_ends() {
    let (mut realm, mut view, _, near, _) = scene(STURDY);
    realm.handle(cast(charge_id(), Some(near.guid)));
    let events = view.drain(&mut realm);
    realm.handle(RealmCommand::AttackStop);
    let objects = patch_objects(&events);
    let [(guid, spell_id, centre, radius)] = objects[..] else {
        panic!("one object for the patch, has {objects:?}");
    };
    assert_eq!(spell_id, consecration_id());
    assert_eq!(radius, 8.0);
    assert!(
        (flat(centre, near.position) - 2.0).abs() < 0.5,
        "{centre:?} is where the charge ended, beside {:?}",
        near.position
    );
    assert!(!destroyed(&view.run(&mut realm, 7.5), guid), "still up");
    assert!(destroyed(&view.run(&mut realm, 1.0), guid), "gone at 8 s");
}

/// Charges `near` and stops swinging: where the patch is centred.
fn charged(realm: &mut Realm, view: &mut View, near: &Mob) -> [f32; 3] {
    realm.handle(cast(charge_id(), Some(near.guid)));
    let events = view.drain(realm);
    assert_eq!(
        refusal(&events, charge_id()),
        None,
        "Bulwark Charge goes off"
    );
    realm.handle(RealmCommand::AttackStop);
    view.drain(realm);
    let objects = patch_objects(&events);
    let [(_, _, centre, _)] = objects[..] else {
        panic!("one object for the patch, has {objects:?}");
    };
    centre
}

/// The charger wears Consecrated Ground as the patch lands under them, and loses it 12 yd out.
#[test]
fn the_charger_wears_the_ward_inside_the_patch() {
    let (mut realm, mut view, me, near, _) = scene(STURDY);
    let centre = charged(&mut realm, &mut view, &near);
    assert!(
        view.aura(me, ward_id()).is_some(),
        "the charger stands in their patch"
    );
    view.run(&mut realm, 1.0);
    assert!(
        view.aura(me, ward_id()).is_some(),
        "still inside a second on"
    );
    // 12 yd south of where the charge ended, away from the camp.
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [centre[0], centre[1] - 12.0, centre[2]],
        orientation: 0.0,
    });
    view.run(&mut realm, 0.2);
    assert!(
        view.aura(me, ward_id()).is_none(),
        "12 yd out, the ward is off"
    );
}

/// Standing still, the charger keeps the ward while the patch is down and loses it as it ends.
#[test]
fn the_ward_goes_when_the_patch_ends() {
    let (mut realm, mut view, me, near, _) = scene(CAMP);
    charged(&mut realm, &mut view, &near);
    view.run(&mut realm, 7.5);
    assert!(
        view.aura(me, ward_id()).is_some(),
        "on while the patch is down"
    );
    view.run(&mut realm, WATCH_SECS - 7.5);
    assert!(view.aura(me, ward_id()).is_none(), "gone with the patch");
}

/// A blow that landed on the player: the damage its report gave, the health it took, and whether
/// the ward was on before and after the tick it landed on.
#[derive(Debug)]
struct Blow {
    reported: u32,
    lost: u32,
    warded: bool,
}

/// The blows that land on `me` over `secs`, one 0.1 s tick at a time: only a tick with one
/// blow, no other harm to the player and the ward the same before and after.
fn blows(realm: &mut Realm, view: &mut View, me: u64, secs: f32) -> Vec<Blow> {
    let mut out = Vec::new();
    for _ in 0..(secs / 0.1).round() as u32 {
        let (health, warded) = (view.health(me), view.aura(me, ward_id()).is_some());
        let events = view.run(realm, 0.1);
        let landed: Vec<u32> = events
            .iter()
            .filter_map(|e| match e {
                SessionEvent::AttackerState(s) if s.victim == me && s.damage > 0 => Some(s.damage),
                _ => None,
            })
            .collect();
        let other_harm = events.iter().any(|e| {
            matches!(e, SessionEvent::SpellDamageLog(l) if l.target == me)
                || matches!(e, SessionEvent::PeriodicAuraLog(l) if l.target == me)
        });
        let still = view.aura(me, ward_id()).is_some() == warded;
        if let (&[reported], false, true) = (&landed[..], other_harm, still) {
            out.push(Blow {
                reported,
                lost: health - view.health(me),
                warded,
            });
        }
    }
    out
}

/// The mob at the charge's end swings at the Vanguard standing still: through the patch's eight
/// seconds each blow takes 90% of what its report says (rounded down), and after it ends the
/// whole of it. The report is the blow before the ward, so one seed's blows test both.
#[test]
fn a_blow_inside_the_patch_is_a_tenth_lighter() {
    let (mut realm, mut view, me, near, _) = scene(CAMP);
    charged(&mut realm, &mut view, &near);
    let blows = blows(&mut realm, &mut view, me, WATCH_SECS + 6.0);
    let (inside, outside): (Vec<&Blow>, Vec<&Blow>) = blows.iter().partition(|b| b.warded);
    assert!(!inside.is_empty() && !outside.is_empty(), "{blows:?}");
    for b in inside {
        assert_eq!(b.lost, b.reported * 9 / 10, "{b:?}");
    }
    for b in outside {
        assert_eq!(b.lost, b.reported, "{b:?}");
    }
}
