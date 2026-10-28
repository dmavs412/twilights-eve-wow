//! The Kitchen window's state Lua reads (ADR 0025, spec #447): `GetKitchen()`, the recipe book
//! (`GetKitchenRecipe`, `GetKitchenRecipeSlot`), the pantry, today's orders, the fish that cut into
//! bait and the six lessons; what the filled pot would cook (`GetKitchenPrediction`); and the last
//! cook's result (`GetKitchenCooked`). The window queues `KitchenPredict`, `KitchenCook`,
//! `KitchenDeliver`, `KitchenCut` and `KitchenClose` for the host. The host pushes the views and
//! fires `KITCHEN_UPDATE`, `KITCHEN_PREDICTION`, `KITCHEN_COOKED` and `KITCHEN_CLOSED` itself.
//! Indices are 1-based in Lua; an order is 0-based in [`KitchenCall::Deliver`].

use mlua::{Lua, MultiValue, Value, Variadic};

use super::Model;

/// One page of the kitchen's recipe book, as the realm's `KitchenRecipe`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KitchenRecipeView {
    pub key: String,
    /// 0 tier, 1 dish, 2 legend, 3 plant.
    pub kind: u8,
    pub effect: String,
    pub family: String,
    /// 1, 10, 20 or 30; 0 for a legend or plant meal.
    pub band: u8,
    pub known: bool,
    /// "" and 0 while unknown.
    pub name: String,
    pub meal: u32,
    /// The ingredient items; an unrevealed slot is 0.
    pub slots: Vec<u32>,
}

/// One ingredient the player holds, as the realm's `KitchenIngredient`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KitchenIngredientView {
    pub item: u32,
    pub count: u32,
    pub level: u8,
    /// A cooking herb's category, or "fish", "legend" or "plant".
    pub category: String,
    pub untried: bool,
}

/// One of today's orders, as the realm's `KitchenOrderRow`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KitchenOrderView {
    pub label: String,
    /// 0 fish, 1 meal, 2 herb.
    pub kind: u8,
    pub copper: u32,
    pub qty: u32,
    pub have: u32,
    pub done: bool,
    pub can_deliver: bool,
    /// "" for none.
    pub bonus: String,
    pub bonus_rare: bool,
}

/// The open kitchen, field for field as the realm's `Kitchen` event.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KitchenView {
    /// The kitchen's name and its cook's.
    pub name: String,
    pub cook: String,
    pub book: Vec<KitchenRecipeView>,
    pub pantry: Vec<KitchenIngredientView>,
    /// Cook, recipe, eat, order, reveal, discover.
    pub lessons: [bool; 6],
    pub orders: Vec<KitchenOrderView>,
    /// `(fish, held, bait)`.
    pub cuttable: Vec<(u32, u32, u32)>,
}

/// What the filled pot would cook, as the realm's `KitchenPrediction`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KitchenPredictionView {
    pub ready: bool,
    /// 0 for a trial, which cooks whatever it matches.
    pub meal: u32,
    pub note: String,
    pub why: Vec<String>,
}

/// A cooked pot, as the realm's `KitchenCooked`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KitchenCookedView {
    pub meal: u32,
    pub mush: bool,
    pub close: bool,
    /// The recipe keys the cook taught.
    pub learned: Vec<String>,
    /// `(item, unknown recipes it appears in)`.
    pub revealed: Vec<(u32, u32)>,
    pub line: String,
}

/// How the pot is filled: from a known recipe, guessing an unknown one, or anything at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KitchenMode {
    Recipe,
    Discover,
    Experiment,
}

impl KitchenMode {
    /// The mode for Lua's "recipe", "discover" or "experiment".
    fn parse(s: &str) -> Option<Self> {
        match s {
            "recipe" => Some(Self::Recipe),
            "discover" => Some(Self::Discover),
            "experiment" => Some(Self::Experiment),
            _ => None,
        }
    }
}

/// One call the window queued for the host. `slots` are items, 0 for an empty slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KitchenCall {
    Predict {
        mode: KitchenMode,
        recipe: String,
        slots: Vec<u32>,
    },
    Cook {
        mode: KitchenMode,
        recipe: String,
        slots: Vec<u32>,
    },
    /// The order at this 0-based index.
    Deliver(u8),
    /// Cut one of this fish into its bait.
    Cut(u32),
    Close,
}

impl super::UiScript {
    /// Push the open kitchen `GetKitchen()` and its siblings answer, or `None` once it closes.
    pub fn set_kitchen(&mut self, view: Option<KitchenView>) {
        self.model_mut().kitchen = view;
    }

    /// Push what the filled pot would cook, as `GetKitchenPrediction()` answers.
    pub fn set_kitchen_prediction(&mut self, view: Option<KitchenPredictionView>) {
        self.model_mut().kitchen_prediction = view;
    }

