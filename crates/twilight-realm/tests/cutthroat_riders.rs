//! The Cutthroat's Sinister Strike in a crowd: it strikes its target alone, and gives one combo
//! point on a target bleeding from the caster's Rupture. Played through the realm's seam by a
//! level 10 Cutthroat, at fixed seeds.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const FIRST_SEED: u64 = 0x5EED_0112;
/// Melee reach, with room for a mob that stopped a step short.
const REACH: f32 = 5.0;

static SINISTER_STRIKE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "sinister_strike"));
static RUPTURE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "rupture"));

/// A level 10 Cutthroat in the world at `seed`, `unarmed` to make the fight last: the realm, its
/// guid and what the client has seen.
fn cutthroat(seed: u64, unarmed: bool) -> (Realm, u64, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"cutthroat\"");
    if unarmed {
        blob = drop_slot(&drop_slot(&blob, 15), 16);
    }
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    (realm, me, view)
}

/// The camp's mobs the client has been told of in `events`.
fn camp_mobs(events: &[SessionEvent]) -> Vec<Mob> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } if flat(*position, CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect()
}

/// The scorpid camp pulled onto a Cutthroat west of it, facing east.
struct Pack {
    realm: Realm,
    view: View,
    me: u64,
    mobs: Vec<Mob>,
}

const WEST: [f32; 3] = [CAMP[0] - 25.0, CAMP[1], 0.0];

/// A Cutthroat at `seed` with the camp pulled onto them: every mob has stopped within reach.
fn pulled(seed: u64) -> Pack {
    let (mut realm, me, mut view) = cutthroat(seed, false);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: WEST,
        orientation: 0.0,
    });
    let streamed = view.run(&mut realm, 1.0);
    let mut mobs = camp_mobs(&streamed);
    assert_eq!(mobs.len(), 4, "the scorpid camp streams in");
    for mob in &mobs {
        pull(&mut realm, &mut view, mob);
    }
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: WEST,
        orientation: 0.0,
    });
    for _ in 0..600 {
        for e in view.run(&mut realm, 0.1) {
            if let SessionEvent::MonsterMove { guid, path, .. } = e {
                if let (Some(mob), Some(end)) =
                    (mobs.iter_mut().find(|m| m.guid == guid), path.last())
                {
                    mob.position = *end;
                }
            }
        }
        assert!(view.health(me) > 0, "died waiting for the camp");
        if mobs.iter().all(|m| flat(m.position, WEST) <= REACH - 1.0) {
            // The paths end in reach; the mobs have still to walk them.
            view.run(&mut realm, 10.0);
            assert!(view.health(me) > 0, "died waiting for the camp");
            return Pack {
                realm,
                view,
                me,
                mobs,
            };
        }
    }
    panic!("the camp never came to hand");
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

/// The combo points the client shows, and the guid they are on.
fn combo(view: &View, me: u64) -> (u8, u64) {
    let f = view.fields(me);
    (
        f.player_combo_points().unwrap_or(0),
        f.player_combo_target(),
    )
}

#[test]
fn sinister_strike_hits_one_dummy() {
    for seed in FIRST_SEED..FIRST_SEED + 20 {
        let mut pack = pulled(seed);
        let primary = pack.mobs[0].clone();
        assert_eq!(strike(&mut pack, &primary), [primary.guid]);
    }
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
