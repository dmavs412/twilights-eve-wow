//! A character's talents: the points they have to spend, learning a talent of the current
//! Calling's tree, the respec, and what the learned talents add to a fight. The tree is data
//! ([`crate::callings::TalentTab`]); nothing here names a talent.
//!
//! The client reads a talent's rank as the highest rank spell it knows (vmangos
//! `Player::LearnTalent` teaches the new rank's spell), so a learned talent's rank spells ride the
//! spellbook, a learn sends `SMSG_LEARNED_SPELL`, and a respec one `SMSG_REMOVED_SPELL` a rank.

use benilla_protocol::SessionEvent;

use crate::callings::{self, TalentDef, TalentEffect, TalentTab};
use crate::characters::Char;
use crate::fields::values;
use crate::rules::stats::WarriorStats;
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

/// The size of each learned talent's effects that `pick` takes, summed.
fn sum(c: &Char, pick: impl Fn(&TalentEffect) -> bool) -> u32 {
    c.talents
        .iter()
        .filter_map(|&(id, r)| find(c, id).map(|(_, t)| (t, r)))
        .flat_map(|(t, r)| t.effects.iter().map(move |e| (e, r)))
        .filter(|(e, _)| pick(e))
        .map(|(e, r)| e.per_rank()[usize::from(r) - 1])
        .sum()
}

/// What a skill costs `c`, in whole rage points.
pub(crate) fn rage_cost(c: &Char, skill: &callings::Skill) -> u32 {
    let less = sum(
        c,
        |e| matches!(e, TalentEffect::SkillRageCost { skill: s, .. } if *s == skill.key),
    );
    skill.rage_cost.saturating_sub(less)
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

/// The bleed a melee crit leaves: `(aura spell, tick ms, duration ms, percent of the swing)`.
pub(crate) fn crit_bleed(c: &Char) -> Option<(u32, u32, u32, u32)> {
    c.talents.iter().find_map(|&(id, r)| {
        let (_, t) = find(c, id)?;
        t.effects.iter().find_map(|e| match e {
            TalentEffect::CritBleed {
                aura_spell_id,
                tick_ms,
                duration_ms,
                per_rank,
            } => Some((
                *aura_spell_id,
                *tick_ms,
                *duration_ms,
                per_rank[usize::from(r) - 1],
            )),
            _ => None,
        })
    })
}

/// The character's stats: gear's, and the talents' on top.
pub(crate) fn stats(c: &Char) -> WarriorStats {
    let mut s = crate::equipment::stats(c);
    s.crit_pct += f64::from(sum(c, |e| matches!(e, TalentEffect::MeleeCritPct { .. })));
    s
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
    if t.client_spellmod.is_some() {
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
        let Some(m) = &t.client_spellmod else {
            continue;
        };
        let r = usize::from(self::rank(c, t.talent_id));
        let value = if r == 0 { 0 } else { m.per_rank[r - 1] };
        match cells
            .iter_mut()
            .find(|(f, b, o, _)| (*f, *b, *o) == (m.flat, m.family_bit, m.op))
        {
            Some(cell) => cell.3 += value,
            None => cells.push((m.flat, m.family_bit, m.op, value)),
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
