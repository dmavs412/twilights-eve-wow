//! The Ranger's procs and riders on its Talent Rows (#507): Jagged Barbs' and Bloodscent's bleed
//! ticks, Rhythmic Draw's faster Shaft, Flurry's free crits and Rapid Assault's extra arrow.
//! Played through the realm's seam by a shipped Ranger of 24, 25 yd west of the sturdy scorpid
//! south of the Crossroads and facing it.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::cast;
use common::initiate::cast_bar;
use common::ranger::{mana, ranger, ranger_choice, ranger_scene, shots};
use common::vanguard::STURDY;

/// Jagged Barbs: each bleed tick has a 20% chance to put Sundered on the bleeding scorpid, three
/// stacks at most; never on the Ranger. Plain Barbed Arrow sunders nothing.
#[test]
fn jagged_barbs_ticks_sunder_the_bleeding_mob_on_a_winning_roll() {
    let barbs = ranger_choice("barbed_arrow", "jagged_barbs");
    let sundered = ranger("aura_sundered_5");
    let plain = ranger("barbed_arrow");
    for (rows, arrow, sunders) in [
        (vec![("barbed_arrow", barbs)], barbs, true),
        (vec![], plain, false),
    ] {
        let mut hit = 0;
        for seed in 0..12 {
            let mut s = ranger_scene(0x507_B000 + seed, &rows, STURDY);
            let mob = s.mob.guid;
            s.cast_at_mob(arrow);
            let mut most = 0;
            for _ in 0..85 {
                s.view.run(&mut s.realm, 0.1);
                assert!(s.view.aura(s.me, sundered).is_none(), "never on the Ranger");
                if let Some(slot) = s.view.aura(mob, sundered) {
                    most = most.max(slot.stacks);
                }
            }
            assert!(most <= 3, "seed {seed}: {most} stacks");
            hit += usize::from(most > 0);
        }
        if sunders {
            assert!((2..=11).contains(&hit), "{hit} of 12 bleeds sundered");
        } else {
            assert_eq!(hit, 0, "plain Barbed Arrow sunders nothing");
        }
    }
}

/// The mana each tick of the bleed `spell_id` gave the Ranger in `events`.
fn tick_mana(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellEnergizeLog(l) if l.spell_id == spell_id => Some(l.amount),
            _ => None,
        })
        .collect()
}

/// Bloodscent: each bleed tick gives 2% of the Ranger's base mana, which is two fifths of Barbed
/// Arrow's 5% cost. Plain Barbed Arrow gives none.
#[test]
fn bloodscents_ticks_give_two_pct_of_base_mana() {
    let scent = ranger_choice("barbed_arrow", "bloodscent");
    // The bleed ticks under Barbed Arrow's aura, whichever arrow put it on.
    let bleed = ranger("aura_barbed_arrow");
    let mut s = ranger_scene(0x507_C000, &[("barbed_arrow", scent)], STURDY);
    let before = mana(&s.view, s.me);
    s.cast_at_mob(scent);
    let cost = before - mana(&s.view, s.me);
    let events = s.view.run(&mut s.realm, 8.5);
    let ticks = tick_mana(&events, bleed);
    assert!(ticks.len() >= 3, "a refund a tick: {ticks:?}");
    // Base mana is 20 to 20.95 times the cost; 2% of it, rounded down.
    let (low, high) = (cost * 40 / 100, (cost * 40 + 38) / 100);
    assert!(
        ticks.iter().all(|&t| t >= low && t <= high && t > 0),
        "{ticks:?}, cost {cost}"
    );

    let mut s = ranger_scene(0x507_C000, &[], STURDY);
    s.cast_at_mob(ranger("barbed_arrow"));
    let events = s.view.run(&mut s.realm, 8.5);
    assert!(
        tick_mana(&events, bleed).is_empty(),
        "plain Barbed Arrow gives none"
    );
}

/// Rhythmic Draw, the old talent at its full rank: each cast that lands takes 0.5 s off the next
/// Puncturing Shaft's bar, stacking, and the Shaft spends them. Plain Snapshot takes nothing off.
#[test]
fn rhythmic_draw_shortens_the_next_shaft_by_half_a_second_a_cast() {
    let draw = ranger_choice("snapshot", "rhythmic_draw");
    let buff = ranger("aura_rhythmic_draw_5");
    let shaft = ranger("puncturing_shaft");
    for (rows, shot, bar) in [
        (vec![("snapshot", draw)], draw, 2000),
        (vec![], ranger("snapshot"), 3000),
    ] {
        let mut s = ranger_scene(0x507_D000, &rows, STURDY);
        // Two that land: a miss grants nothing.
        let mut landed = 0;
        for _ in 0..10 {
            landed += usize::from(!shots(&s.cast_at_mob(shot), shot).is_empty());
            if landed == 2 {
                break;
            }
        }
        assert_eq!(landed, 2);
        let events = s.cast_at_mob(shaft);
        assert_eq!(cast_bar(&events, shaft), Some(bar), "{rows:?}");
        s.view.run(&mut s.realm, 2.1);
        assert!(s.view.aura(s.me, buff).is_none(), "the Shaft spent it");
    }
}

/// Flurry, the old talent at its full rank: after Rapid Fire's Flurry, two Snapshots cost no mana
/// and crit; the third costs mana.
#[test]
fn flurry_makes_the_next_two_snapshots_free_certain_crits() {
    let flurry = ranger_choice("rapid_fire", "flurry");
    let snapshot = ranger("snapshot");
    let mut s = ranger_scene(0x507_E000, &[("rapid_fire", flurry)], STURDY);
    s.realm.handle(cast(flurry, None));
    s.view.drain(&mut s.realm);
    let mut landed = 0;
    for n in 0..6 {
        let before = mana(&s.view, s.me);
        let events = s.cast_at_mob(snapshot);
        let hits = shots(&events, snapshot);
        if landed < 2 {
            assert_eq!(mana(&s.view, s.me), before, "Snapshot {n} is free");
            assert!(
                hits.iter().all(|&(_, _, crit)| crit),
                "Snapshot {n} crits: {hits:?}"
            );
        } else {
            assert!(
                mana(&s.view, s.me) < before,
                "the third landed one costs mana"
            );
            break;
        }
        landed += usize::from(!hits.is_empty());
    }
    assert_eq!(landed, 2);
}

/// Rapid Assault: while Rapid Fire's buff is up, each landed Snapshot strikes twice, the second
/// for half a bow roll; a Snapshot choice set on its row too.
#[test]
fn rapid_assault_adds_a_half_strike_to_each_snapshot() {
    let assault = ranger_choice("rapid_fire", "rapid_assault");
    let draw = ranger_choice("snapshot", "rhythmic_draw");
    for (rows, shot) in [
        (vec![("rapid_fire", assault)], ranger("snapshot")),
        (vec![("rapid_fire", assault), ("snapshot", draw)], draw),
    ] {
        let mut doubled = 0;
        for seed in 0..6 {
            let mut s = ranger_scene(0x507_F000 + seed, &rows, STURDY);
            s.realm.handle(cast(assault, None));
            s.view.drain(&mut s.realm);
            let hits = shots(&s.cast_at_mob(shot), shot);
            if hits.is_empty() {
                continue;
            }
            assert_eq!(hits.len(), 2, "{rows:?}: {hits:?}");
            assert!(
                hits[1].1 < hits[0].1,
                "the extra arrow is the smaller: {hits:?}"
            );
            doubled += 1;
        }
        assert!(doubled >= 3, "{rows:?}: {doubled} doubled");
    }
}
