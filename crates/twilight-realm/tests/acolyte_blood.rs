//! The Acolyte's blood: Blood Lance and Dark Mending cost a share of max health, a cast that
//! would spend the last of it is refused, Dark Mending heals up to max and takes 15% of what it
//! healed back over 6 s, and Blood Lance's HoT lands on the friendly player with the least
//! health near the target (the caster, until other players exist). Played through the realm's
//! seam with the shipped Acolyte, and a test Calling for the refusal.

mod common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

static BLOOD_LANCE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("acolyte", "blood_lance"));
static DARK_MENDING: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("acolyte", "dark_mending"));
/// The test Calling's own spells, as `TEST_CALLINGS` numbers them.
const TEST_BLEED: u32 = 2061;
const TEST_LANCE: u32 = 8092;
const SPELL_FAILED_NO_POWER: u8 = 0x4d;
const SPELL_FAILED_NOT_READY: u8 = 0x3c;

/// A Priest-Class Calling: `bleed` spends 96% of max health, `lance` 5%, both instant.
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
key = "bleed"
spell_id = 2061
power_cost = 96
cooldown_ms = 0
gcd_ms = 0
target = "self"
meta = { cost_kind = "health" }
effects = []

[[calling.skill]]
key = "lance"
spell_id = 8092
power_cost = 5
cooldown_ms = 0
gcd_ms = 0
target = "self"
meta = { cost_kind = "health" }
effects = []

[[calling.skill]]
key = "coag"
spell_id = 2060
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "heal", amount = { base = 50.0 } }, { kind = "absorb", pct = 30, pct_of = "full_heal", duration_ms = 10000, school = "physical" }]

[[calling.skill]]
key = "coag_of_healed"
spell_id = 2054
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "heal", amount = { base = 50.0 } }, { kind = "absorb", pct = 30, pct_of = "heal", duration_ms = 10000, school = "physical" }]

[[calling.skill]]
key = "thump"
spell_id = 6788
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "periodic_damage", tick_ms = 1000, duration_ms = 1000, tick = { base = 40.0 } }]
"#;
/// The test Calling's heals with a Physical shield of 30% of their full heal, and of what they
/// healed; and its Physical tick of 40 on the caster, to weigh a shield with.
const TEST_COAG: u32 = 2060;
const TEST_COAG_OF_HEALED: u32 = 2054;
const TEST_THUMP: u32 = 6788;

/// A level 10 `calling`, 10 yd from a camp mob: the realm, their guid, the mob and the view.
fn world(calling: &str, callings: Option<&str>) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Vellune", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace(
            "calling = \"basic_warrior\"",
            &format!("calling = \"{calling}\""),
        );
    let mut realm = Realm::with_seed(3);
    if let Some(callings) = callings {
        realm = realm.with_callings(callings);
    }
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
    stand_at(&mut realm, &mob, 10.0);
    view.drain(&mut realm);
    (realm, guid, mob, view)
}

fn max_health(view: &View, me: u64) -> u32 {
    view.fields(me).unit_max_health().unwrap()
}

#[test]
fn blood_lance_costs_3_percent_of_max_health_and_its_hot_lands_on_the_caster() {
    let (mut realm, me, mob, mut view) = world("acolyte", None);
    let max = max_health(&view, me);
    assert_eq!(view.health(me), max);
    realm.handle(cast(*BLOOD_LANCE, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), *BLOOD_LANCE), None);
    let landed = view.run(&mut realm, 1.6);
    let hit = landed
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == *BLOOD_LANCE => Some(l.damage),
            _ => None,
        })
        .expect("Blood Lance lands after its bar");
    assert_eq!(view.health(me), max - max * 3 / 100, "3% of max health");

    let hot: Vec<u32> = view
        .run(&mut realm, 6.1)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == *BLOOD_LANCE => {
                assert_eq!(l.target, me, "the HoT is on the caster");
                match l.ticks[0] {
                    PeriodicTick::Heal { amount } => Some(amount),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        hot,
        vec![hit * 20 / 100 / 3; 3],
        "20% of the hit in three ticks"
    );
}

#[test]
fn a_health_cost_at_or_above_the_health_left_is_refused() {
    let (mut realm, me, _, mut view) = world("test_priest", Some(TEST_CALLINGS));
    let max = max_health(&view, me);
    realm.handle(cast(TEST_BLEED, None));
    view.drain(&mut realm);
    assert_eq!(view.health(me), max - max * 96 / 100, "4% left");
    realm.handle(cast(TEST_LANCE, None));
    assert_eq!(
        refusal(&view.drain(&mut realm), TEST_LANCE),
        Some(SPELL_FAILED_NO_POWER),
        "5% of max is more than is left"
    );
}

