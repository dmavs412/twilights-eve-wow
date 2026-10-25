//! Fishing (ADR 0018): the cast, the bobber, the bite, the Reeling Bar's result and Angler XP.
//! The cast is checked at once for what needs no host. The water and the pool wait for the
//! realm's next tick, which has the host. The bobber is a run-time GameObject the realm keeps
//! (`crate::objects`); this session names the ones whose cast ended in `spent_bobbers`.
//!
//! The pole and the lure sit in the Journal's gear slots (#371), filled from the Satchel, and so is
//! the bait (ADR 0023). The lure loses a point of durability at each bite; the chosen bait is
//! taken one per cast when the bobber lands. What both add is worked out once, at the landing
//! ([`Boost`]), and the cast keeps it to the bite. The Fishing Journal is each fish's best catch and the bait.

use benilla_protocol::messages::GossipOption;
use benilla_protocol::{FishPage, SatchelPick, SessionEvent};

use super::cast::{refused, SPELL_FAILED_EQUIPPED_ITEM_CLASS, SPELL_FAILED_NOT_KNOWN};
use super::InWorld;
use crate::audience::Audience;
use crate::characters::{Char, FishLog};
use crate::equipment;
use crate::fields::{guid_pairs, values, UNIT_CHANNEL_SPELL, UNIT_FIELD_CHANNEL_OBJECT};
use crate::fish::{self, Kind, Pool, PoolFish, Style};
use crate::hub::Npc;
use crate::inventory::Pos;
use crate::items;
use crate::objects::{ObjectKind, ObjectSpec};
use crate::rng::Rng;
use crate::rules::game_clock;
use crate::rules::gear::SLOT_MAIN_HAND;
use crate::rules::xp::gray_level;
use crate::rumours;
use crate::FishingGearSlot;
use crate::{clock, Host};

/// `SPELL_FAILED_NOT_FISHABLE`: "Your cast didn't land in fishable water".
const SPELL_FAILED_NOT_FISHABLE: u8 = 0x34;
/// The red line when nothing in the water bites at the character's Angler Level.
const NOTHING_BITING: &str = "Nothing is biting";
/// Seconds past the bar's timeout a result is still believed, and the bar kept open.
const RESULT_SLACK_SECS: f32 = 2.0;
/// The fastest win believed, as a share of the time the meter takes to fill held all the way.
const FASTEST_WIN: f32 = 0.95;

/// Lee Brown's menu lines: the merchant window and the lesson.
const OPTION_BROWSE: u32 = 1;
const OPTION_TEACH: u32 = 2;
/// `GOSSIP_ICON_VENDOR` and `GOSSIP_ICON_TRAINER`.
const ICON_VENDOR: u8 = 1;
const ICON_TRAINER: u8 = 3;
/// `GOSSIP_ICON_CHAT`.
const ICON_CHAT: u8 = 0;

/// The Journal's best catch of a fish.
const CAUGHT: u8 = 1;
const PERFECT: u8 = 2;

/// What a cast's lure and bait add up to (#329).
#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct Boost {
    /// Counted as this many Angler Levels more, for the zone and the bite gate only.
    angler_levels: u32,
    /// The bite comes this percent sooner (negative) or later.
    bite_wait_pct: i32,
    /// The catch zone is this percent taller, after the cap.
    zone_pct: i32,
    /// Percentage points more treasure chance.
    treasure_points: u32,
    /// The bait on this cast, which a Legendary Fish may ask for.
    bait: Option<u32>,
}

impl Boost {
    fn with(mut self, e: &fish::Effect) -> Self {
        self.angler_levels += e.angler_levels;
        self.bite_wait_pct += e.bite_wait_pct;
        self.zone_pct += e.zone_pct;
        self.treasure_points += e.treasure_points;
        self
    }
}

/// A Fishing cast under way, from the cast to the end of the Reeling Bar.
pub(super) struct Fishing {
    /// The bobber's guid; 0 until it lands.
    bobber: u64,
    stage: Stage,
    /// The main hand's entry to show again when the cast ends: the pole shows while it lasts.
    restore: u32,
}

enum Stage {
    /// Cast: the next tick, which has the host, lands it or refuses it.
    Cast,
    /// The bobber floats: the bite comes in `bite_in` seconds, and the cast is lost in `left`.
    Floating {
        pool: &'static Pool,
        boost: Boost,
        /// The key of the Fishing Water the bobber is in.
        water: Option<&'static str>,
        /// The Water Kind of the bobber's water.
        kind: Kind,
        bite_in: f32,
        left: f32,
    },
    /// A fish bit and the Reeling Bar is open.
    Reeling(Bite),
}

