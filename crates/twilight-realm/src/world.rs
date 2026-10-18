//! The character in the world: the login burst that puts it there, the mobs around it, and the
//! fights between them, one tick at a time.

mod advance;
mod call_for_help;
mod cast;
mod creature_cast;
mod hearth;
mod power;

use benilla_protocol::messages::{AttackSwingError, LevelUpInfo, ObjectType, XpGain};
use benilla_protocol::{EntityKind, MoveSpeeds, ObjectFields, SessionEvent};

use crate::characters::Char;
use crate::clock;
use crate::combat;
use crate::content;
use crate::dev::DevMultipliers;
use crate::equipment;
use crate::fields::*;
use crate::looting::{self, Looting};
use crate::mobs::{Mob, State, COMBAT_REACH};
use crate::rng::Rng;
use crate::rules::melee::Outcome;
use crate::rules::power::{self as power_rules, PowerKind};
use crate::rules::stats::CharacterStats;
use crate::rules::{aggro, leash, rage, xp};
use crate::space::distance;
use crate::Host;
use power::Power;

mod death;
mod discovery;
mod dungeon;
mod food;
mod phase;
mod procs;
mod regen;
mod rest;

pub(crate) use dungeon::come_out;

/// vanilla's player speeds (`baseMoveSpeed`): yd/s, turn in rad/s.
const PLAYER_SPEEDS: MoveSpeeds = MoveSpeeds {
    walk: 2.5,
    run: 7.0,
    run_back: 4.5,
    swim: 4.722_222,
    swim_back: 2.5,
    turn_rate: std::f32::consts::PI,
};

/// The game time the sky is held at (hours, minutes): evening twilight, after the 21:00 sunset.
const DUSK: (u8, u8) = (21, 15);

/// A player's combat reach (`DEFAULT_COMBAT_REACH`).
const PLAYER_COMBAT_REACH: f32 = 1.5;

/// How far from the player a chasing mob stops.
const CHASE_DISTANCE: f32 = 2.0;
/// How far the player may move from where a chase leg was aimed before the mob lays a new one.
const CHASE_SLACK: f32 = 1.0;
/// The shortest time between two chase legs.
const REPATH_SECS: f32 = 0.25;

// `PLAYER_SKILL_INFO` skill lines (`SkillLine.dbc`).
const SKILL_DEFENSE: u32 = 95;
const SKILL_UNARMED: u32 = 162;

/// What a tick did that the realm acts on.
#[derive(Default)]
pub(crate) struct Ticked {
    /// The character gained a level: save it.
    pub(crate) leveled: bool,
    /// The character discovered an area: save it.
    pub(crate) discovered: bool,
}

/// The player's side of a fight.
struct Player {
    stats: CharacterStats,
    health: u32,
    /// The Calling's Class's bar: rage, energy or mana.
    power: Power,
    /// Seconds until the next swing; 0 is ready.
    swing_timer: f32,
    /// [`Self::swing_timer`] for the off-hand weapon.
    off_swing_timer: f32,
    /// Who auto-attack swings at, from `CMSG_ATTACKSWING` to `CMSG_ATTACKSTOP`.
    attacking: Option<u64>,
    /// The last swing refusal sent, so it is sent once and not every tick.
    refused: Option<AttackSwingError>,
    /// Seconds into the current health-regeneration tick.
    regen: f32,
    in_combat: bool,
    /// Killed, and the toll taken: lying dead until Release Spirit.
    dead: bool,
}

impl Player {
    fn alive(&self) -> bool {
        self.health > 0
    }

    /// New stats (a level, a piece of gear): the power's max follows them.
    fn set_stats(&mut self, stats: CharacterStats) {
        self.stats = stats;
        self.power.resize(&stats);
    }
}

/// The session of the character that is in the world.
pub(crate) struct InWorld {
    pub(crate) guid: u64,
    /// Seconds since the last autosave.
    pub(crate) since_save: f32,
    map: u32,
    player: Player,
    /// The mobs near the player. In the open world they are the [`crate::shard::Shard`]'s, lent
    /// while this session is served; in a Dungeon or a Phased Dungeon's Run, the player's own.
    pub(crate) mobs: Vec<Mob>,
    /// Whether `mobs` is the shard's, lent.
    pub(crate) lent: bool,
    /// The shard's mobs, handed back when the player left the open world while they were lent.
    pub(crate) handed_back: Option<Vec<Mob>>,
    /// The mobs and Hub NPCs whose create the client has been sent, in the open world.
    pub(crate) seen: std::collections::HashSet<u64>,
    /// The lent mobs whose fight this session runs this tick; every other is another
    /// session's to run.
    pub(crate) runs: std::collections::HashSet<u64>,
    /// The open-world spawns that died since the shard last heard: `(spawn, respawn wait)`.
    pub(crate) died: Vec<(u32, f32)>,
    dev: DevMultipliers,
    /// The corpses with loot on them.
    pub(crate) looting: Looting,
    /// Cooldowns, the queued strike and the auras the skills leave.
    skills: cast::Skills,
    /// The teleport the client has yet to acknowledge.
    teleports: hearth::Teleports,
    /// This session's sales, on the merchant window's buyback tab.
    pub(crate) buyback: crate::vendor::Buyback,
    /// The dice for gear wear, apart from the fight's so that wearing changes no fight.
    wear: Rng,
    /// Where Release Spirit brings the player back: the Hub, or a Dungeon's entrance.
    pub(crate) respawn: death::Respawn,
    /// Food or drink being taken, sitting down.
    meal: Option<food::Meal>,
    /// A worldport waits for the client's ack.
    transfer: bool,
    /// Standing in an inn: `PLAYER_FLAGS_RESTING` is up and the pool fills.
    resting: bool,
    /// The open world, or the Phased Dungeon the player stands in.
    phase: crate::phased::Phase,
    /// That dungeon's Run.
    run: Option<crate::phased::Run>,
    /// The Phased Dungeons whose doorway portal the client has.
    portals: Vec<u32>,
    /// What each creature casts.
    pub(crate) casters: crate::creature_spells::Casters,
}