#[test]
fn dark_mending_heals_up_to_max_then_takes_15_percent_of_it_back() {
    let (mut realm, me, _, mut view) = world("acolyte", None);
    let max = max_health(&view, me);
    realm.handle(cast(*DARK_MENDING, None));
    let mending = view.drain(&mut realm);
    assert_eq!(refusal(&mending, *DARK_MENDING), None);
    let healed = mending
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellHealLog(l) if l.spell_id == *DARK_MENDING => Some(l.amount),
            _ => None,
        })
        .expect("a heal");
    // It paid 15% of max health, so it can heal no more than that back.
    assert_eq!(healed, max * 15 / 100, "capped at max health");
    assert_eq!(view.health(me), max);
    let toll: u32 = view
        .run(&mut realm, 6.1)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == *DARK_MENDING && l.target == me => {
                match l.ticks[0] {
                    PeriodicTick::Damage { amount, .. } => Some(amount),
                    _ => None,
                }
            }
            _ => None,
        })
        .sum();
    let tick = (healed as f32 * 0.15 / 3.0).round() as u32;
    assert_eq!(toll, tick * 3, "15% of the heal over 6 s, rounded per tick");
}

#[test]
fn dark_mending_cast_twice_inside_8_seconds_is_refused_for_cooldown() {
    let (mut realm, _, _, mut view) = world("acolyte", None);
    realm.handle(cast(*DARK_MENDING, None));
    assert_eq!(refusal(&view.drain(&mut realm), *DARK_MENDING), None);
    view.run(&mut realm, 2.0);
    realm.handle(cast(*DARK_MENDING, None));
    assert_eq!(
        refusal(&view.drain(&mut realm), *DARK_MENDING),
        Some(SPELL_FAILED_NOT_READY),
        "the 8 s cooldown is still running"
    );
    view.run(&mut realm, 7.0);
    realm.handle(cast(*DARK_MENDING, None));
    assert_eq!(refusal(&view.drain(&mut realm), *DARK_MENDING), None);
}

// Blood Lance's and Dark Mending's Skill Choices (#508), played by a shipped Acolyte at 24, most
// of them at the Test Grounds' dummies, which take hits but never fight back or die.

use common::acolyte::{
    acolyte, acolyte_pack, blight_stacks, dummy, row_choice, soaked, yard, Yard, DUMMY_SPOTS,
};
use common::cutthroat::{damage_of, healed};
use common::initiate::cast_bar;

/// Where the Acolyte stands in the Test Grounds: 17.5 yd from the first dummy, so a Blood Lance's
/// heal over time, which looks 15 yd around the target, passes them by.
const YARD_SPOT: [f32; 2] = [-9655.0, 255.0];

/// The hits `spell_id` dealt `target` in `events` that did not crit.
fn plain_hits(events: &[SessionEvent], spell_id: u32, target: u64) -> Vec<u32> {
    damage_of(events, spell_id)
        .into_iter()
        .filter(|&(t, _, crit)| t == target && !crit)
        .map(|(_, d, _)| d)
        .collect()
}

/// Exsanguinate's hit is 5% more a stack of the Acolyte's Creeping Blight on the target as it
/// lands, before its own stack: 1.25 times plain Blood Lance's at 5 stacks, and held to 1.5 times
/// from 10 stacks on.
#[test]
fn exsanguinate_hits_5_percent_harder_a_blight_stack_up_to_half_again() {
    let exsanguinate = row_choice("blood_lance", "exsanguinate");
    let lance = acolyte("blood_lance");
    let mut y = yard(6, &[], YARD_SPOT);
    let mut plain = Vec::new();
    while plain.is_empty() {
        plain = plain_hits(&y.cast_through(lance, dummy(0)), lance, dummy(0));
    }
    let base = plain[0];

    let mut y = yard(6, &[("blood_lance", exsanguinate)], YARD_SPOT);
    y.cast_through(acolyte("creeping_blight"), dummy(0));
    let mut seen = Vec::new();
    while y
        .view
        .aura(dummy(0), acolyte("aura_creeping_blight"))
        .is_some()
    {
        y.realm.handle(cast(exsanguinate, Some(dummy(0))));
        let mut events = y.view.drain(&mut y.realm);
        assert_eq!(refusal(&events, exsanguinate), None);
        while !common::initiate::went_off(&events, exsanguinate) {
            // The hit lands before this tick's Blight tick grows the stacks.
            let stacks = blight_stacks(&y.view, dummy(0));
            events = y.run(0.1);
            seen.extend(
                plain_hits(&events, exsanguinate, dummy(0))
                    .into_iter()
                    .map(|d| (stacks, d)),
            );
        }
    }
    assert!(seen.iter().any(|s| s.0 == 5), "a hit at 5 stacks: {seen:?}");
    assert!(seen.iter().any(|s| s.0 >= 10), "a hit past 10: {seen:?}");
    for &(stacks, dealt) in &seen {
        let want = base * (100 + (5 * stacks).min(50)) / 100;
        assert!(
            dealt.abs_diff(want) <= 1,
            "{stacks} stacks: {dealt}, want {want} from {base}"
        );
    }
}

