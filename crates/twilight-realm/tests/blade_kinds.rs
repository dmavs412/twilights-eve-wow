//! The Blade kinds (#678): a bleed taken aura that only its caster's bleeds read, a finisher that
//! ticks a bleed now, a cast that spends the caster's own stacks for another aura's, droplets
//! spilled around the caster, a Vanish that drops every threat and stealths in combat, and an
//! aura that makes any skill's next hit a crit. Played through the realm's seam by a test Rogue
//! Calling.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::cutthroat::{combo, energy};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

/// A Rogue-Class Calling with a 10-a-second bleed and one skill of each Blade kind. Its droplet
/// spills from no bleed tick, only from Blood Pool.
const BLADE_CALLINGS: &str = r#"
[[calling]]
key = "test_blade"
name = "Test Blade"
tier = 0
class = 4
cap = 60
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth", "leather"]
bleed_drop = "droplet"
bleed_drop_chance = 0

[[calling.skill]]
key = "builder"
spell_id = 1752
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "add_combo_points", amount = 1 }]

[[calling.skill]]
key = "bleed"
spell_id = 1943
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "periodic_damage", tick_ms = 1000, duration_ms = 12000, tick = { base = 10.0 } }]

[[calling.skill]]
key = "hemorrhage"
spell_id = 16511
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "mod_periodic_taken_pct", pct = 20, duration_ms = 15000, aura_spell_id = 1000420 }]

[[calling.skill]]
key = "open_veins"
spell_id = 1000501
power_cost = 25
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "tick_now", ticks_per_combo = 1 }]

[[calling.skill]]
key = "lust"
spell_id = 1000504
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "grant_aura", aura_spell_id = 1000411 }]

[[calling.skill]]
key = "red_frenzy"
spell_id = 1000502
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "consume_self_stacks", aura_spell_id = 1000411, grant_spell_id = 1000413 }]

[[calling.skill]]
key = "blood_pool"
spell_id = 1000503
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "spill_droplets", count = 3 }]

[[calling.skill]]
key = "poke"
spell_id = 1000505
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "school_damage", amount = { base = 1.0 } }]

[[calling.skill]]
key = "vanish"
spell_id = 1856
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "drop_threat" }, { kind = "grant_aura", aura_spell_id = 1000421 }]

[[calling.skill]]
key = "cold_blood"
spell_id = 14177
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "grant_aura", aura_spell_id = 1000422 }]

[[calling.skill]]
key = "strike"
spell_id = 2098
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 5.0]
effects = [{ kind = "weapon_strike", weapon = "normal", bonus = { base = 0.0 }, unavoidable = true }]

[[calling.aura]]
key = "bloodlust"
spell_id = 1000411
duration_ms = 10000
row = { max_stacks = 5, stack_rule = "stack" }
effects = [{ kind = "mod_damage_done_pct", pct = 3 }]

[[calling.aura]]
key = "droplet"
spell_id = 1000412
duration_ms = 8000
effects = [{ kind = "pickup", grants = "bloodlust", radius_yd = 1.0, scatter_min_yd = 3.0, scatter_max_yd = 6.0 }]

[[calling.aura]]
key = "red_frenzy_aura"
spell_id = 1000413
duration_ms = 10000
row = { max_stacks = 5, stack_rule = "stack" }
effects = [{ kind = "mod_melee_haste_pct", pct = 5 }]

[[calling.aura]]
key = "hemorrhage_aura"
spell_id = 1000420
duration_ms = 15000
effects = [{ kind = "mod_periodic_taken_pct", pct = 20 }]

[[calling.aura]]
key = "stealth_aura"
spell_id = 1000421
duration_ms = 0
effects = [{ kind = "stealth" }]

[[calling.aura]]
key = "cold_blood_aura"
spell_id = 1000422
duration_ms = 20000
charges = 1
effects = [{ kind = "skill_crit_pct", skill = "any", pct = 100 }]
"#;

