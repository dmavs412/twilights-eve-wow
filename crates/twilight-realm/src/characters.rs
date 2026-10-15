//! The account's characters, as character select sees them.

use benilla_protocol::messages::{
    CHAR_CREATE_NAME_IN_USE, CHAR_CREATE_SERVER_LIMIT, CHAR_CREATE_SUCCESS, CHAR_DELETE_SUCCESS,
    CLASS_WARRIOR,
};
use benilla_protocol::wire::Vector3d;
use benilla_protocol::{CharCreateReq, Character};

use crate::content::{self, Calling, CROSSROADS, KALIMDOR, THE_BARRENS};
use crate::inventory::Inventory;
use crate::quests::QuestLog;
use crate::save::SaveError;

/// `CharactersPerRealm`: the select screen's ten rows.
const MAX_CHARACTERS: usize = 10;

// The create and delete result codes the protocol crate does not name (vmangos `ResponseCodes`).
const CHAR_CREATE_DISABLED: u8 = 0x32;
const CHAR_DELETE_FAILED: u8 = 0x3A;
const CHAR_NAME_NO_NAME: u8 = 0x45;
const CHAR_NAME_TOO_SHORT: u8 = 0x46;
const CHAR_NAME_TOO_LONG: u8 = 0x47;
const CHAR_NAME_INVALID_CHARACTER: u8 = 0x48;

/// One character's lasting state.
#[derive(Debug, Clone)]
pub(crate) struct Char {
    pub(crate) guid: u64,
    pub(crate) name: String,
    pub(crate) race: u8,
    pub(crate) gender: u8,
    /// `[skin, face, hair_style, hair_color, facial_hair]`.
    pub(crate) look: [u8; 5],
    pub(crate) calling: Calling,
    pub(crate) level: u8,
    pub(crate) xp: u32,
    pub(crate) map: u32,
    pub(crate) zone: u32,
    pub(crate) position: [f32; 3],
    pub(crate) orientation: f32,
    /// The action bar as the player arranged it, `(slot, kind << 24 | action)`; `None` until
    /// they first change it, which shows the Calling's default bar.
    pub(crate) action_bar: Option<Vec<(u8, u32)>>,
    /// Learned talents of the current Calling's tree, `(talent id, rank)`.
    pub(crate) talents: Vec<(u32, u8)>,
    /// Respecs bought so far, which set the next one's price.
    pub(crate) talent_wipes: u32,
    /// Gear, bags and money.
    pub(crate) inventory: Inventory,
    /// The items cooling down after use, `(entry, the wall-clock second it is ready at)`: time
    /// away counts.
    pub(crate) item_cooldowns: Vec<(u32, f64)>,
    /// The quests under way.
    pub(crate) quests: QuestLog,
}

#[derive(Default)]
pub(crate) struct Characters {
    /// In creation order, as vmangos enumerates them.
    list: Vec<Char>,
    /// The last guid handed out; player guids are plain counters.
    last_guid: u64,
}

impl Characters {
    pub(crate) fn create(&mut self, req: &CharCreateReq) -> u8 {
        let name = match normalize_name(&req.name) {
            Ok(name) => name,
            Err(code) => return code,
        };
        if req.class != CLASS_WARRIOR || !content::valid_race(req.race) || req.gender > 1 {
            return CHAR_CREATE_DISABLED;
        }
        if self.list.iter().any(|c| c.name == name) {
            return CHAR_CREATE_NAME_IN_USE;
        }
        if self.list.len() >= MAX_CHARACTERS {
            return CHAR_CREATE_SERVER_LIMIT;
        }
        self.last_guid += 1;
        let [x, y, z, facing] = CROSSROADS;
        self.list.push(Char {
            guid: self.last_guid,
            name,
            race: req.race,
            gender: req.gender,
            look: [
                req.skin,
                req.face,
                req.hair_style,
                req.hair_color,
                req.facial_hair,
            ],
            calling: Calling::BasicWarrior,
            level: 1,
            xp: 0,
            map: KALIMDOR,
            zone: THE_BARRENS,
            position: [x, y, z],
            orientation: facing,
            action_bar: None,
            talents: Vec::new(),
            talent_wipes: 0,
            inventory: content::starting_inventory(),
            item_cooldowns: Vec::new(),
            quests: QuestLog::default(),
        });
        CHAR_CREATE_SUCCESS
    }

