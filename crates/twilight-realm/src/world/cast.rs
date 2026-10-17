//! The character's skills in the world: the spellbook at login, casting them, and the auras they
//! leave. Every skill is cast from its data ([`crate::callings`]); nothing here names a skill.
//!
//! An instant cast answers as vmangos's does: `SMSG_SPELL_START`, `SMSG_CAST_RESULT` (ok),
//! `SMSG_SPELL_GO` with its hits and misses, then what each effect did (damage, energize, the aura
//! fields). A refusal is a lone failed `SMSG_CAST_RESULT` with the stock reason. An on-next-swing
//! skill answers nothing until the swing it replaces.

use benilla_protocol::messages::{
    ActionButton, MonsterMoveFacing, PeriodicAuraLog, PeriodicTick, SpellCooldown, SpellDamageLog,
    SpellEnergizeLog,
};
use benilla_protocol::SessionEvent;

use super::{in_front, melee_reach, InWorld, Ticked};
use crate::callings::{self, Effect, Seal, Skill, Target, Weapon};
use crate::characters::Char;
use crate::clock;
use crate::fields::*;
use crate::items;
use crate::mobs::State;
use crate::rng::Rng;
use crate::rules::haste::{haste_cut_damage, hasted_attack_time};
use crate::rules::melee::{Outcome, PlayerOffense, ROLL};
use crate::rules::rage;
use crate::rules::school::{crit_scaled, mitigated, School};
use crate::rules::seals;
use crate::rules::skills::{self as formulas, ability_table};
use crate::rules::stats::WarriorStats;
use crate::rules::talents::{crit_damage, deep_wound_tick, heal_tick, scaled};
use crate::space::distance;

/// `UNIT_FIELD_BYTES_1`: stand state, loyalty, shapeshift form, visibility flags.
const UNIT_FIELD_BYTES_1: u16 = 138;
const UNIT_FIELD_AURA: u16 = 47;
/// Four bits a slot, eight slots a dword.
const UNIT_FIELD_AURAFLAGS: u16 = 95;
/// A byte a slot: the caster's level.
const UNIT_FIELD_AURALEVELS: u16 = 101;
/// A byte a slot: stacks - 1.
const UNIT_FIELD_AURAAPPLICATIONS: u16 = 113;
/// Two i16s: the flat positive and negative attack power mods.
const UNIT_FIELD_ATTACK_POWER_MODS: u16 = 166;
const PLAYER_FIELD_COMBO_TARGET: u16 = 714;
/// Byte 1: combo points.
const PLAYER_FIELD_BYTES: u16 = 1222;
/// The aura-state bits, and the one for a unit at or below 20% health
/// (`AURA_STATE_HEALTHLESS_20_PERCENT`, bit 2 of `TargetAuraState`).
const UNIT_FIELD_AURASTATE: u16 = 125;
const AURA_STATE_HEALTHLESS_20: u32 = 0x2;

/// `AFLAG_CANCELABLE`: a buff the player may click off.
const AFLAG_CANCELABLE: u32 = 0x01;
/// `AFLAG_EFF_INDEX_0`: the slot is live.
const AFLAG_EFF_INDEX_0: u32 = 0x02;
/// Slots below this hold buffs, the rest debuffs.
const POSITIVE_SLOTS: u8 = 32;
const AURA_SLOTS: u8 = 48;

// `SpellCastResult` (vmangos `SpellDefines.h`).
const SPELL_FAILED_AFFECTING_COMBAT: u8 = 0x00;
const SPELL_FAILED_BAD_TARGETS: u8 = 0x0a;
const SPELL_FAILED_CASTER_AURASTATE: u8 = 0x12;
const SPELL_FAILED_CASTER_DEAD: u8 = 0x13;
const SPELL_FAILED_NOT_KNOWN: u8 = 0x38;
const SPELL_FAILED_NOT_READY: u8 = 0x3c;
const SPELL_FAILED_NO_POWER: u8 = 0x4d;
const SPELL_FAILED_OUT_OF_RANGE: u8 = 0x59;
const SPELL_FAILED_TARGET_AURASTATE: u8 = 0x67;
const SPELL_FAILED_TARGETS_DEAD: u8 = 0x65;
const SPELL_FAILED_TOO_CLOSE: u8 = 0x76;
const SPELL_FAILED_UNIT_NOT_INFRONT: u8 = 0x7c;

// `SpellMissInfo`, as `SMSG_SPELL_GO`'s miss list carries it.
const SPELL_MISS_MISS: u8 = 1;
const SPELL_MISS_DODGE: u8 = 3;
const SPELL_MISS_PARRY: u8 = 4;
const SPELL_MISS_BLOCK: u8 = 5;

/// `SPELL_HIT_TYPE_CRIT` in a spell damage log.
const SPELL_HIT_TYPE_CRIT: u32 = 0x2;
/// `POWER_RAGE` in an energize log.
const POWER_RAGE: u32 = 1;

/// How long a dodge leaves Overpower open, and the combo point that marks it (vmangos).
const DODGE_WINDOW_SECS: f32 = 5.0;
/// A cooldown counts as over this close to its end: the client's clock and the realm's tick
/// differ by up to a frame, and the client sends the cast the moment its own clock says ready.
const READY_SLACK_SECS: f32 = 0.1;
/// Seconds of float drift an aura clock forgives, so a tick and the fade due on the same tick
/// both land.
const CLOCK_EPSILON: f32 = 1e-3;
/// How far short of the target a charge stops.
const CHARGE_STOP_YD: f32 = 2.0;

/// The skills' lasting state: cooldowns, the global cooldown, the queued strike, the auras.
#[derive(Default)]
pub(super) struct Skills {
    /// `(spell, seconds left)`.
    cooldowns: Vec<(u32, f32)>,
    gcd: f32,
    /// The on-next-swing skill waiting for the next white swing.
    queued: Option<u32>,
    /// The mob that last dodged, and the seconds Overpower stays open on it.
    dodged: Option<(u64, f32)>,
    /// Seconds the caster is held in combat (Bloodrage).
    combat_hold: f32,
    auras: Vec<Aura>,
    /// The timed cast under way: an item's on-use spell (the Hearthstone's).
    casting: Option<TimedCast>,
    /// The mobs whose aura state says they are at or below a fifth of their health.
    low_health: Vec<u64>,
    /// The player's guid, for the aura packets only the player's own auras get.
    me: u64,
}

/// A cast with a cast bar, from an item.
#[derive(Debug, Clone, Copy)]
struct TimedCast {
    spell_id: u32,
    /// The item it is cast from, and that item's template entry.
    item: u64,
    entry: u32,
    /// Seconds until it goes off.
    left: f32,
}

/// One aura on a unit.
#[derive(Debug, Clone, Copy)]
pub(super) struct Aura {
    unit: u64,
    spell_id: u32,
    slot: u8,
    level: u8,
    /// Seconds left.
    left: f32,
    /// Seconds between ticks, 0 for none, and until the next.
    period: f32,
    next_tick: f32,
    kind: AuraKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AuraKind {
    Bleed {
        per_tick: u32,
    },
    Rage {
        per_tick: u32,
    },
    AttackPower(u32),
    AttackSlow {
        pct: u32,
    },
    MoveSlow {
        pct: u32,
    },
    Stun,
    /// A stun that damage ends when `break_on_damage` (Gouge).
    Incapacitate {
        break_on_damage: bool,
    },
    /// Healing on the unit is cut (Mortal Strike's wound). Nothing heals a mob in a fight yet,
    /// so the debuff is all it does.
    HealingReduced,
    /// The player's maximum health is `pct`% higher (Last Stand).
    MaxHealthPct(u32),
    /// The player deals `pct`% more physical damage, with `armor_loss_pct`% less armour; for
    /// `swings` more white swings when it counts them (Death Wish, Enrage).
    DamagePct {
        pct: u32,
        armor_loss_pct: u32,
        swings: Option<u32>,
    },
    /// The player's next `strikes` landed melee blows also strike another enemy within
    /// `radius_yd` (Sweeping Strikes).
    Sweeping {
        strikes: u32,
        radius_yd: f32,
        strike_spell_id: u32,
    },
    /// The player heals `per_tick` a tick (Blood Craze).
    Heal {
        per_tick: u32,
    },
    /// The player's one active Seal, and the attack power it adds (Seal of the Crusader's, sized
    /// when cast).
    Seal {
        seal: Seal,
        attack_power: u32,
    },
    /// The unit takes `pct`% more holy damage (Judgement of the Crusader); the player's landed
    /// swings at it put it back to its full `duration_ms`.
    HolyTaken {
        pct: u32,
        duration_ms: u32,
    },
}

impl AuraKind {
    fn helpful(self) -> bool {
        matches!(
            self,
            AuraKind::Rage { .. }
                | AuraKind::AttackPower(_)
                | AuraKind::MaxHealthPct(_)
                | AuraKind::DamagePct { .. }
                | AuraKind::Sweeping { .. }
                | AuraKind::Heal { .. }
                | AuraKind::Seal { .. }
        )
    }
}

impl Skills {
    pub(super) fn new(me: u64) -> Self {
        Self {
            me,
            ..Self::default()
        }
    }

