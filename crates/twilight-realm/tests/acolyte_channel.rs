//! Channelled skills and drain: Siphon Life runs its drain once a second for five seconds,
//! healing the caster as much as it deals, and a step ends it. Played through the realm's seam
//! with a test Priest Calling whose drain is small, so a camp mob outlives it; the shipped rows
//! are checked against the data.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, Effect};
use twilight_realm::{Realm, RealmCommand};

/// `siphon` a 5 s channel draining 2 a second; `bleed` spends half of max health, so the heals
/// show.
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
key = "siphon"
spell_id = 689
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
meta = { cast_kind = "channel", channel_ms = 5000, channel_tick_ms = 1000, interrupt_on = ["move", "pushback", "kick"] }
effects = [{ kind = "drain", amount = { base = 2.0 }, heal_pct = 100, school = "shadow" }]

[[calling.skill]]
key = "bleed"
spell_id = 2061
power_cost = 50
cooldown_ms = 0
gcd_ms = 0
target = "self"
meta = { cost_kind = "health" }
effects = []

[[calling.skill]]
key = "gorge"
spell_id = 699
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
meta = { cast_kind = "channel", channel_ms = 5000, channel_tick_ms = 1000, interrupt_on = ["move"] }
effects = [{ kind = "drain", amount = { base = 30.0 }, heal_pct = 100, school = "shadow", overheal_absorb_max_pct = 20, overheal_absorb_ms = 10000 }]

[[calling.skill]]
key = "pin"
spell_id = 339
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "root", aura_spell_id = 339, duration_ms = 20000 }]

[[calling.skill]]
key = "hurt"
spell_id = 172
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "periodic_damage", tick_ms = 1000, duration_ms = 1000, tick = { base = 200.0 }, school = "shadow" }]
"#;

/// The test Calling's drain whose overhealing grows a shield to 20% of max health, its root, and
/// its 200 Shadow tick on the caster, to weigh a shield with.
const TEST_GORGE: u32 = 699;
const TEST_PIN: u32 = 339;
const TEST_HURT: u32 = 172;

const SIPHON_LIFE: u32 = 689;
const BLEED: u32 = 2061;

/// A level 10 test Priest at half health, 20 yd from a camp mob: the realm, their guid, the mob
/// and the view.
fn world(seed: u64) -> (Realm, u64, Mob, View) {
    world_at(seed, 20.0, true)
}

/// A level 10 test Priest `yards` from a camp mob, at half health when `bled`.
fn world_at(seed: u64, yards: f32, bled: bool) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Vellune", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_priest\"");
    let mut realm = Realm::with_seed(seed).with_callings(TEST_CALLINGS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [CAMP[0] + 40.0, CAMP[1], 0.0],
        orientation: std::f32::consts::PI,
    });
    let mob = view
        .run(&mut realm, 1.0)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } if flat(*position, CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("a camp mob in view");
    stand_at(&mut realm, &mob, yards);
    if bled {
        realm.handle(cast(BLEED, None));
    }
    view.drain(&mut realm);
    (realm, guid, mob, view)
}

/// The drain's damage and heal amounts in `events`, in order.
fn drained(events: &[SessionEvent]) -> (Vec<u32>, Vec<u32>) {
    let mut dealt = Vec::new();
    let mut healed = Vec::new();
    for e in events {
        match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == SIPHON_LIFE => dealt.push(l.damage),
            SessionEvent::SpellHealLog(l) if l.spell_id == SIPHON_LIFE => healed.push(l.amount),
            _ => {}
        }
    }
    (dealt, healed)
}

/// `UNIT_FIELD_CHANNEL_OBJECT` and `UNIT_CHANNEL_SPELL`: the client draws a channel's beam from
/// them.
const CHANNEL_OBJECT: u16 = 20;
const CHANNEL_SPELL: u16 = 144;

/// The caster's channel spell and object, from the last values update in `events` that sets them.
fn channel_fields(events: &[SessionEvent], caster: u64) -> Option<(u32, u64)> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::ObjectValues { guid, fields } if *guid == caster => {
            let raw: Vec<(u16, u32)> = fields.raw_fields().collect();
            let get = |f: u16| raw.iter().find(|(k, _)| *k == f).map(|(_, v)| *v);
            let object =
                u64::from(get(CHANNEL_OBJECT)?) | u64::from(get(CHANNEL_OBJECT + 1)?) << 32;
            Some((get(CHANNEL_SPELL)?, object))
        }
        _ => None,
    })
}

