//! Creature Spells (ADR 0008): a creature that carries `corruption_creature` casts it at the
//! player it fights: the cast bar, then the curse, which ticks shadow damage for its duration.

mod common;

use benilla_protocol::messages::{MonsterMoveFacing, PeriodicAuraLog, PeriodicTick};
use benilla_protocol::{EntityKind, SessionEvent};
use common::fight;
use common::*;
use twilight_realm::rules::melee::armor_reduced;

/// The Vile Familiar near the Crossroads.
const VILE_FAMILIAR: u32 = 3101;
const CORRUPTION: &str = "corruption_creature";
/// Its own spell id, which the cast bar, the debuff and the damage log name.
static CORRUPTION_SPELL: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| creature_spell(CORRUPTION));
/// The shadow school in a damage log.
const SHADOW: u32 = 5;
const HEAL: &str = "abbot_heal";
static HEAL_SPELL: std::sync::LazyLock<u32> = std::sync::LazyLock::new(|| creature_spell(HEAL));
static GOUGE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("basic_warrior", "gouge"));
/// Whole rage points Gouge costs.
const GOUGE_RAGE: u32 = 15;
/// Low enough that the warrior lives through the curse and builds rage, as in `gouge.rs`.
const LEVEL: u8 = 3;

#[test]
fn a_caster_starts_its_cast_and_the_curse_ticks_shadow_damage_after_it() {
    let realm = Realm::with_seed(1).with_creature_spells(VILE_FAMILIAR, &[CORRUPTION]);
    // Strong enough to kill the familiar soon after its first cast, so no second one refreshes
    // the curse: it then runs its whole length with its caster dead.
    let (mut realm, me, mob) = warrior(realm, 60);
    stand_by(&mut realm, mob.1);
    let mut events = run(&mut realm, 2.2);
    realm.handle(RealmCommand::AttackSwing { guid: mob.0 });
    events.extend(run(&mut realm, 20.0).into_iter().map(|(t, e)| (t + 2.2, e)));

    let at = |pred: &dyn Fn(&SessionEvent) -> bool| -> Vec<f32> {
        events
            .iter()
            .filter(|(_, e)| pred(e))
            .map(|(t, _)| *t)
            .collect()
    };
    let starts = at(&|e| {
        matches!(e, SessionEvent::SpellStart { caster, spell_id, cast_time_ms: 2000, target: Some(t), .. }
            if *caster == mob.0 && *spell_id == *CORRUPTION_SPELL && *t == me)
    });
    let goes = at(&|e| {
        matches!(e, SessionEvent::SpellGo { caster, spell_id, hits, .. }
            if *caster == mob.0 && *spell_id == *CORRUPTION_SPELL && hits.contains(&me))
    });
    assert!(!starts.is_empty(), "the familiar casts");
    assert_eq!(goes.len(), 1, "and the cast goes off, once before it dies");
    let cast = goes[0] - starts[0];
    assert!((cast - 2.0).abs() < 0.15, "after its 2 s cast: {cast}");

    // The curse sits on the player's aura slots under the client's spell id.
    let shown = events.iter().any(|(t, e)| {
        *t >= goes[0]
            && matches!(e, SessionEvent::ObjectValues { guid, fields }
                if *guid == me && (0..48).any(|s| fields.unit_aura(s).is_some_and(|a| a.spell_id == *CORRUPTION_SPELL)))
    });
    assert!(shown, "the player wears the curse");

    // 6 ticks of 10 shadow damage, one each 3 s after the cast.
    let ticks: Vec<(f32, u32)> = events
        .iter()
        .filter_map(|(t, e)| match e {
            SessionEvent::PeriodicAuraLog(PeriodicAuraLog {
                target,
                caster,
                spell_id,
                ticks,
            }) if *target == me && *caster == mob.0 && *spell_id == *CORRUPTION_SPELL => {
                match ticks[..] {
                    [PeriodicTick::Damage { amount, school, .. }] => {
                        assert_eq!(school, SHADOW);
                        Some((*t, amount))
                    }
                    _ => None,
                }
            }
            _ => None,
        })
        .collect();
    assert_eq!(ticks.len(), 6, "{ticks:?}");
    assert!(ticks.iter().all(|(_, amount)| *amount == 10));
    for (n, (t, _)) in ticks.iter().enumerate() {
        let due = goes[0] + 3.0 * (n + 1) as f32;
        assert!((t - due).abs() < 0.15, "{ticks:?}");
    }
}

