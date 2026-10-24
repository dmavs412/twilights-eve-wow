//! Bounty Rank and Blood Coin (#178): a hunter's kill pays Blood Coin to everyone it credits and
//! raises only its owner's rank; any death sets the rank back to 0 and keeps the coin; the
//! Blood Coin vendor sells for it.

mod common;
mod gear_common;

use benilla_protocol::messages::{buy_result, LootItem, VendorItem};
use benilla_protocol::{EntityKind, SessionEvent};
use common::*;
use gear_common::*;
use twilight_realm::rules::notoriety::*;
use twilight_realm::{SessionId, CROSSROADS};

/// Notoriety that stays at Heat 4 through every drain here.
const HOT: f32 = 5000.0;
const HUNTER_ENTRIES: std::ops::Range<u32> = 980_000..990_000;
/// Out in the Barrens, well clear of the Hub, on `FlatGround`.
const OUT: [f32; 3] = [CROSSROADS[0], CROSSROADS[1] + 300.0, 0.0];
/// A two-hander that fells a level 2 hunter in a few blows.
const MEGACHOPPER: u32 = 9490;
const MAIN_HAND: u8 = 15;
const BACKPACK_FIRST: u8 = 23;
const BLOOD_COIN_VENDOR: u32 = 190046;
const MINOR_HEALING_POTION: u32 = 118;
const GREATER_HEALING_POTION: u32 = 1710;

// ── The rules ───────────────────────────────────────────────────────────────────────────────────

#[test]
fn each_rank_adds_a_quarter_and_an_affix_every_two() {
    assert_eq!(rank_multiplier(0), 1.0);
    assert_eq!(rank_multiplier(4), 2.0);
    assert_eq!([0, 1, 2, 3, 4, 5].map(affix_count), [0, 0, 1, 1, 2, 2]);
}

#[test]
fn a_rank_n_hunter_drops_n_plus_1_blood_coin() {
    assert_eq!([0, 1, 5].map(coin_drop), [1, 2, 6]);
}

// ── A hunter's kill ─────────────────────────────────────────────────────────────────────────────

/// Players in their own sessions, from saves that one maker wrote so their guids differ.
struct Players {
    realm: Realm,
    sessions: Vec<SessionId>,
    guids: Vec<u64>,
    log: Vec<Vec<SessionEvent>>,
}

impl Players {
    /// One level 1 warrior with the Megachopper per `(notoriety, bounty_rank)`, out in the
    /// Barrens side by side.
    fn new(each: &[(f32, u32)]) -> Self {
        let names = ["Hunted", "Helper"];
        let mut maker = realm();
        for name in &names[..each.len()] {
            assert_eq!(create(&mut maker, name, HUMAN, 0), CHAR_CREATE_SUCCESS);
        }
        let mut realm = Realm::with_seed(SEED);
        let mut guids = Vec::new();
        for (name, &(notoriety, rank)) in names.iter().zip(each) {
            let guid = roster(&mut maker)
                .into_iter()
                .find(|c| c.name == *name)
                .unwrap()
                .guid;
            let mut blob = maker.export_character(guid).unwrap();
            if let Some(at) = blob.find("[[items]]") {
                blob.truncate(at);
            }
            let blob =
                blob.replace("items = []\n", "").replacen(
                    "xp = 0\n",
                    &format!("xp = 0\nnotoriety = {notoriety:?}\nbounty_rank = {rank}\n"),
                    1,
                ) + &format!("\n[[items]]\nslot = {MAIN_HAND}\nentry = {MEGACHOPPER}\ncount = 1\n");
            realm.import_character(&blob).unwrap();
            guids.push(guid);
        }
        let sessions: Vec<SessionId> = guids
            .iter()
            .map(|&guid| {
                let s = realm.connect();
                realm.handle_as(s, RealmCommand::EnterWorld { guid });
                s
            })
            .collect();
        let log = vec![Vec::new(); sessions.len()];
        let mut p = Self {
            realm,
            sessions,
            guids,
            log,
        };
        for i in 0..p.sessions.len() {
            let at = [OUT[0] + i as f32, OUT[1], OUT[2]];
            p.cmd(
                i,
                RealmCommand::Move {
                    motion: Default::default(),
                    position: at,
                    orientation: 0.0,
                },
            );
        }
        p
    }

