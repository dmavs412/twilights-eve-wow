//! Shared helpers: a realm on flat ground, and the steps a player script repeats.
#![allow(dead_code, unused_imports)]

pub mod acolyte;
pub mod cutthroat;
pub mod fight;
pub mod initiate;
pub mod party;
pub mod ranger;
pub mod vanguard;

/// The old two-handed Zealot as a Tier 0 test Calling, "test_zealot": the borrowed Paladin Seals
/// (`Effect::Seal`), the TOML Judgement of the active Seal, Mortal Strike and Charge, which no
/// shipped Calling has since the Zealot moved into the sheet (#686). Tier 0, for a fight a weak
/// body has to take hits in. Judgement and Hammer of Justice take test ids, since the sheet's
/// Zealot keeps 1000067 and 1000068. Pass it to `Realm::with_callings`; [`test_zealot_spell`]
/// reads its ids.
pub const TEST_ZEALOT: &str = r#"
[[calling]]
key = "test_zealot"
name = "Zealot"
tier = 0
power = "rage"
stat_profile = "warrior"
cap = 10
tier_bonus = { strength = 28, agility = 14, stamina = 28, intellect = 0, spirit = 7 }
shapeshift_form = 17
armour = ["cloth", "leather", "mail", "plate", "shield"]

[[calling.skill]]
key = "seal_of_command"
spell_id = 1000065
spell_name = "Seal of Command"
power_cost = 10
cooldown_ms = 0
gcd_ms = 1500
target = "self"
borrowed = { family_bit = 33 }
effects = [
    { kind = "seal", duration_ms = 30000, seal = { kind = "command", procs_per_minute = 7.0, weapon_pct = 70, proc_spell_id = 20424, proc_spell_name = "Seal of Command", judgement_spell_id = 20467, judgement_spell_name = "Judgement of Command", judgement = { base = 30.0, per_level = 3.0, per_ap = 0.15 } } },
]

[[calling.skill]]
key = "seal_of_the_crusader"
spell_id = 1000066
spell_name = "Seal of the Crusader"
power_cost = 10
cooldown_ms = 0
gcd_ms = 1500
target = "self"
borrowed = { family_bit = 34 }
effects = [
    { kind = "seal", duration_ms = 30000, seal = { kind = "crusader", attack_power = { base = 20.0, per_level = 4.0 }, haste_pct = 40, judgement_spell_id = 20300, judgement_spell_name = "Judgement of the Crusader", judgement_duration_ms = 10000, holy_taken_pct = 20 } },
]

[[calling.skill]]
key = "judgement"
spell_id = 900167
spell_name = "Judgement"
power_cost = 5
cooldown_ms = 10000
gcd_ms = 0
target = "enemy"
range_yd = [0.0, 10.0]
borrowed = { family_bit = 35 }
effects = [{ kind = "judgement" }]

[[calling.skill]]
key = "hammer_of_justice"
spell_id = 900168
spell_name = "Hammer of Justice"
power_cost = 15
cooldown_ms = 60000
gcd_ms = 1500
target = "enemy"
range_yd = [0.0, 10.0]
borrowed = { family_bit = 36 }
effects = [{ kind = "stun", aura_spell_id = 900168, aura_spell_name = "Hammer of Justice", duration_ms = 6000 }]

[[calling.skill]]
key = "mortal_strike"
spell_id = 1000069
spell_name = "Mortal Strike"
power_cost = 30
cooldown_ms = 6000
gcd_ms = 1500
target = "enemy"
range_yd = [0.0, 5.0]
effects = [
    { kind = "weapon_strike", weapon = "normalized", bonus = { base = 25.0, per_level = 2.5 } },
    { kind = "healing_reduction", pct = 50, duration_ms = 10000 },
]

[[calling.skill]]
key = "charge"
spell_id = 1000070
spell_name = "Charge"
power_cost = 0
cooldown_ms = 15000
gcd_ms = 0
target = "enemy"
range_yd = [8.0, 25.0]
requires_out_of_combat = true
effects = [
    { kind = "charge", speed_yd_s = 24.0 },
    { kind = "energize", power = "rage", amount = { base = 11.0, per_level = 0.1 } },
    { kind = "stun", aura_spell_id = 1000190, aura_spell_name = "Charge (stun)", duration_ms = 1000 },
]
"#;

/// [`TEST_ZEALOT`], for `Realm::with_callings`.
pub fn tier_0_zealot() -> String {
    TEST_ZEALOT.to_string()
}

