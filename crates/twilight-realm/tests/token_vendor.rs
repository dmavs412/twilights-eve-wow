//! The Crossroads' Token vendor through the stock merchant window: greens by level band, each
//! row naming its Token price, bought with the Tokens in the bags.

mod common;
mod gear_common;

use benilla_protocol::messages::{buy_result, ItemInfo, VendorItem};
use benilla_protocol::SessionEvent;
use common::*;
use gear_common::*;

/// The Token vendor's creature entry.
const TOKEN_VENDOR: u32 = 190002;
/// The Twilight Token.
const TOKEN: u32 = 90001;

const WARRIORS_PANTS: u32 = 2966; // 1-10, 2 Tokens, the band's first row
const WAR_PAINT_LEGGUARDS: u32 = 14727; // 11-20, 4 Tokens
const ROW_ARMOR_OF_WARRIORS_PANTS: u32 = 94;

/// `UNIT_NPC_FLAG_VENDOR` and `UNIT_NPC_FLAG_REPAIR`.
const NPC_FLAG_VENDOR: u32 = 0x4;
const NPC_FLAG_REPAIR: u32 = 0x4000;

/// A warrior at the Crossroads with the Token vendor's guid and every event so far.
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
        let vendor = events
            .iter()
            .find_map(|e| match e {
                SessionEvent::ObjectCreate { guid, fields, .. }
                    if fields.object_entry() == Some(TOKEN_VENDOR) =>
                {
                    Some(*guid)
                }
                _ => None,
            })
            .expect("the Token vendor stands in the Crossroads");
        let mut realm = realm;
        walk_up_to(&mut realm, vendor);
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

    fn list(&mut self) -> Vec<VendorItem> {
        let guid = self.vendor;
        let events = self.send(RealmCommand::ListInventory { guid });
        events
            .iter()
            .find_map(|e| match e {
                SessionEvent::VendorInventory { vendor, items } if *vendor == guid => {
                    Some(items.clone())
                }
                _ => None,
            })
            .expect("the stock merchant list")
    }

    /// The template the client gets for a row's entry.
    fn query(&mut self, entry: u32) -> ItemInfo {
        let events = self.send(RealmCommand::ItemQuery { entry });
        events
            .iter()
            .find_map(|e| match e {
                SessionEvent::ItemTemplate { entry: e, info } if *e == entry => {
                    info.as_deref().cloned()
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("item {entry} answers its query"))
    }

    /// The listed row whose template names `green`, by its name after the price.
    fn row_of(&mut self, green: &str) -> VendorItem {
        let rows = self.list();
        rows.into_iter()
            .find(|r| self.query(r.entry).name.ends_with(green))
            .unwrap_or_else(|| panic!("{green} is listed"))
    }

    fn buy(&mut self, row: &VendorItem) -> Vec<SessionEvent> {
        let vendor = self.vendor;
        self.send(RealmCommand::BuyItem {
            vendor,
            entry: row.entry,
            count: 1,
        })
    }

    fn carried(&self, entry: u32) -> u32 {
        carried(&self.events, self.me, entry)
    }
}

fn buy_failure(events: &[SessionEvent]) -> Option<u8> {
    events.iter().find_map(|e| match e {
        SessionEvent::VendorBuyFailed { reason, .. } => Some(*reason),
        _ => None,
    })
}

#[test]
fn the_token_vendor_stands_in_the_crossroads_and_sells_without_repairing() {
    let shop = Shop::new(1, 0, &[]);
    let (at, flags) = shop
        .events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                position,
                fields,
                ..
            } if *guid == shop.vendor => Some((*position, fields.unit_npc_flags())),
            _ => None,
        })
        .unwrap();
    assert_eq!(flags & (NPC_FLAG_VENDOR | NPC_FLAG_REPAIR), NPC_FLAG_VENDOR);
    let crossroads = twilight_realm::CROSSROADS;
    let d = ((at[0] - crossroads[0]).powi(2) + (at[1] - crossroads[1]).powi(2)).sqrt();
    // At the inn's door.
    assert!(d < 20.0, "{d} yd from where characters appear");
}

