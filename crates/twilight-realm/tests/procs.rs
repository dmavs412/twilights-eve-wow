//! `proc`: "when X happens, do Y", as the pure trigger rule. A cast proc's skill filter, and the
//! clocks of the movement triggers.

use twilight_realm::callings::ProcEvent;
use twilight_realm::rules::procs::{Happened, Trigger};

#[test]
fn a_cast_proc_filtered_to_one_skill_fires_only_for_that_skill() {
    let trigger = Trigger {
        on: ProcEvent::Cast,
        skill: Some("charge"),
        yields_xp: false,
        school: None,
        ms: 0,
    };
    let cast = |skill| Happened {
        event: ProcEvent::Cast,
        skill,
        yields_xp: false,
        target: None,
        amount: 0,
        school: None,
        was_ms: 0,
    };
    assert!(trigger.fires(&cast(Some("charge"))));
    assert!(!trigger.fires(&cast(Some("heroic_strike"))));
    assert!(!trigger.fires(&Happened {
        event: ProcEvent::Kill,
        skill: None,
        yields_xp: true,
        target: None,
        amount: 0,
        school: None,
        was_ms: 0,
    }));
    let unfiltered = Trigger {
        skill: None,
        ..trigger
    };
    assert!(unfiltered.fires(&cast(Some("heroic_strike"))));
}

/// `moving_for` fires each time the moving clock crosses another `ms`; `still_for` only the first
/// time the still clock reaches it.
#[test]
fn movement_triggers_fire_as_their_clock_crosses_ms() {
    let tick = |event, was_ms, amount| Happened {
        event,
        skill: None,
        yields_xp: false,
        target: None,
        amount,
        school: None,
        was_ms,
    };
    let moving = Trigger {
        on: ProcEvent::MovingFor,
        skill: None,
        yields_xp: false,
        school: None,
        ms: 2000,
    };
    assert!(!moving.fires(&tick(ProcEvent::MovingFor, 1900, 1950)));
    assert!(moving.fires(&tick(ProcEvent::MovingFor, 1950, 2000)));
    assert!(!moving.fires(&tick(ProcEvent::MovingFor, 2000, 2050)));
    assert!(moving.fires(&tick(ProcEvent::MovingFor, 3990, 4040)));
    let still = Trigger {
        on: ProcEvent::StillFor,
        ..moving
    };
    assert!(still.fires(&tick(ProcEvent::StillFor, 1990, 2040)));
    assert!(!still.fires(&tick(ProcEvent::StillFor, 3990, 4040)));
    assert!(!still.fires(&tick(ProcEvent::MovingFor, 1990, 2040)));
}
