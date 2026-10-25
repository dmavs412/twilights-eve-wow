//! A character's belongings: the items in its player slots, the items inside its equipped bags,
//! and its money. Slots are numbered as the wire numbers them (vmangos `EquipmentSlots`,
//! `InventorySlots`): 0-18 equipment, 19-22 the bag slots, 23-38 the backpack. An item inside a
//! bag is at `(bag slot, slot in the bag)`, the wire's own address for it.

use std::collections::BTreeMap;

use benilla_protocol::{SatchelPick, SatchelRow};

use crate::{fish, items};

/// Every player slot the realm keeps.
pub(crate) const SLOTS: usize = 39;
/// `EQUIPMENT_SLOT_END`: slots below it are worn.
pub(crate) const EQUIPMENT_END: u8 = 19;
/// `INVENTORY_SLOT_BAG_START..END`: the four bag slots.
pub(crate) const BAG_SLOTS: std::ops::Range<u8> = 19..23;
/// `INVENTORY_SLOT_ITEM_START..END`: the backpack.
pub(crate) const PACK: std::ops::Range<u8> = 23..39;
/// `INVENTORY_SLOT_BAG_0`: the wire's bag for the player's own slots.
pub(crate) const PLAYER_BAG: u8 = 255;

/// `HIGHGUID_ITEM`, in an item guid's top 16 bits.
const HIGHGUID_ITEM: u64 = 0x4000;

/// Where an item sits, as the wire addresses it: `bag` is [`PLAYER_BAG`] for the player's own
/// slots, or the bag slot (19-22) of the equipped bag that holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Pos {
    pub(crate) bag: u8,
    pub(crate) slot: u8,
}

impl Pos {
    /// Player slot `slot`.
    pub(crate) const fn own(slot: u8) -> Self {
        Self {
            bag: PLAYER_BAG,
            slot,
        }
    }

    /// The player slot, if it is one.
    pub(crate) fn own_slot(self) -> Option<u8> {
        (self.bag == PLAYER_BAG).then_some(self.slot)
    }

    /// Worn equipment, whose stats count.
    pub(crate) fn is_worn(self) -> bool {
        self.own_slot().is_some_and(|s| s < EQUIPMENT_END)
    }

    /// One of the four bag slots.
    pub(crate) fn is_bag_slot(self) -> bool {
        self.own_slot().is_some_and(|s| BAG_SLOTS.contains(&s))
    }

    /// Storage: the backpack or inside a bag.
    pub(crate) fn is_storage(self) -> bool {
        match self.own_slot() {
            Some(s) => PACK.contains(&s),
            None => true,
        }
    }
}

impl From<u8> for Pos {
    fn from(slot: u8) -> Self {
        Self::own(slot)
    }
}

/// One item instance.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Item {
    pub(crate) entry: u32,
    pub(crate) count: u32,
    pub(crate) durability: u32,
    /// Its object guid this session; not saved, handed out afresh at each load.
    pub(crate) guid: u64,
}

impl Item {
    /// Whether it adds its stats when worn: not broken (vmangos `_ApplyItemMods` skips an item at
    /// zero durability).
    pub(crate) fn gives_stats(&self) -> bool {
        items::template(self.entry).is_some_and(|t| t.durability == 0 || self.durability > 0)
    }
}

/// `ITEM_CLASS_QUIVER`: a quiver (subclass 2) or an ammo pouch (3).
const CLASS_QUIVER: u32 = 11;
/// `ITEM_CLASS_PROJECTILE`, whose subclass 2 is arrows and 3 bullets.
const CLASS_PROJECTILE: u32 = 6;

/// Whether the bag `bag` (an entry) takes `entry`: a quiver only arrows, an ammo pouch only
/// bullets (vmangos `ItemCanGoIntoBag`), any other bag anything.
pub(crate) fn bag_holds(bag: u32, entry: u32) -> bool {
    match items::template(bag) {
        Some(b) if b.class == CLASS_QUIVER => items::template(entry)
            .is_some_and(|t| t.class == CLASS_PROJECTILE && t.subclass == b.subclass),
        _ => true,
    }
}

