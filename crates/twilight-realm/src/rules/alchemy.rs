//! Alchemy's rules (ADR 0025), as the Alchemy prototype's `realm` script resolves a make: pure
//! functions over `crate::alchemy`'s data and a list of item entries, with no character and no
//! RNG. Each mirrors the prototype function its doc names.

use std::collections::{BTreeMap, BTreeSet};

use crate::alchemy::{self, Family, Grade, Make, Row, Step};

/// What a list's values add up to: a property, Toxic or Purifying, to its points.
pub(crate) type Sums = BTreeMap<String, f32>;

/// What a character's Herb Note on a herb shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum NoteState {
    Unnoted,
    /// The values' names, after `names_at` crafts.
    Named,
    /// Their numbers too, after `values_at`.
    Filled,
}

/// A make's target, "potion|Restorative|Minor": a family, a property or pair, and a grade.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Target {
    pub(crate) family: Family,
    pub(crate) key: String,
    pub(crate) grade: Grade,
}

/// The prototype's `parseTarget`; None for "" or a target that names nothing.
pub(crate) fn parse_target(text: &str) -> Option<Target> {
    let mut parts = text.split('|');
    let family = family_of_key(parts.next()?)?;
    let key = parts.next()?.to_owned();
    let grade = grade_of_name(parts.next()?)?;
    Some(Target { family, key, grade })
}

/// A family's key, as a target and a grade record spell it.
pub(crate) fn family_key(family: Family) -> &'static str {
    match family {
        Family::Potion => "potion",
        Family::Oil => "oil",
        Family::Hybrid => "hybrid",
        Family::Tincture => "tincture",
        Family::Essence => "essence",
        Family::Flask => "flask",
    }
}

pub(crate) fn family_of_key(key: &str) -> Option<Family> {
    [
        Family::Potion,
        Family::Oil,
        Family::Hybrid,
        Family::Tincture,
        Family::Essence,
        Family::Flask,
    ]
    .into_iter()
    .find(|&f| family_key(f) == key)
}

pub(crate) fn grade_of_name(name: &str) -> Option<Grade> {
    let names = &alchemy::grades().names;
    names.iter().position(|n| n == name).map(|g| Grade(g as u8))
}

/// The family a make's product comes in; a potion make's hybrid is decided by [`resolve`].
pub(crate) fn family_of(make: &Make) -> Family {
    family_of_key(&make.key).unwrap_or(Family::Potion)
}

/// The family whose ladder a make's vials are measured on: a flask's Essences sit on the Essence
/// ladder (the prototype's `RING_FAMILY`).
pub(crate) fn ring_family(make: &Make) -> Family {
    match family_of(make) {
        Family::Flask => Family::Essence,
        f => f,
    }
}

/// What an item is to a make.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Kind {
    Herb,
    Fish,
    Tincture,
    Essence,
    Other,
}

pub(crate) fn kind_of(item: u32) -> Kind {
    if alchemy::herb(item).is_some() {
        Kind::Herb
    } else if crate::fish::fish_items().any(|f| f == item) {
        Kind::Fish
    } else {
        match alchemy::row(item) {
            Some(Row::Tincture { .. }) => Kind::Tincture,
            Some(Row::Essence { .. }) => Kind::Essence,
            _ => Kind::Other,
        }
    }
}

/// How many of each kind a list holds (the prototype's `kindCounts`).
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub(crate) struct Counts {
    pub(crate) herb: u32,
    pub(crate) fish: u32,
    pub(crate) tincture: u32,
    pub(crate) essence: u32,
    pub(crate) other: u32,
}

pub(crate) fn counts(items: &[u32]) -> Counts {
    let mut n = Counts::default();
    for &item in items {
        match kind_of(item) {
            Kind::Herb => n.herb += 1,
            Kind::Fish => n.fish += 1,
            Kind::Tincture => n.tincture += 1,
            Kind::Essence => n.essence += 1,
            Kind::Other => n.other += 1,
        }
    }
    n
}

