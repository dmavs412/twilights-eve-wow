//! Aura copies (#679): `copy_aura` copies the caster's stacking damage over time on the target,
//! with its stacks and time left, onto the nearest other enemies; a `wearer_death` proc spreads it
//! when its wearer dies. Played through the realm's seam with a test Priest Calling.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_spreader"
name = "Test Spreader"
tier = 0
class = 5
cap = 10
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "blight"
spell_id = 589
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 40.0]
effects = [{ kind = "periodic_damage", tick_ms = 1000, duration_ms = 10000, tick = { base = 1.0 }, max_stacks = 255, aura_spell_id = 589, stack_rule = "stack", school = "shadow" }]

[[calling.skill]]
key = "spread_blight"
spell_id = 15487
power_cost = 50
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 40.0]
effects = [{ kind = "copy_aura", aura_spell_id = 589, targets = 2, radius_yd = 10.0 }]

[[calling.skill]]
key = "smite"
spell_id = 585
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 40.0]
effects = [{ kind = "school_damage", school = "holy", amount = { base = 100000.0 } }]

[[calling.aura]]
key = "blight"
spell_id = 589
duration_ms = 10000
row = { max_stacks = 255, stack_rule = "stack" }
effects = [{ kind = "periodic_damage", tick_ms = 1000, tick = { base = 1.0 }, school = "shadow" }]

[[calling.row]]
key = "row_contagion"
level = 10
kind = "talent"
choices = [{ key = "contagion", name = "Contagion", spell_id = 900791, effects = [{ kind = "proc", params = { on = "wearer_death", aura = "blight", spread_aura = 10, spread_stacks = "all" } }] }]

[[calling.row]]
key = "row_epidemic"
level = 10
kind = "talent"
choices = [{ key = "seep", name = "Seep", spell_id = 900792, effects = [{ kind = "proc", params = { on = "wearer_death", aura = "blight", spread_aura = 10 } }] }]
"#;

const BLIGHT: u32 = 589;
const SPREAD: u32 = 15487;
const SMITE: u32 = 585;
const CONTAGION: u32 = 900791;
const SEEP: u32 = 900792;

/// Where the test Priest stands, east of the scorpid camp, and where the first mob is put.
const ME: [f32; 3] = [CAMP[0] + 40.0, CAMP[1], 0.0];
const A: [f32; 3] = [CAMP[0] + 30.0, CAMP[1], 0.0];

/// A level 10 test Priest east of the camp with `rows` set: the realm, their guid, the camp's
/// mobs (at least four) and the view.
fn world(rows: &[(&str, u32)]) -> (Realm, u64, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Vellune", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_spreader\"");
    let mut realm = Realm::with_seed(7).with_callings(TEST_CALLINGS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let mut seen = view.drain(&mut realm);
    for (row, choice) in rows {
        realm.handle(RealmCommand::SetTalentRow {
            row: row.to_string(),
            spell_id: *choice,
        });
    }
    seen.extend(view.run(&mut realm, 1.0));
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: ME,
        orientation: std::f32::consts::PI,
    });
    seen.extend(view.run(&mut realm, 1.0));
    let mobs: Vec<Mob> = seen
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } if flat(*position, CAMP) < 60.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect();
    assert!(
        mobs.len() >= 4,
        "four camp mobs in view, has {}",
        mobs.len()
    );
    (realm, guid, mobs, view)
}

/// Puts each of `mobs` at `A` plus its north offset in yards, in that order, and the Priest back
/// at `ME`.
fn line_up(realm: &mut Realm, mobs: &[&Mob], offsets: &[f32]) {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: ME,
        orientation: std::f32::consts::PI,
    });
    for (mob, dy) in mobs.iter().zip(offsets) {
        realm.place_creature(mob.guid, [A[0], A[1] + dy, 0.0]);
    }
}

/// Casts `spell_id` at `target`, asserting it went: the events.
fn go(realm: &mut Realm, view: &mut View, spell_id: u32, target: u64) -> Vec<SessionEvent> {
    realm.handle(cast(spell_id, Some(target)));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "spell {spell_id} is cast");
    events
}

/// Blight's stacks on `mob`, 0 for none.
fn stacks(view: &View, mob: u64) -> u8 {
    view.aura(mob, BLIGHT).map_or(0, |a| a.stacks)
}

/// `mobs` lined up as [`line_up`] puts them, six stacks of Blight on the first, then 2 s for
/// 8 s left, and the line put back as it was.
fn six_stacks(realm: &mut Realm, view: &mut View, mobs: &[&Mob], offsets: &[f32]) {
    line_up(realm, mobs, offsets);
    for _ in 0..6 {
        go(realm, view, BLIGHT, mobs[0].guid);
    }
    assert_eq!(stacks(view, mobs[0].guid), 6);
    view.run(realm, 2.0);
    line_up(realm, mobs, offsets);
}

