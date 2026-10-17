//! Savage Strikes, the Recruit tree's crit talent: more crit chance for everything the warrior
//! does, and more crit damage on top of every crit. Played through the realm's seam by a level
//! 10 Basic Warrior.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const SAVAGE_STRIKES: u32 = 272;
const CHARGE: u32 = 100;
const FIRE_BLAST: u32 = 2136;

/// A level 10 human warrior with `talents` in its save, in the world, the mob near it.
fn warrior(talents: &[(u32, u8)], seed: u64) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n");
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
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
    (realm, guid, mob, view)
}

/// Each Fire Blast's `(was_crit, damage)` on `seed`'s dice.
fn fire_blast(talents: &[(u32, u8)], seed: u64) -> Vec<(bool, u32)> {
    let (mut realm, _, mob, mut view) = warrior(talents, seed);
    stand_at(&mut realm, &mob, 15.0);
    realm.handle(cast(CHARGE, Some(mob.guid)));
    view.drain(&mut realm);
    realm.handle(cast(FIRE_BLAST, Some(mob.guid)));
    view.drain(&mut realm)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == FIRE_BLAST => {
                Some((l.hit_info != 0, l.damage))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn savage_strikes_raises_the_sheets_crit_chance_by_its_percent() {
    let (_, plain, _, mut p_view) = warrior(&[], 1);
    let (_, savage, _, mut s_view) = warrior(&[(SAVAGE_STRIKES, 2)], 1);
    let before = p_view.fields(plain).player_crit_percentage().unwrap();
    let after = s_view.fields(savage).player_crit_percentage().unwrap();
    assert!((after - before - 6.0).abs() < 1e-3, "{before} -> {after}");
}

#[test]
fn savage_strikes_crits_fire_blast_more_often_and_harder() {
    let (mut base_crits, mut savage_crits, mut total) = (0, 0, 0);
    for seed in 0..400 {
        for (crit, damage) in fire_blast(&[], seed) {
            total += 1;
            base_crits += u32::from(crit);
            assert_eq!(damage, if crit { 67 } else { 45 }, "150% of 45");
        }
        for (crit, damage) in fire_blast(&[(SAVAGE_STRIKES, 2)], seed) {
            savage_crits += u32::from(crit);
            // 150% of 45, times 1.3: 87.75, rounded down.
            assert_eq!(damage, if crit { 87 } else { 45 });
        }
    }
    assert!(total > 300, "{total}");
    assert!(savage_crits > base_crits, "{savage_crits} vs {base_crits}");
}
