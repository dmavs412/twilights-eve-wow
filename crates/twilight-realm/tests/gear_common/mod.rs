//! Helpers for the loot and gear scripts: a warrior in the world with a save's worth of money and
//! items, the mob near the Crossroads, and fights against it.
#![allow(dead_code)]

use benilla_protocol::{EntityKind, ObjectFields, SessionEvent};
use twilight_realm::{Realm, RealmCommand, CROSSROADS};

use crate::common::{create, drain, realm, roster, FlatGround, CHAR_CREATE_SUCCESS, HUMAN};

pub const SEED: u64 = 4;
const CROSSROADS_XY: [f32; 3] = [CROSSROADS[0], CROSSROADS[1], 0.0];

/// One item a test puts in the save: `(player slot, entry, count)`. Slots 0-18 are equipment,
/// 23-38 the backpack.
pub type Carried = (u8, u32, u32);

/// A warrior's save at `level` with `money` and `items`, and nothing else: not even what a new
/// character starts with.
pub fn save_with(level: u8, money: u32, items: &[Carried]) -> (u64, String) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker.export_character(guid).unwrap();
    // The items come last: drop the starting ones.
    if let Some(at) = blob.find("[[items]]") {
        blob.truncate(at);
    }
    let mut blob = blob
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("money = 0\n", &format!("money = {money}\n"))
        .replace("items = []\n", "");
    for (slot, entry, count) in items {
        blob.push_str(&format!(
            "\n[[items]]\nslot = {slot}\nentry = {entry}\ncount = {count}\n"
        ));
    }
    (guid, blob)
}

/// One item a test puts inside an equipped bag: `(bag slot 19-22, slot in the bag, entry,
/// count)`.
pub type Bagged = (u8, u8, u32, u32);

/// [`save_with`], with `bagged` inside the bags `items` equips.
pub fn save_with_bagged(
    level: u8,
    money: u32,
    items: &[Carried],
    bagged: &[Bagged],
) -> (u64, String) {
    let (guid, mut blob) = save_with(level, money, items);
    for (bag, slot, entry, count) in bagged {
        blob.push_str(&format!(
            "
[[items]]
bag = {bag}
slot = {slot}
entry = {entry}
count = {count}
"
        ));
    }
    (guid, blob)
}

/// [`warrior_with`], with `bagged` inside the bags `items` equips.
pub fn warrior_with_bagged(
    level: u8,
    money: u32,
    items: &[Carried],
    bagged: &[Bagged],
) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with_bagged(level, money, items, bagged);
    enter_saved(guid, &blob)
}

/// A realm holding just the save `blob`, entered as `guid`, with the entry burst.
pub fn enter_saved(guid: u64, blob: &str) -> (Realm, u64, Vec<SessionEvent>) {
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

/// A human warrior of `level` in the world (on flat ground) carrying `money` and `items`, with
/// the entry burst.
pub fn warrior_with(level: u8, money: u32, items: &[Carried]) -> (Realm, u64, Vec<SessionEvent>) {
    let (guid, blob) = save_with(level, money, items);
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, guid, events)
}

/// Where `guid` ends up: the end of its last stroll or run in `events`, else `created`.
pub fn settled_at(events: &[SessionEvent], guid: u64, created: [f32; 3]) -> [f32; 3] {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::MonsterMove { guid: g, path, .. } if *g == guid => path.last().copied(),
            _ => None,
        })
        .next_back()
        .unwrap_or(created)
}

/// Moves the player to 2 yd from `mob`, on the Crossroads side, facing it.
pub fn stand_by(realm: &mut Realm, mob: &Mob) {
    let (dx, dy) = (
        CROSSROADS_XY[0] - mob.position[0],
        CROSSROADS_XY[1] - mob.position[1],
    );
    let len = (dx * dx + dy * dy).sqrt();
    let at = [
        mob.position[0] + dx / len * 2.0,
        mob.position[1] + dy / len * 2.0,
        mob.position[2],
    ];
    let facing = (mob.position[1] - at[1]).atan2(mob.position[0] - at[0]);
    realm.handle(RealmCommand::Move {
        position: at,
        orientation: facing,
    });
}

/// Ticks `secs` in 0.1 s steps (the first tick at once), collecting the events.
pub fn run(realm: &mut Realm, secs: f32) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    realm.tick(0.0, &FlatGround);
    events.extend(drain(realm));
    let mut t = 0.0;
    while t < secs - 1e-3 {
        realm.tick(0.1, &FlatGround);
        events.extend(drain(realm));
        t += 0.1;
    }
    events
}

/// Fights `mob` from beside it until it dies (at most two minutes), collecting the events.
pub fn kill(realm: &mut Realm, mob: &Mob) -> Vec<SessionEvent> {
    stand_by(realm, mob);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut events = run(realm, 0.0);
    for _ in 0..1200 {
        if last_health(&events, mob.guid) == Some(0) {
            return events;
        }
        realm.tick(0.1, &FlatGround);
        events.extend(drain(realm));
    }
    panic!("the mob outlived two minutes of fighting");
}

/// Every values update for `guid`.
pub fn values_of(events: &[SessionEvent], guid: u64) -> impl Iterator<Item = &ObjectFields> {
    events.iter().filter_map(move |e| match e {
        SessionEvent::ObjectValues { guid: g, fields } if *g == guid => Some(fields),
        _ => None,
    })
}

pub fn last_health(events: &[SessionEvent], guid: u64) -> Option<u32> {
    values_of(events, guid)
        .filter_map(|f| f.unit_health())
        .last()
}

/// `guid`'s create, merged with every values update after it: what the client holds.
pub fn descriptor(events: &[SessionEvent], guid: u64) -> ObjectFields {
    let mut f = ObjectFields::default();
    for e in events {
        match e {
            SessionEvent::ObjectCreate {
                guid: g, fields, ..
            }
            | SessionEvent::ItemCreate {
                guid: g, fields, ..
            } if *g == guid => f = fields.clone(),
            SessionEvent::ObjectValues { guid: g, fields } if *g == guid => f.merge(fields.clone()),
            _ => {}
        }
    }
    f
}

/// Every item create, `(guid, descriptor)`.
pub fn item_creates(events: &[SessionEvent]) -> Vec<(u64, ObjectFields)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ItemCreate { guid, fields, .. } => Some((*guid, fields.clone())),
            _ => None,
        })
        .collect()
}

/// How many of `entry` the player `me` carries in the backpack, as the client's fields say.
pub fn carried(events: &[SessionEvent], me: u64, entry: u32) -> u32 {
    let player = descriptor(events, me);
    (0..16)
        .filter_map(|i| player.player_pack_slot(i).filter(|g| *g != 0))
        .map(|guid| descriptor(events, guid))
        .filter(|item| item.object_entry() == Some(entry))
        .map(|item| item.item_stack_count().unwrap_or(1))
        .sum()
}

/// The mob's create, out of the entry burst.
pub struct Mob {
    pub guid: u64,
    pub position: [f32; 3],
    pub fields: ObjectFields,
}

pub fn the_mob(events: &[SessionEvent]) -> Mob {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } => Some(Mob {
                guid: *guid,
                position: *position,
                fields: fields.clone(),
            }),
            _ => None,
        })
        .expect("a mob streams in with the world")
}
