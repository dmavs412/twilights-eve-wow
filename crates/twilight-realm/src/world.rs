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
mod fear;
mod food;
mod ground;
mod notoriety;
mod phase;
mod root;
pub(crate) use phase::DoorClick;
mod sleep;
pub(crate) use phase::creatures;
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
/// A kill to share out among the players near it (#148).
#[derive(Debug, Clone)]
pub(crate) struct Kill {
    pub(crate) mob: u64,
    pub(crate) entry: u32,
    pub(crate) level: u8,
    pub(crate) elite: xp::Elite,
    pub(crate) boss: bool,
    /// Items the corpse always holds (a Phased Dungeon creature's `drop`).
    pub(crate) drops: &'static [u32],
    /// The corpse's money, rolled once for everyone; 0 for a kill made alone, whose loot rolls
    /// its own.
    pub(crate) money: u32,
    pub(crate) position: [f32; 3],
    /// A Bounty Hunter's tag: its kill pays Blood Coin and raises its owner's Bounty Rank.
    pub(crate) bounty: Option<crate::mobs::Bounty>,
}

/// One player's part in a kill: their share of its money, and the levels of everyone credited
/// (theirs among them) that divide its XP.
#[derive(Debug, Clone)]
pub(crate) struct Credit {
    pub(crate) kill: Kill,
    /// Their share of the money; `None` alone, when the loot rolls its own.
    pub(crate) money: Option<u32>,
    pub(crate) levels: Vec<u8>,
}

/// A friendly spell one player cast on another (#150): resolved by the caster, applied by the
/// session of `to`.
#[derive(Debug, Clone)]
pub(crate) enum Help {
    Heal {
        to: u64,
        healer: u64,
        spell_id: u32,
        amount: u32,
    },
    Aura {
        to: u64,
        aura: cast::Aura,
    },
}

impl Help {
    pub(crate) fn to(&self) -> u64 {
        match self {
            Help::Heal { to, .. } | Help::Aura { to, .. } => *to,
        }
    }
}

/// Damage one player dealt another's stand-in: resolved by the attacker, taken by the session
/// of `to`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Harm {
    pub(crate) to: u64,
    pub(crate) from: u64,
    pub(crate) amount: u32,
    pub(crate) school: crate::rules::school::School,
}

/// A gaze that went off: `damage` of `school` to everyone within `radius` of `at` who is not
/// behind cover.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Gaze {
    pub(crate) caster: u64,
    pub(crate) spell_id: u32,
    pub(crate) at: [f32; 3],
    pub(crate) radius: f32,
    pub(crate) damage: u32,
    pub(crate) school: crate::rules::school::School,
    /// A nova: cover does not stop it.
    pub(crate) through_cover: bool,
    /// Not damage but a root of this many milliseconds; 0 for damage.
    pub(crate) root_ms: u32,
}

/// A GameObject the Phase sees appear or go: a creature spell's ground patch.
#[derive(Debug, Clone)]
pub(crate) enum Scenery {
    Show(SessionEvent),
    Hide(u64),
}

