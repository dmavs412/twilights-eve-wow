//! The Deeprun Tram's line (spec #586, "The trams"; ADR 0033): Run state that starts once the
//! station bell is rung. Trams come on a timetable that follows the group's pace (Headway): each
//! tram's cars roll in on one track, stand at the platform and back out, and a moving car hits
//! every living player in its lane. A car is a type 11 transport on the Carts' road drive: the
//! realm sends its `SessionEvent::CartRoute` before its create, and removes it once it has backed
//! out behind the mist wall. Tram code is for the tram only; every number is the dungeon's
//! `[tram]` table in `content/deeprun_tram.toml`.
//!
//! The cars carry nothing yet (#590 loads them): [`Line::bring`] is where a tram's creatures are
//! counted, and the Headway check asks whether any of them is alive.

use benilla_protocol::guid::HIGH_TRANSPORT;
use benilla_protocol::SessionEvent;
use serde::Deserialize;

use crate::carts::{self, CartState, Pose};
use crate::phased::{gameobject_create, GameObject, GENERIC, HIGHGUID_GAMEOBJECT, NOT_SELECTABLE};

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
    /// How many cars the line has made, which numbers their guids: a new tram never reuses an
    /// old car's.
    made: u32,
    /// The creatures the trams brought (#590), which the Headway check counts.
    brought: Vec<u64>,
}

impl Line {
    /// A line on `map` just started: tram 1 arrives `first_s` from now.
    pub fn new(tram: &'static Tram, map: u32) -> Self {
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
            brought: Vec::new(),
        }
    }

    /// The line's numbers.
    pub fn tram(&self) -> &'static Tram {
        self.tram
    }

    /// Counts the creature `guid` as one a tram brought (#590).
    pub fn bring(&mut self, guid: u64) {
        self.brought.push(guid);
    }

    /// Whether the creature `guid` is one a tram brought.
    pub fn brought(&self, guid: u64) -> bool {
        self.brought.contains(&guid)
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

    /// The cars and lit lamps in view, with their creates: the whole Phase sees them.
    pub fn in_view(&self) -> Vec<(u64, SessionEvent)> {
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
    fn a_car_rolls_in_in_five_seconds_and_its_period_is_fifty() {
        assert_eq!(tram().leg_ms(), 5_000);
        assert_eq!(tram().period_ms(), 50_000);
    }

    #[test]
    fn with_a_tram_creature_alive_at_each_warning_every_tram_is_one_car_and_the_gaps_are_long() {
        let mut line = Line::new(tram(), 369);
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
