//! Alchemy tables (ADR 0025, #542), as Davis's Alchemy prototype plays them: an alchemist's
//! gossip opens their table, which sends the bench and its tray, previews a make's vials from the
//! character's Herb Notes and makes it, rolling its Toxic. A table sells the Rumours of its tiers
//! and buys Murky Sludge; every make goes in the Brew Log.

use std::collections::HashMap;

use benilla_protocol::messages::{self as m, ChatMessage, GossipOption};
use benilla_protocol::{
    AlchemyFlaskRow, AlchemyGrade, AlchemyGradePage, AlchemyMakeRow, AlchemyMaterial, AlchemyNote,
    AlchemyRumour, AlchemyToxic, AlchemyValue, AlchemyVial, BrewLogRow, SessionEvent,
};

use crate::alchemy::{self, Family, Grade, Make, Row, Step, Table};
use crate::characters::{BrewLog, Char, GradeRecord};
use crate::equipment;
use crate::fields::values;
use crate::herbs::{self, Rarity};
use crate::hub::{self, Npc};
use crate::items::Profession;
use crate::quests::system_line;
use crate::rng::Rng;
use crate::rules::alchemy::{self as rules, Kind, NoteState, Sums, Target, Unknown, Vials};
use crate::rules::game_clock;
use crate::AlchemyAction;

/// The alchemist's one gossip line.
const OPTION_TABLE: u32 = 0;
const ICON_CHAT: u8 = 0;

/// A make's log lines' kinds, as the prototype's `msgs`.
const INFO: u8 = 0;
const LEARN: u8 = 1;
const SAY: u8 = 2;
const PRODUCT: u8 = 3;
const ERROR: u8 = 4;

/// The families a target or a grade page comes in, in the prototype's `FAMILIES` order.
const FAMILIES: [Family; 5] = [
    Family::Potion,
    Family::Hybrid,
    Family::Oil,
    Family::Tincture,
    Family::Essence,
];

/// An alchemist's gossip lines: the table.
pub(crate) fn alchemy_options() -> Vec<GossipOption> {
    vec![GossipOption {
        index: OPTION_TABLE,
        icon: ICON_CHAT,
        coded: false,
        message: "Let me use your alchemy table.".into(),
    }]
}

/// A character's open table: the alchemist's guid and the table's key.
struct Open {
    npc: u64,
    table: &'static str,
}

/// Every character's open table, keyed by player guid. Not saved.
#[derive(Default)]
pub(crate) struct Alchemies(HashMap<u64, Open>);

impl Alchemies {
    /// Forgets `guid`'s open table, as a logout does.
    pub(crate) fn close(&mut self, guid: u64) {
        self.0.remove(&guid);
    }
}

impl crate::Realm {
    /// The alchemist's line: their table opens, and on a first visit they say its line.
    pub(crate) fn alchemy_select(&mut self, npc: &Npc, option: u32) {
        if option != OPTION_TABLE || !self.near(npc) {
            return;
        }
        let Some(table) = alchemy::table_of_alchemist(npc.entry) else {
            return;
        };
        let Some(guid) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        let Some(c) = self.characters.get_mut(guid) else {
            return;
        };
        self.events.push(SessionEvent::GossipComplete);
        let first = !c.alchemy_tables.contains(&table.key);
        if first {
            c.alchemy_tables.push(table.key.clone());
            self.events.push(says(npc.guid(), table, &table.line));
        }
        self.events.push(bench(c, npc.guid(), table));
        self.alchemies.0.insert(
            guid,
            Open {
                npc: npc.guid(),
                table: &table.key,
            },
        );
        if first {
            self.save(guid);
        }
    }

