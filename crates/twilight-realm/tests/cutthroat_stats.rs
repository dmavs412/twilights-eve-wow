//! Evasion's dodge on the character sheet. Played through the realm's seam by a level 10
//! Cutthroat near the Crossroads.

mod common;

use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const SEED: u64 = 0x5EED_0107;
const LEVEL: u8 = 10;

static EVASION: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("cutthroat", "evasion"));

/// A level 10 Cutthroat in the world: the realm, its guid and what the client has seen.
fn cutthroat() -> (Realm, u64, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Garrote", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {LEVEL}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"cutthroat\"");
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
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
