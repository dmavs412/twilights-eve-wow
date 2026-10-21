//! Mobs fight any player: a mob swings at the player it fights and at no one else, turns on the
//! latest one who hurt it when its target dies, and with no one left runs home.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::{cast, refusal, View};
use common::party::*;
use common::*;

/// A Razormane Hunter at Thorn Hill, levels 11–12: its camp kills a level 1 warrior.
const HUNTER: u32 = 3265;
const THORN_HILL: [f32; 3] = [-38.801, -3234.81, 91.889];
const CORRUPTION: &str = "corruption_creature";
static GOUGE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("basic_warrior", "gouge"));
/// Whole rage points Gouge costs.
const GOUGE_RAGE: u32 = 15;

fn north_of(p: [f32; 3], yd: f32) -> [f32; 3] {
    [p[0] + yd, p[1], p[2]]
}

/// A level 1 (player 0) at the camp, fighting the hunter, and a level 20 (player 1) `off` yards
/// from it.
fn pulled(realm: Realm, off: f32) -> (Party, u64) {
    pulled_at(realm, 1, off)
}

fn pulled_at(realm: Realm, level: u8, off: f32) -> (Party, u64) {
    let mut party = Party::new(realm, &[level, 20]);
    party.go(1, north_of(THORN_HILL, off), 0.0);
    party.go(0, THORN_HILL, 0.0);
    party.run(0.0);
    let (hunter, _) = party.unit(0, HUNTER, THORN_HILL);
    let a = party.guids[0];
    assert!(
        party.seen(0).iter().any(|e| matches!(e,
            SessionEvent::AttackStart { attacker, victim } if *attacker == hunter && *victim == a)),
        "the hunter turns on the one who came close"
    );
    (party, hunter)
}

#[test]
fn a_mob_swings_at_its_target_only_then_turns_on_the_next_who_hurt_it() {
    let (mut party, hunter) = pulled(Realm::with_seed(3), 60.0);
    let (a, b) = (party.guids[0], party.guids[1]);

    // The level 20 lands one blow and stops.
    let at = party.unit(1, HUNTER, THORN_HILL).1;
    party.go_facing(1, north_of(at, 1.5), at);
    party.cmd(1, RealmCommand::AttackSwing { guid: hunter });
    let hit = party.run_until(20.0, |got| {
        got.iter().any(|e| swung_at(e, b).contains(&hunter))
    });
    assert!(hit, "the second player hurts the hunter");
    party.cmd(1, RealmCommand::AttackStop);

    let a_died = party.run_until(120.0, |got| died(&got[0], a));
    assert!(a_died, "the camp kills the level 1");
    let before: Vec<u64> = swung_at(party.seen(1), hunter);
    assert!(
        before.iter().all(|&v| v == a),
        "it swung at its target only"
    );

    let turned = party.run_until(10.0, |got| {
        got[1].iter().any(|e| {
            matches!(e,
            SessionEvent::AttackStart { attacker, victim } if *attacker == hunter && *victim == b)
        })
    });
    assert!(turned, "it turns on the one who hurt it");
    let swung = party.run_until(20.0, |got| swung_at(&got[1], hunter).contains(&b));
    assert!(swung, "and swings at them");
}

#[test]
fn with_no_one_else_who_hurt_it_a_mob_runs_home() {
    let (mut party, hunter) = pulled(Realm::with_seed(3), 40.0);
    let (a, b) = (party.guids[0], party.guids[1]);
    assert!(
        party.run_until(120.0, |got| died(&got[0], a)),
        "the level 1 dies"
    );
    let after = party.run(15.0);
    assert!(
        !after[1].iter().any(|e| matches!(e,
            SessionEvent::AttackStart { attacker, victim } if *attacker == hunter && *victim == b)),
        "the one who never hurt it is left alone"
    );
    let max = party
        .seen(0)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate { guid, fields, .. } if *guid == hunter => {
                fields.unit_max_health()
            }
            _ => None,
        })
        .unwrap();
    let healed = after.iter().flatten().any(|e| {
        matches!(e, SessionEvent::ObjectValues { guid, fields }
            if *guid == hunter && fields.unit_health() == Some(max))
    });
    assert!(healed, "it runs home and heals");
}