    /// An action at the open table; nothing without one.
    pub(crate) fn alchemy_action(&mut self, action: AlchemyAction) {
        let Some(guid) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        let Some((npc, key)) = self.alchemies.0.get(&guid).map(|o| (o.npc, o.table)) else {
            return;
        };
        let Some(table) = alchemy::tables().iter().find(|t| t.key == key) else {
            return;
        };
        let wall = self.wall;
        let Some(c) = self.characters.get_mut(guid) else {
            return;
        };
        let changed = match action {
            AlchemyAction::Close => {
                self.alchemies.close(guid);
                self.events.push(SessionEvent::AlchemyClosed);
                return;
            }
            AlchemyAction::Preview {
                make,
                items,
                target,
            } => {
                self.events.push(preview(c, &make, &items, &target));
                return;
            }
            AlchemyAction::Make {
                make: key,
                items,
                target,
            } => {
                let made = make(
                    c,
                    &key,
                    &items,
                    &target,
                    wall,
                    &mut self.rng,
                    &mut self.events,
                );
                lines_to_chat(&made, &mut self.events);
                let ok =
                    matches!(&made, SessionEvent::AlchemyMade { product, .. } if *product != 0);
                self.events.push(made);
                ok
            }
            AlchemyAction::BuyRumour { family, key, grade } => {
                let line = buy_rumour(c, table, &family, &key, grade, &mut self.events);
                let ok = line.is_ok();
                self.events
                    .push(system_line(line.unwrap_or_else(|e| e.to_owned())));
                ok
            }
            AlchemyAction::SellSludge => {
                let line = sell_sludge(c, &mut self.events);
                let ok = line.is_ok();
                self.events
                    .push(system_line(line.unwrap_or_else(|e| e.to_owned())));
                ok
            }
        };
        if changed {
            self.events.push(bench(c, npc, table));
            self.events.push(journal(c));
            self.save(guid);
        }
    }

    /// Closes the served character's table when they stand beyond reach of its alchemist, or
    /// died.
    pub(crate) fn check_open_alchemy(&mut self) {
        let Some(w) = self.world.as_ref() else {
            return;
        };
        let Some(open) = self.alchemies.0.get(&w.guid) else {
            return;
        };
        let reach = match (hub::find(open.npc), self.characters.get(w.guid)) {
            (Some(npc), Some(c)) => c.map == npc.map && hub::within_reach(npc, c.position),
            _ => false,
        };
        if !reach || w.dead() {
            let guid = w.guid;
            self.alchemies.close(guid);
            self.events.push(SessionEvent::AlchemyClosed);
        }
    }
}

/// Every learn, info and error line of a make, as a system line too.
fn lines_to_chat(made: &SessionEvent, events: &mut Vec<SessionEvent>) {
    if let SessionEvent::AlchemyMade { lines, .. } = made {
        for (kind, text) in lines {
            if matches!(*kind, INFO | LEARN | ERROR) {
                events.push(system_line(text.clone()));
            }
        }
    }
}

/// The alchemist of `table`, whose guid is `npc`, says `text`.
fn says(npc: u64, table: &Table, text: &str) -> SessionEvent {
    SessionEvent::Chat(ChatMessage {
        chat_type: m::CHAT_MSG_MONSTER_SAY,
        language: 0,
        sender_guid: npc,
        target_guid: 0,
        sender_name: Some(alchemist_name(table)),
        channel: None,
        text: text.to_owned(),
        chat_tag: 0,
    })
}

fn alchemist_name(table: &Table) -> String {
    hub::npcs()
        .iter()
        .find(|n| n.entry == table.alchemist)
        .map_or_else(String::new, |n| n.name.clone())
}

fn name_of(entry: u32) -> String {
    crate::items::template(entry).map_or_else(String::new, |t| t.name.clone())
}

/// What `c`'s Herb Note on `item` shows.
fn note_state(c: &Char, item: u32) -> NoteState {
    let uses = uses_of(c, item);
    let notes = alchemy::notes();
    if uses >= notes.values_at {
        NoteState::Filled
    } else if uses >= notes.names_at {
        NoteState::Named
    } else {
        NoteState::Unnoted
    }
}

fn uses_of(c: &Char, item: u32) -> u32 {
    c.alchemy_notes
        .iter()
        .find(|n| n.0 == item)
        .map_or(0, |n| n.1)
}

fn record<'a>(c: &'a Char, family: Family, key: &str, grade: Grade) -> Option<&'a GradeRecord> {
    let f = rules::family_key(family);
    c.alchemy_grades
        .iter()
        .find(|r| r.family == f && r.key == key && r.grade == grade.0)
}

fn record_mut<'a>(c: &'a mut Char, family: Family, key: &str, grade: Grade) -> &'a mut GradeRecord {
    let f = rules::family_key(family);
    let at = match c
        .alchemy_grades
        .iter()
        .position(|r| r.family == f && r.key == key && r.grade == grade.0)
    {
        Some(at) => at,
        None => {
            c.alchemy_grades.push(GradeRecord {
                family: f.to_owned(),
                key: key.to_owned(),
                grade: grade.0,
                ..GradeRecord::default()
            });
            c.alchemy_grades.len() - 1
        }
    };
    &mut c.alchemy_grades[at]
}

fn learned(c: &Char, family: Family, key: &str, grade: Grade) -> bool {
    record(c, family, key, grade).is_some_and(|r| r.learned)
}

