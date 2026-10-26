//! The Forager's Grid's rules (#439, ADR 0024) as pure functions: rolling a patch's tiles, pulling
//! one and counting a Stake. Nothing here knows a world type; the realm's `world/herbalism.rs`
//! drives it.

use crate::herbs::{Grid as Tuning, Plant, Rarity};
use crate::rng::Rng;

/// What a tile holds.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Cell {
    /// A plant, with the count it yields. `money` is the Lost Coin Purse's `[min, max]` copper.
    Plant {
        item: u32,
        count: u32,
        bloom: bool,
        money: Option<[u32; 2]>,
    },
    Weed,
    Thorn,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Tile {
    pub(crate) cell: Cell,
    pub(crate) pulled: bool,
}

/// One rolled patch, `size` x `size`, row-major.
#[derive(Clone, Debug)]
pub(crate) struct Grid {
    pub(crate) size: usize,
    pub(crate) tiles: Vec<Tile>,
    pub(crate) pulls_left: u32,
    pub(crate) thorns: u32,
    /// Thorns that cost one pull instead of more.
    pub(crate) free_thorns: u32,
}

/// What one pull revealed, and the pulls it cost.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Pulled {
    pub(crate) cell: Cell,
    pub(crate) cost: u32,
}

impl Grid {
    /// A grid of `cells`, row-major, none pulled.
    pub(crate) fn of(size: usize, cells: Vec<Cell>, pulls: u32) -> Self {
        let thorns = cells.iter().filter(|c| **c == Cell::Thorn).count() as u32;
        Grid {
            size,
            tiles: cells
                .into_iter()
                .map(|cell| Tile {
                    cell,
                    pulled: false,
                })
                .collect(),
            pulls_left: pulls,
            thorns,
            free_thorns: 0,
        }
    }

    /// Whether any tile is still hidden.
    pub(crate) fn any_hidden(&self) -> bool {
        self.tiles.iter().any(|t| !t.pulled)
    }

    /// The indices of the (up to eight) tiles around `tile`.
    fn neighbours(&self, tile: usize) -> impl Iterator<Item = usize> + '_ {
        let size = self.size as i32;
        let (x, y) = (tile as i32 % size, tile as i32 / size);
        (-1..=1).flat_map(move |dy| {
            (-1..=1).filter_map(move |dx| {
                let (nx, ny) = (x + dx, y + dy);
                let inside =
                    (dx, dy) != (0, 0) && (0..size).contains(&nx) && (0..size).contains(&ny);
                inside.then_some((ny * size + nx) as usize)
            })
        })
    }
}

/// A plant of `pool` the Herbalist may find at `level`: not above it, and not a timed plant
/// outside its Bloom.
fn eligible(p: &Plant, level: u32, minute: u32) -> bool {
    p.level <= level && (!p.timed || p.bloom.is_some_and(|b| b.holds(minute)))
}

fn in_bloom(p: &Plant, minute: u32) -> bool {
    p.bloom.is_some_and(|b| b.holds(minute))
}