    /// The flat attack power the player's buffs add.
    fn attack_power_bonus(&self) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .map(|a| match a.kind {
                AuraKind::AttackPower(n) => n,
                AuraKind::Seal { attack_power, .. } => attack_power,
                _ => 0,
            })
            .sum()
    }

    /// How much faster the player's buffs make their swings, in percent (Seal of the
    /// Crusader's).
    fn haste_pct(&self) -> u32 {
        match self.seal() {
            Some(Seal::Crusader { haste_pct, .. }) => haste_pct,
            _ => 0,
        }
    }

    /// The percent more holy damage `unit` takes (Judgement of the Crusader).
    fn holy_taken_pct(&self, unit: u64) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == unit)
            .map(|a| match a.kind {
                AuraKind::HolyTaken { pct, .. } => pct,
                _ => 0,
            })
            .sum()
    }

    /// A landed swing at `unit` puts its judgement back to its full duration ("Your melee
    /// strikes will refresh the spell's duration").
    fn refresh_judgement(&mut self, unit: u64) {
        for a in self.auras.iter_mut().filter(|a| a.unit == unit) {
            if let AuraKind::HolyTaken { duration_ms, .. } = a.kind {
                a.left = clock::secs(duration_ms);
            }
        }
    }

    /// The percents the player's buffs raise their maximum health by, raise the physical damage
    /// they deal by, and take off their armour.
    fn buff_pcts(&self) -> (u32, u32, u32) {
        self.auras.iter().filter(|a| a.unit == self.me).fold(
            (0, 0, 0),
            |(health, damage, armor), a| match a.kind {
                AuraKind::MaxHealthPct(pct) => (health + pct, damage, armor),
                AuraKind::DamagePct {
                    pct,
                    armor_loss_pct,
                    ..
                } => (health, damage + pct, armor + armor_loss_pct),
                _ => (health, damage, armor),
            },
        )
    }

    /// The percent the player's buffs raise the physical damage they deal by.
    pub(super) fn damage_pct(&self) -> u32 {
        self.buff_pcts().1
    }

    /// A white swing went: the buffs that count swings count one, and go at their last. What
    /// went.
    pub(super) fn count_swing(&mut self, events: &mut Vec<SessionEvent>) -> Vec<Aura> {
        let me = self.me;
        for a in self.auras.iter_mut().filter(|a| a.unit == me) {
            if let AuraKind::DamagePct {
                swings: Some(n), ..
            } = &mut a.kind
            {
                *n = n.saturating_sub(1);
            }
        }
        self.remove_where(
            |a| {
                a.unit == me
                    && matches!(
                        a.kind,
                        AuraKind::DamagePct {
                            swings: Some(0),
                            ..
                        }
                    )
            },
            events,
        )
    }

    /// The player's active Seal.
    fn seal(&self) -> Option<Seal> {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .find_map(|a| match a.kind {
                AuraKind::Seal { seal, .. } => Some(seal),
                _ => None,
            })
    }

    /// Whether a skill holds the player in combat.
    pub(super) fn holds_combat(&self) -> bool {
        self.combat_hold > 0.0
    }

    /// Whether `unit` is stunned.
    pub(super) fn stunned(&self, unit: u64) -> bool {
        self.auras.iter().any(|a| {
            a.unit == unit && matches!(a.kind, AuraKind::Stun | AuraKind::Incapacitate { .. })
        })
    }

    /// What `unit`'s swing time is multiplied by.
    pub(super) fn attack_time_mul(&self, unit: u64) -> f32 {
        self.multiplier(unit, |kind| match kind {
            AuraKind::AttackSlow { pct } => Some(1.0 + pct as f32 / 100.0),
            _ => None,
        })
    }

    /// What `unit`'s run speed is multiplied by.
    fn speed_mul(&self, unit: u64) -> f32 {
        self.multiplier(unit, |kind| match kind {
            AuraKind::MoveSlow { pct } => Some(1.0 - pct as f32 / 100.0),
            _ => None,
        })
    }

    /// The product of the factors `factor` reads off `unit`'s auras; 1 with none.
    fn multiplier(&self, unit: u64, factor: impl Fn(AuraKind) -> Option<f32>) -> f32 {
        self.auras
            .iter()
            .filter(|a| a.unit == unit)
            .filter_map(|a| factor(a.kind))
            .product()
    }

    fn cooldown_left(&self, spell_id: u32) -> f32 {
        self.cooldowns
            .iter()
            .find(|(s, _)| *s == spell_id)
            .map_or(0.0, |(_, left)| *left)
    }

    /// The queued on-next-swing skill, taken off the queue.
    pub(super) fn take_queued(&mut self) -> Option<u32> {
        self.queued.take()
    }

    /// `mob` dodged the player: Overpower opens on it, marked with a combo point.
    pub(super) fn dodged_by(&mut self, mob: u64, events: &mut Vec<SessionEvent>) {
        self.dodged = Some((mob, DODGE_WINDOW_SECS));
        let mut pairs = vec![(PLAYER_FIELD_BYTES, 1 << 8)];
        pairs.extend(guid_pairs(PLAYER_FIELD_COMBO_TARGET, mob));
        events.push(values(self.me, &pairs));
    }

    fn close_dodge_window(&mut self, events: &mut Vec<SessionEvent>) {
        if self.dodged.take().is_some() {
            let mut pairs = vec![(PLAYER_FIELD_BYTES, 0)];
            pairs.extend(guid_pairs(PLAYER_FIELD_COMBO_TARGET, 0));
            events.push(values(self.me, &pairs));
        }
    }

    /// Puts `aura` on its unit, in the slot its spell already holds there or a free one.
    fn apply(&mut self, mut aura: Aura, events: &mut Vec<SessionEvent>) -> Option<Aura> {
        if let Some(old) = self
            .auras
            .iter_mut()
            .find(|a| a.unit == aura.unit && a.spell_id == aura.spell_id)
        {
            aura.slot = old.slot;
            *old = aura;
        } else {
            let range = if aura.kind.helpful() {
                0..POSITIVE_SLOTS
            } else {
                POSITIVE_SLOTS..AURA_SLOTS
            };
            let taken: Vec<u8> = self
                .auras
                .iter()
                .filter(|a| a.unit == aura.unit)
                .map(|a| a.slot)
                .collect();
            aura.slot = range.into_iter().find(|s| !taken.contains(s))?;
            self.auras.push(aura);
        }
        if aura.unit == self.me {
            events.push(SessionEvent::AuraDuration {
                slot: aura.slot,
                remaining_ms: clock::ms(aura.left),
            });
        }
        events.push(self.slot_fields(aura.unit, aura.slot));
        Some(aura)
    }

    /// Takes every aura `keep` refuses off, clearing their slots.
    fn remove_where(
        &mut self,
        mut drop: impl FnMut(&Aura) -> bool,
        events: &mut Vec<SessionEvent>,
    ) -> Vec<Aura> {
        let (gone, kept): (Vec<Aura>, Vec<Aura>) = std::mem::take(&mut self.auras)
            .into_iter()
            .partition(|a| drop(a));
        self.auras = kept;
        for a in &gone {
            events.push(self.slot_fields(a.unit, a.slot));
        }
        gone
    }

    /// The `UNIT_FIELD_AURA*` dwords that hold `slot` on `unit`, as its auras now stand.
    fn slot_fields(&self, unit: u64, slot: u8) -> SessionEvent {
        let on_unit = |s: u8| self.auras.iter().find(|a| a.unit == unit && a.slot == s);
        let flags_word = slot / 8;
        let flags = (0..8).fold(0u32, |w, i| {
            let s = flags_word * 8 + i;
            let nibble = on_unit(s).map_or(0, |a| {
                AFLAG_EFF_INDEX_0
                    | if a.kind.helpful() {
                        AFLAG_CANCELABLE
                    } else {
                        0
                    }
            });
            w | nibble << (4 * u32::from(i))
        });
        let byte_word = slot / 4;
        let levels = (0..4).fold(0u32, |w, i| {
            let level = on_unit(byte_word * 4 + i).map_or(0, |a| u32::from(a.level));
            w | level << (8 * u32::from(i))
        });
        values(
            unit,
            &[
                (
                    UNIT_FIELD_AURA + u16::from(slot),
                    on_unit(slot).map_or(0, |a| a.spell_id),
                ),
                (UNIT_FIELD_AURAFLAGS + u16::from(flags_word), flags),
                (UNIT_FIELD_AURALEVELS + u16::from(byte_word), levels),
                // One stack each.
                (UNIT_FIELD_AURAAPPLICATIONS + u16::from(byte_word), 0),
            ],
        )
    }
}

