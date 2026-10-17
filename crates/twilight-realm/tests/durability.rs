//! Durability in play: the stock durability figure reads the items' fields, gear at 0 gives no
//! stats until the goods vendor repairs it, and hits wear gear down.

mod common;
mod gear_common;

use benilla_protocol::{EntityKind, ObjectFields, SessionEvent};
use common::*;
use gear_common::*;

const GNOLL_WAR_HARNESS: u32 = 1211; // green leather chest: 70 armour, +2 str +2 sta, 65 durability
const FINE_SCIMITAR: u32 = 4560; // white sword, 30 durability
const CHEST: u8 = 4;
const MAIN_HAND: u8 = 15;

/// `(player slot, entry, durability)` of a worn item.
type Worn = (u8, u32, u32);

/// A human warrior of `level` with `money`, wearing `worn` at the durabilities given.
fn warrior_wearing(level: u8, money: u32, worn: &[Worn]) -> Script {
    let carried: Vec<Carried> = worn.iter().map(|(s, e, _)| (*s, *e, 1)).collect();
    let (guid, blob) = save_with(level, money, &carried);
    // Each item's table ends its `count` line: set its durability after it.
    let mut blob = blob;
    for (slot, entry, durability) in worn {
        blob = blob.replace(
            &format!("slot = {slot}\nentry = {entry}\ncount = 1\n"),
            &format!("slot = {slot}\nentry = {entry}\ncount = 1\ndurability = {durability}\n"),
        );
    }
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    Script {
        realm,
        me: guid,
        events,
    }
}

struct Script {
    realm: Realm,
    me: u64,
    events: Vec<SessionEvent>,
}

impl Script {
    fn send(&mut self, command: RealmCommand) -> Vec<SessionEvent> {
        self.realm.handle(command);
        let new = drain(&mut self.realm);
        self.events.extend(new.clone());
        new
    }

    fn me(&self) -> ObjectFields {
        descriptor(&self.events, self.me)
    }

    /// The item worn in `slot`: `(guid, durability, max durability)`.
    fn worn(&self, slot: u8) -> (u64, u32, u32) {
        let guid = self.me().player_inv_slot(slot).unwrap();
        let item = descriptor(&self.events, guid);
        (
            guid,
            item.item_durability().unwrap(),
            item.item_max_durability().unwrap(),
        )
    }

    fn vendor(&self) -> u64 {
        self.events
            .iter()
            .find_map(|e| match e {
                SessionEvent::ObjectCreate {
                    guid,
                    kind: EntityKind::Unit,
                    fields,
                    ..
                } if fields.unit_npc_flags() & 0x4000 != 0 => Some(*guid),
                _ => None,
            })
            .expect("the goods vendor repairs")
    }

    fn repair(&mut self, item: u64) -> Vec<SessionEvent> {
        let vendor = self.vendor();
        self.send(RealmCommand::RepairItem { vendor, item })
    }
}

#[test]
fn broken_gear_gives_no_stats_until_it_is_repaired() {
    let naked = warrior_wearing(10, 0, &[]).me();
    let mut s = warrior_wearing(10, 1000, &[(CHEST, GNOLL_WAR_HARNESS, 0)]);
    let (_, durability, max) = s.worn(CHEST);
    assert_eq!(
        (durability, max),
        (0, 65),
        "the durability figure's red chest"
    );
    let broken = s.me();
    assert_eq!(broken.unit_resistance(0), naked.unit_resistance(0));
    assert_eq!(broken.unit_stat(0), naked.unit_stat(0));

    // Repairing everything: 65 points of green leather at item level 15 is 195c.
    s.repair(0);
    assert_eq!(s.me().player_money(), Some(805));
    assert_eq!(s.worn(CHEST).1, 65);
    let fixed = s.me();
    assert_eq!(
        fixed.unit_resistance(0),
        Some(naked.unit_resistance(0).unwrap() + 70)
    );
    assert_eq!(fixed.unit_stat(0), Some(naked.unit_stat(0).unwrap() + 2));
}

#[test]
fn one_item_can_be_repaired_alone_and_nothing_is_repaired_on_credit() {
    let mut s = warrior_wearing(
        10,
        100,
        &[
            (CHEST, GNOLL_WAR_HARNESS, 0),
            (MAIN_HAND, FINE_SCIMITAR, 15),
        ],
    );
    // The harness's 195c is more than the purse: it stays broken, and the money stays.
    let harness = s.worn(CHEST).0;
    s.repair(harness);
    assert_eq!(s.worn(CHEST).1, 0);
    assert_eq!(s.me().player_money(), Some(100));

    // Half a white sword of item level 6 is 15 × 2 × 0.8 = 24c.
    let scimitar = s.worn(MAIN_HAND).0;
    s.repair(scimitar);
    assert_eq!(s.worn(MAIN_HAND).1, 30);
    assert_eq!(s.me().player_money(), Some(76));
}

/// Chest, sword, shield and bow: four of the nine Gear Slot equipment slots a wearing hit picks
/// from (ADR 0007); the others (neck, fingers, trinkets) have no durability.
const KIT: [Worn; 4] = [
    (CHEST, 795, 45),
    (MAIN_HAND, 886, 70),
    (16, 2362, 20),
    (17, 8180, 35),
];

#[test]
fn taking_hits_wears_gear_down_a_point_at_a_time() {
    // A level 60 warrior lets a level 1 mob at the Crossroads beat on it. One damaging hit in
    // two hundred wears a random Gear Slot; each session is an hour of hits, and
    // logging back in heals it.
    let mut s = warrior_wearing(60, 0, &KIT);
    for _session in 0..3 {
        let mob = the_mob(&s.events);
        stand_by(&mut s.realm, &mob);
        // Strike once to pull it, then only stand and take it.
        s.send(RealmCommand::AttackSwing { guid: mob.guid });
        s.send(RealmCommand::AttackStop);
        for _ in 0..36000 {
            s.realm.tick(0.1, &FlatGround);
            let new = drain(&mut s.realm);
            s.events.extend(new);
        }
        let worn: Vec<(u8, u32)> = KIT
            .iter()
            .map(|(slot, _, full)| (*slot, full - s.worn(*slot).1))
            .filter(|(_, lost)| *lost > 0)
            .collect();
        if !worn.is_empty() {
            assert!(
                worn.iter().all(|(_, lost)| *lost <= 2),
                "a point a hit: {worn:?}"
            );
            return;
        }
        s.send(RealmCommand::Logout);
        let me = s.me;
        s.events = s.send(RealmCommand::EnterWorld { guid: me });
        s.realm.tick(0.0, &FlatGround);
        let first = drain(&mut s.realm);
        s.events.extend(first);
    }
    panic!("three hours of hits wore nothing");
}