/// What the realm rolled when the fish bit.
struct Bite {
    fish: u32,
    level: u32,
    /// The fish is a Legendary Fish.
    legend: bool,
    /// The pool it bit in, whose Legend Rumours a catch rolls.
    pool: &'static Pool,
    /// The catch log's place key: the cast's Fishing Water, else the pool.
    place: &'static str,
    start_fill: f32,
    fill_per_sec: f32,
    treasure: Option<Treasure>,
    /// Seconds until the bar is lost unanswered.
    left: f32,
}

/// A treasure chest's contents, rolled at the bite.
struct Treasure {
    item: Option<u32>,
    copper: u32,
}

/// `Angler`: the character's Angler Level and XP, for login and every change.
pub(super) fn angler(c: &Char) -> SessionEvent {
    let a = fish::angler();
    let level = u32::from(c.angler_level);
    SessionEvent::Angler {
        level: c.angler_level,
        xp: c.angler_xp,
        xp_to_next: if level < a.cap {
            a.xp_to_next(level)
        } else {
            0
        },
    }
}

/// The Angler Level a cast counts as: the character's, and what its lure and bait add. The zone
/// height and the bite gate read it; XP and the grey rule read the character's own.
fn effective_level(c: &Char, boost: Boost) -> u32 {
    u32::from(c.angler_level) + boost.angler_levels
}

/// `FishingJournal`: the bait, and every pool's fish with the character's best catch of each.
pub(crate) fn journal(c: &Char) -> SessionEvent {
    let best = |item: u32| {
        c.fish_log
            .iter()
            .find(|l| l.item == item)
            .map_or(0, |l| l.best)
    };
    let pages = fish::fish_items()
        .filter_map(|item| c.fish_log.iter().find(|l| l.item == item && l.caught > 0))
        .map(fish_page)
        .collect();
    SessionEvent::FishingJournal {
        bait: c.bait,
        pole: c.fishing_pole.as_ref().map(|p| p.entry),
        lure: c.fishing_lure.as_ref().map(|l| {
            let max = items::template(l.entry).map_or(0, |t| t.durability);
            (l.entry, l.durability, max)
        }),
        zones: fish::pools()
            .map(|p| {
                (
                    p.zone,
                    p.fish.iter().map(|f| (f.item, best(f.item))).collect(),
                )
            })
            .collect(),
        legends: fish::legends()
            .into_iter()
            .map(|l| {
                (
                    l.item,
                    fish::pool(&l.pool).map_or(0, |p| p.zone),
                    c.legend_hints.contains(&l.item),
                    c.legends_caught.contains(&l.item),
                )
            })
            .collect(),
        pages,
        lure_waiting: c.lure_waiting,
        satchel: c.inventory.satchel.rows(items::Profession::Fishing),
    }
}

/// A Legend Rumour's hit: the note goes to the bags and the hint is recorded, and the Journal
/// follows. Full bags leave the hint unfound, so the roll can come again.
pub(crate) fn find_hint(c: &mut Char, legend: u32, note: u32, events: &mut Vec<SessionEvent>) {
    if let Err(reason) = equipment::receive(c, note, 1, events) {
        events.push(equipment::inventory_failure(reason, 0));
        return;
    }
    c.legend_hints.push(legend);
    events.push(journal(c));
}

/// Records a catch of `item` at `place` (a Fishing Water's key, else the pool's) in the catch
/// log, a perfect one beating a plain one for the best.
fn record_catch(c: &mut Char, item: u32, perfect: bool, place: &str) {
    let quality = if perfect { PERFECT } else { CAUGHT };
    let at = match c.fish_log.iter().position(|l| l.item == item) {
        Some(at) => at,
        None => {
            c.fish_log.push(FishLog {
                item,
                ..FishLog::default()
            });
            c.fish_log.len() - 1
        }
    };
    let log = &mut c.fish_log[at];
    log.best = log.best.max(quality);
    log.caught += 1;
    log.perfect += u32::from(perfect);
    match log.places.iter_mut().find(|(k, _)| k == place) {
        Some((_, n)) => *n += 1,
        None => log.places.push((place.to_owned(), 1)),
    }
}

/// A fish's Fish Page: its counts, and the facts its catch count has unlocked.
fn fish_page(log: &FishLog) -> FishPage {
    let reveal_at = fish::journal().reveal_at;
    let revealed = reveal_at.iter().filter(|&&n| log.caught >= n).count();
    let def = fish::fish(log.item);
    let mut places: Vec<(u32, String, u32)> = log
        .places
        .iter()
        .filter_map(|(key, n)| {
            let (zone, name) = match fish::water(key) {
                Some(w) => (fish::water_zone(w)?, w.name.clone()),
                None => (fish::pool(key)?.zone, String::new()),
            };
            Some((zone, name, *n))
        })
        .collect();
    places.sort_by_key(|&(_, _, n)| std::cmp::Reverse(n));
    let mut page = FishPage {
        item: log.item,
        caught: log.caught,
        perfect: log.perfect,
        places,
        revealed: revealed as u8,
        next_reveal: reveal_at.get(revealed).copied().unwrap_or(0),
        lives_in: Vec::new(),
        kinds: Vec::new(),
        peak_hours: None,
        favourite_bait: None,
    };
    let Some(def) = def else { return page };
    if revealed >= 1 {
        page.lives_in = fish::lives_in(log.item);
    }
    if revealed >= 2 {
        page.kinds = def.kinds.iter().flatten().map(Kind::name).collect();
        page.peak_hours = def.peak.map(|b| {
            let [from, to] = b.hours();
            (from as u8, to as u8)
        });
    }
    if revealed >= 3 {
        page.favourite_bait = def.favourite_bait;
    }
    page
}

