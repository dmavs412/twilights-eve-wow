//! A character's talents: the points they have to spend, learning a talent of the current
//! Calling's tree, the respec, and what the learned talents add to a fight. The tree is data
//! ([`crate::callings::TalentTab`]); nothing here names a talent.
//!
//! The client reads a talent's rank as the highest rank spell it knows (vmangos
//! `Player::LearnTalent` teaches the new rank's spell), so a learned talent's rank spells ride the
//! spellbook, a learn sends `SMSG_LEARNED_SPELL`, and a respec one `SMSG_REMOVED_SPELL` a rank.

use benilla_protocol::SessionEvent;

use crate::callings::{
    self, CritBleedOf, Skill, Stat, TalentDef, TalentEffect, TalentTab, Wielded,
};
use crate::characters::Char;
use crate::fields::values;
use crate::rules::power::PowerKind;
use crate::rules::stats::CharacterStats;
use crate::rules::talents::{points_at, respec_cost};

/// `PLAYER_CHARACTER_POINTS1`: the unspent Talent Points.
const PLAYER_CHARACTER_POINTS1: u16 = 1102;

/// Points needed in a tab per tier above the first (`Talent.dbc` tiers, vmangos).
const POINTS_PER_TIER: u32 = 5;

/// The current Calling's tree.
fn tree(c: &Char) -> &'static [TalentTab] {
    &callings::of(c.calling).talent_tabs
}

/// A talent of the current tree, and its tab.
fn find(c: &Char, talent_id: u32) -> Option<(&'static TalentTab, &'static TalentDef)> {
    tree(c).iter().find_map(|tab| {
        tab.talents
            .iter()
            .find(|t| t.talent_id == talent_id)
            .map(|t| (tab, t))
    })
}

/// The rank `c` has in `talent_id`, 0 for none.
fn rank(c: &Char, talent_id: u32) -> u8 {
    c.talents
        .iter()
        .find(|(id, _)| *id == talent_id)
        .map_or(0, |(_, r)| *r)
}

/// The points spent, in all or in one tab.
fn spent(c: &Char, tab: Option<&TalentTab>) -> u32 {
    c.talents
        .iter()
        .filter(|(id, _)| tab.is_none_or(|tab| tab.talents.iter().any(|t| t.talent_id == *id)))
        .map(|(_, r)| u32::from(*r))
        .sum()
}

/// The Talent Points `c` has left to spend.
pub(crate) fn free_points(c: &Char) -> u32 {
    points_at(c.level).saturating_sub(spent(c, None))
}

/// The points field, as the player's create and every change carry it.
pub(crate) fn points_pair(c: &Char) -> (u16, u32) {
    (PLAYER_CHARACTER_POINTS1, free_points(c))
}

/// Drops from a loaded save what the tree no longer has: a talent it lost, ranks past a talent's
/// last, and the talents bought last when they cost more points than the level gives.
pub(crate) fn keep_valid(c: &mut Char) {
    let mut kept: Vec<(u32, u8)> = Vec::new();
    for &(id, r) in &c.talents {
        let Some((_, t)) = find(c, id) else {
            continue;
        };
        let r = r.min(t.rank_spells.len() as u8);
        if r > 0 && !kept.iter().any(|(k, _)| *k == id) {
            kept.push((id, r));
        }
    }
    let mut points = points_at(c.level);
    kept.retain(|(_, r)| {
        let fits = u32::from(*r) <= points;
        if fits {
            points -= u32::from(*r);
        }
        fits
    });
    c.talents = kept;
}

/// Every learned rank's spell, for the spellbook at login.
pub(crate) fn known_spells(c: &Char) -> impl Iterator<Item = u32> + '_ {
    c.talents.iter().flat_map(|&(id, r)| {
        find(c, id)
            .map(|(_, t)| &t.rank_spells[..usize::from(r).min(t.rank_spells.len())])
            .unwrap_or(&[])
            .iter()
            .copied()
    })
}

/// Every learned talent's effects, each with the index of the rank learned.
fn learned(c: &Char) -> impl Iterator<Item = (&'static TalentEffect, usize)> + '_ {
    c.talents
        .iter()
        .filter_map(|&(id, r)| find(c, id).map(|(_, t)| (t, r)))
        .flat_map(|(t, r)| t.effects.iter().map(move |e| (e, usize::from(r) - 1)))
}

/// The size of each learned talent's effects that `pick` takes, summed.
fn sum(c: &Char, pick: impl Fn(&TalentEffect) -> bool) -> u32 {
    learned(c)
        .filter(|(e, _)| pick(e))
        .map(|(e, r)| e.per_rank()[r])
        .sum()
}

/// A skill `c` can cast: one of the current Calling's, or a learned active talent's.
pub(crate) fn skill(c: &Char, spell_id: u32) -> Option<&'static Skill> {
    let own = callings::of(c.calling).skills.iter();
    let talents = tree(c)
        .iter()
        .flat_map(|tab| &tab.talents)
        .filter(|t| rank(c, t.talent_id) > 0)
        .filter_map(|t| t.skill.as_ref());
    own.chain(talents).find(|s| s.spell_id == spell_id)
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
pub(crate) fn skill_crit_damage_pct(c: &Char, builder: bool) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::SkillCritDamagePct { builders, .. } if builder || !builders),
    )
}

