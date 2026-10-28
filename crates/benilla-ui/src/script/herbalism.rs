//! The Forager's Grid's state Lua reads (spec #435): `GetHerbGrid()`, `GetHerbTile(i)`, the known
//! plants, the start-of-patch notes, and (#480) the plants in Bloom, the basket and the last log
//! entry; and the calls the window queues for the host: `HerbPull`, `HerbSweep`, `HerbProbe`,
//! `HerbSnip`, `HerbSight`, `HerbTwine`, `HerbArmCharm`, `HerbLeave` and `HerbTourSeen`.
//! The host pushes the open grid into the [`Model`] and fires `HERB_GRID_UPDATE` and
//! `HERB_GRID_CLOSED` itself. Tile numbers are 1-based in Lua and 0-based here.
//!
//! The Herbalism Journal tab (#443) reads `GetHerbalistInfo()`, `GetHerbGear()`, the Satchel
//! (`GetHerbSatchel`, `GetHerbSatchelItem`), the zones and plants (`GetHerbJournalZones`,
//! `GetHerbJournalZone`, `GetHerbJournalPlant`, `GetHerbRevealAt`) and the perks
//! (`GetHerbPerkSlots`, `GetHerbPerkSlot`, `GetNumHerbPerks`, `GetHerbPerk`). It queues
//! `SetHerbTool`, `SetHerbConsumable`, `PickHerbPerk` and `WithdrawHerbItem` as
//! [`HerbJournalCall`]s. The host pushes the views and fires `HERBALIST_UPDATE` and
//! `HERBALISM_JOURNAL_UPDATE`. Perk slots are 1-based in Lua and 0-based here.

use mlua::{Lua, MultiValue, Value};

use super::Model;

/// One tile of the open grid, as the realm sent it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HerbTileView {
    /// 0 hidden, 1 pulled plant, 2 pulled weed, 3 pulled thorn.
    pub state: u8,
    /// What the player knows of a hidden tile: 0 nothing, 1 a plant, 2 a weed, 3 a thorn.
    pub known: u8,
    /// The Stake on the tile; -1 for none.
    pub stake: i8,
    pub item: u32,
    pub count: u32,
    pub bloom: bool,
    /// The plant part key of a pulled or missed plant; "" otherwise.
    pub part: String,
    /// An unpulled tile shown when the patch ended.
    pub missed: bool,
}

/// One basket row of the open patch, merged by item.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HerbBasketView {
    pub item: u32,
    pub count: u32,
    pub part: String,
}

/// The last action on the open patch: kind 0 plant, 1 weed, 2 thorn, 3 tool skill, 4 consumable.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HerbLogView {
    pub kind: u8,
    pub title: String,
    pub place: String,
    pub xp: u32,
}

/// What the Journal tells about one plant in the open patch.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HerbKnownView {
    pub item: u32,
    pub present: bool,
    /// 1-based rows; empty before the Journal reveals them.
    pub rows: Vec<u8>,
}

/// The open Forager's Grid, field for field as the realm's `HerbGrid` event.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HerbGridView {
    pub size: u8,
    pub pulls_left: u8,
    pub tiles: Vec<HerbTileView>,
    pub known: Vec<HerbKnownView>,
    pub notes: Vec<String>,
    pub log: Option<HerbLogView>,
    pub tool: u32,
    pub skill_cost: u8,
    pub consumable: u32,
    pub consumable_ready: bool,
    pub charm_armed: bool,
    /// "Night", "Dawn", "Day" or "Dusk".
    pub time_block: String,
    /// The Herbalist level the patch was rolled at.
    pub grown_for: u8,
    /// The zone's plants in Bloom now, and how much a blooming plant yields.
    pub in_bloom: Vec<u32>,
    pub bloom_factor: f32,
    pub basket: Vec<HerbBasketView>,
    /// Thorns pulled and Herbalist XP gained this patch.
    pub thorns: u8,
    pub xp: u32,
    /// The `HERB_GRID_CLOSED` reason on the last grid of a patch that ran out of pulls or tiles.
    pub over: Option<u8>,
}

