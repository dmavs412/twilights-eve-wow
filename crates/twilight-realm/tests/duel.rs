//! Duels (spec #152): a challenge reaches both players, the accept starts a 3 s countdown and
//! then the fight; a decline or a cancel calls it off on both sides, and a challenge to a player
//! already in one is refused. Played by two (or three) level 10 warriors through the realm's seam.

mod common;

use benilla_protocol::SessionEvent;
use common::party::*;
use common::*;

/// The client's duel spell (`content/duel.toml`).
const DUEL: u32 = 7266;
/// `SPELL_FAILED_TARGET_DUELING`.
const TARGET_DUELING: u8 = 0x68;
/// `SPELL_FAILED_OUT_OF_RANGE`.
const OUT_OF_RANGE: u8 = 0x59;

/// `n` players side by side, 2 yd apart.
fn players(n: usize) -> Party {
    let mut party = Party::new(Realm::with_seed(1), &vec![10; n]);
    let here = position_of(&party, 0);
    for i in 1..n {
        party.go(i, [here[0] + 2.0 * i as f32, here[1], here[2]], 0.0);
    }
    party.run(0.2);
    party
}

/// Where player `i` stands, from their own create.
fn position_of(party: &Party, i: usize) -> [f32; 3] {
    let guid = party.guids[i];
    party
        .seen(i)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g, position, ..
            } if *g == guid => Some(*position),
            _ => None,
        })
        .expect("the player's own create")
}

/// Player `from` challenges player `to`; the flag's guid, when the challenge went out.
fn challenge(party: &mut Party, from: usize, to: usize) -> Option<u64> {
    let target = party.guids[to];
    party.cmd(
        from,
        RealmCommand::CastSpell {
            spell_id: DUEL,
            target: Some(target),
        },
    );
    requested(party.seen(to))
        .last()
        .map(|(arbiter, _)| *arbiter)
}

fn requested(events: &[SessionEvent]) -> Vec<(u64, u64)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::DuelRequested {
                arbiter,
                challenger,
            } => Some((*arbiter, *challenger)),
            _ => None,
        })
        .collect()
}

fn countdowns(events: &[SessionEvent]) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::DuelCountdown { seconds } => Some(*seconds),
            _ => None,
        })
        .collect()
}

fn completes(events: &[SessionEvent]) -> Vec<bool> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::DuelComplete { started } => Some(*started),
            _ => None,
        })
        .collect()
}

fn winners(events: &[SessionEvent]) -> Vec<(bool, String, String)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::DuelWinner {
                fled,
                winner,
                loser,
            } => Some((*fled, winner.clone(), loser.clone())),
            _ => None,
        })
        .collect()
}

/// The challenge's own cast result: `None` for a success, the reason for a refusal.
fn refusal(events: &[SessionEvent]) -> Option<Option<u8>> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::CastResult {
            spell_id, reason, ..
        } if *spell_id == DUEL => Some(*reason),
        _ => None,
    })
}

#[test]
fn a_challenge_reaches_both_and_the_accept_starts_the_countdown() {
    let mut party = players(2);
    let a = party.guids[0];
    let arbiter = challenge(&mut party, 0, 1).expect("the challenge goes out");
    assert_eq!(refusal(party.seen(0)), Some(None), "the cast succeeds");
    for i in 0..2 {
        assert_eq!(
            requested(party.seen(i)),
            [(arbiter, a)],
            "player {i} is told"
        );
    }
    // The challenger's client accepts its own challenge: that starts nothing.
    party.cmd(0, RealmCommand::DuelAccept { arbiter });
    assert!(countdowns(party.seen(1)).is_empty());
    party.cmd(1, RealmCommand::DuelAccept { arbiter });
    for i in 0..2 {
        assert_eq!(countdowns(party.seen(i)), [3]);
    }
}

#[test]
fn after_the_countdown_they_fight_and_a_yield_loses() {
    let mut party = players(2);
    let arbiter = challenge(&mut party, 0, 1).unwrap();
    party.cmd(1, RealmCommand::DuelAccept { arbiter });
    party.run(3.2);
    party.cmd(1, RealmCommand::DuelCancel { arbiter });
    for i in 0..2 {
        assert_eq!(completes(party.seen(i)), [true], "a fought duel ends");
        assert_eq!(
            winners(party.seen(i)),
            [(false, "Varian".to_string(), "Anduin".to_string())],
            "the one who yielded loses"
        );
    }
}

#[test]
fn a_cancel_in_the_countdown_calls_it_off() {
    let mut party = players(2);
    let arbiter = challenge(&mut party, 0, 1).unwrap();
    party.cmd(1, RealmCommand::DuelAccept { arbiter });
    party.run(2.0);
    party.cmd(0, RealmCommand::DuelCancel { arbiter });
    for i in 0..2 {
        assert_eq!(completes(party.seen(i)), [false]);
        assert!(winners(party.seen(i)).is_empty(), "no one won");
    }
}

#[test]
fn a_decline_or_a_cancel_clears_both_sides() {
    let mut party = players(2);
    let arbiter = challenge(&mut party, 0, 1).unwrap();
    party.cmd(1, RealmCommand::DuelCancel { arbiter });
    for i in 0..2 {
        assert_eq!(completes(party.seen(i)), [false], "declined");
    }
    // Both are free again: the challenged challenges back, and the challenger calls it off.
    let again = challenge(&mut party, 1, 0).expect("free to duel again");
    party.cmd(1, RealmCommand::DuelCancel { arbiter: again });
    for i in 0..2 {
        assert_eq!(completes(party.seen(i)), [false, false], "cancelled");
    }
    assert!(challenge(&mut party, 0, 1).is_some(), "and free again");
}

#[test]
fn a_challenge_to_a_player_in_a_duel_is_refused() {
    let mut party = players(3);
    let arbiter = challenge(&mut party, 0, 1).unwrap();
    challenge(&mut party, 2, 1);
    assert_eq!(refusal(party.seen(2)), Some(Some(TARGET_DUELING)));
    assert_eq!(
        requested(party.seen(1)).len(),
        1,
        "only the first reached them"
    );
    party.cmd(1, RealmCommand::DuelAccept { arbiter });
    challenge(&mut party, 2, 0);
    assert_eq!(refusal(party.seen(2)), Some(Some(TARGET_DUELING)));
}

#[test]
fn walking_away_from_an_unanswered_challenge_lapses_it() {
    let mut party = players(2);
    challenge(&mut party, 0, 1).unwrap();
    let here = position_of(&party, 0);
    party.go(0, [here[0] + 40.0, here[1], here[2]], 0.0);
    party.run(0.2);
    for i in 0..2 {
        assert_eq!(completes(party.seen(i)), [false]);
    }
    challenge(&mut party, 0, 1);
    assert_eq!(refusal(party.seen(0)), Some(Some(OUT_OF_RANGE)));
}

#[test]
fn a_logout_in_the_challenge_calls_it_off() {
    let mut party = players(2);
    challenge(&mut party, 0, 1).unwrap();
    party.cmd(1, RealmCommand::Logout);
    party.run(0.2);
    assert_eq!(completes(party.seen(0)), [false]);
    assert!(winners(party.seen(0)).is_empty());
}
