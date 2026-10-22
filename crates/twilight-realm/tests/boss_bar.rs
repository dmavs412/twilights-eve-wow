//! The Boss Bar's list (#281): each character is sent the flagged creatures in a fight and in
//! their view, oldest fight first, complete on every change. The content flags no creature yet,
//! so the tests flag entry 46, a level 9 to 10 spawn pair run in Elwynn's south-west.

mod common;

use benilla_protocol::{EntityKind, SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;
use twilight_realm::RealmCommand;

/// The flagged creature's entry.
const FLAGGED: u32 = 46;
/// An entry no test flags.
const PLAIN: u32 = 524;
/// Two spawns of the flagged entry, 49 yd apart.
const FIRST: [f32; 3] = [-9893.45, -747.853, 20.695];
const SECOND: [f32; 3] = [-9860.71, -711.289, 29.038];
/// A spawn of the plain entry.
const PLAIN_AT: [f32; 3] = [-9751.72, -603.903, 39.734];

fn party(levels: &[u8]) -> Party {
    let realm = realm().with_bosses(&[FLAGGED]);
    // Every character carries a Megachopper (9490) in the main hand: the flat table (ADR 0012)
    // gives the flagged creatures no level-gap miss against a high level character.
    Party::edited(
        realm,
        levels,
        START_ELWYNN,
        Box::new(NoGround),
        |_, blob| {
            let mut blob = blob;
            if let Some(at) = blob.find("[[items]]") {
                blob.truncate(at);
            }
            blob.replace("items = []\n", "") + "\n[[items]]\nslot = 15\nentry = 9490\ncount = 1\n"
        },
    )
}

/// Yards from `at` toward `from`'s side, so a character stands in reach of a creature.
fn beside(at: [f32; 3]) -> [f32; 3] {
    [at[0] - 1.5, at[1], at[2]]
}

/// The lists player events carry, in order.
fn lists(events: &[SessionEvent]) -> Vec<Vec<u64>> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::BossFight { guids } => Some(guids.clone()),
            _ => None,
        })
        .collect()
}

/// The guid of the creature of `entry` standing at `at`, which player `i` has seen created.
fn guid_at(p: &Party, i: usize, entry: u32, at: [f32; 3]) -> u64 {
    p.seen(i)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } if fields.object_entry() == Some(entry) && flat_distance(*position, at) < 3.0 => {
                Some(*guid)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("a creature of {entry} near {at:?}"))
}

/// Player 0 beside `at`, facing it, swinging, with the world settled around them.
fn engage(p: &mut Party, at: [f32; 3], entry: u32) -> u64 {
    p.go_facing(0, beside(at), at);
    p.run(0.5);
    let guid = guid_at(p, 0, entry, at);
    p.cmd(0, RealmCommand::AttackSwing { guid });
    guid
}

#[test]
fn a_flagged_creature_pulled_is_listed_once() {
    let mut p = party(&[40]);
    let boss = engage(&mut p, FIRST, FLAGGED);
    p.run(1.0);
    let quiet = p.run(1.0);
    assert!(lists(&quiet[0]).is_empty(), "a quiet tick sends nothing");
    assert_eq!(lists(p.seen(0)), vec![vec![boss]]);
}

#[test]
fn a_character_in_view_hears_the_list_and_one_out_of_view_hears_nothing() {
    let mut p = party(&[40, 40, 40]);
    p.go(1, [FIRST[0] + 20.0, FIRST[1], FIRST[2]], 0.0);
    p.go(2, [FIRST[0] + 400.0, FIRST[1], FIRST[2]], 0.0);
    p.run(0.5);
    let boss = engage(&mut p, FIRST, FLAGGED);
    p.run(2.0);
    assert!(lists(p.seen(0)).contains(&vec![boss]));
    assert!(lists(p.seen(1)).contains(&vec![boss]));
    assert!(lists(p.seen(2)).is_empty());
}

#[test]
fn two_flagged_creatures_list_in_fight_order_and_a_death_drops_one() {
    let mut p = party(&[60]);
    let first = engage(&mut p, FIRST, FLAGGED);
    p.run(0.5);
    p.cmd(0, RealmCommand::AttackStop);
    p.go_facing(0, beside(SECOND), SECOND);
    p.run(0.5);
    let second = guid_at(&p, 0, FLAGGED, SECOND);
    p.cmd(0, RealmCommand::AttackSwing { guid: second });
    p.run(1.0);
    let seen = lists(p.seen(0));
    assert!(seen.contains(&vec![first, second]), "{seen:?}");
    assert!(
        !seen.contains(&vec![second, first]),
        "the first pulled stays first"
    );
    // The level 60 character kills them both; the first to die leaves the other.
    let dropped = p.run_until(60.0, |got| {
        lists(&got[0])
            .iter()
            .any(|l| l == &vec![second] || l == &vec![first])
    });
    assert!(dropped, "{:?}", lists(p.seen(0)));
}

#[test]
fn an_evade_empties_the_list() {
    let mut p = party(&[40]);
    let boss = engage(&mut p, FIRST, FLAGGED);
    p.run(1.0);
    p.cmd(0, RealmCommand::AttackStop);
    // 70 yd from the creature's home: past its 50 yd leash, inside the 100 yd view.
    p.go(0, [FIRST[0] + 70.0, FIRST[1], FIRST[2]], 0.0);
    let ended = p.run_until(60.0, |got| lists(&got[0]).last() == Some(&Vec::new()));
    assert!(ended, "{:?}", lists(p.seen(0)));
    let all = lists(p.seen(0));
    assert_eq!(all.first(), Some(&vec![boss]));
}

#[test]
fn walking_into_a_running_fight_lists_it_and_walking_out_keeps_it() {
    let mut p = party(&[40, 40]);
    p.go(1, [FIRST[0] + 400.0, FIRST[1], FIRST[2]], 0.0);
    p.run(0.5);
    let boss = engage(&mut p, FIRST, FLAGGED);
    p.run(1.0);
    assert!(lists(p.seen(1)).is_empty());
    p.go(1, [FIRST[0] + 20.0, FIRST[1], FIRST[2]], 0.0);
    p.run(1.0);
    assert_eq!(lists(p.seen(1)), vec![vec![boss]]);
    // A creature in a fight stays in every client's view until it is done (`Shard::keeps`), so
    // walking away mid-fight leaves the list as it was.
    p.go(1, [FIRST[0] + 400.0, FIRST[1], FIRST[2]], 0.0);
    p.run(2.0);
    assert_eq!(lists(p.seen(1)), vec![vec![boss]]);
}

#[test]
fn an_unflagged_creature_pulled_sends_nothing() {
    let mut p = party(&[40]);
    engage(&mut p, PLAIN_AT, PLAIN);
    p.run(3.0);
    assert!(lists(p.seen(0)).is_empty());
}