/// Each make's refusal, as the prototype's `MAKES` words it.
const RULES: [(&str, &str); 5] = [
    ("potion", "Two to four herbs."),
    ("tincture", "Five herbs."),
    (
        "essence",
        "Fifteen herbs, or three Tinctures with up to two herbs as a wash.",
    ),
    (
        "oil",
        "Two Tinctures and two fish, with up to two herbs as a wash.",
    ),
    ("flask", "Two Essences, with up to two herbs as a wash."),
];

/// The prototype's `MAKES[..].valid`: whether `counts` fill `make`, from its limits. A kind the
/// make takes no part of breaks it.
pub(crate) fn valid(make: &Make, n: Counts) -> Result<(), &'static str> {
    let herb_body = make.herbs.is_some_and(|[lo, hi]| {
        (lo..=hi).contains(&n.herb) && n.fish == 0 && n.tincture == 0 && n.essence == 0
    });
    let wash = make.wash.unwrap_or(0);
    let fish = make.fish.unwrap_or(0);
    let tincture_body = make
        .tinctures
        .is_some_and(|t| n.tincture == t && n.essence == 0 && n.fish == fish && n.herb <= wash);
    let essence_body = make
        .essences
        .is_some_and(|e| n.essence == e && n.tincture == 0 && n.fish == fish && n.herb <= wash);
    if n.other == 0 && (herb_body || tincture_body || essence_body) {
        return Ok(());
    }
    Err(RULES
        .iter()
        .find(|(k, _)| *k == make.key)
        .map_or("That won't make anything.", |(_, m)| m))
}

/// The prototype's `sumProps`: the herbs' values, added.
pub(crate) fn sum_values(herbs: &[u32]) -> Sums {
    let mut sums = Sums::new();
    for h in herbs.iter().filter_map(|&i| alchemy::herb(i)) {
        for (p, &v) in &h.values {
            *sums.entry(p.clone()).or_default() += v as f32;
        }
    }
    sums
}

fn get(sums: &Sums, p: &str) -> f32 {
    sums.get(p).copied().unwrap_or(0.0)
}

/// The prototype's `mainProp`: the target's property when the sums carry any of it, else the one
/// with the most points, the earlier in `[props] order` winning a tie.
pub(crate) fn main_prop<'a>(sums: &Sums, target: Option<&'a str>) -> Option<&'a str> {
    if let Some(t) = target.filter(|&t| get(sums, t) > 0.0) {
        return Some(t);
    }
    let mut best: Option<&'static str> = None;
    for p in alchemy::props().order.iter().map(String::as_str) {
        if get(sums, p) > 0.0 && best.is_none_or(|b| get(sums, p) > get(sums, b)) {
            best = Some(p);
        }
    }
    best
}

/// The prototype's `failChance`: 0 for no net Toxic, 1 for no main points.
pub(crate) fn fail_chance(net: f32, main: f32) -> f32 {
    if net <= 0.0 {
        0.0
    } else if main <= 0.0 {
        1.0
    } else {
        net / (net + main)
    }
}

/// The points net Toxic pays into a make that holds.
pub(crate) fn payoff(net: f32) -> f32 {
    net * alchemy::toxic().payoff
}

/// The Toxic a Tincture or Essence carries out of a make.
pub(crate) fn carried(net: f32) -> u32 {
    (net * alchemy::toxic().keep).floor() as u32
}

/// A chance as a whole percent, rounded half up.
pub(crate) fn pct(chance: f32) -> u32 {
    (chance * 100.0).round() as u32
}

/// The prototype's `stepOf`: the step `ratio` rounds down to, x1 below the second.
fn step_of(ratio: f32) -> Step {
    let steps = &alchemy::grades().steps;
    let s = (0..steps.len())
        .rev()
        .find(|&s| ratio >= steps[s])
        .unwrap_or(0);
    Step(s as u8)
}

