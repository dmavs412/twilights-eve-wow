//! Seal of Iron, a buff whose procs fire on the player's landed melee hits, and Wrathful Iron, a
//! talent proc that fires only while that buff is up. The engine half is played through the
//! realm's seam on a test Calling (`Realm::with_callings`) whose rows carry the same generic
//! effects as the Vanguard's, with a free strike to count hits by; the content half reads the
//! Vanguard's rows.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, AuraEffect, Effect, ProcEvent, TalentEffect};
use twilight_realm::{Realm, RealmCommand};

const SEED: u64 = 0x5EED_0105;
const LEVEL: u8 = 20;

/// The test skills: a free weapon strike, and the seal.
const STRIKE: u32 = 1752;
const SEAL: u32 = 20165;
/// The armour buff one of the seal's procs puts up lasts 10 s.
const ARMOR_MS: u32 = 10_000;
/// The test talent, Wrathful Iron's shape at 50%, and its one rank's spell.
const WRATHFUL: u32 = 99005;
const WRATHFUL_SPELL: u32 = 99051;
/// Landed hits to count over.
const HITS: usize = 100;

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_seal"
name = "Test Seal"
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
effects = [{ kind = "weapon_strike", weapon = "normal", bonus = { base = 0.0 } }]

[[calling.skill]]
key = "seal"
spell_id = 20165
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
range_yd = [0.0, 0.0]
effects = [{ kind = "grant_aura", aura_spell_id = 20165 }]

[[calling.aura]]
key = "test_seal_of_iron"
spell_id = 20165
duration_ms = 3600000
effects = [
    { kind = "proc", on = "melee_hit", chance = 10, heal_pct = 2 },
    { kind = "proc", on = "melee_hit", chance = 10, apply_aura = "test_seal_armor" },
]

[[calling.aura]]
key = "test_seal_armor"
spell_id = 10291
duration_ms = 10000
effects = [{ kind = "armor_items_pct", pct = 20 }]

[[calling.talent_tab]]
id = 9991
name = "Test"
background = "WarriorArms"

[[calling.talent_tab.talent]]
key = "wrathful"
talent_id = 99005
tier = 1
column = 1
rank_spells = [99051]
spell_name = "Test Wrathful"
effects = [{ kind = "proc", on = "melee_hit", while_aura = "test_seal_of_iron", chance = 50, per_rank = [0], weapon_pct = 25, school = "holy" }]
"#;

/// A level 20 human of the test Calling, with `talents` learned, by the scorpid camp south of the
/// Crossroads: the camp's mobs and the view.
fn at_the_camp(talents: &[(u32, u8)]) -> (Realm, u64, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Iron", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {LEVEL}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"test_seal\"");
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
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
    let camp = camp_in(&view.run(&mut realm, 1.0));
    assert!(!camp.is_empty(), "the scorpid camp streams in");
    (realm, guid, camp, view)
}

/// The scorpid camp's mobs that `events` stream in.
fn camp_in(events: &[SessionEvent]) -> Vec<Mob> {
    events
        .iter()
        .filter_map(|e| match e {
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
        .collect()
}

/// Strikes the camp's mobs one after another until `HITS` strikes have landed, walking off for
/// the camp to respawn when it is down: every event.
fn strike_camp(realm: &mut Realm, view: &mut View, me: u64, camp: &[Mob]) -> Vec<SessionEvent> {
    let mut seen = Vec::new();
    let mut landed = 0;
    let mut camp = camp.to_vec();
    for _ in 0..40 {
        for mob in &camp {
            stand_at(realm, mob, 2.0);
            for _ in 0..200 {
                realm.handle(cast(STRIKE, Some(mob.guid)));
                let events = view.run(realm, 0.1);
                landed += events
                    .iter()
                    .filter(|e| {
                        matches!(e, SessionEvent::SpellDamageLog(l)
                        if l.attacker == me && l.spell_id == STRIKE && l.damage > 0)
                    })
                    .count();
                let down = refusal(&events, STRIKE).is_some();
                seen.extend(events);
                if landed >= HITS {
                    return seen;
                }
                if down {
                    break;
                }
            }
        }
        // Away past the respawn distance for the respawn time, and back.
        realm.handle(RealmCommand::Move {
            motion: Default::default(),
            position: [CAMP[0] + 100.0, CAMP[1], 0.0],
            orientation: std::f32::consts::PI,
        });
        seen.extend(view.run(realm, 65.0));
        realm.handle(RealmCommand::Move {
            motion: Default::default(),
            position: [CAMP[0] + 40.0, CAMP[1], 0.0],
            orientation: std::f32::consts::PI,
        });
        camp = camp_in(&view.run(realm, 1.0));
    }
    panic!("the camp ran out after {landed} landed strikes");
}

fn heals(events: &[SessionEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::SpellHealLog(l) if l.spell_id == SEAL))
        .count()
}

/// The armour buff's applications: the only aura the test puts up for 10 s.
fn armor_buffs(events: &[SessionEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::AuraDuration { remaining_ms, .. } if *remaining_ms == ARMOR_MS))
        .count()
}

