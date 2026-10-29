//! **Advancement** (`GLOSSARY.md`), in the world: the character becomes level 1 of the Calling it
//! advanced into. Its skills are replaced by the new Calling's and its Talent Rows are cleared, and
//! that Calling's Tier Bonus lifts its base stats, and its proficiency is sent again. Gear, bags,
//! gold and every quest, done or under way, stay.

use benilla_protocol::SessionEvent;

use super::power::Power;
use super::{cast, sheet_pairs, InWorld};
use crate::callings;
use crate::characters::Char;
use crate::content::Calling;
use crate::fields::*;
use crate::rules::xp;

impl InWorld {
    /// `c` advances into `to`, and the client is told everything that changed.
    pub(crate) fn advance(&mut self, c: &mut Char, to: Calling, events: &mut Vec<SessionEvent>) {
        // The old Calling's spells first, while it is still current: its skills and set choices.
        let old: Vec<u32> = callings::of(c.calling)
            .skills
            .iter()
            .map(|s| s.spell_id)
            .chain(crate::talents::known_spells(c))
            .collect();
        for spell_id in old {
            events.push(SessionEvent::SpellRemoved { spell_id });
        }
        c.rows.clear();
        c.calling = to;
        events.push(SessionEvent::Callings {
            by_guid: vec![(c.guid, callings::of(to).key.clone())],
        });
        c.level = 1;
        c.xp = 0;
        c.notoriety = crate::rules::notoriety::advanced(c.notoriety);
        c.action_bar = None;
        for s in callings::of(c.calling)
            .skills
            .iter()
            .filter(|s| s.learn_level <= 1)
        {
            events.push(SessionEvent::SpellLearned {
                spell_id: s.spell_id,
            });
        }
        events.extend(crate::talents::spell_mods(c));
        events.push(crate::talents::rows_event(c, false));
        // The new Calling's armour, for the client's equip checks and tooltips.
        crate::equipment::proficiency(c, events);
        events.push(cast::action_buttons(c));
        // An old skill queued on the next swing goes with it.
        self.skills.take_queued();

        let p = &mut self.player;
        p.stats = crate::talents::stats(c);
        p.health = p.stats.max_health;
        // The new Calling's bar (rage empty, the rest full) and its profile's class byte.
        let def = callings::of(c.calling);
        p.power = Power::fresh(&p.stats, def.power, def.energy_regen());
        let mut pairs = vec![
            (PLAYER_XP, c.xp),
            (PLAYER_NEXT_LEVEL_XP, xp::xp_to_next(c.level).unwrap_or(0)),
            (UNIT_FIELD_HEALTH, p.health),
            p.power.field(),
            (
                UNIT_FIELD_BYTES_0,
                u32::from(c.race)
                    | u32::from(c.class()) << 8
                    | u32::from(c.gender) << 16
                    | p.power.kind.id() << 24,
            ),
            cast::stance(c),
        ];
        pairs.extend(sheet_pairs(c, &cast::white(&p.stats, &self.skills)));
        events.push(values(self.guid, &pairs));
    }
}
