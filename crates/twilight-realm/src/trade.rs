//! **Player trade** (#162): vanilla's trade window between two players, of any faction (#160).
//! One asks (`CMSG_INITIATE_TRADE`), the other opens it (`CMSG_BEGIN_TRADE`); each offers up to
//! six items and some gold, and any change drops both accepts. Once both accept, the realm swaps
//! everything in one step or nothing: both sides need the gold they offer and the bag space for
//! what they get. Items have no binding (#166), so anything in the bags may go. The seventh,
//! not-traded slot is ignored: there is no enchanting. A trade ends when either side walks out
//! of range, leaves the map, dies, enters combat or logs out.
//!
//! The flow is vmangos' (`TradeHandler.cpp`): the asked gets `BEGIN_TRADE`, both get
//! `OPEN_WINDOW`, each change sends both the changed side (`SMSG_TRADE_STATUS_EXTENDED`), an
//! accept shows on the partner's column (`ACCEPT`) and a dropped one as `BACK_TO_TRADE`.

use benilla_protocol::messages::{
    TradeItem, TradeStatus, TradeStatusExtended, TRADE_SLOT_COUNT, TRADE_SLOT_TRADED_COUNT,
};
use benilla_protocol::SessionEvent;

use crate::characters::Char;
use crate::equipment;
use crate::inventory::{Pos, PLAYER_BAG};
use crate::{items, quests, Realm, RealmCommand, SessionId};

/// How close two players stand to trade, in yards (vmangos `TRADE_DISTANCE`).
const TRADE_YD: f32 = 11.0;

/// The trades under way.
#[derive(Default)]
pub(crate) struct Trades(Vec<Trade>);

/// One trade: the one who asked and the one asked, open once the asked accepted the request.
struct Trade {
    sides: [Side; 2],
    open: bool,
}

/// One player's half of a trade.
#[derive(Default)]
struct Side {
    guid: u64,
    /// The guid of the item offered in each traded slot.
    items: [Option<u64>; TRADE_SLOT_TRADED_COUNT],
    /// Copper.
    gold: u32,
    accepted: bool,
}

impl Trade {
    /// Which side `guid` is, if they are in this trade.
    fn side_of(&self, guid: u64) -> Option<usize> {
        self.sides.iter().position(|s| s.guid == guid)
    }
}

/// A player in the world, as a trade sees them.
struct Trader {
    session: SessionId,
    guid: u64,
    map: u32,
    position: [f32; 3],
    alive: bool,
    in_combat: bool,
}

impl RealmCommand {
    /// Whether this is one of the trade window's commands.
    pub(crate) fn is_trade(&self) -> bool {
        matches!(
            self,
            RealmCommand::InitiateTrade { .. }
                | RealmCommand::BeginTrade
                | RealmCommand::BusyTrade
                | RealmCommand::IgnoreTrade
                | RealmCommand::AcceptTrade
                | RealmCommand::UnacceptTrade
                | RealmCommand::CancelTrade
                | RealmCommand::SetTradeGold { .. }
                | RealmCommand::SetTradeItem { .. }
                | RealmCommand::ClearTradeItem { .. }
        )
    }
}

impl Realm {
    /// Everyone in the world, the served session included.
    fn traders(&self) -> Vec<Trader> {
        let worlds = std::iter::once((self.active, &self.world))
            .chain(self.parked.iter().map(|(id, s)| (*id, &s.world)));
        worlds
            .filter_map(|(session, w)| {
                let w = w.as_ref()?;
                let c = self.characters.get(w.guid)?;
                Some(Trader {
                    session,
                    guid: c.guid,
                    map: c.map,
                    position: c.position,
                    alive: w.alive(),
                    in_combat: w.in_combat(),
                })
            })
            .collect()
    }

    /// The session of the player `guid` in the world.
    fn trader_session(&self, guid: u64) -> Option<SessionId> {
        self.traders()
            .into_iter()
            .find(|t| t.guid == guid)
            .map(|t| t.session)
    }

    fn send_trade_status(&mut self, guid: u64, status: TradeStatus) {
        if let Some(session) = self.trader_session(guid) {
            self.send_to(session, SessionEvent::TradeStatus { status });
        }
    }

