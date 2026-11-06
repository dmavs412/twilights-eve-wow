//! The Alchemy window's state Lua reads (ADR 0025, spec #522): `GetAlchemyBench()` and its
//! siblings for the open table's makes, tray, targets and Rumours (`GetAlchemyTier`,
//! `GetAlchemyMake`, `GetAlchemyTrayItem`, `GetAlchemyTrayValue`, `GetAlchemyTarget`,
//! `GetAlchemyRumour`); what the list would come out as (`GetAlchemyPreview`,
//! `GetAlchemyPreviewVial`, `GetAlchemyPreviewToxic`); the last make's result (`GetAlchemyMade`,
//! `GetAlchemyMadeVial`, `GetAlchemyMadeToxic`, `GetAlchemyMadeLine`); and a product's Strength
//! Step read off its item description (`GetAlchemyItemStep`). The window queues `AlchemyPreview`,
//! `AlchemyMake`, `AlchemyBuyRumour`, `AlchemySellSludge` and `AlchemyClose` for the host. The host
//! pushes the views and fires `ALCHEMY_UPDATE`, `ALCHEMY_PREVIEW`, `ALCHEMY_MADE` and
//! `ALCHEMY_CLOSED` itself. Indices are 1-based in Lua; a grade is the realm's 0-based index.

use mlua::{Lua, MultiValue, Value, Variadic};

use super::Model;

/// One of the table's five makes, as the realm's `AlchemyMakeRow`; a limit is 0 where the make
/// has none.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AlchemyMakeView {
    pub key: String,
    pub name: String,
    pub station: String,
    pub verb: String,
    pub rule: String,
    pub open: bool,
    /// `[min, max]` herbs as the body.
    pub herbs: [u32; 2],
    pub tinctures: u32,
    pub essences: u32,
    pub fish: u32,
    pub wash: u32,
}

/// One material held, as the realm's `AlchemyMaterial`. `kind` is 0 herb, 1 fish, 2 Tincture,
/// 3 Essence; `values` are a herb's note, `(name, points)` with `None` while only the name shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AlchemyMaterialView {
    pub item: u32,
    pub count: u32,
    pub kind: u8,
    pub level: u8,
    pub values: Vec<(String, Option<u32>)>,
    pub prop: String,
    pub points: f32,
    pub toxic: u32,
    pub step: f32,
}

/// One grade a make may target, as the realm's `AlchemyGrade`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AlchemyGradeView {
    pub family: String,
    pub key: String,
    pub grade: u8,
    pub name: String,
    pub learned: bool,
    pub rumoured: bool,
    /// 0 unless learned or rumoured.
    pub threshold: u32,
}

/// One Rumour the table sells, as the realm's `AlchemyRumour`; `price` in copper.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AlchemyRumourView {
    pub family: String,
    pub key: String,
    pub grade: u8,
    pub name: String,
    pub price: u32,
}

/// The open alchemy table, field for field as the realm's `AlchemyBench` event.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AlchemyBenchView {
    /// The table's name, its alchemist's, and the alchemist's first-visit line.
    pub table: String,
    pub alchemist: String,
    pub line: String,
    /// The grades its Rumours come in.
    pub tiers: Vec<String>,
    pub makes: Vec<AlchemyMakeView>,
    pub tray: Vec<AlchemyMaterialView>,
    pub targets: Vec<AlchemyGradeView>,
    pub rumours: Vec<AlchemyRumourView>,
    /// Murky Sludge held.
    pub sludge: u32,
}

/// One property's vial, as the realm's `AlchemyVial`. `unknown` is 0 none, 1 some, 2 all.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AlchemyVialView {
    pub prop: String,
    pub value: f32,
    pub unknown: u8,
    pub main: bool,
    pub target: bool,
    pub bonus: f32,
    /// The target's threshold when known, else 0.
    pub tick: u32,
}

/// The Toxic vial, as the realm's `AlchemyToxic`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AlchemyToxicView {
    pub raw: u32,
    pub purify: u32,
    pub net: u32,
    pub unknown: bool,
    pub fail_pct: u32,
    pub bonus: f32,
}

