//! Food and drink (spec #1, "Gear and economy"): used from the bags out of combat, the character
//! sits under the item's spell and is restored evenly over its duration, every two seconds as
//! vanilla's regeneration ticks (vmangos `SPELL_AURA_MOD_REGEN`, `Player::RegenerateHealth`).
//! Standing up, a hit or a fight ends the meal. A Warrior has no mana, so drink restores nothing.
//! A potion (no duration) restores at once, in a fight or out, and then cools down.

use benilla_protocol::SessionEvent;

use super::InWorld;
use crate::characters::{Char, WellFed};
use crate::clock;
use crate::cooking;
use crate::equipment;
use crate::fields::*;
use crate::items;

/// The buff slot the meal's aura shows in: the last one, clear of the skills' auras.
const AURA_SLOT: u16 = 31;
/// The buff slot the Well Fed aura shows in, beside the meal's: the two share their flags and
/// levels words, so both are always written together.
const FED_SLOT: u16 = 30;
pub(super) const UNIT_FIELD_AURA: u16 = 47;
pub(super) const UNIT_FIELD_AURAFLAGS: u16 = 95;
pub(super) const UNIT_FIELD_AURALEVELS: u16 = 101;
/// `AFLAG_CANCELABLE | AFLAG_EFF_INDEX_0`.
pub(super) const AURA_FLAGS: u32 = 0x03;
/// `UNIT_FIELD_BYTES_1`: its low byte is the stand state, 1 sitting.
const UNIT_FIELD_BYTES_1: u16 = 138;
const UNIT_STAND_STATE_SIT: u32 = 1;
/// The regeneration tick.
const TICK_SECS: f32 = 2.0;
/// `SPELL_FAILED_AFFECTING_COMBAT`.
const AFFECTING_COMBAT: u8 = 0x00;