/// The prototype's `gradeFor`: the highest grade of `family` whose threshold `pts` meets, at its
/// step.
pub(crate) fn grade_for(family: Family, pts: f32) -> Option<(Grade, Step)> {
    let ladder = alchemy::ladder(family);
    (0..ladder.len()).rev().find_map(|g| {
        let n = ladder[g] as f32;
        (pts >= n).then(|| (Grade(g as u8), step_of(pts / n)))
    })
}

/// The best hybrid a potion's sums make.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Hybrid {
    pub(crate) pair: &'static str,
    pub(crate) grade: Grade,
    pub(crate) step: Step,
    /// Both properties' points.
    pub(crate) main: f32,
}

/// A pair's two properties.
pub(crate) fn halves(pair: &str) -> (&str, &str) {
    pair.split_once('+').unwrap_or((pair, pair))
}

/// The prototype's `pickHybrid`: each pair both of whose properties the sums carry, measured at
/// the lower of the two plus half the payoff each; the best by grade, then step, then points. A
/// hybrid target wins when it qualifies at all.
pub(crate) fn pick_hybrid(sums: &Sums, net: f32, target: Option<&str>) -> Option<Hybrid> {
    let half = payoff(net) / 2.0;
    let mut best: Option<Hybrid> = None;
    for pair in &alchemy::props().pairs {
        let (a, b) = halves(pair);
        if get(sums, a) <= 0.0 || get(sums, b) <= 0.0 {
            continue;
        }
        let low = (get(sums, a) + half).min(get(sums, b) + half);
        let Some((grade, step)) = grade_for(Family::Hybrid, low) else {
            continue;
        };
        let c = Hybrid {
            pair,
            grade,
            step,
            main: get(sums, a) + get(sums, b),
        };
        if target == Some(pair.as_str()) {
            return Some(c);
        }
        if best.as_ref().is_none_or(|b| {
            (c.grade, c.step) > (b.grade, b.step)
                || (c.grade, c.step) == (b.grade, b.step) && c.main > b.main
        }) {
            best = Some(c);
        }
    }
    best
}

/// Whether the hybrid beats the single potion (the prototype's lines 690 to 699): a targeted
/// hybrid wins when it qualifies; otherwise the hybrid wins when its grade beats the single's,
/// and on a tie too unless the player targeted a single potion.
fn hybrid_wins(hy: &Hybrid, single: Option<(Grade, Step)>, target: Option<&str>) -> bool {
    let targeted_single = target.is_some_and(|t| alchemy::props().order.iter().any(|p| p == t));
    target == Some(hy.pair)
        || single.is_none_or(|(g, _)| hy.grade > g || (hy.grade == g && !targeted_single))
}

/// A make's outcome if it holds (the prototype's `resolve`).
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Resolved {
    pub(crate) sums: Sums,
    pub(crate) family: Family,
    /// The property, or a hybrid's pair; None for a flask and an empty sludge.
    pub(crate) prop: Option<String>,
    pub(crate) main: f32,
    pub(crate) toxic_in: f32,
    pub(crate) purify: f32,
    pub(crate) net: f32,
    pub(crate) fail: f32,
    pub(crate) payoff: f32,
    /// None for a flask, whose grades are its Essences'.
    pub(crate) grade: Option<(Grade, Step)>,
    pub(crate) carried: u32,
    /// The product's item entry; None for a sludge, whose reason is `note`.
    pub(crate) product: Option<u32>,
    pub(crate) note: String,
}

impl Resolved {
    fn sludge(sums: Sums, family: Family, note: String) -> Self {
        Resolved {
            sums,
            family,
            prop: None,
            main: 0.0,
            toxic_in: 0.0,
            purify: 0.0,
            net: 0.0,
            fail: 0.0,
            payoff: 0.0,
            grade: None,
            carried: 0,
            product: None,
            note,
        }
    }

    pub(crate) fn is_sludge(&self) -> bool {
        self.product.is_none()
    }
}

/// A Tincture or Essence in a list: its property, points, Toxic and grade.
struct Body {
    prop: &'static str,
    points: f32,
    toxic: u32,
}

