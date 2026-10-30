//! A character's Talent Rows (ADR 0028): which choice each row of the current Calling has set,
//! setting one, and what the set choices add to a fight. The rows are data
//! ([`crate::callings::TalentRow`]); nothing here names a talent or a skill.
//!
//! A set Skill Choice replaces its skill in the spell book and on the bar; a set Talent adds its
//! effects, or its skill when it is an active one. Every choice is one rank.

use benilla_protocol::{SessionEvent, TalentChoiceView, TalentRowView};

use crate::callings::{
    self, CritBleedOf, RowChoice, RowKind, Skill, Stat, TalentDef, TalentEffect, TalentRow, Wielded,
};
use crate::characters::Char;
use crate::rules::power::PowerKind;
use crate::rules::stats::CharacterStats;

/// The current Calling's rows, in level order.
pub(crate) fn rows(c: &Char) -> &'static [TalentRow] {
    &callings::of(c.calling).rows
}

/// The choice `c` has set on `row`, if any.
pub(crate) fn chosen(c: &Char, row: &TalentRow) -> Option<&'static RowChoice> {
    let &(_, spell_id) = c.rows.iter().find(|(key, _)| *key == row.key)?;
    rows(c)
        .iter()
        .find(|r| r.key == row.key)?
        .choices
        .iter()
        .find(|choice| choice.spell_id() == spell_id)
}

/// Every set choice of `c`'s rows.
fn set_choices(c: &Char) -> impl Iterator<Item = &'static RowChoice> + '_ {
    rows(c).iter().filter_map(|row| chosen(c, row))
}

/// The set Talents' definitions.
fn set_talents(c: &Char) -> impl Iterator<Item = &'static TalentDef> + '_ {
    set_choices(c).filter_map(|choice| match choice {
        RowChoice::Talent(t) => Some(t),
        RowChoice::Skill(_) => None,
    })
}

/// Drops from a loaded save a row the Calling lacks, a spell that is not one of that row's
/// choices, and a row the level has not opened.
pub(crate) fn keep_valid(c: &mut Char) {
    let level = c.level;
    let own = rows(c);
    c.rows.retain(|(key, spell_id)| {
        own.iter().any(|row| {
            row.key == *key
                && level >= row.level
                && row.choices.iter().any(|ch| ch.spell_id() == *spell_id)
        })
    });
}

/// Every set choice's spell, for the spell book.
pub(crate) fn known_spells(c: &Char) -> impl Iterator<Item = u32> + '_ {
    set_choices(c).map(RowChoice::spell_id)
}

/// The skill keys a set Skill Choice has taken out of the spell book.
pub(crate) fn replaced(c: &Char) -> impl Iterator<Item = &'static str> + '_ {
    set_choices(c).filter_map(|choice| match choice {
        RowChoice::Skill(s) => s.replaces.as_deref(),
        RowChoice::Talent(_) => None,
    })
}

/// Whether the skill `key` of `c`'s Calling answers to `wanted`: the same key, or a Skill Choice
/// of the skill `wanted`.
fn answers(c: &Char, key: &str, wanted: &str) -> bool {
    key == wanted
        || rows(c)
            .iter()
            .flat_map(|row| &row.choices)
            .filter_map(RowChoice::skill)
            .any(|s| s.key == key && s.is(wanted))
}

