//! Two of Cutthroat's row choices (#505) that pay out on an event: Phantom Guile's Evasion gives
//! 10 Energy and a combo point on the target for each dodge, and Executioner's Toll's killing
//! Eviscerate heals a fifth of the maximum health and gives 80 Energy. Played through the realm's
//! seam by a level 24 Cutthroat at the scorpid camp south of the Crossroads, at a fixed seed.

mod common;

use benilla_protocol::SessionEvent;
use common::cutthroat::*;
use common::fight::*;
use common::*;
use twilight_realm::RealmCommand;

const SEED: u64 = 0x5EED_0505;
/// `VictimState` of a dodged swing.
const DODGED: u32 = 2;

static SINISTER_STRIKE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "sinister_strike"));
static EVISCERATE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "eviscerate"));
static EVASION: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "evasion"));
static EVASION_AURA: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "aura_evasion"));
static PHANTOM_GUILE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("cutthroat", "evasion", "phantom_guile"));
static PHANTOM_GUILE_AURA: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "aura_phantom_guile"));
static EXECUTIONERS_TOLL: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("cutthroat", "eviscerate", "executioners_toll"));

/// How many swings at `me` in `events` were dodged.
fn dodges(events: &[SessionEvent], me: u64) -> u32 {
    events
        .iter()
        .filter(|e| {
            matches!(e, SessionEvent::AttackerState(s)
                if s.victim == me && s.victim_state == DODGED)
        })
        .count() as u32
}

/// One step of a fight under Evasion that saw a dodge.
#[derive(Debug)]
struct Dodged {
    dodges: u32,
    /// The Energy before the step, and each energize the step logged under the buff.
    energy: u32,
    gave: Vec<u32>,
    /// The points on the fisted mob before and after the step.
    points: (u8, u8),
    /// Where the points sit after the step.
    on: u64,
}

/// The camp pulled onto a level 24 Cutthroat with fists and `rows` set, who casts `evasion`,
/// spends its Energy with two Sinister Strikes at another mob and fists the first mob: each step
/// with a dodge in up to 30 s (the logs under `aura`), until `want` dodges, and the fisted mob.
fn dodging(rows: &[(&str, u32)], evasion: u32, aura: u32, want: u32) -> (Vec<Dodged>, u64) {
    let Pack {
        mut realm,
        mut view,
        me,
        mobs,
    } = pulled(SEED, ROWS_OPEN, true, rows);
    realm.handle(cast(evasion, None));
    assert_eq!(refusal(&view.drain(&mut realm), evasion), None, "Evasion");
    for _ in 0..2 {
        try_cast(&mut realm, &mut view, *SINISTER_STRIKE, mobs[1].guid);
        view.run(&mut realm, 1.1);
    }
    let mob = mobs[0].guid;
    realm.handle(RealmCommand::AttackSwing { guid: mob });
    let mut steps = Vec::new();
    let mut seen = 0;
    for _ in 0..300 {
        let before = match combo(&view, me) {
            (points, on) if on == mob => points,
            _ => 0,
        };
        let energy = energy(&view, me);
        let events = view.run(&mut realm, 0.1);
        let d = dodges(&events, me);
        if d == 0 {
            continue;
        }
        if view.health(mob) == 0 {
            break;
        }
        let (after, on) = combo(&view, me);
        steps.push(Dodged {
            dodges: d,
            energy,
            gave: energized(&events, aura),
            points: (before, after),
            on,
        });
        seen += d;
        if seen >= want {
            break;
        }
    }
    (steps, mob)
}

#[test]
fn a_dodge_under_phantom_guile_gives_ten_energy_and_a_point_on_the_target() {
    let (steps, mob) = dodging(
        &[("evasion", *PHANTOM_GUILE)],
        *PHANTOM_GUILE,
        *PHANTOM_GUILE_AURA,
        4,
    );
    assert!(steps.len() >= 2, "too few dodges: {steps:?}");
    for s in &steps {
        assert_eq!(s.on, mob, "the points go on the fisted mob: {s:?}");
        let more = (s.points.0 + s.dodges as u8).min(5);
        assert_eq!(s.points.1, more, "a point a dodge: {s:?}");
        // A full bar takes no more and logs nothing.
        if s.energy + 10 * s.dodges <= 100 {
            assert_eq!(
                s.gave,
                vec![10; s.dodges as usize],
                "10 Energy a dodge: {s:?}"
            );
        }
    }
    assert!(
        steps.iter().any(|s| !s.gave.is_empty()),
        "no dodge came while the bar had room: {steps:?}"
    );
}

