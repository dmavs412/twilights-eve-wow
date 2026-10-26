//! The character save: one versioned TOML blob per character. The realm decides when to save
//! ([`Storage`]); its host decides where the blob lives.

use serde::{Deserialize, Serialize};

use crate::characters::{Char, FishLog};
use crate::content::{self, Calling};
use crate::dungeon::Place;
use crate::inn::Bind;
use crate::inventory::{bag_size, is_journal, Inventory, Item, Pos, Satchel};

/// The save format this realm writes. It also reads versions 1 to 4, which have the same layout;
/// a save from any other version is refused. From version 3 the spell ids are the sheet's own,
/// from version 4 the talent ids too (ADR 0011): an older save's action bar and talents name
/// stock ids, remapped on load through [`OLD_IDS`]. Version 5 adds the Satchel (ADR 0023): an
/// older save's bag Journal Items move into it on load, once.
pub const SAVE_VERSION: u32 = 5;

/// The oldest save version the realm still reads. Version 2's quests are Hunting Quests, done once
/// per character; version 1's are read the same way.
const OLDEST_VERSION: u32 = 1;

/// The last version whose spell ids were stock 1.12.1 ones.
const LAST_STOCK_SPELLS_VERSION: u32 = 2;
/// The last version whose talent ids were stock `Talent.dbc` ones.
const LAST_STOCK_TALENTS_VERSION: u32 = 3;

/// The one-time tables of stock spell and talent ids to the sheet's own, `{spells, talents}`,
/// written by `tools/twilights-eve/migrate_from_scratch.py` (#204) and `migrate_talent_ids.py`
/// (#205).
const OLD_IDS: &str = include_str!("../../../data/twilights-eve/spell_id_remap.json");

/// One of [`OLD_IDS`]'s tables, `spells` or `talents`.
fn old_ids(table: &str) -> std::collections::HashMap<u32, u32> {
    let tables: std::collections::HashMap<String, std::collections::HashMap<String, u32>> =
        serde_json::from_str(OLD_IDS).expect("spell_id_remap.json parses");
    tables[table]
        .iter()
        .map(|(old, &new)| (old.parse().expect("an old id"), new))
        .collect()
}

/// Action button kind 0: a spell.
const ACTION_SPELL: u32 = 0;

/// A stock-id save's action bar with each spell button's id remapped; a spell the table lacks
/// leaves its button empty. Other buttons (macros, items) stay.
fn remap_action_bar(bar: Vec<(u8, u32)>) -> Vec<(u8, u32)> {
    let table = old_ids("spells");
    bar.into_iter()
        .filter_map(|(slot, packed)| {
            if packed >> 24 != ACTION_SPELL {
                return Some((slot, packed));
            }
            let new = table.get(&(packed & 0x00FF_FFFF))?;
            Some((slot, ACTION_SPELL << 24 | new))
        })
        .collect()
}

/// A stock-id save's talents with each `Talent.dbc` id remapped; a talent the table lacks is
/// dropped, and its points come back, since points are counted from the level.
fn remap_talents(talents: Vec<(u32, u8)>) -> Vec<(u32, u8)> {
    let table = old_ids("talents");
    talents
        .into_iter()
        .filter_map(|(id, rank)| Some((*table.get(&id)?, rank)))
        .collect()
}

/// Seconds between the in-world autosaves.
pub(crate) const AUTOSAVE_SECS: f32 = 60.0;

/// What the realm asks its host to do with a character's save.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum Storage {
    /// Write (or overwrite) this character's save.
    Write { guid: u64, blob: String },
    /// The character was deleted: remove its save.
    Remove { guid: u64 },
}

/// Why a save could not be imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    /// Not a save, or a damaged one.
    Unreadable,
    /// A save of another format version.
    Version(u32),
    /// The realm already has a character with this name or guid.
    Duplicate,
}

/// Just the version, read first so an older or newer layout fails as a version, not as garbage.
#[derive(Deserialize)]
struct Header {
    version: u32,
}