fn sears(events: &[SessionEvent], me: u64) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.attacker == me && l.spell_id == WRATHFUL_SPELL => {
                Some(l.damage)
            }
            _ => None,
        })
        .collect()
}

fn seal_up(realm: &mut Realm, view: &mut View) {
    realm.handle(cast(SEAL, None));
    assert_eq!(refusal(&view.drain(realm), SEAL), None, "the seal");
}

#[test]
fn seal_of_iron_heals_and_hardens_at_about_ten_percent_a_hit_each() {
    let (mut realm, me, camp, mut view) = at_the_camp(&[]);
    seal_up(&mut realm, &mut view);
    let seen = strike_camp(&mut realm, &mut view, me, &camp);
    // 10% each over 100 hits: 10 expected, wide bounds for one seed.
    let (h, a) = (heals(&seen), armor_buffs(&seen));
    assert!((3..=20).contains(&h), "{h} heals");
    assert!((3..=20).contains(&a), "{a} armour buffs");
    assert!(view.aura(me, SEAL).is_some(), "the seal is still up");
}

#[test]
fn without_the_seal_there_are_no_procs() {
    let (mut realm, me, camp, mut view) = at_the_camp(&[(WRATHFUL, 1)]);
    let seen = strike_camp(&mut realm, &mut view, me, &camp);
    assert_eq!(heals(&seen), 0);
    assert_eq!(armor_buffs(&seen), 0);
    assert!(sears(&seen, me).is_empty(), "Wrathful Iron needs the seal");
}

#[test]
fn wrathful_iron_sears_only_while_the_seal_is_up() {
    let (mut realm, me, camp, mut view) = at_the_camp(&[(WRATHFUL, 1)]);
    seal_up(&mut realm, &mut view);
    let seen = strike_camp(&mut realm, &mut view, me, &camp);
    let burns = sears(&seen, me);
    // 50% over 100 hits.
    assert!((30..=70).contains(&burns.len()), "{} sears", burns.len());
    assert!(burns.iter().all(|&d| d > 0), "{burns:?}");
    // Holy: every sear is the same 25% of the average swing.
    assert!(burns.iter().all(|&d| d == burns[0]), "{burns:?}");
}

#[test]
fn the_vanguards_rows_carry_the_seal_and_wrathful_iron() {
    let v = callings::calling("vanguard").unwrap();
    let seal = v
        .skills
        .iter()
        .find(|s| s.key == "seal_of_iron")
        .expect("Seal of Iron is on the bar");
    let aura = v
        .auras
        .iter()
        .find(|a| a.key == "aura_seal_of_iron")
        .unwrap();
    assert_eq!(
        seal.effects,
        [Effect::GrantAura {
            aura_spell_id: aura.spell_id
        }]
    );
    let procs: Vec<_> = aura
        .effects
        .iter()
        .filter_map(|e| match e {
            AuraEffect::Proc(p) => Some(p),
            _ => None,
        })
        .collect();
    assert_eq!(procs.len(), 2);
    assert!(procs
        .iter()
        .all(|p| p.on == ProcEvent::MeleeHit && p.chance == 25));
    assert_eq!(procs[0].heal_pct, 4);
    assert_eq!(
        procs[1].apply_aura.as_deref(),
        Some("aura_seal_of_iron_armor")
    );
    let wrathful = v
        .talent_tabs
        .iter()
        .flat_map(|t| &t.talents)
        .find(|t| t.key == "wrathful_iron")
        .unwrap();
    assert!(matches!(
        &wrathful.effects[..],
        [TalentEffect::Proc { on: ProcEvent::MeleeHit, while_aura: Some(a), weapon_pct: 25, chances, .. }]
            if a == "aura_seal_of_iron" && chances == &[10, 20, 30, 40, 50]
    ));
}

/// The seal's two procs are one buff on the bar, not two.
#[test]
fn the_seal_is_one_buff() {
    let (mut realm, me, _camp, mut view) = at_the_camp(&[]);
    seal_up(&mut realm, &mut view);
    view.run(&mut realm, 0.5);
    let slots = view
        .fields(me)
        .unit_auras()
        .filter(|a| a.spell_id == SEAL)
        .count();
    assert_eq!(slots, 1);
}
