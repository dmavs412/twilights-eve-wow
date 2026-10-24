//! The Cutthroat's Garrote: from Stealth, an 18 s bleed ticking every 3 s that awards a combo
//! point. Played through the realm's seam by a level 10 Cutthroat, fists only.

mod common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

static STEALTH: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "stealth"));
static GARROTE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "garrote"));
static GARROTE_AURA: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "aura_garrote"));
/// `SPELL_FAILED_ONLY_STEALTHED`.
const ONLY_STEALTHED: u8 = 0x57;

/// A level 10 Cutthroat, fists only, with a camp mob beside it; its guid.
fn world() -> (Realm, View, u64, Mob) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"cutthroat\"");
    let blob = drop_slot(&drop_slot(&blob, 15), 16);
    let mut realm = Realm::with_seed(0x5EED_0366);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
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
