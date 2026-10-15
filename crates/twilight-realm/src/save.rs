//! The character save: one versioned TOML blob per character. The realm decides when to save
//! ([`Storage`]); its host decides where the blob lives.

use serde::{Deserialize, Serialize};

use crate::characters::Char;
use crate::clock;
use crate::content::{self, Calling};
use crate::inventory::{Inventory, SLOTS};

/// The save format this realm writes and reads. A save from any other version is refused.
pub const SAVE_VERSION: u32 = 1;

/// Seconds between the in-world autosaves.
pub(crate) const AUTOSAVE_SECS: f32 = 60.0;

/// What the realm asks its host to do with a character's save.
#[derive(Debug, Clone, PartialEq)]
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

/// Version 1 of the save. Field names are the format: renaming one is a new version.
#[derive(Serialize, Deserialize)]
struct SaveV1 {
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
    /// `[item entry, ms left]` per item cooling down after use. Optional within version 1.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    item_cooldowns: Vec<[u32; 2]>,
    /// Tokens counted on the character, from before they were an item in the bags: read, and
    /// turned into Tokens in the backpack; never written.
    #[serde(default, skip_serializing)]
    tokens: u32,
    /// The quests ever handed in. Added within version 1: a save without it has finished none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    completed: Vec<u32>,
    /// Gear and bags. Added within version 1: a save without them carries nothing. Kept last,
    /// since TOML writes an array of tables after every plain key.
    #[serde(default)]
    items: Vec<SavedItem>,
    /// The quest log. Added within version 1: a save without it has no quests under way. Left
    /// out when empty, so it never writes a plain key after `items`' tables.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    quests: Vec<crate::quests::SavedQuest>,
}

/// One item in a save.
#[derive(Serialize, Deserialize)]
struct SavedItem {
    /// The player slot: 0-18 worn, 23-38 the backpack.
    slot: u8,
    entry: u32,
    count: u32,
    /// Left out: full.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    durability: Option<u32>,
}

fn saved_items(inventory: &Inventory) -> Vec<SavedItem> {
    inventory
        .iter()
        .map(|(slot, item)| SavedItem {
            slot,
            entry: item.entry,
            count: item.count,
            durability: Some(item.durability),
        })
        .collect()
}

/// The inventory a save describes; an unknown item, a bad slot or a slot named twice makes it
/// unreadable.
fn load_inventory(money: u32, saved: &[SavedItem]) -> Result<Inventory, SaveError> {
    let mut inventory = Inventory::default();
    inventory.money = money;
    for s in saved {
        let template = crate::items::template(s.entry).ok_or(SaveError::Unreadable)?;
        if usize::from(s.slot) >= SLOTS || inventory.get(s.slot).is_some() || s.count == 0 {
            return Err(SaveError::Unreadable);
        }
        let mut item = inventory.new_item(s.entry, s.count);
        item.durability = s.durability.unwrap_or(template.durability);
        inventory.put(s.slot, item);
    }
    Ok(inventory)
}

pub(crate) fn export(c: &Char) -> String {
    let save = SaveV1 {
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
        item_cooldowns: c
            .item_cooldowns
            .iter()
            .map(|&(entry, left)| [entry, clock::ms(left)])
            .collect(),
        tokens: 0,
        completed: c.quests.completed.clone(),
        items: saved_items(&c.inventory),
        quests: crate::quests::saved(&c.quests),
    };
    toml::to_string(&save).expect("a save always serializes")
}

pub(crate) fn import(blob: &str) -> Result<Char, SaveError> {
    let header: Header = toml::from_str(blob).map_err(|_| SaveError::Unreadable)?;
    if header.version != SAVE_VERSION {
        return Err(SaveError::Version(header.version));
    }
    let s: SaveV1 = toml::from_str(blob).map_err(|_| SaveError::Unreadable)?;
    if !content::valid_race(s.race) || s.gender > 1 || s.level == 0 {
        return Err(SaveError::Unreadable);
    }
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
        action_bar: s.action_bar,
        talents: s.talents,
        talent_wipes: s.talent_wipes,
        inventory: load_inventory(s.money, &s.items)?,
        item_cooldowns: s
            .item_cooldowns
            .iter()
            .map(|&[entry, ms]| (entry, clock::secs(ms)))
            .collect(),
        quests: crate::quests::QuestLog {
            completed: s.completed,
            ..crate::quests::load(&s.quests)?
        },
    };
    if s.tokens > 0 {
        // As many as the backpack takes: all of them in any save from those builds.
        let room = (1..=s.tokens)
            .rev()
            .find(|&n| c.inventory.fits(crate::quests::TOKEN, n));
        if let Some(n) = room {
            let _ = c.inventory.store(crate::quests::TOKEN, n);
        }
    }
    crate::quests::settle(&mut c);
    crate::talents::keep_valid(&mut c);
    Ok(c)
}
