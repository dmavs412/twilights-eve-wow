//! What the Tier 1 Warrior's talents do in a fight, played through the realm's seam: the active
//! talents cast as skills once learned, the weapon and shield talents, and the procs. The
//! talents ride the save, as a character who learned them would come back with.

mod common;

use benilla_protocol::{ObjectFields, SessionEvent};
use common::*;

// `Talent.dbc` ids (`content/callings.toml`), and the spells their effects go under.
const CONCUSSION_BLOW: u32 = 152;
const CONCUSSION_BLOW_SPELL: u32 = 12809;
const PIERCING_HOWL: u32 = 160;
const PIERCING_HOWL_SPELL: u32 = 12323;
const LAST_STAND: u32 = 153;
const LAST_STAND_SPELL: u32 = 12975;
const LAST_STAND_AURA: u32 = 12976;
const DEATH_WISH: u32 = 165;
const DEATH_WISH_SPELL: u32 = 12328;
const SWEEPING_STRIKES: u32 = 133;
const SWEEPING_STRIKES_SPELL: u32 = 12292;
const SWEEPING_STRIKE_HIT: u32 = 12723;

/// `SPELL_FAILED_NOT_KNOWN`.
const NOT_KNOWN: u8 = 0x38;

#[test]
fn an_active_talent_is_cast_only_once_learned() {
    let (mut realm, _, mob, mut view) = tanking(2, &[], 15);
    realm.handle(cast(CONCUSSION_BLOW_SPELL, Some(mob.guid)));
    assert_eq!(
        refusal(&view.drain(&mut realm), CONCUSSION_BLOW_SPELL),
        Some(NOT_KNOWN)
    );
}

#[test]
fn concussion_blow_stuns_its_target() {
    for seed in 0..20 {
        let (mut realm, me, mob, mut view) = tanking_seeded(seed, 2, &[(CONCUSSION_BLOW, 1)], 15);
        let rage = view.rage(me);
        realm.handle(cast(CONCUSSION_BLOW_SPELL, Some(mob.guid)));
        let events = view.drain(&mut realm);
        assert_eq!(refusal(&events, CONCUSSION_BLOW_SPELL), None);
        assert_eq!(view.rage(me), rage - 15, "15 rage");
        if !go_hits(&events, CONCUSSION_BLOW_SPELL).is_some_and(|h| h.contains(&mob.guid)) {
            continue; // Missed or dodged: no stun, try another fight.
        }
        assert!(
            view.aura(mob.guid, CONCUSSION_BLOW_SPELL).is_some(),
            "stunned"
        );
        // Five seconds of no blows, then it is gone.
        let stunned = view.run(&mut realm, 4.8);
        assert!(!swings(&stunned, mob.guid), "a stunned mob does not swing");
        view.run(&mut realm, 0.4);
        assert!(view.aura(mob.guid, CONCUSSION_BLOW_SPELL).is_none());
        // A 45 s cooldown.
        realm.handle(cast(CONCUSSION_BLOW_SPELL, Some(mob.guid)));
        assert!(refusal(&view.drain(&mut realm), CONCUSSION_BLOW_SPELL).is_some());
        return;
    }
    panic!("no Concussion Blow landed in twenty fights");
}

#[test]
fn piercing_howl_slows_every_enemy_around() {
    let (mut realm, _, mob, mut view) = tanking(2, &[(PIERCING_HOWL, 1)], 10);
    realm.handle(cast(PIERCING_HOWL_SPELL, None));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, PIERCING_HOWL_SPELL), None);
    let hits = go_hits(&events, PIERCING_HOWL_SPELL).expect("it went off");
    assert!(hits.contains(&mob.guid), "{hits:?}");
    assert!(view.aura(mob.guid, PIERCING_HOWL_SPELL).is_some());
    view.run(&mut realm, 6.2);
    assert!(
        view.aura(mob.guid, PIERCING_HOWL_SPELL).is_none(),
        "six seconds"
    );
}