    /// Push the last cook's result, as `GetKitchenCooked()` answers.
    pub fn set_kitchen_cooked(&mut self, view: Option<KitchenCookedView>) {
        self.model_mut().kitchen_cooked = view;
    }

    /// Drain the `Kitchen*` calls since the last drain, oldest first.
    pub fn take_kitchen_calls(&mut self) -> Vec<KitchenCall> {
        std::mem::take(&mut self.model_mut().kitchen_calls)
    }
}

fn queue(lua: &Lua, call: KitchenCall) {
    lua.app_data_mut::<Model>()
        .expect("model app_data")
        .kitchen_calls
        .push(call);
}

fn num(v: impl Into<f64>) -> Value {
    Value::Number(v.into())
}

fn text(lua: &Lua, s: &str) -> mlua::Result<Value> {
    Ok(Value::String(lua.create_string(s)?))
}

/// The 0-based index for a 1-based Lua number.
fn index(i: i64) -> Option<usize> {
    usize::try_from(i.checked_sub(1)?).ok()
}

/// The mode, recipe and slots of a `KitchenPredict` or `KitchenCook`: nil or a bad item is an
/// empty slot; an unknown mode is `None`.
fn pot(
    mode: String,
    key: Option<String>,
    items: Variadic<Option<i64>>,
) -> Option<(KitchenMode, String, Vec<u32>)> {
    let mode = KitchenMode::parse(&mode)?;
    let slots = items
        .iter()
        .map(|item| item.and_then(|i| u32::try_from(i).ok()).unwrap_or(0))
        .collect();
    Some((mode, key.unwrap_or_default(), slots))
}