/// One call the window queued for the host, tiles 0-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HerbCall {
    Pull(u8),
    Sweep {
        tile: u8,
        across: bool,
    },
    Probe(u8),
    Snip(u8),
    Sight([u8; 3]),
    Twine(u8),
    ArmCharm,
    Leave,
    /// The guided tour was seen; needs no open grid.
    TourSeen,
}

/// The character's Herbalist Level and perk slots, as the realm's `Herbalist` event.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HerbalistView {
    pub knows: bool,
    pub level: u32,
    pub xp: u32,
    pub to_next: u32,
    pub title: String,
    /// One per perk slot the level opens: the picked perk's key, or "" for an empty slot.
    pub perks: Vec<String>,
    /// The Forager's Grid guided tour has been seen.
    pub tour_seen: bool,
}

/// One perk of `herbs.toml`, as the Journal lists it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HerbPerkView {
    pub key: String,
    pub name: String,
    pub boon: bool,
    pub text: String,
}

/// The Herbalism Journal, as the realm's `HerbalismJournal` event.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HerbJournalView {
    /// The Tool and Consumable slots' items; 0 for empty.
    pub tool: u32,
    pub consumable: u32,
    pub satchel: Vec<super::SatchelSlot>,
    /// `(item, tiles pulled)`.
    pub picks: Vec<(u32, u32)>,
    /// `(AreaTable id, plant items)`.
    pub zones: Vec<(u32, Vec<u32>)>,
    pub reveal_at: [u32; 3],
    pub perks: Vec<HerbPerkView>,
}

/// One call the Journal queued for the host, perk slots 0-based.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HerbJournalCall {
    SetTool(u32),
    SetConsumable(Option<u32>),
    PickPerk { slot: u8, key: String },
    Withdraw { item: u32, count: u32 },
}

impl super::UiScript {
    /// Push the character's Herbalist Level `GetHerbalistInfo()` and the perk getters answer.
    pub fn set_herbalist(&mut self, view: HerbalistView) {
        self.model_mut().herbalist = view;
    }

    /// Push the Herbalism Journal the `GetHerb*` Journal getters answer.
    pub fn set_herb_journal(&mut self, view: HerbJournalView) {
        self.model_mut().herb_journal = view;
    }

    /// Drain the Journal's `SetHerbTool`, `SetHerbConsumable`, `PickHerbPerk` and
    /// `WithdrawHerbItem` calls since the last drain, oldest first.
    pub fn take_herb_journal_calls(&mut self) -> Vec<HerbJournalCall> {
        std::mem::take(&mut self.model_mut().herb_journal_calls)
    }

    /// Push the open grid `GetHerbGrid()` and its siblings answer, or `None` once it closes.
    pub fn set_herb_grid(&mut self, view: Option<HerbGridView>) {
        self.model_mut().herb_grid = view;
    }

    /// Drain the `Herb*` calls since the last drain, oldest first.
    pub fn take_herb_calls(&mut self) -> Vec<HerbCall> {
        std::mem::take(&mut self.model_mut().herb_calls)
    }
}

/// The 0-based tile for a 1-based Lua number; `None` below 1 or past 255.
fn tile(i: i64) -> Option<u8> {
    u8::try_from(i.checked_sub(1)?).ok()
}

fn queue(lua: &Lua, call: Option<HerbCall>) {
    if let Some(call) = call {
        lua.app_data_mut::<Model>()
            .expect("model app_data")
            .herb_calls
            .push(call);
    }
}

fn num(v: impl Into<f64>) -> Value {
    Value::Number(v.into())
}

fn queue_journal(lua: &Lua, call: HerbJournalCall) {
    lua.app_data_mut::<Model>()
        .expect("model app_data")
        .herb_journal_calls
        .push(call);
}

/// The item for a slot: nil for 0.
fn item_or_nil(item: u32) -> Value {
    if item == 0 {
        Value::Nil
    } else {
        num(item)
    }
}

/// The 0-based index for a 1-based Lua number.
fn index(i: i64) -> Option<usize> {
    usize::try_from(i.checked_sub(1)?).ok()
}

