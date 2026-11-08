//! The Deeprun Tram's line (spec #586, "The trams"; ADR 0033): Run state that starts once the
//! station bell is rung. Trams come on a timetable that follows the group's pace (Headway): each
//! tram's cars roll in on one track, stand at the platform and back out, and a moving car hits
//! every living player in its lane. A car is a type 11 transport on the Carts' road drive: the
//! realm sends its `SessionEvent::CartRoute` before its create, and removes it once it has backed
//! out behind the mist wall. Tram code is for the tram only; every number is the dungeon's
//! `[tram]` table in `content/deeprun_tram.toml`.
//!
//! What a car carries (#590): when a wave tram's car stops, the line rolls its creatures (fighters
//! and a porter), which the shard makes at the car's door and hands back to [`Line::bring`]. The
//! line wakes them, sets each Lampman's lamps, keeps the Tribute Pile and walks each porter from
//! the pile to a standing car. The Headway check asks whether any creature it brought is alive.
//! The boss tram's car carries no wave; its stop is #591's.

use benilla_protocol::guid::HIGH_TRANSPORT;
use benilla_protocol::SessionEvent;
use serde::Deserialize;

use crate::carts::{self, CartState, Pose};
use crate::mobs::Mob;
use crate::phased::{gameobject_create, GameObject, GENERIC, HIGHGUID_GAMEOBJECT, NOT_SELECTABLE};
use crate::rng::Rng;
use crate::space::flat_distance;

/// A walk at a creature's own walk speed.
const FULL_SPEED_PCT: u32 = 100;

/// How near its point a walking creature counts as there.
const THERE_YD: f32 = 0.5;

/// The crates on a Tribute Pile (GLOSSARY.md).
const PILE_CRATES: usize = 12;

/// A Phased Dungeon's `[tram]` table: the line's numbers.
#[derive(Deserialize)]
pub struct Tram {
    /// The lever whose use starts the line.
    pub bell: u32,
    /// The cars' GameObject entry and display.
    pub car_entry: u32,
    pub car_display: u32,
    /// The cars' Z.
    pub car_z: f32,
    /// The car stops along y, lead first: a tram of n cars fills the first n.
    pub stops: Vec<f32>,
    /// How far before its stop each car starts, behind the mist wall.
    pub run_in_yd: f32,
    pub speed_yd_s: f32,
    /// Standing at the platform.
    pub dwell_s: f32,
    /// The departure warning, this long before the dwell ends.
    pub whistle_s: f32,
    /// The bell to tram 1's arrival.
    pub first_s: f32,
    /// The gap after tram 1 and after a tram whose warning found a tram creature alive.
    pub gap_s: f32,
    /// The gap after a tram whose warning found none alive.
    pub fast_gap_s: f32,
    pub max_cars: usize,
    /// The wave trams; one boss tram follows them.
    pub wave_trams: u32,
    /// A lane: this far either side of its track's x...
    pub lane_half_yd: f32,
    /// ...this far along y from a car's centre...
    pub lane_reach_yd: f32,
    /// ...this far above or below the floor...
    pub lane_z_yd: f32,
    pub floor_z: f32,
    /// ...between the mist wall and the station's north end.
    pub wall_y: f32,
    pub lane_north_y: f32,
    /// A hit: this percent of maximum health, unaffected by armour...
    pub hit_pct: u32,
    /// ...and rooted this long under this spell.
    pub hit_root_ms: u32,
    pub hit_spell: u32,
    /// Its name, which `tests/content_spell_ids.rs` checks the id against.
    pub hit_spell_name: String,
    /// The `SoundEntries` of an arrival's warning and a departure's.
    pub bell_sound: u32,
    pub whistle_sound: u32,
    /// The lamp's `GameObjectDisplayInfo`.
    pub lamp_display: u32,
    /// What a wave tram's car carries (#590): this many fighters, each slot rolling one of
    /// `fighters` with equal odds...
    pub fighter_slots: usize,
    pub fighters: Vec<u32>,
    /// ...and at most one of them `lampman`; then one `porter`, while a crate is left.
    pub lampman: u32,
    pub porter: u32,
    /// A fighter's level is rolled in this range.
    pub fighter_levels: [u8; 2],
    pub porter_level: u8,
    /// How far apart along y a car's creatures stand at its door.
    pub step_off_spacing_yd: f32,
    /// How long a creature waits after stepping off.
    pub wake_s: f32,
    /// How long a porter takes to lift a crate...
    pub lift_s: f32,
    /// ...and its walk speed, in percent, carrying one.
    pub carry_speed_pct: u32,
    /// The porter's gold glow, a marker.
    pub gilded_spell: u32,
    pub gilded_spell_name: String,
    /// A Lampman sets a lamp this often...
    pub lamp_every_s: f32,
    /// ...that slows every player within this many yards by this percent, under this spell.
    pub lamp_radius_yd: f32,
    pub lamp_slow_pct: u32,
    pub lamp_spell: u32,
    pub lamp_spell_name: String,
    /// The Tribute Pile: each crate's `GameObjectDisplayInfo`, the first crate's guid (the entry
    /// is the guid; the rest follow it) and every crate's point.
    pub crate_display: u32,
    pub pile_guid: u32,
    pub pile: Vec<[f32; 3]>,
    /// Track a, then track b.
    pub track: Vec<Track>,
}

