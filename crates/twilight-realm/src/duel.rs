//! **Duels** (spec #152, `content/duel.toml`): vanilla's duel between two players. A challenge
//! is a cast of the duel spell at a player; both are told (`SMSG_DUEL_REQUESTED`), the challenged
//! accepts, both count down (`SMSG_DUEL_COUNTDOWN`) and then fight. Each player is in one duel at
//! most. A duel ends through one path: a cancel before the fight is told as such
//! (`SMSG_DUEL_COMPLETE` 0), and a fight ends with a winner (`SMSG_DUEL_WINNER`, then
//! `SMSG_DUEL_COMPLETE` 1).
//!
//! A duel is state the realm keeps on its two players, as a trade is ([`crate::trade`]).

use std::sync::OnceLock;

use benilla_protocol::SessionEvent;
use serde::Deserialize;

use crate::content::PLAYER_FACTION_TEMPLATE;
use crate::fields::{guid_pairs, values, UNIT_FIELD_FACTIONTEMPLATE};
use crate::mobs::Mob;
use crate::world::InWorld;
use crate::{Realm, RealmCommand, SessionId};

/// `HIGHGUID_GAMEOBJECT`, in the duel flag's guid's top 16 bits.
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
/// Where the duel flags' guid counters start, clear of the Phases' GameObjects.
const ARBITER_BASE: u64 = 0x00F0_0000;
/// `PLAYER_DUEL_ARBITER` (a guid) and `PLAYER_DUEL_TEAM` (1 the challenger, 2 the challenged,
/// once the fight is on).
const PLAYER_DUEL_ARBITER: u16 = 188;
const PLAYER_DUEL_TEAM: u16 = 196;

// `SpellCastResult` codes a refused challenge answers with.
const SPELL_FAILED_BAD_TARGETS: u8 = 0x0a;
const SPELL_FAILED_CASTER_DEAD: u8 = 0x13;
const SPELL_FAILED_OUT_OF_RANGE: u8 = 0x59;
const SPELL_FAILED_TARGETS_DEAD: u8 = 0x65;
/// `SPELL_FAILED_TARGET_DUELING`: either player is in a duel already.
const SPELL_FAILED_TARGET_DUELING: u8 = 0x68;

/// `content/duel.toml`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Content {
    pub(crate) challenge_spell: u32,
    pub(crate) countdown_secs: u32,
    pub(crate) request_range_yd: f32,
    pub(crate) hostile_faction: u32,
    pub(crate) flag: Flag,
}

/// `content/duel.toml`'s `[flag]`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Flag {
    pub(crate) entry: u32,
    pub(crate) display_id: u32,
    pub(crate) type_id: u32,
    pub(crate) faction: u32,
    pub(crate) bounds_yd: f32,
    pub(crate) out_of_bounds_secs: f32,
}

pub(crate) fn content() -> &'static Content {
    static CONTENT: OnceLock<Content> = OnceLock::new();
    CONTENT.get_or_init(|| {
        toml::from_str(include_str!("../content/duel.toml")).expect("content/duel.toml parses")
    })
}

/// The duels under way, and the last flag guid handed out.
#[derive(Default)]
pub(crate) struct Duels {
    duels: Vec<Duel>,
    last_arbiter: u64,
}

/// One duel: the challenger and the challenged, its flag's guid, and how far it has come.
pub(crate) struct Duel {
    pub(crate) players: [u64; 2],
    pub(crate) arbiter: u64,
    pub(crate) state: State,
    /// The spells of the auras each player wears from the other, by wearer: what comes off when
    /// it ends.
    worn: [Vec<u32>; 2],
    /// Where its flag stands, and on which map, once raised.
    flag: Option<(u32, [f32; 3])>,
    /// Seconds each player has left out of bounds before they lose, while they are out.
    out: [Option<f32>; 2],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum State {
    /// Challenged, not yet answered.
    Requested,
    /// Accepted: the fight starts when the seconds left run out.
    Countdown(f32),
    Fighting,
}

impl Duel {
    fn side_of(&self, guid: u64) -> Option<usize> {
        self.players.iter().position(|&p| p == guid)
    }
}

/// A player in the world, as a duel sees them.
struct Duellist {
    session: SessionId,
    guid: u64,
    map: u32,
    position: [f32; 3],
    alive: bool,
}

impl Duels {
    /// The duel `guid` is in, by index.
    fn of(&self, guid: u64) -> Option<usize> {
        self.duels.iter().position(|d| d.side_of(guid).is_some())
    }