#[test]
fn the_list_is_the_buyers_band_with_each_rows_token_price_in_its_name() {
    let mut shop = Shop::new(5, 0, &[]);
    let rows = shop.list();
    assert_eq!(rows.len(), 14, "the 1-10 band");
    assert_eq!(
        rows.iter().map(|r| r.slot).collect::<Vec<_>>(),
        (1..=14).collect::<Vec<_>>()
    );
    assert!(
        rows.iter().all(|r| r.price == 0),
        "no coins: Tokens pay for it"
    );
    assert!(rows
        .iter()
        .all(|r| r.buy_count == 1 && r.current_count == u32::MAX));

    // The first row reads as the Warrior's Pants at 2 Tokens, with the pants' own stats.
    let pants = shop.query(rows[0].entry);
    assert_eq!(pants.name, "2 Tokens: Warrior's Pants");
    assert_eq!(pants.armor, ROW_ARMOR_OF_WARRIORS_PANTS);
    assert_eq!(pants.quality, 2);
    assert_eq!(pants.buy_price, 0);
    assert_eq!(rows[0].display_id, pants.display_info_id);
    // The fourth row, the Hunting Bow, costs one Token.
    assert_eq!(shop.query(rows[3].entry).name, "1 Token: Hunting Bow");
    // The pants themselves keep their name.
    assert_eq!(shop.query(WARRIORS_PANTS).name, "Warrior's Pants");
}

#[test]
fn a_higher_level_sees_the_next_band() {
    let mut shop = Shop::new(15, 0, &[]);
    let rows = shop.list();
    assert_eq!(rows.len(), 25, "the 11-20 band");
    assert_eq!(
        shop.query(rows[0].entry).name,
        "4 Tokens: War Paint Legguards"
    );
}

#[test]
fn buying_spends_the_tokens_and_puts_the_green_in_the_bags() {
    let mut shop = Shop::new(5, 0, &[(23, TOKEN, 5)]);
    let pants = shop.row_of("Warrior's Pants");
    let events = shop.buy(&pants);
    assert_eq!(buy_failure(&events), None);
    assert_eq!(shop.carried(TOKEN), 3);
    assert_eq!(shop.carried(WARRIORS_PANTS), 1);
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::ItemPushResult(p) if p.from_npc && p.item_entry == WARRIORS_PANTS
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::VendorBuyResult {
            slot: 1,
            purchase_count: 1,
            ..
        }
    )));
    // The purchase is saved.
    let blob = shop.realm.export_character(shop.me).unwrap();
    assert!(
        blob.contains(&format!("entry = {WARRIORS_PANTS}")),
        "{blob}"
    );
}

#[test]
fn the_tokens_can_come_from_several_stacks_and_the_last_one_goes() {
    let mut shop = Shop::new(5, 0, &[(23, TOKEN, 1), (30, TOKEN, 1)]);
    let pants = shop.row_of("Warrior's Pants");
    shop.buy(&pants);
    assert_eq!(shop.carried(TOKEN), 0);
    assert_eq!(shop.carried(WARRIORS_PANTS), 1);
}

#[test]
fn buying_without_enough_tokens_is_refused_with_the_stock_message() {
    let mut shop = Shop::new(5, 0, &[(23, TOKEN, 1)]);
    let pants = shop.row_of("Warrior's Pants");
    let events = shop.buy(&pants);
    assert_eq!(buy_failure(&events), Some(buy_result::NOT_ENOUGH_MONEY));
    assert_eq!(shop.carried(TOKEN), 1);
    assert_eq!(shop.carried(WARRIORS_PANTS), 0);

    // Money is not Tokens.
    let mut rich = Shop::new(5, 100_000, &[]);
    let pants = rich.row_of("Warrior's Pants");
    let events = rich.buy(&pants);
    assert_eq!(buy_failure(&events), Some(buy_result::NOT_ENOUGH_MONEY));
    assert_eq!(rich.carried(WARRIORS_PANTS), 0);
}

