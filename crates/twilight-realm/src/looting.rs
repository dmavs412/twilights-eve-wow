//! Corpses and their loot: what each kill dropped (`rules::loot`), and the stock loot window's
//! open, take-money, take-item and release (vmangos `LootHandler.cpp`, solo looting).

use std::collections::HashMap;

use benilla_protocol::messages::{loot_error, loot_type, slot_type, LootItem};
use benilla_protocol::SessionEvent;

use crate::characters::Char;
use crate::equipment;
use crate::items;
use crate::rng::Rng;
use crate::rules::loot::{money, pool, quality, NORTHSHIRE_ROLL, QUALITY_ROLL};

/// `UNIT_DYNAMIC_FLAGS`, and its `UNIT_DYNFLAG_LOOTABLE` bit: the sparkle, the loot cursor and
/// right-click looting.
const UNIT_DYNAMIC_FLAGS: u16 = 143;
const UNIT_DYNFLAG_LOOTABLE: u32 = 0x1;

/// A corpse's dynamic flags, lootable or not.
pub(crate) fn lootable_pair(lootable: bool) -> (u16, u32) {
    (
        UNIT_DYNAMIC_FLAGS,
        if lootable { UNIT_DYNFLAG_LOOTABLE } else { 0 },
    )
}

/// One corpse's loot.
#[derive(Debug, Clone)]
pub(crate) struct Loot {
    /// Copper not yet taken.
    pub(crate) money: u32,
    /// `(entry, count)` by wire slot; `None` once taken.
    pub(crate) items: Vec<Option<(u32, u32)>>,
}

impl Loot {
    /// What a mob of `level` drops: its money and one item.
    pub(crate) fn roll(level: u8, rng: &mut Rng) -> Self {
        let (lo, hi) = money(level);
        let q = quality(rng.below(QUALITY_ROLL));
        let pool = pool(level, q);
        let items = if pool.is_empty() {
            Vec::new()
        } else {
            vec![Some((pool[rng.below(pool.len() as u32) as usize], 1))]
        };
        Self {
            money: rng.urand(lo, hi),
            items,
        }
    }

    /// What a Northshire mob of `level` drops (spec #272): its money, then one roll for Gear,
    /// 8% a Token white, 4% a World green within a level of the mob's, else nothing. The vanilla
    /// pool never runs here, so there is no grey vendor trash either.
    pub(crate) fn northshire(level: u8, rng: &mut Rng) -> Self {
        use crate::rules::loot::{northshire_greens, northshire_roll, northshire_whites, Pick};
        let (lo, hi) = money(level);
        let money = rng.urand(lo, hi);
        let list: &[u32] = match northshire_roll(level, rng.below(NORTHSHIRE_ROLL)) {
            Some(Pick::White) => northshire_whites(),
            Some(Pick::Green) => northshire_greens(level),
            None => &[],
        };
        let items = if list.is_empty() {
            Vec::new()
        } else {
            vec![Some((list[rng.below(list.len() as u32) as usize], 1))]
        };
        Self { money, items }
    }

    /// What a Dungeon boss of `level` drops: her money and always one of the blues in `pool`.
    pub(crate) fn boss(level: u8, pool: &[u32], rng: &mut Rng) -> Self {
        let (lo, hi) = money(level);
        let items = if pool.is_empty() {
            Vec::new()
        } else {
            vec![Some((pool[rng.below(pool.len() as u32) as usize], 1))]
        };
        Self {
            money: rng.urand(lo, hi),
            items,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.money == 0 && self.items.iter().all(Option::is_none)
    }
}

/// The corpses with loot, and which one the player has open.
#[derive(Default)]
pub(crate) struct Looting {
    corpses: HashMap<u64, Loot>,
    open: Option<u64>,
}

/// What a loot command did that the realm acts on.
#[derive(Default)]
pub(crate) struct Looted {
    /// Money or an item went into the bags: save.
    pub(crate) took: bool,
    /// The corpse's lootable flag changed: `(corpse, still lootable)`.
    pub(crate) flag: Option<(u64, bool)>,
}

impl Looting {
    /// A kill left `loot` on `corpse`; whether there is anything to take.
    pub(crate) fn drop(&mut self, corpse: u64, loot: Loot) -> bool {
        if loot.is_empty() {
            return false;
        }
        self.corpses.insert(corpse, loot);
        true
    }

