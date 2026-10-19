//! **Notoriety** in the world (`GLOSSARY.md`, spec #175): achievements raise it, the Hub drains
//! it, and the character wears their Heat Level as an aura.

use benilla_protocol::SessionEvent;

use super::InWorld;
use crate::characters::Char;
use crate::rules::notoriety;

impl InWorld {
    /// `c` earned `gain` Notoriety.
    pub(crate) fn notorious(&mut self, c: &mut Char, gain: f32) {
        c.notoriety += gain.max(0.0);
    }

    /// The Hub's drain while the player rests there alive and out of combat, and the Heat aura
    /// brought in line with the Heat Level.
    pub(super) fn notoriety_tick(&mut self, dt: f32, c: &mut Char, events: &mut Vec<SessionEvent>) {
        if self.player.alive() && !self.player.in_combat && notoriety::in_hub(self.map, c.position)
        {
            c.notoriety = notoriety::drained(c.notoriety, dt);
        }
        self.show_heat(c, events);
    }

    /// Puts up the aura of `c`'s Heat Level, taking down the one it replaces.
    fn show_heat(&mut self, c: &Char, events: &mut Vec<SessionEvent>) {
        let level = notoriety::heat_level(c.notoriety);
        if level == self.heat_shown {
            return;
        }
        if let Some(old) = notoriety::heat_aura(self.heat_shown) {
            self.hide_marker(old, events);
        }
        if let Some(new) = notoriety::heat_aura(level) {
            self.show_marker(new, true, events);
        }
        self.heat_shown = level;
    }
}