    /// A trade window command from the served player.
    pub(crate) fn handle_trade(&mut self, command: RealmCommand) {
        let Some(me) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        match command {
            RealmCommand::InitiateTrade { target } => self.initiate_trade(me, target),
            RealmCommand::BeginTrade => self.begin_trade(me),
            RealmCommand::BusyTrade => self.refuse_trade(me, TradeStatus::Busy),
            RealmCommand::IgnoreTrade => self.refuse_trade(me, TradeStatus::IgnoreYou),
            RealmCommand::CancelTrade => self.end_trade(me, TradeStatus::Canceled),
            RealmCommand::AcceptTrade => self.accept_trade(me),
            RealmCommand::UnacceptTrade => self.unaccept_trade(me),
            RealmCommand::SetTradeGold { copper } => {
                self.change_offer(me, |_, side| {
                    side.gold = copper;
                    true
                });
            }
            RealmCommand::SetTradeItem {
                trade_slot,
                bag,
                slot,
            } => {
                self.change_offer(me, |c, side| set_item(c, side, trade_slot, bag, slot));
            }
            RealmCommand::ClearTradeItem { trade_slot } => {
                self.change_offer(me, |_, side| {
                    match side.items.get_mut(usize::from(trade_slot)) {
                        Some(cell) => cell.take().is_some(),
                        None => false,
                    }
                });
            }
            _ => {}
        }
    }

    /// The trade `guid` is in, by index.
    fn trade_of(&self, guid: u64) -> Option<usize> {
        self.trades.0.iter().position(|t| t.side_of(guid).is_some())
    }

    fn initiate_trade(&mut self, me: u64, target: u64) {
        let all = self.traders();
        let Some(mine) = all.iter().find(|t| t.guid == me) else {
            return;
        };
        let refusal = match all.iter().find(|t| t.guid == target && t.guid != me) {
            _ if self.trade_of(me).is_some() => Some(TradeStatus::Busy),
            _ if !mine.alive => Some(TradeStatus::YouDead),
            None => Some(TradeStatus::NoTarget),
            Some(them) if !them.alive => Some(TradeStatus::TargetDead),
            Some(them) if !in_reach(mine, them) => Some(TradeStatus::TargetTooFar),
            Some(_) if self.trade_of(target).is_some() => Some(TradeStatus::Busy),
            Some(_) => None,
        };
        if let Some(status) = refusal {
            self.send_trade_status(me, status);
            return;
        }
        self.trades.0.push(Trade {
            sides: [
                Side {
                    guid: me,
                    ..Side::default()
                },
                Side {
                    guid: target,
                    ..Side::default()
                },
            ],
            open: false,
        });
        self.send_trade_status(target, TradeStatus::BeginTrade { partner: me });
    }

    /// The asked player accepts the request: both windows open.
    fn begin_trade(&mut self, me: u64) {
        let Some(i) = self.trade_of(me) else {
            return;
        };
        let trade = &mut self.trades.0[i];
        if trade.open || trade.side_of(me) != Some(1) {
            return;
        }
        trade.open = true;
        let guids = trade.sides.each_ref().map(|s| s.guid);
        for g in guids {
            self.send_trade_status(g, TradeStatus::OpenWindow);
        }
    }

    /// The asked player turns the request down: the one who asked hears `status`.
    fn refuse_trade(&mut self, me: u64, status: TradeStatus) {
        let Some(i) = self.trade_of(me) else {
            return;
        };
        if self.trades.0[i].open || self.trades.0[i].side_of(me) != Some(1) {
            return;
        }
        let trade = self.trades.0.remove(i);
        self.send_trade_status(trade.sides[0].guid, status);
    }

    /// `guid`'s trade ends unswapped: both hear `status`.
    fn end_trade(&mut self, guid: u64, status: TradeStatus) {
        let Some(i) = self.trade_of(guid) else {
            return;
        };
        let trade = self.trades.0.remove(i);
        for side in &trade.sides {
            self.send_trade_status(side.guid, status);
        }
    }

    /// `me` changes their offer with `change`, which says whether it changed anything: both
    /// accepts drop, and both clients see the new offer.
    fn change_offer(&mut self, me: u64, change: impl FnOnce(&Char, &mut Side) -> bool) {
        let Some(i) = self.trade_of(me) else {
            return;
        };
        let Some(c) = self.characters.get(me) else {
            return;
        };
        let trade = &mut self.trades.0[i];
        let Some(s) = trade.side_of(me).filter(|_| trade.open) else {
            return;
        };
        if !change(c, &mut trade.sides[s]) {
            return;
        }
        self.drop_accepts(i);
        self.show_offer(i, s);
    }