/// Every set Talent's effects, each with the index of its one rank.
fn learned(c: &Char) -> impl Iterator<Item = (&'static TalentEffect, usize)> + '_ {
    set_talents(c).flat_map(|t| t.effects.iter().map(|e| (e, 0)))
}

/// The size of each learned talent's effects that `pick` takes, summed.
fn sum(c: &Char, pick: impl Fn(&TalentEffect) -> bool) -> u32 {
    learned(c)
        .filter(|(e, _)| pick(e))
        .map(|(e, r)| e.per_rank()[r])
        .sum()
}

/// How many more stacks the learned talents give the aura `key`.
pub(crate) fn aura_max_stacks(c: &Char, key: &str) -> u32 {
    learned(c)
        .map(|(e, r)| match e {
            TalentEffect::AuraMaxStacks { aura, per_rank } if aura == key => per_rank[r],
            _ => 0,
        })
        .sum()
}

/// The percent of maximum health a droplet's pickup heals.
pub(crate) fn pickup_heal_pct(c: &Char) -> u32 {
    sum(c, |e| matches!(e, TalentEffect::PickupHealPct { .. }))
}

/// The points of the character's own power a droplet's pickup gives.
pub(crate) fn pickup_energize(c: &Char) -> u32 {
    sum(c, |e| matches!(e, TalentEffect::PickupEnergize { .. }))
}

/// The yards farther away a droplet is picked up.
pub(crate) fn pickup_radius_yd(c: &Char) -> f32 {
    learned(c)
        .map(|(e, r)| match e {
            TalentEffect::PickupRadiusYd { per_rank } => per_rank[r],
            _ => 0.0,
        })
        .sum()
}

/// A skill `c` can cast: one of the current Calling's that its level has reached and no set Skill
/// Choice replaces, or a set choice's (a Skill Choice, or an active Talent's).
pub(crate) fn skill(c: &Char, spell_id: u32) -> Option<&'static Skill> {
    let own = callings::of(c.calling)
        .skills
        .iter()
        .filter(|s| u32::from(s.learn_level) <= u32::from(c.level))
        .filter(|s| !replaced(c).any(|k| k == s.key));
    let chosen = set_choices(c).filter_map(RowChoice::skill);
    own.chain(chosen).find(|s| s.spell_id == spell_id)
}

/// Why a row could not be set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    NoSuchRow,
    /// Below its level.
    Locked,
    NotAChoice,
    /// Already set, and no innkeeper within service range.
    NeedsInnkeeper,
    /// That choice is the one set.
    Same,
}

/// `SetTalentRow`: sets `row_key` to the choice `spell_id`. The old spell (the plain skill for an
/// empty Skill Choice row, else the previous choice) leaves the spell book and the bar for the
/// new one, and the rows are sent again.
pub(crate) fn set(
    c: &mut Char,
    row_key: &str,
    spell_id: u32,
    at_innkeeper: bool,
    events: &mut Vec<SessionEvent>,
) -> Result<(), Refusal> {
    let row = rows(c)
        .iter()
        .find(|r| r.key == row_key)
        .ok_or(Refusal::NoSuchRow)?;
    if c.level < row.level {
        return Err(Refusal::Locked);
    }
    if !row.choices.iter().any(|ch| ch.spell_id() == spell_id) {
        return Err(Refusal::NotAChoice);
    }
    let old = chosen(c, row);
    if old.is_some_and(|o| o.spell_id() == spell_id) {
        return Err(Refusal::Same);
    }
    if old.is_some() && !at_innkeeper {
        return Err(Refusal::NeedsInnkeeper);
    }
    let old_spell = match (old, row.kind) {
        (Some(o), _) => Some(o.spell_id()),
        (None, RowKind::SkillChoice) => callings::of(c.calling)
            .skills
            .iter()
            .find(|s| row.skill.as_deref() == Some(s.key.as_str()))
            .map(|s| s.spell_id),
        (None, RowKind::Talent) => None,
    };
    c.rows.retain(|(key, _)| key != row_key);
    c.rows.push((row_key.to_owned(), spell_id));
    if let Some(old_spell) = old_spell {
        events.push(SessionEvent::SpellRemoved {
            spell_id: old_spell,
        });
        let bar = c
            .action_bar
            .get_or_insert_with(|| callings::default_bar(c.calling));
        for (_, packed) in bar.iter_mut() {
            // Kind 0, a spell, in the top byte.
            if *packed == old_spell {
                *packed = spell_id;
            }
        }
    }
    events.push(SessionEvent::SpellLearned { spell_id });
    events.push(crate::world::action_buttons(c));
    if row.kind == RowKind::Talent {
        events.extend(spell_mods(c));
    }
    events.push(rows_event(c, at_innkeeper));
    Ok(())
}

impl crate::Realm {
    /// The Calling of the character in the world; `None` at character select.
    pub fn current_calling(&self) -> Option<&'static callings::CallingDef> {
        let w = self.world.as_ref()?;
        self.characters.get(w.guid).map(|c| callings::of(c.calling))
    }

    /// `SetTalentRow` from the character in the world: a refusal answers nothing; a set redoes the
    /// sheet and saves.
    pub(crate) fn set_talent_row(&mut self, row: &str, spell_id: u32) {
        let at_innkeeper = self.at_innkeeper();
        let Some(w) = &mut self.world else {
            return;
        };
        let Some(c) = self.characters.get_mut(w.guid) else {
            return;
        };
        if set(c, row, spell_id, at_innkeeper, &mut self.events).is_ok() {
            w.refresh_gear(c, &mut self.events);
            let guid = w.guid;
            self.save(guid);
        }
    }
}

