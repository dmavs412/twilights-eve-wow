//! A Calling's bar is its own `power` (callings.csv, ADR 0011), whatever vanilla class table its
//! stats follow: an energy bar on a warrior profile, and a focus bar, which no shipped Calling
//! runs on yet, that fills like a vanilla pet's. Played through the realm's seam on test Callings
//! (`Realm::with_callings`).

mod common;

use common::fight::*;
use common::*;

/// `Powers`.
const RAGE: u8 = 1;
const FOCUS: u8 = 2;
const ENERGY: u8 = 3;

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_energetic_warrior"
name = "Energetic Warrior"
tier = 0
power = "energy"
stat_profile = "warrior"
cap = 10
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "jab"
spell_id = 900001
power_cost = 30
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = []

[[calling]]
key = "test_focused"
name = "Focused"
tier = 0
power = "focus"
stat_profile = "hunter"
cap = 10
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["leather"]

[[calling.skill]]
key = "aim"
spell_id = 900002
power_cost = 40
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = []
"#;

/// A level 1 human of `calling` in the world, and the view.
fn entered(calling: &str) -> (Realm, u64, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker.export_character(guid).unwrap().replace(
        "calling = \"basic_warrior\"",
        &format!("calling = \"{calling}\""),
    );
    let mut realm = Realm::with_seed(0x5EED_0206).with_callings(TEST_CALLINGS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    (realm, guid, view)
}

#[test]
fn an_energy_calling_on_a_warrior_profile_gets_an_energy_bar() {
    let (_, me, view) = entered("test_energetic_warrior");
    let f = view.fields(me);
    assert_eq!(f.unit_power_type(), ENERGY);
    assert_eq!(f.unit_power(ENERGY), Some(100), "full at login");
    assert_eq!(f.unit_max_power(ENERGY), Some(100));
    assert_eq!(f.unit_power(RAGE).unwrap_or(0), 0, "no rage bar");
    assert_eq!(
        f.unit_bytes_0().map(|b| (b >> 8) & 0xff),
        Some(1),
        "the warrior profile's class byte"
    );
}

#[test]
fn a_focus_bar_starts_full_and_fills_ten_a_tick_after_a_spend() {
    let (mut realm, me, mut view) = entered("test_focused");
    let f = view.fields(me);
    assert_eq!(f.unit_power_type(), FOCUS);
    assert_eq!(
        (f.unit_power(FOCUS), f.unit_max_power(FOCUS)),
        (Some(100), Some(100))
    );
    realm.handle(cast(900002, None));
    view.drain(&mut realm);
    assert_eq!(view.fields(me).unit_power(FOCUS), Some(60), "40 spent");
    view.run(&mut realm, 2.1);
    assert_eq!(view.fields(me).unit_power(FOCUS), Some(70), "one tick: 10");
}
