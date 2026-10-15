//! Every weapon line the Warrior knows sits at the level's maximum, 5 × level, current and max,
//! and every armour line its Calling allows at 1 of 1: at login, after a level-up, after
//! Advancement, and whatever weapon is in hand. The slice has no Weapon Master and no skill-ups. Played through the realm's seam, read off the player's
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
/// Every Warrior weapon line, known from level 1 by every Calling of the Warrior line: axes,
/// two-handed axes, bows, guns, maces, two-handed maces, polearms, swords, two-handed swords,
/// staves, fist weapons, daggers, thrown, crossbows.
const WEAPONS: [u16; 14] = [
    44, 172, 45, 46, 54, 160, 229, 43, 55, 136, 473, 173, 176, 226,
];

/// The armour lines a Basic Warrior wears: Cloth, Leather, Mail and Shield.
const BASIC_ARMOUR: [u16; 4] = [415, 414, 413, 433];
/// A Vanguard's and a Zealot's: the Basic Warrior's and Plate Mail.
const PLATE_ARMOUR: [u16; 5] = [415, 414, 413, 293, 433];

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

/// What a Warrior of `level` wearing `armour` should read: every weapon line at `5 × level`,
/// every armour line at 1 of 1.
fn at_max(level: u16, armour: &[u16]) -> BTreeMap<u16, (u16, u16)> {
    let skill = 5 * level;
    let mut ids = vec![UNARMED, DEFENSE];
    ids.extend(WEAPONS);
    let mut lines: BTreeMap<_, _> = ids.into_iter().map(|id| (id, (skill, skill))).collect();
    lines.extend(armour.iter().map(|&id| (id, (1, 1))));
    lines
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
    assert_eq!(
        skill_lines(&descriptor(&events, me)),
        at_max(1, &BASIC_ARMOUR)
    );
}

#[test]
fn a_level_25_vanguard_knows_every_line_at_125() {
    let (_, me, events) = questing_as("vanguard", 25, &[], 0, &[]);
    assert_eq!(
        skill_lines(&descriptor(&events, me)),
        at_max(25, &PLATE_ARMOUR)
    );
}

#[test]
fn a_level_up_raises_every_line_to_the_new_maximum() {
    let (mut realm, me, mut events) = with_dev_xp(10, "vanguard", 1.0);
    assert_eq!(
        skill_lines(&descriptor(&events, me)),
        at_max(10, &PLATE_ARMOUR)
    );
    events.extend(hand_in(&mut realm, giver(&events, ZUNJIN), CROCOLISK_SKINS));
    let now = descriptor(&events, me);
    assert_eq!(now.unit_level(), Some(11));
    assert_eq!(skill_lines(&now), at_max(11, &PLATE_ARMOUR));
}

#[test]
fn advancement_brings_every_line_back_to_five_and_adds_plate() {
    let (mut realm, me, mut events) = questing(10, &[], 0, &[]);
    assert_eq!(
        skill_lines(&descriptor(&events, me)),
        at_max(10, &BASIC_ARMOUR)
    );
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
    assert_eq!(skill_lines(&now), at_max(1, &PLATE_ARMOUR));
}

#[test]
fn the_weapon_in_hand_changes_no_line() {
    for weapon in [FINE_SCIMITAR, CROOKED_STAFF] {
        let (_, me, events) = warrior_with(8, 0, &[(MAIN_HAND, weapon, 1)]);
        assert_eq!(
            skill_lines(&descriptor(&events, me)),
            at_max(8, &BASIC_ARMOUR),
            "{weapon}"
        );
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
    assert_eq!(skill_lines(&now), at_max(8, &BASIC_ARMOUR));
}
