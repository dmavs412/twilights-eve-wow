//! Food and drink (spec #1, "Gear and economy"): used from the bags out of combat, the character
//! sits under the item's spell and is restored evenly over its duration, every two seconds as
//! vanilla's regeneration ticks (vmangos `SPELL_AURA_MOD_REGEN`, `Player::RegenerateHealth`).
//! Standing up, a hit or a fight ends the meal. A Warrior has no mana, so drink restores nothing.
//! A potion (no duration) restores at once, in a fight or out, and then cools down.

use benilla_protocol::SessionEvent;

use super::InWorld;
use crate::characters::Char;
use crate::clock;
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
    /// `CMSG_USE_ITEM` on the item in player slot `slot`, if it is food, drink or a potion:
    /// `None` for any other item, else whether one was used up.
    pub(crate) fn eat_or_drink(
        &mut self,
        slot: crate::inventory::Pos,
        c: &mut Char,
        wall: f64,
        events: &mut Vec<SessionEvent>,
    ) -> Option<bool> {
        let item = c.inventory.get(slot)?;
        let t = items::template(item.entry)
            .filter(|t| t.use_spell != 0 && (t.restores_health > 0 || t.restores_mana > 0))?;
        if t.duration_ms == 0 {
            return Some(self.quaff(slot, t, c, wall, events));
        }
        Some(self.sit_and_eat(slot, t, c, events))
    }

    /// A potion: one of the stack restores its health at once, and the item cools down.
    fn quaff(
        &mut self,
        slot: crate::inventory::Pos,
        t: &items::Template,
        c: &mut Char,
        wall: f64,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        if self.dead() {
            return false;
        }
        let cooling = c.item_cooldowns.iter().any(|&(e, ready)| {
            e == t.entry && ready - wall > f64::from(super::cast::READY_SLACK_SECS)
        });
        if cooling {
            events.push(SessionEvent::CastResult {
                spell_id: t.use_spell,
                success: false,
                reason: Some(super::cast::SPELL_FAILED_NOT_READY),
                arg: None,
            });
            return false;
        }
        let Some(item_guid) = use_up_one(slot, c, events) else {
            return false;
        };
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
        if t.use_cooldown_ms > 0 {
            c.item_cooldowns.retain(|&(e, _)| e != t.entry);
            let rest = f64::from(clock::secs(t.use_cooldown_ms as u32));
            c.item_cooldowns.push((t.entry, wall + rest));
        }
        let me = self.guid;
        self.heal_from(me, c, t.use_spell, t.restores_health, events);
        true
    }

    fn sit_and_eat(
        &mut self,
        slot: crate::inventory::Pos,
        t: &items::Template,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        if self.dead() {
            return false;
        }
        if c.inventory.get(slot).is_none() {
            return false;
        }
        // Any level eats anything: the level resets with each Tier.
        if self.player.in_combat {
            events.push(SessionEvent::CastResult {
                spell_id: t.use_spell,
                success: false,
                reason: Some(AFFECTING_COMBAT),
                arg: None,
            });
            return false;
        }
        let Some(item_guid) = use_up_one(slot, c, events) else {
            return false;
        };
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
        let duration = clock::secs(t.duration_ms);
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

/// Takes one of the stack in `slot` out of the bags, telling the client: the item's guid.
fn use_up_one(
    slot: crate::inventory::Pos,
    c: &mut Char,
    events: &mut Vec<SessionEvent>,
) -> Option<u64> {
    let item = c.inventory.get(slot)?;
    let item_guid = item.guid;
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
    Some(item_guid)
}