    fn cmd(&mut self, i: usize, command: RealmCommand) -> Vec<SessionEvent> {
        self.realm.handle_as(self.sessions[i], command);
        self.collect()[i].clone()
    }

    fn collect(&mut self) -> Vec<Vec<SessionEvent>> {
        let mut got = vec![Vec::new(); self.sessions.len()];
        for (s, e) in self.realm.drain_session_events() {
            if let Some(i) = self.sessions.iter().position(|&x| x == s) {
                got[i].push(e.clone());
                self.log[i].push(e);
            }
        }
        got
    }

    fn run(&mut self, secs: f32) {
        for _ in 0..(secs * 10.0).round() as u32 {
            self.realm.tick(0.1, &FlatGround);
            self.collect();
        }
    }

    /// The first hunter player `i`'s client was shown.
    fn hunter(&self, i: usize) -> Option<u64> {
        self.log[i].iter().find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                fields,
                ..
            } if fields
                .object_entry()
                .is_some_and(|e| HUNTER_ENTRIES.contains(&e)) =>
            {
                Some(*guid)
            }
            _ => None,
        })
    }

    fn dead(&self, guid: u64) -> bool {
        self.log.iter().flatten().any(|e| {
            matches!(e, SessionEvent::ObjectValues { guid: g, fields }
                if *g == guid && fields.unit_health() == Some(0))
        })
    }

    /// Waits for the owner's hunter, and everyone fights it until it falls.
    fn kill_the_hunter(&mut self) -> u64 {
        for _ in 0..300 {
            if self.hunter(0).is_some() {
                break;
            }
            self.run(1.0);
        }
        let hunter = self.hunter(0).expect("a hunter came");
        for _ in 0..1200 {
            for i in 0..self.sessions.len() {
                self.face(i, hunter);
                self.cmd(i, RealmCommand::AttackSwing { guid: hunter });
            }
            self.run(0.5);
            if self.dead(hunter) {
                return hunter;
            }
        }
        panic!("the hunter outlived ten minutes of fighting");
    }

    /// Where player `i`'s client last saw `unit`: its create, or the end of its latest move.
    fn seen_at(&self, i: usize, unit: u64) -> Option<[f32; 3]> {
        self.log[i].iter().rev().find_map(|e| match e {
            SessionEvent::MonsterMove { guid, path, .. } if *guid == unit => path.last().copied(),
            SessionEvent::ObjectCreate { guid, position, .. } if *guid == unit => Some(*position),
            _ => None,
        })
    }

    /// Player `i` steps up to `unit` and turns to face it, as a player at the keyboard would.
    fn face(&mut self, i: usize, unit: u64) {
        let Some(them) = self.seen_at(i, unit) else {
            return;
        };
        let me = [OUT[0] + i as f32, OUT[1], OUT[2]];
        let (dx, dy) = (them[0] - me[0], them[1] - me[1]);
        let len = dx.hypot(dy).max(0.01);
        // Within a yard and a half of it, on this player's side.
        let reach = (len - 1.5).max(0.0) / len;
        let at = [me[0] + dx * reach, me[1] + dy * reach, them[2]];
        self.cmd(
            i,
            RealmCommand::Move {
                motion: Default::default(),
                position: at,
                orientation: dy.atan2(dx).rem_euclid(std::f32::consts::TAU),
            },
        );
    }

    /// What player `i`'s loot window on `corpse` shows.
    fn loot(&mut self, i: usize, corpse: u64) -> Vec<LootItem> {
        self.cmd(i, RealmCommand::Loot { guid: corpse })
            .into_iter()
            .find_map(|e| match e {
                SessionEvent::LootResponse { items, .. } => Some(items),
                _ => None,
            })
            .expect("a loot window")
    }

    fn rank(&self, i: usize) -> u32 {
        saved(&self.realm, self.guids[i], "bounty_rank = ").map_or(0, |v| v as u32)
    }
}

fn saved(realm: &Realm, guid: u64, key: &str) -> Option<f32> {
    realm
        .export_character(guid)
        .unwrap()
        .lines()
        .find_map(|l| l.strip_prefix(key))
        .map(|v| v.parse().unwrap())
}