/// Whether the bag `bag` is a quiver or ammo pouch, which ammo goes into first.
fn special(bag: u32) -> bool {
    items::template(bag).is_some_and(|b| b.class == CLASS_QUIVER)
}

/// How many items a bag of `entry` holds, 0 for an item that is not a bag.
pub(crate) fn bag_size(entry: u32) -> u8 {
    items::template(entry).map_or(0, |t| t.container_slots.min(36) as u8)
}

/// A Journal's Satchel (ADR 0023): the Journal Items a character holds outside its bags. An item
/// with no durability is a count per entry; one with durability is a piece with its own wear.
#[derive(Debug, Clone, Default)]
pub(crate) struct Satchel {
    pub(crate) counts: BTreeMap<u32, u32>,
    /// In arrival order.
    pub(crate) pieces: Vec<Item>,
}

/// Whether `entry` is a Journal Item: its row names a profession.
pub(crate) fn is_journal(entry: u32) -> bool {
    items::template(entry).is_some_and(|t| t.journal.is_some())
}

/// Whether a Journal Item is a piece rather than a count.
fn is_piece(entry: u32) -> bool {
    items::template(entry).is_some_and(|t| t.durability > 0)
}

impl Satchel {
    /// The rows of `profession`'s tab: counted rows by entry ascending, then the pieces.
    pub(crate) fn rows(&self, profession: items::Profession) -> Vec<SatchelRow> {
        let of = |entry: u32| items::template(entry).and_then(|t| t.journal) == Some(profession);
        let tackle = |entry: u32| {
            fish::pole(entry).is_some()
                || fish::lure(entry).is_some()
                || fish::bait(entry).is_some()
        };
        let counted = self
            .counts
            .iter()
            .filter(|(entry, count)| of(**entry) && **count > 0)
            .map(|(&item, &count)| SatchelRow {
                item,
                count,
                wear: None,
                tackle: tackle(item),
            });
        let pieces = self
            .pieces
            .iter()
            .filter(|p| of(p.entry))
            .map(|p| SatchelRow {
                item: p.entry,
                count: 1,
                wear: Some((
                    p.durability,
                    items::template(p.entry).map_or(0, |t| t.durability),
                )),
                tackle: tackle(p.entry),
            });
        counted.chain(pieces).collect()
    }

    /// How many of `entry` the Satchel holds, a piece counting one.
    pub(crate) fn count(&self, entry: u32) -> u32 {
        self.counts.get(&entry).copied().unwrap_or(0)
            + self.pieces.iter().filter(|p| p.entry == entry).count() as u32
    }

    /// Puts a bag item in: a piece keeps its durability.
    pub(crate) fn deposit(&mut self, item: Item) {
        if is_piece(item.entry) {
            self.pieces.push(item);
        } else {
            *self.counts.entry(item.entry).or_insert(0) += item.count;
        }
    }

    /// Removes up to `count` of `entry`, counted first, then the latest pieces; how many left.
    pub(crate) fn take(&mut self, entry: u32, count: u32) -> u32 {
        let mut left = count;
        if let Some(have) = self.counts.get_mut(&entry) {
            let n = left.min(*have);
            *have -= n;
            left -= n;
            if *have == 0 {
                self.counts.remove(&entry);
            }
        }
        while left > 0 {
            let Some(at) = self.pieces.iter().rposition(|p| p.entry == entry) else {
                break;
            };
            self.pieces.remove(at);
            left -= 1;
        }
        count - left
    }

