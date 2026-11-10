//! Mana kinds (#680): a `school_damage` hit that grows with the caster's mana fill
//! (`pct_at_full_mana`), and one that adds a share of the mana an `all_remaining` cast spent
//! (`per_mana_pct`). Played through the realm's seam by a level 30 test Mage Calling, whose bar
//! holds well over the 620 mana the burst tests need.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::initiate::{mana, max_mana};
use common::*;
use twilight_realm::{Realm, RealmCommand};

/// `plain` an arcane hit of 20; `swell` the same hit up to 50% more by the mana fill, free;
/// `swell_paid` the same for 100 mana; `torrent` a 3 s channel of the swelling hit that gives back
/// 100 mana after each tick's hit, so each tick finds another fill; `surge` an arcane hit of 20
/// costing 20 mana and up to 400 more, plus half the mana spent; `sip` and `gulp` spend 1 and 50
/// mana.
const TEST_CALLINGS: &str = r#"
[[calling]]
key = "test_mage"
name = "Test Mage"
tier = 0
class = 8
cap = 30
tier_bonus = { strength = 0, agility = 0, stamina = 0, intellect = 0, spirit = 0 }
shapeshift_form = 0
armour = ["cloth"]

[[calling.skill]]
key = "plain"
spell_id = 2136
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "school_damage", school = "arcane", amount = { base = 20.0 } }]

[[calling.skill]]
key = "swell"
spell_id = 5143
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "school_damage", school = "arcane", amount = { base = 20.0 }, pct_at_full_mana = 50 }]

[[calling.skill]]
key = "swell_paid"
spell_id = 5144
power_cost = 100
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
effects = [{ kind = "school_damage", school = "arcane", amount = { base = 20.0 }, pct_at_full_mana = 50 }]

[[calling.skill]]
key = "torrent"
spell_id = 5145
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
meta = { cast_kind = "channel", channel_ms = 3000, channel_tick_ms = 1000, interrupt_on = ["move"] }
effects = [
    { kind = "school_damage", school = "arcane", amount = { base = 20.0 }, pct_at_full_mana = 50 },
    { kind = "energize", power = "mana", amount = { base = 100.0 } },
]

[[calling.skill]]
key = "surge"
spell_id = 1449
power_cost = 20
cooldown_ms = 0
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 30.0]
meta = { cost_kind = "all_remaining", cost_extra_max = 400 }
effects = [{ kind = "school_damage", school = "arcane", amount = { base = 20.0 }, per_mana_pct = 50 }]

[[calling.skill]]
key = "sip"
spell_id = 1459
power_cost = 1
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = []

[[calling.skill]]
key = "gulp"
spell_id = 1460
power_cost = 50
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = []
"#;

const PLAIN: u32 = 2136;
const SWELL: u32 = 5143;
const SWELL_PAID: u32 = 5144;
const TORRENT: u32 = 5145;
const SURGE: u32 = 1449;
const SIP: u32 = 1459;
const GULP: u32 = 1460;

/// A level 30 test Mage 20 yd from a scorpid camp mob at `seed`: the realm, their guid, the mob
/// and the view.
fn world(seed: u64) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Weaver", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", "level = 30\n")
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
    (realm, guid, mob, view)
}

/// Spends the caster's mana down to `left` with `gulp` and `sip`, no time passing.
fn spend_to(realm: &mut Realm, view: &mut View, me: u64, left: u32) {
    while mana(view, me) > left {
        let spell_id = if mana(view, me) - left >= 50 {
            GULP
        } else {
            SIP
        };
        realm.handle(cast(spell_id, None));
        assert_eq!(refusal(&view.drain(realm), spell_id), None);
    }
    assert_eq!(mana(view, me), left);
}

/// Each blow of `spell_id` in `events`: `(damage, crit)`.
fn hits(events: &[SessionEvent], spell_id: u32) -> Vec<(u32, bool)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id => {
                Some((l.damage, l.hit_info != 0))
            }
            _ => None,
        })
        .collect()
}

/// Casts the free `spell_id` at `mob` until a blow lands that is no crit: its damage.
fn plain_hit(realm: &mut Realm, view: &mut View, mob: u64, spell_id: u32) -> u32 {
    for _ in 0..50 {
        realm.handle(cast(spell_id, Some(mob)));
        let events = view.drain(realm);
        assert_eq!(refusal(&events, spell_id), None);
        if let Some(&(damage, false)) = hits(&events, spell_id).first() {
            return damage;
        }
    }
    panic!("no plain blow of {spell_id} in 50 casts");
}

