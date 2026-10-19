//! Player trade (#162): a request opens the window on both sides, items and gold change hands
//! both ways once both accept, any change drops both accepts, and a swap that does not fit leaves
//! both sides as they were. Played by two level 1 warriors through the realm's seam.

mod common;

use benilla_protocol::messages::{TradeStatus, TradeStatusExtended};
use benilla_protocol::SessionEvent;
use common::party::*;
use common::*;

/// A white one-hander, one per stack.
const DULL_BLADE: u32 = 1384;
/// Another.
const FINE_SCIMITAR: u32 = 4560;
/// Rough Arrow: 200 a stack.
const ROUGH_ARROW: u32 = 2512;
/// The backpack's first slot, and its last.
const PACK_FIRST: u8 = 23;
const PACK_LAST: u8 = 38;
/// `INVENTORY_SLOT_BAG_0`: the wire's bag for the player's own slots.
const PLAYER_BAG: u8 = 255;
/// `EQUIP_ERR_INVENTORY_FULL`.
const INVENTORY_FULL: u8 = 50;

/// What a test puts in each player's save: their copper and their `(slot, entry, count)`s.
type Kit = (u32, Vec<(u8, u32, u32)>);

/// Player 0 carries a Dull Blade and 500 copper, player 1 a Fine Scimitar and 1000.
fn kits() -> [Kit; 2] {
    [
        (500, vec![(PACK_FIRST, DULL_BLADE, 1)]),
        (1000, vec![(PACK_FIRST, FINE_SCIMITAR, 1)]),
    ]
}

/// Two players side by side carrying `kits`, and nothing else.
fn pair(kits: [Kit; 2]) -> Party {
    let mut party = Party::edited(
        Realm::with_seed(1),
        &[1, 1],
        0,
        Box::new(NoGround),
        |i, blob| carrying(blob, &kits[i]),
    );
    let here = position_of(&party, 0);
    party.go(1, [here[0] + 2.0, here[1], here[2]], 0.0);
    party.run(0.2);
    party
}

/// `blob` with its items and money swapped for `kit`'s.
fn carrying(mut blob: String, (money, items): &Kit) -> String {
    if let Some(at) = blob.find("[[items]]") {
        blob.truncate(at);
    }
    let mut blob = blob
        .replace("money = 0\n", &format!("money = {money}\n"))
        .replace("items = []\n", "");
    for (slot, entry, count) in items {
        blob.push_str(&format!(
            "\n[[items]]\nslot = {slot}\nentry = {entry}\ncount = {count}\n"
        ));
    }
    blob
}

/// Where player `i` stands, from their own create.
fn position_of(party: &Party, i: usize) -> [f32; 3] {
    let guid = party.guids[i];
    party
        .seen(i)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g, position, ..
            } if *g == guid => Some(*position),
            _ => None,
        })
        .expect("the player's own create")
}

/// Player 0 asks player 1, who accepts: the window is open on both sides.
fn opened(party: &mut Party) {
    let b = party.guids[1];
    party.cmd(0, RealmCommand::InitiateTrade { target: b });
    party.cmd(1, RealmCommand::BeginTrade);
}

fn statuses(events: &[SessionEvent]) -> Vec<TradeStatus> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::TradeStatus { status } => Some(*status),
            _ => None,
        })
        .collect()
}

/// The last snapshot of a column in `events`: the partner's when `theirs`.
fn column(events: &[SessionEvent], theirs: bool) -> Option<&TradeStatusExtended> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::TradeStatusExtended { state } if state.their_window == theirs => {
            Some(state.as_ref())
        }
        _ => None,
    })
}

/// What player `i` has after the last command: their save's copper and the item entries in it.
fn holdings(party: &Party, i: usize) -> (u32, Vec<u32>) {
    let blob = party.realm.export_character(party.guids[i]).unwrap();
    let money = blob
        .lines()
        .find_map(|l| l.strip_prefix("money = "))
        .map_or(0, |m| m.parse().unwrap());
    let entries = blob
        .lines()
        .filter_map(|l| l.strip_prefix("entry = "))
        .map(|e| e.parse().unwrap())
        .collect();
    (money, entries)
}

/// Since how many events player `i` had been sent.
fn since(party: &Party, i: usize, mark: usize) -> Vec<SessionEvent> {
    party.seen(i)[mark..].to_vec()
}

#[test]
fn a_request_opens_the_window_on_both_sides() {
    let mut party = pair(kits());
    let a = party.guids[0];
    let b = party.guids[1];
    party.cmd(0, RealmCommand::InitiateTrade { target: b });
    assert_eq!(
        statuses(party.seen(1)),
        [TradeStatus::BeginTrade { partner: a }]
    );
    assert!(statuses(party.seen(0)).is_empty(), "the asker waits");
    party.cmd(1, RealmCommand::BeginTrade);
    assert_eq!(statuses(party.seen(0)), [TradeStatus::OpenWindow]);
    assert_eq!(
        statuses(party.seen(1)),
        [
            TradeStatus::BeginTrade { partner: a },
            TradeStatus::OpenWindow
        ]
    );
}