/// Splintering Lance at the middle dummy hits the two beside it, 6 yd off, for 30% of its hit, and
/// grows the Blight on the one that wears it; the dummy 12 yd off takes nothing. Plain Blood Lance
/// hits its target alone and grows no neighbour.
#[test]
fn splintering_lance_hits_the_dummies_within_8_yards_for_30_percent_and_grows_their_blight() {
    let splinter = row_choice("blood_lance", "splintering_lance");
    let mut y = yard(7, &[("blood_lance", splinter)], YARD_SPOT);
    y.cast_through(acolyte("creeping_blight"), dummy(0));
    let events = y.cast_through(splinter, dummy(1));
    let hits = damage_of(&events, splinter);
    let hit = hits
        .iter()
        .find(|h| h.0 == dummy(1))
        .expect("the target is hit")
        .1;
    let mut splashed: Vec<(u64, u32)> = hits
        .iter()
        .filter(|h| h.0 != dummy(1))
        .map(|h| (h.0, h.1))
        .collect();
    splashed.sort();
    let mut want = vec![(dummy(0), hit * 30 / 100), (dummy(2), hit * 30 / 100)];
    want.sort();
    assert_eq!(splashed, want, "{hits:?}");
    assert_eq!(
        blight_stacks(&y.view, dummy(0)),
        2,
        "the Blighted neighbour grew"
    );
    assert_eq!(blight_stacks(&y.view, dummy(2)), 0, "no Blight to grow");

    let lance = acolyte("blood_lance");
    let mut y = yard(7, &[], YARD_SPOT);
    y.cast_through(acolyte("creeping_blight"), dummy(0));
    let events = y.cast_through(lance, dummy(1));
    let hits = damage_of(&events, lance);
    assert!(hits.iter().all(|h| h.0 == dummy(1)), "{hits:?}");
    assert_eq!(blight_stacks(&y.view, dummy(0)), 1);
}

/// Sanguine Barrier's Lance at a pulled scorpid shields the Acolyte, the HoT's ally, for 20% of the
/// hit: the scorpids' blows it soaks come to exactly that. Plain Blood Lance shields nothing.
#[test]
fn sanguine_barrier_shields_the_acolyte_for_20_percent_of_the_hit() {
    let barrier = row_choice("blood_lance", "sanguine_barrier");
    let mut p = acolyte_pack(8, &[("blood_lance", barrier)]);
    let (me, mob) = (p.me, p.mobs[0].guid);
    let events = cast_through(&mut p.realm, &mut p.view, barrier, mob);
    let hit = damage_of(&events, barrier)
        .iter()
        .find(|h| h.0 == mob)
        .expect("the Lance hits")
        .1;
    let soak = soaked(&mut p.realm, &mut p.view, me, 6.0, |_| true);
    assert_eq!(
        soak,
        (hit * 20 + 50) / 100,
        "the shield soaked its size, rounded"
    );

    let lance = acolyte("blood_lance");
    let mut p = acolyte_pack(8, &[]);
    let (me, mob) = (p.me, p.mobs[0].guid);
    cast_through(&mut p.realm, &mut p.view, lance, mob);
    assert_eq!(soaked(&mut p.realm, &mut p.view, me, 6.0, |_| true), 0);
}

/// Casts `spell_id` at `target` and ticks until it goes off, as `common::initiate` does.
fn cast_through(
    realm: &mut Realm,
    view: &mut View,
    spell_id: u32,
    target: u64,
) -> Vec<SessionEvent> {
    common::initiate::cast_through(realm, view, spell_id, target)
}

/// Coagulation on an Acolyte at full health still leaves its shield, Dark Mending's heal being
/// capped; plain Dark Mending leaves none.
#[test]
fn coagulation_shields_an_acolyte_at_full_health() {
    let coagulation = row_choice("dark_mending", "coagulation");
    let mut y = yard(9, &[("dark_mending", coagulation)], YARD_SPOT);
    let me = y.me;
    assert_eq!(y.view.health(me), max_health(&y.view, me), "full health");
    y.cast_through(coagulation, me);
    assert!(y.view.aura(me, coagulation).is_some(), "the shield is up");

    let mending = acolyte("dark_mending");
    let mut y = yard(9, &[], YARD_SPOT);
    let me = y.me;
    y.cast_through(mending, me);
    assert!(
        !y.view
            .fields(me)
            .raw_fields()
            .any(|(_, v)| v == coagulation),
        "no Coagulation shield"
    );
}

