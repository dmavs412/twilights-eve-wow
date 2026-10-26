//! The Forager's Grid's state Lua reads (spec #435): `GetHerbGrid()`, `GetHerbTile(i)`, the known
//! plants and the start-of-patch notes, and the calls the window queues for the host: `HerbPull`,
//! `HerbSweep`, `HerbProbe`, `HerbSnip`, `HerbSight`, `HerbTwine`, `HerbArmCharm` and `HerbLeave`.
//! The host pushes the open grid into the [`Model`] and fires `HERB_GRID_UPDATE` and
//! `HERB_GRID_CLOSED` itself. Tile numbers are 1-based in Lua and 0-based here.

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
    pub log: String,
    pub tool: u32,
    pub skill_cost: u8,
    pub consumable: u32,
    pub consumable_ready: bool,
    pub charm_armed: bool,
}

/// One call the window queued for the host, tiles 0-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HerbCall {
    Pull(u8),
    Sweep { tile: u8, across: bool },
    Probe(u8),
    Snip(u8),
    Sight([u8; 3]),
    Twine(u8),
    ArmCharm,
    Leave,
}

impl super::UiScript {
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

/// Register the `GetHerb*` getters and the `Herb*` calls.
pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    // GetHerbGrid(): open, size, pullsLeft, tool, skillCost, consumable, consumableReady,
    // charmArmed, log. With no grid open, `false` alone.
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
                Value::String(lua.create_string(&grid.log)?),
            ]))
        })?,
    )?;
    // GetHerbTile(i), 1-based: state, known, stake (nil for none), item, count, bloom. Nothing for
    // a tile the grid does not have.
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
    Ok(())
}
