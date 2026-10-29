//! Blink: a forward teleport that sheds stuns and stops where the ground ends. Played through the
//! realm's seam with a test Mage Calling.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;
use twilight_realm::{Realm, RealmCommand};

/// `blink` 20 yd and `stun_self` a 10 s stun on the caster.
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
"#;

const BLINK: u32 = 1953;
const STUN: u32 = 5211;

/// Where the test Mage stands: east of the scorpid camp, facing it.
const STAND: [f32; 3] = [CAMP[0] + 40.0, CAMP[1], 0.0];

/// A level 10 test Mage east of the camp: the realm, their guid and the view.
fn world(seed: u64) -> (Realm, u64, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Weaver", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_mage\"");
    let mut realm = Realm::with_seed(seed).with_callings(TEST_CALLINGS);
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
    view.run(&mut realm, 1.0);
    (realm, guid, view)
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

#[test]
fn blink_moves_the_caster_20_yards_forward_and_clears_a_stun() {
    let (mut realm, me, mut view) = world(1);
    go(&mut realm, &mut view, STUN, None);
    assert!(view.aura(me, STUN).is_some(), "stunned");
    go(&mut realm, &mut view, BLINK, None);
    assert!(view.aura(me, STUN).is_none(), "the stun is shed");
    let to = teleported(&view.run(&mut realm, 0.1)).expect("a teleport");
    let ahead = [STAND[0] - 20.0, STAND[1], 0.0];
    assert!(flat(to, ahead) < 0.01, "20 yd along the facing: {to:?}");
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
    let (mut realm, _, mut view) = world(3);
    go(&mut realm, &mut view, BLINK, None);
    realm.tick(0.1, &Cliff);
    let to = teleported(&view.drain(&mut realm)).expect("a teleport");
    assert!(
        flat(to, [STAND[0] - 10.0, STAND[1], 0.0]) < 0.01,
        "as far as the ground goes: {to:?}"
    );
}
