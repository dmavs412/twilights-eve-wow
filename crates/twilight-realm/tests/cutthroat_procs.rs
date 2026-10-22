//! The Cutthroat's procs: Phantom Guile on a dodge, Relentless Strikes and Ruthlessness on a
//! finisher, Executioner's Toll on an experience-yielding kill. Played through the realm's seam
//! by a Cutthroat with the talents in its save, at fixed seeds.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const SEED: u64 = 0x5EED_0110;
/// A seed where every Sinister Strike of `three_eviscerates` lands.
const EVISCERATE_SEED: u64 = 1;
const ENERGY: u8 = 3;
/// `VictimState` of a dodged swing.
const DODGED: u32 = 2;

static SINISTER_STRIKE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "sinister_strike"));
static EVISCERATE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "eviscerate"));
static EVASION: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "evasion"));

/// `Talent.dbc` ids of the talents under test, and Reflexive Strike for more dodges.
static REFLEXIVE_STRIKE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("cutthroat", "reflexive_strike"));
static PHANTOM_GUILE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("cutthroat", "phantom_guile"));
static EXECUTIONERS_TOLL: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("cutthroat", "executioners_toll"));
static RUTHLESSNESS: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("cutthroat", "ruthlessness"));
static RELENTLESS_STRIKES: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("cutthroat", "relentless_strikes"));

/// A Cutthroat of `level` with `talents` in its save, in the world: the realm, its guid, the mobs
/// in view (those at the scorpid camp south of the Crossroads when `camp`, else those at entry)
/// and what the client has seen.
fn cutthroat(
    level: u8,
    talents: &[(u32, u8)],
    camp: bool,
    unarmed: bool,
) -> (Realm, u64, Vec<Mob>, View) {
    cutthroat_seeded(SEED, level, talents, camp, unarmed)
}

/// [`cutthroat`] at `seed`.
fn cutthroat_seeded(
    seed: u64,
    level: u8,
    talents: &[(u32, u8)],
    camp: bool,
    unarmed: bool,
) -> (Realm, u64, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"cutthroat\"");
    if unarmed {
        // Fists, so that the fight lasts for as many dodges as the test wants.
        blob = drop_slot(&drop_slot(&blob, 15), 16);
    }
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
    }
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let mut seen = view.drain(&mut realm);
    if camp {
        view.run(&mut realm, 1.0);
        realm.handle(RealmCommand::Move {
            motion: Default::default(),
            position: [CAMP[0] + 40.0, CAMP[1], 0.0],
            orientation: std::f32::consts::PI,
        });
        seen = view.run(&mut realm, 1.0);
    }
    let mobs: Vec<Mob> = seen
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } if !camp || flat(*position, CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect();
    assert!(!mobs.is_empty(), "a mob in view");
    (realm, guid, mobs, view)
}

fn energy(view: &View, me: u64) -> u32 {
    view.fields(me).unit_power(ENERGY).unwrap()
}

/// The combo points the client shows, and the guid they are on.
fn combo(view: &View, me: u64) -> (u8, u64) {
    let f = view.fields(me);
    (
        f.player_combo_points().unwrap_or(0),
        f.player_combo_target(),
    )
}

/// How many swings at `me` in `events` were dodged.
fn dodges(events: &[SessionEvent], me: u64) -> u32 {
    events
        .iter()
        .filter(|e| {
            matches!(e, SessionEvent::AttackerState(s)
                if s.victim == me && s.victim_state == DODGED)
        })
        .count() as u32
}

/// Casts `spell_id` at `mob`, answered.
fn cast_at(realm: &mut Realm, view: &mut View, spell_id: u32, mob: &Mob) {
    realm.handle(cast(spell_id, Some(mob.guid)));
    let events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "spell {spell_id}");
}

/// Spends Energy with two Sinister Strikes at `mob` and stops the auto-attack they start, so a
/// refund shows under the bar's cap.
fn drain_energy(realm: &mut Realm, view: &mut View, mob: &Mob) {
    for _ in 0..2 {
        cast_at(realm, view, *SINISTER_STRIKE, mob);
        realm.handle(RealmCommand::AttackStop);
        view.run(realm, 1.5);
    }
}

#[test]
fn phantom_guile_gives_a_point_and_ten_energy_on_each_dodge() {
    // Evasion and Reflexive Strike make most of the mob's swings miss the player.
    let talents = [(*PHANTOM_GUILE, 3), (*REFLEXIVE_STRIKE, 5)];
    let (mut realm, me, camp, mut view) = cutthroat(10, &talents, true, true);
    let mob = &camp[0];
    // The rest of the camp turns on the player, for dodges enough.
    for other in camp[1..].iter().rev() {
        pull(&mut realm, &mut view, other);
    }
    stand_at(&mut realm, mob, 2.0);
    cast_at(&mut realm, &mut view, *EVASION, mob);
    // Two strikes at another mob spend the Energy, one at this mob puts a real point on it, and
    // the player's fists leave it alive for the dodges.
    stand_at(&mut realm, &camp[1], 2.0);
    drain_energy(&mut realm, &mut view, &camp[1]);
    stand_at(&mut realm, mob, 2.0);
    cast_at(&mut realm, &mut view, *SINISTER_STRIKE, mob);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    assert_eq!(combo(&view, me).1, mob.guid);
    let mut seen = 0;
    for _ in 0..600 {
        let (e0, (c0, _)) = (energy(&view, me), combo(&view, me));
        let events = view.run(&mut realm, 0.1);
        let d = dodges(&events, me);
        if d > 0 {
            let (e1, (c1, on)) = (energy(&view, me), combo(&view, me));
            assert_eq!((c1, on), ((c0 + d as u8).min(5), mob.guid), "combo points");
            // A regeneration tick may land in the same step.
            let gave = [e0 + 10 * d, e0 + 10 * d + 20].map(|e| e.min(100));
            assert!(gave.contains(&e1), "energy {e0} -> {e1} on {d} dodge(s)");
            seen += d;
        }
        if seen >= 3 {
            return;
        }
    }
    panic!("only {seen} dodges");
}

