//! Hammer of the Vanguard, a talent that teaches a skill: 150% of a weapon roll as Holy damage
//! from 20 yd, always a critical strike on a target standing in the caster's Consecration. The
//! spell book is the Vanguard's own, a level 12 one with 15 points in The Righteous; the strike is
//! played through the realm's seam on a test Calling (`Realm::with_callings`) whose active talent
//! and charge carry the same generic effects as the Vanguard's at no cost, so no rage is needed.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, CritIf, Effect};
use twilight_realm::rules::school::School;
use twilight_realm::{Realm, RealmCommand};

const SEED: u64 = 0x5EED_0104;
/// The save holds 16 points at level 12; the import does not check them against the level.
const LEVEL: u8 = 12;
/// The client's school index for Holy.
const HOLY: u8 = 1;

/// The test Calling's charge (Bulwark Charge's spell) and the Hammer's spell and talent.
const CHARGE: u32 = 100;
const HAMMER: u32 = 12809;
const HAMMER_TALENT: u32 = 99104;

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_hammer"
name = "Test Hammer"
tier = 0
class = 1
cap = 30
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth", "leather", "mail", "plate", "shield"]

[[calling.skill]]
key = "charge"
spell_id = 100
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [8.0, 25.0]
effects = [
    { kind = "charge", speed_yd_s = 24.0 },
    { kind = "ground_effect", radius_yd = 8.0, tick_ms = 1000, duration_ms = 8000, school = "holy", amount = { base = 4.0, per_level = 0.6 } },
]

[[calling.talent_tab]]
id = 9992
name = "Test"
background = "WarriorArms"

[[calling.talent_tab.talent]]
key = "hammer"
talent_id = 99104
tier = 1
column = 1
rank_spells = [12809]
spell_name = "Test Hammer"
effects = []

[calling.talent_tab.talent.skill]
key = "hammer"
spell_id = 12809
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 20.0]
effects = [{ kind = "weapon_strike", weapon = "normal", bonus = { base = 0.0 }, weapon_pct = 150, school = "holy", crit_if_target_in = "ground_effect" }]
"#;

fn vanguard() -> &'static callings::CallingDef {
    callings::calling("vanguard").expect("the Vanguard is a shipped Calling")
}

fn talent(key: &str) -> &'static callings::TalentDef {
    vanguard()
        .talent_tabs
        .iter()
        .flat_map(|t| &t.talents)
        .find(|t| t.key == key)
        .unwrap_or_else(|| panic!("the Vanguard has {key}"))
}

/// 15 points in The Righteous, then the Hammer on row 4.
fn righteous(with_hammer: bool) -> Vec<(u32, u8)> {
    let mut picks = vec![
        (talent("righteous_fury").talent_id, 5),
        (talent("wrathful_iron").talent_id, 5),
        (talent("hallowed_ground").talent_id, 3),
        (talent("zealous_restoration").talent_id, 2),
    ];
    if with_hammer {
        picks.push((talent("hammer_of_vanguard").talent_id, 1));
    }
    picks
}

/// A save of `calling` at `level` with `talents`.
fn save(calling: &str, level: u8, talents: &[(u32, u8)]) -> (u64, String) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Hammer", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace(
            "calling = \"basic_warrior\"",
            &format!("calling = \"{calling}\""),
        )
        .replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
    (guid, blob)
}

/// What a Vanguard with `talents` gets on entering the world.
fn vanguard_entry(talents: &[(u32, u8)]) -> Vec<SessionEvent> {
    let (guid, blob) = save("vanguard", LEVEL, talents);
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm)
}

fn spellbook(events: &[SessionEvent]) -> Vec<u32> {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellBook { spell_ids, .. } => Some(spell_ids.clone()),
            _ => None,
        })
        .expect("the entry sends the spell book")
}