/// `SMSG_INITIAL_SPELLS`: the current Calling's skills and learned talents' rank spells, and the
/// item cooldowns still running at the wall clock's `wall` (the skills' own start afresh each
/// session).
pub(super) fn spell_book(c: &Char, wall: f64) -> SessionEvent {
    SessionEvent::SpellBook {
        spell_ids: callings::of(c.calling)
            .skills
            .iter()
            .map(|s| s.spell_id)
            .chain(crate::talents::known_spells(c))
            .collect(),
        cooldowns: c
            .item_cooldowns
            .iter()
            .filter(|&&(_, ready)| ready > wall)
            .filter_map(|&(entry, ready)| {
                let t = items::template(entry)?;
                Some(SpellCooldown {
                    spell_id: t.use_spell as u16,
                    item_id: entry as u16,
                    category: 0,
                    spell_cd_ms: clock::ms((ready - wall) as f32),
                    category_cd_ms: 0,
                })
            })
            .collect(),
    }
}

/// `SMSG_ACTION_BUTTONS`: the bar as the player left it, or the Calling's default.
pub(super) fn action_buttons(c: &Char) -> SessionEvent {
    let bar = c
        .action_bar
        .clone()
        .unwrap_or_else(|| callings::default_bar(c.calling));
    SessionEvent::ActionButtons {
        buttons: bar
            .into_iter()
            .map(|(slot, packed)| ActionButton {
                slot,
                action: packed & 0x00FF_FFFF,
                kind: (packed >> 24) as u8,
            })
            .collect(),
    }
}

/// The Calling's stance in `UNIT_FIELD_BYTES_1` byte 2: the client checks a skill's stances
/// against it before it sends the cast.
pub(super) fn stance(c: &Char) -> (u16, u32) {
    (
        UNIT_FIELD_BYTES_1,
        u32::from(callings::of(c.calling).shapeshift_form) << 16,
    )
}

/// The player's stats with their buffs: what strikes roll from. White swings roll from
/// [`white`], which adds the haste.
pub(super) fn buffed(stats: &WarriorStats, skills: &Skills) -> WarriorStats {
    let bonus = skills.attack_power_bonus();
    let mut s = *stats;
    let (health_pct, _, armor_loss_pct) = skills.buff_pcts();
    s.max_health += s.max_health * health_pct / 100;
    s.armor -= s.armor * armor_loss_pct.min(100) / 100;
    if bonus > 0 {
        s.attack_power += bonus as i32;
        let extra = formulas::ap_damage(bonus as i32, clock::secs(s.attack_time_ms));
        s.min_damage += extra;
        s.max_damage += extra;
    }
    s
}

/// [`buffed`] with the haste: what white swings roll from and time by, and what the sheet shows.
/// Faster swings, each white hit cut to match, so the haste alone adds no damage a second; a
/// strike (Mortal Strike, Heroic Strike) keeps its full weapon roll.
pub(super) fn white(stats: &WarriorStats, skills: &Skills) -> WarriorStats {
    let mut s = buffed(stats, skills);
    let haste = skills.haste_pct();
    if haste > 0 {
        s.attack_time_ms = hasted_attack_time(s.attack_time_ms, haste);
        s.min_damage = haste_cut_damage(s.min_damage, haste);
        s.max_damage = haste_cut_damage(s.max_damage, haste);
    }
    s
}

/// The sheet fields buffs move: the attack power mods, the damage they add and the swing time.
fn buff_sheet(stats: &WarriorStats, skills: &Skills) -> Vec<(u16, u32)> {
    let s = white(stats, skills);
    vec![
        (
            UNIT_FIELD_ATTACK_POWER_MODS,
            skills.attack_power_bonus() & 0xFFFF,
        ),
        (UNIT_FIELD_MINDAMAGE, float(s.min_damage)),
        (UNIT_FIELD_MAXDAMAGE, float(s.max_damage)),
        (UNIT_FIELD_BASEATTACKTIME, s.attack_time_ms),
        (UNIT_FIELD_BASEATTACKTIME + 1, s.attack_time_ms),
    ]
}

/// How one strike against one target went.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Landed {
    Hit,
    Crit,
    Missed(u8),
}

fn refused(spell_id: u32, reason: u8) -> SessionEvent {
    SessionEvent::CastResult {
        spell_id,
        success: false,
        reason: Some(reason),
        arg: None,
    }
}

/// `SMSG_SPELLNONMELEEDAMAGELOG` for `damage` of `school` from `attacker`'s `spell_id`: nothing
/// absorbed, resisted or blocked, flagged as a crit when it was one.
fn damage_log(
    attacker: u64,
    target: u64,
    spell_id: u32,
    damage: u32,
    school: School,
    crit: bool,
) -> SpellDamageLog {
    SpellDamageLog {
        target,
        attacker,
        spell_id,
        damage,
        school: school.index(),
        absorb: 0,
        resist: 0,
        periodic: false,
        blocked: 0,
        hit_info: if crit { SPELL_HIT_TYPE_CRIT } else { 0 },
    }
}

impl InWorld {
    /// `CMSG_CAST_SPELL`: refused with the stock reason, queued on the next swing, or cast.
    pub(crate) fn cast(
        &mut self,
        spell_id: u32,
        target: Option<u64>,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) -> Ticked {
        let mut ticked = Ticked::default();
        let Some(skill) = crate::talents::skill(c, spell_id) else {
            events.push(refused(spell_id, SPELL_FAILED_NOT_KNOWN));
            return ticked;
        };
        let targets = match self.check(skill, target, c) {
            Ok(targets) => targets,
            Err(reason) => {
                events.push(refused(spell_id, reason));
                return ticked;
            }
        };
        if skill.effects.iter().any(|e| {
            matches!(
                e,
                Effect::WeaponStrike {
                    on_next_swing: true,
                    ..
                }
            )
        }) {
            // Paid and answered when the swing comes (`CURRENT_MELEE_SPELL`).
            self.skills.queued = Some(spell_id);
            return ticked;
        }
        self.pay(skill, c, events);

        // Roll who it lands on first: `SMSG_SPELL_GO` lists the misses.
        let level = c.level;
        let landed: Vec<(u64, Landed)> = targets
            .iter()
            .map(|&t| (t, self.roll(skill, t, c, rng)))
            .collect();
        let hits: Vec<u64> = landed
            .iter()
            .filter(|(_, l)| !matches!(l, Landed::Missed(_)))
            .map(|(t, _)| *t)
            .collect();
        let misses: Vec<(u64, u8)> = landed
            .iter()
            .filter_map(|(t, l)| match l {
                Landed::Missed(why) => Some((*t, *why)),
                _ => None,
            })
            .collect();
        events.push(SessionEvent::SpellStart {
            caster: self.guid,
            spell_id,
            cast_flags: 0,
            cast_time_ms: 0,
            target,
            ammo_display_id: None,
        });
        events.push(SessionEvent::CastResult {
            spell_id,
            success: true,
            reason: None,
            arg: None,
        });
        events.push(SessionEvent::SpellGo {
            caster: self.guid,
            spell_id,
            cast_flags: 0,
            hits: hits.clone(),
            misses: misses.clone(),
            target,
            go_target: None,
            dest: None,
            ammo_display_id: None,
            item_caster: None,
        });
        for &(guid, why) in &misses {
            if why == SPELL_MISS_DODGE {
                self.skills.dodged_by(guid, events);
            }
        }
        for (guid, how) in landed {
            if !matches!(how, Landed::Missed(_)) {
                for effect in &skill.effects {
                    self.effect(skill, effect, guid, how, level, c, rng, events, &mut ticked);
                }
            }
            // A cast at a mob, landed or not, turns it on the player.
            if let Some(mob) = self.mobs.iter_mut().find(|m| m.guid == guid) {
                if mob.state == State::Idle {
                    super::engage(mob, self.guid, events);
                }
            }
        }
        let p = &self.player;
        events.push(values(
            self.guid,
            &[(UNIT_FIELD_HEALTH, p.health), (UNIT_FIELD_POWER2, p.rage)],
        ));
        self.update_aura_states(events);
        ticked
    }