#[test]
fn a_charge_carries_its_caster_to_the_player_and_lands_a_blow() {
    let realm = Realm::with_seed(1).with_creature_spells(VILE_FAMILIAR, &["rushing_charge"]);
    let (mut realm, me, mob) = warrior(realm, 10);
    stand_by(&mut realm, mob.1);
    run(&mut realm, 0.5);
    // Out past the charge's 8 yd minimum, within its 25 yd.
    let away = [mob.1[0] + 15.0, mob.1[1], mob.1[2]];
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: away,
        orientation: std::f32::consts::PI,
    });
    let events = run(&mut realm, 4.0);
    let rush = events.iter().position(|(_, e)| {
        matches!(e, SessionEvent::MonsterMove { guid, path, facing: MonsterMoveFacing::Target(t), .. }
            if *guid == mob.0 && *t == me && path.len() == 2)
    });
    let rush = rush.expect("the familiar charges");
    let SessionEvent::MonsterMove { path, .. } = &events[rush].1 else {
        unreachable!()
    };
    let end = path[1];
    let gap = ((end[0] - away[0]).powi(2) + (end[1] - away[1]).powi(2)).sqrt();
    assert!(gap < 5.0, "it ends in melee reach of the player: {gap}");
    assert!(
        events[rush..].iter().any(|(_, e)| matches!(
            e, SessionEvent::AttackerState(a) if a.attacker == mob.0 && a.victim == me
        )),
        "and lands a blow"
    );
}

/// The physical school in a damage log.
const PHYSICAL: u8 = 0;

#[test]
fn a_hit_deals_its_percent_of_the_casters_average_swing() {
    for (key, pct) in [("bite_wolf", 100), ("backstab_cutpurse", 150)] {
        let spell_id = creature_spell(key);
        let mut plain = 0;
        for seed in 0..10 {
            let realm = Realm::with_seed(seed).with_creature_spells(VILE_FAMILIAR, &[key]);
            let (mut realm, me, mob, mut view) = warrior_seen(realm, 30);
            stand_by(&mut realm, mob.1);
            let mut events = Vec::new();
            for _ in 0..200 {
                realm.tick(0.1, &FlatGround);
                events.extend(view.drain(&mut realm));
            }
            let caster = &view.objects[&mob.0];
            let swing = (caster.unit_min_damage().unwrap() as u32
                + caster.unit_max_damage().unwrap() as u32)
                / 2;
            let level = caster.unit_level().unwrap() as u8;
            let armor = view.objects[&me].unit_resistance(0).unwrap() as u32;
            let want = armor_reduced(swing * pct / 100, armor, level);
            for e in &events {
                let SessionEvent::SpellDamageLog(l) = e else {
                    continue;
                };
                if l.attacker != mob.0 || l.spell_id != spell_id {
                    continue;
                }
                assert_eq!((l.target, l.school), (me, PHYSICAL), "{key} seed {seed}");
                // A plain hit: no crit flag, nothing blocked.
                if l.hit_info == 0 && l.blocked == 0 {
                    assert_eq!(l.damage, want, "{key} seed {seed}: {swing} x {pct}%");
                    plain += 1;
                }
            }
        }
        assert!(plain > 0, "{key}: some seed lands a plain hit");
    }
}

