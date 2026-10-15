//! The Token vendor's half of the merchant window: the buyer's band of greens
//! ([`rules::tokens`]), paid for with the Twilight Tokens in the bags.
//!
//! The 1.12 merchant window can show a price only in copper (extended costs came in 2.0), so each
//! row is listed under an entry of its own whose template is the green's with the price in front
//! of its name ("2 Tokens: Warrior's Pants") and no copper price. Buying a row hands over the
//! green itself, under its own entry and name. A buyer short of Tokens gets the stock "You don't
//! have enough money." refusal, the only one 1.12 has for a price not met.

use benilla_protocol::messages::{buy_result, ItemInfo, VendorItem};
use benilla_protocol::SessionEvent;

use crate::characters::Char;
use crate::quests::TOKEN;
use crate::vendor::Traded;
use crate::{equipment, items, rules};

/// A row's entry is the green's plus this: above 1.12's items and the realm's own entries.
const LISTING: u32 = 1_000_000;

/// The green a row's `entry` lists, and its Token price; `None` for any other entry.
fn listed(entry: u32) -> Option<(u32, u32)> {
    let green = entry.checked_sub(LISTING)?;
    Some((green, rules::tokens::price(green)?))
}

/// A row's name: the price, then the green's own name.
fn row_name(tokens: u32, name: &str) -> String {
    match tokens {
        1 => format!("1 Token: {name}"),
        n => format!("{n} Tokens: {name}"),
    }
}

/// The merchant rows a character of `level` is offered, numbered from 1.
pub(crate) fn rows(level: u8) -> Vec<VendorItem> {
    rules::tokens::stock(level)
        .into_iter()
        .filter_map(|(green, _)| items::template(green))
        .enumerate()
        .map(|(i, t)| VendorItem {
            slot: i as u32 + 1,
            entry: t.entry + LISTING,
            display_id: t.display_id,
            current_count: u32::MAX,
            price: 0,
            max_durability: t.durability,
            buy_count: 1,
        })
        .collect()
}

/// `SMSG_ITEM_QUERY_SINGLE_RESPONSE`'s template for a row's entry: the green's, named with its
/// price and costing no copper. `None` for an entry that is no row.
pub(crate) fn row_info(entry: u32) -> Option<ItemInfo> {
    let (green, tokens) = listed(entry)?;
    let mut info = items::info(items::template(green)?);
    info.name = row_name(tokens, &info.name);
    info.buy_price = 0;
    Some(info)
}

/// `CMSG_BUY_ITEM` at the Token vendor: `count` of the row `entry`, which the buyer's band must
/// list, for its Tokens. The buyer is within reach.
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
    let stock = rules::tokens::stock(c.level);
    let Some((row, (green, tokens))) = listed(entry).and_then(|(green, _)| {
        stock
            .iter()
            .copied()
            .enumerate()
            .find(|(_, (e, _))| *e == green)
    }) else {
        return fail(buy_result::CANT_FIND_ITEM, events);
    };
    let count = u32::from(count.max(1));
    let price = tokens * count;
    if c.inventory.count(TOKEN) < price {
        return fail(buy_result::NOT_ENOUGH_MONEY, events);
    }
    // The bags are checked before the Tokens go, as vmangos checks before it takes the price.
    if !c.inventory.fits(green, count) {
        events.push(equipment::inventory_failure(
            equipment::EQUIP_ERR_INVENTORY_FULL,
            0,
        ));
        return Traded::default();
    }
    equipment::take(c, TOKEN, price, events);
    // Taking the Tokens only freed room.
    let _ = equipment::receive_from(c, green, count, true, events);
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
