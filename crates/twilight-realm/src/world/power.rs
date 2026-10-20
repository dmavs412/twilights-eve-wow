//! The player's one power bar in the world: the kind its Calling runs on, what it holds, and its
//! regeneration on the two-second tick every kind shares (the rules are `rules::power` and
//! `rules::rage`).

use crate::fields::UNIT_FIELD_POWER1;
use crate::rules::power::{self, PowerKind};
use crate::rules::rage;
use crate::rules::stats::CharacterStats;

/// One bar, stored as its client field carries it: rage in tenths, energy, focus and mana whole.
pub(super) struct Power {
    pub(super) kind: PowerKind,
    pub(super) current: u32,
    pub(super) max: u32,
    /// Seconds into the current regen tick.
    tick: f32,
    /// The part of a mana tick under a whole point, carried to the next.
    fraction: f64,
    /// Seconds left of the pause a cast that spent mana started.
    pause: f32,
}

impl Power {
    /// A bar of the Calling's `kind` for `stats` at login or after Advancement: rage empty, the
    /// rest full (vmangos `Player::InitStatsForLevel`, `ResurrectPlayer`).
    pub(super) fn fresh(stats: &CharacterStats, kind: PowerKind) -> Self {
        let max = power::cap(kind, stats.max_mana, stats.max_energy_bonus);
        Self {
            kind,
            current: if kind == PowerKind::Rage { 0 } else { max },
            max,
            tick: 0.0,
            fraction: 0.0,
            pause: 0.0,
        }
    }

    /// The max follows new `stats` (a level, a piece of gear); what is held stays, clamped.
    pub(super) fn resize(&mut self, stats: &CharacterStats) {
        self.max = power::cap(self.kind, stats.max_mana, stats.max_energy_bonus);
        self.current = self.current.min(self.max);
    }

    /// The current value's field.
    pub(super) fn field(&self) -> (u16, u32) {
        (UNIT_FIELD_POWER1 + self.kind.id() as u16, self.current)
    }

    /// Whole points held.
    pub(super) fn points(&self) -> u32 {
        self.current / self.kind.stored(1)
    }

    pub(super) fn has(&self, points: u32) -> bool {
        self.current >= self.kind.stored(points)
    }

    /// Pays `points`; spending mana pauses its regeneration (`Player::SetLastManaUse`).
    pub(super) fn spend(&mut self, points: u32) {
        self.current = self.current.saturating_sub(self.kind.stored(points));
        if points > 0 && self.kind == PowerKind::Mana {
            self.pause = power::MANA_PAUSE_SECS;
        }
    }

    /// `points` of `kind`, capped; nothing at all when the bar is of another kind. Whether it
    /// changed.
    pub(super) fn gain(&mut self, kind: PowerKind, points: u32) -> bool {
        self.gain_stored(kind, kind.stored(points))
    }

    /// [`Self::gain`] in stored units (tenths of rage).
    pub(super) fn gain_stored(&mut self, kind: PowerKind, stored: u32) -> bool {
        if kind != self.kind || stored == 0 {
            return false;
        }
        let was = self.current;
        self.current = (self.current + stored).min(self.max);
        self.current != was
    }

    /// Every point of `kind` held, taken; 0 when the bar is of another kind (Execute).
    pub(super) fn take_all(&mut self, kind: PowerKind) -> u32 {
        if kind != self.kind {
            return 0;
        }
        let points = self.points();
        self.current = 0;
        points
    }

    /// `dt` seconds pass: rage decays out of combat, energy and focus fill, mana fills from spirit
    /// at `mana_per_tick` outside the five-second pause. Whether the value changed.
    pub(super) fn regenerate(&mut self, dt: f32, in_combat: bool, mana_per_tick: f64) -> bool {
        let was = self.current;
        self.pause = (self.pause - dt).max(0.0);
        if self.kind == PowerKind::Rage && (in_combat || self.current == 0) {
            // The first decay tick starts when the fight ends.
            self.tick = 0.0;
            return false;
        }
        self.tick += dt;
        while self.tick >= power::REGEN_TICK_SECS {
            self.tick -= power::REGEN_TICK_SECS;
            match self.kind {
                PowerKind::Rage => {
                    self.current = self.current.saturating_sub(rage::DECAY_PER_TICK);
                    if self.current == 0 {
                        self.tick = 0.0;
                        break;
                    }
                }
                PowerKind::Energy => {
                    self.current = (self.current + power::ENERGY_PER_TICK).min(self.max);
                }
                PowerKind::Focus => {
                    self.current = (self.current + power::FOCUS_PER_TICK).min(self.max);
                }
                PowerKind::Mana => {
                    if self.pause > 0.0 {
                        continue;
                    }
                    self.fraction += mana_per_tick;
                    let whole = self.fraction.floor();
                    self.fraction -= whole;
                    self.current = (self.current + whole as u32).min(self.max);
                }
            }
        }
        self.current != was
    }
}

/// Combo points: on one target at a time, five at most (vmangos `Player::AddComboPoints`,
/// `ClearComboPoints`). The client shows them in `PLAYER_FIELD_BYTES` byte 1 against
/// `PLAYER_FIELD_COMBO_TARGET`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct Combo {
    pub(super) target: u64,
    pub(super) count: u8,
}

impl Combo {
    pub(super) const MAX: u8 = 5;

    /// `amount` points on `target`; points on another target are lost first.
    pub(super) fn add(&mut self, target: u64, amount: u32) {
        if target != self.target {
            *self = Self::default();
        }
        self.target = target;
        self.count = (u32::from(self.count) + amount).min(u32::from(Self::MAX)) as u8;
    }

    /// The points held on `target`.
    pub(super) fn on(&self, target: u64) -> u8 {
        if target == self.target {
            self.count
        } else {
            0
        }
    }

    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
}
