//! Catalyst consumes the Initiate's auras on a target: a Cold shatters for a multiple of Biting
//! Cold's hit, an Ember refunds mana, and with nothing to consume it does nothing. Played through
//! the realm's seam with a test Mage Calling whose numbers are easy to read.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, Effect};
use twilight_realm::rules::school::School;
use twilight_realm::{Realm, RealmCommand};

const MANA: u8 = 0;

/// `cold` a hit of 1 with a snare, `ember` a burn, `catalyst` the consume (a shatter of 200% of a
/// 20 hit, a 20% refund) and `drain` spends 10 mana.
const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_mage"
name = "Test Mage"
tier = 0
class = 8
cap = 10
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "cold"
spell_id = 116
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [
    { kind = "school_damage", school = "frost", amount = { base = 1.0 } },
    { kind = "mod_run_speed", pct = -50, duration_ms = 5000 },
]

[[calling.skill]]
key = "ember"
spell_id = 133
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "periodic_damage", tick_ms = 3000, duration_ms = 12000, tick = { base = 2.0 }, max_stacks = 3 }]

[[calling.skill]]
key = "catalyst"
spell_id = 8921
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [
    { kind = "consume_aura", aura_spell_id = 116, duration_ms = 5000, amount = { base = 20.0 }, school = "frost", amount_pct = 200 },
    { kind = "consume_aura", aura_spell_id = 133, duration_ms = 12000, energize_pct_of_max = 20 },
]

[[calling.skill]]
key = "drain"
spell_id = 2912
power_cost = 10
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = []
"#;

const COLD: u32 = 116;
const EMBER: u32 = 133;
const CATALYST: u32 = 8921;
const DRAIN: u32 = 2912;

/// A level 10 test Mage at the scorpid camp, at `seed`: the realm, their guid, the camp's mobs
/// and the view.
fn world(seed: u64) -> (Realm, u64, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Weaver", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_mage\"");
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
    let mobs: Vec<Mob> = view
        .run(&mut realm, 1.0)
        .iter()
        .filter_map(|e| match e {
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
        .collect();
    assert!(!mobs.is_empty(), "the camp is in view");
    (realm, guid, mobs, view)
}

/// Casts `spell_id` at `target` (or at the caster): the events, after asserting it went.
fn go(realm: &mut Realm, view: &mut View, spell_id: u32, target: Option<u64>) -> Vec<SessionEvent> {
    realm.handle(cast(spell_id, target));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "spell {spell_id} is cast");
    events
}

/// The damage `spell_id` logged on `target`.
fn damage(events: &[SessionEvent], spell_id: u32, target: u64) -> Option<u32> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id && l.target == target => {
            Some(l.damage)
        }
        _ => None,
    })
}

/// The mana `spell_id` gave back.
fn refund(events: &[SessionEvent], spell_id: u32) -> Option<u32> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellEnergizeLog(l) if l.spell_id == spell_id => Some(l.amount),
        _ => None,
    })
}

/// Catalyst on a Cold, then on an Ember (mana spent first so the refund shows): the shatter's
/// damage and the refund, as a share of max mana in percent.
fn consume_both() -> (u32, f32) {
    let (mut realm, me, mobs, mut view) = world(7);
    let mob = &mobs[0];
    stand_at(&mut realm, mob, 20.0);
    go(&mut realm, &mut view, COLD, Some(mob.guid));
    assert!(view.aura(mob.guid, COLD).is_some(), "the Cold is on");
    let shatter = go(&mut realm, &mut view, CATALYST, Some(mob.guid));
    assert!(view.aura(mob.guid, COLD).is_none(), "the Cold is consumed");
    let shatter = damage(&shatter, CATALYST, mob.guid).expect("a shatter");
    for _ in 0..20 {
        go(&mut realm, &mut view, DRAIN, None);
    }
    go(&mut realm, &mut view, EMBER, Some(mob.guid));
    let refunded = go(&mut realm, &mut view, CATALYST, Some(mob.guid));
    assert!(
        view.aura(mob.guid, EMBER).is_none(),
        "the Ember is consumed"
    );
    let max = view.fields(me).unit_max_power(MANA).unwrap();
    let back = refund(&refunded, CATALYST).expect("a refund");
    (shatter, back as f32 * 100.0 / max as f32)
}

