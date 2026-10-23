//! Bulwark Taunt (ADR 0020): a `self_area` skill with a `taunt` effect. Every enemy in reach
//! turns on the caster, whose threat there rises to the highest, and fights the caster for the
//! Taunt's duration. Played through the realm's seam with a test Calling whose taunt is free.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::{cast, refusal};
use common::party::*;
use common::*;
use twilight_realm::callings::{Effect, Target};

/// A free `self_area` taunt of radius 10 and 4 s.
const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_tank"
name = "Test Tank"
tier = 0
class = 1
cap = 60
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "taunt"
spell_id = 1161
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self_area"
radius_yd = 10.0
max_targets = 100
effects = [{ kind = "taunt", duration_ms = 4000 }]
"#;

const TAUNT: u32 = 1161;
/// A Razormane Hunter at Thorn Hill, levels 11–12.
const HUNTER: u32 = 3265;
const THORN_HILL: [f32; 3] = [-38.801, -3234.81, 91.889];

fn east_of(p: [f32; 3], yd: f32) -> [f32; 3] {
    [p[0] + yd, p[1], p[2]]
}

/// `n` level 60 test tanks, in a realm where the taunt is a skill; none yet near the camp.
fn party(n: usize) -> Party {
    Party::edited(
        Realm::with_seed(3).with_callings(TEST_CALLINGS),
        &vec![60; n],
        0,
        Box::new(NoGround),
        |_, blob| blob.replace("calling = \"basic_warrior\"", "calling = \"test_tank\""),
    )
}

/// Whether any session saw `hunter` turn on `victim` from `from` events on.
fn turned_on(party: &Party, from: [usize; 2], hunter: u64, victim: u64) -> bool {
    (0..2).any(|i| {
        party.seen(i)[from[i]..].iter().any(|e| {
            matches!(e, SessionEvent::AttackStart { attacker, victim: v }
                if *attacker == hunter && *v == victim)
        })
    })
}

fn seen_lens(party: &Party) -> [usize; 2] {
    [party.seen(0).len(), party.seen(1).len()]
}

/// The player stands `yards` from an idle hunter and casts the taunt at once, before any tick;
/// whether the hunter turned on them.
fn taunt_from(yards: f32) -> bool {
    let mut party = party(2);
    party.go(0, east_of(THORN_HILL, 60.0), 0.0);
    party.go(1, east_of(THORN_HILL, 200.0), 0.0);
    party.run(0.0);
    let (hunter, at) = party.unit(0, HUNTER, THORN_HILL);
    let me = party.guids[0];
    party.go(0, east_of(at, yards), std::f32::consts::PI);
    let from = seen_lens(&party);
    party.cmd(0, cast(TAUNT, None));
    turned_on(&party, from, hunter, me)
}

#[test]
fn an_idle_mob_eight_yards_off_turns_on_the_taunter() {
    assert!(taunt_from(8.0));
}

#[test]
fn an_idle_mob_twelve_yards_off_is_left_alone() {
    assert!(!taunt_from(12.0));
}

#[test]
fn a_taunt_pulls_a_mob_off_the_one_it_fights_and_another_takes_it_back() {
    let mut party = party(2);
    party.go(1, east_of(THORN_HILL, 200.0), 0.0);
    party.go(0, THORN_HILL, 0.0);
    party.run(0.0);
    let (hunter, at) = party.unit(0, HUNTER, THORN_HILL);
    let (a, b) = (party.guids[0], party.guids[1]);
    assert!(
        turned_on(&party, [0, 0], hunter, a),
        "the hunter turns on the one who came close"
    );

    // The second player walks up and taunts: the hunter had a's threat alone.
    party.go(1, east_of(at, 8.0), std::f32::consts::PI);
    let from = seen_lens(&party);
    party.cmd(1, cast(TAUNT, None));
    assert_eq!(refusal(party.seen(1), TAUNT), None, "the taunt is cast");
    party.run(0.5);
    assert!(
        turned_on(&party, from, hunter, b),
        "it turns on the taunter"
    );

    // Two seconds on, the first player taunts: their threat is level with b's, so only the Taunt
    // takes the mob from its current target.
    party.run(1.5);
    let from = seen_lens(&party);
    party.cmd(0, cast(TAUNT, None));
    party.run(0.5);
    assert!(
        turned_on(&party, from, hunter, a),
        "a later taunt takes it back inside the first's four seconds"
    );
}

#[test]
fn the_shipped_bulwark_taunt_loads_at_level_twenty_on_the_area_around_the_caster() {
    let skill = twilight_realm::callings::skill(spell("vanguard", "bulwark_taunt"))
        .expect("vanguard/bulwark_taunt loads");
    assert_eq!(skill.learn_level, 20);
    assert_eq!(skill.target, Target::SelfArea);
    assert_eq!(skill.radius_yd, 10.0);
    assert_eq!(skill.power_cost, 0);
    assert_eq!(skill.cooldown_ms, 10000);
    assert_eq!(skill.effects, vec![Effect::Taunt { duration_ms: 4000 }]);
}