#[test]
fn a_cutthroat_without_the_talent_gets_nothing_from_a_dodge() {
    let (mut realm, me, camp, mut view) = cutthroat(10, &[(*REFLEXIVE_STRIKE, 5)], true, true);
    let mob = &camp[0];
    stand_at(&mut realm, mob, 2.0);
    cast_at(&mut realm, &mut view, *EVASION, mob);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut seen = 0;
    for _ in 0..300 {
        let events = view.run(&mut realm, 0.1);
        seen += dodges(&events, me);
        assert_eq!(combo(&view, me).0, 0, "a point from a dodge");
    }
    assert!(seen > 0, "no dodges to prove it");
}

/// Three Eviscerates, each on a fresh mob of the camp after a Sinister Strike builds its point:
/// the Energy before and after each, and the points left.
///
/// The seed is one where every Sinister Strike lands. Under the flat table (ADR 0012) a strike can
/// miss, and a missed one leaves the client view showing a combo point the realm never gave (bug
/// #305), so the Eviscerate is refused.
fn three_eviscerates(talents: &[(u32, u8)]) -> Vec<(u32, u32, (u8, u64))> {
    let (mut realm, me, camp, mut view) =
        cutthroat_seeded(EVISCERATE_SEED, 10, talents, true, false);
    let mut rounds = Vec::new();
    for mob in camp.iter().take(3) {
        stand_at(&mut realm, mob, 2.0);
        let mut built = false;
        for _ in 0..8 {
            while energy(&view, me) < 75 {
                view.run(&mut realm, 0.5);
            }
            cast_at(&mut realm, &mut view, *SINISTER_STRIKE, mob);
            realm.handle(RealmCommand::AttackStop);
            view.run(&mut realm, 1.5);
            if combo(&view, me).0 > 0 {
                built = true;
                break;
            }
        }
        assert!(built, "Sinister Strike never landed");
        let before = energy(&view, me);
        cast_at(&mut realm, &mut view, *EVISCERATE, mob);
        rounds.push((before, energy(&view, me), combo(&view, me)));
        realm.handle(RealmCommand::AttackStop);
        view.run(&mut realm, 1.5);
    }
    assert_eq!(rounds.len(), 3, "the camp has three mobs");
    rounds
}

#[test]
fn relentless_strikes_refunds_twenty_five_energy_on_every_eviscerate() {
    for (before, after, _) in three_eviscerates(&[(*RELENTLESS_STRIKES, 5)]) {
        assert_eq!(after, before - 35 + 25);
    }
    for (before, after, _) in three_eviscerates(&[]) {
        assert_eq!(after, before - 35, "without the talent");
    }
}

#[test]
fn ruthlessness_leaves_one_point_after_every_finisher() {
    let rounds = three_eviscerates(&[(*RUTHLESSNESS, 3)]);
    for (i, (_, _, (points, _))) in rounds.iter().enumerate() {
        assert_eq!(*points, 1, "round {i}");
    }
    for (_, _, (points, _)) in three_eviscerates(&[]) {
        assert_eq!(points, 0, "without the talent");
    }
}

/// A level `level` Cutthroat with `talents` that the camp has worn down by a fifth, killing the
/// first mob of the camp with Sinister Strikes (cast whenever the bar can pay) and swings: the
/// health and the Energy the killing step gave, the maximum health and the health it was missing.
fn kill_gains(level: u8, talents: &[(u32, u8)]) -> (u32, u32, u32, u32) {
    let (mut realm, me, camp, mut view) = cutthroat(level, talents, true, false);
    let max = view.fields(me).unit_max_health().unwrap();
    for other in camp.iter().rev() {
        pull(&mut realm, &mut view, other);
    }
    for _ in 0..600 {
        view.run(&mut realm, 0.1);
        assert!(view.health(me) > 0, "died waiting to be hurt");
        if max - view.health(me) >= max / 5 {
            break;
        }
    }
    let mob = &camp[0];
    stand_at(&mut realm, mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    for _ in 0..1200 {
        let (h0, e0) = (view.health(me), energy(&view, me));
        // A paid cast takes 40 off the step's Energy.
        let mut cost = 0;
        if e0 >= 40 {
            realm.handle(cast(*SINISTER_STRIKE, Some(mob.guid)));
            if refusal(&view.drain(&mut realm), *SINISTER_STRIKE).is_none() {
                cost = 40;
            }
        }
        view.run(&mut realm, 0.1);
        assert!(view.health(me) > 0, "died");
        if view.health(mob.guid) == 0 {
            let (h1, e1) = (view.health(me), energy(&view, me));
            return (
                h1.saturating_sub(h0),
                (e1 + cost).saturating_sub(e0),
                max,
                max - h0.min(max),
            );
        }
    }
    panic!("the mob outlived two minutes");
}

#[test]
fn executioners_toll_heals_and_refunds_energy_on_an_experience_kill() {
    let (health, power, max, missing) = kill_gains(5, &[(*EXECUTIONERS_TOLL, 3)]);
    let heal = max * 15 / 100;
    assert!(missing >= heal, "only {missing} of {max} was missing");
    assert!(health >= heal - 10, "healed {health} of {max}");
    // 60 Energy, less what a nearly full bar clips.
    assert!(power >= 40, "refunded {power}");
}