    /// Both accepts in trade `i` drop; each who had accepted sees the Trade button again, and so
    /// does their partner's column.
    fn drop_accepts(&mut self, i: usize) {
        let trade = &mut self.trades.0[i];
        if !trade.sides.iter().any(|s| s.accepted) {
            return;
        }
        trade.sides.iter_mut().for_each(|s| s.accepted = false);
        let guids = trade.sides.each_ref().map(|s| s.guid);
        for g in guids {
            self.send_trade_status(g, TradeStatus::BackToTrade);
        }
    }

    /// Side `s` of trade `i` as both clients show it: in its owner's own column and in the
    /// partner's.
    fn show_offer(&mut self, i: usize, s: usize) {
        let trade = &self.trades.0[i];
        let side = &trade.sides[s];
        let partner = trade.sides[1 - s].guid;
        let Some(c) = self.characters.get(side.guid) else {
            return;
        };
        let snapshot = |their_window| {
            let mut slots: [Option<TradeItem>; TRADE_SLOT_COUNT] = Default::default();
            for (cell, guid) in slots.iter_mut().zip(&side.items) {
                *cell = guid
                    .and_then(|g| c.inventory.iter().find(|(_, it)| it.guid == g))
                    .map(|(_, it)| trade_item(it));
            }
            SessionEvent::TradeStatusExtended {
                state: Box::new(TradeStatusExtended {
                    their_window,
                    gold: side.gold,
                    enchant_spell_id: 0,
                    slots,
                }),
            }
        };
        let (own, theirs) = (snapshot(false), snapshot(true));
        let owner = side.guid;
        if let Some(session) = self.trader_session(owner) {
            self.send_to(session, own);
        }
        if let Some(session) = self.trader_session(partner) {
            self.send_to(session, theirs);
        }
    }

    fn accept_trade(&mut self, me: u64) {
        let Some(i) = self.trade_of(me) else {
            return;
        };
        let trade = &mut self.trades.0[i];
        let Some(s) = trade.side_of(me).filter(|_| trade.open) else {
            return;
        };
        trade.sides[s].accepted = true;
        let partner = trade.sides[1 - s].guid;
        if trade.sides.iter().all(|side| side.accepted) {
            self.swap(i);
        } else {
            self.send_trade_status(partner, TradeStatus::Accept);
        }
    }

    fn unaccept_trade(&mut self, me: u64) {
        let Some(i) = self.trade_of(me) else {
            return;
        };
        let trade = &mut self.trades.0[i];
        let Some(s) = trade.side_of(me).filter(|_| trade.open) else {
            return;
        };
        trade.sides[s].accepted = false;
        let partner = trade.sides[1 - s].guid;
        self.send_trade_status(partner, TradeStatus::BackToTrade);
    }