/// The spell id of [`TEST_ZEALOT`]'s skill `key`.
pub fn test_zealot_spell(key: &str) -> u32 {
    let tables: toml::Table = TEST_ZEALOT.parse().unwrap();
    tables["calling"][0]["skill"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["key"].as_str() == Some(key))
        .and_then(|s| s["spell_id"].as_integer())
        .unwrap_or_else(|| panic!("{key} is a skill of the test Zealot")) as u32
}

/// The **Champion**, "test_champion": a Warrior-Class Calling in Tier 3 whose Tier Bonus wins any
/// fight a flow test takes on the way to what it checks (a quest's kill, a loot roll, a call for
/// help), however hard creatures get. A test of a fight's balance uses a shipped Calling instead,
/// so a balance change breaks only the tests about balance. Pass `CHAMPION_CALLINGS` to
/// `Realm::with_callings` and put a save on it with [`as_champion`].
pub const CHAMPION_CALLINGS: &str = r#"
[[calling]]
key = "test_champion"
name = "Test Champion"
tier = 3
class = 1
cap = 60
tier_bonus = { strength = 2000, agility = 0, stamina = 2000, intellect = 0, spirit = 0 }
shapeshift_form = 17
armour = ["cloth", "leather", "mail", "plate"]

[[calling.skill]]
key = "none"
spell_id = 2061
power_cost = 0
cooldown_ms = 0
gcd_ms = 0
target = "self"
effects = []
"#;

/// `blob`, a new warrior's save, on the Champion Calling.
pub fn as_champion(blob: &str) -> String {
    let champion = blob.replace("calling = \"basic_warrior\"", "calling = \"test_champion\"");
    assert_ne!(champion, blob, "a Basic Warrior's save");
    champion
}

pub use benilla_protocol::messages::{
    CHAR_CREATE_NAME_IN_USE, CHAR_CREATE_SERVER_LIMIT, CHAR_CREATE_SUCCESS, CHAR_DELETE_SUCCESS,
};
use benilla_protocol::{CharAction, CharCreateReq, Character, SessionEvent};
pub use twilight_realm::{Host, Realm, RealmCommand};

pub const CHAR_CREATE_DISABLED: u8 = 0x32;
pub const CHAR_DELETE_FAILED: u8 = 0x3A;
pub const CHAR_NAME_NO_NAME: u8 = 0x45;
pub const CHAR_NAME_TOO_SHORT: u8 = 0x46;
pub const CHAR_NAME_TOO_LONG: u8 = 0x47;
pub const CHAR_NAME_INVALID_CHARACTER: u8 = 0x48;

pub const HUMAN: u8 = 1;
pub const ORC: u8 = 2;
pub const TAUREN: u8 = 6;
pub const WARRIOR: u8 = 1;
pub const MAGE: u8 = 8;

pub const KALIMDOR: u32 = 1;
pub const THE_BARRENS: u32 = 17;

/// The master sheet's realm data (`server_spells.json`): a test names a sheet spell by its
/// Calling and key and reads its id here, never a number (ADR 0011).
fn sheet() -> &'static serde_json::Value {
    static SHEET: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    SHEET.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../data/twilights-eve/server_spells.json"
        ))
        .expect("server_spells.json parses")
    })
}

fn sheet_calling(calling: &str) -> &'static serde_json::Value {
    sheet()
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["key"] == calling)
        .unwrap_or_else(|| panic!("{calling} is not a Calling of the sheet"))
}

fn id_of(row: &serde_json::Value, what: &str) -> u32 {
    row["spell_id"]
        .as_u64()
        .unwrap_or_else(|| panic!("{what} has no spell id")) as u32
}

/// The spell id of `calling`'s skill or aura `key` on the sheet.
pub fn spell(calling: &str, key: &str) -> u32 {
    let c = sheet_calling(calling);
    let row = ["skills", "auras"]
        .iter()
        .flat_map(|list| c[*list].as_array().unwrap())
        .find(|r| r["key"] == key)
        .unwrap_or_else(|| panic!("{calling}/{key} is not a skill or aura of the sheet"));
    id_of(row, &format!("{calling}/{key}"))
}

