//! A ground area: Miasma at a point gives every enemy inside its damage-over-time aura and every
//! friendly player inside its healing aura, and takes them off on leaving. Noxious Vapors puts a
//! hit-chance debuff on what Miasma ticks on, and Morbid Fascination refunds mana on damage to a
//! target inside. Played through the realm's seam with a test Priest Calling.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings::{self, Effect};
use twilight_realm::{Realm, RealmCommand};

const MANA: u8 = 0;

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_priest"
name = "Test Priest"
tier = 0
class = 5
cap = 10
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "miasma"
spell_id = 2120
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "dest"
range_yd = [0.0, 30.0]
effects = [{ kind = "ground_area", radius_yd = 8.0, duration_ms = 10000, enemy_aura_spell_id = 2120, ally_aura_spell_id = 2121 }]

[[calling.skill]]
key = "mend"
spell_id = 2061
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "heal", amount = { base = 40.0 } }]

[[calling.skill]]
key = "bleed"
spell_id = 8092
power_cost = 50
cooldown_ms = 0
gcd_ms = 0
target = "self"
meta = { cost_kind = "health" }
effects = []

[[calling.skill]]
key = "drain"
spell_id = 5
power_cost = 50
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = []

[[calling.aura]]
key = "miasma_enemy"
spell_id = 2120
duration_ms = 10000
effects = [{ kind = "periodic_damage", tick_ms = 1000, tick = { base = 1.0 }, school = "shadow" }]

[[calling.aura]]
key = "miasma_ally"
spell_id = 2121
duration_ms = 10000
effects = [{ kind = "mod_healing_taken_pct", pct = 15 }]

[[calling.aura]]
key = "vapors"
spell_id = 15317
duration_ms = 2000
effects = [{ kind = "mod_hit_chance", pct = -100 }]

[[calling.talent_tab]]
id = 910
name = "Test"
background = "PriestShadow"

[[calling.talent_tab.talent]]
key = "noxious_vapors"
talent_id = 482
tier = 1
column = 1
rank_spells = [15275, 15317]
spell_name = "Noxious Vapors"
effects = [{ kind = "proc", on = "periodic_tick", skill = "miasma", per_rank = [0, 0], target_auras = [["vapors"], ["vapors"]] }]

[[calling.talent_tab.talent]]
key = "morbid_fascination"
talent_id = 881
tier = 1
column = 2
rank_spells = [17322, 17323, 17325]
spell_name = "Morbid Fascination"
effects = [{ kind = "proc", on = "damage_done", requires_target_aura = "miasma_enemy", per_rank = [0, 0, 0], energize_pct_of_base = [4, 8, 12] }]
"#;

const MIASMA: u32 = 2120;
const MIASMA_ALLY: u32 = 2121;
const MEND: u32 = 2061;
const BLEED: u32 = 8092;
const DRAIN: u32 = 5;
const VAPORS: u32 = 15317;
/// Morbid Fascination's rank 3 spell: its refunds are logged under it.
const MORBID_3: u32 = 17325;

const NOXIOUS_VAPORS: u32 = 482;
const MORBID_FASCINATION: u32 = 881;

/// Where the test Priest stands: east of the scorpid camp, out of its notice.
const STAND: [f32; 3] = [CAMP[0] + 40.0, CAMP[1], 0.0];

/// A level 10 test Priest with `talents` east of the camp: the realm, their guid, the camp's
/// mobs and the view.
fn world(talents: &[(u32, u8)]) -> (Realm, u64, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Vellune", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_priest\"");
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
    }
    let mut realm = Realm::with_seed(4).with_callings(TEST_CALLINGS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: STAND,
        orientation: std::f32::consts::PI,
    });
    let mobs: Vec<Mob> = view
        .run(&mut realm, 1.0)
        .iter()
        .filter_map(|e| match e {
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
        .collect();
    assert!(mobs.len() >= 2, "the camp is in view");
    (realm, guid, mobs, view)
}

/// Casts Miasma at `dest`, asserting it went.
fn miasma(realm: &mut Realm, view: &mut View, dest: [f32; 3]) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::CastSpellAtDest {
        spell_id: MIASMA,
        dest,
    });
    let events = view.drain(realm);
    assert_eq!(refusal(&events, MIASMA), None, "Miasma is cast");
    events
}

/// Miasma's ticks on `mob` in `events`.
fn ticks_on(events: &[SessionEvent], mob: u64) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::PeriodicAuraLog(l) if l.spell_id == MIASMA && l.target == mob))
        .count()
}

