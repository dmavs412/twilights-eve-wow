//! Blink: a forward teleport that sheds stuns and stops where the ground ends. Played through the
//! realm's seam with a test Mage Calling.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

/// `blink` 20 yd and `stun_self` a 10 s stun on the caster.
const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_mage"
name = "Test Mage"
tier = 0
class = 8
cap = 10
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "blink"
spell_id = 1953
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "blink", distance_yd = 20.0 }]

[[calling.skill]]
key = "stun_self"
spell_id = 5211
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "stun", aura_spell_id = 5211, duration_ms = 10000 }]
"#;

const BLINK: u32 = 1953;
const STUN: u32 = 5211;

/// Where the test Mage stands: east of the scorpid camp, facing it.
const STAND: [f32; 3] = [CAMP[0] + 40.0, CAMP[1], 0.0];

/// A level 10 test Mage east of the camp: the realm, their guid and the view.
fn world(seed: u64) -> (Realm, u64, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Weaver", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_mage\"");
    let mut realm = Realm::with_seed(seed).with_callings(TEST_CALLINGS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: STAND,
        orientation: std::f32::consts::PI,
    });
    view.run(&mut realm, 1.0);
    (realm, guid, view)
}

/// Casts `spell_id`, asserting it went.
fn go(realm: &mut Realm, view: &mut View, spell_id: u32, target: Option<u64>) {
    realm.handle(cast(spell_id, target));
    assert_eq!(
        refusal(&view.drain(realm), spell_id),
        None,
        "spell {spell_id} is cast"
    );
}

/// Where the player was teleported to in `events`.
fn teleported(events: &[SessionEvent]) -> Option<[f32; 3]> {
    events.iter().find_map(|e| match e {
        SessionEvent::Teleport { position, .. } => Some(*position),
        _ => None,
    })
}

#[test]
fn blink_moves_the_caster_20_yards_forward_and_clears_a_stun() {
    let (mut realm, me, mut view) = world(1);
    go(&mut realm, &mut view, STUN, None);
    assert!(view.aura(me, STUN).is_some(), "stunned");
    go(&mut realm, &mut view, BLINK, None);
    assert!(view.aura(me, STUN).is_none(), "the stun is shed");
    let to = teleported(&view.run(&mut realm, 0.1)).expect("a teleport");
    let ahead = [STAND[0] - 20.0, STAND[1], 0.0];
    assert!(flat(to, ahead) < 0.01, "20 yd along the facing: {to:?}");
}

/// Flat ground that ends 10 yd west of where the test Mage stands.
struct Cliff;

impl twilight_realm::Host for Cliff {
    fn ground_height(&self, _map: u32, x: f32, _y: f32, _z_hint: f32) -> Option<f32> {
        (x >= STAND[0] - 10.0).then_some(0.0)
    }
    fn line_of_sight(&self, _map: u32, _from: [f32; 3], _to: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _map: u32, _x: f32, _y: f32) -> Option<u32> {
        None
    }
}

#[test]
fn blink_stops_where_the_ground_ends() {
    let (mut realm, _, mut view) = world(3);
    go(&mut realm, &mut view, BLINK, None);
    realm.tick(0.1, &Cliff);
    let to = teleported(&view.drain(&mut realm)).expect("a teleport");
    assert!(
        flat(to, [STAND[0] - 10.0, STAND[1], 0.0]) < 0.01,
        "as far as the ground goes: {to:?}"
    );
}

// Frostbite, and the passives Shattered Blink and Arcane Momentum (#506), played by a shipped
// Initiate at 24 on the level 8 or 9 mob south of the Crossroads.

use common::initiate::*;
use common::vanguard::STURDY;
use twilight_realm::callings::Effect;

/// The first Frostbite at a clean mob slows it and roots nothing, since its own slow came with
/// the cast; the second, at the mob that slow left behind, roots it 2 s under the choice's root
/// aura. Plain Biting Cold twice roots nothing.
#[test]
fn frostbite_roots_a_slowed_target_for_2_s_and_not_one_only_this_cast_slows() {
    let frostbite = choice("initiate", "biting_cold", "frostbite");
    let Effect::Root {
        aura_spell_id: root,
        duration_ms,
    } = choice_skill("biting_cold", "frostbite").effects[2]
    else {
        panic!("Frostbite's third effect is its root");
    };
    let slow = initiate("aura_biting_cold");
    let mut s = initiate_scene(3, &[("biting_cold", frostbite)], STURDY);
    let mob = s.mob.guid;
    cast_through(&mut s.realm, &mut s.view, frostbite, mob);
    assert!(s.view.aura(mob, slow).is_some(), "slowed");
    assert!(
        s.view.aura(mob, root).is_none(),
        "not rooted by its own slow"
    );
    cast_through(&mut s.realm, &mut s.view, frostbite, mob);
    assert!(s.view.aura(mob, root).is_some(), "rooted: it was slowed");
    let secs = lasts(&mut s.realm, &mut s.view, mob, root, 5.0);
    let want = duration_ms as f32 / 1000.0;
    assert!((secs - want).abs() <= 0.15, "rooted {secs} s, want {want}");

    let cold = initiate("biting_cold");
    let mut s = initiate_scene(3, &[], STURDY);
    let mob = s.mob.guid;
    for _ in 0..2 {
        cast_through(&mut s.realm, &mut s.view, cold, mob);
        assert!(
            s.view.aura(mob, root).is_none(),
            "plain Biting Cold roots nothing"
        );
    }
}

