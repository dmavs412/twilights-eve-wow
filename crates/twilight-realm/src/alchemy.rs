//! Alchemy's content (`content/alchemy.toml`, ADR 0025 amended 2026-11-01), as Davis's Alchemy
//! prototype plays it: the herbs' values, the grade ladders, the Toxic rule's tunables, the five
//! makes, the three tables, the Rumour prices, Murky Sludge and the 45 product kinds. The item
//! rows are built here at load, one per entry of each product's block, by the file header's
//! formula; `crate::items` finds them through [`item`].

// The tables, the makes and the products' uses (#542 on) read the rest.
#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::sync::{OnceLock, RwLock};

use serde::Deserialize;

use crate::herbs::Rarity;
use crate::items::{Profession, Template};

/// The five product properties and the two values that make nothing (`[props]`).
#[derive(Deserialize)]
pub(crate) struct Props {
    /// In the prototype's PROPS order, which every list and entry block follows.
    pub(crate) order: [String; 5],
    pub(crate) toxic: String,
    pub(crate) purifying: String,
    /// The ten hybrid pairs, `"A+B"`, in PAIRS order.
    pub(crate) pairs: [String; 10],
}

/// A herb's value budget (`[budget]`): base + floor(level / per_levels) + rarity[its rarity].
#[derive(Deserialize)]
pub(crate) struct Budget {
    pub(crate) base: u32,
    pub(crate) per_levels: u32,
    /// By [`Rarity`], lowercase.
    pub(crate) rarity: BTreeMap<String, u32>,
}

/// An alchemy herb (`[[herb]]`).
#[derive(Deserialize, Debug)]
pub(crate) struct Herb {
    /// A herbs.toml item.
    pub(crate) item: u32,
    /// A property, Toxic or Purifying, to its points.
    pub(crate) values: BTreeMap<String, u32>,
}

/// Herb Notes (`[notes]`): a herb used in `names_at` crafts shows which values it holds; in
/// `values_at`, their numbers.
#[derive(Deserialize)]
pub(crate) struct Notes {
    pub(crate) names_at: u32,
    pub(crate) values_at: u32,
}

/// The Brew Log (`[log]`): it keeps a character's last `keep` makes.
#[derive(Deserialize)]
pub(crate) struct Log {
    pub(crate) keep: u32,
}

/// The four grades and the five Strength Steps (`[grades]`).
#[derive(Deserialize)]
pub(crate) struct Grades {
    pub(crate) names: [String; 4],
    /// Each grade's item level.
    pub(crate) bands: [u8; 4],
    pub(crate) quality: [u8; 4],
    /// Copper.
    pub(crate) sell_price: [u32; 4],
    /// x1 to x2.
    pub(crate) steps: [f32; 5],
    /// The grade whose item name has no prefix.
    pub(crate) standard: String,
}

/// Each family's threshold per grade, the same for every property (`[ladder]`).
#[derive(Deserialize)]
pub(crate) struct Ladder {
    pub(crate) potion: [u32; 4],
    pub(crate) hybrid: [u32; 4],
    pub(crate) tincture: [u32; 4],
    pub(crate) oil: [u32; 4],
    pub(crate) essence: [u32; 4],
}

/// The most Toxic a Tincture and an Essence can carry.
#[derive(Deserialize)]
pub(crate) struct CarriedMax {
    pub(crate) tincture: u32,
    pub(crate) essence: u32,
}

/// The Toxic rule's tunables (`[toxic]`).
#[derive(Deserialize)]
pub(crate) struct Toxic {
    /// The points each net Toxic point pays into a craft that holds.
    pub(crate) payoff: f32,
    /// The share of net Toxic a Tincture or Essence carries, rounded down.
    pub(crate) keep: f32,
    /// The most wash herbs a make whose body is not herbs takes.
    pub(crate) wash: u32,
    pub(crate) carried_max: CarriedMax,
}

/// What the products do when used (`[use]`). Durations are real seconds.
#[derive(Deserialize)]
pub(crate) struct Use {
    pub(crate) potion_cooldown_secs: u32,
    pub(crate) potion_shield_secs: u32,
    pub(crate) oil_shield_secs: u32,
    pub(crate) oil_secs: u32,
    pub(crate) flask_secs: u32,
    pub(crate) oil_proc_chance: f32,
    /// The stock spells the item rows name, so the client lets them be used.
    pub(crate) potion_spell: u32,
    pub(crate) potion_spell_name: String,
    /// The stock potion cooldown group.
    pub(crate) potion_category: u32,
    pub(crate) oil_spell: u32,
    pub(crate) oil_spell_name: String,
    pub(crate) flask_spell: u32,
    pub(crate) flask_spell_name: String,
    /// The stock aura the client shows while a flask is on.
    pub(crate) flask_aura_spell: u32,
    pub(crate) flask_aura_spell_name: String,
}

/// A make (`[[make]]`).
#[derive(Deserialize)]
pub(crate) struct Make {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) station: String,
    pub(crate) verb: String,
    /// The window's text.
    pub(crate) rule: String,
    /// `[min, max]` herbs as the body.
    pub(crate) herbs: Option<[u32; 2]>,
    /// Tinctures of one property as the body.
    pub(crate) tinctures: Option<u32>,
    /// Essences as the body.
    pub(crate) essences: Option<u32>,
    /// Any fish.toml `[[fish]]` items, never a legend.
    pub(crate) fish: Option<u32>,
    /// The most wash herbs.
    pub(crate) wash: Option<u32>,
    /// The make whose first product opens it per character; "" for one open from the start.
    pub(crate) opens_after: String,
}

/// An alchemy table (`[[table]]`).
#[derive(Deserialize)]
pub(crate) struct Table {
    pub(crate) key: String,
    pub(crate) name: String,
    /// The hub NPC entry whose gossip opens it.
    pub(crate) alchemist: u32,
    /// The grades whose Rumours the alchemist sells.
    pub(crate) tiers: Vec<String>,
    /// Said on a character's first visit.
    pub(crate) line: String,
}