#[test]
fn miasma_ticks_on_a_mob_inside_and_not_on_one_outside() {
    let (mut realm, _, mobs, mut view) = world(&[]);
    let inside = &mobs[0];
    let outside = mobs
        .iter()
        .find(|m| flat(m.position, inside.position) > 8.0)
        .expect("a camp mob more than 8 yd from the first");
    stand_at(&mut realm, inside, 25.0);
    view.drain(&mut realm);
    miasma(&mut realm, &mut view, inside.position);
    assert!(view.aura(inside.guid, MIASMA).is_some(), "inside: the aura");
    assert!(view.aura(outside.guid, MIASMA).is_none(), "outside: none");
    let first = view.run(&mut realm, 1.05);
    assert_eq!(ticks_on(&first, inside.guid), 1, "a tick a second");
    assert_eq!(ticks_on(&first, outside.guid), 0);
    // Struck, the mob runs at the caster, out of the circle, and the aura comes off.
    view.run(&mut realm, 5.0);
    assert!(view.aura(inside.guid, MIASMA).is_none(), "off once it left");
}

#[test]
fn miasma_raises_the_healing_its_caster_receives_by_15_percent_inside() {
    let (mut realm, me, _, mut view) = world(&[]);
    let mend = |realm: &mut Realm, view: &mut View| {
        realm.handle(cast(BLEED, None));
        view.drain(realm);
        realm.handle(cast(MEND, None));
        view.drain(realm)
            .iter()
            .find_map(|e| match e {
                SessionEvent::SpellHealLog(l) if l.spell_id == MEND => Some(l.amount),
                _ => None,
            })
            .expect("a heal")
    };
    assert_eq!(mend(&mut realm, &mut view), 40);
    miasma(&mut realm, &mut view, STAND);
    assert!(
        view.aura(me, MIASMA_ALLY).is_some(),
        "the caster stands in it"
    );
    assert_eq!(mend(&mut realm, &mut view), 46, "15% more");
}

#[test]
fn noxious_vapors_makes_the_mobs_inside_miss() {
    let (mut realm, me, mobs, mut view) = world(&[(NOXIOUS_VAPORS, 2)]);
    // Draw a mob in, then put Miasma under the caster's feet.
    pull(&mut realm, &mut view, &mobs[0]);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: STAND,
        orientation: std::f32::consts::PI,
    });
    view.run(&mut realm, 6.0);
    miasma(&mut realm, &mut view, STAND);
    view.run(&mut realm, 1.05);
    assert!(
        view.aura(mobs[0].guid, VAPORS).is_some(),
        "the first tick brings the vapors"
    );
    let health = view.health(me);
    let mut swings = 0;
    for e in view.run(&mut realm, 6.0) {
        if let SessionEvent::AttackerState(s) = e {
            if s.victim == me {
                swings += 1;
                assert_eq!(s.damage, 0, "every swing misses");
            }
        }
    }
    assert!(swings > 0, "the mob swung");
    assert!(view.health(me) >= health, "nothing landed");
}

#[test]
fn morbid_fascination_refunds_base_mana_on_each_damaging_tick() {
    let (mut realm, me, mobs, mut view) = world(&[(MORBID_FASCINATION, 3)]);
    for _ in 0..4 {
        realm.handle(cast(DRAIN, None));
    }
    view.drain(&mut realm);
    let inside = &mobs[0];
    stand_at(&mut realm, inside, 25.0);
    view.drain(&mut realm);
    miasma(&mut realm, &mut view, inside.position);
    let before = view.fields(me).unit_power(MANA).unwrap();
    let first = view.run(&mut realm, 1.05);
    assert_eq!(ticks_on(&first, inside.guid), 1);
    let refunds: Vec<u32> = first
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellEnergizeLog(l) if l.spell_id == MORBID_3 => Some(l.amount),
            _ => None,
        })
        .collect();
    assert_eq!(refunds.len(), 1, "one refund for the tick");
    assert!(refunds[0] > 0);
    assert_eq!(
        view.fields(me).unit_power(MANA).unwrap(),
        before + refunds[0]
    );
}

#[test]
fn the_shipped_miasma_is_a_ground_area_with_both_auras() {
    let acolyte = callings::calling("acolyte").unwrap();
    let miasma = acolyte.skills.iter().find(|s| s.key == "miasma").unwrap();
    let (enemy, ally) = (
        spell("acolyte", "aura_miasma"),
        spell("acolyte", "aura_miasma_ally"),
    );
    assert!(matches!(
        miasma.effects[..],
        [Effect::GroundArea {
            enemy_aura_spell_id: Some(e),
            ally_aura_spell_id: Some(a),
            ..
        }] if (e, a) == (enemy, ally)
    ));
    assert!(
        callings::aura("aura_miasma").is_some(),
        "the enemy aura loads"
    );
    assert!(
        callings::aura("aura_miasma_ally").is_some(),
        "the ally aura loads"
    );
}
