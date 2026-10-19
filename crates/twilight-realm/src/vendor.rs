//! The stock merchant window against a Hub vendor (vmangos `ItemHandler.cpp`): its list, buying,
//! selling and buying back, and repairs.

use benilla_protocol::messages::{buy_result, sell_result, VendorItem};
use benilla_protocol::SessionEvent;

use crate::characters::Char;
use crate::equipment;
use crate::fields::{guid_pairs, values};
use crate::hub::{self, Npc};
use crate::inventory::{Item, Pos};
use crate::items;
use crate::rules::durability;

/// `CMSG_LIST_INVENTORY`: the vendor's rows, numbered from 1, or the Token vendor's for the
/// buyer's level, or the Blood Coin vendor's; nothing for an NPC out of reach or that is no
/// vendor.
pub(crate) fn list(vendor: u64, c: &Char, events: &mut Vec<SessionEvent>) {
    let Some(npc) = reachable(vendor, c) else {
        return;
    };
    if let Some(currency) = npc.currency() {
        let items = currency.rows(c.level);
        events.push(SessionEvent::VendorInventory { vendor, items });
        return;
    }
    let items = npc
        .stock
        .iter()
        .filter_map(|&entry| items::template(entry))
        .enumerate()
        .map(|(i, t)| VendorItem {
            slot: i as u32 + 1,
            entry: t.entry,
            display_id: t.display_id,
            current_count: u32::MAX,
            price: t.buy_price,
            max_durability: t.durability,
            buy_count: t.buy_count.max(1),
        })
        .collect();
    events.push(SessionEvent::VendorInventory { vendor, items });
}

/// What a trade did that the realm acts on.
#[derive(Default)]
pub(crate) struct Traded {
    /// Money or items changed hands: save.
    pub(crate) changed: bool,
    /// A worn item's durability came back from 0: its stats return.
    pub(crate) gear_changed: bool,
}

/// `CMSG_BUY_ITEM`: `count` bundles of `entry` (vmangos `Player::BuyItemFromVendor`).
pub(crate) fn buy(
    vendor: u64,
    entry: u32,
    count: u8,
    c: &mut Char,
    events: &mut Vec<SessionEvent>,
) -> Traded {
    let fail = |reason, events: &mut Vec<SessionEvent>| {
        events.push(SessionEvent::VendorBuyFailed {
            vendor,
            item_entry: entry,
            reason,
        });
        Traded::default()
    };
    let Some(npc) = reachable(vendor, c) else {
        return fail(buy_result::DISTANCE_TOO_FAR, events);
    };
    if let Some(currency) = npc.currency() {
        return currency.buy(vendor, entry, count, c, events);
    }
    let Some(row) = npc.stock.iter().position(|e| *e == entry) else {
        return fail(buy_result::CANT_FIND_ITEM, events);
    };
    let Some(t) = items::template(entry) else {
        return fail(buy_result::CANT_FIND_ITEM, events);
    };
    let count = u32::from(count.max(1));
    let price = t.buy_price * count;
    if c.inventory.money < price {
        return fail(buy_result::NOT_ENOUGH_MONEY, events);
    }
    if let Err(code) = equipment::receive_from(c, entry, t.buy_count.max(1) * count, true, events) {
        events.push(equipment::inventory_failure(code, 0));
        return Traded::default();
    }
    c.inventory.money -= price;
    events.push(values(c.guid, &[equipment::money_pair(&c.inventory)]));
    events.push(SessionEvent::VendorBuyResult {
        vendor,
        slot: row as u32 + 1,
        new_count: u32::MAX,
        purchase_count: count,
    });
    Traded {
        changed: true,
        gear_changed: false,
    }
}

// The buyback tab's player fields (`UpdateFields_1_12_1.h`): twelve guids, prices and expiries.
const PLAYER_FIELD_VENDORBUYBACK_SLOT_1: u16 = 624;
const PLAYER_FIELD_BUYBACK_PRICE_1: u16 = 1226;
const PLAYER_FIELD_BUYBACK_TIMESTAMP_1: u16 = 1238;
/// `BUYBACK_SLOT_START`: the first buyback slot's player inventory slot.
const BUYBACK_SLOT_START: u32 = 69;
/// How long a sale stays on offer, which the client only sorts by (vmangos: 30 hours).
const BUYBACK_SECS: u32 = 30 * 3600;

