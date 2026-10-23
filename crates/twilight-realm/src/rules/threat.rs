//! **Threat** (#147): each mob's table of the players it hates, and whom it fights. Damage is
//! threat 1:1, healing half its amount split across the mobs that hate the one healed, and a
//! player takes the mob from its target only past 110% of the target's threat in melee reach or
//! 130% beyond it (vanilla).

/// The share of a heal's amount that becomes the healer's threat.
pub const HEAL_FACTOR: f32 = 0.5;

/// How far past the target's threat a player in melee reach of the mob must go to take it.
pub const MELEE_PULL_OVER: f32 = 1.1;

/// How far past the target's threat a player out of melee reach must go to take it.
pub const RANGED_PULL_OVER: f32 = 1.3;

/// One mob's threat per player, in the order they joined it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Table(Vec<(u64, f32)>);

impl Table {
    /// Adds `amount` to `player`'s threat, putting them on the table if they are not.
    pub fn add(&mut self, player: u64, amount: f32) {
        match self.0.iter_mut().find(|(g, _)| *g == player) {
            Some((_, t)) => *t += amount,
            None => self.0.push((player, amount)),
        }
    }

    /// Puts `player` on the table with `amount` if they are not on it yet: the puller's seed.
    pub fn seed(&mut self, player: u64, amount: f32) {
        if !self.contains(player) {
            self.0.push((player, amount));
        }
    }

    /// Raises `player`'s threat to the table's highest, leaving it if already higher, and puts
    /// them on the table if they are not (a Taunt).
    pub fn raise_to_top(&mut self, player: u64) {
        let top = self.0.iter().map(|&(_, t)| t).fold(0.0, f32::max);
        match self.0.iter_mut().find(|(g, _)| *g == player) {
            Some((_, t)) => *t = t.max(top),
            None => self.0.push((player, top)),
        }
    }

    pub fn contains(&self, player: u64) -> bool {
        self.0.iter().any(|(g, _)| *g == player)
    }

    /// `player`'s threat; 0 when they are not on the table.
    pub fn of(&self, player: u64) -> f32 {
        self.0
            .iter()
            .find(|(g, _)| *g == player)
            .map_or(0.0, |(_, t)| *t)
    }

    /// Takes `player` off the table: they died, left or went out of reach.
    pub fn remove(&mut self, player: u64) {
        self.0.retain(|(g, _)| *g != player);
    }

    /// Keeps on the table only the players `keep` says may stay.
    pub fn retain(&mut self, keep: impl Fn(u64) -> bool) {
        self.0.retain(|(g, _)| keep(*g));
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whom the mob fights, now fighting `current`: `reach` says of each player whether they can
    /// be fought (`None` when dead or gone) and whether they stand in melee reach. `current` keeps
    /// the mob until another passes the pull-over; when `current` cannot be fought, the highest
    /// who can (the earliest on a tie). `None` leaves it no one to fight.
    pub fn pick(&self, current: u64, reach: impl Fn(u64) -> Option<bool>) -> Option<u64> {
        let mut best: Option<(u64, f32)> = None;
        for &(g, t) in &self.0 {
            if g != current && reach(g).is_some() && best.is_none_or(|(_, b)| t > b) {
                best = Some((g, t));
            }
        }
        if reach(current).is_none() || !self.contains(current) {
            return best.map(|(g, _)| g);
        }
        let held = self.of(current);
        match best {
            Some((g, t)) => {
                let factor = if reach(g) == Some(true) {
                    MELEE_PULL_OVER
                } else {
                    RANGED_PULL_OVER
                };
                (t > held * factor).then_some(g).or(Some(current))
            }
            None => Some(current),
        }
    }
}

/// The threat a heal of `amount` gives the healer on each of `hating` mobs that hate the one
/// healed; 0 when none do.
pub fn heal_share(amount: u32, hating: usize) -> f32 {
    if hating == 0 {
        return 0.0;
    }
    amount as f32 * HEAL_FACTOR / hating as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rows: &[(u64, f32)]) -> Table {
        let mut t = Table::default();
        rows.iter().for_each(|&(g, a)| t.add(g, a));
        t
    }

    #[test]
    fn the_target_keeps_the_mob_until_the_pull_over() {
        let t = table(&[(1, 100.0), (2, 110.0), (3, 129.0)]);
        // 2 is in melee at exactly 110%: not past it. 3 is ranged under 130%.
        assert_eq!(t.pick(1, |g| Some(g == 2)), Some(1));
        let t = table(&[(1, 100.0), (2, 111.0)]);
        assert_eq!(t.pick(1, |_| Some(true)), Some(2));
        assert_eq!(t.pick(1, |_| Some(false)), Some(1));
        let t = table(&[(1, 100.0), (3, 131.0)]);
        assert_eq!(t.pick(1, |_| Some(false)), Some(3));
    }

    #[test]
    fn a_gone_target_hands_the_mob_to_the_next_highest() {
        let t = table(&[(1, 500.0), (2, 10.0), (3, 40.0)]);
        assert_eq!(t.pick(1, |g| (g != 1).then_some(true)), Some(3));
        assert_eq!(t.pick(1, |_| None), None);
    }

    #[test]
    fn a_seed_never_lowers_threat() {
        let mut t = table(&[(1, 5.0)]);
        t.seed(1, 50.0);
        t.seed(2, 50.0);
        assert_eq!((t.of(1), t.of(2)), (5.0, 50.0));
    }

    #[test]
    fn raise_to_top_lifts_a_lower_player_to_the_highest() {
        let mut t = table(&[(1, 500.0), (2, 10.0)]);
        t.raise_to_top(2);
        assert_eq!((t.of(1), t.of(2)), (500.0, 500.0));
    }

    #[test]
    fn raise_to_top_leaves_the_top_player_alone() {
        let mut t = table(&[(1, 500.0), (2, 10.0)]);
        t.raise_to_top(1);
        assert_eq!((t.of(1), t.of(2)), (500.0, 10.0));
    }

    #[test]
    fn raise_to_top_puts_a_stranger_on_the_table_at_the_top() {
        let mut t = table(&[(1, 500.0), (2, 10.0)]);
        t.raise_to_top(3);
        assert_eq!(t.of(3), 500.0);
        assert!(t.contains(3));
    }

    #[test]
    fn healing_is_half_split_across_the_mobs() {
        assert_eq!(heal_share(100, 2), 25.0);
        assert_eq!(heal_share(100, 0), 0.0);
    }
}
