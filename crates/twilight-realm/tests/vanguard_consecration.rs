//! Consecration, the ground effect Bulwark Charge leaves where the charge ends: every second for
//! eight, each enemy inside the patch takes Holy damage. Played through the realm's seam by a
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