/// The session's sales, on offer for buying back (vmangos clears them at logout).
#[derive(Default)]
pub(crate) struct Buyback {
    slots: [Option<Sold>; 12],
    /// Sales so far, which orders them oldest first.
    sales: u32,
}

struct Sold {
    item: Item,
    price: u32,
    stamp: u32,
}

impl Buyback {
    /// Offers `item`, sold for `price`, in the first empty slot or in place of the oldest sale
    /// (`Player::AddItemToBuyBackSlot`).
    fn add(&mut self, owner: u64, item: Item, price: u32, events: &mut Vec<SessionEvent>) {
        let i = self
            .slots
            .iter()
            .position(Option::is_none)
            .unwrap_or_else(|| {
                (0..self.slots.len())
                    .min_by_key(|i| self.slots[*i].as_ref().map_or(0, |s| s.stamp))
                    .unwrap_or(0)
            });
        if let Some(old) = self.slots[i].take() {
            events.push(SessionEvent::ObjectDestroyed(old.item.guid));
        }
        self.sales += 1;
        let stamp = self.sales + BUYBACK_SECS;
        events.push(values(owner, &buyback_pairs(i, item.guid, price, stamp)));
        self.slots[i] = Some(Sold { item, price, stamp });
    }

    /// Takes slot `i`'s sale off the tab.
    fn take(&mut self, owner: u64, i: usize, events: &mut Vec<SessionEvent>) -> Option<Sold> {
        let sold = self.slots.get_mut(i)?.take()?;
        events.push(values(owner, &buyback_pairs(i, 0, 0, 0)));
        Some(sold)
    }
}

fn buyback_pairs(i: usize, guid: u64, price: u32, stamp: u32) -> Vec<(u16, u32)> {
    let i = i as u16;
    let mut pairs = guid_pairs(PLAYER_FIELD_VENDORBUYBACK_SLOT_1 + 2 * i, guid).to_vec();
    pairs.push((PLAYER_FIELD_BUYBACK_PRICE_1 + i, price));
    pairs.push((PLAYER_FIELD_BUYBACK_TIMESTAMP_1 + i, stamp));
    pairs
}

/// `CMSG_SELL_ITEM`: `count` of item `guid`, 0 for the whole stack, for its sell price; the sale
/// goes on the buyback tab. A success is silent.
pub(crate) fn sell(
    vendor: u64,
    guid: u64,
    count: u8,
    c: &mut Char,
    buyback: &mut Buyback,
    events: &mut Vec<SessionEvent>,
) -> Traded {
    let fail = |reason, events: &mut Vec<SessionEvent>| {
        events.push(SessionEvent::VendorSellFailed {
            vendor,
            item_guid: guid,
            reason,
        });
        Traded::default()
    };
    if reachable(vendor, c).is_none() {
        return fail(sell_result::CANT_FIND_VENDOR, events);
    }
    let Some((slot, have, entry)) = c
        .inventory
        .iter()
        .find(|(_, i)| i.guid == guid)
        .map(|(s, i)| (s, i.count, i.entry))
    else {
        return fail(sell_result::CANT_FIND_ITEM, events);
    };
    if !equipment::may_leave(c, slot) {
        return fail(sell_result::ONLY_EMPTY_BAG, events);
    }
    let sell_price = items::template(entry).map_or(0, |t| t.sell_price);
    if sell_price == 0 {
        return fail(sell_result::CANT_SELL_ITEM, events);
    }
    let count = match u32::from(count) {
        0 => have,
        n => n.min(have),
    };
    let sold = if count == have {
        let Some(item) = c.inventory.take(slot) else {
            return Traded::default();
        };
        equipment::send_slots(c, &[slot], events);
        item
    } else {
        // Part of a stack: the rest stays, and the sold part is an item of its own.
        if let Some(item) = c.inventory.get_mut(slot) {
            item.count -= count;
            events.push(values(
                item.guid,
                &[(equipment::ITEM_FIELD_STACK_COUNT, item.count)],
            ));
        }
        let item = c.inventory.new_item(entry, count);
        events.push(equipment::item_create(c, slot, &item));
        item
    };
    let price = sell_price * count;
    c.inventory.money += price;
    events.push(values(c.guid, &[equipment::money_pair(&c.inventory)]));
    buyback.add(c.guid, sold, price, events);
    Traded {
        changed: true,
        gear_changed: slot.is_worn(),
    }
}