#[test]
fn a_hit_takes_the_place_of_a_swing() {
    let spell_id = creature_spell("bite_wolf");
    let realm = Realm::with_seed(3).with_creature_spells(VILE_FAMILIAR, &["bite_wolf"]);
    let (mut realm, me, mob, mut view) = warrior_seen(realm, 30);
    stand_by(&mut realm, mob.1);
    let mut events = Vec::new();
    for n in 1..=300 {
        realm.tick(0.1, &FlatGround);
        events.extend(
            view.drain(&mut realm)
                .into_iter()
                .map(|e| (n as f32 * 0.1, e)),
        );
    }
    let attack = view.objects[&mob.0].unit_base_attack_time(0).unwrap() as f32 / 1000.0;
    let at = |pred: &dyn Fn(&SessionEvent) -> bool| -> Vec<f32> {
        events
            .iter()
            .filter(|(_, e)| pred(e))
            .map(|(t, _)| *t)
            .collect()
    };
    let swings = at(
        &|e| matches!(e, SessionEvent::AttackerState(a) if a.attacker == mob.0 && a.victim == me),
    );
    let bites = at(
        &|e| matches!(e, SessionEvent::SpellGo { caster, spell_id: s, .. } if *caster == mob.0 && *s == spell_id),
    );
    assert!(bites.len() > 2, "{bites:?}");
    for bite in &bites {
        let Some(next) = swings.iter().find(|t| *t > bite) else {
            continue;
        };
        assert!(
            next - bite >= attack - 0.15,
            "a swing {:.1} s after the bite at {bite}, the attack time is {attack} s",
            next - bite
        );
    }
}

#[test]
fn a_spell_with_a_minimum_range_waits_until_its_target_is_past_it() {
    let spell_id = creature_spell("throw_kobold");
    // The player keeps `yd` yards past wherever the familiar last ran to, for 3 s.
    let started = |realm: &mut Realm, mob: (u64, [f32; 3]), yd: f32| {
        stand_by(realm, mob.1);
        run(realm, 0.5);
        let mut to = mob.1;
        let mut began = false;
        for _ in 0..30 {
            realm.handle(RealmCommand::Move {
                motion: Default::default(),
                position: [to[0] + yd, to[1], to[2]],
                orientation: std::f32::consts::PI,
            });
            realm.tick(0.1, &FlatGround);
            for e in drain(realm) {
                match e {
                    SessionEvent::MonsterMove { guid, path, .. } if guid == mob.0 => {
                        to = *path.last().unwrap_or(&to);
                    }
                    SessionEvent::SpellStart {
                        caster,
                        spell_id: s,
                        ..
                    } if caster == mob.0 && s == spell_id => began = true,
                    _ => {}
                }
            }
        }
        began
    };
    let realm = Realm::with_seed(1).with_creature_spells(VILE_FAMILIAR, &["throw_kobold"]);
    let (mut realm, _, mob) = warrior(realm, 30);
    assert!(!started(&mut realm, mob, 3.0), "no throw at 3 yd");
    let realm = Realm::with_seed(1).with_creature_spells(VILE_FAMILIAR, &["throw_kobold"]);
    let (mut realm, _, mob) = warrior(realm, 30);
    assert!(started(&mut realm, mob, 12.0), "a throw at 12 yd");
}

#[test]
fn a_cooldown_range_rearms_inside_its_range() {
    let spell_id = creature_spell("backstab_cutpurse");
    let mut gaps = Vec::new();
    for seed in 0..10 {
        let realm =
            Realm::with_seed(seed).with_creature_spells(VILE_FAMILIAR, &["backstab_cutpurse"]);
        let (mut realm, _, mob) = warrior(realm, 30);
        stand_by(&mut realm, mob.1);
        let goes: Vec<f32> = run(&mut realm, 60.0)
            .into_iter()
            .filter(|(_, e)| {
                matches!(e, SessionEvent::SpellGo { caster, spell_id: s, .. }
                    if *caster == mob.0 && *s == spell_id)
            })
            .map(|(t, _)| t)
            .collect();
        gaps.extend(goes.windows(2).map(|w| w[1] - w[0]));
    }
    assert!(gaps.len() > 20, "{gaps:?}");
    // 5 to 9 s of cooldown, then up to a second before the next decision and a tick to land.
    assert!(gaps.iter().all(|g| (4.95..=10.25).contains(g)), "{gaps:?}");
    // Rolled at each cast: some gaps near the short end, some near the long one.
    assert!(gaps.iter().any(|g| *g < 6.5), "{gaps:?}");
    assert!(gaps.iter().any(|g| *g > 8.5), "{gaps:?}");
}

