//! The stat kinds Cutthroat talents and Evasion use: dodge, armor penetration and max Energy,
//! and run speed as a talent. Played through the realm's seam by a level 10 Cutthroat with the
//! talents in its save, near the Crossroads.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const SEED: u64 = 0x5EED_0107;
const LEVEL: u8 = 10;

/// `Talent.dbc` ids of the Cutthroat talents under test.
static REFLEXIVE_STRIKE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("cutthroat", "reflexive_strike"));
static PREDATORS_STEP: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("cutthroat", "predators_step"));
static SERRATED_STRIKES: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("cutthroat", "serrated_strikes"));
static EVASION: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "evasion"));
/// A player's run speed, yd/s.
const RUN: f32 = 7.0;
const ENERGY: u8 = 3;

/// A level 10 Cutthroat with `talents` in its save, in the world: the realm, its guid, a mob
/// near it, what the client has seen, and the run speed the entry burst gave.
fn cutthroat(talents: &[(u32, u8)]) -> (Realm, u64, Mob, View, f32) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {LEVEL}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"cutthroat\"");
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
    }
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let entry = view.drain(&mut realm);
    let run = entry
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g,
                speeds: Some(s),
                ..
            } if *g == guid => Some(s.run),
            _ => None,
        })
        .expect("the player's create carries speeds");
    let mob = entry
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
    (realm, guid, mob, view, run)
}

fn dodge(view: &View, me: u64) -> f32 {
    view.fields(me).player_dodge_percentage().unwrap()
}

fn crit(view: &View, me: u64) -> f32 {
    view.fields(me).player_crit_percentage().unwrap()
}

#[test]
fn reflexive_strike_adds_dodge_and_crit_to_the_sheet() {
    let (_, me, _, bare, _) = cutthroat(&[]);
    let (_, me2, _, trained, _) = cutthroat(&[(*REFLEXIVE_STRIKE, 5)]);
    assert!(
        (dodge(&trained, me2) - dodge(&bare, me) - 5.0).abs() < 1e-3,
        "dodge {} -> {}",
        dodge(&bare, me),
        dodge(&trained, me2)
    );
    assert!(
        (crit(&trained, me2) - crit(&bare, me) - 5.0).abs() < 1e-3,
        "crit {} -> {}",
        crit(&bare, me),
        crit(&trained, me2)
    );
}

#[test]
fn evasion_adds_fifty_percent_dodge_for_fifteen_seconds() {
    let (mut realm, me, _, mut view, _) = cutthroat(&[]);
    let base = dodge(&view, me);
    realm.handle(cast(*EVASION, None));
    assert_eq!(refusal(&view.drain(&mut realm), *EVASION), None, "the cast");
    assert!((dodge(&view, me) - base - 50.0).abs() < 1e-3, "on");
    view.run(&mut realm, 10.0);
    assert!(
        (dodge(&view, me) - base - 50.0).abs() < 1e-3,
        "still on at 10 s"
    );
    view.run(&mut realm, 6.0);
    assert!((dodge(&view, me) - base).abs() < 1e-3, "off after 16 s");
}

#[test]
fn predators_step_raises_run_speed_and_max_energy() {
    let (_, me, _, bare, run_bare) = cutthroat(&[]);
    let (_, me2, _, trained, run) = cutthroat(&[(*PREDATORS_STEP, 5)]);
    assert!((run_bare - RUN).abs() < 1e-3, "{run_bare}");
    assert!((run - RUN * 1.15).abs() < 1e-3, "{run}");
    assert_eq!(bare.fields(me).unit_max_power(ENERGY), Some(100));
    assert_eq!(trained.fields(me2).unit_max_power(ENERGY), Some(125));
}

/// The first white swing's damage at the mob, with `talents`, from the same seed; 0 for a miss.
fn first_swing(talents: &[(u32, u8)]) -> u32 {
    let (mut realm, me, mob, mut view, _) = cutthroat(talents);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    for _ in 0..40 {
        for e in view.run(&mut realm, 0.1) {
            if let SessionEvent::AttackerState(s) = e {
                if s.attacker == me {
                    return s.damage;
                }
            }
        }
    }
    panic!("no swing in four seconds");
}

#[test]
fn serrated_strikes_ignore_armor() {
    let plain = first_swing(&[]);
    let serrated = first_swing(&[(*SERRATED_STRIKES, 5)]);
    assert!(plain > 0, "the seed lands a swing");
    assert!(serrated > plain, "{plain} -> {serrated}");
}