/// Rumour prices (`[rumour]`).
#[derive(Deserialize)]
pub(crate) struct Rumour {
    /// Copper, by grade name.
    pub(crate) price: BTreeMap<String, u32>,
}

/// Murky Sludge (`[sludge]`), what every failed or empty craft makes.
#[derive(Deserialize)]
pub(crate) struct Sludge {
    pub(crate) entry: u32,
    pub(crate) name: String,
    pub(crate) sell_price: u32,
    pub(crate) icon: String,
    pub(crate) icon_base: u32,
    pub(crate) display_id: u32,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Family {
    Potion,
    Oil,
    Hybrid,
    Tincture,
    Essence,
    Flask,
}

/// A product's number at the four grades: one value, or a range's two ends.
#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(untagged)]
pub(crate) enum Num {
    One([u32; 4]),
    Range([[u32; 2]; 4]),
}

/// One of a product's effects (`effects`), run in order when it is used: a kind, and its
/// parameters, each a `nums` key or a literal.
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub(crate) struct UseEffect {
    pub(crate) kind: String,
    #[serde(flatten)]
    pub(crate) params: BTreeMap<String, toml::Value>,
}

/// A product kind (`[[product]]`).
#[derive(Deserialize, PartialEq, Debug)]
pub(crate) struct Product {
    pub(crate) family: Family,
    /// A property, a hybrid's pair, or a flask's two Essences in PROPS order, as `"A+B"`.
    pub(crate) key: String,
    /// The base name; a grade prefixes it, except for a flask.
    pub(crate) name: String,
    pub(crate) icon: String,
    pub(crate) icon_base: u32,
    /// Every row of the kind shares it (ADR 0025).
    pub(crate) display_id: u32,
    /// The first item entry of its block.
    pub(crate) entry: u32,
    /// The rows in its block.
    pub(crate) block: u32,
    #[serde(default)]
    pub(crate) nums: BTreeMap<String, Num>,
    /// The tooltip line: `{n}` a number, `{n.0}` and `{n.1}` a range's ends, `{n*5}` the number
    /// times 5 after the step.
    #[serde(default)]
    pub(crate) text: String,
    /// An Essence's flask stat, a `nums` key.
    pub(crate) stat: Option<String>,
    /// An oil that procs on every hit.
    #[serde(default)]
    pub(crate) every_hit: bool,
    #[serde(default)]
    pub(crate) effects: Vec<UseEffect>,
}

#[derive(Deserialize)]
struct File {
    props: Props,
    budget: Budget,
    herb: Vec<Herb>,
    notes: Notes,
    log: Log,
    grades: Grades,
    ladder: Ladder,
    toxic: Toxic,
    #[serde(rename = "use")]
    use_: Use,
    make: Vec<Make>,
    table: Vec<Table>,
    rumour: Rumour,
    sludge: Sludge,
    product: Vec<Product>,
}

/// A grade, 0 (Minor) to 3 (Greater).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct Grade(pub(crate) u8);

/// A Strength Step, 0 (x1) to 4 (x2).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct Step(pub(crate) u8);

const GRADES: u32 = 4;
const STEPS: u32 = 5;
/// The grade and step pairs a product comes in: state = grade x [`STEPS`] + step.
const STATES: u32 = GRADES * STEPS;

/// The last alchemy entry, as the file's header gives it.
const LAST_ENTRY: u32 = 971649;

/// Every row stacks to this many.
const STACK: u32 = 20;

/// An Essence's text starts so; the flask's tooltip takes the rest as the stat's label.
const IN_A_FLASK: &str = "In a flask: ";

/// `item_class` consumable, trade goods and junk.
const CONSUMABLE: u32 = 0;
const TRADE_GOODS: u32 = 7;
const JUNK: u32 = 15;

/// Each effect kind of the file's header, and its parameters that name a `nums` key.
const KINDS: [(&str, &[&str]); 11] = [
    ("heal", &["amount"]),
    ("restore_power", &["mana", "power"]),
    ("shield", &["amount", "burst", "mana_per_hit"]),
    ("thrown_damage", &["amount"]),
    ("damage", &["amount"]),
    ("drop_threat", &["amount"]),
    ("heal_over_time", &["tick", "total"]),
    ("heal_pct_of_damage", &[]),
    ("sleep", &[]),
    ("burn_mana", &["amount"]),
    ("invisible", &[]),
];

fn state(grade: Grade, step: Step) -> u32 {
    u32::from(grade.0) * STEPS + u32::from(step.0)
}

fn of_state(s: u32) -> (Grade, Step) {
    (Grade((s / STEPS) as u8), Step((s % STEPS) as u8))
}

/// The first index of first state `s1` in a flask of one Essence twice, whose rows hold the
/// pairs s1 <= s2.
fn twofold_start(s1: u32) -> u32 {
    s1 * STATES - s1 * s1.saturating_sub(1) / 2
}

/// A product key's two halves: a hybrid's or a flask's properties.
fn halves(key: &str) -> (&str, &str) {
    key.split_once('+').unwrap_or((key, key))
}

/// A flask of one Essence twice.
fn twofold(p: &Product) -> bool {
    let (a, b) = halves(&p.key);
    p.family == Family::Flask && a == b
}

/// The size of a product's block, by its family's formula.
fn block_of(f: &File, p: &Product) -> u32 {
    let max = &f.toxic.carried_max;
    match p.family {
        Family::Potion | Family::Oil | Family::Hybrid => STATES,
        Family::Tincture => STATES * (max.tincture + 1),
        Family::Essence => STATES * (max.essence + 1),
        Family::Flask if twofold(p) => twofold_start(STATES),
        Family::Flask => STATES * STATES,
    }
}

/// What an alchemy item entry is.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Row {
    Potion {
        product: &'static Product,
        grade: Grade,
        step: Step,
    },
    Oil {
        product: &'static Product,
        grade: Grade,
        step: Step,
    },
    Hybrid {
        product: &'static Product,
        grade: Grade,
        step: Step,
    },
    /// `toxic`: the Toxic it carries.
    Tincture {
        product: &'static Product,
        grade: Grade,
        step: Step,
        toxic: u32,
    },
    Essence {
        product: &'static Product,
        grade: Grade,
        step: Step,
        toxic: u32,
    },
    /// `first`: the Essence of the key's first property; `second`: the other's. In a flask of one
    /// Essence twice, `first` is the lower state.
    Flask {
        product: &'static Product,
        first: (Grade, Step),
        second: (Grade, Step),
    },
    Sludge,
}

