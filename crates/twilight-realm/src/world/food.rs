//! Food and drink (spec #1, "Gear and economy"): used from the bags out of combat, the character
//! sits under the item's spell and is restored evenly over its duration, every two seconds as
//! vanilla's regeneration ticks (vmangos `SPELL_AURA_MOD_REGEN`, `Player::RegenerateHealth`).
//! Standing up, a hit or a fight ends the meal. A Warrior has no mana, so drink restores nothing.

use benilla_protocol::SessionEvent;

use super::InWorld;
use crate::characters::Char;
use crate::equipment;
use crate::fields::*;
use crate::items;

/// The buff slot the meal's aura shows in: the last one, clear of the skills' auras.
const AURA_SLOT: u16 = 31;
const UNIT_FIELD_AURA: u16 = 47;
const UNIT_FIELD_AURAFLAGS: u16 = 95;
const UNIT_FIELD_AURALEVELS: u16 = 101;
/// `AFLAG_CANCELABLE | AFLAG_EFF_INDEX_0`.
const AURA_FLAGS: u32 = 0x03;
/// `UNIT_FIELD_BYTES_1`: its low byte is the stand state, 1 sitting.
const UNIT_FIELD_BYTES_1: u16 = 138;
const UNIT_STAND_STATE_SIT: u32 = 1;
/// The regeneration tick.
const TICK_SECS: f32 = 2.0;
/// `SPELL_FAILED_AFFECTING_COMBAT`.
const AFFECTING_COMBAT: u8 = 0x00;
/// `EQUIP_ERR_CANT_EQUIP_LEVEL_I`.
const CANT_USE_LEVEL: u8 = 1;
/// `BAG_PLAYER_INVENTORY`.
const PLAYER_BAG: u8 = 255;

/// A meal in progress.
pub(super) struct Meal {
    /// Health restored over the whole meal, and so far.
    total: u32,
    restored: u32,
    duration: f32,
    elapsed: f32,
    /// Seconds into the current regeneration tick.
    tick: f32,
    /// Where the character sat down: a step away stands them up.
    at: [f32; 3],
}

impl InWorld {
    /// `CMSG_USE_ITEM` on food or drink at a wire position.
    pub(crate) fn use_item(
        &mut self,
        bag: u8,
        slot: u8,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        if bag != PLAYER_BAG || self.dead() {
            return false;
        }
        let Some(item) = c.inventory.get(slot) else {
            return false;
        };
        let Some(t) = items::template(item.entry).filter(|t| t.use_spell != 0) else {
            return false;
        };
        if c.level < t.required_level {
            events.push(SessionEvent::InventoryFailure {
                reason: CANT_USE_LEVEL,
                required_level: Some(u32::from(t.required_level)),
                item_guid: item.guid,
                bag_slot: 0,
            });
            return false;
        }
        if self.player.in_combat {
            events.push(SessionEvent::CastResult {
                spell_id: t.use_spell,
                success: false,
                reason: Some(AFFECTING_COMBAT),
                arg: None,
            });
            return false;
        }
        let item_guid = item.guid;
        // One of the stack is eaten.
        if item.count > 1 {
            if let Some(item) = c.inventory.get_mut(slot) {
                item.count -= 1;
                events.push(values(
                    item.guid,
                    &[(equipment::ITEM_FIELD_STACK_COUNT, item.count)],
                ));
            }
        } else {
            c.inventory.take(slot);
            equipment::send_slots(c, &[slot], events);
            events.push(SessionEvent::ObjectDestroyed(item_guid));
        }
        self.stop_eating(events);
        events.push(SessionEvent::CastResult {
            spell_id: t.use_spell,
            success: true,
            reason: None,
            arg: None,
        });
        events.push(SessionEvent::SpellGo {
            caster: self.guid,
            spell_id: t.use_spell,
            cast_flags: 0,
            hits: vec![self.guid],
            misses: Vec::new(),
            target: Some(self.guid),
            go_target: None,
            dest: None,
            ammo_display_id: None,
            item_caster: Some(item_guid),
        });
        let duration = t.duration_ms as f32 / 1000.0;
        let mut pairs = vec![
            (UNIT_FIELD_AURA + AURA_SLOT, t.use_spell),
            (
                UNIT_FIELD_AURAFLAGS + AURA_SLOT / 8,
                AURA_FLAGS << (4 * (AURA_SLOT % 8)),
            ),
            (
                UNIT_FIELD_AURALEVELS + AURA_SLOT / 4,
                u32::from(c.level) << (8 * (AURA_SLOT % 4)),
            ),
        ];
        pairs.push((UNIT_FIELD_BYTES_1, UNIT_STAND_STATE_SIT));
        events.push(values(self.guid, &pairs));
        events.push(SessionEvent::AuraDuration {
            slot: AURA_SLOT as u8,
            remaining_ms: t.duration_ms,
        });
        self.meal = Some(Meal {
            total: t.restores_health,
            restored: 0,
            duration,
            elapsed: 0.0,
            tick: 0.0,
            at: c.position,
        });
        true
    }

    /// The meal goes on by `dt`: each regeneration tick restores its share, and the end the rest.
    pub(super) fn eat(&mut self, dt: f32, c: &Char, events: &mut Vec<SessionEvent>) {
        let Some(meal) = &mut self.meal else {
            return;
        };
        if c.position != meal.at || self.player.in_combat || !self.player.alive() {
            self.stop_eating(events);
            return;
        }
        meal.elapsed = (meal.elapsed + dt).min(meal.duration);
        meal.tick += dt;
        let done = meal.elapsed >= meal.duration;
        if meal.tick < TICK_SECS && !done {
            return;
        }
        meal.tick -= TICK_SECS;
        let due = (meal.total as f32 * meal.elapsed / meal.duration) as u32;
        let heal = due.saturating_sub(meal.restored);
        meal.restored = due;
        let p = &mut self.player;
        if heal > 0 && p.health < p.stats.max_health {
            p.health = (p.health + heal).min(p.stats.max_health);
            events.push(values(self.guid, &[(UNIT_FIELD_HEALTH, p.health)]));
        }
        if done {
            self.stop_eating(events);
        }
    }

    /// The meal ends: the aura goes and the character stands.
    pub(super) fn stop_eating(&mut self, events: &mut Vec<SessionEvent>) {
        if self.meal.take().is_none() {
            return;
        }
        events.push(values(
            self.guid,
            &[
                (UNIT_FIELD_AURA + AURA_SLOT, 0),
                (UNIT_FIELD_AURAFLAGS + AURA_SLOT / 8, 0),
                (UNIT_FIELD_AURALEVELS + AURA_SLOT / 4, 0),
                (UNIT_FIELD_BYTES_1, 0),
            ],
        ));
    }
}