/// The spell id of the Creature Spell `key` (`content/creature_spells.toml`, ADR 0011).
pub fn creature_spell(key: &str) -> u32 {
    static FILE: std::sync::LazyLock<toml::Value> = std::sync::LazyLock::new(|| {
        toml::from_str(include_str!("../../content/creature_spells.toml")).unwrap()
    });
    FILE["spell"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["key"].as_str() == Some(key))
        .unwrap_or_else(|| panic!("{key} is not a creature spell"))["spell_id"]
        .as_integer()
        .unwrap() as u32
}

/// The hand-written creature template `entry` of `content/mobs.toml`, or of a Phased Dungeon's
/// own file (`twilight_realm::phased_dungeon_sources`).
pub fn mob_template(entry: u32) -> &'static toml::Value {
    static FILES: std::sync::LazyLock<Vec<toml::Value>> = std::sync::LazyLock::new(|| {
        let dungeons = twilight_realm::phased_dungeon_sources()
            .iter()
            .map(|(_, t)| *t);
        std::iter::once(include_str!("../../content/mobs.toml"))
            .chain(dungeons)
            .map(|text| toml::from_str(text).unwrap())
            .collect()
    });
    FILES
        .iter()
        .filter_map(|f| f.get("template").and_then(|t| t.as_array()))
        .flatten()
        .find(|t| t["entry"].as_integer() == Some(entry.into()))
        .unwrap_or_else(|| panic!("{entry} is not a template of mobs.toml or a dungeon file"))
}

/// The Creature Spell keys template `entry` names, in order.
pub fn mob_spells(entry: u32) -> Vec<&'static str> {
    mob_template(entry)
        .get("spells")
        .and_then(|s| s.as_array())
        .map(|s| s.iter().map(|k| k.as_str().unwrap()).collect())
        .unwrap_or_default()
}

/// The spell ids of `calling`'s skills or auras `keys`, in order.
pub fn spells(calling: &str, keys: &[&str]) -> Vec<u32> {
    keys.iter().map(|key| spell(calling, key)).collect()
}

/// The spell id of the choice `key` of `calling`'s Talent Row for `skill` (a Skill Choice row's
/// skill key, or a Talent row's key) on the sheet.
pub fn choice(calling: &str, skill: &str, key: &str) -> u32 {
    let row = sheet_calling(calling)["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == skill)
        .unwrap_or_else(|| panic!("{calling}/{skill} is not a row of the sheet"));
    let choice = row["choices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["key"] == key)
        .unwrap_or_else(|| panic!("{calling}/{skill}/{key} is not a choice of the sheet"));
    id_of(choice, &format!("{calling}/{skill}/{key}"))
}

/// Flat ground at `z = 0` with nothing in the way, and no area anywhere: nothing to discover.
pub struct FlatGround;

impl Host for FlatGround {
    fn ground_height(&self, _map: u32, _x: f32, _y: f32, _z_hint: f32) -> Option<f32> {
        Some(0.0)
    }
    fn line_of_sight(&self, _map: u32, _from: [f32; 3], _to: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _map: u32, _x: f32, _y: f32) -> Option<u32> {
        None
    }
}

/// [`FlatGround`] with a pond: water at surface Z [`Pond::SURFACE`] within `radius` of `centre`,
/// and the whole map in area `area`.
pub struct Pond {
    pub centre: [f32; 2],
    pub radius: f32,
    pub area: Option<u32>,
}

impl Pond {
    /// The pond's surface height.
    pub const SURFACE: f32 = 0.5;
}

impl Host for Pond {
    fn ground_height(&self, _map: u32, _x: f32, _y: f32, _z_hint: f32) -> Option<f32> {
        Some(0.0)
    }
    fn line_of_sight(&self, _map: u32, _from: [f32; 3], _to: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _map: u32, _x: f32, _y: f32) -> Option<u32> {
        self.area
    }
    fn liquid_at(&self, _map: u32, x: f32, y: f32) -> Option<f32> {
        let [cx, cy] = self.centre;
        ((x - cx).hypot(y - cy) <= self.radius).then_some(Self::SURFACE)
    }
}

/// A flat floor at `z` everywhere and a wall between every two points: no creature ever sees the
/// character, so none turns on them. For a quest or dialog test inside a building: a flat host
/// lays all of a building's storeys on one plane, so with sight a Phased Dungeon's upper-floor
/// monsters stand beside the ground floor and join in (#314).
pub struct Unseen(pub f32);

impl Host for Unseen {
    fn ground_height(&self, _map: u32, _x: f32, _y: f32, _z_hint: f32) -> Option<f32> {
        Some(self.0)
    }
    fn line_of_sight(&self, _map: u32, _from: [f32; 3], _to: [f32; 3]) -> bool {
        false
    }
    fn area_at(&self, _map: u32, _x: f32, _y: f32) -> Option<u32> {
        None
    }
}

/// [`FlatGround`] with a scripted map of areas: each `(centre, radius, AreaTable id)`, the first
/// whose circle holds a point naming it.
pub struct Areas(pub Vec<([f32; 2], f32, u32)>);

impl Host for Areas {
    fn ground_height(&self, _map: u32, _x: f32, _y: f32, _z_hint: f32) -> Option<f32> {
        Some(0.0)
    }
    fn line_of_sight(&self, _map: u32, _from: [f32; 3], _to: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _map: u32, x: f32, y: f32) -> Option<u32> {
        self.0
            .iter()
            .find(|([cx, cy], r, _)| (x - cx).hypot(y - cy) <= *r)
            .map(|&(_, _, id)| id)
    }
}

pub fn realm() -> Realm {
    Realm::new()
}

pub fn drain(realm: &mut Realm) -> Vec<SessionEvent> {
    realm.drain_events()
}

pub fn create_cmd(name: &str, race: u8, gender: u8, class: u8) -> RealmCommand {
    RealmCommand::CreateCharacter(CharCreateReq {
        name: name.into(),
        race,
        class,
        gender,
        skin: 0,
        face: 0,
        hair_style: 0,
        hair_color: 0,
        facial_hair: 0,
        start: 0,
    })
}

/// The create result code in the events so far.
pub fn create_result(realm: &mut Realm) -> u8 {
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::CharActionResult {
                action: CharAction::Create,
                code,
            } => Some(code),
            _ => None,
        })
        .expect("a create result")
}