fn body_of(item: u32) -> Option<Body> {
    match alchemy::row(item)? {
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
        } => Some(Body {
            prop: &product.key,
            points: alchemy::points(product.family, grade, step),
            toxic,
        }),
        _ => None,
    }
}

/// Whether a make's body is herbs: a potion, a Tincture, or an Essence of fifteen herbs.
fn herb_body(make: &Make, items: &[u32]) -> bool {
    make.herbs.is_some() && !items.iter().any(|&i| kind_of(i) == Kind::Tincture)
}

/// The item entry of a potion, oil or hybrid of `family` keyed `key`, or a Tincture or Essence
/// carrying `toxic`.
pub(crate) fn entry_at(
    family: Family,
    key: &str,
    grade: Grade,
    step: Step,
    toxic: u32,
) -> Option<u32> {
    let product = alchemy::product(family, key)?;
    let row = match family {
        Family::Potion => Row::Potion {
            product,
            grade,
            step,
        },
        Family::Oil => Row::Oil {
            product,
            grade,
            step,
        },
        Family::Hybrid => Row::Hybrid {
            product,
            grade,
            step,
        },
        Family::Tincture => Row::Tincture {
            product,
            grade,
            step,
            toxic,
        },
        Family::Essence => Row::Essence {
            product,
            grade,
            step,
            toxic,
        },
        Family::Flask => return None,
    };
    Some(alchemy::entry_of(&row))
}

/// The prototype's `resolve`: what `items` at `make` come out as if the roll holds. `target` is
/// the target's property or pair.
pub(crate) fn resolve(make: &Make, items: &[u32], target: Option<&str>) -> Resolved {
    let props = alchemy::props();
    let (toxic_key, purify_key) = (props.toxic.as_str(), props.purifying.as_str());
    let family = family_of(make);
    let herbs: Vec<u32> = items
        .iter()
        .copied()
        .filter(|&i| kind_of(i) == Kind::Herb)
        .collect();
    let (mut sums, prop, main, toxic_in, purify, parts) = if herb_body(make, items) {
        let sums = sum_values(&herbs);
        let Some(prop) = main_prop(&sums, target).map(str::to_owned) else {
            return Resolved::sludge(
                sums,
                family,
                "Nothing in it but Toxic and Purifying.".to_owned(),
            );
        };
        let (main, t, p) = (
            get(&sums, &prop),
            get(&sums, toxic_key),
            get(&sums, purify_key),
        );
        (sums, Some(prop), main, t, p, Vec::new())
    } else {
        let flask = family == Family::Flask;
        let body: Vec<Body> = items.iter().filter_map(|&i| body_of(i)).collect();
        let wash = sum_values(&herbs);
        let one: BTreeSet<&str> = body.iter().map(|b| b.prop).collect();
        if body.is_empty() || (!flask && one.len() != 1) {
            let note = if body.is_empty() {
                "Nothing in it but Toxic and Purifying."
            } else {
                "Tinctures of different properties will not settle together."
            };
            return Resolved::sludge(Sums::new(), family, note.to_owned());
        }
        let mut sums = Sums::new();
        let mut main = 0.0;
        let mut toxic_in = 0.0;
        for b in &body {
            *sums.entry(b.prop.to_owned()).or_default() += b.points;
            main += b.points;
            toxic_in += b.toxic as f32;
        }
        toxic_in += get(&wash, toxic_key);
        let purify = get(&wash, purify_key);
        let parts: Vec<(&str, f32)> = body.iter().map(|b| (b.prop, b.points)).collect();
        let prop = (!flask).then(|| body[0].prop.to_owned());
        (sums, prop, main, toxic_in, purify, parts)
    };
    if !herb_body(make, items) {
        if toxic_in > 0.0 {
            sums.insert(toxic_key.to_owned(), toxic_in);
        }
        if purify > 0.0 {
            sums.insert(purify_key.to_owned(), purify);
        }
    }
    let net = (toxic_in - purify).max(0.0);
    let base = Resolved {
        sums,
        family,
        prop,
        main,
        toxic_in,
        purify,
        net,
        fail: fail_chance(net, main),
        payoff: payoff(net),
        grade: None,
        carried: 0,
        product: None,
        note: String::new(),
    };
    if family == Family::Potion {
        let single = base
            .prop
            .as_deref()
            .and_then(|_| grade_for(Family::Potion, main + base.payoff));
        if let Some(hy) =
            pick_hybrid(&base.sums, net, target).filter(|h| hybrid_wins(h, single, target))
        {
            return Resolved {
                family: Family::Hybrid,
                prop: Some(hy.pair.to_owned()),
                main: hy.main,
                fail: fail_chance(net, hy.main),
                grade: Some((hy.grade, hy.step)),
                product: entry_at(Family::Hybrid, hy.pair, hy.grade, hy.step, 0),
                ..base
            };
        }
    }
    if family == Family::Flask {
        let order = |p: &str| props.order.iter().position(|o| o == p).unwrap_or(0);
        let mut after: Vec<(&str, Grade, Step)> = parts
            .iter()
            .map(|&(p, pts)| {
                let (g, s) = grade_for(Family::Essence, pts + base.payoff / 2.0)
                    .unwrap_or((Grade(0), Step(0)));
                (p, g, s)
            })
            .collect();
        after.sort_by_key(|&(p, g, s)| (order(p), g, s));
        let product = (after.len() == 2)
            .then(|| {
                let key = format!("{}+{}", after[0].0, after[1].0);
                let product = alchemy::product(Family::Flask, &key)?;
                Some(alchemy::entry_of(&Row::Flask {
                    product,
                    first: (after[0].1, after[0].2),
                    second: (after[1].1, after[1].2),
                }))
            })
            .flatten();
        return Resolved { product, ..base };
    }
    let prop = base.prop.clone().unwrap_or_default();
    let Some((grade, step)) = grade_for(family, main + base.payoff) else {
        return Resolved {
            note: format!("Too little {prop} to hold together."),
            ..base
        };
    };
    let max = &alchemy::toxic().carried_max;
    let carried = match family {
        Family::Tincture => carried(net).min(max.tincture),
        Family::Essence => carried(net).min(max.essence),
        _ => 0,
    };
    Resolved {
        grade: Some((grade, step)),
        carried,
        product: entry_at(family, &prop, grade, step, carried),
        ..base
    }
}