#[test]
fn a_creature_with_no_spells_never_casts() {
    let (mut realm, _, mob) = warrior(Realm::with_seed(1), 10);
    stand_by(&mut realm, mob.1);
    let events = run(&mut realm, 20.0);
    assert!(!events.iter().any(|(_, e)| matches!(
        e,
        SessionEvent::SpellStart { caster, .. } | SessionEvent::SpellGo { caster, .. } if *caster == mob.0
    )));
}

#[test]
fn a_caster_killed_mid_cast_lands_nothing() {
    let mut mid_cast = 0;
    for seed in 0..400 {
        let realm = Realm::with_seed(seed).with_creature_spells(VILE_FAMILIAR, &[CORRUPTION]);
        // A heavy sword, so the kill lands inside a cast; vary when the first swing lands so
        // some seed ends the cast rather than coming before it.
        let (mut realm, me, mob) = warrior_holding(realm, 60, RIVERPAW_BLADE);
        stand_by(&mut realm, mob.1);
        let lead = 0.1 * (seed % 20) as f32;
        let mut events = run(&mut realm, lead);
        realm.handle(RealmCommand::AttackSwing { guid: mob.0 });
        events.extend(run(&mut realm, 4.0));
        let started = events
            .iter()
            .any(|(_, e)| matches!(e, SessionEvent::SpellStart { caster, .. } if *caster == mob.0));
        let died = events.iter().position(|(_, e)| {
            matches!(e, SessionEvent::ObjectValues { guid, fields }
                if *guid == mob.0 && fields.unit_health() == Some(0))
        });
        let went = events.iter().position(
            |(_, e)| matches!(e, SessionEvent::SpellGo { caster, .. } if *caster == mob.0),
        );
        let (true, Some(died)) = (started, died) else {
            continue;
        };
        if went.is_some_and(|g| g < died) {
            continue;
        }
        mid_cast += 1;
        assert!(went.is_none(), "seed {seed}: a dead caster's cast went off");
        assert!(
            !events.iter().any(|(_, e)| matches!(e,
                SessionEvent::PeriodicAuraLog(l) if l.target == me)),
            "seed {seed}: the curse landed"
        );
    }
    assert!(mid_cast > 0, "some seed kills the familiar mid-cast");
}

