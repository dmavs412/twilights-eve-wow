//! Iron Juggernaut: a skill's strike adds a percent of the shield's Block Value. The engine half is
//! played through the realm's seam on a test Calling (`Realm::with_callings`) with a free strike
//! and the talent's shape, in the starting outfit's sword and shield; the content half reads the
//! Vanguard's rows.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, TalentEffect};
use twilight_realm::{Realm, RealmCommand};

const SEED: u64 = 0x5EED_0106;
const STRIKE: u32 = 1752;
const JUGGERNAUT: u32 = 99106;
const OFF_HAND: u8 = 16;

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_juggernaut"
name = "Test Juggernaut"
tier = 0
class = 1
cap = 30
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth", "leather", "mail", "plate", "shield"]

[[calling.skill]]
key = "strike"
spell_id = 1752
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 5.0]
effects = [{ kind = "weapon_strike", weapon = "normal", bonus = { base = 0.0 }, unavoidable = true }]

[[calling.talent_tab]]
id = 9993
name = "Test"
background = "WarriorArms"

[[calling.talent_tab.talent]]
key = "juggernaut"
talent_id = 99106
tier = 1
column = 1
rank_spells = [99161]
spell_name = "Test Juggernaut"
effects = [{ kind = "skill_block_value_pct", skill = "strike", per_rank = [300] }]
"#;

/// The first landed strike's damage at the camp's first mob, from the same seed.
fn first_strike(talented: bool, shield: bool) -> u32 {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Jugg", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 20\n")
        .replace(
            "calling = \"basic_warrior\"",
            "calling = \"test_juggernaut\"",
        );
    if talented {
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [[{JUGGERNAUT}, 1]]\n"),
        );
    }
    if !shield {
        blob = drop_slot(&blob, OFF_HAND);
    }
    let mut realm = Realm::with_seed(SEED).with_callings(TEST_CALLINGS);
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
                kind: benilla_protocol::EntityKind::Unit,
                position,
                ..
            } if flat(*position, CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("the scorpid camp streams in");
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(STRIKE, Some(mob.guid)));
    view.drain(&mut realm)
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.attacker == guid && l.spell_id == STRIKE => {
                Some(l.damage)
            }
            _ => None,
        })
        .expect("the strike lands")
}

#[test]
fn the_strike_adds_block_value_with_a_shield_only() {
    // A test rank of 300% makes the Block Value's share plain past the weapon roll's spread.
    assert!(first_strike(true, true) > first_strike(false, true));
    assert_eq!(first_strike(true, false), first_strike(false, false));
}

#[test]
fn the_vanguards_iron_juggernaut_feeds_both_strikes() {
    let t = callings::calling("vanguard")
        .unwrap()
        .talent_tabs
        .iter()
        .flat_map(|t| &t.talents)
        .find(|t| t.key == "iron_juggernaut")
        .unwrap();
    let skills: Vec<(&str, &[u32])> = t
        .effects
        .iter()
        .filter_map(|e| match e {
            TalentEffect::SkillBlockValuePct { skill, per_rank } => {
                Some((skill.as_str(), per_rank.as_slice()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        skills,
        [
            ("righteous_smash", &[50, 100, 150][..]),
            ("zealous_cleave", &[50, 100, 150][..])
        ]
    );
}
