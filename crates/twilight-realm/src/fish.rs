//! Fishing's content (`content/fish.toml`, ADR 0018): the cast, the Reeling Bar's tunables, Angler
//! levels, the fish, their pools by zone, the Fishing Waters, lures and bait. The file's item rows
//! are item templates, which `crate::items` loads.

// Lures, bait and the Legendary Fish (#329, #331) read the rest.
#![allow(dead_code)]

use std::sync::{OnceLock, RwLock};

use serde::Deserialize;

/// The cast (`[spell]`).
#[derive(Deserialize)]
pub(crate) struct Spell {
    /// The Fishing spell, whose effect summons the bobber.
    pub(crate) id: u32,
    /// The weapon subclass the main hand needs: a fishing pole.
    pub(crate) pole_subclass: u32,
    pub(crate) bobber_entry: u32,
    pub(crate) bobber_display: u32,
    /// `[min, max]` yards ahead of the caster the bobber lands.
    pub(crate) land_yd: [f32; 2],
    /// How long the bobber floats before it is lost unanswered.
    pub(crate) cast_secs: f32,
    /// `[min, max]` seconds from landing to the bite.
    pub(crate) bite_secs: [f32; 2],
}

/// The Reeling Bar's tunables (`[bar]`); fill and zone heights are fractions of the track.
#[derive(Deserialize)]
pub(crate) struct Bar {
    pub(crate) start_fill: f32,
    pub(crate) fill_per_sec: f32,
    pub(crate) drain_per_sec: f32,
    pub(crate) timeout_secs: f32,
    pub(crate) zone_base: f32,
    pub(crate) zone_per_level: f32,
    pub(crate) zone_max: f32,
    pub(crate) difficulty_per_level_above: f32,
    pub(crate) gate_levels_above: u32,
    pub(crate) treasure_base: f32,
    pub(crate) treasure_per_level: f32,
    pub(crate) treasure_fill_secs: f32,
    pub(crate) treasure_appears_secs: [f32; 2],
    /// A fish's roll weight is times this inside its peak hours.
    pub(crate) peak_factor: f32,
    /// A fish's roll weight is times this with its favourite bait on the cast.
    pub(crate) favourite_bait_factor: f32,
}

/// The Fishing Journal's tunables (`[journal]`).
#[derive(Deserialize)]
pub(crate) struct Journal {
    /// The catch counts that unlock a Fish Page's three tiers of facts.
    pub(crate) reveal_at: [u32; 3],
}

/// A Water Kind: what a fish bites in.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    River,
    Lake,
    Coast,
    Pond,
}

impl Kind {
    /// The lowercase word the row and the Fish Page write.
    pub(crate) fn name(&self) -> String {
        match self {
            Kind::River => "river",
            Kind::Lake => "lake",
            Kind::Coast => "coast",
            Kind::Pond => "pond",
        }
        .to_owned()
    }
}

/// Angler levels and XP (`[angler]`).
#[derive(Deserialize)]
pub(crate) struct Angler {
    pub(crate) cap: u32,
    pub(crate) xp_base: u32,
    pub(crate) xp_per_level: u32,
    pub(crate) catch_xp_base: u32,
    pub(crate) catch_xp_per_level: u32,
    pub(crate) rarity_factor: RarityFactor,
    pub(crate) perfect_factor: f32,
    pub(crate) legend_bonus_xp: u32,
    /// Title bands, rising by `from`.
    pub(crate) titles: Vec<Title>,
}

/// A title band: held from Angler level `from` until the next band.
#[derive(Deserialize)]
pub(crate) struct Title {
    pub(crate) from: u32,
    pub(crate) title: String,
}

impl Angler {
    /// The title of the last band whose `from` is at or below `level`.
    pub(crate) fn title(&self, level: u32) -> &str {
        self.titles
            .iter()
            .rev()
            .find(|t| t.from <= level)
            .map_or("", |t| t.title.as_str())
    }

    /// The XP from Angler level `level` to the next: base + per level x level.
    pub(crate) fn xp_to_next(&self, level: u32) -> u32 {
        self.xp_base + self.xp_per_level * level
    }
}

