//! The Crossroads' goods vendor through the stock merchant window: its food and drink, buying,
//! selling and buyback, and repairs (vmangos `ItemHandler.cpp`, `NPCHandler.cpp`).

mod common;
mod gear_common;

use benilla_protocol::messages::{sell_result, VendorItem};
use benilla_protocol::{EntityKind, ObjectFields, SessionEvent};
use common::*;
use gear_common::*;

const TOUGH_JERKY: u32 = 117; // 25c for 5, sells for 1c
const REFRESHING_SPRING_WATER: u32 = 159;

/// `UNIT_NPC_FLAG_VENDOR` and `UNIT_NPC_FLAG_REPAIR`.
const NPC_FLAG_VENDOR: u32 = 0x4;
const NPC_FLAG_REPAIR: u32 = 0x4000;

/// The goods vendor's create in `events`.
fn the_vendor(events: &[SessionEvent]) -> (u64, [f32; 3], ObjectFields) {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } if fields.unit_npc_flags() & NPC_FLAG_VENDOR != 0 => {
                Some((*guid, *position, fields.clone()))
            }
            _ => None,
        })
        .expect("the goods vendor stands in the Crossroads")
}

/// A warrior in front of the goods vendor, with every event so far.
struct Shop {
    realm: Realm,
    me: u64,
    vendor: u64,
    events: Vec<SessionEvent>,
}

impl Shop {
    fn new(level: u8, money: u32, items: &[Carried]) -> Self {
        Self::with_bagged(level, money, items, &[])
    }

    /// [`Shop::new`], with `bagged` inside the bags `items` equips.
    fn with_bagged(level: u8, money: u32, items: &[Carried], bagged: &[Bagged]) -> Self {
        let (realm, me, events) = warrior_with_bagged(level, money, items, bagged);
        let vendor = the_vendor(&events).0;
        Self {
            realm,
            me,
            vendor,
            events,
        }
    }

    fn send(&mut self, command: RealmCommand) -> Vec<SessionEvent> {
        self.realm.handle(command);
        let new = drain(&mut self.realm);
        self.events.extend(new.clone());
        new
    }

    fn buy(&mut self, entry: u32, count: u8) -> Vec<SessionEvent> {
        let vendor = self.vendor;
        self.send(RealmCommand::BuyItem {
            vendor,
            entry,
            count,
        })
    }

    fn me(&self) -> ObjectFields {
        descriptor(&self.events, self.me)
    }

    fn money(&self) -> u32 {
        self.me().player_money().unwrap()
    }

    /// The item in backpack slot `slot` (23-38): `(guid, entry, count)`.
    fn pack(&self, slot: u8) -> Option<(u64, u32, u32)> {
        let guid = self.me().player_pack_slot(slot - 23).filter(|g| *g != 0)?;
        let item = descriptor(&self.events, guid);
        Some((guid, item.object_entry()?, item.item_stack_count()?))
    }
}

impl Shop {
    fn sell(&mut self, item: u64, count: u8) -> Vec<SessionEvent> {
        let vendor = self.vendor;
        self.send(RealmCommand::SellItem {
            vendor,
            item,
            count,
        })
    }

    fn buy_back(&mut self, slot: u32) -> Vec<SessionEvent> {
        let vendor = self.vendor;
        self.send(RealmCommand::BuybackItem { vendor, slot })
    }

    /// Buyback slot `i` (0-11): `(item guid, price)`, `None` when empty.
    fn buyback(&self, i: u8) -> Option<(u64, u32)> {
        let me = self.me();
        let guid = me.player_buyback_slot(i).filter(|g| *g != 0)?;
        Some((guid, me.player_buyback_price(i).unwrap_or(0)))
    }

    /// The backpack slot holding item `guid`.
    fn pack_slot_of(&self, guid: u64) -> Option<u8> {
        (23..39).find(|s| self.pack(*s).is_some_and(|(g, ..)| g == guid))
    }
}

fn sell_failure(events: &[SessionEvent]) -> Option<u8> {
    events.iter().find_map(|e| match e {
        SessionEvent::VendorSellFailed { reason, .. } => Some(*reason),
        _ => None,
    })
}

fn buy_failure(events: &[SessionEvent]) -> Option<u8> {
    events.iter().find_map(|e| match e {
        SessionEvent::VendorBuyFailed { reason, .. } => Some(*reason),
        _ => None,
    })
}

fn listing(events: &[SessionEvent]) -> Option<(u64, Vec<VendorItem>)> {
    events.iter().find_map(|e| match e {
        SessionEvent::VendorInventory { vendor, items } => Some((*vendor, items.clone())),
        _ => None,
    })
}