    /// Whether `skill` can go off now at `target`, and on whom.
    fn check(&self, skill: &Skill, target: Option<u64>, c: &Char) -> Result<Vec<u64>, u8> {
        let p = &self.player;
        if !p.alive() {
            return Err(SPELL_FAILED_CASTER_DEAD);
        }
        if self.skills.cooldown_left(skill.spell_id) > READY_SLACK_SECS
            || (skill.gcd_ms > 0 && self.skills.gcd > READY_SLACK_SECS)
        {
            return Err(SPELL_FAILED_NOT_READY);
        }
        if p.rage < crate::talents::rage_cost(c, skill) * 10 {
            return Err(SPELL_FAILED_NO_POWER);
        }
        if skill.requires_out_of_combat && p.in_combat {
            return Err(SPELL_FAILED_AFFECTING_COMBAT);
        }
        // Judgement needs a Seal to judge.
        if skill.effects.contains(&Effect::Judgement) && self.skills.seal().is_none() {
            return Err(SPELL_FAILED_CASTER_AURASTATE);
        }
        match skill.target {
            Target::Caster => Ok(vec![self.guid]),
            Target::SelfArea => Ok(self
                .mobs
                .iter()
                .filter(|m| m.alive() && distance(m.position, c.position) <= skill.radius_yd)
                .take(skill.max_targets as usize)
                .map(|m| m.guid)
                .collect()),
            Target::Enemy => {
                let guid = target.ok_or(SPELL_FAILED_BAD_TARGETS)?;
                let mob = self
                    .mobs
                    .iter()
                    .find(|m| m.guid == guid)
                    .ok_or(SPELL_FAILED_BAD_TARGETS)?;
                if !mob.alive() {
                    return Err(SPELL_FAILED_TARGETS_DEAD);
                }
                let d = distance(mob.position, c.position);
                let [min, max] = skill.range_yd;
                // A 5 yd skill is a melee one: it reaches as far as a swing does.
                let max = if max <= 5.0 { melee_reach() } else { max };
                if d > max {
                    return Err(SPELL_FAILED_OUT_OF_RANGE);
                }
                if d < min {
                    return Err(SPELL_FAILED_TOO_CLOSE);
                }
                if !in_front(c.orientation, c.position, mob.position) {
                    return Err(SPELL_FAILED_UNIT_NOT_INFRONT);
                }
                if skill.requires_target_dodged
                    && !self.skills.dodged.is_some_and(|(m, _)| m == guid)
                {
                    return Err(SPELL_FAILED_CASTER_AURASTATE);
                }
                if skill.requires_target_health_pct > 0
                    && u64::from(mob.health) * 100
                        > u64::from(mob.stats.max_health)
                            * u64::from(skill.requires_target_health_pct)
                {
                    return Err(SPELL_FAILED_TARGET_AURASTATE);
                }
                Ok(vec![guid])
            }
        }
    }

    /// Takes the cast's costs and starts its cooldowns.
    fn pay(&mut self, skill: &Skill, c: &Char, events: &mut Vec<SessionEvent>) {
        let p = &mut self.player;
        p.rage -= crate::talents::rage_cost(c, skill) * 10;
        let health_cost = p.stats.base_health * skill.health_cost_pct_of_base_health / 100;
        // A cost never kills.
        p.health = p.health.saturating_sub(health_cost).max(1);
        let s = &mut self.skills;
        if skill.cooldown_ms > 0 {
            s.cooldowns.retain(|(id, _)| *id != skill.spell_id);
            s.cooldowns
                .push((skill.spell_id, clock::secs(skill.cooldown_ms)));
        }
        if skill.gcd_ms > 0 {
            s.gcd = clock::secs(skill.gcd_ms);
        }
        if skill.puts_in_combat_ms > 0 {
            s.combat_hold = s.combat_hold.max(clock::secs(skill.puts_in_combat_ms));
        }
        if skill.requires_target_dodged {
            s.close_dodge_window(events);
        }
    }

    /// The melee table's verdict on `skill` at `target`: a strike or a hit-table skill rolls
    /// avoidance, then crit (the two-roll yellow model); anything else lands.
    fn roll(&self, skill: &Skill, target: u64, c: &Char, rng: &mut Rng) -> Landed {
        let mut rolls = false;
        let mut unavoidable = false;
        for e in &skill.effects {
            match *e {
                Effect::WeaponStrike { unavoidable: u, .. } => {
                    rolls = true;
                    unavoidable |= u;
                }
                Effect::SchoolDamage {
                    melee_hit_table: true,
                    ..
                }
                | Effect::Execute { .. } => rolls = true,
                // Magic misses on the level-based chance alone.
                Effect::SchoolDamage { school, .. } if school.is_magic() => {
                    rolls = true;
                    unavoidable = true;
                }
                _ => {}
            }
        }
        let Some(mob) = self.mobs.iter().find(|m| m.guid == target) else {
            return Landed::Hit;
        };
        if !rolls {
            return Landed::Hit;
        }
        let stats = buffed(&self.player.stats, &self.skills);
        let from_front = in_front(mob.orientation, mob.position, c.position);
        let (table, crit) = ability_table(
            PlayerOffense {
                level: stats.level,
                weapon_skill: stats.weapon_skill,
                crit_pct: stats.crit_pct,
            },
            mob.defense(),
            from_front,
            unavoidable,
        );
        match table.outcome(rng.below(ROLL)) {
            Outcome::Miss => Landed::Missed(SPELL_MISS_MISS),
            Outcome::Dodge => Landed::Missed(SPELL_MISS_DODGE),
            Outcome::Parry => Landed::Missed(SPELL_MISS_PARRY),
            Outcome::Block => Landed::Missed(SPELL_MISS_BLOCK),
            // A talent's extra chance for this skill (Improved Overpower), in basis points.
            _ if rng.below(ROLL) < crit + crate::talents::skill_crit_pct(c, &skill.key) * 100 => {
                Landed::Crit
            }
            _ => Landed::Hit,
        }
    }

