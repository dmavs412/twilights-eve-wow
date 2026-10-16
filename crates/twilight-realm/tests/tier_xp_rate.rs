//! The Tier XP Rate (spec #28): every XP award is multiplied by the character's Tier, ×1 at
//! Tier 0, ×3 at Tier 1, ×5 at Tier 2 and ×7 at Tier 3, and the `?xp=` dev multiplier on top.
//! Played through the realm's front door: the same kill and the same quest, by Callings of each
//! Tier at the same level.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::{stand_at, Mob};
use common::*;
use gear_common::{Carried, SEED};
use quest_common::*;
use twilight_realm::rules::xp::tier_rate;
use twilight_realm::DevMultipliers;

/// Level 5 kills level 3: 70 × 3/5 = 42 XP at ×1.
const KILL_XP_AT_X1: u32 = 42;
/// The level 10 Crocolisk Skins, a Hunting Quest: half of level 10's 7600 XP at ×1.
const QUEST_XP_AT_X1: u32 = 3800;

#[test]
fn the_rate_is_1_3_5_and_7_by_tier() {
    assert_eq!([0, 1, 2, 3].map(tier_rate), [1, 3, 5, 7]);
}

#[test]
fn a_basic_warriors_kill_and_quest_xp_is_unchanged() {
    assert_eq!(kill_xp_as("basic_warrior", 1.0), KILL_XP_AT_X1);
    assert_eq!(quest_xp_as("basic_warrior", 1.0), QUEST_XP_AT_X1);
}

#[test]
fn a_vanguard_earns_three_times_a_basic_warriors_xp() {
    assert_eq!(kill_xp_as("vanguard", 1.0), 3 * KILL_XP_AT_X1);
    assert_eq!(quest_xp_as("vanguard", 1.0), 3 * QUEST_XP_AT_X1);
}

#[test]
fn a_tier_2_character_earns_five_times() {
    assert_eq!(kill_xp_as("zealot", 1.0), 5 * KILL_XP_AT_X1);
    assert_eq!(quest_xp_as("zealot", 1.0), 5 * QUEST_XP_AT_X1);
}

#[test]
fn the_dev_multiplier_multiplies_on_top_of_the_tier_rate() {
    assert_eq!(kill_xp_as("vanguard", 2.0), 2 * 3 * KILL_XP_AT_X1);
    assert_eq!(quest_xp_as("vanguard", 2.0), 2 * 3 * QUEST_XP_AT_X1);
    assert_eq!(kill_xp_as("basic_warrior", 2.0), 2 * KILL_XP_AT_X1);
}

#[test]
fn the_xp_bar_shows_the_multiplied_kill() {
    let (mut realm, me, events) = in_world_as("vanguard", 5, 1.0, &[]);
    let start = own_xp(&events, me);
    let events = kill(&mut realm, first_mob(&events));
    assert_eq!(xp_shown(&events, me), Some(start + 3 * KILL_XP_AT_X1));
}

// ── Helpers ─────────────────────────────────────────────────────────────────────────────────────

/// A level 5 character of `calling`, in a realm with `?xp=` at `xp`, and the entry burst.
fn in_world_as(
    calling: &str,
    level: u8,
    xp: f32,
    quests: &[Underway],
) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(level, EIGHT_SKINS, 0, quests);
    let blob = blob.replace(
        "calling = \"basic_warrior\"",
        &format!("calling = \"{calling}\""),
    );
    let dev = DevMultipliers {
        xp,
        ..DevMultipliers::default()
    };
    let mut realm = Realm::with_seed(SEED).with_dev(dev);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

/// Eight skins, enough for Zun'jin's Crocolisk Skins (level 10).
const EIGHT_SKINS: &[Carried] = &[(23, CROCOLISK_SKIN, 8)];

/// The XP a level 5 `calling` earns killing the level 3 scorpid, as the combat log reports it.
fn kill_xp_as(calling: &str, xp: f32) -> u32 {
    let (mut realm, _, events) = in_world_as(calling, 5, xp, &[]);
    let events = kill(&mut realm, first_mob(&events));
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::XpGain(x) if x.kill => Some(x.total),
            _ => None,
        })
        .expect("the kill pays XP")
}

/// The XP a level 5 `calling` earns handing in Crocolisk Skins, as the quest's completion shows.
fn quest_xp_as(calling: &str, xp: f32) -> u32 {
    let (mut realm, _, events) = in_world_as(calling, 5, xp, &[(CROCOLISK_SKINS, [0; 4])]);
    let zunjin = giver(&events, ZUNJIN);
    let turned_in = hand_in(&mut realm, zunjin, CROCOLISK_SKINS);
    completed(&turned_in).expect("the quest completes").xp
}

/// The first mob to stream in with the world: a level 3 scorpid by the Crossroads.
fn first_mob(events: &[SessionEvent]) -> Mob {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("a mob streams in with the world")
}

/// Stands by `mob` and auto-attacks it until it dies, collecting the events.
fn kill(realm: &mut Realm, mob: Mob) -> Vec<SessionEvent> {
    stand_at(realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut events = Vec::new();
    for _ in 0..1200 {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(realm));
        let dead = events.iter().any(|e| matches!(e,
            SessionEvent::ObjectValues { guid, fields } if *guid == mob.guid && fields.unit_health() == Some(0)));
        if dead {
            return events;
        }
    }
    panic!("the mob outlived two minutes of fighting");
}

/// The XP bar's last value.
fn xp_shown(events: &[SessionEvent], me: u64) -> Option<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectValues { guid, fields } if *guid == me => fields.player_xp(),
            _ => None,
        })
        .next_back()
}

/// The XP on the player's own create.
fn own_xp(events: &[SessionEvent], me: u64) -> u32 {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate { guid, fields, .. } if *guid == me => fields.player_xp(),
            _ => None,
        })
        .expect("our own create carries XP")
}