#[test]
fn last_stand_lends_thirty_percent_more_health_for_twenty_seconds() {
    let (mut realm, me, _, mut view) = tier1_at(10, SEED, &[(LAST_STAND, 1)], &[]);
    let (max, health) = (view.max_health(me), view.health(me));
    realm.handle(cast(LAST_STAND_SPELL, None));
    view.drain(&mut realm);
    let more = max * 30 / 100;
    assert_eq!(view.max_health(me), max + more);
    assert_eq!(view.health(me), health + more);
    assert!(view.aura(me, LAST_STAND_AURA).is_some());
    view.run(&mut realm, 20.2);
    assert_eq!(view.max_health(me), max);
    assert!(view.health(me) <= max);
    assert!(view.aura(me, LAST_STAND_AURA).is_none());
}

#[test]
fn death_wish_costs_armour_while_it_lasts() {
    let (mut realm, me, _, mut view) = tanking(2, &[(DEATH_WISH, 1)], 10);
    let armor = view.fields(me).unit_resistance(0).unwrap();
    realm.handle(cast(DEATH_WISH_SPELL, None));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, DEATH_WISH_SPELL), None);
    assert!(view.aura(me, DEATH_WISH_SPELL).is_some());
    assert_eq!(
        view.fields(me).unit_resistance(0).unwrap(),
        armor - armor * 20 / 100
    );
}

#[test]
fn death_wish_adds_a_fifth_to_the_damage_dealt() {
    // The same fight twice, with and without Death Wish: the dice fall the same.
    let first = |seed: u64, wish: bool| {
        let (mut realm, me, mob, mut view) = tier1_at(10, seed, &[(DEATH_WISH, 1)], &[]);
        if wish {
            // Death Wish's rage, from a Charge.
            stand_at(&mut realm, &mob, 15.0);
            realm.handle(cast(CHARGE, Some(mob.guid)));
            view.drain(&mut realm);
            realm.handle(cast(DEATH_WISH_SPELL, None));
            assert_eq!(refusal(&view.drain(&mut realm), DEATH_WISH_SPELL), None);
        }
        stand_at(&mut realm, &mob, 2.0);
        first_swing(&mut realm, &mut view, me, &mob)
    };
    for seed in 0..20 {
        let (plain, wished) = (first(seed, false), first(seed, true));
        if plain == 0 {
            continue; // Missed or avoided.
        }
        assert_eq!(wished, plain * 120 / 100, "{plain} -> {wished}");
        return;
    }
    panic!("no first swing landed in twenty fights");
}

#[test]
fn sweeping_strikes_carries_a_landed_blow_to_a_second_enemy() {
    let (mut realm, me, a, mut view) = tanking(2, &[(SWEEPING_STRIKES, 1)], 30);
    realm.handle(cast(SWEEPING_STRIKES_SPELL, None));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, SWEEPING_STRIKES_SPELL), None);
    assert!(view.aura(me, SWEEPING_STRIKES_SPELL).is_some());
    realm.handle(RealmCommand::AttackSwing { guid: a.guid });
    let mut events = Vec::new();
    for _ in 0..200 {
        events.extend(view.run(&mut realm, 0.1));
        if !sweeps(&events).is_empty() {
            break;
        }
    }
    let sweeps = sweeps(&events);
    let (victim, damage) = *sweeps.first().expect("a blow swept to another mob");
    assert_ne!(victim, a.guid);
    assert_ne!(victim, me);
    // As much as the blow it came with.
    assert!(events.iter().any(
        |e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == me && s.damage == damage)
    ));
}

fn sweeps(events: &[SessionEvent]) -> Vec<(u64, u32)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == SWEEPING_STRIKE_HIT => {
                Some((l.target, l.damage))
            }
            _ => None,
        })
        .collect()
}

// ── Weapons, shields and the sheet ───────────────────────────────────────────────────────────

const DEFLECTION: u32 = 130;
const TWO_HANDED_SPECIALIZATION: u32 = 136;
const ONE_HANDED_SPECIALIZATION: u32 = 702;
const AXE_SPECIALIZATION: u32 = 132;
const SHIELD_SPECIALIZATION: u32 = 1601;
const SHIELD_SPECIALIZATION_RAGE: u32 = 23602;
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
/// A two-handed axe, a one-handed mace and a shield, for levels 2 to 4.
const LARGE_AXE: u32 = 2491;
const FLANGED_MACE: u32 = 766;
const WARRIORS_SHIELD: u32 = 1438;

