//! The Zealot's borrowed Paladin skills, through the realm's seam: Hammer of Justice's stun, the
//! Seals (one at a time; Seal of Command's holy procs) and Judgement consuming the Seal.

mod common;

use benilla_protocol::messages::SpellDamageLog;
use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;

static HAMMER_OF_JUSTICE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("zealot", "hammer_of_justice"));
static SEAL_OF_COMMAND: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("zealot", "seal_of_command"));
static SEAL_OF_THE_CRUSADER: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("zealot", "seal_of_the_crusader"));
static JUDGEMENT: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("zealot", "judgement"));
static MORTAL_STRIKE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("zealot", "mortal_strike"));
const MAIN_HAND: u8 = 15;
/// A two-handed axe.
const LARGE_AXE: u32 = 2491;
/// Judgement of Command Rank 1, which the log names.
const JUDGEMENT_OF_COMMAND: u32 = 20467;
/// Judgement of the Crusader Rank 3, the debuff judging Seal of the Crusader leaves.
const JUDGEMENT_OF_THE_CRUSADER: u32 = 20300;
/// Spell.dbc's Seal of Command proc, which the log names.
const SEAL_OF_COMMAND_PROC: u32 = 20424;
/// `SPELL_SCHOOL_HOLY`.
const HOLY: u8 = 1;

#[test]
fn hammer_of_justice_stuns_for_six_seconds_and_the_mob_does_not_swing() {
    let (mut realm, me, mob, mut view) = tanking(20);
    stand_at(&mut realm, &mob, 8.0);
    let rage = view.rage(me);
    realm.handle(cast(*HAMMER_OF_JUSTICE, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, *HAMMER_OF_JUSTICE), None, "in 10 yd reach");
    assert_eq!(view.rage(me), rage - 15, "15 rage");
    let hits = go_hits(&events, *HAMMER_OF_JUSTICE).expect("it went off");
    assert!(hits.contains(&mob.guid), "{hits:?}");
    let stun = view
        .aura(mob.guid, *HAMMER_OF_JUSTICE)
        .expect("the stun shows");
    assert!(stun.slot >= 32, "a debuff");

    stand_at(&mut realm, &mob, 2.0);
    let stunned = view.run(&mut realm, 5.8);
    assert!(!swings(&stunned, mob.guid), "a stunned mob does not swing");
    assert!(view.aura(mob.guid, *HAMMER_OF_JUSTICE).is_some());
    view.run(&mut realm, 0.4);
    assert!(
        view.aura(mob.guid, *HAMMER_OF_JUSTICE).is_none(),
        "six seconds"
    );
    let after = view.run(&mut realm, 3.0);
    assert!(swings(&after, mob.guid), "and swings again after");

    realm.handle(cast(*HAMMER_OF_JUSTICE, Some(mob.guid)));
    assert!(
        refusal(&view.drain(&mut realm), *HAMMER_OF_JUSTICE).is_some(),
        "a 60 s cooldown"
    );
}

#[test]
fn a_seal_costs_ten_rage_in_battle_stance() {
    let (mut realm, me, _, mut view) = tanking(12);
    let rage = view.rage(me);
    realm.handle(cast(*SEAL_OF_COMMAND, None));
    assert_eq!(refusal(&view.drain(&mut realm), *SEAL_OF_COMMAND), None);
    assert_eq!(view.rage(me), rage - 10);
}

#[test]
fn seal_of_command_makes_landed_swings_proc_holy_damage() {
    let (mut realm, me, mob, mut view) = tanking(12);
    realm.handle(cast(*SEAL_OF_COMMAND, None));
    view.drain(&mut realm);
    let seal = view.aura(me, *SEAL_OF_COMMAND).expect("the Seal shows");
    assert!(seal.slot < 32, "a buff");

    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = view.run(&mut realm, 20.0);
    let procs: Vec<&SpellDamageLog> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(s) if s.spell_id == SEAL_OF_COMMAND_PROC => Some(s),
            _ => None,
        })
        .collect();
    assert!(!procs.is_empty(), "seven procs a minute: some in 20 s");
    for p in &procs {
        assert_eq!(p.school, HOLY, "logged as holy");
        assert_eq!((p.attacker, p.target), (me, mob.guid));
        assert!(p.damage > 0);
    }
    // Each proc rides on a white swing that landed just before it.
    for (i, e) in events.iter().enumerate() {
        if matches!(e, SessionEvent::SpellDamageLog(s) if s.spell_id == SEAL_OF_COMMAND_PROC) {
            let swing = events[..i].iter().rev().find_map(|e| match e {
                SessionEvent::AttackerState(s) if s.attacker == me => Some(s.damage),
                _ => None,
            });
            assert!(swing.is_some_and(|d| d > 0), "a landed swing first");
        }
    }
    let swings = events
        .iter()
        .filter(|e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == me && s.damage > 0))
        .count();
    assert!(procs.len() < swings, "a chance, not every swing");
}