/// A meal in progress.
pub(super) struct Meal {
    /// The item's spell, whose aura shows while sitting.
    spell: u32,
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
        self.meal = Some(Meal {
            spell: t.use_spell,
            total: t.restores_health,
            restored: 0,
            duration: clock::secs(t.duration_ms),
            elapsed: 0.0,
            tick: 0.0,
            at: c.position,
        });
        let mut pairs = self.food_aura_pairs();
        pairs.push((UNIT_FIELD_BYTES_1, UNIT_STAND_STATE_SIT));
        events.push(values(self.guid, &pairs));
        events.push(SessionEvent::AuraDuration {
            slot: AURA_SLOT as u8,
            remaining_ms: t.duration_ms,
        });
        true
    }

    /// `CMSG_USE_ITEM` on a meal (ADR 0025): one is eaten, and its Well Fed buff replaces the one
    /// on. Refused in combat, as food is; nothing when dead. Whether one was used up.
    pub(crate) fn eat_meal(
        &mut self,
        slot: crate::inventory::Pos,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        if self.dead() {
            return false;
        }
        let Some((entry, buff)) = c
            .inventory
            .get(slot)
            .and_then(|i| Some((i.entry, cooking::buff_of(i.entry)?)))
        else {
            return false;
        };
        let eat = cooking::eat();
        if self.player.in_combat {
            events.push(SessionEvent::CastResult {
                spell_id: eat.use_spell,
                success: false,
                reason: Some(AFFECTING_COMBAT),
                arg: None,
            });
            return false;
        }
        let Some(item_guid) = use_up_one(slot, c, events) else {
            return false;
        };
        events.push(SessionEvent::CastResult {
            spell_id: eat.use_spell,
            success: true,
            reason: None,
            arg: None,
        });
        events.push(SessionEvent::SpellGo {
            caster: self.guid,
            spell_id: eat.use_spell,
            cast_flags: 0,
            hits: vec![self.guid],
            misses: Vec::new(),
            target: Some(self.guid),
            go_target: None,
            dest: None,
            ammo_display_id: None,
            item_caster: Some(item_guid),
        });
        let mut line = format!("You eat {}.", item_name(entry));
        if let Some(was) = c.well_fed {
            line.push_str(&format!(" It replaces {}.", item_name(was.meal)));
        }
        events.push(crate::quests::system_line(line));
        let minutes = if buff.legendary {
            eat.legendary_minutes
        } else {
            eat.minutes
        };
        c.well_fed = Some(WellFed {
            meal: entry,
            left_secs: (minutes * 60) as f32,
        });
        self.well_fed_changed(c, events);
        true
    }

    /// The Well Fed buff runs down by `dt`; at nothing it wears off.
    pub(super) fn well_fed_tick(&mut self, dt: f32, c: &mut Char, events: &mut Vec<SessionEvent>) {
        let Some(fed) = &mut c.well_fed else {
            return;
        };
        fed.left_secs -= dt;
        if fed.left_secs <= 0.0 {
            self.end_well_fed(c, "wears off", events);
        }
    }

    /// The character died: a legendary meal's buff survives it, any other ends.
    pub(super) fn well_fed_dies(&mut self, c: &mut Char, events: &mut Vec<SessionEvent>) {
        let Some(fed) = c.well_fed else {
            return;
        };
        if self.fed.as_ref().is_some_and(|f| f.legendary) {
            let line = format!("{} survives your death.", item_name(fed.meal));
            events.push(crate::quests::system_line(line));
        } else {
            self.end_well_fed(c, "ends", events);
        }
    }

    /// The Well Fed buff ends, with "<meal> `how`." in the chat.
    fn end_well_fed(&mut self, c: &mut Char, how: &str, events: &mut Vec<SessionEvent>) {
        let Some(fed) = c.well_fed.take() else {
            return;
        };
        let line = format!("{} {how}.", item_name(fed.meal));
        events.push(crate::quests::system_line(line));
        self.well_fed_changed(c, events);
    }

    /// `c.well_fed` changed: the stats, the speed and the aura follow it, and a damage stack
    /// built under the old one falls away.
    pub(super) fn well_fed_changed(&mut self, c: &Char, events: &mut Vec<SessionEvent>) {
        let speed_was = self.fed_speed_mul();
        self.fed = cooking::fed(c);
        self.damage_stack = 0;
        self.refresh_gear(c, events);
        if self.fed_speed_mul() != speed_was {
            self.update_speed(self.guid, events);
        }
        self.show_food_auras(c, events);
    }

    /// At login under a Well Fed buff: its aura, and its speed. The stats already carry it.
    pub(super) fn show_well_fed(&mut self, c: &Char, events: &mut Vec<SessionEvent>) {
        if self.fed.is_none() {
            return;
        }
        if self.fed_speed_mul() != 1.0 {
            self.update_speed(self.guid, events);
        }
        self.show_food_auras(c, events);
    }

    /// The meal's and the Well Fed slots' fields, and the Well Fed timer while it lasts.
    fn show_food_auras(&self, c: &Char, events: &mut Vec<SessionEvent>) {
        events.push(values(self.guid, &self.food_aura_pairs()));
        if let Some(fed) = c.well_fed.filter(|_| self.fed.is_some()) {
            events.push(SessionEvent::AuraDuration {
                slot: FED_SLOT as u8,
                remaining_ms: clock::ms(fed.left_secs),
            });
        }
    }

    /// The `UNIT_FIELD_AURA*` fields of the meal's slot and the Well Fed slot, as both stand.
    fn food_aura_pairs(&self) -> Vec<(u16, u32)> {
        let meal = self.meal.as_ref().map_or(0, |m| m.spell);
        let fed = self.fed.as_ref().map_or(0, |_| cooking::eat().buff_spell);
        let level = u32::from(self.player.stats.level);
        let (mut flags, mut levels) = (0, 0);
        for (slot, spell) in [(AURA_SLOT, meal), (FED_SLOT, fed)] {
            if spell != 0 {
                flags |= AURA_FLAGS << (4 * (slot % 8));
                levels |= level << (8 * (slot % 4));
            }
        }
        vec![
            (UNIT_FIELD_AURA + AURA_SLOT, meal),
            (UNIT_FIELD_AURA + FED_SLOT, fed),
            (UNIT_FIELD_AURAFLAGS + AURA_SLOT / 8, flags),
            (UNIT_FIELD_AURALEVELS + AURA_SLOT / 4, levels),
        ]
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
        let mut pairs = self.food_aura_pairs();
        pairs.push((UNIT_FIELD_BYTES_1, 0));
        events.push(values(self.guid, &pairs));
    }
}

/// Item `entry`'s name, "" for none.
fn item_name(entry: u32) -> String {
    items::template(entry).map_or_else(String::new, |t| t.name.clone())
}

/// Takes one of the stack in `slot` out of the bags, telling the client: the item's guid.
pub(super) fn use_up_one(
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