/// Rolls a patch of `pool` for a Herbalist of `level` at game minute `minute`, with `thorns`
/// thorns.
pub(crate) fn roll(
    rng: &mut Rng,
    pool: &[&Plant],
    level: u32,
    minute: u32,
    tuning: &Tuning,
    thorns: u32,
) -> Grid {
    let total = tuning.size as usize * tuning.size as usize;
    let thorns = (thorns as usize).min(total);
    let mut open: Vec<(&Plant, f32)> = pool
        .iter()
        .copied()
        .filter(|p| p.rarity != Rarity::Legendary && eligible(p, level, minute))
        .map(|p| {
            let mut weight = p.weight as f32;
            if in_bloom(p, minute) {
                weight *= tuning.bloom_factor;
            }
            let boost = match p.rarity {
                Rarity::Uncommon => tuning.uncommon_boost,
                Rarity::Rare => tuning.rare_boost,
                _ => 0.0,
            };
            (p, weight * (1.0 + boost * level.saturating_sub(1) as f32))
        })
        .filter(|(_, w)| *w > 0.0)
        .collect();
    let want = rng.urand(tuning.plants[0], tuning.plants[1]) as usize;
    let want = want.min(total - thorns);
    let mut picked: Vec<&Plant> = Vec::new();
    while picked.len() < want && !open.is_empty() {
        let sum: f32 = open.iter().map(|(_, w)| w).sum();
        let mut at = rng.frand(0.0, sum);
        let mut i = 0;
        while i + 1 < open.len() && at >= open[i].1 {
            at -= open[i].1;
            i += 1;
        }
        picked.push(open.remove(i).0);
    }
    let legendary = pool
        .iter()
        .copied()
        .find(|p| p.rarity == Rarity::Legendary && eligible(p, level, minute));
    if let Some(l) = legendary {
        if rng.frand(0.0, 1.0) < tuning.legendary_chance {
            if picked.is_empty() {
                picked.push(l);
            } else {
                let i = rng.below(picked.len() as u32) as usize;
                picked[i] = l;
            }
        }
    }
    let mut cells: Vec<Cell> = picked
        .iter()
        .map(|p| {
            let bloom = in_bloom(p, minute);
            Cell::Plant {
                item: p.item,
                count: if bloom {
                    tuning.bloom_factor.round() as u32
                } else {
                    1
                },
                bloom,
                money: p.money,
            }
        })
        .collect();
    cells.extend(std::iter::repeat(Cell::Thorn).take(thorns));
    cells.resize(total, Cell::Weed);
    for i in (1..cells.len()).rev() {
        cells.swap(i, rng.below(i as u32 + 1) as usize);
    }
    let mut grid = Grid::of(tuning.size as usize, cells, tuning.pulls);
    grid.thorns = thorns as u32;
    grid
}

/// Pulls `tile`: it is revealed and the pulls it costs come off. `None` for a tile off the grid
/// or already pulled.
pub(crate) fn pull(grid: &mut Grid, tile: usize) -> Option<Pulled> {
    let cell = match grid.tiles.get(tile) {
        Some(t) if !t.pulled => t.cell,
        _ => return None,
    };
    let cost = match cell {
        Cell::Thorn if grid.free_thorns > 0 => {
            grid.free_thorns -= 1;
            1
        }
        Cell::Thorn => 1 + crate::herbs::grid().thorn_cost,
        _ => 1,
    };
    grid.tiles[tile].pulled = true;
    grid.pulls_left = grid.pulls_left.saturating_sub(cost);
    Some(Pulled { cell, cost })
}

