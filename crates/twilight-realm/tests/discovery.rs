//! Discovery (spec #28): the first time a character stands in a subzone its fog lifts (the
//! explored-zones bit), "Discovered: <name>" shows (`SMSG_EXPLORATION_EXPERIENCE`), and it earns
//! 1/8 of the XP from its level to the next, times the Tier XP Rate. Once per character: through
//! Advancement and a reload, nothing is paid twice and the map stays revealed. Played on a host
//! with a scripted map of areas.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::{descriptor, SEED};
use quest_common::*;
use twilight_realm::DevMultipliers;

// `AreaTable.dbc` ids and explore bits (`content/areas.toml`).
const THE_CROSSROADS: u32 = 380;
const CROSSROADS_BIT: u32 = 104;
const CAMP_TAURAJO: u32 = 378;
const CAMP_TAURAJO_BIT: u32 = 102;
const BAEL_MODAN: u32 = 359;
const BAEL_MODAN_BIT: u32 = 85;

/// Level 1 to 2 is 400 XP: an eighth of it.
const LEVEL_1_DISCOVERY_XP: u32 = 50;

// Where each area lies on the scripted map: the Crossroads around the Hub, the others far off.
const AT_CROSSROADS: [f32; 3] = [-441.8, -2648.6, 0.0];
const AT_TAURAJO: [f32; 3] = [-2350.0, -1900.0, 0.0];
const AT_BAEL_MODAN: [f32; 3] = [-4000.0, -2200.0, 0.0];
/// Open ground no area claims.
const NOWHERE: [f32; 3] = [-1200.0, -2600.0, 0.0];

fn barrens() -> Areas {
    Areas(vec![
        ([AT_CROSSROADS[0], AT_CROSSROADS[1]], 150.0, THE_CROSSROADS),
        ([AT_TAURAJO[0], AT_TAURAJO[1]], 150.0, CAMP_TAURAJO),
        ([AT_BAEL_MODAN[0], AT_BAEL_MODAN[1]], 150.0, BAEL_MODAN),
    ])
}

#[test]
fn entering_a_new_subzone_lifts_its_fog_announces_it_and_pays_an_eighth_of_a_level() {
    let (mut realm, me, _) = in_world(1, "basic_warrior", 1.0);
    let events = step(&mut realm, AT_CROSSROADS);
    assert_eq!(
        discovered(&events),
        vec![(THE_CROSSROADS, LEVEL_1_DISCOVERY_XP)]
    );
    assert!(explored(&descriptor(&events, me), CROSSROADS_BIT));
    let gain = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::XpGain(x) => Some(*x),
            _ => None,
        })
        .expect("the XP is logged");
    assert_eq!((gain.total, gain.kill), (LEVEL_1_DISCOVERY_XP, false));
    assert_eq!(
        descriptor(&events, me).player_xp(),
        Some(LEVEL_1_DISCOVERY_XP)
    );
}

#[test]
fn re_entering_a_subzone_pays_nothing() {
    let (mut realm, _, _) = in_world(1, "basic_warrior", 1.0);
    step(&mut realm, AT_CROSSROADS);
    step(&mut realm, NOWHERE);
    let back = step(&mut realm, AT_CROSSROADS);
    assert!(discovered(&back).is_empty(), "{back:?}");
}

#[test]
fn each_discovery_keeps_the_bits_already_set() {
    let (mut realm, me, mut events) = in_world(1, "basic_warrior", 1.0);
    events.extend(step(&mut realm, AT_TAURAJO));
    events.extend(step(&mut realm, AT_CROSSROADS));
    events.extend(step(&mut realm, AT_BAEL_MODAN));
    let me_now = descriptor(&events, me);
    for bit in [CAMP_TAURAJO_BIT, CROSSROADS_BIT, BAEL_MODAN_BIT] {
        assert!(explored(&me_now, bit), "bit {bit}");
    }
}

#[test]
fn ground_no_area_claims_discovers_nothing() {
    let (mut realm, _, _) = in_world(1, "basic_warrior", 1.0);
    assert!(discovered(&step(&mut realm, NOWHERE)).is_empty());
}

#[test]
fn the_discovery_xp_is_an_eighth_of_the_characters_own_level() {
    // Level 5 to 6 is 2800 XP.
    let (mut realm, _, _) = in_world(5, "basic_warrior", 1.0);
    assert_eq!(
        discovered(&step(&mut realm, AT_CROSSROADS)),
        vec![(THE_CROSSROADS, 2800 / 8)]
    );
}

#[test]
fn the_tier_xp_rate_and_the_dev_multiplier_scale_it() {
    let (mut realm, _, _) = in_world(1, "vanguard", 1.0);
    assert_eq!(
        discovered(&step(&mut realm, AT_CROSSROADS)),
        vec![(THE_CROSSROADS, 3 * LEVEL_1_DISCOVERY_XP)]
    );
    let (mut realm, _, _) = in_world(1, "vanguard", 2.0);
    assert_eq!(
        discovered(&step(&mut realm, AT_CROSSROADS)),
        vec![(THE_CROSSROADS, 2 * 3 * LEVEL_1_DISCOVERY_XP)]
    );
}