/// The sheet's crit, parry and block chances.
fn sheet(view: &View, me: u64) -> (f32, f32, f32) {
    let f = view.fields(me);
    (
        f.player_crit_percentage().unwrap(),
        f.player_parry_percentage().unwrap(),
        f.player_block_percentage().unwrap(),
    )
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
        // The Worn Shortsword is one-handed: 10% more at 5/5, and nothing from the two-handed one.
        assert_eq!(
            first(seed, &[(ONE_HANDED_SPECIALIZATION, 5)], &[]),
            plain * 110 / 100
        );
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
        sheet(&view, me).0
    };
    let axe = [(MAIN_HAND, LARGE_AXE)];
    let spec = [(AXE_SPECIALIZATION, 5)];
    assert!((crit(&spec, &axe) - crit(&[], &axe) - 5.0).abs() < 1e-4);
    assert!((crit(&spec, &[]) - crit(&[], &[])).abs() < 1e-4, "a sword");
}

#[test]
fn deflection_gives_a_parry_chance_with_a_weapon_in_hand() {
    let (_, me, _, view) = tier1_at(10, SEED, &[], &[]);
    assert_eq!(sheet(&view, me).1, 0.0, "no Parry skill");
    let (_, me, _, view) = tier1_at(10, SEED, &[(DEFLECTION, 5)], &[]);
    assert!((sheet(&view, me).1 - 5.0).abs() < 1e-4);
}

#[test]
fn a_shield_blocks_and_shield_specialization_blocks_more_and_gives_rage() {
    let shield = [(OFF_HAND, WARRIORS_SHIELD)];
    let (_, me, _, view) = tier1_at(10, SEED, &[], &[]);
    assert_eq!(sheet(&view, me).2, 0.0, "no shield");
    let (_, me, _, view) = tier1_at(10, SEED, &[], &shield);
    assert!(
        (sheet(&view, me).2 - 5.0).abs() < 1e-4,
        "5% at the level's Defense"
    );
    let (_, me, _, view) = tier1_at(10, SEED, &[(SHIELD_SPECIALIZATION, 5)], &shield);
    assert!((sheet(&view, me).2 - 10.0).abs() < 1e-4);

    // At 5/5 every block gives a rage.
    let (mut blocks, mut rages) = (0, 0);
    for seed in 0..10 {
        let (mut realm, me, _, mut view) = pulled(seed, 6, &[(SHIELD_SPECIALIZATION, 5)], &shield);
        for _ in 0..300 {
            let events = view.run(&mut realm, 0.1);
            blocks += events
                .iter()
                .filter(|e| {
                    matches!(e, SessionEvent::AttackerState(s) if s.victim == me && s.blocked > 0)
                })
                .count();
            rages += energized(&events, SHIELD_SPECIALIZATION_RAGE);
            if view.health(me) < view.max_health(me) / 3 {
                break;
            }
        }
    }
    assert!(blocks > 0, "the camp's blows were never blocked");
    assert_eq!(rages, blocks);
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
        // Twice the blow, and a fifth of the bonus more.
        let blow = plain / 2;
        assert_eq!(impaled, blow * 2 + blow * 20 / 100, "seed {seed}");
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
const CROSSROADS_XY: [f32; 3] = [-441.8, -2648.6, 0.0];
/// A camp of four level 3 mobs south of the Crossroads (`content/barrens.toml`, 9000011).
const CAMP: [f32; 3] = [-376.0, -2837.0, 0.0];
/// The Tier 1 Warrior's Charge.
const CHARGE: u32 = 6178;

#[derive(Clone)]
struct Mob {
    guid: u64,
    position: [f32; 3],
}

/// What the client knows: every object's fields, merged as the packets arrive.
#[derive(Default)]
struct View {
    objects: std::collections::HashMap<u64, ObjectFields>,
}

impl View {
    fn see(&mut self, events: &[SessionEvent]) {
        for e in events {
            match e {
                SessionEvent::ObjectCreate { guid, fields, .. } => {
                    self.objects.insert(*guid, fields.clone());
                }
                SessionEvent::ObjectValues { guid, fields } => {
                    self.objects.entry(*guid).or_default().merge(fields.clone());
                }
                _ => {}
            }
        }
    }

    fn drain(&mut self, realm: &mut Realm) -> Vec<SessionEvent> {
        let events = drain(realm);
        self.see(&events);
        events
    }

    fn run(&mut self, realm: &mut Realm, secs: f32) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        for _ in 0..(secs / 0.1).round() as u32 {
            realm.tick(0.1, &FlatGround);
            events.extend(self.drain(realm));
        }
        events
    }

    fn fields(&self, guid: u64) -> &ObjectFields {
        self.objects
            .get(&guid)
            .expect("an object the client has seen")
    }

    fn rage(&self, guid: u64) -> u32 {
        self.fields(guid).unit_power(1).unwrap_or(0) / 10
    }

    fn health(&self, guid: u64) -> u32 {
        self.fields(guid).unit_health().unwrap()
    }

    fn max_health(&self, guid: u64) -> u32 {
        self.fields(guid).unit_max_health().unwrap()
    }

    fn aura(&self, guid: u64, spell_id: u32) -> Option<benilla_protocol::messages::UnitAuraSlot> {
        self.fields(guid)
            .unit_auras()
            .find(|a| a.spell_id == spell_id)
    }
}