// ── Helpers ───────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_gouge_mid_cast_cancels_it_and_the_spell_waits_out_its_cooldown() {
    let mut gouged = 0;
    for seed in 0..10 {
        let realm = Realm::with_seed(seed).with_creature_spells(VILE_FAMILIAR, &[CORRUPTION]);
        let (mut realm, me, mob) = warrior(realm, LEVEL);
        stand_by(&mut realm, mob.1);
        realm.handle(RealmCommand::AttackSwing { guid: mob.0 });
        let mut view = fight::View::default();
        let mut found = false;
        for _ in 0..600 {
            realm.tick(0.1, &FlatGround);
            let events = view.drain(&mut realm);
            if dead(&view, me) || dead(&view, mob.0) {
                break;
            }
            let starts = events
                .iter()
                .any(|e| matches!(e, SessionEvent::SpellStart { caster, .. } if *caster == mob.0));
            if starts && rage_of(&view, me) >= GOUGE_RAGE {
                found = true;
                break;
            }
        }
        if !found {
            continue;
        }
        gouged += 1;
        realm.handle(RealmCommand::AttackStop);
        realm.handle(fight::cast(*GOUGE, Some(mob.0)));
        let first = view.drain(&mut realm);
        assert_eq!(
            fight::refusal(&first, *GOUGE),
            None,
            "seed {seed}: gouge cast"
        );
        let mut events: Vec<(f32, SessionEvent)> = first.into_iter().map(|e| (0.0, e)).collect();
        for n in 1..=170 {
            realm.tick(0.1, &FlatGround);
            events.extend(drain(&mut realm).into_iter().map(|e| (n as f32 * 0.1, e)));
        }
        assert!(
            events.iter().any(
                |(_, e)| matches!(e, SessionEvent::SpellFailedOther { caster, spell_id }
                if *caster == mob.0 && *spell_id == *CORRUPTION_SPELL)
            ),
            "seed {seed}: the client sees the cast end"
        );
        assert!(
            !events.iter().any(
                |(_, e)| matches!(e, SessionEvent::SpellGo { caster, .. } if *caster == mob.0)
            ),
            "seed {seed}: the cancelled cast went off"
        );
        assert!(
            !events.iter().any(|(t, e)| *t < 15.0
                && matches!(e, SessionEvent::SpellStart { caster, .. } if *caster == mob.0)),
            "seed {seed}: it cast again inside its cooldown"
        );
    }
    assert!(gouged > 0, "some seed gouges the familiar mid-cast");
}

#[test]
fn an_incapacitated_creature_starts_no_cast() {
    for seed in 0..10 {
        let realm = Realm::with_seed(seed).with_creature_spells(VILE_FAMILIAR, &[CORRUPTION]);
        let (mut realm, me, mob) = warrior(realm, LEVEL);
        stand_by(&mut realm, mob.1);
        realm.handle(RealmCommand::AttackSwing { guid: mob.0 });
        let mut view = fight::View::default();
        // Swing until there is rage for Gouge and the familiar is between casts.
        let mut ready = false;
        for _ in 0..600 {
            realm.tick(0.1, &FlatGround);
            let events = view.drain(&mut realm);
            if dead(&view, me) || dead(&view, mob.0) {
                break;
            }
            let casting = events
                .iter()
                .any(|e| matches!(e, SessionEvent::SpellStart { caster, .. } if *caster == mob.0));
            if rage_of(&view, me) >= GOUGE_RAGE && !casting {
                ready = true;
                break;
            }
        }
        if !ready {
            continue;
        }
        realm.handle(RealmCommand::AttackStop);
        realm.handle(fight::cast(*GOUGE, Some(mob.0)));
        let mut events = view.drain(&mut realm);
        assert_eq!(
            fight::refusal(&events, *GOUGE),
            None,
            "seed {seed}: gouge cast"
        );
        // Gouge lasts 4 s; the familiar thinks once a second, and starts no cast in it.
        for _ in 0..38 {
            realm.tick(0.1, &FlatGround);
            events.extend(drain(&mut realm));
        }
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, SessionEvent::SpellStart { caster, .. } if *caster == mob.0)),
            "seed {seed}: an incapacitated familiar began a cast"
        );
        return;
    }
    panic!("no seed leaves the familiar between casts with rage in hand");
}