/// `damage` raised by `pct` percent, rounded as the realm rounds a raise.
fn raised(damage: u32, pct: u32) -> u32 {
    (damage * (100 + pct) + 50) / 100
}

#[test]
fn a_full_bar_makes_the_swelling_hit_half_again_as_big() {
    let (mut realm, me, mob, mut view) = world(1);
    let base = plain_hit(&mut realm, &mut view, mob.guid, PLAIN);
    assert_eq!(mana(&view, me), max_mana(&view, me), "a full bar");
    let swollen = plain_hit(&mut realm, &mut view, mob.guid, SWELL);
    assert_eq!(swollen, raised(base, 50), "1.5 times {base}");
}

#[test]
fn a_half_bar_after_the_cost_makes_it_a_quarter_bigger() {
    for seed in 0..20 {
        let (mut realm, me, mob, mut view) = world(0x680_0000 + seed);
        let base = plain_hit(&mut realm, &mut view, mob.guid, PLAIN);
        let max = max_mana(&view, me);
        // Half the bar is left once the cast has paid its 100.
        spend_to(&mut realm, &mut view, me, max / 2 + 100);
        realm.handle(cast(SWELL_PAID, Some(mob.guid)));
        let events = view.drain(&mut realm);
        assert_eq!(refusal(&events, SWELL_PAID), None);
        assert_eq!(mana(&view, me), max / 2);
        let Some(&(damage, false)) = hits(&events, SWELL_PAID).first() else {
            continue;
        };
        let pct = 50 * (max / 2) / max;
        assert!(pct == 25 || max % 2 == 1, "{pct} at {max}");
        assert_eq!(damage, raised(base, pct), "1.25 times {base}");
        return;
    }
    panic!("no plain blow in 20 worlds");
}

#[test]
fn each_tick_of_a_channel_reads_the_fill_at_that_tick() {
    for seed in 0..20 {
        let (mut realm, me, mob, mut view) = world(0x680_1000 + seed);
        let base = plain_hit(&mut realm, &mut view, mob.guid, PLAIN);
        let max = max_mana(&view, me);
        let start = max / 5;
        spend_to(&mut realm, &mut view, me, start);
        realm.handle(cast(TORRENT, Some(mob.guid)));
        assert_eq!(refusal(&view.drain(&mut realm), TORRENT), None);
        let ticks = hits(&view.run(&mut realm, 3.2), TORRENT);
        // A missed tick gives no mana back, and a crit hides the raise: try another world.
        if ticks.len() != 3 || ticks.iter().any(|t| t.1) {
            continue;
        }
        let want: Vec<(u32, bool)> = (0..3)
            .map(|k| (raised(base, 50 * (start + 100 * k) / max), false))
            .collect();
        assert_eq!(ticks, want, "from {start} of {max}, 100 more a tick");
        assert_ne!(ticks[0], ticks[2], "the raise grows with the bar");
        return;
    }
    panic!("no channel of three plain ticks in 20 worlds");
}

/// The `surge` blow from a bar of 20 and `above` more, the mana left after it, and the plain hit.
fn surge_from(above: u32) -> (u32, u32, u32) {
    for seed in 0..20 {
        let (mut realm, me, mob, mut view) = world(0x680_2000 + seed);
        let base = plain_hit(&mut realm, &mut view, mob.guid, PLAIN);
        spend_to(&mut realm, &mut view, me, 20 + above);
        realm.handle(cast(SURGE, Some(mob.guid)));
        let events = view.drain(&mut realm);
        assert_eq!(refusal(&events, SURGE), None);
        if let Some(&(damage, false)) = hits(&events, SURGE).first() {
            return (damage, mana(&view, me), base);
        }
    }
    panic!("no plain surge in 20 worlds");
}

#[test]
fn a_surge_adds_half_the_mana_it_spent_up_to_its_extra_max() {
    let (damage, left, base) = surge_from(600);
    assert_eq!(left, 200, "20 and 400 more spent");
    assert_eq!(damage, base + (20 + 400) / 2);

    let (damage, left, base) = surge_from(100);
    assert_eq!(left, 0, "20 and the 100 left spent");
    assert_eq!(damage, base + (20 + 100) / 2);
}