/// `CMSG_BUYBACK_ITEM`: player buyback `slot` (69-80) back into the backpack, for what it sold for.
pub(crate) fn buy_back(
    vendor: u64,
    slot: u32,
    c: &mut Char,
    buyback: &mut Buyback,
    events: &mut Vec<SessionEvent>,
) -> Traded {
    let i = slot.wrapping_sub(BUYBACK_SLOT_START) as usize;
    let Some(sold) = buyback.slots.get(i).and_then(Option::as_ref) else {
        events.push(SessionEvent::VendorBuyFailed {
            vendor,
            item_entry: 0,
            reason: buy_result::CANT_FIND_ITEM,
        });
        return Traded::default();
    };
    if reachable(vendor, c).is_none() {
        events.push(SessionEvent::VendorSellFailed {
            vendor,
            item_guid: sold.item.guid,
            reason: sell_result::CANT_FIND_VENDOR,
        });
        return Traded::default();
    }
    if c.inventory.money < sold.price {
        events.push(SessionEvent::VendorBuyFailed {
            vendor,
            item_entry: sold.item.entry,
            reason: buy_result::NOT_ENOUGH_MONEY,
        });
        return Traded::default();
    }
    let Some(dst) = c.inventory.free_slot(sold.item.entry) else {
        events.push(equipment::inventory_failure(
            equipment::EQUIP_ERR_INVENTORY_FULL,
            sold.item.guid,
        ));
        return Traded::default();
    };
    let Some(sold) = buyback.take(c.guid, i, events) else {
        return Traded::default();
    };
    c.inventory.money -= sold.price;
    c.inventory.put(dst, sold.item);
    equipment::send_slots(c, &[dst], events);
    events.push(values(c.guid, &[equipment::money_pair(&c.inventory)]));
    Traded {
        changed: true,
        gear_changed: false,
    }
}

/// `CMSG_REPAIR_ITEM`: item `guid` back to full durability, or with `guid` 0 everything carried,
/// in slot order (vmangos `Player::DurabilityRepairAll`). Each repair is paid as it is made; the
/// first the purse cannot cover stops it. Nothing answers but the fields.
pub(crate) fn repair(
    vendor: u64,
    guid: u64,
    c: &mut Char,
    events: &mut Vec<SessionEvent>,
) -> Traded {
    let mut traded = Traded::default();
    if !reachable(vendor, c).is_some_and(Npc::repairs) {
        return traded;
    }
    let slots: Vec<Pos> = c
        .inventory
        .iter()
        .filter(|(_, i)| guid == 0 || i.guid == guid)
        .map(|(s, _)| s)
        .collect();
    for slot in slots {
        let Some(item) = c.inventory.get(slot) else {
            continue;
        };
        let cost = durability::repair_cost(item.entry, item.durability);
        if cost == 0 {
            continue;
        }
        if c.inventory.money < cost {
            break;
        }
        c.inventory.money -= cost;
        let Some(item) = c.inventory.get_mut(slot) else {
            continue;
        };
        let was_broken = item.durability == 0;
        item.durability = items::template(item.entry).map_or(0, |t| t.durability);
        events.push(values(
            item.guid,
            &[(equipment::ITEM_FIELD_DURABILITY, item.durability)],
        ));
        traded.changed = true;
        traded.gear_changed |= was_broken && slot.is_worn();
    }
    if traded.changed {
        events.push(values(c.guid, &[equipment::money_pair(&c.inventory)]));
    }
    traded
}

/// The vendor `guid`, if the character stands within reach of it.
fn reachable(guid: u64, c: &Char) -> Option<&'static Npc> {
    hub::find(guid).filter(|n| n.is_vendor() && hub::within_reach(n, c.position))
}