#[test]
fn catalyst_shatters_a_cold_for_twice_its_hit_and_refunds_a_fifth_of_mana_for_an_ember() {
    let (shatter, refund) = consume_both();
    assert_eq!(shatter, 40, "200% of a 20 hit");
    assert!((refund - 20.0).abs() < 0.5, "{refund}% of max mana");
}

#[test]
fn catalyst_with_nothing_to_consume_does_nothing() {
    let (mut realm, me, mobs, mut view) = world(8);
    let mob = &mobs[0];
    stand_at(&mut realm, mob, 20.0);
    for _ in 0..5 {
        go(&mut realm, &mut view, DRAIN, None);
    }
    let mana = view.fields(me).unit_power(MANA);
    let events = go(&mut realm, &mut view, CATALYST, Some(mob.guid));
    assert_eq!(damage(&events, CATALYST, mob.guid), None);
    assert_eq!(refund(&events, CATALYST), None);
    assert_eq!(view.fields(me).unit_power(MANA), mana);
}

#[test]
fn the_shipped_catalyst_shatters_for_twice_biting_colds_frost_hit() {
    let initiate = callings::calling("initiate").unwrap();
    let skill = |key: &str| initiate.skills.iter().find(|s| s.key == key).unwrap();
    let cold = skill("biting_cold")
        .effects
        .iter()
        .find_map(|e| match e {
            Effect::SchoolDamage { amount, .. } => Some(*amount),
            _ => None,
        })
        .unwrap();
    let consumes: Vec<&Effect> = skill("catalyst").effects.iter().collect();
    assert!(matches!(
        consumes[0],
        Effect::ConsumeAura { aura_spell_id, amount: Some(a), school: School::Frost, amount_pct: 200, .. }
            if *a == cold && *aura_spell_id == spell("initiate", "aura_biting_cold")
    ));
    assert!(matches!(
        consumes[1],
        Effect::ConsumeAura {
            aura_spell_id,
            amount: None,
            energize_pct_of_max: 10,
            ..
        } if *aura_spell_id == spell("initiate", "aura_searing_ember")
    ));
}

// Catalyst's Skill Choices and the passive Catalytic Resonance (#506), played by a shipped
// Initiate at 24 on the level 8 or 9 mob south of the Crossroads, or on two mobs of the camp east
// of the Crossroads.

use common::cutthroat::energized;
use common::initiate::*;
use common::vanguard::{Scene, STURDY};

/// Puts Biting Cold and Searing Ember on the scene's mob: the Cold's hit, when both landed and the
/// Cold did not crit.
fn cold_and_ember(s: &mut Scene) -> Option<u32> {
    let (cold, ember) = (initiate("biting_cold"), initiate("searing_ember"));
    let mob = s.mob.guid;
    let events = cast_through(&mut s.realm, &mut s.view, cold, mob);
    cast_through(&mut s.realm, &mut s.view, ember, mob);
    let burning = s.view.aura(mob, initiate("aura_searing_ember")).is_some();
    plain_hits_on(&events, cold, mob)
        .first()
        .copied()
        .filter(|_| burning && s.view.aura(mob, cold).is_some())
}

/// Thermal Shock's shatter is 2.6 times Biting Cold's hit (rounded) and its Ember refund 13% of
/// maximum mana; plain Catalyst's are twice the hit and 10%.
#[test]
fn thermal_shocks_shatter_is_2_6_times_biting_colds_hit_and_its_refund_13_percent_of_max_mana() {
    let shock = choice("initiate", "catalyst", "thermal_shock");
    for (spell, rows, times, pct) in [
        (shock, vec![("catalyst", shock)], 2.6, 0.13),
        (initiate("catalyst"), vec![], 2.0, 0.10),
    ] {
        let mut seen = 0;
        for seed in 0..10 {
            let mut s = initiate_scene(seed, &rows, STURDY);
            let (me, mob) = (s.me, s.mob.guid);
            let Some(cold) = cold_and_ember(&mut s) else {
                continue;
            };
            let events = cast_through(&mut s.realm, &mut s.view, spell, mob);
            let want = (max_mana(&s.view, me) as f32 * pct).round() as u32;
            assert_eq!(energized(&events, spell), [want], "seed {seed}: {rows:?}");
            let Some(&shatter) = plain_hits_on(&events, spell, mob).first() else {
                continue;
            };
            assert_eq!(
                shatter,
                (cold as f32 * times).round() as u32,
                "seed {seed}: {rows:?}"
            );
            seen += 1;
        }
        assert!(seen > 0, "{rows:?}: a shatter that did not crit");
    }
}