#[test]
fn a_dodge_under_plain_evasion_gives_neither() {
    let (steps, mob) = dodging(&[], *EVASION, *EVASION_AURA, 4);
    assert!(!steps.is_empty(), "no dodges to prove it");
    for s in &steps {
        assert!(s.gave.is_empty(), "Energy from a dodge: {s:?}");
        assert!(
            s.on != mob || s.points.1 == 0,
            "a point from a dodge: {s:?}"
        );
    }
}

/// One cast of a finisher that landed on a mob.
#[derive(Debug)]
struct Finish {
    killed: bool,
    /// The health and the Energy logged under the finisher.
    healed: Vec<u32>,
    gave: Vec<u32>,
    /// The health missing just before the cast.
    missing: u32,
}

/// The camp pulled onto a level 24 Cutthroat with `rows` set until a quarter of its health is
/// gone; then the camp's mobs one by one, built up with Sinister Strikes and hit with `finisher`
/// once it can kill: each `finisher` that landed, up to the first that killed, and the maximum
/// health.
fn finishing(rows: &[(&str, u32)], finisher: u32) -> (Vec<Finish>, u32) {
    let Pack {
        mut realm,
        mut view,
        me,
        mobs: camp,
    } = pulled(SEED, ROWS_OPEN, false, rows);
    let max = view.fields(me).unit_max_health().unwrap();
    for _ in 0..600 {
        if max - view.health(me) >= max / 4 {
            break;
        }
        view.run(&mut realm, 0.1);
        assert!(view.health(me) > 0, "died waiting to be hurt");
    }
    assert!(
        max - view.health(me) >= max / 4,
        "the camp never wore down a quarter"
    );
    let mut finishes = Vec::new();
    for mob in &camp {
        for _ in 0..300 {
            if view.health(mob.guid) == 0 {
                break;
            }
            let points = match combo(&view, me) {
                (points, on) if on == mob.guid => u32::from(points),
                _ => 0,
            };
            // A level 24 Eviscerate deals about 45 and 45 a point; a mob under that dies.
            let killable = points > 0 && view.health(mob.guid) <= 40 * (points + 1);
            if energy(&view, me) >= 35 {
                let spell_id = if killable { finisher } else { *SINISTER_STRIKE };
                let missing = max - view.health(me);
                let events = try_cast(&mut realm, &mut view, spell_id, mob.guid);
                let landed = events
                    .as_ref()
                    .is_some_and(|e| damage_of(e, spell_id).iter().any(|d| d.0 == mob.guid));
                if spell_id == finisher && landed {
                    let events = events.unwrap();
                    let killed = view.health(mob.guid) == 0;
                    finishes.push(Finish {
                        killed,
                        healed: healed(&events, finisher),
                        gave: energized(&events, finisher),
                        missing,
                    });
                    if killed {
                        return (finishes, max);
                    }
                }
            }
            view.run(&mut realm, 0.1);
            assert!(view.health(me) > 0, "died in the fight");
        }
    }
    panic!("no {finisher} killed a mob: {finishes:?}");
}

#[test]
fn executioners_tolls_killing_blow_heals_a_fifth_of_max_health_and_gives_80_energy() {
    let (finishes, max) = finishing(&[("eviscerate", *EXECUTIONERS_TOLL)], *EXECUTIONERS_TOLL);
    let kill = finishes.last().unwrap();
    assert!(kill.missing >= max / 5, "{kill:?} of {max}");
    let fifth = (f64::from(max) * 0.2).round() as u32;
    assert_eq!(kill.healed, [fifth], "{kill:?} of {max}");
    assert_eq!(kill.gave, [80], "{kill:?}");
    for f in finishes.iter().filter(|f| !f.killed) {
        assert!(
            f.healed.is_empty() && f.gave.is_empty(),
            "a Toll that did not kill: {f:?}"
        );
    }
}

#[test]
fn eviscerates_killing_blow_heals_nothing_and_gives_no_energy() {
    let (finishes, _) = finishing(&[], *EVISCERATE);
    let kill = finishes.last().unwrap();
    assert!(kill.healed.is_empty() && kill.gave.is_empty(), "{kill:?}");
}
