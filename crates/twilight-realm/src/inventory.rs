//! A character's belongings: the items in its player slots and its money. Slots are numbered as
//! the wire numbers them (vmangos `EquipmentSlots`, `InventorySlots`): 0-18 equipment, 19-22 the
//! bag slots, 23-38 the backpack. Equipped bags (and their contents) are not modelled yet.

use crate::items;

/// Every player slot the realm keeps.
pub(crate) const SLOTS: usize = 39;
/// `EQUIPMENT_SLOT_END`: slots below it are worn.
pub(crate) const EQUIPMENT_END: u8 = 19;
/// `INVENTORY_SLOT_ITEM_START..END`: the backpack.
pub(crate) const PACK: std::ops::Range<u8> = 23..39;

/// `HIGHGUID_ITEM`, in an item guid's top 16 bits.
const HIGHGUID_ITEM: u64 = 0x4000;

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

#[derive(Debug, Clone)]
pub(crate) struct Inventory {
    slots: [Option<Item>; SLOTS],
    /// Copper.
    pub(crate) money: u32,
    /// The last item guid counter handed out.
    last_guid: u64,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            money: 0,
            last_guid: 0,
        }
    }
}

/// The backpack has no room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Full;

impl Inventory {
    pub(crate) fn get(&self, slot: u8) -> Option<&Item> {
        self.slots.get(usize::from(slot))?.as_ref()
    }

    pub(crate) fn get_mut(&mut self, slot: u8) -> Option<&mut Item> {
        self.slots.get_mut(usize::from(slot))?.as_mut()
    }

    pub(crate) fn take(&mut self, slot: u8) -> Option<Item> {
        self.slots.get_mut(usize::from(slot))?.take()
    }

    /// Puts `item` in `slot`, which must be a player slot.
    pub(crate) fn put(&mut self, slot: u8, item: Item) {
        self.slots[usize::from(slot)] = Some(item);
    }

    /// Every occupied slot and its item, in slot order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (u8, &Item)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|item| (i as u8, item)))
    }

    /// The worn items that add their stats.
    pub(crate) fn worn_entries(&self) -> Vec<u32> {
        self.iter()
            .filter(|(slot, item)| *slot < EQUIPMENT_END && item.gives_stats())
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

    /// The first empty backpack slot.
    pub(crate) fn free_pack_slot(&self) -> Option<u8> {
        PACK.clone().find(|s| self.get(*s).is_none())
    }

    /// Stores `count` of `entry` in the backpack, onto its stacks with room first and then into
    /// free slots (vmangos `Player::CanStoreItem`), all or nothing. The slots it changed, and
    /// whether each now holds a new item.
    pub(crate) fn store(&mut self, entry: u32, count: u32) -> Result<Vec<(u8, bool)>, Full> {
        let stack = items::template(entry).map_or(1, |t| t.stack_size());
        // Plan first, so a store that does not fit changes nothing.
        let mut left = count;
        let mut plan = Vec::new();
        for slot in PACK {
            if left == 0 {
                break;
            }
            if let Some(item) = self.get(slot) {
                if item.entry == entry && item.count < stack {
                    let add = left.min(stack - item.count);
                    plan.push((slot, add, false));
                    left -= add;
                }
            }
        }
        for slot in PACK {
            if left == 0 {
                break;
            }
            if self.get(slot).is_none() {
                let add = left.min(stack);
                plan.push((slot, add, true));
                left -= add;
            }
        }
        if left > 0 {
            return Err(Full);
        }
        Ok(plan
            .into_iter()
            .map(|(slot, add, new)| {
                if new {
                    let item = self.new_item(entry, add);
                    self.put(slot, item);
                } else if let Some(item) = self.get_mut(slot) {
                    item.count += add;
                }
                (slot, new)
            })
            .collect())
    }
}
