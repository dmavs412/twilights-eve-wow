//! A worn trinket's click (spec #458): its Trinket Spell goes off at once, off the global
//! cooldown, and the item cools down for the spell's `cooldown_ms`, kept in the save. Putting one
//! on starts vanilla's 30 s equip wait.

use benilla_protocol::SessionEvent;

use super::cast::{
    refused, CAST_FLAG_UNKNOWN2, READY_SLACK_SECS, SPELL_FAILED_CASTER_DEAD,
    SPELL_FAILED_NOT_READY, SPELL_FAILED_SPELL_IN_PROGRESS,
};
use super::InWorld;
use crate::characters::Char;
use crate::inventory::Pos;
use crate::trinket_spells::{self, TrinketSpell};
use crate::{clock, items};

/// The trinket slots, as `CMSG_USE_ITEM` names them under the player's own bag.
const TRINKET_SLOTS: [u8; 2] = [12, 13];
/// Seconds a trinket waits after it goes on (vmangos `Player::EquipItem`, the client's fixed
/// `SMSG_ITEM_COOLDOWN`).
const EQUIP_WAIT_SECS: f64 = 30.0;

/// The Trinket Spell of the item at `at`, and the item's guid and entry.
fn worn_trinket(c: &Char, at: Pos) -> Option<(&'static TrinketSpell, u64, u32)> {
    let item = c.inventory.get(at)?;
    let spell = trinket_spells::spell(items::template(item.entry)?.use_spell)?;
    Some((spell, item.guid, item.entry))
}

/// Whether the item at `slot` has a Trinket Spell: its click goes to [`InWorld::use_trinket`].
pub(crate) fn is_trinket(c: &Char, slot: Pos) -> bool {
    c.inventory
        .get(slot)
        .and_then(|i| items::template(i.entry))
        .is_some_and(|t| trinket_spells::spell(t.use_spell).is_some())
}

/// Seconds left on `entry`'s cooldown at `wall`, 0 when ready.
fn cooling(c: &Char, entry: u32, wall: f64) -> f64 {
    c.item_cooldowns
        .iter()
        .filter(|&&(e, _)| e == entry)
        .map(|&(_, ready)| ready - wall)
        .fold(0.0, f64::max)
}

/// Puts `entry` on cooldown until `ready`.
fn cool_until(c: &mut Char, entry: u32, ready: f64) {
    c.item_cooldowns.retain(|&(e, _)| e != entry);
    c.item_cooldowns.push((entry, ready));
}

impl InWorld {
    /// `CMSG_USE_ITEM` on an item with a Trinket Spell. Only a worn one goes off: one in a bag
    /// does nothing, as the client equips it on a right-click. A refusal gives the stock reason.
    pub(crate) fn use_trinket(
        &mut self,
        slot: Pos,
        c: &mut Char,
        wall: f64,
        events: &mut Vec<SessionEvent>,
    ) {
        if !slot.is_worn() || !TRINKET_SLOTS.contains(&slot.slot) {
            return;
        }
        let Some((spell, guid, entry)) = worn_trinket(c, slot) else {
            return;
        };
        let reason = if !self.player.alive() {
            Some(SPELL_FAILED_CASTER_DEAD)
        } else if self.skills.casting() {
            Some(SPELL_FAILED_SPELL_IN_PROGRESS)
        } else if cooling(c, entry, wall) > f64::from(READY_SLACK_SECS) {
            Some(SPELL_FAILED_NOT_READY)
        } else {
            None
        };
        if let Some(reason) = reason {
            events.push(refused(spell.spell_id, reason));
            return;
        }
        events.push(SessionEvent::SpellGo {
            caster: self.guid,
            spell_id: spell.spell_id,
            cast_flags: CAST_FLAG_UNKNOWN2,
            hits: vec![self.guid],
            misses: Vec::new(),
            target: None,
            go_target: None,
            dest: None,
            ammo_display_id: None,
            item_caster: Some(guid),
        });
        cool_until(c, entry, wall + f64::from(clock::secs(spell.cooldown_ms)));
        if let Some(effect) = spell.effect {
            match effect {}
        }
    }
}

/// The equip wait: each trinket with a Trinket Spell that `changed` put in a trinket slot waits
/// [`EQUIP_WAIT_SECS`], unless its own cooldown has longer to run. The client hears of the wait
/// only when it is the longer one.
pub(crate) fn equip_wait(c: &mut Char, changed: &[Pos], wall: f64, events: &mut Vec<SessionEvent>) {
    for at in changed {
        if !at.is_worn() || !TRINKET_SLOTS.contains(&at.slot) {
            continue;
        }
        let Some((spell, guid, entry)) = worn_trinket(c, *at) else {
            continue;
        };
        if cooling(c, entry, wall) >= EQUIP_WAIT_SECS {
            continue;
        }
        cool_until(c, entry, wall + EQUIP_WAIT_SECS);
        events.push(SessionEvent::ItemCooldown {
            item_guid: guid,
            spell_id: spell.spell_id,
        });
    }
}

/// `SMSG_SPELL_COOLDOWN` at login for the Trinket Spells still cooling at `wall`, whose ids do
/// not fit the spell book's `u16` cooldowns; `None` when none is.
pub(crate) fn login_cooldowns(c: &Char, wall: f64) -> Option<SessionEvent> {
    let cooldowns: Vec<(u32, u32)> = c
        .item_cooldowns
        .iter()
        .filter(|&&(_, ready)| ready > wall)
        .filter_map(|&(entry, ready)| {
            let spell = trinket_spells::spell(items::template(entry)?.use_spell)?;
            Some((spell.spell_id, clock::ms((ready - wall) as f32)))
        })
        .collect();
    (!cooldowns.is_empty()).then_some(SessionEvent::SpellCooldowns {
        caster: c.guid,
        cooldowns,
    })
}
