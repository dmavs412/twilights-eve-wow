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

use crate::{Realm, RealmCommand, SessionId};

/// `HIGHGUID_GAMEOBJECT`, in the duel flag's guid's top 16 bits.
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
/// Where the duel flags' guid counters start, clear of the Phases' GameObjects.
const ARBITER_BASE: u64 = 0x00F0_0000;

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
    name: String,
    map: u32,
    position: [f32; 3],
    alive: bool,
}

impl Duels {
    /// The duel `guid` is in, by index.
    fn of(&self, guid: u64) -> Option<usize> {
        self.duels.iter().position(|d| d.side_of(guid).is_some())
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
                    name: c.name.clone(),
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
        });
        let requested = SessionEvent::DuelRequested {
            arbiter,
            challenger: me,
        };
        self.send_to_player(me, requested.clone());
        self.send_to_player(target, requested);
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
            let name = |g: Option<u64>| {
                all.iter()
                    .find(|d| Some(d.guid) == g)
                    .map(|d| d.name.clone())
                    .unwrap_or_default()
            };
            let loser = duel.players.into_iter().find(|&p| Some(p) != winner);
            let line = SessionEvent::DuelWinner {
                fled,
                winner: name(winner),
                loser: name(loser),
            };
            let map = all
                .iter()
                .find(|d| duel.side_of(d.guid).is_some())
                .map(|d| d.map);
            let seen: Vec<SessionId> = all
                .iter()
                .filter(|d| Some(d.map) == map)
                .map(|d| d.session)
                .collect();
            for session in seen {
                self.send_to(session, line.clone());
            }
        }
        for p in duel.players {
            self.send_to_player(p, SessionEvent::DuelComplete { started });
        }
    }

    /// Each tick: a countdown runs down to the fight, an unanswered challenge lapses when the two
    /// stray out of range, and a duel ends when one of its players is gone: called off before
    /// the fight, lost by the one who left during it.
    pub(crate) fn tick_duels(&mut self, dt: f32) {
        let all = self.duellists();
        let mut ended: Vec<(usize, Option<u64>)> = Vec::new();
        for (i, duel) in self.duels.duels.iter_mut().enumerate() {
            let [a, b] = duel.players.map(|g| all.iter().find(|d| d.guid == g));
            let (Some(a), Some(b)) = (a, b) else {
                let stayed = [a, b].into_iter().flatten().map(|d| d.guid).next();
                ended.push((i, stayed));
                continue;
            };
            match duel.state {
                State::Requested if !in_range(a, b) => ended.push((i, None)),
                State::Countdown(left) if left <= dt => duel.state = State::Fighting,
                State::Countdown(left) => duel.state = State::Countdown(left - dt),
                _ => {}
            }
        }
        // Highest first, so each index still names its duel.
        for (i, winner) in ended.into_iter().rev() {
            self.end_duel(i, winner, false);
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
