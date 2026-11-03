//! The Writ Board (spec #557): which Writ each pool shows in each window. A pool is the Writs one
//! board GameObject offers for one zone (their `zone_or_sort`), and it shows one of them per
//! window. The board changes every `window_days` at midnight UTC, all pools at once, by the
//! realm's wall clock. The draw is a pure function of the window, so every host agrees and nothing
//! is saved; a character's save keeps only the window of each Writ's last hand-in.

use std::sync::{Mutex, OnceLock};

use serde::Deserialize;
use toml::value::Datetime;

use crate::characters::Char;
use crate::quests::content::{self, Kind, Quest};
use crate::rng::Rng;

const DAY_SECS: u64 = 86_400;

/// `content/writs.toml`'s `[board]` table, as written.
#[derive(Deserialize)]
struct Written {
    /// Window 0 starts at this date's midnight UTC, `yyyy-mm-dd`.
    anchor: String,
    window_days: u32,
    /// `[first, second]` zones: the second pool's draw skips the band the first drew.
    #[serde(default)]
    apart: Vec<(i32, i32)>,
}

#[derive(Deserialize)]
struct File {
    board: Written,
}

/// The board's clock and draw rules.
struct Board {
    /// Window 0's start, in seconds since the Unix epoch.
    anchor_secs: u64,
    window_secs: u64,
    apart: Vec<(i32, i32)>,
}

fn board() -> &'static Board {
    static BOARD: OnceLock<Board> = OnceLock::new();
    BOARD.get_or_init(|| {
        let file: File = toml::from_str(include_str!("../content/writs.toml"))
            .unwrap_or_else(|e| panic!("content/writs.toml parses: {e}"));
        let b = file.board;
        let date = b
            .anchor
            .parse::<Datetime>()
            .ok()
            .filter(|d| d.time.is_none())
            .and_then(|d| d.date)
            .unwrap_or_else(|| {
                panic!(
                    "content/writs.toml's anchor {:?} is no yyyy-mm-dd date",
                    b.anchor
                )
            });
        let days = crate::unlocks::days_from_civil(
            i64::from(date.year),
            i64::from(date.month),
            i64::from(date.day),
        );
        assert!(b.window_days > 0, "content/writs.toml's window_days is 0");
        Board {
            anchor_secs: u64::try_from(days).expect("the Writ Board's anchor is after 1970")
                * DAY_SECS,
            window_secs: u64::from(b.window_days) * DAY_SECS,
            apart: b.apart,
        }
    })
}

/// The window `wall_secs` (seconds since the Unix epoch) falls in: the whole `window_days` periods
/// since the anchor's midnight UTC; 0 before it.
pub(crate) fn window(wall_secs: u64) -> u32 {
    let b = board();
    u32::try_from(wall_secs.saturating_sub(b.anchor_secs) / b.window_secs).unwrap_or(u32::MAX)
}

/// The Writs one board offers for one zone, by level: their index is their band.
struct Pool {
    object: u32,
    zone: i32,
    writs: Vec<u32>,
}

/// Every pool, the second of each `apart` pair after the first.
fn pools() -> Vec<Pool> {
    let mut by_level: Vec<(u32, i32, Vec<(u8, u32)>)> = Vec::new();
    for q in content::quests()
        .into_iter()
        .filter(|q| q.kind == Kind::Writ)
    {
        let object = q
            .giver_object
            .expect("the content load checks a Writ names its board");
        let zone = q.zone_or_sort();
        match by_level
            .iter_mut()
            .find(|(o, z, _)| *o == object && *z == zone)
        {
            Some((_, _, writs)) => writs.push((q.level, q.id)),
            None => by_level.push((object, zone, vec![(q.level, q.id)])),
        }
    }
    let mut pools: Vec<Pool> = by_level
        .into_iter()
        .map(|(object, zone, mut writs)| {
            writs.sort();
            Pool {
                object,
                zone,
                writs: writs.into_iter().map(|(_, id)| id).collect(),
            }
        })
        .collect();
    pools.sort_by_key(|p| board().apart.iter().any(|&(_, second)| second == p.zone));
    pools
}

/// What each pool shows in `window`, by index in `pools`, given what each showed in the window
/// before (none before window 0). A pool never shows its last Writ again, and the second pool of an
/// `apart` pair skips the band the first drew; when that leaves nothing (a pool of one), the rules
/// give way.
fn draw(pools: &[Pool], window: u32, before: Option<&[u32]>) -> Vec<u32> {
    let mut shown: Vec<u32> = Vec::with_capacity(pools.len());
    for (i, p) in pools.iter().enumerate() {
        let skipped_band = board()
            .apart
            .iter()
            .filter(|&&(_, second)| second == p.zone)
            .find_map(|&(first, _)| {
                let j = pools[..i]
                    .iter()
                    .position(|o| o.object == p.object && o.zone == first)?;
                pools[j].writs.iter().position(|&w| w == shown[j])
            });
        let last = before.map(|b| b[i]);
        let open = |skip_band: bool| -> Vec<u32> {
            p.writs
                .iter()
                .enumerate()
                .filter(|&(band, &w)| Some(w) != last && !(skip_band && Some(band) == skipped_band))
                .map(|(_, &w)| w)
                .collect()
        };
        let mut from = open(true);
        if from.is_empty() {
            from = open(false);
        }
        if from.is_empty() {
            from = p.writs.clone();
        }
        let mut rng = Rng::new(u64::from(window) << 32 | u64::from(p.zone as u32));
        shown.push(from[rng.below(from.len() as u32) as usize]);
    }
    shown
}

/// The Writs showing in `window`, one per pool. It steps forward from window 0, each window's
/// draw knowing the one before, and keeps the last window it reached to step on from.
pub(crate) fn showing(window: u32) -> Vec<u32> {
    struct Reached {
        window: u32,
        /// How many Writs the pools held: a test may add some.
        writs: usize,
        shown: Vec<u32>,
    }
    static LAST: Mutex<Option<Reached>> = Mutex::new(None);
    let pools = pools();
    let writs = pools.iter().map(|p| p.writs.len()).sum();
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    let (mut at, mut shown) = match last.take() {
        Some(r) if r.writs == writs && r.window <= window => (r.window, r.shown),
        _ => (0, draw(&pools, 0, None)),
    };
    while at < window {
        at += 1;
        shown = draw(&pools, at, Some(&shown));
    }
    *last = Some(Reached {
        window,
        writs,
        shown: shown.clone(),
    });
    shown
}

/// Whether the board offers Writ `q` to `c` in `window`: its pool shows it, and `c` has not handed
/// it in during this window.
pub(crate) fn open_to(c: &Char, q: &Quest, window: u32) -> bool {
    showing(window).contains(&q.id) && c.quests.writ_done(q.id) != Some(window)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-11-02T00:00Z.
    const ANCHOR: u64 = 1_793_577_600;
    const HOUR: u64 = 3600;

    #[test]
    fn a_window_is_two_days_from_the_anchors_midnight_and_0_before_it() {
        assert_eq!(window(ANCHOR), 0);
        assert_eq!(window(ANCHOR + 47 * HOUR + 59 * 60), 0);
        assert_eq!(window(ANCHOR + 48 * HOUR), 1);
        assert_eq!(window(ANCHOR - 24 * HOUR), 0);
        assert_eq!(window(0), 0);
    }
}
