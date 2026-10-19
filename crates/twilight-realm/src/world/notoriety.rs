//! **Notoriety** in the world (`GLOSSARY.md`, spec #175): achievements raise it, the Hub drains
//! it, and the character wears their Heat Level as an aura. Above Heat 0 a **Bounty Hunter**
//! comes after them on the Heat Level's timer, and gives up when they reach the Hub or a Phase.

use benilla_protocol::messages::GossipPoi;
use benilla_protocol::SessionEvent;

use super::{engage, eye, InWorld};
use crate::bounty::{self, Affix, Draw};
use crate::characters::{Char, Marked};
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

/// How far the Marked victor walks before its map flag moves with it.
const FLAG_MOVES_YD: f32 = 5.0;

/// The map flag's `AreaPOI.dbc` flags and `POIIcons.blp` cell: every vmangos row's, a red flag.
const FLAG_FLAGS: u32 = 99;
const FLAG_ICON: u32 = 6;

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
        self.marked_tick(dt, c, events);
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
        // No new hunter comes while the last one's victory stands.
        if c.marked.is_some() || !open_world || !self.player.alive() || self.hunter_out() {
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
        let (hunter, stats) = self.armed_hunter(draw, level, rank, &affixes, at, c.position);
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

    /// The Bounty Hunter of `draw` at `level` for this player at Bounty Rank `rank`, carrying
    /// `affixes`, at `at` facing `me`; and its stats, which its adds take a share of.
    fn armed_hunter(
        &mut self,
        draw: Draw,
        level: u8,
        rank: u32,
        affixes: &[&'static Affix],
        at: [f32; 3],
        me: [f32; 3],
    ) -> (Mob, MobStats) {
        let mut stats = draw.stats(level, rank);
        for a in affixes {
            stats.armor += stats.armor * a.armor_pct / 100;
        }
        let mut hunter = self.made_hunter(draw, stats, at, me);
        hunter.bounty = Some(Bounty {
            owner: self.guid,
            rank,
            victor: false,
        });
        hunter.attack_time_mul = affixes
            .iter()
            .map(|a| 100.0 / (100.0 + a.attack_speed_pct as f32))
            .product();
        hunter.extra_spells = affixes
            .iter()
            .flat_map(|a| creature_spells::resolve(&a.spells))
            .collect();
        hunter.affixes = affixes.iter().map(|a| a.key.as_str()).collect();
        (hunter, stats)
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
        self.remove_units(&gone, events);
        self.hunter_in = None;
    }

    /// The player's living Bounty Hunter of this session, not yet a victor: its index.
    fn own_hunter(&self) -> Option<usize> {
        let me = self.guid;
        self.mobs
            .iter()
            .position(|m| m.alive() && m.bounty.is_some_and(|b| b.owner == me && !b.victor))
    }

    /// The player died: to their own Bounty Hunter (it or one of its adds landed the last
    /// blow), they are **Marked** and it stays where it won (#179). Marked already, nothing
    /// changes: the penalty does not stack and the clock runs on.
    pub(super) fn marked_by_death(&mut self, c: &mut Char, events: &mut Vec<SessionEvent>) {
        let me = self.guid;
        let Some(killer) = self.last_hit_by.take() else {
            return;
        };
        let by_hunter = self.mobs.iter().any(|m| {
            m.guid == killer
                && (m.hunts == Some(me) || m.bounty.is_some_and(|b| b.owner == me && !b.victor))
        });
        if !by_hunter || c.marked.is_some() || self.own_hunter().is_none() {
            return;
        }
        // Its adds go: the victory is its own.
        let adds: Vec<u64> = self
            .mobs
            .iter()
            .filter(|m| m.hunts == Some(me) && m.bounty.is_none())
            .map(|m| m.guid)
            .collect();
        self.remove_units(&adds, events);
        let Some(i) = self.own_hunter() else {
            return;
        };
        let victor = &mut self.mobs[i];
        victor.settle(notoriety::marked_roam_yd());
        let Some(draw) = Draw::of_entry(victor.template.entry) else {
            return;
        };
        c.marked = Some(Marked {
            secs_left: notoriety::marked_secs(),
            draw: [draw.archetype, draw.first, draw.epithet].map(|n| n as u32),
            level: victor.stats.level,
            rank: victor.bounty.map_or(0, |b| b.rank),
            affixes: victor.affixes.iter().map(|k| k.to_string()).collect(),
            map: self.map,
            home: victor.position,
        });
        self.victor_flag = None;
    }

    /// While **Marked**: the logged-in clock runs down, the penalty and the debuff follow the
    /// mark, and the victor is kept in its zone, flagged on the map. Its end takes the victor.
    fn marked_tick(&mut self, dt: f32, c: &mut Char, events: &mut Vec<SessionEvent>) {
        if let Some(m) = &mut c.marked {
            m.secs_left -= dt;
            if m.secs_left <= 0.0 {
                c.marked = None;
                let me = self.guid;
                let victors: Vec<u64> = self
                    .mobs
                    .iter()
                    .filter(|m| m.bounty.is_some_and(|b| b.owner == me && b.victor))
                    .map(|m| m.guid)
                    .collect();
                self.remove_units(&victors, events);
            }
        }
        let marked = c.marked.is_some();
        if marked != self.marked_applied {
            self.marked_applied = marked;
            self.refresh_gear(c, events);
        }
        let show = marked && self.player.alive();
        if show != self.marked_shown {
            if show {
                self.show_marker(notoriety::marked_aura(), false, events);
            } else {
                self.hide_marker(notoriety::marked_aura(), events);
            }
            self.marked_shown = show;
        }
        if let Some(m) = c.marked.clone() {
            self.bring_victor(&m, c.position, events);
            self.flag_victor(&m, events);
        }
    }

    /// The victor, back at its home spot when the player comes within view of it and it is
    /// missing: after a login, once its corpse is gone, or after the world forgot it idle out of
    /// everyone's view.
    fn bring_victor(&mut self, m: &Marked, me: [f32; 3], events: &mut Vec<SessionEvent>) {
        let mine = self.guid;
        let here = self.lent
            && self.phase == Phase::OpenWorld
            && self.map == m.map
            && crate::space::flat_distance(me, m.home) <= crate::population::VIEW_YD;
        if !here
            || self
                .mobs
                .iter()
                .any(|x| x.bounty.is_some_and(|b| b.owner == mine && b.victor))
        {
            return;
        }
        let [archetype, first, epithet] = m.draw.map(|n| n as usize);
        let draw = Draw {
            archetype,
            first,
            epithet,
        };
        // A save from before the archetypes changed brings nothing back.
        if Draw::of_entry(draw.entry()) != Some(draw) {
            return;
        }
        let affixes: Vec<&'static Affix> =
            m.affixes.iter().filter_map(|k| bounty::affix(k)).collect();
        let (mut victor, _) = self.armed_hunter(draw, m.level, m.rank, &affixes, m.home, me);
        victor.settle(notoriety::marked_roam_yd());
        events.push(victor.create());
        self.mobs.push(victor);
        self.victor_flag = None;
    }

    /// A map flag on the victor (`SMSG_GOSSIP_POI`), put down again once it has walked a way.
    fn flag_victor(&mut self, m: &Marked, events: &mut Vec<SessionEvent>) {
        let me = self.guid;
        if self.map != m.map {
            return;
        }
        let Some(victor) = self
            .mobs
            .iter()
            .find(|x| x.alive() && x.bounty.is_some_and(|b| b.owner == me && b.victor))
        else {
            return;
        };
        let at = [victor.position[0], victor.position[1]];
        if self
            .victor_flag
            .is_some_and(|f| (f[0] - at[0]).hypot(f[1] - at[1]) < FLAG_MOVES_YD)
        {
            return;
        }
        self.victor_flag = Some(at);
        events.push(SessionEvent::GossipPoi(GossipPoi {
            flags: FLAG_FLAGS,
            pos: at,
            icon: FLAG_ICON,
            data: 0,
            name: victor.template.name.to_string(),
        }));
    }

    /// Takes `units` out of the world: their auras with them, the player's swing at one stopped.
    fn remove_units(&mut self, units: &[u64], events: &mut Vec<SessionEvent>) {
        if units.is_empty() {
            return;
        }
        for &guid in units {
            self.auras_die_with(guid, events);
        }
        if self.player.attacking.is_some_and(|t| units.contains(&t)) {
            self.stop_attack(events);
        }
        self.mobs.retain(|m| !units.contains(&m.guid));
        events.push(SessionEvent::ObjectsRemoved(units.to_vec()));
    }
}
