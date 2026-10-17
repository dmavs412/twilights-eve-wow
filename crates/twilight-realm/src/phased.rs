//! Phased Dungeons (ADR 0006): dungeons on the open world map, each a set of footprint boxes in
//! its own content file. A character standing in a box is in that dungeon's Phase and sees,
//! targets and fights only that Phase's creatures; everyone shares one Run per dungeon, which
//! starts when the first character enters and resets the dungeon when the last one leaves.

use std::sync::OnceLock;

use serde::Deserialize;

/// Which creatures a character or creature shares the world with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Phase {
    #[default]
    OpenWorld,
    /// Inside the Phased Dungeon of this id.
    Dungeon(u32),
}

/// One axis-aligned box of a footprint: x, y and z ranges in world coordinates.
#[derive(Deserialize)]
pub(crate) struct Footprint {
    x: [f32; 2],
    y: [f32; 2],
    z: [f32; 2],
}

impl Footprint {
    fn holds(&self, [x, y, z]: [f32; 3]) -> bool {
        (self.x[0]..=self.x[1]).contains(&x)
            && (self.y[0]..=self.y[1]).contains(&y)
            && (self.z[0]..=self.z[1]).contains(&z)
    }
}

/// A creature of a Phased Dungeon, there from the Run's start at full health.
#[derive(Deserialize)]
pub(crate) struct Creature {
    pub(crate) guid: u32,
    pub(crate) entry: u32,
    pub(crate) level: u8,
    pub(crate) position: [f32; 3],
    pub(crate) orientation: f32,
}

/// A Phased Dungeon's content file.
#[derive(Deserialize)]
pub(crate) struct Dungeon {
    pub(crate) id: u32,
    pub(crate) map: u32,
    footprint: Vec<Footprint>,
    pub(crate) creature: Vec<Creature>,
}

impl Dungeon {
    /// Whether `position` on `map` is inside the footprint.
    pub(crate) fn holds(&self, map: u32, position: [f32; 3]) -> bool {
        map == self.map && self.footprint.iter().any(|b| b.holds(position))
    }
}

/// Every Phased Dungeon: one content file each.
pub(crate) fn content() -> &'static [Dungeon] {
    static DUNGEONS: OnceLock<Vec<Dungeon>> = OnceLock::new();
    DUNGEONS.get_or_init(|| {
        [include_str!("../content/northshire_abbey.toml")]
            .into_iter()
            .map(|file| toml::from_str(file).expect("a Phased Dungeon's content file parses"))
            .collect()
    })
}

/// The Phase of a character standing at `position` on `map`.
pub(crate) fn at(map: u32, position: [f32; 3]) -> Phase {
    content()
        .iter()
        .find(|d| d.holds(map, position))
        .map_or(Phase::OpenWorld, |d| Phase::Dungeon(d.id))
}

/// A dungeon's Run: from the first character in to the last one out.
pub(crate) struct Run {
    /// Seconds since it started, for the timed events to come.
    pub(crate) elapsed: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marshal_mcbride_stands_outside_the_abbey() {
        let mcbride = crate::mobs::content()
            .spawn
            .iter()
            .find(|s| s.entry == 197)
            .unwrap();
        assert_eq!(at(mcbride.map, mcbride.position), Phase::OpenWorld);
    }
}
