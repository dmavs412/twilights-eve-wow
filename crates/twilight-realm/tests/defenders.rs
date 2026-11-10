//! Defenders (#700): a friendly creature that hostile creatures notice and fight, that fights
//! back, that players heal, and whose fight ends where it stands. The defender is Corporal
//! Keeshan's template, the hostile a Prairie Wolf, both made with `Realm::spawn_defender_for` and
//! `Realm::spawn_for` far out past The Barrens' camps.

mod common;

use benilla_protocol::SessionEvent;
use common::party::*;
use common::*;
use twilight_realm::{Audience, CROSSROADS};

/// Far out past The Barrens' camps: nothing else streams in.
const FAR_AWAY: [f32; 3] = [CROSSROADS[0] + 3000.0, CROSSROADS[1], 0.0];
const PRAIRIE_WOLF: u32 = 2958;
const KEESHAN: u32 = 349;
const MEND: u32 = 2061;
const HUNT: u32 = 99870;

/// A Calling with one instant ally heal, as `tests/shared_world.rs` makes `test_healer`.
const HEALERS: &str = r#"
[[calling]]
key = "test_mender"
name = "Test Mender"
tier = 0
class = 5
cap = 60
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "mend"
spell_id = 2061
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "ally"
range_yd = [0.0, 40.0]
effects = [{ kind = "heal", amount = { base = 40.0 } }]
"#;

/// A quest that counts a wolf's kill and a Keeshan's.
const QUESTS: &str = r#"
[[quest]]
id = 99870
kind = "story"
giver = 197
level = 5
title = "Teeth in the Dark"
objectives = "-"
details = "-"
progress = "-"
completion = "-"
money = 0
kill = [[2958, 1], [349, 1]]
"#;

fn at(dx: f32) -> [f32; 3] {
    [FAR_AWAY[0] + dx, FAR_AWAY[1], 0.0]
}

/// Characters of `levels`, each holding the kill quest, the one at `healer` a Test Mender, and
/// each at `at(spots[i])`.
fn party(levels: &[u8], healer: Option<usize>, spots: &[f32]) -> Party {
    let realm = realm().with_callings(HEALERS).with_quests(QUESTS);
    let mut party = Party::edited(realm, levels, 0, Box::new(NoGround), move |i, blob| {
        let mut blob = if Some(i) == healer {
            blob.replace("calling = \"basic_warrior\"", "calling = \"test_mender\"")
        } else {
            blob
        };
        blob.push_str(&format!(
            "\n[[quests]]\nid = {HUNT}\nslot = 0\nkills = [0, 0, 0, 0]\n"
        ));
        blob
    });
    for (i, &dx) in spots.iter().enumerate() {
        party.go(i, at(dx), 0.0);
    }
    // A tick for the characters to come out on the map their position is on.
    party.run(0.5);
    party
}

/// A wolf of `level` at `at(dx)` for `audience`.
fn wolf(party: &mut Party, level: u8, dx: f32, audience: &Audience) -> u64 {
    let map = party.realm.player_positions()[0].0;
    party
        .realm
        .spawn_for(
            party.host.as_ref(),
            PRAIRIE_WOLF,
            level,
            at(dx),
            map,
            audience.clone(),
        )
        .expect("a wolf")
}

/// A defender of `level` at `at(dx)` for `audience`.
fn defender(party: &mut Party, level: u8, dx: f32, audience: &Audience) -> u64 {
    let map = party.realm.player_positions()[0].0;
    party
        .realm
        .spawn_defender_for(
            party.host.as_ref(),
            KEESHAN,
            level,
            at(dx),
            map,
            audience.clone(),
        )
        .expect("a defender")
}

/// Whether `events` show `attacker` turning on `victim`.
fn attack_started(events: &[SessionEvent], attacker: u64, victim: u64) -> bool {
    events.iter().any(|e| {
        matches!(e, SessionEvent::AttackStart { attacker: a, victim: v }
            if *a == attacker && *v == victim)
    })
}