/// The row at index `i` of `p`'s block.
fn decode(f: &File, p: &'static Product, i: u32) -> Row {
    let max = &f.toxic.carried_max;
    match p.family {
        Family::Potion | Family::Oil | Family::Hybrid => {
            let (grade, step) = of_state(i);
            match p.family {
                Family::Potion => Row::Potion {
                    product: p,
                    grade,
                    step,
                },
                Family::Oil => Row::Oil {
                    product: p,
                    grade,
                    step,
                },
                _ => Row::Hybrid {
                    product: p,
                    grade,
                    step,
                },
            }
        }
        Family::Tincture => {
            let (grade, step) = of_state(i / (max.tincture + 1));
            Row::Tincture {
                product: p,
                grade,
                step,
                toxic: i % (max.tincture + 1),
            }
        }
        Family::Essence => {
            let (grade, step) = of_state(i / (max.essence + 1));
            Row::Essence {
                product: p,
                grade,
                step,
                toxic: i % (max.essence + 1),
            }
        }
        Family::Flask if twofold(p) => {
            let s1 = (0..STATES)
                .rev()
                .find(|&s| twofold_start(s) <= i)
                .unwrap_or(0);
            Row::Flask {
                product: p,
                first: of_state(s1),
                second: of_state(s1 + i - twofold_start(s1)),
            }
        }
        Family::Flask => Row::Flask {
            product: p,
            first: of_state(i / STATES),
            second: of_state(i % STATES),
        },
    }
}

/// The item entry of `row`, by the file header's formula.
pub(crate) fn entry_of(row: &Row) -> u32 {
    let max = &toxic().carried_max;
    match *row {
        Row::Potion {
            product,
            grade,
            step,
        }
        | Row::Oil {
            product,
            grade,
            step,
        }
        | Row::Hybrid {
            product,
            grade,
            step,
        } => product.entry + state(grade, step),
        Row::Tincture {
            product,
            grade,
            step,
            toxic,
        } => product.entry + state(grade, step) * (max.tincture + 1) + toxic,
        Row::Essence {
            product,
            grade,
            step,
            toxic,
        } => product.entry + state(grade, step) * (max.essence + 1) + toxic,
        Row::Flask {
            product,
            first,
            second,
        } => {
            let (a, b) = (state(first.0, first.1), state(second.0, second.1));
            let index = if twofold(product) {
                let (s1, s2) = (a.min(b), a.max(b));
                twofold_start(s1) + s2 - s1
            } else {
                a * STATES + b
            };
            product.entry + index
        }
        Row::Sludge => sludge().entry,
    }
}

/// What item entry `entry` is; None for an entry outside alchemy's.
pub(crate) fn row(entry: u32) -> Option<Row> {
    if entry == sludge().entry {
        return Some(Row::Sludge);
    }
    let p = products()
        .into_iter()
        .find(|p| (p.entry..p.entry + p.block).contains(&entry))?;
    Some(decode(file(), p, entry - p.entry))
}

/// A step's factor, x1 to x2.
fn factor(f: &File, step: Step) -> f32 {
    f.grades.steps[usize::from(step.0)]
}

/// `n` times `factor`, rounded half up (the prototype's `Math.round`).
fn scale(n: u32, factor: f32) -> u32 {
    (n as f32 * factor).round() as u32
}

fn ladder_in(f: &File, family: Family) -> [u32; 4] {
    let l = &f.ladder;
    match family {
        Family::Potion => l.potion,
        Family::Oil => l.oil,
        Family::Hybrid => l.hybrid,
        Family::Tincture => l.tincture,
        Family::Essence => l.essence,
        Family::Flask => panic!("a flask has no ladder: its grades are its Essences'"),
    }
}

fn points_in(f: &File, family: Family, grade: Grade, step: Step) -> f32 {
    ladder_in(f, family)[usize::from(grade.0)] as f32 * factor(f, step)
}

/// The points a product of `family` at `grade` and `step` passes up the ladder: the grade's
/// threshold times the step (a Minor Tincture x1.25 is 6.25).
pub(crate) fn points(family: Family, grade: Grade, step: Step) -> f32 {
    points_in(file(), family, grade, step)
}

/// `p`'s tooltip line at `grade` and `step`, its numbers filled in.
pub(crate) fn text(p: &Product, grade: Grade, step: Step) -> String {
    text_at(file(), p, grade, step)
}

/// `n` at `step`, rounded half up.
pub(crate) fn scaled(n: u32, step: Step) -> u32 {
    scale(n, factor(file(), step))
}

/// Points as the prototype's `fmtPts` writes them: "15", "7.5", "6.25".
pub(crate) fn fmt_points(points: f32) -> String {
    if points.fract() == 0.0 {
        format!("{points}")
    } else {
        let two = format!("{points:.2}");
        two.strip_suffix('0').map_or(two.clone(), str::to_owned)
    }
}

/// A step as the prototype's `fmtStep` writes it: "x1", "x1.25".
pub(crate) fn fmt_step(step: Step) -> String {
    step_text(file(), step)
}

fn step_text(f: &File, step: Step) -> String {
    format!("x{}", fmt_points(factor(f, step)))
}

/// `secs` as a tooltip says it: "2 hours", "30 min".
fn fmt_secs(secs: u32) -> String {
    match secs {
        3600 => "1 hour".to_owned(),
        s if s % 3600 == 0 => format!("{} hours", s / 3600),
        s if s % 60 == 0 => format!("{} min", s / 60),
        s => format!("{s} s"),
    }
}