/// A catch's XP factor by its fish's rarity.
#[derive(Deserialize)]
pub(crate) struct RarityFactor {
    pub(crate) common: f32,
    pub(crate) uncommon: f32,
    pub(crate) rare: f32,
}

impl RarityFactor {
    pub(crate) fn of(&self, rarity: Rarity) -> f32 {
        match rarity {
            Rarity::Common => self.common,
            Rarity::Uncommon => self.uncommon,
            Rarity::Rare => self.rare,
        }
    }
}

/// How a fish moves on the Reeling Bar's track.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Style {
    Smooth,
    Mixed,
    Sinker,
    Floater,
    Dart,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Rarity {
    Common,
    Uncommon,
    Rare,
}

/// A block of the game day; a fish's peak and a Legendary Fish's criterion name one.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Block {
    Night,
    Dawn,
    Day,
    Dusk,
}

impl Block {
    /// Its `[from, to)` game hours; `night` wraps past midnight.
    pub(crate) fn hours(self) -> [u32; 2] {
        match self {
            Block::Night => [21, 3],
            Block::Dawn => [3, 9],
            Block::Day => [9, 15],
            Block::Dusk => [15, 21],
        }
    }

    /// Whether game minute `minute_of_day` falls in it.
    pub(crate) fn holds(self, minute_of_day: u32) -> bool {
        let [from, to] = self.hours();
        let hour = minute_of_day / 60;
        if from <= to {
            (from..to).contains(&hour)
        } else {
            hour >= from || hour < to
        }
    }
}

/// One fish (`[[fish]]`), by its item.
#[derive(Deserialize)]
pub(crate) struct Fish {
    pub(crate) item: u32,
    pub(crate) style: Style,
    /// 0 to 100.
    pub(crate) difficulty: u32,
    pub(crate) rarity: Rarity,
    /// The Water Kinds it bites in; `None` for every kind.
    #[serde(default)]
    pub(crate) kinds: Option<Vec<Kind>>,
    /// The block of the game day in which its weight is times `peak_factor`.
    #[serde(default)]
    pub(crate) peak: Option<Block>,
    /// The bait item whose presence on the cast multiplies its weight.
    #[serde(default)]
    pub(crate) favourite_bait: Option<u32>,
    /// The key of the Fishing Water the bobber must be in.
    #[serde(default)]
    pub(crate) water: Option<String>,
    /// The block of the game day in which it bites.
    #[serde(default)]
    pub(crate) block: Option<Block>,
    /// The bait item the cast must carry.
    #[serde(default)]
    pub(crate) bait: Option<u32>,
    /// The lure item that must be in the Lure slot.
    #[serde(default)]
    pub(crate) lure: Option<u32>,
}

impl Fish {
    /// Whether it has a water, block, bait or lure condition.
    pub(crate) fn has_conditions(&self) -> bool {
        self.water.is_some() || self.block.is_some() || self.bait.is_some() || self.lure.is_some()
    }

    /// Whether its conditions hold for a cast in `water` (a Fishing Water key) with `bait` chosen
    /// and `lure` slotted, at game minute `minute_of_day`.
    pub(crate) fn conditions_hold(
        &self,
        water: Option<&str>,
        bait: Option<u32>,
        lure: Option<u32>,
        minute_of_day: u32,
    ) -> bool {
        self.water.as_deref().is_none_or(|w| water == Some(w))
            && self.bait.is_none_or(|b| bait == Some(b))
            && self.lure.is_none_or(|l| lure == Some(l))
            && self
                .block
                .is_none_or(|b| block_holds(b, minute_of_day, bait))
    }

    /// Whether it bites in water of `kind`.
    pub(crate) fn bites_in(&self, kind: Kind) -> bool {
        self.kinds.as_ref().is_none_or(|k| k.contains(&kind))
    }

    /// Whether game minute `minute_of_day` is inside its peak block.
    pub(crate) fn in_peak(&self, minute_of_day: u32) -> bool {
        self.peak.is_some_and(|b| b.holds(minute_of_day))
    }