/// The health `events` last show `guid` at.
fn health(events: &[SessionEvent], guid: u64) -> Option<u32> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::ObjectValues { guid: g, fields } if *g == guid => fields.unit_health(),
        SessionEvent::ObjectCreate {
            guid: g, fields, ..
        } if *g == guid => fields.unit_health(),
        _ => None,
    })
}

/// The maximum health `guid` was created with.
fn max_health(events: &[SessionEvent], guid: u64) -> u32 {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g, fields, ..
            } if *g == guid => fields.unit_max_health(),
            _ => None,
        })
        .expect("its create")
}

/// Whether `events` credit `entry`'s kill to the hunt quest.
fn kill_counted(events: &[SessionEvent], entry: u32) -> bool {
    events.iter().any(|e| {
        matches!(e, SessionEvent::QuestObjectiveKill { quest_id, entry: k, .. }
            if *quest_id == HUNT && *k == entry)
    })
}

fn healed(events: &[SessionEvent], target: u64, healer: u64) -> bool {
    events.iter().any(
        |e| matches!(e, SessionEvent::SpellHealLog(l) if l.target == target && l.healer == healer),
    )
}

fn moved(events: &[SessionEvent], guid: u64) -> bool {
    events.iter().any(
        |e| matches!(e, SessionEvent::MonsterMove { guid: g, stop, .. } if *g == guid && !stop),
    )
}

/// Ticks in 0.1 s steps until `done` holds for what player `i`'s client has been sent in all, at
/// most `secs`; whether it did.
fn until(party: &mut Party, i: usize, secs: f32, done: impl Fn(&[SessionEvent]) -> bool) -> bool {
    let mut t = 0.0;
    while t < secs {
        party.run(0.1);
        if done(party.seen(i)) {
            return true;
        }
        t += 0.1;
    }
    false
}

/// One character 60 yards off, a level 30 defender, and a level 30 wolf 10 yards from it, both
/// for the character.
fn duel() -> (Party, u64, u64) {
    let mut party = party(&[30], None, &[-60.0]);
    let audience = Audience::of(&[party.guids[0]]);
    let keeshan = defender(&mut party, 30, 0.0, &audience);
    let wolf = wolf(&mut party, 30, 10.0, &audience);
    (party, keeshan, wolf)
}

#[test]
fn a_wolf_attacks_a_defender_near_it_and_the_defender_swings_back() {
    let (mut party, keeshan, wolf) = duel();
    let both = until(&mut party, 0, 10.0, |seen| {
        swung_at(seen, wolf).contains(&keeshan) && swung_at(seen, keeshan).contains(&wolf)
    });
    assert!(both, "the wolf and the defender swing at each other");
}

#[test]
fn a_character_who_stands_off_is_never_turned_on() {
    let (mut party, keeshan, wolf) = duel();
    let me = party.guids[0];
    party.run(20.0);
    let seen = party.seen(0);
    assert!(swung_at(seen, wolf).contains(&keeshan), "the fight was on");
    for unit in [keeshan, wolf] {
        assert!(!attack_started(seen, unit, me));
        assert!(!swung_at(seen, unit).contains(&me));
    }
}

#[test]
fn a_defender_kills_a_wolf_and_the_character_near_gets_the_credit() {
    let mut party = party(&[30], None, &[-40.0]);
    let audience = Audience::of(&[party.guids[0]]);
    let _keeshan = defender(&mut party, 42, 0.0, &audience);
    let wolf = wolf(&mut party, 5, 3.0, &audience);
    assert!(
        until(&mut party, 0, 30.0, |seen| died(seen, wolf)),
        "the wolf died"
    );
    party.run(0.5);
    assert!(kill_counted(party.seen(0), PRAIRIE_WOLF));
}

