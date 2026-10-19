//! Conviction, the Vanguard's stacking aura: a landed hit adds a stack (held to the aura row's
//! `max_stacks`), and Zealous Cleave takes them all for Holy damage a stack. The engine half is
//! played through the realm's seam on a test Calling (`Realm::with_callings`), whose two skills
//! carry the same generic effects as the Vanguard's, so every hit lands; the content half reads
//! the Vanguard's rows.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, Effect, StackRule, TalentEffect};
use twilight_realm::{Realm, RealmCommand};

const SEED: u64 = 0x5EED_0101;
const LEVEL: u8 = 20;

/// The test skills: one adds a stack, one takes them all. The aura is Conviction's spell.
const TAG: u32 = 1752;
const BURST: u32 = 2098;
const CONVICTION: u32 = 78;
/// The test talent, Zealous Restoration's shape: the burst heals for 10/20/30% of its damage.
const RESTORATION: u32 = 99001;

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

[[calling.talent_tab]]
id = 9990
name = "Test"
background = "WarriorArms"

[[calling.talent_tab.talent]]
key = "restoration"
talent_id = 99001
tier = 1
column = 1
rank_spells = [99011, 99012, 99013]
spell_name = "Test Restoration"
effects = [{ kind = "consume_heal_pct", skill = "burst", per_rank = [10, 20, 30] }]
"#;

/// A level 20 human of the test Calling, with `talents` learned, beside the scorpid camp south of
/// the Crossroads: the camp's mobs and the view. Stands two yards from the first mob.
fn at_the_camp(talents: &[(u32, u8)]) -> (Realm, u64, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {LEVEL}\n"))
        .replace(
            "calling = \"basic_warrior\"",
            "calling = \"test_conviction\"",
        );
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
    }
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
    let (mut realm, me, camp, mut view) = at_the_camp(&[]);
    for want in 1..=3 {
        cast_at(&mut realm, &mut view, TAG, &camp[0]);
        assert_eq!(stacks(&view, me), want);
    }
}

#[test]
fn six_hits_leave_five_the_most() {
    let (mut realm, me, camp, mut view) = at_the_camp(&[]);
    for _ in 0..6 {
        cast_at(&mut realm, &mut view, TAG, &camp[0]);
    }
    assert_eq!(stacks(&view, me), 5);
}

#[test]
fn the_stacks_fade_together_after_the_last_hit() {
    let (mut realm, me, camp, mut view) = at_the_camp(&[]);
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
    let (mut realm, me, camp, mut view) = at_the_camp(&[]);
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
    let (mut realm, _, camp, mut view) = at_the_camp(&[]);
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
        realm.handle(RealmCommand::AttackSwing { guid: other.guid });
    }
    realm.handle(RealmCommand::AttackStop);
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
fn zealous_restoration_heals_a_share_of_the_bonus_damage() {
    for (rank, pct) in [(1u8, 10u32), (3, 30)] {
        let (mut realm, me, camp, mut view) = at_the_camp(&[(RESTORATION, rank)]);
        let health = worn_down(&mut realm, me, &camp, &mut view, 60);
        for _ in 0..5 {
            cast_at(&mut realm, &mut view, TAG, &camp[0]);
        }
        let events = cast_at(&mut realm, &mut view, BURST, &camp[0]);
        let bonus: u32 = damage_of(&events, BURST).iter().sum();
        assert_eq!(bonus, 5 * (5 + u32::from(LEVEL)));
        assert_eq!(view.health(me), health + bonus * pct / 100, "rank {rank}");
    }
}

#[test]
fn without_the_talent_the_cleave_heals_nothing() {
    let (mut realm, me, camp, mut view) = at_the_camp(&[]);
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
        (284, 1, 15000, 5, StackRule::Refresh)
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
    assert_eq!(consumed, Some(284));
}

#[test]
fn zealous_restoration_heals_ten_twenty_thirty_percent() {
    let vanguard = callings::calling("vanguard").unwrap();
    let talent = vanguard
        .talent_tabs
        .iter()
        .flat_map(|t| &t.talents)
        .find(|t| t.key == "zealous_restoration")
        .unwrap();
    assert_eq!(
        talent.effects,
        [TalentEffect::ConsumeHealPct {
            skill: "zealous_cleave".into(),
            per_rank: vec![10, 20, 30]
        }]
    );
}
