//! Forest's Edge's Noise meter (#171): a Run-wide stack count shown as a debuff on every character
//! in the Phase, raised by the Run's clock and drained by hiding spots.

mod common;

use benilla_protocol::START_ELWYNN;
use common::fight::View;
use common::party::*;
use common::*;

/// `HIGHGUID_GAMEOBJECT`, in a GameObject guid's top 16 bits.
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
/// The first hiding spot, and where it stands (`content/forest_edge.toml`).
const SPOT: u64 = HIGHGUID_GAMEOBJECT << 48 | 790001;
const SPOT_AT: [f32; 3] = [-9928.0, 570.0, 38.0];
/// Inside the hollow, and out on the road beyond its west edge.
const INSIDE: [f32; 3] = [-9990.0, 600.0, 38.0];
const OUTSIDE: [f32; 3] = [-10200.0, 600.0, 38.0];
/// The meter's debuff (`[meter] client_spell`).
const NOISE_SPELL: u32 = 12323;

fn party(n: usize) -> Party {
    Party::starting(realm(), &vec![5; n], START_ELWYNN, Box::new(NoGround))
}

/// The stacks of the Noise debuff player `i` wears, as their client holds them.
fn noise(party: &Party, i: usize) -> u32 {
    let mut view = View::default();
    view.see(party.seen(i));
    view.aura(party.guids[i], NOISE_SPELL)
        .map_or(0, |a| u32::from(a.stacks))
}

fn hide(party: &mut Party, i: usize) {
    party.go(i, SPOT_AT, 0.0);
    party.cmd(i, RealmCommand::UseGameObject { guid: SPOT });
}

#[test]
fn the_clock_adds_a_stack_at_30_seconds() {
    let mut p = party(1);
    p.go(0, INSIDE, 0.0);
    p.run(29.0);
    assert_eq!(noise(&p, 0), 0, "a stack before 30 s");
    p.run(2.0);
    assert_eq!(noise(&p, 0), 1);
    p.run(30.0);
    assert_eq!(noise(&p, 0), 2);
}

#[test]
fn everyone_in_the_phase_sees_the_same_stacks_and_so_does_one_who_comes_in_later() {
    let mut p = party(3);
    p.go(0, INSIDE, 0.0);
    p.go(1, INSIDE, 0.0);
    p.go(2, OUTSIDE, 0.0);
    p.run(31.0);
    assert_eq!((noise(&p, 0), noise(&p, 1)), (1, 1));
    assert_eq!(noise(&p, 2), 0, "the meter shows outside the Phase");
    p.go(2, INSIDE, 0.0);
    p.run(0.5);
    assert_eq!(
        noise(&p, 2),
        1,
        "a character entering sees the Run's stacks"
    );
}

#[test]
fn leaving_and_coming_back_leaves_the_meter_as_it_was() {
    let mut p = party(2);
    p.go(0, INSIDE, 0.0);
    p.go(1, INSIDE, 0.0);
    p.run(31.0);
    p.go(0, OUTSIDE, 0.0);
    p.run(1.0);
    assert_eq!(noise(&p, 0), 0, "the debuff leaves with the character");
    assert_eq!(noise(&p, 1), 1);
    p.go(0, INSIDE, 0.0);
    p.run(0.5);
    assert_eq!(noise(&p, 0), 1);
    assert_eq!(noise(&p, 1), 1);
}

#[test]
fn a_hide_drains_a_stack_every_2_seconds_and_stops_when_the_hider_moves() {
    let mut p = party(1);
    p.go(0, INSIDE, 0.0);
    p.run(91.0);
    assert_eq!(noise(&p, 0), 3);
    hide(&mut p, 0);
    p.run(5.0);
    assert_eq!(noise(&p, 0), 1, "two stacks drained in 5 s");
    // The hide ends the moment they step away.
    p.go(0, [SPOT_AT[0] + 2.0, SPOT_AT[1], SPOT_AT[2]], 0.0);
    p.run(6.0);
    assert_eq!(
        noise(&p, 0),
        1,
        "a stack drained while they stood elsewhere"
    );
}

#[test]
fn two_hiders_drain_twice_as_fast() {
    let mut p = party(2);
    p.go(0, INSIDE, 0.0);
    p.go(1, INSIDE, 0.0);
    p.run(91.0);
    assert_eq!(noise(&p, 0), 3);
    hide(&mut p, 0);
    hide(&mut p, 1);
    p.run(2.5);
    assert_eq!(noise(&p, 0), 1, "one stack each in the first 2 s");
    assert_eq!(noise(&p, 1), 1);
}

#[test]
fn a_hiding_spot_out_of_reach_hides_no_one() {
    let mut p = party(1);
    p.go(0, INSIDE, 0.0);
    p.run(91.0);
    p.cmd(0, RealmCommand::UseGameObject { guid: SPOT });
    p.run(6.0);
    assert_eq!(noise(&p, 0), 3, "a hide from across the hollow");
}