/// How much of a vial the notes leave unknown.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Unknown {
    None,
    /// Some of its points are known.
    Some,
    All,
}

/// One property's vial.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Vial {
    pub(crate) prop: String,
    pub(crate) value: f32,
    pub(crate) unknown: Unknown,
    pub(crate) main: bool,
    pub(crate) target: bool,
    /// Its share of the Toxic payoff.
    pub(crate) bonus: f32,
    /// The target's threshold, when it is learned or rumoured.
    pub(crate) tick: Option<u32>,
}

/// The Toxic vial.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct ToxicVial {
    pub(crate) raw: f32,
    pub(crate) purify: f32,
    pub(crate) net: f32,
    pub(crate) unknown: bool,
    pub(crate) fail: f32,
    pub(crate) bonus: f32,
}

/// What the window's vials show (the prototype's `bars`).
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Vials {
    /// The five properties, in `[props]` order.
    pub(crate) rows: Vec<Vial>,
    pub(crate) toxic: ToxicVial,
    /// The herbs with no note.
    pub(crate) unnoted: u32,
    /// The vials' top.
    pub(crate) scale: f32,
    pub(crate) hybrid: bool,
}

/// The prototype's `bars`: what `items` at `make` show from the Herb Notes (`note`), or the true
/// sums `truth` after a make. `tick_known`: the target's threshold is learned or rumoured.
pub(crate) fn vials(
    make: &Make,
    items: &[u32],
    target: Option<&Target>,
    note: &dyn Fn(u32) -> NoteState,
    tick_known: bool,
    truth: Option<&Sums>,
) -> Vials {
    let props = alchemy::props();
    let (toxic_key, purify_key) = (props.toxic.as_str(), props.purifying.as_str());
    let family = family_of(make);
    let herb_body = herb_body(make, items);
    let mut known = Sums::new();
    let mut maybe = BTreeSet::new();
    let mut unnoted = 0;
    for &item in items {
        if let Some(h) = alchemy::herb(item) {
            let state = note(item);
            if state == NoteState::Unnoted {
                unnoted += 1;
                continue;
            }
            for (q, &v) in &h.values {
                if !herb_body && q != toxic_key && q != purify_key {
                    continue;
                }
                if state == NoteState::Filled {
                    *known.entry(q.clone()).or_default() += v as f32;
                } else {
                    maybe.insert(q.as_str());
                }
            }
        } else if let Some(b) = body_of(item) {
            *known.entry(b.prop.to_owned()).or_default() += b.points;
            if b.toxic > 0 {
                *known.entry(toxic_key.to_owned()).or_default() += b.toxic as f32;
            }
        }
    }
    let vals = truth.unwrap_or(&known);
    let tgt = target
        .filter(|t| t.family == family || (family == Family::Potion && t.family == Family::Hybrid));
    let tgt_key = tgt.map(|t| t.key.as_str());
    let net = (get(vals, toxic_key) - get(vals, purify_key)).max(0.0);
    let bonus = payoff(net);
    let hy = (family == Family::Potion)
        .then(|| pick_hybrid(vals, net, tgt_key))
        .flatten()
        .filter(|h| {
            let single_target = tgt
                .filter(|t| t.family == Family::Potion)
                .map(|t| t.key.as_str());
            let single = main_prop(vals, single_target)
                .and_then(|p| grade_for(Family::Potion, get(vals, p) + bonus));
            target_wins(h, single, tgt)
        });
    let mains: Vec<&str> = if family == Family::Flask {
        props
            .order
            .iter()
            .map(String::as_str)
            .filter(|p| get(vals, p) > 0.0)
            .collect()
    } else if let Some(h) = &hy {
        let (a, b) = halves(h.pair);
        vec![a, b]
    } else {
        main_prop(vals, tgt_key).into_iter().collect()
    };
    let main: f32 = mains.iter().map(|p| get(vals, p)).sum();
    let fail = fail_chance(net, main);
    let bonus_of = |p: &str| {
        if bonus > 0.0 && mains.contains(&p) {
            bonus / mains.len() as f32
        } else {
            0.0
        }
    };
    let tgt_props: Vec<&str> = match tgt {
        Some(t) if t.family == Family::Hybrid => {
            let (a, b) = halves(&t.key);
            vec![a, b]
        }
        Some(t) => vec![t.key.as_str()],
        None => Vec::new(),
    };
    let tick = tgt
        .filter(|_| tick_known)
        .map(|t| alchemy::ladder(t.family)[usize::from(t.grade.0)]);
    let ring = alchemy::ladder(ring_family(make))[3] as f32;
    let scale = props
        .order
        .iter()
        .map(|p| get(vals, p) + bonus_of(p))
        .chain([ring, tick.unwrap_or(0) as f32, get(vals, toxic_key)])
        .fold(0.0, f32::max);
    let unknown = |p: &str| {
        if truth.is_none() && maybe.contains(p) {
            if get(vals, p) > 0.0 {
                Unknown::Some
            } else {
                Unknown::All
            }
        } else {
            Unknown::None
        }
    };
    Vials {
        rows: props
            .order
            .iter()
            .map(|p| Vial {
                prop: p.clone(),
                value: get(vals, p),
                unknown: unknown(p),
                main: mains.contains(&p.as_str()),
                target: tgt_props.contains(&p.as_str()),
                bonus: bonus_of(p),
                tick: tgt_props.contains(&p.as_str()).then_some(tick).flatten(),
            })
            .collect(),
        toxic: ToxicVial {
            raw: get(vals, toxic_key),
            purify: get(vals, purify_key),
            net,
            unknown: truth.is_none() && (maybe.contains(toxic_key) || maybe.contains(purify_key)),
            fail,
            bonus,
        },
        unnoted,
        scale,
        hybrid: hy.is_some(),
    }
}

