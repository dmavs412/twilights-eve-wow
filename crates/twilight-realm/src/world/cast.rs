//! The character's skills in the world: the spellbook at login, casting them, and the auras they
//! leave. Every skill is cast from its data ([`crate::callings`]); nothing here names a skill.

use benilla_protocol::messages::ActionButton;
use benilla_protocol::SessionEvent;

use super::{InWorld, Ticked};
use crate::callings;
use crate::characters::Char;
use crate::rng::Rng;

/// `UNIT_FIELD_BYTES_1`: stand state, loyalty, shapeshift form, visibility flags.
const UNIT_FIELD_BYTES_1: u16 = 138;

/// `SMSG_INITIAL_SPELLS`: the current Calling's skills, with nothing cooling down.
pub(super) fn spell_book(c: &Char) -> SessionEvent {
    SessionEvent::SpellBook {
        spell_ids: callings::of(c.calling)
            .skills
            .iter()
            .map(|s| s.spell_id)
            .collect(),
        cooldowns: Vec::new(),
    }
}

/// `SMSG_ACTION_BUTTONS`: the bar as the player left it, or the Calling's default.
pub(super) fn action_buttons(c: &Char) -> SessionEvent {
    let bar = c
        .action_bar
        .clone()
        .unwrap_or_else(|| callings::default_bar(c.calling));
    SessionEvent::ActionButtons {
        buttons: bar
            .into_iter()
            .map(|(slot, packed)| ActionButton {
                slot,
                action: packed & 0x00FF_FFFF,
                kind: (packed >> 24) as u8,
            })
            .collect(),
    }
}

/// The Calling's stance in `UNIT_FIELD_BYTES_1` byte 2: the client checks a skill's stances
/// against it before it sends the cast.
pub(super) fn stance(c: &Char) -> (u16, u32) {
    (
        UNIT_FIELD_BYTES_1,
        u32::from(callings::of(c.calling).shapeshift_form) << 16,
    )
}

impl InWorld {
    /// `CMSG_CAST_SPELL`.
    pub(crate) fn cast(
        &mut self,
        _spell_id: u32,
        _target: Option<u64>,
        _c: &mut Char,
        _rng: &mut Rng,
        _events: &mut Vec<SessionEvent>,
    ) -> Ticked {
        Ticked::default()
    }
}