    /// Its weight in a roll, from its pool row's `base`: doubled (by the `[bar]` factors) inside
    /// its peak hours and again with its favourite bait on the cast.
    pub(crate) fn roll_weight(&self, base: f32, minute_of_day: u32, bait: Option<u32>) -> f32 {
        let bar = bar();
        let peak = if self
            .peak
            .is_some_and(|b| block_holds(b, minute_of_day, bait))
        {
            bar.peak_factor
        } else {
            1.0
        };
        let favourite = if self.favourite_bait.is_some() && self.favourite_bait == bait {
            bar.favourite_bait_factor
        } else {
            1.0
        };
        base * peak * favourite
    }
}

/// One zone's catch (`[[pool]]`). Its fish and treasure weights sum to 100.
#[derive(Deserialize)]
pub(crate) struct Pool {
    pub(crate) key: String,
    /// The zone's `AreaTable.dbc` id.
    pub(crate) zone: u32,
    /// The Water Kind of the zone's unnamed water.
    pub(crate) kind: Kind,
    /// The Journal's text for the zone.
    pub(crate) description: String,
    pub(crate) fish: Vec<PoolFish>,
    pub(crate) treasure: Vec<Treasure>,
    /// A treasure chest's `[min, max]` copper.
    pub(crate) copper: [u32; 2],
}

/// A fish in a pool: its weight and the level it is caught at.
#[derive(Deserialize)]
pub(crate) struct PoolFish {
    pub(crate) item: u32,
    pub(crate) weight: f32,
    pub(crate) level: u32,
}

/// A treasure roll's item and weight.
#[derive(Deserialize)]
pub(crate) struct Treasure {
    pub(crate) item: u32,
    pub(crate) weight: f32,
}

/// A Fishing Water (`[[water]]`): a named stretch of water, by area or by circle.
#[derive(Deserialize)]
pub(crate) struct Water {
    pub(crate) key: String,
    pub(crate) name: String,
    /// The `AreaTable.dbc` id it covers.
    #[serde(default)]
    pub(crate) area: Option<u32>,
    /// `[map, x, y, radius]`.
    #[serde(default)]
    pub(crate) circle: Option<[f32; 4]>,
    /// Its pool's key; `None` fishes its zone's pool.
    #[serde(default)]
    pub(crate) pool: Option<String>,
    /// The zone a circle with no `pool` and no `area` lies in, which its Fish Page names; `None`
    /// when `pool` or `area` already gives it.
    #[serde(default)]
    pub(crate) zone: Option<u32>,
    /// Its Water Kind; `None` takes its pool's.
    #[serde(default)]
    pub(crate) kind: Option<Kind>,
}

/// What a pole, a lure or a bait does to a cast; each key 0 when not set.
#[derive(Deserialize, Default)]
pub(crate) struct Effect {
    /// Counts as this many Angler levels more.
    #[serde(default)]
    pub(crate) angler_levels: u32,
    /// The bite comes this percent sooner (negative) or later.
    #[serde(default)]
    pub(crate) bite_wait_pct: i32,
    /// The catch zone is this percent taller.
    #[serde(default)]
    pub(crate) zone_pct: i32,
    /// The Reeling Bar starts this percent fuller (negative: emptier).
    #[serde(default)]
    pub(crate) start_fill_pct: i32,
    /// The Reeling Bar fills this percent faster (negative: slower).
    #[serde(default)]
    pub(crate) fill_pct: i32,
    /// The Reeling Bar drains this percent faster (negative: slower).
    #[serde(default)]
    pub(crate) drain_pct: i32,
    /// Percentage points more treasure chance.
    #[serde(default)]
    pub(crate) treasure_points: u32,
    /// Where the Reeling Bar refills to the first time it reaches zero; 0 for no second hook.
    #[serde(default)]
    pub(crate) second_hook: f32,
    /// This many more of the fish land with a catch.
    #[serde(default)]
    pub(crate) extra_fish: u32,
    /// Fish of these rarities drop out of the bite roll.
    #[serde(default)]
    pub(crate) ignored_by: Vec<Rarity>,
    /// This block of the game day counts as now, for peaks and conditions.
    #[serde(default)]
    pub(crate) always_block: Option<Block>,
}

/// A fishing pole (`[[pole]]`); it never wears.
#[derive(Deserialize)]
pub(crate) struct Pole {
    pub(crate) item: u32,
    #[serde(flatten)]
    pub(crate) effect: Effect,
}