    /// The character created last.
    pub(crate) fn newest(&self) -> Option<&Char> {
        self.list.iter().max_by_key(|c| c.guid)
    }

    /// Hands out only guids above `guid` from now on.
    pub(crate) fn reserve(&mut self, guid: u64) {
        self.last_guid = self.last_guid.max(guid);
    }

    /// Adds a character read from a save, keeping its guid.
    pub(crate) fn insert(&mut self, c: Char) -> Result<u64, SaveError> {
        if self
            .list
            .iter()
            .any(|o| o.guid == c.guid || o.name == c.name)
        {
            return Err(SaveError::Duplicate);
        }
        self.last_guid = self.last_guid.max(c.guid);
        let guid = c.guid;
        self.list.push(c);
        // Saves load in any order; the roster keeps creation order, which guids follow.
        self.list.sort_by_key(|c| c.guid);
        Ok(guid)
    }

    pub(crate) fn delete(&mut self, guid: u64) -> u8 {
        let before = self.list.len();
        self.list.retain(|c| c.guid != guid);
        if self.list.len() < before {
            CHAR_DELETE_SUCCESS
        } else {
            CHAR_DELETE_FAILED
        }
    }

    pub(crate) fn get(&self, guid: u64) -> Option<&Char> {
        self.list.iter().find(|c| c.guid == guid)
    }

    pub(crate) fn get_mut(&mut self, guid: u64) -> Option<&mut Char> {
        self.list.iter_mut().find(|c| c.guid == guid)
    }

    pub(crate) fn set_position(&mut self, guid: u64, position: [f32; 3], orientation: f32) {
        if let Some(c) = self.list.iter_mut().find(|c| c.guid == guid) {
            c.position = position;
            c.orientation = orientation;
        }
    }

    /// `CMSG_SET_ACTION_BUTTON`: puts `packed` on `slot` of the character's bar; 0 clears it.
    pub(crate) fn set_action_button(&mut self, guid: u64, slot: u8, packed: u32) {
        if let Some(c) = self.list.iter_mut().find(|c| c.guid == guid) {
            let bar = c
                .action_bar
                .get_or_insert_with(|| crate::callings::default_bar(c.calling));
            bar.retain(|&(s, _)| s != slot);
            if packed != 0 {
                bar.push((slot, packed));
                bar.sort_unstable_by_key(|&(s, _)| s);
            }
        }
    }

    pub(crate) fn roster(&self) -> Vec<Character> {
        self.list.iter().map(Char::roster_entry).collect()
    }
}

impl Char {
    fn roster_entry(&self) -> Character {
        let [skin, face, hair_style, hair_color, facial_hair] = self.look;
        let [x, y, z] = self.position;
        Character {
            guid: self.guid,
            name: self.name.clone(),
            race: self.race,
            class: CLASS_WARRIOR,
            gender: self.gender,
            skin,
            face,
            hair_style,
            hair_color,
            facial_hair,
            level: self.level,
            zone: self.zone,
            map: self.map,
            position: Vector3d { x, y, z },
            flags: 0,
            equipment: crate::equipment::roster_gear(&self.inventory),
            pet_display_id: 0,
            pet_level: 0,
            pet_family: 0,
        }
    }
}

/// vmangos' name rules for a latin realm: 2–12 letters, stored first-upper, rest-lower.
fn normalize_name(raw: &str) -> Result<String, u8> {
    let len = raw.chars().count();
    if len == 0 {
        return Err(CHAR_NAME_NO_NAME);
    }
    if !raw.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(CHAR_NAME_INVALID_CHARACTER);
    }
    if len < 2 {
        return Err(CHAR_NAME_TOO_SHORT);
    }
    if len > 12 {
        return Err(CHAR_NAME_TOO_LONG);
    }
    let lower = raw.to_ascii_lowercase();
    Ok(lower[..1].to_ascii_uppercase() + &lower[1..])
}