/// A Creature Spell cast at a character of another session: the realm lands it there.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Foreign {
    pub(crate) target: u64,
    pub(crate) caster: u64,
    pub(crate) level: u8,
    pub(crate) from: [f32; 3],
    pub(crate) spell: &'static crate::creature_spells::CreatureSpell,
}

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
    /// The Calling's own bar: rage, energy, focus or mana.
    power: Power,
    /// Seconds until the next swing; 0 is ready.
    swing_timer: f32,
    /// [`Self::swing_timer`] for the off-hand weapon.
    off_swing_timer: f32,
    /// Who auto-attack swings at, from `CMSG_ATTACKSWING` to `CMSG_ATTACKSTOP`.
    attacking: Option<u64>,
    /// The last swing refusal sent, so it is sent once and not every tick.
    refused: Option<AttackSwingError>,
    /// An auto-repeat shot's `(spell id, target)`, from its cast until it stops.
    shooting: Option<(u32, u64)>,
    /// Seconds until the next auto-repeat shot; 0 is ready.
    shot_timer: f32,
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
    /// The other living players sharing this player's mobs, where they stand and the share of
    /// their health left, as of the tick's start.
    pub(crate) others: Vec<(u64, [f32; 3], f32)>,
    /// The lent mobs this session killed, for the realm to share out (#148).
    pub(crate) kills: Vec<Kill>,
    /// This player's parts in kills nearby, paid on the next tick.
    pub(crate) credits: Vec<Credit>,
    /// The heals this session cast on other players (#150), for the realm to deliver.
    pub(crate) helps: Vec<Help>,
    /// What other players cast on this one, applied on the next tick.
    pub(crate) helped: Vec<Help>,
    /// The damage this session dealt to other players' stand-ins, for the realm to deliver.
    pub(crate) harms: Vec<Harm>,
    /// The damage other players dealt this one, taken on the next tick.
    pub(crate) harmed: Vec<Harm>,
    /// The player whose damage stops at 1 health (a duel opponent): their blow that would kill
    /// beats this one instead.
    pub(crate) foe: Option<u64>,
    /// The foe whose blow left this player at 1 health, for the realm to settle.
    pub(crate) beaten_by: Option<u64>,
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
    /// The Phased Dungeons whose doorway portal the client has.
    portals: Vec<u32>,
    /// The dungeons whose door is open, as of the realm's last sync.
    pub(crate) doors_open: Vec<u32>,
    /// The doors the client has, by dungeon, each with the state it was last shown in.
    doors_shown: std::collections::HashMap<u32, bool>,
    /// The guids of the dungeon GameObjects the client has.
    dungeon_objects: std::collections::HashSet<u64>,
    /// The stops whose inner portal is open in the Run the player is in, as of the realm's last
    /// sync.
    pub(crate) inner_open: Vec<String>,
    /// The dungeon and stop whose box the player stood in at the last Phase check.
    in_stop: Option<(u32, usize)>,
    /// The guids of the open-world quest GameObjects the client has.
    quest_objects: std::collections::HashSet<u64>,
    /// The quest GameObjects the player used, each with the seconds until it returns.
    quest_spent: Vec<(u64, f32)>,
    /// The stacks of the Phase's meter the player's aura shows; 0 for none.
    noise_shown: u32,
    /// Stacks the player's doings added to the Run's meter since the shard last took them.
    pub(crate) noise_gain: u32,
    /// Stacks the player's hiding took off it since the shard last took them.
    noise_drain: u32,
    /// The hiding spot the player is crouched at.
    hiding: Option<phase::Hide>,
    /// Gazes that went off here this tick, for the realm to land on everyone in the Phase.
    gazes: Vec<Gaze>,
    /// Raid warnings of casts that began here this tick, for the realm to send to the Phase.
    warnings: Vec<String>,
    /// Creature Spells cast here at characters of other sessions, for the realm to land.
    foreign: Vec<Foreign>,
    /// The other living characters in this player's Phase and where they stand, as of this tick:
    /// whom a Creature Spell with a random target may pick.
    pub(crate) phase_mates: Vec<(u64, [f32; 3])>,
    /// The fear the player runs under.
    fear: Option<fear::Fear>,
    /// The root the player is held by.
    root: Option<root::Root>,
    /// Root and unroot orders sent so far: the counter the client acks.
    move_modes: u32,
    /// The ground patches this session's creatures put down, burning until their time is up.
    patches: Vec<ground::Patch>,
    /// Patches put down so far, which numbers the next one's GameObject.
    patch_serial: u32,
    /// Patches that appeared or went this tick, for the realm to show the Phase.
    scenery: Vec<Scenery>,
    /// What each creature casts.
    pub(crate) casters: crate::creature_spells::Casters,
    /// The Heat Level whose aura the player wears; 0 for none.
    heat_shown: u8,
    /// Seconds of open-world play until the next Bounty Hunter; `None` before the count starts.
    hunter_in: Option<f32>,
    /// Bounty Hunters sent this session, for their guids.
    hunters_sent: u32,
    /// The unit whose damage last landed on the player, for who killed them.
    last_hit_by: Option<u64>,
    /// Whether the Marked debuff is up.
    marked_shown: bool,
    /// Whether the stats carry the Marked penalty.
    marked_applied: bool,
    /// Where the Marked victor's map flag was last put.
    victor_flag: Option<[f32; 2]>,
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
                power: Power::fresh(
                    &stats,
                    crate::callings::of(c.calling).power,
                    crate::callings::of(c.calling).energy_regen(),
                ),
                swing_timer: 0.0,
                off_swing_timer: 0.0,
                shooting: None,
                shot_timer: 0.0,
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
            others: Vec::new(),
            kills: Vec::new(),
            credits: Vec::new(),
            helps: Vec::new(),
            helped: Vec::new(),
            harms: Vec::new(),
            harmed: Vec::new(),
            foe: None,
            beaten_by: None,
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
            portals: Vec::new(),
            doors_open: Vec::new(),
            doors_shown: Default::default(),
            dungeon_objects: Default::default(),
            inner_open: Vec::new(),
            in_stop: None,
            quest_objects: Default::default(),
            quest_spent: Vec::new(),
            noise_shown: 0,
            noise_gain: 0,
            noise_drain: 0,
            hiding: None,
            gazes: Vec::new(),
            warnings: Vec::new(),
            foreign: Vec::new(),
            phase_mates: Vec::new(),
            fear: None,
            root: None,
            move_modes: 0,
            patches: Vec::new(),
            patch_serial: 0,
            scenery: Vec::new(),
            casters: Default::default(),
            heat_shown: 0,
            hunter_in: None,
            hunters_sent: 0,
            last_hit_by: None,
            marked_shown: false,
            marked_applied: c.marked.is_some(),
            victor_flag: None,
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
        if self.fear.is_some() {
            events.push(SessionEvent::AttackSwingError(
                AttackSwingError::DeadOrUnattackable,
            ));
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
            self.pull(i, None, events);
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

    /// The shard this player shares, when they stand on an open-world map: its open world, or the
    /// Run of the Phased Dungeon they stand in. The realm lends it its mobs and sends it what
    /// comes into view.
    pub(crate) fn shard_place(&self) -> Option<crate::shard::Place> {
        (!self.transfer && content::open_world(self.map)).then_some(crate::shard::Place {
            map: self.map,
            phase: self.phase,
        })
    }

    /// Whether this session runs mob `guid`'s fight and AI: every mob of its own, and the lent
    /// ones the realm gave it this tick.
    /// The player as another client is sent them: a remote player, public fields only.
    pub(crate) fn remote_create(&self, c: &Char) -> SessionEvent {
        let mut create = self.player_create(c);
        if let SessionEvent::ObjectCreate { fields, .. } = &mut create {
            let pairs: Vec<(u16, u32)> = fields
                .raw_fields()
                .filter(|&(f, _)| crate::shard::public_player_field(f))
                .collect();
            *fields = ObjectFields::from_pairs(&pairs).into_created(ObjectType::Player);
        }
        create
    }

    /// Whether other clients are kept from seeing the player: stealthed.
    pub(crate) fn hidden(&self) -> bool {
        self.skills.stealthed()
    }

    pub(crate) fn alive(&self) -> bool {
        self.player.alive()
    }

    pub(crate) fn in_combat(&self) -> bool {
        self.player.in_combat
    }

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
        for credit in std::mem::take(&mut self.credits) {
            self.take_credit(credit, c, rng, events, &mut ticked);
        }
        // Damage first: a cast's own hit lands before the aura it puts on, so it does not wake
        // the one it incapacitates.
        for harm in std::mem::take(&mut self.harmed) {
            if self.player.alive() {
                self.player_takes_from(harm.from, harm.amount, harm.school, events);
                self.woken_by_damage(self.guid, events);
            }
        }
        for help in std::mem::take(&mut self.helped) {
            match help {
                Help::Heal {
                    healer,
                    spell_id,
                    amount,
                    ..
                } => {
                    self.heal_from(healer, c, spell_id, amount, events);
                }
                Help::Aura { aura, .. } => self.wear(aura, events),
            }
        }
        self.land_blink(host, c, events);
        let me = c.position;
        self.cross_phase(c, events);
        self.show_portals(me, events);
        self.show_quest_objects(dt, me, events);
        if self.player.alive() {
            self.discover(host, c, events, &mut ticked);
        }
        self.rest(dt, c, events);
        self.notoriety_tick(dt, host, c, rng, events);
        self.hide_tick(dt, c);
        // A corpse that respawned or streamed out takes its loot with it.
        let mobs = &self.mobs;
        self.looting
            .retain(|corpse| mobs.iter().any(|m| m.guid == corpse && !m.alive()));
        let p = &mut self.player;
        p.swing_timer = (p.swing_timer - dt).max(0.0);
        p.off_swing_timer = (p.off_swing_timer - dt).max(0.0);
        p.shot_timer = (p.shot_timer - dt).max(0.0);

        self.wake_sleepers(me, events);
        // Hostile mobs notice a living player who comes too close, and bring their camp.
        let p = &mut self.player;
        if p.alive() {
            for i in 0..self.mobs.len() {
                let mob = &self.mobs[i];
                if mob.state != State::Idle
                    || mob.asleep
                    || !mob.template.hostile
                    || mob.hunts.is_some_and(|owner| owner != self.guid)
                {
                    continue;
                }
                let r = self.notice_radius(mob, c.level);
                if distance(mob.position, me) <= r
                    && host.line_of_sight(self.map, eye(mob.position), eye(me))
                {
                    self.pull(i, Some(host), events);
                }
            }
        }

        self.player_swing(c, rng, events, &mut ticked);
        self.auto_shot(c, rng, events, &mut ticked);
        for i in 0..self.mobs.len() {
            // A stand-in's player acts in their own session.
            if self.runs(self.mobs[i].guid) && !self.mobs[i].remote {
                self.mob_tick(i, dt, host, c, rng, events, &mut ticked);
            }
        }
        self.skills_tick(dt, wall, c, rng, events, &mut ticked);
        self.movement_tick(dt, c, rng, events, &mut ticked);
        self.fear_tick(dt, host, c, rng, events);
        self.root_tick(dt, events);
        self.patch_tick(dt);
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

    /// Mob `i` turns on the player. A hostile mob calls the idle campmates within earshot, each
    /// of whom joins after a short delay (social aggro); a neutral one comes alone. The call
    /// goes one hop: a mob that joins calls no one. With a `host`, a campmate joins only when it
    /// has line of sight to the pulled mob; an attack passes none and calls through walls.
    fn pull(&mut self, i: usize, host: Option<&dyn Host>, events: &mut Vec<SessionEvent>) {
        let camp = self.mobs[i].camp;
        let at = self.mobs[i].position;
        let map = self.map;
        engage(&mut self.mobs[i], self.guid, events);
        if !self.mobs[i].template.hostile {
            return;
        }
        for mob in &mut self.mobs {
            let d = distance(mob.position, at);
            if mob.camp == camp
                && mob.state == State::Idle
                && !mob.asleep
                && mob.assist_in.is_none()
                && d <= aggro::SOCIAL_RANGE_YD
                && host.map_or(true, |h| h.line_of_sight(map, eye(mob.position), eye(at)))
            {
                mob.assist_in = Some(aggro::social_delay(d));
            }
        }
    }

    /// Everyone in `camp` fighting `target` runs home together.
    fn evade_camp(
        &mut self,
        camp: u32,
        target: u64,
        route: Option<(&crate::dungeon::Route, u32, &dyn Host)>,
        events: &mut Vec<SessionEvent>,
    ) {
        for mob in &mut self.mobs {
            if mob.camp == camp && mob.state == State::Engaged && mob.target == target {
                evade(mob, target, route, events);
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
        if !p.alive() || self.fear.is_some() {
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
        // The armour the player's debuffs took off it (Sundered).
        let mut stats = stats;
        stats.armor_pen += self.skills.armor_loss(target);
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
        let through = self.skills.absorb_on(
            mob.guid,
            swing.damage,
            crate::rules::school::School::Physical,
            events,
        );
        wound(
            mob,
            through,
            self.guid,
            crate::rules::school::School::Physical,
            &mut self.harms,
            events,
        );
        let idle = mob.state == State::Idle;
        if swing.damage > 0 && crate::wear::on_hit(c, &mut self.wear, events) {
            self.refresh_gear(c, events);
        }
        if idle {
            self.pull(i, None, events);
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
        if mob.template.inert {
            if let Some(t) = mob.since_hurt {
                if t + dt >= crate::mobs::INERT_RESET_SECS {
                    mob.since_hurt = None;
                    mob.health = mob.stats.max_health;
                    events.push(values(mob.guid, &[(UNIT_FIELD_HEALTH, mob.health)]));
                } else {
                    mob.since_hurt = Some(t + dt);
                }
            }
            return;
        }
        // Asleep: no aggro, no threat, no movement, until `wake_sleepers` wakes it.
        if mob.asleep {
            return;
        }
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
                mob.engaged_secs = 0.0;
                if mob.assist_in.is_none() {
                    mob.stroll(dt, map, host, rng, events);
                }
            }
            State::Dead => mob.engaged_secs = 0.0,
            State::Evading => {
                mob.engaged_secs = 0.0;
                if mob.walk.is_none() {
                    mob.state = State::Idle;
                    mob.orientation = mob.home_orientation;
                    mob.health = mob.stats.max_health;
                    mob.threat.clear();
                    events.push(values(mob.guid, &[(UNIT_FIELD_HEALTH, mob.health)]));
                }
            }
            // Another living player's fight: theirs to run.
            State::Engaged
                if mob.target != self.guid && self.others.iter().any(|o| o.0 == mob.target) => {}
            State::Engaged => {
                let target = mob.target;
                // Its threat table picks whom it fights; the dead and the gone leave it, and once
                // it strays past its leash, so do those it was dragged after past it (#165).
                let (guid, others, at) = (self.guid, &self.others, mob.position);
                let home = mob.home;
                let strayed = mob.leashes && leash::beyond(home, at);
                let me_alive = self.player.alive();
                let reach = |g: u64| {
                    let p = if g == guid {
                        me_alive.then_some(me)
                    } else {
                        others.iter().find(|o| o.0 == g).map(|o| o.1)
                    };
                    p.filter(|&p| !strayed || !leash::beyond(home, p))
                        .map(|p| distance(at, p) <= melee_reach())
                };
                mob.threat.retain(|g| reach(g).is_some());
                let next = mob.threat.pick(target, reach);
                if let Some(next) = next.filter(|&n| n != target) {
                    engage(mob, next, events);
                    return;
                }
                // It gives up only with no one left on its table.
                if next.is_none() {
                    let camp = mob.camp;
                    self.evade_camp(camp, target, route.map(|r| (r, map, host)), events);
                    return;
                }
                mob.engaged_secs += dt;
                if self.skills.stunned(mob.guid) {
                    interrupt_cast(mob, events);
                    return;
                }
                if self.mob_cast(i, dt, me, c.orientation, host, rng, events) {
                    return;
                }
                let mob = &mut self.mobs[i];
                // A creature with no melee casts from where it stands and never swings.
                if !mob.template.melee {
                    return;
                }
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
                    * mob.attack_time_mul
                    * self.skills.attack_time_mul(mob.guid);
                if swing.damage > 0 {
                    let attacker = mob.guid;
                    self.player_takes(
                        attacker,
                        swing.damage,
                        crate::rules::school::School::Physical,
                        events,
                    );
                    if crate::wear::on_hit(c, &mut self.wear, events) {
                        self.refresh_gear(c, events);
                    }
                }
                let attacker = self.mobs[i].guid;
                self.struck(swing.outcome, attacker, c, rng, events, ticked);
            }
        }
    }

    /// The gazes that went off here since the realm last asked, and the raid warnings of the
    /// casts that began.
    pub(crate) fn take_gazes(&mut self) -> (Vec<Gaze>, Vec<String>, Vec<Foreign>, Vec<Scenery>) {
        (
            std::mem::take(&mut self.gazes),
            std::mem::take(&mut self.warnings),
            std::mem::take(&mut self.foreign),
            std::mem::take(&mut self.scenery),
        )
    }

    /// A Creature Spell from another session's creature lands on the player.
    pub(crate) fn spell_from_afar(&mut self, cast: &Foreign, events: &mut Vec<SessionEvent>) {
        if self.player.alive() {
            self.creature_spell_lands(cast.caster, cast.level, cast.from, cast.spell, events);
        }
    }

    /// A gaze from the Phase lands on the player standing at `at`: `damage` unless they are
    /// dead, beyond its radius or behind one of the dungeon's `cover` pillars.
    pub(crate) fn gazed(
        &mut self,
        gaze: &Gaze,
        at: Option<[f32; 3]>,
        cover: &[crate::phased::Cover],
        events: &mut Vec<SessionEvent>,
    ) {
        let Some(at) = at.filter(|_| self.player.alive()) else {
            return;
        };
        let shielded = !gaze.through_cover && crate::phased::covered(cover, gaze.at, at);
        if distance(gaze.at, at) > gaze.radius || shielded {
            return;
        }
        if gaze.root_ms > 0 {
            self.start_root(gaze.spell_id, gaze.root_ms, events);
            return;
        }
        events.push(SessionEvent::SpellDamageLog(
            benilla_protocol::messages::SpellDamageLog {
                target: self.guid,
                attacker: gaze.caster,
                spell_id: gaze.spell_id,
                damage: gaze.damage,
                school: gaze.school.index(),
                absorb: 0,
                resist: 0,
                periodic: false,
                blocked: 0,
                hit_info: 0,
            },
        ));
        self.player_takes(gaze.caster, gaze.damage, gaze.school, events);
    }

    /// The player loses `damage` to a creature's blow or spell: rage from it, and a timed cast
    /// broken (the Hearthstone's interrupts on damage).
    fn player_takes(
        &mut self,
        from: u64,
        damage: u32,
        school: crate::rules::school::School,
        events: &mut Vec<SessionEvent>,
    ) {
        if damage > 0 {
            self.last_hit_by = Some(from);
        }
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

    /// The player loses `damage` from `from`: as [`Self::player_takes`], save that the foe's
    /// blow that would kill leaves them at 1 health, beaten.
    pub(crate) fn player_takes_from(
        &mut self,
        from: u64,
        damage: u32,
        school: crate::rules::school::School,
        events: &mut Vec<SessionEvent>,
    ) {
        let spared = Some(from) == self.foe;
        let damage = if spared {
            damage.min(self.player.health.saturating_sub(1))
        } else {
            damage
        };
        self.player_takes(from, damage, school, events);
        if spared && self.player.health <= 1 {
            self.beaten_by = Some(from);
        }
    }

    /// `mob` dies at the player's hand: the fight ends, and the kill is credited, here and now
    /// when alone, or by the realm to everyone near a shard's mob.
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
        let boss = Self::is_boss(mob);
        let drops = match mob.spawn_row() {
            Some(spawn) if matches!(spawn.phase, crate::phased::Phase::Dungeon(_)) => {
                crate::phased::drops(spawn.map, spawn.guid)
            }
            _ => &[],
        };
        let engaged = mob.state == State::Engaged;
        mob.state = State::Dead;
        mob.walk = None;
        drop_cast(mob, events);
        // A Phased Dungeon's dead stay down until its Run resets; a mob no spawn placed never
        // comes back.
        if self.phase == crate::phased::Phase::OpenWorld && mob.spawn_row().is_some() {
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
        events.push(values(mob.guid, &pairs));

        let elite = if mob.template.elite() {
            elite_kind
        } else {
            xp::Elite::No
        };
        let (victim, base_xp) = (mob.guid, xp::kill_xp(c.level, mob.level(), elite));
        let money = if self.lent {
            let (lo, hi) = crate::rules::loot::money(mob.level());
            rng.urand(lo, hi)
        } else {
            0
        };
        let kill = Kill {
            mob: victim,
            entry,
            level: mob.level(),
            elite,
            boss,
            drops,
            money,
            position: mob.position,
            bounty: mob.bounty,
        };
        if self.lent {
            self.kills.push(kill);
        } else {
            let credit = Credit {
                money: None,
                levels: vec![c.level],
                kill,
            };
            self.take_credit(credit, c, rng, events, ticked);
        }
        self.proc(
            crate::rules::procs::Happened {
                was_ms: 0,
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
    }

    /// What this session cast on other players since it was last asked, to deliver.
    pub(crate) fn take_helps(&mut self) -> Vec<Help> {
        let auras = self.take_sent().into_iter().map(|aura| Help::Aura {
            to: aura.unit(),
            aura,
        });
        let mut helps = std::mem::take(&mut self.helps);
        helps.extend(auras);
        helps
    }

    /// The player's part in a kill: their own roll of its loot with their share of its money,
    /// their share of its XP by the party formula, and its quest credit.
    pub(crate) fn take_credit(
        &mut self,
        credit: Credit,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let k = &credit.kill;
        let mut loot = if k.boss {
            let pool = crate::dungeon::boss_drops(c.level, c.race);
            looting::Loot::boss(k.level, &pool, rng)
        } else {
            looting::Loot::roll(k.level, rng)
        };
        if let Some(money) = credit.money {
            loot.money = money;
        }
        loot.items
            .extend(k.drops.iter().map(|&item| Some((item, 1))));
        // A Bounty Hunter pays everyone it credits in Blood Coin; its owner's Bounty Rank rises,
        // or, if it was the victor they are Marked by, the mark lifts (the realm lifts it too when
        // someone else kills it).
        if let Some(b) = k.bounty {
            let coin = crate::rules::notoriety::blood_coin();
            loot.items
                .push(Some((coin, crate::rules::notoriety::coin_drop(b.rank))));
            if b.owner == c.guid && b.victor {
                c.marked = None;
            } else if b.owner == c.guid {
                c.bounty_rank += 1;
            }
        }
        // The quest items the player's quests still want, after the corpse's own drop.
        loot.items
            .extend(crate::quests::drops(c, k.entry, rng).into_iter().map(Some));
        let gain = crate::rules::notoriety::kill_gain(c.level, k.level, k.elite, k.boss)
            + loot
                .items
                .iter()
                .flatten()
                .filter_map(|&(item, _)| crate::items::template(item))
                .map(|t| crate::rules::notoriety::drop_gain(t.quality))
                .sum::<f32>();
        self.notorious(c, gain);
        let lootable = self.looting.drop(k.mob, loot);
        events.push(values(k.mob, &[looting::lootable_pair(lootable)]));
        let base = xp::party_kill_xp(&credit.levels, c.level, k.level, k.elite);
        let gained = self.scaled_xp(c, base);
        if gained > 0 {
            // Rested XP doubles the kill, multiplied XP and all, while the pool lasts.
            let rested = crate::rest::spend(c, gained);
            events.push(SessionEvent::XpGain(XpGain {
                victim: k.mob,
                total: gained + rested,
                base: gained,
                kill: true,
            }));
            if rested > 0 {
                events.push(values(self.guid, &crate::rest::pairs(c)));
            }
            ticked.leveled |= self.give_xp(c, gained + rested, events);
        }
        crate::quests::credit_kill(c, k.entry, events);
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

    /// What an award of `base` XP pays `c`: times the Tier XP Rate, then the Heat Level's
    /// multiplier, then the dev multiplier; nothing at the Tier Cap.
    fn scaled_xp(&self, c: &Char, base: u32) -> u32 {
        if at_tier_cap(c) {
            0
        } else {
            let heat = crate::rules::notoriety::heat_level(c.notoriety);
            self.dev.xp(crate::rules::notoriety::with_heat(
                at_tier_rate(c, base),
                heat,
            ))
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
        // A mob held by an incapacitate is still in the fight but is no threat to the player.
        let attacked = self.mobs.iter().any(|m| {
            m.state == State::Engaged && m.target == self.guid && !self.skills.incapacitated(m.guid)
        });
        let in_combat = p.alive() && (attacking || attacked || self.skills.holds_combat());
        if in_combat != p.in_combat {
            p.in_combat = in_combat;
            events.push(values(
                self.guid,
                &[(
                    UNIT_FIELD_FLAGS,
                    fear::unit_flags(in_combat, self.fear.is_some()),
                )],
            ));
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

/// `mob` turns on the player, who joins its threat table with the pull seed if not on it:
/// `SMSG_AI_REACTION`, `SMSG_ATTACKSTART`, and its target and combat
/// flag.
pub(crate) fn engage(mob: &mut Mob, player: u64, events: &mut Vec<SessionEvent>) {
    // A dummy never turns on anyone.
    if mob.template.inert {
        return;
    }
    let seed = mob.stats.max_health as f32 * crate::group::get().pull_threat_health_fraction;
    mob.threat.seed(player, seed);
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

/// `mob` loses `damage` of `school` from the player `from`. A stand-in's health stops at 1 and
/// the damage goes into `harms`, for the session whose player it stands in for.
pub(crate) fn wound(
    mob: &mut Mob,
    damage: u32,
    from: u64,
    school: crate::rules::school::School,
    harms: &mut Vec<Harm>,
    events: &mut Vec<SessionEvent>,
) {
    // A dummy's health stops at 1; the clock to its reset starts over with every hit.
    if mob.template.inert {
        mob.health = mob.health.saturating_sub(damage).max(1);
        if damage > 0 {
            mob.since_hurt = Some(0.0);
            events.push(values(mob.guid, &[(UNIT_FIELD_HEALTH, mob.health)]));
        }
        return;
    }
    if mob.remote {
        mob.health = mob.health.saturating_sub(damage).max(1);
        if damage > 0 {
            harms.push(Harm {
                to: mob.guid,
                from,
                amount: damage,
                school,
            });
        }
        return;
    }
    mob.health = mob.health.saturating_sub(damage);
    if damage > 0 {
        events.push(values(mob.guid, &[(UNIT_FIELD_HEALTH, mob.health)]));
    }
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
    mob.cast_at = None;
    events.push(SessionEvent::SpellFailedOther {
        caster: mob.guid,
        spell_id: spell.spell_id,
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
        // The ranged block; a character without a bow, gun or crossbow shows zero damage.
        (
            UNIT_FIELD_RANGED_ATTACK_POWER,
            s.ranged_attack_power.max(0) as u32,
        ),
        (
            UNIT_FIELD_RANGEDATTACKTIME,
            s.ranged.map_or(0, |r| {
                crate::rules::haste::hasted_attack_time(r.attack_time_ms, s.quiver_haste_pct)
            }),
        ),
        (
            UNIT_FIELD_MINRANGEDDAMAGE,
            float(s.ranged.map_or(0.0, |r| r.min_damage)),
        ),
        (
            UNIT_FIELD_MAXRANGEDDAMAGE,
            float(s.ranged.map_or(0.0, |r| r.max_damage)),
        ),
        (crate::ammo::PLAYER_AMMO_ID, c.inventory.ammo),
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
    let kind = crate::callings::of(c.calling).power;
    let mut pairs = vec![
        (OBJECT_FIELD_TYPE, TYPE_PLAYER),
        (OBJECT_FIELD_SCALE_X, float(1.0)),
        (UNIT_FIELD_HEALTH, stats.max_health),
        power.field(),
        (UNIT_FIELD_MAXPOWER1, stats.max_mana),
        (UNIT_FIELD_MAXPOWER2, rage::MAX),
        (
            UNIT_FIELD_MAXPOWER1 + PowerKind::Focus.id() as u16,
            power_rules::FOCUS_MAX,
        ),
        (
            UNIT_FIELD_MAXPOWER1 + PowerKind::Energy.id() as u16,
            power_rules::ENERGY_MAX + stats.max_energy_bonus,
        ),
        (UNIT_FIELD_FACTIONTEMPLATE, content::PLAYER_FACTION_TEMPLATE),
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
