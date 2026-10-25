//! The Reeling Bar's state Lua reads (ADR 0018): `GetReelState()`, the bar the host's simulation
//! runs, and `GetAnglerInfo()`, the character's Angler Level. The host pushes both into the
//! [`Model`] and fires `FISHING_REEL_START`, `FISHING_REEL_END` and `ANGLER_UPDATE` itself. The
//! Fishing Journal's getters and `SetFishingBait` live here too, and the host fires
//! `FISHING_JOURNAL_UPDATE`.

use mlua::{Lua, MultiValue, Value};

use super::Model;

/// One frame of an open Reeling Bar. Heights and positions are fractions of the track, 0 at the
/// bottom and 1 at the top.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ReelView {
    pub zone_bottom: f32,
    pub zone_height: f32,
    /// The fish's centre.
    pub fish_pos: f32,
    /// The catch meter, 0 to 1.
    pub progress: f32,
    pub chest_shown: bool,
    /// The chest's centre.
    pub chest_pos: f32,
    /// The chest's own fill, 0 to 1.
    pub chest_fill: f32,
    pub legendary: bool,
    /// The fish's item entry.
    pub fish_item: u32,
    /// Whether the fish is inside the catch zone this frame.
    pub fish_in_zone: bool,
    /// Whether the reel button is held.
    pub reeling: bool,
}

/// The character's Angler Level and the XP toward the next.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct AnglerInfo {
    pub(crate) level: u32,
    pub(crate) xp: u32,
    pub(crate) to_next: u32,
    pub(crate) title: String,
}

/// One zone of the Fishing Journal: its name, AreaTable id, the pool's description and its fish
/// as `(item, best)`, `best` being 0 none, 1 caught, 2 perfect.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct JournalZone {
    pub name: String,
    pub zone: u32,
    pub description: String,
    pub fish: Vec<(u32, u8)>,
}

/// One Legendary Fish row: the item, its zone's name, whether its hint is found and whether it is
/// caught.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalLegend {
    pub item: u32,
    pub zone: String,
    pub hint_found: bool,
    pub caught: bool,
}

/// One fish's page, zone names already resolved. Mirrors `FishPage`: tiers the catch count has
/// not unlocked arrive empty.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct JournalPage {
    pub item: u32,
    pub caught: u32,
    pub perfect: u32,
    /// `(zone name, water name or "", catches there)`.
    pub places: Vec<(String, String, u32)>,
    pub revealed: u8,
    pub next_reveal: u32,
    /// `(zone name, water name or "")`.
    pub lives_in: Vec<(String, String)>,
    pub kinds: Vec<String>,
    pub peak_hours: Option<(u8, u8)>,
    pub favourite_bait: Option<u32>,
}

/// One `SetFishingGear` or `ClearFishingGear` call: the slot, and the bag item that fills it
/// (`None` clears it), in the Lua API's bag numbering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GearRequest {
    pub lure: bool,
    pub from: Option<(i64, u32)>,
}

/// The last `FishingJournal` the host pushed, zone names already resolved.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct JournalView {
    pub bait: Option<u32>,
    pub pole: Option<u32>,
    /// Total casts whose bobber floated.
    pub casts: u32,
    /// Total fish caught.
    pub caught: u32,
    /// `(item, durability, max durability)`.
    pub lure: Option<(u32, u32, u32)>,
    pub pages: Vec<JournalPage>,
    pub zones: Vec<JournalZone>,
    pub legends: Vec<JournalLegend>,
}

impl super::UiScript {
    /// Push the Fishing Journal `GetFishingJournal()` and its two siblings answer.
    pub fn set_fishing_journal(&mut self, view: JournalView) {
        self.model_mut().journal = view;
    }

    /// Drain the `SetFishingBait(item or nil)` calls since the last drain, oldest first.
    pub fn take_bait_requests(&mut self) -> Vec<Option<u32>> {
        std::mem::take(&mut self.model_mut().bait_requests)
    }

    /// Drain the `SetFishingGear` and `ClearFishingGear` calls since the last drain, oldest first.
    pub fn take_gear_requests(&mut self) -> Vec<GearRequest> {
        std::mem::take(&mut self.model_mut().gear_requests)
    }

    /// Push the open bar, or `None` once it closes.
    pub fn set_reel(&mut self, view: Option<ReelView>) {
        self.model_mut().reel = view;
    }

    /// Push the Angler Level `GetAnglerInfo()` answers.
    pub fn set_angler(&mut self, level: u32, xp: u32, to_next: u32, title: String) {
        self.model_mut().angler = AnglerInfo {
            level,
            xp,
            to_next,
            title,
        };
    }
}

