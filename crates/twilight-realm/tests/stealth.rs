//! Stealth: a real stealth state. A level 10 Cutthroat near the Crossroads is noticed only inside
//! the short stealth range, its first blow from stealth is a critical strike that breaks it, a
//! blow taken breaks it, and it cannot be cast in combat.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const SEED: u64 = 0x5EED_0109;
const LEVEL: u8 = 10;

const STEALTH: u32 = 1784;
const SINISTER_STRIKE: u32 = 1752;
/// `SPELL_FAILED_AFFECTING_COMBAT`.
const AFFECTING_COMBAT: u8 = 0x00;

/// A level 10 Cutthroat in the world: the realm, its guid, the hostile mob near it, and what the
/// client has seen.
fn cutthroat(seed: u64) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {LEVEL}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"cutthroat\"");
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let entry = view.drain(&mut realm);
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
    (realm, guid, mob, view)
}

fn stealthed(view: &View, me: u64) -> bool {
    view.aura(me, STEALTH).is_some()
}

/// Whether the mob went for the player in `events`.
fn aggroed(events: &[SessionEvent], mob: &Mob, me: u64) -> bool {
    events.iter().any(|e| match e {
        SessionEvent::AttackStart { attacker, victim } => *attacker == mob.guid && *victim == me,
        _ => false,
    })
}

/// Casts Stealth out of combat; the aura is up.
fn go_stealthed(realm: &mut Realm, view: &mut View, me: u64) {
    realm.handle(cast(STEALTH, None));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, STEALTH), None, "the cast");
    assert!(stealthed(view, me), "the aura is up");
}

/// Moves the player to `yards` from the mob, ticks a second, and says whether the mob came.
fn approach(realm: &mut Realm, view: &mut View, mob: &Mob, me: u64, yards: f32) -> bool {
    stand_at(realm, mob, yards);
    aggroed(&view.run(realm, 1.0), mob, me)
}

#[test]
fn a_stealthed_cutthroat_passes_a_hostile_mob_unnoticed_until_close() {
    // Without stealth the mob notices at 4 yd (5 yd at this level); the control.
    let (mut realm, me, mob, mut view) = cutthroat(SEED);
    assert!(
        approach(&mut realm, &mut view, &mob, me, 4.0),
        "noticed at 4 yd"
    );

    let (mut realm, me, mob, mut view) = cutthroat(SEED);
    go_stealthed(&mut realm, &mut view, me);
    assert!(!approach(&mut realm, &mut view, &mob, me, 6.0), "6 yd");
    assert!(!approach(&mut realm, &mut view, &mob, me, 4.0), "4 yd");
    assert!(stealthed(&view, me), "walking past does not break it");
    assert!(
        approach(&mut realm, &mut view, &mob, me, 2.0),
        "noticed at 2 yd"
    );
}

#[test]
fn stealth_lasts_until_it_breaks() {
    let (mut realm, me, _, mut view) = cutthroat(SEED);
    go_stealthed(&mut realm, &mut view, me);
    view.run(&mut realm, 60.0);
    assert!(stealthed(&view, me), "still up after a minute");
}

#[test]
fn the_first_sinister_strike_from_stealth_crits_on_every_seed_and_breaks_it() {
    for seed in 0..40 {
        let (mut realm, me, mob, mut view) = cutthroat(SEED + seed);
        go_stealthed(&mut realm, &mut view, me);
        view.run(&mut realm, 1.0);
        // Next to it, the move and the strike in one go: the mob has not had a tick to notice.
        stand_at(&mut realm, &mob, 2.0);
        realm.handle(cast(SINISTER_STRIKE, Some(mob.guid)));
        let events = view.drain(&mut realm);
        assert_eq!(
            refusal(&events, SINISTER_STRIKE),
            None,
            "seed {seed}: the cast"
        );
        let hits: Vec<bool> = events
            .iter()
            .filter_map(|e| match e {
                SessionEvent::SpellDamageLog(l) if l.spell_id == SINISTER_STRIKE => {
                    Some(l.hit_info != 0)
                }
                _ => None,
            })
            .collect();
        assert_eq!(hits, [true], "seed {seed}: one blow, and a crit");
        assert!(
            !stealthed(&view, me),
            "seed {seed}: the strike broke stealth"
        );
    }
}

#[test]
fn the_second_strike_is_rolled_on_the_table() {
    // With no stealth up there is no guaranteed crit: over many seeds some blows do not crit.
    let mut crits = 0;
    let mut blows = 0;
    for seed in 0..40 {
        let (mut realm, _, mob, mut view) = cutthroat(SEED + seed);
        stand_at(&mut realm, &mob, 2.0);
        realm.handle(cast(SINISTER_STRIKE, Some(mob.guid)));
        for e in view.drain(&mut realm) {
            if let SessionEvent::SpellDamageLog(l) = e {
                if l.spell_id == SINISTER_STRIKE {
                    blows += 1;
                    crits += u32::from(l.hit_info != 0);
                }
            }
        }
    }
    assert!(blows > 0 && crits < blows, "{crits} of {blows}");
}

#[test]
fn a_white_swing_from_stealth_crits_and_breaks_it() {
    let (mut realm, me, mob, mut view) = cutthroat(SEED);
    go_stealthed(&mut realm, &mut view, me);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = view.drain(&mut realm);
    let events = [events, view.run(&mut realm, 3.0)].concat();
    let first = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::AttackerState(s) if s.attacker == me => Some(s.hit_info),
            _ => None,
        })
        .expect("a swing");
    assert_ne!(first & 0x80, 0, "a critical hit");
    assert!(!stealthed(&view, me));
}

#[test]
fn taking_damage_breaks_stealth() {
    let (mut realm, me, mob, mut view) = cutthroat(SEED);
    go_stealthed(&mut realm, &mut view, me);
    // Within the mob's reach the stealth is seen through; it fights, and its first blow lands.
    stand_at(&mut realm, &mob, 2.0);
    let mut hurt = false;
    for _ in 0..200 {
        let events = view.run(&mut realm, 0.1);
        hurt = events.iter().any(|e| match e {
            SessionEvent::AttackerState(s) => s.victim == me && s.damage > 0,
            _ => false,
        });
        if hurt {
            break;
        }
        assert!(stealthed(&view, me), "stealth holds until a blow lands");
    }
    assert!(hurt, "the mob lands a blow in twenty seconds");
    assert!(!stealthed(&view, me), "the blow broke it");
}

#[test]
fn stealth_cannot_be_cast_in_combat() {
    let (mut realm, me, mob, mut view) = cutthroat(SEED);
    assert!(approach(&mut realm, &mut view, &mob, me, 2.0), "pulled");
    realm.handle(cast(STEALTH, None));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, STEALTH), Some(AFFECTING_COMBAT));
    assert!(!stealthed(&view, me));
}
