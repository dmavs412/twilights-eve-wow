//! The Forager's Grid's rules (#439, ADR 0024) as pure functions: rolling a patch's tiles, pulling
//! one and counting a Stake. Nothing here knows a world type; the realm's `world/herbalism.rs`
//! drives it.

use crate::fish::Block;
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
    /// What the player knows of each hidden tile: 0 nothing, 1 plant, 2 weed, 3 thorn.
    pub(crate) known: Vec<u8>,
    /// Tiles a probe counted: their hidden neighbours' plants show as a Stake.
    pub(crate) probed: Vec<bool>,
    /// Added to a pulled plant's count (Greedy Hands).
    pub(crate) yield_bonus: u32,
    /// Added to a pulled weed's cost (Greedy Hands).
    pub(crate) weed_extra: u32,
    /// Whether the chosen active consumable can still be used on this patch.
    pub(crate) ready: bool,
    /// The Forager's Charm is armed for the next pull.
    pub(crate) charm_armed: bool,
    /// Overgrowth: pulled plants show no Stake.
    pub(crate) hide_plant_stake: bool,
    /// Wild Luck: nothing shows a Stake.
    pub(crate) hide_stake: bool,
}

/// The perks a Herbalist holds that change a patch.
#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct Perks {
    pub(crate) gloves: bool,
    pub(crate) keeneye: bool,
    pub(crate) reach: bool,
    pub(crate) bramble: bool,
    pub(crate) greedyhands: bool,
    pub(crate) overgrowth: bool,
    pub(crate) wildluck: bool,
    pub(crate) nightowl: bool,
    pub(crate) thornwise: bool,
    /// A Well Fed buff's extra pulls.
    pub(crate) pulls: u32,
    /// A Well Fed buff's percent on the uncommon and rare plants' weights.
    pub(crate) rarity_pct: u32,
}

