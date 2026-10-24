//! The vendors paid in an item from the bags rather than copper: the Token vendors' half of the
//! merchant window, either the buyer's band of greens or one flat list ([`rules::tokens`]) for the
//! Twilight Tokens in the bags, and the Blood Coin vendors', their stock ([`rules::notoriety`])
//! for Blood Coin.
//!
//! The 1.12 merchant window can show a price only in copper (extended costs came in 2.0), so each
//! row is listed under an entry of its own whose template is the item's with the price in front
//! of its name ("2 Tokens: Warrior's Pants") and no copper price. Buying a row hands over the
//! item itself, under its own entry and name. A buyer short of the currency gets the stock "You
//! don't have enough money." refusal, the only one 1.12 has for a price not met.

use benilla_protocol::messages::{buy_result, ItemInfo, VendorItem};
use benilla_protocol::SessionEvent;

use crate::characters::Char;
use crate::quests::TOKEN;
use crate::vendor::Traded;
use crate::{equipment, items, rules};

/// What a vendor takes in place of copper, and what it sells for it.
pub(crate) struct Currency {
    /// The item paid over.
    item: fn() -> u32,
    /// A row's entry is its item's plus this: above 1.12's items, the realm's own entries and
    /// every other currency's rows.
    listing: u32,
    /// The price's words, one and many.
    one: &'static str,
    many: &'static str,
    /// What a character of a level is offered, `(item entry, price)`, in the window's order.
    stock: fn(u8) -> Vec<(u32, u32)>,
    /// The price of an item, if it is sold at any level.
    price: fn(u32) -> Option<u32>,
}

/// The Twilight Token, for the Token vendor's greens.
pub(crate) const TOKENS: Currency = Currency {
    item: || TOKEN,
    listing: 1_000_000,
    one: "Token",
    many: "Tokens",
    stock: rules::tokens::stock,
    price: rules::tokens::price,
};

/// Blood Coin, for the Blood Coin vendor's stock.
pub(crate) const BLOOD_COIN: Currency = Currency {
    item: rules::notoriety::blood_coin,
    listing: 2_000_000,
    one: "Blood Coin",
    many: "Blood Coin",
    stock: |_| rules::notoriety::coin_stock(),
    price: rules::notoriety::coin_price,
};

/// The Twilight Token, for the Goldshire Token vendor's flat list. Its own listing range, so its
/// prices change independently of the bands'.
pub(crate) const FLAT_TOKENS: Currency = Currency {
    item: || TOKEN,
    listing: 3_000_000,
    one: "Token",
    many: "Tokens",
    stock: |_| rules::tokens::flat_stock(),
    price: rules::tokens::flat_price,
};

const CURRENCIES: [&Currency; 3] = [&TOKENS, &BLOOD_COIN, &FLAT_TOKENS];

impl Currency {
    /// The item a row's `entry` lists, and its price; `None` for any other entry.
    fn listed(&self, entry: u32) -> Option<(u32, u32)> {
        let item = entry.checked_sub(self.listing)?;
        Some((item, (self.price)(item)?))
    }

    /// A row's name: the price, then the item's own name.
    fn row_name(&self, price: u32, name: &str) -> String {
        match price {
            1 => format!("1 {}: {name}", self.one),
            n => format!("{n} {}: {name}", self.many),
        }
    }

    /// The merchant rows a character of `level` is offered, numbered from 1.
    pub(crate) fn rows(&self, level: u8) -> Vec<VendorItem> {
        (self.stock)(level)
            .into_iter()
            .filter_map(|(item, _)| items::template(item))
            .enumerate()
            .map(|(i, t)| VendorItem {
                slot: i as u32 + 1,
                entry: t.entry + self.listing,
                display_id: t.display_id,
                current_count: u32::MAX,
                price: 0,
                max_durability: t.durability,
                buy_count: 1,
            })
            .collect()
    }

    /// `CMSG_BUY_ITEM` at this currency's vendor: `count` of the row `entry`, which the buyer's
    /// stock must list, for its price. The buyer is within reach.
    pub(crate) fn buy(
        &self,
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
        let stock = (self.stock)(c.level);
        let Some((row, (item, each))) = self.listed(entry).and_then(|(item, _)| {
            stock
                .iter()
                .copied()
                .enumerate()
                .find(|(_, (e, _))| *e == item)
        }) else {
            return fail(buy_result::CANT_FIND_ITEM, events);
        };
        let count = u32::from(count.max(1));
        let price = each * count;
        let currency = (self.item)();
        if c.inventory.count(currency) < price {
            return fail(buy_result::NOT_ENOUGH_MONEY, events);
        }
        // The bags are checked before the price goes, as vmangos checks before it takes it.
        if !c.inventory.fits(item, count) {
            events.push(equipment::inventory_failure(
                equipment::EQUIP_ERR_INVENTORY_FULL,
                0,
            ));
            return Traded::default();
        }
        equipment::take(c, currency, price, events);
        // Taking the price only freed room.
        let _ = equipment::receive_from(c, item, count, true, events);
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
}

/// `SMSG_ITEM_QUERY_SINGLE_RESPONSE`'s template for a row's entry, of any currency: the item's,
/// named with its price and costing no copper. `None` for an entry that is no row.
pub(crate) fn row_info(entry: u32) -> Option<ItemInfo> {
    CURRENCIES.iter().find_map(|cur| {
        let (item, price) = cur.listed(entry)?;
        let mut info = items::info(items::template(item)?);
        info.name = cur.row_name(price, &info.name);
        info.buy_price = 0;
        Some(info)
    })
}