#[test]
fn siphon_life_ticks_five_times_over_five_seconds_and_heals_what_it_deals() {
    let (mut realm, me, mob, mut view) = world(1);
    realm.handle(cast(SIPHON_LIFE, Some(mob.guid)));
    let started = view.drain(&mut realm);
    assert_eq!(refusal(&started, SIPHON_LIFE), None);
    assert_eq!(
        channel_fields(&started, me),
        Some((SIPHON_LIFE, mob.guid)),
        "the caster's channel fields name the spell and the mob, for the beam"
    );
    assert!(started.iter().any(|e| matches!(
        e,
        SessionEvent::ChannelStart {
            spell_id: SIPHON_LIFE,
            duration_ms: 5000
        }
    )));
    let early = view.run(&mut realm, 0.9);
    assert!(
        drained(&early).0.is_empty(),
        "the first tick is a second in"
    );
    let mut events = early;
    events.extend(view.run(&mut realm, 4.2));
    let (dealt, healed) = drained(&events);
    // A tick that missed would log nothing; none miss on this seed.
    assert_eq!(dealt.len(), 5, "five ticks: {dealt:?}");
    assert_eq!(dealt, healed, "each tick heals what it dealt");
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::ChannelUpdate { remaining_ms: 0 })));
    assert_eq!(
        channel_fields(&events, me),
        Some((0, 0)),
        "both cleared at the end"
    );
    assert!(
        drained(&view.run(&mut realm, 2.0)).0.is_empty(),
        "then it is over"
    );
}

#[test]
fn a_step_after_two_ticks_ends_the_channel() {
    let (mut realm, me, mob, mut view) = world(2);
    realm.handle(cast(SIPHON_LIFE, Some(mob.guid)));
    let started = view.drain(&mut realm);
    assert_eq!(channel_fields(&started, me), Some((SIPHON_LIFE, mob.guid)));
    let two = view.run(&mut realm, 2.1);
    assert_eq!(drained(&two).0.len(), 2);
    stand_at(&mut realm, &mob, 25.0);
    let broken = view.drain(&mut realm);
    assert_eq!(
        channel_fields(&broken, me),
        Some((0, 0)),
        "a broken channel clears both"
    );
    assert!(broken.iter().any(|e| matches!(
        e,
        SessionEvent::SpellFailedOther {
            spell_id: SIPHON_LIFE,
            ..
        }
    )));
    assert!(
        drained(&view.run(&mut realm, 4.0)).0.is_empty(),
        "no more ticks"
    );
}

#[test]
fn the_shipped_siphon_life_is_a_five_second_drain_and_blood_ties_waits_on_the_client() {
    let acolyte = callings::calling("acolyte").unwrap();
    let siphon = acolyte
        .skills
        .iter()
        .find(|s| s.key == "siphon_life")
        .unwrap();
    assert_eq!(siphon.meta.shape.cast_kind.as_deref(), Some("channel"));
    assert_eq!(siphon.meta.shape.channel_ms, 5000);
    assert_eq!(siphon.meta.shape.channel_tick_ms, 1000);
    assert!(matches!(
        siphon.effects[..],
        [Effect::Drain { heal_pct: 100, .. }]
    ));
}

/// The test Calling's drain at full health, 30 yd from a rooted camp mob: each tick's whole heal
/// is overhealing, and the shield it grows stops at 20% of max health, which a 200 tick on the
/// caster shows: it loses 200 less the shield. Without the shield it loses all 200.
#[test]
fn overhealing_grows_a_shield_that_stops_at_20_percent_of_max_health() {
    let (mut realm, me, mob, mut view) = world_at(4, 29.0, false);
    let max = view.fields(me).unit_max_health().unwrap();
    assert_eq!(view.health(me), max, "full health");
    realm.handle(cast(TEST_PIN, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), TEST_PIN), None);
    realm.handle(cast(TEST_GORGE, Some(mob.guid)));
    let mut events = view.drain(&mut realm);
    assert_eq!(refusal(&events, TEST_GORGE), None);
    events.extend(view.run(&mut realm, 3.05));
    let overhealed: u32 = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == TEST_GORGE => Some(l.damage),
            _ => None,
        })
        .sum();
    let cap = max * 20 / 100;
    assert!(
        overhealed > cap,
        "{overhealed} overhealed, past the {cap} cap"
    );
    assert_eq!(view.health(me), max, "still full");
    realm.handle(cast(TEST_HURT, None));
    assert_eq!(refusal(&view.drain(&mut realm), TEST_HURT), None);
    view.run(&mut realm, 1.05);
    assert_eq!(max - view.health(me), 200 - cap, "the shield soaked {cap}");

    let (mut realm, me, _, mut view) = world_at(4, 29.0, false);
    realm.handle(cast(TEST_HURT, None));
    view.drain(&mut realm);
    view.run(&mut realm, 1.05);
    assert_eq!(max - view.health(me), 200, "no shield");
}

// Siphon Life's Skill Choices (#508), played by a shipped Acolyte at 24 at the Test Grounds'
// dummies, which take hits but never fight back or die.

use common::acolyte::{acolyte, blight_stacks, dummy, row_choice, yard};

