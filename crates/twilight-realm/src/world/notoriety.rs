//! **Notoriety** in the world (`GLOSSARY.md`, spec #175): achievements raise it, the Hub drains
//! it, and the character wears their Heat Level as an aura. Above Heat 0 a **Bounty Hunter**
//! comes after them on the Heat Level's timer, and gives up when they reach the Hub or a Phase.

use benilla_protocol::SessionEvent;

use super::{engage, eye, InWorld};
use crate::bounty::{self, Draw};
use crate::characters::Char;
use crate::creature_spells;
use crate::mobs::{self, Bounty, Mob};
use crate::phased::Phase;
use crate::rng::Rng;
use crate::rules::notoriety;
use crate::rules::stats::MobStats;
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

    /// Whether a living Bounty Hunter of this player's is out (its adds aside).
    fn hunter_out(&self) -> bool {
        let me = self.guid;
        self.mobs
            .iter()
            .any(|m| m.bounty.is_some_and(|b| b.owner == me) && m.alive())
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

    /// A Bounty Hunter comes for `c`: an elite of a rolled archetype, as strong as their Bounty
    /// Rank makes it and with its affixes, out of their sight a little way off, already on their
    /// trail.
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
        let rank = c.bounty_rank;
        let affixes = bounty::roll_affixes(rank, rng);
        let mut stats = draw.stats(level, rank);
        for a in &affixes {
            stats.armor += stats.armor * a.armor_pct / 100;
        }
        let mut hunter = self.made_hunter(draw, stats, at, c.position);
        hunter.bounty = Some(Bounty {
            owner: self.guid,
            rank,
        });
        hunter.attack_time_mul = affixes
            .iter()
            .map(|a| 100.0 / (100.0 + a.attack_speed_pct as f32))
            .product();
        hunter.extra_spells = affixes
            .iter()
            .flat_map(|a| creature_spells::resolve(&a.spells))
            .collect();
        let mut pack = vec![hunter];
        for adds in affixes.iter().filter_map(|a| a.adds) {
            for k in 0..adds.count {
                let mut s = stats;
                s.max_health = (s.max_health * adds.pct / 100).max(1);
                s.min_damage *= adds.pct as f32 / 100.0;
                s.max_damage *= adds.pct as f32 / 100.0;
                // Beside it, a couple of yards to either side.
                let side = if k % 2 == 0 { 1.0 } else { -1.0 } * (2.0 + k as f32);
                let by = [at[0] + side, at[1] + side, at[2]];
                pack.push(self.made_hunter(draw, s, by, c.position));
            }
        }
        for mut mob in pack {
            events.push(mob.create());
            engage(&mut mob, self.guid, events);
            self.mobs.push(mob);
        }
    }

    /// One hunter of `draw` with `stats` at `at`, facing `me`, with a guid of its own.
    fn made_hunter(&mut self, draw: Draw, stats: MobStats, at: [f32; 3], me: [f32; 3]) -> Mob {
        self.hunters_sent = self.hunters_sent.wrapping_add(1);
        let low = HUNTER_GUID_BASE | (self.guid as u32 & 0xFFF) << 8 | self.hunters_sent & 0xFF;
        let facing = (me[1] - at[1]).atan2(me[0] - at[0]);
        Mob::hunter(
            mobs::unit_guid(draw.entry(), low),
            draw.template(),
            stats,
            at,
            facing,
            self.guid,
        )
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
