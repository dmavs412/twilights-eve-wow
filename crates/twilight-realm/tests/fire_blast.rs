//! Fire Blast, the first non-physical `school_damage`: played through the realm's seam by a level
//! 10 Basic Warrior at a level 3 mob. Fire ignores armour, is never dodged, parried or blocked,
//! misses only on the level-based chance, and crits for 150%.
//!
//! Level 10 because a Charge then gives the 10 rage Fire Blast costs; its damage is the row's
//! 20 + 2.5 per level, 45.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;

const CHARGE: u32 = 100;
const FIRE_BLAST: u32 = 2136;
const FIRE_SCHOOL: u8 = 2;
const SPELL_MISS_MISS: u8 = 1;

/// One level 10 Basic Warrior's Fire Blast at the nearest mob, on `seed`'s dice: the events of
/// the cast.
fn fire_blast(seed: u64) -> Vec<SessionEvent> {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n");
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
                kind: benilla_protocol::EntityKind::Unit,
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
    assert_eq!(refusal(&view.drain(&mut realm), CHARGE), None, "the charge");
    realm.handle(cast(FIRE_BLAST, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, FIRE_BLAST), None, "fire blast is cast");
    events
}

#[test]
fn fire_blast_ignores_armour_is_never_avoided_and_crits_for_150_percent() {
    let (mut hits, mut crits) = (Vec::new(), Vec::new());
    for seed in 0..300 {
        for e in &fire_blast(seed) {
            match e {
                SessionEvent::SpellDamageLog(l) if l.spell_id == FIRE_BLAST => {
                    assert_eq!(l.school, FIRE_SCHOOL, "shown as fire");
                    if l.hit_info == 0 {
                        hits.push(l.damage);
                    } else {
                        crits.push(l.damage);
                    }
                }
                SessionEvent::SpellGo {
                    spell_id: FIRE_BLAST,
                    misses,
                    ..
                } => {
                    for (_, why) in misses {
                        assert_eq!(*why, SPELL_MISS_MISS, "not a dodge, parry or block");
                    }
                }
                _ => {}
            }
        }
    }
    assert!(!hits.is_empty() && !crits.is_empty(), "{hits:?} {crits:?}");
    // 20 + 2.5 x 10 whatever the mob's armour; a crit is 150% of it.
    assert!(hits.iter().all(|d| *d == 45), "{hits:?}");
    assert!(crits.iter().all(|d| *d == 67), "{crits:?}");
}
