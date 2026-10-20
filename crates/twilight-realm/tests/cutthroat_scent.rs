//! The Cutthroat's Scent of Blood: a critical strike bleeds the target for a share of the crit's
//! own damage, over 8 s, a tick every 2 s. Played through the realm's seam by a level 10 Cutthroat
//! with the talent in its save, at fixed seeds.

mod common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::rules::talents::scaled_share;
use twilight_realm::{Realm, RealmCommand};

static SINISTER_STRIKE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "sinister_strike"));
/// The stock bleed (Pounce Bleed) the talent's rows apply.
const SCENT_BLEED: u32 = 9007;
/// `Talent.dbc` id of Scent of Blood (borrowed Improved Rupture).
static SCENT_OF_BLOOD: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("cutthroat", "scent_of_blood"));

/// A level 10 Cutthroat, fists only, with `talents` in its save, in the world at `seed`.
fn cutthroat(seed: u64, talents: &[(u32, u8)]) -> (Realm, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"cutthroat\"");
    let blob = drop_slot(&drop_slot(&blob, 15), 16);
    let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
    let blob = blob.replace(
        "money = 0\n",
        &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
    );
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    (realm, view)
}

fn camp_mob(events: &[SessionEvent]) -> Mob {
    events
        .iter()
        .find_map(|e| match e {
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
        .expect("a camp mob streams in")
}

fn bleed_ticks(events: &[SessionEvent]) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == SCENT_BLEED => Some(&l.ticks),
            _ => None,
        })
        .flatten()
        .filter_map(|t| match t {
            PeriodicTick::Damage { amount, .. } => Some(*amount),
            _ => None,
        })
        .collect()
}

/// The damage of a Sinister Strike crit that leaves the mob standing, and the bleed's ticks over
/// the 8 s after it, at the first seed that has one. The camp's mobs are frail: the bleed may kill
/// the mob before its last tick.
fn crit_and_bleed(talents: &[(u32, u8)]) -> Option<(u32, Vec<u32>)> {
    for seed in 0..200 {
        let (mut realm, mut view) = cutthroat(seed, talents);
        realm.handle(RealmCommand::Move {
            motion: Default::default(),
            position: [CAMP[0] + 40.0, CAMP[1], 0.0],
            orientation: std::f32::consts::PI,
        });
        let mob = camp_mob(&view.run(&mut realm, 1.0));
        stand_at(&mut realm, &mob, 2.0);
        realm.handle(cast(*SINISTER_STRIKE, Some(mob.guid)));
        let events = view.drain(&mut realm);
        let crit = events.iter().find_map(|e| match e {
            SessionEvent::SpellDamageLog(l)
                if l.spell_id == *SINISTER_STRIKE && l.hit_info != 0 =>
            {
                Some(l.damage)
            }
            _ => None,
        });
        let Some(damage) = crit else { continue };
        if view.health(mob.guid) == 0 {
            continue;
        }
        let events = view.run(&mut realm, 8.5);
        return Some((damage, bleed_ticks(&events)));
    }
    None
}

#[test]
fn a_hundred_damage_crit_at_thirty_percent_is_thirty_in_all() {
    assert_eq!(scaled_share(100, 30), 30);
}

#[test]
fn scent_of_blood_3_of_3_bleeds_thirty_percent_of_the_crit_over_four_ticks() {
    let (damage, ticks) = crit_and_bleed(&[(*SCENT_OF_BLOOD, 3)]).expect("a survivable crit");
    let total = scaled_share(damage, 30);
    // Four ticks, a tick every 2 s for 8 s, the remainder on the first ticks.
    let schedule: Vec<u32> = (0..4)
        .map(|i| total / 4 + u32::from(i < total % 4))
        .collect();
    assert_eq!(schedule.iter().sum::<u32>(), total);
    assert!(!ticks.is_empty(), "the crit of {damage} bleeds");
    assert_eq!(ticks, schedule[..ticks.len()], "{damage} -> {total}");
}

#[test]
fn without_the_talent_a_crit_leaves_no_scent_bleed() {
    let (_, ticks) = crit_and_bleed(&[]).expect("a survivable crit");
    assert!(ticks.is_empty(), "{ticks:?}");
}
