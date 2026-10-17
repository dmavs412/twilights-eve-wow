//! The Callings the master CSV defines: `data/twilights-eve/server_spells.json`, which
//! `tools/twilights-eve/build_spells.py` writes, read into the same [`CallingDef`] the TOML tables
//! make. A Calling comes from here when the sheet has a `calling` row for it (the file then
//! carries its `tier`); the others stay on `content/callings.toml`.
//!
//! A skill or talent whose effects the realm has no kind for yet loads as unmodelled: a talent
//! with no effects (learnable, shown as WIP), and a skill dropped until its effects land, so the
//! bar and the spell book hold only what the realm can cast.

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::callings::{
    Armour, CallingDef, Effect, Prereq, Skill, TalentDef, TalentEffect, TalentTab, Target,
    TierBonus,
};

const SERVER_SPELLS: &str = include_str!("../../../data/twilights-eve/server_spells.json");

#[derive(Deserialize)]
struct GenCalling {
    key: String,
    name: Option<String>,
    tier: Option<u8>,
    tier_cap: Option<u8>,
    tier_bonus: Option<TierBonus>,
    shapeshift_form: Option<u8>,
    #[serde(default)]
    armour: Vec<Armour>,
    #[serde(default)]
    skills: Vec<GenSkill>,
    #[serde(default)]
    talent_tabs: Vec<GenTab>,
}

#[derive(Deserialize)]
struct GenEffect {
    kind: String,
    #[serde(default)]
    params: Map<String, Value>,
}

#[derive(Deserialize)]
struct GenSkill {
    key: String,
    spell_id: u32,
    #[serde(default)]
    realm_cost: u32,
    #[serde(default)]
    realm_cooldown_ms: u32,
    #[serde(default)]
    realm_gcd_ms: u32,
    #[serde(default)]
    realm_range_min: f32,
    #[serde(default)]
    realm_range_max: f32,
    #[serde(default)]
    realm_radius_yd: f32,
    target: Option<Target>,
    #[serde(default)]
    requires_out_of_combat: bool,
    #[serde(default)]
    effects: Vec<GenEffect>,
}

#[derive(Deserialize)]
struct GenTab {
    id: Option<u32>,
    name: String,
    background: Option<String>,
    #[serde(default)]
    order: u32,
    talents: Vec<GenTalent>,
}

#[derive(Deserialize)]
struct Seat {
    row: u8,
    column: u8,
}

#[derive(Deserialize)]
struct GenTalent {
    key: String,
    seat: Seat,
    #[serde(default)]
    exceptional: bool,
    prereq: Option<Prereq>,
    ranks: Vec<GenRank>,
}

#[derive(Deserialize)]
struct GenRank {
    spell_id: u32,
    talent_id: u32,
    #[serde(default)]
    effects: Vec<GenEffect>,
}

/// Every Calling the sheet defines.
pub(crate) fn callings() -> Vec<CallingDef> {
    let all: Vec<Value> =
        serde_json::from_str(SERVER_SPELLS).expect("data/twilights-eve/server_spells.json parses");
    // Only a Calling with a `calling` row (so a tier) is read; the others are still the TOML's
    // and carry rows (a party target) the realm has no kind for.
    all.into_iter()
        .filter(|c| c.get("tier").is_some())
        .map(|c| {
            calling(
                serde_json::from_value(c)
                    .expect("a calling in data/twilights-eve/server_spells.json reads"),
            )
        })
        .collect()
}

fn calling(g: GenCalling) -> CallingDef {
    let tier = g.tier.expect("filtered on tier");
    let missing = |field: &str| -> ! {
        panic!(
            "server_spells.json {}: the calling row has no {field}",
            g.key
        )
    };
    let mut tabs: Vec<&GenTab> = g.talent_tabs.iter().collect();
    tabs.sort_by_key(|t| t.order);
    CallingDef {
        name: g.name.clone().unwrap_or_else(|| missing("name")),
        tier,
        cap: g.tier_cap.unwrap_or_else(|| missing("tier_cap")),
        tier_bonus: g.tier_bonus.unwrap_or_else(|| missing("tier_bonus")),
        shapeshift_form: g
            .shapeshift_form
            .unwrap_or_else(|| missing("shapeshift_form")),
        armour: g.armour.clone(),
        skills: g.skills.iter().filter_map(skill).collect(),
        talent_tabs: tabs.into_iter().map(|t| tab(&g.key, t)).collect(),
        key: g.key.clone(),
    }
}

