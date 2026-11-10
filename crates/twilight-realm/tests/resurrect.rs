//! Resurrection (#682): a test Priest brings a dead friend back where they lie, at a share of
//! their health or of the health the cast cost, once the friend accepts the stock popup. And a
//! heal over time on a friend paid by the health the cast cost. Two players, each in their own
//! session, at the Crossroads.

mod common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::SessionEvent;
use common::fight::*;
use common::party::{NoGround, Party};
use twilight_realm::{Realm, RealmCommand, CROSSROADS};

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_priest"
name = "Test Priest"
tier = 0
class = 5
cap = 10
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "resurrect"
spell_id = 2006
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "ally"
range_yd = [0.0, 30.0]
requires_out_of_combat = true
effects = [{ kind = "resurrect", health_pct = 30 }]

[[calling.skill]]
key = "blood_rite"
spell_id = 2010
power_cost = 20
cooldown_ms = 0
gcd_ms = 0
target = "ally"
range_yd = [0.0, 30.0]
requires_out_of_combat = true
meta = { cost_kind = "health" }
effects = [{ kind = "resurrect", pct_of = "health_cost", pct = 200 }]

[[calling.skill]]
key = "blood_renew"
spell_id = 139
power_cost = 15
cooldown_ms = 0
gcd_ms = 0
target = "ally"
range_yd = [0.0, 40.0]
meta = { cost_kind = "health" }
effects = [{ kind = "periodic_heal", tick_ms = 2000, duration_ms = 6000, pct_of = "health_cost", pct = 150, aura_spell_id = 6074 }]

[[calling.skill]]
key = "war_cry"
spell_id = 2687
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
puts_in_combat_ms = 10000
effects = []

[[calling.aura]]
key = "blood_renewal"
spell_id = 6074
duration_ms = 6000
effects = [{ kind = "periodic_heal", tick_ms = 2000, pct_of = "health_cost", pct = 150 }]
"#;

const RESURRECT: u32 = 2006;
const BLOOD_RITE: u32 = 2010;
const BLOOD_RENEW: u32 = 139;
/// The aura row the heal over time goes on under, as the build's `apply_aura` names it.
const BLOOD_RENEWAL: u32 = 6074;
const WAR_CRY: u32 = 2687;

const SPELL_FAILED_AFFECTING_COMBAT: u8 = 0x00;
const SPELL_FAILED_TARGET_NOT_DEAD: u8 = 0x6e;

/// Where player 0 stands, and player 1 10 yd east of them.
fn spot(i: usize) -> [f32; 3] {
    [
        CROSSROADS[0] + 10.0 * i as f32,
        CROSSROADS[1],
        CROSSROADS[2],
    ]
}

/// Two test Priests of `levels`, standing at their spots.
fn priests(levels: [u8; 2]) -> Party {
    let realm = Realm::with_seed(3).with_callings(TEST_CALLINGS);
    let mut party = Party::edited(realm, &levels, 0, Box::new(NoGround), |_, blob| {
        blob.replace("calling = \"basic_warrior\"", "calling = \"test_priest\"")
    });
    party.go(0, spot(0), 0.0);
    party.go(1, spot(1), std::f32::consts::PI);
    party.run(0.5);
    party
}

/// Player 1 dies where they stand, as a killing blow would kill them, and lies there.
fn kill_b(party: &mut Party) {
    party.go(1, spot(1), std::f32::consts::PI);
    party.realm.kill_player();
    party.run(0.2);
}

/// What player `i`'s client knows of `guid`.
fn view(party: &Party, i: usize) -> View {
    let mut v = View::default();
    v.see(party.seen(i));
    v
}

fn health(party: &Party, i: usize) -> u32 {
    view(party, i).health(party.guids[i])
}

fn max_health(party: &Party, i: usize) -> u32 {
    view(party, i)
        .fields(party.guids[i])
        .unit_max_health()
        .unwrap()
}

/// The resurrection offers in `events`, by caster.
fn offers(events: &[SessionEvent]) -> Vec<u64> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ResurrectRequest { caster, .. } => Some(*caster),
            _ => None,
        })
        .collect()
}

/// Player 0 casts `spell` on player 1, and the offer has a tick to reach them: what player 1 got.
fn offer_to_b(party: &mut Party, spell: u32) -> Vec<SessionEvent> {
    let b = party.guids[1];
    party.cmd(0, cast(spell, Some(b)));
    assert_eq!(refusal(party.seen(0), spell), None, "the cast goes off");
    party.run(0.3).swap_remove(1)
}

fn answer(party: &mut Party, accept: bool) {
    let caster = party.guids[0];
    party.cmd(1, RealmCommand::ResurrectResponse { caster, accept });
    party.run(0.2);
}

/// Whether player 1 stands where they died.
fn b_stands_where_they_fell(party: &Party) -> bool {
    party
        .realm
        .player_positions()
        .iter()
        .any(|&(_, at)| flat(at, spot(1)) < 0.1)
}