    /// The player `guid` fights in a duel, once the countdown is over.
    pub(crate) fn foe_of(&self, guid: u64) -> Option<u64> {
        let duel = &self.duels[self.of(guid)?];
        (duel.state == State::Fighting).then(|| duel.players.into_iter().find(|&p| p != guid))?
    }

    /// `from` put an aura of `spell` on `to`: when they are fighting each other, it comes off at
    /// the end.
    pub(crate) fn note_worn(&mut self, from: u64, to: u64, spell: u32) {
        if self.foe_of(to) != Some(from) {
            return;
        }
        let Some(i) = self.of(to) else {
            return;
        };
        let duel = &mut self.duels[i];
        if let Some(side) = duel.side_of(to) {
            duel.worn[side].push(spell);
        }
    }
}

impl Realm {
    /// Everyone in the world, the served session included.
    fn duellists(&self) -> Vec<Duellist> {
        let worlds = std::iter::once((self.active, &self.world))
            .chain(self.parked.iter().map(|(id, s)| (*id, &s.world)));
        worlds
            .filter_map(|(session, w)| {
                let w = w.as_ref()?;
                let c = self.characters.get(w.guid)?;
                Some(Duellist {
                    session,
                    guid: c.guid,
                    map: c.map,
                    position: c.position,
                    alive: w.alive(),
                })
            })
            .collect()
    }

    fn send_to_player(&mut self, guid: u64, event: SessionEvent) {
        let session = self
            .duellists()
            .into_iter()
            .find(|d| d.guid == guid)
            .map(|d| d.session);
        if let Some(session) = session {
            self.send_to(session, event);
        }
    }

    /// The served player casts the duel spell at `target`: a challenge, or its refusal.
    pub(crate) fn challenge(&mut self, target: Option<u64>) {
        let Some(me) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        let spell_id = content().challenge_spell;
        let all = self.duellists();
        let Some(mine) = all.iter().find(|d| d.guid == me) else {
            return;
        };
        let them = target.and_then(|t| all.iter().find(|d| d.guid == t && t != me));
        let refusal = match them {
            _ if !mine.alive => Some(SPELL_FAILED_CASTER_DEAD),
            None => Some(SPELL_FAILED_BAD_TARGETS),
            Some(_) if self.duels.of(me).is_some() => Some(SPELL_FAILED_TARGET_DUELING),
            Some(t) if self.duels.of(t.guid).is_some() => Some(SPELL_FAILED_TARGET_DUELING),
            Some(t) if !t.alive => Some(SPELL_FAILED_TARGETS_DEAD),
            Some(t) if !in_range(mine, t) => Some(SPELL_FAILED_OUT_OF_RANGE),
            Some(_) => None,
        };
        let result = SessionEvent::CastResult {
            spell_id,
            success: refusal.is_none(),
            reason: refusal,
            arg: None,
        };
        self.events.push(result);
        let Some(them) = them.filter(|_| refusal.is_none()) else {
            return;
        };
        let target = them.guid;
        self.duels.last_arbiter += 1;
        let arbiter = HIGHGUID_GAMEOBJECT << 48 | (ARBITER_BASE + self.duels.last_arbiter);
        self.duels.duels.push(Duel {
            players: [me, target],
            arbiter,
            state: State::Requested,
            worn: Default::default(),
            flag: None,
            out: [None; 2],
        });
        let requested = SessionEvent::DuelRequested {
            arbiter,
            challenger: me,
        };
        self.send_to_player(me, requested.clone());
        self.send_to_player(target, requested);
        // Both carry the flag's guid from the challenge on (vmangos `Spell::EffectDuel`).
        for p in [me, target] {
            self.show_player_fields(p, &guid_pairs(PLAYER_DUEL_ARBITER, arbiter));
        }
    }

