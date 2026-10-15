//! Gear: carried items stream in with the character, go on and off through the stock character
//! frame, and change the character sheet and the fight; what a Warrior cannot use is refused.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;

const FINE_SCIMITAR: u32 = 4560; // white 1H sword, 3-6 at 1.9 s
const DULL_BLADE: u32 = 1384; // white 1H sword, 1-3 at 1.9 s
const CROOKED_STAFF: u32 = 1388; // white staff, two-handed
const WARRIORS_SHIELD: u32 = 1438; // required level 4
const FIRE_WAND: u32 = 5069;
const GNOLL_WAR_HARNESS: u32 = 1211; // leather chest, 70 armour, +2 str +2 sta, required level 10
const LARGE_OGRE_CHAIN_ARMOR: u32 = 914; // required level 25
const JOUSTERS_CHESTPLATE: u32 = 8157; // plate, 339 armour, +15 str +4 sta, required level 40
const TUNNEL_PICK: u32 = 756; // two-handed axe, required level 24

const BACKPACK: u8 = 255;
const MAIN_HAND: u8 = 15;
const OFF_HAND: u8 = 16;
const CHEST: u8 = 4;
const PACK_1: u8 = 23;
const PACK_2: u8 = 24;

/// A warrior carrying `items`, with every event so far, so the descriptors can be merged.
struct Script {
    realm: Realm,
    me: u64,
    events: Vec<SessionEvent>,
}

impl Script {
    fn new(level: u8, items: &[Carried]) -> Self {
        let (realm, me, events) = warrior_with(level, 0, items);
        Self { realm, me, events }
    }

    /// [`Script::new`] as a character of `calling`.
    fn as_calling(calling: &str, level: u8, items: &[Carried]) -> Self {
        let (guid, blob) = save_with(level, 0, items);
        let blob = blob.replace(
            "calling = \"basic_warrior\"",
            &format!("calling = \"{calling}\""),
        );
        let (realm, me, events) = enter_saved(guid, &blob);
        Self { realm, me, events }
    }

    /// Sends `command`; its events, which also join the script's.
    fn send(&mut self, command: RealmCommand) -> Vec<SessionEvent> {
        self.realm.handle(command);
        let new = drain(&mut self.realm);
        self.events.extend(new.clone());
        new
    }

    /// The player as the client now holds it.
    fn me(&self) -> benilla_protocol::ObjectFields {
        descriptor(&self.events, self.me)
    }

    /// The guid of the item in player slot `slot`, 0 for none.
    fn slot(&self, slot: u8) -> u64 {
        let f = self.me();
        if slot < 23 {
            f.player_inv_slot(slot).unwrap_or(0)
        } else {
            f.player_pack_slot(slot - 23).unwrap_or(0)
        }
    }

    /// The guid of the item of `entry` among the creates.
    fn item(&self, entry: u32) -> u64 {
        item_creates(&self.events)
            .into_iter()
            .find(|(_, f)| f.object_entry() == Some(entry))
            .expect("the item's object")
            .0
    }

    fn equip(&mut self, slot: u8) -> Vec<SessionEvent> {
        self.send(RealmCommand::AutoEquipItem {
            bag: BACKPACK,
            slot,
        })
    }
}

fn refusal(events: &[SessionEvent]) -> Option<(u8, Option<u32>)> {
    events.iter().find_map(|e| match e {
        SessionEvent::InventoryFailure {
            reason,
            required_level,
            ..
        } => Some((*reason, *required_level)),
        _ => None,
    })
}

#[test]
fn carried_items_stream_in_ahead_of_the_character() {
    let s = Script::new(1, &[(PACK_1, FINE_SCIMITAR, 1), (CHEST, 2087, 1)]);
    let sword = s.item(FINE_SCIMITAR);
    assert_eq!(s.slot(PACK_1), sword);
    assert_eq!(s.me().player_visible_item_entry(CHEST), Some(2087));
    let create_at = |guid| {
        s.events.iter().position(|e| match e {
            SessionEvent::ItemCreate { guid: g, .. }
            | SessionEvent::ObjectCreate { guid: g, .. } => *g == guid,
            _ => false,
        })
    };
    assert!(create_at(sword) < create_at(s.me));
}

