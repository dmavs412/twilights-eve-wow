//! A creature that flees at low health (#472), tried on a Kobold Vermin, whose kit in
//! `content/elwynn_overrides.toml` runs at 15%: the first time its health falls there it runs
//! from its target for at most 15 yd or 4 s, swinging at nothing, and then fights on. It flees once
//! a fight.

mod common;

use benilla_protocol::messages::CHAT_MSG_MONSTER_SAY;
use benilla_protocol::SessionEvent;
use common::fight::flat;
use common::party::*;
use common::*;
use twilight_realm::{Audience, CROSSROADS};

/// Far out past The Barrens' camps: nothing else streams in.
const FAR_AWAY: [f32; 3] = [CROSSROADS[0] + 3000.0, CROSSROADS[1], 0.0];
const KOBOLD_VERMIN: u32 = 6;
/// `UNIT_FIELD_FLAGS` and its fleeing bit.
const UNIT_FIELD_FLAGS: u16 = 46;
const UNIT_FLAG_FLEEING: u32 = 0x0080_0000;

fn at(dx: f32, dy: f32) -> [f32; 3] {
    [FAR_AWAY[0] + dx, FAR_AWAY[1] + dy, 0.0]
}

/// What the character saw, each event with the seconds since the creature was brought low.
struct Seen(Vec<(f32, SessionEvent)>);

impl Seen {
    /// The fleeing bit on each of `guid`'s flags updates, with the time.
    fn fleeing(&self, guid: u64) -> Vec<(f32, bool)> {
        self.0
            .iter()
            .filter_map(|(t, e)| match e {
                SessionEvent::ObjectValues { guid: g, fields } if *g == guid => fields
                    .raw_fields()
                    .find_map(|(f, v)| (f == UNIT_FIELD_FLAGS).then_some(v))
                    .map(|v| (*t, v & UNIT_FLAG_FLEEING != 0)),
                _ => None,
            })
            .collect()
    }

    /// When `guid` swung.
    fn swings(&self, guid: u64) -> Vec<f32> {
        self.0
            .iter()
            .filter(|(_, e)| matches!(e, SessionEvent::AttackerState(s) if s.attacker == guid))
            .map(|(t, _)| *t)
            .collect()
    }

    /// Where each of `guid`'s runs ends, with the time it set off.
    fn runs(&self, guid: u64) -> Vec<(f32, [f32; 3])> {
        self.0
            .iter()
            .filter_map(|(t, e)| match e {
                SessionEvent::MonsterMove {
                    guid: g,
                    path,
                    stop: false,
                    ..
                } if *g == guid => path.last().map(|end| (*t, *end)),
                _ => None,
            })
            .collect()
    }
}

/// A level 20 character stands still at the origin and pulls a Kobold Vermin 3 yd east with a
/// swing; a second stands idle 12 yd east and 6 yd north, beyond social range of the first but
/// 6 yd from its run. Once the first fights, its health is put at 14% and 8 s go by. The party,
/// the two creatures and what the character saw.
fn brought_low() -> (Party, u64, u64, Seen) {
    let mut party = Party::new(realm(), &[20]);
    party.go(0, at(0.0, 0.0), 0.0);
    party.run(0.5);
    let map = party.realm.player_positions()[0].0;
    let audience = Audience::of(&[party.guids[0]]);
    let spawn = |p: &mut Party, to: [f32; 3]| {
        p.realm
            .spawn_for(p.host.as_ref(), KOBOLD_VERMIN, 2, to, map, audience.clone())
            .expect("a kobold")
    };
    let runner = spawn(&mut party, at(3.0, 0.0));
    let mate = spawn(&mut party, at(12.0, 6.0));
    party.run(0.5);
    // A neutral creature: one swing pulls it.
    party.cmd(0, RealmCommand::AttackSwing { guid: runner });
    let engaged = party.run_until(3.0, |got| {
        got[0]
            .iter()
            .any(|e| matches!(e, SessionEvent::AttackStart { attacker, .. } if *attacker == runner))
    });
    assert!(engaged, "the vermin turned on the character");
    party.cmd(0, RealmCommand::AttackStop);
    party.realm.set_creature_health_pct(runner, 14);
    let mut seen = Vec::new();
    let mut t = 0.0;
    while t < 8.0 {
        t += 0.1;
        for e in party.run(0.1).swap_remove(0) {
            seen.push((t, e));
        }
    }
    (party, runner, mate, Seen(seen))
}

#[test]
fn at_14_percent_it_runs_more_than_10_yd_swings_at_nothing_and_comes_back() {
    let (_, runner, _, seen) = brought_low();
    let flags = seen.fleeing(runner);
    let [(began, true), (ended, false), ..] = flags[..] else {
        panic!("flees, then stops: {flags:?}");
    };
    assert!(began <= 0.2, "at once: {began}");
    assert!(ended - began <= 4.05, "for at most 4 s: {began} to {ended}");
    let me = at(0.0, 0.0);
    let farthest = seen
        .runs(runner)
        .iter()
        .filter(|(t, _)| *t >= began && *t <= ended)
        .map(|(_, end)| flat(*end, me))
        .fold(0.0, f32::max);
    assert!(farthest > 10.0, "more than 10 yd off: {farthest}");
    assert!(
        farthest < 3.0 + 15.5,
        "no more than 15 yd of running: {farthest}"
    );
    let swings = seen.swings(runner);
    assert!(
        !swings.iter().any(|&t| t > began && t <= ended),
        "no swing while it runs: {swings:?}, {began} to {ended}"
    );
    assert!(
        swings.iter().any(|&t| t > ended && t <= began + 6.0),
        "back on its target within 6 s: {swings:?}"
    );
}

#[test]
fn it_says_its_flee_line_as_it_runs() {
    let (_, runner, _, seen) = brought_low();
    let said: Vec<&str> = seen
        .0
        .iter()
        .filter_map(|(_, e)| match e {
            SessionEvent::Chat(c)
                if c.chat_type == CHAT_MSG_MONSTER_SAY && c.sender_guid == runner =>
            {
                Some(c.text.as_str())
            }
            _ => None,
        })
        .filter(|text| *text != "You no take candle!")
        .collect();
    assert_eq!(said, ["Yiieeeee! Me run!"]);
}

#[test]
fn its_run_pulls_no_creature_6_yd_away() {
    let (_, _, mate, seen) = brought_low();
    let turned = seen.0.iter().any(|(_, e)| {
        matches!(e, SessionEvent::AttackStart { attacker, .. } if *attacker == mate)
            || matches!(e, SessionEvent::AttackerState(s) if s.attacker == mate)
    });
    assert!(!turned, "the idle kobold stays idle");
}

#[test]
fn it_flees_once_a_fight() {
    let (mut party, runner, _, _) = brought_low();
    party.realm.set_creature_health_pct(runner, 10);
    let got = party.run(5.0).swap_remove(0);
    let fled = got.iter().any(|e| match e {
        SessionEvent::ObjectValues { guid, fields } if *guid == runner => fields
            .raw_fields()
            .any(|(f, v)| f == UNIT_FIELD_FLAGS && v & UNIT_FLAG_FLEEING != 0),
        _ => false,
    });
    assert!(!fled, "no second flee at 10%");
    assert!(!died(&got, runner), "it fought on");
}