/// The number placeholder `hole` names (`n`, `n.0`, `n.1` or `n*5`) at `grade`, scaled by
/// `factor`; None when `nums` has no such number.
fn number(hole: &str, nums: &BTreeMap<String, Num>, grade: Grade, factor: f32) -> Option<u32> {
    let g = usize::from(grade.0);
    if let Some((name, times)) = hole.split_once('*') {
        let Num::One(v) = nums.get(name)? else {
            return None;
        };
        return Some(scale(v[g], factor) * times.parse::<u32>().ok()?);
    }
    if let Some((name, end)) = hole.split_once('.') {
        let Num::Range(v) = nums.get(name)? else {
            return None;
        };
        return Some(scale(*v[g].get(end.parse::<usize>().ok()?)?, factor));
    }
    let Num::One(v) = nums.get(hole)? else {
        return None;
    };
    Some(scale(v[g], factor))
}

/// `text` with each placeholder filled from `nums` at `grade`, scaled by `factor`; an error names
/// the first one `nums` cannot fill.
fn fill(
    text: &str,
    nums: &BTreeMap<String, Num>,
    grade: Grade,
    factor: f32,
) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let close = rest[open..]
            .find('}')
            .ok_or_else(|| format!("{text:?} leaves a {{ open"))?
            + open;
        let hole = &rest[open + 1..close];
        let n = number(hole, nums, grade, factor)
            .ok_or_else(|| format!("{{{hole}}} names no number of its product"))?;
        out.push_str(&n.to_string());
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// A product's text at `grade` and `step`; the load checks that every one fills.
fn text_at(f: &File, p: &Product, grade: Grade, step: Step) -> String {
    fill(&p.text, &p.nums, grade, factor(f, step)).unwrap_or_default()
}

/// `base` with `grade`'s name before it, none for the standard grade.
fn graded(f: &File, grade: Grade, base: &str) -> String {
    let name = &f.grades.names[usize::from(grade.0)];
    if *name == f.grades.standard {
        base.to_owned()
    } else {
        format!("{name} {base}")
    }
}

/// A Tincture's or Essence's first sentences: its step, its points and its Toxic.
fn carried(f: &File, p: &Product, grade: Grade, step: Step, toxic: u32) -> String {
    let toxic = if toxic > 0 {
        format!(" Carries Toxic {toxic}: it rolls again wherever it goes next.")
    } else {
        " Clean.".to_owned()
    };
    format!(
        "Strength {}. Carries {} {} points.{toxic}",
        step_text(f, step),
        fmt_points(points_in(f, p.family, grade, step)),
        p.key
    )
}

/// The item row of `row` at `entry`. `essence` finds a flask's Essence product by property.
fn template_of(
    f: &File,
    entry: u32,
    row: &Row,
    essence: &dyn Fn(&str) -> Option<&'static Product>,
) -> Template {
    let u = &f.use_;
    let graded_row = |p: &Product, grade: Grade, name: String| {
        let g = usize::from(grade.0);
        Template {
            entry,
            name,
            quality: f.grades.quality[g],
            item_level: f.grades.bands[g],
            display_id: p.display_id,
            sell_price: f.grades.sell_price[g],
            stackable: STACK,
            ..Template::default()
        }
    };
    let strength = |step: Step| format!("Strength {}.", step_text(f, step));
    match *row {
        Row::Potion {
            product,
            grade,
            step,
        }
        | Row::Hybrid {
            product,
            grade,
            step,
        } => Template {
            class: CONSUMABLE,
            use_spell: u.potion_spell,
            use_category: u.potion_category,
            use_category_cooldown_ms: (u.potion_cooldown_secs * 1000) as i32,
            description: format!(
                "Use: {} {}",
                text_at(f, product, grade, step),
                strength(step)
            ),
            ..graded_row(product, grade, graded(f, grade, &product.name))
        },
        Row::Oil {
            product,
            grade,
            step,
        } => Template {
            class: CONSUMABLE,
            use_spell: u.oil_spell,
            description: format!(
                "Use: coat a weapon for {}. {} {}",
                fmt_secs(u.oil_secs),
                text_at(f, product, grade, step),
                strength(step)
            ),
            ..graded_row(product, grade, graded(f, grade, &product.name))
        },
        Row::Tincture {
            product,
            grade,
            step,
            toxic,
        } => Template {
            class: TRADE_GOODS,
            journal: Some(Profession::Alchemy),
            description: carried(f, product, grade, step, toxic),
            ..graded_row(product, grade, graded(f, grade, &product.name))
        },
        Row::Essence {
            product,
            grade,
            step,
            toxic,
        } => Template {
            class: TRADE_GOODS,
            journal: Some(Profession::Alchemy),
            description: format!(
                "{} {}",
                carried(f, product, grade, step, toxic),
                text_at(f, product, grade, step)
            ),
            ..graded_row(product, grade, graded(f, grade, &product.name))
        },
        Row::Flask {
            product,
            first,
            second,
        } => {
            let (a, b) = halves(&product.key);
            let part = |prop: &str, (grade, step): (Grade, Step)| {
                let Some(e) = essence(prop) else {
                    return String::new();
                };
                let text = text_at(f, e, grade, step);
                let label = text.strip_prefix(IN_A_FLASK).unwrap_or(&text);
                format!(
                    "{} ({} {})",
                    label.trim_end_matches('.'),
                    graded(f, grade, &e.name),
                    step_text(f, step)
                )
            };
            Template {
                class: CONSUMABLE,
                use_spell: u.flask_spell,
                description: format!(
                    "Use: {} and {} for {}. Survives death; one flask at a time.",
                    part(a, first),
                    part(b, second),
                    fmt_secs(u.flask_secs)
                ),
                ..graded_row(product, first.0.max(second.0), product.name.clone())
            }
        }
        Row::Sludge => Template {
            entry,
            name: f.sludge.name.clone(),
            quality: 0,
            item_level: 1,
            class: JUNK,
            display_id: f.sludge.display_id,
            sell_price: f.sludge.sell_price,
            stackable: STACK,
            journal: Some(Profession::Alchemy),
            description: "Vendor junk.".to_owned(),
            ..Template::default()
        },
    }
}

