//! Which of The Barrens' thousands of spawns are in the world: only those near a player, so a
//! tick costs the same anywhere in the zone. Spawns stream in as a player comes within view
//! and out once every player has left it, and the dead come back once the players have been away
//! a while.

use std::collections::HashMap;
use std::sync::OnceLock;

use benilla_protocol::SessionEvent;

use crate::mobs::{self, Mob, State};
use crate::rules::respawn;
use crate::space::flat_distance;
use crate::Host;

/// How far a player sees mobs: a spawn whose home is this close is in the world (vmangos's
/// continent visibility is about 90–100 yd). The one view range: a client is sent what is this
/// close to its player.
pub(crate) const VIEW_YD: f32 = 100.0;
/// How far an idle or dead mob's home may be before it leaves the world; above [`VIEW_YD`] so a
/// player on the edge does not make it flicker.
const FORGET_YD: f32 = 110.0;
/// How far the player moves before the view is looked over again.
const RESCAN_YD: f32 = 8.0;
/// The longest between two looks, for the mobs that ran home from a fight out of view.
const RESCAN_SECS: f32 = 1.0;
/// The side of a spatial-index cell.
const CELL_YD: f32 = 64.0;

/// The spawns' homes by grid cell.
struct Grid {
    /// By map, then cell.
    cells: HashMap<(u32, i32, i32), Vec<u32>>,
}

fn cell(x: f32, y: f32) -> (i32, i32) {
    ((x / CELL_YD).floor() as i32, (y / CELL_YD).floor() as i32)
}

fn grid() -> &'static Grid {
    static GRID: OnceLock<Grid> = OnceLock::new();
    GRID.get_or_init(|| {
        let mut cells: HashMap<(u32, i32, i32), Vec<u32>> = HashMap::new();
        // A Dungeon's spawns are its instance's, not the open world's.
        let world = mobs::content().spawn.iter().enumerate();
        // A Phased Dungeon's creatures come with its Run, not the view.
        let open = |s: &mobs::Spawn| {
            crate::content::open_world(s.map)
                && s.phase == crate::phased::Phase::OpenWorld
                && s.encounter.is_none()
        };
        for (i, s) in world.filter(|(_, s)| open(s)) {
            let (cx, cy) = cell(s.position[0], s.position[1]);
            cells.entry((s.map, cx, cy)).or_default().push(i as u32);
        }
        Grid { cells }
    })
}

/// A dead spawn waiting to come back.
struct Corpse {
    spawn: u32,
    /// Seconds of the player's absence it takes in all.
    wait: f32,
    /// Seconds of the player's absence still needed.
    remaining: f32,
}

/// The world's share of the spawns.
#[derive(Default)]
pub(crate) struct Population {
    /// Where the players stood at the last look over the view; `None` forces one.
    scanned_at: Option<Vec<[f32; 3]>>,
    /// Seconds since that look.
    since_scan: f32,
    /// Which spawns have a mob in the world, by spawn index.
    present: Vec<bool>,
    dead: Vec<Corpse>,
    /// Counts every spawn, so each comes back at its own level from its range.
    spawns: u32,
}

