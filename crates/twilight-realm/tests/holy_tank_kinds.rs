//! The holy tank kinds (#677): a `judgement` that judges an aura row of the Calling (`seal=`), a
//! `mod_threat_pct` aura, the creature-type bonus and gate (`vs_type`, `only_type`), and
//! `add_aura_stacks` once per enemy hit (`per_target`). Played through the realm's seam with a
//! test Paladin Calling.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;
use twilight_realm::{Audience, Realm, RealmCommand};

const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_paladin"
name = "Test Paladin"
tier = 0
class = 2
cap = 10
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "seal_of_iron"
spell_id = 900101
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "grant_aura", aura_spell_id = 900102 }]

[[calling.skill]]
key = "judge"
spell_id = 900103
power_cost = 10
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "judgement", seal_spell_id = 900102, amount = { base = 50.0 }, school = "holy", heal_pct = 10 }]

[[calling.skill]]
key = "zeal"
spell_id = 900104
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = [{ kind = "grant_aura", aura_spell_id = 900105 }]

[[calling.skill]]
key = "holy_bolt"
spell_id = 900106
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "school_damage", school = "holy", amount = { base = 40.0 } }]

[[calling.skill]]
key = "rock"
spell_id = 900107
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "school_damage", amount = { base = 40.0 } }]

[[calling.skill]]
key = "exorcism"
spell_id = 900108
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "school_damage", school = "holy", amount = { base = 40.0 }, vs_type = "undead", vs_type_pct = 100 }]

[[calling.skill]]
key = "turn"
spell_id = 900109
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "stun", aura_spell_id = 900110, duration_ms = 4000, only_type = "undead" }]

[[calling.skill]]
key = "rally"
spell_id = 900111
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self_area"
radius_yd = 10.0
max_targets = 100
effects = [{ kind = "taunt", duration_ms = 3000 }, { kind = "add_aura_stacks", aura_spell_id = 900112, stacks = 1, duration_ms = 20000, max_stacks = 5, stack_rule = "stack", per_target = true }]

[[calling.skill]]
key = "shout"
spell_id = 900113
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self_area"
radius_yd = 10.0
max_targets = 100
effects = [{ kind = "taunt", duration_ms = 3000 }, { kind = "add_aura_stacks", aura_spell_id = 900114, stacks = 1, duration_ms = 20000, max_stacks = 5, stack_rule = "stack" }]

[[calling.aura]]
key = "aura_seal_of_iron"
spell_id = 900102
duration_ms = 30000
effects = [{ kind = "block_pct", pct = 5 }]

[[calling.aura]]
key = "aura_zeal"
spell_id = 900105
duration_ms = 30000
effects = [{ kind = "mod_threat_pct", pct = 50, school = "holy" }]
"#;

const SEAL_OF_IRON: u32 = 900101;
const SEAL_AURA: u32 = 900102;
const JUDGE: u32 = 900103;
const ZEAL: u32 = 900104;
const ZEAL_AURA: u32 = 900105;
const HOLY_BOLT: u32 = 900106;
const ROCK: u32 = 900107;
const EXORCISM: u32 = 900108;
const TURN: u32 = 900109;
const TURN_STUN: u32 = 900110;
const RALLY: u32 = 900111;
const RALLY_STACKS: u32 = 900112;
const SHOUT: u32 = 900113;
const SHOUT_STACKS: u32 = 900114;

/// `creature_type = 6` (undead) in `content/barrens.toml`.
const UNDEAD: u32 = 7067;
/// A Bristleback Hunter, `creature_type = 7` (humanoid).
const HUMANOID: u32 = 3258;

/// Where the test Paladin stands: east of the scorpid camp, out of its notice, facing west.
const STAND: [f32; 3] = [CAMP[0] + 40.0, CAMP[1], 0.0];

/// A level 10 test Paladin east of the camp: the realm, their guid and the view.
fn world() -> (Realm, u64, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Vellune", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        .replace("calling = \"basic_warrior\"", "calling = \"test_paladin\"");
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
    view.run(&mut realm, 1.0);
    (realm, guid, view)
}

