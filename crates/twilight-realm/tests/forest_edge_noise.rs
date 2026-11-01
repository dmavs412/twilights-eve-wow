//! Forest's Edge's Noise meter (#171): a Run-wide stack count shown on the Noise Bar of every
//! character in the Phase (#536), raised by the Run's clock and drained by Cover (#538).

mod common;

use benilla_protocol::{PhaseMeterState, SessionEvent, START_ELWYNN};
use common::fight::View;
use common::party::*;
use common::*;

/// `HIGHGUID_GAMEOBJECT`, in a GameObject guid's top 16 bits.
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
/// Where the first haystack stands (`content/forest_edge.toml`), Cover 2.5 yd round.
const SPOT_AT: [f32; 3] = [-9928.0, 570.0, 38.0];
/// The second haystack (790002), and the awake Outrunner (790325) that wanders 8 yd around its
/// post 15.8 yd west of it.
const SPOT_2_AT: [f32; 3] = [-9950.0, 585.0, 38.0];
const OUTRUNNER: u32 = 96006;
const OUTRUNNER_AT: [f32; 3] = [-9965.0, 590.0, 38.81];
/// The aura a character in Cover wears (`[meter] cover_aura`).
const IN_COVER: u32 = 1900042;
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

/// Player `i` walks into the first haystack's Cover; nothing is clicked.
fn hide(party: &mut Party, i: usize) {
    party.go(i, SPOT_AT, 0.0);
}

/// Whether player `i`'s client shows them wearing In Cover now.
fn in_cover(party: &Party, i: usize) -> bool {
    let guid = party.guids[i];
    let mut view = View::default();
    view.see(party.seen(i));
    view.objects.contains_key(&guid) && view.aura(guid, IN_COVER).is_some()
}

/// Whether `mob` started an attack on `victim` in `events`.
fn attacked(events: &[SessionEvent], mob: u64, victim: u64) -> bool {
    events.iter().any(|e| {
        matches!(e, SessionEvent::AttackStart { attacker, victim: v }
            if *attacker == mob && *v == victim)
    })
}

/// The walk-through lists player `i` was sent, each with its place in what they saw.
fn walk_throughs(party: &Party, i: usize) -> Vec<(usize, Vec<u64>)> {
    party
        .seen(i)
        .iter()
        .enumerate()
        .filter_map(|(at, e)| match e {
            SessionEvent::WalkThrough { guids } => Some((at, guids.clone())),
            _ => None,
        })
        .collect()
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
fn walking_into_cover_wears_in_cover_and_drains_a_stack_every_2_seconds_until_they_step_out() {
    let mut p = party(1);
    p.go(0, INSIDE, 0.0);
    p.run(91.0);
    assert_eq!(noise(&p, 0), 3);
    hide(&mut p, 0);
    p.run(5.0);
    assert!(in_cover(&p, 0), "no In Cover at the haystack");
    assert_eq!(noise(&p, 0), 1, "two stacks drained in 5 s");
    // Cover ends the moment they step out of its radius.
    p.go(0, [SPOT_AT[0] + 3.0, SPOT_AT[1], SPOT_AT[2]], 0.0);
    p.run(6.0);
    assert!(!in_cover(&p, 0), "In Cover 3 yd out");
    assert_eq!(
        noise(&p, 0),
        1,
        "a stack drained while they stood outside Cover"
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
fn a_character_just_outside_the_radius_is_not_in_cover() {
    let mut p = party(1);
    p.go(0, INSIDE, 0.0);
    p.run(91.0);
    p.go(0, [SPOT_AT[0] + 2.6, SPOT_AT[1], SPOT_AT[2]], 0.0);
    p.run(6.0);
    assert!(!in_cover(&p, 0), "In Cover 2.6 yd from the haystack");
    assert_eq!(noise(&p, 0), 3, "a drain from 2.6 yd");
    no_howl(&p);
}

#[test]
fn an_awake_outrunner_does_not_notice_a_character_in_cover_but_does_one_outside_it() {
    // At level 5 the level 27 Outrunner notices from 40 yd (18 + 22), all round its wander.
    let mut p = party(1);
    let me = p.guids[0];
    p.go(0, SPOT_2_AT, 0.0);
    p.run(10.0);
    let (outrunner, _) = p.unit(0, OUTRUNNER, OUTRUNNER_AT);
    assert!(in_cover(&p, 0), "no In Cover at the second haystack");
    assert!(
        !attacked(p.seen(0), outrunner, me),
        "the Outrunner engaged a character in Cover"
    );
    // As far from its post on the other side, clear of every Cover.
    let out = [
        2.0 * OUTRUNNER_AT[0] - SPOT_2_AT[0],
        2.0 * OUTRUNNER_AT[1] - SPOT_2_AT[1],
        SPOT_2_AT[2],
    ];
    p.go(0, out, 0.0);
    assert!(
        p.run_until(5.0, |got| attacked(&got[0], outrunner, me)),
        "the Outrunner never engaged a character outside Cover"
    );
}

#[test]
fn starting_an_attack_in_cover_takes_in_cover_off_that_tick() {
    let mut p = party(1);
    p.go(0, SPOT_2_AT, 0.0);
    p.run(1.0);
    assert!(in_cover(&p, 0));
    let (outrunner, _) = p.unit(0, OUTRUNNER, OUTRUNNER_AT);
    p.cmd(0, RealmCommand::AttackSwing { guid: outrunner });
    p.run(0.0);
    assert!(!in_cover(&p, 0), "In Cover after starting an attack");
}

#[test]
fn entering_sends_the_cover_to_walk_through_before_its_creates_and_leaving_sends_none() {
    let cover: Vec<u64> = (790001..=790006)
        .map(|g| HIGHGUID_GAMEOBJECT << 48 | g)
        .collect();
    let mut p = party(1);
    p.run(0.5);
    assert!(walk_throughs(&p, 0).is_empty(), "a list outside the Phase");
    p.go(0, INSIDE, 0.0);
    p.run(0.5);
    let lists = walk_throughs(&p, 0);
    assert_eq!(lists.len(), 1, "one list on entering");
    let (sent_at, mut guids) = lists[0].clone();
    guids.sort_unstable();
    assert_eq!(guids, cover);
    let created: Vec<usize> = p
        .seen(0)
        .iter()
        .enumerate()
        .filter(
            |(_, e)| matches!(e, SessionEvent::ObjectCreate { guid, .. } if cover.contains(guid)),
        )
        .map(|(at, _)| at)
        .collect();
    assert!(!created.is_empty(), "no Cover created");
    assert!(
        created.iter().all(|&at| at > sent_at),
        "a Cover create before the list"
    );
    p.go(0, OUTSIDE, 0.0);
    p.run(0.5);
    let lists = walk_throughs(&p, 0);
    assert_eq!(lists.len(), 2);
    assert_eq!(lists[1].1, Vec::<u64>::new(), "leaving sends an empty list");
}