/// The item rows of `products`, by entry; with `sludge`, Murky Sludge's too. A flask finds its
/// Essences in the shipped products, then in `products`.
fn build(f: &File, products: &'static [Product], sludge: bool) -> HashMap<u32, Template> {
    let shipped = &table().product;
    let essence = |prop: &str| {
        shipped
            .iter()
            .chain(products)
            .find(|p| p.family == Family::Essence && p.key == prop)
    };
    let mut items = HashMap::new();
    for p in products {
        for i in 0..p.block {
            let entry = p.entry + i;
            items.insert(entry, template_of(f, entry, &decode(f, p, i), &essence));
        }
    }
    if sludge {
        let entry = f.sludge.entry;
        items.insert(entry, template_of(f, entry, &Row::Sludge, &essence));
    }
    items
}

/// The shipped item rows, Murky Sludge's with them.
fn shipped_items() -> &'static HashMap<u32, Template> {
    static ITEMS: OnceLock<HashMap<u32, Template>> = OnceLock::new();
    ITEMS.get_or_init(|| build(table(), &table().product, true))
}

/// The item row `entry`, shipped or from a product a test added.
pub(crate) fn item(entry: u32) -> Option<&'static Template> {
    shipped_items()
        .get(&entry)
        .or_else(|| extra().into_iter().find_map(|(_, items)| items.get(&entry)))
}

/// The key `budget.rarity` holds a rarity under.
fn rarity_key(r: Rarity) -> &'static str {
    match r {
        Rarity::Common => "common",
        Rarity::Uncommon => "uncommon",
        Rarity::Rare => "rare",
        Rarity::Legendary => "legendary",
    }
}

const RARITIES: [Rarity; 4] = [
    Rarity::Common,
    Rarity::Uncommon,
    Rarity::Rare,
    Rarity::Legendary,
];

fn budget_of(b: &Budget, level: u8, rarity: Option<Rarity>) -> u32 {
    b.base + u32::from(level) / b.per_levels + rarity.map_or(0, |r| b.rarity[rarity_key(r)])
}

/// Checks a parsed `alchemy.toml` on its own, panicking on the first row that breaks a rule.
fn check(file: &File) {
    let props = &file.props;
    let order = &props.order;
    for h in &file.herb {
        for name in h.values.keys() {
            assert!(
                order.contains(name) || *name == props.toxic || *name == props.purifying,
                "herb {}'s value {name} is no property, Toxic or Purifying",
                h.item
            );
        }
    }
    for r in RARITIES {
        assert!(
            file.budget.rarity.contains_key(rarity_key(r)),
            "[budget] rarity has no {}",
            rarity_key(r)
        );
    }
    let l = &file.ladder;
    for (family, ladder) in [
        ("potion", l.potion),
        ("hybrid", l.hybrid),
        ("tincture", l.tincture),
        ("oil", l.oil),
        ("essence", l.essence),
    ] {
        assert!(
            ladder.windows(2).all(|w| w[0] < w[1]),
            "the {family} ladder {ladder:?} does not rise"
        );
    }
    assert!(
        file.grades.names.contains(&file.grades.standard),
        "[grades] standard {} is no grade",
        file.grades.standard
    );
    // The most Toxic a Tincture or Essence can carry: the most any herb holds, from the most
    // herbs its make takes.
    let most = file
        .herb
        .iter()
        .filter_map(|h| h.values.get(&props.toxic))
        .copied()
        .max()
        .unwrap_or(0);
    let herbs_of = |key: &str| {
        file.make
            .iter()
            .find(|m| m.key == key)
            .and_then(|m| m.herbs)
            .map_or(0, |[_, max]| max)
    };
    let carried = |herbs: u32| (most as f32 * file.toxic.keep * herbs as f32).floor() as u32;
    let max = &file.toxic.carried_max;
    assert!(
        max.tincture == carried(herbs_of("tincture"))
            && max.essence == carried(herbs_of("essence")),
        "[toxic] carried_max is not the most Toxic a Tincture and an Essence can carry"
    );
    check_products(file);
    for m in &file.make {
        assert!(
            m.opens_after.is_empty()
                || file
                    .make
                    .iter()
                    .any(|o| o.key == m.opens_after && o.key != m.key),
            "make {}'s opens_after {:?} is no other make",
            m.key,
            m.opens_after
        );
    }
    for g in &file.grades.names {
        assert!(
            file.rumour.price.contains_key(g),
            "[rumour] has no price for {g}"
        );
    }
    for t in &file.table {
        for tier in &t.tiers {
            assert!(
                file.grades.names.contains(tier),
                "table {}'s tier {tier} is no grade",
                t.key
            );
        }
    }
}

