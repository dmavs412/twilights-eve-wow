//! The Noise Bar's state Lua reads (spec #533): `GetPhaseMeter()`, the meter of the Phased Dungeon
//! the character stands in. The host pushes it into the [`Model`] and fires `BENILLA_PHASE_METER`
//! itself, with the cause line when the stacks rose. A meter is no unit, so it takes the Reeling
//! Bar's road (`fishing.rs`) rather than a unit token.

use mlua::{Lua, MultiValue, Value};

use super::Model;

/// The meter a Phased Dungeon's bar draws.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PhaseMeterView {
    /// The meter's name, "Noise" in Forest's Edge.
    pub name: String,
    pub stacks: u32,
    /// The most stacks it holds.
    pub max: u32,
    /// The icon at the bar's full end.
    pub end_icon: String,
}

impl super::UiScript {
    /// Push the meter, or `None` once the bar comes down.
    pub fn set_phase_meter(&mut self, view: Option<PhaseMeterView>) {
        self.model_mut().phase_meter = view;
    }
}

/// Register `GetPhaseMeter`.
pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    // GetPhaseMeter(): name, stacks, max, endIcon. Nothing with no bar up.
    lua.globals().set(
        "GetPhaseMeter",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(m) = &model.phase_meter else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                Value::String(lua.create_string(&m.name)?),
                Value::Number(f64::from(m.stacks)),
                Value::Number(f64::from(m.max)),
                Value::String(lua.create_string(&m.end_icon)?),
            ]))
        })?,
    )
}