/// Player 0, level 20 and fighting the hunter at the camp, builds Gouge's rage and gouges it; the
/// hunter's guid and its max health.
fn gouged(seed: u64) -> (Party, u64, u32) {
    let (mut party, hunter) = pulled_at(Realm::with_seed(seed), 20, 60.0);
    let max = max_health(party.seen(0), hunter);
    party.cmd(0, RealmCommand::AttackSwing { guid: hunter });
    for _ in 0..600 {
        if rage_of(&party, 0) >= GOUGE_RAGE {
            break;
        }
        party.run(0.1);
    }
    assert!(rage_of(&party, 0) >= GOUGE_RAGE, "rage for Gouge");
    party.cmd(0, RealmCommand::AttackStop);
    party.cmd(0, cast(*GOUGE, Some(hunter)));
    assert_eq!(refusal(party.seen(0), *GOUGE), None, "gouge is cast");
    (party, hunter, max)
}

#[test]
fn a_gouged_mob_hit_by_a_second_player_stays_in_the_fight_unhealed() {
    let (mut party, hunter, max) = gouged(3);
    let b = party.guids[1];
    let at = party.unit(1, HUNTER, THORN_HILL).1;
    party.go_facing(1, north_of(at, 1.5), at);
    party.cmd(1, RealmCommand::AttackSwing { guid: hunter });
    let hit = party.run_until(20.0, |got| swung_at(&got[1], b).contains(&hunter));
    assert!(hit, "the second player hurts the hunter");
    let after = party.run(20.0);
    // Until the two of them kill it.
    let all: Vec<&SessionEvent> = after[0]
        .iter()
        .take_while(|e| !died(std::slice::from_ref(*e), hunter))
        .collect();
    assert!(
        !all.iter().any(|e| matches!(e,
            SessionEvent::AttackStop { attacker, .. } if *attacker == hunter)),
        "it never leaves the fight"
    );
    assert!(
        !all.iter()
            .any(|e| matches!(e, SessionEvent::ObjectValues { guid, fields }
            if *guid == hunter && fields.unit_health() == Some(max))),
        "it never resets to full health"
    );
    assert!(died(&after[0], hunter), "and they fight it to the death");
}

#[test]
fn a_mob_resets_only_once_both_players_are_out_of_leash_range() {
    let (mut party, hunter, max) = gouged(3);
    let b = party.guids[1];
    let at = party.unit(1, HUNTER, THORN_HILL).1;
    party.go_facing(1, north_of(at, 1.5), at);
    party.cmd(1, RealmCommand::AttackSwing { guid: hunter });
    assert!(party.run_until(20.0, |got| swung_at(&got[1], b).contains(&hunter)));
    party.cmd(1, RealmCommand::AttackStop);

    // Player 0 runs off past the leash: the hunter turns on player 1 and stays in the fight.
    party.cmd(0, RealmCommand::AttackStop);
    party.go(0, north_of(THORN_HILL, -200.0), 0.0);
    // Long enough for Gouge to end and the hunter to chase player 0 out to its leash.
    let first = party.run(25.0);
    assert!(
        !first.iter().flatten().any(|e| matches!(e,
            SessionEvent::AttackStop { attacker, .. } if *attacker == hunter)),
        "one player left in reach holds it"
    );

    // Now player 1 runs off too: it gives up, runs home and heals.
    party.go(1, north_of(THORN_HILL, 200.0), 0.0);
    let healed = party.run_until(60.0, |got| {
        got.iter().flatten().any(|e| {
            matches!(e, SessionEvent::ObjectValues { guid, fields }
            if *guid == hunter && fields.unit_health() == Some(max))
        })
    });
    assert!(healed, "with no one in reach it resets");
}

/// `hunter`'s max health from its create in `events`.
fn max_health(events: &[SessionEvent], hunter: u64) -> u32 {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate { guid, fields, .. } if *guid == hunter => {
                fields.unit_max_health()
            }
            _ => None,
        })
        .unwrap()
}

/// Player `i`'s rage, in whole points, as their client has it.
fn rage_of(party: &Party, i: usize) -> u32 {
    let mut view = View::default();
    view.see(party.seen(i));
    view.rage(party.guids[i])
}

#[test]
fn a_creature_spells_damage_lands_on_its_target_alone() {
    let realm = Realm::with_seed(3).with_creature_spells(HUNTER, &[CORRUPTION]);
    // Sturdy enough to outlast the cast and a tick.
    let (mut party, hunter) = pulled_at(realm, 8, 40.0);
    let (a, b) = (party.guids[0], party.guids[1]);
    party.run_until(60.0, |got| died(&got[0], a));
    let ticks: Vec<u64> = party
        .seen(0)
        .iter()
        .chain(party.seen(1))
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.caster == hunter => Some(l.target),
            _ => None,
        })
        .collect();
    assert!(!ticks.is_empty(), "the curse ticks");
    assert!(ticks.iter().all(|&t| t == a), "on its target: {ticks:?}");
    assert!(!ticks.contains(&b));
}