impl InWorld {
    /// Logs `c` in: what vmangos sends from `CMSG_PLAYER_LOGIN` to the player's own create.
    pub(crate) fn enter(
        c: &Char,
        dev: DevMultipliers,
        wall: f64,
        events: &mut Vec<SessionEvent>,
    ) -> Self {
        let stats = crate::talents::stats(c);
        events.push(SessionEvent::Connected {
            self_guid: c.guid,
            name: c.name.clone(),
            billing_time_rested: 0,
            // Every tutorial already seen: a Local Realm has no new-player script to follow.
            tutorial_flags: Some(vec![0xff; 32]),
            // A reply that hid nothing.
            addon_info: Some(Vec::new()),
        });
        events.push(SessionEvent::Worldport {
            map_id: c.map,
            position: c.position,
            orientation: c.orientation,
            needs_ack: false,
        });
        events.push(c.bind.update());
        // The dusk clock: evening twilight, just after the 21:00 sunset, and a timescale of zero
        // so the client never advances it. The sky over the world stays at dusk.
        events.push(SessionEvent::TimeSpeed {
            hours: DUSK.0,
            minutes: DUSK.1,
            day_serial: 0,
            timescale: 0.0,
        });
        events.push(cast::spell_book(c, wall));
        events.extend(crate::talents::spell_mods(c));
        events.push(cast::action_buttons(c));
        // The proficiencies and the carried items' objects, ahead of the player that names them.
        equipment::login(c, events);
        let w = Self {
            guid: c.guid,
            since_save: 0.0,
            map: c.map,
            player: Player {
                stats,
                health: stats.max_health,
                power: Power::fresh(&stats),
                swing_timer: 0.0,
                off_swing_timer: 0.0,
                attacking: None,
                refused: None,
                regen: 0.0,
                in_combat: false,
                dead: false,
            },
            mobs: Vec::new(),
            lent: false,
            handed_back: None,
            seen: Default::default(),
            runs: Default::default(),
            died: Vec::new(),
            dev,
            looting: Looting::default(),
            skills: cast::Skills::new(c.guid),
            teleports: hearth::Teleports::default(),
            buyback: crate::vendor::Buyback::default(),
            wear: Rng::new(c.guid ^ 0xD0_4AB1_1170),
            respawn: death::Respawn::home(c.map),
            meal: None,
            transfer: false,
            resting: crate::rest::in_inn(c.map, c.position),
            phase: Default::default(),
            run: None,
            portals: Vec::new(),
            casters: Default::default(),
        };
        events.push(w.player_create(c));
        w
    }

    /// The player's movement speeds: the dev multiplier's, and the run speed talents add.
    pub(crate) fn player_speeds(&self) -> MoveSpeeds {
        let mut speeds = self.dev.speeds(PLAYER_SPEEDS);
        speeds.run *= 1.0 + self.player.stats.run_speed_pct as f32 / 100.0;
        speeds
    }

    /// The player's own create, as it stands.
    fn player_create(&self, c: &Char) -> SessionEvent {
        SessionEvent::ObjectCreate {
            guid: c.guid,
            kind: EntityKind::Player,
            display_id: Some(content::display_id(c.race, c.gender)),
            position: c.position,
            orientation: c.orientation,
            scale: 1.0,
            speeds: Some(self.player_speeds()),
            mover: None,
            transport_progress: None,
            transport: None,
            spline: None,
            fields: player_fields(c, &self.player.stats, &self.player.power),
        }
    }

    /// The entry of the living friendly creature `guid` in the player's view (their map and
    /// Phase) and within talking reach of `at`: a Field Giver the player can speak to.
    pub(crate) fn friendly_within_reach(&self, guid: u64, at: [f32; 3]) -> Option<u32> {
        self.mobs
            .iter()
            .find(|m| m.guid == guid && m.alive() && m.template.friendly)
            .filter(|m| crate::space::within_interaction(m.position, at))
            .map(|m| m.template.entry)
    }

    /// `CMSG_SET_SELECTION`: the player's target, which the client shows in `UNIT_FIELD_TARGET`.
    pub(crate) fn select(&mut self, target: u64, events: &mut Vec<SessionEvent>) {
        events.push(values(self.guid, &guid_pairs(UNIT_FIELD_TARGET, target)));
    }

    /// `CMSG_ATTACKSWING`: start auto-attack on `target`, echoed as `SMSG_ATTACKSTART`.
    pub(crate) fn attack(&mut self, target: u64, events: &mut Vec<SessionEvent>) {
        let Some(i) = self
            .mobs
            .iter()
            .position(|m| m.guid == target && m.attackable())
        else {
            events.push(SessionEvent::AttackSwingError(
                AttackSwingError::DeadOrUnattackable,
            ));
            return;
        };
        if !self.player.alive() {
            return;
        }
        if let Some(old) = self.player.attacking.replace(target) {
            if old != target {
                events.push(SessionEvent::AttackStop {
                    attacker: self.guid,
                    victim: old,
                });
            }
        }
        self.player.refused = None;
        events.push(SessionEvent::AttackStart {
            attacker: self.guid,
            victim: target,
        });
        // Being attacked turns a mob on its attacker (`CreatureAI::AttackedBy`).
        if self.mobs[i].state == State::Idle {
            self.pull(i, events);
        }
    }

    /// `CMSG_ATTACKSTOP`.
    pub(crate) fn stop_attack(&mut self, events: &mut Vec<SessionEvent>) {
        if let Some(victim) = self.player.attacking.take() {
            events.push(SessionEvent::AttackStop {
                attacker: self.guid,
                victim,
            });
        }
    }

    /// The map of the shard this player shares, when they stand in the open world: the realm lends
    /// it its mobs and sends it what comes into view.
    pub(crate) fn shard_map(&self) -> Option<u32> {
        (!self.transfer
            && content::open_world(self.map)
            && self.phase == crate::phased::Phase::OpenWorld)
            .then_some(self.map)
    }