/// A heal's shield of 30% of its full heal (`pct_of=full_heal`), cast at full health, soaks 30% of
/// the heal's 50 of a 40 Physical tick; one of 30% of what it healed soaks nothing, as the heal
/// healed nothing. Played by a test Calling, whose numbers are small enough to weigh exactly.
#[test]
fn a_shield_of_the_full_heal_is_whole_at_full_health_and_one_of_the_heal_is_not() {
    let weighed = |heal: u32| -> u32 {
        let (mut realm, me, _, mut view) = world("test_priest", Some(TEST_CALLINGS));
        assert_eq!(view.health(me), max_health(&view, me));
        realm.handle(cast(heal, None));
        assert_eq!(refusal(&view.drain(&mut realm), heal), None);
        realm.handle(cast(TEST_THUMP, None));
        assert_eq!(refusal(&view.drain(&mut realm), TEST_THUMP), None);
        let before = view.health(me);
        view.run(&mut realm, 1.05);
        before - view.health(me)
    };
    assert_eq!(weighed(TEST_COAG), 40 - 50 * 30 / 100, "a 15 shield");
    assert_eq!(weighed(TEST_COAG_OF_HEALED), 40, "no shield");
}

/// Ticks `y` and casts Blood Lance at the far dummy, whose heal over time cannot reach the
/// Acolyte, until they are below `pct` percent of their health.
fn bleed_below(y: &mut Yard, pct: u32) {
    let me = y.me;
    let max = max_health(&y.view, me);
    while y.view.health(me) * 100 >= max * pct {
        y.cast_through(acolyte("blood_lance"), dummy(3));
    }
}

/// Dark Surplus's heal bursts what it heals past the Acolyte's maximum health as Shadow at the
/// dummies within 8 yd of them: nothing below full enough for the whole heal to land, and the
/// overhealing once it no longer fits. Plain Dark Mending bursts nothing.
#[test]
fn dark_surplus_bursts_its_overhealing_at_the_dummies_within_8_yards() {
    let surplus = row_choice("dark_mending", "dark_surplus");
    let near = [DUMMY_SPOTS[0][0] - 5.0, DUMMY_SPOTS[0][1]];
    let mut y = yard(10, &[("dark_mending", surplus)], near);
    let me = y.me;
    bleed_below(&mut y, 45);
    let low = y.cast_through(surplus, me);
    let [full] = healed(&low, surplus)[..] else {
        panic!("one heal: {low:?}");
    };
    assert!(
        y.view.health(me) < max_health(&y.view, me),
        "the whole heal fit"
    );
    assert!(
        damage_of(&low, surplus).is_empty(),
        "no overhealing, no burst"
    );

    // At full health it pays 15% and heals that back; the rest bursts.
    let mut y = yard(10, &[("dark_mending", surplus)], near);
    let me = y.me;
    let events = y.cast_through(surplus, me);
    let [fit] = healed(&events, surplus)[..] else {
        panic!("one heal: {events:?}");
    };
    assert!(fit < full, "this time it overhealed");
    let bursts = damage_of(&events, surplus);
    // Dummy 0 stands 5 yd off and dummy 1 7.8 yd; each burst rolls to miss.
    assert!(!bursts.is_empty(), "it burst");
    for &(target, dealt, crit) in &bursts {
        assert!(target == dummy(0) || target == dummy(1), "{bursts:?}");
        let over = full - fit;
        let want = if crit { over * 150 / 100 } else { over };
        assert!(dealt.abs_diff(want) <= 1, "{dealt}, want {want}");
    }

    let mending = acolyte("dark_mending");
    let mut y = yard(10, &[], near);
    let events = y.cast_through(mending, y.me);
    assert!(damage_of(&events, mending).is_empty());
}

/// After Sanguine Flow's Dark Mending the next Blood Lance has no cast bar, and the one after has
/// its 1.5 s again; after plain Dark Mending the Lance keeps its bar.
#[test]
fn sanguine_flow_makes_the_next_lance_instant_and_only_the_next() {
    let flow = row_choice("dark_mending", "sanguine_flow");
    let lance = acolyte("blood_lance");
    let mut y = yard(11, &[("dark_mending", flow)], YARD_SPOT);
    let me = y.me;
    y.cast_through(flow, me);
    assert!(y.view.aura(me, acolyte("aura_sanguine_flow")).is_some());
    assert_eq!(cast_bar(&y.cast_through(lance, dummy(0)), lance), Some(0));
    assert!(
        y.view.aura(me, acolyte("aura_sanguine_flow")).is_none(),
        "spent"
    );
    assert_eq!(
        cast_bar(&y.cast_through(lance, dummy(0)), lance),
        Some(1500)
    );

    let mut y = yard(11, &[], YARD_SPOT);
    y.cast_through(acolyte("dark_mending"), y.me);
    assert_eq!(
        cast_bar(&y.cast_through(lance, dummy(0)), lance),
        Some(1500)
    );
}