fn coin_in(items: &[LootItem]) -> u32 {
    items
        .iter()
        .filter(|l| l.item_id == blood_coin())
        .map(|l| l.count)
        .sum()
}

#[test]
fn a_hunters_kill_raises_its_owners_rank_and_pays_blood_coin() {
    let mut p = Players::new(&[(HOT, 0)]);
    let hunter = p.kill_the_hunter();
    assert_eq!(p.rank(0), 1);
    assert_eq!(coin_in(&p.loot(0, hunter)), coin_drop(0));
}

#[test]
fn everyone_credited_gets_blood_coin_and_only_the_owner_ranks_up() {
    let mut p = Players::new(&[(HOT, 2), (0.0, 0)]);
    let hunter = p.kill_the_hunter();
    assert_eq!(p.rank(0), 3, "the owner's rank goes up");
    assert_eq!(p.rank(1), 0, "the helper's does not");
    assert_eq!(coin_in(&p.loot(0, hunter)), coin_drop(2));
    assert_eq!(coin_in(&p.loot(1, hunter)), coin_drop(2));
}

// ── Death ───────────────────────────────────────────────────────────────────────────────────────

#[test]
fn any_death_resets_the_rank_and_keeps_the_blood_coin() {
    let (guid, blob) = save_with(1, 0, &[(BACKPACK_FIRST, blood_coin(), 7)]);
    let blob = blob.replacen("xp = 0\n", "xp = 0\nbounty_rank = 4\n", 1);
    let (mut realm, me, events) = enter_saved(guid, &blob);
    assert_eq!(saved(&realm, me, "bounty_rank = "), Some(4.0));
    let mob = the_mob(&events);
    stand_by(&mut realm, &mob);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    realm.handle(RealmCommand::AttackStop);
    let mut all = events;
    for _ in 0..12_000 {
        all.extend(run(&mut realm, 0.1));
        if last_health(&all, me) == Some(0) {
            break;
        }
    }
    assert_eq!(last_health(&all, me), Some(0), "the mob killed them");
    assert_eq!(saved(&realm, me, "bounty_rank = "), None, "rank 0");
    assert_eq!(carried(&all, me, blood_coin()), 7, "the coin stays");
}

#[test]
fn an_old_save_loads_at_rank_0() {
    let (guid, blob) = save_with(5, 0, &[]);
    assert!(!blob.contains("bounty_rank"));
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    assert_eq!(saved(&realm, guid, "bounty_rank = "), None);
}

// ── The Blood Coin vendor ───────────────────────────────────────────────────────────────────────

/// A warrior at the Blood Coin vendor carrying `coins`.
fn at_the_broker(coins: u32) -> (Realm, u64, u64, Vec<VendorItem>) {
    let items: &[Carried] = if coins > 0 {
        &[(BACKPACK_FIRST, 90002, coins)]
    } else {
        &[]
    };
    let (mut realm, me, events) = warrior_with(5, 0, items);
    assert_eq!(blood_coin(), 90002);
    let vendor = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate { guid, fields, .. }
                if fields.object_entry() == Some(BLOOD_COIN_VENDOR) =>
            {
                Some(*guid)
            }
            _ => None,
        })
        .expect("the Blood Coin vendor stands in the Crossroads");
    walk_up_to(&mut realm, vendor);
    realm.handle(RealmCommand::ListInventory { guid: vendor });
    let rows = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::VendorInventory { items, .. } => Some(items),
            _ => None,
        })
        .expect("the merchant list");
    (realm, me, vendor, rows)
}

#[test]
fn the_broker_lists_its_stock_priced_in_blood_coin() {
    let (mut realm, _, _, rows) = at_the_broker(0);
    assert_eq!(rows.len(), coin_stock().len());
    assert!(rows.iter().all(|r| r.price == 0), "no copper price");
    realm.handle(RealmCommand::ItemQuery {
        entry: rows[0].entry,
    });
    let name = drain(&mut realm).into_iter().find_map(|e| match e {
        SessionEvent::ItemTemplate { info, .. } => info.map(|i| i.name),
        _ => None,
    });
    assert_eq!(name.as_deref(), Some("1 Blood Coin: Minor Healing Potion"));
}