/// The `(weapons, armour)` subclass masks the client was last told.
fn proficiencies(events: &[SessionEvent]) -> (u32, u32) {
    let last = |class| {
        events
            .iter()
            .rev()
            .find_map(|e| match e {
                SessionEvent::Proficiency {
                    item_class,
                    subclass_mask,
                } if *item_class == class => Some(*subclass_mask),
                _ => None,
            })
            .expect("a proficiency")
    };
    (last(2), last(4))
}

#[test]
fn a_basic_warriors_proficiencies_are_sent_at_login() {
    let s = Script::new(1, &[]);
    let (weapons, armour) = proficiencies(&s.events);
    assert_ne!(weapons & 1 << 7, 0, "one-handed swords");
    assert_ne!(weapons & 1 << 10, 0, "staves");
    assert_ne!(weapons & 1 << 6, 0, "polearms at level 1");
    assert_eq!(weapons & 1 << 19, 0, "never wands");
    assert_eq!(
        armour & 0b111_1111,
        0b110_1111,
        "misc, cloth, leather, mail, bucklers and shields; no plate"
    );
}

#[test]
fn a_vanguards_proficiencies_add_plate() {
    let s = Script::as_calling("vanguard", 1, &[]);
    let (_, armour) = proficiencies(&s.events);
    assert_eq!(armour & 0b111_1111, 0b111_1111, "plate too");
}

#[test]
fn a_basic_warrior_is_refused_plate_for_want_of_proficiency() {
    let mut s = Script::new(10, &[(PACK_1, JOUSTERS_CHESTPLATE, 1)]);
    let events = s.equip(PACK_1);
    // `EQUIP_ERR_NO_REQUIRED_PROFICIENCY`, not the level.
    assert_eq!(refusal(&events), Some((8, None)));
    assert_eq!(s.slot(CHEST), 0);
}

#[test]
fn a_level_1_vanguard_wears_plate_and_gear_far_above_its_level() {
    let mut s = Script::as_calling(
        "vanguard",
        1,
        &[(PACK_1, JOUSTERS_CHESTPLATE, 1), (PACK_2, TUNNEL_PICK, 1)],
    );
    for slot in [PACK_1, PACK_2] {
        let events = s.equip(slot);
        assert_eq!(refusal(&events), None, "{events:?}");
    }
    assert_eq!(s.slot(CHEST), s.item(JOUSTERS_CHESTPLATE));
    assert_eq!(s.slot(MAIN_HAND), s.item(TUNNEL_PICK));
}

#[test]
fn plate_worn_by_a_vanguard_keeps_giving_its_stats_after_a_reload() {
    let mut s = Script::as_calling("vanguard", 1, &[(PACK_1, JOUSTERS_CHESTPLATE, 1)]);
    let naked = s.me();
    s.equip(PACK_1);
    let worn = s.me();
    assert_eq!(worn.player_posstat(0), Some(15), "the green +15 strength");
    assert_eq!(
        worn.unit_resistance(0),
        Some(naked.unit_resistance(0).unwrap() + 339)
    );
    let blob = s.realm.export_character(s.me).unwrap();
    let (_, me, events) = enter_saved(s.me, &blob);
    let again = descriptor(&events, me);
    assert_eq!(
        again.player_visible_item_entry(CHEST),
        Some(JOUSTERS_CHESTPLATE)
    );
    assert_eq!(again.player_posstat(0), Some(15));
    assert_eq!(again.unit_resistance(0), worn.unit_resistance(0));
}

#[test]
fn equipping_a_sword_puts_it_in_the_main_hand_and_sets_the_swing() {
    let mut s = Script::new(1, &[(PACK_1, FINE_SCIMITAR, 1)]);
    let sword = s.item(FINE_SCIMITAR);
    let events = s.equip(PACK_1);
    assert_eq!(refusal(&events), None);

    assert_eq!(s.slot(MAIN_HAND), sword);
    assert_eq!(s.slot(PACK_1), 0);
    let me = s.me();
    assert_eq!(me.player_visible_item_entry(MAIN_HAND), Some(FINE_SCIMITAR));
    assert_eq!(me.unit_base_attack_time(0), Some(1900));
    assert!((me.unit_min_damage().unwrap() - 6.936).abs() < 1e-3);
    assert!((me.unit_max_damage().unwrap() - 9.936).abs() < 1e-3);
}