/// Deep Freeze stuns the mob for 3 s under the choice's stun aura after consuming a Biting Cold,
/// and stuns nothing after consuming only a Searing Ember; plain Catalyst stuns nothing.
#[test]
fn deep_freeze_stuns_for_3_s_after_consuming_a_cold_and_not_after_only_an_ember() {
    let freeze = choice("initiate", "catalyst", "deep_freeze");
    let twilight_realm::callings::Effect::Stun {
        aura_spell_id: stun,
        duration_ms,
        ..
    } = choice_skill("catalyst", "deep_freeze").effects[2]
    else {
        panic!("Deep Freeze's third effect is its stun");
    };
    let (cold, ember) = (initiate("biting_cold"), initiate("searing_ember"));

    let mut s = initiate_scene(3, &[("catalyst", freeze)], STURDY);
    let mob = s.mob.guid;
    cast_through(&mut s.realm, &mut s.view, cold, mob);
    cast_through(&mut s.realm, &mut s.view, freeze, mob);
    assert!(s.view.aura(mob, cold).is_none(), "the Cold is consumed");
    assert!(s.view.aura(mob, stun).is_some(), "stunned");
    let secs = lasts(&mut s.realm, &mut s.view, mob, stun, 5.0);
    let want = duration_ms as f32 / 1000.0;
    assert!((secs - want).abs() <= 0.15, "stunned {secs} s, want {want}");

    let mut s = initiate_scene(3, &[("catalyst", freeze)], STURDY);
    let mob = s.mob.guid;
    cast_through(&mut s.realm, &mut s.view, ember, mob);
    cast_through(&mut s.realm, &mut s.view, freeze, mob);
    assert!(
        s.view.aura(mob, stun).is_none(),
        "an Ember alone stuns nothing"
    );

    let mut s = initiate_scene(3, &[], STURDY);
    let mob = s.mob.guid;
    cast_through(&mut s.realm, &mut s.view, cold, mob);
    cast_through(&mut s.realm, &mut s.view, initiate("catalyst"), mob);
    assert!(
        s.view.aura(mob, stun).is_none(),
        "plain Catalyst stuns nothing"
    );
}

/// Catalytic Chain consumes the Ember on one of two pulled mobs and puts a fresh one on the other
/// beside it; plain Catalyst leaves the other clean.
#[test]
fn catalytic_chain_puts_a_consumed_ember_on_a_second_mob_within_10_yards() {
    let chain = choice("initiate", "catalyst", "catalytic_chain");
    let (ember, burn) = (initiate("searing_ember"), initiate("aura_searing_ember"));
    for (spell, rows, spreads) in [
        (chain, vec![("catalyst", chain)], true),
        (initiate("catalyst"), vec![], false),
    ] {
        let mut p = pair(3, &rows);
        let [target, other] = p.mobs.clone();
        cast_through(&mut p.realm, &mut p.view, ember, target.guid);
        assert!(p.view.aura(other.guid, burn).is_none(), "{rows:?}: clean");
        cast_through(&mut p.realm, &mut p.view, spell, target.guid);
        assert!(
            p.view.aura(target.guid, burn).is_none(),
            "{rows:?}: consumed"
        );
        assert_eq!(p.view.aura(other.guid, burn).is_some(), spreads, "{rows:?}");
    }
}

/// Under Catalytic Resonance a Catalyst that consumes an Ember grants its buff, and so does a
/// Skill Choice of Catalyst (Thermal Shock): the proc's `skill=catalyst` answers to it. Without
/// the Talent, no buff.
#[test]
fn catalytic_resonance_follows_catalyst_and_its_skill_choice() {
    let resonance = choice("initiate", "passives", "catalytic_resonance");
    let shock = choice("initiate", "catalyst", "thermal_shock");
    let buff = initiate("aura_catalytic_resonance_5");
    let ember = initiate("searing_ember");
    for (spell, rows, granted) in [
        (initiate("catalyst"), vec![("passives", resonance)], true),
        (
            shock,
            vec![("passives", resonance), ("catalyst", shock)],
            true,
        ),
        (initiate("catalyst"), vec![], false),
    ] {
        let mut s = initiate_scene(3, &rows, STURDY);
        let (me, mob) = (s.me, s.mob.guid);
        cast_through(&mut s.realm, &mut s.view, ember, mob);
        cast_through(&mut s.realm, &mut s.view, spell, mob);
        assert_eq!(s.view.aura(me, buff).is_some(), granted, "{rows:?}");
    }
}