#[test]
fn casting_another_seal_removes_seal_of_command() {
    let (mut realm, me, _, mut view) = tanking(20);
    realm.handle(cast(*SEAL_OF_COMMAND, None));
    view.drain(&mut realm);
    assert!(view.aura(me, *SEAL_OF_COMMAND).is_some());
    // Past the global cooldown.
    view.run(&mut realm, 1.6);
    realm.handle(cast(*SEAL_OF_THE_CRUSADER, None));
    assert_eq!(
        refusal(&view.drain(&mut realm), *SEAL_OF_THE_CRUSADER),
        None
    );
    assert!(
        view.aura(me, *SEAL_OF_COMMAND).is_none(),
        "one Seal at a time"
    );
    assert!(view.aura(me, *SEAL_OF_THE_CRUSADER).is_some());
}

#[test]
fn judgement_with_no_seal_fails_and_spends_no_rage() {
    let (mut realm, me, mob, mut view) = tanking(10);
    stand_at(&mut realm, &mob, 5.0);
    let rage = view.rage(me);
    realm.handle(cast(*JUDGEMENT, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert!(refusal(&events, *JUDGEMENT).is_some(), "no Seal to judge");
    assert_eq!(go_hits(&events, *JUDGEMENT), None);
    assert_eq!(view.rage(me), rage, "nothing spent");
}

#[test]
fn judging_seal_of_command_consumes_it_for_a_holy_hit() {
    let (mut realm, me, mob, mut view) = tanking(15);
    realm.handle(cast(*SEAL_OF_COMMAND, None));
    view.drain(&mut realm);
    stand_at(&mut realm, &mob, 5.0);
    let rage = view.rage(me);
    let health = view.health(mob.guid);
    let ap = view.fields(me).unit_attack_power().unwrap() as f64;

    realm.handle(cast(*JUDGEMENT, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, *JUDGEMENT), None);
    assert!(go_hits(&events, *JUDGEMENT).is_some_and(|h| h.contains(&mob.guid)));
    assert_eq!(view.rage(me), rage - 5, "5 rage");
    assert!(
        view.aura(me, *SEAL_OF_COMMAND).is_none(),
        "the Seal is spent"
    );
    let hit = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellDamageLog(s) if s.spell_id == JUDGEMENT_OF_COMMAND => Some(s),
            _ => None,
        })
        .expect("Judgement of Command's hit");
    assert_eq!(hit.school, HOLY);
    assert_eq!((hit.attacker, hit.target), (me, mob.guid));
    // A level 1 Zealot: 30 + 3 a level + 0.15 an attack power point, and no armour off it.
    let want = (30.0 + 3.0 + 0.15 * ap).round() as u32;
    assert_eq!(hit.damage, want, "at {ap} attack power");
    assert_eq!(view.health(mob.guid), health.saturating_sub(want));
}