/// The realm's [`Effect`] for a row's effect, or `None` for a kind it does not have yet.
fn effect(e: &GenEffect) -> Option<Effect> {
    let mut object = e.params.clone();
    object.insert("kind".into(), e.kind.clone().into());
    serde_json::from_value(Value::Object(object)).ok()
}

/// The skill, or `None` while it has no effects or any is a kind the realm lacks.
fn skill(g: &GenSkill) -> Option<Skill> {
    let effects: Option<Vec<Effect>> = g.effects.iter().map(effect).collect();
    let effects = effects.filter(|e| !e.is_empty())?;
    Some(Skill {
        key: g.key.clone(),
        spell_id: g.spell_id,
        rage_cost: g.realm_cost,
        health_cost_pct_of_base_health: 0,
        cooldown_ms: g.realm_cooldown_ms,
        gcd_ms: g.realm_gcd_ms,
        target: g.target.unwrap_or(Target::Enemy),
        range_yd: [g.realm_range_min, g.realm_range_max],
        radius_yd: g.realm_radius_yd,
        max_targets: 0,
        requires_out_of_combat: g.requires_out_of_combat,
        requires_target_dodged: false,
        puts_in_combat_ms: 0,
        requires_target_health_pct: 0,
        extra_targets: 0,
        extra_target_radius_yd: 0.0,
        borrowed: None,
        effects,
    })
}

fn tab(calling: &str, g: &GenTab) -> TalentTab {
    let missing = |field: &str| -> ! {
        panic!(
            "server_spells.json {calling}/{}: the talent_tab row has no {field}",
            g.name
        )
    };
    TalentTab {
        id: g.id.unwrap_or_else(|| missing("id")),
        name: g.name.clone(),
        background: g
            .background
            .clone()
            .unwrap_or_else(|| missing("background")),
        talents: g.talents.iter().map(talent).collect(),
    }
}

fn talent(g: &GenTalent) -> TalentDef {
    TalentDef {
        key: g.key.clone(),
        talent_id: g.ranks[0].talent_id,
        tier: g.seat.row,
        column: g.seat.column,
        rank_spells: g.ranks.iter().map(|r| r.spell_id).collect(),
        prereq: g.prereq.as_ref().map(|p| Prereq {
            key: p.key.clone(),
            rank: p.rank,
        }),
        exceptional: g.exceptional,
        effects: talent_effects(&g.ranks),
        skill: None,
        client_spellmods: Vec::new(),
    }
}

/// What the talent does, one [`TalentEffect`] per effect slot with a value per rank; none at all
/// if any slot is a kind the realm lacks. A rank's size is its `pct` or `amount` param.
fn talent_effects(ranks: &[GenRank]) -> Vec<TalentEffect> {
    (0..ranks[0].effects.len())
        .map(|slot| talent_effect(ranks, slot))
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default()
}

fn talent_effect(ranks: &[GenRank], slot: usize) -> Option<TalentEffect> {
    let first = ranks[0].effects.get(slot)?;
    let mut object = first.params.clone();
    let key = ["pct", "amount"]
        .into_iter()
        .find(|k| object.contains_key(*k))?;
    object.remove(key);
    // An effect that can be a percent or a flat amount (`mod_stat`) reads which it was.
    object.insert("flat".into(), (key == "amount").into());
    let sizes: Option<Vec<Value>> = ranks
        .iter()
        .map(|r| r.effects.get(slot)?.params.get(key).cloned())
        .collect();
    object.insert("per_rank".into(), Value::Array(sizes?));
    object.insert("kind".into(), first.kind.clone().into());
    serde_json::from_value(Value::Object(object)).ok()
}