    /// Whether this session runs mob `guid`'s fight and AI: every mob of its own, and the lent
    /// ones the realm gave it this tick.
    pub(crate) fn runs(&self, guid: u64) -> bool {
        !self.lent || self.runs.contains(&guid)
    }

    /// The player leaves the mobs where they are: the client is told they are gone, the shard's
    /// go back to it untouched, and the player's own are dropped.
    pub(crate) fn drop_view(&mut self, events: &mut Vec<SessionEvent>) {
        let units: Vec<u64> = if self.lent {
            self.seen.drain().collect()
        } else {
            self.mobs.iter().map(|m| m.guid).collect()
        };
        if !units.is_empty() {
            events.push(SessionEvent::ObjectsRemoved(units));
        }
        if self.lent {
            self.handed_back = Some(std::mem::take(&mut self.mobs));
            self.lent = false;
        }
        self.mobs.clear();
        self.seen.clear();
    }

    /// Advances the world by `dt` seconds around `c`, to the wall clock's `wall`.
    pub(crate) fn tick(
        &mut self,
        dt: f32,
        wall: f64,
        host: &dyn Host,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) -> Ticked {
        let mut ticked = Ticked::default();
        if self.transfer {
            return ticked;
        }
        self.land_blink(host, c, events);
        let me = c.position;
        self.cross_phase(c, events);
        self.show_portals(me, events);
        if let Some(run) = &mut self.run {
            run.elapsed += dt;
        }
        if self.player.alive() {
            self.discover(host, c, events, &mut ticked);
        }
        self.rest(dt, c, events);
        // A corpse that respawned or streamed out takes its loot with it.
        let mobs = &self.mobs;
        self.looting
            .retain(|corpse| mobs.iter().any(|m| m.guid == corpse && !m.alive()));
        let p = &mut self.player;
        p.swing_timer = (p.swing_timer - dt).max(0.0);
        p.off_swing_timer = (p.off_swing_timer - dt).max(0.0);

        // Hostile mobs notice a living player who comes too close, and bring their camp.
        if p.alive() {
            for i in 0..self.mobs.len() {
                let mob = &self.mobs[i];
                if mob.state != State::Idle || !mob.template.hostile {
                    continue;
                }
                let r = self.notice_radius(mob, c.level);
                if distance(mob.position, me) <= r
                    && host.line_of_sight(self.map, eye(mob.position), eye(me))
                {
                    self.pull(i, events);
                }
            }
        }

        self.dazed_wake(c, host, events);
        self.player_swing(c, rng, events, &mut ticked);
        for i in 0..self.mobs.len() {
            if self.runs(self.mobs[i].guid) {
                self.mob_tick(i, dt, host, c, rng, events, &mut ticked);
            }
        }
        self.skills_tick(dt, wall, c, rng, events, &mut ticked);
        // A next-cast buff that ran out puts the client's cast bar back.
        self.sync_next_cast_mods(c, events);
        self.call_for_help(events);
        self.update_aura_states(events);
        if !self.player.alive() && !self.player.dead {
            self.die(c, events);
        }
        self.update_combat(dt, events);
        self.eat(dt, c, events);
        self.regenerate(dt, c, events);
        ticked
    }

    /// How close the player may come before an idle `mob` notices them: a stealthed player is
    /// noticed only inside the short stealth range.
    fn notice_radius(&self, mob: &Mob, player_level: u8) -> f32 {
        let r = aggro::radius(mob.template.detection_range, player_level, mob.level());
        if self.skills.stealthed() {
            r.min(aggro::stealth_radius(player_level, mob.level()))
        } else {
            r
        }
    }

    /// A dazed mob whose hold is over: it notices the player as any idle mob does, or goes home.
    fn dazed_wake(&mut self, c: &Char, host: &dyn Host, events: &mut Vec<SessionEvent>) {
        let me = c.position;
        let route = crate::dungeon::route_on(self.map);
        for i in 0..self.mobs.len() {
            let mob = &self.mobs[i];
            if mob.state != State::Dazed || self.skills.stunned(mob.guid) || !self.runs(mob.guid) {
                continue;
            }
            let r = self.notice_radius(mob, c.level);
            let notices = mob.template.hostile
                && distance(mob.position, me) <= r
                && host.line_of_sight(self.map, eye(mob.position), eye(me));
            if notices {
                self.pull(i, events);
            } else {
                let map = self.map;
                evade(
                    &mut self.mobs[i],
                    self.guid,
                    route.map(|r| (r, map, host)),
                    events,
                );
            }
        }
    }

    /// Mob `i` turns on the player. A hostile mob calls the idle campmates within earshot, each
    /// of whom joins after a short delay (social aggro); a neutral one comes alone. The call
    /// goes one hop: a mob that joins calls no one.
    fn pull(&mut self, i: usize, events: &mut Vec<SessionEvent>) {
        let camp = self.mobs[i].camp;
        let at = self.mobs[i].position;
        engage(&mut self.mobs[i], self.guid, events);
        if !self.mobs[i].template.hostile {
            return;
        }
        for mob in &mut self.mobs {
            let d = distance(mob.position, at);
            if mob.camp == camp
                && mob.state == State::Idle
                && mob.assist_in.is_none()
                && d <= aggro::SOCIAL_RANGE_YD
            {
                mob.assist_in = Some(aggro::social_delay(d));
            }
        }
    }

    /// Everyone in `camp` that is fighting runs home together.
    fn evade_camp(
        &mut self,
        camp: u32,
        route: Option<(&crate::dungeon::Route, u32, &dyn Host)>,
        events: &mut Vec<SessionEvent>,
    ) {
        for mob in &mut self.mobs {
            if mob.camp == camp && mob.state == State::Engaged {
                evade(mob, self.guid, route, events);
            }
        }
    }

