//! **Playing together** (`content/group.toml`): the tunables of threat and shared credit.

use std::sync::OnceLock;

#[derive(serde::Deserialize)]
pub(crate) struct Group {
    /// The pull seed's share of the mob's max health (#147).
    pub(crate) pull_threat_health_fraction: f32,
    /// How near a dying mob a player shares its kill (#148).
    pub(crate) credit_range_yd: f32,
    /// The health a Phased Dungeon creature gains per player past the first (ADR 0029).
    head_count_health_per_player: f32,
    /// The damage a Phased Dungeon creature gains per player past the first (ADR 0029).
    head_count_damage_per_player: f32,
}

impl Group {
    /// What a creature's health is multiplied by at a Head Count of `n`; 0 reads as 1.
    pub(crate) fn health_mul(&self, n: u32) -> f32 {
        1.0 + self.head_count_health_per_player * n.saturating_sub(1) as f32
    }

    /// What a creature's swing damage is multiplied by at a Head Count of `n`; 0 reads as 1.
    pub(crate) fn damage_mul(&self, n: u32) -> f32 {
        1.0 + self.head_count_damage_per_player * n.saturating_sub(1) as f32
    }
}

pub(crate) fn get() -> &'static Group {
    static GROUP: OnceLock<Group> = OnceLock::new();
    GROUP.get_or_init(|| {
        toml::from_str(include_str!("../content/group.toml")).expect("content/group.toml parses")
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_content_parses() {
        assert!(super::get().pull_threat_health_fraction > 0.0);
    }

    #[test]
    fn the_head_count_multiplies_health_by_half_and_damage_by_a_tenth_a_player() {
        let g = super::get();
        for (n, health, damage) in [(1, 1.0, 1.0), (2, 1.5, 1.1), (5, 3.0, 1.4)] {
            assert!((g.health_mul(n) - health).abs() < 1e-6, "n = {n}");
            assert!((g.damage_mul(n) - damage).abs() < 1e-6, "n = {n}");
        }
        assert_eq!(g.health_mul(0), 1.0);
        assert_eq!(g.damage_mul(0), 1.0);
    }
}