#[test]
fn a_defender_that_dies_pays_nothing_and_its_wolf_turns_on_the_healer() {
    let mut party = party(&[30], Some(0), &[-30.0]);
    let healer = party.guids[0];
    let audience = Audience::of(&[healer]);
    let keeshan = defender(&mut party, 5, 0.0, &audience);
    let wolf = wolf(&mut party, 30, 10.0, &audience);
    party.run(0.1);
    let full = max_health(party.seen(0), keeshan);
    // Hurt, so the heal heals.
    assert!(until(&mut party, 0, 15.0, |seen| health(seen, keeshan)
        .is_some_and(|h| h + 40 < full)));
    party.cmd(
        0,
        RealmCommand::CastSpell {
            spell_id: MEND,
            target: Some(keeshan),
        },
    );
    party.run(0.3);
    assert!(healed(party.seen(0), keeshan, healer), "the heal landed");
    assert!(
        until(&mut party, 0, 90.0, |seen| died(seen, keeshan)),
        "the defender died"
    );
    assert!(
        until(&mut party, 0, 10.0, |seen| attack_started(
            seen, wolf, healer
        )),
        "the wolf turned on the healer"
    );
    let seen = party.seen(0);
    assert!(!kill_counted(seen, KEESHAN));
    assert!(!seen
        .iter()
        .any(|e| matches!(e, SessionEvent::XpGain(x) if x.victim == keeshan)));
}

#[test]
fn a_heal_lands_on_a_hurt_defender_who_never_turns_on_the_healer() {
    let mut party = party(&[30], Some(0), &[-30.0]);
    let healer = party.guids[0];
    let audience = Audience::of(&[healer]);
    let keeshan = defender(&mut party, 30, 0.0, &audience);
    let wolf = wolf(&mut party, 30, 10.0, &audience);
    party.run(0.1);
    let full = max_health(party.seen(0), keeshan);
    assert!(
        until(&mut party, 0, 15.0, |seen| health(seen, keeshan)
            .is_some_and(|h| h + 40 < full)),
        "the wolf hurt the defender"
    );
    let before = health(party.seen(0), keeshan).unwrap();
    let mark = party.seen(0).len();
    party.cmd(
        0,
        RealmCommand::CastSpell {
            spell_id: MEND,
            target: Some(keeshan),
        },
    );
    party.run(0.3);
    let since = &party.seen(0)[mark..];
    let at = since
        .iter()
        .position(|e| {
            matches!(e, SessionEvent::SpellHealLog(l)
                if l.target == keeshan && l.healer == healer && l.amount > 0)
        })
        .expect("a SpellHealLog naming the defender");
    // The health it is sent with the heal, before the wolf's next blow.
    let after = health(&since[..at + 2], keeshan);
    assert!(after.is_some_and(|h| h > before), "its health went up");
    party.run(10.0);
    let seen = party.seen(0);
    assert!(!attack_started(seen, keeshan, healer));
    assert!(!swung_at(seen, keeshan).contains(&healer));
    let _ = wolf;
}

#[test]
fn a_wolf_never_notices_a_defender_of_another_audience() {
    let mut party = party(&[30, 30], None, &[-60.0, -62.0]);
    let (first, second) = (
        Audience::of(&[party.guids[0]]),
        Audience::of(&[party.guids[1]]),
    );
    let wolf = wolf(&mut party, 30, 10.0, &first);
    let keeshan = defender(&mut party, 30, 0.0, &second);
    party.run(10.0);
    for i in 0..2 {
        assert!(!attack_started(party.seen(i), wolf, keeshan));
        assert!(!swung_at(party.seen(i), wolf).contains(&keeshan));
    }
}

#[test]
fn a_defender_whose_fight_is_over_stays_where_it_stopped_and_keeps_its_health() {
    let mut party = party(&[30], None, &[-60.0]);
    let audience = Audience::of(&[party.guids[0]]);
    let keeshan = defender(&mut party, 34, 0.0, &audience);
    let wolf = wolf(&mut party, 30, 10.0, &audience);
    party.run(0.1);
    let full = max_health(party.seen(0), keeshan);
    assert!(
        until(&mut party, 0, 120.0, |seen| died(seen, wolf)),
        "the defender won"
    );
    party.run(0.5);
    let left = health(party.seen(0), keeshan).expect("its health");
    assert!(left < full, "it was hurt");
    let mark = party.seen(0).len();
    party.run(15.0);
    let after = &party.seen(0)[mark..];
    assert!(!moved(after, keeshan), "no evade move");
    assert_eq!(
        health(party.seen(0), keeshan),
        Some(left),
        "no heal to full"
    );
}
