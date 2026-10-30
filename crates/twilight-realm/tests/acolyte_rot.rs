//! The Rot-Weaver: Creeping Blight grows a stack each tick and leeches half its ticks; Blood
//! Lance and Settle grow a worn Blight; Necrotic Detonation consumes the stacks for damage per
//! stack and heals around the target. Played through the realm's seam with a test Priest
//! Calling.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings;
use twilight_realm::{Realm, RealmCommand};

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
key = "creeping_blight"
spell_id = 589
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "periodic_damage", tick_ms = 1000, duration_ms = 10000, tick = { base = 2.0 }, max_stacks = 255, stack_rule = "replace", stacks_per_tick = 1, leech_pct = 50, school = "shadow" }]

[[calling.skill]]
key = "blood_lance"
spell_id = 8092
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
meta = { cast_time_ms = 1500 }
effects = [{ kind = "school_damage", school = "shadow", amount = { base = 1.0 } }, { kind = "grow_aura", aura_spell_id = 589, stacks = 1 }]

[[calling.skill]]
key = "settle"
spell_id = 15487
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "periodic_damage", tick_ms = 1000, duration_ms = 10000, tick = { base = 2.0 }, max_stacks = 255, aura_spell_id = 589, stack_rule = "replace", stacks_per_tick = 1, leech_pct = 50, school = "shadow", if_worn = "grow" }]

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
key = "necrotic_detonation"
spell_id = 15286
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "consume_aura", aura_spell_id = 589, amount = { base = 10.0 }, school = "shadow", per_stack = true, heal_allies_pct = 150, radius_yd = 10.0 }]

[[calling.skill]]
key = "wither"
spell_id = 980
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "periodic_damage", tick_ms = 60000, duration_ms = 60000, tick = { base = 0.0 }, max_stacks = 255, stack_rule = "stack", school = "shadow", weaken_per_stack_pct = 2, weaken_max_pct = 20 }]
"#;

const BLIGHT: u32 = 589;
const BLOOD_LANCE: u32 = 8092;
const BLEED: u32 = 2061;
const SETTLE: u32 = 15487;
const DETONATION: u32 = 15286;

/// A level 10 test Priest at `seed`, `yards` from the first camp mob, at half health so heals
/// show: the realm, the camp's mobs and the view.
fn world(seed: u64, yards: f32) -> (Realm, Vec<Mob>, View) {
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
    assert!(mobs.len() >= 2, "the camp is in view");
    stand_at(&mut realm, &mobs[0], yards);
    realm.handle(cast(BLEED, None));
    view.drain(&mut realm);
    (realm, mobs, view)
}

/// Casts `spell_id` at `target`, asserting it went: the events.
fn go(realm: &mut Realm, view: &mut View, spell_id: u32, target: u64) -> Vec<SessionEvent> {
    realm.handle(cast(spell_id, Some(target)));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "spell {spell_id} is cast");
    events
}

/// The heals logged under `spell_id` in `events`.
fn heals(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellHealLog(l) if l.spell_id == spell_id => Some(l.amount),
            _ => None,
        })
        .collect()
}