fn threshold_known(c: &Char, family: Family, key: &str, grade: Grade) -> bool {
    record(c, family, key, grade).is_some_and(|r| r.learned || r.rumoured)
}

fn threshold(family: Family, grade: Grade) -> u32 {
    alchemy::ladder(family)[usize::from(grade.0)]
}

/// The keys a family's grades come in: the pairs for a hybrid, else the properties.
fn keys_of(family: Family) -> &'static [String] {
    let props = alchemy::props();
    if family == Family::Hybrid {
        &props.pairs
    } else {
        &props.order
    }
}

/// Every (family, key, grade), in the prototype's `allGradeKeys` order.
fn all_grades() -> Vec<(Family, &'static str, Grade)> {
    FAMILIES
        .iter()
        .flat_map(|&f| {
            keys_of(f).iter().flat_map(move |k| {
                (0..alchemy::grades().names.len() as u8).map(move |g| (f, k.as_str(), Grade(g)))
            })
        })
        .collect()
}

/// The x1 item's name of a grade: "Lesser Healing Potion".
fn grade_name(family: Family, key: &str, grade: Grade) -> String {
    rules::entry_at(family, key, grade, Step(0), 0).map_or_else(String::new, name_of)
}

/// Whether the make `key` is open to `c`: one open from the start always is.
fn is_open(c: &Char, make: &Make) -> bool {
    make.opens_after.is_empty() || c.alchemy_makes.contains(&make.key)
}

/// Every item `c` holds at least one of in the bags or Satchel, ascending.
fn held_items(c: &Char) -> Vec<u32> {
    let mut entries: Vec<u32> = c
        .inventory
        .iter()
        .map(|(_, i)| i.entry)
        .chain(c.inventory.satchel.counts.keys().copied())
        .chain(c.inventory.satchel.pieces.iter().map(|p| p.entry))
        .collect();
    entries.sort_unstable();
    entries.dedup();
    entries.retain(|&item| c.inventory.count(item) > 0);
    entries
}

/// What a herb's note shows of its values, in `[props]` order, then Toxic and Purifying.
fn note_values(c: &Char, item: u32) -> Vec<AlchemyValue> {
    let Some(h) = alchemy::herb(item) else {
        return Vec::new();
    };
    let state = note_state(c, item);
    if state == NoteState::Unnoted {
        return Vec::new();
    }
    all_values()
        .filter_map(|p| {
            h.values.get(p).map(|&v| AlchemyValue {
                name: p.to_owned(),
                points: (state == NoteState::Filled).then_some(v),
            })
        })
        .collect()
}

/// The five properties, Toxic and Purifying (the prototype's `ALL_VALUES`).
fn all_values() -> impl Iterator<Item = &'static str> {
    let props = alchemy::props();
    props
        .order
        .iter()
        .map(String::as_str)
        .chain([props.toxic.as_str(), props.purifying.as_str()])
}

/// The tray: every herb, fish, Tincture and Essence `c` holds, by kind, then level, then name.
fn tray(c: &Char) -> Vec<AlchemyMaterial> {
    let mut rows: Vec<(String, AlchemyMaterial)> = held_items(c)
        .into_iter()
        .filter_map(|item| {
            let kind = rules::kind_of(item);
            let mut row = AlchemyMaterial {
                item,
                count: c.inventory.count(item),
                ..AlchemyMaterial::default()
            };
            match kind {
                Kind::Herb => {
                    row.level = alchemy::herb_level(item);
                    row.values = note_values(c, item);
                }
                Kind::Fish => row.kind = 1,
                Kind::Tincture | Kind::Essence => {
                    let Some(
                        Row::Tincture {
                            product,
                            grade,
                            step,
                            toxic,
                        }
                        | Row::Essence {
                            product,
                            grade,
                            step,
                            toxic,
                        },
                    ) = alchemy::row(item)
                    else {
                        return None;
                    };
                    row.kind = if kind == Kind::Tincture { 2 } else { 3 };
                    row.prop = product.key.clone();
                    row.points = alchemy::points(product.family, grade, step);
                    row.toxic = toxic;
                    row.step = alchemy::grades().steps[usize::from(step.0)];
                }
                Kind::Other => return None,
            }
            Some((name_of(item), row))
        })
        .collect();
    rows.sort_by(|(an, a), (bn, b)| (a.kind, a.level, an).cmp(&(b.kind, b.level, bn)));
    rows.into_iter().map(|(_, r)| r).collect()
}