    /// Sends the player `guid`'s `pairs` to every client on their map: their own, and every other
    /// that may see them.
    fn show_player_fields(&mut self, guid: u64, pairs: &[(u16, u32)]) {
        let all = self.duellists();
        let Some(map) = all.iter().find(|d| d.guid == guid).map(|d| d.map) else {
            return;
        };
        let event = values(guid, pairs);
        for d in all.iter().filter(|d| d.map == map) {
            self.send_to(d.session, event.clone());
        }
    }

    /// Duel `i` was accepted: its flag goes up midway between the two, seen by everyone near.
    fn raise_flag(&mut self, i: usize) {
        let all = self.duellists();
        let duel = &self.duels.duels[i];
        let [Some(a), Some(b)] = duel.players.map(|g| all.iter().find(|d| d.guid == g)) else {
            return;
        };
        let mid = std::array::from_fn(|k| (a.position[k] + b.position[k]) / 2.0);
        let f = &content().flag;
        let create = crate::phased::gameobject_create(&crate::phased::GameObject {
            guid: duel.arbiter,
            entry: f.entry,
            display_id: f.display_id,
            type_id: f.type_id,
            flags: 0,
            faction: f.faction,
            position: mid,
            orientation: 0.0,
        });
        let map = a.map;
        self.duels.duels[i].flag = Some((map, mid));
        let near: Vec<SessionId> = all
            .iter()
            .filter(|d| {
                d.map == map
                    && crate::space::flat_distance(d.position, mid) <= crate::population::VIEW_YD
            })
            .map(|d| d.session)
            .collect();
        for session in near {
            self.send_to(session, create.clone());
        }
    }

    /// A duel window command from the served player.
    pub(crate) fn handle_duel(&mut self, command: RealmCommand) {
        let Some(me) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        let Some(i) = self.duels.of(me) else {
            return;
        };
        let duel = &mut self.duels.duels[i];
        match command {
            // The challenger's client accepts its own challenge too; only the answer counts.
            RealmCommand::DuelAccept { arbiter } => {
                if arbiter != duel.arbiter
                    || duel.state != State::Requested
                    || duel.side_of(me) != Some(1)
                {
                    return;
                }
                let secs = content().countdown_secs;
                duel.state = State::Countdown(secs as f32);
                let players = duel.players;
                self.raise_flag(i);
                for p in players {
                    self.send_to_player(p, SessionEvent::DuelCountdown { seconds: secs });
                }
            }
            // A decline, a cancel or, once fighting, a yield: the one who sent it loses.
            RealmCommand::DuelCancel { arbiter } => {
                if arbiter != duel.arbiter {
                    return;
                }
                let winner = duel.players.into_iter().find(|&p| p != me);
                self.end_duel(i, winner, false);
            }
            _ => {}
        }
    }