/// Casts Blink standing 6 yd from the scene's mob, out of its notice, and acknowledges the
/// teleport it lands as.
fn blink_beside(s: &mut common::vanguard::Scene) {
    let mob = s.mob.clone();
    stand_at(&mut s.realm, &mob, 6.0);
    s.view.run(&mut s.realm, 0.5);
    let blink = initiate("blink");
    s.realm.handle(cast(blink, None));
    let events = s.view.drain(&mut s.realm);
    assert_eq!(refusal(&events, blink), None, "Blink goes off");
    let counter = s
        .view
        .run(&mut s.realm, 0.2)
        .iter()
        .find_map(|e| match e {
            SessionEvent::Teleport { counter, .. } => Some(*counter),
            _ => None,
        })
        .expect("Blink lands as a teleport");
    s.realm.handle(RealmCommand::TeleportAck { counter });
}

/// Under Shattered Blink, a Blink roots the mob 6 yd from where the Initiate stood for 4 s;
/// without it, nothing.
#[test]
fn shattered_blink_roots_a_mob_within_8_yards_of_where_the_initiate_stood() {
    let shattered = choice("initiate", "passives", "shattered_blink");
    let ring = initiate("aura_shattered_blink_2");
    for (rows, rooted) in [(vec![("passives", shattered)], true), (vec![], false)] {
        let mut s = initiate_scene(3, &rows, STURDY);
        let mob = s.mob.guid;
        blink_beside(&mut s);
        assert_eq!(s.view.aura(mob, ring).is_some(), rooted, "{rows:?}");
        if rooted {
            // Blink landed 0.2 s ago.
            let secs = lasts(&mut s.realm, &mut s.view, mob, ring, 6.0);
            assert!((secs - 3.8).abs() <= 0.15, "rooted {secs} s more");
        }
    }
}

/// Turns back to the scene's mob, which the Blink went past: 10 yd from it, facing it.
fn face(s: &mut common::vanguard::Scene) {
    let mob = s.mob.clone();
    stand_at(&mut s.realm, &mob, 10.0);
    s.view.run(&mut s.realm, 0.1);
}

/// Under Arcane Momentum a Blink grants its buff, and a Ley-Spark inside its 5 s hits for 1.45
/// times plain Ley-Spark's hit (rounded); without it, Blink grants nothing.
#[test]
fn arcane_momentum_after_blink_makes_ley_spark_hit_45_percent_harder() {
    let momentum = choice("initiate", "passives", "arcane_momentum");
    let buff = initiate("aura_arcane_momentum_3");
    let spark = initiate("ley_spark");
    let (mut plain, mut quick) = (Vec::new(), Vec::new());
    for seed in 0..10 {
        let mut s = initiate_scene(seed, &[("passives", momentum)], STURDY);
        let (me, mob) = (s.me, s.mob.guid);
        blink_beside(&mut s);
        assert!(s.view.aura(me, buff).is_some(), "seed {seed}: the buff");
        face(&mut s);
        let events = cast_through(&mut s.realm, &mut s.view, spark, mob);
        quick.extend(plain_hits_on(&events, spark, mob));

        let mut s = initiate_scene(seed, &[], STURDY);
        let (me, mob) = (s.me, s.mob.guid);
        blink_beside(&mut s);
        assert!(s.view.aura(me, buff).is_none(), "seed {seed}: no buff");
        face(&mut s);
        let events = cast_through(&mut s.realm, &mut s.view, spark, mob);
        plain.extend(plain_hits_on(&events, spark, mob));
    }
    assert!(
        !plain.is_empty() && !quick.is_empty(),
        "{plain:?} {quick:?}"
    );
    let hit = plain[0];
    assert!(plain.iter().all(|&h| h == hit), "{plain:?}");
    let want = (hit as f32 * 1.45).round() as u32;
    assert!(quick.iter().all(|&h| h == want), "{quick:?}, want {want}");
}