    /// The player's auto-attack, when its timer is up and the target is in reach and in front.
    fn player_swing(
        &mut self,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let p = &mut self.player;
        let Some(target) = p.attacking else {
            return;
        };
        if !p.alive() {
            return;
        }
        // The off hand swings on its own timer, at the main hand's target and under its reach and
        // facing, without a refusal of its own.
        if p.off_swing_timer <= 0.0 && p.stats.off_hand.is_some() {
            self.off_hand_swing(target, c, rng, events, ticked);
        }
        let p = &mut self.player;
        if p.swing_timer > 0.0 || !p.alive() {
            return;
        }
        let Some(i) = self.mobs.iter().position(|m| m.guid == target && m.alive()) else {
            p.attacking = None;
            return;
        };
        let mob = &self.mobs[i];
        let refusal = if distance(mob.position, c.position) > melee_reach() {
            Some(AttackSwingError::NotInRange)
        } else if !in_front(c.orientation, c.position, mob.position) {
            Some(AttackSwingError::BadFacing)
        } else {
            None
        };
        if let Some(refusal) = refusal {
            if p.refused != Some(refusal) {
                p.refused = Some(refusal);
                events.push(SessionEvent::AttackSwingError(refusal));
            }
            return;
        }
        p.refused = None;
        p.swing_timer = clock::secs(cast::white(&p.stats, &self.skills).attack_time_ms);

        let from_front = in_front(mob.orientation, mob.position, c.position);
        // A queued Heroic Strike takes this swing's place.
        if self.next_swing_strike(target, from_front, c, rng, events, ticked) {
            return;
        }
        self.white_swing(i, from_front, true, false, c, rng, events, ticked);
    }

    /// The off-hand weapon's swing at `target`, when it is alive, in reach and in front; the
    /// off hand's timer restarts from the weapon's own speed.
    fn off_hand_swing(
        &mut self,
        target: u64,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let Some(i) = self.mobs.iter().position(|m| m.guid == target && m.alive()) else {
            return;
        };
        let mob = &self.mobs[i];
        if distance(mob.position, c.position) > melee_reach()
            || !in_front(c.orientation, c.position, mob.position)
        {
            return;
        }
        let Some(stats) = cast::off_hand_white(&self.player.stats, &self.skills) else {
            return;
        };
        self.player.off_swing_timer = clock::secs(stats.attack_time_ms);
        let from_front = in_front(mob.orientation, mob.position, c.position);
        self.white_swing(i, from_front, false, true, c, rng, events, ticked);
    }

    /// One white swing at mob `i`, and what its landing sets off; `may_swing_again`: whether a
    /// talent may give it one more swing (an extra swing never gives another); `off_hand`: with
    /// the off-hand weapon.
    #[allow(clippy::too_many_arguments)]
    fn white_swing(
        &mut self,
        i: usize,
        from_front: bool,
        may_swing_again: bool,
        off_hand: bool,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let target = self.mobs[i].guid;
        let stats = if off_hand {
            cast::off_hand_white(&self.player.stats, &self.skills)
                .unwrap_or_else(|| cast::white(&self.player.stats, &self.skills))
        } else {
            cast::white(&self.player.stats, &self.skills)
        };
        let opener = self.skills.break_stealth(events) == Some(true);
        let mut swing = combat::player_swing(&stats, &self.mobs[i], from_front, opener, rng);
        swing.off_hand = off_hand;
        swing.damage = self.dealt(c, swing.damage);
        // The next-swing buffs count main-hand swings only.
        if !off_hand {
            let counted = self.skills.count_swing(events);
            self.faded(&counted, events);
        }
        events.push(SessionEvent::AttackerState(swing.report(self.guid, target)));
        if swing.outcome == Outcome::Dodge {
            self.skills.dodged_by(target, events);
        }

        // Rage from what landed, or from three quarters of what a dodge or parry stopped.
        let p = &mut self.player;
        let earned = match swing.outcome {
            Outcome::Dodge | Outcome::Parry => rage::from_avoided(swing.would_be, p.stats.level),
            _ => rage::from_damage_dealt(swing.damage, p.stats.level),
        };
        if p.power.gain_stored(PowerKind::Rage, earned) {
            events.push(values(self.guid, &[p.power.field()]));
        }

        let mob = &mut self.mobs[i];
        mob.health = mob.health.saturating_sub(swing.damage);
        if swing.damage > 0 {
            events.push(values(mob.guid, &[(UNIT_FIELD_HEALTH, mob.health)]));
        }
        let idle = matches!(mob.state, State::Idle | State::Dazed);
        if swing.damage > 0 && crate::wear::on_hit(c, &mut self.wear, events) {
            self.refresh_gear(c, events);
        }
        if idle {
            self.pull(i, events);
        }
        if self.mobs[i].health == 0 {
            self.kill(i, c, rng, events, ticked);
        } else {
            if swing.damage > 0 {
                self.woken_by_damage(target, events);
            }
            if swing.outcome == Outcome::Crit {
                self.crit_landed(target, swing.damage, c, events);
            }
        }
        if swing.damage > 0 {
            self.sweep(target, swing.damage, c, rng, events, ticked);
            self.seal_swing_landed(target, c, rng, events, ticked);
            self.melee_hit(target, c, rng, events, ticked);
            self.white_swing_landed(i, from_front, may_swing_again, c, rng, events, ticked);
        }
    }

    /// What the player's talents make of a white swing that landed on mob `i`: rage, a stun, one
    /// more swing.
    #[allow(clippy::too_many_arguments)]
    fn white_swing_landed(
        &mut self,
        i: usize,
        from_front: bool,
        may_swing_again: bool,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let on_hit = crate::talents::on_hit(c);
        if let Some((spell_id, pct)) = on_hit.rage {
            if rng.below(100) < pct {
                self.gain_rage(spell_id, 1, events);
            }
        }
        if let Some((aura_spell_id, duration_ms, pct)) = on_hit.stun {
            if self.mobs[i].alive() && rng.below(100) < pct {
                self.stun(
                    self.mobs[i].guid,
                    aura_spell_id,
                    duration_ms,
                    c.level,
                    events,
                );
            }
        }
        if may_swing_again
            && on_hit.extra_swing_pct > 0
            && rng.below(100) < on_hit.extra_swing_pct
            && self.mobs[i].alive()
        {
            self.white_swing(i, from_front, false, false, c, rng, events, ticked);
        }
    }

