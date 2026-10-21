//! The Crossroads' Token vendor through the stock merchant window: the Token whites for levels 1
//! to 10 and greens by level band above, each row naming its Token price, bought with the Tokens
//! in the bags.

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

const NOVICES_WOOLEN_VEST: u32 = 900200; // 1-10, 3 Tokens, the band's first row
const GNOLL_SKULL_BASHER: u32 = 1440; // 11-20, 4 Tokens, the band's first row
const ROW_ARMOR_OF_NOVICES_WOOLEN_VEST: u32 = 26;

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
    assert_eq!(rows.len(), 22, "the 1-10 band: the Token whites");
    assert_eq!(
        rows.iter().map(|r| r.slot).collect::<Vec<_>>(),
        (1..=22).collect::<Vec<_>>()
    );
    assert!(
        rows.iter().all(|r| r.price == 0),
        "no coins: Tokens pay for it"
    );
    assert!(rows
        .iter()
        .all(|r| r.buy_count == 1 && r.current_count == u32::MAX));

    // The first row reads as the Novice's Woolen Vest at 3 Tokens, with the vest's own stats.
    let vest = shop.query(rows[0].entry);
    assert_eq!(vest.name, "3 Tokens: Novice's Woolen Vest");
    assert_eq!(vest.armor, ROW_ARMOR_OF_NOVICES_WOOLEN_VEST);
    assert_eq!(vest.quality, 1);
    assert_eq!(vest.buy_price, 0);
    assert_eq!(rows[0].display_id, vest.display_info_id);
    // The thirteenth row, the Pendant of Brawn, costs one Token.
    assert_eq!(shop.query(rows[12].entry).name, "1 Token: Pendant of Brawn");
    // The vest itself keeps their name.
    assert_eq!(shop.query(NOVICES_WOOLEN_VEST).name, "Novice's Woolen Vest");
}

#[test]
fn a_higher_level_sees_the_next_band() {
    let mut shop = Shop::new(15, 0, &[]);
    let rows = shop.list();
    assert_eq!(rows.len(), 12, "the 11-20 band");
    assert_eq!(
        shop.query(rows[0].entry).name,
        "4 Tokens: Gnoll Skull Basher"
    );
}

#[test]
fn buying_spends_the_tokens_and_puts_the_green_in_the_bags() {
    let mut shop = Shop::new(5, 0, &[(23, TOKEN, 5)]);
    let vest = shop.row_of("Novice's Woolen Vest");
    let events = shop.buy(&vest);
    assert_eq!(buy_failure(&events), None);
    assert_eq!(shop.carried(TOKEN), 2);
    assert_eq!(shop.carried(NOVICES_WOOLEN_VEST), 1);
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::ItemPushResult(p) if p.from_npc && p.item_entry == NOVICES_WOOLEN_VEST
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
        blob.contains(&format!("entry = {NOVICES_WOOLEN_VEST}")),
        "{blob}"
    );
}

#[test]
fn the_tokens_can_come_from_several_stacks_and_the_last_one_goes() {
    let mut shop = Shop::new(5, 0, &[(23, TOKEN, 2), (30, TOKEN, 1)]);
    let vest = shop.row_of("Novice's Woolen Vest");
    shop.buy(&vest);
    assert_eq!(shop.carried(TOKEN), 0);
    assert_eq!(shop.carried(NOVICES_WOOLEN_VEST), 1);
}

#[test]
fn buying_without_enough_tokens_is_refused_with_the_stock_message() {
    let mut shop = Shop::new(5, 0, &[(23, TOKEN, 1)]);
    let vest = shop.row_of("Novice's Woolen Vest");
    let events = shop.buy(&vest);
    assert_eq!(buy_failure(&events), Some(buy_result::NOT_ENOUGH_MONEY));
    assert_eq!(shop.carried(TOKEN), 1);
    assert_eq!(shop.carried(NOVICES_WOOLEN_VEST), 0);

    // Money is not Tokens.
    let mut rich = Shop::new(5, 100_000, &[]);
    let vest = rich.row_of("Novice's Woolen Vest");
    let events = rich.buy(&vest);
    assert_eq!(buy_failure(&events), Some(buy_result::NOT_ENOUGH_MONEY));
    assert_eq!(rich.carried(NOVICES_WOOLEN_VEST), 0);
}

#[test]
fn a_full_backpack_keeps_the_tokens() {
    let mut items: Vec<Carried> = (24..39).map(|s| (s, 4560, 1)).collect();
    items.push((23, TOKEN, 5));
    let mut shop = Shop::new(5, 0, &items);
    let vest = shop.row_of("Novice's Woolen Vest");
    let events = shop.buy(&vest);
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
    let mut shop = Shop::with_bagged(5, 0, &items, &[(19, 0, TOKEN, 4)]);
    let vest = shop.row_of("Novice's Woolen Vest");
    let events = shop.buy(&vest);
    assert_eq!(buy_failure(&events), None);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SessionEvent::InventoryFailure { .. })),
        "room in the bag"
    );
    let blob = shop.realm.export_character(shop.me).unwrap();
    assert!(
        blob.contains(&format!("entry = {NOVICES_WOOLEN_VEST}")),
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
    // The vest is the band below, and the real entry is no row at all.
    let first = shop.list()[0];
    let events = shop.buy(&VendorItem {
        entry: NOVICES_WOOLEN_VEST,
        ..first
    });
    assert_eq!(buy_failure(&events), Some(buy_result::CANT_FIND_ITEM));
    let mut low = Shop::new(5, 0, &[(23, TOKEN, 20)]);
    let vest = low.row_of("Novice's Woolen Vest");
    let events = shop.buy(&vest);
    assert_eq!(buy_failure(&events), Some(buy_result::CANT_FIND_ITEM));
    assert_eq!(shop.carried(TOKEN), 20);

    // Walk away and the vendor is out of reach.
    let legguards = shop.row_of("Gnoll Skull Basher");
    shop.send(RealmCommand::Move {
        motion: Default::default(),
        position: [
            twilight_realm::CROSSROADS[0] + 40.0,
            twilight_realm::CROSSROADS[1],
            twilight_realm::CROSSROADS[2],
        ],
        orientation: 0.0,
    });
    let events = shop.buy(&legguards);
    assert_eq!(buy_failure(&events), Some(buy_result::DISTANCE_TOO_FAR));
    assert_eq!(shop.carried(GNOLL_SKULL_BASHER), 0);
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