    /// One effect of a landed cast on `target`.
    #[allow(clippy::too_many_arguments)]
    fn effect(
        &mut self,
        skill: &Skill,
        effect: &Effect,
        target: u64,
        how: Landed,
        level: u8,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let stats = buffed(&self.player.stats, &self.skills);
        let ap = stats.attack_power;
        // Talents that raise the skill's amounts (Improved Rend, Improved Battle Shout).
        let pct = crate::talents::amount_pct(c, &skill.key);
        let sized = |l: formulas::Linear| scaled(l.at(level, ap), pct);
        // Talents that make the skill's auras last longer (Booming Voice).
        let longer = crate::talents::duration_pct(c, &skill.key);
        let aura = |kind: AuraKind, spell_id: u32, duration_ms: u32, period_ms: u32| Aura {
            unit: target,
            spell_id,
            slot: 0,
            level,
            left: clock::secs(scaled(duration_ms, longer)),
            period: clock::secs(period_ms),
            next_tick: clock::secs(period_ms),
            kind,
        };
        match *effect {
            Effect::WeaponStrike { weapon, bonus, .. } => {
                let raw = weapon_roll(&stats, weapon, rng) + sized(bonus) as f32;
                self.strike(
                    skill.spell_id,
                    target,
                    raw as u32,
                    School::Physical,
                    how,
                    c,
                    rng,
                    events,
                    ticked,
                );
            }
            Effect::SchoolDamage { school, amount, .. } => {
                let raw = sized(amount);
                self.strike(
                    skill.spell_id,
                    target,
                    raw,
                    school,
                    how,
                    c,
                    rng,
                    events,
                    ticked,
                );
            }
            Effect::PeriodicDamage {
                tick_ms,
                duration_ms,
                tick,
            } => {
                let per_tick = sized(tick);
                let a = aura(
                    AuraKind::Bleed { per_tick },
                    skill.spell_id,
                    duration_ms,
                    tick_ms,
                );
                self.skills.apply(a, events);
            }
            Effect::Charge { speed_yd_s } => {
                let Some(mob) = self.mobs.iter().find(|m| m.guid == target) else {
                    return;
                };
                let (from, to) = (c.position, mob.position);
                let d = distance(from, to);
                let t = ((d - CHARGE_STOP_YD) / d).max(0.0);
                let dest = std::array::from_fn(|i| from[i] + (to[i] - from[i]) * t);
                let duration = distance(from, dest) / speed_yd_s;
                c.position = dest;
                c.orientation = (to[1] - from[1])
                    .atan2(to[0] - from[0])
                    .rem_euclid(std::f32::consts::TAU);
                events.push(SessionEvent::MonsterMove {
                    guid: self.guid,
                    transport: None,
                    start: from,
                    spline_id: 1,
                    path: vec![from, dest],
                    facing: MonsterMoveFacing::Target(target),
                    stop: false,
                    duration_ms: clock::ms(duration),
                    flying: false,
                    run_mode: true,
                });
            }
            Effect::EnergizeRage { amount } => {
                let gained = amount.at(level, ap) + crate::talents::extra_rage(c, &skill.key);
                let p = &mut self.player;
                p.rage = (p.rage + gained * 10).min(rage::MAX);
                events.push(SessionEvent::SpellEnergizeLog(SpellEnergizeLog {
                    target: self.guid,
                    caster: self.guid,
                    spell_id: skill.spell_id,
                    power: POWER_RAGE,
                    amount: gained,
                }));
            }
            Effect::PeriodicRage {
                aura_spell_id,
                per_tick,
                tick_ms,
                duration_ms,
            } => {
                let a = aura(
                    AuraKind::Rage { per_tick },
                    aura_spell_id,
                    duration_ms,
                    tick_ms,
                );
                self.skills.apply(a, events);
            }
            Effect::Stun {
                aura_spell_id,
                duration_ms,
            } => {
                self.skills
                    .apply(aura(AuraKind::Stun, aura_spell_id, duration_ms, 0), events);
            }
            Effect::Incapacitate {
                duration_ms,
                break_on_damage,
                drop_combat,
            } => {
                // Damage in the same cast came first, so it does not wake the target.
                if self.mobs.iter().any(|m| m.guid == target && m.alive()) {
                    let kind = AuraKind::Incapacitate { break_on_damage };
                    self.skills
                        .apply(aura(kind, skill.spell_id, duration_ms, 0), events);
                    if drop_combat {
                        self.drop_combat(target, events);
                    }
                }
            }
            Effect::AttackSpeedSlow { pct, duration_ms } => {
                let a = aura(AuraKind::AttackSlow { pct }, skill.spell_id, duration_ms, 0);
                self.skills.apply(a, events);
            }
            Effect::MovementSlow { pct, duration_ms } => {
                let a = aura(AuraKind::MoveSlow { pct }, skill.spell_id, duration_ms, 0);
                self.skills.apply(a, events);
                self.update_speed(target);
            }
            Effect::AuraAttackPower {
                duration_ms,
                amount,
            } => {
                let n = sized(amount);
                let a = aura(AuraKind::AttackPower(n), skill.spell_id, duration_ms, 0);
                self.skills.apply(a, events);
                let sheet = buff_sheet(&self.player.stats, &self.skills);
                events.push(values(self.guid, &sheet));
            }
            Effect::HealingReduction { duration_ms, .. } => {
                let a = aura(AuraKind::HealingReduced, skill.spell_id, duration_ms, 0);
                self.skills.apply(a, events);
            }
            Effect::Execute {
                base,
                per_extra_rage,
            } => {
                // The cost is paid; every rage point left goes into the blow.
                let extra = self.player.rage / 10;
                self.player.rage = 0;
                let raw = sized(base) + extra * sized(per_extra_rage);
                self.strike(
                    skill.spell_id,
                    target,
                    raw,
                    School::Physical,
                    how,
                    c,
                    rng,
                    events,
                    ticked,
                );
            }
            Effect::AuraMaxHealthPct {
                aura_spell_id,
                pct,
                duration_ms,
            } => {
                let before = buffed(&self.player.stats, &self.skills).max_health;
                let a = aura(AuraKind::MaxHealthPct(pct), aura_spell_id, duration_ms, 0);
                self.skills.apply(a, events);
                let after = buffed(&self.player.stats, &self.skills).max_health;
                self.player.health += after.saturating_sub(before);
                events.push(values(self.guid, &self.sheet_now()));
            }
            Effect::AuraDamagePct {
                pct,
                armor_loss_pct,
                duration_ms,
            } => {
                let kind = AuraKind::DamagePct {
                    pct,
                    armor_loss_pct,
                    swings: None,
                };
                self.skills
                    .apply(aura(kind, skill.spell_id, duration_ms, 0), events);
                events.push(values(self.guid, &self.sheet_now()));
            }
            Effect::SweepingStrikes {
                strikes,
                duration_ms,
                radius_yd,
                strike_spell_id,
            } => {
                let kind = AuraKind::Sweeping {
                    strikes,
                    radius_yd,
                    strike_spell_id,
                };
                self.skills
                    .apply(aura(kind, skill.spell_id, duration_ms, 0), events);
            }
            Effect::Seal { duration_ms, seal } => {
                // One Seal at a time: the new one replaces any other.
                let me = self.guid;
                let gone = self.skills.remove_where(
                    |a| {
                        a.unit == me
                            && a.spell_id != skill.spell_id
                            && matches!(a.kind, AuraKind::Seal { .. })
                    },
                    events,
                );
                self.faded(&gone, events);
                // Talents that raise the Seal's amounts (Improved Seal of the Crusader) raise its
                // attack power and its judgement's holy damage taken alike.
                let mut raised = seal;
                let attack_power = match &mut raised {
                    Seal::Crusader {
                        attack_power,
                        holy_taken_pct,
                        ..
                    } => {
                        *holy_taken_pct = scaled(*holy_taken_pct, pct);
                        sized(*attack_power)
                    }
                    Seal::Command { .. } => 0,
                };
                let kind = AuraKind::Seal {
                    seal: raised,
                    attack_power,
                };
                self.skills
                    .apply(aura(kind, skill.spell_id, duration_ms, 0), events);
                let sheet = buff_sheet(&self.player.stats, &self.skills);
                events.push(values(self.guid, &sheet));
            }
            Effect::Judgement => {
                let me = self.guid;
                let gone = self.skills.remove_where(
                    |a| a.unit == me && matches!(a.kind, AuraKind::Seal { .. }),
                    events,
                );
                self.faded(&gone, events);
                let judged = gone.iter().find_map(|a| match a.kind {
                    AuraKind::Seal { seal, .. } => Some(seal),
                    _ => None,
                });
                match judged {
                    Some(Seal::Command {
                        judgement_spell_id,
                        judgement,
                        ..
                    }) => {
                        let raw = sized(judgement);
                        self.holy_hit(judgement_spell_id, target, raw, c, rng, events, ticked);
                    }
                    Some(Seal::Crusader {
                        judgement_spell_id,
                        judgement_duration_ms,
                        holy_taken_pct,
                        ..
                    }) => {
                        let kind = AuraKind::HolyTaken {
                            pct: holy_taken_pct,
                            duration_ms: judgement_duration_ms,
                        };
                        let a = aura(kind, judgement_spell_id, judgement_duration_ms, 0);
                        self.skills.apply(a, events);
                    }
                    None => {}
                }
            }
        }
    }

    /// A white swing of the player's landed on `target`: its judgement lasts afresh, and the
    /// active Seal procs (Seal of Command's holy hit).
    pub(super) fn seal_swing_landed(
        &mut self,
        target: u64,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        self.skills.refresh_judgement(target);
        let Some(Seal::Command {
            procs_per_minute,
            weapon_pct,
            proc_spell_id,
            ..
        }) = self.skills.seal()
        else {
            return;
        };
        if !self.mobs.iter().any(|m| m.guid == target && m.alive()) {
            return;
        }
        let stats = white(&self.player.stats, &self.skills);
        if rng.below(ROLL) >= seals::ppm_chance(procs_per_minute, stats.attack_time_ms) {
            return;
        }
        let roll = weapon_roll(&stats, Weapon::Normal, rng);
        let raw = seals::weapon_pct_damage(roll, weapon_pct);
        self.holy_hit(proc_spell_id, target, raw, c, rng, events, ticked);
    }