/// What a make's list would come out as, as the realm's `AlchemyPreview`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AlchemyPreviewView {
    pub ready: bool,
    pub note: String,
    pub vials: Vec<AlchemyVialView>,
    pub toxic: AlchemyToxicView,
    pub unnoted: u32,
    pub scale: f32,
    pub rings: [u32; 4],
    pub hybrid: bool,
    pub known: bool,
    /// 0 while `known` is false.
    pub product: u32,
    pub sludge: bool,
    pub carried: u32,
}

/// A make's result, as the realm's `AlchemyMade`. `lines` are `(kind, text)`: 0 info, 1 learn,
/// 2 say, 3 product, 4 error; `product` is 0 when the make was refused.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AlchemyMadeView {
    pub product: u32,
    pub failed: bool,
    pub fail_pct: u32,
    pub vials: Vec<AlchemyVialView>,
    pub toxic: AlchemyToxicView,
    pub scale: f32,
    pub rings: [u32; 4],
    pub lines: Vec<(u8, String)>,
}

/// One herb's Herb Note, as the realm's `AlchemyNote`. `rarity` is 0 common to 3 legendary;
/// `values` are `(name, points)` with `None` while only the name shows; `budget` is 0 until the
/// values show.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AlchemyNoteView {
    pub item: u32,
    pub uses: u32,
    pub level: u8,
    pub rarity: u8,
    pub zones: String,
    pub values: Vec<(String, Option<u32>)>,
    pub budget: u32,
}

/// One grade's Journal page, as the realm's `AlchemyGradePage`. `steps` counts the makes at each
/// Strength Step, x1 to x2; `threshold` is 0 unless learned or rumoured.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AlchemyGradePageView {
    pub family: String,
    pub key: String,
    pub grade: u8,
    pub name: String,
    pub base: String,
    pub learned: bool,
    pub rumoured: bool,
    pub threshold: u32,
    pub steps: [u32; 5],
    pub effect: String,
}

/// One flask kind and how many were made, as the realm's `AlchemyFlaskRow`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AlchemyFlaskView {
    pub key: String,
    pub name: String,
    pub stats: String,
    pub made: u32,
}

/// One Brew Log line, as the realm's `BrewLogRow`. `inputs` are `(item, count)`; `toxic` is "" for
/// a clean make.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BrewLogView {
    pub when: String,
    pub make: String,
    pub inputs: Vec<(u32, u32)>,
    pub sums: String,
    pub toxic: String,
    pub result: u32,
    pub note: String,
}

/// The Alchemy Journal, field for field as the realm's `AlchemyJournal` event. `log` is newest
/// first; `open` the make keys open.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AlchemyJournalView {
    pub satchel: Vec<super::SatchelSlot>,
    pub notes: Vec<AlchemyNoteView>,
    pub grades: Vec<AlchemyGradePageView>,
    pub flasks: Vec<AlchemyFlaskView>,
    pub log: Vec<BrewLogView>,
    pub open: Vec<String>,
    pub last: String,
}

/// One call the window queued for the host, carrying the realm's `AlchemyAction` fields. `items`
/// are item entries; `target` is "potion|Restorative|Minor", or "" for none. `Withdraw` is the
/// Journal's: `count` of the Alchemy Satchel's `item` back to the bags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlchemyCall {
    Withdraw {
        item: u32,
        count: u32,
    },
    Preview {
        make: String,
        items: Vec<u32>,
        target: String,
    },
    Make {
        make: String,
        items: Vec<u32>,
        target: String,
    },
    BuyRumour {
        family: String,
        key: String,
        grade: u8,
    },
    SellSludge,
    Close,
}

impl super::UiScript {
    /// Push the open table `GetAlchemyBench()` and its siblings answer, or `None` once it closes.
    pub fn set_alchemy(&mut self, view: Option<AlchemyBenchView>) {
        self.model_mut().alchemy = view;
    }

