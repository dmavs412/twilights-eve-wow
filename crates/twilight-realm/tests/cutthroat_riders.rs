//! The Cutthroat's Sinister Strike in a crowd: it strikes its target alone, and gives one combo
//! point on a target bleeding from the caster's Rupture. Flurry of Blades, a choice of its row
//! (#505), also strikes a second mob in reach. Played through the realm's seam by a level 10
//! Cutthroat (24 for the choice), at fixed seeds.

mod common;

use benilla_protocol::SessionEvent;
use common::cutthroat::{camp_mobs, combo, cutthroat_save, damage_of, Pack, ROWS_OPEN};
use common::fight::*;
use common::vanguard::enter;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const FIRST_SEED: u64 = 0x5EED_0112;

static SINISTER_STRIKE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "sinister_strike"));
static RUPTURE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "rupture"));
static FLURRY_OF_BLADES: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("cutthroat", "sinister_strike", "flurry_of_blades"));

/// A level 10 Cutthroat in the world at `seed`, `unarmed` to make the fight last: the realm, its
/// guid and what the client has seen.
fn cutthroat(seed: u64, unarmed: bool) -> (Realm, u64, View) {
    let (me, blob) = cutthroat_save(10, unarmed);
    let (mut realm, mut view, _) = enter(me, &blob, seed);
    view.run(&mut realm, 1.0);
    (realm, me, view)
}

/// A level 10 Cutthroat at `seed` with the camp pulled onto them: every mob has stopped within
/// reach.
fn pulled(seed: u64) -> Pack {
    common::cutthroat::pulled(seed, 10, false, &[])
}

/// Casts Sinister Strike at `target`: the guids it landed on or missed, sorted.
fn strike(pack: &mut Pack, target: &Mob) -> Vec<u64> {
    pack.realm.handle(cast(*SINISTER_STRIKE, Some(target.guid)));
    let events = pack.view.drain(&mut pack.realm);
    assert_eq!(refusal(&events, *SINISTER_STRIKE), None, "Sinister Strike");
    let mut landed: Vec<u64> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellGo {
                spell_id,
                hits,
                misses,
                ..
            } if *spell_id == *SINISTER_STRIKE => Some(
                hits.iter()
                    .copied()
                    .chain(misses.iter().map(|(g, _)| *g))
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .filter(|g| *g != pack.me)
        .collect();
    landed.sort_unstable();
    landed
}

#[test]
fn sinister_strike_hits_one_dummy() {
    for seed in FIRST_SEED..FIRST_SEED + 20 {
        let mut pack = pulled(seed);
        let primary = pack.mobs[0].clone();
        assert_eq!(strike(&mut pack, &primary), [primary.guid]);
    }
}

/// Casts `spell_id` at `target`: the damage it logged on each mob, in the order logged.
fn struck(pack: &mut Pack, spell_id: u32, target: &Mob) -> Vec<(u64, u32)> {
    pack.realm.handle(cast(spell_id, Some(target.guid)));
    let events = pack.view.drain(&mut pack.realm);
    assert_eq!(refusal(&events, spell_id), None, "spell {spell_id}");
    damage_of(&events, spell_id)
        .into_iter()
        .map(|(guid, damage, _)| (guid, damage))
        .collect()
}

#[test]
fn flurry_of_blades_with_two_mobs_in_reach_damages_both() {
    let rows = [("sinister_strike", *FLURRY_OF_BLADES)];
    let mut checked = 0;
    for seed in FIRST_SEED..FIRST_SEED + 10 {
        let mut pack = common::cutthroat::pulled(seed, ROWS_OPEN, false, &rows);
        let primary = pack.mobs[0].clone();
        let hits = struck(&mut pack, *FLURRY_OF_BLADES, &primary);
        let Some(&(_, first)) = hits.iter().find(|h| h.0 == primary.guid) else {
            continue;
        };
        let others: Vec<&(u64, u32)> = hits.iter().filter(|h| h.0 != primary.guid).collect();
        assert_eq!(
            others.len(),
            1,
            "seed {seed}: a second mob takes a share: {hits:?}"
        );
        assert_eq!(
            others[0].1,
            first / 2,
            "seed {seed}: half the strike: {hits:?}"
        );
        // The plain strike on the same dice strikes its target alone.
        let mut plain = common::cutthroat::pulled(seed, ROWS_OPEN, false, &[]);
        let hits = struck(&mut plain, *SINISTER_STRIKE, &primary);
        assert!(
            hits.iter().all(|h| h.0 == primary.guid),
            "seed {seed}: {hits:?}"
        );
        checked += 1;
    }
    assert!(checked >= 3, "only {checked} strikes landed");
}

/// A camp mob struck by Sinister Strike, Rupture and Sinister Strike again, gaps between, by a
/// Cutthroat with fists, so the mob outlives them. The combo points the last strike left, or
/// `None` when a blow missed or a cast was refused.
fn last_strike_points(seed: u64) -> Option<u8> {
    let (mut realm, me, mut view) = cutthroat(seed, true);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [CAMP[0] + 40.0, CAMP[1], 0.0],
        orientation: std::f32::consts::PI,
    });
    let mob = camp_mobs(&view.run(&mut realm, 1.0)).swap_remove(0);
    stand_at(&mut realm, &mob, 2.0);
    let spells = [*SINISTER_STRIKE, *RUPTURE, *SINISTER_STRIKE];
    for (i, &spell_id) in spells.iter().enumerate() {
        realm.handle(cast(spell_id, Some(mob.guid)));
        if refusal(&view.drain(&mut realm), spell_id).is_some() {
            return None;
        }
        let (points, on) = combo(&view, me);
        if i + 1 == spells.len() {
            // A killing blow takes the bleed, and the points, with the target.
            if view.health(mob.guid) == 0 {
                return None;
            }
            return (on == mob.guid && points > 0).then_some(points);
        }
        view.run(&mut realm, 1.5);
        let (points, _) = combo(&view, me);
        // A strike that landed left a point; a Rupture that landed spent it.
        if (spell_id == *SINISTER_STRIKE) != (points == 1) {
            return None;
        }
        if spell_id == *RUPTURE && view.aura(mob.guid, *RUPTURE).is_none() {
            return None;
        }
    }
    None
}

#[test]
fn sinister_strike_gives_a_ruptured_dummy_one_point() {
    let mut checked = 0;
    for seed in FIRST_SEED..FIRST_SEED + 20 {
        if let Some(points) = last_strike_points(seed) {
            assert_eq!(points, 1, "seed {seed:#x}");
            checked += 1;
        }
    }
    assert!(checked >= 3, "only {checked} Ruptured dummies were struck");
}