    /// `raw` holy damage at `target`: no armour stops it. Logged as holy, taken off, maybe a
    /// kill.
    #[allow(clippy::too_many_arguments)]
    fn holy_hit(
        &mut self,
        spell_id: u32,
        target: u64,
        raw: u32,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let Some(mob) = self.mobs.iter().find(|m| m.guid == target) else {
            return;
        };
        let school = School::Holy;
        let damage = mitigated(school, raw, mob.stats.armor, self.player.stats.level);
        let damage = seals::holy_taken(damage, self.skills.holy_taken_pct(target));
        events.push(SessionEvent::SpellDamageLog(damage_log(
            self.guid, target, spell_id, damage, school, false,
        )));
        self.hurt(target, damage, c, rng, events, ticked);
    }

    /// `raw` physical damage at `target`, after its armour: logged, taken off, maybe a kill.
    #[allow(clippy::too_many_arguments)]
    fn strike(
        &mut self,
        spell_id: u32,
        target: u64,
        raw: u32,
        school: School,
        how: Landed,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let Some(mob) = self.mobs.iter().find(|m| m.guid == target) else {
            return;
        };
        let mut damage = mitigated(school, raw, mob.stats.armor, self.player.stats.level);
        if how == Landed::Crit {
            damage = crit_scaled(school, damage, crate::talents::skill_crit_damage_pct(c));
        }
        damage = self.dealt(c, damage);
        events.push(SessionEvent::SpellDamageLog(damage_log(
            self.guid,
            target,
            spell_id,
            damage,
            school,
            how == Landed::Crit,
        )));
        self.hurt(target, damage, c, rng, events, ticked);
        if how == Landed::Crit {
            self.crit_landed(target, c, events);
        }
        // A blow at one enemy sweeps on; an area's blows do not.
        if !school.is_magic()
            && callings::skill(spell_id).is_some_and(|s| s.target == Target::Enemy)
        {
            self.sweep(target, damage, c, rng, events, ticked);
        }
    }

    /// `damage` the player deals, raised by their talents for the weapon in hand and by their
    /// buffs.
    pub(super) fn dealt(&self, c: &Char, damage: u32) -> u32 {
        let pct = crate::talents::weapon_damage_pct(c) + self.skills.damage_pct();
        damage * (100 + pct) / 100
    }

