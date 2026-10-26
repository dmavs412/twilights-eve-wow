//! The account's characters, as character select sees them.

use benilla_protocol::messages::{
    CHAR_CREATE_NAME_IN_USE, CHAR_CREATE_SERVER_LIMIT, CHAR_CREATE_SUCCESS, CHAR_DELETE_SUCCESS,
    CLASS_WARRIOR,
};
use benilla_protocol::wire::Vector3d;
use benilla_protocol::{CharCreateReq, Character};

use crate::content::{
    self, Calling, CROSSROADS, EASTERN_KINGDOMS, ELWYNN_FOREST, KALIMDOR, THE_BARRENS,
};
use crate::inventory::{Inventory, Item};
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

/// One fish's row in the Fishing Journal's catch log.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct FishLog {
    pub(crate) item: u32,
    /// The best catch: 0 none, 1 caught, 2 perfect.
    pub(crate) best: u8,
    pub(crate) caught: u32,
    pub(crate) perfect: u32,
    /// `(place key, catches there)`: a Fishing Water's key when the bobber was in one, else the
    /// pool's.
    #[serde(default)]
    pub(crate) places: Vec<(String, u32)>,
}

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
    /// The `AreaTable.dbc` ids of the areas discovered, ascending: kept for good, through
    /// Advancement.
    pub(crate) discovered: Vec<u32>,
    /// Where the Hearthstone takes them.
    pub(crate) bind: crate::inn::Bind,
    /// Rested XP, and when they last left.
    pub(crate) rest: crate::rest::Rest,
    /// **Notoriety** (`GLOSSARY.md`): how much the underworld has noticed them.
    pub(crate) notoriety: f32,
    /// **Bounty Rank**: the Bounty Hunters they have killed since they last died.
    pub(crate) bounty_rank: u32,
    /// **Marked** (#179): their own Bounty Hunter killed them and is still out there.
    pub(crate) marked: Option<Marked>,
    /// The entries of the Phased Dungeon doors they have opened, ascending: kept for good.
    pub(crate) doors_opened: Vec<u32>,
    /// The ids of the Calling Rumours they have finished, ascending: kept for good, Advancement
    /// leaves them alone.
    pub(crate) finished_rumours: Vec<u32>,
    /// Whether Lee Brown has taught them Fishing (ADR 0018).
    pub(crate) knows_fishing: bool,
    /// Their **Angler Level**, from 1, and the XP toward the next.
    pub(crate) angler_level: u8,
    pub(crate) angler_xp: u32,
    /// The Fishing Journal's catch log, one row per fish caught at least once.
    pub(crate) fish_log: Vec<FishLog>,
    /// Casts whose bobber floated.
    pub(crate) fishing_casts: u32,
    /// The Journal's chosen bait, a `[[bait]]` item.
    pub(crate) bait: Option<u32>,
    /// The Journal's Pole slot: a fishing pole out of the bags. A cast reads it, not the main hand.
    pub(crate) fishing_pole: Option<Item>,
    /// The Journal's Lure slot: a `[[lure]]` item, which wears one point per bite.
    pub(crate) fishing_lure: Option<Item>,
    /// The entry of the lure that ran out in the Lure slot: the next one of it to reach the
    /// Satchel refills the slot (#392).
    pub(crate) lure_waiting: Option<u32>,
    /// Whether they have picked from the Calling Choice: it is offered until they do.
    pub(crate) calling_chosen: bool,
    /// The Legendary Fish (trophy items) they have caught: each bites for them once.
    pub(crate) legends_caught: Vec<u32>,
    /// The Legendary Fish they hold a hint of, from a Legend Rumour or an angler's gossip.
    pub(crate) legend_hints: Vec<u32>,
    /// Whether Brother Ansel has taught them Herbalism (#438).
    pub(crate) knows_herbalism: bool,
    pub(crate) herbalist_level: u8,
    pub(crate) herbalist_xp: u32,
    /// `(item, tiles pulled)`, ascending by item.
    pub(crate) herb_picks: Vec<(u32, u32)>,
    /// Whether Find Herbs is on.
    pub(crate) herb_tracking: bool,
    /// The perk keys they hold, one per slot filled.
    pub(crate) herb_perks: Vec<String>,
    /// The Journal's Tool slot, a `[[tool]]` item the Satchel holds (#440).
    pub(crate) herb_tool: Option<u32>,
    /// The Journal's Consumable slot, a `[[consumable]]` item; it keeps its choice when the
    /// Satchel runs out.
    pub(crate) herb_consumable: Option<u32>,
    /// How many times they have reset their perks.
    pub(crate) perk_resets: u32,
}