/// A level 3 creature of `entry` `west` yards west and `north` yards north of the stand, seen.
fn spawn(realm: &mut Realm, view: &mut View, me: u64, entry: u32, west: f32, north: f32) -> u64 {
    let at = [STAND[0] - west, STAND[1] + north, 0.0];
    let guid = realm
        .spawn_for(&FlatGround, entry, 3, at, 1, Audience::of(&[me]))
        .expect("a creature of the entry");
    view.run(realm, 0.5);
    guid
}

/// Casts `spell_id` at `target`, asserting it went; what the client saw, with a tick after.
fn cast_at(realm: &mut Realm, view: &mut View, spell_id: u32, target: u64) -> Vec<SessionEvent> {
    realm.set_creature_health_pct(target, 100);
    realm.handle(cast(spell_id, Some(target)));
    let mut events = view.drain(realm);
    assert_eq!(refusal(&events, spell_id), None, "{spell_id} is cast");
    events.extend(view.run(realm, 0.1));
    events
}

/// The damage and school `spell_id` logged on `target` in `events`.
fn hit(events: &[SessionEvent], spell_id: u32, target: u64) -> Option<(u32, u8)> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id && l.target == target => {
            Some((l.damage, l.school))
        }
        _ => None,
    })
}

#[test]
fn judging_a_worn_seal_takes_it_off_hits_in_holy_and_heals_the_caster() {
    let (mut realm, me, mut view) = world();
    let mob = spawn(&mut realm, &mut view, me, HUMANOID, 15.0, 0.0);
    realm.set_player_power(200);
    realm.handle(cast(SEAL_OF_IRON, None));
    view.drain(&mut realm);
    assert!(view.aura(me, SEAL_AURA).is_some(), "the Seal is worn");
    realm.set_player_health(1);
    view.drain(&mut realm);
    let events = cast_at(&mut realm, &mut view, JUDGE, mob);
    assert!(view.aura(me, SEAL_AURA).is_none(), "judging takes it off");
    let (damage, school) = hit(&events, JUDGE, mob).expect("the judgement hits");
    assert_eq!(school, 1, "holy");
    assert!(
        damage == 50 || damage == 75,
        "50, or 75 on a crit; has {damage}"
    );
    let max = view.fields(me).unit_max_health().unwrap();
    let healed = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellHealLog(l) if l.spell_id == JUDGE && l.target == me => {
                Some(l.amount)
            }
            _ => None,
        })
        .expect("the caster heals");
    assert_eq!(healed, max / 10, "10% of the caster's maximum health");
}

#[test]
fn judging_with_no_seal_worn_is_refused_and_costs_nothing() {
    let (mut realm, me, mut view) = world();
    let mob = spawn(&mut realm, &mut view, me, HUMANOID, 15.0, 0.0);
    realm.set_player_power(200);
    view.drain(&mut realm);
    let mana = view.fields(me).unit_power(0).unwrap();
    realm.handle(cast(JUDGE, Some(mob)));
    let events = view.drain(&mut realm);
    assert!(refusal(&events, JUDGE).is_some(), "refused with no Seal");
    assert!(hit(&events, JUDGE, mob).is_none());
    assert_eq!(
        view.fields(me).unit_power(0).unwrap(),
        mana,
        "costs nothing"
    );
}

/// The threat `spell_id` adds on `mob` and the damage it dealt.
fn threat_of(realm: &mut Realm, view: &mut View, me: u64, spell_id: u32, mob: u64) -> (f32, u32) {
    let before = realm.creature_threat(mob, me);
    let events = cast_at(realm, view, spell_id, mob);
    let (damage, _) = hit(&events, spell_id, mob).expect("a hit");
    (realm.creature_threat(mob, me) - before, damage as u32)
}