/// The product roster, the blocks, the display ids, the effects and the texts.
fn check_products(file: &File) {
    let order = &file.props.order;
    let keys = |family: Family| {
        let mut keys: Vec<&str> = file
            .product
            .iter()
            .filter(|p| p.family == family)
            .map(|p| p.key.as_str())
            .collect();
        keys.sort_unstable();
        keys
    };
    let sorted = |want: Vec<String>| {
        let mut want = want;
        want.sort_unstable();
        want
    };
    let props = sorted(order.to_vec());
    for family in [
        Family::Potion,
        Family::Oil,
        Family::Tincture,
        Family::Essence,
    ] {
        assert!(
            keys(family) == props,
            "the {family:?} products are not one per [props] order property"
        );
    }
    assert!(
        keys(Family::Hybrid) == sorted(file.props.pairs.to_vec()),
        "the hybrids are not one per [props] pairs pair"
    );
    let flasks: Vec<String> = (0..order.len())
        .flat_map(|i| (i..order.len()).map(move |j| format!("{}+{}", order[i], order[j])))
        .collect();
    assert!(
        keys(Family::Flask) == sorted(flasks),
        "the flasks are not one per pair of properties in PROPS order"
    );
    let mut blocks = vec![(file.sludge.entry, 1, file.sludge.name.as_str())];
    let mut displays = vec![file.sludge.display_id];
    for p in &file.product {
        let want = block_of(file, p);
        assert!(
            p.block == want,
            "product {} has a block of {}, not {want}",
            p.name,
            p.block
        );
        blocks.push((p.entry, p.block, p.name.as_str()));
        displays.push(p.display_id);
        check_effects(p);
        for g in 0..GRADES as u8 {
            if let Err(e) = fill(&p.text, &p.nums, Grade(g), 1.0) {
                panic!("product {}'s text: {e}", p.name);
            }
        }
        if p.family == Family::Essence {
            assert!(
                p.stat.as_ref().is_some_and(|s| p.nums.contains_key(s))
                    && p.text.starts_with(IN_A_FLASK),
                "Essence {} needs a stat among its nums and a text that starts {IN_A_FLASK:?}",
                p.name
            );
        }
    }
    blocks.sort_unstable();
    for w in blocks.windows(2) {
        assert!(
            w[0].0 + w[0].1 <= w[1].0,
            "the blocks of {} and {} overlap",
            w[0].2,
            w[1].2
        );
    }
    let last = blocks
        .last()
        .map_or(0, |&(entry, block, _)| entry + block - 1);
    assert!(
        last == LAST_ENTRY,
        "the last entry is {last}, not {LAST_ENTRY}"
    );
    displays.sort_unstable();
    assert!(
        displays.windows(2).all(|w| w[0] != w[1]),
        "two product kinds share a display id"
    );
}

/// Every effect is a known kind, and every number it names is one of its product's.
fn check_effects(p: &Product) {
    for e in &p.effects {
        let Some((_, names)) = KINDS.iter().find(|(kind, _)| *kind == e.kind) else {
            panic!("product {}'s effect kind {} is unknown", p.name, e.kind);
        };
        for name in names.iter().copied() {
            let Some(value) = e.params.get(name) else {
                continue;
            };
            assert!(
                value.as_str().is_some_and(|n| p.nums.contains_key(n)),
                "product {}'s {} {name} names no number of its nums",
                p.name,
                e.kind
            );
        }
    }
}

/// Checks a parsed `alchemy.toml` against the other content: herbs.toml and the hub NPCs.
fn check_world(file: &File) {
    // The shipped plant rows only: a test's added zones may list a herb at another level.
    let plants = crate::herbs::shipped_plants();
    for h in &file.herb {
        assert!(
            crate::items::template(h.item)
                .is_some_and(|t| t.journal == Some(Profession::Herbalism)),
            "herb {} is no herbs.toml item",
            h.item
        );
        let rows = plants.iter().filter(|p| p.item == h.item);
        let level = rows
            .clone()
            .map(|p| {
                if p.zone == crate::herbs::ANY {
                    1
                } else {
                    p.level
                }
            })
            .min()
            .unwrap_or(1) as u8;
        let want = budget_of(&file.budget, level, rows.map(|p| p.rarity).max());
        let sum: u32 = h.values.values().sum();
        assert!(
            sum == want,
            "herb {}'s values add up to {sum}, not its budget {want}",
            h.item
        );
    }
    let npcs = crate::hub::npcs();
    for t in &file.table {
        assert!(
            npcs.iter().any(|n| n.entry == t.alchemist && n.alchemist),
            "table {}'s alchemist {} is no hub NPC",
            t.key,
            t.alchemist
        );
    }
}

/// Parses and checks an `alchemy.toml`'s text, on its own.
fn load(text: &str) -> File {
    let file: File = toml::from_str(text).expect("content/alchemy.toml parses");
    check(&file);
    file
}

/// The file, checked on its own. The item rows read only this: `crate::items` may ask for a row
/// while another content file is still loading.
fn table() -> &'static File {
    static FILE: OnceLock<File> = OnceLock::new();
    FILE.get_or_init(|| load(include_str!("../content/alchemy.toml")))
}

/// The file, once it has passed the checks against the other content too.
fn file() -> &'static File {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| check_world(table()));
    table()
}

/// Herbs and products a test added, in `alchemy.toml`'s shape.
#[derive(Deserialize)]
struct Added {
    #[serde(default)]
    herb: Vec<Herb>,
    #[serde(default)]
    product: Vec<Product>,
}

type AddedRows = (&'static Added, &'static HashMap<u32, Template>);

/// Each added file's text, what it parsed to, and its products' item rows.
fn added() -> &'static RwLock<Vec<(String, AddedRows)>> {
    static ADDED: OnceLock<RwLock<Vec<(String, AddedRows)>>> = OnceLock::new();
    ADDED.get_or_init(Default::default)
}

/// Every added file, oldest first.
fn extra() -> Vec<AddedRows> {
    added().read().unwrap().iter().map(|&(_, a)| a).collect()
}

/// Adds the herbs and products in `toml` alongside the shipped ones, for the rest of the process.
/// Parallel tests add the same text; it is added once.
pub(crate) fn add(toml: &str) {
    if added().read().unwrap().iter().any(|(text, _)| text == toml) {
        return;
    }
    let parsed: &'static Added = Box::leak(Box::new(
        toml::from_str(toml).expect("the added alchemy content parses"),
    ));
    let items = Box::leak(Box::new(build(file(), &parsed.product, false)));
    let mut all = added().write().unwrap();
    if all.iter().all(|(text, _)| text != toml) {
        all.push((toml.to_owned(), (parsed, items)));
    }
}

pub(crate) fn props() -> &'static Props {
    &file().props
}

/// Every alchemy herb, the shipped ones first.
pub(crate) fn herbs() -> Vec<&'static Herb> {
    file()
        .herb
        .iter()
        .chain(extra().into_iter().flat_map(|(a, _)| &a.herb))
        .collect()
}

/// The alchemy herb whose item is `item`.
pub(crate) fn herb(item: u32) -> Option<&'static Herb> {
    herbs().into_iter().find(|h| h.item == item)
}

pub(crate) fn notes() -> &'static Notes {
    &file().notes
}

