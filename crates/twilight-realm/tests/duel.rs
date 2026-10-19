//! Duels (spec #152): a challenge reaches both players, the accept starts a 3 s countdown and
//! then the fight; a decline or a cancel calls it off on both sides, and a challenge to a player
//! already in one is refused. Played by two (or three) level 10 warriors through the realm's seam.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::{cast, View};
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
    players_at(&vec![10; n], |_, blob| blob)
}

/// A player of each of `levels` in a row 2 yd apart northward, each save passed through `edit`;
/// the first faces the second.
fn players_at(levels: &[u8], edit: impl Fn(usize, String) -> String) -> Party {
    let mut party = Party::edited(Realm::with_seed(1), levels, 0, Box::new(NoGround), edit);
    let here = position_of(&party, 0);
    party.go(0, here, 0.0);
    for i in 1..levels.len() {
        party.go(i, [here[0] + 2.0 * i as f32, here[1], here[2]], 0.0);
    }
    party.run(0.2);
    party
}

/// Players 0 and 1 in a duel, past the countdown; its flag's guid.
fn fighting(party: &mut Party) -> u64 {
    let arbiter = challenge(party, 0, 1).expect("the challenge goes out");
    party.cmd(1, RealmCommand::DuelAccept { arbiter });
    party.run(3.2);
    arbiter
}

/// Player `i`'s client's view of the world so far.
fn view_of(party: &Party, i: usize) -> View {
    let mut view = View::default();
    view.see(party.seen(i));
    view
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

// ── The fight (#181) ─────────────────────────────────────────────────────────────────────────

/// Gouge, the Basic Warrior's incapacitate.
const GOUGE: u32 = 1776;
/// Whole rage points Gouge costs.
const GOUGE_RAGE: u32 = 15;
/// Searing Ember, the Initiate's burn.
const EMBER: u32 = 133;

/// Player 0 swings at player 1 until `done` holds for what player 1's client has, at most `secs`.
fn swing_until(party: &mut Party, secs: f32, done: impl Fn(&Party) -> bool) -> bool {
    let b = party.guids[1];
    party.cmd(0, RealmCommand::AttackSwing { guid: b });
    let mut t = 0.0;
    while t < secs {
        if done(party) {
            return true;
        }
        party.run(0.1);
        t += 0.1;
    }
    done(party)
}

fn ticks_on(events: &[SessionEvent], target: u64) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::PeriodicAuraLog(l) if l.target == target))
        .count()
}

#[test]
fn a_blow_in_a_duel_lands_on_the_opponent() {
    let mut party = players(2);
    fighting(&mut party);
    let b = party.guids[1];
    let full = view_of(&party, 1).health(b);
    let hit = swing_until(&mut party, 10.0, |p| view_of(p, 1).health(b) < full);
    assert!(hit, "player 1's health drops");
}

#[test]
fn a_third_player_cannot_hurt_a_duellist() {
    let mut party = players(3);
    fighting(&mut party);
    let a = party.guids[0];
    let c = party.guids[2];
    let full = view_of(&party, 0).health(a);
    party.go_facing(2, position_of(&party, 2), position_of(&party, 0));
    party.cmd(2, RealmCommand::AttackSwing { guid: a });
    let after = party.run(5.0);
    assert!(
        swung_at(&after[2], c).is_empty() && swung_at(&after[0], c).is_empty(),
        "no blow lands"
    );
    assert_eq!(view_of(&party, 0).health(a), full);
}

#[test]
fn a_killing_blow_leaves_the_loser_at_1_health_and_ends_the_duel() {
    let mut party = players_at(&[20, 1], |_, blob| blob);
    fighting(&mut party);
    let b = party.guids[1];
    let won = swing_until(&mut party, 60.0, |p| !winners(p.seen(1)).is_empty());
    assert!(won, "the duel ends");
    assert_eq!(
        winners(party.seen(1)),
        [(false, "Varian".to_string(), "Anduin".to_string())]
    );
    assert_eq!(view_of(&party, 1).health(b), 1, "left at 1 health");
    assert_eq!(completes(party.seen(1)), [true]);
    // The swings stop, and nothing refills them.
    let after = party.run(2.0);
    assert!(
        swung_at(&after[0], party.guids[0]).is_empty(),
        "no more blows"
    );
    assert!(view_of(&party, 1).health(b) < 10, "not refilled");
}

#[test]
fn a_gouge_lands_on_the_opponent_and_comes_off_at_the_end() {
    let mut party = players(2);
    let arbiter = fighting(&mut party);
    let b = party.guids[1];
    let raged = swing_until(&mut party, 60.0, |p| {
        view_of(p, 0).rage(p.guids[0]) >= GOUGE_RAGE
    });
    assert!(raged, "rage for Gouge");
    party.cmd(0, RealmCommand::AttackStop);
    party.cmd(0, cast(GOUGE, Some(b)));
    party.run(0.3);
    assert!(
        view_of(&party, 1).aura(b, GOUGE).is_some(),
        "player 1 is gouged"
    );
    party.cmd(1, RealmCommand::DuelCancel { arbiter });
    party.run(0.2);
    assert!(view_of(&party, 1).aura(b, GOUGE).is_none(), "it comes off");
}

