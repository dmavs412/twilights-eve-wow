//! Fishing (ADR 0018): the cast, the bobber, the bite, the Reeling Bar's result and Angler XP.
//! The cast is checked at once for what needs no host. The water and the pool wait for the
//! realm's next tick, which has the host. The bobber is a run-time GameObject the realm keeps
//! (`crate::objects`); this session names the ones whose cast ended in `spent_bobbers`.
//!
//! A lure (#329) is a ten-minute aura under its item's use spell; the chosen bait is taken one per
//! cast when the bobber lands. What both add is worked out once, at the landing ([`Boost`]), and
//! the cast keeps it to the bite. The Fishing Journal is each fish's best catch and the bait.

use benilla_protocol::messages::GossipOption;
use benilla_protocol::SessionEvent;

use super::cast::{refused, SPELL_FAILED_EQUIPPED_ITEM_CLASS, SPELL_FAILED_NOT_KNOWN};
use super::food::UNIT_FIELD_AURALEVELS;
use super::food::{use_up_one, AURA_FLAGS, UNIT_FIELD_AURA, UNIT_FIELD_AURAFLAGS};
use super::InWorld;
use crate::audience::Audience;
use crate::characters::Char;
use crate::equipment;
use crate::fields::{guid_pairs, values, UNIT_CHANNEL_SPELL, UNIT_FIELD_CHANNEL_OBJECT};
use crate::fish::{self, Pool, PoolFish, Style};
use crate::hub::Npc;
use crate::inventory::Pos;
use crate::items;
use crate::objects::{ObjectKind, ObjectSpec};
use crate::rng::Rng;
use crate::rules::game_clock;
use crate::rules::gear::SLOT_MAIN_HAND;
use crate::rules::xp::gray_level;
use crate::rumours;
use crate::{clock, Host};

/// `SPELL_FAILED_NOT_FISHABLE`: "Your cast didn't land in fishable water".
const SPELL_FAILED_NOT_FISHABLE: u8 = 0x34;
/// `ITEM_CLASS_WEAPON`.
const WEAPON: u32 = 2;
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

/// The buff slot a lure's aura shows in: the one before the meal's.
const LURE_AURA_SLOT: u16 = 30;

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

/// The lure on the character: its item and the seconds left.
pub(super) struct Lure {
    item: u32,
    left: f32,
}

