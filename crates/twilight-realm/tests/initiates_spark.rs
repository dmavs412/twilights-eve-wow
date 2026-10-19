//! Initiate's Spark, the Recruit tree's Fire Blast talent: each rank makes Fire Blast hit harder
//! and gives rage back. Played through the realm's seam by a level 10 Basic Warrior.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const INITIATES_SPARK: u32 = 31;
const CHARGE: u32 = 100;
const FIRE_BLAST: u32 = 2136;

/// Fire Blast's `(damage, rage left after the cast)` for a warrior with `rank` of the talent.
/// The dice are seeded; only non-crit casts are compared, so the damage is the row's plain number.
fn fire_blast(rank: u8) -> (u32, u32) {
    for seed in 0..400 {
        let mut maker = realm();
        assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
        let guid = roster(&mut maker)[0].guid;
        let mut blob = maker
            .export_character(guid)
            .unwrap()
            .replace("level = 1\n", "level = 10\n");
        if rank > 0 {
            blob = blob.replace(
                "money = 0\n",
                &format!("money = 0\ntalents = [[{INITIATES_SPARK}, {rank}]]\n"),
            );
        }
        let mut realm = Realm::with_seed(seed);
        realm.import_character(&blob).unwrap();
        realm.handle(RealmCommand::EnterWorld { guid });
        realm.tick(0.0, &FlatGround);
        let mut view = View::default();
        let mob = view
            .drain(&mut realm)
            .iter()
            .find_map(|e| match e {
                SessionEvent::ObjectCreate {
                    guid,
                    kind: EntityKind::Unit,
                    position,
                    ..
                } => Some(Mob {
                    guid: *guid,
                    position: *position,
                }),
                _ => None,
            })
            .expect("a mob near the Crossroads");
        stand_at(&mut realm, &mob, 15.0);
        realm.handle(cast(CHARGE, Some(mob.guid)));
        view.drain(&mut realm);
        realm.handle(cast(FIRE_BLAST, Some(mob.guid)));
        let events = view.drain(&mut realm);
        let hit = events.iter().find_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == FIRE_BLAST && l.hit_info == 0 => {
                Some(l.damage)
            }
            _ => None,
        });
        if let Some(damage) = hit {
            return (damage, view.rage(guid));
        }
    }
    panic!("no plain Fire Blast hit in 400 seeds");
}

#[test]
fn each_rank_of_initiates_spark_raises_fire_blasts_damage_and_rage() {
    // The row's 45 damage, plus 10% per rank (rounded); the Charge's 10 rage is spent, the talent gives
    // 2 per rank back.
    assert_eq!(fire_blast(0), (45, 0));
    assert_eq!(fire_blast(1), (50, 2));
    assert_eq!(fire_blast(2), (54, 4));
    assert_eq!(fire_blast(3), (59, 6));
}
