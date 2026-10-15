//! The character in the world: the login burst that puts it there, the mobs around it, and the
//! fights between them, one tick at a time.

use benilla_protocol::messages::{AttackSwingError, LevelUpInfo, ObjectType, XpGain};
use benilla_protocol::{EntityKind, MoveSpeeds, ObjectFields, SessionEvent};

use crate::characters::Char;
use crate::combat;
use crate::content;
use crate::dev::DevMultipliers;
use crate::equipment;
use crate::fields::*;
use crate::looting::{self, Looting};
use crate::mobs::{distance, Mob, State, COMBAT_REACH};
use crate::population::Population;
use crate::rng::Rng;
use crate::rules::melee::Outcome;
use crate::rules::stats::WarriorStats;
use crate::rules::{aggro, leash, rage, xp};
use crate::Host;

/// vanilla's player speeds (`baseMoveSpeed`): yd/s, turn in rad/s.
const PLAYER_SPEEDS: MoveSpeeds = MoveSpeeds {
    walk: 2.5,
    run: 7.0,
    run_back: 4.5,
    swim: 4.722_222,
    swim_back: 2.5,
    turn_rate: std::f32::consts::PI,
};

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
}

/// The player's side of a fight.
struct Player {
    stats: WarriorStats,
    health: u32,
    /// Tenths.
    rage: u32,
    /// Seconds until the next swing; 0 is ready.
    swing_timer: f32,
    /// Who auto-attack swings at, from `CMSG_ATTACKSWING` to `CMSG_ATTACKSTOP`.
    attacking: Option<u64>,
    /// The last swing refusal sent, so it is sent once and not every tick.
    refused: Option<AttackSwingError>,
    /// Seconds into the current rage-decay tick.
    decay: f32,
    in_combat: bool,
}

impl Player {
    fn alive(&self) -> bool {
        self.health > 0
    }
}

/// The session of the character that is in the world.
pub(crate) struct InWorld {
    pub(crate) guid: u64,
    /// Seconds since the last autosave.
    pub(crate) since_save: f32,
    map: u32,
    player: Player,
    /// The mobs near the player; they stream in from the first tick on, which brings the ground.
    mobs: Vec<Mob>,
    population: Population,
    dev: DevMultipliers,
    /// The corpses with loot on them.
    pub(crate) looting: Looting,
    /// The Hub's NPCs are in the world: they come with the first tick, after the mobs.
    hub_shown: bool,
    /// This session's sales, on the merchant window's buyback tab.
    pub(crate) buyback: crate::vendor::Buyback,
    /// The dice for gear wear, apart from the fight's so that wearing changes no fight.
    wear: Rng,
}

impl InWorld {
    /// Logs `c` in: what vmangos sends from `CMSG_PLAYER_LOGIN` to the player's own create.
    pub(crate) fn enter(c: &Char, dev: DevMultipliers, events: &mut Vec<SessionEvent>) -> Self {
        let stats = equipment::stats(c);
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
        // Mid-afternoon at the game clock's real-time pace (one game minute a minute).
        events.push(SessionEvent::TimeSpeed {
            hours: 15,
            minutes: 0,
            day_serial: 0,
            timescale: 1.0 / 60.0,
        });
        // The proficiencies and the carried items' objects, ahead of the player that names them.
        equipment::login(c, events);
        events.push(SessionEvent::ObjectCreate {
            guid: c.guid,
            kind: EntityKind::Player,
            display_id: Some(content::display_id(c.race, c.gender)),
            position: c.position,
            orientation: c.orientation,
            scale: 1.0,
            speeds: Some(dev.speeds(PLAYER_SPEEDS)),
            mover: None,
            transport_progress: None,
            transport: None,
            spline: None,
            fields: player_fields(c, &stats),
        });
        Self {
            guid: c.guid,
            since_save: 0.0,
            map: c.map,
            player: Player {
                stats,
                health: stats.max_health,
                rage: 0,
                swing_timer: 0.0,
                attacking: None,
                refused: None,
                decay: 0.0,
                in_combat: false,
            },
            mobs: Vec::new(),
            population: Population::default(),
            dev,
            looting: Looting::default(),
            hub_shown: false,
            buyback: crate::vendor::Buyback::default(),
            wear: Rng::new(c.guid ^ 0xD0_4AB1_1170),
        }
    }