/// A human Tier 1 Warrior of `level` with `talents` (`(talent id, rank)`) and `items` in its save,
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
        .replace("calling = \"basic_warrior\"", "calling = \"warrior\"");
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

/// The save without its `[[items]]` row for `slot`.
fn drop_slot(blob: &str, slot: u8) -> String {
    let mut out = String::new();
    for (i, part) in blob.split("[[items]]").enumerate() {
        if i == 0 {
            out.push_str(part);
        } else if !part.contains(&format!("slot = {slot}\n")) {
            out.push_str("[[items]]");
            out.push_str(part);
        }
    }
    out
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

/// A Tier 1 Warrior of `level` with `talents` and `items` that pulled the nearest camp without
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

/// A Tier 1 Warrior of `level` with `talents` and `items` that pulled the nearest camp south of
/// the Crossroads and stopped swinging: its mobs beat on the warrior from now on.
fn pulled(
    seed: u64,
    level: u8,
    talents: &[(u32, u8)],
    items: &[(u8, u32)],
) -> (Realm, u64, Mob, View) {
    let (mut realm, me, _, mut view) = tier1_at(level, seed, talents, items);
    // Out of aggro range of the four level 3 scorpids south of the Crossroads, then a mob of
    // theirs: its camp comes along.
    realm.handle(RealmCommand::Move {
        position: [CAMP[0] + 40.0, CAMP[1], 0.0],
        orientation: std::f32::consts::PI,
    });
    let streamed = view.run(&mut realm, 1.0);
    let mob = streamed
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
        .next()
        .expect("the scorpid camp streams in");
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
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

/// Whether `attacker` swung in `events`.
fn swings(events: &[SessionEvent], attacker: u64) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == attacker))
}

fn cast(spell_id: u32, target: Option<u64>) -> RealmCommand {
    RealmCommand::CastSpell { spell_id, target }
}

/// Moves the player to `yards` from `mob`, on the Crossroads side, facing it.
fn stand_at(realm: &mut Realm, mob: &Mob, yards: f32) {
    let (dx, dy) = (
        CROSSROADS_XY[0] - mob.position[0],
        CROSSROADS_XY[1] - mob.position[1],
    );
    let len = (dx * dx + dy * dy).sqrt();
    let at = [
        mob.position[0] + dx / len * yards,
        mob.position[1] + dy / len * yards,
        mob.position[2],
    ];
    realm.handle(RealmCommand::Move {
        position: at,
        orientation: (mob.position[1] - at[1]).atan2(mob.position[0] - at[0]),
    });
}

fn flat(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn go_hits(events: &[SessionEvent], spell_id: u32) -> Option<Vec<u64>> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellGo {
            spell_id: s, hits, ..
        } if *s == spell_id => Some(hits.clone()),
        _ => None,
    })
}

fn refusal(events: &[SessionEvent], spell_id: u32) -> Option<u8> {
    events.iter().find_map(|e| match e {
        SessionEvent::CastResult {
            spell_id: s,
            success: false,
            reason,
            ..
        } if *s == spell_id => *reason,
        _ => None,
    })
}