#[test]
fn creeping_blight_grows_each_tick_and_a_recast_starts_it_over() {
    let (mut realm, mobs, mut view) = world(1, 20.0);
    let mob = &mobs[0];
    let dealt = |events: &[SessionEvent]| -> Vec<u32> {
        events
            .iter()
            .filter_map(|e| match e {
                SessionEvent::PeriodicAuraLog(l) if l.spell_id == BLIGHT => match l.ticks[0] {
                    benilla_protocol::messages::PeriodicTick::Damage { amount, .. } => Some(amount),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    };
    go(&mut realm, &mut view, BLIGHT, mob.guid);
    assert_eq!(view.aura(mob.guid, BLIGHT).map(|a| a.stacks), Some(1));
    let mut log = Vec::new();
    let mut healed = Vec::new();
    for _ in 0..3 {
        let tick = view.run(&mut realm, 1.0);
        log.extend(dealt(&tick));
        healed.extend(heals(&tick, BLIGHT));
    }
    assert_eq!(log, [2, 4, 6], "one more stack after each tick");
    assert_eq!(healed, [1, 2, 3], "half of each tick back");
    assert_eq!(view.aura(mob.guid, BLIGHT).map(|a| a.stacks), Some(4));
    go(&mut realm, &mut view, BLIGHT, mob.guid);
    assert_eq!(
        view.aura(mob.guid, BLIGHT).map(|a| a.stacks),
        Some(1),
        "a recast starts over"
    );
    assert_eq!(dealt(&view.run(&mut realm, 1.0)), [2]);
}

#[test]
fn blood_lance_adds_a_stack_and_keeps_the_timer() {
    let (mut realm, mobs, mut view) = world(5, 20.0);
    let mob = &mobs[0];
    go(&mut realm, &mut view, BLIGHT, mob.guid);
    realm.handle(cast(BLOOD_LANCE, Some(mob.guid)));
    view.drain(&mut realm);
    // The first tick at 1 s leaves two stacks; the lance lands at 1.5 s.
    view.run(&mut realm, 1.55);
    assert_eq!(view.aura(mob.guid, BLIGHT).map(|a| a.stacks), Some(3));
    let tick = view.run(&mut realm, 0.5);
    let dealt: Vec<u32> = tick
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == BLIGHT => match l.ticks[0] {
                benilla_protocol::messages::PeriodicTick::Damage { amount, .. } => Some(amount),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(dealt, [6], "three stacks of 2");
    view.run(&mut realm, 7.85);
    assert!(view.aura(mob.guid, BLIGHT).is_some(), "9.9 s in");
    view.run(&mut realm, 0.15);
    assert!(
        view.aura(mob.guid, BLIGHT).is_none(),
        "gone 10.05 s after its own cast"
    );

    let (mut realm, mobs, mut view) = world(6, 20.0);
    go(&mut realm, &mut view, BLOOD_LANCE, mobs[0].guid);
    view.run(&mut realm, 1.6);
    assert!(
        view.aura(mobs[0].guid, BLIGHT).is_none(),
        "a lance puts no Blight on a mob without one"
    );
}

#[test]
fn settle_grows_a_worn_blight_and_puts_on_a_missing_one() {
    let (mut realm, mobs, mut view) = world(7, 20.0);
    let mob = &mobs[0];
    go(&mut realm, &mut view, SETTLE, mob.guid);
    assert_eq!(view.aura(mob.guid, BLIGHT).map(|a| a.stacks), Some(1));
    view.run(&mut realm, 2.05);
    assert_eq!(view.aura(mob.guid, BLIGHT).map(|a| a.stacks), Some(3));
    go(&mut realm, &mut view, SETTLE, mob.guid);
    assert_eq!(view.aura(mob.guid, BLIGHT).map(|a| a.stacks), Some(4));
    view.run(&mut realm, 7.8);
    assert!(view.aura(mob.guid, BLIGHT).is_some(), "9.85 s in");
    view.run(&mut realm, 0.2);
    assert!(
        view.aura(mob.guid, BLIGHT).is_none(),
        "fades 10.05 s after the first application"
    );
}

#[test]
fn necrotic_detonation_at_five_stacks_deals_five_times_heals_and_clears_the_blight() {
    let detonate = |stacks: usize| {
        let (mut realm, mobs, mut view) = world(3, 8.0);
        let mob = &mobs[0];
        go(&mut realm, &mut view, BLIGHT, mob.guid);
        view.run(&mut realm, (stacks - 1) as f32 + 0.05);
        let events = go(&mut realm, &mut view, DETONATION, mob.guid);
        assert!(
            view.aura(mob.guid, BLIGHT).is_none(),
            "the Blight is consumed"
        );
        let dealt = events
            .iter()
            .find_map(|e| match e {
                SessionEvent::SpellDamageLog(l) if l.spell_id == DETONATION => Some(l.damage),
                _ => None,
            })
            .expect("the detonation's damage");
        (dealt, heals(&events, DETONATION))
    };
    let (one, _) = detonate(1);
    let (five, healed) = detonate(5);
    assert_eq!(five, one * 5, "five times one stack");
    assert_eq!(healed, [five * 150 / 100], "150% of it heals the caster");
}

// Creeping Blight's and Miasma's Skill Choices (#508), played by a shipped Acolyte at 24, most of
// them at the Test Grounds' dummies, which take hits but never fight back or die.

use common::acolyte::{
    acolyte, acolyte_scene, blight_stacks, dummy, row_choice, yard, Yard, DUMMY_SPOTS,
};

/// Where the Acolyte stands in the Test Grounds: 17.5 yd from the first dummy, so a Blood Lance's
/// heal over time, which looks 15 yd around the target, passes them by.
const YARD_SPOT: [f32; 2] = [-9655.0, 255.0];
/// Seeds enough for the rolls to land and to miss.
const SEEDS: u64 = 30;

fn blight() -> u32 {
    acolyte("aura_creeping_blight")
}

/// The units in `events` that first show `spell_id` in an aura slot, each after the unit whose
/// `spell_id` tick logged last before it: `(source, unit)`. `known` holds the units already
/// wearing it, and takes the new ones.
fn spreads(events: &[SessionEvent], spell_id: u32, known: &mut Vec<u64>) -> Vec<(u64, u64)> {
    let mut last = None;
    let mut out = Vec::new();
    for e in events {
        match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == spell_id => last = Some(l.target),
            SessionEvent::ObjectValues { guid, fields }
                if !known.contains(guid) && fields.raw_fields().any(|(_, v)| v == spell_id) =>
            {
                known.push(*guid);
                out.extend(last.map(|source| (source, *guid)));
            }
            _ => {}
        }
    }
    out
}

/// Epidemic's ticks copy the Blight, with its stacks, onto the next dummy 6 yd off; the copy's own
/// ticks then copy it onto the third, which stands 12 yd from the first and so out of its 10 yd
/// reach. Plain Creeping Blight never spreads.
#[test]
fn epidemic_copies_the_blight_with_its_stacks_and_the_copy_spreads_on() {
    let epidemic = row_choice("creeping_blight", "epidemic");
    let mut chained = 0;
    for seed in 0..SEEDS {
        let mut y = yard(seed, &[("creeping_blight", epidemic)], YARD_SPOT);
        y.cast_through(epidemic, dummy(0));
        let mut known = vec![dummy(0)];
        let mut seen = Vec::new();
        for _ in 0..125 {
            let events = y.run(0.1);
            for (source, unit) in spreads(&events, blight(), &mut known) {
                assert_eq!(
                    blight_stacks(&y.view, unit),
                    blight_stacks(&y.view, source),
                    "seed {seed}: the copy carries the source's stacks"
                );
                seen.push((source, unit));
            }
        }
        assert!(
            !seen.contains(&(dummy(0), dummy(2))),
            "seed {seed}: 12 yd is out of reach"
        );
        if seen.contains(&(dummy(0), dummy(1))) && seen.contains(&(dummy(1), dummy(2))) {
            chained += 1;
        }
    }
    assert!(chained > 0, "no seed spread the Blight down the row");

    let mut y = yard(1, &[], YARD_SPOT);
    y.cast_through(acolyte("creeping_blight"), dummy(0));
    for _ in 0..125 {
        y.run(0.1);
        for i in 1..4 {
            assert_eq!(blight_stacks(&y.view, dummy(i)), 0, "plain Blight spreads");
        }
    }
}

/// The Blight ticks on dummy 0 in `events`, each with whether aura_virulence went up on the
/// Acolyte before the next tick.
fn virulence_rolls(events: &[SessionEvent], me: u64, virulence: u32) -> Vec<bool> {
    let mut rolls: Vec<bool> = Vec::new();
    for e in events {
        match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == blight() && l.target == dummy(0) => {
                rolls.push(false)
            }
            SessionEvent::ObjectValues { guid, fields }
                if *guid == me && fields.raw_fields().any(|(_, v)| v == virulence) =>
            {
                if let Some(last) = rolls.last_mut() {
                    *last = true;
                }
            }
            _ => {}
        }
    }
    rolls
}

/// A Virulence Blight on dummy 0 for its 12 s, with Blood Lances at it throughout when `lances`
/// (each adds a stack): its ticks, each with whether it granted aura_virulence.
fn virulence_run(seed: u64, choice: u32, lances: bool) -> (Yard, Vec<bool>) {
    let mut y = yard(seed, &[("creeping_blight", choice)], YARD_SPOT);
    let virulence = acolyte("aura_virulence");
    let mut events = y.cast_through(choice, dummy(0));
    let lance = acolyte("blood_lance");
    while lances && events.len() < 100_000 {
        events.extend(y.cast_through(lance, dummy(0)));
        if y.view.aura(dummy(0), blight()).is_none() {
            break;
        }
    }
    events.extend(y.run(12.5));
    let rolls = virulence_rolls(&events, y.me, virulence);
    (y, rolls)
}

/// Each Virulence tick grants aura_virulence at 2.5% a stack: a Blight a stream of Lances grows
/// past 10 stacks grants it more often than one left at 1 to 4. Plain Creeping Blight never does.
#[test]
fn virulence_grants_its_buff_on_a_winning_roll_more_often_with_more_stacks() {
    let choice = row_choice("creeping_blight", "virulence");
    let (mut few, mut many, mut shown) = (0, 0, false);
    for seed in 0..SEEDS {
        let (_, rolls) = virulence_run(seed, choice, false);
        assert_eq!(rolls.len(), 4, "seed {seed}: four ticks");
        few += rolls.iter().filter(|&&r| r).count();
        let (y, rolls) = virulence_run(seed, choice, true);
        many += rolls.iter().filter(|&&r| r).count();
        shown |= y.view.aura(y.me, acolyte("aura_virulence")).is_some();
    }
    assert!(many > few, "{many} grants with Lances, {few} without");
    assert!(few > 0, "a low stack still rolls");
    assert!(shown, "a grant shows aura_virulence on the Acolyte");

    let plain = acolyte("creeping_blight");
    for seed in 0..5 {
        let mut y = yard(seed, &[], YARD_SPOT);
        let events = [y.cast_through(plain, dummy(0)), y.run(12.5)].concat();
        let rolls = virulence_rolls(&events, y.me, acolyte("aura_virulence"));
        assert!(
            rolls.iter().all(|&r| !r),
            "seed {seed}: plain grants nothing"
        );
    }
}

/// While Withering Blight is on the mob, each of its blows at the Acolyte takes 2% less of their
/// health a stack than its report: 10% at 5 stacks. Plain Creeping Blight cuts nothing.
#[test]
fn withering_blight_at_5_stacks_cuts_the_mobs_swing_by_10_percent() {
    let withering = row_choice("creeping_blight", "withering_blight");
    let cut = |choice: u32, rows: &[(&str, u32)]| -> Vec<(u32, u32, u32)> {
        let mut s = acolyte_scene(4, rows, common::vanguard::STURDY);
        let (me, mob) = (s.me, s.mob.guid);
        let lance = acolyte("blood_lance");
        common::initiate::cast_through(&mut s.realm, &mut s.view, choice, mob);
        while blight_stacks(&s.view, mob) < 5 {
            common::initiate::cast_through(&mut s.realm, &mut s.view, lance, mob);
        }
        let mut blows = Vec::new();
        for _ in 0..120 {
            let (health, stacks) = (s.view.health(me), blight_stacks(&s.view, mob));
            let events = s.view.run(&mut s.realm, 0.1);
            let landed: Vec<(u64, u32)> = events
                .iter()
                .filter_map(|e| match e {
                    SessionEvent::AttackerState(a) if a.victim == me && a.damage > 0 => {
                        Some((a.attacker, a.damage))
                    }
                    _ => None,
                })
                .collect();
            let other = events.iter().any(|e| match e {
                SessionEvent::SpellHealLog(l) => l.target == me,
                SessionEvent::PeriodicAuraLog(l) => l.target == me,
                SessionEvent::SpellDamageLog(l) => l.target == me,
                _ => false,
            });
            if let (&[(from, reported)], false, true) = (&landed[..], other, stacks >= 5) {
                if from == mob {
                    blows.push((stacks, reported, health - s.view.health(me)));
                }
            }
        }
        blows
    };
    let blows = cut(withering, &[("creeping_blight", withering)]);
    assert!(
        blows.iter().any(|b| b.0 == 5),
        "a blow at 5 stacks: {blows:?}"
    );
    for &(stacks, reported, lost) in &blows {
        let pct = (2 * stacks).min(20);
        assert_eq!(
            lost,
            reported * (100 - pct) / 100,
            "{stacks} stacks: {blows:?}"
        );
    }
    let blows = cut(acolyte("creeping_blight"), &[]);
    assert!(!blows.is_empty());
    assert!(blows.iter().all(|&(_, r, l)| l == r), "{blows:?}");
}

/// Casts the Miasma version `spell_id` at `dest` in the Test Grounds, asserting it went.
fn cloud_at(y: &mut Yard, spell_id: u32, dest: [f32; 3]) -> Vec<SessionEvent> {
    y.realm
        .handle(RealmCommand::CastSpellAtDest { spell_id, dest });
    let events = y.view.drain(&mut y.realm);
    assert_eq!(refusal(&events, spell_id), None, "{spell_id} is cast");
    events
}

/// Between dummies 0 and 1, both inside a cloud of 8 yd.
const BETWEEN: [f32; 3] = [-9640.0, 261.0, 47.2];

/// Breeding Ground adds a stack every 2 s to the Blighted dummy inside, besides the Blight's own
/// tick every 3 s, and nothing to the dummy inside without a Blight; plain Miasma adds none.
#[test]
fn breeding_ground_grows_a_blighted_dummy_inside_every_2_seconds() {
    let breeding = row_choice("miasma", "breeding_ground");
    let grown = |rows: &[(&str, u32)], cloud: u32| -> (u32, u32) {
        let mut y = yard(2, rows, YARD_SPOT);
        y.cast_through(acolyte("creeping_blight"), dummy(0));
        cloud_at(&mut y, cloud, BETWEEN);
        y.run(6.5);
        (
            blight_stacks(&y.view, dummy(0)),
            blight_stacks(&y.view, dummy(1)),
        )
    };
    let (bred, bare) = grown(&[("miasma", breeding)], breeding);
    let (plain, _) = grown(&[], acolyte("miasma"));
    // Ticks at 3 and 6 s grow both; the cloud's clock at 2, 4 and 6 s grows Breeding Ground's.
    assert_eq!(plain, 3, "one stack and two ticks");
    assert_eq!(bred, plain + 3, "three more from the cloud");
    assert_eq!(bare, 0, "a dummy without the Blight gains nothing");
}

/// Noxious Vapors puts its own enemy aura on a dummy inside: Miasma's ticks and 10 points less to
/// hit. Plain Miasma puts its plain aura.
#[test]
fn noxious_vapors_enemy_aura_is_on_a_dummy_inside() {
    let vapors = row_choice("miasma", "noxious_vapors");
    let aura = acolyte("aura_noxious_vapors");
    let mut y = yard(3, &[("miasma", vapors)], YARD_SPOT);
    cloud_at(&mut y, vapors, BETWEEN);
    y.run(0.5);
    assert!(y.view.aura(dummy(0), aura).is_some(), "inside");
    assert!(y.view.aura(dummy(3), aura).is_none(), "15 yd off");
    let row = callings::aura("aura_noxious_vapors").expect("the aura loads");
    assert!(row
        .effects
        .contains(&callings::AuraEffect::ModHitChance { pct: -10 }));
    assert!(row
        .effects
        .iter()
        .any(|e| matches!(e, callings::AuraEffect::PeriodicDamage { .. })));

    let mut y = yard(3, &[], YARD_SPOT);
    cloud_at(&mut y, acolyte("miasma"), BETWEEN);
    y.run(0.5);
    assert!(y.view.aura(dummy(0), aura).is_none());
    assert!(y.view.aura(dummy(0), acolyte("aura_miasma")).is_some());
}

/// Miasma's damage ticks on `unit` in `events`.
fn miasma_ticks(events: &[SessionEvent], unit: u64) -> usize {
    events
        .iter()
        .filter(|e| {
            matches!(e, SessionEvent::PeriodicAuraLog(l)
                if l.spell_id == acolyte("aura_miasma") && l.target == unit)
        })
        .count()
}

/// Shroud of Rot's cloud sits on the Acolyte: cast 5 yd from dummy 0, it follows them to 5 yd
/// from dummy 3, 18 yd down the row, which takes its ticks while dummy 0 takes none. Its
/// DynamicObject goes and comes back at their feet. Plain Miasma stays where it was dropped.
#[test]
fn shroud_of_rot_moves_with_the_acolyte() {
    let shroud = row_choice("miasma", "shroud_of_rot");
    let near_first = [DUMMY_SPOTS[0][0] - 5.0, DUMMY_SPOTS[0][1]];
    let near_last = [DUMMY_SPOTS[3][0] - 5.0, DUMMY_SPOTS[3][1]];
    let mut y = yard(5, &[("miasma", shroud)], near_first);
    y.realm.handle(cast(shroud, None));
    let cast = y.view.drain(&mut y.realm);
    assert_eq!(refusal(&cast, shroud), None, "Shroud of Rot is cast");
    let first = dynamic_object(&cast).expect("the cloud shows");
    let moved = [y.walk(near_last), y.run(3.0)].concat();
    assert!(
        moved
            .iter()
            .any(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == first.0)),
        "the cloud's old object goes"
    );
    let again = dynamic_object(&moved).expect("the cloud shows again");
    assert!(
        flat(again.1, [near_last[0], near_last[1], 0.0]) < 1.0,
        "at the Acolyte's feet: {again:?}"
    );
    assert!(
        miasma_ticks(&moved, dummy(3)) > 0,
        "the cloud reached dummy 3"
    );
    assert_eq!(miasma_ticks(&moved, dummy(0)), 0, "dummy 0 is left behind");

    let mut y = yard(5, &[], near_first);
    let at = [near_first[0], near_first[1], 47.2];
    cloud_at(&mut y, acolyte("miasma"), at);
    let moved = [y.walk(near_last), y.run(3.0)].concat();
    assert!(miasma_ticks(&moved, dummy(0)) > 0, "plain Miasma stays");
    assert_eq!(miasma_ticks(&moved, dummy(3)), 0);
}

/// The first DynamicObject `events` create: its guid and centre.
fn dynamic_object(events: &[SessionEvent]) -> Option<(u64, [f32; 3])> {
    events.iter().find_map(|e| match e {
        SessionEvent::ObjectCreate {
            guid,
            kind: EntityKind::DynamicObject,
            position,
            ..
        } => Some((*guid, *position)),
        _ => None,
    })
}

/// A damage over time that weakens by stacks (`weaken_per_stack_pct` 2, `weaken_max_pct` 20) at 15
/// stacks cuts each blow of the level 8 or 9 mob south of the Crossroads by 20%, not 30%. Played by
/// the test Calling, whose `wither` adds a stack a cast and deals nothing, so the mob lives.
#[test]
fn a_weakening_by_stacks_at_15_stacks_stops_at_20_percent() {
    let wither = 980;
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Vellune", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace(
            "level = 1
",
            "level = 10
",
        )
        .replace("calling = \"basic_warrior\"", "calling = \"test_priest\"");
    let mut realm = Realm::with_seed(9).with_callings(TEST_CALLINGS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    let mob = common::acolyte::walk_west_of(&mut realm, &mut view, common::vanguard::STURDY);
    pull(&mut realm, &mut view, &mob);
    for _ in 0..15 {
        go(&mut realm, &mut view, wither, mob.guid);
    }
    assert_eq!(view.aura(mob.guid, wither).map(|a| a.stacks), Some(15));
    let mut blows = Vec::new();
    for _ in 0..100 {
        let health = view.health(me);
        let events = view.run(&mut realm, 0.1);
        // One blow in the tick, from the withered mob: a campmate it pulled has no stacks.
        if let [(from, reported)] = events
            .iter()
            .filter_map(|e| match e {
                SessionEvent::AttackerState(a) if a.victim == me && a.damage > 0 => {
                    Some((a.attacker, a.damage))
                }
                _ => None,
            })
            .collect::<Vec<_>>()[..]
        {
            if from == mob.guid {
                blows.push((reported, health - view.health(me)));
            }
        }
    }
    assert!(!blows.is_empty());
    for &(reported, lost) in &blows {
        assert_eq!(lost, reported * 80 / 100, "{blows:?}");
    }
    assert!(
        blows.iter().any(|&(r, _)| r * 80 / 100 != r * 70 / 100),
        "a blow that tells 20% from 30%: {blows:?}"
    );
}

/// A Razormane Hunter at Thorn Hill, levels 11 and 12.
const HUNTER: u32 = 3265;
const THORN_HILL: [f32; 3] = [-38.801, -3234.81, 91.889];

/// A weakening by stacks cuts the mob's hits at anyone, not only at the one who put it on: two
/// level 20 test Priests, each in their own session; the Razormane Hunter fights player 1, and
/// player 0 puts 10 stacks of `wither` (2% a stack) on it from 25 yards. Each of its blows at
/// player 1 then takes 20% less of their health than it reports.
#[test]
fn a_weakening_by_stacks_cuts_the_mobs_blows_at_another_player() {
    use common::party::{NoGround, Party};
    let wither = 980;
    let realm = Realm::with_seed(3).with_callings(TEST_CALLINGS);
    let mut party = Party::edited(realm, &[20, 20], 0, Box::new(NoGround), |_, blob| {
        blob.replace("calling = \"basic_warrior\"", "calling = \"test_priest\"")
    });
    let near = [THORN_HILL[0] + 25.0, THORN_HILL[1], THORN_HILL[2]];
    party.go(0, near, std::f32::consts::PI);
    party.go(1, THORN_HILL, 0.0);
    party.run(0.0);
    let (hunter, _) = party.unit(1, HUNTER, THORN_HILL);
    let them = party.guids[1];
    assert!(
        party.seen(1).iter().any(|e| matches!(e,
            SessionEvent::AttackStart { attacker, victim } if *attacker == hunter && *victim == them)),
        "the hunter fights player 1"
    );
    for _ in 0..10 {
        party.cmd(0, cast(wither, Some(hunter)));
        assert_eq!(refusal(party.seen(0), wither), None, "wither is cast");
    }
    let mut caster = View::default();
    caster.see(party.seen(0));
    assert_eq!(caster.aura(hunter, wither).map(|a| a.stacks), Some(10));
    let mut theirs = View::default();
    theirs.see(party.seen(1));
    let mut blows = Vec::new();
    // Ten seconds of blows, stopping while a full one cannot kill.
    for _ in 0..100 {
        let health = theirs.health(them);
        if health < 100 {
            break;
        }
        let got = party.run(0.1);
        theirs.see(&got[1]);
        // One blow in the tick, from the withered hunter: a campmate it pulled has no stacks.
        if let [(from, reported)] = got[1]
            .iter()
            .filter_map(|e| match e {
                SessionEvent::AttackerState(a) if a.victim == them && a.damage > 0 => {
                    Some((a.attacker, a.damage))
                }
                _ => None,
            })
            .collect::<Vec<_>>()[..]
        {
            if from == hunter {
                blows.push((reported, health - theirs.health(them)));
            }
        }
    }
    assert!(!blows.is_empty(), "the hunter lands blows on player 1");
    for &(reported, lost) in &blows {
        assert_eq!(lost, reported * 80 / 100, "{blows:?}");
    }
}