    /// Keeps only the corpses `keep` says are still lying there (the rest despawned or respawned).
    pub(crate) fn retain(&mut self, keep: impl Fn(u64) -> bool) {
        self.corpses.retain(|corpse, _| keep(*corpse));
        if self.open.is_some_and(|c| !self.corpses.contains_key(&c)) {
            self.open = None;
        }
    }

    /// The corpse or chest whose window is open.
    pub(crate) fn open_guid(&self) -> Option<u64> {
        self.open
    }

    /// What is left on `corpse`.
    pub(crate) fn loot_of(&self, corpse: u64) -> Option<&Loot> {
        self.corpses.get(&corpse)
    }

    /// A click on a chest holding `loot`: the stock window opens on it as on a corpse.
    pub(crate) fn offer(&mut self, chest: u64, loot: Loot, events: &mut Vec<SessionEvent>) {
        self.corpses.insert(chest, loot);
        self.open(chest, true, events);
    }

    /// `CMSG_LOOT` on `corpse`, which `near` says is in reach.
    pub(crate) fn open(&mut self, corpse: u64, near: bool, events: &mut Vec<SessionEvent>) {
        let Some(loot) = self.corpses.get(&corpse) else {
            events.push(SessionEvent::LootError {
                guid: corpse,
                error: loot_error::DIDNT_KILL,
            });
            return;
        };
        if !near {
            events.push(SessionEvent::LootError {
                guid: corpse,
                error: loot_error::TOO_FAR,
            });
            return;
        }
        self.open = Some(corpse);
        let items = loot
            .items
            .iter()
            .enumerate()
            .filter_map(|(slot, row)| {
                let (entry, count) = (*row)?;
                Some(LootItem {
                    slot: slot as u8,
                    item_id: entry,
                    count,
                    display_info_id: items::template(entry).map_or(0, |t| t.display_id),
                    random_property_id: 0,
                    slot_type: slot_type::ALLOW_LOOT,
                })
            })
            .collect();
        events.push(SessionEvent::LootResponse {
            guid: corpse,
            loot_type: loot_type::CORPSE,
            gold: loot.money,
            items,
        });
    }

    /// `CMSG_LOOT_MONEY`: the open corpse's copper into the purse.
    pub(crate) fn take_money(&mut self, c: &mut Char, events: &mut Vec<SessionEvent>) -> Looted {
        let Some(loot) = self.open.and_then(|g| self.corpses.get_mut(&g)) else {
            return Looted::default();
        };
        if loot.money == 0 {
            return Looted::default();
        }
        c.inventory.money = c.inventory.money.saturating_add(loot.money);
        loot.money = 0;
        // Solo, vmangos sends no `SMSG_LOOT_MONEY_NOTIFY`: the coin line clears and the purse
        // changes.
        events.push(SessionEvent::LootClearMoney);
        events.push(crate::fields::values(
            c.guid,
            &[equipment::money_pair(&c.inventory)],
        ));
        Looted {
            took: true,
            flag: None,
        }
    }

    /// `CMSG_AUTOSTORE_LOOT_ITEM`: the open corpse's row `slot` into the backpack.
    pub(crate) fn take_item(
        &mut self,
        c: &mut Char,
        slot: u8,
        events: &mut Vec<SessionEvent>,
    ) -> Looted {
        let Some(loot) = self.open.and_then(|g| self.corpses.get_mut(&g)) else {
            return Looted::default();
        };
        let Some(row) = loot.items.get_mut(usize::from(slot)) else {
            return Looted::default();
        };
        let Some((entry, count)) = *row else {
            return Looted::default();
        };
        if let Err(reason) = equipment::receive(c, entry, count, events) {
            events.push(equipment::inventory_failure(reason, 0));
            return Looted::default();
        }
        *row = None;
        events.push(SessionEvent::LootRemoved { slot });
        Looted {
            took: true,
            flag: None,
        }
    }

    /// `CMSG_LOOT_RELEASE`: the window closes; an emptied corpse stops being lootable.
    pub(crate) fn release(&mut self, events: &mut Vec<SessionEvent>) -> Looted {
        let Some(corpse) = self.open.take() else {
            return Looted::default();
        };
        events.push(SessionEvent::LootReleaseResponse { guid: corpse });
        if self.corpses.get(&corpse).is_some_and(Loot::is_empty) {
            self.corpses.remove(&corpse);
            return Looted {
                took: false,
                flag: Some((corpse, false)),
            };
        }
        Looted::default()
    }
}