impl Perks {
    /// The perks named by `keys`; an empty slot's "" names none.
    pub(crate) fn of(keys: &[String]) -> Self {
        let has = |k: &str| keys.iter().any(|x| x == k);
        Perks {
            gloves: has("gloves"),
            keeneye: has("keeneye"),
            reach: has("reach"),
            bramble: has("bramble"),
            greedyhands: has("greedyhands"),
            overgrowth: has("overgrowth"),
            wildluck: has("wildluck"),
            nightowl: has("nightowl"),
            thornwise: has("thornwise"),
            ..Perks::default()
        }
    }
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
        let total = cells.len();
        Grid {
            size,
            known: vec![0; total],
            probed: vec![false; total],
            yield_bonus: 0,
            weed_extra: 0,
            ready: false,
            charm_armed: false,
            hide_plant_stake: false,
            hide_stake: false,
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
    pub(crate) fn neighbours(&self, tile: usize) -> impl Iterator<Item = usize> + '_ {
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
    perks: Perks,
) -> Grid {
    let total = tuning.size as usize * tuning.size as usize;
    let thorns = (thorns as usize + if perks.bramble { 2 } else { 0 }).min(total);
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
            if matches!(p.rarity, Rarity::Uncommon | Rarity::Rare) {
                weight *= 1.0 + perks.rarity_pct as f32 / 100.0;
            }
            (p, weight * (1.0 + boost * level.saturating_sub(1) as f32))
        })
        .filter(|(_, w)| *w > 0.0)
        .collect();
    let want = rng.urand(tuning.plants[0], tuning.plants[1]) as usize
        + if perks.overgrowth { 3 } else { 0 };
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
    if perks.bramble {
        // Two random plants rise one rarity, to a plant of that rarity the Herbalist can grow.
        for _ in 0..2.min(picked.len()) {
            let i = rng.below(picked.len() as u32) as usize;
            let up = match picked[i].rarity {
                Rarity::Common => Rarity::Uncommon,
                Rarity::Uncommon => Rarity::Rare,
                _ => continue,
            };
            let better: Vec<&Plant> = pool
                .iter()
                .copied()
                .filter(|p| {
                    p.rarity == up
                        && eligible(p, level, minute)
                        && !picked.iter().any(|q| q.item == p.item)
                })
                .collect();
            if !better.is_empty() {
                picked[i] = better[rng.below(better.len() as u32) as usize];
            }
        }
    }
    let legendary = pool
        .iter()
        .copied()
        .find(|p| p.rarity == Rarity::Legendary && eligible(p, level, minute));
    if let Some(l) = legendary {
        let chance = tuning.legendary_chance * if perks.wildluck { 5.0 } else { 1.0 };
        if rng.frand(0.0, 1.0) < chance {
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
    let mut pulls = tuning.pulls;
    if perks.nightowl {
        pulls = if Block::Night.holds(minute) || Block::Dusk.holds(minute) {
            pulls + 1
        } else {
            pulls.saturating_sub(1)
        };
    }
    if perks.thornwise {
        pulls = pulls.saturating_sub(1);
    }
    pulls += perks.pulls;
    let mut grid = Grid::of(tuning.size as usize, cells, pulls);
    grid.thorns = thorns as u32;
    if perks.gloves {
        grid.free_thorns = 1;
    }
    if perks.greedyhands {
        grid.yield_bonus = 1;
        grid.weed_extra = 1;
    }
    grid.hide_plant_stake = perks.overgrowth;
    grid.hide_stake = perks.wildluck;
    if perks.thornwise {
        for (i, t) in grid.tiles.iter().enumerate() {
            if t.cell == Cell::Thorn {
                grid.known[i] = 3;
            }
        }
    }
    if perks.keeneye {
        let plants: Vec<usize> = grid
            .tiles
            .iter()
            .enumerate()
            .filter(|(_, t)| matches!(t.cell, Cell::Plant { .. }))
            .map(|(i, _)| i)
            .collect();
        if !plants.is_empty() {
            grid.known[plants[rng.below(plants.len() as u32) as usize]] = 1;
        }
    }
    grid
}

/// Reveals `tile` without charging a pull; the cell and the extra pulls a thorn costs (none for a
/// free thorn). `None` for a tile off the grid or already pulled.
pub(crate) fn reveal(grid: &mut Grid, tile: usize) -> Option<(Cell, u32)> {
    let cell = match grid.tiles.get(tile) {
        Some(t) if !t.pulled => t.cell,
        _ => return None,
    };
    let extra = match cell {
        Cell::Thorn if grid.free_thorns > 0 => {
            grid.free_thorns -= 1;
            0
        }
        Cell::Thorn => crate::herbs::grid().thorn_cost,
        _ => 0,
    };
    grid.tiles[tile].pulled = true;
    Some((cell, extra))
}

/// Pulls `tile`: it is revealed and the pulls it costs come off. An armed Forager's Charm doubles
/// a plant's yield or costs a weed or thorn one more pull, and is spent. `None` for a tile off the
/// grid or already pulled.
pub(crate) fn pull(grid: &mut Grid, tile: usize) -> Option<Pulled> {
    let (mut cell, extra) = reveal(grid, tile)?;
    let charm = std::mem::take(&mut grid.charm_armed);
    if charm {
        grid.ready = false;
    }
    let mut cost = 1 + extra;
    match &mut cell {
        Cell::Plant { count, .. } => {
            *count += grid.yield_bonus;
            if charm {
                *count *= 2;
            }
        }
        Cell::Weed => cost += grid.weed_extra + u32::from(charm),
        Cell::Thorn => cost += u32::from(charm),
    }
    grid.tiles[tile].cell = cell;
    grid.pulls_left = grid.pulls_left.saturating_sub(cost);
    Some(Pulled { cell, cost })
}

/// The tiles a Sweep from `tile` reveals: a line of `length` tiles, across or down, shifted to
/// fit the grid.
pub(crate) fn sweep_line(size: usize, tile: usize, across: bool, length: usize) -> Vec<usize> {
    let length = length.min(size);
    let (x, y) = (tile % size, tile / size);
    if across {
        let start = x.min(size - length);
        (0..length).map(|k| y * size + start + k).collect()
    } else {
        let start = y.min(size - length);
        (0..length).map(|k| (start + k) * size + x).collect()
    }
}

/// The tiles of the 3 x 3 around `tile`, itself included, that are on the grid.
pub(crate) fn square(grid: &Grid, tile: usize) -> Vec<usize> {
    let mut all: Vec<usize> = grid.neighbours(tile).collect();
    all.push(tile);
    all
}

/// What a hidden tile shows when marked: 1 plant, 2 weed, 3 thorn.
pub(crate) fn kind_of(cell: Cell) -> u8 {
    match cell {
        Cell::Plant { .. } => 1,
        Cell::Weed => 2,
        Cell::Thorn => 3,
    }
}

/// The Stake on `tile`: for a pulled plant or weed, or a probed hidden one, the hidden plants
/// among its eight neighbours. Overgrowth hides it on pulled plants and Wild Luck everywhere.
pub(crate) fn stake(grid: &Grid, tile: usize) -> Option<u8> {
    let t = grid.tiles.get(tile)?;
    if !(t.pulled || grid.probed[tile]) || t.cell == Cell::Thorn || grid.hide_stake {
        return None;
    }
    if t.pulled && grid.hide_plant_stake && matches!(t.cell, Cell::Plant { .. }) {
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
            let g = roll(
                &mut Rng::new(seed),
                &pool,
                30,
                NOON,
                t,
                t.thorns,
                Perks::default(),
            );
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
            let g = roll(
                &mut Rng::new(seed),
                &pool,
                level,
                NOON,
                t,
                t.thorns,
                Perks::default(),
            );
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
                    let g = roll(
                        &mut Rng::new(seed),
                        &pool,
                        30,
                        minute,
                        t,
                        t.thorns,
                        Perks::default(),
                    );
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

    #[test]
    fn a_sweep_line_shifts_to_fit_the_grid() {
        assert_eq!(sweep_line(5, 0, true, 3), [0, 1, 2]);
        assert_eq!(sweep_line(5, 4, true, 3), [2, 3, 4]);
        assert_eq!(sweep_line(5, 7, false, 3), [7, 12, 17]);
        assert_eq!(sweep_line(5, 2, false, 3), [2, 7, 12]);
        assert_eq!(sweep_line(5, 22, false, 3), [12, 17, 22]);
    }

    #[test]
    fn an_armed_charm_doubles_a_plant_and_is_spent() {
        let mut g = two_by_two();
        g.ready = true;
        g.charm_armed = true;
        let got = pull(&mut g, 0).unwrap();
        assert!(matches!(got.cell, Cell::Plant { count: 2, .. }));
        assert_eq!(got.cost, 1);
        assert!(!g.charm_armed && !g.ready);
        assert!(matches!(
            pull(&mut g, 2).unwrap().cell,
            Cell::Plant { count: 1, .. }
        ));
    }

    #[test]
    fn an_armed_charm_costs_a_weed_one_more_pull() {
        let mut g = two_by_two();
        g.charm_armed = true;
        assert_eq!(pull(&mut g, 1).unwrap().cost, 2);
    }

    #[test]
    fn greedy_hands_adds_a_yield_and_a_weed_pull() {
        let mut g = two_by_two();
        g.yield_bonus = 1;
        g.weed_extra = 1;
        assert!(matches!(
            pull(&mut g, 0).unwrap().cell,
            Cell::Plant { count: 2, .. }
        ));
        assert_eq!(pull(&mut g, 1).unwrap().cost, 2);
    }

    #[test]
    fn night_owl_and_thornwise_move_the_pulls() {
        let t = herbs::grid();
        let pool = herbs::plants_in("elwynn");
        let perks = |night, thornwise| Perks {
            nightowl: night,
            thornwise,
            ..Perks::default()
        };
        let pulls =
            |minute, p| roll(&mut Rng::new(1), &pool, 30, minute, t, t.thorns, p).pulls_left;
        assert_eq!(pulls(23 * 60, perks(true, false)), t.pulls + 1);
        assert_eq!(pulls(17 * 60, perks(true, false)), t.pulls + 1);
        assert_eq!(pulls(NOON, perks(true, false)), t.pulls - 1);
        assert_eq!(pulls(5 * 60, perks(true, false)), t.pulls - 1);
        assert_eq!(pulls(NOON, perks(false, true)), t.pulls - 1);
    }

    #[test]
    fn a_well_fed_pull_bonus_adds_its_pulls() {
        let t = herbs::grid();
        let pool = herbs::plants_in("elwynn");
        let fed = Perks {
            pulls: 1,
            ..Perks::default()
        };
        let g = roll(&mut Rng::new(1), &pool, 30, NOON, t, t.thorns, fed);
        assert_eq!(g.pulls_left, t.pulls + 1);
    }

    #[test]
    fn thornwise_shows_every_thorn() {
        let t = herbs::grid();
        let pool = herbs::plants_in("elwynn");
        let perks = Perks {
            thornwise: true,
            ..Perks::default()
        };
        let g = roll(&mut Rng::new(3), &pool, 30, NOON, t, t.thorns, perks);
        for (i, tile) in g.tiles.iter().enumerate() {
            assert_eq!(g.known[i] == 3, tile.cell == Cell::Thorn);
        }
    }

    #[test]
    fn bramble_adds_two_thorns_and_overgrowth_three_plants() {
        let t = herbs::grid();
        let pool = herbs::plants_in("elwynn");
        let base = roll(
            &mut Rng::new(5),
            &pool,
            30,
            NOON,
            t,
            t.thorns,
            Perks::default(),
        );
        let bramble = Perks {
            bramble: true,
            ..Perks::default()
        };
        let g = roll(&mut Rng::new(5), &pool, 30, NOON, t, t.thorns, bramble);
        assert_eq!(counts(&g).1, counts(&base).1 + 2);
        let grown = Perks {
            overgrowth: true,
            ..Perks::default()
        };
        let g = roll(&mut Rng::new(5), &pool, 30, NOON, t, t.thorns, grown);
        assert!(counts(&g).0 >= t.plants[0] as usize + 3);
    }

    #[test]
    fn overgrowth_and_wild_luck_hide_the_stake() {
        let mut g = two_by_two();
        pull(&mut g, 1);
        assert_eq!(stake(&g, 1), Some(2));
        g.hide_stake = true;
        assert_eq!(stake(&g, 1), None);
        g.hide_stake = false;
        g.hide_plant_stake = true;
        assert_eq!(stake(&g, 1), Some(2), "a weed still shows it");
        pull(&mut g, 0);
        assert_eq!(stake(&g, 0), None, "a plant does not");
    }

    #[test]
    fn a_probed_tile_shows_its_stake_while_hidden() {
        let mut g = two_by_two();
        g.probed[1] = true;
        assert_eq!(stake(&g, 1), Some(2));
        assert_eq!(stake(&g, 2), None);
    }
}