/// The `AlchemyBench` of `table`, opened at the alchemist `npc`, for `c`.
fn bench(c: &Char, npc: u64, table: &Table) -> SessionEvent {
    let grades = &alchemy::grades().names;
    let makes = alchemy::makes()
        .iter()
        .map(|mk| AlchemyMakeRow {
            key: mk.key.clone(),
            name: mk.name.clone(),
            station: mk.station.clone(),
            verb: mk.verb.clone(),
            rule: mk.rule.clone(),
            open: is_open(c, mk),
            herbs: mk.herbs.unwrap_or([0, 0]),
            tinctures: mk.tinctures.unwrap_or(0),
            essences: mk.essences.unwrap_or(0),
            fish: mk.fish.unwrap_or(0),
            wash: mk.wash.unwrap_or(0),
        })
        .collect();
    let targets = all_grades()
        .into_iter()
        .map(|(f, k, g)| {
            let r = record(c, f, k, g);
            AlchemyGrade {
                family: rules::family_key(f).to_owned(),
                key: k.to_owned(),
                grade: g.0,
                name: grade_name(f, k, g),
                learned: r.is_some_and(|r| r.learned),
                rumoured: r.is_some_and(|r| r.rumoured),
                threshold: if threshold_known(c, f, k, g) {
                    threshold(f, g)
                } else {
                    0
                },
            }
        })
        .collect();
    let rumours = all_grades()
        .into_iter()
        .filter(|&(f, k, g)| {
            table.tiers.contains(&grades[usize::from(g.0)]) && !threshold_known(c, f, k, g)
        })
        .map(|(f, k, g)| AlchemyRumour {
            family: rules::family_key(f).to_owned(),
            key: k.to_owned(),
            grade: g.0,
            name: grade_name(f, k, g),
            price: alchemy::rumour_price(g),
        })
        .collect();
    SessionEvent::AlchemyBench {
        npc,
        table: table.key.clone(),
        alchemist: alchemist_name(table),
        line: table.line.clone(),
        tiers: table.tiers.clone(),
        makes,
        tray: tray(c),
        targets,
        rumours,
        sludge: c.inventory.count(alchemy::sludge().entry),
    }
}

/// A herb's zones as a note says them: "Elwynn, Redridge", "every zone", ", night only" for a
/// legendary that blooms at night.
fn zones_of(item: u32) -> String {
    if herbs::plants_in(herbs::ANY).iter().any(|p| p.item == item) {
        return "every zone".to_owned();
    }
    let mut names: Vec<String> = Vec::new();
    let mut night = false;
    for z in herbs::zones() {
        for p in herbs::plants_in(&z.key)
            .into_iter()
            .filter(|p| p.item == item)
        {
            let mut name = z.key.clone();
            if let Some(first) = name.get_mut(0..1) {
                first.make_ascii_uppercase();
            }
            if !names.contains(&name) {
                names.push(name);
            }
            night |= p.rarity == Rarity::Legendary && p.bloom == Some(crate::fish::Block::Night);
        }
    }
    let mut text = names.join(", ");
    if night {
        text.push_str(", night only");
    }
    text
}

/// The label of an Essence's flask stat: "Armour", from "In a flask: +{armour} Armour.".
fn stat_label(prop: &str) -> String {
    let text = alchemy::product(Family::Essence, prop).map_or("", |p| p.text.as_str());
    let rest = text.split_once("} ").map_or(text, |(_, r)| r);
    rest.trim_end_matches('.').to_owned()
}

/// A Brew Log line's sums, as the prototype's `propsText`: "Restorative 4, Toxic 2".
fn sums_text(sums: &[(String, f32)]) -> String {
    let text: Vec<String> = sums
        .iter()
        .map(|(p, v)| format!("{p} {}", alchemy::fmt_points(*v)))
        .collect();
    if text.is_empty() {
        "nothing".to_owned()
    } else {
        text.join(", ")
    }
}