    /// A melee blow of the player's landed `damage` on `from`: under Sweeping Strikes, the
    /// nearest other living enemy in reach takes as much.
    pub(super) fn sweep(
        &mut self,
        from: u64,
        damage: u32,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let me = self.guid;
        let Some(i) = self
            .skills
            .auras
            .iter()
            .position(|a| a.unit == me && matches!(a.kind, AuraKind::Sweeping { .. }))
        else {
            return;
        };
        let AuraKind::Sweeping {
            strikes,
            radius_yd,
            strike_spell_id,
        } = self.skills.auras[i].kind
        else {
            return;
        };
        let other = self
            .mobs
            .iter()
            .filter(|m| m.guid != from && m.alive())
            .map(|m| (m.guid, distance(m.position, c.position)))
            .filter(|(_, d)| *d <= radius_yd)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(guid, _)| guid);
        let Some(other) = other.filter(|_| damage > 0) else {
            return;
        };
        if strikes <= 1 {
            let gone = self.skills.remove_where(
                |a| a.unit == me && matches!(a.kind, AuraKind::Sweeping { .. }),
                events,
            );
            self.faded(&gone, events);
        } else if let AuraKind::Sweeping { strikes, .. } = &mut self.skills.auras[i].kind {
            *strikes -= 1;
        }
        events.push(SessionEvent::SpellDamageLog(damage_log(
            me,
            other,
            strike_spell_id,
            damage,
            School::Physical,
            false,
        )));
        self.hurt(other, damage, c, rng, events, ticked);
    }

    /// A mob's swing at the player went `outcome`: what the player's talents make of a block or
    /// a crit taken (Shield Specialization's rage, Blood Craze, Enrage).
    pub(super) fn struck(
        &mut self,
        outcome: Outcome,
        c: &Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) {
        if !self.player.alive() {
            return;
        }
        let on = crate::talents::on_struck(c);
        match outcome {
            Outcome::Block => {
                if let Some((spell_id, pct)) = on.block_rage {
                    if rng.below(100) < pct {
                        self.gain_rage(spell_id, 1, events);
                    }
                }
            }
            Outcome::Crit => {
                if let Some((spell_id, tick_ms, duration_ms, pct)) = on.crit_heal {
                    let max = buffed(&self.player.stats, &self.skills).max_health;
                    let per_tick = heal_tick(max, pct, duration_ms / tick_ms.max(1));
                    let kind = AuraKind::Heal { per_tick };
                    self.buff_self(kind, spell_id, c.level, duration_ms, tick_ms, events);
                }
                if let Some((spell_id, duration_ms, swings, pct)) = on.crit_damage {
                    let kind = AuraKind::DamagePct {
                        pct,
                        armor_loss_pct: 0,
                        swings: Some(swings),
                    };
                    self.buff_self(kind, spell_id, c.level, duration_ms, 0, events);
                }
            }
            _ => {}
        }
    }

    /// An aura of the player's own on the player.
    fn buff_self(
        &mut self,
        kind: AuraKind,
        spell_id: u32,
        level: u8,
        duration_ms: u32,
        period_ms: u32,
        events: &mut Vec<SessionEvent>,
    ) {
        let aura = Aura {
            unit: self.guid,
            spell_id,
            slot: 0,
            level,
            left: clock::secs(duration_ms),
            period: clock::secs(period_ms),
            next_tick: clock::secs(period_ms),
            kind,
        };
        self.skills.apply(aura, events);
    }

    /// `amount` rage for the player, logged under `spell_id`.
    pub(super) fn gain_rage(&mut self, spell_id: u32, amount: u32, events: &mut Vec<SessionEvent>) {
        let p = &mut self.player;
        p.rage = (p.rage + amount * 10).min(rage::MAX);
        events.push(SessionEvent::SpellEnergizeLog(SpellEnergizeLog {
            target: self.guid,
            caster: self.guid,
            spell_id,
            power: POWER_RAGE,
            amount,
        }));
        events.push(values(self.guid, &[(UNIT_FIELD_POWER2, p.rage)]));
    }

    /// `unit` is stunned for `duration_ms`, under `spell_id` (Mace Specialization).
    pub(super) fn stun(
        &mut self,
        unit: u64,
        spell_id: u32,
        duration_ms: u32,
        level: u8,
        events: &mut Vec<SessionEvent>,
    ) {
        let aura = Aura {
            unit,
            spell_id,
            slot: 0,
            level,
            left: clock::secs(duration_ms),
            period: 0.0,
            next_tick: 0.0,
            kind: AuraKind::Stun,
        };
        self.skills.apply(aura, events);
    }

    /// What the buffs move on the sheet besides attack power: maximum health and armour, and
    /// the health.
    pub(super) fn sheet_now(&self) -> Vec<(u16, u32)> {
        let s = buffed(&self.player.stats, &self.skills);
        vec![
            (UNIT_FIELD_MAXHEALTH, s.max_health),
            (UNIT_FIELD_RESISTANCES, s.armor),
            (UNIT_FIELD_HEALTH, self.player.health),
        ]
    }

    /// A melee crit landed on `target`: the bleed a talent leaves (Deep Wounds), which each new
    /// crit refreshes.
    pub(super) fn crit_landed(&mut self, target: u64, c: &Char, events: &mut Vec<SessionEvent>) {
        let Some((spell_id, tick_ms, duration_ms, pct)) = crate::talents::crit_bleed(c) else {
            return;
        };
        if !self.mobs.iter().any(|m| m.guid == target && m.alive()) {
            return;
        }
        let s = buffed(&self.player.stats, &self.skills);
        let period = clock::secs(tick_ms);
        let bleed = Aura {
            unit: target,
            spell_id,
            slot: 0,
            level: c.level,
            left: clock::secs(duration_ms),
            period,
            next_tick: period,
            kind: AuraKind::Bleed {
                per_tick: deep_wound_tick(s.min_damage, s.max_damage, pct),
            },
        };
        self.skills.apply(bleed, events);
    }

    /// The mob `target` stops fighting the player, and the player it: the mob stands dazed until
    /// its hold is over (`State::Dazed`).
    fn drop_combat(&mut self, target: u64, events: &mut Vec<SessionEvent>) {
        let Some(mob) = self.mobs.iter_mut().find(|m| m.guid == target && m.alive()) else {
            return;
        };
        let was_engaged = mob.state == State::Engaged;
        mob.state = State::Dazed;
        if !was_engaged {
            return;
        }
        mob.walk = None;
        events.push(SessionEvent::AttackStop {
            attacker: mob.guid,
            victim: self.guid,
        });
        let mut pairs = vec![(UNIT_FIELD_FLAGS, 0)];
        pairs.extend(guid_pairs(UNIT_FIELD_TARGET, 0));
        events.push(values(mob.guid, &pairs));
        if self.player.attacking == Some(target) {
            self.player.attacking = None;
            events.push(SessionEvent::AttackStop {
                attacker: self.guid,
                victim: target,
            });
        }
    }

    /// `unit` took damage: every incapacitate on it that breaks on damage ends.
    pub(super) fn woken_by_damage(&mut self, unit: u64, events: &mut Vec<SessionEvent>) {
        let gone = self.skills.remove_where(
            |a| {
                a.unit == unit
                    && matches!(
                        a.kind,
                        AuraKind::Incapacitate {
                            break_on_damage: true
                        }
                    )
            },
            events,
        );
        self.faded(&gone, events);
    }

    /// `target` loses `damage`; at 0 it dies at the player's hand.
    fn hurt(
        &mut self,
        target: u64,
        damage: u32,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let Some(i) = self.mobs.iter().position(|m| m.guid == target && m.alive()) else {
            return;
        };
        let mob = &mut self.mobs[i];
        mob.health = mob.health.saturating_sub(damage);
        events.push(values(mob.guid, &[(UNIT_FIELD_HEALTH, mob.health)]));
        if matches!(mob.state, State::Idle | State::Dazed) {
            super::engage(mob, self.guid, events);
        }
        if mob.health == 0 {
            self.kill(i, c, rng, events, ticked);
        } else {
            self.woken_by_damage(target, events);
        }
    }

    /// The queued on-next-swing strike replaces this white swing at `target`, when its rage is
    /// still there. Whether it did.
    pub(super) fn next_swing_strike(
        &mut self,
        target: u64,
        from_front: bool,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) -> bool {
        let Some(spell_id) = self.skills.take_queued() else {
            return false;
        };
        let Some(skill) = callings::skill(spell_id) else {
            return false;
        };
        if self.player.rage < crate::talents::rage_cost(c, skill) * 10 {
            events.push(refused(spell_id, SPELL_FAILED_NO_POWER));
            return false;
        }
        let Some(i) = self.mobs.iter().position(|m| m.guid == target && m.alive()) else {
            return false;
        };
        let bonus = skill.effects.iter().find_map(|e| match e {
            Effect::WeaponStrike { bonus, .. } => Some(*bonus),
            _ => None,
        });
        let Some(bonus) = bonus else {
            return false;
        };
        self.pay(skill, c, events);
        // A next-swing strike rolls exactly as the white swing it replaces, its bonus added.
        let mut stats = buffed(&self.player.stats, &self.skills);
        let pct = crate::talents::amount_pct(c, &skill.key);
        let extra = scaled(bonus.at(c.level, stats.attack_power), pct) as f32;
        stats.min_damage += extra;
        stats.max_damage += extra;
        // Cleave's other targets: the nearest living mobs beside the first, each on its own roll.
        let at = self.mobs[i].position;
        let mut beside: Vec<usize> = (0..self.mobs.len())
            .filter(|&j| {
                j != i
                    && self.mobs[j].alive()
                    && distance(self.mobs[j].position, at) <= skill.extra_target_radius_yd
            })
            .collect();
        beside.sort_by(|&a, &b| {
            distance(self.mobs[a].position, at).total_cmp(&distance(self.mobs[b].position, at))
        });
        beside.truncate(skill.extra_targets as usize);
        let mut swings = vec![(
            target,
            crate::combat::player_swing(&stats, &self.mobs[i], from_front, rng),
        )];
        for j in beside {
            let m = &self.mobs[j];
            let front = in_front(m.orientation, m.position, c.position);
            swings.push((m.guid, crate::combat::player_swing(&stats, m, front, rng)));
        }
        let missed_by = |outcome: Outcome| match outcome {
            Outcome::Miss => Some(SPELL_MISS_MISS),
            Outcome::Dodge => Some(SPELL_MISS_DODGE),
            Outcome::Parry => Some(SPELL_MISS_PARRY),
            _ => None,
        };
        events.push(SessionEvent::SpellStart {
            caster: self.guid,
            spell_id,
            cast_flags: 0,
            cast_time_ms: 0,
            target: Some(target),
            ammo_display_id: None,
        });
        events.push(SessionEvent::CastResult {
            spell_id,
            success: true,
            reason: None,
            arg: None,
        });
        events.push(SessionEvent::SpellGo {
            caster: self.guid,
            spell_id,
            cast_flags: 0,
            hits: swings
                .iter()
                .filter(|(_, s)| missed_by(s.outcome).is_none())
                .map(|(guid, _)| *guid)
                .collect(),
            misses: swings
                .iter()
                .filter_map(|(guid, s)| missed_by(s.outcome).map(|m| (*guid, m)))
                .collect(),
            target: Some(target),
            go_target: None,
            dest: None,
            ammo_display_id: None,
            item_caster: None,
        });
        events.push(values(self.guid, &[(UNIT_FIELD_POWER2, self.player.rage)]));
        // The first target's dodge opens Overpower, as a white swing's would.
        if missed_by(swings[0].1.outcome) == Some(SPELL_MISS_DODGE) {
            self.skills.dodged_by(target, events);
        }
        let impale = crate::talents::skill_crit_damage_pct(c);
        for (guid, mut swing) in swings {
            if missed_by(swing.outcome).is_some() {
                continue;
            }
            if swing.outcome == Outcome::Crit {
                swing.damage = crit_damage(swing.would_be, impale);
            }
            swing.damage = self.dealt(c, swing.damage);
            events.push(SessionEvent::SpellDamageLog(SpellDamageLog {
                blocked: swing.blocked,
                ..damage_log(
                    self.guid,
                    guid,
                    spell_id,
                    swing.damage,
                    School::Physical,
                    swing.outcome == Outcome::Crit,
                )
            }));
            self.hurt(guid, swing.damage, c, rng, events, ticked);
            if swing.outcome == Outcome::Crit {
                self.crit_landed(guid, c, events);
            }
            if guid == target {
                self.sweep(guid, swing.damage, c, rng, events, ticked);
            }
        }
        true
    }

    /// The skills' clocks: cooldowns, the dodge window, and every aura's ticks and fading.
    pub(super) fn skills_tick(
        &mut self,
        dt: f32,
        wall: f64,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let s = &mut self.skills;
        s.gcd = (s.gcd - dt).max(0.0);
        s.combat_hold = (s.combat_hold - dt).max(0.0);
        for (_, left) in &mut s.cooldowns {
            *left -= dt;
        }
        s.cooldowns.retain(|(_, left)| *left > 0.0);
        c.item_cooldowns.retain(|&(_, ready)| ready > wall);
        self.timed_cast_tick(dt, wall, c, events);
        let s = &mut self.skills;
        if let Some((_, left)) = &mut s.dodged {
            *left -= dt;
            if *left <= 0.0 {
                s.close_dodge_window(events);
            }
        }

        // Ticks first, then fading: the last tick lands as the aura runs out.
        let mut due = Vec::new();
        for a in &mut s.auras {
            a.left -= dt;
            if a.period > 0.0 {
                a.next_tick -= dt;
                while a.next_tick <= CLOCK_EPSILON {
                    a.next_tick += a.period;
                    due.push(*a);
                }
            }
        }
        for a in due {
            match a.kind {
                AuraKind::Bleed { per_tick } => {
                    if !self.mobs.iter().any(|m| m.guid == a.unit && m.alive()) {
                        continue;
                    }
                    events.push(SessionEvent::PeriodicAuraLog(PeriodicAuraLog {
                        target: a.unit,
                        caster: self.guid,
                        spell_id: a.spell_id,
                        ticks: vec![PeriodicTick::Damage {
                            amount: per_tick,
                            school: 0,
                            absorb: 0,
                            resist: 0,
                        }],
                    }));
                    self.hurt(a.unit, per_tick, c, rng, events, ticked);
                }
                AuraKind::Heal { per_tick } => {
                    let max = buffed(&self.player.stats, &self.skills).max_health;
                    let p = &mut self.player;
                    if !p.alive() {
                        continue;
                    }
                    p.health = (p.health + per_tick).min(max);
                    events.push(SessionEvent::PeriodicAuraLog(PeriodicAuraLog {
                        target: self.guid,
                        caster: self.guid,
                        spell_id: a.spell_id,
                        ticks: vec![PeriodicTick::Heal { amount: per_tick }],
                    }));
                    events.push(values(self.guid, &[(UNIT_FIELD_HEALTH, p.health)]));
                }
                AuraKind::Rage { per_tick } => {
                    let p = &mut self.player;
                    p.rage = (p.rage + per_tick * 10).min(rage::MAX);
                    events.push(SessionEvent::PeriodicAuraLog(PeriodicAuraLog {
                        target: self.guid,
                        caster: self.guid,
                        spell_id: a.spell_id,
                        ticks: vec![PeriodicTick::Energize {
                            power: POWER_RAGE,
                            amount: per_tick,
                        }],
                    }));
                    events.push(values(self.guid, &[(UNIT_FIELD_POWER2, p.rage)]));
                }
                _ => {}
            }
        }
        let gone = self
            .skills
            .remove_where(|a| a.left <= CLOCK_EPSILON, events);
        self.faded(&gone, events);
    }

    /// What an aura's going undoes.
    pub(super) fn faded(&mut self, gone: &[Aura], events: &mut Vec<SessionEvent>) {
        for a in gone {
            match a.kind {
                AuraKind::MoveSlow { .. } => self.update_speed(a.unit),
                AuraKind::AttackPower(_) | AuraKind::Seal { .. } => {
                    let sheet = buff_sheet(&self.player.stats, &self.skills);
                    events.push(values(self.guid, &sheet));
                }
                AuraKind::MaxHealthPct(_) | AuraKind::DamagePct { .. } => {
                    // What was over the maximum goes with it.
                    let max = buffed(&self.player.stats, &self.skills).max_health;
                    self.player.health = self.player.health.min(max);
                    events.push(values(self.guid, &self.sheet_now()));
                }
                _ => {}
            }
        }
    }

    /// `CMSG_CANCEL_AURA`: the player clicks off one of their own buffs; debuffs stay.
    pub(crate) fn cancel_aura(&mut self, spell_id: u32, events: &mut Vec<SessionEvent>) {
        let me = self.guid;
        let gone = self.skills.remove_where(
            |a| a.unit == me && a.spell_id == spell_id && a.kind.helpful(),
            events,
        );
        self.faded(&gone, events);
    }

    /// `unit` died: its auras go with it.
    pub(super) fn auras_die_with(&mut self, unit: u64, events: &mut Vec<SessionEvent>) {
        let gone = self.skills.remove_where(|a| a.unit == unit, events);
        self.faded(&gone, events);
    }

    /// Sets and clears each mob's health-under-20% aura state as its health crosses a fifth: the
    /// client greys Execute unless its target has it.
    pub(super) fn update_aura_states(&mut self, events: &mut Vec<SessionEvent>) {
        let low: Vec<u64> = self
            .mobs
            .iter()
            .filter(|m| m.alive() && u64::from(m.health) * 5 <= u64::from(m.stats.max_health))
            .map(|m| m.guid)
            .collect();
        for m in &self.mobs {
            let (was, is) = (
                self.skills.low_health.contains(&m.guid),
                low.contains(&m.guid),
            );
            if was != is {
                let state = if is { AURA_STATE_HEALTHLESS_20 } else { 0 };
                events.push(values(m.guid, &[(UNIT_FIELD_AURASTATE, state)]));
            }
        }
        self.skills.low_health = low;
    }

    fn update_speed(&mut self, unit: u64) {
        let mul = self.skills.speed_mul(unit);
        if let Some(mob) = self.mobs.iter_mut().find(|m| m.guid == unit) {
            mob.speed_mul = mul;
        }
    }
}

