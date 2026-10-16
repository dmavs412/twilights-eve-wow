//! The 1.12.1 update-field indices the realm writes (`UpdateFields_1_12_1.h`), in dwords.

use benilla_protocol::{ObjectFields, SessionEvent};

pub(crate) const OBJECT_FIELD_GUID: u16 = 0;
pub(crate) const OBJECT_FIELD_TYPE: u16 = 2;
pub(crate) const OBJECT_FIELD_ENTRY: u16 = 3;
pub(crate) const OBJECT_FIELD_SCALE_X: u16 = 4;

pub(crate) const UNIT_FIELD_TARGET: u16 = 16;
pub(crate) const UNIT_FIELD_HEALTH: u16 = 22;
pub(crate) const UNIT_FIELD_POWER2: u16 = 24;
pub(crate) const UNIT_FIELD_MAXHEALTH: u16 = 28;
pub(crate) const UNIT_FIELD_MAXPOWER2: u16 = 30;
pub(crate) const UNIT_FIELD_LEVEL: u16 = 34;
pub(crate) const UNIT_FIELD_FACTIONTEMPLATE: u16 = 35;
pub(crate) const UNIT_FIELD_BYTES_0: u16 = 36;
pub(crate) const UNIT_FIELD_FLAGS: u16 = 46;
pub(crate) const UNIT_FIELD_BASEATTACKTIME: u16 = 126;
pub(crate) const UNIT_FIELD_BOUNDINGRADIUS: u16 = 129;
pub(crate) const UNIT_FIELD_COMBATREACH: u16 = 130;
pub(crate) const UNIT_FIELD_DISPLAYID: u16 = 131;
pub(crate) const UNIT_FIELD_NATIVEDISPLAYID: u16 = 132;
pub(crate) const UNIT_FIELD_MINDAMAGE: u16 = 134;
pub(crate) const UNIT_FIELD_MAXDAMAGE: u16 = 135;
pub(crate) const UNIT_FIELD_STAT0: u16 = 150;
/// Seven schools; `[0]` is armour.
pub(crate) const UNIT_FIELD_RESISTANCES: u16 = 155;
pub(crate) const UNIT_FIELD_BASE_HEALTH: u16 = 163;
pub(crate) const UNIT_FIELD_ATTACK_POWER: u16 = 165;

pub(crate) const PLAYER_BYTES: u16 = 193;
pub(crate) const PLAYER_BYTES_2: u16 = 194;
pub(crate) const PLAYER_BYTES_3: u16 = 195;
pub(crate) const PLAYER_XP: u16 = 716;
pub(crate) const PLAYER_NEXT_LEVEL_XP: u16 = 717;
/// 128 skills × `[id | step << 16, value | max << 16, bonus]`.
pub(crate) const PLAYER_SKILL_INFO_1_1: u16 = 718;
pub(crate) const PLAYER_BLOCK_PERCENTAGE: u16 = 1106;
pub(crate) const PLAYER_DODGE_PERCENTAGE: u16 = 1107;
pub(crate) const PLAYER_PARRY_PERCENTAGE: u16 = 1108;
pub(crate) const PLAYER_CRIT_PERCENTAGE: u16 = 1109;
/// 64 dwords of explore bits, bit `n` the `AreaTable.dbc` row whose explore flag is `n`.
pub(crate) const PLAYER_EXPLORED_ZONES_1: u16 = 1111;
/// Seven schools of float multipliers, 1.0 for none.
pub(crate) const PLAYER_FIELD_MOD_DAMAGE_DONE_PCT: u16 = 1215;
pub(crate) const PLAYER_FIELD_WATCHED_FACTION_INDEX: u16 = 1261;

/// `TYPEMASK_OBJECT | TYPEMASK_UNIT | TYPEMASK_PLAYER`.
pub(crate) const TYPE_PLAYER: u32 = 0x01 | 0x08 | 0x10;
/// `TYPEMASK_OBJECT | TYPEMASK_UNIT`.
pub(crate) const TYPE_UNIT: u32 = 0x01 | 0x08;
/// `POWER_RAGE`.
pub(crate) const POWER_RAGE: u32 = 1;
/// `UNIT_FLAG_IN_COMBAT`.
pub(crate) const UNIT_FLAG_IN_COMBAT: u32 = 0x0008_0000;
/// `UNIT_FLAG_PVP_ATTACKABLE`: player-controlled. Every player carries it; without it the
/// client's `CanAttack` lets the player attack hostile mobs only, never neutral ones.
pub(crate) const UNIT_FLAG_PVP_ATTACKABLE: u32 = 0x8;

/// The two dwords of a guid field.
pub(crate) fn guid_pairs(field: u16, guid: u64) -> [(u16, u32); 2] {
    [(field, guid as u32), (field + 1, (guid >> 32) as u32)]
}

/// A float field's dword.
pub(crate) fn float(v: f32) -> u32 {
    v.to_bits()
}

/// A values update carrying just these fields.
pub(crate) fn values(guid: u64, pairs: &[(u16, u32)]) -> SessionEvent {
    SessionEvent::ObjectValues {
        guid,
        fields: ObjectFields::from_pairs(pairs),
    }
}