#[test]
fn items_and_gold_change_hands_both_ways() {
    let mut party = pair(kits());
    opened(&mut party);
    party.cmd(0, set(0, PACK_FIRST));
    party.cmd(0, RealmCommand::SetTradeGold { copper: 100 });
    party.cmd(1, set(0, PACK_FIRST));
    party.cmd(1, RealmCommand::SetTradeGold { copper: 300 });

    let offer = column(party.seen(1), true).expect("player 1 sees player 0's offer");
    assert_eq!(offer.gold, 100);
    assert_eq!(offer.slots[0].as_ref().map(|i| i.entry), Some(DULL_BLADE));
    let own = column(party.seen(0), false).expect("player 0 sees their own");
    assert_eq!(own.slots[0].as_ref().map(|i| i.entry), Some(DULL_BLADE));

    party.cmd(0, RealmCommand::AcceptTrade);
    assert_eq!(statuses(party.seen(1)).last(), Some(&TradeStatus::Accept));
    party.cmd(1, RealmCommand::AcceptTrade);
    for i in 0..2 {
        assert_eq!(statuses(party.seen(i)).last(), Some(&TradeStatus::Complete));
    }
    assert_eq!(holdings(&party, 0), (500 - 100 + 300, vec![FINE_SCIMITAR]));
    assert_eq!(holdings(&party, 1), (1000 - 300 + 100, vec![DULL_BLADE]));
}

#[test]
fn a_change_after_an_accept_drops_both_accepts() {
    let mut party = pair(kits());
    opened(&mut party);
    party.cmd(0, set(0, PACK_FIRST));
    party.cmd(0, RealmCommand::AcceptTrade);
    let marks = [party.seen(0).len(), party.seen(1).len()];
    party.cmd(1, RealmCommand::SetTradeGold { copper: 50 });
    for i in 0..2 {
        assert_eq!(
            statuses(&since(&party, i, marks[i])),
            [TradeStatus::BackToTrade]
        );
    }
    // Player 0's accept went with the change: player 1's alone swaps nothing.
    party.cmd(1, RealmCommand::AcceptTrade);
    assert!(!statuses(party.seen(1)).contains(&TradeStatus::Complete));
    party.cmd(0, RealmCommand::AcceptTrade);
    assert_eq!(statuses(party.seen(0)).last(), Some(&TradeStatus::Complete));
    assert_eq!(holdings(&party, 0), (550, vec![]));
    assert_eq!(holdings(&party, 1), (950, vec![FINE_SCIMITAR, DULL_BLADE]));
}

#[test]
fn a_swap_into_full_bags_is_refused_and_changes_nothing() {
    // Player 1's backpack is full of whole stacks of arrows.
    let full: Vec<(u8, u32, u32)> = (PACK_FIRST..=PACK_LAST)
        .map(|slot| (slot, ROUGH_ARROW, 200))
        .collect();
    let mut party = pair([kits()[0].clone(), (1000, full)]);
    opened(&mut party);
    party.cmd(0, set(0, PACK_FIRST));
    party.cmd(0, RealmCommand::SetTradeGold { copper: 100 });
    let before = [holdings(&party, 0), holdings(&party, 1)];
    party.cmd(0, RealmCommand::AcceptTrade);
    party.cmd(1, RealmCommand::AcceptTrade);
    assert!(
        party.seen(1).iter().any(|e| matches!(e,
            SessionEvent::InventoryFailure { reason, .. } if *reason == INVENTORY_FULL)),
        "the full side is told"
    );
    for i in 0..2 {
        let got = statuses(party.seen(i));
        assert!(!got.contains(&TradeStatus::Complete));
        assert_eq!(
            got.last(),
            Some(&TradeStatus::BackToTrade),
            "the window stays open"
        );
    }
    assert_eq!([holdings(&party, 0), holdings(&party, 1)], before);
}

#[test]
fn a_cancel_or_walking_away_ends_the_trade() {
    let mut party = pair(kits());
    opened(&mut party);
    party.cmd(1, RealmCommand::CancelTrade);
    for i in 0..2 {
        assert_eq!(statuses(party.seen(i)).last(), Some(&TradeStatus::Canceled));
    }

    opened(&mut party);
    assert_eq!(
        statuses(party.seen(0)).last(),
        Some(&TradeStatus::OpenWindow)
    );
    let here = position_of(&party, 0);
    party.go(0, [here[0] + 30.0, here[1], here[2]], 0.0);
    party.run(0.2);
    for i in 0..2 {
        assert_eq!(statuses(party.seen(i)).last(), Some(&TradeStatus::Canceled));
    }
    assert_eq!(holdings(&party, 0), (500, vec![DULL_BLADE]));
}

#[test]
fn a_request_out_of_reach_is_refused() {
    let mut party = pair(kits());
    let here = position_of(&party, 0);
    party.go(1, [here[0] + 30.0, here[1], here[2]], 0.0);
    let b = party.guids[1];
    party.cmd(0, RealmCommand::InitiateTrade { target: b });
    assert_eq!(statuses(party.seen(0)), [TradeStatus::TargetTooFar]);
    assert!(statuses(party.seen(1)).is_empty());
}

/// Offers the backpack item at `slot` in trade slot `trade_slot`.
fn set(trade_slot: u8, slot: u8) -> RealmCommand {
    RealmCommand::SetTradeItem {
        trade_slot,
        bag: PLAYER_BAG,
        slot,
    }
}