/// A weapon roll before armour: a white swing's, or a normalized one.
fn weapon_roll(stats: &WarriorStats, weapon: Weapon, rng: &mut Rng) -> f32 {
    let roll = rng.frand(stats.min_damage, stats.max_damage);
    match weapon {
        Weapon::Normal => roll,
        Weapon::Normalized => {
            formulas::normalized_roll(roll, stats.attack_power, clock::secs(stats.attack_time_ms))
        }
    }
}

// ── Item casts ────────────────────────────────────────────────────────────────────────────────────
//
// An item's on-use spell with a cast bar: `SMSG_SPELL_START` with its cast time, then, unless
// moving or a hit interrupts it, `SMSG_CAST_RESULT` (ok), `SMSG_SPELL_GO` naming the item, its
// effect, and the item's cooldown, which the character keeps (it is saved).

/// `SPELL_FAILED_INTERRUPTED`.
const SPELL_FAILED_INTERRUPTED: u8 = 0x23;
/// `SPELL_FAILED_SPELL_IN_PROGRESS`.
const SPELL_FAILED_SPELL_IN_PROGRESS: u8 = 0x61;
/// `CAST_FLAG_UNKNOWN2`, which vmangos sets on every player cast's start and go.
const CAST_FLAG_UNKNOWN2: u16 = 0x02;

impl InWorld {
    /// `CMSG_USE_ITEM` on the item at player slot `slot`: its on-use spell starts casting, or is
    /// refused with the stock reason. An item with no on-use spell the realm runs does nothing.
    pub(crate) fn use_item(
        &mut self,
        slot: crate::inventory::Pos,
        c: &Char,
        wall: f64,
        events: &mut Vec<SessionEvent>,
    ) {
        let Some(item) = c.inventory.get(slot) else {
            return;
        };
        let Some(t) = items::template(item.entry) else {
            return;
        };
        let Some(cast_ms) = super::hearth::cast_time_ms(t.use_spell) else {
            return;
        };
        let spell_id = t.use_spell;
        let reason = if !self.player.alive() {
            Some(SPELL_FAILED_CASTER_DEAD)
        } else if self.skills.casting.is_some() {
            Some(SPELL_FAILED_SPELL_IN_PROGRESS)
        } else if c
            .item_cooldowns
            .iter()
            .any(|&(e, ready)| e == item.entry && ready - wall > f64::from(READY_SLACK_SECS))
        {
            Some(SPELL_FAILED_NOT_READY)
        } else {
            None
        };
        if let Some(reason) = reason {
            events.push(refused(spell_id, reason));
            return;
        }
        self.skills.casting = Some(TimedCast {
            spell_id,
            item: item.guid,
            entry: item.entry,
            left: clock::secs(cast_ms),
        });
        events.push(SessionEvent::SpellStart {
            caster: self.guid,
            spell_id,
            cast_flags: CAST_FLAG_UNKNOWN2,
            cast_time_ms: cast_ms,
            target: None,
            ammo_display_id: None,
        });
    }

    /// The timed cast under way stops short (`Spell::cancel`): the bar shows "Interrupted", and
    /// nothing goes on cooldown.
    pub(crate) fn interrupt_cast(&mut self, events: &mut Vec<SessionEvent>) {
        let Some(cast) = self.skills.casting.take() else {
            return;
        };
        events.push(SessionEvent::SpellFailedOther {
            caster: self.guid,
            spell_id: cast.spell_id,
        });
        events.push(refused(cast.spell_id, SPELL_FAILED_INTERRUPTED));
    }

    /// The timed cast's clock: when it runs out the spell goes off.
    fn timed_cast_tick(
        &mut self,
        dt: f32,
        wall: f64,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) {
        let Some(cast) = &mut self.skills.casting else {
            return;
        };
        cast.left -= dt;
        if cast.left > CLOCK_EPSILON {
            return;
        }
        let cast = *cast;
        self.skills.casting = None;
        events.push(SessionEvent::CastResult {
            spell_id: cast.spell_id,
            success: true,
            reason: None,
            arg: None,
        });
        events.push(SessionEvent::SpellGo {
            caster: self.guid,
            spell_id: cast.spell_id,
            cast_flags: CAST_FLAG_UNKNOWN2,
            hits: vec![self.guid],
            misses: Vec::new(),
            target: None,
            go_target: None,
            dest: None,
            ammo_display_id: None,
            item_caster: Some(cast.item),
        });
        if let Some(t) = items::template(cast.entry) {
            if t.use_cooldown_ms > 0 {
                c.item_cooldowns.retain(|&(e, _)| e != cast.entry);
                let rest = f64::from(clock::secs(t.use_cooldown_ms as u32));
                c.item_cooldowns.push((cast.entry, wall + rest));
            }
        }
        self.item_spell_effect(cast.spell_id, c, events);
    }
}