#[test]
fn the_sword_swings_on_its_own_timer() {
    let mut s = Script::new(1, &[(PACK_1, FINE_SCIMITAR, 1)]);
    s.equip(PACK_1);
    let mob = the_mob(&s.events);
    stand_by(&mut s.realm, &mob);
    s.send(RealmCommand::AttackSwing { guid: mob.guid });
    let events = run(&mut s.realm, 5.8);
    let swings = events
        .iter()
        .filter(|e| matches!(e, SessionEvent::AttackerState(a) if a.attacker == s.me))
        .count();
    // 0, 1.9, 3.8 and 5.7 s; bare fists would swing three times.
    assert_eq!(swings, 4);
}

#[test]
fn armour_and_stats_change_the_sheet() {
    let mut s = Script::new(10, &[(PACK_1, GNOLL_WAR_HARNESS, 1)]);
    let before = s.me();
    s.equip(PACK_1);
    let after = s.me();
    assert_eq!(
        after.player_visible_item_entry(CHEST),
        Some(GNOLL_WAR_HARNESS)
    );
    assert_eq!(
        after.unit_resistance(0),
        Some(before.unit_resistance(0).unwrap() + 70)
    );
    assert_eq!(after.unit_stat(0), Some(before.unit_stat(0).unwrap() + 2));
    assert_eq!(after.unit_stat(2), Some(before.unit_stat(2).unwrap() + 2));
    assert_eq!(after.player_posstat(0), Some(2), "the green +2 strength");
    assert_eq!(after.player_posstat(2), Some(2));
    assert_eq!(
        after.unit_max_health(),
        Some(before.unit_max_health().unwrap() + 20)
    );
    assert_eq!(
        after.unit_attack_power(),
        Some(before.unit_attack_power().unwrap() + 4)
    );
}

#[test]
fn taking_it_off_puts_it_in_the_backpack_and_the_sheet_back() {
    let mut s = Script::new(1, &[(PACK_1, FINE_SCIMITAR, 1)]);
    let naked = s.me();
    s.equip(PACK_1);
    s.send(RealmCommand::AutoStoreBagItem {
        src_bag: BACKPACK,
        src_slot: MAIN_HAND,
        dst_bag: BACKPACK,
    });
    assert_eq!(s.slot(MAIN_HAND), 0);
    assert_eq!(s.slot(PACK_1), s.item(FINE_SCIMITAR));
    let me = s.me();
    assert_eq!(
        me.player_visible_item_entry(MAIN_HAND),
        None,
        "nothing worn there"
    );
    assert_eq!(me.unit_base_attack_time(0), Some(2000));
    assert_eq!(me.unit_min_damage(), naked.unit_min_damage());
}

#[test]
fn dragging_onto_a_worn_item_swaps_the_two() {
    let mut s = Script::new(1, &[(PACK_1, FINE_SCIMITAR, 1), (PACK_2, DULL_BLADE, 1)]);
    s.equip(PACK_1);
    s.send(RealmCommand::SwapInvItem {
        src: PACK_2,
        dst: MAIN_HAND,
    });
    assert_eq!(s.slot(MAIN_HAND), s.item(DULL_BLADE));
    assert_eq!(s.slot(PACK_2), s.item(FINE_SCIMITAR));
    assert_eq!(s.me().unit_base_attack_time(0), Some(1900));
}

#[test]
fn a_two_hander_sends_the_shield_to_the_backpack() {
    let mut s = Script::new(
        4,
        &[(OFF_HAND, WARRIORS_SHIELD, 1), (PACK_1, CROOKED_STAFF, 1)],
    );
    let events = s.equip(PACK_1);
    assert_eq!(refusal(&events), None);
    assert_eq!(s.slot(MAIN_HAND), s.item(CROOKED_STAFF));
    assert_eq!(s.slot(OFF_HAND), 0);
    assert_eq!(s.slot(PACK_1), s.item(WARRIORS_SHIELD));
}

#[test]
fn a_shield_beside_a_two_hander_is_refused() {
    let mut s = Script::new(
        4,
        &[(MAIN_HAND, CROOKED_STAFF, 1), (PACK_1, WARRIORS_SHIELD, 1)],
    );
    let events = s.equip(PACK_1);
    // `EQUIP_ERR_CANT_EQUIP_WITH_TWOHANDED`.
    assert_eq!(refusal(&events), Some((13, None)));
    assert_eq!(s.slot(PACK_1), s.item(WARRIORS_SHIELD));
}

