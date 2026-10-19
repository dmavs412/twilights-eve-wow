//! **Notoriety** in the world (`GLOSSARY.md`, spec #175): achievements raise it, the Hub drains
//! it, and the character wears their Heat Level as an aura. Above Heat 0 a **Bounty Hunter**
//! comes after them on the Heat Level's timer, and gives up when they reach the Hub or a Phase.

use benilla_protocol::SessionEvent;

use super::{engage, eye, InWorld};
use crate::bounty::{self, Draw};
use crate::characters::Char;
use crate::mobs::{self, Mob};
use crate::phased::Phase;
use crate::rng::Rng;
use crate::rules::notoriety;
use crate::Host;

/// The low guids of Bounty Hunters start here, clear of the spawns' and a call for help's.
const HUNTER_GUID_BASE: u32 = 0xA0_0000;

/// How many spots around the player a hunter's spawn tries for one out of their sight.
const SPAWN_TRIES: u32 = 12;

impl InWorld {
    /// `c` earned `gain` Notoriety.
    pub(crate) fn notorious(&mut self, c: &mut Char, gain: f32) {
        c.notoriety += gain.max(0.0);
    }

    /// The Hub's drain while the player rests there alive and out of combat, the Heat aura
    /// brought in line with the Heat Level, and the Bounty Hunter's timer.
    pub(super) fn notoriety_tick(
        &mut self,
        dt: f32,
        host: &dyn Host,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) {
        let in_hub = notoriety::in_hub(self.map, c.position);
        if self.player.alive() && !self.player.in_combat && in_hub {
            c.notoriety = notoriety::drained(c.notoriety, dt);
        }
        self.show_heat(c, events);
        if in_hub {
            self.recall_hunter(events);
        }
        self.hunter_timer(dt, in_hub, host, c, rng, events);
    }

    /// Puts up the aura of `c`'s Heat Level, taking down the one it replaces.
    fn show_heat(&mut self, c: &Char, events: &mut Vec<SessionEvent>) {
        let level = notoriety::heat_level(c.notoriety);
        if level == self.heat_shown {
            return;
        }
        if let Some(old) = notoriety::heat_aura(self.heat_shown) {
            self.hide_marker(old, events);
        }
        if let Some(new) = notoriety::heat_aura(level) {
            self.show_marker(new, true, events);
        }
        self.heat_shown = level;
    }

    /// Whether a living Bounty Hunter of this player's is out.
    fn hunter_out(&self) -> bool {
        self.mobs
            .iter()
            .any(|m| m.hunts == Some(self.guid) && m.alive())
    }

    /// Counts the Heat Level's time down while the player is alive in the open world with no
    /// hunter out, and sends one when it runs out. The Hub and a Phase stop the clock.
    fn hunter_timer(
        &mut self,
        dt: f32,
        in_hub: bool,
        host: &dyn Host,
        c: &Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) {
        let Some(full) = notoriety::hunter_secs(notoriety::heat_level(c.notoriety)) else {
            self.hunter_in = None;
            return;
        };
        let open_world = self.lent && self.phase == Phase::OpenWorld && !in_hub;
        if !open_world || !self.player.alive() || self.hunter_out() {
            return;
        }
        // A higher Heat Level's shorter wait cuts one already running.
        let left = self.hunter_in.get_or_insert(full);
        *left = left.min(full) - dt;
        if *left > 0.0 {
            return;
        }
        self.hunter_in = None;
        self.send_hunter(host, c, rng, events);
    }

    /// A Bounty Hunter comes for `c`: an elite of a rolled archetype, out of their sight a little
    /// way off, already on their trail.
    fn send_hunter(
        &mut self,
        host: &dyn Host,
        c: &Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) {
        let Some(at) = self.hunter_spot(host, c.position, rng) else {
            return;
        };
        let draw = Draw::roll(rng);
        let level = c.level.saturating_add(bounty::hunter().level_above);
        self.hunters_sent = self.hunters_sent.wrapping_add(1);
        let low = HUNTER_GUID_BASE | (self.guid as u32 & 0xFFF) << 8 | self.hunters_sent & 0xFF;
        let me = c.position;
        let facing = (me[1] - at[1]).atan2(me[0] - at[0]);
        let mut mob = Mob::hunter(
            mobs::unit_guid(draw.entry(), low),
            draw.template(),
            draw.stats(level),
            at,
            facing,
            self.guid,
        );
        events.push(mob.create());
        engage(&mut mob, self.guid, events);
        self.mobs.push(mob);
    }

    /// Where a hunter comes in: on the ground within its spawn range of `me`, never in the Hub,
    /// and out of their sight when any such spot is.
    fn hunter_spot(&self, host: &dyn Host, me: [f32; 3], rng: &mut Rng) -> Option<[f32; 3]> {
        let (near, far) = bounty::hunter().spawn_yd;
        let mut spot = None;
        for _ in 0..SPAWN_TRIES {
            let angle = rng.frand(0.0, std::f32::consts::TAU);
            let r = rng.frand(near, far);
            let (x, y) = (me[0] + r * angle.cos(), me[1] + r * angle.sin());
            let z = host
                .ground_height(self.map, x, y, me[2] + 2.0)
                .unwrap_or(me[2]);
            let at = [x, y, z];
            if notoriety::in_hub(self.map, at) {
                continue;
            }
            let hidden = !host.line_of_sight(self.map, eye(at), eye(me));
            if hidden || spot.is_none() {
                spot = Some(at);
            }
            if hidden {
                break;
            }
        }
        spot
    }

    /// The player's living Bounty Hunter gives up and leaves, and the timer starts over.
    pub(super) fn recall_hunter(&mut self, events: &mut Vec<SessionEvent>) {
        let me = self.guid;
        let gone: Vec<u64> = self
            .mobs
            .iter()
            .filter(|m| m.hunts == Some(me) && m.alive())
            .map(|m| m.guid)
            .collect();
        if gone.is_empty() {
            return;
        }
        for &guid in &gone {
            self.auras_die_with(guid, events);
        }
        if self.player.attacking.is_some_and(|t| gone.contains(&t)) {
            self.stop_attack(events);
        }
        self.mobs.retain(|m| !gone.contains(&m.guid));
        events.push(SessionEvent::ObjectsRemoved(gone));
        self.hunter_in = None;
    }
}
