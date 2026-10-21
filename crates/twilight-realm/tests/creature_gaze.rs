//! A gaze (#235): a long Creature Spell cast whose landing hits every character in the caster's
//! Phase who is within its radius and not behind a cover pillar, announced by a raid warning.
//! The Gaze Hall (`content/gaze_hall.toml`) has one pillar and a Vile Familiar, which this realm
//! gives `test_gaze`: 20 shadow damage within 15 yd after a 2 s cast, first at 5 s of combat, then
//! 10 s after each landing.

mod common;

use benilla_protocol::messages::CHAT_MSG_RAID_WARNING;
use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

/// The Vile Familiar's entry, which the Gaze Hall places at (-9650, 20).
const FAMILIAR: u32 = 3101;
/// In melee reach of the familiar, so it never leaves its spot.
const EXPOSED: [f32; 3] = [-9650.0, 21.5, 47.2];
/// 12 yd east of it, with the pillar (5 yd east, 2.5 yd wide) on the line between.
const COVERED: [f32; 3] = [-9638.0, 20.0, 47.2];
/// In the Phase but 18 yd north, past the gaze's 15 yd.
const FAR: [f32; 3] = [-9650.0, 38.0, 47.2];
/// Out in Elwynn, clear of the box.
const OUTSIDE: [f32; 3] = [-9720.0, 20.0, 47.2];
const DAMAGE: u32 = 20;

static GAZE: std::sync::LazyLock<u32> = std::sync::LazyLock::new(|| creature_spell("test_gaze"));

struct Seen {
    at: f32,
    player: usize,
    event: SessionEvent,
}

struct Fight {
    log: Vec<Seen>,
    guids: Vec<u64>,
}

impl Fight {
    fn times(&self, pred: impl Fn(&Seen) -> bool) -> Vec<f32> {
        self.log.iter().filter(|s| pred(s)).map(|s| s.at).collect()
    }

    /// When player `i` was sent a gaze's cast bar.
    fn starts(&self, i: usize) -> Vec<f32> {
        self.times(|s| {
            s.player == i
                && matches!(&s.event, SessionEvent::SpellStart { spell_id, .. } if *spell_id == *GAZE)
        })
    }

    /// When the gaze's damage hit player `i`, and for how much.
    fn hits(&self, i: usize) -> Vec<(f32, u32)> {
        let me = self.guids[i];
        self.log
            .iter()
            .filter_map(|s| match &s.event {
                SessionEvent::SpellDamageLog(l) if l.target == me && l.spell_id == *GAZE => {
                    Some((s.at, l.damage))
                }
                _ => None,
            })
            .collect()
    }

    /// How many raid warnings player `i` was sent.
    fn warnings(&self, i: usize) -> Vec<String> {
        self.log
            .iter()
            .filter(|s| s.player == i)
            .filter_map(|s| match &s.event {
                SessionEvent::Chat(m) if m.chat_type == CHAT_MSG_RAID_WARNING => {
                    Some(m.text.clone())
                }
                _ => None,
            })
            .collect()
    }
}

/// Four characters, the first exposed and pulling the familiar; `secs` of fighting with the
/// other three placed once it has engaged.
fn fight(secs: f32) -> Fight {
    let realm = realm().with_creature_spells(FAMILIAR, &["test_gaze"]);
    let mut p = Party::starting(realm, &[20; 4], START_ELWYNN, Box::new(NoGround));
    let mut fight = Fight {
        log: Vec::new(),
        guids: p.guids.clone(),
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
    p.go(0, EXPOSED, 0.0);
    step(&mut p, 1.0, &mut fight);
    p.go(1, COVERED, 0.0);
    p.go(2, FAR, 0.0);
    p.go(3, OUTSIDE, 0.0);
    step(&mut p, secs, &mut fight);
    fight
}

#[test]
fn an_exposed_character_takes_the_damage_and_one_behind_the_pillar_or_out_of_range_takes_none() {
    let f = fight(10.0);
    let hits = f.hits(0);
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].1, DAMAGE, "flat damage");
    assert!(f.hits(1).is_empty(), "behind the pillar: {:?}", f.hits(1));
    assert!(f.hits(2).is_empty(), "past the radius: {:?}", f.hits(2));
    assert!(f.hits(3).is_empty(), "outside the Phase: {:?}", f.hits(3));
}

#[test]
fn the_gaze_waits_for_first_cast_and_then_for_its_cooldown() {
    let f = fight(30.0);
    let engaged = f
        .times(|s| {
            s.player == 0
                && matches!(&s.event, SessionEvent::AttackStart { victim, .. } if *victim == f.guids[0])
        })
        .into_iter()
        .next()
        .expect("the familiar engages");
    let (starts, hits) = (f.starts(0), f.hits(0));
    assert!(starts.len() >= 2, "{starts:?}");
    assert!(
        starts[0] - engaged >= 5.0 - 0.15,
        "first cast at {} after engaging at {engaged}",
        starts[0]
    );
    assert!(
        starts[0] - engaged < 7.5,
        "and soon after: {}",
        starts[0] - engaged
    );
    let cast = hits[0].0 - starts[0];
    assert!((cast - 2.0).abs() < 0.25, "its 2 s cast: {cast}");
    assert!(
        starts[1] - hits[0].0 >= 10.0 - 0.15,
        "the next waits its cooldown from the landing: {starts:?} {hits:?}"
    );
}

#[test]
fn the_warning_goes_once_per_cast_start_to_the_phase_only() {
    let f = fight(30.0);
    let casts = f.starts(0).len();
    assert!(casts >= 2, "{casts}");
    for i in 0..3 {
        let warned = f.warnings(i);
        assert_eq!(warned.len(), casts, "player {i}: {warned:?}");
        assert!(warned
            .iter()
            .all(|t| t == "Baleful Gaze! Get behind cover!"));
    }
    assert!(f.warnings(3).is_empty(), "outside the Phase");
}

#[test]
fn nothing_else_is_cast_or_swung_during_the_gaze() {
    let f = fight(20.0);
    let (start, go) = (f.starts(0)[0], f.hits(0)[0].0);
    let during: Vec<&SessionEvent> = f
        .log
        .iter()
        .filter(|s| s.player == 0 && s.at > start + 0.05 && s.at < go - 0.15)
        .map(|s| &s.event)
        .filter(|e| {
            matches!(
                e,
                SessionEvent::AttackerState(_)
                    | SessionEvent::SpellStart { .. }
                    | SessionEvent::SpellGo { .. }
            )
        })
        .collect();
    assert!(during.is_empty(), "{during:?}");
}
