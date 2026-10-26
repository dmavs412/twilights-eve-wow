//! The game's rules as pure functions of their inputs, with vmangos (patch 1.12) as the reference
//! (`docs/reference/vanilla-combat-rules.md`). The realm's world runs on them, and so will the
//! systems built on top: aggro and leashing, skills, gear.

pub mod aggro;
pub mod durability;
pub mod forage;
pub mod game_clock;
pub mod gear;
pub mod haste;
pub mod leash;
pub mod loot;
pub mod melee;
pub mod notoriety;
pub mod power;
pub mod procs;
pub mod quest;
pub mod rage;
pub mod regen;
pub mod respawn;
pub mod school;
pub mod seals;
pub mod skills;
pub mod spell_power;
pub mod stats;
pub mod talents;
pub mod threat;
pub mod tokens;
pub mod xp;
