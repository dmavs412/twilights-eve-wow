//! A creature running from a fight (#472): the first time an engaged creature's health falls to
//! its template's `flee_at_pct`, it runs straight away from its target at run speed, along the
//! ground a feared player's run takes, for at most [`FLEE_YD`] or [`FLEE_SECS`], whichever comes
//! first. It stays engaged on the same target with its threat table as it was, swings and casts
//! nothing, and calls no one. Leaving combat lets it flee again in the next fight. Nothing here
//! names a creature.

use benilla_protocol::SessionEvent;

use super::fear::flee_to;
use super::{drop_cast, InWorld};
use crate::fields::{values, UNIT_FIELD_FLAGS, UNIT_FLAG_FLEEING, UNIT_FLAG_IN_COMBAT};
use crate::mobs::{Flee, Mob, State};
use crate::space::distance;
use crate::Host;

/// The longest a flee runs, in yards and in seconds.
const FLEE_YD: f32 = 15.0;
const FLEE_SECS: f32 = 4.0;
/// Seconds of run in each leg, so a snare that lands mid-flee slows the next one.
const LEG_SECS: f32 = 0.5;
/// A leg shorter than this is no leg: the creature is cornered and the flee ends.
const MIN_LEG_YD: f32 = 0.5;
/// Seconds of float drift a flee's clock forgives.
const CLOCK_EPSILON: f32 = 1e-3;

impl InWorld {
    /// Starts the flee of each engaged creature whose health has just reached its flee percent,
    /// once a fight, and lets a creature out of combat flee again in its next.
    pub(super) fn start_flees(&mut self, events: &mut Vec<SessionEvent>) {
        for mob in &mut self.mobs {
            if mob.state != State::Engaged {
                mob.fled = false;
                mob.fleeing = None;
                continue;
            }
            let Some(pct) = mob.template.flee_at_pct else {
                continue;
            };
            let low = mob.health > 0 && mob.health * 100 <= mob.stats.max_health * pct;
            if mob.fled || mob.remote || mob.template.inert || !low {
                continue;
            }
            mob.fled = true;
            mob.fleeing = Some(Flee {
                secs_left: FLEE_SECS,
                yards_left: FLEE_YD,
            });
            mob.flee_due = mob.template.say.as_ref().is_some_and(|s| s.flee.is_some());
            drop_cast(mob, events);
            events.extend(mob.halt());
            events.push(SessionEvent::AttackStop {
                attacker: mob.guid,
                victim: mob.target,
            });
            let flags = UNIT_FLAG_IN_COMBAT | UNIT_FLAG_FLEEING;
            events.push(values(mob.guid, &[(UNIT_FIELD_FLAGS, flags)]));
        }
    }
}

/// Runs `mob`'s flee on by `dt`, away from its target at `from`: a new leg when the last is done,
/// none while `held` (rooted or stunned), though the clock still runs. When the time or the
/// yards are spent, or a wall corners it, it turns on its target again.
pub(super) fn flee_step(
    mob: &mut Mob,
    from: [f32; 3],
    dt: f32,
    map: u32,
    host: &dyn Host,
    held: bool,
    events: &mut Vec<SessionEvent>,
) {
    let Some(mut flee) = mob.fleeing else {
        return;
    };
    flee.secs_left -= dt;
    if flee.secs_left <= CLOCK_EPSILON || (mob.walk.is_none() && flee.yards_left < MIN_LEG_YD) {
        end_flee(mob, events);
        return;
    }
    if held {
        events.extend(mob.halt());
    } else if mob.walk.is_none() {
        let speed = mob.run_speed();
        let yards = (speed * LEG_SECS)
            .min(flee.yards_left)
            .min(speed * flee.secs_left);
        let away = (mob.position[1] - from[1]).atan2(mob.position[0] - from[0]);
        let end = flee_to(map, host, mob.position, away, yards);
        let d = distance(mob.position, end);
        if d < MIN_LEG_YD {
            end_flee(mob, events);
            return;
        }
        flee.yards_left -= d;
        events.push(mob.run_to(end, end));
    }
    mob.fleeing = Some(flee);
}

/// `mob` stops running and turns on its target again; its chase lays a leg at once.
fn end_flee(mob: &mut Mob, events: &mut Vec<SessionEvent>) {
    mob.fleeing = None;
    mob.repath_timer = 0.0;
    events.extend(mob.halt());
    events.push(SessionEvent::AttackStart {
        attacker: mob.guid,
        victim: mob.target,
    });
    events.push(values(mob.guid, &[(UNIT_FIELD_FLAGS, UNIT_FLAG_IN_COMBAT)]));
}
