//! A nova, a fear and a random target (#239), tried on the Gaze Hall's Vile Familiar
//! (`content/gaze_hall.toml`), which each test gives one Creature Spell.

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;
use twilight_realm::RealmCommand;

/// The Vile Familiar's entry, which the Gaze Hall places at (-9650, 20).
const FAMILIAR: u32 = 3101;
const CASTER: [f32; 3] = [-9650.0, 20.0, 47.2];
/// In melee reach of the familiar, so it never leaves its spot.
const EXPOSED: [f32; 3] = [-9650.0, 21.5, 47.2];
/// 8 yd east, with the Gaze Hall's pillar (5 yd east, 2.5 yd wide) between.
const BEHIND_PILLAR: [f32; 3] = [-9642.0, 20.0, 47.2];
/// 8 yd south, in the open.
const NEAR: [f32; 3] = [-9650.0, 12.0, 47.2];
/// In the Phase but 16 yd north.
const FAR: [f32; 3] = [-9650.0, 36.0, 47.2];
/// Out in Elwynn, clear of the box.
const OUTSIDE: [f32; 3] = [-9720.0, 20.0, 47.2];
/// `UNIT_FIELD_FLAGS` and its fleeing bit.
const UNIT_FIELD_FLAGS: u16 = 46;
const UNIT_FLAG_FLEEING: u32 = 0x0080_0000;
const SPELL_FAILED_FLEEING: u8 = 0x1e;
const SPELL_FAILED_NOT_KNOWN: u8 = 0x38;

struct Seen {
    at: f32,
    player: usize,
    event: SessionEvent,
}

struct Fight {
    log: Vec<Seen>,
    guids: Vec<u64>,
    /// What each cast command was answered with, and when.
    answers: Vec<(f32, Option<u8>)>,
}

impl Fight {
    fn of(&self, i: usize) -> impl Iterator<Item = &Seen> {
        self.log.iter().filter(move |s| s.player == i)
    }

    /// The fleeing bit on each flags update player `i` was sent, with the time.
    fn fleeing(&self, i: usize) -> Vec<(f32, bool)> {
        self.of(i)
            .filter_map(|s| match &s.event {
                SessionEvent::ObjectValues { guid, fields } if *guid == self.guids[i] => fields
                    .raw_fields()
                    .find_map(|(f, v)| (f == UNIT_FIELD_FLAGS).then_some(v))
                    .map(|v| (s.at, v & UNIT_FLAG_FLEEING != 0)),
                _ => None,
            })
            .collect()
    }

    /// When player `i` was handed the client's control (`true`) or had it taken (`false`).
    fn control(&self, i: usize) -> Vec<(f32, bool)> {
        self.of(i)
            .filter_map(|s| match &s.event {
                SessionEvent::ClientControl { mover, allow_move } if *mover == self.guids[i] => {
                    Some((s.at, *allow_move))
                }
                _ => None,
            })
            .collect()
    }

    /// Where each server-driven run of player `i` began and ended.
    fn runs(&self, i: usize) -> Vec<([f32; 3], [f32; 3])> {
        self.of(i)
            .filter_map(|s| match &s.event {
                SessionEvent::MonsterMove { guid, path, .. } if *guid == self.guids[i] => {
                    Some((path[0], *path.last().unwrap()))
                }
                _ => None,
            })
            .collect()
    }

    /// The damage the spell `id` did to player `i`.
    fn hits(&self, i: usize, id: u32) -> Vec<u32> {
        self.of(i)
            .filter_map(|s| match &s.event {
                SessionEvent::SpellDamageLog(l)
                    if l.target == self.guids[i] && l.spell_id == id =>
                {
                    Some(l.damage)
                }
                _ => None,
            })
            .collect()
    }
}

/// One character per place, the first pulling the familiar, which `spell` it casts. `ask` is a
/// time (seconds into the fight) at which player 0 tries a cast, and what comes of it.
fn fight(spell: &str, places: &[[f32; 3]], secs: f32, ask: &[f32]) -> Fight {
    let realm = realm().with_creature_spells(FAMILIAR, &[spell]);
    let mut p = Party::starting(
        realm,
        &vec![20; places.len()],
        START_ELWYNN,
        Box::new(NoGround),
    );
    let mut fight = Fight {
        log: Vec::new(),
        guids: p.guids.clone(),
        answers: Vec::new(),
    };
    let mut at = 0.0;
    let mut step = |p: &mut Party, secs: f32, fight: &mut Fight| {
        let mut left = secs;
        while left > 1e-3 {
            for (player, events) in p.run(0.1).into_iter().enumerate() {
                fight
                    .log
                    .extend(events.into_iter().map(|event| Seen { at, player, event }));
            }
            at += 0.1;
            left -= 0.1;
        }
    };
    p.go(0, places[0], 0.0);
    step(&mut p, 1.0, &mut fight);
    for (i, &place) in places.iter().enumerate().skip(1) {
        p.go(i, place, 0.0);
    }
    let mut asked = ask.to_vec();
    asked.sort_by(f32::total_cmp);
    let mut done = 0.0;
    for t in asked {
        step(&mut p, t - done, &mut fight);
        done = t;
        let session = p.sessions[0];
        let before = p.log.get(&session).map_or(0, Vec::len);
        p.cmd(
            0,
            RealmCommand::CastSpell {
                spell_id: 1,
                target: None,
            },
        );
        let reason = p.log[&session][before..].iter().find_map(|e| match e {
            SessionEvent::CastResult {
                spell_id: 1,
                reason,
                ..
            } => Some(*reason),
            _ => None,
        });
        fight.answers.push((t, reason.flatten()));
    }
    step(&mut p, (secs - done).max(0.0), &mut fight);
    fight
}