/// The save. Field names are the format: renaming one is a new version.
#[derive(Serialize, Deserialize)]
struct Save {
    version: u32,
    guid: u64,
    name: String,
    race: u8,
    gender: u8,
    /// `[skin, face, hair_style, hair_color, facial_hair]`.
    look: [u8; 5],
    calling: Calling,
    level: u8,
    xp: u32,
    map: u32,
    zone: u32,
    position: [f32; 3],
    orientation: f32,
    /// `[slot, kind << 24 | action]` per button, once the player has arranged the bar. Optional,
    /// so a save from before it existed still reads as version 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    action_bar: Option<Vec<(u8, u32)>>,
    /// Copper. Added within version 1: a save without it has none.
    #[serde(default)]
    money: u32,
    /// `[talent id, rank]` per learned talent. Added within version 1: a save without them has none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    talents: Vec<(u32, u8)>,
    /// Respecs bought, which price the next. Added within version 1.
    #[serde(default)]
    talent_wipes: u32,
    /// The ammo's entry, 0 for none (#157). Added within version 2: a save without it has none.
    #[serde(default, skip_serializing_if = "is_zero")]
    ammo: u32,
    /// `[item entry, the Unix ms it is ready at]` per item cooling down after use, so the time
    /// between sessions counts. Optional within version 1.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    item_ready_at: Vec<(u32, u64)>,
    /// The quests handed in: every Advancement and Repeatable Quest ever, and the Hunting Quests
    /// of this Tier. Added within version 1: a save without it has finished none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    completed: Vec<u32>,
    /// The `AreaTable.dbc` ids of the areas discovered. Added within version 2: a save without it
    /// has discovered none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    discovered: Vec<u32>,
    /// The Hearthstone's bind point. Added within version 2: a save without it is bound to the
    /// Crossroads inn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bind: Option<SavedBind>,
    /// The rested XP pool. Added within version 2: a save without it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rested: Option<f32>,
    /// The Unix ms they last left the world at, and whether in an inn: the time away banks rest
    /// on the next login. Added within version 2: a save without it banks none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    left_at: Option<u64>,
    #[serde(default)]
    left_in_inn: bool,
    /// Notoriety (#176). Added within version 2: a save without it has none.
    #[serde(default, skip_serializing_if = "is_nothing")]
    notoriety: f32,
    /// Bounty Rank (#178). Added within version 2: a save without it has none.
    #[serde(default, skip_serializing_if = "is_zero")]
    bounty_rank: u32,
    /// Marked (#179). Added within version 2: a save without it is not Marked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    marked: Option<crate::characters::Marked>,
    /// The entries of the Phased Dungeon doors they have opened. Added within version 4: a save
    /// without it has opened none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    doors_opened: Vec<u32>,
    /// The ids of the Calling Rumours they have finished. Added within version 4: a save without
    /// it has finished none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    finished_rumours: Vec<u32>,
    /// Fishing (#327), all added within version 4: a save without them has not learned it and
    /// is Angler Level 1.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    knows_fishing: bool,
    #[serde(default = "first_angler_level")]
    angler_level: u8,
    #[serde(default, skip_serializing_if = "is_zero")]
    angler_xp: u32,
    /// The Fishing Journal (#329), added within version 4: a save without it has caught nothing
    /// and chosen no bait. `[fish item, 1 caught or 2 perfect]` per fish.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    catches: Vec<(u32, u8)>,
    /// The catch log (#372), added within version 4. It replaces `catches`, which is read for an
    /// older save and never written.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    fish_log: Vec<FishLog>,
    /// Casts whose bobber floated; an older save loads at 0.
    #[serde(default)]
    fishing_casts: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bait: Option<u32>,
    /// The Legendary Fish (#331) caught and hinted, as trophy items.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    legends_caught: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    legend_hints: Vec<u32>,
    /// Herbalism (#438), all added within version 5: a save without them has not learned it and
    /// is Herbalist Level 1.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    knows_herbalism: bool,
    #[serde(default = "first_angler_level")]
    herbalist_level: u8,
    #[serde(default, skip_serializing_if = "is_zero")]
    herbalist_xp: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    herb_picks: Vec<(u32, u32)>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    herb_tracking: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    herb_perks: Vec<String>,
    #[serde(default, skip_serializing_if = "is_zero")]
    perk_resets: u32,
    /// The entry the empty Lure slot waits to refill with (version 5). A plain key, so before the
    /// gear tables.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lure_waiting: Option<u32>,
    /// Whether they have picked from the Calling Choice. A plain key, so before the tables; an old
    /// save has not chosen.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    calling_chosen: bool,
    /// The Fishing Journal's Pole and Lure slots (#371), added within version 4: a save without
    /// them has both empty. Each is an item at slot 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fishing_pole: Option<SavedItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fishing_lure: Option<SavedItem>,
    /// The Satchel's Journal Items (version 5). Before `items`, since TOML writes an array of
    /// tables after every plain key.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    satchel: Vec<SavedHeld>,
    /// Gear and bags. Added within version 1: a save without them carries nothing. Kept last,
    /// since TOML writes an array of tables after every plain key.
    #[serde(default)]
    items: Vec<SavedItem>,
    /// The quest log. Added within version 1: a save without it has no quests under way. Left
    /// out when empty, so it never writes a plain key after `items`' tables.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    quests: Vec<crate::quests::SavedQuest>,
}