/// The Talent Rows event: every row of the current Calling, open or not, and its set choice.
pub(crate) fn rows_event(c: &Char, at_innkeeper: bool) -> SessionEvent {
    let def = callings::of(c.calling);
    SessionEvent::TalentRows {
        calling: def.name.clone(),
        at_innkeeper,
        rows: def
            .rows
            .iter()
            .map(|row| TalentRowView {
                key: row.key.clone(),
                level: row.level,
                open: c.level >= row.level,
                skill: def
                    .skills
                    .iter()
                    .find(|s| row.skill.as_deref() == Some(s.key.as_str()))
                    .map(|s| s.name.clone())
                    .unwrap_or_default(),
                set: chosen(c, row).map_or(0, RowChoice::spell_id),
                choices: row
                    .choices
                    .iter()
                    .map(|ch| TalentChoiceView {
                        spell_id: ch.spell_id(),
                        name: ch.name().to_owned(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// Whether `c` wields `weapon` in the main hand.
pub(crate) fn wields(c: &Char, weapon: Wielded) -> bool {
    use crate::rules::gear::{CLASS_WEAPON, INVTYPE_2HWEAPON, SLOT_MAIN_HAND};
    let Some(t) = c
        .inventory
        .get(SLOT_MAIN_HAND)
        .and_then(|item| crate::items::template(item.entry))
        .filter(|t| t.class == CLASS_WEAPON)
    else {
        return false;
    };
    // `ItemSubclassWeapon`: 0/1 axes, 4/5 maces, 7/8 swords, one- and two-handed.
    match weapon {
        Wielded::OneHand => t.inventory_type != INVTYPE_2HWEAPON,
        Wielded::TwoHand => t.inventory_type == INVTYPE_2HWEAPON,
        Wielded::Axe => matches!(t.subclass, 0 | 1),
        Wielded::Mace => matches!(t.subclass, 4 | 5),
        Wielded::Sword => matches!(t.subclass, 7 | 8),
    }
}

/// The percent the physical damage `c` deals rises by, for the weapon in hand.
pub(crate) fn weapon_damage_pct(c: &Char) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::WeaponDamagePct { weapon, .. } if wields(c, *weapon)),
    )
}

/// The percent of the whole crit a skill's crit adds (Impale); a `builders` talent (Lethality)
/// counts only when the skill is a combo point builder.
pub(crate) fn skill_crit_damage_pct(c: &Char, skill: Option<&str>, builder: bool) -> u32 {
    sum(c, |e| {
        matches!(e, TalentEffect::SkillCritDamagePct { builders, skill: s, .. }
            if (builder || !builders)
                && s.as_deref().is_none_or(|s| skill.is_some_and(|k| answers(c, k, s))))
    })
}

/// The percent `school` damage rises by on a target wearing one of a talent's aura rows, given
/// `wears` (an aura row key).
pub(crate) fn damage_pct_vs(
    c: &Char,
    school: crate::rules::school::School,
    wears: impl Fn(&str) -> bool,
) -> u32 {
    sum(c, |e| match e {
        TalentEffect::DamagePctVsAura {
            aura, school: s, ..
        } => *s == school && aura.iter().any(|k| wears(k)),
        _ => false,
    })
}

/// The armour a skill's strikes skip on top of their own (Piercing Force).
pub(crate) fn skill_armor_ignore(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::SkillArmorIgnore { skill: s, .. } if answers(c, skill, s)),
    )
}

/// Whether a talent takes away `skill`'s minimum range; `ranged`: whether it is a shot.
pub(crate) fn no_min_range(c: &Char, skill: &str, ranged: bool) -> bool {
    sum(c, |e| {
        matches!(e, TalentEffect::NoMinRange { skill: s, .. }
            if answers(c, skill, s) || (ranged && s == "ranged"))
    }) > 0
}

/// The percent a skill's amounts rise by on a target `distance` yards off, from the talents
/// that only count close in (Point Blank).
pub(crate) fn amount_pct_near(c: &Char, skill: &str, distance: f32) -> u32 {
    sum(c, |e| {
        matches!(e, TalentEffect::SkillAmountPct { skill: s, max_distance_yd, .. }
            if answers(c, skill, s) && *max_distance_yd > 0.0 && distance <= *max_distance_yd)
    })
}

/// The percent chance a skill also lands on one more enemy in front of the caster (Flurry of
/// Blades).
pub(crate) fn extra_target_pct(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::SkillExtraTarget { skill: s, .. } if answers(c, skill, s)),
    )
}