#[test]
fn seal_of_the_crusader_quickens_swings_and_raises_attack_power_until_it_fades() {
    let (mut realm, me, mob, mut view) = tanking(12);
    let speed = view.fields(me).unit_base_attack_time(0).unwrap();
    assert_eq!(view.fields(me).unit_attack_power_mods(), (0, 0));

    realm.handle(cast(*SEAL_OF_THE_CRUSADER, None));
    view.drain(&mut realm);
    let seal = view
        .aura(me, *SEAL_OF_THE_CRUSADER)
        .expect("the Seal shows");
    assert!(seal.slot < 32, "a buff");
    // A level 1 Zealot: 20 + 4 attack power, and 40% faster.
    assert_eq!(view.fields(me).unit_attack_power_mods(), (24, 0));
    let hasted = view.fields(me).unit_base_attack_time(0).unwrap();
    assert_eq!(
        hasted,
        (f64::from(speed) / 1.4).round() as u32,
        "from {speed} ms"
    );

    // The swings come at the hasted time, a tenth of a second at a time.
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut at = Vec::new();
    for tick in 0..60 {
        let events = view.run(&mut realm, 0.1);
        if events
            .iter()
            .any(|e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == me))
        {
            at.push(tick);
        }
    }
    realm.handle(RealmCommand::AttackStop);
    assert!(at.len() >= 3, "swings at ticks {at:?}");
    for gap in at.windows(2).map(|w| w[1] - w[0]) {
        let ms: u32 = gap * 100;
        assert!(
            ms.abs_diff(hasted) <= 100 && ms < speed,
            "a swing every {ms} ms, not {speed} ({at:?})"
        );
    }

    // Thirty seconds on, it fades and the sheet is as it was.
    view.run(&mut realm, 30.0 - 6.0 - 0.2);
    assert!(view.aura(me, *SEAL_OF_THE_CRUSADER).is_some());
    view.run(&mut realm, 0.4);
    assert!(view.aura(me, *SEAL_OF_THE_CRUSADER).is_none(), "30 s");
    assert_eq!(view.fields(me).unit_attack_power_mods(), (0, 0));
    assert_eq!(view.fields(me).unit_base_attack_time(0), Some(speed));
}

/// Seal of the Crusader cuts white swings only: Mortal Strike under it keeps its full weapon roll
/// and adds the Seal's attack power. The same fight under Seal of Command, which moves neither,
/// is the yardstick.
#[test]
fn mortal_strike_under_seal_of_the_crusader_keeps_its_full_weapon_roll() {
    // A Large Axe, whose roll is big enough that a cut by 1.4 would show.
    let axe = |blob: String| {
        drop_slot(&blob, MAIN_HAND)
            + &format!("\n[[items]]\nslot = {MAIN_HAND}\nentry = {LARGE_AXE}\ncount = 1\n")
    };
    let strike = |seal: u32| {
        let (mut realm, me, mob, mut view) = tanking_as(40, &axe);
        realm.handle(cast(seal, None));
        assert_eq!(refusal(&view.drain(&mut realm), seal), None);
        view.run(&mut realm, 1.6);
        let ap = view.fields(me).unit_attack_power_mods();
        realm.handle(cast(*MORTAL_STRIKE, Some(mob.guid)));
        let events = view.drain(&mut realm);
        assert_eq!(refusal(&events, *MORTAL_STRIKE), None);
        let hit = events
            .iter()
            .find_map(|e| match e {
                SessionEvent::SpellDamageLog(s) if s.spell_id == *MORTAL_STRIKE => Some(s.damage),
                _ => None,
            })
            .expect("Mortal Strike's hit");
        (hit, ap)
    };
    let (plain, plain_mods) = strike(*SEAL_OF_COMMAND);
    let (sealed, sealed_mods) = strike(*SEAL_OF_THE_CRUSADER);
    assert!(plain > 0, "the yardstick strike landed");
    assert_eq!(plain_mods, (0, 0));
    assert_eq!(sealed_mods, (24, 0), "the Seal's 20 + 4 attack power");
    // Cut by 1.4, the axe's roll loses what the Seal's attack power adds; uncut, it only adds.
    assert!(sealed > plain, "{sealed} under the Seal, {plain} without");
}

#[test]
fn seal_of_command_replaces_seal_of_the_crusader_and_its_haste() {
    let (mut realm, me, _, mut view) = tanking(20);
    let speed = view.fields(me).unit_base_attack_time(0).unwrap();
    realm.handle(cast(*SEAL_OF_THE_CRUSADER, None));
    view.drain(&mut realm);
    assert!(view.fields(me).unit_base_attack_time(0).unwrap() < speed);
    view.run(&mut realm, 1.6);
    realm.handle(cast(*SEAL_OF_COMMAND, None));
    assert_eq!(refusal(&view.drain(&mut realm), *SEAL_OF_COMMAND), None);
    assert!(view.aura(me, *SEAL_OF_THE_CRUSADER).is_none());
    assert!(view.aura(me, *SEAL_OF_COMMAND).is_some());
    assert_eq!(view.fields(me).unit_attack_power_mods(), (0, 0));
    assert_eq!(view.fields(me).unit_base_attack_time(0), Some(speed));
}

