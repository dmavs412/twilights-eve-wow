//! Which of The Barrens' thousands of spawns are in the world: only those near the player, so a
//! tick costs the same anywhere in the zone. Spawns stream in as the player comes within view
//! and out as they leave it, and the dead come back once the player has been away a while.

use std::collections::HashMap;
use std::sync::OnceLock;

use benilla_protocol::SessionEvent;

use crate::mobs::{self, Mob, State};
use crate::rules::respawn;
use crate::Host;

/// How far the player sees mobs: a spawn whose home is this close is in the world (vmangos's
/// continent visibility is about 90–100 yd).
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
    cells: HashMap<(i32, i32), Vec<u32>>,
}

fn cell(x: f32, y: f32) -> (i32, i32) {
    ((x / CELL_YD).floor() as i32, (y / CELL_YD).floor() as i32)
}

fn grid() -> &'static Grid {
    static GRID: OnceLock<Grid> = OnceLock::new();
    GRID.get_or_init(|| {
        let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        // A Dungeon's spawns are its instance's, not the open world's.
        let world = mobs::content().spawn.iter().enumerate();
        for (i, s) in world.filter(|(_, s)| s.map == crate::content::KALIMDOR) {
            cells
                .entry(cell(s.position[0], s.position[1]))
                .or_default()
                .push(i as u32);
        }
        Grid { cells }
    })
}

/// A dead spawn waiting to come back.
struct Corpse {
    spawn: u32,
    /// Seconds of the player's absence still needed.
    remaining: f32,
}

/// The world's share of the spawns.
#[derive(Default)]
pub(crate) struct Population {
    /// Where the player stood at the last look over the view; `None` forces one.
    scanned_at: Option<[f32; 3]>,
    /// Seconds since that look.
    since_scan: f32,
    /// Which spawns have a mob in the world, by spawn index.
    present: Vec<bool>,
    dead: Vec<Corpse>,
    /// Counts every spawn, so each comes back at its own level from its range.
    spawns: u32,
}

impl Population {
    /// Brings spawns within view of `me` into `mobs` and lets distant ones go, and brings back
    /// the dead whose time is up.
    pub(crate) fn update(
        &mut self,
        dt: f32,
        me: [f32; 3],
        map: u32,
        host: &dyn Host,
        mobs: &mut Vec<Mob>,
        events: &mut Vec<SessionEvent>,
    ) {
        let content = mobs::content();
        if self.present.len() != content.spawn.len() {
            self.present = vec![false; content.spawn.len()];
        }

        // The dead come back once the player has been away long enough.
        let mut back = Vec::new();
        self.dead.retain_mut(|c| {
            let home = content.spawn[c.spawn as usize].position;
            c.remaining = respawn::countdown(c.remaining, dt, flat_distance(home, me));
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
            && self
                .scanned_at
                .is_some_and(|at| flat_distance(at, me) < RESCAN_YD)
        {
            return;
        }
        self.scanned_at = Some(me);
        self.since_scan = 0.0;

        // Out of view: the idle and the dead leave; a fight or a run home stays.
        let mut gone = Vec::new();
        mobs.retain(|m| {
            let leaves = matches!(m.state, State::Idle | State::Dead)
                && flat_distance(m.home, me) > FORGET_YD;
            if leaves {
                gone.push(m.guid);
                self.present[m.spawn as usize] = false;
            }
            !leaves
        });
        if !gone.is_empty() {
            events.push(SessionEvent::ObjectsRemoved(gone));
        }

        // Into view.
        let (lo, hi) = (
            cell(me[0] - VIEW_YD, me[1] - VIEW_YD),
            cell(me[0] + VIEW_YD, me[1] + VIEW_YD),
        );
        let grid = grid();
        for cx in lo.0..=hi.0 {
            for cy in lo.1..=hi.1 {
                let Some(list) = grid.cells.get(&(cx, cy)) else {
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
                    let [x, y, z] = spawn.position;
                    let z = host.ground_height(map, x, y, z + 2.0).unwrap_or(z);
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

    /// A mob of `spawn` died: it comes back once the player has been away long enough.
    pub(crate) fn died(&mut self, spawn: u32) {
        self.dead.push(Corpse {
            spawn,
            remaining: respawn::SECS,
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

/// The distance in the plane.
fn flat_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}