#[test]
fn copy_aura_puts_the_stacks_and_time_left_on_the_two_nearest() {
    let (mut realm, _, mobs, mut view) = world(&[]);
    let [a, b, c, d] = [&mobs[0], &mobs[1], &mobs[2], &mobs[3]];
    // Three mobs within 10 yd of a, d the farthest.
    six_stacks(&mut realm, &mut view, &[a, d, b, c], &[0.0, 9.0, 3.0, 6.0]);
    go(&mut realm, &mut view, SPREAD, a.guid);
    assert_eq!(stacks(&view, b.guid), 6, "the nearest");
    assert_eq!(stacks(&view, c.guid), 6, "the next nearest");
    assert_eq!(stacks(&view, d.guid), 0, "two targets only");
    assert_eq!(stacks(&view, a.guid), 6, "the original is untouched");
    // The copies end with the original's 8 s, not the row's 10.
    view.run(&mut realm, 7.5);
    assert_eq!(stacks(&view, b.guid), 6, "still on at 7.5 s");
    assert_eq!(stacks(&view, c.guid), 6);
    view.run(&mut realm, 1.0);
    assert_eq!(stacks(&view, b.guid), 0, "gone by 8.5 s");
    assert_eq!(stacks(&view, c.guid), 0);
    assert_eq!(stacks(&view, a.guid), 0, "the original too");
}

#[test]
fn copy_aura_skips_a_mob_already_wearing_it_and_one_out_of_reach() {
    let (mut realm, _, mobs, mut view) = world(&[]);
    let [a, worn, b, far] = [&mobs[0], &mobs[1], &mobs[2], &mobs[3]];
    let (line, offsets) = ([a, worn, b, far], [0.0, 2.0, 4.0, 12.0]);
    line_up(&mut realm, &line, &offsets);
    go(&mut realm, &mut view, BLIGHT, worn.guid);
    six_stacks(&mut realm, &mut view, &line, &offsets);
    go(&mut realm, &mut view, SPREAD, a.guid);
    assert_eq!(stacks(&view, worn.guid), 1, "the wearer keeps its own");
    assert_eq!(stacks(&view, b.guid), 6, "the nearest that lacks it");
    assert_eq!(stacks(&view, far.guid), 0, "12 yd is out of reach");
}

#[test]
fn copy_aura_with_no_blight_on_the_target_is_refused_and_costs_nothing() {
    let (mut realm, me, mobs, mut view) = world(&[]);
    let mana = view.fields(me).unit_power(0).unwrap();
    line_up(&mut realm, &[&mobs[0], &mobs[1]], &[0.0, 3.0]);
    realm.handle(cast(SPREAD, Some(mobs[0].guid)));
    let events = view.drain(&mut realm);
    assert!(refusal(&events, SPREAD).is_some(), "refused");
    assert_eq!(
        view.fields(me).unit_power(0).unwrap(),
        mana,
        "no mana spent"
    );
    assert_eq!(stacks(&view, mobs[1].guid), 0);
}

#[test]
fn a_wearer_death_proc_spreads_every_stack_and_the_full_duration() {
    let (mut realm, _, mobs, mut view) = world(&[("row_contagion", CONTAGION)]);
    let [a, b, c] = [&mobs[0], &mobs[1], &mobs[2]];
    six_stacks(&mut realm, &mut view, &[a, b, c], &[0.0, 3.0, 6.0]);
    go(&mut realm, &mut view, SMITE, a.guid);
    assert_eq!(view.health(a.guid), 0, "a dies");
    assert_eq!(stacks(&view, b.guid), 6, "every stack on the nearest");
    assert_eq!(stacks(&view, c.guid), 0, "one copy only");
    // The row's full 10 s, not the 8 s a had left.
    view.run(&mut realm, 9.5);
    assert_eq!(stacks(&view, b.guid), 6, "still on at 9.5 s");
    view.run(&mut realm, 1.0);
    assert_eq!(stacks(&view, b.guid), 0, "gone by 10.5 s");
}

#[test]
fn a_mob_dying_without_blight_spreads_nothing() {
    let (mut realm, _, mobs, mut view) = world(&[("row_contagion", CONTAGION)]);
    let [a, b] = [&mobs[0], &mobs[1]];
    line_up(&mut realm, &[a, b], &[0.0, 3.0]);
    go(&mut realm, &mut view, SMITE, a.guid);
    assert_eq!(view.health(a.guid), 0, "a dies");
    assert_eq!(stacks(&view, b.guid), 0, "nothing to spread");
}

#[test]
fn a_wearer_death_proc_without_spread_stacks_spreads_one_stack() {
    let (mut realm, _, mobs, mut view) = world(&[("row_epidemic", SEEP)]);
    let [a, b] = [&mobs[0], &mobs[1]];
    six_stacks(&mut realm, &mut view, &[a, b], &[0.0, 3.0]);
    go(&mut realm, &mut view, SMITE, a.guid);
    assert_eq!(view.health(a.guid), 0, "a dies");
    assert_eq!(stacks(&view, b.guid), 1, "one stack");
}