/// `c`'s Alchemy Journal: the Satchel, the Herb Notes, the grade pages, the flasks and the Brew
/// Log, for login and every change.
pub(crate) fn journal(c: &Char) -> SessionEvent {
    let mut notes: Vec<(String, AlchemyNote)> = alchemy::herbs()
        .into_iter()
        .map(|h| {
            let filled = note_state(c, h.item) == NoteState::Filled;
            let note = AlchemyNote {
                item: h.item,
                uses: uses_of(c, h.item),
                level: alchemy::herb_level(h.item),
                rarity: alchemy::herb_rarity(h.item).map_or(0, |r| r as u8),
                zones: zones_of(h.item),
                values: note_values(c, h.item),
                budget: if filled { alchemy::budget(h.item) } else { 0 },
            };
            (name_of(h.item), note)
        })
        .collect();
    notes.sort_by(|(an, a), (bn, b)| (a.level, an).cmp(&(b.level, bn)));
    let grades = all_grades()
        .into_iter()
        .map(|(f, k, g)| {
            let r = record(c, f, k, g);
            let learned = r.is_some_and(|r| r.learned);
            let product = alchemy::product(f, k);
            AlchemyGradePage {
                family: rules::family_key(f).to_owned(),
                key: k.to_owned(),
                grade: g.0,
                name: grade_name(f, k, g),
                base: product.map_or_else(String::new, |p| p.name.clone()),
                learned,
                rumoured: r.is_some_and(|r| r.rumoured),
                threshold: if threshold_known(c, f, k, g) {
                    threshold(f, g)
                } else {
                    0
                },
                steps: r.map_or([0; 5], |r| r.steps),
                effect: match product {
                    Some(p)
                        if learned
                            && matches!(f, Family::Potion | Family::Hybrid | Family::Oil) =>
                    {
                        alchemy::text(p, g, Step(0))
                    }
                    _ => String::new(),
                },
            }
        })
        .collect();
    let flasks = alchemy::products()
        .into_iter()
        .filter(|p| p.family == Family::Flask)
        .map(|p| {
            let (a, b) = rules::halves(&p.key);
            AlchemyFlaskRow {
                key: p.key.clone(),
                name: p.name.clone(),
                stats: format!("{} and {}", stat_label(a), stat_label(b)),
                made: c
                    .alchemy_flasks
                    .iter()
                    .find(|f| f.0 == p.key)
                    .map_or(0, |f| f.1),
            }
        })
        .collect();
    let log = c
        .brew_log
        .iter()
        .rev()
        .map(|l| BrewLogRow {
            when: l.when.clone(),
            make: l.make.clone(),
            inputs: l.inputs.clone(),
            sums: sums_text(&l.sums),
            toxic: if l.net_toxic > 0 {
                format!("{} · {}%", l.net_toxic, l.fail_pct)
            } else {
                String::new()
            },
            result: l.result,
            note: l.note.clone(),
        })
        .collect();
    SessionEvent::AlchemyJournal {
        satchel: c.inventory.satchel.rows(Profession::Alchemy),
        notes: notes.into_iter().map(|(_, n)| n).collect(),
        grades,
        flasks,
        log,
        open: alchemy::makes()
            .iter()
            .filter(|mk| is_open(c, mk))
            .map(|mk| mk.key.clone())
            .collect(),
        last: c.alchemy_last.clone(),
    }
}

/// The prototype's `slotCheck`: why `items` cannot go to `make` now, if they cannot.
fn slot_check(c: &Char, make: &Make, items: &[u32]) -> Result<(), String> {
    if !is_open(c, make) {
        return Err(format!("{}s are not open yet.", make.name));
    }
    rules::valid(make, rules::counts(items)).map_err(str::to_owned)?;
    for (item, n) in tally(items) {
        if c.inventory.count(item) < n {
            return Err(format!("You don't have enough {}.", name_of(item)));
        }
    }
    Ok(())
}

/// Each distinct item of `items` and its count, in the order first put in.
fn tally(items: &[u32]) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    for &item in items {
        match out.iter_mut().find(|(i, _)| *i == item) {
            Some(row) => row.1 += 1,
            None => out.push((item, 1)),
        }
    }
    out
}

/// The vials' protocol rows.
fn vial_rows(v: &Vials) -> (Vec<AlchemyVial>, AlchemyToxic) {
    let rows = v
        .rows
        .iter()
        .map(|r| AlchemyVial {
            prop: r.prop.clone(),
            value: r.value,
            unknown: match r.unknown {
                Unknown::None => 0,
                Unknown::Some => 1,
                Unknown::All => 2,
            },
            main: r.main,
            target: r.target,
            bonus: r.bonus,
            tick: r.tick.unwrap_or(0),
        })
        .collect();
    let t = &v.toxic;
    let toxic = AlchemyToxic {
        raw: t.raw as u32,
        purify: t.purify as u32,
        net: t.net as u32,
        unknown: t.unknown,
        fail_pct: rules::pct(t.fail),
        bonus: t.bonus,
    };
    (rows, toxic)
}