/// The bind point in a save.
#[derive(Serialize, Deserialize)]
struct SavedBind {
    map: u32,
    position: [f32; 3],
    orientation: f32,
    /// The `AreaTable` id that names it.
    area: u32,
}

impl From<&Bind> for SavedBind {
    fn from(b: &Bind) -> Self {
        Self {
            map: b.place.map,
            position: b.place.position,
            orientation: b.place.orientation,
            area: b.area,
        }
    }
}

impl From<&SavedBind> for Bind {
    fn from(s: &SavedBind) -> Self {
        Self {
            place: Place {
                map: s.map,
                position: s.position,
                orientation: s.orientation,
            },
            area: s.area,
        }
    }
}

/// One Satchel entry in a save: a counted row, or a piece with its durability.
#[derive(Serialize, Deserialize)]
struct SavedHeld {
    entry: u32,
    count: u32,
    /// A piece's wear; left out for a counted row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    durability: Option<u32>,
}

fn saved_satchel(satchel: &Satchel) -> Vec<SavedHeld> {
    let counted = satchel.counts.iter().map(|(&entry, &count)| SavedHeld {
        entry,
        count,
        durability: None,
    });
    let pieces = satchel.pieces.iter().map(|p| SavedHeld {
        entry: p.entry,
        count: 1,
        durability: Some(p.durability),
    });
    counted.chain(pieces).collect()
}

/// Fills `inventory`'s Satchel from a save; an unknown item makes it unreadable.
fn load_satchel(inventory: &mut Inventory, saved: &[SavedHeld]) -> Result<(), SaveError> {
    for s in saved {
        let template = crate::items::template(s.entry).ok_or(SaveError::Unreadable)?;
        if s.count == 0 {
            return Err(SaveError::Unreadable);
        }
        let mut item = inventory.new_item(s.entry, s.count);
        item.durability = s.durability.unwrap_or(template.durability);
        inventory.satchel.deposit(item);
    }
    Ok(())
}

/// A version 4 save kept its Journal Items in the bags: move them to the Satchel.
fn move_journal_items(inventory: &mut Inventory) {
    let held: Vec<Pos> = inventory
        .iter()
        .filter(|(_, item)| is_journal(item.entry))
        .map(|(at, _)| at)
        .collect();
    for at in held {
        if let Some(item) = inventory.take(at) {
            inventory.satchel.deposit(item);
        }
    }
}

/// One item in a save.
#[derive(Serialize, Deserialize)]
struct SavedItem {
    /// The bag slot (19-22) of the equipped bag it is in; left out for a player slot. Added within
    /// version 1: a save without it holds nothing in bags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bag: Option<u8>,
    /// The player slot (0-18 worn, 19-22 bags, 23-38 the backpack), or the slot in its bag.
    slot: u8,
    entry: u32,
    count: u32,
    /// Left out: full.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    durability: Option<u32>,
}

fn saved_gear(item: &Option<Item>) -> Option<SavedItem> {
    item.as_ref().map(|item| SavedItem {
        bag: None,
        slot: 0,
        entry: item.entry,
        count: item.count,
        durability: Some(item.durability),
    })
}