/// The chosen bait's row while the character carries one.
fn carried_bait(c: &Char) -> Option<&'static fish::Bait> {
    c.bait
        .filter(|&b| c.inventory.count(b) > 0)
        .and_then(fish::bait)
}

/// Whether the Pole slot holds a fishing pole.
fn holds_pole(c: &Char) -> bool {
    c.fishing_pole.is_some()
}

/// What the Pole slot's pole and the Lure slot's lure add to a cast.
fn gear_boost(c: &Char) -> Boost {
    let pole = c
        .fishing_pole
        .as_ref()
        .and_then(|p| fish::pole(p.entry))
        .map_or(Boost::default(), |p| Boost::default().with(&p.effect));
    c.fishing_lure
        .as_ref()
        .and_then(|l| fish::lure(l.entry))
        .map_or(pole, |l| pole.with(&l.effect))
}

/// The main hand's entry, as other players see it.
fn main_hand_entry(c: &Char) -> u32 {
    c.inventory
        .get(Pos::own(SLOT_MAIN_HAND))
        .map_or(0, |i| i.entry)
}

/// The fields that show `entry` in the main hand.
fn show_hand(guid: u64, entry: u32) -> SessionEvent {
    values(guid, &[(equipment::visible_item(SLOT_MAIN_HAND), entry)])
}

/// The fish of `pool` that bite at Angler Level `effective`: those fewer than
/// `gate_levels_above` levels above it, and whose kinds include the water's `kind`.
fn biters(pool: &'static Pool, effective: u32, kind: Kind) -> Vec<&'static PoolFish> {
    let gate = fish::bar().gate_levels_above;
    pool.fish
        .iter()
        .filter(|f| {
            f.level < effective + gate && fish::fish(f.item).is_some_and(|d| d.bites_in(kind))
        })
        .collect()
}

/// A pool fish's weight in the bite roll at game minute `minute`, with `bait` on the cast.
fn roll_weight(f: &PoolFish, minute: u32, bait: Option<u32>) -> f32 {
    fish::fish(f.item).map_or(f.weight, |d| d.roll_weight(f.weight, minute, bait))
}

/// One of `rows` by `weight`.
fn pick<'a, T>(rows: &'a [T], weight: impl Fn(&T) -> f32, rng: &mut Rng) -> Option<&'a T> {
    let total: f32 = rows.iter().map(&weight).sum();
    if total <= 0.0 {
        return None;
    }
    let mut roll = rng.frand(0.0, total);
    for r in rows {
        roll -= weight(r);
        if roll < 0.0 {
            return Some(r);
        }
    }
    rows.last()
}

/// The wire's style byte: 0 mixed, 1 dart, 2 smooth, 3 sinker, 4 floater.
fn style_byte(style: Style) -> u8 {
    match style {
        Style::Mixed => 0,
        Style::Dart => 1,
        Style::Smooth => 2,
        Style::Sinker => 3,
        Style::Floater => 4,
    }
}

/// A catch's Angler XP: base + per level x the fish's level, times its rarity's factor, times
/// the perfect factor for a perfect catch. A fish gray to `angler_level` is worth nothing.
fn catch_xp(item: u32, level: u32, perfect: bool, angler_level: u8) -> u32 {
    if level <= u32::from(gray_level(angler_level)) {
        return 0;
    }
    let a = fish::angler();
    let rarity = match fish::fish(item) {
        Some(f) => a.rarity_factor.of(f.rarity),
        None if fish::legend(item).is_some() => a.rarity_factor.rare,
        None => 1.0,
    };
    let perfect = if perfect { a.perfect_factor } else { 1.0 };
    ((a.catch_xp_base + a.catch_xp_per_level * level) as f32 * rarity * perfect).round() as u32
}

/// Adds `xp` Angler XP, levelling up along the curve to the cap.
fn gain_angler_xp(c: &mut Char, xp: u32) {
    let a = fish::angler();
    c.angler_xp += xp;
    while u32::from(c.angler_level) < a.cap
        && c.angler_xp >= a.xp_to_next(u32::from(c.angler_level))
    {
        c.angler_xp -= a.xp_to_next(u32::from(c.angler_level));
        c.angler_level += 1;
    }
    if u32::from(c.angler_level) >= a.cap {
        c.angler_xp = 0;
    }
}