    /// A mob's tick: chase, swing, or walk home.
    #[allow(clippy::too_many_arguments)]
    fn mob_tick(
        &mut self,
        i: usize,
        dt: f32,
        host: &dyn Host,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let me = c.position;
        // Inside a Dungeon, its route graph keeps every run within the walls.
        let route = crate::dungeon::route_on(self.map);
        let map = self.map;
        let phase = self.phase;
        // A rooted mob stands where it is.
        let rooted = self.skills.rooted(self.mobs[i].guid);
        let mob = &mut self.mobs[i];
        if rooted {
            events.extend(mob.halt());
        } else {
            mob.advance(dt);
        }
        mob.swing_timer = (mob.swing_timer - dt).max(0.0);
        mob.repath_timer = (mob.repath_timer - dt).max(0.0);
        if let Some(t) = mob.assist_in {
            if mob.state != State::Idle || !self.player.alive() {
                mob.assist_in = None;
            } else if t <= dt {
                mob.assist_in = None;
                engage(mob, self.guid, events);
            } else {
                mob.assist_in = Some(t - dt);
            }
        }
        match mob.state {
            State::Idle => {
                if mob.assist_in.is_none() {
                    mob.stroll(dt, map, host, rng, events);
                }
            }
            State::Dead | State::Dazed => {}
            State::Evading => {
                if mob.walk.is_none() {
                    mob.state = State::Idle;
                    mob.orientation = mob.home_orientation;
                    mob.health = mob.stats.max_health;
                    events.push(values(mob.guid, &[(UNIT_FIELD_HEALTH, mob.health)]));
                }
            }
            // Another player's fight, handed over mid-tick: theirs to run from the next.
            State::Engaged if mob.target != self.guid => {}
            State::Engaged => {
                if !self.player.alive() || leash::beyond(mob.home, mob.position) {
                    let camp = mob.camp;
                    self.evade_camp(camp, route.map(|r| (r, map, host)), events);
                    return;
                }
                if self.skills.stunned(mob.guid) {
                    interrupt_cast(mob, events);
                    return;
                }
                if self.mob_cast(i, dt, me, host, rng, events) {
                    return;
                }
                let mob = &mut self.mobs[i];
                let d = distance(mob.position, me);
                if d > melee_reach() && rooted {
                    return;
                }
                if d > melee_reach() {
                    let stale = mob
                        .walk
                        .as_ref()
                        .is_none_or(|w| distance(w.aimed_at, me) > CHASE_SLACK);
                    if stale && mob.repath_timer <= 0.0 {
                        mob.repath_timer = REPATH_SECS;
                        let run = match route {
                            Some(r) => {
                                let path = r.path(mob.position, me, map, host);
                                mob.run_path(short_of(path, CHASE_DISTANCE), me)
                            }
                            None => {
                                let to = chase_point(mob.position, me, map, host);
                                match crate::phased::chase_within(phase, map, to, me) {
                                    Some(to) => mob.run_to(to, me),
                                    None => return,
                                }
                            }
                        };
                        events.push(run);
                    }
                    return;
                }
                if mob.walk.is_none() {
                    mob.orientation = (me[1] - mob.position[1])
                        .atan2(me[0] - mob.position[0])
                        .rem_euclid(std::f32::consts::TAU);
                }
                if mob.swing_timer > 0.0 {
                    return;
                }
                let stats = cast::buffed(&self.player.stats, &self.skills);
                let from_front = in_front(c.orientation, me, mob.position);
                let miss = self.skills.miss_bonus(mob.guid);
                let swing = combat::mob_swing(mob, &stats, from_front, miss, rng);
                events.push(SessionEvent::AttackerState(
                    swing.report(mob.guid, self.guid),
                ));
                mob.swing_timer = clock::secs(mob.template.base_attack_time)
                    * self.skills.attack_time_mul(mob.guid);
                if swing.damage > 0 {
                    self.player_takes(swing.damage, crate::rules::school::School::Physical, events);
                    if crate::wear::on_hit(c, &mut self.wear, events) {
                        self.refresh_gear(c, events);
                    }
                }
                let attacker = self.mobs[i].guid;
                self.struck(swing.outcome, attacker, c, rng, events, ticked);
            }
        }
    }

    /// The player loses `damage` to a creature's blow or spell: rage from it, and a timed cast
    /// broken (the Hearthstone's interrupts on damage).
    fn player_takes(
        &mut self,
        damage: u32,
        school: crate::rules::school::School,
        events: &mut Vec<SessionEvent>,
    ) {
        self.skills.break_stealth(events);
        let damage = self.skills.absorb(damage, school, events);
        let p = &mut self.player;
        p.health = p.health.saturating_sub(damage);
        p.power.gain_stored(
            PowerKind::Rage,
            rage::from_damage_taken(damage, p.stats.level),
        );
        events.push(values(
            self.guid,
            &[(UNIT_FIELD_HEALTH, p.health), p.power.field()],
        ));
        self.interrupt_item_cast(events);
    }

