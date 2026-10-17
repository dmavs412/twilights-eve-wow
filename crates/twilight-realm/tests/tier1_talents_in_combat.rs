//! What the Vanguard's talents do in a fight, played through the realm's seam: the weapon
//! talents, Impale and Booming Voice, and the procs. The talents ride the save, as a character
//! who learned them would come back with.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;

// ── Weapons and the sheet ────────────────────────────────────────────────────────────────────

const TWO_HANDED_SPECIALIZATION: u32 = 136;
const AXE_SPECIALIZATION: u32 = 132;
const IMPALE: u32 = 662;
const BOOMING_VOICE: u32 = 158;
const UNBRIDLED_WRATH: u32 = 159;
const UNBRIDLED_WRATH_RAGE: u32 = 12964;
const SWORD_SPECIALIZATION: u32 = 123;
const MACE_SPECIALIZATION: u32 = 125;
const MACE_STUN: u32 = 5530;
const BLOOD_CRAZE: u32 = 661;
const BLOOD_CRAZE_3: u32 = 16491;
const ENRAGE: u32 = 155;
const ENRAGE_5: u32 = 14204;

const MORTAL_STRIKE: u32 = 12294;
const BATTLE_SHOUT: u32 = 6192;

const MAIN_HAND: u8 = 15;
const OFF_HAND: u8 = 16;
/// A two-handed axe and a one-handed mace, for levels 2 to 4.
const LARGE_AXE: u32 = 2491;
const FLANGED_MACE: u32 = 766;

/// The sheet's crit chance.
fn crit_chance(view: &View, me: u64) -> f32 {
    view.fields(me).player_crit_percentage().unwrap()
}

#[test]
fn weapon_specializations_add_to_the_damage_of_their_weapons() {
    // The same fight with and without the talent: the dice fall the same.
    let first = |seed: u64, talents: &[(u32, u8)], items: &[(u8, u32)]| {
        let (mut realm, me, mob, mut view) = tier1_at(10, seed, talents, items);
        stand_at(&mut realm, &mob, 2.0);
        first_swing(&mut realm, &mut view, me, &mob)
    };
    let axe = [(MAIN_HAND, LARGE_AXE)];
    let mut checked = 0;
    for seed in 0..20 {
        let (plain, plain_axe) = (first(seed, &[], &[]), first(seed, &[], &axe));
        if plain == 0 || plain_axe == 0 {
            continue;
        }
        // The Worn Shortsword is one-handed: nothing from the two-handed talent.
        assert_eq!(first(seed, &[(TWO_HANDED_SPECIALIZATION, 5)], &[]), plain);
        // A two-handed axe: 5% more at 5/5.
        assert_eq!(
            first(seed, &[(TWO_HANDED_SPECIALIZATION, 5)], &axe),
            plain_axe * 105 / 100
        );
        checked += 1;
    }
    assert!(
        checked >= 3,
        "only {checked} fights landed their first swings"
    );
}

#[test]
fn axe_specialization_raises_the_crit_chance_with_an_axe_only() {
    let crit = |talents: &[(u32, u8)], items: &[(u8, u32)]| {
        let (_, me, _, view) = tier1_at(10, SEED, talents, items);
        crit_chance(&view, me)
    };
    let axe = [(MAIN_HAND, LARGE_AXE)];
    let spec = [(AXE_SPECIALIZATION, 5)];
    assert!((crit(&spec, &axe) - crit(&[], &axe) - 5.0).abs() < 1e-4);
    assert!((crit(&spec, &[]) - crit(&[], &[])).abs() < 1e-4, "a sword");
}

#[test]
fn impale_raises_a_skills_crit() {
    // The same Mortal Strike with and without Impale, on fights that go the same.
    let strike = |seed: u64, impale: bool| {
        let talents: &[(u32, u8)] = if impale { &[(IMPALE, 2)] } else { &[] };
        let (mut realm, me, mob, mut view) = pulled(seed, 3, talents, &[]);
        while view.rage(me) < 30 {
            view.run(&mut realm, 0.1);
            if view.health(me) == 0 {
                return None;
            }
        }
        realm.handle(cast(MORTAL_STRIKE, Some(mob.guid)));
        let events = view.drain(&mut realm);
        events.iter().find_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == MORTAL_STRIKE => {
                Some((l.damage, l.hit_info & 0x2 != 0))
            }
            _ => None,
        })
    };
    for seed in 0..150 {
        let Some((plain, true)) = strike(seed, false) else {
            continue;
        };
        let (impaled, crit) = strike(seed, true).expect("the same strike");
        assert!(crit);
        // Twice the blow, and a fifth more of the whole crit.
        let blow = plain / 2;
        assert_eq!(impaled, blow * 2 * 120 / 100, "seed {seed}");
        return;
    }
    panic!("no Mortal Strike crit in 150 fights");
}