/// The gossip line of `npc`'s angler hints, at this index plus the hint's place.
const OPTION_HINT: u32 = 80;

/// The line "Heard anything about the fish around here?" for each of `npc`'s hints that `c`'s
/// Angler Level has reached.
pub(crate) fn hint_options(npc: &Npc, c: &Char) -> Vec<GossipOption> {
    npc.angler_hints
        .iter()
        .enumerate()
        .filter(|(_, h)| u32::from(c.angler_level) >= h.min_angler_level)
        .map(|(i, _)| GossipOption {
            index: OPTION_HINT + i as u32,
            icon: ICON_CHAT,
            coded: false,
            message: "Heard anything about the fish around here?".into(),
        })
        .collect()
}

/// Whether `option` is one of an angler hint's lines.
pub(crate) fn is_hint_option(option: u32) -> bool {
    (OPTION_HINT..OPTION_HINT + 10).contains(&option)
}

/// Lee Brown's gossip lines for `c`: his goods, and the lesson while they do not know Fishing.
pub(crate) fn angler_options(c: &Char) -> Vec<GossipOption> {
    let mut options = vec![GossipOption {
        index: OPTION_BROWSE,
        icon: ICON_VENDOR,
        coded: false,
        message: "Let me browse your goods.".into(),
    }];
    if !c.knows_fishing {
        options.push(GossipOption {
            index: OPTION_TEACH,
            icon: ICON_TRAINER,
            coded: false,
            message: "Teach me to fish.".into(),
        });
    }
    options
}

impl InWorld {
    /// `CMSG_CAST_SPELL` of Fishing: refused with the stock reason without the skill or a pole,
    /// else cast, for the next tick to land.
    pub(super) fn cast_fishing(&mut self, c: &Char, events: &mut Vec<SessionEvent>) {
        let id = fish::spell().id;
        if !c.knows_fishing {
            events.push(refused(id, SPELL_FAILED_NOT_KNOWN));
            return;
        }
        if !holds_pole(c) {
            events.push(refused(id, SPELL_FAILED_EQUIPPED_ITEM_CLASS));
            return;
        }
        // A new cast ends a channel under way.
        self.end_channel(true, events);
        let restore = main_hand_entry(c);
        if let Some(pole) = &c.fishing_pole {
            events.push(show_hand(self.guid, pole.entry));
        }
        self.fishing = Some(Fishing {
            bobber: 0,
            stage: Stage::Cast,
            restore,
        });
    }

    /// The cast or the bar under way is lost: the client is told the fish got away, the line
    /// comes in, and the bobber goes.
    pub(crate) fn lose_fish(&mut self, events: &mut Vec<SessionEvent>) {
        let Some(f) = self.end_cast(events) else {
            return;
        };
        events.push(SessionEvent::FishingEnd {
            caught: None,
            perfect: false,
            treasure: false,
        });
        if f.bobber != 0 {
            self.reel_in(f.bobber, true, events);
        }
    }

    /// The cast under way is over: the main hand shows what it holds again.
    fn end_cast(&mut self, events: &mut Vec<SessionEvent>) -> Option<Fishing> {
        let f = self.fishing.take()?;
        events.push(show_hand(self.guid, f.restore));
        Some(f)
    }

    /// The channel to `bobber` ends, `broken` when it stopped short, and the bobber is spent.
    fn reel_in(&mut self, bobber: u64, broken: bool, events: &mut Vec<SessionEvent>) {
        self.channel_closed(fish::spell().id, broken, events);
        self.spent_bobbers.push(bobber);
    }

