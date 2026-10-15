//! The character's skills in the world: the spellbook at login, casting them, and the auras they
//! leave. Every skill is cast from its data ([`crate::callings`]); nothing here names a skill.

use benilla_protocol::SessionEvent;

use crate::callings;
use crate::characters::Char;

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