/// The percent chance a skill also lands on one more enemy in front of the caster (Flurry of
/// Blades).
pub(crate) fn extra_target_pct(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::SkillExtraTarget { skill: s, .. } if s == skill),
    )
}

/// The percent chance a skill's combo point gains one more, on a target that is bleeding or
/// not (Serrated Strikes).
pub(crate) fn bonus_combo_pct(c: &Char, skill: &str, bleeding: bool) -> u32 {
    sum(c, |e| {
        matches!(e, TalentEffect::SkillBonusCombo { skill: s, target_state, .. }
            if s == skill && (bleeding || target_state.is_none()))
    })
}

/// The milliseconds a skill's auras last longer (Bloodletting).
pub(crate) fn duration_ms(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::SkillDurationMs { skill: s, .. } if s == skill),
    )
}

/// The percent a skill's auras last longer.
pub(crate) fn duration_pct(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::SkillDurationPct { skill: s, .. } if s == skill),
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

/// What a skill costs `c`, in whole points of its power.
pub(crate) fn power_cost(c: &Char, skill: &callings::Skill) -> u32 {
    let less = sum(
        c,
        |e| matches!(e, TalentEffect::SkillCost { skill: s, .. } if *s == skill.key),
    );
    skill.power_cost.saturating_sub(less)
}

/// The percent a skill's damage and buff amounts rise by.
pub(crate) fn amount_pct(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::SkillAmountPct { skill: s, .. } if s == skill),
    )
}

/// The extra rage a skill gives.
pub(crate) fn extra_rage(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::SkillRage { skill: s, .. } if s == skill),
    )
}

/// The extra critical strike chance of a skill, in percent.
pub(crate) fn skill_crit_pct(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::SkillCritPct { skill: s, .. } if s == skill),
    )
}

/// The percent of the bonus damage its consumed stacks dealt that a skill heals the caster for.
pub(crate) fn consume_heal_pct(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::ConsumeHealPct { skill: s, .. } if s == skill),
    )
}

/// The percent more damage a tick of the patches a skill leaves deals.
pub(crate) fn ground_amount_pct(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::GroundAmountPct { skill: s, .. } if s == skill),
    )
}

/// The milliseconds the patches a skill leaves last longer.
pub(crate) fn ground_duration_ms(c: &Char, skill: &str) -> u32 {
    sum(
        c,
        |e| matches!(e, TalentEffect::GroundDurationMs { skill: s, .. } if s == skill),
    )
}

/// The bleeds a melee crit leaves: `(aura spell, tick ms, duration ms, percent, what of)`.
pub(crate) fn crit_bleeds(c: &Char) -> Vec<(u32, u32, u32, u32, CritBleedOf)> {
    c.talents
        .iter()
        .filter_map(|&(id, r)| {
            let (_, t) = find(c, id)?;
            t.effects.iter().find_map(|e| match e {
                TalentEffect::CritBleed {
                    aura_spell_id,
                    tick_ms,
                    duration_ms,
                    of,
                    per_rank,
                } => Some((
                    *aura_spell_id,
                    *tick_ms,
                    *duration_ms,
                    per_rank[usize::from(r) - 1],
                    *of,
                )),
                _ => None,
            })
        })
        .collect()
}

/// The character's stats: gear's, and the talents' on top.
pub(crate) fn stats(c: &Char) -> CharacterStats {
    let mut s = crate::equipment::stats(c);
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
    match PowerKind::of_class(s.class) {
        PowerKind::Mana => s.max_mana += max_power,
        PowerKind::Energy => s.max_energy_bonus = max_power,
        PowerKind::Rage => {}
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
    s
}

const STATS: [Stat; 5] = [
    Stat::Strength,
    Stat::Agility,
    Stat::Stamina,
    Stat::Intellect,
    Stat::Spirit,
];

/// One learned proc that `happened` sets off, before its chance is rolled.
pub(crate) struct Proc {
    pub(crate) chance: u32,
    /// Percent of the maximum health healed.
    pub(crate) heal_pct: u32,
    /// The keys of the buffs it grants.
    pub(crate) auras: Vec<String>,
    /// Points of the character's own power given.
    pub(crate) energize: u32,
    /// Combo points added on the event's target.
    pub(crate) combo_points: u32,
}

/// The procs of `c`'s talents that `happened` sets off.
pub(crate) fn procs(c: &Char, happened: &crate::rules::procs::Happened) -> Vec<Proc> {
    learned(c)
        .filter_map(|(e, r)| match e {
            TalentEffect::Proc {
                on,
                skill,
                yields_xp,
                chance,
                chances,
                per_rank,
                auras,
                energize,
                combo_points,
            } => {
                let trigger = crate::rules::procs::Trigger {
                    on: *on,
                    skill: skill.as_deref(),
                    yields_xp: *yields_xp,
                };
                trigger.fires(happened).then(|| Proc {
                    chance: chances.get(r).copied().unwrap_or(*chance),
                    heal_pct: per_rank[r],
                    auras: auras.get(r).cloned().unwrap_or_default(),
                    energize: energize.get(r).copied().unwrap_or(0),
                    combo_points: combo_points.get(r).copied().unwrap_or(0),
                })
            }
            _ => None,
        })
        .collect()
}

/// The percent `c`'s out-of-combat health regeneration is raised by.
pub(crate) fn health_regen_pct(c: &Char) -> u32 {
    sum(c, |e| {
        matches!(e, TalentEffect::ModHealthRegenOutOfCombat { .. })
    })
}

/// Why a talent could not be learned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// Not in the current Calling's tree.
    NotInTree,
    /// Already at that rank or past its last.
    Rank,
    NoPoints,
    /// Not enough points spent in the tab for its tier.
    Tier,
    /// Its prerequisite is not at the rank it needs.
    Prereq,
}