    /// The bobbers of the casts that ended since the last call, for the realm to remove.
    pub(crate) fn take_spent_bobbers(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.spent_bobbers)
    }

    /// The cast's clock, with the host: a fresh cast lands or is refused, a floating bobber
    /// bites, and an unanswered bobber or bar is lost. `Some` with the bobber to make when the
    /// cast lands.
    pub(crate) fn fishing_tick(
        &mut self,
        dt: f32,
        wall: f64,
        host: &dyn Host,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) -> Option<ObjectSpec> {
        let f = self.fishing.as_mut()?;
        match &mut f.stage {
            Stage::Cast => return self.land(host, c, rng, events),
            Stage::Floating {
                pool,
                boost,
                water,
                kind,
                bite_in,
                left,
            } => {
                *bite_in -= dt;
                *left -= dt;
                if *bite_in <= 0.0 {
                    let (pool, boost, water, kind) = (*pool, *boost, *water, *kind);
                    self.bite(pool, boost, water, kind, wall, c, rng, events);
                } else if *left <= 0.0 {
                    self.lose_fish(events);
                }
            }
            Stage::Reeling(bite) => {
                bite.left -= dt;
                if bite.left <= 0.0 {
                    self.lose_fish(events);
                }
            }
        }
        None
    }

    /// The cast lands a random distance ahead: on water whose pool has a fish that bites, the
    /// bobber floats and one of the chosen bait goes; elsewhere the cast is refused and takes none.
    fn land(
        &mut self,
        host: &dyn Host,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) -> Option<ObjectSpec> {
        let s = fish::spell();
        let d = rng.frand(s.land_yd[0], s.land_yd[1]);
        let [x, y, _] = c.position;
        let (x, y) = (x + d * c.orientation.cos(), y + d * c.orientation.sin());
        let surface = host
            .liquid_at(c.map, x, y)
            .filter(|&z| host.ground_height(c.map, x, y, z).is_none_or(|g| z >= g));
        let Some(z) = surface else {
            self.end_cast(events);
            events.push(refused(s.id, SPELL_FAILED_NOT_FISHABLE));
            return None;
        };
        let bait = carried_bait(c);
        let mut boost = bait.map_or(gear_boost(c), |b| gear_boost(c).with(&b.effect));
        boost.bait = bait.map(|b| b.item);
        let effective = effective_level(c, boost);
        let area = host.area_at(c.map, x, y);
        let fishing_water = fish::water_at(c.map, x, y, area);
        let water = fishing_water.map(|w| w.key.as_str());
        let pool = fish::pool_at(c.map, x, y, area).map(|p| (p, fish::kind_at(p, fishing_water)));
        let pool = pool.filter(|&(p, kind)| !biters(p, effective, kind).is_empty());
        let Some((pool, kind)) = pool else {
            self.end_cast(events);
            events.push(SessionEvent::Notification {
                text: NOTHING_BITING.into(),
            });
            return None;
        };
        // A bait chosen but none held casts bare, and the choice stays.
        if let Some(b) = bait {
            take_bait(c, b.item, events);
        }
        let wait = rng.frand(s.bite_secs[0], s.bite_secs[1]);
        let f = self.fishing.as_mut()?;
        f.stage = Stage::Floating {
            pool,
            boost,
            water,
            kind,
            bite_in: wait * (1.0 + boost.bite_wait_pct as f32 / 100.0).max(0.0),
            left: s.cast_secs,
        };
        Some(ObjectSpec {
            entry: s.bobber_entry,
            display: s.bobber_display,
            kind: ObjectKind::Bobber,
            map: c.map,
            position: [x, y, z],
            orientation: c.orientation,
        })
    }

    /// The realm made the bobber `bobber` at `at`: the cast goes off and the caster channels to
    /// it, so every client that sees them draws the line.
    pub(crate) fn bobber_landed(
        &mut self,
        bobber: u64,
        at: [f32; 3],
        events: &mut Vec<SessionEvent>,
    ) {
        let Some(f) = &mut self.fishing else {
            return;
        };
        f.bobber = bobber;
        let s = fish::spell();
        events.push(SessionEvent::SpellStart {
            caster: self.guid,
            spell_id: s.id,
            cast_flags: 0,
            cast_time_ms: 0,
            target: None,
            ammo_display_id: None,
        });
        events.push(SessionEvent::CastResult {
            spell_id: s.id,
            success: true,
            reason: None,
            arg: None,
        });
        events.push(SessionEvent::SpellGo {
            caster: self.guid,
            spell_id: s.id,
            cast_flags: 0,
            hits: Vec::new(),
            misses: Vec::new(),
            target: None,
            go_target: None,
            dest: Some(at),
            ammo_display_id: None,
            item_caster: None,
        });
        events.push(SessionEvent::ChannelStart {
            spell_id: s.id,
            duration_ms: clock::ms(s.cast_secs),
        });
        let mut pairs = vec![(UNIT_CHANNEL_SPELL, s.id)];
        pairs.extend(guid_pairs(UNIT_FIELD_CHANNEL_OBJECT, bobber));
        events.push(values(self.guid, &pairs));
    }

    /// A fish of `pool` bites: the bobber splashes, and the realm rolls the fish, the bar and any
    /// treasure, and opens the bar.
    #[allow(clippy::too_many_arguments)]
    fn bite(
        &mut self,
        pool: &'static Pool,
        boost: Boost,
        water: Option<&'static str>,
        kind: Kind,
        wall: f64,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) {
        let effective = effective_level(c, boost);
        // A Legendary Fish of the pool whose criteria hold bites before the pool is rolled.
        let minute = game_clock::minute_of_day(wall);
        let legend = fish::legends().into_iter().find(|l| {
            l.pool == pool.key
                && !c.legends_caught.contains(&l.item)
                && l.level <= effective
                && l.criteria_hold(water, boost.bait, minute)
                && rng.frand(0.0, 100.0) < l.chance
        });
        let rolled = match legend {
            Some(l) => Some((l.item, l.level, l.style, l.difficulty, true)),
            None => pick(
                &biters(pool, effective, kind),
                |f| roll_weight(f, minute, boost.bait),
                rng,
            )
            .and_then(|f| {
                let def = fish::fish(f.item)?;
                Some((f.item, f.level, def.style, def.difficulty, false))
            }),
        };
        let (Some((item, level, style, base, legendary)), Some(f)) =
            (rolled, self.fishing.as_mut())
        else {
            return self.lose_fish(events);
        };
        let bar = fish::bar();
        let above = level.saturating_sub(effective) as f32;
        let difficulty = (base as f32 + bar.difficulty_per_level_above * above).min(100.0);
        let zone_height = (bar.zone_base + bar.zone_per_level * (effective - 1) as f32)
            .min(bar.zone_max)
            * (1.0 + boost.zone_pct as f32 / 100.0);
        let chance = bar.treasure_base
            + bar.treasure_per_level * f32::from(c.angler_level)
            + boost.treasure_points as f32 / 100.0;
        let treasure = (rng.frand(0.0, 1.0) < chance).then(|| {
            let [lo, hi] = bar.treasure_appears_secs;
            let appears_at = rng.frand(lo, hi);
            let copper = rng.urand(pool.copper[0], pool.copper[1]);
            let item = pick(&pool.treasure, |t| t.weight, rng).map(|t| t.item);
            (appears_at, Treasure { item, copper })
        });
        events.push(SessionEvent::GameObjectCustomAnim {
            guid: f.bobber,
            anim_id: 0,
        });
        events.push(SessionEvent::FishingBite {
            bobber: f.bobber,
            fish: item,
            level: level.min(u32::from(u8::MAX)) as u8,
            style: style_byte(style),
            difficulty,
            zone_height,
            start_fill: bar.start_fill,
            fill_per_sec: bar.fill_per_sec,
            drain_per_sec: bar.drain_per_sec,
            timeout_secs: bar.timeout_secs,
            seed: rng.seed(),
            treasure: treasure
                .as_ref()
                .map(|&(at, _)| (at, bar.treasure_fill_secs)),
            legendary,
        });
        f.stage = Stage::Reeling(Bite {
            fish: item,
            level,
            legend: legendary,
            pool,
            place: water.unwrap_or(pool.key.as_str()),
            start_fill: bar.start_fill,
            fill_per_sec: bar.fill_per_sec,
            treasure: treasure.map(|(_, t)| t),
            left: bar.timeout_secs + RESULT_SLACK_SECS,
        });
        wear_lure(c, events);
    }

    /// `FishingResult` with a bar open: a win the realm believes lands the fish, and the chest
    /// when it was held, and pays Angler XP. The cast always ends. Whether the character changed.
    pub(crate) fn fishing_result(
        &mut self,
        won: bool,
        perfect: bool,
        treasure: bool,
        secs: f32,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        let reeling = matches!(
            self.fishing,
            Some(Fishing {
                stage: Stage::Reeling(_),
                ..
            })
        );
        if !reeling {
            return false;
        }
        let Some(Fishing {
            bobber,
            stage: Stage::Reeling(bite),
            ..
        }) = self.end_cast(events)
        else {
            return false;
        };
        let timeout = fish::bar().timeout_secs;
        let fastest = (1.0 - bite.start_fill) / bite.fill_per_sec * FASTEST_WIN;
        let won = won && secs >= fastest && secs <= timeout + RESULT_SLACK_SECS;
        let mut caught = None;
        let mut chest = false;
        if won {
            match equipment::receive(c, bite.fish, 1, events) {
                Ok(()) => caught = Some(bite.fish),
                Err(reason) => events.push(equipment::inventory_failure(reason, 0)),
            }
            if let Some(t) = bite.treasure.filter(|_| treasure) {
                chest = true;
                if let Some(item) = t.item {
                    if let Err(reason) = equipment::receive(c, item, 1, events) {
                        events.push(equipment::inventory_failure(reason, 0));
                    }
                }
                c.inventory.money = c.inventory.money.saturating_add(t.copper);
                events.push(values(c.guid, &[equipment::money_pair(&c.inventory)]));
            }
        }
        if let Some(item) = caught {
            let xp = catch_xp(item, bite.level, perfect, c.angler_level);
            gain_angler_xp(c, xp);
            // A Legendary Fish pays its bonus once, and never bites for them again.
            let first_legend = bite.legend && !c.legends_caught.contains(&item);
            if first_legend {
                c.legends_caught.push(item);
                gain_angler_xp(c, fish::angler().legend_bonus_xp);
            }
            events.push(angler(c));
            record_catch(c, item, perfect, bite.place);
            // Every catch in the pool rolls its Legend Rumours.
            let mut hinted = false;
            for (legend, note) in rumours::legend_catch_hits(c, &bite.pool.key, rng) {
                find_hint(c, legend, note, events);
                hinted = true;
            }
            // A hint sends the Journal itself, after the log changed.
            if !hinted {
                events.push(journal(c));
            }
            // A collect quest may want the fish that just landed in the Satchel.
            crate::quests::refresh(c, events);
        }
        events.push(SessionEvent::FishingEnd {
            caught,
            perfect: won && perfect,
            treasure: chest,
        });
        self.reel_in(bobber, false, events);
        won
    }
}