/// One of the tram's tracks.
#[derive(Deserialize)]
pub struct Track {
    pub key: String,
    pub x: f32,
    /// Where its cars' doors open, on the centre strip side (#590).
    pub door_x: f32,
    /// Its lamp at the platform's south end, lit while a warning lasts.
    pub lamp: [f32; 3],
    pub lamp_guid: u32,
}

impl Tram {
    /// Refuses a table the line cannot run: a pile that is not [`PILE_CRATES`] points, or fighter
    /// slots with no fighter but the Lampman to fill them. The creature templates are checked
    /// where their spawn rows are made ([`crate::mobs::content`]).
    pub(crate) fn check(&self, dungeon: &str) {
        assert_eq!(
            self.pile.len(),
            PILE_CRATES,
            "{dungeon}: the Tribute Pile is {PILE_CRATES} points"
        );
        assert!(
            self.fighter_slots <= 1 || self.fighters.iter().any(|&e| e != self.lampman),
            "{dungeon}: a car's fighter slots need a fighter besides the Lampman"
        );
        assert!(
            self.fighter_levels[0] <= self.fighter_levels[1],
            "{dungeon}: fighter_levels run low to high"
        );
        assert!(self.track.len() >= 2, "{dungeon}: tracks a and b");
    }

    /// The creatures the line brings in, each entry once, with its level range: the fighters
    /// (the Lampman among them), then the porter.
    pub(crate) fn creature_rows(&self) -> Vec<(u32, [u8; 2])> {
        let mut rows: Vec<(u32, [u8; 2])> = Vec::new();
        for &entry in self.fighters.iter().chain([&self.lampman]) {
            if !rows.iter().any(|r| r.0 == entry) {
                rows.push((entry, self.fighter_levels));
            }
        }
        if !rows.iter().any(|r| r.0 == self.porter) {
            rows.push((self.porter, [self.porter_level; 2]));
        }
        rows
    }

    /// The door of a car of track `track` stopped at `y`, on the platform floor.
    pub(crate) fn door(&self, track: usize, y: f32) -> [f32; 3] {
        [self.track[track].door_x, y, self.floor_z]
    }

    /// The middle of the Tribute Pile.
    fn pile_centre(&self) -> [f32; 3] {
        let n = self.pile.len().max(1) as f32;
        let sum = self
            .pile
            .iter()
            .fold([0.0; 3], |s, p| [s[0] + p[0], s[1] + p[1], s[2] + p[2]]);
        [sum[0] / n, sum[1] / n, sum[2] / n]
    }

    /// Crate `k`'s guid: a GameObject whose entry is its guid.
    fn crate_guid(&self, k: usize) -> u64 {
        HIGHGUID_GAMEOBJECT << 48 | u64::from(self.pile_guid + k as u32)
    }

    /// One way along a car's run in, in ms.
    pub fn leg_ms(&self) -> u64 {
        ms(self.run_in_yd / self.speed_yd_s)
    }

    pub fn dwell_ms(&self) -> u64 {
        ms(self.dwell_s)
    }

    /// A car's out and back.
    pub fn period_ms(&self) -> u64 {
        2 * (self.leg_ms() + self.dwell_ms())
    }

    /// The lamp's guid on track `track`: a GameObject whose entry is its guid.
    pub fn lamp_guid(&self, track: usize) -> u64 {
        HIGHGUID_GAMEOBJECT << 48 | u64::from(self.track[track].lamp_guid)
    }

    /// The track tram `n` (from 1) runs on: odd trams on track a, even on track b, and the boss
    /// tram on track a.
    fn track_of(&self, n: u32) -> usize {
        if n > self.wave_trams {
            0
        } else {
            ((n + 1) % 2) as usize
        }
    }

    /// The lamp of `track`, lit.
    fn lamp_create(&self, track: usize) -> SessionEvent {
        let t = &self.track[track];
        gameobject_create(&GameObject {
            guid: self.lamp_guid(track),
            entry: t.lamp_guid,
            display_id: self.lamp_display,
            type_id: GENERIC,
            flags: NOT_SELECTABLE,
            faction: 0,
            position: t.lamp,
            orientation: 0.0,
        })
    }
}

fn ms(secs: f32) -> u64 {
    (f64::from(secs) * 1000.0).round() as u64
}