/// Creates a warrior and returns the result code.
pub fn create(realm: &mut Realm, name: &str, race: u8, gender: u8) -> u8 {
    realm.handle(create_cmd(name, race, gender, WARRIOR));
    create_result(realm)
}

/// Asks for the roster and returns it.
pub fn roster(realm: &mut Realm) -> Vec<Character> {
    realm.handle(RealmCommand::ListCharacters);
    drain(realm)
        .into_iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::CharacterList { characters, .. } => Some(characters),
            _ => None,
        })
        .expect("a roster")
}

/// Creates a warrior, enters the world with it, and returns its guid and the entry's events.
pub fn enter_new(realm: &mut Realm, name: &str, race: u8) -> (u64, Vec<SessionEvent>) {
    assert_eq!(create(realm, name, race, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(realm)
        .into_iter()
        .find(|c| c.name == name)
        .expect("created")
        .guid;
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    (guid, drain(realm))
}

fn hub_rows() -> Vec<toml::Value> {
    let hub: toml::Table = include_str!("../../content/hub.toml").parse().unwrap();
    hub["npc"].as_array().unwrap().clone()
}

/// The object guid of the Hub NPC with creature `entry` (`HIGHGUID_UNIT`, entry, spawn).
pub fn hub_guid(entry: u32) -> u64 {
    let row = hub_rows()
        .into_iter()
        .find(|n| n["entry"].as_integer() == Some(i64::from(entry)))
        .unwrap_or_else(|| panic!("{entry} is a Hub NPC"));
    0xF130 << 48 | u64::from(entry) << 24 | row["guid"].as_integer().unwrap() as u64
}

/// A Hub NPC's home in `content/hub.toml`, by its object guid.
pub fn hub_home(npc: u64) -> [f32; 3] {
    let (entry, low) = ((npc >> 24) & 0xFF_FFFF, npc & 0xFF_FFFF);
    let row = hub_rows()
        .into_iter()
        .find(|n| {
            n["entry"].as_integer() == Some(entry as i64)
                && n["guid"].as_integer() == Some(low as i64)
        })
        .unwrap_or_else(|| panic!("{npc:#x} is a Hub NPC"));
    let p = row["position"].as_array().unwrap();
    std::array::from_fn(|i| p[i].as_float().unwrap() as f32)
}

/// Walks the player in the world up to the Hub NPC `npc`, near enough to deal with it.
pub fn walk_up_to(realm: &mut Realm, npc: u64) {
    let [x, y, _] = hub_home(npc);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: [x + 1.5, y, 0.0],
        orientation: std::f32::consts::PI,
    });
}
