//! Worn Spell Power adds to spell damage and healing by vanilla's cast-time coefficient
//! (spec #272, ticket #274): the shares as pure rules, and a 2 s bolt in the world.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::rules::spell_power::{periodic_spell_power_bonus, spell_power_bonus};
use twilight_realm::{Realm, RealmCommand};

#[test]
fn a_direct_spell_takes_its_cast_time_over_three_and_a_half_seconds() {
    // 2 s of 3.5 s: 10 x 2000 / 3500 = 5.71, rounded down.
    assert_eq!(spell_power_bonus(10, 2000), 5);
    // An instant counts as 1.5 s: 10 x 1500 / 3500 = 4.28.
    assert_eq!(spell_power_bonus(10, 0), 4);
    assert_eq!(spell_power_bonus(10, 1000), 4);
    // 3.5 s and longer take it all.
    assert_eq!(spell_power_bonus(10, 3500), 10);
    assert_eq!(spell_power_bonus(10, 6000), 10);
    assert_eq!(spell_power_bonus(0, 2000), 0);
}

#[test]
fn a_periodic_effect_takes_its_duration_over_fifteen_seconds_spread_over_its_ticks() {
    // 15 s takes it all: 10 over 5 ticks.
    assert_eq!(periodic_spell_power_bonus(10, 15000, 5), 2);
    // 12 s: 10 x 12000 / 15000 = 8, over 4 ticks.
    assert_eq!(periodic_spell_power_bonus(10, 12000, 4), 2);
    // Longer than 15 s is capped there: 10 over 10 ticks.
    assert_eq!(periodic_spell_power_bonus(10, 30000, 10), 1);
    // Rounded down at each step: 10 x 6000 / 15000 = 4, over 3 ticks.
    assert_eq!(periodic_spell_power_bonus(10, 6000, 3), 1);
    assert_eq!(periodic_spell_power_bonus(10, 15000, 0), 0);
}

/// A Mage-Class Calling whose `bolt` is a 2 s arcane hit of 20.
const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_sp_mage"
name = "Test Spell Power Mage"
tier = 0
class = 8
cap = 10
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "bolt"
spell_id = 2912
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
meta = { cast_time_ms = 2000 }
effects = [{ kind = "school_damage", school = "arcane", amount = { base = 20.0 } }]
"#;

/// A scratch staff with `spell_power = 10`, through the quests test seam's item table.
const STAFF: u32 = 999_001;
const STAFF_ITEMS: &str = r#"
[[item]]
entry = 999001
name = "Scratch Staff"
quality = 2
item_level = 10
required_level = 1
class = 2
subclass = 10
inventory_type = 17
display_id = 1542
speed_ms = 2800
damage = [[5.0, 9.0, 0]]
spell_power = 10
"#;

const BOLT: u32 = 2912;
const SLOT_MAIN_HAND: u8 = 15;

/// A level 10 test mage at `seed`, holding the scratch staff when `staff`: the realm, the
/// nearest mob, the view.
fn world(seed: u64, staff: bool) -> (Realm, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Weaver", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker.export_character(guid).unwrap();
    // The staff is the only item: drop the starting ones.
    if let Some(at) = blob.find("[[items]]") {
        blob.truncate(at);
    }
    let mut blob = blob
        .replace("level = 1\n", "level = 10\n")
        .replace("items = []\n", "")
        .replace("calling = \"basic_warrior\"", "calling = \"test_sp_mage\"");
    if staff {
        blob.push_str(&format!(
            "\n[[items]]\nslot = {SLOT_MAIN_HAND}\nentry = {STAFF}\ncount = 1\n"
        ));
    }
    let mut realm = Realm::with_seed(seed)
        .with_callings(TEST_CALLINGS)
        .with_quests(STAFF_ITEMS);
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
    let seen = view.run(&mut realm, 1.0);
    let mob = seen
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
        .expect("a mob in view");
    (realm, mob, view)
}

/// The first bolt that lands without a crit at `seed`, with or without the staff.
fn plain_bolt(seed: u64, staff: bool) -> Option<u32> {
    let (mut realm, mob, mut view) = world(seed, staff);
    stand_at(&mut realm, &mob, 10.0);
    view.drain(&mut realm);
    for _ in 0..3 {
        realm.handle(cast(BOLT, Some(mob.guid)));
        for e in view.run(&mut realm, 2.5) {
            if let SessionEvent::SpellDamageLog(l) = e {
                if l.spell_id == BOLT && l.hit_info == 0 {
                    return Some(l.damage);
                }
            }
        }
    }
    None
}

#[test]
fn a_staff_with_ten_spell_power_adds_five_to_a_two_second_bolt() {
    // 10 x 2000 / 3500 = 5, before the crit roll.
    let seed = 0x5B00_0001;
    let bare = plain_bolt(seed, false).expect("a plain bolt without the staff");
    let armed = plain_bolt(seed, true).expect("a plain bolt with the staff");
    assert_eq!(bare, 20, "the bolt's own 20");
    assert_eq!(armed, bare + 5);
}