#[test]
fn the_goods_vendor_stands_in_the_crossroads_and_lists_food_and_drink() {
    let (mut realm, _, events) = warrior_with(1, 0, &[]);
    let (vendor, at, fields) = the_vendor(&events);
    assert_eq!(fields.unit_npc_flags(), NPC_FLAG_VENDOR | NPC_FLAG_REPAIR);
    let crossroads = [twilight_realm::CROSSROADS[0], twilight_realm::CROSSROADS[1]];
    let d = ((at[0] - crossroads[0]).powi(2) + (at[1] - crossroads[1]).powi(2)).sqrt();
    // In the inn's hall with them.
    assert!(d < 10.0, "{d} yd from where characters appear");

    realm.handle(RealmCommand::ListInventory { guid: vendor });
    let (from, items) = listing(&drain(&mut realm)).expect("the stock merchant list");
    assert_eq!(from, vendor);
    let jerky = items.iter().find(|i| i.entry == TOUGH_JERKY).unwrap();
    assert_eq!((jerky.price, jerky.buy_count), (25, 5));
    assert_eq!(jerky.current_count, u32::MAX, "unlimited");
    assert!(items.iter().any(|i| i.entry == REFRESHING_SPRING_WATER));
    assert_eq!(
        items.len(),
        16,
        "the Crossroads' four tiers of food and drink"
    );
    let slots: Vec<u32> = items.iter().map(|i| i.slot).collect();
    assert_eq!(
        slots,
        (1..=16).collect::<Vec<_>>(),
        "rows are numbered from 1"
    );
}

#[test]
fn the_vendor_stands_on_the_ground_and_streams_with_the_view_as_every_hub_npc() {
    let (mut realm, _, events) = warrior_with(1, 0, &[]);
    let (vendor, at, _) = the_vendor(&events);
    assert_eq!(at[2], 0.0, "the host's ground, not the content's z");

    // Far off, it leaves the world; back at the Hub, it comes back.
    realm.handle(RealmCommand::Move {
        position: [
            twilight_realm::CROSSROADS[0] + 300.0,
            twilight_realm::CROSSROADS[1],
            0.0,
        ],
        orientation: 0.0,
    });
    let away = run(&mut realm, 0.0);
    assert!(away
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectsRemoved(gone) if gone.contains(&vendor))));
    realm.handle(RealmCommand::Move {
        position: [
            twilight_realm::CROSSROADS[0],
            twilight_realm::CROSSROADS[1],
            0.0,
        ],
        orientation: 0.0,
    });
    let back = run(&mut realm, 0.0);
    assert_eq!(the_vendor(&back).0, vendor);
}

#[test]
fn buying_food_takes_its_price_and_puts_the_bundle_in_the_backpack() {
    let mut shop = Shop::new(1, 100, &[]);
    let events = shop.buy(TOUGH_JERKY, 1);
    assert_eq!(shop.money(), 75);
    let (_, entry, count) = shop.pack(23).expect("the jerky in the first backpack slot");
    assert_eq!((entry, count), (TOUGH_JERKY, 5));
    let (slot, new_count, bought) = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::VendorBuyResult {
                slot,
                new_count,
                purchase_count,
                ..
            } => Some((*slot, *new_count, *purchase_count)),
            _ => None,
        })
        .expect("SMSG_BUY_ITEM");
    assert_eq!(
        (slot, new_count, bought),
        (5, u32::MAX, 1),
        "row 5, unlimited"
    );
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::ItemPushResult(p) if p.from_npc && p.item_entry == TOUGH_JERKY && p.count == 5
    )));

    // Two bundles at once go onto the same stack.
    shop.buy(TOUGH_JERKY, 2);
    assert_eq!(shop.money(), 25);
    assert_eq!(shop.pack(23).unwrap().2, 15);
}

#[test]
fn what_the_player_cannot_afford_is_refused() {
    let mut shop = Shop::new(1, 24, &[]);
    let events = shop.buy(TOUGH_JERKY, 1);
    assert_eq!(
        buy_failure(&events),
        Some(benilla_protocol::messages::buy_result::NOT_ENOUGH_MONEY)
    );
    assert_eq!(shop.money(), 24);
    assert_eq!(shop.pack(23), None);
}

#[test]
fn the_vendor_sells_only_its_own_stock_and_only_within_reach() {
    let mut shop = Shop::new(1, 10_000, &[]);
    let events = shop.buy(4560, 1);
    assert_eq!(
        buy_failure(&events),
        Some(benilla_protocol::messages::buy_result::CANT_FIND_ITEM)
    );
    shop.send(RealmCommand::Move {
        position: [
            twilight_realm::CROSSROADS[0] + 30.0,
            twilight_realm::CROSSROADS[1],
            twilight_realm::CROSSROADS[2],
        ],
        orientation: 0.0,
    });
    let events = shop.buy(TOUGH_JERKY, 1);
    assert_eq!(
        buy_failure(&events),
        Some(benilla_protocol::messages::buy_result::DISTANCE_TOO_FAR)
    );
    assert_eq!(shop.money(), 10_000);
}

