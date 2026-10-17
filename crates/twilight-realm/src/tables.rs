//! The per-level tables the rules read, parsed once from the TOML under `content/`, which is
//! generated from the vmangos world DB dumps in `data/twilights-eve/`.

use std::sync::OnceLock;

use serde::Deserialize;

/// The highest level any table covers: the end of the game.
pub(crate) const MAX_LEVEL: u8 = 60;

#[derive(Deserialize)]
pub(crate) struct Levels {
    /// Indexed by level - 1.
    pub(crate) xp_to_next: Vec<u32>,
    pub(crate) warrior: ClassLevels,
    pub(crate) paladin: ClassLevels,
    pub(crate) hunter: ClassLevels,
    pub(crate) rogue: ClassLevels,
    pub(crate) priest: ClassLevels,
    pub(crate) shaman: ClassLevels,
    pub(crate) mage: ClassLevels,
    pub(crate) warlock: ClassLevels,
    pub(crate) druid: ClassLevels,
}

impl Levels {
    /// The tables of a Class by its `ChrClasses.dbc` id (1 warrior, 2 paladin, 3 hunter,
    /// 4 rogue, 5 priest, 7 shaman, 8 mage, 9 warlock, 11 druid); an unknown id reads as a
    /// warrior.
    pub(crate) fn class(&self, class: u8) -> &ClassLevels {
        match class {
            2 => &self.paladin,
            3 => &self.hunter,
            4 => &self.rogue,
            5 => &self.priest,
            7 => &self.shaman,
            8 => &self.mage,
            9 => &self.warlock,
            11 => &self.druid,
            _ => &self.warrior,
        }
    }
}

/// One Class's per-level tables.
#[derive(Deserialize)]
pub(crate) struct ClassLevels {
    pub(crate) base_health: Vec<u32>,
    pub(crate) base_mana: Vec<u32>,
    pub(crate) agi_per_crit: Vec<f64>,
    pub(crate) agi_per_dodge: Vec<f64>,
    pub(crate) race_stats: RaceStats,
}

/// Naked `[str, agi, sta, int, spi]` per level, by race. Every Class has every race: a pair
/// vanilla never allowed is synthesised by `extract_class_levels.py` from the race's Warrior
/// offset.
#[derive(Deserialize)]
pub(crate) struct RaceStats {
    pub(crate) human: Vec<[u32; 5]>,
    pub(crate) orc: Vec<[u32; 5]>,
    pub(crate) dwarf: Vec<[u32; 5]>,
    pub(crate) night_elf: Vec<[u32; 5]>,
    pub(crate) undead: Vec<[u32; 5]>,
    pub(crate) tauren: Vec<[u32; 5]>,
    pub(crate) gnome: Vec<[u32; 5]>,
    pub(crate) troll: Vec<[u32; 5]>,
}

impl RaceStats {
    /// `ChrRaces.dbc` ids 1..=8.
    pub(crate) fn of(&self, race: u8) -> &[[u32; 5]] {
        match race {
            1 => &self.human,
            2 => &self.orc,
            3 => &self.dwarf,
            4 => &self.night_elf,
            5 => &self.undead,
            6 => &self.tauren,
            7 => &self.gnome,
            _ => &self.troll,
        }
    }
}

/// `creature_classlevelstats` for the three unit classes, indexed by level - 1.
#[derive(Deserialize)]
pub(crate) struct MobClassStats {
    pub(crate) warrior: MobClassLevels,
    pub(crate) paladin: MobClassLevels,
    pub(crate) mage: MobClassLevels,
}

#[derive(Deserialize)]
pub(crate) struct MobClassLevels {
    pub(crate) health: Vec<u32>,
    pub(crate) melee_damage_avg: Vec<f64>,
    pub(crate) armor: Vec<u32>,
}

pub(crate) fn levels() -> &'static Levels {
    static LEVELS: OnceLock<Levels> = OnceLock::new();
    LEVELS.get_or_init(|| {
        toml::from_str(include_str!("../content/levels.toml")).expect("content/levels.toml parses")
    })
}

pub(crate) fn mob_class_stats() -> &'static MobClassStats {
    static STATS: OnceLock<MobClassStats> = OnceLock::new();
    STATS.get_or_init(|| {
        toml::from_str(include_str!("../content/mob_class_stats.toml"))
            .expect("content/mob_class_stats.toml parses")
    })
}

/// Row `level` of a level-indexed table, clamped into it.
pub(crate) fn at<T: Copy>(table: &[T], level: u8) -> T {
    let i = usize::from(level.max(1) - 1).min(table.len() - 1);
    table[i]
}