const BUILDER: u32 = 1752;
const BLEED: u32 = 1943;
const HEMORRHAGE: u32 = 16511;
const OPEN_VEINS: u32 = 1000501;
const RED_FRENZY: u32 = 1000502;
const BLOOD_POOL: u32 = 1000503;
const LUST: u32 = 1000504;
const POKE: u32 = 1000505;
const VANISH: u32 = 1856;
const COLD_BLOOD: u32 = 14177;
const STRIKE: u32 = 2098;
const BLOODLUST: u32 = 1000411;
const DROPLET: u32 = 1000412;
const FRENZY: u32 = 1000413;
const STEALTH: u32 = 1000421;
const COLD_BLOOD_AURA: u32 = 1000422;

/// `UNIT_FLAG_IN_COMBAT`.
const UNIT_FLAG_IN_COMBAT: u32 = 0x0008_0000;

/// Where the test Rogue stands: east of the scorpid camp, out of its notice.
const STAND: [f32; 3] = [CAMP[0] + 40.0, CAMP[1], 0.0];

/// A level 20 test Rogue, fists only, at `STAND` on `seed`'s dice: the realm, the view, their
/// guid and the camp's mobs.
fn blade(seed: u64) -> (Realm, View, u64, Vec<Mob>) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", "level = 20\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_blade\"");
    let blob = drop_slot(&drop_slot(&blob, 15), 16);
    let mut realm = Realm::with_seed(seed).with_callings(BLADE_CALLINGS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: STAND,
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
    (realm, view, me, mobs)
}

/// Casts `spell_id` at `target`, asserting it went: what it sent.
fn cast_ok(
    realm: &mut Realm,
    view: &mut View,
    spell_id: u32,
    target: Option<u64>,
) -> Vec<SessionEvent> {
    realm.handle(cast(spell_id, target));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "{spell_id} is cast");
    events
}

/// The amounts `spell_id` ticked for, cast by `caster`, in `events`.
fn ticks(events: &[SessionEvent], spell_id: u32, caster: u64) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == spell_id && l.caster == caster => {
                l.ticks.iter().find_map(|t| match t {
                    benilla_protocol::messages::PeriodicTick::Damage { amount, .. } => {
                        Some(*amount)
                    }
                    _ => None,
                })
            }
            _ => None,
        })
        .collect()
}

/// The droplets `events` create: guid and centre.
fn droplets(events: &[SessionEvent]) -> Vec<(u64, [f32; 3])> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::DynamicObject,
                position,
                fields,
                ..
            } if fields.dynamicobject_spell_id() == Some(DROPLET) => Some((*guid, *position)),
            _ => None,
        })
        .collect()
}

// ── Bleed damage taken ──────────────────────────────────────────────────────────────────────────

/// A second Rogue-Class Calling with the same bleed and nothing else.
const OTHER_BLADE: &str = r#"
[[calling]]
key = "test_blade_b"
name = "Test Blade B"
tier = 0
class = 4
cap = 60
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth", "leather"]

[[calling.skill]]
key = "bleed"
spell_id = 1943
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "periodic_damage", tick_ms = 1000, duration_ms = 12000, tick = { base = 10.0 } }]
"#;

/// A Razormane Hunter at Thorn Hill, levels 11 and 12.
const HUNTER: u32 = 3265;
const THORN_HILL: [f32; 3] = [-38.801, -3234.81, 91.889];