#[test]
fn booming_voice_makes_battle_shout_last_longer() {
    let (mut realm, _, _, mut view) = tanking(6, &[(BOOMING_VOICE, 5)], 10);
    realm.handle(cast(BATTLE_SHOUT, None));
    let events = view.drain(&mut realm);
    let lasts = events.iter().find_map(|e| match e {
        SessionEvent::AuraDuration { remaining_ms, .. } => Some(*remaining_ms),
        _ => None,
    });
    // Two minutes, and half again.
    assert_eq!(lasts, Some(180_000));
}

// ── Procs ────────────────────────────────────────────────────────────────────────────────────

/// `secs` of fighting the nearest mob, once for each of `seeds`: the warrior, the mob and the
/// events of each.
fn fights(
    seeds: std::ops::Range<u64>,
    level: u8,
    talents: &[(u32, u8)],
    items: &[(u8, u32)],
    secs: f32,
) -> Vec<(u64, Mob, Vec<SessionEvent>)> {
    seeds
        .map(|seed| {
            let (mut realm, me, mob, mut view) = tier1_at(level, seed, talents, items);
            stand_at(&mut realm, &mob, 2.0);
            realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
            let events = view.run(&mut realm, secs);
            (me, mob, events)
        })
        .collect()
}

fn energized(events: &[SessionEvent], spell_id: u32) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::SpellEnergizeLog(l) if l.spell_id == spell_id))
        .count()
}

#[test]
fn unbridled_wrath_gives_rage_for_landed_swings() {
    let procs = |talents: &[(u32, u8)]| -> usize {
        fights(0..20, 6, talents, &[], 6.0)
            .iter()
            .map(|(_, _, e)| energized(e, UNBRIDLED_WRATH_RAGE))
            .sum()
    };
    assert_eq!(procs(&[]), 0);
    assert!(
        procs(&[(UNBRIDLED_WRATH, 5)]) >= 3,
        "40% of the landed swings"
    );
}

#[test]
fn sword_specialization_swings_again_now_and_then() {
    // The extra swing comes at once: two of the warrior's swings in one tick.
    let doubled = |talents: &[(u32, u8)]| -> usize {
        let mut n = 0;
        for seed in 0..60 {
            let (mut realm, me, mob, mut view) = tier1_at(6, seed, talents, &[]);
            stand_at(&mut realm, &mob, 2.0);
            realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
            for _ in 0..80 {
                let swings = view
                    .run(&mut realm, 0.1)
                    .iter()
                    .filter(|e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == me))
                    .count();
                n += usize::from(swings > 1);
            }
        }
        n
    };
    assert_eq!(doubled(&[]), 0);
    assert!(doubled(&[(SWORD_SPECIALIZATION, 5)]) > 0);
}

#[test]
fn mace_specialization_stuns_now_and_then_with_a_mace() {
    let stunned = |items: &[(u8, u32)]| {
        fights(0..80, 6, &[(MACE_SPECIALIZATION, 5)], items, 8.0)
            .iter()
            .filter(|(_, mob, events)| {
                events.iter().any(|e| match e {
                    SessionEvent::ObjectValues { guid, fields } if *guid == mob.guid => {
                        fields.unit_aura_ids().any(|id| id == MACE_STUN)
                    }
                    _ => false,
                })
            })
            .count()
    };
    assert!(stunned(&[(MAIN_HAND, FLANGED_MACE)]) > 0);
    assert_eq!(stunned(&[]), 0, "not with a sword");
}

#[test]
fn a_crit_taken_brings_blood_craze_and_enrage() {
    for seed in 0..20 {
        let (mut realm, me, _, mut view) = pulled(seed, 9, &[(BLOOD_CRAZE, 3), (ENRAGE, 5)], &[]);
        for _ in 0..600 {
            let events = view.run(&mut realm, 0.1);
            if view.health(me) == 0 {
                break;
            }
            let critted = events.iter().any(|e| {
                matches!(e, SessionEvent::AttackerState(s) if s.victim == me && s.hit_info & 0x80 != 0)
            });
            if !critted {
                continue;
            }
            assert!(view.aura(me, ENRAGE_5).is_some(), "enraged");
            assert!(view.aura(me, BLOOD_CRAZE_3).is_some(), "regenerating");
            let later = view.run(&mut realm, 1.1);
            assert!(later.iter().any(|e| matches!(
                e,
                SessionEvent::PeriodicAuraLog(l) if l.spell_id == BLOOD_CRAZE_3
            )));
            return;
        }
    }
    panic!("no mob crit the warrior in twenty fights");
}

