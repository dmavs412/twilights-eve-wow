//! Zealous Cleave hits the target plus the two nearest other enemies in a front cone: the sheet's
//! `max_targets`, `area_shape` and `area_center` read by the cast, played through the realm's
//! seam by a level 1 Vanguard who has pulled the scorpid camp south of the Crossroads onto
//! them. Each target rolls its own outcome, so the cast's targets are its hits and its misses.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;
use twilight_realm::callings;
use twilight_realm::{Realm, RealmCommand};

const SEED: u64 = 0x5EED_0102;
const LEVEL: u8 = 1;
/// Zealous Cleave's cost, in whole rage.
const RAGE: u32 = 20;
/// Melee reach, with room for a mob that stopped a step short.
const REACH: f32 = 5.0;

/// Zealous Cleave's spell id, from the Vanguard's rows.
fn cleave_id() -> u32 {
    callings::calling("vanguard")
        .unwrap()
        .skills
        .iter()
        .find(|s| s.key == "zealous_cleave")
        .expect("the Vanguard has Zealous Cleave")
        .spell_id
}

/// The camp's mobs as they stand after being pulled: where each stopped.
struct Pack {
    realm: Realm,
    view: View,
    me: u64,
    mobs: Vec<Mob>,
}

/// A level 1 Vanguard west of the scorpid camp, facing east, with the camp pulled onto them and
/// 20 rage from the blows. Every mob has stopped within reach.
fn pulled() -> Pack {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", &format!("level = {LEVEL}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"vanguard\"");
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    let west = [CAMP[0] - 25.0, CAMP[1], 0.0];
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: west,
        orientation: 0.0,
    });
    let streamed = view.run(&mut realm, 1.0);
    let mut mobs: Vec<Mob> = streamed
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: benilla_protocol::EntityKind::Unit,
                position,
                ..
            } if flat(*position, CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect();
    assert_eq!(mobs.len(), 4, "the scorpid camp streams in");
    for mob in &mobs {
        pull(&mut realm, &mut view, mob);
    }
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: west,
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
        let close = mobs.iter().all(|m| flat(m.position, west) <= REACH - 1.0);
        if close && view.rage(me) >= RAGE {
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
    panic!("the camp never came to hand with {RAGE} rage");
}

/// Whether `mob` stands in the half-circle east of `at`, the way the Vanguard faces.
fn east_of(at: [f32; 3], mob: &Mob) -> bool {
    mob.position[0] > at[0]
}

/// Casts Zealous Cleave at `target`: the guids it landed on or missed.
fn cleave(pack: &mut Pack, target: &Mob) -> Vec<u64> {
    let id = cleave_id();
    pack.realm.handle(cast(id, Some(target.guid)));
    let events = pack.view.drain(&mut pack.realm);
    assert_eq!(refusal(&events, id), None, "Zealous Cleave goes off");
    let mut landed: Vec<u64> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellGo {
                spell_id,
                hits,
                misses,
                ..
            } if *spell_id == id => Some(
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
fn with_four_in_reach_in_front_it_lands_on_exactly_three() {
    let mut pack = pulled();
    let at = [CAMP[0] - 25.0, CAMP[1], 0.0];
    for mob in &pack.mobs {
        assert!(east_of(at, mob), "the setup has every mob in front");
        assert!(flat(mob.position, at) <= REACH, "and in reach");
    }
    let primary = pack.mobs[0].clone();
    let landed = cleave(&mut pack, &primary);
    assert_eq!(landed.len(), 3, "the target and two more: {landed:?}");
    assert!(landed.contains(&primary.guid), "the target is one of them");
    // The two beside it are the nearest to the caster, not the farthest.
    let mut by_reach: Vec<&Mob> = pack
        .mobs
        .iter()
        .filter(|m| m.guid != primary.guid)
        .collect();
    by_reach.sort_by(|a, b| flat(a.position, at).total_cmp(&flat(b.position, at)));
    for near in &by_reach[..2] {
        assert!(landed.contains(&near.guid), "the nearer two are hit");
    }
    assert!(!landed.contains(&by_reach[2].guid), "the farthest is not");
}

#[test]
fn an_enemy_behind_the_caster_is_not_hit() {
    let mut pack = pulled();
    // Step into the pack, between its two eastern and its two western mobs, still facing east.
    pack.mobs
        .sort_by(|a, b| a.position[0].total_cmp(&b.position[0]));
    let at = [
        (pack.mobs[1].position[0] + pack.mobs[2].position[0]) / 2.0,
        pack.mobs.iter().map(|m| m.position[1]).sum::<f32>() / 4.0,
        pack.mobs[1].position[2],
    ];
    pack.realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: at,
        orientation: 0.0,
    });
    let (behind, front) = pack.mobs.split_at(2);
    for mob in &pack.mobs {
        assert!(flat(mob.position, at) <= REACH, "all four are in reach");
    }
    for mob in front {
        assert!(east_of(at, mob), "two stand in front");
    }
    for mob in behind {
        assert!(!east_of(at, mob), "two stand behind");
    }
    let (primary, front) = (front[0].clone(), front.to_vec());
    let landed = cleave(&mut pack, &primary);
    let mut want: Vec<u64> = front.iter().map(|m| m.guid).collect();
    want.sort_unstable();
    assert_eq!(landed, want, "only the two in front are hit");
}
