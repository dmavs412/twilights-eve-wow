//! A feared player (#239): the client's control is taken, `UNIT_FLAG_FLEEING` is set, and the
//! realm walks the player away from the caster at run speed, turning at random every second or
//! two and stopping short of walls and drops, as vmangos's fleeing movement does. Damage does not
//! break it. The player can neither cast nor swing until it ends.

use benilla_protocol::messages::MonsterMoveFacing;
use benilla_protocol::SessionEvent;

use super::{eye, InWorld};
use crate::characters::Char;
use crate::clock;
use crate::fields::UNIT_FLAG_PVP_ATTACKABLE;
use crate::fields::{values, UNIT_FIELD_FLAGS, UNIT_FLAG_FLEEING, UNIT_FLAG_IN_COMBAT};
use crate::rng::Rng;
use crate::space::distance;
use crate::Host;

/// Seconds of float drift a fear's clock forgives.
const CLOCK_EPSILON: f32 = 1e-3;
/// The shortest and longest straight run before the player turns, in seconds.
const RUN_SECS: (f32, f32) = (1.0, 2.0);
/// The most a turn swings the heading from straight away from the caster, in radians.
const SWERVE: f32 = std::f32::consts::FRAC_PI_3;
/// Yards between ground and sight checks along a run.
const STEP_YD: f32 = 1.0;
/// The most the ground may rise or fall in one step before the run stops there.
const MAX_STEP_HEIGHT: f32 = 2.0;
/// A run shorter than this is no run: the player turns again.
const MIN_RUN_YD: f32 = 0.5;

/// The fear a player runs under.
pub(super) struct Fear {
    /// Seconds of fleeing left.
    left: f32,
    /// Where the caster stood, which the player runs from.
    from: [f32; 3],
    /// The straight run under way.
    walk: Option<Walk>,
    /// The id of the last spline sent.
    splines: u32,
}

/// One straight run, from `start` to `end` over `secs`, `gone` of them done.
struct Walk {
    start: [f32; 3],
    end: [f32; 3],
    secs: f32,
    gone: f32,
}

/// `UNIT_FIELD_FLAGS` of a player.
pub(super) fn unit_flags(in_combat: bool, fleeing: bool) -> u32 {
    let combat = if in_combat { UNIT_FLAG_IN_COMBAT } else { 0 };
    let fear = if fleeing { UNIT_FLAG_FLEEING } else { 0 };
    UNIT_FLAG_PVP_ATTACKABLE | combat | fear
}

impl InWorld {
    /// The player flees from `from` for `duration_ms`: whatever they were casting or swinging at
    /// stops, and a fear already running is lengthened, never shortened.
    pub(super) fn start_fear(
        &mut self,
        from: [f32; 3],
        duration_ms: u32,
        events: &mut Vec<SessionEvent>,
    ) {
        let secs = clock::secs(duration_ms);
        if let Some(fear) = &mut self.fear {
            fear.left = fear.left.max(secs);
            return;
        }
        self.interrupt_cast(events);
        self.end_channel(true, events);
        self.stop_shooting(events);
        self.stop_attack(events);
        self.fear = Some(Fear {
            left: secs,
            from,
            walk: None,
            splines: 0,
        });
        events.push(SessionEvent::ClientControl {
            mover: self.guid,
            allow_move: false,
        });
        self.send_flags(events);
    }

    /// Control goes back to the client.
    fn end_fear(&mut self, events: &mut Vec<SessionEvent>) {
        self.fear = None;
        events.push(SessionEvent::ClientControl {
            mover: self.guid,
            allow_move: true,
        });
        self.send_flags(events);
    }

    fn send_flags(&self, events: &mut Vec<SessionEvent>) {
        let flags = unit_flags(self.player.in_combat, self.fear.is_some());
        events.push(values(self.guid, &[(UNIT_FIELD_FLAGS, flags)]));
    }

    /// Advances a fear by `dt`: the run under way goes on, and when it is done the player turns
    /// and starts another. The fear ends when its time is up or the player dies.
    pub(super) fn fear_tick(
        &mut self,
        dt: f32,
        host: &dyn Host,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) {
        let Some(mut fear) = self.fear.take() else {
            return;
        };
        fear.left -= dt;
        if let Some(w) = &mut fear.walk {
            w.gone = (w.gone + dt).min(w.secs);
            let t = if w.secs > 0.0 { w.gone / w.secs } else { 1.0 };
            c.position = std::array::from_fn(|i| w.start[i] + (w.end[i] - w.start[i]) * t);
            if w.gone >= w.secs - CLOCK_EPSILON {
                fear.walk = None;
            }
        }
        if fear.left <= CLOCK_EPSILON || !self.player.alive() {
            self.end_fear(events);
            return;
        }
        if fear.walk.is_none() {
            let speed = self.player_speeds().run * self.player_speed_mul();
            let (lo, hi) = RUN_SECS;
            let secs = rng.frand(lo, hi).min(fear.left);
            let away = (c.position[1] - fear.from[1]).atan2(c.position[0] - fear.from[0]);
            let heading = away + rng.frand(-SWERVE, SWERVE);
            let end = flee_to(self.map, host, c.position, heading, speed * secs);
            let yards = distance(c.position, end);
            if yards >= MIN_RUN_YD {
                let secs = yards / speed;
                fear.splines += 1;
                events.push(SessionEvent::MonsterMove {
                    guid: self.guid,
                    transport: None,
                    start: c.position,
                    spline_id: fear.splines,
                    path: vec![c.position, end],
                    facing: MonsterMoveFacing::None,
                    stop: false,
                    duration_ms: clock::ms(secs),
                    flying: false,
                    run_mode: true,
                });
                c.orientation = heading.rem_euclid(std::f32::consts::TAU);
                fear.walk = Some(Walk {
                    start: c.position,
                    end,
                    secs,
                    gone: 0.0,
                });
            }
        }
        self.fear = Some(fear);
    }
}

/// Where a run of up to `yards` from `start` along `heading` ends: on the ground, short of any
/// wall, and short of a drop or a rise too sharp to walk. Where the host has no ground loaded the
/// run keeps its height.
pub(super) fn flee_to(
    map: u32,
    host: &dyn Host,
    start: [f32; 3],
    heading: f32,
    yards: f32,
) -> [f32; 3] {
    let (dx, dy) = (heading.cos(), heading.sin());
    let mut at = start;
    let mut gone = 0.0;
    while gone < yards {
        let step = STEP_YD.min(yards - gone);
        let (x, y) = (at[0] + dx * step, at[1] + dy * step);
        let z = match host.ground_height(map, x, y, at[2] + MAX_STEP_HEIGHT) {
            Some(z) if (z - at[2]).abs() <= MAX_STEP_HEIGHT => z,
            Some(_) => break,
            None => at[2],
        };
        let next = [x, y, z];
        if !host.line_of_sight(map, eye(at), eye(next)) {
            break;
        }
        at = next;
        gone += step;
    }
    at
}