/// What being **Marked** holds (#179): the logged-in time left, and the hunter that won, as
/// much of it as brings it back at its home spot after a login.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Marked {
    pub(crate) secs_left: f32,
    /// Its draw: archetype, first name and epithet, as indexes into `notoriety.toml`.
    pub(crate) draw: [u32; 3],
    pub(crate) level: u8,
    /// The Bounty Rank it came at.
    pub(crate) rank: u32,
    /// Its affixes' keys.
    pub(crate) affixes: Vec<String>,
    pub(crate) map: u32,
    /// Where it won, which it roams around.
    pub(crate) home: [f32; 3],
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
        // The client always sends START_ELWYNN; START_BARRENS survives only for the realm's tests
        // until the Barrens is deleted (#374).
        let elwynn = req.start == benilla_protocol::START_ELWYNN;
        let (map, zone, [x, y, z, facing], bind) = if elwynn {
            let at = content::NORTHSHIRE;
            (
                EASTERN_KINGDOMS,
                ELWYNN_FOREST,
                at,
                crate::inn::Bind::northshire(),
            )
        } else {
            (
                KALIMDOR,
                THE_BARRENS,
                CROSSROADS,
                crate::inn::Bind::crossroads_inn(),
            )
        };
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
            calling: Calling::BASIC_WARRIOR,
            level: 1,
            xp: 0,
            map,
            zone,
            position: [x, y, z],
            orientation: facing,
            action_bar: None,
            talents: Vec::new(),
            talent_wipes: 0,
            inventory: content::starting_inventory(Calling::BASIC_WARRIOR),
            item_cooldowns: Vec::new(),
            quests: QuestLog::default(),
            discovered: Vec::new(),
            bind,
            rest: crate::rest::Rest::default(),
            notoriety: 0.0,
            bounty_rank: 0,
            marked: None,
            doors_opened: Vec::new(),
            finished_rumours: Vec::new(),
            knows_fishing: false,
            angler_level: 1,
            angler_xp: 0,
            fish_log: Vec::new(),
            fishing_casts: 0,
            bait: None,
            fishing_pole: None,
            fishing_lure: None,
            lure_waiting: None,
            calling_chosen: false,
            legends_caught: Vec::new(),
            legend_hints: Vec::new(),
            knows_herbalism: false,
            herbalist_level: 1,
            herbalist_xp: 0,
            herb_picks: Vec::new(),
            herb_tracking: false,
            herb_perks: Vec::new(),
            herb_tool: None,
            herb_consumable: None,
            perk_resets: 0,
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
    /// The class byte of the current Calling: its stat profile's `ChrClasses.dbc` id, which the wire
    /// carries until the client reads the Calling (#207).
    pub(crate) fn class(&self) -> u8 {
        crate::callings::of(self.calling).class()
    }

    /// The level the character's body and combat run on: `level` mapped onto its Tier's band
    /// (ADR 0012). `level` stays the real one, for progress and what the client shows.
    pub(crate) fn power_level(&self) -> u8 {
        crate::rules::stats::power_level(crate::callings::of(self.calling).tier, self.level)
    }

    fn roster_entry(&self) -> Character {
        let [skin, face, hair_style, hair_color, facial_hair] = self.look;
        let [x, y, z] = self.position;
        Character {
            guid: self.guid,
            name: self.name.clone(),
            race: self.race,
            class: self.class(),
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