    /// The index of the piece `pick` names: the first with its durability, else the last of the
    /// entry to arrive.
    fn piece_at(&self, pick: SatchelPick) -> Option<usize> {
        match pick.durability {
            Some(d) => self
                .pieces
                .iter()
                .position(|p| p.entry == pick.item && p.durability == d),
            None => self.pieces.iter().rposition(|p| p.entry == pick.item),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Inventory {
    /// The Journal Items held outside the bags.
    pub(crate) satchel: Satchel,
    slots: [Option<Item>; SLOTS],
    /// What each equipped bag holds, by bag slot (19-22), sized by the bag; empty with no bag.
    bags: [Vec<Option<Item>>; 4],
    /// Copper.
    pub(crate) money: u32,
    /// The ammo's entry (`PLAYER_AMMO_ID`), 0 for none; the stacks stay in the bags.
    pub(crate) ammo: u32,
    /// The last item guid counter handed out.
    last_guid: u64,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            satchel: Satchel::default(),
            slots: std::array::from_fn(|_| None),
            bags: std::array::from_fn(|_| Vec::new()),
            money: 0,
            ammo: 0,
            last_guid: 0,
        }
    }
}

/// The bags have no room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Full;

impl Inventory {
    fn cell(&self, at: Pos) -> Option<&Option<Item>> {
        match at.own_slot() {
            Some(s) => self.slots.get(usize::from(s)),
            None => self.bag_cells(at.bag)?.get(usize::from(at.slot)),
        }
    }

    fn cell_mut(&mut self, at: Pos) -> Option<&mut Option<Item>> {
        match at.own_slot() {
            Some(s) => self.slots.get_mut(usize::from(s)),
            None => {
                let i = usize::from(at.bag.checked_sub(BAG_SLOTS.start)?);
                self.bags.get_mut(i)?.get_mut(usize::from(at.slot))
            }
        }
    }

    fn bag_cells(&self, bag: u8) -> Option<&Vec<Option<Item>>> {
        self.bags
            .get(usize::from(bag.checked_sub(BAG_SLOTS.start)?))
    }

    /// Whether `at` names a place an item can be: a player slot, or a slot of an equipped bag.
    pub(crate) fn exists(&self, at: Pos) -> bool {
        self.cell(at).is_some()
    }

    pub(crate) fn get(&self, at: impl Into<Pos>) -> Option<&Item> {
        self.cell(at.into())?.as_ref()
    }

    pub(crate) fn get_mut(&mut self, at: impl Into<Pos>) -> Option<&mut Item> {
        self.cell_mut(at.into())?.as_mut()
    }

    /// Takes the item at `at`. A bag taken from a bag slot leaves no bag there; the caller has
    /// made sure it held nothing.
    pub(crate) fn take(&mut self, at: impl Into<Pos>) -> Option<Item> {
        let at = at.into();
        let item = self.cell_mut(at)?.take();
        if at.is_bag_slot() {
            let i = usize::from(at.slot - BAG_SLOTS.start);
            debug_assert!(self.bags[i].iter().all(Option::is_none), "an empty bag");
            self.bags[i].clear();
        }
        item
    }

    /// Puts `item` at `at`, which must exist. A bag put in a bag slot opens its own slots.
    pub(crate) fn put(&mut self, at: impl Into<Pos>, item: Item) {
        let at = at.into();
        if at.is_bag_slot() {
            let i = usize::from(at.slot - BAG_SLOTS.start);
            self.bags[i] = vec![None; usize::from(bag_size(item.entry))];
        }
        if let Some(cell) = self.cell_mut(at) {
            *cell = Some(item);
        }
    }

