//! The Talents panel's state (ADR 0028, spec #498, `assets/ui/Talents.xml`): the current
//! Calling's Talent Rows as the realm sent them. Lua reads `GetTalentRows()`, `GetTalentRow(i)`
//! and `GetTalentChoice(i, j)`; the panel queues `SetTalentRowChoice(i, j)` and
//! `OpenTalentRows()` for the host, which sends them to the realm. The host pushes the rows and
//! fires `TALENT_ROWS_UPDATE`. The client decides nothing: the realm refuses a locked row or a
//! set row away from an innkeeper.
//!
//! `GameTooltip:SetTalent(i, j)` keeps its 1.12 name and shows choice `j` of row `i` as its spell,
//! through the spell builder the spell book's hover uses. Indices are 1-based in Lua.

use mlua::{Lua, MultiValue, Table, Value};

use super::tooltip_spell::{set_spell_by_id, SpellRenderOpts};
use super::Model;

/// One choice of a row, as the realm's `TalentChoiceView`, with the icon the host resolved.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TalentChoiceView {
    pub spell_id: u32,
    pub name: String,
    /// The spell's icon from its own Spell.dbc row in the client's catalog; `None` without one.
    pub icon: Option<String>,
}

/// One Talent Row, as the realm's `TalentRowView`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TalentRowView {
    pub key: String,
    pub level: u8,
    /// The character's level has reached `level`.
    pub open: bool,
    /// A Skill Choice row's plain skill's name; "" for a Talent row.
    pub skill: String,
    /// The set choice's spell id; 0 when empty.
    pub set: u32,
    pub choices: Vec<TalentChoiceView>,
}

/// The current Calling's rows, as the realm's `TalentRows` event.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TalentRowsView {
    pub calling: String,
    /// An innkeeper is within service range, so a set row may change.
    pub at_innkeeper: bool,
    pub rows: Vec<TalentRowView>,
}

/// One call the panel queued for the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TalentRowsCall {
    /// Set the row keyed `row` to the choice `spell_id`.
    Set { row: String, spell_id: u32 },
    /// The panel asks for the rows; the realm answers with them.
    Open,
}

impl super::UiScript {
    /// Push the rows `GetTalentRows()` and its siblings answer.
    pub fn set_talent_rows(&mut self, view: TalentRowsView) {
        self.model_mut().talent_rows = view;
    }

    /// Drain the panel's calls since the last drain, oldest first.
    pub fn take_talent_rows_calls(&mut self) -> Vec<TalentRowsCall> {
        std::mem::take(&mut self.model_mut().talent_rows_calls)
    }
}

/// The 0-based index for a 1-based Lua number.
fn index(i: i64) -> Option<usize> {
    usize::try_from(i.checked_sub(1)?).ok()
}

fn row_at(model: &Model, i: i64) -> Option<&TalentRowView> {
    model.talent_rows.rows.get(index(i)?)
}

fn choice_at(model: &Model, i: i64, j: i64) -> Option<(&TalentRowView, &TalentChoiceView)> {
    let row = row_at(model, i)?;
    Some((row, row.choices.get(index(j)?)?))
}

fn text(lua: &Lua, s: &str) -> mlua::Result<Value> {
    Ok(Value::String(lua.create_string(s)?))
}

fn queue(lua: &Lua, call: TalentRowsCall) {
    lua.app_data_mut::<Model>()
        .expect("model app_data")
        .talent_rows_calls
        .push(call);
}

/// The spells of every choice on the panel, whose tooltip views the host pushes ahead of a hover.
pub(super) fn tooltip_subjects(model: &Model) -> impl Iterator<Item = u32> + '_ {
    model
        .talent_rows
        .rows
        .iter()
        .flat_map(|r| r.choices.iter().map(|c| c.spell_id))
        .filter(|&id| id != 0)
}

/// Register the `GetTalentRow*` getters and the panel's calls.
pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    // GetTalentRows(): count, atInnkeeper, callingName.
    g.set(
        "GetTalentRows",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let v = &model.talent_rows;
            Ok(MultiValue::from_vec(vec![
                Value::Integer(v.rows.len() as i64),
                Value::Boolean(v.at_innkeeper),
                text(lua, &v.calling)?,
            ]))
        })?,
    )?;
    // GetTalentRow(i): key, level, open, skillName, setSpell; nothing out of range.
    g.set(
        "GetTalentRow",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(r) = row_at(&model, i) else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                text(lua, &r.key)?,
                Value::Integer(i64::from(r.level)),
                Value::Boolean(r.open),
                text(lua, &r.skill)?,
                Value::Integer(i64::from(r.set)),
            ]))
        })?,
    )?;
    // GetTalentChoice(i, j): spellId, name, icon; nothing out of range.
    g.set(
        "GetTalentChoice",
        lua.create_function(|lua, (i, j): (i64, i64)| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some((_, c)) = choice_at(&model, i, j) else {
                return Ok(MultiValue::new());
            };
            let icon = match &c.icon {
                Some(icon) => text(lua, icon)?,
                None => Value::Nil,
            };
            Ok(MultiValue::from_vec(vec![
                Value::Integer(i64::from(c.spell_id)),
                text(lua, &c.name)?,
                icon,
            ]))
        })?,
    )?;
    // SetTalentRowChoice(i, j): queues choice j of row i for the realm; out of range does nothing.
    g.set(
        "SetTalentRowChoice",
        lua.create_function(|lua, (i, j): (i64, i64)| {
            let call = {
                let model = lua.app_data_ref::<Model>().expect("model app_data");
                choice_at(&model, i, j).map(|(r, c)| TalentRowsCall::Set {
                    row: r.key.clone(),
                    spell_id: c.spell_id,
                })
            };
            if let Some(call) = call {
                queue(lua, call);
            }
            Ok(())
        })?,
    )?;
    g.set(
        "OpenTalentRows",
        lua.create_function(|lua, ()| {
            queue(lua, TalentRowsCall::Open);
            Ok(())
        })?,
    )?;
    Ok(())
}

/// Register `GameTooltip:SetTalent(i, j)`: choice `j` of row `i` as its spell, the choice's name
/// standing in until the spell's view arrives.
pub(super) fn install_tooltip_method(lua: &Lua, m: &Table) -> mlua::Result<()> {
    m.set(
        "SetTalent",
        lua.create_function(|lua, (this, i, j): (Table, i64, i64)| {
            let choice = {
                let model = lua.app_data_ref::<Model>().expect("model app_data");
                choice_at(&model, i, j).map(|(_, c)| (c.spell_id, c.name.clone()))
            };
            let Some((spell_id, name)) = choice else {
                return Ok(());
            };
            set_spell_by_id(
                lua,
                &this,
                spell_id,
                Some(name),
                SpellRenderOpts::default(),
                None,
            )
        })?,
    )?;
    Ok(())
}
