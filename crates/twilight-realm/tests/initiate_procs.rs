//! Ley-Line Tap (#506): Ley-Spark's Skill Choice whose landed cast rolls 50% to give its mana
//! cost back, or, at a bar that was full before the cast, to shield the Initiate for twice the
//! cost instead. Played through the realm's seam by a shipped Initiate at 24 on the level 8 or 9
//! mob south of the Crossroads.

mod common;

use benilla_protocol::SessionEvent;
use common::cutthroat::energized;
use common::initiate::*;
use common::vanguard::{Scene, STURDY};
use common::*;

/// Seeds enough for the 50% roll to land and to miss.
const SEEDS: u64 = 12;

fn tap() -> u32 {
    choice("initiate", "ley_spark", "ley_line_tap")
}

/// Casts `spell_id` at the scene's mob through its cast bar: the events, and the mana it took off
/// the bar.
fn spark(s: &mut Scene, spell_id: u32) -> (Vec<SessionEvent>, i64) {
    let before = i64::from(mana(&s.view, s.me));
    let events = cast_through(&mut s.realm, &mut s.view, spell_id, s.mob.guid);
    (events, before - i64::from(mana(&s.view, s.me)))
}

/// Below a full bar each cast either costs Ley-Spark's mana or gives all of it back, logged under
/// Ley-Line Tap; plain Ley-Spark gives nothing back.
#[test]
fn below_full_mana_ley_line_taps_hit_gives_back_its_cost_and_ley_spark_gives_nothing() {
    let tap = tap();
    let (mut kept, mut back) = (Vec::new(), Vec::new());
    for seed in 0..SEEDS {
        let mut s = initiate_scene(seed, &[("ley_spark", tap)], STURDY);
        // The first cast leaves the bar below full; the five-second pause holds it there.
        spark(&mut s, tap);
        let (events, spent) = spark(&mut s, tap);
        match energized(&events, tap)[..] {
            [] => kept.push(spent),
            [amount] => back.push((i64::from(amount), spent)),
            ref more => panic!("one refund a cast, logged {more:?}"),
        }
    }
    assert!(!kept.is_empty() && !back.is_empty(), "{kept:?} {back:?}");
    let cost = kept[0];
    assert!(cost > 0 && kept.iter().all(|&k| k == cost), "{kept:?}");
    for (amount, spent) in back {
        assert_eq!((amount, spent), (cost, 0), "the whole cost comes back");
    }

    let plain = initiate("ley_spark");
    for seed in 0..SEEDS {
        let mut s = initiate_scene(seed, &[], STURDY);
        spark(&mut s, plain);
        let (events, spent) = spark(&mut s, plain);
        assert!(energized(&events, plain).is_empty(), "seed {seed}");
        assert_eq!(spent, cost, "seed {seed}");
    }
}

/// From a full bar each cast costs Ley-Spark's mana and gives none back; when the roll lands, a
/// shield goes up that soaks exactly twice that cost of the mob's blows.
#[test]
fn at_full_mana_ley_line_tap_shields_for_twice_its_cost_and_gives_nothing_back() {
    let tap = tap();
    let (mut shielded, mut bare) = (0, 0);
    for seed in 0..SEEDS {
        let mut s = initiate_scene(seed, &[("ley_spark", tap)], STURDY);
        let me = s.me;
        assert_eq!(mana(&s.view, me), max_mana(&s.view, me), "a full bar");
        let (events, spent) = spark(&mut s, tap);
        assert!(
            energized(&events, tap).is_empty(),
            "seed {seed}: nothing back"
        );
        if s.view.aura(me, tap).is_none() {
            bare += 1;
            continue;
        }
        shielded += 1;
        let blows = blows(&mut s.realm, &mut s.view, me, 10.0, |v| {
            v.aura(me, tap).is_some()
        });
        let soaked: u32 = blows
            .iter()
            .filter(|b| b.watched)
            .map(|b| b.reported - b.lost)
            .sum();
        assert_eq!(i64::from(soaked), 2 * spent, "seed {seed}: {blows:?}");
        assert!(
            s.view.aura(me, tap).is_none(),
            "seed {seed}: the shield broke"
        );
    }
    assert!(shielded > 0 && bare > 0, "{shielded} shielded, {bare} bare");
}
