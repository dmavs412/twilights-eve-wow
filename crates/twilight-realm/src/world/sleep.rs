//! Sleeping creatures: a creature that starts asleep ignores players and stays where it lies
//! until something wakes it, for the rest of its life. Nothing here names a creature; the dungeon
//! file says who sleeps, in which Camp, and how lightly.

use benilla_protocol::SessionEvent;

use super::InWorld;
use crate::fields::{values, UNIT_NPC_EMOTESTATE};
use crate::mobs::{Mob, State};
use crate::phased::Meter;
use crate::rules::aggro;
use crate::space::distance;

impl InWorld {
    /// Wakes the sleepers a character has come near, that were hurt along with their Camp, or
    /// that a waker has passed. Before the mobs notice anyone, so a sleeper woken this tick is
    /// an ordinary mob from now on.
    pub(super) fn wake_sleepers(&mut self, me: [f32; 3], events: &mut Vec<SessionEvent>) {
        if !self.mobs.iter().any(|m| m.asleep) {
            return;
        }
        let mut watchers: Vec<[f32; 3]> = self.others.iter().map(|o| o.1).collect();
        if self.player.alive() {
            watchers.push(me);
        }
        let meter = self.meter();
        wake(&mut self.mobs, &watchers, meter, self.noise_shown, events);
    }
}

