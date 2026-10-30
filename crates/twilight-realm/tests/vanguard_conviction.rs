//! Conviction, the Vanguard's stacking aura: a landed hit adds a stack (held to the aura row's
//! `max_stacks`), and Zealous Cleave takes them all for Holy damage a stack. The engine half is
//! played through the realm's seam on a test Calling (`Realm::with_callings`), whose two skills
//! carry the same generic effects as the Vanguard's, so every hit lands; the content half reads
//! the Vanguard's rows. Two Talent Row choices (#504) are played on the shipped Vanguard by the
//! scorpid camp: Penitent's Smash heals a share of its strike (`heal_pct`), and Ceaseless Zeal's
//! block or parry adds a Conviction stack and rage.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, Effect, StackRule};
use twilight_realm::{Realm, RealmCommand};

const SEED: u64 = 0x5EED_0101;
const LEVEL: u8 = 20;

/// The test skills: one adds a stack, one takes them all. The aura is Conviction's spell.
const TAG: u32 = 1752;
const BURST: u32 = 2098;
const CONVICTION: u32 = 78;

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_conviction"
name = "Test Conviction"
tier = 0
class = 4
cap = 30
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth", "leather"]

[[calling.skill]]
key = "tag"
spell_id = 1752
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 5.0]
effects = [{ kind = "add_aura_stacks", aura_spell_id = 78, stacks = 1, duration_ms = 15000, max_stacks = 5, stack_rule = "refresh" }]

[[calling.skill]]
key = "burst"
spell_id = 2098
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 5.0]
effects = [{ kind = "consume_aura_stacks", aura_spell_id = 78, per_stack = { base = 5.0, per_level = 1.0 }, school = "holy" }]
"#;

/// A level 20 human of the test Calling beside the scorpid camp south of the Crossroads: the
/// camp's mobs and the view. Stands two yards from the first mob.
fn at_the_camp() -> (Realm, u64, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {LEVEL}\n"))
        .replace(
            "calling = \"basic_warrior\"",
            "calling = \"test_conviction\"",
        );
    let mut realm = Realm::with_seed(SEED).with_callings(TEST_CALLINGS);
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
    let streamed = view.run(&mut realm, 1.0);
    let camp: Vec<Mob> = streamed
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: benilla_protocol::EntityKind::Unit,
                position,
                ..
            } if flat(*position, CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect();
    assert!(!camp.is_empty(), "the scorpid camp streams in");
    stand_at(&mut realm, &camp[0], 2.0);
    (realm, guid, camp, view)
}

/// Casts `spell_id` at `mob`, answered, and what it logged.
fn cast_at(realm: &mut Realm, view: &mut View, spell_id: u32, mob: &Mob) -> Vec<SessionEvent> {
    realm.handle(cast(spell_id, Some(mob.guid)));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "spell {spell_id}");
    events
}

/// The damage the cast of `spell_id` logged, one entry a hit.
fn damage_of(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id => Some(l.damage),
            _ => None,
        })
        .collect()
}

/// The stacks of Conviction the client shows on the player; 0 with none.
fn stacks(view: &View, me: u64) -> u8 {
    view.aura(me, CONVICTION).map_or(0, |a| a.stacks)
}

#[test]
fn three_hits_leave_three_stacks() {
    let (mut realm, me, camp, mut view) = at_the_camp();
    for want in 1..=3 {
        cast_at(&mut realm, &mut view, TAG, &camp[0]);
        assert_eq!(stacks(&view, me), want);
    }
}

#[test]
fn six_hits_leave_five_the_most() {
    let (mut realm, me, camp, mut view) = at_the_camp();
    for _ in 0..6 {
        cast_at(&mut realm, &mut view, TAG, &camp[0]);
    }
    assert_eq!(stacks(&view, me), 5);
}

#[test]
fn the_stacks_fade_together_after_the_last_hit() {
    let (mut realm, me, camp, mut view) = at_the_camp();
    cast_at(&mut realm, &mut view, TAG, &camp[0]);
    view.run(&mut realm, 10.0);
    cast_at(&mut realm, &mut view, TAG, &camp[0]);
    // Refresh starts the 15 s over: the first stack's own 5 s left do not end them.
    view.run(&mut realm, 10.0);
    assert_eq!(stacks(&view, me), 2);
    view.run(&mut realm, 6.0);
    assert_eq!(stacks(&view, me), 0);
}