/// Where the Acolyte stands in the Test Grounds: 17.5 yd from the first dummy.
const YARD_SPOT: [f32; 2] = [-9655.0, 255.0];

/// For each 0.1 s step of a channel of `spell_id` at a Blighted dummy 0, the stacks the step added
/// past the Blight's own tick, and whether the drain hit in it.
fn siphon_growth(seed: u64, rows: &[(&str, u32)], spell_id: u32) -> Vec<(u32, bool)> {
    let mut y = yard(seed, rows, YARD_SPOT);
    let blight = acolyte("aura_creeping_blight");
    y.cast_through(acolyte("creeping_blight"), dummy(0));
    y.cast_through(spell_id, dummy(0));
    let mut steps = Vec::new();
    for _ in 0..55 {
        let before = blight_stacks(&y.view, dummy(0));
        let events = y.run(0.1);
        let ticked = events.iter().any(|e| {
            matches!(e, SessionEvent::PeriodicAuraLog(l) if l.spell_id == blight && l.target == dummy(0))
        });
        let hit = events.iter().any(|e| {
            matches!(e, SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id && l.damage > 0)
        });
        let after = blight_stacks(&y.view, dummy(0));
        steps.push((after - before - u32::from(ticked), hit));
    }
    steps
}

/// Each Feeding Siphon tick that hits adds a stack to the Blight on its target 30% of the time;
/// plain Siphon Life adds none.
#[test]
fn feeding_siphon_adds_a_blight_stack_on_a_winning_roll() {
    let feeding = row_choice("siphon_life", "feeding_siphon");
    let (mut grown, mut hits) = (0, 0);
    for seed in 0..12 {
        for (more, hit) in siphon_growth(seed, &[("siphon_life", feeding)], feeding) {
            assert!(more <= u32::from(hit), "a stack only on a tick that hits");
            grown += more;
            hits += u32::from(hit);
        }
    }
    assert!(
        grown > 0 && grown < hits,
        "{grown} of {hits} ticks grew the Blight"
    );

    for seed in 0..3 {
        let steps = siphon_growth(seed, &[], acolyte("siphon_life"));
        assert!(steps.iter().any(|s| s.1), "plain Siphon Life hits");
        assert!(steps.iter().all(|s| s.0 == 0), "and grows nothing");
    }
}

/// Gorging Siphon at full health, every tick overhealing, puts its shield on the Acolyte under its
/// own spell; plain Siphon Life puts none.
#[test]
fn gorging_siphon_at_full_health_shields_the_acolyte() {
    let gorging = row_choice("siphon_life", "gorging_siphon");
    let mut y = yard(13, &[("siphon_life", gorging)], YARD_SPOT);
    let me = y.me;
    assert_eq!(
        y.view.health(me),
        y.view.fields(me).unit_max_health().unwrap()
    );
    y.cast_through(gorging, dummy(0));
    y.run(1.05);
    assert!(y.view.aura(me, gorging).is_some(), "the shield is up");

    let siphon = acolyte("siphon_life");
    let mut y = yard(13, &[], YARD_SPOT);
    let me = y.me;
    y.cast_through(siphon, dummy(0));
    y.run(1.05);
    assert!(y.view.aura(me, siphon).is_none());
}

/// Unbroken Siphon goes on ticking when the Acolyte steps 3 yd after two ticks, and twice more:
/// the channel is not broken and its later ticks land (one may miss). Plain Siphon Life breaks at
/// the first step after its two ticks.
#[test]
fn unbroken_siphon_keeps_ticking_while_the_acolyte_moves() {
    let unbroken = row_choice("siphon_life", "unbroken_siphon");
    let ticks = |rows: &[(&str, u32)], spell_id: u32| -> (usize, bool) {
        let mut y = yard(14, rows, YARD_SPOT);
        let mut events = y.cast_through(spell_id, dummy(0));
        events.extend(y.run(2.1));
        events.extend(y.walk([YARD_SPOT[0], YARD_SPOT[1] - 3.0]));
        events.extend(y.run(1.0));
        events.extend(y.walk([YARD_SPOT[0] + 2.0, YARD_SPOT[1] - 3.0]));
        events.extend(y.run(3.0));
        let broken = events.iter().any(
            |e| matches!(e, SessionEvent::SpellFailedOther { spell_id: s, .. } if *s == spell_id),
        );
        let landed = events
            .iter()
            .filter(|e| matches!(e, SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id))
            .count();
        (landed, broken)
    };
    let (landed, broken) = ticks(&[("siphon_life", unbroken)], unbroken);
    assert!(!broken && landed >= 4, "{landed} ticks, broken {broken}");
    let (landed, broken) = ticks(&[], acolyte("siphon_life"));
    assert!(broken && landed <= 2, "{landed} ticks, broken {broken}");
}