/// Wakes each asleep mob of `mobs` that (a) has a watcher within its wake radius at `stacks` of
/// the Run's `meter`, (b) is hurt, or has a Camp mate that is, or (c) has a waking creature
/// within that creature's radius. A woken Camp mate joins a fight its hurt mate is in.
fn wake(
    mobs: &mut [Mob],
    watchers: &[[f32; 3]],
    meter: Option<&Meter>,
    stacks: u32,
    events: &mut Vec<SessionEvent>,
) {
    let mut woken = vec![false; mobs.len()];
    // The Camps with a hurt sleeper, and where its fight is if it is in one.
    let mut hurt: Vec<(u32, Option<[f32; 3]>)> = Vec::new();
    for (i, m) in mobs.iter().enumerate().filter(|(_, m)| m.asleep) {
        // A hit on a sleeper turns it on its attacker (`pull`), so a fight counts as hurt too.
        if m.state != State::Idle || m.health < m.stats.max_health {
            woken[i] = true;
            hurt.push((m.camp, (m.state == State::Engaged).then_some(m.position)));
            continue;
        }
        let near = meter.is_some_and(|meter| {
            let radius = meter.wake_radius(stacks, m.wake_radius_mod);
            watchers.iter().any(|&w| distance(m.position, w) <= radius)
        });
        let passed = mobs.iter().any(|w| {
            !w.asleep
                && w.alive()
                && w.template
                    .wakes_sleepers
                    .is_some_and(|r| distance(w.position, m.position) <= r)
        });
        woken[i] = near || passed;
    }
    for (i, m) in mobs.iter_mut().enumerate() {
        if !m.asleep {
            continue;
        }
        let mates = hurt.iter().find(|(camp, _)| *camp == m.camp);
        if mates.is_some() && !woken[i] {
            woken[i] = true;
            if let Some(&(_, Some(fight))) = mates {
                if m.template.hostile && m.alive() && m.assist_in.is_none() {
                    m.assist_in = Some(aggro::social_delay(distance(m.position, fight)));
                }
            }
        }
        if woken[i] {
            m.asleep = false;
            events.push(values(m.guid, &[(UNIT_NPC_EMOTESTATE, 0)]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mobs::{Spawn, Template};
    use crate::phased::Phase;
    use crate::rules::school::School;
    use crate::rules::stats::UnitClass;
    use crate::shard::{Place, Shard};
    use crate::world::wound;

    fn meter() -> Meter {
        Meter {
            name: "Noise".into(),
            max: 20,
            client_spell: 1,
            clock_stack_every_s: 30.0,
            on_first_damage: 1,
            on_death: 1,
            hide_drain_every_s: 2.0,
            wake_base_yd: 3.0,
            wake_per_stack_yd: 1.5,
        }
    }

    /// The Abbey's creatures, a Run with several mobs in it; all awake, each in a Camp of its own.
    fn creatures() -> Vec<Mob> {
        let shard = Shard::new(Place {
            map: 0,
            phase: Phase::Dungeon(1),
        });
        assert!(shard.mobs.len() >= 3, "the Abbey holds creatures");
        shard.mobs
    }

    fn east_of(m: &Mob, yd: f32) -> [f32; 3] {
        let [x, y, z] = m.position;
        [x + yd, y, z]
    }

    fn woke(mobs: &mut [Mob], watchers: &[[f32; 3]], stacks: u32) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        wake(mobs, watchers, Some(&meter()), stacks, &mut events);
        events
    }

    #[test]
    fn a_sleeper_ten_yards_off_wakes_at_5_stacks_not_at_none() {
        let mut mobs = creatures();
        mobs[0].asleep = true;
        let watcher = east_of(&mobs[0], 10.0);
        woke(&mut mobs, &[watcher], 0);
        assert!(mobs[0].asleep, "3 yd at no stacks");
        woke(&mut mobs, &[watcher], 4);
        assert!(mobs[0].asleep, "9 yd at 4 stacks");
        let events = woke(&mut mobs, &[watcher], 5);
        assert!(!mobs[0].asleep, "10.5 yd at 5 stacks");
        assert_eq!(events.len(), 1, "the lying-down emote is cleared");
    }

    #[test]
    fn wake_radius_mod_shifts_the_threshold() {
        let mut mobs = creatures();
        let watcher = east_of(&mobs[0], 6.0);
        mobs[0].asleep = true;
        woke(&mut mobs, &[watcher], 0);
        assert!(mobs[0].asleep, "6 yd, 3 yd radius");
        mobs[0].wake_radius_mod = 3.0;
        woke(&mut mobs, &[watcher], 0);
        assert!(!mobs[0].asleep, "a light sleeper's 6 yd radius");
    }

    #[test]
    fn a_deep_sleeper_stays_down_where_an_ordinary_one_wakes() {
        let mut mobs = creatures();
        let watcher = east_of(&mobs[0], 2.0);
        mobs[0].asleep = true;
        mobs[0].wake_radius_mod = -2.0;
        woke(&mut mobs, &[watcher], 0);
        assert!(mobs[0].asleep, "2 yd against a 1 yd radius");
        woke(&mut mobs, &[watcher], 1);
        assert!(!mobs[0].asleep, "2.5 yd radius with a stack");
    }

    #[test]
    fn hurting_one_gnoll_wakes_its_camp_and_no_other() {
        let mut mobs = creatures();
        for m in &mut mobs {
            m.asleep = true;
        }
        mobs[0].camp = 7;
        mobs[1].camp = 7;
        mobs[2].camp = 8;
        mobs[0].health -= 1;
        woke(&mut mobs, &[], 0);
        assert!(!mobs[0].asleep && !mobs[1].asleep, "the camp");
        assert!(mobs[2].asleep, "another camp");
        assert!(mobs[3..].iter().all(|m| m.asleep), "the rest");
    }

    #[test]
    fn a_sleeper_is_attackable_and_wakes_on_damage() {
        let mut mobs = creatures();
        mobs[0].asleep = true;
        assert!(mobs[0].attackable());
        wound(
            &mut mobs[0],
            1,
            1,
            School::Physical,
            &mut Vec::new(),
            &mut Vec::new(),
        );
        woke(&mut mobs, &[], 0);
        assert!(!mobs[0].asleep);
    }

    #[test]
    fn a_waker_passing_wakes_the_sleepers_in_its_radius_only() {
        let mut mobs = creatures();
        let mut waker = Template::hunter(
            1,
            "Waker".into(),
            String::new(),
            1,
            1,
            UnitClass::Warrior,
            2000,
            1.0,
            Vec::new(),
            Vec::new(),
        );
        waker.wakes_sleepers = Some(15.0);
        mobs[0].template = Box::leak(Box::new(waker));
        mobs[0].position = [0.0; 3];
        mobs[1].asleep = true;
        mobs[1].position = [10.0, 0.0, 0.0];
        mobs[2].asleep = true;
        mobs[2].position = [20.0, 0.0, 0.0];
        woke(&mut mobs, &[], 0);
        assert!(!mobs[1].asleep, "10 yd from the waker");
        assert!(mobs[2].asleep, "20 yd from the waker");
    }

    #[test]
    fn a_reset_run_puts_a_woken_sleeper_back_to_sleep() {
        let entry = crate::mobs::content().spawn[0].entry;
        let spawn: &'static Spawn = Box::leak(Box::new(Spawn {
            entry,
            asleep: true,
            ..Default::default()
        }));
        let mut mob = Mob::from_spawn(spawn, 0, 1, 0.0).unwrap();
        assert!(mob.asleep);
        mob.asleep = false;
        // A Run's reset builds its creatures from their rows again.
        assert!(Mob::from_spawn(spawn, 0, 1, 0.0).unwrap().asleep);
    }

    #[test]
    fn a_sleeper_shows_the_sleep_emote_until_it_wakes() {
        let mut mobs = creatures();
        let state = |m: &Mob| {
            let SessionEvent::ObjectCreate { fields, .. } = m.create() else {
                panic!("a create");
            };
            let found = fields.raw_fields().find(|&(f, _)| f == UNIT_NPC_EMOTESTATE);
            found
        };
        assert_eq!(state(&mobs[0]), None);
        mobs[0].asleep = true;
        assert_eq!(state(&mobs[0]), Some((UNIT_NPC_EMOTESTATE, 3)));
    }
}