/// Two level 20 Rogues of two test Callings, each in their own session, bleed the same Hunter
/// from 20 yards; player 0 then puts Hemorrhage on it. Player 0's ticks deal 20% more after it,
/// player 1's the same as before.
#[test]
fn bleed_damage_taken_raises_only_its_casters_bleeds() {
    use common::party::{NoGround, Party};
    let callings = format!("{BLADE_CALLINGS}\n{OTHER_BLADE}");
    let realm = Realm::with_seed(3).with_callings(&callings);
    let mut party = Party::edited(realm, &[20, 20], 0, Box::new(NoGround), |i, blob| {
        let calling = ["test_blade", "test_blade_b"][i];
        blob.replace(
            "calling = \"basic_warrior\"",
            &format!("calling = \"{calling}\""),
        )
    });
    party.go(
        0,
        [THORN_HILL[0] + 20.0, THORN_HILL[1], THORN_HILL[2]],
        std::f32::consts::PI,
    );
    party.go(1, [THORN_HILL[0] - 20.0, THORN_HILL[1], THORN_HILL[2]], 0.0);
    party.run(0.0);
    let (hunter, _) = party.unit(0, HUNTER, THORN_HILL);
    let (me, them) = (party.guids[0], party.guids[1]);
    party.cmd(0, cast(BLEED, Some(hunter)));
    assert_eq!(
        refusal(party.seen(0), BLEED),
        None,
        "player 0 bleeds the hunter"
    );
    party.cmd(1, cast(BLEED, Some(hunter)));
    assert_eq!(
        refusal(party.seen(1), BLEED),
        None,
        "player 1 bleeds the hunter"
    );
    let before = party.run(2.05);
    assert_eq!(ticks(&before[0], BLEED, me), [10, 10], "player 0's ticks");
    assert_eq!(ticks(&before[1], BLEED, them), [10, 10], "player 1's ticks");
    party.cmd(0, cast(HEMORRHAGE, Some(hunter)));
    assert_eq!(
        refusal(party.seen(0), HEMORRHAGE),
        None,
        "Hemorrhage is cast"
    );
    let after = party.run(3.0);
    assert_eq!(ticks(&after[0], BLEED, me), [12, 12, 12], "20% more");
    assert_eq!(ticks(&after[1], BLEED, them), [10, 10, 10], "not another's");
}

// ── Tick now ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn tick_now_at_three_points_deals_three_ticks_at_once_and_leaves_the_timer() {
    let (mut realm, mut view, me, mobs) = blade(1);
    let mob = &mobs[0];
    stand_at(&mut realm, mob, 20.0);
    for _ in 0..3 {
        cast_ok(&mut realm, &mut view, BUILDER, Some(mob.guid));
    }
    assert_eq!(combo(&view, me), (3, mob.guid));
    cast_ok(&mut realm, &mut view, BLEED, Some(mob.guid));
    let first = view.run_topped_up(&mut realm, mob.guid, 1.5);
    assert_eq!(ticks(&first, BLEED, me), [10], "one tick in 1.5 s");
    let forced = cast_ok(&mut realm, &mut view, OPEN_VEINS, Some(mob.guid));
    assert_eq!(ticks(&forced, BLEED, me), [10, 10, 10], "three at once");
    assert_eq!(combo(&view, me).0, 0, "the points are spent");
    // The bleed ticks on its own clock: 11 more, the last at 12 s from its cast.
    let rest = view.run_topped_up(&mut realm, mob.guid, 10.0);
    assert_eq!(ticks(&rest, BLEED, me).len(), 10, "ticks 2 to 11");
    assert!(view.aura(mob.guid, BLEED).is_some(), "still on at 11.5 s");
    let last = view.run_topped_up(&mut realm, mob.guid, 1.0);
    assert_eq!(ticks(&last, BLEED, me).len(), 1, "the 12th tick");
    assert!(view.aura(mob.guid, BLEED).is_none(), "gone at 12.5 s");
}

#[test]
fn tick_now_with_no_bleed_on_the_target_is_refused_at_no_cost() {
    let (mut realm, mut view, me, mobs) = blade(1);
    let mob = &mobs[0];
    stand_at(&mut realm, mob, 20.0);
    cast_ok(&mut realm, &mut view, BUILDER, Some(mob.guid));
    let before = energy(&view, me);
    realm.handle(cast(OPEN_VEINS, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert!(refusal(&events, OPEN_VEINS).is_some(), "refused");
    assert_eq!(energy(&view, me), before, "no Energy spent");
    assert_eq!(combo(&view, me), (1, mob.guid), "no point spent");
}

// ── Spending your own stacks ────────────────────────────────────────────────────────────────────

#[test]
fn consume_self_stacks_turns_four_bloodlust_into_four_of_the_granted_aura() {
    let (mut realm, mut view, me, _) = blade(1);
    realm.handle(cast(RED_FRENZY, None));
    let events = view.drain(&mut realm);
    assert!(
        refusal(&events, RED_FRENZY).is_some(),
        "refused with no Bloodlust"
    );
    for _ in 0..4 {
        cast_ok(&mut realm, &mut view, LUST, None);
    }
    assert_eq!(view.aura(me, BLOODLUST).map(|a| a.stacks), Some(4));
    cast_ok(&mut realm, &mut view, RED_FRENZY, None);
    assert!(view.aura(me, BLOODLUST).is_none(), "no Bloodlust left");
    assert_eq!(view.aura(me, FRENZY).map(|a| a.stacks), Some(4));
}

// ── Spilling droplets ───────────────────────────────────────────────────────────────────────────

#[test]
fn spill_droplets_puts_three_in_the_ring_around_the_caster_and_one_grants_its_aura() {
    let (mut realm, mut view, me, _) = blade(1);
    let events = cast_ok(&mut realm, &mut view, BLOOD_POOL, None);
    let spilled = droplets(&events);
    assert_eq!(spilled.len(), 3, "three droplets");
    for (_, at) in &spilled {
        let away = flat(*at, STAND);
        assert!((3.0..=6.0).contains(&away), "{away} yd from the caster");
    }
    let (object, at) = spilled[0];
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: at,
        orientation: 0.0,
    });
    let events = view.run(&mut realm, 0.2);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == object)),
        "the droplet is picked up"
    );
    assert_eq!(view.aura(me, BLOODLUST).map(|a| a.stacks), Some(1));
}