#[test]
fn judging_seal_of_the_crusader_consumes_it_for_a_holy_damage_taken_debuff() {
    let (mut realm, me, mob, mut view) = tanking(15);
    let speed = view.fields(me).unit_base_attack_time(0).unwrap();
    realm.handle(cast(*SEAL_OF_THE_CRUSADER, None));
    view.drain(&mut realm);
    stand_at(&mut realm, &mob, 5.0);
    let rage = view.rage(me);

    realm.handle(cast(*JUDGEMENT, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, *JUDGEMENT), None);
    assert!(go_hits(&events, *JUDGEMENT).is_some_and(|h| h.contains(&mob.guid)));
    assert_eq!(view.rage(me), rage - 5, "5 rage");
    assert!(
        view.aura(me, *SEAL_OF_THE_CRUSADER).is_none(),
        "the Seal is spent"
    );
    assert_eq!(view.fields(me).unit_attack_power_mods(), (0, 0));
    assert_eq!(view.fields(me).unit_base_attack_time(0), Some(speed));
    let debuff = view
        .aura(mob.guid, JUDGEMENT_OF_THE_CRUSADER)
        .expect("the judgement shows on the target");
    assert!(debuff.slot >= 32, "a debuff");
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SessionEvent::SpellDamageLog(_))),
        "no hit of its own"
    );

    // Ten seconds, with no swing to refresh it.
    view.run(&mut realm, 9.8);
    assert!(view.aura(mob.guid, JUDGEMENT_OF_THE_CRUSADER).is_some());
    view.run(&mut realm, 0.4);
    assert!(view.aura(mob.guid, JUDGEMENT_OF_THE_CRUSADER).is_none());
}

/// Judgement of the Crusader, kept up by a landed swing past its 10 s, and then Seal of Command
/// judged on the same target: the holy hit is a fifth bigger.
#[test]
fn holy_damage_at_a_judged_target_rises_by_a_fifth() {
    let (hit, ap) = judged_judgement_of_command(tanking(25));
    let plain = (30.0 + 3.0 + 0.15 * ap).round();
    assert_eq!(
        hit,
        (plain * 1.2).round() as u32,
        "{plain} at {ap} attack power, a fifth more"
    );
}

/// Improved Seal of the Crusader 3/3: the Seal's attack power and the judgement's holy damage
/// taken, each 15% more, and the client's tooltip cell for the attack power.
#[test]
fn improved_seal_of_the_crusader_raises_its_attack_power_and_judgement() {
    let improved = |blob: String| {
        blob.replace("level = 1\n", "level = 4\n").replace(
            "xp = 0\n",
            &format!(
                "xp = 0\ntalents = [[{}, 3]]\n",
                talent("zealot", "improved_seal_of_the_crusader")
            ),
        )
    };
    let (mut realm, me, _, mut view) = tanking_as(12, &improved);
    realm.handle(cast(*SEAL_OF_THE_CRUSADER, None));
    view.drain(&mut realm);
    // A level 4 Zealot: 20 + 4·4 = 36, 15% more is 41.4.
    assert_eq!(view.fields(me).unit_attack_power_mods(), (41, 0));

    let (hit, ap) = judged_judgement_of_command(tanking_as(25, &improved));
    let plain = (30.0 + 3.0 * 4.0 + 0.15 * ap).round();
    assert_eq!(
        hit,
        (plain * 1.23).round() as u32,
        "{plain} at {ap} attack power, 23% more"
    );
}

#[test]
fn improved_seal_of_the_crusader_tells_the_client_the_seals_attack_power_is_higher() {
    let (_, _, _, entry) = zealot_at_as(SEED, &|blob: String| {
        blob.replace("level = 1\n", "level = 4\n").replace(
            "xp = 0\n",
            &format!(
                "xp = 0\ntalents = [[{}, 2]]\n",
                talent("zealot", "improved_seal_of_the_crusader")
            ),
        )
    });
    let cell = entry
        .iter()
        .filter_map(|e| match *e {
            // `SPELLMOD_EFFECT1`, a percent on Seal of the Crusader's family bit.
            SessionEvent::SpellModifier {
                flat: false,
                mask_bit: 34,
                op: 3,
                value,
            } => Some(value),
            _ => None,
        })
        .next_back();
    assert_eq!(cell, Some(10), "two ranks: 10%");
}