/// Register `GetReelState` and `GetAnglerInfo`.
pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    // GetReelState(): active, zoneBottom, zoneHeight, fishPos, progress, chestShown, chestPos,
    // chestFill, legendary, fishItem, fishInZone, reeling. With no bar open, `false` alone.
    g.set(
        "GetReelState",
        lua.create_function(|lua, ()| {
            let reel = lua.app_data_ref::<Model>().expect("model app_data").reel;
            let Some(r) = reel else {
                return Ok(MultiValue::from_vec(vec![Value::Boolean(false)]));
            };
            let n = |v: f32| Value::Number(f64::from(v));
            Ok(MultiValue::from_vec(vec![
                Value::Boolean(true),
                n(r.zone_bottom),
                n(r.zone_height),
                n(r.fish_pos),
                n(r.progress),
                Value::Boolean(r.chest_shown),
                n(r.chest_pos),
                n(r.chest_fill),
                Value::Boolean(r.legendary),
                Value::Number(f64::from(r.fish_item)),
                Value::Boolean(r.fish_in_zone),
                Value::Boolean(r.reeling),
            ]))
        })?,
    )?;
    // GetAnglerInfo(): level, xp, xpToNext, title; level 0 until the host's first push.
    g.set(
        "GetAnglerInfo",
        lua.create_function(|lua, ()| {
            let a = lua
                .app_data_ref::<Model>()
                .expect("model app_data")
                .angler
                .clone();
            Ok((
                f64::from(a.level),
                f64::from(a.xp),
                f64::from(a.to_next),
                a.title,
            ))
        })?,
    )?;
    // GetFishingJournal(): baitItem (nil for none), zoneCount, casts, caught.
    g.set(
        "GetFishingJournal",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let bait = model
                .journal
                .bait
                .map_or(Value::Nil, |i| Value::Number(f64::from(i)));
            Ok(MultiValue::from_vec(vec![
                bait,
                Value::Number(model.journal.zones.len() as f64),
                Value::Number(f64::from(model.journal.casts)),
                Value::Number(f64::from(model.journal.caught)),
            ]))
        })?,
    )?;
    // GetFishingJournalZone(i), 1-based: the zone's name, AreaTable id and description, then
    // `item, best` for each fish. Nothing for an index with no zone.
    g.set(
        "GetFishingJournalZone",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let zone = usize::try_from(i - 1)
                .ok()
                .and_then(|i| model.journal.zones.get(i));
            let mut out = Vec::new();
            if let Some(zone) = zone {
                out.push(Value::String(lua.create_string(&zone.name)?));
                out.push(Value::Number(f64::from(zone.zone)));
                out.push(Value::String(lua.create_string(&zone.description)?));
                for &(item, best) in &zone.fish {
                    out.push(Value::Number(f64::from(item)));
                    out.push(Value::Number(f64::from(best)));
                }
            }
            Ok(MultiValue::from_vec(out))
        })?,
    )?;
    // GetFishingLegends(): `item, zoneName, hintFound, caught` for each Legendary Fish.
    g.set(
        "GetFishingLegends",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let mut out = Vec::new();
            for l in &model.journal.legends {
                out.push(Value::Number(f64::from(l.item)));
                out.push(Value::String(lua.create_string(&l.zone)?));
                out.push(Value::Boolean(l.hint_found));
                out.push(Value::Boolean(l.caught));
            }
            Ok(MultiValue::from_vec(out))
        })?,
    )?;
    // SetFishingBait(item or nil): queues the choice for the host to send as `SetBait`.
    g.set(
        "SetFishingBait",
        lua.create_function(|lua, item: Option<u32>| {
            lua.app_data_mut::<Model>()
                .expect("model app_data")
                .bait_requests
                .push(item);
            Ok(())
        })?,
    )?;
    // GetFishingGear(): poleItem, lureItem, lureDurability, lureMax; nil for an empty slot.
    g.set(
        "GetFishingGear",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let n = |v: u32| Value::Number(f64::from(v));
            let j = &model.journal;
            Ok(MultiValue::from_vec(vec![
                j.pole.map_or(Value::Nil, n),
                j.lure.map_or(Value::Nil, |l| n(l.0)),
                j.lure.map_or(Value::Nil, |l| n(l.1)),
                j.lure.map_or(Value::Nil, |l| n(l.2)),
            ]))
        })?,
    )?;
    // SetFishingGear("POLE" or "LURE", bag, slot): queues the item for the host to send.
    g.set(
        "SetFishingGear",
        lua.create_function(|lua, (slot, bag, at): (String, i64, u32)| {
            if let Some(lure) = gear_slot(&slot) {
                lua.app_data_mut::<Model>()
                    .expect("model app_data")
                    .gear_requests
                    .push(GearRequest {
                        lure,
                        from: Some((bag, at)),
                    });
            }
            Ok(())
        })?,
    )?;
    // ClearFishingGear("POLE" or "LURE"): queues the slot's item going back to the bags.
    g.set(
        "ClearFishingGear",
        lua.create_function(|lua, slot: String| {
            if let Some(lure) = gear_slot(&slot) {
                lua.app_data_mut::<Model>()
                    .expect("model app_data")
                    .gear_requests
                    .push(GearRequest { lure, from: None });
            }
            Ok(())
        })?,
    )?;
    // GetFishPage(item): caught, perfect, revealed, nextReveal, placeCount, livesCount, kinds,
    // peakFrom, peakTo, favouriteBait. `kinds` is comma-joined. Nil for a fish with no page.
    g.set(
        "GetFishPage",
        lua.create_function(|lua, item: u32| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(p) = model.journal.pages.iter().find(|p| p.item == item) else {
                return Ok(MultiValue::from_vec(vec![Value::Nil]));
            };
            let n = |v: u32| Value::Number(f64::from(v));
            Ok(MultiValue::from_vec(vec![
                n(p.caught),
                n(p.perfect),
                n(u32::from(p.revealed)),
                n(p.next_reveal),
                n(p.places.len() as u32),
                n(p.lives_in.len() as u32),
                Value::String(lua.create_string(p.kinds.join(","))?),
                p.peak_hours.map_or(Value::Nil, |h| n(u32::from(h.0))),
                p.peak_hours.map_or(Value::Nil, |h| n(u32::from(h.1))),
                p.favourite_bait.map_or(Value::Nil, n),
            ]))
        })?,
    )?;
    // GetFishPagePlace(item, i), 1-based: zoneName, waterName, count.
    g.set(
        "GetFishPagePlace",
        lua.create_function(|lua, (item, i): (u32, i64)| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let mut out = Vec::new();
            if let Some((zone, water, count)) = page_row(&model.journal, item, i, |p| &p.places) {
                out.push(Value::String(lua.create_string(zone)?));
                out.push(Value::String(lua.create_string(water)?));
                out.push(Value::Number(f64::from(*count)));
            }
            Ok(MultiValue::from_vec(out))
        })?,
    )?;
    // GetFishPageHabitat(item, i), 1-based: zoneName, waterName.
    g.set(
        "GetFishPageHabitat",
        lua.create_function(|lua, (item, i): (u32, i64)| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let mut out = Vec::new();
            if let Some((zone, water)) = page_row(&model.journal, item, i, |p| &p.lives_in) {
                out.push(Value::String(lua.create_string(zone)?));
                out.push(Value::String(lua.create_string(water)?));
            }
            Ok(MultiValue::from_vec(out))
        })?,
    )?;
    // BenillaGetCursorItem(): the entry of the item on the cursor, nil when it carries none. 1.12
    // has no `GetCursorInfo`.
    g.set(
        "BenillaGetCursorItem",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            Ok(match &model.cursor {
                Some(super::cursor::CursorPayload::Item(item)) => Some(f64::from(item.item_id)),
                _ => None,
            })
        })?,
    )?;
    // BenillaGetCursorItemPos(): the bag and 1-based slot the cursor's item came from, nil when it
    // carries none.
    g.set(
        "BenillaGetCursorItemPos",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            Ok(match &model.cursor {
                Some(super::cursor::CursorPayload::Item(item)) => (
                    Value::Number(item.bag as f64),
                    Value::Number(f64::from(item.slot)),
                ),
                _ => (Value::Nil, Value::Nil),
            })
        })?,
    )?;
    Ok(())
}

/// `Some(true)` for "LURE", `Some(false)` for "POLE", `None` for anything else.
fn gear_slot(name: &str) -> Option<bool> {
    match name {
        "POLE" => Some(false),
        "LURE" => Some(true),
        _ => None,
    }
}

/// Row `i` (1-based) of the list `rows` picks from `item`'s page.
fn page_row<T>(
    journal: &JournalView,
    item: u32,
    i: i64,
    rows: impl Fn(&JournalPage) -> &Vec<T>,
) -> Option<&T> {
    let page = journal.pages.iter().find(|p| p.item == item)?;
    rows(page).get(usize::try_from(i - 1).ok()?)
}