/// The Lure slot's lure loses a point at a bite. At zero it breaks: the first piece of its entry
/// in the Satchel takes its place, or the slot empties and waits for one (`lure_waiting`). The
/// Journal follows either way.
fn wear_lure(c: &mut Char, events: &mut Vec<SessionEvent>) {
    let Some(lure) = &mut c.fishing_lure else {
        return;
    };
    lure.durability = lure.durability.saturating_sub(1);
    if lure.durability > 0 {
        events.push(journal(c));
        return;
    }
    let entry = lure.entry;
    let name = items::template(entry).map_or_else(String::new, |t| t.name.clone());
    c.fishing_lure = None;
    let line = match c.inventory.satchel.take_first(entry) {
        Some(next) => {
            c.fishing_lure = Some(next);
            format!("Your {name} broke. Another one is on the line.")
        }
        None => {
            c.lure_waiting = Some(entry);
            format!("Your {name} broke.")
        }
    };
    events.push(crate::quests::system_line(line));
    events.push(journal(c));
}

/// An empty Lure slot that waits for a lure takes the first piece of that entry in the Satchel,
/// if one is there now. The Journal follows when it did.
pub(crate) fn refill(c: &mut Char, events: &mut Vec<SessionEvent>) {
    if c.fishing_lure.is_some() {
        return;
    }
    let Some(entry) = c.lure_waiting else {
        return;
    };
    let Some(piece) = c.inventory.satchel.take_first(entry) else {
        return;
    };
    c.fishing_lure = Some(piece);
    c.lure_waiting = None;
    events.push(journal(c));
}