/// A stock lure (`[[lure]]`); its item row carries its durability.
#[derive(Deserialize)]
pub(crate) struct Lure {
    pub(crate) item: u32,
    #[serde(flatten)]
    pub(crate) effect: Effect,
}

/// A custom bait (`[[bait]]`), used up one per cast.
#[derive(Deserialize)]
pub(crate) struct Bait {
    pub(crate) item: u32,
    #[serde(flatten)]
    pub(crate) effect: Effect,
}

/// A Legendary Fish (`[[legend]]`): it bites ahead of its pool's fish while every criterion holds.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Legend {
    /// Its trophy item.
    pub(crate) item: u32,
    pub(crate) name: String,
    /// The key of the pool it bites in.
    pub(crate) pool: String,
    /// The Angler Level it needs, counting a pole's.
    pub(crate) level: u32,
    pub(crate) style: Style,
    pub(crate) difficulty: u32,
    /// Percent per bite while the criteria hold.
    pub(crate) chance: f32,
    /// The key of the Fishing Water the bobber must be in.
    #[serde(default)]
    pub(crate) water: Option<String>,
    /// The block of the game day the cast must fall in.
    #[serde(default)]
    pub(crate) block: Option<Block>,
    /// The bait item the cast must carry.
    #[serde(default)]
    pub(crate) bait: Option<u32>,
}

impl Legend {
    /// Whether the criteria hold for a cast in `water` (a Fishing Water key) with `bait` chosen,
    /// at game minute `minute_of_day`.
    pub(crate) fn criteria_hold(
        &self,
        water: Option<&str>,
        bait: Option<u32>,
        minute_of_day: u32,
    ) -> bool {
        self.water.as_deref().is_none_or(|w| water == Some(w))
            && self.bait.is_none_or(|b| bait == Some(b))
            && self
                .block
                .is_none_or(|b| block_holds(b, minute_of_day, bait))
    }
}

/// Whether `block` holds at game minute `minute_of_day`, or the cast's `bait` makes it always hold.
pub(crate) fn block_holds(block: Block, minute_of_day: u32, bait: Option<u32>) -> bool {
    block.holds(minute_of_day)
        || bait
            .and_then(self::bait)
            .is_some_and(|b| b.effect.always_block == Some(block))
}

#[derive(Deserialize)]
struct File {
    spell: Spell,
    bar: Bar,
    angler: Angler,
    journal: Journal,
    fish: Vec<Fish>,
    pool: Vec<Pool>,
    water: Vec<Water>,
    legend: Vec<Legend>,
    pole: Vec<Pole>,
    lure: Vec<Lure>,
    bait: Vec<Bait>,
}

/// Checks a parsed `fish.toml`, panicking on the first row that breaks a rule.
fn check(file: &File) {
    for f in &file.fish {
        assert!(
            f.favourite_bait
                .is_none_or(|b| file.bait.iter().any(|x| x.item == b)),
            "fish {} names a favourite_bait that is no [[bait]]",
            f.item
        );
        assert!(
            f.water
                .as_deref()
                .is_none_or(|k| file.water.iter().any(|w| w.key == k)),
            "fish {} names a Fishing Water that does not exist",
            f.item
        );
        assert!(
            f.bait.is_none_or(|b| file.bait.iter().any(|x| x.item == b)),
            "fish {} names a bait that is no [[bait]]",
            f.item
        );
        assert!(
            f.lure.is_none_or(|l| file.lure.iter().any(|x| x.item == l)),
            "fish {} names a lure that is no [[lure]]",
            f.item
        );
    }
    let titles = &file.angler.titles;
    assert!(
        titles.first().is_some_and(|t| t.from == 1)
            && titles.windows(2).all(|w| w[0].from < w[1].from),
        "[angler] titles must start at level 1 and rise"
    );
    for p in &file.pool {
        assert!(
            !p.description.trim().is_empty(),
            "pool {} has no description",
            p.key
        );
    }
    for p in &file.pole {
        assert!(
            crate::items::template(p.item)
                .is_some_and(|t| t.class == 2 && t.subclass == file.spell.pole_subclass),
            "pole {} is not a fishing pole item",
            p.item
        );
    }
    for l in &file.lure {
        assert!(
            crate::items::template(l.item).is_some_and(|t| t.durability > 0),
            "lure {} has no durability",
            l.item
        );
    }
    // Every kind a pool or a Fishing Water gives its pool must leave a fish that bites at the
    // top Angler Level.
    let bites = |pool: &Pool, kind: Kind| {
        pool.fish.iter().any(|pf| {
            file.fish
                .iter()
                .find(|f| f.item == pf.item)
                .is_some_and(|f| !f.has_conditions() && f.bites_in(kind))
                && pf.level < file.angler.cap + file.bar.gate_levels_above
        })
    };
    for p in &file.pool {
        assert!(
            bites(p, p.kind),
            "pool {} has no fish that bites in its kind {:?}",
            p.key,
            p.kind
        );
    }
    for w in &file.water {
        let Some(kind) = w.kind else { continue };
        let pool = match &w.pool {
            Some(key) => file.pool.iter().find(|p| &p.key == key),
            None => w.area.and_then(|a| {
                let zone = crate::mobs::area_parent(a).copied().unwrap_or(a);
                file.pool.iter().find(|p| p.zone == zone)
            }),
        };
        if let Some(pool) = pool {
            assert!(
                bites(pool, kind),
                "water {} leaves pool {} with no fish that bites in its kind {:?}",
                w.key,
                pool.key,
                kind
            );
        }
    }
}