/// The percent chance a skill's combo point gains one more, on a target that is bleeding or
/// not (Serrated Strikes).
pub(crate) fn bonus_combo_pct(c: &Char, skill: &str, bleeding: bool) -> u32 {
    sum(c, |e| {
        matches!(e, TalentEffect::SkillBonusCombo { skill: s, target_state, .. }
            if answers(c, skill, s) && (bleeding || target_state.is_none()))
    })
}

/// The milliseconds a skill's auras last longer (Bloodletting).
pub(crate) fn duration_ms(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::SkillDurationMs { skill: s, .. } if answers(c, skill, s)),
    )
}

/// The percent a skill's auras last longer.
pub(crate) fn duration_pct(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::SkillDurationPct { skill: s, .. } if answers(c, skill, s)),
    )
}

/// What a landed white swing may do besides its damage.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct OnHit {
    /// `(energize spell, percent)`: 1 rage.
    pub(crate) rage: Option<(u32, u32)>,
    /// The percent chance of one more swing.
    pub(crate) extra_swing_pct: u32,
    /// `(aura spell, duration ms, percent)`: a stun on the target.
    pub(crate) stun: Option<(u32, u32, u32)>,
}

/// What `c`'s landed white swings may do, for the weapon in hand.
pub(crate) fn on_hit(c: &Char) -> OnHit {
    let mut hit = OnHit::default();
    for (e, r) in learned(c) {
        match e {
            TalentEffect::HitRagePct { spell_id, per_rank } => {
                hit.rage = Some((*spell_id, per_rank[r]));
            }
            TalentEffect::ExtraSwingPct { weapon, per_rank } if wields(c, *weapon) => {
                hit.extra_swing_pct += per_rank[r];
            }
            TalentEffect::HitStunPct {
                weapon,
                aura_spell_id,
                duration_ms,
                per_rank,
            } if wields(c, *weapon) => {
                hit.stun = Some((*aura_spell_id, *duration_ms, per_rank[r]));
            }
            _ => {}
        }
    }
    hit
}

/// What being struck may do for `c`.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct OnStruck {
    /// `(energize spell, percent)`: 1 rage for a block.
    pub(crate) block_rage: Option<(u32, u32)>,
    /// `(aura spell, tick ms, duration ms, percent of max health)`: a heal after a crit taken.
    pub(crate) crit_heal: Option<(u32, u32, u32, u32)>,
    /// `(aura spell, duration ms, swings, percent)`: more damage after a crit taken.
    pub(crate) crit_damage: Option<(u32, u32, u32, u32)>,
}

/// What `c` gets from being struck.
pub(crate) fn on_struck(c: &Char) -> OnStruck {
    let mut struck = OnStruck::default();
    for (e, r) in learned(c) {
        match e {
            TalentEffect::BlockRagePct { spell_id, per_rank } => {
                struck.block_rage = Some((*spell_id, per_rank[r]));
            }
            TalentEffect::CritTakenHealPct {
                aura_spell_ids,
                tick_ms,
                duration_ms,
                per_rank,
                ..
            } => {
                struck.crit_heal = Some((aura_spell_ids[r], *tick_ms, *duration_ms, per_rank[r]));
            }
            TalentEffect::CritTakenDamagePct {
                aura_spell_ids,
                duration_ms,
                swings,
                per_rank,
                ..
            } => {
                struck.crit_damage = Some((aura_spell_ids[r], *duration_ms, *swings, per_rank[r]));
            }
            _ => {}
        }
    }
    struck
}

/// What a skill whose row prices it at `base` costs `c`, in whole points of its power, its
/// talents applied.
pub(crate) fn power_cost_from(c: &Char, skill: &callings::Skill, base: u32) -> u32 {
    let less = sum(
        c,
        |e| matches!(e, TalentEffect::SkillCost { skill: s, flat: true, .. } if skill.is(s)),
    );
    let more = sum(
        c,
        |e| matches!(e, TalentEffect::SkillCost { skill: s, flat: false, .. } if skill.is(s)),
    );
    crate::rules::talents::scaled(base, more).saturating_sub(less)
}