    /// `mob` dies at the player's hand: the fight ends and the kill's XP is paid.
    fn kill(
        &mut self,
        i: usize,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let elite_kind = self.elite_kind();
        let mob = &mut self.mobs[i];
        let entry = mob.template.entry;
        let mut loot = if Self::is_boss(mob) {
            let pool = crate::dungeon::boss_drops(c.level, c.race);
            looting::Loot::boss(mob.level(), &pool, rng)
        } else {
            looting::Loot::roll(mob.level(), rng)
        };
        // The quest items the player's quests still want, after the corpse's own drop.
        loot.items
            .extend(crate::quests::drops(c, entry, rng).into_iter().map(Some));
        let lootable = self.looting.drop(mob.guid, loot);
        let engaged = mob.state == State::Engaged;
        mob.state = State::Dead;
        mob.walk = None;
        drop_cast(mob, events);
        // A Phased Dungeon's dead stay down until its Run resets.
        if self.phase == crate::phased::Phase::OpenWorld {
            let t = mob.template;
            let wait = t
                .respawn_secs
                .unwrap_or_else(|| crate::rules::respawn::wait(t.rank));
            self.died.push((mob.spawn, wait));
        }
        self.skills.target_died(mob.guid, events);
        if self.player.attacking == Some(mob.guid) {
            self.player.attacking = None;
            events.push(SessionEvent::AttackStop {
                attacker: self.guid,
                victim: mob.guid,
            });
        }
        if engaged {
            events.push(SessionEvent::AttackStop {
                attacker: mob.guid,
                victim: self.guid,
            });
        }
        let mut pairs = vec![(UNIT_FIELD_HEALTH, 0), (UNIT_FIELD_FLAGS, 0)];
        pairs.extend(guid_pairs(UNIT_FIELD_TARGET, 0));
        pairs.push(looting::lootable_pair(lootable));
        events.push(values(mob.guid, &pairs));

        let elite = if mob.template.elite() {
            elite_kind
        } else {
            xp::Elite::No
        };
        let (victim, base_xp) = (mob.guid, xp::kill_xp(c.level, mob.level(), elite));
        let gained = self.scaled_xp(c, base_xp);
        if gained > 0 {
            // Rested XP doubles the kill, multiplied XP and all, while the pool lasts.
            let rested = crate::rest::spend(c, gained);
            events.push(SessionEvent::XpGain(XpGain {
                victim,
                total: gained + rested,
                base: gained,
                kill: true,
            }));
            if rested > 0 {
                events.push(values(self.guid, &crate::rest::pairs(c)));
            }
            ticked.leveled |= self.give_xp(c, gained + rested, events);
        }
        self.proc(
            crate::rules::procs::Happened {
                event: crate::callings::ProcEvent::Kill,
                skill: None,
                yields_xp: base_xp > 0,
                target: None,
                amount: 0,
                school: None,
            },
            c,
            rng,
            events,
            ticked,
        );
        self.auras_die_with(victim, events);
        crate::quests::credit_kill(c, entry, events);
    }

    /// Pays a quest's `base` XP, scaled by the Tier XP Rate and the dev multiplier as kill XP is;
    /// nothing at the Tier Cap. What it paid.
    pub(crate) fn reward_xp(
        &mut self,
        c: &mut Char,
        base: u32,
        events: &mut Vec<SessionEvent>,
    ) -> u32 {
        let gained = self.scaled_xp(c, base);
        if gained > 0 {
            self.give_xp(c, gained, events);
        }
        gained
    }

    /// What an award of `base` XP pays `c`: times the Tier XP Rate, then the dev multiplier;
    /// nothing at the Tier Cap.
    fn scaled_xp(&self, c: &Char, base: u32) -> u32 {
        if at_tier_cap(c) {
            0
        } else {
            self.dev.xp(at_tier_rate(c, base))
        }
    }

    /// Adds `amount` XP, levelling up as often as it covers (`Player::GiveXP`). Whether a level
    /// was gained.
    fn give_xp(&mut self, c: &mut Char, amount: u32, events: &mut Vec<SessionEvent>) -> bool {
        let Some(mut next) = xp::xp_to_next(c.level).filter(|_| !at_tier_cap(c)) else {
            return false;
        };
        c.xp += amount;
        let mut leveled = false;
        while c.xp >= next {
            c.xp -= next;
            leveled = true;
            let old = self.player.stats;
            c.level += 1;
            let new = crate::talents::stats(c);
            self.player.set_stats(new);
            self.player.health = new.max_health;
            let mut powers = [0; 5];
            powers[PowerKind::Mana.id() as usize] = new.max_mana.saturating_sub(old.max_mana);
            events.push(SessionEvent::LevelUp(LevelUpInfo {
                level: u32::from(c.level),
                health: new.max_health.saturating_sub(old.max_health),
                powers,
                stats: std::array::from_fn(|s| new.base[s].saturating_sub(old.base[s])),
            }));
            match xp::xp_to_next(c.level) {
                // The Tier Cap: what is left over is not banked.
                Some(_) if at_tier_cap(c) => {
                    c.xp = 0;
                    break;
                }
                Some(n) => next = n,
                None => {
                    // The end of the game: nothing more to bank.
                    c.xp = 0;
                    break;
                }
            }
        }
        let mut pairs = vec![(PLAYER_XP, c.xp)];
        if leveled {
            pairs.push((PLAYER_NEXT_LEVEL_XP, xp::xp_to_next(c.level).unwrap_or(0)));
            pairs.push(crate::talents::points_pair(c));
            pairs.push((UNIT_FIELD_HEALTH, self.player.health));
            pairs.extend(sheet_pairs(
                c,
                &cast::white(&self.player.stats, &self.skills),
            ));
        }
        events.push(values(self.guid, &pairs));
        leveled
    }

    /// The player's in-combat flag, and the power's regeneration (rage decay out of combat).
    fn update_combat(&mut self, dt: f32, events: &mut Vec<SessionEvent>) {
        let p = &mut self.player;
        let attacking = p
            .attacking
            .is_some_and(|t| self.mobs.iter().any(|m| m.guid == t && m.alive()));
        let attacked = self
            .mobs
            .iter()
            .any(|m| m.state == State::Engaged && m.target == self.guid);
        let in_combat = p.alive() && (attacking || attacked || self.skills.holds_combat());
        if in_combat != p.in_combat {
            p.in_combat = in_combat;
            let combat = if in_combat { UNIT_FLAG_IN_COMBAT } else { 0 };
            let flags = UNIT_FLAG_PVP_ATTACKABLE | combat;
            events.push(values(self.guid, &[(UNIT_FIELD_FLAGS, flags)]));
        }
        if p.power.regenerate(dt, in_combat, p.stats.mana_regen) {
            events.push(values(self.guid, &[p.power.field()]));
        }
    }
}