/// The vials of `items` at `make` as `c` sees them, or with the true sums `truth`.
fn vials_for(
    c: &Char,
    make: &Make,
    items: &[u32],
    target: Option<&Target>,
    truth: Option<&Sums>,
) -> Vials {
    let tick_known = target.is_some_and(|t| threshold_known(c, t.family, &t.key, t.grade));
    rules::vials(
        make,
        items,
        target,
        &|i| note_state(c, i),
        tick_known,
        truth,
    )
}

/// Whether `c` knows the product `entry`: its grade learned, or its flask kind made.
fn product_known(c: &Char, entry: u32) -> bool {
    match alchemy::row(entry) {
        Some(Row::Flask { product, .. }) => c.alchemy_flasks.iter().any(|f| f.0 == product.key),
        Some(
            Row::Potion { product, grade, .. }
            | Row::Oil { product, grade, .. }
            | Row::Hybrid { product, grade, .. }
            | Row::Tincture { product, grade, .. }
            | Row::Essence { product, grade, .. },
        ) => learned(c, product.family, &product.key, grade),
        _ => false,
    }
}

/// The prototype's `preview`: what `items` at the make `key` would come out as.
fn preview(c: &Char, key: &str, items: &[u32], target: &str) -> SessionEvent {
    let target = rules::parse_target(target);
    let Some(make) = alchemy::make(key) else {
        return not_ready("No such make.".to_owned());
    };
    let v = vials_for(c, make, items, target.as_ref(), None);
    let (vials, toxic) = vial_rows(&v);
    let rings = alchemy::ladder(rules::ring_family(make));
    let check = slot_check(c, make, items);
    let r = check
        .is_ok()
        .then(|| rules::resolve(make, items, target.as_ref().map(|t| t.key.as_str())));
    let known = r
        .as_ref()
        .and_then(|r| r.product)
        .is_some_and(|p| product_known(c, p));
    SessionEvent::AlchemyPreview {
        ready: check.is_ok(),
        note: check.err().unwrap_or_default(),
        vials,
        toxic,
        unnoted: v.unnoted,
        scale: v.scale,
        rings,
        hybrid: v.hybrid,
        known,
        product: if known {
            r.as_ref().and_then(|r| r.product).unwrap_or(0)
        } else {
            0
        },
        sludge: r.as_ref().is_some_and(|r| r.is_sludge()),
        carried: r.as_ref().map_or(0, |r| r.carried),
    }
}

fn not_ready(note: String) -> SessionEvent {
    SessionEvent::AlchemyPreview {
        ready: false,
        note,
        vials: Vec::new(),
        toxic: AlchemyToxic::default(),
        unnoted: 0,
        scale: 0.0,
        rings: [0; 4],
        hybrid: false,
        known: false,
        product: 0,
        sludge: false,
        carried: 0,
    }
}

/// An `AlchemyMade` that only says why nothing was made.
fn refused(text: String) -> SessionEvent {
    SessionEvent::AlchemyMade {
        product: 0,
        failed: false,
        fail_pct: 0,
        vials: Vec::new(),
        toxic: AlchemyToxic::default(),
        scale: 0.0,
        rings: [0; 4],
        lines: vec![(ERROR, text)],
    }
}

/// "Day 2, 08:00" at the wall clock `wall`.
fn when(wall: f64) -> String {
    let (hour, minute) = game_clock::hour_minute(wall);
    let day = game_clock::day_serial(wall) + 1;
    format!("Day {day}, {hour:02}:{minute:02}")
}

/// A grade's name in a "Not the target." line: "the plain grade" or its name.
fn grade_word(grade: Grade, plain: &str) -> String {
    let g = &alchemy::grades();
    let name = &g.names[usize::from(grade.0)];
    if *name == g.standard {
        plain.to_owned()
    } else {
        name.clone()
    }
}