#[test]
fn a_hurt_creature_heals_fifteen_percent_of_its_max_health_after_the_cast_time() {
    let mut healed = 0;
    for seed in 0..10 {
        let Some((mut realm, _, mob, mut view)) = hurt_and_healing(seed, 0) else {
            continue;
        };
        let max = view.objects[&mob].unit_max_health().unwrap();
        let before = view.objects[&mob].unit_health().unwrap();
        let mut events = Vec::new();
        for n in 1..=34 {
            realm.tick(0.1, &FlatGround);
            events.extend(
                view.drain(&mut realm)
                    .into_iter()
                    .map(|e| (n as f32 * 0.1, e)),
            );
        }
        let go = events
            .iter()
            .find(|(_, e)| heal_go(e, mob))
            .map(|(t, _)| *t);
        let go = go.unwrap_or_else(|| panic!("seed {seed}: the heal goes off"));
        assert!(
            (go - 3.0).abs() < 0.25,
            "seed {seed}: after its 3 s cast: {go}"
        );
        let log = events.iter().find_map(|(_, e)| match e {
            SessionEvent::SpellHealLog(l) if l.target == mob => Some(*l),
            _ => None,
        });
        let log = log.unwrap_or_else(|| panic!("seed {seed}: the heal is logged"));
        assert_eq!((log.healer, log.spell_id), (mob, *HEAL_SPELL));
        assert_eq!(log.amount, max * 15 / 100, "seed {seed}");
        assert_eq!(
            view.objects[&mob].unit_health(),
            Some(before + max * 15 / 100),
            "seed {seed}: its health field rose"
        );
        assert!(before + log.amount <= max, "seed {seed}: no overheal");
        healed += 1;
    }
    assert!(healed > 0, "some seed leaves the hurt familiar healing");
}

#[test]
fn a_creature_at_full_health_does_not_heal() {
    let realm = Realm::with_seed(1).with_creature_spells(VILE_FAMILIAR, &[HEAL]);
    let (mut realm, _, mob) = warrior(realm, LEVEL);
    stand_by(&mut realm, mob.1);
    let events = run(&mut realm, 3.0);
    let started = events
        .iter()
        .any(|(_, e)| matches!(e, SessionEvent::SpellStart { caster, .. } if *caster == mob.0));
    assert!(!started, "a full-health familiar began a heal");
}

/// Whether `e` is `mob`'s heal going off on itself.
fn heal_go(e: &SessionEvent, mob: u64) -> bool {
    match e {
        SessionEvent::SpellGo {
            caster,
            spell_id,
            hits,
            ..
        } => *caster == mob && *spell_id == *HEAL_SPELL && hits == &[mob],
        _ => false,
    }
}

/// Whether `e` is `mob`'s heal ending short.
fn heal_failed(e: &SessionEvent, mob: u64) -> bool {
    match e {
        SessionEvent::SpellFailedOther { caster, spell_id } => {
            *caster == mob && *spell_id == *HEAL_SPELL
        }
        _ => false,
    }
}

#[test]
fn a_gouge_mid_heal_cancels_it() {
    let mut gouged = 0;
    for seed in 0..40 {
        let Some((mut realm, _, mob, mut view)) = hurt_and_healing(seed, GOUGE_RAGE) else {
            continue;
        };
        realm.handle(fight::cast(*GOUGE, Some(mob)));
        let mut events = view.drain(&mut realm);
        for _ in 0..34 {
            realm.tick(0.1, &FlatGround);
            events.extend(view.drain(&mut realm));
        }
        gouged += 1;
        let failed = events.iter().any(|e| heal_failed(e, mob));
        assert!(failed, "seed {seed}: the client sees the cast end");
        let landed = events
            .iter()
            .any(|e| matches!(e, SessionEvent::SpellHealLog(_)));
        assert!(!landed, "seed {seed}: the cancelled heal landed");
    }
    assert!(gouged > 0, "some seed gouges the familiar mid-heal");
}