#[test]
fn a_wand_is_refused_for_want_of_proficiency() {
    let mut s = Script::new(10, &[(PACK_1, FIRE_WAND, 1)]);
    let events = s.equip(PACK_1);
    // `EQUIP_ERR_NO_REQUIRED_PROFICIENCY`: "You do not have the required proficiency for that item."
    assert_eq!(refusal(&events), Some((8, None)));
    assert_eq!(s.slot(PACK_1), s.item(FIRE_WAND));
    assert_eq!(s.me().player_visible_item_entry(17), None);
}

#[test]
fn no_item_is_refused_for_level() {
    let mut s = Script::new(1, &[(PACK_1, LARGE_OGRE_CHAIN_ARMOR, 1)]);
    let events = s.equip(PACK_1);
    assert_eq!(refusal(&events), None, "{events:?}");
    assert_eq!(s.slot(CHEST), s.item(LARGE_OGRE_CHAIN_ARMOR));
}

#[test]
fn an_items_query_reports_no_required_level() {
    let mut s = Script::new(1, &[]);
    for entry in [LARGE_OGRE_CHAIN_ARMOR, JOUSTERS_CHESTPLATE] {
        let info = s
            .send(RealmCommand::ItemQuery { entry })
            .into_iter()
            .find_map(|e| match e {
                SessionEvent::ItemTemplate { info, .. } => info,
                _ => None,
            })
            .expect("its template");
        assert_eq!(info.required_level, 0, "{entry}");
    }
}

#[test]
fn an_item_in_the_wrong_slot_is_refused() {
    let mut s = Script::new(1, &[(PACK_1, FINE_SCIMITAR, 1)]);
    let events = s.send(RealmCommand::SwapInvItem {
        src: PACK_1,
        dst: CHEST,
    });
    // `EQUIP_ERR_ITEM_DOESNT_GO_TO_SLOT`.
    assert_eq!(refusal(&events), Some((3, None)));
    assert_eq!(s.slot(CHEST), 0);
}

#[test]
fn destroying_an_item_removes_it() {
    let mut s = Script::new(1, &[(PACK_1, FINE_SCIMITAR, 1)]);
    let sword = s.item(FINE_SCIMITAR);
    let events = s.send(RealmCommand::DestroyItem {
        bag: BACKPACK,
        slot: PACK_1,
        count: 0,
    });
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == sword)));
    assert_eq!(s.slot(PACK_1), 0);
}

#[test]
fn the_select_screen_shows_worn_gear() {
    let (_, blob) = save_with(
        1,
        0,
        &[(MAIN_HAND, FINE_SCIMITAR, 1), (PACK_1, DULL_BLADE, 1)],
    );
    let mut realm = realm();
    realm.import_character(&blob).unwrap();
    let c = &roster(&mut realm)[0];
    // Fine Scimitar's display, a one-hander.
    assert_eq!(c.equipment[usize::from(MAIN_HAND)].inventory_type, 13);
    assert_ne!(c.equipment[usize::from(MAIN_HAND)].display_id, 0);
    assert_eq!(c.equipment[usize::from(CHEST)].display_id, 0);
}

#[test]
fn gear_keeps_counting_after_a_level_up() {
    // A level 4 warrior 40 XP short of 5, the sword in hand; the level 3 mob's kill is 52 XP.
    let (me, blob) = save_with(4, 0, &[(MAIN_HAND, FINE_SCIMITAR, 1)]);
    let blob = blob.replace("xp = 0\n", "xp = 2060\n");
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let mut events = drain(&mut realm);
    let mob = the_mob(&events);
    events.extend(kill(&mut realm, &mob));
    assert!(events.iter().any(|e| matches!(e, SessionEvent::LevelUp(_))));
    assert_eq!(descriptor(&events, me).unit_base_attack_time(0), Some(1900));
}

#[test]
fn changing_gear_asks_for_a_save() {
    let mut s = Script::new(1, &[(PACK_1, FINE_SCIMITAR, 1)]);
    s.realm.drain_storage();
    s.equip(PACK_1);
    assert!(s.realm.drain_storage().iter().any(
        |op| matches!(op, twilight_realm::Storage::Write { blob, .. } if blob.contains("slot = 15"))
    ));
}