pub(crate) fn log() -> &'static Log {
    &file().log
}

pub(crate) fn grades() -> &'static Grades {
    &file().grades
}

/// `family`'s threshold per grade. A flask has none: its grades are its Essences'.
pub(crate) fn ladder(family: Family) -> [u32; 4] {
    ladder_in(file(), family)
}

pub(crate) fn toxic() -> &'static Toxic {
    &file().toxic
}

pub(crate) fn use_() -> &'static Use {
    &file().use_
}

pub(crate) fn makes() -> &'static [Make] {
    &file().make
}

pub(crate) fn make(key: &str) -> Option<&'static Make> {
    makes().iter().find(|m| m.key == key)
}

pub(crate) fn tables() -> &'static [Table] {
    &file().table
}

/// The table whose alchemist is the hub NPC `entry`.
pub(crate) fn table_of_alchemist(entry: u32) -> Option<&'static Table> {
    tables().iter().find(|t| t.alchemist == entry)
}

/// A Rumour of `grade`'s price, in copper.
pub(crate) fn rumour_price(grade: Grade) -> u32 {
    let f = file();
    f.rumour.price[&f.grades.names[usize::from(grade.0)]]
}

pub(crate) fn sludge() -> &'static Sludge {
    &file().sludge
}

/// Every product kind, the shipped ones first.
pub(crate) fn products() -> Vec<&'static Product> {
    file()
        .product
        .iter()
        .chain(extra().into_iter().flat_map(|(a, _)| &a.product))
        .collect()
}

/// The product of `family` keyed `key`.
pub(crate) fn product(family: Family, key: &str) -> Option<&'static Product> {
    products()
        .into_iter()
        .find(|p| p.family == family && p.key == key)
}

/// A herb's level: the lowest its pool rows list it at (`cooking::ingredient_level`).
pub(crate) fn herb_level(item: u32) -> u8 {
    crate::cooking::ingredient_level(item)
}

/// A herb's rarity: the rarest of its pool rows in every zone and the shared finds; None for an
/// item no pool lists.
pub(crate) fn herb_rarity(item: u32) -> Option<Rarity> {
    crate::herbs::zones()
        .into_iter()
        .map(|z| z.key.as_str())
        .chain([crate::herbs::ANY])
        .flat_map(crate::herbs::plants_in)
        .filter(|p| p.item == item)
        .map(|p| p.rarity)
        .max()
}

