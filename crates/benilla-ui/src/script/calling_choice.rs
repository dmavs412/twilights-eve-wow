//! The Calling Choice popup's state Lua reads (spec #414): `GetNumCallingChoices()`,
//! `GetCallingChoice(i)` and `GetCallingChoiceSkill(i, j)`. The host pushes the options with
//! [`UiScript::set_calling_choice`], which fires `CALLING_CHOICE_SHOW`, and drains the picks
//! `ChooseCalling(i)` queued.

use mlua::{Lua, MultiValue, Value};

use super::Model;

/// One skill of a Calling, resolved from `Spell.dbc` by the host.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CallingSkillView {
    pub name: String,
    /// The icon's texture path, without extension; empty with no icon.
    pub icon: String,
    pub description: String,
}

/// One Calling the popup offers.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CallingOptionView {
    /// The Advancement Quest picking it puts in the log.
    pub quest: u32,
    pub name: String,
    pub icon: String,
    /// The power bar's display name: "Rage", "Energy", "Mana" or "Focus".
    pub power: String,
    pub blurb: String,
    pub objectives: String,
    pub skills: Vec<CallingSkillView>,
}

/// The options of one `CallingChoice`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CallingChoiceView {
    pub options: Vec<CallingOptionView>,
}

impl super::UiScript {
    /// Push the options `GetCallingChoice` answers and fire `CALLING_CHOICE_SHOW`; `None` clears
    /// them without firing.
    pub fn set_calling_choice(&mut self, view: Option<CallingChoiceView>) {
        let shown = view.is_some();
        self.model_mut().calling_choice = view.unwrap_or_default();
        if shown {
            self.fire_event("CALLING_CHOICE_SHOW", Vec::new());
        }
    }

    /// Drain the quest ids `ChooseCalling` queued since the last drain, oldest first.
    pub fn take_calling_choice_requests(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.model_mut().calling_choice_requests)
    }
}

/// Register the four Calling Choice globals.
pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    // GetNumCallingChoices(): the option count.
    g.set(
        "GetNumCallingChoices",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            Ok(model.calling_choice.options.len() as f64)
        })?,
    )?;
    // GetCallingChoice(i), 1-based: name, icon, powerName, blurb, objectives, skillCount. Nothing
    // for an index with no option.
    g.set(
        "GetCallingChoice",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(o) = usize::try_from(i - 1)
                .ok()
                .and_then(|i| model.calling_choice.options.get(i))
            else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                Value::String(lua.create_string(&o.name)?),
                Value::String(lua.create_string(&o.icon)?),
                Value::String(lua.create_string(&o.power)?),
                Value::String(lua.create_string(&o.blurb)?),
                Value::String(lua.create_string(&o.objectives)?),
                Value::Number(o.skills.len() as f64),
            ]))
        })?,
    )?;
    // GetCallingChoiceSkill(i, j), both 1-based: name, icon, description.
    g.set(
        "GetCallingChoiceSkill",
        lua.create_function(|lua, (i, j): (i64, i64)| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let skill = usize::try_from(i - 1)
                .ok()
                .and_then(|i| model.calling_choice.options.get(i))
                .zip(usize::try_from(j - 1).ok())
                .and_then(|(o, j)| o.skills.get(j));
            let Some(s) = skill else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                Value::String(lua.create_string(&s.name)?),
                Value::String(lua.create_string(&s.icon)?),
                Value::String(lua.create_string(&s.description)?),
            ]))
        })?,
    )?;
    // ChooseCalling(i): queues the option's quest id for the host to send as `ChooseCalling`.
    g.set(
        "ChooseCalling",
        lua.create_function(|lua, i: i64| {
            let mut model = lua.app_data_mut::<Model>().expect("model app_data");
            let quest = usize::try_from(i - 1)
                .ok()
                .and_then(|i| model.calling_choice.options.get(i))
                .map(|o| o.quest);
            if let Some(quest) = quest {
                model.calling_choice_requests.push(quest);
            }
            Ok(())
        })?,
    )?;
    Ok(())
}