/// A level 20 human of the test Calling with the Hammer learned, from `seed`, by the scorpid
/// camp south of the Crossroads: the camp's first mob and the view.
fn at_the_camp(seed: u64) -> (Realm, u64, Mob, View) {
    let (guid, blob) = save("test_hammer", 20, &[(HAMMER_TALENT, 1)]);
    let mut realm = Realm::with_seed(seed).with_callings(TEST_CALLINGS);
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
        .expect("the scorpid camp streams in");
    (realm, guid, mob, view)
}

/// The Hammer's hits in `events`: (damage, crit, school).
fn hammer_hits(events: &[SessionEvent], me: u64) -> Vec<(u32, bool, u8)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.attacker == me && l.spell_id == HAMMER => {
                Some((l.damage, l.hit_info != 0, l.school))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_hammer_is_in_the_vanguards_spell_book_only_once_learned() {
    let hammer = talent("hammer_of_vanguard").rank_spells[0];
    assert!(spellbook(&vanguard_entry(&righteous(true))).contains(&hammer));
    assert!(!spellbook(&vanguard_entry(&righteous(false))).contains(&hammer));
}

#[test]
fn the_vanguards_hammer_is_a_twenty_yard_holy_strike_at_half_again_a_weapon_roll() {
    let t = talent("hammer_of_vanguard");
    let skill = t.skill.as_ref().expect("the talent teaches a skill");
    assert_eq!(skill.spell_id, t.rank_spells[0]);
    assert_eq!(
        (skill.power_cost, skill.cooldown_ms, skill.gcd_ms),
        (30, 6000, 0)
    );
    assert_eq!(skill.range_yd, [0.0, 20.0]);
    assert!(matches!(
        skill.effects[..],
        [Effect::WeaponStrike {
            weapon_pct: 150,
            school: School::Holy,
            crit_if_target_in: Some(CritIf::GroundEffect),
            ..
        }]
    ));
}

#[test]
fn the_hammer_strikes_from_eighteen_yards_for_half_again_a_weapon_roll_in_holy() {
    let (mut realm, me, mob, mut view) = at_the_camp(SEED);
    let mut hits = Vec::new();
    for _ in 0..10 {
        stand_at(&mut realm, &mob, 18.0);
        realm.handle(cast(HAMMER, Some(mob.guid)));
        let events = view.drain(&mut realm);
        if !hits.is_empty() && refusal(&events, HAMMER).is_some() {
            // The mob died to the hammers.
            break;
        }
        assert_eq!(refusal(&events, HAMMER), None, "the cast at 18 yd");
        hits.extend(hammer_hits(&events, me));
        view.run(&mut realm, 0.1);
    }
    let sheet = view.fields(me);
    let (min, max) = (
        sheet.unit_min_damage().unwrap(),
        sheet.unit_max_damage().unwrap(),
    );
    let plain: Vec<u32> = hits.iter().filter(|h| !h.1).map(|h| h.0).collect();
    assert!(!plain.is_empty(), "{hits:?}");
    assert!(hits.iter().all(|h| h.2 == HOLY), "Holy: {hits:?}");
    // Holy ignores armour, so a plain hit is the weapon roll's 150% and no less.
    for d in plain {
        let d = d as f32;
        assert!(
            d >= (1.5 * min).floor() - 1.0 && d <= (1.5 * max).ceil() + 1.0,
            "{d} outside 1.5 x {min}..{max}"
        );
    }
}

#[test]
fn inside_the_casters_consecration_the_hammer_always_crits() {
    for seed in 0..20 {
        let (mut realm, me, mob, mut view) = at_the_camp(SEED + seed);
        stand_at(&mut realm, &mob, 12.0);
        realm.handle(cast(CHARGE, Some(mob.guid)));
        assert_eq!(
            refusal(&view.drain(&mut realm), CHARGE),
            None,
            "seed {seed}: the charge"
        );
        view.run(&mut realm, 1.0);
        realm.handle(cast(HAMMER, Some(mob.guid)));
        let events = view.drain(&mut realm);
        assert_eq!(refusal(&events, HAMMER), None, "seed {seed}: the cast");
        let hits = hammer_hits(&events, me);
        assert!(
            matches!(hits[..], [(_, true, HOLY)]),
            "seed {seed}: {hits:?}"
        );
    }
}