#[test]
fn marta_coyle_lists_the_same_rows_as_rusk() {
    let (_, _, _, rusk) = at_the_broker(0);
    let (mut realm, _, marta) = in_elwynn_beside(5, &[], 190070);
    realm.handle(RealmCommand::ListInventory { guid: marta });
    let rows = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::VendorInventory { items, .. } => Some(items),
            _ => None,
        })
        .expect("the merchant list");
    assert_eq!(rows.len(), coin_stock().len());
    assert_eq!(
        rows.iter().map(|r| r.entry).collect::<Vec<_>>(),
        rusk.iter().map(|r| r.entry).collect::<Vec<_>>()
    );
}

#[test]
fn buying_takes_the_blood_coin_and_gives_the_potion() {
    let (mut realm, me, vendor, rows) = at_the_broker(6);
    let row = rows
        .iter()
        .find(|r| r.entry % 1_000_000 == GREATER_HEALING_POTION)
        .unwrap();
    realm.handle(RealmCommand::BuyItem {
        vendor,
        entry: row.entry,
        count: 1,
    });
    let events = drain(&mut realm);
    assert_eq!(carried(&events, me, GREATER_HEALING_POTION), 1);
    let blob = realm.export_character(me).unwrap();
    assert!(blob.contains("entry = 90002\ncount = 1\n"), "{blob}");
}

#[test]
fn a_buyer_short_of_blood_coin_is_refused_and_keeps_it() {
    let (mut realm, me, vendor, rows) = at_the_broker(2);
    let row = rows
        .iter()
        .find(|r| r.entry % 1_000_000 == GREATER_HEALING_POTION)
        .unwrap();
    realm.handle(RealmCommand::BuyItem {
        vendor,
        entry: row.entry,
        count: 1,
    });
    let events = drain(&mut realm);
    let reason = events.iter().find_map(|e| match e {
        SessionEvent::VendorBuyFailed { reason, .. } => Some(*reason),
        _ => None,
    });
    assert_eq!(reason, Some(buy_result::NOT_ENOUGH_MONEY));
    let blob = realm.export_character(me).unwrap();
    assert!(blob.contains("entry = 90002\ncount = 2\n"), "{blob}");
    assert!(!blob.contains(&format!("entry = {GREATER_HEALING_POTION}\n")));
}

// ── Potions ─────────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_potion_heals_at_once_and_then_cools_down() {
    let (guid, blob) = save_with(8, 0, &[(BACKPACK_FIRST, MINOR_HEALING_POTION, 2)]);
    let (mut realm, me, events) = enter_saved(guid, &blob);
    let mob = the_mob(&events);
    stand_by(&mut realm, &mob);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    realm.handle(RealmCommand::AttackStop);
    let mut all = events;
    let max = descriptor(&all, me).unit_max_health().unwrap();
    for _ in 0..3000 {
        if last_health(&all, me).is_some_and(|h| h + 80 <= max) {
            break;
        }
        all.extend(run(&mut realm, 0.1));
    }
    assert!(
        last_health(&all, me).is_some_and(|h| h + 80 <= max),
        "the mob took 80 of {max} health in five minutes: at {:?}",
        last_health(&all, me)
    );
    let before = last_health(&all, me).unwrap();
    assert!(before > 0);
    realm.handle(RealmCommand::UseItem {
        bag: 255,
        slot: BACKPACK_FIRST,
    });
    let used = drain(&mut realm);
    let healed = used.iter().find_map(|e| match e {
        SessionEvent::SpellHealLog(l) if l.target == me => Some(l.amount),
        _ => None,
    });
    assert_eq!(
        healed,
        Some(80),
        "Minor Healing Potion's middle roll, in a fight"
    );

    realm.handle(RealmCommand::UseItem {
        bag: 255,
        slot: BACKPACK_FIRST,
    });
    let again = drain(&mut realm);
    assert!(
        !again
            .iter()
            .any(|e| matches!(e, SessionEvent::SpellHealLog(_))),
        "two minutes apart"
    );
}