/// Whether `entry` belongs in `slot`.
fn fits_slot(slot: FishingGearSlot, entry: u32) -> bool {
    match slot {
        FishingGearSlot::Pole => fish::pole(entry).is_some(),
        FishingGearSlot::Lure => fish::lure(entry).is_some(),
    }
}

fn gear_cell(c: &mut Char, slot: FishingGearSlot) -> &mut Option<crate::inventory::Item> {
    match slot {
        FishingGearSlot::Pole => &mut c.fishing_pole,
        FishingGearSlot::Lure => &mut c.fishing_lure,
    }
}

/// Moves the Satchel piece `from` names into `slot`, and what the slot held into the Satchel;
/// `None` puts the slot's item into the Satchel, and clears a Lure slot's `lure_waiting`. The
/// Journal follows. Whether anything changed.
fn put_gear(
    c: &mut Char,
    slot: FishingGearSlot,
    from: Option<SatchelPick>,
    events: &mut Vec<SessionEvent>,
) -> bool {
    let lure = slot == FishingGearSlot::Lure;
    let changed = match from {
        None => {
            let held = gear_cell(c, slot).take();
            let waiting = lure && c.lure_waiting.take().is_some();
            let moved = held.is_some();
            if let Some(held) = held {
                c.inventory.satchel.deposit(held);
            }
            moved || waiting
        }
        Some(pick) => {
            if !fits_slot(slot, pick.item) {
                return false;
            }
            let Some(item) = c.inventory.satchel.take_piece(pick) else {
                return false;
            };
            if let Some(old) = gear_cell(c, slot).replace(item) {
                c.inventory.satchel.deposit(old);
            }
            if lure {
                c.lure_waiting = None;
            }
            true
        }
    };
    if changed {
        events.push(journal(c));
    }
    changed
}

/// Takes one of bait `item` from the Satchel. The chosen bait stays when it was the last.
fn take_bait(c: &mut Char, item: u32, events: &mut Vec<SessionEvent>) {
    if c.inventory.count(item) > 0 {
        equipment::take(c, item, 1, events);
    }
}

