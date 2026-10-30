//! The Cutthroat's Garrote: from Stealth, an 18 s bleed ticking every 3 s that awards a combo
//! point. Two choices of its row (#505): Brazen Garrote needs no Stealth, and Choking Garrote
//! also stuns for 3 s. Played through the realm's seam by a level 10 Cutthroat (24 for the
//! choices), fists only.

mod common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::{EntityKind, SessionEvent};
use common::cutthroat::{set_rows, ROWS_OPEN};
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, Effect};
use twilight_realm::{Realm, RealmCommand};

static STEALTH: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "stealth"));
static GARROTE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "garrote"));
static GARROTE_AURA: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "aura_garrote"));
static BRAZEN_GARROTE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("cutthroat", "garrote", "brazen_garrote"));
static CHOKING_GARROTE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("cutthroat", "garrote", "choking_garrote"));
/// `SPELL_FAILED_ONLY_STEALTHED`.
const ONLY_STEALTHED: u8 = 0x57;

/// Choking Garrote's stun aura, as the sheet names it.
static CHOKING_STUN: std::sync::LazyLock<u32> = std::sync::LazyLock::new(|| {
    let cutthroat = callings::calling("cutthroat").unwrap();
    let row = cutthroat.rows.iter().find(|r| r.key == "garrote").unwrap();
    let choking = row
        .choices
        .iter()
        .filter_map(|c| c.skill())
        .find(|s| s.spell_id == *CHOKING_GARROTE)
        .unwrap();
    choking
        .effects
        .iter()
        .find_map(|e| match e {
            Effect::Stun { aura_spell_id, .. } => Some(*aura_spell_id),
            _ => None,
        })
        .expect("Choking Garrote stuns")
});

/// A level 10 Cutthroat, fists only, with a camp mob beside it; its guid.
fn world() -> (Realm, View, u64, Mob) {
    world_with(10, &[])
}

/// [`world`] at `level`, each `(row, choice)` set.
fn world_with(level: u8, rows: &[(&str, u32)]) -> (Realm, View, u64, Mob) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"cutthroat\"");
    let blob = drop_slot(&drop_slot(&blob, 15), 16);
    let mut realm = Realm::with_seed(0x5EED_0366);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    set_rows(&mut realm, &mut view, rows);
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
        .expect("a camp mob streams in");
    (realm, view, me, mob)
}

#[test]
fn garrote_out_of_stealth_is_refused() {
    let (mut realm, mut view, _, mob) = world();
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(*GARROTE, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, *GARROTE), Some(ONLY_STEALTHED));
    assert!(view.aura(mob.guid, *GARROTE_AURA).is_none());
}

#[test]
fn garrote_from_stealth_bleeds_the_mob_and_awards_a_combo_point() {
    let (mut realm, mut view, me, mob) = world();
    realm.handle(cast(*STEALTH, None));
    view.drain(&mut realm);
    assert!(view.aura(me, *STEALTH).is_some(), "stealthed");
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(*GARROTE, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, *GARROTE), None, "garrote lands");
    assert!(
        view.aura(mob.guid, *GARROTE_AURA).is_some(),
        "the mob bleeds"
    );
    assert!(view.aura(me, *STEALTH).is_none(), "stealth is broken");
    let f = view.fields(me);
    assert_eq!(f.player_combo_points(), Some(1));
    assert_eq!(f.player_combo_target(), mob.guid);

    let events = view.run(&mut realm, 3.2);
    let ticks: Vec<u32> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == *GARROTE_AURA => Some(&l.ticks),
            _ => None,
        })
        .flatten()
        .filter_map(|t| match t {
            PeriodicTick::Damage { amount, .. } => Some(*amount),
            _ => None,
        })
        .collect();
    assert_eq!(ticks.len(), 1, "one tick at 3 s");
    // The tick grows with the Power Level (ADR 0012), 16 for a level 10 Tier 1 character:
    // 8 + 0.8 * 16 = 20.8, which a bleed takes unreduced by armour.
    assert_eq!(ticks[0], 21);
}

// ── Cutthroat's rows (#505) ─────────────────────────────────────────────────────────────────────

/// A level 24 Cutthroat with `rows` set pulls the camp mob into a fight, and casts `garrote` at
/// it unstealthed: the reason it was refused, if it was, and whether the mob bleeds.
fn garrote_in_combat(rows: &[(&str, u32)], garrote: u32) -> (Option<u8>, bool) {
    let (mut realm, mut view, _, mob) = world_with(ROWS_OPEN, rows);
    pull(&mut realm, &mut view, &mob);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(garrote, Some(mob.guid)));
    let events = view.drain(&mut realm);
    (
        refusal(&events, garrote),
        view.aura(mob.guid, *GARROTE_AURA).is_some(),
    )
}

#[test]
fn brazen_garrote_lands_out_of_stealth_in_combat_where_garrote_is_refused() {
    let rows = [("garrote", *BRAZEN_GARROTE)];
    assert_eq!(garrote_in_combat(&rows, *BRAZEN_GARROTE), (None, true));
    assert_eq!(
        garrote_in_combat(&[], *GARROTE),
        (Some(ONLY_STEALTHED), false)
    );
}

/// A level 24 Cutthroat with `rows` set casts `garrote` at the camp mob from Stealth: how many
/// seconds Choking Garrote's stun stayed on the mob.
fn stunned_by(rows: &[(&str, u32)], garrote: u32) -> f32 {
    let (mut realm, mut view, me, mob) = world_with(ROWS_OPEN, rows);
    realm.handle(cast(*STEALTH, None));
    view.drain(&mut realm);
    assert!(view.aura(me, *STEALTH).is_some(), "stealthed");
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(garrote, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, garrote), None, "the garrote lands");
    assert!(
        view.aura(mob.guid, *GARROTE_AURA).is_some(),
        "the mob bleeds"
    );
    let mut steps = 0;
    while view.aura(mob.guid, *CHOKING_STUN).is_some() && steps < 100 {
        view.run(&mut realm, 0.1);
        steps += 1;
    }
    steps as f32 / 10.0
}

#[test]
fn choking_garrote_stuns_the_target_for_three_seconds() {
    let secs = stunned_by(&[("garrote", *CHOKING_GARROTE)], *CHOKING_GARROTE);
    assert!((secs - 3.0).abs() <= 0.15, "stunned {secs} s");
    assert_eq!(stunned_by(&[], *GARROTE), 0.0, "Garrote stuns nothing");
}