/// The prototype's `make`: takes `items`, counts their notes, rolls the Toxic and stores the
/// product or Murky Sludge; the `AlchemyMade`, with an error line alone when refused.
fn make(
    c: &mut Char,
    key: &str,
    items: &[u32],
    target: &str,
    wall: f64,
    rng: &mut Rng,
    events: &mut Vec<SessionEvent>,
) -> SessionEvent {
    let Some(make) = alchemy::make(key) else {
        return refused("No such make.".to_owned());
    };
    if let Err(e) = slot_check(c, make, items) {
        return refused(e);
    }
    let target = rules::parse_target(target);
    let r = rules::resolve(make, items, target.as_ref().map(|t| t.key.as_str()));
    let sludge = alchemy::sludge().entry;
    let mut room = c.inventory.clone();
    if room.store(r.product.unwrap_or(sludge), 1).is_err() || room.store(sludge, 1).is_err() {
        return refused("Your bags are full.".to_owned());
    }
    let mut lines: Vec<(u8, String)> = Vec::new();
    let notes = alchemy::notes();
    for (item, n) in tally(items) {
        equipment::take(c, item, n, events);
        if alchemy::herb(item).is_none() {
            continue;
        }
        let was = uses_of(c, item);
        let now = was + n;
        match c.alchemy_notes.binary_search_by_key(&item, |n| n.0) {
            Ok(at) => c.alchemy_notes[at].1 = now,
            Err(at) => c.alchemy_notes.insert(at, (item, now)),
        }
        let herb = name_of(item);
        if now >= notes.values_at && was < notes.values_at {
            lines.push((
                LEARN,
                format!("Herb Note filled: {herb} now shows each value."),
            ));
        } else if now >= notes.names_at && was < notes.names_at {
            lines.push((
                INFO,
                format!("Herb Note started: {herb} names what it holds."),
            ));
        }
    }
    let failed = !r.is_sludge() && r.fail > 0.0 && rng.frand(0.0, 1.0) < r.fail;
    let pct = rules::pct(r.fail);
    let mut log = BrewLog {
        when: when(wall),
        make: make.key.clone(),
        inputs: tally(items),
        sums: all_values()
            .filter_map(|p| {
                r.sums
                    .get(p)
                    .filter(|&&v| v > 0.0)
                    .map(|&v| (p.to_owned(), v))
            })
            .collect(),
        net_toxic: r.net as u32,
        fail_pct: pct,
        result: sludge,
        note: r.note.clone(),
    };
    let product = match r.product.filter(|_| !failed) {
        None => {
            if r.is_sludge() {
                lines.push((SAY, format!("Murky Sludge. {}", r.note)));
            } else {
                log.note = format!("The Toxic took it ({pct}% chance).");
                lines.push((
                    SAY,
                    format!("Murky Sludge. The Toxic took it: a {pct}% chance came up."),
                ));
            }
            sludge
        }
        Some(entry) => {
            log.result = entry;
            let prop = r.prop.clone().unwrap_or_default();
            let bonus = alchemy::fmt_points(r.payoff);
            let held = if r.net > 0.0 {
                let on = if prop.is_empty() {
                    String::new()
                } else {
                    format!(" {prop}")
                };
                log.note = if prop.is_empty() {
                    format!("Held through {pct}% Toxic: +{bonus}, split between the Essences.")
                } else {
                    format!("Held through {pct}% Toxic: +{bonus} {prop}.")
                };
                format!(" It held through {pct}% Toxic and gained {bonus}{on}.")
            } else {
                String::new()
            };
            let name = name_of(entry);
            match alchemy::row(entry) {
                Some(Row::Flask { product: p, .. }) => {
                    lines.push((PRODUCT, format!("{name}, into your bags.{held}")));
                    let first = !c.alchemy_flasks.iter().any(|f| f.0 == p.key);
                    match c.alchemy_flasks.iter_mut().find(|f| f.0 == p.key) {
                        Some(f) => f.1 += 1,
                        None => c.alchemy_flasks.push((p.key.clone(), 1)),
                    }
                    if first {
                        let kinds = alchemy::products()
                            .into_iter()
                            .filter(|p| p.family == Family::Flask)
                            .count();
                        c.alchemy_last = name.clone();
                        lines.push((
                            LEARN,
                            format!(
                                "New flask kind: {name} ({} of {kinds}).",
                                c.alchemy_flasks.len()
                            ),
                        ));
                    }
                }
                _ => {
                    let (grade, step) = r.grade.unwrap_or((Grade(0), Step(0)));
                    let carrying = if r.carried > 0 {
                        format!(" carrying Toxic {}", r.carried)
                    } else {
                        String::new()
                    };
                    lines.push((
                        PRODUCT,
                        format!(
                            "{name} {}{carrying}, into your bags.{held}",
                            alchemy::fmt_step(step)
                        ),
                    ));
                    if let Some(line) = target.as_ref().and_then(|t| missed(t, &r, grade)) {
                        lines.push((INFO, line));
                    }
                    let rec = record_mut(c, r.family, &prop, grade);
                    rec.steps[usize::from(step.0)] += 1;
                    if !rec.learned {
                        rec.learned = true;
                        let x1 = grade_name(r.family, &prop, grade);
                        lines.push((
                            LEARN,
                            format!("New grade learned: {x1}. Its page now shows its threshold."),
                        ));
                        c.alchemy_last = x1;
                    }
                }
            }
            for opens in alchemy::makes()
                .iter()
                .filter(|mk| mk.opens_after == make.key)
            {
                if !c.alchemy_makes.contains(&opens.key) {
                    c.alchemy_makes.push(opens.key.clone());
                    lines.push((
                        LEARN,
                        format!(
                            "{}s are open: {} at the {}.",
                            opens.name, opens.rule, opens.station
                        ),
                    ));
                }
            }
            entry
        }
    };
    if let Err(reason) = equipment::receive(c, product, 1, events) {
        events.push(equipment::inventory_failure(reason, 0));
    }
    c.brew_log.push(log);
    let keep = alchemy::log().keep as usize;
    if c.brew_log.len() > keep {
        let extra = c.brew_log.len() - keep;
        c.brew_log.drain(..extra);
    }
    let v = vials_for(c, make, items, target.as_ref(), Some(&r.sums));
    let (vials, toxic) = vial_rows(&v);
    SessionEvent::AlchemyMade {
        product,
        failed,
        fail_pct: pct,
        vials,
        toxic,
        scale: v.scale,
        rings: alchemy::ladder(rules::ring_family(make)),
        lines,
    }
}