#[test]
fn a_threat_aura_raises_the_threat_of_its_school_alone() {
    let (mut realm, me, mut view) = world();
    let mob = spawn(&mut realm, &mut view, me, HUMANOID, 15.0, 0.0);
    // The pull seeds threat from the mob's health; the hits after it are read.
    cast_at(&mut realm, &mut view, ROCK, mob);
    let (threat, damage) = threat_of(&mut realm, &mut view, me, HOLY_BOLT, mob);
    assert_eq!(threat, damage as f32, "1:1 with no aura");
    let (threat, damage) = threat_of(&mut realm, &mut view, me, ROCK, mob);
    assert_eq!(threat, damage as f32, "1:1 with no aura");
    realm.handle(cast(ZEAL, None));
    view.drain(&mut realm);
    assert!(
        view.aura(me, ZEAL_AURA).is_some(),
        "the threat aura is worn"
    );
    let (threat, damage) = threat_of(&mut realm, &mut view, me, HOLY_BOLT, mob);
    assert_eq!(threat, damage as f32 * 1.5, "holy makes 50% more");
    let (threat, damage) = threat_of(&mut realm, &mut view, me, ROCK, mob);
    assert_eq!(threat, damage as f32, "physical makes the same");
}

#[test]
fn a_type_bonus_doubles_the_hit_on_that_creature_type_alone() {
    let (mut realm, me, mut view) = world();
    let undead = spawn(&mut realm, &mut view, me, UNDEAD, 15.0, 5.0);
    let humanoid = spawn(&mut realm, &mut view, me, HUMANOID, 15.0, -5.0);
    let events = cast_at(&mut realm, &mut view, EXORCISM, undead);
    let (damage, _) = hit(&events, EXORCISM, undead).expect("a hit");
    assert!(
        damage == 80 || damage == 120,
        "double on undead; has {damage}"
    );
    let events = cast_at(&mut realm, &mut view, EXORCISM, humanoid);
    let (damage, _) = hit(&events, EXORCISM, humanoid).expect("a hit");
    assert!(
        damage == 40 || damage == 60,
        "normal on a humanoid; has {damage}"
    );
}

#[test]
fn a_type_gated_stun_lands_on_that_creature_type_alone() {
    let (mut realm, me, mut view) = world();
    let undead = spawn(&mut realm, &mut view, me, UNDEAD, 15.0, 5.0);
    let humanoid = spawn(&mut realm, &mut view, me, HUMANOID, 15.0, -5.0);
    cast_at(&mut realm, &mut view, TURN, undead);
    assert!(
        view.aura(undead, TURN_STUN).is_some(),
        "the undead is stunned"
    );
    cast_at(&mut realm, &mut view, TURN, humanoid);
    assert!(
        view.aura(humanoid, TURN_STUN).is_none(),
        "the humanoid is not"
    );
}

/// Three humanoids within the 10 yd of the area casts, none else.
fn three_around(realm: &mut Realm, view: &mut View, me: u64) {
    for north in [-3.0, 0.0, 3.0] {
        spawn(realm, view, me, HUMANOID, 5.0, north);
    }
}

fn stacks(view: &View, me: u64, spell_id: u32) -> u8 {
    view.aura(me, spell_id).map_or(0, |a| a.stacks)
}

#[test]
fn a_per_target_area_cast_adds_a_stack_for_each_enemy_hit_up_to_the_most() {
    let (mut realm, me, mut view) = world();
    three_around(&mut realm, &mut view, me);
    realm.handle(cast(RALLY, None));
    view.drain(&mut realm);
    assert_eq!(stacks(&view, me, RALLY_STACKS), 3, "one a mob");
    realm.handle(cast(RALLY, None));
    view.drain(&mut realm);
    assert_eq!(stacks(&view, me, RALLY_STACKS), 5, "held to max_stacks");
}

#[test]
fn an_area_cast_without_per_target_adds_its_stacks_once() {
    let (mut realm, me, mut view) = world();
    three_around(&mut realm, &mut view, me);
    realm.handle(cast(SHOUT, None));
    view.drain(&mut realm);
    assert_eq!(stacks(&view, me, SHOUT_STACKS), 1);
}