/// One car of a tram: an out and back from behind the wall to its stop.
struct Car {
    guid: u64,
    track: usize,
    /// Its route: from behind the wall to its stop.
    path: [[f32; 3]; 2],
    along: Vec<f32>,
    /// The line's clock when it started rolling in, which is `dwell_ms` into its period.
    started_ms: u64,
    /// The players it has hit on the way in, and on the way out.
    hit_in: Vec<u64>,
    hit_out: Vec<u64>,
    /// The boss tram's, which carries no wave.
    boss: bool,
    /// It has stopped at the platform.
    stopped: bool,
}

/// A car that stopped since the shard last asked: its track and stop.
struct Arrival {
    track: usize,
    stop_y: f32,
    boss: bool,
}

/// What a tram creature is to the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// It turns on the closest player once awake.
    Fighter,
    /// A fighter that also sets lamps.
    Lampman,
    /// It never fights; it carries crates from the pile onto the cars.
    Porter,
}

/// A creature stepping off a car, for the shard to make: the line's `seq`th, which numbers its
/// guid.
pub(crate) struct Stepping {
    pub(crate) entry: u32,
    pub(crate) level: u8,
    pub(crate) at: [f32; 3],
    pub(crate) facing: f32,
    pub(crate) kind: Kind,
    pub(crate) seq: u32,
    /// The marker auras it wears, `(spell, stacks)`.
    pub(crate) wears: Vec<(u32, u32)>,
}

/// Where a porter is in its walk.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Porter {
    /// Not yet awake.
    Waking,
    /// Walking to crate `k` on the pile, which no other porter takes meanwhile.
    ToCrate(usize),
    /// Lifting crate `k`, until the line's clock reaches `until_ms`.
    Lifting { k: usize, until_ms: u64 },
    /// Carrying crate `k` to `to`.
    Carrying { k: usize, to: Dest },
    /// Beside the pile, which has no crate for it.
    Waiting,
}

/// Where a porter carries its crate.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Dest {
    /// The door of the standing car of this guid.
    Car(u64),
    /// This track's door at the middle stop, to wait for a car.
    Track(usize),
}

/// A crate of the Tribute Pile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Crate {
    OnPile,
    Carried,
    Escaped,
}

/// A creature a tram brought.
struct Brought {
    guid: u64,
    kind: Kind,
    /// The line's clock when it stepped off.
    made_ms: u64,
    /// A fighter turned on a player.
    turned: bool,
    /// It died; the line has done with it.
    dead: bool,
    /// A Lampman's next lamp, on the line's clock.
    next_lamp_ms: u64,
    /// A porter's walk, and the track it last walked to.
    porter: Porter,
    last_track: usize,
}

/// A Lampman's lamp where he stood.
struct Lamp {
    guid: u64,
    owner: u64,
    position: [f32; 3],
}

/// What the line reports for the Conductor (#591) and the chest (#592).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    /// Crates on the Tribute Pile, not carried and not escaped.
    pub on_pile: usize,
    pub escaped: usize,
    pub porters_made: u32,
    pub porters_dead: u32,
}

/// A lamp's lit spell: track `track`'s lamp is in view until the line's clock reaches `until_ms`.
struct Lit {
    track: usize,
    until_ms: u64,
}

/// A tram's departure whistle, due at `at_ms` on the line's clock.
struct Whistle {
    track: usize,
    at_ms: u64,
}

/// A car hit a player: deal the line's hit to `player`, from `car`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub player: u64,
    pub car: u64,
}

/// A player in the line's Phase, as a lane sees them.
pub struct Standing {
    pub guid: u64,
    pub position: [f32; 3],
    pub alive: bool,
}

/// The tram line of one Run.
pub struct Line {
    tram: &'static Tram,
    /// The map its dungeon stands on, which the cars' routes name.
    map: u32,
    /// Seconds since the line started, summed from each tick's `dt`...
    elapsed_s: f64,
    /// ...and in whole milliseconds, which the timetable counts in.
    now_ms: u64,
    /// The next tram to warn, from 1, and when it arrives; past the boss tram, none.
    next: u32,
    next_arrival_ms: u64,
    /// The last tram's cars.
    last_cars: usize,
    cars: Vec<Car>,
    lit: Vec<Lit>,
    whistles: Vec<Whistle>,
    /// The sounds due since the realm last asked: `(sound, lamp guid)`.
    sounds: Vec<(u32, u64)>,
    /// How many cars and lamps the line has made, which numbers their guids: a new tram never
    /// reuses an old car's.
    made: u32,
    /// The dice its cars' creatures are rolled with.
    rng: Rng,
    /// The cars that stopped since the shard last asked.
    arrivals: Vec<Arrival>,
    /// How many creatures the line has rolled, which numbers their guids.
    seq: u32,
    /// The creatures the trams brought (#590), which the Headway check counts.
    brought: Vec<Brought>,
    /// The Tribute Pile, crate by crate.
    pile: Vec<Crate>,
    lamps: Vec<Lamp>,
    porters_made: u32,
    porters_dead: u32,
}

