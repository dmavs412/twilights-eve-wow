//! The Initiate's Ley Ward: an instant self cast that absorbs damage equal to 10% of the caster's
//! maximum mana, with a 30 s cooldown. Played through the realm's seam with a test Mage Calling.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

const MANA: u8 = 0;

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_mage"
name = "Test Mage"
tier = 0
class = 8
cap = 10
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "ley_ward"
spell_id = 3000
power_cost = 0
cooldown_ms = 30000
gcd_ms = 0
target = "self"
effects = [{ kind = "absorb", pct = 10, pct_of = "max_mana", duration_ms = 20000, radius_yd = 0.0 }]

[[calling.skill]]
key = "toll"
spell_id = 3001
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "periodic_damage", tick_ms = 100, duration_ms = 100, tick = { base = 100.0 }, school = "shadow" }]
"#;

const LEY_WARD: u32 = 3000;
const TOLL: u32 = 3001;
/// The damage one toll deals.
const HIT: u32 = 100;

/// A level 10 test Mage in a camp: the realm, their guid and the view.
fn world() -> (Realm, u64, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Vellune", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_mage\"");
    let mut realm = Realm::with_seed(5).with_callings(TEST_CALLINGS);
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
    stand_at(&mut realm, &mob, 20.0);
    view.drain(&mut realm);
    (realm, guid, view)
}

/// Casts `spell_id` at nothing, returning the events.
fn go(realm: &mut Realm, view: &mut View, spell_id: u32) -> Vec<SessionEvent> {
    realm.handle(cast(spell_id, None));
    view.drain(realm)
}

#[test]
fn ley_ward_absorbs_a_tenth_of_max_mana_then_drops_and_waits_out_its_cooldown() {
    let (mut realm, me, mut view) = world();
    // A tenth of the bar, rounded as the realm rounds a share.
    let ward = (view.fields(me).unit_max_power(MANA).unwrap() * 10 + 50) / 100;
    assert!(
        ward > 0 && ward < HIT,
        "a ward of {ward} against a hit of {HIT}"
    );

    let events = go(&mut realm, &mut view, LEY_WARD);
    assert_eq!(refusal(&events, LEY_WARD), None, "Ley Ward is cast");
    assert!(view.aura(me, LEY_WARD).is_some(), "the ward is up");

    // The first hit loses `ward` to it and takes the ward down.
    let before = view.health(me);
    go(&mut realm, &mut view, TOLL);
    view.run(&mut realm, 0.15);
    assert_eq!(view.health(me), before - (HIT - ward), "{ward} soaked");
    assert!(view.aura(me, LEY_WARD).is_none(), "the ward is spent");

    // The next hit lands whole.
    let before = view.health(me);
    go(&mut realm, &mut view, TOLL);
    view.run(&mut realm, 0.15);
    assert_eq!(view.health(me), before - HIT);

    // A second cast inside 30 s is refused.
    view.run(&mut realm, 10.0);
    let events = go(&mut realm, &mut view, LEY_WARD);
    assert!(refusal(&events, LEY_WARD).is_some(), "on cooldown");
}