    /// Push what the list would come out as, as `GetAlchemyPreview()` answers.
    pub fn set_alchemy_preview(&mut self, view: Option<AlchemyPreviewView>) {
        self.model_mut().alchemy_preview = view;
    }

    /// Push the last make's result, as `GetAlchemyMade()` answers.
    pub fn set_alchemy_made(&mut self, view: Option<AlchemyMadeView>) {
        self.model_mut().alchemy_made = view;
    }

    /// Push the Alchemy Journal `GetAlchemyJournal()` and its siblings answer.
    pub fn set_alchemy_journal(&mut self, view: AlchemyJournalView) {
        self.model_mut().alchemy_journal = view;
    }

    /// Drain the `Alchemy*` calls since the last drain, oldest first.
    pub fn take_alchemy_calls(&mut self) -> Vec<AlchemyCall> {
        std::mem::take(&mut self.model_mut().alchemy_calls)
    }
}

fn queue(lua: &Lua, call: AlchemyCall) {
    lua.app_data_mut::<Model>()
        .expect("model app_data")
        .alchemy_calls
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

/// A make's items from Lua: nil, 0 or a bad entry is dropped.
fn items_of(items: Variadic<Option<i64>>) -> Vec<u32> {
    items
        .iter()
        .filter_map(|item| item.and_then(|i| u32::try_from(i).ok()))
        .filter(|&i| i != 0)
        .collect()
}

/// The Strength Step a product's description ends on: "x1.25" from "... Strength x1.25.".
fn step_of(description: &str) -> Option<String> {
    let (_, rest) = description.rsplit_once("Strength x")?;
    let digits: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let digits = digits.trim_end_matches('.');
    (!digits.is_empty()).then(|| format!("x{digits}"))
}

/// A vial's getter values: prop, value, unknown, main, target, bonus, tick.
fn vial(lua: &Lua, v: Option<&AlchemyVialView>) -> mlua::Result<MultiValue> {
    let Some(v) = v else {
        return Ok(MultiValue::new());
    };
    Ok(MultiValue::from_vec(vec![
        text(lua, &v.prop)?,
        num(v.value),
        num(v.unknown),
        Value::Boolean(v.main),
        Value::Boolean(v.target),
        num(v.bonus),
        num(v.tick),
    ]))
}

/// The Toxic vial's getter values: raw, purify, net, unknown, failPct, bonus.
fn toxic(t: Option<&AlchemyToxicView>) -> MultiValue {
    let Some(t) = t else {
        return MultiValue::new();
    };
    MultiValue::from_vec(vec![
        num(t.raw),
        num(t.purify),
        num(t.net),
        Value::Boolean(t.unknown),
        num(t.fail_pct),
        num(t.bonus),
    ])
}

/// Register the `GetAlchemy*` getters and the `Alchemy*` calls.
pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    // GetAlchemyBench(): table, alchemist, line, numMakes, numTray, numTargets, numRumours,
    // sludge; nothing with no table open.
    g.set(
        "GetAlchemyBench",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(b) = &model.alchemy else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                text(lua, &b.table)?,
                text(lua, &b.alchemist)?,
                text(lua, &b.line)?,
                num(b.makes.len() as f64),
                num(b.tray.len() as f64),
                num(b.targets.len() as f64),
                num(b.rumours.len() as f64),
                num(b.sludge),
            ]))
        })?,
    )?;
    // GetAlchemyTier(i): the name of the table's i-th Rumour grade, nil past the last.
    g.set(
        "GetAlchemyTier",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let tier = index(i)
                .zip(model.alchemy.as_ref())
                .and_then(|(i, b)| b.tiers.get(i));
            Ok(tier.map(|t| lua.create_string(t)).transpose()?)
        })?,
    )?;
    // GetAlchemyMake(i): key, name, station, verb, rule, open, herbsMin, herbsMax, tinctures,
    // essences, fish, wash.
    g.set(
        "GetAlchemyMake",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let m = index(i)
                .zip(model.alchemy.as_ref())
                .and_then(|(i, b)| b.makes.get(i));
            let Some(m) = m else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                text(lua, &m.key)?,
                text(lua, &m.name)?,
                text(lua, &m.station)?,
                text(lua, &m.verb)?,
                text(lua, &m.rule)?,
                Value::Boolean(m.open),
                num(m.herbs[0]),
                num(m.herbs[1]),
                num(m.tinctures),
                num(m.essences),
                num(m.fish),
                num(m.wash),
            ]))
        })?,
    )?;
    // GetAlchemyTrayItem(i): item, count, kind, level, numValues, prop, points, toxic, step.
    g.set(
        "GetAlchemyTrayItem",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let t = index(i)
                .zip(model.alchemy.as_ref())
                .and_then(|(i, b)| b.tray.get(i));
            let Some(t) = t else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                num(t.item),
                num(t.count),
                num(t.kind),
                num(t.level),
                num(t.values.len() as f64),
                text(lua, &t.prop)?,
                num(t.points),
                num(t.toxic),
                num(t.step),
            ]))
        })?,
    )?;
    // GetAlchemyTrayValue(i, j): name, points; nil points while only the name is known.
    g.set(
        "GetAlchemyTrayValue",
        lua.create_function(|lua, (i, j): (i64, i64)| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let v = index(i)
                .zip(model.alchemy.as_ref())
                .and_then(|(i, b)| b.tray.get(i))
                .zip(index(j))
                .and_then(|(t, j)| t.values.get(j));
            let Some((name, points)) = v else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                text(lua, name)?,
                points.map_or(Value::Nil, num),
            ]))
        })?,
    )?;
    // GetAlchemyTarget(i): family, key, grade, name, learned, rumoured, threshold.
    g.set(
        "GetAlchemyTarget",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let t = index(i)
                .zip(model.alchemy.as_ref())
                .and_then(|(i, b)| b.targets.get(i));
            let Some(t) = t else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                text(lua, &t.family)?,
                text(lua, &t.key)?,
                num(t.grade),
                text(lua, &t.name)?,
                Value::Boolean(t.learned),
                Value::Boolean(t.rumoured),
                num(t.threshold),
            ]))
        })?,
    )?;
    // GetAlchemyRumour(i): family, key, grade, name, price.
    g.set(
        "GetAlchemyRumour",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let r = index(i)
                .zip(model.alchemy.as_ref())
                .and_then(|(i, b)| b.rumours.get(i));
            let Some(r) = r else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                text(lua, &r.family)?,
                text(lua, &r.key)?,
                num(r.grade),
                text(lua, &r.name)?,
                num(r.price),
            ]))
        })?,
    )?;
    // GetAlchemyPreview(): ready, note, unnoted, scale, ring1, ring2, ring3, ring4, hybrid, known,
    // product, sludge, carried; nothing before the first preview.
    g.set(
        "GetAlchemyPreview",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(p) = &model.alchemy_preview else {
                return Ok(MultiValue::new());
            };
            let mut out = vec![
                Value::Boolean(p.ready),
                text(lua, &p.note)?,
                num(p.unnoted),
                num(p.scale),
            ];
            out.extend(p.rings.iter().map(|&r| num(r)));
            out.extend([
                Value::Boolean(p.hybrid),
                Value::Boolean(p.known),
                num(p.product),
                Value::Boolean(p.sludge),
                num(p.carried),
            ]);
            Ok(MultiValue::from_vec(out))
        })?,
    )?;
    // GetAlchemyPreviewVial(i), 1 to 5: prop, value, unknown, main, target, bonus, tick.
    g.set(
        "GetAlchemyPreviewVial",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let v = index(i)
                .zip(model.alchemy_preview.as_ref())
                .and_then(|(i, p)| p.vials.get(i));
            vial(lua, v)
        })?,
    )?;
    // GetAlchemyPreviewToxic(): raw, purify, net, unknown, failPct, bonus.
    g.set(
        "GetAlchemyPreviewToxic",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            Ok(toxic(model.alchemy_preview.as_ref().map(|p| &p.toxic)))
        })?,
    )?;
    // GetAlchemyMade(): product, failed, failPct, numLines, scale, ring1, ring2, ring3, ring4;
    // nothing before a make.
    g.set(
        "GetAlchemyMade",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(m) = &model.alchemy_made else {
                return Ok(MultiValue::new());
            };
            let mut out = vec![
                num(m.product),
                Value::Boolean(m.failed),
                num(m.fail_pct),
                num(m.lines.len() as f64),
                num(m.scale),
            ];
            out.extend(m.rings.iter().map(|&r| num(r)));
            Ok(MultiValue::from_vec(out))
        })?,
    )?;
    g.set(
        "GetAlchemyMadeVial",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let v = index(i)
                .zip(model.alchemy_made.as_ref())
                .and_then(|(i, m)| m.vials.get(i));
            vial(lua, v)
        })?,
    )?;
    g.set(
        "GetAlchemyMadeToxic",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            Ok(toxic(model.alchemy_made.as_ref().map(|m| &m.toxic)))
        })?,
    )?;
    // GetAlchemyMadeLine(i): kind, text.
    g.set(
        "GetAlchemyMadeLine",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let line = index(i)
                .zip(model.alchemy_made.as_ref())
                .and_then(|(i, m)| m.lines.get(i));
            let Some((kind, line)) = line else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![num(*kind), text(lua, line)?]))
        })?,
    )?;
    // GetAlchemyItemStep(item): "x1.25" from the "Strength x1.25." its description ends on, nil
    // for an item the client does not know or whose description names no step.
    g.set(
        "GetAlchemyItemStep",
        lua.create_function(|lua, item: Option<i64>| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let step = item
                .and_then(|i| u32::try_from(i).ok())
                .and_then(|i| model.item_templates.get(&i))
                .and_then(|t| step_of(&t.description));
            Ok(step.map(|s| lua.create_string(&s)).transpose()?)
        })?,
    )?;
    // AlchemyPreview(make, target, ...) and AlchemyMake(make, target, ...): the make's key, the
    // target ("" or nil for none), then the list's item entries.
    g.set(
        "AlchemyPreview",
        lua.create_function(
            |lua, (make, target, items): (String, Option<String>, Variadic<Option<i64>>)| {
                queue(
                    lua,
                    AlchemyCall::Preview {
                        make,
                        items: items_of(items),
                        target: target.unwrap_or_default(),
                    },
                );
                Ok(())
            },
        )?,
    )?;
    g.set(
        "AlchemyMake",
        lua.create_function(
            |lua, (make, target, items): (String, Option<String>, Variadic<Option<i64>>)| {
                queue(
                    lua,
                    AlchemyCall::Make {
                        make,
                        items: items_of(items),
                        target: target.unwrap_or_default(),
                    },
                );
                Ok(())
            },
        )?,
    )?;
    // AlchemyBuyRumour(family, key, grade): grade is the realm's 0-based index.
    g.set(
        "AlchemyBuyRumour",
        lua.create_function(|lua, (family, key, grade): (String, String, i64)| {
            if let Ok(grade) = u8::try_from(grade) {
                queue(lua, AlchemyCall::BuyRumour { family, key, grade });
            }
            Ok(())
        })?,
    )?;
    g.set(
        "AlchemySellSludge",
        lua.create_function(|lua, ()| {
            queue(lua, AlchemyCall::SellSludge);
            Ok(())
        })?,
    )?;
    g.set(
        "AlchemyClose",
        lua.create_function(|lua, ()| {
            queue(lua, AlchemyCall::Close);
            Ok(())
        })?,
    )?;
    install_journal(lua)
}

