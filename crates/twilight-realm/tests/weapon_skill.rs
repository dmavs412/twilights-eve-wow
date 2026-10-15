//! Every skill line the Warrior knows sits at the level's maximum, 5 × level, current and max:
//! at login, after a level-up, after Advancement, and whatever weapon is in hand. The slice has
//! no Weapon Master and no skill-ups. Played through the realm's seam, read off the player's
//! `PLAYER_SKILL_INFO` fields as the client holds them.

mod common;
mod gear_common;
mod quest_common;

use std::collections::BTreeMap;

use benilla_protocol::{ObjectFields, SessionEvent};
use common::*;
use gear_common::*;
use quest_common::*;
use twilight_realm::DevMultipliers;

// `SkillLine.dbc` ids.
const UNARMED: u16 = 162;
const DEFENSE: u16 = 95;
const POLEARMS: u16 = 229;
/// Every Warrior weapon line but Polearms, which comes at level 20
/// (`data/twilights-eve/warrior_proficiency.json`): axes, two-handed axes, bows, guns, maces,
/// two-handed maces, swords, two-handed swords, staves, fist weapons, daggers, thrown, crossbows.
const WEAPONS_FROM_LEVEL_1: [u16; 13] = [44, 172, 45, 46, 54, 160, 43, 55, 136, 473, 173, 176, 226];

const MAIN_HAND: u8 = 15;
const BACKPACK: u8 = 23;
const FINE_SCIMITAR: u32 = 4560; // 1H sword
const CROOKED_STAFF: u32 = 1388; // staff

/// The Warrior trainer, who gives the Advancement Quest into Tier 1.
const UZZEK: u32 = 5810;
const THE_WARRIORS_PATH: u32 = 64001;

/// The skill lines in `f`: id → `(value, max)`. A line in two slots fails the test.
fn skill_lines(f: &ObjectFields) -> BTreeMap<u16, (u16, u16)> {
    let mut lines = BTreeMap::new();
    for slot in 0..128 {
        let Some(s) = f.player_skill(slot) else {
            continue;
        };
        if s.skill_id == 0 {
            continue;
        }
        let before = lines.insert(s.skill_id, (s.value, s.max));
        assert!(before.is_none(), "skill line {} in two slots", s.skill_id);
    }
    lines
}

/// What a Warrior of `level` should read: every line it knows at `5 × level`.
fn at_max(level: u16) -> BTreeMap<u16, (u16, u16)> {
    let skill = 5 * level;
    let mut ids = vec![UNARMED, DEFENSE];
    ids.extend(WEAPONS_FROM_LEVEL_1);
    if level >= 20 {
        ids.push(POLEARMS);
    }
    ids.into_iter().map(|id| (id, (skill, skill))).collect()
}

fn with_dev_xp(level: u8, calling: &str, xp: f32) -> (Realm, u64, Vec<SessionEvent>) {
    let quest = &[(CROCOLISK_SKINS, [0; 4])];
    let (guid, blob) = save_with_quests(level, &[(BACKPACK, CROCOLISK_SKIN, 8)], 0, quest);
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
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

#[test]
fn a_new_warrior_knows_every_line_at_five() {
    let (_, me, events) = questing(1, &[], 0, &[]);
    assert_eq!(skill_lines(&descriptor(&events, me)), at_max(1));
}

#[test]
fn a_level_25_warrior_knows_polearms_and_every_line_at_125() {
    let (_, me, events) = questing_as("warrior", 25, &[], 0, &[]);
    assert_eq!(skill_lines(&descriptor(&events, me)), at_max(25));
}

#[test]
fn a_level_up_raises_every_line_to_the_new_maximum() {
    let (mut realm, me, mut events) = with_dev_xp(10, "warrior", 1.0);
    assert_eq!(skill_lines(&descriptor(&events, me)), at_max(10));
    events.extend(hand_in(&mut realm, giver(&events, ZUNJIN), CROCOLISK_SKINS));
    let now = descriptor(&events, me);
    assert_eq!(now.unit_level(), Some(11));
    assert_eq!(skill_lines(&now), at_max(11));
}

#[test]
fn reaching_level_20_brings_polearms_at_100() {
    let (mut realm, me, mut events) = with_dev_xp(19, "warrior", 1000.0);
    events.extend(hand_in(&mut realm, giver(&events, ZUNJIN), CROCOLISK_SKINS));
    let now = descriptor(&events, me);
    let level = now.unit_level().unwrap() as u16;
    assert!(level >= 20, "level {level}");
    assert_eq!(skill_lines(&now), at_max(level));
}

#[test]
fn advancement_brings_every_line_back_to_five() {
    let (mut realm, me, mut events) = questing(10, &[], 0, &[]);
    assert_eq!(skill_lines(&descriptor(&events, me)), at_max(10));
    let uzzek = giver(&events, UZZEK);
    realm.handle(RealmCommand::QuestGiverQueryQuest {
        npc: uzzek,
        quest: THE_WARRIORS_PATH,
    });
    events.extend(drain(&mut realm));
    events.extend(accept(&mut realm, uzzek, THE_WARRIORS_PATH));
    events.extend(hand_in(&mut realm, uzzek, THE_WARRIORS_PATH));
    let now = descriptor(&events, me);
    assert_eq!(now.unit_level(), Some(1));
    assert_eq!(skill_lines(&now), at_max(1));
}

#[test]
fn the_weapon_in_hand_changes_no_line() {
    for weapon in [FINE_SCIMITAR, CROOKED_STAFF] {
        let (_, me, events) = warrior_with(8, 0, &[(MAIN_HAND, weapon, 1)]);
        assert_eq!(skill_lines(&descriptor(&events, me)), at_max(8), "{weapon}");
    }

    // Swapping the staff in for the sword keeps every line where it was.
    let (mut realm, me, mut events) = warrior_with(
        8,
        0,
        &[(MAIN_HAND, FINE_SCIMITAR, 1), (BACKPACK, CROOKED_STAFF, 1)],
    );
    realm.handle(RealmCommand::AutoEquipItem {
        bag: 255,
        slot: BACKPACK,
    });
    events.extend(drain(&mut realm));
    let now = descriptor(&events, me);
    assert_eq!(
        now.player_visible_item_entry(MAIN_HAND),
        Some(CROOKED_STAFF),
        "the staff is in hand"
    );
    assert_eq!(skill_lines(&now), at_max(8));
}