#[test]
fn an_ember_burns_the_opponent_and_stops_at_the_end() {
    let mut party = players_at(&[10, 10], |i, blob| {
        if i == 0 {
            blob.replace("calling = \"basic_warrior\"", "calling = \"initiate\"")
        } else {
            blob
        }
    });
    let arbiter = fighting(&mut party);
    let b = party.guids[1];
    let full = view_of(&party, 1).health(b);
    party.cmd(0, cast(EMBER, Some(b)));
    let burning = party.run(4.0);
    assert!(ticks_on(&burning[1], b) > 0, "it ticks on player 1");
    assert!(view_of(&party, 1).health(b) < full);
    assert!(view_of(&party, 1).aura(b, EMBER).is_some());
    party.cmd(1, RealmCommand::DuelCancel { arbiter });
    let after = party.run(5.0);
    assert_eq!(ticks_on(&after[1], b), 0, "no more ticks");
    assert!(
        view_of(&party, 1).aura(b, EMBER).is_none(),
        "the burn comes off"
    );
}

// ── The flag and the bounds (#182) ───────────────────────────────────────────────────────────

fn flag_created(events: &[SessionEvent], arbiter: u64) -> bool {
    events.iter().any(|e| {
        matches!(e, SessionEvent::ObjectCreate { guid, kind: benilla_protocol::EntityKind::GameObject, .. }
            if *guid == arbiter)
    })
}

fn flag_destroyed(events: &[SessionEvent], arbiter: u64) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == arbiter))
}

/// The last `PLAYER_DUEL_ARBITER` and `PLAYER_DUEL_TEAM` player `i`'s client has for `guid`.
fn duel_fields(party: &Party, i: usize, guid: u64) -> (u64, u32) {
    let view = view_of(party, i);
    let f = view.fields(guid);
    (f.player_duel_arbiter(), f.player_duel_team())
}

fn bounds(events: &[SessionEvent]) -> Vec<bool> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::DuelOutOfBounds => Some(false),
            SessionEvent::DuelInBounds => Some(true),
            _ => None,
        })
        .collect()
}

#[test]
fn the_flag_goes_up_on_the_accept_and_comes_down_at_the_end() {
    let mut party = players(3);
    let (a, b) = (party.guids[0], party.guids[1]);
    let arbiter = challenge(&mut party, 0, 1).unwrap();
    for i in 0..2 {
        assert!(
            !flag_created(party.seen(i), arbiter),
            "no flag before the accept"
        );
    }
    assert_eq!(duel_fields(&party, 0, a).0, arbiter, "both carry its guid");
    assert_eq!(duel_fields(&party, 0, b).0, arbiter);
    party.cmd(1, RealmCommand::DuelAccept { arbiter });
    for i in 0..3 {
        assert!(
            flag_created(party.seen(i), arbiter),
            "player {i} sees the flag"
        );
    }
    party.run(3.2);
    assert_eq!(
        duel_fields(&party, 1, a),
        (arbiter, 1),
        "the challenger's side"
    );
    assert_eq!(
        duel_fields(&party, 0, b),
        (arbiter, 2),
        "the challenged's side"
    );
    party.cmd(1, RealmCommand::DuelCancel { arbiter });
    for i in 0..3 {
        assert!(
            flag_destroyed(party.seen(i), arbiter),
            "player {i} sees it go"
        );
    }
    assert_eq!(duel_fields(&party, 0, b), (0, 0), "no flag, no side");
}

#[test]
fn leaving_the_bounds_warns_and_coming_back_clears_it() {
    let mut party = players(2);
    fighting(&mut party);
    let here = position_of(&party, 1);
    party.go(1, [here[0] + 120.0, here[1], here[2]], 0.0);
    party.run(2.0);
    assert_eq!(bounds(party.seen(1)), [false], "warned");
    party.go(1, here, 0.0);
    party.run(0.3);
    assert_eq!(bounds(party.seen(1)), [false, true], "cleared");
    party.run(10.0);
    assert!(winners(party.seen(1)).is_empty(), "still fighting");
}

#[test]
fn staying_out_of_bounds_ten_seconds_loses_as_fled() {
    let mut party = players(2);
    fighting(&mut party);
    let here = position_of(&party, 1);
    party.go(1, [here[0] + 150.0, here[1], here[2]], 0.0);
    party.run(9.5);
    assert!(winners(party.seen(0)).is_empty(), "not yet");
    party.run(1.0);
    assert_eq!(
        winners(party.seen(0)),
        [(true, "Varian".to_string(), "Anduin".to_string())],
        "player 1 fled"
    );
    assert_eq!(completes(party.seen(1)), [true]);
}