/// Register the `GetKitchen*` getters and the `Kitchen*` calls.
pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    // GetKitchen(): name, cook, numBook, numPantry, numOrders, numCuttable; nothing with no
    // kitchen open.
    g.set(
        "GetKitchen",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(k) = &model.kitchen else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                text(lua, &k.name)?,
                text(lua, &k.cook)?,
                num(k.book.len() as f64),
                num(k.pantry.len() as f64),
                num(k.orders.len() as f64),
                num(k.cuttable.len() as f64),
            ]))
        })?,
    )?;
    // GetKitchenRecipe(i): key, kind, effect, family, band, known, name, meal, numSlots.
    g.set(
        "GetKitchenRecipe",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let r = index(i)
                .zip(model.kitchen.as_ref())
                .and_then(|(i, k)| k.book.get(i));
            let Some(r) = r else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                text(lua, &r.key)?,
                num(r.kind),
                text(lua, &r.effect)?,
                text(lua, &r.family)?,
                num(r.band),
                Value::Boolean(r.known),
                text(lua, &r.name)?,
                num(r.meal),
                num(r.slots.len() as f64),
            ]))
        })?,
    )?;
    // GetKitchenRecipeSlot(i, j): the item in slot j of recipe i, 0 while hidden or missing.
    g.set(
        "GetKitchenRecipeSlot",
        lua.create_function(|lua, (i, j): (i64, i64)| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let item = index(i)
                .zip(model.kitchen.as_ref())
                .and_then(|(i, k)| k.book.get(i))
                .zip(index(j))
                .and_then(|(r, j)| r.slots.get(j));
            Ok(item.copied().unwrap_or(0) as f64)
        })?,
    )?;
    // GetKitchenIngredient(i): item, count, level, category, untried.
    g.set(
        "GetKitchenIngredient",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let p = index(i)
                .zip(model.kitchen.as_ref())
                .and_then(|(i, k)| k.pantry.get(i));
            let Some(p) = p else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                num(p.item),
                num(p.count),
                num(p.level),
                text(lua, &p.category)?,
                Value::Boolean(p.untried),
            ]))
        })?,
    )?;
    // GetKitchenOrder(i): label, kind, copper, qty, have, done, canDeliver, bonus, bonusRare.
    g.set(
        "GetKitchenOrder",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let o = index(i)
                .zip(model.kitchen.as_ref())
                .and_then(|(i, k)| k.orders.get(i));
            let Some(o) = o else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                text(lua, &o.label)?,
                num(o.kind),
                num(o.copper),
                num(o.qty),
                num(o.have),
                Value::Boolean(o.done),
                Value::Boolean(o.can_deliver),
                text(lua, &o.bonus)?,
                Value::Boolean(o.bonus_rare),
            ]))
        })?,
    )?;
    // GetKitchenCuttable(i): fish, count, bait.
    g.set(
        "GetKitchenCuttable",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let c = index(i)
                .zip(model.kitchen.as_ref())
                .and_then(|(i, k)| k.cuttable.get(i));
            let Some(&(fish, count, bait)) = c else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![num(fish), num(count), num(bait)]))
        })?,
    )?;
    // GetKitchenLesson(i), 1 to 6: done.
    g.set(
        "GetKitchenLesson",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let done = index(i)
                .zip(model.kitchen.as_ref())
                .and_then(|(i, k)| k.lessons.get(i));
            Ok(done.copied())
        })?,
    )?;
    // GetKitchenPrediction(): ready, meal, note, numWhy; nothing before the first prediction.
    g.set(
        "GetKitchenPrediction",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(p) = &model.kitchen_prediction else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                Value::Boolean(p.ready),
                num(p.meal),
                text(lua, &p.note)?,
                num(p.why.len() as f64),
            ]))
        })?,
    )?;
    g.set(
        "GetKitchenPredictionWhy",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let why = index(i)
                .zip(model.kitchen_prediction.as_ref())
                .and_then(|(i, p)| p.why.get(i));
            Ok(why.map(|w| lua.create_string(w)).transpose()?)
        })?,
    )?;
    // GetKitchenCooked(): meal, mush, close, line, numLearned, numRevealed; nothing before a cook.
    g.set(
        "GetKitchenCooked",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(c) = &model.kitchen_cooked else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                num(c.meal),
                Value::Boolean(c.mush),
                Value::Boolean(c.close),
                text(lua, &c.line)?,
                num(c.learned.len() as f64),
                num(c.revealed.len() as f64),
            ]))
        })?,
    )?;
    // GetKitchenCookedLearned(i): the recipe key, and its name and meal from the open book ("" and
    // 0 when the book does not list it).
    g.set(
        "GetKitchenCookedLearned",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let key = index(i)
                .zip(model.kitchen_cooked.as_ref())
                .and_then(|(i, c)| c.learned.get(i));
            let Some(key) = key else {
                return Ok(MultiValue::new());
            };
            let page = model
                .kitchen
                .as_ref()
                .and_then(|k| k.book.iter().find(|r| &r.key == key));
            Ok(MultiValue::from_vec(vec![
                text(lua, key)?,
                text(lua, page.map_or("", |r| r.name.as_str()))?,
                num(page.map_or(0, |r| r.meal)),
            ]))
        })?,
    )?;
    // GetKitchenCookedRevealed(i): item, the unknown recipes it appears in.
    g.set(
        "GetKitchenCookedRevealed",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let r = index(i)
                .zip(model.kitchen_cooked.as_ref())
                .and_then(|(i, c)| c.revealed.get(i));
            let Some(&(item, count)) = r else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![num(item), num(count)]))
        })?,
    )?;
    // KitchenPredict(mode, key, ...) and KitchenCook(mode, key, ...): mode "recipe", "discover" or
    // "experiment", the recipe key ("" or nil for an experiment), then the pot's items, nil or 0
    // for an empty slot.
    g.set(
        "KitchenPredict",
        lua.create_function(
            |lua, (mode, key, items): (String, Option<String>, Variadic<Option<i64>>)| {
                if let Some((mode, recipe, slots)) = pot(mode, key, items) {
                    queue(
                        lua,
                        KitchenCall::Predict {
                            mode,
                            recipe,
                            slots,
                        },
                    );
                }
                Ok(())
            },
        )?,
    )?;
    g.set(
        "KitchenCook",
        lua.create_function(
            |lua, (mode, key, items): (String, Option<String>, Variadic<Option<i64>>)| {
                if let Some((mode, recipe, slots)) = pot(mode, key, items) {
                    queue(
                        lua,
                        KitchenCall::Cook {
                            mode,
                            recipe,
                            slots,
                        },
                    );
                }
                Ok(())
            },
        )?,
    )?;
    // KitchenDeliver(i): fills order i, 1-based.
    g.set(
        "KitchenDeliver",
        lua.create_function(|lua, i: i64| {
            if let Some(i) = index(i).and_then(|i| u8::try_from(i).ok()) {
                queue(lua, KitchenCall::Deliver(i));
            }
            Ok(())
        })?,
    )?;
    g.set(
        "KitchenCut",
        lua.create_function(|lua, fish: u32| {
            queue(lua, KitchenCall::Cut(fish));
            Ok(())
        })?,
    )?;
    g.set(
        "KitchenClose",
        lua.create_function(|lua, ()| {
            queue(lua, KitchenCall::Close);
            Ok(())
        })?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pot_reads_its_mode_and_empty_slots() {
        let items = Variadic::from_iter([Some(2447), None, Some(0)]);
        assert_eq!(
            pot("discover".into(), Some("stam_10".into()), items),
            Some((KitchenMode::Discover, "stam_10".into(), vec![2447, 0, 0]))
        );
        assert_eq!(pot("stew".into(), None, Variadic::new()), None);
    }
}