impl crate::Realm {
    /// The served session's Fishing clock, with the host: a cast that lands gets its bobber,
    /// seen by everyone in the world on its map, and the bobbers of the casts that ended go.
    pub(crate) fn tick_fishing(&mut self, dt: f32, host: &dyn Host) {
        let Some(w) = &mut self.world else {
            return;
        };
        let Some(c) = self.characters.get_mut(w.guid) else {
            return;
        };
        let landed = w.fishing_tick(dt, self.wall, host, c, &mut self.rng, &mut self.events);
        self.remove_spent_bobbers();
        let Some(spec) = landed else {
            return;
        };
        let at = spec.position;
        let audience = self.in_world_on(spec.map);
        let bobber = self.objects.spawn(spec, audience);
        if let Some(w) = &mut self.world {
            w.bobber_landed(bobber, at, &mut self.events);
        }
    }

    /// Removes the served session's spent bobbers from the run-time objects; each client that
    /// had one is sent its destroy on its next tick.
    pub(crate) fn remove_spent_bobbers(&mut self) {
        let Some(w) = &mut self.world else {
            return;
        };
        for bobber in w.take_spent_bobbers() {
            self.objects.remove(bobber);
        }
    }

    /// Everyone in the world on `map`.
    fn in_world_on(&self, map: u32) -> Audience {
        let worlds = self
            .world
            .iter()
            .chain(self.parked.values().filter_map(|s| s.world.as_ref()));
        let guids: Vec<u64> = worlds
            .filter_map(|w| self.characters.get(w.guid))
            .filter(|c| c.map == map)
            .map(|c| c.guid)
            .collect();
        Audience::of(&guids)
    }

    /// `FishingResult`: the bar's outcome, saved when it changed the character.
    pub(crate) fn fishing_result(&mut self, won: bool, perfect: bool, treasure: bool, secs: f32) {
        let Some(w) = &mut self.world else {
            return;
        };
        let Some(c) = self.characters.get_mut(w.guid) else {
            return;
        };
        let changed = w.fishing_result(
            won,
            perfect,
            treasure,
            secs,
            c,
            &mut self.rng,
            &mut self.events,
        );
        let guid = w.guid;
        self.remove_spent_bobbers();
        if changed {
            self.save(guid);
        }
    }

    /// `SetBait`: a `[[bait]]` item the character carries becomes the Journal's bait, and `None`
    /// clears it; the Journal comes back. Anything else is ignored.
    pub(crate) fn set_bait(&mut self, item: Option<u32>) {
        let Some(guid) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        let Some(c) = self.characters.get_mut(guid) else {
            return;
        };
        let fits = item.is_none_or(|i| fish::bait(i).is_some() && c.inventory.count(i) > 0);
        if !fits {
            return;
        }
        c.bait = item;
        self.events.push(journal(c));
        self.save(guid);
    }

    /// `SetFishingGear`: see [`RealmCommand::SetFishingGear`]. Saved when it changed anything.
    pub(crate) fn set_fishing_gear(&mut self, slot: FishingGearSlot, from: Option<SatchelPick>) {
        let Some(guid) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        let Some(c) = self.characters.get_mut(guid) else {
            return;
        };
        if put_gear(c, slot, from, &mut self.events) {
            self.save(guid);
        }
    }

    /// A line of an angler hint in `npc`'s menu: the Legend Rumour's note text is its reply and the
    /// hint is recorded. Ignored below the hint's Angler Level.
    pub(crate) fn hint_select(&mut self, npc: &Npc, option: u32) {
        if !self.near(npc) || !is_hint_option(option) {
            return;
        }
        let Some(hint) = npc.angler_hints.get((option - OPTION_HINT) as usize) else {
            return;
        };
        let Some(guid) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        let Some(c) = self.characters.get_mut(guid) else {
            return;
        };
        if u32::from(c.angler_level) < hint.min_angler_level {
            return;
        }
        let note = rumours::legend_note(hint.legend)
            .and_then(|n| crate::quests::content::item_template(n).or_else(|| items::template(n)));
        self.events.push(SessionEvent::GossipComplete);
        if let Some(note) = note {
            self.events
                .push(crate::quests::system_line(note.pages.join(" ")));
        }
        if !c.legend_hints.contains(&hint.legend) {
            c.legend_hints.push(hint.legend);
            self.events.push(journal(c));
            self.save(guid);
        }
    }

    /// A line of Lee Brown's menu: his goods open the merchant window; the lesson teaches
    /// Fishing. Both close the menu.
    pub(crate) fn angler_select(&mut self, npc: &Npc, option: u32) {
        if !self.near(npc) || !matches!(option, OPTION_BROWSE | OPTION_TEACH) {
            return;
        }
        let Some(guid) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        let Some(c) = self.characters.get_mut(guid) else {
            return;
        };
        self.events.push(SessionEvent::GossipComplete);
        if option == OPTION_BROWSE {
            crate::vendor::list(npc.guid(), c, &mut self.events);
            return;
        }
        if c.knows_fishing {
            return;
        }
        c.knows_fishing = true;
        self.events.push(SessionEvent::SpellLearned {
            spell_id: fish::spell().id,
        });
        self.save(guid);
    }
}