    /// Duel `i` ends. A fight ends with `winner` (`fled` when the loser ran off) told to everyone
    /// on the map; a duel that never got to the fight is only called off.
    pub(crate) fn end_duel(&mut self, i: usize, winner: Option<u64>, fled: bool) {
        let duel = self.duels.duels.remove(i);
        let started = duel.state == State::Fighting;
        if started {
            let all = self.duellists();
            // From the roster: one who logged out has a name still.
            let name = |g: Option<u64>| {
                g.and_then(|g| self.characters.get(g))
                    .map(|c| c.name.clone())
                    .unwrap_or_default()
            };
            let loser = duel.players.into_iter().find(|&p| Some(p) != winner);
            let line = SessionEvent::DuelWinner {
                fled,
                winner: name(winner),
                loser: name(loser),
            };
            // Everyone on the flag's map, and both of them wherever they are.
            let map = duel.flag.map(|(map, _)| map);
            let seen: Vec<SessionId> = all
                .iter()
                .filter(|d| Some(d.map) == map || duel.side_of(d.guid).is_some())
                .map(|d| d.session)
                .collect();
            for session in seen {
                self.send_to(session, line.clone());
            }
            // Vanilla's cleanup: each stops fighting the other, the auras each put on the other
            // come off, and each sees the other friendly again. Health, power and cooldowns stay.
            for (side, &p) in duel.players.iter().enumerate() {
                let foe = duel.players[1 - side];
                let worn = &duel.worn[side];
                self.on_world(p, |w, events| {
                    w.part_from(foe, worn, events);
                    events.push(values(
                        foe,
                        &[(UNIT_FIELD_FACTIONTEMPLATE, PLAYER_FACTION_TEMPLATE)],
                    ));
                });
            }
        }
        // The flag comes down, and neither carries it, or a side, any more.
        if let Some((map, _)) = duel.flag {
            let gone = SessionEvent::ObjectDestroyed(duel.arbiter);
            let on_map: Vec<SessionId> = self
                .duellists()
                .iter()
                .filter(|d| d.map == map)
                .map(|d| d.session)
                .collect();
            for session in on_map {
                self.send_to(session, gone.clone());
            }
        }
        for p in duel.players {
            let [lo, hi] = guid_pairs(PLAYER_DUEL_ARBITER, 0);
            self.show_player_fields(p, &[lo, hi, (PLAYER_DUEL_TEAM, 0)]);
            self.send_to_player(p, SessionEvent::DuelComplete { started });
        }
    }

    /// Runs `f` on the world of the player `guid`, served or parked, with that session's events.
    fn on_world(&mut self, guid: u64, f: impl FnOnce(&mut InWorld, &mut Vec<SessionEvent>)) {
        if let Some(w) = self.world.as_mut().filter(|w| w.guid == guid) {
            return f(w, &mut self.events);
        }
        for s in self.parked.values_mut() {
            if let Some(w) = s.world.as_mut().filter(|w| w.guid == guid) {
                return f(w, &mut s.events);
            }
        }
    }

    /// The served player fights a duel: their foe stands in their world, as they stand now, so
    /// every blow and spell lands on them as on a mob (`Mob::remote`).
    pub(crate) fn seat_foes(&mut self) {
        let Some(me) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        let foe = self.duels.foe_of(me);
        let stand_in = foe.and_then(|foe| {
            let w = self
                .parked
                .values()
                .filter_map(|s| s.world.as_ref())
                .find(|w| w.guid == foe)?;
            let c = self.characters.get(foe)?;
            let (stats, health) = w.stand_in_stats();
            Some(Mob::stand_in(
                foe,
                stats,
                health,
                c.position,
                c.orientation,
                me,
            ))
        });
        let Some(w) = self.world.as_mut() else {
            return;
        };
        w.mobs.retain(|m| !m.remote);
        w.foe = foe;
        w.mobs.extend(stand_in);
    }

    /// The served session is parked or hands its mobs back: its foe's stand-in goes, and the
    /// damage dealt to it goes to the foe's session, to take on its next tick.
    pub(crate) fn unseat_foes(&mut self) {
        let Some(w) = self.world.as_mut() else {
            return;
        };
        w.mobs.retain(|m| !m.remote);
        for harm in std::mem::take(&mut w.harms) {
            let target = self
                .parked
                .values_mut()
                .filter_map(|s| s.world.as_mut())
                .find(|w| w.guid == harm.to);
            if let Some(t) = target {
                t.harmed.push(harm);
            }
        }
    }