#[test]
fn zealous_cleave_takes_every_stack_for_holy_damage_a_stack() {
    let (mut realm, me, camp, mut view) = at_the_camp();
    for _ in 0..6 {
        cast_at(&mut realm, &mut view, TAG, &camp[0]);
    }
    let events = cast_at(&mut realm, &mut view, BURST, &camp[0]);
    // Placeholder: 5 + 1 a level of Holy for each of the 5 stacks.
    assert_eq!(damage_of(&events, BURST), vec![5 * (5 + u32::from(LEVEL))]);
    assert_eq!(stacks(&view, me), 0);
    assert_eq!(view.aura(me, CONVICTION), None);
}

#[test]
fn with_no_stacks_there_is_no_bonus_and_the_cast_still_goes_off() {
    let (mut realm, _, camp, mut view) = at_the_camp();
    let events = cast_at(&mut realm, &mut view, BURST, &camp[0]);
    assert!(damage_of(&events, BURST).is_empty());
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::SpellGo {
            spell_id: BURST,
            ..
        }
    )));
}

/// The player's health after the camp has worn them down by at least `missing`.
fn worn_down(realm: &mut Realm, me: u64, camp: &[Mob], view: &mut View, missing: u32) -> u32 {
    let full = view.health(me);
    for other in camp.iter().rev() {
        pull(realm, view, other);
    }
    stand_at(realm, &camp[0], 2.0);
    for _ in 0..600 {
        view.run(realm, 0.1);
        let health = view.health(me);
        assert!(health > 0, "died waiting to be hurt");
        if full - health.min(full) >= missing {
            return health;
        }
    }
    panic!("the camp never hurt the player by {missing}");
}

#[test]
fn without_the_talent_the_cleave_heals_nothing() {
    let (mut realm, me, camp, mut view) = at_the_camp();
    let health = worn_down(&mut realm, me, &camp, &mut view, 20);
    for _ in 0..5 {
        cast_at(&mut realm, &mut view, TAG, &camp[0]);
    }
    cast_at(&mut realm, &mut view, BURST, &camp[0]);
    assert_eq!(view.health(me), health);
}

// ── The Vanguard's own rows ──────────────────────────────────────────────────────────────────

#[test]
fn righteous_smash_adds_a_stack_and_zealous_cleave_takes_them() {
    let vanguard = callings::calling("vanguard").unwrap();
    let conviction_id = common::spell("vanguard", "conviction");
    let conviction = vanguard.auras.iter().find(|a| a.key == "conviction");
    // The aura row carries no effect of its own, so it is not a granted buff.
    assert!(conviction.is_none());

    let smash = vanguard
        .skills
        .iter()
        .find(|s| s.key == "righteous_smash")
        .unwrap();
    let Some(Effect::AddAuraStacks {
        aura_spell_id,
        stacks,
        duration_ms,
        max_stacks,
        stack_rule,
    }) = smash
        .effects
        .iter()
        .copied()
        .find(|e| matches!(e, Effect::AddAuraStacks { .. }))
    else {
        panic!("Righteous Smash adds stacks");
    };
    assert_eq!(
        (aura_spell_id, stacks, duration_ms, max_stacks, stack_rule),
        (conviction_id, 1, 15000, 5, StackRule::Refresh)
    );

    let cleave = vanguard
        .skills
        .iter()
        .find(|s| s.key == "zealous_cleave")
        .unwrap();
    let consumed = cleave.effects.iter().find_map(|e| match e {
        Effect::ConsumeAuraStacks { aura_spell_id, .. } => Some(*aura_spell_id),
        _ => None,
    });
    assert_eq!(consumed, Some(conviction_id));
}

// ── Penitent's Smash and Ceaseless Zeal, on the shipped Vanguard (#504) ─────────────────────────

static RIGHTEOUS_SMASH: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| common::spell("vanguard", "righteous_smash"));
static PENITENTS_SMASH: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("vanguard", "righteous_smash", "penitents_smash"));
static BULWARK_CHARGE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| common::spell("vanguard", "bulwark_charge"));

/// What the player healed under `spell_id` in `events`.
fn healed(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellHealLog(h) if h.spell_id == spell_id => Some(h.amount),
            _ => None,
        })
        .collect()
}

/// A level 4 Vanguard on `seed`'s dice with `rows` set, who charged the camp's first mob and then
/// stood in its blows until 30 health down.
fn hurt_at_the_camp(seed: u64, rows: &[(&str, u32)]) -> vanguard::Scene {
    let mut s = vanguard::scene(4, true, seed, rows, CAMP);
    s.cast_at_mob(*BULWARK_CHARGE);
    let full = s.view.health(s.me);
    let me = s.me;
    s.run_until(30.0, "30 health lost", |view, _| {
        view.health(me) + 30 <= full
    });
    s
}