/// Parses and checks a `fish.toml`'s text.
fn load(text: &str) -> File {
    let file: File = toml::from_str(text).expect("content/fish.toml parses");
    check(&file);
    file
}

fn file() -> &'static File {
    static FILE: OnceLock<File> = OnceLock::new();
    FILE.get_or_init(|| {
        let file = load(include_str!("../content/fish.toml"));
        for l in &file.legend {
            assert!(
                file.pool.iter().any(|p| p.key == l.pool),
                "legend {} names pool {:?}, which does not exist",
                l.item,
                l.pool
            );
            assert!(
                l.water
                    .as_deref()
                    .is_none_or(|k| file.water.iter().any(|w| w.key == k)),
                "legend {} names a Fishing Water that does not exist",
                l.item
            );
            assert!(
                l.bait.is_none_or(|b| file.bait.iter().any(|x| x.item == b)),
                "legend {} names bait that is no [[bait]]",
                l.item
            );
        }
        file
    })
}

/// Fish, pools, Fishing Waters and Legendary Fish a test added (`Realm::with_fish`), in
/// `fish.toml`'s shape.
#[derive(Deserialize)]
struct Added {
    #[serde(default)]
    fish: Vec<Fish>,
    #[serde(default)]
    pool: Vec<Pool>,
    #[serde(default)]
    water: Vec<Water>,
    #[serde(default)]
    legend: Vec<Legend>,
}

/// Each added file's text, and what it parsed to.
fn added() -> &'static RwLock<Vec<(String, &'static Added)>> {
    static ADDED: OnceLock<RwLock<Vec<(String, &'static Added)>>> = OnceLock::new();
    ADDED.get_or_init(Default::default)
}

/// Every added file, oldest first.
fn extra() -> Vec<&'static Added> {
    added().read().unwrap().iter().map(|&(_, a)| a).collect()
}

/// Adds the `[[fish]]`, `[[pool]]`, `[[water]]` and `[[legend]]` rows in `toml` alongside the shipped ones,
/// for the rest of the process. Parallel tests add the same text; it is added once.
pub(crate) fn add(toml: &str) {
    let parsed: Added = toml::from_str(toml).expect("the added fishing content parses");
    let mut all = added().write().unwrap();
    if all.iter().all(|(text, _)| text != toml) {
        all.push((toml.to_owned(), Box::leak(Box::new(parsed))));
    }
}

pub(crate) fn spell() -> &'static Spell {
    &file().spell
}

pub(crate) fn bar() -> &'static Bar {
    &file().bar
}

pub(crate) fn angler() -> &'static Angler {
    &file().angler
}