// ── Vanish ──────────────────────────────────────────────────────────────────────────────────────

#[test]
fn vanish_drops_every_threat_and_stealths_in_combat() {
    let (mut realm, mut view, me, mobs) = blade(2);
    let pulled = &mobs[..2];
    for mob in pulled {
        stand_at(&mut realm, mob, 20.0);
        cast_ok(&mut realm, &mut view, POKE, Some(mob.guid));
    }
    let fight = view.run(&mut realm, 5.0);
    for mob in pulled {
        assert!(
            fight.iter().any(|e| matches!(e,
                SessionEvent::AttackerState(a) if a.attacker == mob.guid && a.victim == me)),
            "{} swings at the Rogue",
            mob.guid
        );
    }
    assert_ne!(
        view.fields(me).unit_flags() & UNIT_FLAG_IN_COMBAT,
        0,
        "in combat"
    );
    let mut events = cast_ok(&mut realm, &mut view, VANISH, None);
    events.extend(view.run(&mut realm, 0.5));
    for mob in pulled {
        assert!(
            events.iter().any(|e| matches!(e,
                SessionEvent::AttackStop { attacker, victim } if *attacker == mob.guid && *victim == me)),
            "{} gives up the fight",
            mob.guid
        );
        assert!(
            !events.iter().any(|e| matches!(e,
                SessionEvent::AttackerState(a) if a.attacker == mob.guid && a.victim == me)),
            "{} swings no more",
            mob.guid
        );
    }
    assert_eq!(
        view.fields(me).unit_flags() & UNIT_FLAG_IN_COMBAT,
        0,
        "out of combat"
    );
    assert!(view.aura(me, STEALTH).is_some(), "stealthed");
}

// ── Any skill's next crit ───────────────────────────────────────────────────────────────────────

#[test]
fn a_one_charge_crit_for_any_skill_makes_the_next_skill_hit_a_crit_and_goes() {
    for seed in 0..10 {
        let (mut realm, mut view, me, mobs) = blade(seed);
        let mob = &mobs[0];
        stand_at(&mut realm, mob, 2.0);
        cast_ok(&mut realm, &mut view, COLD_BLOOD, None);
        assert!(view.aura(me, COLD_BLOOD_AURA).is_some(), "Cold Blood is up");
        let events = cast_ok(&mut realm, &mut view, STRIKE, Some(mob.guid));
        realm.handle(RealmCommand::AttackStop);
        let hits: Vec<bool> = common::cutthroat::damage_of(&events, STRIKE)
            .into_iter()
            .map(|(_, _, crit)| crit)
            .collect();
        // A miss spends no charge; another seed.
        if hits.is_empty() {
            continue;
        }
        assert_eq!(hits, [true], "seed {seed}: the strike crits");
        assert!(
            view.aura(me, COLD_BLOOD_AURA).is_none(),
            "seed {seed}: spent"
        );
        return;
    }
    panic!("no seed's strike landed");
}