/// Register the `GetHerb*` getters and the `Herb*` calls.
pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    // GetHerbGrid(): open, size, pullsLeft, tool, skillCost, consumable, consumableReady,
    // charmArmed, timeBlock, grownFor, bloomFactor, thorns, xp, over (nil until the patch ends by
    // pulls or tiles). With no grid open, `false` alone.
    g.set(
        "GetHerbGrid",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(grid) = &model.herb_grid else {
                return Ok(MultiValue::from_vec(vec![Value::Boolean(false)]));
            };
            Ok(MultiValue::from_vec(vec![
                Value::Boolean(true),
                num(grid.size),
                num(grid.pulls_left),
                num(grid.tool),
                num(grid.skill_cost),
                num(grid.consumable),
                Value::Boolean(grid.consumable_ready),
                Value::Boolean(grid.charm_armed),
                Value::String(lua.create_string(&grid.time_block)?),
                num(grid.grown_for),
                num(grid.bloom_factor),
                num(grid.thorns),
                num(grid.xp),
                grid.over.map_or(Value::Nil, num),
            ]))
        })?,
    )?;
    // GetNumHerbBloom() and GetHerbBloom(i), 1-based: the item of a plant in Bloom now.
    g.set(
        "GetNumHerbBloom",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            Ok(model.herb_grid.as_ref().map_or(0, |g| g.in_bloom.len()) as f64)
        })?,
    )?;
    g.set(
        "GetHerbBloom",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let item = index(i)
                .zip(model.herb_grid.as_ref())
                .and_then(|(i, grid)| grid.in_bloom.get(i));
            Ok(item.map(|&item| item as f64))
        })?,
    )?;
    // GetNumHerbBasket() and GetHerbBasket(i), 1-based: item, count, part.
    g.set(
        "GetNumHerbBasket",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            Ok(model.herb_grid.as_ref().map_or(0, |g| g.basket.len()) as f64)
        })?,
    )?;
    g.set(
        "GetHerbBasket",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let row = index(i)
                .zip(model.herb_grid.as_ref())
                .and_then(|(i, grid)| grid.basket.get(i));
            let Some(row) = row else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                num(row.item),
                num(row.count),
                Value::String(lua.create_string(&row.part)?),
            ]))
        })?,
    )?;
    // GetHerbLog(): kind, title, place, xp of the last action; nothing when the grid has none.
    g.set(
        "GetHerbLog",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(log) = model.herb_grid.as_ref().and_then(|g| g.log.as_ref()) else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                num(log.kind),
                Value::String(lua.create_string(&log.title)?),
                Value::String(lua.create_string(&log.place)?),
                num(log.xp),
            ]))
        })?,
    )?;
    // GetHerbTile(i), 1-based: state, known, stake (nil for none), item, count, bloom, part,
    // missed. Nothing for a tile the grid does not have.
    g.set(
        "GetHerbTile",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let t = tile(i)
                .zip(model.herb_grid.as_ref())
                .and_then(|(i, grid)| grid.tiles.get(usize::from(i)));
            let Some(t) = t else {
                return Ok(MultiValue::new());
            };
            let stake = if t.stake < 0 {
                Value::Nil
            } else {
                num(t.stake)
            };
            Ok(MultiValue::from_vec(vec![
                num(t.state),
                num(t.known),
                stake,
                num(t.item),
                num(t.count),
                Value::Boolean(t.bloom),
                Value::String(lua.create_string(&t.part)?),
                Value::Boolean(t.missed),
            ]))
        })?,
    )?;
    // GetHerbKnown(): how many plants the Journal tells about.
    g.set(
        "GetHerbKnown",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            Ok(model.herb_grid.as_ref().map_or(0, |g| g.known.len()) as f64)
        })?,
    )?;
    // GetHerbKnownItem(i), 1-based: item, present, rows as a string like "2, 4" ("" before the
    // Journal reveals them).
    g.set(
        "GetHerbKnownItem",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let k = tile(i)
                .zip(model.herb_grid.as_ref())
                .and_then(|(i, grid)| grid.known.get(usize::from(i)));
            let Some(k) = k else {
                return Ok(MultiValue::new());
            };
            let rows = k
                .rows
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            Ok(MultiValue::from_vec(vec![
                num(k.item),
                Value::Boolean(k.present),
                Value::String(lua.create_string(&rows)?),
            ]))
        })?,
    )?;
    // GetNumHerbNotes() and GetHerbNote(i), 1-based.
    g.set(
        "GetNumHerbNotes",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            Ok(model.herb_grid.as_ref().map_or(0, |g| g.notes.len()) as f64)
        })?,
    )?;
    g.set(
        "GetHerbNote",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let note = tile(i)
                .zip(model.herb_grid.as_ref())
                .and_then(|(i, grid)| grid.notes.get(usize::from(i)));
            Ok(note.map(|n| lua.create_string(n)).transpose()?)
        })?,
    )?;
    g.set(
        "HerbPull",
        lua.create_function(|lua, i: i64| {
            queue(lua, tile(i).map(HerbCall::Pull));
            Ok(())
        })?,
    )?;
    g.set(
        "HerbSweep",
        lua.create_function(|lua, (i, across): (i64, bool)| {
            queue(lua, tile(i).map(|tile| HerbCall::Sweep { tile, across }));
            Ok(())
        })?,
    )?;
    g.set(
        "HerbProbe",
        lua.create_function(|lua, i: i64| {
            queue(lua, tile(i).map(HerbCall::Probe));
            Ok(())
        })?,
    )?;
    g.set(
        "HerbSnip",
        lua.create_function(|lua, i: i64| {
            queue(lua, tile(i).map(HerbCall::Snip));
            Ok(())
        })?,
    )?;
    g.set(
        "HerbSight",
        lua.create_function(|lua, (a, b, c): (i64, i64, i64)| {
            queue(
                lua,
                tile(a)
                    .zip(tile(b))
                    .zip(tile(c))
                    .map(|((a, b), c)| HerbCall::Sight([a, b, c])),
            );
            Ok(())
        })?,
    )?;
    g.set(
        "HerbTwine",
        lua.create_function(|lua, i: i64| {
            queue(lua, tile(i).map(HerbCall::Twine));
            Ok(())
        })?,
    )?;
    g.set(
        "HerbArmCharm",
        lua.create_function(|lua, ()| {
            queue(lua, Some(HerbCall::ArmCharm));
            Ok(())
        })?,
    )?;
    g.set(
        "HerbLeave",
        lua.create_function(|lua, ()| {
            queue(lua, Some(HerbCall::Leave));
            Ok(())
        })?,
    )?;
    g.set(
        "HerbTourSeen",
        lua.create_function(|lua, ()| {
            queue(lua, Some(HerbCall::TourSeen));
            Ok(())
        })?,
    )?;
    install_journal(lua)
}

