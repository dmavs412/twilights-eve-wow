//! Forest's Edge's Noise meter (#171): a Run-wide stack count shown on the Noise Bar of every
//! character in the Phase (#536), raised by the Run's clock and drained by hiding spots.

mod common;

use benilla_protocol::{PhaseMeterState, SessionEvent, START_ELWYNN};
use common::fight::View;
use common::party::*;
use common::*;

/// `HIGHGUID_GAMEOBJECT`, in a GameObject guid's top 16 bits.
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
/// The first hiding spot, and where it stands (`content/forest_edge.toml`).
const SPOT: u64 = HIGHGUID_GAMEOBJECT << 48 | 790001;
const SPOT_AT: [f32; 3] = [-9928.0, 570.0, 38.0];
/// Inside the hollow by the Portal, far from the Riverpaw camp (#173), and out on the road beyond
/// its west edge.
const INSIDE: [f32; 3] = [-9925.0, 560.0, 38.0];
const OUTSIDE: [f32; 3] = [-10200.0, 600.0, 38.0];
/// Piercing Howl, the debuff that showed the meter before the Noise Bar.
const PIERCING_HOWL: u32 = 12323;
/// The Camp 1 Riverpaw Mystic, asleep (guid 790302), and the aura a sleeper wears
/// (`[meter] sleep_aura`).
const MYSTIC: u32 = 96007;
const MYSTIC_AT: [f32; 3] = [-9955.0, 608.0, 37.23];
const SLEEPING: u32 = 1900041;

fn party(n: usize) -> Party {
    Party::starting(realm(), &vec![5; n], START_ELWYNN, Box::new(NoGround))
}

/// The meters player `i` was sent, in order (`SessionEvent::PhaseMeter`).
fn meters(party: &Party, i: usize) -> Vec<Option<PhaseMeterState>> {
    party
        .seen(i)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PhaseMeter { meter } => Some(meter.clone()),
            _ => None,
        })
        .collect()
}

/// The meter player `i` was last sent; `None` with no bar up.
fn last_meter(party: &Party, i: usize) -> Option<PhaseMeterState> {
    meters(party, i).pop().flatten()
}

/// The stacks on the Noise Bar player `i` was last sent; 0 with no bar up.
fn noise(party: &Party, i: usize) -> u32 {
    last_meter(party, i).map_or(0, |m| m.stacks)
}

/// No character's client ever showed them wearing the old Noise debuff.
fn no_howl(party: &Party) {
    for (i, &guid) in party.guids.iter().enumerate() {
        let mut view = View::default();
        let wore = party.seen(i).iter().any(|e| {
            view.see(std::slice::from_ref(e));
            view.objects.contains_key(&guid) && view.aura(guid, PIERCING_HOWL).is_some()
        });
        assert!(!wore, "player {i} wore Piercing Howl");
    }
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
    no_howl(&p);
}

#[test]
fn stepping_in_puts_the_bar_up_empty_and_leaving_takes_it_down() {
    let mut p = party(1);
    p.run(0.5);
    assert!(meters(&p, 0).is_empty(), "no bar outside the Phase");
    p.go(0, INSIDE, 0.0);
    p.run(0.5);
    assert_eq!(
        meters(&p, 0),
        vec![Some(PhaseMeterState {
            name: "Noise".into(),
            stacks: 0,
            max: 40,
            end_icon: r"Interface\Benilla\noise\hogger_head".into(),
            rose: None,
        })],
        "the bar goes up empty, once"
    );
    p.go(0, OUTSIDE, 0.0);
    p.run(0.5);
    assert_eq!(meters(&p, 0).last(), Some(&None), "and comes down outside");
    assert_eq!(meters(&p, 0).len(), 2);
    no_howl(&p);
}

#[test]
fn the_clocks_rise_names_its_cause_and_a_hides_drain_names_none() {
    let mut p = party(1);
    p.go(0, INSIDE, 0.0);
    p.run(31.0);
    let rose = last_meter(&p, 0).unwrap();
    assert_eq!(
        (rose.stacks, rose.rose.as_deref()),
        (1, Some("The camp stirs"))
    );
    hide(&mut p, 0);
    p.run(2.5);
    let drained = last_meter(&p, 0).unwrap();
    assert_eq!((drained.stacks, drained.rose), (0, None), "a hide's drain");
    no_howl(&p);
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
    no_howl(&p);
}

#[test]
fn leaving_and_coming_back_leaves_the_meter_as_it_was() {
    let mut p = party(2);
    p.go(0, INSIDE, 0.0);
    p.go(1, INSIDE, 0.0);
    p.run(31.0);
    p.go(0, OUTSIDE, 0.0);
    p.run(1.0);
    assert_eq!(noise(&p, 0), 0, "the bar leaves with the character");
    assert_eq!(noise(&p, 1), 1);
    p.go(0, INSIDE, 0.0);
    p.run(0.5);
    assert_eq!(noise(&p, 0), 1);
    assert_eq!(noise(&p, 1), 1);
    no_howl(&p);
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
    no_howl(&p);
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
    no_howl(&p);
}

#[test]
fn a_sleeper_wears_sleeping_until_a_character_steps_up_and_wakes_it() {
    let mut p = party(1);
    p.go(0, [MYSTIC_AT[0] + 10.0, MYSTIC_AT[1], MYSTIC_AT[2]], 0.0);
    p.run(0.5);
    let (mystic, _) = p.unit(0, MYSTIC, MYSTIC_AT);
    let sleeping = |p: &Party| {
        let mut view = View::default();
        view.see(p.seen(0));
        view.aura(mystic, SLEEPING).is_some()
    };
    assert!(sleeping(&p), "asleep with a character 10 yd east");
    p.go(0, [MYSTIC_AT[0] + 1.0, MYSTIC_AT[1], MYSTIC_AT[2]], 0.0);
    p.run(0.5);
    assert!(!sleeping(&p), "woken from 1 yd");
}

#[test]
fn a_hiding_spot_out_of_reach_hides_no_one() {
    let mut p = party(1);
    p.go(0, INSIDE, 0.0);
    p.run(91.0);
    p.cmd(0, RealmCommand::UseGameObject { guid: SPOT });
    p.run(6.0);
    assert_eq!(noise(&p, 0), 3, "a hide from across the hollow");
    no_howl(&p);
}
