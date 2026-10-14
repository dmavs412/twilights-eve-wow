//! The 1.12.1 update-field indices the realm writes (`UpdateFields_1_12_1.h`), in dwords.

pub(crate) const OBJECT_FIELD_GUID: u16 = 0;
pub(crate) const OBJECT_FIELD_TYPE: u16 = 2;
pub(crate) const OBJECT_FIELD_SCALE_X: u16 = 4;

pub(crate) const UNIT_FIELD_HEALTH: u16 = 22;
pub(crate) const UNIT_FIELD_POWER2: u16 = 24;
pub(crate) const UNIT_FIELD_MAXHEALTH: u16 = 28;
pub(crate) const UNIT_FIELD_MAXPOWER2: u16 = 30;
pub(crate) const UNIT_FIELD_LEVEL: u16 = 34;
pub(crate) const UNIT_FIELD_FACTIONTEMPLATE: u16 = 35;
pub(crate) const UNIT_FIELD_BYTES_0: u16 = 36;
pub(crate) const UNIT_FIELD_BASEATTACKTIME: u16 = 126;
pub(crate) const UNIT_FIELD_BOUNDINGRADIUS: u16 = 129;
pub(crate) const UNIT_FIELD_COMBATREACH: u16 = 130;
pub(crate) const UNIT_FIELD_DISPLAYID: u16 = 131;
pub(crate) const UNIT_FIELD_NATIVEDISPLAYID: u16 = 132;
pub(crate) const UNIT_FIELD_MINDAMAGE: u16 = 134;
pub(crate) const UNIT_FIELD_MAXDAMAGE: u16 = 135;
pub(crate) const UNIT_FIELD_STAT0: u16 = 150;
pub(crate) const UNIT_FIELD_BASE_HEALTH: u16 = 163;

pub(crate) const PLAYER_BYTES: u16 = 193;
pub(crate) const PLAYER_BYTES_2: u16 = 194;
pub(crate) const PLAYER_BYTES_3: u16 = 195;
pub(crate) const PLAYER_XP: u16 = 716;
pub(crate) const PLAYER_NEXT_LEVEL_XP: u16 = 717;
pub(crate) const PLAYER_FIELD_WATCHED_FACTION_INDEX: u16 = 1261;

/// `TYPEMASK_OBJECT | TYPEMASK_UNIT | TYPEMASK_PLAYER`.
pub(crate) const TYPE_PLAYER: u32 = 0x01 | 0x08 | 0x10;
/// `POWER_RAGE`.
pub(crate) const POWER_RAGE: u32 = 1;