impl Line {
    /// A line on `map` just started: tram 1 arrives `first_s` from now. `seed` rolls its cars'
    /// creatures.
    pub fn new(tram: &'static Tram, map: u32, seed: u64) -> Self {
        Self {
            tram,
            map,
            elapsed_s: 0.0,
            now_ms: 0,
            next: 1,
            next_arrival_ms: ms(tram.first_s),
            last_cars: 0,
            cars: Vec::new(),
            lit: Vec::new(),
            whistles: Vec::new(),
            sounds: Vec::new(),
            made: 0,
            rng: Rng::new(seed),
            arrivals: Vec::new(),
            seq: 0,
            brought: Vec::new(),
            pile: vec![Crate::OnPile; tram.pile.len()],
            lamps: Vec::new(),
            porters_made: 0,
            porters_dead: 0,
        }
    }

    /// The line's numbers.
    pub fn tram(&self) -> &'static Tram {
        self.tram
    }

    /// Counts the creature `guid`, of `kind`, as one a tram brought, stepping off now.
    pub fn bring(&mut self, guid: u64, kind: Kind) {
        if kind == Kind::Porter {
            self.porters_made += 1;
        }
        self.brought.push(Brought {
            guid,
            kind,
            made_ms: self.now_ms,
            turned: false,
            dead: false,
            next_lamp_ms: self.now_ms + ms(self.tram.wake_s),
            porter: Porter::Waking,
            last_track: 0,
        });
    }

    /// Whether the creature `guid` is one a tram brought.
    pub fn brought(&self, guid: u64) -> bool {
        self.brought.iter().any(|b| b.guid == guid)
    }

    /// The pile's crates, the escapes and the porters, for the Conductor (#591) and the chest
    /// (#592).
    pub fn report(&self) -> Report {
        let count = |c: Crate| self.pile.iter().filter(|&&p| p == c).count();
        Report {
            on_pile: count(Crate::OnPile),
            escaped: count(Crate::Escaped),
            porters_made: self.porters_made,
            porters_dead: self.porters_dead,
        }
    }

    /// The creatures of each wave car that stopped since the last ask: `fighter_slots` fighters,
    /// each rolled from `fighters` with a second Lampman rerolled, then a porter while any crate
    /// has yet to escape. They stand at the car's door along y, facing the centre strip.
    pub(crate) fn take_stepping(&mut self) -> Vec<Stepping> {
        let tram = self.tram;
        let mut out = Vec::new();
        for a in std::mem::take(&mut self.arrivals) {
            // The boss tram's stop brings the Conductor (#591), not a wave.
            if a.boss {
                continue;
            }
            let mut kinds = Vec::new();
            let mut lampman = false;
            for _ in 0..tram.fighter_slots {
                let entry = loop {
                    let e = tram.fighters[self.rng.below(tram.fighters.len() as u32) as usize];
                    if !(e == tram.lampman && lampman) {
                        break e;
                    }
                };
                lampman |= entry == tram.lampman;
                let [lo, hi] = tram.fighter_levels;
                let level = self.rng.urand(u32::from(lo), u32::from(hi)) as u8;
                let kind = if entry == tram.lampman {
                    Kind::Lampman
                } else {
                    Kind::Fighter
                };
                kinds.push((entry, level, kind, Vec::new()));
            }
            if self.pile.iter().any(|&c| c != Crate::Escaped) {
                let wears = vec![(tram.gilded_spell, 0)];
                kinds.push((tram.porter, tram.porter_level, Kind::Porter, wears));
            }
            let door = tram.door(a.track, a.stop_y);
            let facing = 0.0f32.atan2(tram.pile_centre()[0] - door[0]);
            for (j, (entry, level, kind, wears)) in kinds.into_iter().enumerate() {
                self.seq += 1;
                let k = j as f32 - 1.0;
                out.push(Stepping {
                    entry,
                    level,
                    at: [door[0], door[1] + k * tram.step_off_spacing_yd, door[2]],
                    facing,
                    kind,
                    seq: self.seq,
                    wears,
                });
            }
        }
        out
    }

    /// The fighters awake since the last ask, each once: the realm turns each on the closest
    /// living player.
    pub(crate) fn take_waking(&mut self) -> Vec<u64> {
        let (now, wake) = (self.now_ms, ms(self.tram.wake_s));
        self.brought
            .iter_mut()
            .filter(|b| b.kind != Kind::Porter && !b.turned && !b.dead)
            .filter(|b| b.made_ms + wake <= now)
            .map(|b| {
                b.turned = true;
                b.guid
            })
            .collect()
    }

    /// The lamps' points, which slow the players near them.
    pub fn lamp_points(&self) -> Vec<[f32; 3]> {
        self.lamps.iter().map(|l| l.position).collect()
    }

    /// The tram creatures' own turn, among the shard's `mobs`: a death puts a carried crate back
    /// and puts out a Lampman's lamps, each awake Lampman sets his lamp when it is due, and each
    /// awake porter walks on; one that boarded a standing car leaves `mobs`. The moves the
    /// clients that see the walkers are sent.
    pub(crate) fn run_creatures(&mut self, mobs: &mut Vec<Mob>) -> Vec<SessionEvent> {
        let tram = self.tram;
        let now = self.now_ms;
        let wake = ms(tram.wake_s);
        let standing = self.standing_doors();
        let mut events = Vec::new();
        let mut boarded = Vec::new();
        for i in 0..self.brought.len() {
            let b = &self.brought[i];
            if b.dead {
                continue;
            }
            let Some(mob) = mobs.iter_mut().find(|m| m.guid == b.guid) else {
                continue;
            };
            if !mob.alive() {
                self.died(i);
                continue;
            }
            if now < b.made_ms + wake {
                continue;
            }
            match b.kind {
                Kind::Fighter => {}
                Kind::Lampman => {
                    if now >= b.next_lamp_ms {
                        self.made += 1;
                        let guid = mist_lamp_guid(tram.lamp_display, self.made);
                        let owner = b.guid;
                        self.lamps.push(Lamp {
                            guid,
                            owner,
                            position: mob.position,
                        });
                        self.brought[i].next_lamp_ms += ms(tram.lamp_every_s);
                    }
                }
                Kind::Porter => {
                    if self.porter_step(i, mob, &standing, &mut events) {
                        boarded.push(mob.guid);
                    }
                }
            }
        }
        mobs.retain(|m| !boarded.contains(&m.guid));
        events
    }

    /// Creature `i` died: a porter's carried crate goes back on the pile, and a Lampman's lamps
    /// go out.
    fn died(&mut self, i: usize) {
        let b = &mut self.brought[i];
        b.dead = true;
        let guid = b.guid;
        match (b.kind, b.porter) {
            (Kind::Porter, Porter::Carrying { k, .. }) => {
                self.pile[k] = Crate::OnPile;
                self.porters_dead += 1;
            }
            (Kind::Porter, _) => self.porters_dead += 1,
            (Kind::Lampman, _) => self.lamps.retain(|l| l.owner != guid),
            (Kind::Fighter, _) => {}
        }
    }

    /// The cars standing at the platform now: guid, track and door.
    fn standing_doors(&self) -> Vec<(u64, usize, [f32; 3])> {
        let (leg, dwell) = (self.tram.leg_ms(), self.tram.dwell_ms());
        self.cars
            .iter()
            .filter(|c| {
                let arrived = c.started_ms + leg;
                (arrived..arrived + dwell).contains(&self.now_ms)
            })
            .map(|c| (c.guid, c.track, self.tram.door(c.track, c.path[1][1])))
            .collect()
    }

    /// Porter `i`'s walk on: to the closest free crate on the pile (or beside the pile, to wait
    /// for one), a lift, then to the door of the closest standing car (or its last track's middle
    /// door, to wait for one). Whether it boarded a standing car, its crate escaped.
    fn porter_step(
        &mut self,
        i: usize,
        mob: &mut Mob,
        standing: &[(u64, usize, [f32; 3])],
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        let tram = self.tram;
        let now = self.now_ms;
        let state = self.brought[i].porter;
        let carry = tram.carry_speed_pct;
        match state {
            Porter::Waking | Porter::Waiting => {
                let next = self.free_crate(mob.position);
                let (state, to) = match next {
                    Some(k) => (Porter::ToCrate(k), tram.pile[k]),
                    None => (Porter::Waiting, tram.pile_centre()),
                };
                self.brought[i].porter = state;
                walk_toward(mob, to, FULL_SPEED_PCT, events);
            }
            Porter::ToCrate(k) => {
                if there(mob, tram.pile[k]) {
                    self.brought[i].porter = Porter::Lifting {
                        k,
                        until_ms: now + ms(tram.lift_s),
                    };
                } else {
                    walk_toward(mob, tram.pile[k], FULL_SPEED_PCT, events);
                }
            }
            Porter::Lifting { k, until_ms } => {
                if now >= until_ms {
                    self.pile[k] = Crate::Carried;
                    let to = self.carry_to(i, mob.position, standing);
                    self.brought[i].porter = Porter::Carrying { k, to };
                    walk_toward(mob, self.dest_point(to, standing), carry, events);
                }
            }
            Porter::Carrying { k, to } => {
                let still = match to {
                    Dest::Car(car) => standing.iter().any(|s| s.0 == car),
                    Dest::Track(_) => standing.is_empty(),
                };
                if !still {
                    let to = self.carry_to(i, mob.position, standing);
                    self.brought[i].porter = Porter::Carrying { k, to };
                    walk_toward(mob, self.dest_point(to, standing), carry, events);
                    return false;
                }
                let point = self.dest_point(to, standing);
                if !there(mob, point) {
                    walk_toward(mob, point, carry, events);
                } else if matches!(to, Dest::Car(_)) {
                    self.pile[k] = Crate::Escaped;
                    self.brought[i].porter = Porter::Waiting;
                    return true;
                }
            }
        }
        false
    }

    /// The closest crate on the pile to `at` that no porter is walking to or lifting.
    fn free_crate(&self, at: [f32; 3]) -> Option<usize> {
        let taken = |k: usize| {
            self.brought.iter().any(|b| {
                !b.dead
                    && matches!(b.porter,
                        Porter::ToCrate(j) | Porter::Lifting { k: j, .. } if j == k)
            })
        };
        (0..self.pile.len())
            .filter(|&k| self.pile[k] == Crate::OnPile && !taken(k))
            .min_by(|&a, &b| {
                flat_distance(at, self.tram.pile[a])
                    .total_cmp(&flat_distance(at, self.tram.pile[b]))
            })
    }

    /// Where porter `i` at `at` carries its crate: the closest standing car, whose track it
    /// last walked to from then on; with none standing, its last track's middle door.
    fn carry_to(&mut self, i: usize, at: [f32; 3], standing: &[(u64, usize, [f32; 3])]) -> Dest {
        let closest = standing
            .iter()
            .min_by(|a, b| flat_distance(at, a.2).total_cmp(&flat_distance(at, b.2)));
        match closest {
            Some(&(car, track, _)) => {
                self.brought[i].last_track = track;
                Dest::Car(car)
            }
            None => Dest::Track(self.brought[i].last_track),
        }
    }

    /// The point of `to`: a standing car's door, or a track's door at the middle stop.
    fn dest_point(&self, to: Dest, standing: &[(u64, usize, [f32; 3])]) -> [f32; 3] {
        let middle = |track: usize| {
            let y = self.tram.stops[self.tram.stops.len() / 2];
            self.tram.door(track, y)
        };
        match to {
            Dest::Car(car) => standing
                .iter()
                .find(|s| s.0 == car)
                .map_or(middle(0), |s| s.2),
            Dest::Track(track) => middle(track),
        }
    }

    /// Runs the line `dt` seconds on: each tram due its warning gets its cars rolling, each
    /// whistle due sounds, and each car that has backed out behind the wall is gone. `alive` is
    /// whether any creature a tram brought is alive, for the Headway check.
    pub fn tick(&mut self, dt: f32, alive: bool) {
        self.elapsed_s += f64::from(dt);
        self.now_ms = (self.elapsed_s * 1000.0).round() as u64;
        let leg = self.tram.leg_ms();
        while self.next <= self.tram.wave_trams + 1
            && self.next_arrival_ms.saturating_sub(leg) <= self.now_ms
        {
            self.warn(alive);
        }
        let now = self.now_ms;
        let due: Vec<usize> = self
            .whistles
            .iter()
            .filter(|w| w.at_ms <= now)
            .map(|w| w.track)
            .collect();
        self.whistles.retain(|w| w.at_ms > now);
        for track in due {
            self.sounds
                .push((self.tram.whistle_sound, self.tram.lamp_guid(track)));
            self.lit.push(Lit {
                track,
                until_ms: now + ms(self.tram.whistle_s),
            });
        }
        for car in self.cars.iter_mut() {
            if !car.stopped && car.started_ms + leg <= now {
                car.stopped = true;
                self.arrivals.push(Arrival {
                    track: car.track,
                    stop_y: car.path[1][1],
                    boss: car.boss,
                });
            }
        }
        let period = self.tram.period_ms();
        let dwell = self.tram.dwell_ms();
        self.cars.retain(|c| dwell + now - c.started_ms < period);
        self.lit.retain(|l| l.until_ms > now);
    }

    /// The warning of the next tram: its cars start rolling in, the bell rings on its track's
    /// lamp, and the lamp lights until it arrives. From tram 2 on, Headway: with no tram creature
    /// `alive` the tram gets one car more than the last and the gap after it is short; with one,
    /// one car fewer and the long gap. The boss tram is one car on track a, and the last.
    fn warn(&mut self, alive: bool) {
        let tram = self.tram;
        let n = self.next;
        let (cars, gap_s) = if n == 1 {
            (1, tram.gap_s)
        } else if n > tram.wave_trams {
            (1, 0.0)
        } else if alive {
            (self.last_cars.saturating_sub(1).max(1), tram.gap_s)
        } else {
            ((self.last_cars + 1).min(tram.max_cars), tram.fast_gap_s)
        };
        let cars = cars.min(tram.stops.len());
        let track = tram.track_of(n);
        let arrival = self.next_arrival_ms;
        let started = arrival.saturating_sub(tram.leg_ms());
        let x = tram.track[track].x;
        for &stop in &tram.stops[..cars] {
            self.made += 1;
            let path = [
                [x, stop - tram.run_in_yd, tram.car_z],
                [x, stop, tram.car_z],
            ];
            self.cars.push(Car {
                guid: car_guid(tram.car_entry, self.made),
                track,
                path,
                along: carts::along(&path),
                started_ms: started,
                hit_in: Vec::new(),
                hit_out: Vec::new(),
                boss: n > tram.wave_trams,
                stopped: false,
            });
        }
        self.sounds.push((tram.bell_sound, tram.lamp_guid(track)));
        self.lit.push(Lit {
            track,
            until_ms: arrival,
        });
        self.whistles.push(Whistle {
            track,
            at_ms: (arrival + tram.dwell_ms()).saturating_sub(ms(tram.whistle_s)),
        });
        self.last_cars = cars;
        self.next += 1;
        self.next_arrival_ms = arrival + ms(gap_s);
    }

    /// `car` `t` ms into its period.
    fn pose(&self, car: &Car) -> (Pose, u64) {
        let t = self.tram.dwell_ms() + self.now_ms - car.started_ms;
        let pose = carts::sample(
            &car.path,
            &car.along,
            self.tram.dwell_ms(),
            self.tram.leg_ms(),
            1,
            t,
            true,
        );
        (pose, t)
    }

    /// The sounds due since the last ask: `(sound, guid)`, each played on a lamp.
    pub fn take_sounds(&mut self) -> Vec<(u32, u64)> {
        std::mem::take(&mut self.sounds)
    }

    /// Each moving car hits every living player in its lane near it, at most once a leg.
    pub fn hits(&mut self, players: &[Standing]) -> Vec<Hit> {
        let tram = self.tram;
        let (dwell, leg) = (tram.dwell_ms(), tram.leg_ms());
        let poses: Vec<(Pose, u64)> = self.cars.iter().map(|c| self.pose(c)).collect();
        let mut hits = Vec::new();
        for (car, (pose, t)) in self.cars.iter_mut().zip(poses) {
            if pose.state != CartState::Rolling {
                continue;
            }
            let x = tram.track[car.track].x;
            let hit = if t < dwell + leg {
                &mut car.hit_in
            } else {
                &mut car.hit_out
            };
            for p in players.iter().filter(|p| p.alive) {
                let [px, py, pz] = p.position;
                let in_lane = (px - x).abs() <= tram.lane_half_yd
                    && (py - pose.point[1]).abs() <= tram.lane_reach_yd
                    && (pz - tram.floor_z).abs() <= tram.lane_z_yd
                    && (tram.wall_y..=tram.lane_north_y).contains(&py);
                if in_lane && !hit.contains(&p.guid) {
                    hit.push(p.guid);
                    hits.push(Hit {
                        player: p.guid,
                        car: car.guid,
                    });
                }
            }
        }
        hits
    }

    /// The cars, the lit track lamps, the Lampmen's lamps and the crates on the Tribute Pile in
    /// view, with their creates: the whole Phase sees them.
    pub fn in_view(&self) -> Vec<(u64, SessionEvent)> {
        let tram = self.tram;
        let still = |guid: u64, entry: u32, display_id: u32, position: [f32; 3]| {
            let create = gameobject_create(&GameObject {
                guid,
                entry,
                display_id,
                type_id: GENERIC,
                flags: NOT_SELECTABLE,
                faction: 0,
                position,
                orientation: 0.0,
            });
            (guid, create)
        };
        let mut seen = self.moving_in_view();
        for l in &self.lamps {
            seen.push(still(
                l.guid,
                tram.lamp_display,
                tram.lamp_display,
                l.position,
            ));
        }
        for (k, _) in self
            .pile
            .iter()
            .enumerate()
            .filter(|(_, &c)| c == Crate::OnPile)
        {
            let guid = tram.crate_guid(k);
            let entry = tram.pile_guid + k as u32;
            seen.push(still(guid, entry, tram.crate_display, tram.pile[k]));
        }
        seen
    }

    /// The cars and lit track lamps in view, with their creates.
    fn moving_in_view(&self) -> Vec<(u64, SessionEvent)> {
        let mut seen: Vec<(u64, SessionEvent)> = self
            .cars
            .iter()
            .map(|c| {
                let (pose, t) = self.pose(c);
                (c.guid, self.car_create(c, pose, t))
            })
            .collect();
        for track in 0..self.tram.track.len() {
            if self.lit.iter().any(|l| l.track == track) {
                seen.push((self.tram.lamp_guid(track), self.tram.lamp_create(track)));
            }
        }
        seen
    }

    /// `car`'s create at `pose`, anchored `t` ms into its period.
    fn car_create(&self, car: &Car, pose: Pose, t: u64) -> SessionEvent {
        let mut create = gameobject_create(&GameObject {
            guid: car.guid,
            entry: self.tram.car_entry,
            display_id: self.tram.car_display,
            type_id: carts::TRANSPORT,
            flags: 0,
            faction: 0,
            position: pose.point,
            orientation: pose.yaw,
        });
        if let SessionEvent::ObjectCreate {
            transport_progress, ..
        } = &mut create
        {
            *transport_progress = Some(t as u32);
        }
        create
    }

    /// The route of the car `guid`, which goes before its create: the client drives the car
    /// from it.
    pub fn route_of(&self, guid: u64) -> Option<SessionEvent> {
        let car = self.cars.iter().find(|c| c.guid == guid)?;
        let tram = self.tram;
        Some(SessionEvent::CartRoute {
            key: tram.track[car.track].key.clone(),
            name: String::new(),
            map: self.map,
            wagon_guid: car.guid,
            horse_guid: 0,
            driver_guid: 0,
            points: car.path.to_vec(),
            speed_yd_s: tram.speed_yd_s,
            dwell_ms: tram.dwell_ms() as u32,
            period_ms: tram.period_ms() as u32,
            stops: Vec::new(),
            backs_out: true,
        })
    }
}