/// Whether `c` stands at their Calling's Tier Cap, where XP stops until they advance.
fn at_tier_cap(c: &Char) -> bool {
    c.level >= crate::callings::of(c.calling).cap
}

/// An award of `base` XP times `c`'s Tier XP Rate; the dev multiplier goes on top.
fn at_tier_rate(c: &Char, base: u32) -> u32 {
    base * xp::tier_rate(crate::callings::of(c.calling).tier)
}

/// `mob` turns on the player: `SMSG_AI_REACTION`, `SMSG_ATTACKSTART`, and its target and combat
/// flag.
fn engage(mob: &mut Mob, player: u64, events: &mut Vec<SessionEvent>) {
    mob.state = State::Engaged;
    mob.target = player;
    events.push(SessionEvent::AiReaction {
        unit: mob.guid,
        // `AI_REACTION_HOSTILE`.
        reaction: 2,
    });
    events.push(SessionEvent::AttackStart {
        attacker: mob.guid,
        victim: player,
    });
    let mut pairs = vec![(UNIT_FIELD_FLAGS, UNIT_FLAG_IN_COMBAT)];
    pairs.extend(guid_pairs(UNIT_FIELD_TARGET, player));
    events.push(values(mob.guid, &pairs));
}

/// `mob` gives up the fight and runs home, where it heals; inside a Dungeon, along its route.
fn evade(
    mob: &mut Mob,
    player: u64,
    route: Option<(&crate::dungeon::Route, u32, &dyn Host)>,
    events: &mut Vec<SessionEvent>,
) {
    mob.state = State::Evading;
    drop_cast(mob, events);
    events.push(SessionEvent::AttackStop {
        attacker: mob.guid,
        victim: player,
    });
    let mut pairs = vec![(UNIT_FIELD_FLAGS, 0)];
    pairs.extend(guid_pairs(UNIT_FIELD_TARGET, 0));
    events.push(values(mob.guid, &pairs));
    let home = mob.home;
    let run = match route {
        Some((r, map, host)) => mob.run_path(r.path(mob.position, home, map, host), home),
        None => mob.run_to(home, home),
    };
    events.push(run);
}

/// `mob`'s Creature Spell under way stops short, and lands nothing.
fn drop_cast(
    mob: &mut Mob,
    events: &mut Vec<SessionEvent>,
) -> Option<&'static crate::creature_spells::CreatureSpell> {
    let (spell, _) = mob.casting.take()?;
    events.push(SessionEvent::SpellFailedOther {
        caster: mob.guid,
        spell_id: spell.client_spell_id,
    });
    Some(spell)
}

/// Crowd control lands on `mob`: its Creature Spell under way is cancelled and goes on cooldown.
fn interrupt_cast(mob: &mut Mob, events: &mut Vec<SessionEvent>) {
    if let Some(spell) = drop_cast(mob, events) {
        mob.spell_cooldowns
            .push((spell.key.as_str(), clock::secs(spell.cooldown_ms)));
    }
}

/// `path` stopped `by` yards short of its end, as a chase stops short of its quarry.
fn short_of(mut path: Vec<[f32; 3]>, by: f32) -> Vec<[f32; 3]> {
    if let [.., from, to] = path[..] {
        let d = distance(from, to);
        let t = if d > by { (d - by) / d } else { 0.0 };
        let end = std::array::from_fn(|i| from[i] + (to[i] - from[i]) * t);
        *path.last_mut().expect("two points") = end;
    }
    path
}

/// The melee reach between the player and a mob.
fn melee_reach() -> f32 {
    crate::rules::melee::reach(PLAYER_COMBAT_REACH, COMBAT_REACH)
}

/// Whether `target` is in the front half-circle of a unit at `from` facing `orientation`
/// (`HasInArc(π)`).
fn in_front(orientation: f32, from: [f32; 3], target: [f32; 3]) -> bool {
    let (dx, dy) = (target[0] - from[0], target[1] - from[1]);
    if dx == 0.0 && dy == 0.0 {
        return true;
    }
    let angle = (dy.atan2(dx) - orientation + std::f32::consts::PI)
        .rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    angle.abs() <= std::f32::consts::FRAC_PI_2
}

/// Where a mob at `from` runs to reach a player at `to`: `CHASE_DISTANCE` short of them, on the
/// ground.
fn chase_point(from: [f32; 3], to: [f32; 3], map: u32, host: &dyn Host) -> [f32; 3] {
    let d = distance(from, to);
    let t = if d > CHASE_DISTANCE {
        (d - CHASE_DISTANCE) / d
    } else {
        0.0
    };
    let x = from[0] + (to[0] - from[0]) * t;
    let y = from[1] + (to[1] - from[1]) * t;
    let z = host.ground_height(map, x, y, to[2] + 2.0).unwrap_or(to[2]);
    [x, y, z]
}

/// A unit's eyes, for line of sight: two yards over its feet.
fn eye(p: [f32; 3]) -> [f32; 3] {
    [p[0], p[1], p[2] + 2.0]
}