/// The fish caught as `item`.
pub(crate) fn fish(item: u32) -> Option<&'static Fish> {
    let shipped = file().fish.iter();
    shipped
        .chain(extra().into_iter().flat_map(|a| &a.fish))
        .find(|f| f.item == item)
}

/// Every pool: the shipped ones, then any a test added.
pub(crate) fn pools() -> impl Iterator<Item = &'static Pool> {
    let shipped = file().pool.iter();
    shipped.chain(extra().into_iter().flat_map(|a| &a.pool))
}

/// The pool of the zone `zone` (`AreaTable.dbc` id).
pub(crate) fn pool_for_zone(zone: u32) -> Option<&'static Pool> {
    pools().find(|p| p.zone == zone)
}

/// The pool named `key`.
pub(crate) fn pool(key: &str) -> Option<&'static Pool> {
    pools().find(|p| p.key == key)
}

/// The pool fished at `(x, y)` on `map`, where the host names the area `area`: that of the
/// Fishing Water whose circle holds the point, else the one covering `area`, else the zone's
/// (the area's parent zone, as the Zone Factor resolves it).
pub(crate) fn pool_at(map: u32, x: f32, y: f32, area: Option<u32>) -> Option<&'static Pool> {
    match water_at(map, x, y, area).and_then(|w| w.pool.as_deref()) {
        Some(key) => pool(key),
        None => {
            let area = area?;
            pool_for_zone(crate::mobs::area_parent(area).copied().unwrap_or(area))
        }
    }
}

/// The Fishing Water at `(x, y)` on `map`, where the host names the area `area`: the one whose
/// circle holds the point, else the one covering `area`.
pub(crate) fn water_at(map: u32, x: f32, y: f32, area: Option<u32>) -> Option<&'static Water> {
    let all = waters();
    let in_circle = |w: &&Water| {
        w.circle
            .is_some_and(|[m, cx, cy, r]| m as u32 == map && (x - cx).hypot(y - cy) <= r)
    };
    all.iter().copied().find(in_circle).or_else(|| {
        all.iter()
            .copied()
            .find(|w| area.is_some() && w.area == area)
    })
}

/// Every Legendary Fish: the shipped ones, then any a test added.
pub(crate) fn legends() -> Vec<&'static Legend> {
    let shipped = file().legend.iter();
    shipped
        .chain(extra().into_iter().flat_map(|a| &a.legend))
        .collect()
}

/// The Legendary Fish whose trophy is `item`.
pub(crate) fn legend(item: u32) -> Option<&'static Legend> {
    legends().into_iter().find(|l| l.item == item)
}

/// The Fishing Water named `key`.
pub(crate) fn water(key: &str) -> Option<&'static Water> {
    file().water.iter().find(|w| w.key == key)
}

/// Every Fishing Water: the shipped ones, then any a test added.
pub(crate) fn waters() -> Vec<&'static Water> {
    let shipped = file().water.iter();
    shipped
        .chain(extra().into_iter().flat_map(|a| &a.water))
        .collect()
}

/// The Water Kind of a cast in `water` (a Fishing Water, if the bobber is in one) of `pool`: the
/// water's own kind, else the pool's.
pub(crate) fn kind_at(pool: &Pool, water: Option<&Water>) -> Kind {
    water.and_then(|w| w.kind).unwrap_or(pool.kind)
}

/// The Fishing Journal's tunables.
pub(crate) fn journal() -> &'static Journal {
    &file().journal
}

/// Every fish's item, in `[[fish]]` order: the shipped ones, then any a test added.
pub(crate) fn fish_items() -> impl Iterator<Item = u32> {
    let shipped = file().fish.iter();
    shipped
        .chain(extra().into_iter().flat_map(|a| &a.fish))
        .map(|f| f.item)
}

/// The pool a Fishing Water fishes: its `pool`, else the pool of its area's zone, else of its
/// `zone`.
fn water_pool(w: &Water) -> Option<&'static Pool> {
    match (&w.pool, w.area) {
        (Some(key), _) => pool(key),
        (None, Some(area)) => {
            pool_for_zone(crate::mobs::area_parent(area).copied().unwrap_or(area))
        }
        (None, None) => pool_for_zone(w.zone?),
    }
}

