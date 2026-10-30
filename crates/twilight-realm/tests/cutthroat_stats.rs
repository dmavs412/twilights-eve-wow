//! Evasion's dodge on the character sheet, and Second Wind (#505), the choice of its row that
//! also heals. Played through the realm's seam by a level 10 Cutthroat (24 for the choice) near
//! the Crossroads.

mod common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::SessionEvent;
use common::cutthroat::{set_rows, ROWS_OPEN};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const SEED: u64 = 0x5EED_0107;
const LEVEL: u8 = 10;

static EVASION: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "evasion"));
static SECOND_WIND: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("cutthroat", "evasion", "second_wind"));

/// A level 10 Cutthroat in the world: the realm, its guid and what the client has seen.
fn cutthroat() -> (Realm, u64, View) {
    cutthroat_with(LEVEL, &[])
}

/// [`cutthroat`] at `level`, each `(row, choice)` set.
fn cutthroat_with(level: u8, rows: &[(&str, u32)]) -> (Realm, u64, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"cutthroat\"");
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    set_rows(&mut realm, &mut view, rows);
    (realm, guid, view)
}

fn dodge(view: &View, me: u64) -> f32 {
    view.fields(me).player_dodge_percentage().unwrap()
}

#[test]
fn evasion_adds_fifty_percent_dodge_for_fifteen_seconds() {
    let (mut realm, me, mut view) = cutthroat();
    let base = dodge(&view, me);
    realm.handle(cast(*EVASION, None));
    assert_eq!(refusal(&view.drain(&mut realm), *EVASION), None, "the cast");
    assert!((dodge(&view, me) - base - 50.0).abs() < 1e-3, "on");
    view.run(&mut realm, 10.0);
    assert!(
        (dodge(&view, me) - base - 50.0).abs() < 1e-3,
        "still on at 10 s"
    );
    view.run(&mut realm, 6.0);
    assert!((dodge(&view, me) - base).abs() < 1e-3, "off after 16 s");
}

/// The health each heal tick of `spell_id` in `events` gave.
fn heal_ticks(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == spell_id => Some(&l.ticks),
            _ => None,
        })
        .flatten()
        .filter_map(|t| match t {
            PeriodicTick::Heal { amount } => Some(*amount),
            _ => None,
        })
        .collect()
}

#[test]
fn second_wind_heals_thirty_percent_of_max_health_over_its_fifteen_seconds_and_raises_dodge() {
    let (mut realm, me, mut view) = cutthroat_with(ROWS_OPEN, &[("evasion", *SECOND_WIND)]);
    let max = view.fields(me).unit_max_health().unwrap();
    let base = dodge(&view, me);
    realm.handle(cast(*SECOND_WIND, None));
    let mut events = view.drain(&mut realm);
    assert_eq!(refusal(&events, *SECOND_WIND), None, "the cast");
    assert!(
        (dodge(&view, me) - base - 50.0).abs() < 1e-3,
        "Evasion's dodge"
    );
    events.extend(view.run(&mut realm, 16.0));
    assert!((dodge(&view, me) - base).abs() < 1e-3, "off after 16 s");
    let ticks = heal_ticks(&events, *SECOND_WIND);
    assert_eq!(ticks.len(), 5, "a tick every 3 s: {ticks:?}");
    let total: u32 = ticks.iter().sum();
    let want = (f64::from(max) * 0.3).round() as u32;
    assert!(
        total.abs_diff(want) <= 2,
        "healed {total} of {max}: {ticks:?}"
    );

    let (mut realm, _, mut view) = cutthroat_with(ROWS_OPEN, &[]);
    realm.handle(cast(*EVASION, None));
    let mut events = view.drain(&mut realm);
    events.extend(view.run(&mut realm, 16.0));
    assert!(heal_ticks(&events, *EVASION).is_empty(), "Evasion heals");
}