#[test]
fn at_the_tier_cap_a_discovery_still_shows_but_pays_nothing() {
    let (mut realm, me, _) = in_world(10, "basic_warrior", 1.0);
    let events = step(&mut realm, AT_CROSSROADS);
    assert_eq!(discovered(&events), vec![(THE_CROSSROADS, 0)]);
    assert!(explored(&descriptor(&events, me), CROSSROADS_BIT));
}

#[test]
fn discoveries_survive_a_save_and_reload() {
    let (mut realm, me, _) = in_world(1, "basic_warrior", 1.0);
    step(&mut realm, AT_CROSSROADS);
    let blob = realm.export_character(me).expect("a save");

    let mut fresh = Realm::with_seed(SEED);
    fresh.import_character(&blob).unwrap();
    fresh.handle(RealmCommand::EnterWorld { guid: me });
    let mut events = drain(&mut fresh);
    assert!(
        explored(&descriptor(&events, me), CROSSROADS_BIT),
        "the map is revealed from the login on"
    );
    events = step(&mut fresh, AT_CROSSROADS);
    assert!(discovered(&events).is_empty(), "never paid twice");
}

#[test]
fn a_save_without_discoveries_loads_with_none() {
    let (mut realm, me, _) = in_world(1, "basic_warrior", 1.0);
    step(&mut realm, AT_CROSSROADS);
    let blob = realm.export_character(me).expect("a save");
    let old: String = blob
        .lines()
        .filter(|l| !l.starts_with("discovered"))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_ne!(old, blob, "the save names its discoveries");

    let mut fresh = Realm::with_seed(SEED);
    fresh.import_character(&old).unwrap();
    fresh.handle(RealmCommand::EnterWorld { guid: me });
    let events = drain(&mut fresh);
    assert!(!explored(&descriptor(&events, me), CROSSROADS_BIT));
    assert_eq!(
        discovered(&step(&mut fresh, AT_CROSSROADS)),
        vec![(THE_CROSSROADS, LEVEL_1_DISCOVERY_XP)]
    );
}

#[test]
fn after_advancement_the_map_stays_revealed_and_rediscovery_pays_nothing() {
    let (mut realm, me, events) = in_world(10, "basic_warrior", 1.0);
    let uzzek = giver(&events, UZZEK);
    step(&mut realm, AT_TAURAJO);
    step(&mut realm, AT_CROSSROADS);
    realm.handle(RealmCommand::QuestGiverQueryQuest {
        npc: uzzek,
        quest: THE_WARRIORS_PATH,
    });
    accept(&mut realm, uzzek, THE_WARRIORS_PATH);
    let advanced = hand_in(&mut realm, uzzek, THE_WARRIORS_PATH);
    assert_eq!(descriptor(&advanced, me).unit_level(), Some(1), "advanced");

    assert!(discovered(&step(&mut realm, AT_TAURAJO)).is_empty());
    // A subzone never seen still pays, at the Vanguard's rate.
    assert_eq!(
        discovered(&step(&mut realm, AT_BAEL_MODAN)),
        vec![(BAEL_MODAN, 3 * LEVEL_1_DISCOVERY_XP)]
    );

    realm.handle(RealmCommand::Logout);
    realm.handle(RealmCommand::EnterWorld { guid: me });
    let login = descriptor(&drain(&mut realm), me);
    for bit in [CAMP_TAURAJO_BIT, CROSSROADS_BIT, BAEL_MODAN_BIT] {
        assert!(explored(&login, bit), "bit {bit} after Advancement");
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────────────────────────

/// The Warrior trainer, and the Advancement Quest into Vanguard he gives at the Tier Cap.
const UZZEK: u32 = 5810;
const THE_WARRIORS_PATH: u32 = 64001;

/// A character of `calling` at `level`, standing on open ground no area claims, in a realm with
/// `?xp=` at `xp`; and the entry burst.
fn in_world(level: u8, calling: &str, xp: f32) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(level, &[], 0, &[]);
    let blob = blob.replace(
        "calling = \"basic_warrior\"",
        &format!("calling = \"{calling}\""),
    );
    let dev = DevMultipliers {
        xp,
        ..DevMultipliers::default()
    };
    let mut realm = Realm::with_seed(SEED).with_dev(dev);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    // The Hub's NPCs stream in on flat ground, before the player steps anywhere.
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

/// Moves the player to `to` and ticks once on the scripted map: the events.
fn step(realm: &mut Realm, to: [f32; 3]) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: to,
        orientation: 0.0,
    });
    realm.tick(0.0, &barrens());
    drain(realm)
}

/// Each discovery announced: `(area, xp)`.
fn discovered(events: &[SessionEvent]) -> Vec<(u32, u32)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ExplorationXp(x) => Some((x.area_id, x.xp)),
            _ => None,
        })
        .collect()
}

/// Whether explore bit `bit` is set in the player's `PLAYER_EXPLORED_ZONES`.
fn explored(me: &benilla_protocol::ObjectFields, bit: u32) -> bool {
    me.player_explored_zone_slot((bit / 32) as u16) & (1 << (bit % 32)) != 0
}