/// The prototype's "Not the target." line, when `r` is of the target's kind but not it.
fn missed(t: &Target, r: &rules::Resolved, grade: Grade) -> Option<String> {
    let prop = r.prop.as_deref().unwrap_or_default();
    let potion_like = |f: Family| matches!(f, Family::Potion | Family::Hybrid);
    let kin = t.family == r.family || (potion_like(t.family) && potion_like(r.family));
    if !kin || (t.key == prop && t.grade == grade) {
        return None;
    }
    let why = if r.family == Family::Hybrid && t.family == Family::Potion {
        let (a, b) = rules::halves(prop);
        format!("Both {a} and {b} cleared the hybrid ladder, so it became a hybrid.")
    } else if t.family == Family::Hybrid && r.family == Family::Potion {
        format!("Only {prop} cleared the hybrid ladder.")
    } else if t.key != prop {
        format!("The herbs carried no {}, so {prop} led.", t.key)
    } else {
        format!(
            "It reached {}, not {}.",
            grade_word(grade, "the plain grade"),
            grade_word(t.grade, "plain")
        )
    };
    Some(format!("Not the target. {why}"))
}

/// The prototype's `rumour`: buys the Rumour of a grade `table` sells; its line, or the refusal.
fn buy_rumour(
    c: &mut Char,
    table: &Table,
    family: &str,
    key: &str,
    grade: u8,
    events: &mut Vec<SessionEvent>,
) -> Result<String, &'static str> {
    let names = &alchemy::grades().names;
    let family = rules::family_of_key(family).filter(|f| FAMILIES.contains(f));
    let grade = Grade(grade);
    let (Some(family), Some(name)) = (family, names.get(usize::from(grade.0))) else {
        return Err("No such rumour.");
    };
    if !keys_of(family).iter().any(|k| k == key) {
        return Err("No such rumour.");
    }
    if !table.tiers.contains(name) {
        return Err("No rumour of that grade here.");
    }
    if threshold_known(c, family, key, grade) {
        return Err("You already know that one.");
    }
    let price = alchemy::rumour_price(grade);
    if c.inventory.money < price {
        return Err("You can't afford that rumour.");
    }
    c.inventory.money -= price;
    events.push(values(c.guid, &[equipment::money_pair(&c.inventory)]));
    record_mut(c, family, key, grade).rumoured = true;
    let x1 = grade_name(family, key, grade);
    c.alchemy_last = format!("a rumour of {x1}");
    Ok(format!(
        "Rumour: {x1} needs {key} {}.",
        threshold(family, grade)
    ))
}

/// The prototype's `sellSludge`: every Murky Sludge held, at its price each.
fn sell_sludge(c: &mut Char, events: &mut Vec<SessionEvent>) -> Result<String, &'static str> {
    let s = alchemy::sludge();
    let n = c.inventory.count(s.entry);
    if n == 0 {
        return Err("No sludge to sell.");
    }
    equipment::take(c, s.entry, n, events);
    let copper = n * s.sell_price;
    c.inventory.money += copper;
    events.push(values(c.guid, &[equipment::money_pair(&c.inventory)]));
    Ok(format!("Sold {n} {} for {copper}c.", s.name))
}
