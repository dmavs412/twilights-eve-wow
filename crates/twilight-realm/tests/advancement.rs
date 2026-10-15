//! Tiers and Advancement, played through the realm's seam: the Tier Cap that stops XP, the
//! Advancement Quest at the Warrior trainer, what Advancement replaces and what it keeps, the
//! Tier Bonus, and Champion's quest that cannot be finished.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;
use quest_common::*;
use twilight_realm::DevMultipliers;

/// That warrior, as [`questing`] puts it in the world, with the dev multipliers set.
fn questing_with(
    dev: DevMultipliers,
    level: u8,
    items: &[Carried],
    quests: &[Underway],
) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(level, items, 0, quests);
    let mut realm = Realm::with_seed(SEED).with_dev(dev);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

fn xp_times(xp: f32) -> DevMultipliers {
    DevMultipliers {
        xp,
        ..DevMultipliers::default()
    }
}

/// Eight skins, enough for Zun'jin's Crocolisk Skins (level 10).
const EIGHT_SKINS: &[Carried] = &[(23, CROCOLISK_SKIN, 8)];

// ── The Tier Cap ────────────────────────────────────────────────────────────────────────────────

#[test]
fn xp_past_the_tier_cap_is_not_banked() {
    // Three levels' worth at level 9: a Basic Warrior stops at 10 with nothing over.
    let (mut realm, me, mut events) =
        questing_with(xp_times(3.0), 9, EIGHT_SKINS, &[(CROCOLISK_SKINS, [0; 4])]);
    let zunjin = giver(&events, ZUNJIN);
    events.extend(hand_in(&mut realm, zunjin, CROCOLISK_SKINS));
    let me_now = descriptor(&events, me);
    assert_eq!(me_now.unit_level(), Some(10));
    assert_eq!(me_now.player_xp(), Some(0));
    let blob = realm.export_character(me).unwrap();
    assert!(
        blob.contains("level = 10\n") && blob.contains("xp = 0\n"),
        "{blob}"
    );
}

#[test]
fn a_basic_warrior_at_the_tier_cap_gains_no_xp() {
    let (mut realm, me, mut events) = questing_with(
        xp_times(50.0),
        10,
        EIGHT_SKINS,
        &[(CROCOLISK_SKINS, [0; 4])],
    );
    let zunjin = giver(&events, ZUNJIN);
    let turned_in = hand_in(&mut realm, zunjin, CROCOLISK_SKINS);
    let done = completed(&turned_in).expect("the quest still completes");
    assert_eq!(done.xp, 0, "no XP at the cap, even with ?xp=");
    assert_eq!(done.money, 400, "the rest of the reward still comes");
    assert!(!turned_in
        .iter()
        .any(|e| matches!(e, SessionEvent::LevelUp(_))));
    events.extend(turned_in);
    let me_now = descriptor(&events, me);
    assert_eq!(me_now.unit_level(), Some(10));
    assert_eq!(me_now.player_xp(), Some(0));
}

#[test]
fn a_tier_1_warrior_levels_past_ten_and_stops_at_thirty() {
    let quest = &[(CROCOLISK_SKINS, [0; 4])];
    let (mut realm, me, mut events) = questing_as("warrior", 10, EIGHT_SKINS, 0, quest);
    events.extend(hand_in(&mut realm, giver(&events, ZUNJIN), CROCOLISK_SKINS));
    assert_eq!(descriptor(&events, me).unit_level(), Some(11));

    let (mut realm, me, mut events) = questing_as("warrior", 30, EIGHT_SKINS, 0, quest);
    let turned_in = hand_in(&mut realm, giver(&events, ZUNJIN), CROCOLISK_SKINS);
    assert_eq!(completed(&turned_in).unwrap().xp, 0, "Tier 1's cap is 30");
    events.extend(turned_in);
    assert_eq!(descriptor(&events, me).unit_level(), Some(30));
}

// ── The Tier Bonus ──────────────────────────────────────────────────────────────────────────────

#[test]
fn a_tier_1_warrior_has_the_tier_bonus_on_its_sheet() {
    let (_, me, basic) = questing(1, &[], 0, &[]);
    let (_, _, tier1) = questing_as("warrior", 1, &[], 0, &[]);
    let (basic, tier1) = (descriptor(&basic, me), descriptor(&tier1, me));
    let stats = |f: &benilla_protocol::ObjectFields| -> [u32; 5] {
        std::array::from_fn(|i| f.unit_stat(i as u8).unwrap())
    };
    let [s, a, st, i, sp] = stats(&basic);
    assert_eq!(stats(&tier1), [s + 12, a + 6, st + 12, i, sp + 3]);
    assert_eq!(
        tier1.unit_max_health().unwrap(),
        basic.unit_max_health().unwrap() + 120
    );
    assert_eq!(
        tier1.unit_attack_power().unwrap(),
        basic.unit_attack_power().unwrap() + 24
    );
}