    /// Both accepted trade `i`: everything changes hands, or, when a side lacks the gold it
    /// offers or the room for what it gets, nothing does and both accepts drop.
    fn swap(&mut self, i: usize) {
        let guids = self.trades.0[i].sides.each_ref().map(|s| s.guid);
        let (Some(a), Some(b)) = (self.characters.get(guids[0]), self.characters.get(guids[1]))
        else {
            return self.end_trade(guids[0], TradeStatus::Canceled);
        };
        let sides = &self.trades.0[i].sides;
        let offers = [offered(a, &sides[0]), offered(b, &sides[1])];
        let gold = [sides[0].gold, sides[1].gold];
        // A side whose offer is gone from its bags, or short of gold or room, refuses it.
        let short = [(a, 0), (b, 1)].map(|(c, s)| {
            let Some(mine) = &offers[s] else {
                return true;
            };
            let Some(theirs) = &offers[1 - s] else {
                return false;
            };
            if c.inventory.money < gold[s] {
                return true;
            }
            let mut room = c.inventory.clone();
            for (at, _) in mine {
                room.take(*at);
            }
            theirs
                .iter()
                .any(|(_, (entry, count))| room.store(*entry, *count).is_err())
        });
        if short.iter().any(|&s| s) {
            for (s, &lacks) in short.iter().enumerate() {
                if lacks {
                    let fail = equipment::inventory_failure(equipment::EQUIP_ERR_INVENTORY_FULL, 0);
                    if let Some(session) = self.trader_session(guids[s]) {
                        self.send_to(session, fail);
                    }
                }
            }
            self.drop_accepts(i);
            return;
        }
        let [Some(from_a), Some(from_b)] = offers else {
            return;
        };
        self.trades.0.remove(i);
        for (s, (gives, gets)) in [(0, (&from_a, &from_b)), (1, (&from_b, &from_a))] {
            let mut events = Vec::new();
            let Some(c) = self.characters.get_mut(guids[s]) else {
                continue;
            };
            for (at, _) in gives {
                equipment::destroy(c, at.bag, at.slot, 0, &mut events);
            }
            for (_, (entry, count)) in gets {
                let _ = equipment::receive(c, *entry, *count, &mut events);
            }
            c.inventory.money = c.inventory.money - gold[s] + gold[1 - s];
            events.push(crate::fields::values(
                c.guid,
                &[equipment::money_pair(&c.inventory)],
            ));
            quests::refresh(c, &mut events);
            events.push(SessionEvent::TradeStatus {
                status: TradeStatus::Complete,
            });
            if let Some(session) = self.trader_session(guids[s]) {
                for e in events {
                    self.send_to(session, e);
                }
            }
            self.save(guids[s]);
        }
    }

    /// Each tick: a trade ends when either side is gone, dead, in combat, on another map or out
    /// of reach.
    pub(crate) fn tick_trades(&mut self) {
        let all = self.traders();
        let ended: Vec<u64> = self
            .trades
            .0
            .iter()
            .filter(|t| {
                let [a, b] = t
                    .sides
                    .each_ref()
                    .map(|s| all.iter().find(|p| p.guid == s.guid));
                match (a, b) {
                    (Some(a), Some(b)) => {
                        !a.alive || !b.alive || a.in_combat || b.in_combat || !in_reach(a, b)
                    }
                    _ => true,
                }
            })
            .map(|t| t.sides[0].guid)
            .collect();
        for guid in ended {
            self.end_trade(guid, TradeStatus::Canceled);
        }
    }
}

/// Whether two players stand close enough to trade: the same map, within [`TRADE_YD`].
fn in_reach(a: &Trader, b: &Trader) -> bool {
    a.map == b.map && crate::space::flat_distance(a.position, b.position) <= TRADE_YD
}

/// Puts the item at the wire's (`bag`, `slot`) into `side`'s `trade_slot`: an item in the
/// backpack or a bag, not offered in another slot. Whether it went in.
fn set_item(c: &Char, side: &mut Side, trade_slot: u8, bag: u8, slot: u8) -> bool {
    let at = if bag == PLAYER_BAG {
        Pos::own(slot)
    } else {
        Pos { bag, slot }
    };
    let Some(item) = c.inventory.get(at).filter(|_| at.is_storage()) else {
        return false;
    };
    let guid = item.guid;
    if side.items.contains(&Some(guid)) {
        return false;
    }
    match side.items.get_mut(usize::from(trade_slot)) {
        Some(cell) => {
            *cell = Some(guid);
            true
        }
        None => false,
    }
}

/// What `side` offers from `c`'s bags: each item's place and `(entry, count)`. `None` when an
/// offered item is no longer there.
fn offered(c: &Char, side: &Side) -> Option<Vec<(Pos, (u32, u32))>> {
    side.items
        .iter()
        .flatten()
        .map(|&g| {
            c.inventory
                .iter()
                .find(|(at, it)| it.guid == g && at.is_storage())
                .map(|(at, it)| (at, (it.entry, it.count)))
        })
        .collect()
}

/// A trade slot's item block for `item`.
fn trade_item(item: &crate::inventory::Item) -> TradeItem {
    let t = items::template(item.entry);
    TradeItem {
        entry: item.entry,
        display_id: t.map_or(0, |t| t.display_id),
        count: item.count,
        wrapped: false,
        gift_creator: 0,
        perm_enchant: 0,
        creator: 0,
        charges: 0,
        suffix_factor: 0,
        random_prop_id: 0,
        lock_id: 0,
        max_durability: t.map_or(0, |t| t.durability),
        durability: item.durability,
    }
}