/// The percent of the shield's Block Value a skill's strike adds.
pub(crate) fn block_value_pct(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::SkillBlockValuePct { skill: s, .. } if answers(c, skill, s)),
    )
}

/// The percent a skill's damage and buff amounts rise by, whatever the target.
pub(crate) fn amount_pct(c: &Char, skill: &str) -> u32 {
    sum(c, |e| {
        matches!(e, TalentEffect::SkillAmountPct {
            skill: s,
            applies_to: callings::AppliesTo::Both,
            requires_target_aura: None,
            max_distance_yd,
            ..
        } if answers(c, skill, s) && *max_distance_yd == 0.0)
    })
}

/// The percent a skill's `part` (its hit or its ticks) rises by on a target that wears what
/// `wears` says it does (an aura row key).
pub(crate) fn amount_pct_on(
    c: &Char,
    skill: &str,
    part: callings::AppliesTo,
    wears: impl Fn(&str) -> bool,
) -> u32 {
    sum(c, |e| match e {
        TalentEffect::SkillAmountPct {
            skill: s,
            applies_to,
            requires_target_aura,
            max_distance_yd,
            ..
        } => {
            *max_distance_yd == 0.0
                && answers(c, skill, s)
                && (*applies_to == callings::AppliesTo::Both || *applies_to == part)
                && requires_target_aura.as_deref().is_none_or(&wears)
        }
        _ => false,
    })
}

/// The extra rage a skill gives.
pub(crate) fn extra_rage(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::SkillRage { skill: s, .. } if answers(c, skill, s)),
    )
}

/// The extra critical strike chance of a skill, in percent.
pub(crate) fn skill_crit_pct(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::SkillCritPct { skill: s, .. } if s == "any" || answers(c, skill, s)),
    )
}

/// The percent of the bonus damage its consumed stacks dealt that a skill heals the caster for.
pub(crate) fn consume_heal_pct(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::ConsumeHealPct { skill: s, .. } if answers(c, skill, s)),
    )
}

/// The percent a skill's consume outcomes are raised by (Thermal Shock).
pub(crate) fn consume_amount_pct(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::ConsumeAmountPct { skill: s, .. } if answers(c, skill, s)),
    )
}

/// The percent more damage a tick of the patches a skill leaves deals.
pub(crate) fn ground_amount_pct(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::GroundAmountPct { skill: s, .. } if answers(c, skill, s)),
    )
}

/// The milliseconds the patches a skill leaves last longer.
pub(crate) fn ground_duration_ms(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::GroundDurationMs { skill: s, .. } if answers(c, skill, s)),
    )
}

/// The bleeds a melee crit leaves: `(aura spell, tick ms, duration ms, percent, what of)`.
pub(crate) fn crit_bleeds(c: &Char) -> Vec<(u32, u32, u32, u32, CritBleedOf)> {
    learned(c)
        .filter_map(|(e, r)| match e {
            TalentEffect::CritBleed {
                aura_spell_id,
                tick_ms,
                duration_ms,
                of,
                per_rank,
            } => Some((*aura_spell_id, *tick_ms, *duration_ms, per_rank[r], *of)),
            _ => None,
        })
        .collect()
}