/// [`hybrid_wins`] for the vials, whose target is a whole [`Target`].
fn target_wins(hy: &Hybrid, single: Option<(Grade, Step)>, target: Option<&Target>) -> bool {
    let targeted_single = target.is_some_and(|t| t.family == Family::Potion);
    target.is_some_and(|t| t.key == hy.pair)
        || single.is_none_or(|(g, _)| hy.grade > g || (hy.grade == g && !targeted_single))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PEACEBLOOM: u32 = 2447;
    const AMBER_SAP: u32 = 961025;
    const BRIARTHORN: u32 = 2450;
    const EARTHROOT: u32 = 2449;

    fn item(name: &str) -> u32 {
        alchemy::herbs()
            .into_iter()
            .map(|h| h.item)
            .find(|&i| crate::items::template(i).is_some_and(|t| t.name == name))
            .unwrap_or_else(|| panic!("no herb {name}"))
    }

    fn make(key: &str) -> &'static Make {
        alchemy::make(key).unwrap()
    }

    fn n(item: u32, count: usize) -> Vec<u32> {
        vec![item; count]
    }

    fn tincture(key: &str, grade: u8, step: u8) -> u32 {
        entry_at(Family::Tincture, key, Grade(grade), Step(step), 0).unwrap()
    }

    fn essence(key: &str, grade: u8, step: u8) -> u32 {
        entry_at(Family::Essence, key, Grade(grade), Step(step), 0).unwrap()
    }

    #[test]
    fn the_herbs_are_the_ones_named() {
        assert_eq!(item("Peacebloom"), PEACEBLOOM);
        assert_eq!(item("Briarthorn"), BRIARTHORN);
        assert_eq!(item("Earthroot"), EARTHROOT);
        assert_eq!(item("Amber Sap"), AMBER_SAP);
    }

    #[test]
    fn peacebloom_brews_minor_healing_potions() {
        let r = resolve(make("potion"), &n(PEACEBLOOM, 2), None);
        assert_eq!(r.product, Some(963100));
        assert_eq!(r.fail, 0.0);
        let r = resolve(make("potion"), &n(PEACEBLOOM, 4), None);
        assert_eq!(r.product, Some(963104), "8 / 4 = 2; Lesser needs 12");
    }

    #[test]
    fn peacebloom_and_amber_sap_brew_a_bloodfire_bomb_unless_a_potion_is_targeted() {
        let mut items = n(PEACEBLOOM, 2);
        items.extend(n(AMBER_SAP, 2));
        let r = resolve(make("potion"), &items, None);
        assert_eq!(r.family, Family::Hybrid);
        assert_eq!(
            r.product,
            Some(963322),
            "each side 5 on the hybrid ladder, x1.5"
        );
        assert_eq!(pct(r.fail), 20);
        let t = parse_target("potion|Restorative|Minor").unwrap();
        let r = resolve(make("potion"), &items, Some(&t.key));
        assert_eq!(r.product, Some(963102), "4 + 2 = 6, x1.5");
        assert_eq!(pct(r.fail), 33);
    }

    #[test]
    fn five_briarthorn_distill_a_plain_tincture_carrying_toxic_7() {
        let r = resolve(make("tincture"), &n(BRIARTHORN, 5), None);
        assert_eq!(pct(r.fail), 60);
        assert_eq!(r.grade, Some((Grade(2), Step(0))));
        assert_eq!(r.carried, 7);
        assert_eq!(r.product, Some(963747));
    }

    #[test]
    fn earthroot_purifies_a_briarthorn_tincture() {
        let mut items = n(BRIARTHORN, 3);
        items.extend(n(EARTHROOT, 2));
        let r = resolve(make("tincture"), &items, None);
        assert_eq!(r.net, 7.0);
        assert_eq!(pct(r.fail), 47);
        assert_eq!(r.carried, 3);
        assert_eq!(r.product, Some(963703));
    }

    #[test]
    fn two_tinctures_and_two_perch_mix_a_minor_leeching_oil() {
        let perch = crate::fish::fish_items()
            .find(|&f| crate::items::template(f).is_some_and(|t| t.name == "Goldshire Perch"))
            .unwrap();
        let mut items = n(tincture("Restorative", 0, 0), 2);
        items.extend(n(perch, 2));
        assert_eq!(valid(make("oil"), counts(&items)), Ok(()));
        let r = resolve(make("oil"), &items, None);
        assert_eq!(r.product, Some(963200));
        assert_eq!(r.fail, 0.0);
    }

    #[test]
    fn tinctures_of_two_properties_condense_into_sludge() {
        let mut items = n(tincture("Restorative", 0, 0), 2);
        items.push(tincture("Fortifying", 0, 0));
        let r = resolve(make("essence"), &items, None);
        assert!(r.is_sludge());
        assert_eq!(
            r.note,
            "Tinctures of different properties will not settle together."
        );
    }

    #[test]
    fn two_essences_join_a_flask_of_stone_and_ember() {
        let items = [essence("Fortifying", 1, 0), essence("Volatile", 0, 0)];
        let r = resolve(make("flask"), &items, None);
        assert_eq!(r.product, Some(968720));
        let items = [essence("Volatile", 0, 0), essence("Fortifying", 1, 0)];
        assert_eq!(resolve(make("flask"), &items, None).product, Some(968720));
    }

    #[test]
    fn the_fail_chance_and_the_main_property_break_their_edges() {
        assert_eq!(fail_chance(0.0, 5.0), 0.0);
        assert_eq!(fail_chance(3.0, 0.0), 1.0);
        let sums = Sums::from([
            ("Volatile".to_owned(), 4.0),
            ("Restorative".to_owned(), 4.0),
        ]);
        assert_eq!(main_prop(&sums, None), Some("Restorative"));
    }

    #[test]
    fn each_make_refuses_a_list_that_breaks_its_rule() {
        let c = |items: &[u32]| counts(items);
        assert_eq!(
            valid(make("potion"), c(&n(PEACEBLOOM, 1))),
            Err("Two to four herbs.")
        );
        assert_eq!(
            valid(make("tincture"), c(&n(PEACEBLOOM, 4))),
            Err("Five herbs.")
        );
        assert_eq!(valid(make("essence"), c(&n(PEACEBLOOM, 15))), Ok(()));
        let mut wash = n(tincture("Restorative", 0, 0), 3);
        wash.extend(n(EARTHROOT, 3));
        assert!(valid(make("essence"), c(&wash)).is_err());
        assert!(valid(make("flask"), c(&n(PEACEBLOOM, 2))).is_err());
    }

    #[test]
    fn the_vials_show_what_peacebloom_s_note_knows() {
        let items = [PEACEBLOOM];
        let restorative = |v: &Vials| {
            v.rows
                .iter()
                .find(|r| r.prop == "Restorative")
                .unwrap()
                .clone()
        };
        let v = vials(
            make("potion"),
            &items,
            None,
            &|_| NoteState::Unnoted,
            false,
            None,
        );
        assert_eq!(v.unnoted, 1);
        assert_eq!(restorative(&v).value, 0.0);
        assert_eq!(restorative(&v).unknown, Unknown::None);
        let v = vials(
            make("potion"),
            &items,
            None,
            &|_| NoteState::Named,
            false,
            None,
        );
        assert_eq!(restorative(&v).unknown, Unknown::All);
        let v = vials(
            make("potion"),
            &items,
            None,
            &|_| NoteState::Filled,
            false,
            None,
        );
        assert_eq!(restorative(&v).value, 2.0);
        assert!(restorative(&v).main);
    }
}