impl Population {
    /// Brings spawns within view of any of `players` into `mobs` and lets go those far from them
    /// all, and brings back the dead whose time is up.
    pub(crate) fn update(
        &mut self,
        dt: f32,
        players: &[[f32; 3]],
        map: u32,
        host: &dyn Host,
        mobs: &mut Vec<Mob>,
        events: &mut Vec<SessionEvent>,
    ) {
        let content = mobs::content();
        if self.present.len() != content.spawn.len() {
            self.present = vec![false; content.spawn.len()];
        }

        let nearest = |at: [f32; 3]| {
            players
                .iter()
                .map(|&p| flat_distance(at, p))
                .fold(f32::INFINITY, f32::min)
        };
        // The dead come back once the players have been away long enough.
        let mut back = Vec::new();
        self.dead.retain_mut(|c| {
            let home = content.spawn[c.spawn as usize].position;
            c.remaining = respawn::countdown(c.remaining, c.wait, dt, nearest(home));
            if c.remaining > 0.0 {
                return true;
            }
            back.push(c.spawn);
            false
        });
        for spawn in back {
            if let Some(i) = mobs.iter().position(|m| m.spawn == spawn) {
                events.push(SessionEvent::ObjectDestroyed(mobs.swap_remove(i).guid));
                self.present[spawn as usize] = false;
            }
            self.scanned_at = None;
        }

        self.since_scan += dt;
        if self.since_scan < RESCAN_SECS
            && self.scanned_at.as_ref().is_some_and(|at| {
                at.len() == players.len()
                    && at
                        .iter()
                        .zip(players)
                        .all(|(&a, &p)| flat_distance(a, p) < RESCAN_YD)
            })
        {
            return;
        }
        self.scanned_at = Some(players.to_vec());
        self.since_scan = 0.0;

        // Out of view: the idle and the dead leave; a fight or a run home stays.
        let mut gone = Vec::new();
        mobs.retain(|m| {
            let leaves =
                matches!(m.state, State::Idle | State::Dead) && nearest(m.home) > FORGET_YD;
            if leaves {
                gone.push(m.guid);
                if let Some(present) = self.present.get_mut(m.spawn as usize) {
                    *present = false;
                }
            }
            !leaves
        });
        if !gone.is_empty() {
            events.push(SessionEvent::ObjectsRemoved(gone));
        }

        // Into view.
        let grid = grid();
        for &me in players {
            let (lo, hi) = (
                cell(me[0] - VIEW_YD, me[1] - VIEW_YD),
                cell(me[0] + VIEW_YD, me[1] + VIEW_YD),
            );
            self.stream_in(lo, hi, me, map, host, grid, mobs, events);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn stream_in(
        &mut self,
        lo: (i32, i32),
        hi: (i32, i32),
        me: [f32; 3],
        map: u32,
        host: &dyn Host,
        grid: &Grid,
        mobs: &mut Vec<Mob>,
        events: &mut Vec<SessionEvent>,
    ) {
        let content = mobs::content();
        for cx in lo.0..=hi.0 {
            for cy in lo.1..=hi.1 {
                let Some(list) = grid.cells.get(&(map, cx, cy)) else {
                    continue;
                };
                for &i in list {
                    let spawn = &content.spawn[i as usize];
                    if self.present[i as usize]
                        || flat_distance(spawn.position, me) > VIEW_YD
                        || self.dead.iter().any(|c| c.spawn == i)
                    {
                        continue;
                    }
                    let z = crate::ground::grounded_z(host, map, spawn.position, spawn.grounded);
                    let level = self.level(spawn.guid, spawn.level_min, spawn.level_max);
                    if let Some(mob) = Mob::spawn(i, level, z) {
                        events.push(mob.create());
                        mobs.push(mob);
                        self.present[i as usize] = true;
                    }
                }
            }
        }
    }

    /// A mob of `spawn` died: it comes back once the player has been away `wait` seconds.
    pub(crate) fn died(&mut self, spawn: u32, wait: f32) {
        self.dead.push(Corpse {
            spawn,
            wait,
            remaining: wait,
        });
    }

    /// A level in `lo..=hi` for spawn `guid`, from a hash rather than the realm's dice, so
    /// streaming the world in does not change how fights roll.
    fn level(&mut self, guid: u32, lo: u8, hi: u8) -> u8 {
        self.spawns = self.spawns.wrapping_add(1);
        if hi <= lo {
            return lo;
        }
        let mut z = u64::from(guid) << 32 | u64::from(self.spawns);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        lo + (z % u64::from(hi - lo + 1)) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{EASTERN_KINGDOMS, KALIMDOR};

    struct Flat;
    impl Host for Flat {
        fn ground_height(&self, _: u32, _: f32, _: f32, z: f32) -> Option<f32> {
            Some(z - 2.0)
        }
        fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
            true
        }
        fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
            None
        }
    }

    /// The first spawn of `entry` and the mobs that stream in standing on it on `map`.
    fn stream_at(entry: u32, map: u32) -> (&'static mobs::Spawn, Vec<Mob>) {
        let spawn = mobs::content()
            .spawn
            .iter()
            .find(|s| s.entry == entry)
            .unwrap();
        let mut mobs = Vec::new();
        Population::default().update(
            0.1,
            &[spawn.position],
            map,
            &Flat,
            &mut mobs,
            &mut Vec::new(),
        );
        (spawn, mobs)
    }

    #[test]
    fn elwynn_streams_in_on_eastern_kingdoms() {
        // Hogger at Forest's Edge, Marshal McBride in Northshire Abbey.
        for (entry, at) in [(448, [-10125.0, 662.0]), (197, [-8902.0, -162.0])] {
            let (spawn, mobs) = stream_at(entry, EASTERN_KINGDOMS);
            assert_eq!(spawn.map, EASTERN_KINGDOMS);
            assert!(
                flat_distance(spawn.position, [at[0], at[1], 0.0]) < 400.0,
                "{entry} far off"
            );
            let here =
                |m: &&Mob| m.template.entry == entry && m.position[..2] == spawn.position[..2];
            assert!(
                mobs.iter().any(|m| here(&m)),
                "{entry} did not stream in at its spawn"
            );
        }
    }

    #[test]
    fn a_continent_streams_only_its_own_spawns() {
        let (_, mobs) = stream_at(197, KALIMDOR);
        assert!(
            mobs.is_empty(),
            "Kalimdor shows Northshire's {} mobs",
            mobs.len()
        );
    }
}
