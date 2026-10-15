//! The Tier 1 Warrior's three-tab tree, played through the realm's seam: 29 points at the cap,
//! the tabs' tier gates, and the Protection talents the sheet shows.

mod common;
mod gear_common;

use benilla_protocol::{ObjectFields, SessionEvent};
use common::*;
use gear_common::*;

// `Talent.dbc` ids in the Tier 1 tree (`content/callings.toml`).
const ANTICIPATION: u32 = 138;
const TOUGHNESS: u32 = 140;
const CRUELTY: u32 = 157;
const SWEEPING_STRIKES: u32 = 133;
/// WIP: no effect yet.
const IRON_WILL: u32 = 141;
const LAST_STAND: u32 = 153;
/// A Basic Warrior talent the Tier 1 tree does not have.
const IMPROVED_HEROIC_STRIKE: u32 = 124;

/// Brawler Gloves: 62 armour, worn in the hands slot.
const BRAWLER_GLOVES: u32 = 720;
const HANDS: u8 = 9;

/// A human Tier 1 Warrior of `level` in the world wearing `items`, with the entry burst.
fn tier1(level: u8, items: &[Carried]) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with(level, 0, items);
    let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"warrior\"");
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

fn learn(realm: &mut Realm, events: &mut Vec<SessionEvent>, talent_id: u32, ranks: u32) {
    for rank in 0..ranks {
        realm.handle(RealmCommand::LearnTalent { talent_id, rank });
    }
    events.extend(drain(realm));
}

/// The Defense line's bonus on the sheet (`PLAYER_SKILL_INFO` slot 1, its third dword).
fn defense_bonus(f: &ObjectFields) -> i32 {
    f.player_skill(1)
        .map_or(0, |s| i32::from(s.temp_bonus) + i32::from(s.perm_bonus))
}

#[test]
fn a_tier_1_warrior_at_the_cap_has_29_points_across_three_tabs() {
    let (mut realm, me, mut events) = tier1(30, &[]);
    assert_eq!(descriptor(&events, me).player_talent_points(), Some(29));
    // Not the Basic Warrior's tree any more.
    learn(&mut realm, &mut events, IMPROVED_HEROIC_STRIKE, 1);
    assert_eq!(descriptor(&events, me).player_talent_points(), Some(29));
    // Sweeping Strikes sits at tier 5: 20 points in Arms first.
    learn(&mut realm, &mut events, SWEEPING_STRIKES, 1);
    assert_eq!(descriptor(&events, me).player_talent_points(), Some(29));
    learn(&mut realm, &mut events, CRUELTY, 5);
    assert_eq!(descriptor(&events, me).player_talent_points(), Some(24));
}

#[test]
fn anticipation_raises_defense_and_toughness_the_armour_of_items() {
    let (mut realm, me, mut events) = tier1(30, &[(HANDS, BRAWLER_GLOVES, 1)]);
    let before = descriptor(&events, me);
    learn(&mut realm, &mut events, ANTICIPATION, 5);
    let after = descriptor(&events, me);
    assert_eq!(defense_bonus(&after), defense_bonus(&before) + 10);

    learn(&mut realm, &mut events, TOUGHNESS, 5);
    let armor = |f: &ObjectFields| f.unit_resistance(0).unwrap();
    // 10% more of the gloves' 62, rounded: 6.
    assert_eq!(armor(&descriptor(&events, me)), armor(&before) + 6);
}

#[test]
fn a_wip_talent_is_learnable_and_opens_the_next_tier() {
    let (mut realm, me, mut events) = tier1(30, &[]);
    // Last Stand sits at tier 3 of Protection: 10 points there first.
    learn(&mut realm, &mut events, LAST_STAND, 1);
    learn(&mut realm, &mut events, ANTICIPATION, 5);
    assert_eq!(descriptor(&events, me).player_talent_points(), Some(24));
    // Iron Will does nothing yet, but takes its five points.
    learn(&mut realm, &mut events, IRON_WILL, 5);
    assert_eq!(descriptor(&events, me).player_talent_points(), Some(19));
    learn(&mut realm, &mut events, LAST_STAND, 1);
    assert_eq!(descriptor(&events, me).player_talent_points(), Some(18));
}