    /// Each tick: a countdown runs down to the fight, an unanswered challenge lapses when the two
    /// stray out of range, and a duel ends when one of its players is gone: called off before
    /// the fight, lost by the one who left during it.
    pub(crate) fn tick_duels(&mut self, dt: f32) {
        // A foe's blow left a player at 1 health: the foe wins.
        let worlds = std::iter::once(&mut self.world)
            .chain(self.parked.values_mut().map(|s| &mut s.world))
            .flatten();
        let beaten: Vec<(u64, u64)> = worlds
            .filter_map(|w| w.beaten_by.take().map(|by| (w.guid, by)))
            .collect();
        for (loser, winner) in beaten {
            if self.duels.foe_of(loser) == Some(winner) {
                if let Some(i) = self.duels.of(loser) {
                    self.end_duel(i, Some(winner), false);
                }
            }
        }

        let all = self.duellists();
        let mut ended: Vec<(usize, Option<u64>, bool)> = Vec::new();
        let mut fights: Vec<[u64; 2]> = Vec::new();
        let mut bounds: Vec<(u64, SessionEvent)> = Vec::new();
        let f = &content().flag;
        for (i, duel) in self.duels.duels.iter_mut().enumerate() {
            let [a, b] = duel.players.map(|g| all.iter().find(|d| d.guid == g));
            let (Some(a), Some(b)) = (a, b) else {
                let stayed = [a, b].into_iter().flatten().map(|d| d.guid).next();
                ended.push((i, stayed, false));
                continue;
            };
            match duel.state {
                State::Requested if !in_range(a, b) => ended.push((i, None, false)),
                // Death to anything but the foe, or leaving the map: called off before the
                // fight, lost during it.
                State::Countdown(_) if !a.alive || !b.alive || a.map != b.map => {
                    ended.push((i, None, false));
                }
                State::Fighting if !a.alive || !b.alive => {
                    let winner = if a.alive { a.guid } else { b.guid };
                    ended.push((i, Some(winner), false));
                }
                State::Fighting if a.map != b.map => {
                    let flag_map = duel.flag.map(|(map, _)| map);
                    let winner = if Some(a.map) == flag_map {
                        a.guid
                    } else {
                        b.guid
                    };
                    ended.push((i, Some(winner), true));
                }
                State::Countdown(left) if left <= dt => {
                    duel.state = State::Fighting;
                    fights.push(duel.players);
                }
                State::Countdown(left) => duel.state = State::Countdown(left - dt),
                State::Fighting => {
                    // Out of the flag's bounds: warned, and beaten as fled once the time is up.
                    let Some((map, at)) = duel.flag else {
                        continue;
                    };
                    for (side, d) in [a, b].into_iter().enumerate() {
                        let away = d.map != map
                            || crate::space::flat_distance(d.position, at) > f.bounds_yd;
                        match (away, duel.out[side]) {
                            (true, None) => {
                                duel.out[side] = Some(f.out_of_bounds_secs);
                                bounds.push((d.guid, SessionEvent::DuelOutOfBounds));
                            }
                            (true, Some(left)) if left <= dt => {
                                let winner = duel.players[1 - side];
                                ended.push((i, Some(winner), true));
                                break;
                            }
                            (true, Some(left)) => duel.out[side] = Some(left - dt),
                            (false, Some(_)) => {
                                duel.out[side] = None;
                                bounds.push((d.guid, SessionEvent::DuelInBounds));
                            }
                            (false, None) => {}
                        }
                    }
                }
                _ => {}
            }
        }
        for (p, event) in bounds {
            self.send_to_player(p, event);
        }
        // The fight is on: each client sees the other hostile, and both on their sides.
        let hostile = content().hostile_faction;
        for [a, b] in fights {
            for (p, foe) in [(a, b), (b, a)] {
                let seen_hostile = values(foe, &[(UNIT_FIELD_FACTIONTEMPLATE, hostile)]);
                self.send_to_player(p, seen_hostile);
            }
            self.show_player_fields(a, &[(PLAYER_DUEL_TEAM, 1)]);
            self.show_player_fields(b, &[(PLAYER_DUEL_TEAM, 2)]);
        }
        // Highest first, so each index still names its duel.
        for (i, winner, fled) in ended.into_iter().rev() {
            self.end_duel(i, winner, fled);
        }
    }
}

/// Whether two players stand close enough to challenge: the same map, within the range.
fn in_range(a: &Duellist, b: &Duellist) -> bool {
    a.map == b.map
        && crate::space::flat_distance(a.position, b.position) <= content().request_range_yd
}

impl RealmCommand {
    /// Whether this is one of the duel window's commands.
    pub(crate) fn is_duel(&self) -> bool {
        matches!(
            self,
            RealmCommand::DuelAccept { .. } | RealmCommand::DuelCancel { .. }
        )
    }
}