#[test]
fn a_nova_hits_everyone_within_its_radius_cover_or_not_and_no_one_beyond() {
    let nova = creature_spell("test_nova");
    let f = fight(
        "test_nova",
        &[EXPOSED, BEHIND_PILLAR, NEAR, FAR, OUTSIDE],
        10.0,
        &[],
    );
    for (i, what) in [
        (0, "in reach"),
        (1, "behind the pillar"),
        (2, "in the open"),
    ] {
        let hits = f.hits(i, nova);
        assert!(!hits.is_empty(), "{what} takes it");
        assert!(hits.iter().all(|&d| d == 30), "{what}: flat 30, {hits:?}");
    }
    assert!(
        f.hits(3, nova).is_empty(),
        "16 yd out: {:?}",
        f.hits(3, nova)
    );
    assert!(f.hits(4, nova).is_empty(), "outside the Phase");
}

#[test]
fn a_fear_takes_control_for_its_duration_and_gives_it_back() {
    let f = fight("test_fear", &[EXPOSED], 14.0, &[]);
    let control = f.control(0);
    let [(taken, false), (given, true)] = control[..] else {
        panic!("control taken once and given back once: {control:?}");
    };
    assert!(
        (given - taken - 6.0).abs() < 0.3,
        "6 s of fear: {taken} to {given}"
    );
    let flags = f.fleeing(0);
    assert!(
        flags
            .iter()
            .any(|&(at, fleeing)| fleeing && at <= taken + 0.2),
        "FLEEING set when control goes: {flags:?}"
    );
    assert_eq!(
        flags.last().map(|f| f.1),
        Some(false),
        "and cleared: {flags:?}"
    );
}

#[test]
fn a_feared_character_runs_from_the_caster_and_can_not_cast() {
    let f = fight("test_fear", &[EXPOSED], 14.0, &[5.0, 13.0]);
    let control = f.control(0);
    let [(taken, false), (given, true)] = control[..] else {
        panic!("{control:?}");
    };
    let runs = f.runs(0);
    assert!(runs.len() >= 3, "a turn every second or two: {runs:?}");
    let (first, last) = (runs[0].0, runs.last().unwrap().1);
    assert!(
        flat_distance(last, CASTER) > flat_distance(first, CASTER) + 10.0,
        "away from the caster: {first:?} to {last:?}"
    );
    for (from, to) in &runs {
        let yards = flat_distance(*from, *to);
        assert!(yards <= 7.0 * 2.0 + 0.1, "no faster than a run: {yards}");
    }
    let (during, after) = (&f.answers[0], &f.answers[1]);
    assert!(
        during.0 > taken && during.0 < given,
        "{during:?} in the fear"
    );
    assert_eq!(
        during.1,
        Some(SPELL_FAILED_FLEEING),
        "no casting while feared"
    );
    assert!(after.0 > given, "{after:?} after it");
    assert_eq!(
        after.1,
        Some(SPELL_FAILED_NOT_KNOWN),
        "control back: the cast is judged on its spell again"
    );
}

#[test]
fn a_random_target_is_a_character_in_the_phase_within_range_and_the_fight_stays_put() {
    let snare = creature_spell("test_snare_random");
    let f = fight(
        "test_snare_random",
        &[EXPOSED, NEAR, FAR, OUTSIDE],
        100.0,
        &[],
    );
    let mut aimed = std::collections::HashSet::new();
    for s in &f.log {
        if let SessionEvent::SpellStart {
            spell_id,
            target: Some(t),
            ..
        } = &s.event
        {
            if *spell_id == snare {
                aimed.insert(*t);
            }
        }
    }
    assert!(
        aimed.contains(&f.guids[0]),
        "the one it fights is a candidate"
    );
    assert!(aimed.contains(&f.guids[1]), "so is the other in range");
    assert!(!aimed.contains(&f.guids[2]), "not 16 yd out, past its 10");
    assert!(!aimed.contains(&f.guids[3]), "not outside the Phase");
    // A cast at the other character slows them in their own session.
    let slowed = f.of(1).any(|s| {
        matches!(&s.event, SessionEvent::ForceSpeedChange { guid, speed, .. }
            if *guid == f.guids[1] && *speed < 7.0)
    });
    assert!(slowed, "the character it picked is snared");
    // The creature fights the same character all along.
    let attacker = f
        .log
        .iter()
        .find_map(|s| match &s.event {
            SessionEvent::AttackStart { attacker, victim } if *victim == f.guids[0] => {
                Some(*attacker)
            }
            _ => None,
        })
        .expect("the familiar engages the first character");
    let victims: Vec<u64> = f
        .log
        .iter()
        .filter(|s| s.player == 0)
        .flat_map(|s| swung_at(std::slice::from_ref(&s.event), attacker))
        .collect();
    assert!(!victims.is_empty());
    assert!(victims.iter().all(|&v| v == f.guids[0]), "{victims:?}");
}