/// The character's stats: gear's and the Well Fed buff's, and the talents' on top.
pub(crate) fn stats(c: &Char) -> CharacterStats {
    let mut s = crate::equipment::stats(c);
    if let Some(fed) = crate::cooking::fed(c) {
        s.add_stats(fed.stats);
    }
    s.crit_pct += f64::from(sum(c, |e| matches!(e, TalentEffect::MeleeCritPct { .. })));
    s.crit_pct += f64::from(sum(c, |e| matches!(e, TalentEffect::ModCritChance { .. })));
    s.crit_damage_pct = sum(c, |e| matches!(e, TalentEffect::ModCritDamage { .. }));
    // Talents' dodge adds to the agility-based base; the roll table never passes 100%.
    s.dodge_pct = (s.dodge_pct + f64::from(sum(c, |e| matches!(e, TalentEffect::DodgePct { .. }))))
        .min(100.0);
    s.armor_pen = sum(c, |e| matches!(e, TalentEffect::ArmorPen { .. }));
    s.run_speed_pct = sum(c, |e| matches!(e, TalentEffect::ModRunSpeed { .. }));
    s.offhand_damage_pct = sum(c, |e| matches!(e, TalentEffect::OffhandDamagePct { .. }));
    // Max power is of the Calling's own bar; a rage bar has a fixed cap.
    let max_power = sum(c, |e| matches!(e, TalentEffect::MaxPower { .. }));
    match crate::callings::of(c.calling).power {
        PowerKind::Mana => s.max_mana += max_power,
        PowerKind::Energy => s.max_energy_bonus = max_power,
        PowerKind::Rage | PowerKind::Focus => {}
    }
    s.crit_pct += f64::from(sum(
        c,
        |e| matches!(e, TalentEffect::WeaponCritPct { weapon, .. } if wields(c, *weapon)),
    ));
    s.defense += sum(c, |e| matches!(e, TalentEffect::Defense { .. })) as u16;
    // Parrying takes a weapon in hand (`Unit::GetUnitParryChance`).
    if wields(c, Wielded::OneHand) || wields(c, Wielded::TwoHand) {
        s.parry_pct += f64::from(sum(c, |e| matches!(e, TalentEffect::ParryPct { .. })));
    }
    // A shield's block chance, with the Defense talents give.
    if s.block_value > 0 || s.block_pct > 0.0 {
        s.block_pct = crate::rules::stats::block_pct(s.defense, s.level)
            + f64::from(sum(c, |e| matches!(e, TalentEffect::BlockPct { .. })));
        s.block_pct = s.block_pct.min(100.0);
        let pct = sum(c, |e| matches!(e, TalentEffect::BlockValuePct { .. }));
        s.block_value = crate::rules::talents::scaled(s.block_value, pct);
    }
    // Agility's armour is not the items'.
    let items_armor = s.armor.saturating_sub(2 * s.base[1]);
    let pct = sum(c, |e| matches!(e, TalentEffect::ItemArmorPct { .. }));
    s.armor += crate::rules::talents::scaled(items_armor, pct) - items_armor;
    // Stat talents, last: a percent is of the total, base, Tier Bonus and gear.
    for (i, stat) in STATS.into_iter().enumerate() {
        let pct = sum(
            c,
            |e| matches!(e, TalentEffect::ModStat { stat: s, flat: false, .. } if *s == stat),
        );
        let flat = sum(
            c,
            |e| matches!(e, TalentEffect::ModStat { stat: s, flat: true, .. } if *s == stat),
        );
        let delta = s.base[i] * pct / 100 + flat;
        if delta > 0 {
            s.raise_stat(i, delta);
        }
    }
    // Then the maximum health, stamina's share included (Vitality).
    let pct = sum(c, |e| matches!(e, TalentEffect::MaxHealthPct { .. }));
    s.max_health = crate::rules::talents::scaled(s.max_health, pct);
    s
}

/// The percent more healing `c` receives at `health_pct` percent of their health.
pub(crate) fn healing_taken_pct(c: &Char, health_pct: u32) -> u32 {
    sum(c, |e| {
        matches!(e, TalentEffect::HealingTakenPct { below_health_pct: b, .. }
            if *b == 0 || health_pct < *b)
    })
}

const STATS: [Stat; 5] = [
    Stat::Strength,
    Stat::Agility,
    Stat::Stamina,
    Stat::Intellect,
    Stat::Spirit,
];