/// Register the Herbalism Journal's getters and calls.
fn install_journal(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    // GetHerbalistInfo(): knows, level, xp, xpToNext, title, tourSeen; level 0 until the host's
    // first push.
    g.set(
        "GetHerbalistInfo",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let h = &model.herbalist;
            Ok(MultiValue::from_vec(vec![
                Value::Boolean(h.knows),
                num(h.level),
                num(h.xp),
                num(h.to_next),
                Value::String(lua.create_string(&h.title)?),
                Value::Boolean(h.tour_seen),
            ]))
        })?,
    )?;
    // GetHerbGear(): the Tool and Consumable slots' items, nil for an empty slot.
    g.set(
        "GetHerbGear",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            Ok(MultiValue::from_vec(vec![
                item_or_nil(model.herb_journal.tool),
                item_or_nil(model.herb_journal.consumable),
            ]))
        })?,
    )?;
    // GetHerbSatchel(): the Satchel's row count. GetHerbSatchelItem(i), 1-based: item, count.
    g.set(
        "GetHerbSatchel",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            Ok(model.herb_journal.satchel.len() as f64)
        })?,
    )?;
    g.set(
        "GetHerbSatchelItem",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(row) = index(i).and_then(|i| model.herb_journal.satchel.get(i)) else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![num(row.item), num(row.count)]))
        })?,
    )?;
    // GetHerbJournalZones(): the zone count. GetHerbJournalZone(i): areaId, numPlants.
    // GetHerbJournalPlant(zone, j): item, picks.
    g.set(
        "GetHerbJournalZones",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            Ok(model.herb_journal.zones.len() as f64)
        })?,
    )?;
    g.set(
        "GetHerbJournalZone",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some((area, plants)) = index(i).and_then(|i| model.herb_journal.zones.get(i))
            else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                num(*area),
                num(plants.len() as f64),
            ]))
        })?,
    )?;
    g.set(
        "GetHerbJournalPlant",
        lua.create_function(|lua, (zone, j): (i64, i64)| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let item = index(zone)
                .and_then(|z| model.herb_journal.zones.get(z))
                .zip(index(j))
                .and_then(|((_, plants), j)| plants.get(j));
            let Some(&item) = item else {
                return Ok(MultiValue::new());
            };
            let picks = model
                .herb_journal
                .picks
                .iter()
                .find(|p| p.0 == item)
                .map_or(0, |p| p.1);
            Ok(MultiValue::from_vec(vec![num(item), num(picks)]))
        })?,
    )?;
    // GetHerbRevealAt(): the pick counts that unlock a Plant Page's three lines.
    g.set(
        "GetHerbRevealAt",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let [a, b, c] = model.herb_journal.reveal_at;
            Ok(MultiValue::from_vec(vec![num(a), num(b), num(c)]))
        })?,
    )?;
    // GetHerbPerkSlots(): the perk slots the level opens. GetHerbPerkSlot(i): the picked perk's
    // key, nil for an empty slot.
    g.set(
        "GetHerbPerkSlots",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            Ok(model.herbalist.perks.len() as f64)
        })?,
    )?;
    g.set(
        "GetHerbPerkSlot",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let key = index(i)
                .and_then(|i| model.herbalist.perks.get(i))
                .filter(|k| !k.is_empty());
            Ok(key.map(|k| lua.create_string(k)).transpose()?)
        })?,
    )?;
    // GetNumHerbPerks(): the perk count. GetHerbPerk(i): key, name, isBoon, text.
    g.set(
        "GetNumHerbPerks",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            Ok(model.herb_journal.perks.len() as f64)
        })?,
    )?;
    g.set(
        "GetHerbPerk",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(p) = index(i).and_then(|i| model.herb_journal.perks.get(i)) else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                Value::String(lua.create_string(&p.key)?),
                Value::String(lua.create_string(&p.name)?),
                Value::Boolean(p.boon),
                Value::String(lua.create_string(&p.text)?),
            ]))
        })?,
    )?;
    g.set(
        "SetHerbTool",
        lua.create_function(|lua, item: u32| {
            queue_journal(lua, HerbJournalCall::SetTool(item));
            Ok(())
        })?,
    )?;
    g.set(
        "SetHerbConsumable",
        lua.create_function(|lua, item: Option<u32>| {
            queue_journal(lua, HerbJournalCall::SetConsumable(item));
            Ok(())
        })?,
    )?;
    g.set(
        "PickHerbPerk",
        lua.create_function(|lua, (slot, key): (i64, String)| {
            if let Some(slot) = index(slot).and_then(|s| u8::try_from(s).ok()) {
                queue_journal(lua, HerbJournalCall::PickPerk { slot, key });
            }
            Ok(())
        })?,
    )?;
    // WithdrawHerbItem(i, one): queues moving Satchel row `i` (the whole stack, or one) to the bags.
    g.set(
        "WithdrawHerbItem",
        lua.create_function(|lua, (i, one): (i64, Option<bool>)| {
            let row = {
                let model = lua.app_data_ref::<Model>().expect("model app_data");
                index(i)
                    .and_then(|i| model.herb_journal.satchel.get(i))
                    .copied()
            };
            if let Some(row) = row {
                let count = if one == Some(true) { 1 } else { row.count };
                queue_journal(
                    lua,
                    HerbJournalCall::Withdraw {
                        item: row.item,
                        count,
                    },
                );
            }
            Ok(())
        })?,
    )?;
    Ok(())
}