/// A Journal slot's item from a save; an unknown item leaves the slot empty.
fn load_gear(inventory: &mut Inventory, saved: &Option<SavedItem>) -> Option<Item> {
    let s = saved.as_ref()?;
    let template = crate::items::template(s.entry)?;
    let mut item = inventory.new_item(s.entry, s.count.max(1));
    item.durability = s.durability.unwrap_or(template.durability);
    Some(item)
}

fn saved_items(inventory: &Inventory) -> Vec<SavedItem> {
    inventory
        .iter()
        .map(|(at, item)| SavedItem {
            bag: at.own_slot().is_none().then_some(at.bag),
            slot: at.slot,
            entry: item.entry,
            count: item.count,
            durability: Some(item.durability),
        })
        .collect()
}

/// The inventory a save describes; an unknown item, a bad slot, a slot named twice or a bag slot
/// holding what is no bag makes it unreadable. A bag's contents come after the bag. Gear the ten
/// Gear Slots retired is left out.
fn load_inventory(money: u32, ammo: u32, saved: &[SavedItem]) -> Result<Inventory, SaveError> {
    let mut inventory = Inventory::default();
    inventory.money = money;
    inventory.ammo = ammo;
    for s in saved.iter().filter(|s| !crate::items::retired(s.entry)) {
        let template = crate::items::template(s.entry).ok_or(SaveError::Unreadable)?;
        let at = s
            .bag
            .map_or(Pos::own(s.slot), |bag| Pos { bag, slot: s.slot });
        if !inventory.exists(at) || inventory.get(at).is_some() || s.count == 0 {
            return Err(SaveError::Unreadable);
        }
        if at.is_bag_slot() && bag_size(s.entry) == 0 {
            return Err(SaveError::Unreadable);
        }
        let mut item = inventory.new_item(s.entry, s.count);
        item.durability = s.durability.unwrap_or(template.durability);
        inventory.put(at, item);
    }
    Ok(inventory)
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}

fn first_angler_level() -> u8 {
    1
}

fn is_nothing(v: &f32) -> bool {
    *v == 0.0
}

pub(crate) fn export(c: &Char) -> String {
    let save = Save {
        version: SAVE_VERSION,
        guid: c.guid,
        name: c.name.clone(),
        race: c.race,
        gender: c.gender,
        look: c.look,
        calling: c.calling,
        level: c.level,
        xp: c.xp,
        map: c.map,
        zone: c.zone,
        position: c.position,
        orientation: c.orientation,
        action_bar: c.action_bar.clone(),
        money: c.inventory.money,
        talents: c.talents.clone(),
        talent_wipes: c.talent_wipes,
        ammo: c.inventory.ammo,
        item_ready_at: c
            .item_cooldowns
            .iter()
            .map(|&(entry, ready)| (entry, (ready * 1000.0).round() as u64))
            .collect(),
        completed: c.quests.completed.clone(),
        discovered: c.discovered.clone(),
        bind: Some(SavedBind::from(&c.bind)),
        rested: (c.rest.pool > 0.0).then_some(c.rest.pool),
        left_at: c.rest.left.map(|l| (l.at * 1000.0).round() as u64),
        left_in_inn: c.rest.left.is_some_and(|l| l.in_inn),
        notoriety: c.notoriety,
        bounty_rank: c.bounty_rank,
        marked: c.marked.clone(),
        doors_opened: c.doors_opened.clone(),
        finished_rumours: c.finished_rumours.clone(),
        knows_fishing: c.knows_fishing,
        angler_level: c.angler_level,
        angler_xp: c.angler_xp,
        catches: Vec::new(),
        fish_log: c.fish_log.clone(),
        fishing_casts: c.fishing_casts,
        bait: c.bait,
        legends_caught: c.legends_caught.clone(),
        legend_hints: c.legend_hints.clone(),
        knows_herbalism: c.knows_herbalism,
        herbalist_level: c.herbalist_level,
        herbalist_xp: c.herbalist_xp,
        herb_picks: c.herb_picks.clone(),
        herb_tracking: c.herb_tracking,
        herb_perks: c.herb_perks.clone(),
        perk_resets: c.perk_resets,
        lure_waiting: c.lure_waiting,
        calling_chosen: c.calling_chosen,
        fishing_pole: saved_gear(&c.fishing_pole),
        fishing_lure: saved_gear(&c.fishing_lure),
        satchel: saved_satchel(&c.inventory.satchel),
        items: saved_items(&c.inventory),
        quests: crate::quests::saved(&c.quests),
    };
    toml::to_string(&save).expect("a save always serializes")
}