/// `CMSG_LEARN_TALENT` for the 0-based `rank`: vmangos's `Player::LearnTalent` rules. Learns up to
/// that rank, paying a point a rank, and answers the new spell and points.
pub(crate) fn learn(
    c: &mut Char,
    talent_id: u32,
    rank: u32,
    events: &mut Vec<SessionEvent>,
) -> Result<(), Refusal> {
    let (tab, t) = find(c, talent_id).ok_or(Refusal::NotInTree)?;
    let have = u32::from(self::rank(c, talent_id));
    let want = rank + 1;
    if want <= have || want as usize > t.rank_spells.len() {
        return Err(Refusal::Rank);
    }
    if free_points(c) < want - have {
        return Err(Refusal::NoPoints);
    }
    if spent(c, Some(tab)) < u32::from(t.tier - 1) * POINTS_PER_TIER {
        return Err(Refusal::Tier);
    }
    if let Some(p) = &t.prereq {
        let needed = tab
            .talents
            .iter()
            .find(|o| o.key == p.key)
            .map_or(0, |o| self::rank(c, o.talent_id));
        if needed < p.rank {
            return Err(Refusal::Prereq);
        }
    }
    c.talents.retain(|(id, _)| *id != talent_id);
    c.talents.push((talent_id, want as u8));
    for &spell_id in &t.rank_spells[have as usize..want as usize] {
        events.push(SessionEvent::SpellLearned { spell_id });
    }
    if !t.client_spellmods.is_empty() {
        events.extend(spell_mods(c));
    }
    events.push(values(c.guid, &[points_pair(c)]));
    Ok(())
}

impl crate::Realm {
    /// The Calling of the character in the world, whose Talent Tree the client's talent frame
    /// must show; `None` at character select.
    pub fn current_calling(&self) -> Option<&'static callings::CallingDef> {
        let w = self.world.as_ref()?;
        self.characters.get(w.guid).map(|c| callings::of(c.calling))
    }

    /// `CMSG_LEARN_TALENT` from the character in the world: a refusal answers nothing, as
    /// vmangos's does; a learn redoes the sheet and saves.
    pub(crate) fn learn_talent(&mut self, talent_id: u32, rank: u32) {
        let Some(w) = &mut self.world else {
            return;
        };
        let Some(c) = self.characters.get_mut(w.guid) else {
            return;
        };
        if learn(c, talent_id, rank, &mut self.events).is_ok() {
            w.refresh_gear(c, &mut self.events);
            let guid = w.guid;
            self.save(guid);
        }
    }
}

/// Whether `c` has any talent to wipe.
pub(crate) fn any(c: &Char) -> bool {
    !c.talents.is_empty()
}

/// What `c`'s next respec costs, in copper.
pub(crate) fn wipe_cost(c: &Char) -> u32 {
    respec_cost(c.talent_wipes)
}

/// Unlearns every talent (vmangos `Player::ResetTalents`): each rank's spell goes, every point
/// comes back, and the spell mods return to nothing. The price is the caller's.
pub(crate) fn wipe(c: &mut Char, events: &mut Vec<SessionEvent>) {
    let spells: Vec<u32> = known_spells(c).collect();
    c.talents.clear();
    for spell_id in spells {
        events.push(SessionEvent::SpellRemoved { spell_id });
    }
    events.extend(spell_mods(c));
    events.push(values(c.guid, &[points_pair(c)]));
}

/// Every spell-mod cell the current tree can touch, at its value for what `c` has learned: sent at
/// login and whenever one moves, since each packet sets a cell outright.
pub(crate) fn spell_mods(c: &Char) -> Vec<SessionEvent> {
    let mut cells: Vec<(bool, u8, u8, i32)> = Vec::new();
    for t in tree(c).iter().flat_map(|tab| &tab.talents) {
        let r = usize::from(self::rank(c, t.talent_id));
        for m in &t.client_spellmods {
            let value = if r == 0 { 0 } else { m.per_rank[r - 1] };
            match cells
                .iter_mut()
                .find(|(f, b, o, _)| (*f, *b, *o) == (m.flat, m.family_bit, m.op))
            {
                Some(cell) => cell.3 += value,
                None => cells.push((m.flat, m.family_bit, m.op, value)),
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