#[test]
fn a_full_backpack_keeps_the_tokens() {
    let mut items: Vec<Carried> = (24..39).map(|s| (s, 4560, 1)).collect();
    items.push((23, TOKEN, 5));
    let mut shop = Shop::new(5, 0, &items);
    let pants = shop.row_of("Warrior's Pants");
    let events = shop.buy(&pants);
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::InventoryFailure { reason: 50, .. })));
    assert_eq!(shop.carried(TOKEN), 5);
}

#[test]
fn tokens_in_a_bag_pay_and_the_green_finds_room_in_it() {
    // The backpack is full; the Tokens and the room are in the Small Red Pouch.
    let mut items: Vec<Carried> = (23..39).map(|s| (s, 4560, 1)).collect();
    items.push((19, 805, 1));
    let mut shop = Shop::with_bagged(5, 0, &items, &[(19, 0, TOKEN, 3)]);
    let pants = shop.row_of("Warrior's Pants");
    let events = shop.buy(&pants);
    assert_eq!(buy_failure(&events), None);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SessionEvent::InventoryFailure { .. })),
        "room in the bag"
    );
    let blob = shop.realm.export_character(shop.me).unwrap();
    assert!(
        blob.contains(&format!("entry = {WARRIORS_PANTS}")),
        "{blob}"
    );
    assert!(
        blob.contains(&format!(
            "entry = {TOKEN}
count = 1"
        )),
        "{blob}"
    );
}

#[test]
fn only_the_buyers_band_is_for_sale_and_only_within_reach() {
    let mut shop = Shop::new(15, 0, &[(23, TOKEN, 20)]);
    // The pants are the band below, and the real entry is no row at all.
    let first = shop.list()[0];
    let events = shop.buy(&VendorItem {
        entry: WARRIORS_PANTS,
        ..first
    });
    assert_eq!(buy_failure(&events), Some(buy_result::CANT_FIND_ITEM));
    let mut low = Shop::new(5, 0, &[(23, TOKEN, 20)]);
    let pants = low.row_of("Warrior's Pants");
    let events = shop.buy(&pants);
    assert_eq!(buy_failure(&events), Some(buy_result::CANT_FIND_ITEM));
    assert_eq!(shop.carried(TOKEN), 20);

    // Walk away and the vendor is out of reach.
    let legguards = shop.row_of("War Paint Legguards");
    shop.send(RealmCommand::Move {
        position: [
            twilight_realm::CROSSROADS[0] + 40.0,
            twilight_realm::CROSSROADS[1],
            twilight_realm::CROSSROADS[2],
        ],
        orientation: 0.0,
    });
    let events = shop.buy(&legguards);
    assert_eq!(buy_failure(&events), Some(buy_result::DISTANCE_TOO_FAR));
    assert_eq!(shop.carried(WAR_PAINT_LEGGUARDS), 0);
    assert_eq!(shop.carried(TOKEN), 20);
}

#[test]
fn the_token_vendor_buys_loot_as_any_vendor_does() {
    let mut shop = Shop::new(5, 0, &[(23, 4560, 1)]);
    let me = descriptor(&shop.events, shop.me);
    let scimitar = me.player_pack_slot(0).unwrap();
    let vendor = shop.vendor;
    let events = shop.send(RealmCommand::SellItem {
        vendor,
        item: scimitar,
        count: 0,
    });
    assert!(!events
        .iter()
        .any(|e| matches!(e, SessionEvent::VendorSellFailed { .. })));
    assert!(descriptor(&shop.events, shop.me).player_money().unwrap() > 0);
}