// ── Helpers ──────────────────────────────────────────────────────────────────────────────────

const SEED: u64 = 0x5EED_0019;

/// A human Vanguard of `level` with `talents` (`(talent id, rank)`) and `items` in its save,
/// in the world at the Crossroads; the nearest hostile mob, and the view.
fn tier1_at(
    level: u8,
    seed: u64,
    talents: &[(u32, u8)],
    items: &[(u8, u32)],
) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"vanguard\"");
    // The human's starting Worn Wooden Shield is set aside: a test that wants a shield gives one.
    blob = drop_slot(&blob, OFF_HAND);
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
    }
    for (slot, entry) in items {
        // In place of whatever the slot held.
        blob = drop_slot(&blob, *slot);
        blob.push_str(&format!(
            "\n[[items]]\nslot = {slot}\nentry = {entry}\ncount = 1\n"
        ));
    }
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let mut events = view.drain(&mut realm);
    // The world streams in over the first ticks.
    events.extend(view.run(&mut realm, 1.0));
    let mob = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: benilla_protocol::EntityKind::Unit,
                position,
                fields,
                ..
            } if fields.unit_npc_flags() == 0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .min_by(|a, b| flat(a.position, CROSSROADS_XY).total_cmp(&flat(b.position, CROSSROADS_XY)))
        .expect("a mob near the Crossroads");
    (realm, guid, mob, view)
}

/// [`tanking_seeded`] on the usual dice.
fn tanking(level: u8, talents: &[(u32, u8)], rage: u32) -> (Realm, u64, Mob, View) {
    tanking_seeded(SEED, level, talents, rage)
}

/// [`tanking_with`] and no items changed.
fn tanking_seeded(
    seed: u64,
    level: u8,
    talents: &[(u32, u8)],
    rage: u32,
) -> (Realm, u64, Mob, View) {
    tanking_with(seed, level, talents, &[], rage)
}

/// A Vanguard of `level` with `talents` and `items` that pulled the nearest camp without
/// swinging back, and took its blows until it had `rage`.
fn tanking_with(
    seed: u64,
    level: u8,
    talents: &[(u32, u8)],
    items: &[(u8, u32)],
    rage: u32,
) -> (Realm, u64, Mob, View) {
    let (mut realm, me, mob, mut view) = pulled(seed, level, talents, items);
    for _ in 0..1800 {
        view.run(&mut realm, 0.1);
        assert!(view.health(me) > 0, "died waiting for {rage} rage");
        if view.rage(me) >= rage {
            return (realm, me, mob, view);
        }
    }
    panic!("two minutes of blows built no {rage} rage");
}

/// A Vanguard of `level` with `talents` and `items` that pulled the nearest camp south of
/// the Crossroads and stopped swinging: its mobs beat on the warrior from now on.
fn pulled(
    seed: u64,
    level: u8,
    talents: &[(u32, u8)],
    items: &[(u8, u32)],
) -> (Realm, u64, Mob, View) {
    let (mut realm, me, _, mut view) = tier1_at(level, seed, talents, items);
    // Out of reach of the four level 3 scorpids south of the Crossroads, then each of them in
    // turn: neutral, they come only when attacked themselves.
    realm.handle(RealmCommand::Move {
        position: [CAMP[0] + 40.0, CAMP[1], 0.0],
        orientation: std::f32::consts::PI,
    });
    let streamed = view.run(&mut realm, 1.0);
    let camp: Vec<Mob> = streamed
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
    let mob = camp.first().cloned().expect("the scorpid camp streams in");
    stand_at(&mut realm, &mob, 2.0);
    for other in camp.iter().rev() {
        realm.handle(RealmCommand::AttackSwing { guid: other.guid });
    }
    realm.handle(RealmCommand::AttackStop);
    (realm, me, mob, view)
}

/// Swings at `mob` and returns the first white swing's damage, 0 for one that missed.
fn first_swing(realm: &mut Realm, view: &mut View, me: u64, mob: &Mob) -> u32 {
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    for _ in 0..30 {
        for e in view.run(realm, 0.1) {
            if let SessionEvent::AttackerState(s) = e {
                if s.attacker == me {
                    return s.damage;
                }
            }
        }
    }
    panic!("no swing in three seconds");
}
