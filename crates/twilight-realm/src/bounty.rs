//! **Bounty Hunters** (`GLOSSARY.md`, spec #175, `content/notoriety.toml`): the archetypes a hunter
//! is drawn from, its generated name, and the creature template each name answers the client's
//! creature query with. Nothing here names an archetype: they are rows.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::Deserialize;

use crate::mobs::Template;
use crate::rng::Rng;
use crate::rules::stats::{MobStats, UnitClass};

#[derive(Deserialize)]
pub(crate) struct Hunter {
    entry_base: u32,
    pub(crate) level_above: u8,
    health: f32,
    damage: f32,
    armor: f32,
    pub(crate) spawn_yd: (f32, f32),
    faction_template: u32,
    speed_run: f32,
    subname: String,
}

#[derive(Deserialize)]
pub(crate) struct Archetype {
    /// The row's name, for whoever reads the file.
    #[allow(dead_code)]
    pub(crate) key: String,
    unit_class: UnitClass,
    display_id: u32,
    #[serde(default)]
    equip: Vec<u32>,
    base_attack_time: u32,
    #[serde(default)]
    spells: Vec<String>,
    first: Vec<String>,
    epithet: Vec<String>,
}

#[derive(Deserialize)]
struct File {
    hunter: Hunter,
    archetype: Vec<Archetype>,
}

fn file() -> &'static File {
    static FILE: OnceLock<File> = OnceLock::new();
    FILE.get_or_init(|| {
        toml::from_str(include_str!("../content/notoriety.toml"))
            .expect("content/notoriety.toml parses")
    })
}

/// What every Bounty Hunter shares.
pub(crate) fn hunter() -> &'static Hunter {
    &file().hunter
}

/// The archetypes, in the file's order.
pub(crate) fn archetypes() -> &'static [Archetype] {
    &file().archetype
}

/// The most first names or epithets an archetype may list: the entry's room for each.
const NAME_ROOM: usize = 32;

/// One hunter's draw: an archetype and the two halves of its name, as indexes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Draw {
    pub(crate) archetype: usize,
    pub(crate) first: usize,
    pub(crate) epithet: usize,
}

impl Draw {
    /// A fresh draw from `rng`: any archetype, any name of it.
    pub(crate) fn roll(rng: &mut Rng) -> Self {
        let pick = |n: usize, rng: &mut Rng| (rng.frand(0.0, n as f32) as usize).min(n - 1);
        let archetype = pick(archetypes().len(), rng);
        let a = &archetypes()[archetype];
        Self {
            archetype,
            first: pick(a.first.len(), rng),
            epithet: pick(a.epithet.len(), rng),
        }
    }

    pub(crate) fn archetype(self) -> &'static Archetype {
        &archetypes()[self.archetype]
    }

    /// Its generated name: "Vask the Red".
    pub(crate) fn name(self) -> String {
        let a = self.archetype();
        format!("{} {}", a.first[self.first], a.epithet[self.epithet])
    }

    /// Its realm-range creature entry, one per name.
    pub(crate) fn entry(self) -> u32 {
        hunter().entry_base
            + 1000 * self.archetype as u32
            + (NAME_ROOM * self.first + self.epithet) as u32
    }

    /// The draw `entry` stands for, if it is a hunter's.
    fn of_entry(entry: u32) -> Option<Self> {
        let n = entry.checked_sub(hunter().entry_base)? as usize;
        let draw = Self {
            archetype: n / 1000,
            first: n % 1000 / NAME_ROOM,
            epithet: n % 1000 % NAME_ROOM,
        };
        let a = archetypes().get(draw.archetype)?;
        (draw.first < a.first.len() && draw.epithet < a.epithet.len()).then_some(draw)
    }

    /// Its stats at `level`: its class's, made hard.
    pub(crate) fn stats(self, level: u8) -> MobStats {
        let h = hunter();
        MobStats::new(
            self.archetype().unit_class,
            level,
            h.health,
            h.damage,
            h.armor,
        )
    }

    /// Its creature template, made once per name and kept.
    pub(crate) fn template(self) -> &'static Template {
        template(self.entry()).expect("a draw's entry is a hunter's")
    }
}

/// The creature template of hunter entry `entry`; `None` for any other entry.
pub(crate) fn template(entry: u32) -> Option<&'static Template> {
    static MADE: OnceLock<Mutex<HashMap<u32, &'static Template>>> = OnceLock::new();
    let draw = Draw::of_entry(entry)?;
    let mut made = MADE.get_or_init(Default::default).lock().ok()?;
    Some(*made.entry(entry).or_insert_with(|| {
        let (h, a) = (hunter(), draw.archetype());
        Box::leak(Box::new(Template::hunter(
            entry,
            draw.name(),
            h.subname.clone(),
            a.display_id,
            h.faction_template,
            a.unit_class,
            a.base_attack_time,
            h.speed_run,
            a.spells.clone(),
            a.equip.clone(),
        )))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_draw_round_trips_through_its_entry() {
        for (i, a) in archetypes().iter().enumerate() {
            assert!(a.first.len() <= NAME_ROOM && a.epithet.len() <= NAME_ROOM);
            for first in 0..a.first.len() {
                for epithet in 0..a.epithet.len() {
                    let d = Draw {
                        archetype: i,
                        first,
                        epithet,
                    };
                    assert_eq!(Draw::of_entry(d.entry()), Some(d));
                    assert_eq!(d.template().name, d.name());
                }
            }
        }
        assert!(Draw::of_entry(hunter().entry_base - 1).is_none());
    }

    #[test]
    fn archetypes_have_distinct_keys() {
        let mut keys: Vec<&str> = archetypes().iter().map(|a| a.key.as_str()).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), archetypes().len());
    }
}
