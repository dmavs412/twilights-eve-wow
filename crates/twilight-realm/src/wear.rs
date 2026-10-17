//! Gear wearing down (`rules::durability`): the points hits and death take off what the character
//! wears, sent as each item's durability field, which the stock durability figure reads.

use benilla_protocol::SessionEvent;

use crate::characters::Char;
use crate::equipment::ITEM_FIELD_DURABILITY;
use crate::fields::values;
use crate::inventory::EQUIPMENT_END;
use crate::items;
use crate::rng::Rng;
use crate::rules::durability::{self, WEAR_ROLL, WEAR_SLOTS};

/// A damaging hit the character dealt or took: now and then a point off a random Gear Slot (vmangos `Unit::DealDamage`). Whether a worn item broke, so its stats must go.
pub(crate) fn on_hit(c: &mut Char, rng: &mut Rng, events: &mut Vec<SessionEvent>) -> bool {
    if !durability::wears_on_hit(rng.below(WEAR_ROLL)) {
        return false;
    }
    let slot = WEAR_SLOTS[rng.below(WEAR_SLOTS.len() as u32) as usize];
    let Some(item) = c.inventory.get_mut(slot) else {
        return false;
    };
    if item.durability == 0 {
        return false;
    }
    item.durability -= 1;
    events.push(values(
        item.guid,
        &[(ITEM_FIELD_DURABILITY, item.durability)],
    ));
    item.durability == 0
}

/// Death's toll on every worn item: a tenth of its durability (`Player::DurabilityLossAll`).
/// Whether a worn item broke.
pub(crate) fn on_death(c: &mut Char, events: &mut Vec<SessionEvent>) -> bool {
    let mut broke = false;
    for slot in 0..EQUIPMENT_END {
        let Some(item) = c.inventory.get_mut(slot) else {
            continue;
        };
        let max = items::template(item.entry).map_or(0, |t| t.durability);
        if max == 0 || item.durability == 0 {
            continue;
        }
        item.durability = item.durability.saturating_sub(durability::death_loss(max));
        broke |= item.durability == 0;
        events.push(values(
            item.guid,
            &[(ITEM_FIELD_DURABILITY, item.durability)],
        ));
    }
    broke
}