/// Seal of the Crusader judged on the tanked mob, kept up past its 10 s by one landed swing,
/// and then Seal of Command judged on it: Judgement of Command's damage, and the attack power
/// it was judged at.
fn judged_judgement_of_command(
    (mut realm, me, mob, mut view): (Realm, u64, Mob, View),
) -> (u32, f64) {
    realm.handle(cast(*SEAL_OF_THE_CRUSADER, None));
    view.drain(&mut realm);
    view.run(&mut realm, 1.6);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(*JUDGEMENT, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), *JUDGEMENT), None);
    assert!(view.aura(mob.guid, JUDGEMENT_OF_THE_CRUSADER).is_some());
    realm.handle(cast(*SEAL_OF_COMMAND, None));
    assert_eq!(refusal(&view.drain(&mut realm), *SEAL_OF_COMMAND), None);

    // "Your melee strikes will refresh the spell's duration": one landed swing near its end.
    view.run(&mut realm, 8.0);
    let mut elapsed = 8.0;
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut landed = false;
    for _ in 0..19 {
        let events = view.run(&mut realm, 0.1);
        elapsed += 0.1;
        landed |= events.iter().any(
            |e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == me && s.damage > 0),
        );
        if landed {
            break;
        }
    }
    assert!(landed, "a swing landed before the judgement's 10 s were up");
    realm.handle(RealmCommand::AttackStop);
    // Past the judgement's first 10 s, when Judgement is ready again.
    view.run(&mut realm, 10.5 - elapsed);
    assert!(
        view.aura(mob.guid, JUDGEMENT_OF_THE_CRUSADER).is_some(),
        "refreshed by the swing"
    );

    let ap = view.fields(me).unit_attack_power().unwrap() as f64;
    realm.handle(cast(*JUDGEMENT, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, *JUDGEMENT), None);
    let hit = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellDamageLog(s) if s.spell_id == JUDGEMENT_OF_COMMAND => Some(s),
            _ => None,
        })
        .expect("Judgement of Command's hit");
    assert_eq!(hit.school, HOLY);
    (hit.damage, ap)
}

// ── Helpers ──────────────────────────────────────────────────────────────────────────────────

const SEED: u64 = 0x5EED_0023;

/// A human Zealot from a level 1 save that `edit` changes, in the world at the Crossroads; the
/// view and what entering the world sent.
fn zealot_at_as(
    seed: u64,
    edit: &dyn Fn(String) -> String,
) -> (Realm, u64, View, Vec<SessionEvent>) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("calling = \"basic_warrior\"", "calling = \"test_zealot\"");
    let blob = edit(blob);
    let mut realm = Realm::with_seed(seed).with_callings(&tier_0_zealot());
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let entry = view.drain(&mut realm);
    // The world streams in over the first ticks.
    view.run(&mut realm, 1.0);
    (realm, guid, view, entry)
}

/// A level 1 Zealot that pulled the camp south of the Crossroads without swinging back, and took
/// its blows until it had `rage`.
fn tanking(rage: u32) -> (Realm, u64, Mob, View) {
    tanking_as(rage, &|blob| blob)
}

/// [`tanking`], from a save that `edit` changes.
fn tanking_as(rage: u32, edit: &dyn Fn(String) -> String) -> (Realm, u64, Mob, View) {
    let (mut realm, me, mut view, _) = zealot_at_as(SEED, edit);
    // Out of reach of the four neutral scorpids, then each of them in turn.
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
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
    for other in camp.iter().rev() {
        pull(&mut realm, &mut view, other);
    }
    stand_at(&mut realm, &mob, 2.0);
    for _ in 0..1800 {
        view.run(&mut realm, 0.1);
        assert!(view.health(me) > 0, "died waiting for {rage} rage");
        if view.rage(me) >= rage {
            return (realm, me, mob, view);
        }
    }
    panic!("two minutes of blows built no {rage} rage");
}

/// Whether `mob` swung in `events`.
fn swings(events: &[SessionEvent], mob: u64) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == mob))
}

fn go_hits(events: &[SessionEvent], spell_id: u32) -> Option<Vec<u64>> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellGo {
            spell_id: s, hits, ..
        } if *s == spell_id => Some(hits.clone()),
        _ => None,
    })
}
