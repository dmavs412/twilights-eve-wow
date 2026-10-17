//! **Rested XP**: an inn is a rest area (`content/hub.toml`, the innkeeper's `rest_radius`). Time
//! spent standing in one, or logged out anywhere, banks a pool of rested XP, and a kill pays its
//! XP again out of the pool until it runs out. The rules are vmangos' (`Player::SetRestBonus`,
//! `Player::GetXPRestBonus`, the offline rest in `Player::LoadFromDB` and the resting tick in
//! `Player::Update`).

use crate::characters::Char;
use crate::content::KALIMDOR;
use crate::fields::{PLAYER_BYTES_2, PLAYER_FLAGS, PLAYER_REST_STATE_EXPERIENCE};
use crate::rules::xp;
use crate::space::distance;

/// A level's XP banks over this many seconds at a bubble of 1.
const LEVEL_SECONDS: f32 = 72000.0;
/// The bubble in an inn, logged in or out: a level's 5% every 8 hours.
const IN_INN: f32 = 0.125;
/// The bubble logged out anywhere else.
const OUTSIDE: f32 = 0.031;
/// The pool holds at most three quarters of the level's XP: a level and a half of doubled kills.
const CAP: f32 = 1.5 / 2.0;

/// `PLAYER_BYTES_2`'s top byte: the purple bar, and the stock "You feel rested" line on the way in.
const REST_STATE_RESTED: u32 = 0x01;
const REST_STATE_NORMAL: u32 = 0x02;
/// `PLAYER_FLAGS_RESTING`: the portrait's resting glow and the "Resting" XP bar tooltip.
pub(crate) const PLAYER_FLAGS_RESTING: u32 = 0x20;

/// A character's rest: the pool, and where and when they last left the world.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Rest {
    /// Rested XP banked, in XP.
    pub(crate) pool: f32,
    /// When they left, on the wall clock, and whether in an inn: the time since banks on login.
    pub(crate) left: Option<Left>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Left {
    pub(crate) at: f64,
    pub(crate) in_inn: bool,
}

/// Whether `position` on `map` stands in an inn.
pub(crate) fn in_inn(map: u32, position: [f32; 3]) -> bool {
    map == KALIMDOR
        && crate::hub::npcs().iter().any(|n| {
            n.rest_radius
                .is_some_and(|r| distance(n.position, position) <= r)
        })
}

/// The most `c`'s pool holds: nothing at the Tier Cap or the end of the game.
fn cap(c: &Char) -> f32 {
    match xp::xp_to_next(c.level) {
        Some(next) if c.level < crate::callings::of(c.calling).cap => next as f32 * CAP,
        _ => 0.0,
    }
}

/// Banks `seconds` of rest at `bubble`, up to the cap.
fn bank(c: &mut Char, seconds: f32, bubble: f32) {
    let next = xp::xp_to_next(c.level).unwrap_or(0) as f32;
    c.rest.pool = (c.rest.pool + seconds * next / LEVEL_SECONDS * bubble).min(cap(c));
}

/// Time standing in an inn: `seconds` at the inn's bubble.
pub(crate) fn rest_in_inn(c: &mut Char, seconds: f32) {
    bank(c, seconds, IN_INN);
}

/// Logging in at the wall clock's `wall`: the time away banks, at the inn's bubble if they left
/// in one, and the stamp moves up to now, so a session that ends without a save banks it once.
/// A clock that runs behind the leaving banks nothing.
pub(crate) fn come_back(c: &mut Char, wall: f64) {
    if let Some(left) = &mut c.rest.left {
        let away = (wall - left.at).max(0.0) as f32;
        let bubble = if left.in_inn { IN_INN } else { OUTSIDE };
        left.at = left.at.max(wall);
        bank(c, away, bubble);
    }
}

/// Stamps where and when `c` is leaving, for [`come_back`].
pub(crate) fn leave(c: &mut Char, wall: f64) {
    c.rest.left = Some(Left {
        at: wall,
        in_inn: in_inn(c.map, c.position),
    });
}

/// A kill's `xp` takes its bonus out of the pool: as much again, or what is left. The bonus.
pub(crate) fn spend(c: &mut Char, xp: u32) -> u32 {
    let bonus = (c.rest.pool as u32).min(xp);
    c.rest.pool = (c.rest.pool - bonus as f32).max(0.0);
    bonus
}

/// The pool as the client shows it: its marker on the XP bar, and the rest state.
pub(crate) fn pairs(c: &Char) -> [(u16, u32); 2] {
    let pool = c.rest.pool as u32;
    let state = if pool > 0 {
        REST_STATE_RESTED
    } else {
        REST_STATE_NORMAL
    };
    [
        (PLAYER_REST_STATE_EXPERIENCE, pool),
        (PLAYER_BYTES_2, u32::from(c.look[4]) | state << 24),
    ]
}

/// `PLAYER_FLAGS` for a character resting or not.
pub(crate) fn flags(resting: bool) -> (u16, u32) {
    (PLAYER_FLAGS, if resting { PLAYER_FLAGS_RESTING } else { 0 })
}