    /// `CMSG_SET_SELECTION`: the player's target, which the client shows in `UNIT_FIELD_TARGET`.
    pub(crate) fn select(&mut self, target: u64, events: &mut Vec<SessionEvent>) {
        events.push(values(self.guid, &guid_pairs(UNIT_FIELD_TARGET, target)));
    }

    /// `CMSG_ATTACKSWING`: start auto-attack on `target`, echoed as `SMSG_ATTACKSTART`.
    pub(crate) fn attack(&mut self, target: u64, events: &mut Vec<SessionEvent>) {
        let Some(i) = self.mobs.iter().position(|m| m.guid == target && m.alive()) else {
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

    /// Advances the world by `dt` seconds around `c`.
    pub(crate) fn tick(
        &mut self,
        dt: f32,
        host: &dyn Host,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) -> Ticked {
        let mut ticked = Ticked::default();
        let me = c.position;
        self.population
            .update(dt, me, self.map, host, &mut self.mobs, events);
        if !self.hub_shown {
            self.hub_shown = true;
            events.extend(crate::hub::npcs().iter().map(|n| n.create()));
        }
        // A corpse that respawned or streamed out takes its loot with it.
        let mobs = &self.mobs;
        self.looting
            .retain(|corpse| mobs.iter().any(|m| m.guid == corpse && !m.alive()));
        let p = &mut self.player;
        p.swing_timer = (p.swing_timer - dt).max(0.0);

        // Hostile mobs notice a living player who comes too close, and bring their camp.
        if p.alive() {
            for i in 0..self.mobs.len() {
                let mob = &self.mobs[i];
                if mob.state != State::Idle || !mob.template.hostile {
                    continue;
                }
                let r = aggro::radius(mob.template.detection_range, c.level, mob.level());
                if distance(mob.position, me) <= r
                    && host.line_of_sight(self.map, eye(mob.position), eye(me))
                {
                    self.pull(i, events);
                }
            }
        }

        self.player_swing(c, rng, events, &mut ticked);
        for i in 0..self.mobs.len() {
            self.mob_tick(i, dt, host, c, rng, events);
        }
        self.update_combat(dt, events);
        ticked
    }

    /// Mob `i` turns on the player, and every idle member of its camp with it (social aggro).
    fn pull(&mut self, i: usize, events: &mut Vec<SessionEvent>) {
        let camp = self.mobs[i].camp;
        engage(&mut self.mobs[i], self.guid, events);
        for mob in &mut self.mobs {
            if mob.camp == camp && mob.state == State::Idle {
                engage(mob, self.guid, events);
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

        let from_front = in_front(mob.orientation, mob.position, c.position);
        let swing = combat::player_swing(&p.stats, mob, from_front, rng);
        events.push(SessionEvent::AttackerState(swing.report(self.guid, target)));
        p.swing_timer = p.stats.attack_time_ms as f32 / 1000.0;

        // Rage from what landed, or from three quarters of what a dodge or parry stopped.
        let earned = match swing.outcome {
            Outcome::Dodge | Outcome::Parry => {
                rage::from_damage_dealt(swing.would_be * 3 / 4, p.stats.level)
            }
            _ => rage::from_damage_dealt(swing.damage, p.stats.level),
        };
        if earned > 0 {
            p.rage = (p.rage + earned).min(rage::MAX);
            events.push(values(self.guid, &[(UNIT_FIELD_POWER2, p.rage)]));
        }

        let mob = &mut self.mobs[i];
        mob.health = mob.health.saturating_sub(swing.damage);
        if swing.damage > 0 {
            events.push(values(mob.guid, &[(UNIT_FIELD_HEALTH, mob.health)]));
        }
        let idle = mob.state == State::Idle;
        if swing.damage > 0 && crate::wear::on_hit(c, &mut self.wear, events) {
            self.refresh_gear(c, events);
        }
        if idle {
            self.pull(i, events);
        }
        if self.mobs[i].health == 0 {
            self.kill(i, c, rng, events, ticked);
        }
    }

    /// A mob's tick: chase, swing, or walk home.
    fn mob_tick(
        &mut self,
        i: usize,
        dt: f32,
        host: &dyn Host,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) {
        let me = c.position;
        let mob = &mut self.mobs[i];
        mob.advance(dt);
        mob.swing_timer = (mob.swing_timer - dt).max(0.0);
        mob.repath_timer = (mob.repath_timer - dt).max(0.0);
        match mob.state {
            State::Idle | State::Dead => {}
            State::Evading => {
                if mob.walk.is_none() {
                    mob.state = State::Idle;
                    mob.orientation = mob.home_orientation;
                    mob.health = mob.stats.max_health;
                    events.push(values(mob.guid, &[(UNIT_FIELD_HEALTH, mob.health)]));
                }
            }
            State::Engaged => {
                if !self.player.alive() || leash::beyond(mob.home, mob.position) {
                    evade(mob, self.guid, events);
                    return;
                }
                let d = distance(mob.position, me);
                if d > melee_reach() {
                    let stale = mob
                        .walk
                        .is_none_or(|w| distance(w.aimed_at, me) > CHASE_SLACK);
                    if stale && mob.repath_timer <= 0.0 {
                        let dest = chase_point(mob.position, me, self.map, host);
                        mob.repath_timer = REPATH_SECS;
                        events.push(mob.run_to(dest, me));
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
                let p = &mut self.player;
                let from_front = in_front(c.orientation, me, mob.position);
                let swing = combat::mob_swing(mob, &p.stats, from_front, rng);
                events.push(SessionEvent::AttackerState(
                    swing.report(mob.guid, self.guid),
                ));
                mob.swing_timer = mob.template.base_attack_time as f32 / 1000.0;
                if swing.damage > 0 {
                    p.health = p.health.saturating_sub(swing.damage);
                    p.rage = (p.rage + rage::from_damage_taken(swing.damage, p.stats.level))
                        .min(rage::MAX);
                    events.push(values(
                        self.guid,
                        &[(UNIT_FIELD_HEALTH, p.health), (UNIT_FIELD_POWER2, p.rage)],
                    ));
                    if crate::wear::on_hit(c, &mut self.wear, events) {
                        self.refresh_gear(c, events);
                    }
                }
                let p = &mut self.player;
                if !p.alive() {
                    // Death and the way back are a later ticket's; the fight just ends.
                    if let Some(victim) = p.attacking.take() {
                        events.push(SessionEvent::AttackStop {
                            attacker: self.guid,
                            victim,
                        });
                    }
                }
            }
        }
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
        let mob = &mut self.mobs[i];
        let lootable = self
            .looting
            .drop(mob.guid, looting::Loot::roll(mob.level(), rng));
        let engaged = mob.state == State::Engaged;
        mob.state = State::Dead;
        mob.walk = None;
        self.population.died(mob.spawn);
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
            xp::Elite::OpenWorld
        } else {
            xp::Elite::No
        };
        let gained = self.dev.xp(xp::kill_xp(c.level, mob.level(), elite));
        if gained > 0 {
            events.push(SessionEvent::XpGain(XpGain {
                victim: mob.guid,
                total: gained,
                base: gained,
                kill: true,
            }));
            ticked.leveled |= self.give_xp(c, gained, events);
        }
    }

    /// Adds `amount` XP, levelling up as often as it covers (`Player::GiveXP`). Whether a level
    /// was gained.
    fn give_xp(&mut self, c: &mut Char, amount: u32, events: &mut Vec<SessionEvent>) -> bool {
        let Some(mut next) = xp::xp_to_next(c.level) else {
            return false;
        };
        c.xp += amount;
        let mut leveled = false;
        while c.xp >= next {
            c.xp -= next;
            leveled = true;
            let old = self.player.stats;
            c.level += 1;
            let new = equipment::stats(c);
            self.player.stats = new;
            self.player.health = new.max_health;
            events.push(SessionEvent::LevelUp(LevelUpInfo {
                level: u32::from(c.level),
                health: new.max_health.saturating_sub(old.max_health),
                powers: [0; 5],
                stats: std::array::from_fn(|s| new.base[s].saturating_sub(old.base[s])),
            }));
            match xp::xp_to_next(c.level) {
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
            pairs.push((UNIT_FIELD_HEALTH, self.player.health));
            pairs.extend(sheet_pairs(&self.player.stats));
        }
        events.push(values(self.guid, &pairs));
        leveled
    }

    /// The player's in-combat flag, and rage decay out of combat.
    fn update_combat(&mut self, dt: f32, events: &mut Vec<SessionEvent>) {
        let p = &mut self.player;
        let attacking = p
            .attacking
            .is_some_and(|t| self.mobs.iter().any(|m| m.guid == t && m.alive()));
        let attacked = self.mobs.iter().any(|m| m.state == State::Engaged);
        let in_combat = p.alive() && (attacking || attacked);
        if in_combat != p.in_combat {
            p.in_combat = in_combat;
            let flags = if in_combat { UNIT_FLAG_IN_COMBAT } else { 0 };
            events.push(values(self.guid, &[(UNIT_FIELD_FLAGS, flags)]));
        }
        if in_combat || p.rage == 0 {
            p.decay = 0.0;
            return;
        }
        p.decay += dt;
        let mut decayed = false;
        while p.decay >= rage::DECAY_TICK_SECS && p.rage > 0 {
            p.decay -= rage::DECAY_TICK_SECS;
            p.rage = p.rage.saturating_sub(rage::DECAY_PER_TICK);
            decayed = true;
        }
        if decayed {
            events.push(values(self.guid, &[(UNIT_FIELD_POWER2, p.rage)]));
        }
    }
}

/// `mob` turns on the player: `SMSG_AI_REACTION`, `SMSG_ATTACKSTART`, and its target and combat
/// flag.
fn engage(mob: &mut Mob, player: u64, events: &mut Vec<SessionEvent>) {
    mob.state = State::Engaged;
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

/// `mob` gives up the fight and runs home, where it heals.
fn evade(mob: &mut Mob, player: u64, events: &mut Vec<SessionEvent>) {
    mob.state = State::Evading;
    events.push(SessionEvent::AttackStop {
        attacker: mob.guid,
        victim: player,
    });
    let mut pairs = vec![(UNIT_FIELD_FLAGS, 0)];
    pairs.extend(guid_pairs(UNIT_FIELD_TARGET, 0));
    events.push(values(mob.guid, &pairs));
    let home = mob.home;
    events.push(mob.run_to(home, home));
}

/// The melee reach between the player and a mob (`Unit::GetCombatReachToTarget`), never under
/// 5 yd.
fn melee_reach() -> f32 {
    (PLAYER_COMBAT_REACH + COMBAT_REACH + 4.0 / 3.0).max(5.0)
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
fn sheet_pairs(s: &WarriorStats) -> Vec<(u16, u32)> {
    let skill = u32::from(s.weapon_skill);
    let mut pairs = vec![
        (UNIT_FIELD_LEVEL, u32::from(s.level)),
        (UNIT_FIELD_MAXHEALTH, s.max_health),
        (UNIT_FIELD_BASE_HEALTH, s.base_health),
        (UNIT_FIELD_RESISTANCES, s.armor),
        // Gear's flat attack power rides apart, in the mods.
        (
            UNIT_FIELD_ATTACK_POWER,
            (s.attack_power - s.attack_power_bonus) as u32,
        ),
        (UNIT_FIELD_MINDAMAGE, float(s.min_damage)),
        (UNIT_FIELD_MAXDAMAGE, float(s.max_damage)),
        (UNIT_FIELD_BASEATTACKTIME, s.attack_time_ms),
        (UNIT_FIELD_BASEATTACKTIME + 1, s.attack_time_ms),
        (PLAYER_CRIT_PERCENTAGE, float(s.crit_pct as f32)),
        (PLAYER_DODGE_PERCENTAGE, float(s.dodge_pct as f32)),
        // A Basic Warrior has no Parry skill and no shield.
        (PLAYER_PARRY_PERCENTAGE, float(0.0)),
        (PLAYER_BLOCK_PERCENTAGE, float(0.0)),
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
    pairs.extend(crate::equipment::sheet_pairs(s));
    pairs
}

/// The player's own descriptor at login.
fn player_fields(c: &Char, stats: &WarriorStats) -> ObjectFields {
    let display = content::display_id(c.race, c.gender);
    let [skin, face, hair_style, hair_color, facial_hair] = c.look.map(u32::from);
    let mut pairs = vec![
        (OBJECT_FIELD_TYPE, TYPE_PLAYER),
        (OBJECT_FIELD_SCALE_X, float(1.0)),
        (UNIT_FIELD_HEALTH, stats.max_health),
        (UNIT_FIELD_POWER2, 0),
        (UNIT_FIELD_MAXPOWER2, rage::MAX),
        (
            UNIT_FIELD_FACTIONTEMPLATE,
            content::faction_template(c.race),
        ),
        (
            UNIT_FIELD_BYTES_0,
            u32::from(c.race)
                | u32::from(benilla_protocol::messages::CLASS_WARRIOR) << 8
                | u32::from(c.gender) << 16
                | POWER_RAGE << 24,
        ),
        (UNIT_FIELD_BOUNDINGRADIUS, float(0.389)),
        (UNIT_FIELD_COMBATREACH, float(PLAYER_COMBAT_REACH)),
        (UNIT_FIELD_DISPLAYID, display),
        (UNIT_FIELD_NATIVEDISPLAYID, display),
        (
            PLAYER_BYTES,
            skin | face << 8 | hair_style << 16 | hair_color << 24,
        ),
        // Facial hair, then `REST_STATE_NORMAL` in the top byte.
        (PLAYER_BYTES_2, facial_hair | 0x01 << 24),
        (PLAYER_BYTES_3, u32::from(c.gender)),
        (PLAYER_XP, c.xp),
        (PLAYER_NEXT_LEVEL_XP, xp::xp_to_next(c.level).unwrap_or(0)),
        (PLAYER_FIELD_WATCHED_FACTION_INDEX, u32::MAX),
    ];
    pairs.extend(guid_pairs(OBJECT_FIELD_GUID, c.guid));
    pairs.extend(sheet_pairs(stats));
    pairs.extend(crate::equipment::inventory_pairs(&c.inventory));
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
                && distance(m.position, c.position) <= looting::LOOT_DISTANCE
        });
        self.looting.open(corpse, near, events);
    }

    /// The worn gear changed: redo the stats and the sheet, keeping the health lost so far.
    pub(crate) fn refresh_gear(&mut self, c: &Char, events: &mut Vec<SessionEvent>) {
        let p = &mut self.player;
        let lost = p.stats.max_health.saturating_sub(p.health);
        p.stats = equipment::stats(c);
        if p.alive() {
            p.health = p.stats.max_health.saturating_sub(lost).max(1);
        }
        let mut pairs = sheet_pairs(&p.stats);
        pairs.push((UNIT_FIELD_HEALTH, p.health));
        events.push(values(self.guid, &pairs));
    }
}