/// The zone of a Fishing Water: its pool's.
pub(crate) fn water_zone(w: &Water) -> Option<u32> {
    water_pool(w).map(|p| p.zone)
}

/// Where fish `item` lives, for its Fish Page: every pool holding it whose own kind it bites in,
/// as `(zone, "")`, then every Fishing Water whose pool holds it and whose kind it bites in, as
/// `(its zone, its name)`, each in file order.
pub(crate) fn lives_in(item: u32) -> Vec<(u32, String)> {
    let Some(def) = fish(item) else {
        return Vec::new();
    };
    let holds = |p: &Pool| p.fish.iter().any(|f| f.item == item);
    let pools = pools()
        .filter(|p| holds(p) && def.bites_in(p.kind))
        .map(|p| (p.zone, String::new()));
    let waters = waters().into_iter().filter_map(|w| {
        let pool = water_pool(w).filter(|p| holds(p))?;
        def.bites_in(kind_at(pool, Some(w)))
            .then(|| (pool.zone, w.name.clone()))
    });
    pools.chain(waters).collect()
}

/// The pole that is item `item`.
pub(crate) fn pole(item: u32) -> Option<&'static Pole> {
    file().pole.iter().find(|p| p.item == item)
}

/// The lure that is item `item`.
pub(crate) fn lure(item: u32) -> Option<&'static Lure> {
    file().lure.iter().find(|l| l.item == item)
}