const FINE_SCIMITAR: u32 = 4560; // sells for 37c
const MELTED_CANDLE: u32 = 755; // sells for 1c, stacks to 5

#[test]
fn selling_pays_the_sell_price_and_offers_the_item_back() {
    let mut shop = Shop::new(1, 0, &[(23, FINE_SCIMITAR, 1), (24, MELTED_CANDLE, 3)]);
    let scimitar = shop.pack(23).unwrap().0;
    let events = shop.sell(scimitar, 0);
    assert_eq!(sell_failure(&events), None);
    assert_eq!(shop.money(), 37);
    assert_eq!(shop.pack(23), None);
    assert_eq!(shop.buyback(0), Some((scimitar, 37)));

    // Two of three candles: 2c, the stack keeps one, and the two wait in the next buyback slot.
    let candles = shop.pack(24).unwrap().0;
    shop.sell(candles, 2);
    assert_eq!(shop.money(), 39);
    assert_eq!(shop.pack(24).map(|p| p.2), Some(1));
    let (sold, price) = shop.buyback(1).expect("the two candles");
    assert_eq!(price, 2);
    assert_eq!(descriptor(&shop.events, sold).item_stack_count(), Some(2));
}

#[test]
fn buying_back_costs_what_the_sale_paid() {
    let mut shop = Shop::new(1, 0, &[(23, FINE_SCIMITAR, 1)]);
    let scimitar = shop.pack(23).unwrap().0;
    shop.sell(scimitar, 0);
    shop.buy(TOUGH_JERKY, 1);
    assert_eq!(shop.money(), 12);

    // 12c is short of the 37c it sold for.
    let events = shop.buy_back(69);
    assert_eq!(
        buy_failure(&events),
        Some(benilla_protocol::messages::buy_result::NOT_ENOUGH_MONEY)
    );
    assert_eq!(shop.buyback(0), Some((scimitar, 37)));

    // With the money, the very same scimitar comes back.
    let mut rich = Shop::new(1, 100, &[(23, FINE_SCIMITAR, 1)]);
    let scimitar = rich.pack(23).unwrap().0;
    rich.sell(scimitar, 0);
    let events = rich.buy_back(69);
    assert_eq!(buy_failure(&events), None);
    assert_eq!(rich.money(), 100);
    assert!(
        rich.pack_slot_of(scimitar).is_some(),
        "the very same scimitar"
    );
    assert_eq!(rich.buyback(0), None);
}

#[test]
fn the_buyback_tab_keeps_the_last_twelve_sales() {
    let candles: Vec<Carried> = (23..36).map(|s| (s, MELTED_CANDLE, 1)).collect();
    let mut shop = Shop::new(1, 0, &candles);
    let guids: Vec<u64> = (23..36).map(|s| shop.pack(s).unwrap().0).collect();
    let mut events = Vec::new();
    for g in &guids {
        events.extend(shop.sell(*g, 0));
    }
    assert_eq!(shop.money(), 13);
    let kept: Vec<u64> = (0..12)
        .filter_map(|i| shop.buyback(i))
        .map(|b| b.0)
        .collect();
    assert_eq!(kept.len(), 12);
    assert!(!kept.contains(&guids[0]), "the oldest sale made room");
    assert!(kept.contains(&guids[12]));
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::ObjectDestroyed(g) if *g == guids[0])));
}

#[test]
fn an_item_in_a_bag_sells_and_a_bag_that_holds_items_does_not() {
    const SMALL_RED_POUCH: u32 = 805;
    let mut shop = Shop::with_bagged(
        1,
        0,
        &[(19, SMALL_RED_POUCH, 1)],
        &[(19, 2, FINE_SCIMITAR, 1)],
    );
    let me = shop.me();
    let pouch = me.player_inv_slot(19).unwrap();
    let scimitar = descriptor(&shop.events, pouch).container_slot(2).unwrap();

    let events = shop.sell(pouch, 0);
    assert_eq!(sell_failure(&events), Some(sell_result::ONLY_EMPTY_BAG));

    let events = shop.sell(scimitar, 0);
    assert_eq!(sell_failure(&events), None);
    assert_eq!(shop.money(), 37);
    assert_eq!(
        descriptor(&shop.events, pouch).container_slot(2),
        Some(0),
        "gone from the bag"
    );
}

#[test]
fn selling_what_the_player_does_not_have_is_refused() {
    let mut shop = Shop::new(1, 0, &[]);
    let events = shop.sell(0x4000_0000_0000_0099, 0);
    assert_eq!(
        sell_failure(&events),
        Some(benilla_protocol::messages::sell_result::CANT_FIND_ITEM)
    );
}
