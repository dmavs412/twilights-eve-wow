//! Shared helpers: a realm on flat ground, and the steps a player script repeats.
#![allow(dead_code, unused_imports)]

pub mod fight;

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

/// Flat ground at `z = 0` with nothing in the way.
pub struct FlatGround;

impl Host for FlatGround {
    fn ground_height(&self, _map: u32, _x: f32, _y: f32, _z_hint: f32) -> Option<f32> {
        Some(0.0)
    }
    fn line_of_sight(&self, _map: u32, _from: [f32; 3], _to: [f32; 3]) -> bool {
        true
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