/// Register the Alchemy Journal's getters and `WithdrawAlchemyItem`.
fn install_journal(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    // GetAlchemyJournal(): numSatchel, numNotes, numGrades, numFlasks, numLog, numOpen, last.
    g.set(
        "GetAlchemyJournal",
        lua.create_function(|lua, ()| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let j = &model.alchemy_journal;
            Ok(MultiValue::from_vec(vec![
                num(j.satchel.len() as f64),
                num(j.notes.len() as f64),
                num(j.grades.len() as f64),
                num(j.flasks.len() as f64),
                num(j.log.len() as f64),
                num(j.open.len() as f64),
                text(lua, &j.last)?,
            ]))
        })?,
    )?;
    // IsAlchemyMakeOpen(key): whether the make `key` ("tincture") is open.
    g.set(
        "IsAlchemyMakeOpen",
        lua.create_function(|lua, key: Option<String>| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            Ok(key.is_some_and(|k| model.alchemy_journal.open.contains(&k)))
        })?,
    )?;
    // GetAlchemySatchelItem(i): item, count.
    g.set(
        "GetAlchemySatchelItem",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(row) = index(i).and_then(|i| model.alchemy_journal.satchel.get(i)) else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![num(row.item), num(row.count)]))
        })?,
    )?;
    // GetAlchemyNote(i): item, uses, level, rarity, zones, budget, numValues.
    g.set(
        "GetAlchemyNote",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(n) = index(i).and_then(|i| model.alchemy_journal.notes.get(i)) else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                num(n.item),
                num(n.uses),
                num(n.level),
                num(n.rarity),
                text(lua, &n.zones)?,
                num(n.budget),
                num(n.values.len() as f64),
            ]))
        })?,
    )?;
    // GetAlchemyNoteValue(i, j): name, points; nil points while unknown.
    g.set(
        "GetAlchemyNoteValue",
        lua.create_function(|lua, (i, j): (i64, i64)| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let v = index(i)
                .and_then(|i| model.alchemy_journal.notes.get(i))
                .zip(index(j))
                .and_then(|(n, j)| n.values.get(j));
            let Some((name, points)) = v else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                text(lua, name)?,
                points.map_or(Value::Nil, num),
            ]))
        })?,
    )?;
    // GetAlchemyGradePage(i): family, key, grade, name, base, learned, rumoured, threshold,
    // step1, step2, step3, step4, step5, effect.
    g.set(
        "GetAlchemyGradePage",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(p) = index(i).and_then(|i| model.alchemy_journal.grades.get(i)) else {
                return Ok(MultiValue::new());
            };
            let mut out = vec![
                text(lua, &p.family)?,
                text(lua, &p.key)?,
                num(p.grade),
                text(lua, &p.name)?,
                text(lua, &p.base)?,
                Value::Boolean(p.learned),
                Value::Boolean(p.rumoured),
                num(p.threshold),
            ];
            out.extend(p.steps.iter().map(|&s| num(s)));
            out.push(text(lua, &p.effect)?);
            Ok(MultiValue::from_vec(out))
        })?,
    )?;
    // GetAlchemyFlask(i): key, name, stats, made.
    g.set(
        "GetAlchemyFlask",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(f) = index(i).and_then(|i| model.alchemy_journal.flasks.get(i)) else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                text(lua, &f.key)?,
                text(lua, &f.name)?,
                text(lua, &f.stats)?,
                num(f.made),
            ]))
        })?,
    )?;
    // GetAlchemyLog(i): when, make, sums, toxic, result, note, numInputs; newest first.
    g.set(
        "GetAlchemyLog",
        lua.create_function(|lua, i: i64| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let Some(l) = index(i).and_then(|i| model.alchemy_journal.log.get(i)) else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![
                text(lua, &l.when)?,
                text(lua, &l.make)?,
                text(lua, &l.sums)?,
                text(lua, &l.toxic)?,
                num(l.result),
                text(lua, &l.note)?,
                num(l.inputs.len() as f64),
            ]))
        })?,
    )?;
    // GetAlchemyLogInput(i, j): item, count.
    g.set(
        "GetAlchemyLogInput",
        lua.create_function(|lua, (i, j): (i64, i64)| {
            let model = lua.app_data_ref::<Model>().expect("model app_data");
            let input = index(i)
                .and_then(|i| model.alchemy_journal.log.get(i))
                .zip(index(j))
                .and_then(|(l, j)| l.inputs.get(j));
            let Some(&(item, count)) = input else {
                return Ok(MultiValue::new());
            };
            Ok(MultiValue::from_vec(vec![num(item), num(count)]))
        })?,
    )?;
    // WithdrawAlchemyItem(i, one): queues moving Satchel row `i` (the whole stack, or one) to
    // the bags.
    g.set(
        "WithdrawAlchemyItem",
        lua.create_function(|lua, (i, one): (i64, Option<bool>)| {
            let row = {
                let model = lua.app_data_ref::<Model>().expect("model app_data");
                index(i)
                    .and_then(|i| model.alchemy_journal.satchel.get(i))
                    .copied()
            };
            if let Some(row) = row {
                let count = if one == Some(true) { 1 } else { row.count };
                queue(
                    lua,
                    AlchemyCall::Withdraw {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_drops_nil_and_zero_entries() {
        let items = Variadic::from_iter([Some(2447), None, Some(0), Some(2447)]);
        assert_eq!(items_of(items), vec![2447, 2447]);
    }

    #[test]
    fn the_step_is_read_off_the_description() {
        assert_eq!(
            step_of("Use: Heals 88 to 113 at once. Strength x1.25.").as_deref(),
            Some("x1.25")
        );
        assert_eq!(
            step_of("Strength x1. Carries 4 Restorative points.").as_deref(),
            Some("x1")
        );
        assert_eq!(step_of("Vendor junk."), None);
    }

    #[test]
    fn the_journal_getters_read_the_pushed_journal() {
        let mut s = super::super::UiScript::new().unwrap();
        s.set_alchemy_journal(AlchemyJournalView {
            satchel: vec![super::super::SatchelSlot {
                item: 990001,
                count: 4,
                ..Default::default()
            }],
            notes: vec![AlchemyNoteView {
                item: 2447,
                uses: 2,
                level: 1,
                rarity: 0,
                zones: "Elwynn".into(),
                values: vec![("Restorative".into(), None)],
                budget: 0,
            }],
            grades: vec![AlchemyGradePageView {
                family: "potion".into(),
                key: "Restorative".into(),
                steps: [2, 0, 1, 0, 0],
                threshold: 4,
                learned: true,
                ..Default::default()
            }],
            log: vec![BrewLogView {
                inputs: vec![(2447, 2)],
                sums: "Restorative 4".into(),
                ..Default::default()
            }],
            open: vec!["potion".into()],
            last: "Minor Healing Potion".into(),
            ..Default::default()
        });
        let said = |lua: &str| s.eval::<String>(lua).unwrap();
        assert_eq!(
            said(
                "local a, b, c, d, e, f, last = GetAlchemyJournal() \
                 return a .. b .. c .. d .. e .. f .. last"
            ),
            "111011Minor Healing Potion"
        );
        assert_eq!(
            said("return tostring(IsAlchemyMakeOpen('potion')) .. tostring(IsAlchemyMakeOpen('flask'))"),
            "truefalse"
        );
        assert_eq!(
            said("local n, p = GetAlchemyNoteValue(1, 1) return n .. tostring(p)"),
            "Restorativenil"
        );
        assert_eq!(
            said(
                "local f, k, g, _, _, l, _, t, s1, s2, s3 = GetAlchemyGradePage(1) \
                 return f .. k .. g .. tostring(l) .. t .. s1 .. s2 .. s3"
            ),
            "potionRestorative0true4201"
        );
        assert_eq!(
            said("local i, c = GetAlchemyLogInput(1, 1) return i .. 'x' .. c"),
            "2447x2"
        );
        s.run("WithdrawAlchemyItem(1) WithdrawAlchemyItem(1, true) WithdrawAlchemyItem(2)")
            .unwrap();
        assert_eq!(
            s.take_alchemy_calls(),
            vec![
                AlchemyCall::Withdraw {
                    item: 990001,
                    count: 4
                },
                AlchemyCall::Withdraw {
                    item: 990001,
                    count: 1
                },
            ]
        );
    }
}