/// The character sheet: every field the stock paper doll reads, from the realm's stats.
fn sheet_pairs(c: &Char, s: &CharacterStats) -> Vec<(u16, u32)> {
    let skill = u32::from(s.weapon_skill);
    let mut pairs = vec![
        (UNIT_FIELD_LEVEL, u32::from(s.level)),
        (UNIT_FIELD_MAXHEALTH, s.max_health),
        (UNIT_FIELD_BASE_HEALTH, s.base_health),
        (UNIT_FIELD_MAXPOWER1, s.max_mana),
        (
            UNIT_FIELD_MAXPOWER1 + PowerKind::Energy.id() as u16,
            power_rules::ENERGY_MAX + s.max_energy_bonus,
        ),
        (UNIT_FIELD_RESISTANCES, s.armor),
        // Gear's flat attack power rides apart, in the mods.
        (
            UNIT_FIELD_ATTACK_POWER,
            (s.attack_power - s.attack_power_bonus) as u32,
        ),
        (UNIT_FIELD_MINDAMAGE, float(s.min_damage)),
        (UNIT_FIELD_MAXDAMAGE, float(s.max_damage)),
        (UNIT_FIELD_BASEATTACKTIME, s.attack_time_ms),
        (
            UNIT_FIELD_BASEATTACKTIME + 1,
            s.off_hand.map_or(s.attack_time_ms, |o| o.attack_time_ms),
        ),
        (PLAYER_CRIT_PERCENTAGE, float(s.crit_pct as f32)),
        (PLAYER_DODGE_PERCENTAGE, float(s.dodge_pct as f32)),
        (PLAYER_PARRY_PERCENTAGE, float(s.parry_pct as f32)),
        (PLAYER_BLOCK_PERCENTAGE, float(s.block_pct as f32)),
    ];
    for (i, stat) in s.base.into_iter().enumerate() {
        pairs.push((UNIT_FIELD_STAT0 + i as u16, stat));
    }
    // The skill lines the sheet's Attack row and the table read, at the level's maximum.
    for (slot, id) in [SKILL_UNARMED, SKILL_DEFENSE].into_iter().enumerate() {
        let base = PLAYER_SKILL_INFO_1_1 + 3 * slot as u16;
        pairs.push((base, id));
        pairs.push((base + 1, skill | skill << 16));
        // Gear's Defense is the line's temporary bonus.
        let bonus = if id == SKILL_DEFENSE {
            u32::from(s.defense - s.weapon_skill)
        } else {
            0
        };
        pairs.push((base + 2, bonus));
    }
    pairs.extend(crate::equipment::sheet_pairs(c, s));
    pairs
}

/// The player's own descriptor at login.
fn player_fields(c: &Char, stats: &CharacterStats, power: &Power) -> ObjectFields {
    let display = content::display_id(c.race, c.gender);
    let [skin, face, hair_style, hair_color, _] = c.look.map(u32::from);
    let kind = PowerKind::of_class(c.class());
    let mut pairs = vec![
        (OBJECT_FIELD_TYPE, TYPE_PLAYER),
        (OBJECT_FIELD_SCALE_X, float(1.0)),
        (UNIT_FIELD_HEALTH, stats.max_health),
        power.field(),
        (UNIT_FIELD_MAXPOWER1, stats.max_mana),
        (UNIT_FIELD_MAXPOWER2, rage::MAX),
        (
            UNIT_FIELD_MAXPOWER1 + PowerKind::Energy.id() as u16,
            power_rules::ENERGY_MAX + stats.max_energy_bonus,
        ),
        (
            UNIT_FIELD_FACTIONTEMPLATE,
            content::faction_template(c.race),
        ),
        (
            UNIT_FIELD_BYTES_0,
            u32::from(c.race)
                | u32::from(c.class()) << 8
                | u32::from(c.gender) << 16
                | kind.id() << 24,
        ),
        (UNIT_FIELD_FLAGS, UNIT_FLAG_PVP_ATTACKABLE),
        (UNIT_FIELD_BOUNDINGRADIUS, float(0.389)),
        (UNIT_FIELD_COMBATREACH, float(PLAYER_COMBAT_REACH)),
        (UNIT_FIELD_DISPLAYID, display),
        (UNIT_FIELD_NATIVEDISPLAYID, display),
        (
            PLAYER_BYTES,
            skin | face << 8 | hair_style << 16 | hair_color << 24,
        ),
        crate::rest::flags(crate::rest::in_inn(c.map, c.position)),
        (PLAYER_BYTES_3, u32::from(c.gender)),
        (PLAYER_XP, c.xp),
        (PLAYER_NEXT_LEVEL_XP, xp::xp_to_next(c.level).unwrap_or(0)),
        (PLAYER_FIELD_WATCHED_FACTION_INDEX, u32::MAX),
    ];
    pairs.extend(guid_pairs(OBJECT_FIELD_GUID, c.guid));
    // Facial hair and the rest state in `PLAYER_BYTES_2`, and the pool.
    pairs.extend(crate::rest::pairs(c));
    pairs.extend(sheet_pairs(c, stats));
    pairs.push(cast::stance(c));
    pairs.push(crate::talents::points_pair(c));
    pairs.extend(crate::equipment::inventory_pairs(c));
    pairs.extend(crate::quests::log_pairs(c));
    pairs.extend(crate::areas::explored_pairs(&c.discovered));
    // No school's damage is modified: every multiplier is 1.
    for school in 0..7 {
        pairs.push((PLAYER_FIELD_MOD_DAMAGE_DONE_PCT + school, float(1.0)));
    }
    ObjectFields::from_pairs(&pairs).into_created(ObjectType::Player)
}

// ── Loot and gear ─────────────────────────────────────────────────────────────────────────────────

impl InWorld {
    /// `CMSG_LOOT` on `corpse`: the loot window, if it is a corpse with loot within reach.
    pub(crate) fn open_loot(&mut self, corpse: u64, c: &Char, events: &mut Vec<SessionEvent>) {
        let near = self.mobs.iter().any(|m| {
            m.guid == corpse
                && !m.alive()
                && crate::space::within_interaction(m.position, c.position)
        });
        self.looting.open(corpse, near, events);
    }

    /// The worn gear changed: redo the stats and the sheet, keeping the health lost so far.
    pub(crate) fn refresh_gear(&mut self, c: &Char, events: &mut Vec<SessionEvent>) {
        let lost = cast::buffed(&self.player.stats, &self.skills)
            .max_health
            .saturating_sub(self.player.health);
        let run_was = self.player.stats.run_speed_pct;
        self.player.set_stats(crate::talents::stats(c));
        if self.player.stats.run_speed_pct != run_was {
            self.update_speed(self.guid, events);
        }
        let max = cast::buffed(&self.player.stats, &self.skills).max_health;
        let p = &mut self.player;
        if p.alive() {
            p.health = max.saturating_sub(lost).max(1);
        }
        let mut pairs = sheet_pairs(c, &p.stats);
        pairs.extend(self.sheet_now());
        events.push(values(self.guid, &pairs));
    }
}