/// A warrior fights a familiar carrying `abbot_heal` until the familiar, hurt under 80%, starts
/// its heal; then the warrior stops swinging. The realm, the warrior's and the familiar's guids,
/// and what the client has seen.
fn hurt_and_healing(seed: u64, rage: u32) -> Option<(Realm, u64, u64, fight::View)> {
    let realm = Realm::with_seed(seed).with_creature_spells(VILE_FAMILIAR, &[HEAL]);
    let (mut realm, me, mob, mut view) = warrior_seen(realm, LEVEL);
    stand_by(&mut realm, mob.1);
    realm.handle(RealmCommand::AttackSwing { guid: mob.0 });
    let mut casting = false;
    for _ in 0..600 {
        realm.tick(0.1, &FlatGround);
        let events = view.drain(&mut realm);
        if dead(&view, me) || dead(&view, mob.0) {
            return None;
        }
        for e in &events {
            match e {
                SessionEvent::SpellStart { caster, .. } if *caster == mob.0 => casting = true,
                SessionEvent::SpellGo { caster, .. } if *caster == mob.0 => casting = false,
                _ => {}
            }
        }
        if casting && rage_of(&view, me) >= rage {
            realm.handle(RealmCommand::AttackStop);
            return Some((realm, me, mob.0, view));
        }
    }
    None
}

/// Whether `guid` is known to be at 0 health; `warrior` drained the creates, so a unit not yet
/// updated is unhurt.
fn dead(view: &fight::View, guid: u64) -> bool {
    view.objects
        .get(&guid)
        .and_then(|f| f.unit_health())
        .is_some_and(|h| h == 0)
}

/// Whole rage points the view has seen on `guid`.
fn rage_of(view: &fight::View, guid: u64) -> u32 {
    view.objects
        .get(&guid)
        .and_then(|f| f.unit_power(1))
        .unwrap_or(0)
        / 10
}

/// A human warrior of `level` in `realm`, on flat ground, and the familiar's guid and position.
fn warrior(realm: Realm, level: u8) -> (Realm, u64, (u64, [f32; 3])) {
    let (realm, guid, mob, _) = warrior_seen(realm, level);
    (realm, guid, mob)
}

/// [`warrior`] holding `weapon` in place of the Worn Shortsword it starts with.
fn warrior_holding(realm: Realm, level: u8, weapon: u32) -> (Realm, u64, (u64, [f32; 3])) {
    let (realm, guid, mob, _) = warrior_seen_holding(realm, level, WORN_SHORTSWORD, weapon);
    (realm, guid, mob)
}

/// The sword a new warrior starts with.
const WORN_SHORTSWORD: u32 = 25;
/// Riverpaw Blade, a level-30 blue sword: a level 60 kills the familiar in a swing or two.
const RIVERPAW_BLADE: u32 = 905044;

/// [`warrior`], and the view of what entering the world created.
fn warrior_seen(realm: Realm, level: u8) -> (Realm, u64, (u64, [f32; 3]), fight::View) {
    warrior_seen_holding(realm, level, WORN_SHORTSWORD, WORN_SHORTSWORD)
}

/// [`warrior_seen`] with item `worn` swapped for `held`.
fn warrior_seen_holding(
    mut realm: Realm,
    level: u8,
    worn: u32,
    held: u32,
) -> (Realm, u64, (u64, [f32; 3]), fight::View) {
    let mut maker = common::realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace(&format!("entry = {worn}\n"), &format!("entry = {held}\n"));
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = fight::View::default();
    let created = view.drain(&mut realm);
    let mob = created
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } if fields.object_entry() == Some(VILE_FAMILIAR) => Some((guid, position)),
            _ => None,
        })
        .expect("the familiar is near");
    (realm, guid, mob, view)
}

/// The player 2 yd from `mob`, facing it: close enough that it notices and fights.
fn stand_by(realm: &mut Realm, mob: [f32; 3]) {
    let at = [mob[0] + 2.0, mob[1], mob[2]];
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: at,
        orientation: std::f32::consts::PI,
    });
}

/// Ticks `secs` in 0.1 s steps, each event with the time it came at.
fn run(realm: &mut Realm, secs: f32) -> Vec<(f32, SessionEvent)> {
    let mut events: Vec<_> = drain(realm).into_iter().map(|e| (0.0, e)).collect();
    let steps = (secs * 10.0).round() as u32;
    for n in 1..=steps {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(realm).into_iter().map(|e| (n as f32 * 0.1, e)));
    }
    events
}
