//! Creature Spells (ADR 0008): a creature that carries `corruption_creature` casts it at the
//! player it fights: the cast bar, then the curse, which ticks shadow damage for its duration.

mod common;

use benilla_protocol::messages::{PeriodicAuraLog, PeriodicTick};
use benilla_protocol::{EntityKind, SessionEvent};
use common::*;

/// The Vile Familiar near the Crossroads.
const VILE_FAMILIAR: u32 = 3101;
const CORRUPTION: &str = "corruption_creature";
/// What the client shows for it: Corruption rank 1.
const CORRUPTION_LOOK: u32 = 172;
/// The shadow school in a damage log.
const SHADOW: u32 = 5;

#[test]
fn a_caster_starts_its_cast_and_the_curse_ticks_shadow_damage_after_it() {
    let realm = Realm::with_seed(1).with_creature_spells(VILE_FAMILIAR, &[CORRUPTION]);
    // Strong enough to kill the familiar soon after its first cast, so no second one refreshes
    // the curse: it then runs its whole length with its caster dead.
    let (mut realm, me, mob) = warrior(realm, 60);
    stand_by(&mut realm, mob.1);
    let mut events = run(&mut realm, 2.2);
    realm.handle(RealmCommand::AttackSwing { guid: mob.0 });
    events.extend(run(&mut realm, 20.0).into_iter().map(|(t, e)| (t + 2.2, e)));

    let at = |pred: &dyn Fn(&SessionEvent) -> bool| -> Vec<f32> {
        events
            .iter()
            .filter(|(_, e)| pred(e))
            .map(|(t, _)| *t)
            .collect()
    };
    let starts = at(&|e| {
        matches!(e, SessionEvent::SpellStart { caster, spell_id, cast_time_ms: 2000, target: Some(t), .. }
            if *caster == mob.0 && *spell_id == CORRUPTION_LOOK && *t == me)
    });
    let goes = at(&|e| {
        matches!(e, SessionEvent::SpellGo { caster, spell_id, hits, .. }
            if *caster == mob.0 && *spell_id == CORRUPTION_LOOK && hits.contains(&me))
    });
    assert!(!starts.is_empty(), "the familiar casts");
    assert_eq!(goes.len(), 1, "and the cast goes off, once before it dies");
    let cast = goes[0] - starts[0];
    assert!((cast - 2.0).abs() < 0.15, "after its 2 s cast: {cast}");

    // The curse sits on the player's aura slots under the client's spell id.
    let shown = events.iter().any(|(t, e)| {
        *t >= goes[0]
            && matches!(e, SessionEvent::ObjectValues { guid, fields }
                if *guid == me && (0..48).any(|s| fields.unit_aura(s).is_some_and(|a| a.spell_id == CORRUPTION_LOOK)))
    });
    assert!(shown, "the player wears the curse");

    // 6 ticks of 10 shadow damage, one each 3 s after the cast.
    let ticks: Vec<(f32, u32)> = events
        .iter()
        .filter_map(|(t, e)| match e {
            SessionEvent::PeriodicAuraLog(PeriodicAuraLog {
                target,
                caster,
                spell_id,
                ticks,
            }) if *target == me && *caster == mob.0 && *spell_id == CORRUPTION_LOOK => {
                match ticks[..] {
                    [PeriodicTick::Damage { amount, school, .. }] => {
                        assert_eq!(school, SHADOW);
                        Some((*t, amount))
                    }
                    _ => None,
                }
            }
            _ => None,
        })
        .collect();
    assert_eq!(ticks.len(), 6, "{ticks:?}");
    assert!(ticks.iter().all(|(_, amount)| *amount == 10));
    for (n, (t, _)) in ticks.iter().enumerate() {
        let due = goes[0] + 3.0 * (n + 1) as f32;
        assert!((t - due).abs() < 0.15, "{ticks:?}");
    }
}

#[test]
fn a_creature_with_no_spells_never_casts() {
    let (mut realm, _, mob) = warrior(Realm::with_seed(1), 10);
    stand_by(&mut realm, mob.1);
    let events = run(&mut realm, 20.0);
    assert!(!events.iter().any(|(_, e)| matches!(
        e,
        SessionEvent::SpellStart { caster, .. } | SessionEvent::SpellGo { caster, .. } if *caster == mob.0
    )));
}

#[test]
fn a_caster_killed_mid_cast_lands_nothing() {
    let mut mid_cast = 0;
    for seed in 0..40 {
        let realm = Realm::with_seed(seed).with_creature_spells(VILE_FAMILIAR, &[CORRUPTION]);
        let (mut realm, me, mob) = warrior(realm, 60);
        stand_by(&mut realm, mob.1);
        let mut events = run(&mut realm, 0.5);
        realm.handle(RealmCommand::AttackSwing { guid: mob.0 });
        events.extend(run(&mut realm, 3.0));
        let started = events
            .iter()
            .any(|(_, e)| matches!(e, SessionEvent::SpellStart { caster, .. } if *caster == mob.0));
        let died = events.iter().position(|(_, e)| {
            matches!(e, SessionEvent::ObjectValues { guid, fields }
                if *guid == mob.0 && fields.unit_health() == Some(0))
        });
        let went = events.iter().position(
            |(_, e)| matches!(e, SessionEvent::SpellGo { caster, .. } if *caster == mob.0),
        );
        let (true, Some(died)) = (started, died) else {
            continue;
        };
        if went.is_some_and(|g| g < died) {
            continue;
        }
        mid_cast += 1;
        assert!(went.is_none(), "seed {seed}: a dead caster's cast went off");
        assert!(
            !events.iter().any(|(_, e)| matches!(e,
                SessionEvent::PeriodicAuraLog(l) if l.target == me)),
            "seed {seed}: the curse landed"
        );
    }
    assert!(mid_cast > 0, "some seed kills the familiar mid-cast");
}

// ── Helpers ───────────────────────────────────────────────────────────────────────────────────

/// A human warrior of `level` in `realm`, on flat ground, and the familiar's guid and position.
fn warrior(mut realm: Realm, level: u8) -> (Realm, u64, (u64, [f32; 3])) {
    let mut maker = common::realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"));
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mob = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } if fields.object_entry() == Some(VILE_FAMILIAR) => Some((guid, position)),
            _ => None,
        })
        .expect("the familiar is near");
    (realm, guid, mob)
}

/// The player 2 yd from `mob`, facing it: close enough that it notices and fights.
fn stand_by(realm: &mut Realm, mob: [f32; 3]) {
    let at = [mob[0] + 2.0, mob[1], mob[2]];
    realm.handle(RealmCommand::Move {
        position: at,
        orientation: std::f32::consts::PI,
    });
}

/// Ticks `secs` in 0.1 s steps, each event with the time it came at.
fn run(realm: &mut Realm, secs: f32) -> Vec<(f32, SessionEvent)> {
    let mut events: Vec<_> = drain(realm).into_iter().map(|e| (0.0, e)).collect();
    let steps = (secs * 10.0).round() as u32;
    for n in 1..=steps {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(realm).into_iter().map(|e| (n as f32 * 0.1, e)));
    }
    events
}