    /// Every occupied place and its item: the player slots in order, then each bag's contents.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (Pos, &Item)> {
        let own = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|item| (Pos::own(i as u8), item)));
        let bagged = BAG_SLOTS.zip(self.bags.iter()).flat_map(|(bag, cells)| {
            cells
                .iter()
                .enumerate()
                .filter_map(move |(i, s)| s.as_ref().map(|item| (Pos { bag, slot: i as u8 }, item)))
        });
        own.chain(bagged)
    }

    /// The items inside the bag in bag slot `bag`, by its slot; empty for no bag.
    pub(crate) fn bag_contents(&self, bag: u8) -> &[Option<Item>] {
        self.bag_cells(bag).map_or(&[], Vec::as_slice)
    }

    /// Whether the bag in bag slot `bag` holds anything.
    pub(crate) fn bag_holds_items(&self, bag: u8) -> bool {
        self.bag_contents(bag).iter().any(Option::is_some)
    }

    /// The worn items that add their stats.
    pub(crate) fn worn_entries(&self) -> Vec<u32> {
        self.iter()
            .filter(|(at, item)| at.is_worn() && item.gives_stats())
            .map(|(_, item)| item.entry)
            .collect()
    }

    /// A new item guid.
    pub(crate) fn new_guid(&mut self) -> u64 {
        self.last_guid += 1;
        HIGHGUID_ITEM << 48 | self.last_guid
    }

    /// A fresh item of `entry`, at full durability.
    pub(crate) fn new_item(&mut self, entry: u32, count: u32) -> Item {
        Item {
            entry,
            count,
            durability: items::template(entry).map_or(0, |t| t.durability),
            guid: self.new_guid(),
        }
    }

    /// Every storage place: the backpack, then each equipped bag's slots in order.
    fn storage(&self) -> Vec<Pos> {
        let mut places: Vec<Pos> = PACK.map(Pos::own).collect();
        for bag in BAG_SLOTS {
            let n = self.bag_contents(bag).len() as u8;
            places.extend((0..n).map(|slot| Pos { bag, slot }));
        }
        places
    }

    /// The first empty storage place that takes `entry`: the backpack first, then the bags in
    /// order.
    pub(crate) fn free_slot(&self, entry: u32) -> Option<Pos> {
        self.storage()
            .into_iter()
            .find(|p| self.get(*p).is_none() && self.takes(*p, entry))
    }

    /// Whether the storage place `at` may hold `entry`: anything in the backpack, what its bag
    /// holds in a bag.
    pub(crate) fn takes(&self, at: Pos, entry: u32) -> bool {
        if !BAG_SLOTS.contains(&at.bag) {
            return true;
        }
        self.get(Pos::own(at.bag))
            .is_none_or(|bag| bag_holds(bag.entry, entry))
    }

    /// The best ranged haste of the quivers and ammo pouches in the bag slots: one counts, as in
    /// vanilla.
    pub(crate) fn quiver_haste_pct(&self) -> u32 {
        BAG_SLOTS
            .filter_map(|slot| self.get(slot))
            .filter_map(|bag| items::template(bag.entry))
            .map(|t| t.ranged_haste_pct)
            .max()
            .unwrap_or(0)
    }

    /// The first empty slot of the bag in bag slot `bag`.
    pub(crate) fn free_slot_in_bag(&self, bag: u8) -> Option<Pos> {
        let n = self.bag_contents(bag).len() as u8;
        (0..n)
            .map(|slot| Pos { bag, slot })
            .find(|p| self.get(*p).is_none())
    }

    /// Stores `count` of `entry`, onto its stacks with room first and then into free places
    /// (vmangos `Player::CanStoreItem`), the backpack before the bags, all or nothing. The places
    /// it changed, and whether each now holds a new item. A Journal Item goes to the Satchel
    /// instead, never fails and changes no place.
    pub(crate) fn store(&mut self, entry: u32, count: u32) -> Result<Vec<(Pos, bool)>, Full> {
        if is_journal(entry) {
            if is_piece(entry) {
                for _ in 0..count {
                    let piece = self.new_item(entry, 1);
                    self.satchel.pieces.push(piece);
                }
            } else {
                *self.satchel.counts.entry(entry).or_insert(0) += count;
            }
            return Ok(Vec::new());
        }
        self.store_in_bags(entry, count)
    }

    /// How many of `entry` the bags have room for.
    fn bag_room(&self, entry: u32) -> u32 {
        let stack = items::template(entry).map_or(1, |t| t.stack_size());
        self.storage()
            .into_iter()
            .filter(|at| self.takes(*at, entry))
            .map(|at| match self.get(at) {
                None => stack,
                Some(item) if item.entry == entry => stack.saturating_sub(item.count),
                Some(_) => 0,
            })
            .sum()
    }

    /// Moves up to `count` of the Satchel entry `pick` names into the bags. A counted row moves
    /// what fits; a piece moves one, with its durability. The number moved and the places
    /// changed; `Full` when nothing fits.
    pub(crate) fn withdraw(
        &mut self,
        pick: SatchelPick,
        count: u32,
    ) -> Result<(u32, Vec<(Pos, bool)>), Full> {
        let room = self.bag_room(pick.item);
        if is_piece(pick.item) {
            let Some(at) = self.satchel.piece_at(pick) else {
                return Ok((0, Vec::new()));
            };
            if room == 0 {
                return Err(Full);
            }
            let piece = self.satchel.pieces.remove(at);
            let changed = self.store_in_bags(piece.entry, 1)?;
            if let Some(&(at, _)) = changed.first() {
                if let Some(item) = self.get_mut(at) {
                    item.durability = piece.durability;
                }
            }
            return Ok((1, changed));
        }
        let have = self.satchel.counts.get(&pick.item).copied().unwrap_or(0);
        let n = count.min(have).min(room);
        if n == 0 {
            return if have > 0 && count > 0 {
                Err(Full)
            } else {
                Ok((0, Vec::new()))
            };
        }
        let changed = self.store_in_bags(pick.item, n)?;
        self.satchel.take(pick.item, n);
        Ok((n, changed))
    }

    /// [`Inventory::store`] onto the bags alone.
    fn store_in_bags(&mut self, entry: u32, count: u32) -> Result<Vec<(Pos, bool)>, Full> {
        let stack = items::template(entry).map_or(1, |t| t.stack_size());
        // Where it may go, a quiver or ammo pouch first (vmangos `CanStoreItem`'s special bags).
        let mut places: Vec<Pos> = self
            .storage()
            .into_iter()
            .filter(|at| self.takes(*at, entry))
            .collect();
        places.sort_by_key(|at| {
            !(BAG_SLOTS.contains(&at.bag)
                && self.get(Pos::own(at.bag)).is_some_and(|b| special(b.entry)))
        });
        // Plan first, so a store that does not fit changes nothing.
        let mut left = count;
        let mut plan = Vec::new();
        for &at in &places {
            if left == 0 {
                break;
            }
            if let Some(item) = self.get(at) {
                if item.entry == entry && item.count < stack {
                    let add = left.min(stack - item.count);
                    plan.push((at, add, false));
                    left -= add;
                }
            }
        }
        for &at in &places {
            if left == 0 {
                break;
            }
            if self.get(at).is_none() {
                let add = left.min(stack);
                plan.push((at, add, true));
                left -= add;
            }
        }
        if left > 0 {
            return Err(Full);
        }
        Ok(plan
            .into_iter()
            .map(|(at, add, new)| {
                if new {
                    let item = self.new_item(entry, add);
                    self.put(at, item);
                } else if let Some(item) = self.get_mut(at) {
                    item.count += add;
                }
                (at, new)
            })
            .collect())
    }

    /// Whether [`Inventory::store`] would take `count` of `entry`.
    pub(crate) fn fits(&self, entry: u32, count: u32) -> bool {
        self.clone().store(entry, count).is_ok()
    }

    /// How many of `entry` the backpack, bags and Satchel hold.
    pub(crate) fn count(&self, entry: u32) -> u32 {
        self.storage()
            .into_iter()
            .filter_map(|p| self.get(p))
            .filter(|i| i.entry == entry)
            .map(|i| i.count)
            .sum::<u32>()
            + self.satchel.count(entry)
    }
}