/// The Stake on `tile`: for a pulled plant or weed, the hidden plants among its eight neighbours.
pub(crate) fn stake(grid: &Grid, tile: usize) -> Option<u8> {
    let t = grid.tiles.get(tile)?;
    if !t.pulled || t.cell == Cell::Thorn {
        return None;
    }
    let n = grid
        .neighbours(tile)
        .filter(|&n| !grid.tiles[n].pulled && matches!(grid.tiles[n].cell, Cell::Plant { .. }))
        .count();
    Some(n as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::herbs;

    const NOON: u32 = 12 * 60;

    fn plant(item: u32) -> Cell {
        Cell::Plant {
            item,
            count: 1,
            bloom: false,
            money: None,
        }
    }

    fn counts(g: &Grid) -> (usize, usize, usize) {
        let n = |f: fn(&Cell) -> bool| g.tiles.iter().filter(|t| f(&t.cell)).count();
        (
            n(|c| matches!(c, Cell::Plant { .. })),
            n(|c| *c == Cell::Thorn),
            n(|c| *c == Cell::Weed),
        )
    }

    fn items(g: &Grid) -> Vec<u32> {
        g.tiles
            .iter()
            .filter_map(|t| match t.cell {
                Cell::Plant { item, .. } => Some(item),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_roll_holds_the_tuned_plants_thorns_and_weeds() {
        let t = herbs::grid();
        let pool = herbs::plants_in("elwynn");
        for seed in 0..40 {
            let g = roll(&mut Rng::new(seed), &pool, 30, NOON, t, t.thorns);
            let (plants, thorns, weeds) = counts(&g);
            assert!((t.plants[0] as usize..=t.plants[1] as usize).contains(&plants));
            assert_eq!(thorns, t.thorns as usize);
            assert_eq!(plants + thorns + weeds, (t.size * t.size) as usize);
            assert_eq!(g.pulls_left, t.pulls);
        }
    }

    #[test]
    fn a_plant_above_the_level_never_appears() {
        let t = herbs::grid();
        let pool = herbs::plants_in("elwynn");
        let level = 1;
        for seed in 0..100 {
            let g = roll(&mut Rng::new(seed), &pool, level, NOON, t, t.thorns);
            for item in items(&g) {
                let p = pool.iter().find(|p| p.item == item).unwrap();
                assert!(p.level <= level, "plant {item} needs level {}", p.level);
            }
        }
    }

    #[test]
    fn a_timed_plant_appears_only_in_its_bloom() {
        let t = herbs::grid();
        let pool = herbs::plants_in("elwynn");
        let timed: Vec<_> = pool.iter().filter(|p| p.timed).collect();
        assert!(!timed.is_empty());
        for p in timed {
            let block = p.bloom.unwrap();
            let inside = block.hours()[0] * 60;
            let outside = block.hours()[1] % 24 * 60;
            assert!(block.holds(inside) && !block.holds(outside));
            let seen = |minute| {
                (0..300).any(|seed| {
                    let g = roll(&mut Rng::new(seed), &pool, 30, minute, t, t.thorns);
                    items(&g).contains(&p.item)
                })
            };
            assert!(!seen(outside), "{} grew outside its Bloom", p.item);
            if p.rarity != Rarity::Legendary {
                assert!(seen(inside), "{} never grew in its Bloom", p.item);
            }
        }
    }

    fn two_by_two() -> Grid {
        // P W
        // P T
        Grid::of(2, vec![plant(1), Cell::Weed, plant(2), Cell::Thorn], 6)
    }

    #[test]
    fn a_stake_counts_only_hidden_plants_and_drops_as_they_are_pulled() {
        let mut g = two_by_two();
        assert_eq!(stake(&g, 1), None, "a hidden tile has no stake");
        pull(&mut g, 1);
        assert_eq!(stake(&g, 1), Some(2));
        pull(&mut g, 0);
        assert_eq!(stake(&g, 1), Some(1));
        pull(&mut g, 2);
        assert_eq!(stake(&g, 1), Some(0));
        assert_eq!(stake(&g, 0), Some(0));
    }

    #[test]
    fn a_thorn_has_no_stake() {
        let mut g = two_by_two();
        pull(&mut g, 3);
        assert_eq!(stake(&g, 3), None);
    }

    #[test]
    fn a_thorn_costs_two_pulls_and_a_plant_one() {
        let mut g = two_by_two();
        let thorn = pull(&mut g, 3).unwrap();
        assert_eq!(
            (thorn.cell, thorn.cost),
            (Cell::Thorn, 1 + herbs::grid().thorn_cost)
        );
        assert_eq!(g.pulls_left, 6 - thorn.cost);
        assert_eq!(pull(&mut g, 0).unwrap().cost, 1);
        assert!(pull(&mut g, 0).is_none(), "a tile pulls once");
        assert!(pull(&mut g, 9).is_none());
    }

    #[test]
    fn a_free_thorn_costs_one() {
        let mut g = two_by_two();
        g.free_thorns = 1;
        assert_eq!(pull(&mut g, 3).unwrap().cost, 1);
    }
}