/// Queues `smash` and swings at the scene's mob: the events up to the strike's.
fn smash(s: &mut vanguard::Scene, smash: u32) -> Vec<SessionEvent> {
    s.realm.handle(cast(smash, Some(s.mob.guid)));
    assert_eq!(refusal(&s.view.drain(&mut s.realm), smash), None, "queued");
    s.realm
        .handle(RealmCommand::AttackSwing { guid: s.mob.guid });
    s.run_until(10.0, "the smash", |_, events| {
        events
            .iter()
            .any(|e| matches!(e, SessionEvent::SpellGo { spell_id, .. } if *spell_id == smash))
    })
}

#[test]
fn penitents_smash_heals_a_quarter_of_its_damage() {
    let mut landed = 0;
    for seed in 0..4 {
        let mut s = hurt_at_the_camp(seed, &[("righteous_smash", *PENITENTS_SMASH)]);
        let events = smash(&mut s, *PENITENTS_SMASH);
        let dealt: Vec<u32> = damage_of(&events, *PENITENTS_SMASH);
        match dealt[..] {
            [] => assert!(healed(&events, *PENITENTS_SMASH).is_empty(), "seed {seed}"),
            [damage] => {
                assert_eq!(
                    healed(&events, *PENITENTS_SMASH),
                    [damage * 25 / 100],
                    "seed {seed}"
                );
                landed += 1;
            }
            _ => panic!("seed {seed}: one target, {dealt:?}"),
        }
    }
    assert!(landed >= 2, "{landed} smashes landed");
}

#[test]
fn righteous_smash_heals_nothing() {
    for seed in 0..2 {
        let mut s = hurt_at_the_camp(seed, &[]);
        let events = smash(&mut s, *RIGHTEOUS_SMASH);
        assert!(
            !damage_of(&events, *RIGHTEOUS_SMASH).is_empty(),
            "seed {seed}"
        );
        assert!(healed(&events, *RIGHTEOUS_SMASH).is_empty(), "seed {seed}");
    }
}

static CEASELESS_ZEAL: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("vanguard", "passives", "ceaseless_zeal"));
static SHIELD_OF_THE_PENITENT: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| common::spell("vanguard", "shield_of_penitent"));
static SHIPPED_CONVICTION: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| common::spell("vanguard", "conviction"));

/// The blows at `me` in `events` that `me` blocked or parried.
fn defended(events: &[SessionEvent], me: u64) -> usize {
    events
        .iter()
        .filter(|e| {
            matches!(e, SessionEvent::AttackerState(s) if s.victim == me
                && (s.blocked > 0
                    || s.victim_state == vanguard::BLOCKS
                    || s.victim_state == vanguard::PARRY))
        })
        .count()
}

/// The power `spell_id` gave in `events`, one entry a gain.
fn energized(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellEnergizeLog(l) if l.spell_id == spell_id => Some(l.amount),
            _ => None,
        })
        .collect()
}

/// A level 20 Vanguard on `seed`'s dice with `rows` set, who charged the sturdy mob, put up
/// Shield of the Penitent for its blocks, and stands in the mob's blows with the swing stopped.
fn shielded(seed: u64, rows: &[(&str, u32)]) -> vanguard::Scene {
    let mut s = vanguard::scene(20, true, seed, rows, vanguard::STURDY);
    s.cast_at_mob(*BULWARK_CHARGE);
    s.realm.handle(cast(*SHIELD_OF_THE_PENITENT, None));
    let events = s.view.drain(&mut s.realm);
    assert_eq!(
        refusal(&events, *SHIELD_OF_THE_PENITENT),
        None,
        "the shield"
    );
    s
}

#[test]
fn under_ceaseless_zeal_each_block_or_parry_gives_5_rage_and_a_conviction_stack() {
    let mut s = shielded(SEED, &[("passives", *CEASELESS_ZEAL)]);
    let me = s.me;
    let (mut total, mut gains) = (0, Vec::new());
    s.run_until(20.0, "three blocks or parries", |view, events| {
        total += defended(events, me);
        gains.extend(energized(events, *CEASELESS_ZEAL));
        let stacks = view.aura(me, *SHIPPED_CONVICTION).map_or(0, |a| a.stacks);
        assert_eq!(
            usize::from(stacks),
            total.min(5),
            "a stack each, not a refresh"
        );
        total >= 3
    });
    assert_eq!(gains, vec![5; total]);
}

#[test]
fn without_ceaseless_zeal_a_block_or_parry_adds_no_conviction() {
    let mut s = shielded(SEED, &[]);
    let me = s.me;
    let mut total = 0;
    let events = s.run_until(20.0, "a block or parry", |_, events| {
        total += defended(events, me);
        total >= 1
    });
    assert_eq!(s.view.aura(me, *SHIPPED_CONVICTION), None);
    assert!(energized(&events, *CEASELESS_ZEAL).is_empty());
}
