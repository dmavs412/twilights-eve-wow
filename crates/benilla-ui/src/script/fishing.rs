//! The Reeling Bar's state Lua reads (ADR 0018): `GetReelState()`, the bar the host's simulation
//! runs, and `GetAnglerInfo()`, the character's Angler Level. The host pushes both into the
//! [`Model`] and fires `FISHING_REEL_START`, `FISHING_REEL_END` and `ANGLER_UPDATE` itself.

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
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct AnglerInfo {
    pub(crate) level: u32,
    pub(crate) xp: u32,
    pub(crate) to_next: u32,
}

impl super::UiScript {
    /// Push the open bar, or `None` once it closes.
    pub fn set_reel(&mut self, view: Option<ReelView>) {
        self.model_mut().reel = view;
    }

    /// Push the Angler Level `GetAnglerInfo()` answers.
    pub fn set_angler(&mut self, level: u32, xp: u32, to_next: u32) {
        self.model_mut().angler = AnglerInfo { level, xp, to_next };
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
    // GetAnglerInfo(): level, xp, xpToNext; level 0 until the host's first push.
    g.set(
        "GetAnglerInfo",
        lua.create_function(|lua, ()| {
            let a = lua.app_data_ref::<Model>().expect("model app_data").angler;
            Ok((f64::from(a.level), f64::from(a.xp), f64::from(a.to_next)))
        })?,
    )?;
    Ok(())
}
