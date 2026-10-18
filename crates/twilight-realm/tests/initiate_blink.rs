//! Blink: a forward teleport that sheds stuns, and Shattered Blink's ring of frost that roots
//! the enemies by the starting point. Played through the realm's seam with a test Mage Calling.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

/// `blink` 20 yd, `stun_self` a 10 s stun on the caster, `bolt` a hit of 1 to pull with, and
/// Shattered Blink (2 s, then 4 s roots within 8 yd of the start) on the talent tab.
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
key = "blink"
spell_id = 1953
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "blink", distance_yd = 20.0 }]

[[calling.skill]]
key = "stun_self"
spell_id = 5211
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "stun", aura_spell_id = 5211, duration_ms = 10000 }]

[[calling.skill]]
key = "bolt"
spell_id = 2912
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 100.0]
effects = [{ kind = "school_damage", school = "arcane", amount = { base = 1.0 } }]

[[calling.aura]]
key = "ring_1"
spell_id = 122
duration_ms = 2000
effects = [{ kind = "root" }]

[[calling.aura]]
key = "ring_2"
spell_id = 122
duration_ms = 4000
effects = [{ kind = "root" }]

[[calling.talent_tab]]
id = 906
name = "Test"
background = "MageArcane"

[[calling.talent_tab.talent]]
key = "shattered_blink"
talent_id = 62
tier = 1
column = 1
rank_spells = [11165, 12475]
spell_name = "Shattered Blink"
effects = [{ kind = "proc", on = "cast", skill = "blink", per_rank = [0, 0], area_auras = [["ring_1"], ["ring_2"]], radius_yd = 8.0 }]
"#;

const BLINK: u32 = 1953;
const STUN: u32 = 5211;
const BOLT: u32 = 2912;
const RING: u32 = 122;
const SHATTERED_BLINK: u32 = 62;

/// Where the test Mage stands: east of the scorpid camp, facing it.
const STAND: [f32; 3] = [CAMP[0] + 40.0, CAMP[1], 0.0];

/// A level 10 test Mage with `talents` east of the camp: the realm, their guid, the camp's
/// mobs and the view.
fn world(seed: u64, talents: &[(u32, u8)]) -> (Realm, u64, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Weaver", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_mage\"");
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
    }
    let mut realm = Realm::with_seed(seed).with_callings(TEST_CALLINGS);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    view.run(&mut realm, 1.0);
    realm.handle(RealmCommand::Move {
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

/// Casts `spell_id`, asserting it went.
fn go(realm: &mut Realm, view: &mut View, spell_id: u32, target: Option<u64>) {
    realm.handle(cast(spell_id, target));
    assert_eq!(
        refusal(&view.drain(realm), spell_id),
        None,
        "spell {spell_id} is cast"
    );
}

/// Where the player was teleported to in `events`.
fn teleported(events: &[SessionEvent]) -> Option<[f32; 3]> {
    events.iter().find_map(|e| match e {
        SessionEvent::Teleport { position, .. } => Some(*position),
        _ => None,
    })
}

/// Whether `mob` set off along a path in `events`.
fn moved(events: &[SessionEvent], mob: u64) -> bool {
    events.iter().any(|e| {
        matches!(e, SessionEvent::MonsterMove { guid, duration_ms, .. } if *guid == mob && *duration_ms > 0)
    })
}

#[test]
fn blink_moves_the_caster_20_yards_forward_and_clears_a_stun() {
    let (mut realm, me, _, mut view) = world(1, &[]);
    go(&mut realm, &mut view, STUN, None);
    assert!(view.aura(me, STUN).is_some(), "stunned");
    go(&mut realm, &mut view, BLINK, None);
    assert!(view.aura(me, STUN).is_none(), "the stun is shed");
    let to = teleported(&view.run(&mut realm, 0.1)).expect("a teleport");
    let ahead = [STAND[0] - 20.0, STAND[1], 0.0];
    assert!(flat(to, ahead) < 0.01, "20 yd along the facing: {to:?}");
}

#[test]
fn shattered_blink_roots_a_mob_by_the_start_and_not_one_further_off() {
    let (mut realm, _, mobs, mut view) = world(2, &[(SHATTERED_BLINK, 2)]);
    let (near, far) = (&mobs[0], &mobs[1]);
    go(&mut realm, &mut view, BOLT, Some(near.guid));
    let pulled = view.run(&mut realm, 8.0);
    let near_at = pulled
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::MonsterMove { guid, path, .. } if *guid == near.guid => path.last(),
            _ => None,
        })
        .copied()
        .expect("the near mob came");
    assert!(flat(near_at, STAND) <= 8.0, "beside the caster");
    go(&mut realm, &mut view, BOLT, Some(far.guid));
    assert!(
        flat(far.position, STAND) > 8.0,
        "the far mob is outside the ring"
    );
    // Face away from the camp and blink.
    realm.handle(RealmCommand::Move {
        position: STAND,
        orientation: 0.0,
    });
    go(&mut realm, &mut view, BLINK, None);
    assert!(
        view.aura(near.guid, RING).is_some(),
        "the near mob is rooted"
    );
    assert!(view.aura(far.guid, RING).is_none(), "the far mob is not");
    let rooted = view.run(&mut realm, 3.9);
    assert!(!moved(&rooted, near.guid), "the near mob stands for 4 s");
    assert!(moved(&rooted, far.guid), "the far mob gives chase");
    let freed = view.run(&mut realm, 2.0);
    assert!(view.aura(near.guid, RING).is_none(), "the root fades");
    assert!(moved(&freed, near.guid), "the near mob chases once free");
}

/// Flat ground that ends 10 yd west of where the test Mage stands.
struct Cliff;

impl twilight_realm::Host for Cliff {
    fn ground_height(&self, _map: u32, x: f32, _y: f32, _z_hint: f32) -> Option<f32> {
        (x >= STAND[0] - 10.0).then_some(0.0)
    }
    fn line_of_sight(&self, _map: u32, _from: [f32; 3], _to: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _map: u32, _x: f32, _y: f32) -> Option<u32> {
        None
    }
}

#[test]
fn blink_stops_where_the_ground_ends() {
    let (mut realm, _, _, mut view) = world(3, &[]);
    go(&mut realm, &mut view, BLINK, None);
    realm.tick(0.1, &Cliff);
    let to = teleported(&view.drain(&mut realm)).expect("a teleport");
    assert!(
        flat(to, [STAND[0] - 10.0, STAND[1], 0.0]) < 0.01,
        "as far as the ground goes: {to:?}"
    );
}
