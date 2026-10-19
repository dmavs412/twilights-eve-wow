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
meta = { cast_kind = "channel", channel_ms = 5000, channel_tick_ms = 1000 }
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
"#;

const SIPHON_LIFE: u32 = 689;
const BLEED: u32 = 2061;

/// A level 10 test Priest at half health, 20 yd from a camp mob: the realm, their guid, the mob
/// and the view.
fn world(seed: u64) -> (Realm, u64, Mob, View) {
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
    stand_at(&mut realm, &mob, 20.0);
    realm.handle(cast(BLEED, None));
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

#[test]
fn siphon_life_ticks_five_times_over_five_seconds_and_heals_what_it_deals() {
    let (mut realm, _, mob, mut view) = world(1);
    realm.handle(cast(SIPHON_LIFE, Some(mob.guid)));
    let started = view.drain(&mut realm);
    assert_eq!(refusal(&started, SIPHON_LIFE), None);
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
    assert!(
        drained(&view.run(&mut realm, 2.0)).0.is_empty(),
        "then it is over"
    );
}

#[test]
fn a_step_after_two_ticks_ends_the_channel() {
    let (mut realm, _, mob, mut view) = world(2);
    realm.handle(cast(SIPHON_LIFE, Some(mob.guid)));
    view.drain(&mut realm);
    let two = view.run(&mut realm, 2.1);
    assert_eq!(drained(&two).0.len(), 2);
    stand_at(&mut realm, &mob, 25.0);
    let broken = view.drain(&mut realm);
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
    let blood_ties = acolyte
        .talent_tabs
        .iter()
        .flat_map(|t| &t.talents)
        .find(|t| t.key == "blood_ties")
        .unwrap();
    assert!(
        blood_ties.effects.is_empty(),
        "the client cannot target an ally"
    );
}