/// What a herb's values add up to: base + floor(level / per_levels) + its rarity's.
pub(crate) fn budget(item: u32) -> u32 {
    budget_of(&file().budget, herb_level(item), herb_rarity(item))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn potion(key: &str, grade: u8, step: u8) -> Row {
        Row::Potion {
            product: product(Family::Potion, key).unwrap(),
            grade: Grade(grade),
            step: Step(step),
        }
    }

    #[test]
    fn the_shipped_file_loads() {
        // The shipped rows: a parallel test may have added one.
        let f = file();
        assert_eq!(f.herb.len(), 19);
        assert_eq!(makes().len(), 5);
        assert_eq!(tables().len(), 3);
        assert_eq!(f.product.len(), 45);
        assert_eq!(shipped_items().len(), 8551);
        assert_eq!(rumour_price(Grade(1)), 6000);
        assert_eq!(
            make("essence").map(|m| m.opens_after.as_str()),
            Some("tincture")
        );
        assert_eq!(
            table_of_alchemist(190076).map(|t| t.key.as_str()),
            Some("goldshire")
        );
    }

    #[test]
    fn a_herbs_budget_takes_its_lowest_level_and_rarest_rarity() {
        assert_eq!(budget(785), 5);
        assert_eq!(herb_rarity(785), Some(Rarity::Uncommon));
        assert_eq!(budget(961023), 11);
        assert_eq!(herb(785).map(|h| h.values["Arcane"]), Some(4));
    }

    #[test]
    fn an_entry_and_its_row_are_one_formula() {
        let minor = potion("Restorative", 0, 0);
        assert_eq!(entry_of(&minor), 963100);
        assert_eq!(row(963100), Some(minor));
        assert_eq!(row(963101), Some(potion("Restorative", 0, 1)));
        let restorative = product(Family::Tincture, "Restorative").unwrap();
        let tincture = |toxic| Row::Tincture {
            product: restorative,
            grade: Grade(0),
            step: Step(0),
            toxic,
        };
        assert_eq!(row(963500), Some(tincture(0)));
        assert_eq!(row(963507), Some(tincture(7)));
        assert_eq!(entry_of(&tincture(7)), 963507);
        let vigour = Row::Flask {
            product: product(Family::Flask, "Restorative+Restorative").unwrap(),
            first: (Grade(0), Step(0)),
            second: (Grade(0), Step(0)),
        };
        assert_eq!(row(966600), Some(vigour));
        let stillwater = Row::Flask {
            product: product(Family::Flask, "Calming+Calming").unwrap(),
            first: (Grade(3), Step(4)),
            second: (Grade(3), Step(4)),
        };
        assert_eq!(row(971649), Some(stillwater));
        assert_eq!(entry_of(&stillwater), 971649);
        assert_eq!(row(971650), None);
        assert_eq!(row(963000), Some(Row::Sludge));
    }

    #[test]
    fn every_entry_decodes_to_a_row_that_encodes_back() {
        for p in &file().product {
            for entry in p.entry..p.entry + p.block {
                let r = row(entry).unwrap_or_else(|| panic!("{entry} has no row"));
                assert_eq!(entry_of(&r), entry, "{r:?}");
            }
        }
    }

    #[test]
    fn a_potion_row_says_its_numbers_at_its_step() {
        let t = item(963101).unwrap();
        assert_eq!(t.name, "Minor Healing Potion");
        assert_eq!((t.quality, t.item_level, t.display_id), (1, 1, 995200));
        assert_eq!((t.use_spell, t.use_category), (2598, 4));
        assert_eq!(t.use_category_cooldown_ms, 120000);
        assert_eq!(t.stackable, 20);
        assert_eq!(t.journal, None);
        // 70 x 1.25 = 87.5 rounds to 88; 90 x 1.25 = 112.5 to 113.
        assert_eq!(
            t.description,
            "Use: Heals 88 to 113 at once. Strength x1.25."
        );
        assert_eq!(
            item(963105).unwrap().description,
            "Use: Heals 175 to 225 at once. Strength x1."
        );
        assert_eq!(item(963105).unwrap().name, "Lesser Healing Potion");
        assert_eq!(item(963110).unwrap().name, "Healing Potion");
        let draught = item(963305).unwrap();
        assert_eq!(draught.name, "Lesser Trollblood Draught");
        assert_eq!(
            draught.description,
            "Use: Heals 30 every 2 s for 10 s (150 in all). Strength x1."
        );
    }

    #[test]
    fn an_oil_row_coats_a_weapon() {
        let t = item(963200).unwrap();
        assert_eq!(t.name, "Minor Leeching Oil");
        assert_eq!(
            (t.use_spell, t.use_category, t.use_category_cooldown_ms),
            (22, 0, -1)
        );
        assert_eq!(
            t.description,
            "Use: coat a weapon for 30 min. Chance on hit: heals you for 15. Strength x1."
        );
    }

    #[test]
    fn a_tincture_or_essence_says_its_points_and_its_toxic() {
        assert_eq!(points(Family::Tincture, Grade(0), Step(1)), 6.25);
        assert_eq!(fmt_points(6.25), "6.25");
        assert_eq!(fmt_points(7.5), "7.5");
        assert_eq!(fmt_points(15.0), "15");
        assert_eq!(
            (fmt_step(Step(0)), fmt_step(Step(1))),
            ("x1".to_owned(), "x1.25".to_owned())
        );
        assert_eq!(scaled(70, Step(1)), 88);
        let tincture = Row::Tincture {
            product: product(Family::Tincture, "Restorative").unwrap(),
            grade: Grade(0),
            step: Step(1),
            toxic: 2,
        };
        let t = item(entry_of(&tincture)).unwrap();
        assert_eq!(t.name, "Minor Restorative Tincture");
        assert_eq!((t.class, t.journal), (7, Some(Profession::Alchemy)));
        assert_eq!(
            t.description,
            "Strength x1.25. Carries 6.25 Restorative points. Carries Toxic 2: it rolls again \
             wherever it goes next."
        );
        let essence = Row::Essence {
            product: product(Family::Essence, "Fortifying").unwrap(),
            grade: Grade(1),
            step: Step(1),
            toxic: 0,
        };
        let e = item(entry_of(&essence)).unwrap();
        assert_eq!(e.name, "Lesser Fortifying Essence");
        assert_eq!(
            e.description,
            "Strength x1.25. Carries 56.25 Fortifying points. Clean. In a flask: +125 Armour."
        );
    }

    #[test]
    fn a_flask_names_both_essences_and_takes_the_higher_grade() {
        let flask = Row::Flask {
            product: product(Family::Flask, "Fortifying+Volatile").unwrap(),
            first: (Grade(1), Step(0)),
            second: (Grade(1), Step(1)),
        };
        let t = item(entry_of(&flask)).unwrap();
        assert_eq!(t.name, "Flask of Stone and Ember");
        assert_eq!((t.quality, t.item_level, t.use_spell), (2, 10, 22));
        assert_eq!(
            t.description,
            "Use: +100 Armour (Lesser Fortifying Essence x1) and +31 Attack power (Lesser \
             Volatile Essence x1.25) for 2 hours. Survives death; one flask at a time."
        );
        let mixed = Row::Flask {
            product: product(Family::Flask, "Fortifying+Volatile").unwrap(),
            first: (Grade(0), Step(0)),
            second: (Grade(3), Step(0)),
        };
        assert_eq!(item(entry_of(&mixed)).unwrap().quality, 4);
    }

    #[test]
    fn murky_sludge_is_alchemy_junk() {
        let t = item(963000).unwrap();
        assert_eq!(t.name, "Murky Sludge");
        assert_eq!((t.quality, t.item_level, t.class), (0, 1, 15));
        assert_eq!(t.sell_price, 5);
        assert_eq!(t.journal, Some(Profession::Alchemy));
        assert_eq!(
            crate::items::template(963000).map(|t| t.display_id),
            Some(995245)
        );
    }

    #[test]
    fn an_added_product_and_herb_are_found() {
        add(r#"
            [[herb]]
            item = 961001
            values = { Restorative = 1 }

            [[product]]
            family = "potion"
            key = "Restorative"
            name = "Test Potion"
            icon = "potion_restorative"
            icon_base = 15710
            display_id = 995200
            entry = 972000
            block = 20
            nums = { heal = [1, 2, 3, 4] }
            text = "Heals {heal}."
        "#);
        assert!(herb(961001).is_some());
        assert_eq!(
            item(972005).map(|t| t.description.as_str()),
            Some("Use: Heals 2. Strength x1.")
        );
        assert_eq!(row(972000).map(|r| entry_of(&r)), Some(972000));
    }

    #[test]
    #[should_panic(expected = "does not rise")]
    fn a_ladder_that_falls_is_refused() {
        let text = include_str!("../content/alchemy.toml").replacen(
            "potion = [4, 12, 20, 28]",
            "potion = [4, 12, 12, 28]",
            1,
        );
        load(&text);
    }

    #[test]
    #[should_panic(expected = "names no number")]
    fn an_effect_naming_a_missing_number_is_refused() {
        let text = include_str!("../content/alchemy.toml").replacen(
            "effects = [{ kind = \"heal\", amount = \"heal\" }]",
            "effects = [{ kind = \"heal\", amount = \"healing\" }]",
            1,
        );
        load(&text);
    }

    #[test]
    #[should_panic(expected = "not its budget")]
    fn a_herb_off_its_budget_is_refused() {
        let text = include_str!("../content/alchemy.toml").replacen(
            "values = { Restorative = 2 }",
            "values = { Restorative = 3 }",
            1,
        );
        check_world(&load(&text));
    }
}