/// Whether `mob` has walked to `point` and stopped there.
fn there(mob: &Mob, point: [f32; 3]) -> bool {
    mob.walk.is_none() && flat_distance(mob.position, point) <= THERE_YD
}

/// Starts `mob` walking to `point` at `speed_pct` of its walk speed, unless it is there or
/// already on its way.
fn walk_toward(mob: &mut Mob, point: [f32; 3], speed_pct: u32, events: &mut Vec<SessionEvent>) {
    let heading_there = mob
        .walk
        .as_ref()
        .is_some_and(|w| w.points.last() == Some(&point));
    if heading_there || there(mob, point) {
        return;
    }
    events.push(mob.walk_path(vec![point], speed_pct));
}

/// The guid of the `n`th thing a line made, as a Lampman's lamp of `display`: a GameObject.
fn mist_lamp_guid(display: u32, n: u32) -> u64 {
    HIGHGUID_GAMEOBJECT << 48 | u64::from(display & 0xFF_FFFF) << 24 | u64::from(n & 0xFF_FFFF)
}

/// The guid of the `n`th car a line made: a `HIGHGUID_TRANSPORT` GameObject of `entry`.
fn car_guid(entry: u32, n: u32) -> u64 {
    u64::from(HIGH_TRANSPORT) << 48 | u64::from(entry & 0xFF_FFFF) << 24 | u64::from(n & 0xFF_FFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Deeprun Tram's `[tram]` table.
    fn tram() -> &'static Tram {
        crate::phased::dungeon(18)
            .and_then(|d| d.tram.as_ref())
            .expect("the Deeprun Tram has a line")
    }

    #[test]
    fn the_line_s_bell_is_a_lever_of_its_dungeon_and_its_hit_is_tram_strike() {
        let d = crate::phased::dungeon(18).unwrap();
        let bell = d.gameobject.iter().find(|o| o.guid == tram().bell);
        assert!(
            bell.is_some_and(|o| o.use_kind == Some(crate::phased::UseKind::Lever)),
            "the bell is a lever"
        );
        let strike = crate::creature_spells::get("tram_strike").expect("Tram Strike");
        assert_eq!(strike.spell_id, tram().hit_spell);
        assert_eq!(tram().track.len(), 2, "tracks a and b");
        assert!(
            tram().max_cars <= tram().stops.len(),
            "a stop for every car"
        );
    }

    #[test]
    fn each_tram_creature_has_its_spawn_row_and_only_the_porter_is_porter_level() {
        let rows = tram().creature_rows();
        let entries: Vec<u32> = rows.iter().map(|r| r.0).collect();
        assert_eq!(entries, [97215, 97207, 97216, 97217]);
        for (entry, levels) in rows {
            let row = crate::mobs::tram_row(18, entry).expect("a spawn row");
            let s = &crate::mobs::content().spawn[row as usize];
            assert_eq!([s.level_min, s.level_max], levels);
            assert!(s.appears.is_none(), "no Run starts with it");
        }
        assert_eq!(tram().creature_rows()[3].1, [38, 38]);
    }

    #[test]
    fn a_car_rolls_in_in_five_seconds_and_its_period_is_fifty() {
        assert_eq!(tram().leg_ms(), 5_000);
        assert_eq!(tram().period_ms(), 50_000);
    }

    #[test]
    fn with_a_tram_creature_alive_at_each_warning_every_tram_is_one_car_and_the_gaps_are_long() {
        let mut line = Line::new(tram(), 369, 0);
        let mut arrivals = Vec::new();
        let mut seen = 0;
        for _ in 0..3000 {
            line.tick(0.1, true);
            let made = line.made as usize;
            if made > seen {
                arrivals.push((line.now_ms + tram().leg_ms(), made - seen));
                seen = made;
            }
        }
        let want: Vec<(u64, usize)> = [10, 50, 90, 130, 170, 210, 250]
            .iter()
            .map(|s| (s * 1000, 1))
            .collect();
        assert_eq!(arrivals, want);
    }
}