/// The character a save describes, and lines for the host's log: each quest id it names that the
/// realm no longer has, dropped from the log and the quests handed in.
pub(crate) fn import(blob: &str) -> Result<(Char, Vec<String>), SaveError> {
    let header: Header = toml::from_str(blob).map_err(|_| SaveError::Unreadable)?;
    if !(OLDEST_VERSION..=SAVE_VERSION).contains(&header.version) {
        return Err(SaveError::Version(header.version));
    }
    let s: Save = toml::from_str(blob).map_err(|_| SaveError::Unreadable)?;
    if !content::valid_race(s.race) || s.gender > 1 || s.level == 0 {
        return Err(SaveError::Unreadable);
    }
    let (quests, dropped) = crate::quests::load(&s.quests, &s.completed)?;
    let mut c = Char {
        guid: s.guid,
        name: s.name,
        race: s.race,
        gender: s.gender,
        look: s.look,
        calling: s.calling,
        level: s.level,
        xp: s.xp,
        map: s.map,
        zone: s.zone,
        position: s.position,
        orientation: s.orientation,
        action_bar: if header.version <= LAST_STOCK_SPELLS_VERSION {
            s.action_bar.map(remap_action_bar)
        } else {
            s.action_bar
        },
        talents: if header.version <= LAST_STOCK_TALENTS_VERSION {
            remap_talents(s.talents)
        } else {
            s.talents
        },
        talent_wipes: s.talent_wipes,
        inventory: {
            let mut inventory = load_inventory(s.money, s.ammo, &s.items)?;
            load_satchel(&mut inventory, &s.satchel)?;
            if header.version < 5 {
                move_journal_items(&mut inventory);
            }
            inventory
        },
        item_cooldowns: s
            .item_ready_at
            .iter()
            .map(|&(entry, ms)| (entry, ms as f64 / 1000.0))
            .collect(),
        quests,
        discovered: crate::areas::known(s.discovered),
        bind: s
            .bind
            .as_ref()
            .map_or_else(Bind::crossroads_inn, Bind::from),
        rest: crate::rest::Rest {
            pool: s.rested.unwrap_or(0.0).max(0.0),
            left: s.left_at.map(|ms| crate::rest::Left {
                at: ms as f64 / 1000.0,
                in_inn: s.left_in_inn,
            }),
        },
        notoriety: s.notoriety.max(0.0),
        bounty_rank: s.bounty_rank,
        marked: s.marked,
        doors_opened: s.doors_opened,
        finished_rumours: s.finished_rumours,
        knows_fishing: s.knows_fishing,
        angler_level: s.angler_level.max(1),
        angler_xp: s.angler_xp,
        fish_log: if s.fish_log.is_empty() {
            s.catches
                .iter()
                .map(|&(item, best)| FishLog {
                    item,
                    best,
                    caught: 1,
                    perfect: u32::from(best == 2),
                    places: Vec::new(),
                })
                .collect()
        } else {
            s.fish_log
        },
        fishing_casts: s.fishing_casts,
        bait: s.bait,
        legends_caught: s.legends_caught,
        legend_hints: s.legend_hints,
        knows_herbalism: s.knows_herbalism,
        herbalist_level: s.herbalist_level.max(1),
        herbalist_xp: s.herbalist_xp,
        herb_picks: s.herb_picks,
        herb_tracking: s.herb_tracking,
        herb_perks: s.herb_perks,
        perk_resets: s.perk_resets,
        fishing_pole: None,
        fishing_lure: None,
        lure_waiting: s.lure_waiting,
        calling_chosen: s.calling_chosen,
    };
    c.fishing_pole = load_gear(&mut c.inventory, &s.fishing_pole);
    c.fishing_lure = load_gear(&mut c.inventory, &s.fishing_lure);
    crate::quests::settle(&mut c);
    crate::talents::keep_valid(&mut c);
    let notes = dropped
        .iter()
        .map(|id| {
            format!(
                "character {} ({}): dropped quest {id}, which the realm no longer has",
                c.guid, c.name
            )
        })
        .collect();
    Ok((c, notes))
}