#[test]
fn an_accepted_resurrection_stands_the_friend_up_where_they_lay_at_30_percent() {
    let mut party = priests([20, 20]);
    kill_b(&mut party);
    assert_eq!(health(&party, 1), 0, "player 1 lies dead");
    let got = offer_to_b(&mut party, RESURRECT);
    assert_eq!(offers(&got), [party.guids[0]], "the offer names player 0");
    answer(&mut party, true);
    let max = max_health(&party, 1);
    assert_eq!(health(&party, 1), max * 30 / 100, "back at 30% health");
    assert!(
        b_stands_where_they_fell(&party),
        "not moved to the respawn point"
    );
    // Alive, the offer is spent: a second answer does nothing.
    answer(&mut party, true);
    assert_eq!(health(&party, 1), max * 30 / 100);
}

#[test]
fn a_declined_resurrection_leaves_the_friend_dead() {
    let mut party = priests([20, 20]);
    kill_b(&mut party);
    offer_to_b(&mut party, RESURRECT);
    answer(&mut party, false);
    assert_eq!(health(&party, 1), 0);
    // The offer is gone: a later accept does nothing.
    answer(&mut party, true);
    assert_eq!(health(&party, 1), 0);
}

#[test]
fn an_answer_after_release_does_nothing() {
    let mut party = priests([20, 20]);
    kill_b(&mut party);
    offer_to_b(&mut party, RESURRECT);
    party.cmd(1, RealmCommand::RepopRequest);
    party.run(0.2);
    let max = max_health(&party, 1);
    assert_eq!(health(&party, 1), max, "released at full health");
    party.cmd(
        1,
        RealmCommand::Move {
            motion: Default::default(),
            position: spot(1),
            orientation: 0.0,
        },
    );
    party.realm.set_player_health(10);
    party.run(0.1);
    answer(&mut party, true);
    assert!(
        health(&party, 1) < max * 30 / 100,
        "the old offer does not heal them"
    );
}

#[test]
fn a_resurrection_on_a_living_friend_is_refused_and_costs_nothing() {
    let mut party = priests([20, 20]);
    let before = health(&party, 0);
    let b = party.guids[1];
    party.cmd(0, cast(BLOOD_RITE, Some(b)));
    assert_eq!(
        refusal(party.seen(0), BLOOD_RITE),
        Some(SPELL_FAILED_TARGET_NOT_DEAD)
    );
    party.run(0.3);
    assert_eq!(health(&party, 0), before, "no health paid");
    assert!(offers(party.seen(1)).is_empty());
}

#[test]
fn a_resurrection_in_combat_is_refused_and_costs_nothing() {
    let mut party = priests([20, 20]);
    kill_b(&mut party);
    party.cmd(0, cast(WAR_CRY, None));
    party.run(0.2);
    let before = health(&party, 0);
    let b = party.guids[1];
    party.cmd(0, cast(BLOOD_RITE, Some(b)));
    assert_eq!(
        refusal(party.seen(0), BLOOD_RITE),
        Some(SPELL_FAILED_AFFECTING_COMBAT)
    );
    party.run(0.3);
    assert_eq!(health(&party, 0), before, "no health paid");
    assert!(offers(party.seen(1)).is_empty());
}

#[test]
fn a_blood_resurrection_returns_the_friend_with_twice_what_it_cost() {
    let mut party = priests([20, 20]);
    kill_b(&mut party);
    let before = health(&party, 0);
    let got = offer_to_b(&mut party, BLOOD_RITE);
    assert_eq!(offers(&got), [party.guids[0]]);
    let paid = before - health(&party, 0);
    assert!(paid > 0, "the cast cost health");
    answer(&mut party, true);
    assert_eq!(health(&party, 1), paid * 2);
}

#[test]
fn a_blood_resurrection_stops_at_the_friends_maximum() {
    // A level 60 caster's 20% of their health is more than half a level 10 friend's whole.
    let mut party = priests([60, 10]);
    kill_b(&mut party);
    let before = health(&party, 0);
    offer_to_b(&mut party, BLOOD_RITE);
    let paid = before - health(&party, 0);
    let max = max_health(&party, 1);
    assert!(
        paid * 2 > max,
        "the share passes the maximum: {paid} paid, {max} max"
    );
    answer(&mut party, true);
    assert_eq!(health(&party, 1), max);
}

#[test]
fn a_heal_over_time_on_a_friend_heals_150_percent_of_the_health_it_cost() {
    let mut party = priests([20, 20]);
    party.go(1, spot(1), std::f32::consts::PI);
    party.realm.set_player_health(10);
    party.run(0.2);
    let before = health(&party, 0);
    let b = party.guids[1];
    party.cmd(0, cast(BLOOD_RENEW, Some(b)));
    assert_eq!(refusal(party.seen(0), BLOOD_RENEW), None);
    let paid = before - health(&party, 0);
    assert!(paid > 0, "the cast cost health");
    let got = party.run(6.5).swap_remove(1);
    let ticks: Vec<u32> = got
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(log) if log.spell_id == BLOOD_RENEWAL => {
                match log.ticks[..] {
                    [PeriodicTick::Heal { amount }] => Some(amount),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect();
    assert_eq!(ticks.len(), 3, "a tick every 2 s for 6 s: {ticks:?}");
    let healed: u32 = ticks.iter().sum();
    let want = paid * 150 / 100;
    assert!(
        healed.abs_diff(want) < 3,
        "healed {healed}, want 150% of the {paid} paid"
    );
}
