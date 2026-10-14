//! The character in the world, and the login burst that puts it there.

use benilla_protocol::messages::ObjectType;
use benilla_protocol::{EntityKind, MoveSpeeds, ObjectFields, SessionEvent};

use crate::characters::Char;
use crate::content;
use crate::fields::*;

/// vanilla's player speeds (`baseMoveSpeed`): yd/s, turn in rad/s.
const PLAYER_SPEEDS: MoveSpeeds = MoveSpeeds {
    walk: 2.5,
    run: 7.0,
    run_back: 4.5,
    swim: 4.722_222,
    swim_back: 2.5,
    turn_rate: std::f32::consts::PI,
};

/// XP to level 2 (`player_xp_for_level`).
const XP_TO_LEVEL_2: u32 = 400;

/// The session of the character that is in the world.
pub(crate) struct InWorld {
    pub(crate) guid: u64,
    /// Seconds since the last autosave.
    pub(crate) since_save: f32,
}

impl InWorld {
    /// Logs `c` in: what vmangos sends from `CMSG_PLAYER_LOGIN` to the player's own create.
    pub(crate) fn enter(c: &Char, events: &mut Vec<SessionEvent>) -> Self {
        events.push(SessionEvent::Connected {
            self_guid: c.guid,
            name: c.name.clone(),
            billing_time_rested: 0,
            // Every tutorial already seen: a Local Realm has no new-player script to follow.
            tutorial_flags: Some(vec![0xff; 32]),
            // A reply that hid nothing.
            addon_info: Some(Vec::new()),
        });
        events.push(SessionEvent::Worldport {
            map_id: c.map,
            position: c.position,
            orientation: c.orientation,
            needs_ack: false,
        });
        // Mid-afternoon at the game clock's real-time pace (one game minute a minute).
        events.push(SessionEvent::TimeSpeed {
            hours: 15,
            minutes: 0,
            day_serial: 0,
            timescale: 1.0 / 60.0,
        });
        events.push(SessionEvent::ObjectCreate {
            guid: c.guid,
            kind: EntityKind::Player,
            display_id: Some(content::display_id(c.race, c.gender)),
            position: c.position,
            orientation: c.orientation,
            scale: 1.0,
            speeds: Some(PLAYER_SPEEDS),
            mover: None,
            transport_progress: None,
            transport: None,
            spline: None,
            fields: player_fields(c),
        });
        Self {
            guid: c.guid,
            since_save: 0.0,
        }
    }
}

/// The player's own descriptor at login.
fn player_fields(c: &Char) -> ObjectFields {
    let display = content::display_id(c.race, c.gender);
    let stats = content::base_stats(c.race);
    // vanilla: the first 20 stamina give 1 health each, the rest 10.
    let sta = stats[2];
    let health = content::BASE_HEALTH + sta.min(20) + sta.saturating_sub(20) * 10;
    let [skin, face, hair_style, hair_color, facial_hair] = c.look.map(u32::from);
    let mut pairs = vec![
        (OBJECT_FIELD_GUID, c.guid as u32),
        (OBJECT_FIELD_GUID + 1, (c.guid >> 32) as u32),
        (OBJECT_FIELD_TYPE, TYPE_PLAYER),
        (OBJECT_FIELD_SCALE_X, 1.0f32.to_bits()),
        (UNIT_FIELD_HEALTH, health),
        (UNIT_FIELD_MAXHEALTH, health),
        (UNIT_FIELD_POWER2, 0),
        (UNIT_FIELD_MAXPOWER2, 1000),
        (UNIT_FIELD_LEVEL, u32::from(c.level)),
        (
            UNIT_FIELD_FACTIONTEMPLATE,
            content::faction_template(c.race),
        ),
        (
            UNIT_FIELD_BYTES_0,
            u32::from(c.race)
                | u32::from(benilla_protocol::messages::CLASS_WARRIOR) << 8
                | u32::from(c.gender) << 16
                | POWER_RAGE << 24,
        ),
        (UNIT_FIELD_BASEATTACKTIME, 2000),
        (UNIT_FIELD_BASEATTACKTIME + 1, 2000),
        (UNIT_FIELD_BOUNDINGRADIUS, 0.389f32.to_bits()),
        (UNIT_FIELD_COMBATREACH, 1.5f32.to_bits()),
        (UNIT_FIELD_DISPLAYID, display),
        (UNIT_FIELD_NATIVEDISPLAYID, display),
        (UNIT_FIELD_MINDAMAGE, 1.0f32.to_bits()),
        (UNIT_FIELD_MAXDAMAGE, 2.0f32.to_bits()),
        (UNIT_FIELD_BASE_HEALTH, content::BASE_HEALTH),
        (PLAYER_BYTES, skin | face << 8 | hair_style << 16 | hair_color << 24),
        // Facial hair, then `REST_STATE_NORMAL` in the top byte.
        (PLAYER_BYTES_2, facial_hair | 0x01 << 24),
        (PLAYER_BYTES_3, u32::from(c.gender)),
        (PLAYER_XP, 0),
        (PLAYER_NEXT_LEVEL_XP, XP_TO_LEVEL_2),
        (PLAYER_FIELD_WATCHED_FACTION_INDEX, u32::MAX),
    ];
    for (i, stat) in stats.into_iter().enumerate() {
        pairs.push((UNIT_FIELD_STAT0 + i as u16, stat));
    }
    ObjectFields::from_pairs(&pairs).into_created(ObjectType::Player)
}