/// One proc that `happened` sets off, a learned talent's or a buff's, before its chance is rolled.
pub(crate) struct Proc {
    /// The spell its effects are logged under: the talent rank's, or the buff's.
    pub(crate) spell_id: u32,
    /// The triggering skill goes off again, free.
    pub(crate) repeat_cast: bool,
    /// A skill's cooldown, changed by this many ms.
    pub(crate) cooldown: Option<(&'static str, i32)>,
    /// Percent of the damage done splashed on the enemies within `radius_yd` of the target.
    pub(crate) splash_pct: u32,
    pub(crate) chance: u32,
    /// Percent chance per stack of the ticking aura; replaces `chance` when above 0.
    pub(crate) chance_per_stack: f32,
    /// Percent of the maximum health healed.
    pub(crate) heal_pct: u32,
    /// The keys of the buffs it grants.
    pub(crate) auras: Vec<String>,
    /// Points of the character's own power given.
    pub(crate) energize: u32,
    /// Combo points added on the event's target.
    pub(crate) combo_points: u32,
    /// Damage dealt to the event's target, a formula.
    pub(crate) damage: Option<crate::rules::skills::Linear>,
    /// Damage dealt to the event's target, a percent of the shield's Block Value.
    pub(crate) block_value_pct: u32,
    pub(crate) school: crate::rules::school::School,
    /// The skill whose `skill_amount_pct` talents raise the damage.
    pub(crate) amount_skill: Option<&'static str>,
    /// Damage dealt to the event's target, a percent of the average weapon swing.
    pub(crate) weapon_pct: u32,
    /// Only while the aura row of this key is on the character.
    pub(crate) while_aura: Option<&'static str>,
    /// Yards from the target a consumed aura's copy may spread to; 0 for none.
    pub(crate) spread: f32,
    /// The keys of the auras put on every enemy within `radius_yd` of the caster.
    pub(crate) area_auras: Vec<String>,
    pub(crate) radius_yd: f32,
    /// Percent of the triggering cast's cost given back.
    pub(crate) refund_cost_pct: u32,
    /// A shield of this percent of the triggering cast's cost, for `absorb_ms`.
    pub(crate) absorb_cost_pct: u32,
    pub(crate) absorb_ms: u32,
    /// The shield instead of everything else when the bar was full before the cast, and
    /// everything else without the shield when it was not.
    pub(crate) absorb_at_full_power: bool,
    /// Only when the target wore this aura row's aura (or, `slowed`, a snare) before the cast.
    pub(crate) requires_target_aura: Option<&'static str>,
    /// Percent of the character's base mana given.
    pub(crate) energize_pct_of_base: u32,
    /// The keys of the auras put on the event's target.
    pub(crate) target_auras: Vec<String>,
    /// Yards from the target the ticking aura's copy may spread to; 0 for none.
    pub(crate) spread_aura: f32,
    /// Percent of the damage done that heals the friendly players within `radius_yd`.
    pub(crate) heal_allies_pct: u32,
    /// Only after a cast that cost health.
    pub(crate) requires_health_cost: bool,
    /// What the shield is a share of (the cast's cost when `None`), and the school it soaks.
    pub(crate) absorb_of: Option<callings::PctOf>,
    pub(crate) absorb_school: Option<crate::rules::school::School>,
}

/// The procs of `c`'s talents that `happened` sets off.
pub(crate) fn procs(c: &Char, happened: &crate::rules::procs::Happened) -> Vec<Proc> {
    set_talents(c)
        .flat_map(|t| t.effects.iter().map(move |e| (t, e, 0)))
        .filter_map(|(t, e, r)| match e {
            TalentEffect::Proc {
                on,
                ms,
                repeat_cast,
                cooldown_skill,
                cooldown_ms,
                splash_pct,
                skill,
                yields_xp,
                chance,
                chances,
                chances_per_stack,
                per_rank,
                auras,
                energize,
                combo_points,
                damage_attacker,
                block_value_pct,
                school,
                amount_skill,
                weapon_pct,
                while_aura,
                spread,
                area_auras,
                radius_yd,
                refund_cost_pct,
                absorb_cost_pct,
                absorb_duration_ms,
                at_full_power,
                requires_target_aura,
                energize_pct_of_base,
                target_auras,
                damage_school,
                spread_aura,
                heal_allies_pct,
                cost_kind,
                absorb_of,
                absorb_school,
            } => {
                let trigger = crate::rules::procs::Trigger {
                    on: *on,
                    skill: skill.as_deref(),
                    yields_xp: *yields_xp,
                    school: *damage_school,
                    ms: *ms,
                };
                trigger.fires(happened).then(|| Proc {
                    spell_id: t.rank_spells.get(r).copied().unwrap_or(0),
                    repeat_cast: *repeat_cast,
                    cooldown: cooldown_skill
                        .as_deref()
                        .map(|k| (k, cooldown_ms.get(r).copied().unwrap_or(0))),
                    splash_pct: splash_pct.get(r).copied().unwrap_or(0),
                    chance: chances.get(r).copied().unwrap_or(*chance),
                    chance_per_stack: chances_per_stack.get(r).copied().unwrap_or(0.0),
                    heal_pct: per_rank[r],
                    auras: auras.get(r).cloned().unwrap_or_default(),
                    energize: energize.get(r).copied().unwrap_or(0),
                    combo_points: combo_points.get(r).copied().unwrap_or(0),
                    damage: *damage_attacker,
                    block_value_pct: block_value_pct.get(r).copied().unwrap_or(0),
                    school: *school,
                    amount_skill: amount_skill.as_deref(),
                    weapon_pct: *weapon_pct,
                    while_aura: while_aura.as_deref(),
                    spread: *spread,
                    area_auras: area_auras.get(r).cloned().unwrap_or_default(),
                    radius_yd: *radius_yd,
                    refund_cost_pct: refund_cost_pct.get(r).copied().unwrap_or(0),
                    absorb_cost_pct: *absorb_cost_pct,
                    absorb_ms: *absorb_duration_ms,
                    absorb_at_full_power: at_full_power.as_deref() == Some("absorb"),
                    requires_target_aura: requires_target_aura.as_deref(),
                    energize_pct_of_base: energize_pct_of_base.get(r).copied().unwrap_or(0),
                    target_auras: target_auras.get(r).cloned().unwrap_or_default(),
                    spread_aura: *spread_aura,
                    heal_allies_pct: heal_allies_pct.get(r).copied().unwrap_or(0),
                    requires_health_cost: cost_kind.as_deref() == Some("health"),
                    absorb_of: *absorb_of,
                    absorb_school: *absorb_school,
                })
            }
            _ => None,
        })
        .collect()
}

/// A buff's proc, as [`procs`] gives a talent's, when `happened` sets it off.
pub(crate) fn aura_proc(
    spell_id: u32,
    p: &'static crate::callings::AuraProc,
    happened: &crate::rules::procs::Happened,
) -> Option<Proc> {
    let trigger = crate::rules::procs::Trigger {
        on: p.on,
        skill: p.skill.as_deref(),
        yields_xp: false,
        school: None,
        ms: 0,
    };
    trigger.fires(happened).then(|| Proc {
        spell_id,
        repeat_cast: false,
        cooldown: None,
        splash_pct: 0,
        chance: p.chance,
        chance_per_stack: 0.0,
        heal_pct: p.heal_pct,
        auras: p.apply_aura.iter().cloned().collect(),
        energize: p.energize,
        combo_points: p.add_combo_points,
        damage: p.damage_attacker,
        block_value_pct: p.block_value_pct,
        school: p.school,
        amount_skill: p.amount_skill.as_deref(),
        weapon_pct: p.weapon_pct,
        while_aura: None,
        spread: 0.0,
        area_auras: Vec::new(),
        radius_yd: 0.0,
        refund_cost_pct: 0,
        absorb_cost_pct: 0,
        absorb_ms: 0,
        absorb_at_full_power: false,
        requires_target_aura: None,
        energize_pct_of_base: p.energize_pct_of_base,
        target_auras: Vec::new(),
        spread_aura: 0.0,
        heal_allies_pct: 0,
        requires_health_cost: false,
        absorb_of: None,
        absorb_school: None,
    })
}

/// The percent `c`'s out-of-combat health regeneration is raised by.
pub(crate) fn health_regen_pct(c: &Char) -> u32 {
    sum(c, |e| {
        matches!(e, TalentEffect::ModHealthRegenOutOfCombat { .. })
    })
}

/// The value `c`'s talents give one spell-mod cell; 0 when none touches it.
pub(crate) fn spell_mod_value(c: &Char, flat: bool, bit: u8, op: u8) -> i32 {
    spell_mods(c)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::SpellModifier {
                flat: f,
                mask_bit,
                op: o,
                value,
            } if (f, mask_bit, o) == (flat, bit, op) => Some(value),
            _ => None,
        })
        .unwrap_or(0)
}

/// Every spell-mod cell the current rows' Talents can touch, at its value for what `c` has set:
/// sent at login and whenever one moves, since each packet sets a cell outright.
pub(crate) fn spell_mods(c: &Char) -> Vec<SessionEvent> {
    let mut cells: Vec<(bool, u8, u8, i32)> = Vec::new();
    for row in rows(c) {
        let set = chosen(c, row);
        for choice in &row.choices {
            let RowChoice::Talent(t) = choice else {
                continue;
            };
            let on = set.is_some_and(|s| s.spell_id() == choice.spell_id());
            for m in &t.client_spellmods {
                let value = if on { m.per_rank[0] } else { 0 };
                match cells
                    .iter_mut()
                    .find(|(f, b, o, _)| (*f, *b, *o) == (m.flat, m.family_bit, m.op))
                {
                    Some(cell) => cell.3 += value,
                    None => cells.push((m.flat, m.family_bit, m.op, value)),
                }
            }
        }
    }
    cells
        .into_iter()
        .map(|(flat, mask_bit, op, value)| SessionEvent::SpellModifier {
            flat,
            mask_bit,
            op,
            value,
        })
        .collect()
}