/// The bait that is item `item`.
pub(crate) fn bait(item: u32) -> Option<&'static Bait> {
    file().bait.iter().find(|b| b.item == item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_pick_the_last_band_at_or_below_the_level() {
        assert_eq!(angler().title(9), "Novice Fisher");
        assert_eq!(angler().title(10), "Apprentice Fisher");
        assert_eq!(angler().title(60), "Master Fisher");
    }

    #[test]
    fn the_file_loads_and_its_accessors_answer() {
        assert_eq!(spell().id, 7620);
        assert_eq!(angler().xp_to_next(12), 940);
        assert_eq!(fish(960416).map(|f| f.style), Some(Style::Dart));
        assert_eq!(pool_for_zone(12).map(|p| p.key.as_str()), Some("elwynn"));
        assert!(water("cathedral_canal").is_some_and(|w| w.circle.is_some()));
        assert_eq!(
            water_at(0, -8796.0, -338.0, Some(9)).map(|w| w.key.as_str()),
            Some("northshire_river")
        );
        assert_eq!(lure(6530).map(|l| l.effect.drain_pct), Some(-30));
        assert_eq!(lure(6529).map(|l| l.effect.angler_levels), Some(0));
        assert_eq!(bait(960102).map(|b| b.effect.treasure_points), Some(10));
        assert_eq!(bait(960105).map(|b| b.effect.angler_levels), Some(0));
        assert_eq!(pole(6366).map(|p| p.effect.angler_levels), Some(3));
        assert_eq!(file().journal.reveal_at, [5, 10, 25]);
    }

    #[test]
    fn every_fishing_material_is_a_journal_item_and_nothing_else_is() {
        let f = file();
        let mut materials: Vec<u32> = f.fish.iter().map(|x| x.item).collect();
        materials.extend(f.pole.iter().map(|x| x.item));
        materials.extend(f.lure.iter().map(|x| x.item));
        materials.extend(f.bait.iter().map(|x| x.item));
        materials.extend(legends().iter().map(|l| l.item));
        for item in &materials {
            let t = crate::items::template(*item).expect("a template");
            assert_eq!(t.journal, Some(crate::items::Profession::Fishing), "{item}");
        }
        for (entry, t) in crate::items::all() {
            assert!(
                t.journal.is_none() || materials.contains(entry),
                "{entry} sets journal but is no fishing material"
            );
        }
    }

    #[test]
    #[should_panic(expected = "favourite_bait that is no [[bait]]")]
    fn a_favourite_bait_that_is_no_bait_fails_to_load() {
        let text = include_str!("../content/fish.toml").replacen(
            "favourite_bait = 960101",
            "favourite_bait = 999999",
            1,
        );
        load(&text);
    }

    #[test]
    #[should_panic(expected = "names a Fishing Water that does not exist")]
    fn a_fish_whose_water_is_no_water_fails_to_load() {
        let text = include_str!("../content/fish.toml").replacen(
            "favourite_bait = 960101",
            "favourite_bait = 960101\nwater = \"no_such_water\"",
            1,
        );
        load(&text);
    }

    #[test]
    #[should_panic(expected = "has no fish that bites")]
    fn a_pool_whose_only_fish_is_gated_fails_to_load() {
        // Elwynn keeps only the smallfish, which then bites at dusk alone.
        let text = include_str!("../content/fish.toml");
        let head = "description = \"Still lakes and the Northshire river, full of perch and snapper. Half of Stormwind learned to fish here.\"\nfish = [\n";
        let from = text.find(head).unwrap() + head.len();
        let to = from + text[from..].find("]\n").unwrap();
        let text = format!(
            "{}  {{ item = 6291, weight = 99.0, level = 1 }},\n{}",
            &text[..from],
            &text[to..]
        )
        .replacen("peak = \"day\"", "peak = \"day\"\nblock = \"dusk\"", 1);
        load(&text);
    }

    #[test]
    fn a_fish_whose_conditions_do_not_all_hold_stays_out() {
        let f: Fish = toml::from_str(
            "item = 1\nstyle = \"smooth\"\ndifficulty = 1\nrarity = \"common\"\nblock = \"dusk\"\nlure = 6532\n",
        )
        .unwrap();
        assert!(f.has_conditions());
        assert!(f.conditions_hold(None, None, Some(6532), 16 * 60));
        assert!(!f.conditions_hold(None, None, Some(6532), 12 * 60));
        assert!(!f.conditions_hold(None, None, None, 16 * 60));
    }

    #[test]
    fn a_fish_without_kinds_bites_everywhere() {
        let smallfish = fish(6291).unwrap();
        for kind in [Kind::River, Kind::Lake, Kind::Coast, Kind::Pond] {
            assert!(smallfish.bites_in(kind));
        }
        assert!(!fish(6303).unwrap().bites_in(Kind::River));
    }

    #[test]
    fn the_night_block_wraps_past_midnight() {
        let catfish = fish(6308).unwrap();
        assert!(catfish.in_peak(22 * 60));
        assert!(catfish.in_peak(2 * 60));
        assert!(!catfish.in_peak(3 * 60));
        assert!(!catfish.in_peak(20 * 60 + 59));
    }

    #[test]
    #[should_panic]
    fn a_legend_with_minutes_fails_to_load() {
        let text = include_str!("../content/fish.toml").replacen(
            "water = \"twilight_grove\"\nblock = \"dusk\"",
            "water = \"twilight_grove\"\nminutes = [0, 10]",
            1,
        );
        load(&text);
    }

    #[test]
    fn the_roll_weight_doubles_for_peak_hours_and_favourite_bait() {
        let snapper = fish(6289).unwrap();
        let worms = Some(960101);
        assert_eq!(snapper.roll_weight(37.85, 7 * 60, worms), 37.85 * 4.0);
        assert_eq!(snapper.roll_weight(37.85, 7 * 60, None), 37.85 * 2.0);
        assert_eq!(snapper.roll_weight(37.85, 12 * 60, worms), 37.85 * 2.0);
        assert_eq!(snapper.roll_weight(37.85, 12 * 60, None), 37.85);
    }

    #[test]
    fn corpse_grub_puts_the_night_peak_and_condition_at_any_hour() {
        let catfish = fish(6308).unwrap();
        let grub = Some(960106);
        assert_eq!(catfish.roll_weight(10.0, 12 * 60, grub), 10.0 * 4.0);
        assert_eq!(catfish.roll_weight(10.0, 12 * 60, Some(960105)), 10.0);
        assert!(block_holds(Block::Night, 12 * 60, grub));
        assert!(!block_holds(Block::Dusk, 12 * 60, grub));
        assert!(!block_holds(Block::Night, 12 * 60, None));
    }

    #[test]
    fn a_water_overrides_its_pools_kind() {
        let stormwind = pool("stormwind").unwrap();
        assert_eq!(kind_at(stormwind, None), Kind::Coast);
        assert_eq!(kind_at(stormwind, water("cathedral_canal")), Kind::River);
    }
}