/// A Fishing cast under way, from the cast to the end of the Reeling Bar.
pub(super) struct Fishing {
    /// The bobber's guid; 0 until it lands.
    bobber: u64,
    stage: Stage,
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
pub(super) fn journal(c: &Char) -> SessionEvent {
    let best = |item: u32| {
        c.catches
            .iter()
            .find(|&&(i, _)| i == item)
            .map_or(0, |&(_, b)| b)
    };
    SessionEvent::FishingJournal {
        bait: c.bait,
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

/// Records a catch of `item` in the Journal, a perfect one beating a plain one: whether it
/// changed.
fn record_catch(c: &mut Char, item: u32, perfect: bool) -> bool {
    let quality = if perfect { PERFECT } else { CAUGHT };
    match c.catches.iter_mut().find(|(i, _)| *i == item) {
        Some((_, best)) if *best >= quality => false,
        Some((_, best)) => {
            *best = quality;
            true
        }
        None => {
            c.catches.push((item, quality));
            true
        }
    }
}

/// The chosen bait's row while the character carries one.
fn carried_bait(c: &Char) -> Option<&'static fish::Bait> {
    c.bait
        .filter(|&b| c.inventory.count(b) > 0)
        .and_then(fish::bait)
}

/// Whether the main hand holds a fishing pole.
fn holds_pole(c: &Char) -> bool {
    c.inventory
        .get(Pos::own(SLOT_MAIN_HAND))
        .and_then(|i| items::template(i.entry))
        .is_some_and(|t| t.class == WEAPON && t.subclass == fish::spell().pole_subclass)
}

/// The fish of `pool` that bite at Angler Level `effective`: those fewer than
/// `gate_levels_above` levels above it.
fn biters(pool: &'static Pool, effective: u32) -> Vec<&'static PoolFish> {
    let gate = fish::bar().gate_levels_above;
    pool.fish
        .iter()
        .filter(|f| f.level < effective + gate && fish::fish(f.item).is_some())
        .collect()
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
        self.fishing = Some(Fishing {
            bobber: 0,
            stage: Stage::Cast,
        });
    }

    /// The cast or the bar under way is lost: the client is told the fish got away, the line
    /// comes in, and the bobber goes.
    pub(crate) fn lose_fish(&mut self, events: &mut Vec<SessionEvent>) {
        let Some(f) = self.fishing.take() else {
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

    /// The channel to `bobber` ends, `broken` when it stopped short, and the bobber is spent.
    fn reel_in(&mut self, bobber: u64, broken: bool, events: &mut Vec<SessionEvent>) {
        self.channel_closed(fish::spell().id, broken, events);
        self.spent_bobbers.push(bobber);
    }

    /// The bobbers of the casts that ended since the last call, for the realm to remove.
    pub(crate) fn take_spent_bobbers(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.spent_bobbers)
    }

    /// What the lure on the character adds to a cast.
    fn lure_boost(&self) -> Boost {
        self.lure
            .as_ref()
            .and_then(|l| fish::lure(l.item))
            .map_or(Boost::default(), |l| Boost::default().with(&l.effect))
    }

    /// `CMSG_USE_ITEM` on the item in player slot `slot`, if it is a lure: one of the stack goes
    /// and its aura goes on under the item's use spell for the lure's duration, in place of any
    /// lure before it. The stock spell wants the pole as its target; whatever target the client
    /// sent, the use is taken. Whether it was a lure.
    pub(crate) fn use_lure(
        &mut self,
        slot: Pos,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        let Some(item) = c.inventory.get(slot) else {
            return false;
        };
        let entry = item.entry;
        let Some(lure) = fish::lure(entry) else {
            return false;
        };
        let spell_id = items::template(entry).map_or(0, |t| t.use_spell);
        if self.dead() {
            return true;
        }
        let Some(item_guid) = use_up_one(slot, c, events) else {
            return true;
        };
        events.push(SessionEvent::CastResult {
            spell_id,
            success: true,
            reason: None,
            arg: None,
        });
        events.push(SessionEvent::SpellGo {
            caster: self.guid,
            spell_id,
            cast_flags: 0,
            hits: vec![self.guid],
            misses: Vec::new(),
            target: Some(self.guid),
            go_target: None,
            dest: None,
            ammo_display_id: None,
            item_caster: Some(item_guid),
        });
        let s = LURE_AURA_SLOT;
        events.push(values(
            self.guid,
            &[
                (UNIT_FIELD_AURA + s, spell_id),
                (UNIT_FIELD_AURAFLAGS + s / 8, AURA_FLAGS << (4 * (s % 8))),
                (
                    UNIT_FIELD_AURALEVELS + s / 4,
                    u32::from(c.level) << (8 * (s % 4)),
                ),
            ],
        ));
        let secs = lure.duration_secs as f32;
        events.push(SessionEvent::AuraDuration {
            slot: s as u8,
            remaining_ms: clock::ms(secs),
        });
        self.lure = Some(Lure {
            item: entry,
            left: secs,
        });
        true
    }

    /// The lure's clock: when it runs out its aura goes.
    fn lure_tick(&mut self, dt: f32, events: &mut Vec<SessionEvent>) {
        let Some(l) = &mut self.lure else {
            return;
        };
        l.left -= dt;
        if l.left > 0.0 {
            return;
        }
        self.lure = None;
        let s = LURE_AURA_SLOT;
        events.push(values(
            self.guid,
            &[
                (UNIT_FIELD_AURA + s, 0),
                (UNIT_FIELD_AURAFLAGS + s / 8, 0),
                (UNIT_FIELD_AURALEVELS + s / 4, 0),
            ],
        ));
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
        self.lure_tick(dt, events);
        let f = self.fishing.as_mut()?;
        match &mut f.stage {
            Stage::Cast => return self.land(host, c, rng, events),
            Stage::Floating {
                pool,
                boost,
                water,
                bite_in,
                left,
            } => {
                *bite_in -= dt;
                *left -= dt;
                if *bite_in <= 0.0 {
                    let (pool, boost, water) = (*pool, *boost, *water);
                    self.bite(pool, boost, water, wall, c, rng, events);
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
            self.fishing = None;
            events.push(refused(s.id, SPELL_FAILED_NOT_FISHABLE));
            return None;
        };
        let bait = carried_bait(c);
        let mut boost = bait.map_or(self.lure_boost(), |b| self.lure_boost().with(&b.effect));
        boost.bait = bait.map(|b| b.item);
        let effective = effective_level(c, boost);
        let area = host.area_at(c.map, x, y);
        let water = fish::water_at(c.map, x, y, area).map(|w| w.key.as_str());
        let pool = fish::pool_at(c.map, x, y, area).filter(|&p| !biters(p, effective).is_empty());
        let Some(pool) = pool else {
            self.fishing = None;
            events.push(SessionEvent::Notification {
                text: NOTHING_BITING.into(),
            });
            return None;
        };
        match bait {
            Some(b) => take_bait(c, b.item, events),
            // A bait chosen but no longer carried clears the slot.
            None if c.bait.is_some() => {
                c.bait = None;
                events.push(journal(c));
            }
            None => {}
        }
        let wait = rng.frand(s.bite_secs[0], s.bite_secs[1]);
        let f = self.fishing.as_mut()?;
        f.stage = Stage::Floating {
            pool,
            boost,
            water,
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
        wall: f64,
        c: &Char,
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
            None => pick(&biters(pool, effective), |f| f.weight, rng).and_then(|f| {
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
            start_fill: bar.start_fill,
            fill_per_sec: bar.fill_per_sec,
            treasure: treasure.map(|(_, t)| t),
            left: bar.timeout_secs + RESULT_SLACK_SECS,
        });
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
        }) = self.fishing.take()
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
            let mut changed = record_catch(c, item, perfect) || first_legend;
            // Every catch in the pool rolls its Legend Rumours.
            for (legend, note) in rumours::legend_catch_hits(c, &bite.pool.key, rng) {
                find_hint(c, legend, note, events);
                changed = false;
            }
            if changed {
                events.push(journal(c));
            }
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

/// Takes one of bait `item` from the bags; the last one clears the slot and sends the Journal.
fn take_bait(c: &mut Char, item: u32, events: &mut Vec<SessionEvent>) {
    let slot = c
        .inventory
        .iter()
        .find(|(at, i)| i.entry == item && !at.is_worn())
        .map(|(at, _)| at);
    if let Some(slot) = slot {
        use_up_one(slot, c, events);
    }
    if c.inventory.count(item) == 0 {
        c.bait = None;
        events.push(journal(c));
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
