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
    self, Armour, AuraDef, AuraEffect, AuraRow, CallingDef, ClientSpellMod, Effect, Prereq, Skill,
    SpellMeta, StatProfile, TalentDef, TalentEffect, TalentTab, Target, TierBonus, WeaponKind,
};
use crate::rules::power::PowerKind;

const SERVER_SPELLS: &str = include_str!("../../../data/twilights-eve/server_spells.json");

#[derive(Deserialize)]
struct GenCalling {
    key: String,
    /// Its `callings.csv` row, on every Calling in the file.
    power: PowerKind,
    stat_profile: StatProfile,
    item_class: StatProfile,
    health_pct: u32,
    mana_pct: u32,
    energy_per_tick: Option<u32>,
    #[serde(default)]
    bleed_drop: Option<String>,
    #[serde(default)]
    bleed_drop_chance: u32,
    #[serde(default)]
    weapons: Vec<WeaponKind>,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    spellbook: String,
    #[serde(default)]
    blurb: String,
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
    #[serde(default)]
    auras: Vec<GenAura>,
}

#[derive(Deserialize)]
struct GenAura {
    key: String,
    spell_id: u32,
    #[serde(default)]
    duration_ms: u32,
    #[serde(default)]
    charges: u32,
    #[serde(default)]
    modelled: bool,
    #[serde(default)]
    effects: Vec<GenEffect>,
    /// The aura-only columns; `row.break_on.damage` is read for a skill that applies it (an
    /// incapacitate).
    #[serde(flatten)]
    row: AuraRow,
}

#[derive(Clone, Deserialize)]
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
    learn_level: u8,
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
    requires_stealth: bool,
    #[serde(default)]
    requires_target_out_of_combat: bool,
    /// `Some(false)` for a hidden ability, whose `spell_id` is its `realm_spell_id`.
    client_visible: Option<bool>,
    /// `Some(false)` for a visible skill kept off the spell book (`spellbook=no`), cast only as
    /// another's `becomes=`.
    spellbook: Option<bool>,
    #[serde(default)]
    effects: Vec<GenEffect>,
    #[serde(flatten)]
    meta: SpellMeta,
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
    #[serde(default)]
    key: String,
    spell_id: u32,
    talent_id: u32,
    #[serde(default)]
    effects: Vec<GenEffect>,
    /// An active talent's skill columns; `target` set marks one.
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
    #[serde(flatten)]
    meta: SpellMeta,
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
        power: g.power,
        stat_profile: g.stat_profile,
        item_class: g.item_class,
        health_pct: g.health_pct,
        mana_pct: g.mana_pct,
        energy_per_tick: g.energy_per_tick,
        bleed_drop: g.bleed_drop.clone(),
        bleed_drop_chance: g.bleed_drop_chance,
        weapons: g.weapons.clone(),
        icon: g.icon.clone(),
        spellbook: g.spellbook.clone(),
        blurb: g.blurb.clone(),
        cap: g.tier_cap.unwrap_or_else(|| missing("tier_cap")),
        tier_bonus: g.tier_bonus.unwrap_or_else(|| missing("tier_bonus")),
        shapeshift_form: g
            .shapeshift_form
            .unwrap_or_else(|| missing("shapeshift_form")),
        armour: g.armour.clone(),
        skills: g
            .skills
            .iter()
            .filter(|s| !s.off_the_spell_book())
            .filter_map(|s| skill(s, &g.auras))
            .collect(),
        hidden_skills: g
            .skills
            .iter()
            .filter(|s| s.off_the_spell_book())
            .filter_map(|s| skill(s, &g.auras))
            .collect(),
        talent_tabs: tabs.into_iter().map(|t| tab(&g.key, t, &g.auras)).collect(),
        auras: g.auras.iter().filter_map(aura).collect(),
        key: g.key.clone(),
    }
}

impl GenSkill {
    /// A hidden ability or a `spellbook=no` skill: loaded among the hidden skills, found by key.
    fn off_the_spell_book(&self) -> bool {
        self.client_visible == Some(false) || self.spellbook == Some(false)
    }
}

/// The buff, or `None` while it is unmodelled or any effect is a kind the realm lacks.
fn aura(g: &GenAura) -> Option<AuraDef> {
    let effects: Option<Vec<AuraEffect>> = g
        .effects
        .iter()
        .map(|e| {
            let mut object = e.params.clone();
            object.insert("kind".into(), e.kind.clone().into());
            serde_json::from_value(Value::Object(object)).ok()
        })
        .collect();
    let effects = effects.filter(|e| g.modelled && !e.is_empty())?;
    Some(AuraDef {
        key: g.key.clone(),
        spell_id: g.spell_id,
        duration_ms: g.duration_ms,
        charges: g.charges,
        effects,
        row: g.row.clone(),
    })
}

/// The realm's [`Effect`] for a row's effect, or `None` for a kind it does not have yet. An effect
/// with `apply_aura=key` takes its params from that aura row's effect of the same kind, plus the
/// row's duration and, for `break_on: damage`, `break_on_damage`; params the skill's effect sets
/// itself (`per_combo`) win. An aura that is missing or unmodelled makes it `None`.
fn effect(e: &GenEffect, auras: &[GenAura]) -> Option<Effect> {
    let mut object = e.params.clone();
    // An effect that works on stacks of an aura (`aura=key`) reads the aura row's spell id,
    // duration and stack columns.
    if let Some(key) = object.remove("aura") {
        let aura = auras
            .iter()
            .find(|a| a.modelled && key.as_str() == Some(a.key.as_str()))?;
        object.insert("aura_spell_id".into(), aura.spell_id.into());
        if e.kind == "consume_aura" {
            object.insert("duration_ms".into(), aura.duration_ms.into());
        }
        if e.kind == "add_aura_stacks" {
            object.insert("duration_ms".into(), aura.duration_ms.into());
            object.insert("max_stacks".into(), aura.row.max_stacks.into());
            if let Some(rule) = &aura.row.stack_rule {
                object.insert("stack_rule".into(), rule.clone().into());
            }
        }
    }
    // A ground area's auras (`enemy_aura=key`, `ally_aura=key`) by their rows' spell ids.
    for side in ["enemy_aura", "ally_aura"] {
        if let Some(key) = object.remove(side) {
            let aura = auras
                .iter()
                .find(|a| a.modelled && key.as_str() == Some(a.key.as_str()))?;
            object.insert(format!("{side}_spell_id"), aura.spell_id.into());
        }
    }
    if let Some(key) = object.remove("apply_aura") {
        let aura = auras
            .iter()
            .find(|a| a.modelled && key.as_str() == Some(a.key.as_str()))?;
        let lasting = aura.effects.iter().find(|l| l.kind == e.kind)?;
        let mut merged = lasting.params.clone();
        if aura.duration_ms > 0 {
            merged.insert("duration_ms".into(), aura.duration_ms.into());
        }
        if e.kind == "incapacitate" && aura.row.break_on.contains_key("damage") {
            merged.insert("break_on_damage".into(), true.into());
        }
        if matches!(
            e.kind.as_str(),
            "periodic_damage" | "mod_run_speed" | "periodic_heal"
        ) {
            merged.insert("aura_spell_id".into(), aura.spell_id.into());
        }
        if e.kind == "periodic_damage" {
            if let Some(rule @ ("stack" | "replace")) = aura.row.stack_rule.as_deref() {
                merged.insert("max_stacks".into(), aura.row.max_stacks.into());
                merged.insert("stack_rule".into(), rule.into());
            }
        }
        merged.extend(object);
        object = merged;
    }
    object.insert("kind".into(), e.kind.clone().into());
    serde_json::from_value(Value::Object(object)).ok()
}

/// The skill, or `None` while it has no effects or any is a kind the realm lacks.
fn skill(g: &GenSkill, auras: &[GenAura]) -> Option<Skill> {
    let effects: Option<Vec<Effect>> = g.effects.iter().map(|e| effect(e, auras)).collect();
    let effects = effects.filter(|e| !e.is_empty())?;
    Some(Skill {
        key: g.key.clone(),
        spell_id: g.spell_id,
        learn_level: g.learn_level.max(1),
        power_cost: g.realm_cost,
        health_cost_pct_of_base_health: 0,
        cooldown_ms: g.realm_cooldown_ms,
        gcd_ms: g.realm_gcd_ms,
        target: g.target.unwrap_or(Target::Enemy),
        range_yd: [g.realm_range_min, g.realm_range_max],
        radius_yd: g.realm_radius_yd,
        max_targets: g.meta.shape.max_targets.unwrap_or(u32::MAX),
        requires_out_of_combat: g.requires_out_of_combat,
        requires_stealth: g.requires_stealth,
        requires_target_out_of_combat: g.requires_target_out_of_combat,
        requires_target_dodged: false,
        puts_in_combat_ms: 0,
        requires_target_health_pct: 0,
        extra_targets: 0,
        extra_target_radius_yd: 0.0,
        borrowed: None,
        effects,
        meta: g.meta.clone(),
    })
}

fn tab(calling: &str, g: &GenTab, auras: &[GenAura]) -> TalentTab {
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
        talents: g.talents.iter().filter_map(|t| talent(t, auras)).collect(),
    }
}

/// The talent, or `None` for a row with no ranks. A one-rank talent with a `target` is an active
/// one: its rank's effects are the skill it teaches, under the rank's spell id.
fn talent(g: &GenTalent, auras: &[GenAura]) -> Option<TalentDef> {
    let first = g.ranks.first()?;
    let teaches = match (&g.ranks[..], first.target) {
        ([rank], Some(target)) => skill(
            &GenSkill {
                key: rank.key.clone(),
                spell_id: rank.spell_id,
                learn_level: 0,
                realm_cost: rank.realm_cost,
                realm_cooldown_ms: rank.realm_cooldown_ms,
                realm_gcd_ms: rank.realm_gcd_ms,
                realm_range_min: rank.realm_range_min,
                realm_range_max: rank.realm_range_max,
                realm_radius_yd: rank.realm_radius_yd,
                target: Some(target),
                requires_out_of_combat: false,
                requires_stealth: false,
                requires_target_out_of_combat: false,
                client_visible: None,
                spellbook: None,
                effects: rank.effects.clone(),
                meta: rank.meta.clone(),
            },
            auras,
        ),
        _ => None,
    };
    Some(TalentDef {
        key: g.key.clone(),
        talent_id: first.talent_id,
        tier: g.seat.row,
        column: g.seat.column,
        rank_spells: g.ranks.iter().map(|r| r.spell_id).collect(),
        prereq: g.prereq.as_ref().map(|p| Prereq {
            key: p.key.clone(),
            rank: p.rank,
        }),
        exceptional: g.exceptional,
        effects: if teaches.is_some() {
            Vec::new()
        } else {
            talent_effects(&g.ranks)
        },
        skill: teaches,
        client_spellmods: client_spellmods(&g.ranks),
        rank_spellmods: g.ranks.iter().map(|r| r.meta.spellmods.clone()).collect(),
    })
}

/// The cells the client's spell-mod tables take for each rank's spellmod groups: one per family
/// bit a group reaches, its value at each rank (0 where a rank lacks the group). A group whose op
/// has no number is skipped; the build refuses one.
fn client_spellmods(ranks: &[GenRank]) -> Vec<ClientSpellMod> {
    let groups = ranks
        .iter()
        .map(|r| r.meta.spellmods.len())
        .max()
        .unwrap_or(0);
    let mut cells = Vec::new();
    for i in 0..groups {
        let Some(m) = ranks.iter().find_map(|r| r.meta.spellmods.get(i)) else {
            continue;
        };
        let Some(op) = callings::spell_mod_op(&m.op) else {
            continue;
        };
        let mask = m
            .family_mask
            .or(m.family_bit.map(|b| 1u64 << b))
            .unwrap_or(0);
        let per_rank: Vec<i32> = ranks
            .iter()
            .map(|r| r.meta.spellmods.get(i).map_or(0, |m| m.value as i32))
            .collect();
        for bit in (0..64u8).filter(|b| mask >> b & 1 == 1) {
            cells.push(ClientSpellMod {
                flat: m.kind == "flat",
                op,
                family_bit: bit,
                per_rank: per_rank.clone(),
            });
        }
    }
    cells
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
    if first.kind == "proc" {
        return proc(ranks, slot);
    }
    let mut object = first.params.clone();
    let Some(key) = ["pct", "amount"]
        .into_iter()
        .find(|k| object.contains_key(*k))
    else {
        // A switch with no size (`no_min_range`): on at every rank.
        object.insert("per_rank".into(), vec![1; ranks.len()].into());
        object.insert("kind".into(), first.kind.clone().into());
        return serde_json::from_value(Value::Object(object)).ok();
    };
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

/// A `proc`: its trigger from the first rank, its action (a heal percent, aura keys) per rank.
#[cfg(test)]
mod tests {
    use super::*;

    /// Every passive talent the sheet gives effects loads with them: a talent that silently
    /// loads as WIP, because one slot is a kind the realm lacks, does nothing when learned.
    #[test]
    fn every_passive_talent_with_effects_loads_them() {
        let all: Vec<Value> = serde_json::from_str(SERVER_SPELLS).expect("server_spells parses");
        let mut lost = Vec::new();
        for c in all.into_iter().filter(|c| c.get("tier").is_some()) {
            let g: GenCalling = serde_json::from_value(c).expect("a calling reads");
            for t in g.talent_tabs.iter().flat_map(|tab| &tab.talents) {
                let Some(first) = t.ranks.first() else {
                    continue;
                };
                if first.effects.is_empty() || (t.ranks.len() == 1 && first.target.is_some()) {
                    continue;
                }
                if talent_effects(&t.ranks).is_empty() {
                    let kinds: Vec<String> = (0..first.effects.len())
                        .filter(|s| talent_effect(&t.ranks, *s).is_none())
                        .map(|s| {
                            format!(
                                "{} {:?}",
                                first.effects[s].kind,
                                first.effects[s].params.keys().collect::<Vec<_>>()
                            )
                        })
                        .collect();
                    lost.push(format!("{}/{}: {}", g.key, t.key, kinds.join(", ")));
                }
            }
        }
        assert!(
            lost.is_empty(),
            "talents with no effects:\n{}",
            lost.join("\n")
        );
    }

    /// A talent row with no ranks is dropped; the tab still reads.
    #[test]
    fn a_talent_with_no_ranks_is_dropped() {
        let tab_json = serde_json::json!({
            "id": 1, "name": "Arms", "background": "WarriorArms",
            "talents": [
                {"key": "empty", "seat": {"row": 0, "column": 0}, "ranks": []},
                {"key": "one", "seat": {"row": 0, "column": 1},
                 "ranks": [{"spell_id": 5, "talent_id": 9}]},
            ],
        });
        let g: GenTab = serde_json::from_value(tab_json).expect("a tab reads");
        let tab = tab("Warrior", &g, &[]);
        let keys: Vec<&str> = tab.talents.iter().map(|t| t.key.as_str()).collect();
        assert_eq!(keys, ["one"]);
    }

    /// A `client_visible=no` skill loads as a hidden ability, found by its realm spell id, and is
    /// not in the spell book's skills.
    #[test]
    fn a_hidden_skill_loads_by_its_realm_spell_id() {
        let g: GenCalling = serde_json::from_value(serde_json::json!({
            "key": "test", "power": "rage", "stat_profile": "warrior", "item_class": "warrior", "health_pct": 100, "mana_pct": 100, "name": "Test", "tier": 0, "tier_cap": 10,
            "tier_bonus": {"strength": 0, "agility": 0, "stamina": 0, "intellect": 0, "spirit": 0},
            "shapeshift_form": 17,
            "skills": [
                {"key": "shown", "spell_id": 5, "modelled": true,
                 "effects": [{"kind": "charge", "params": {"speed_yd_s": 24}}]},
                {"key": "unseen", "spell_id": 900001, "client_visible": false,
                 "cast_kind": "instant", "requires": {"combat": "in"},
                 "effects": [{"kind": "charge", "params": {"speed_yd_s": 24}}]},
            ],
        }))
        .expect("a calling reads");
        let calling = calling(g);
        let keys: Vec<&str> = calling.skills.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(keys, ["shown"]);
        let hidden = calling.hidden_skill(900001).expect("found by realm id");
        assert_eq!(hidden.key, "unseen");
        assert_eq!(hidden.meta.shape.cast_kind.as_deref(), Some("instant"));
        assert_eq!(hidden.meta.gates.requires["combat"], "in");
        assert!(calling.hidden_skill(5).is_none());
    }

    fn auras() -> Vec<GenAura> {
        serde_json::from_value(serde_json::json!([
            {"key": "sleep", "spell_id": 1776, "duration_ms": 4000, "modelled": true,
             "break_on": {"damage": true},
             "effects": [{"kind": "incapacitate", "params": {}}]},
            {"key": "unbuilt", "spell_id": 1, "duration_ms": 1000, "modelled": false,
             "effects": [{"kind": "stun", "params": {"aura_spell_id": 1}}]},
        ]))
        .expect("auras read")
    }

    /// An aura row keeps its aura-only columns, `break_on` and `mechanic` among them.
    #[test]
    fn an_aura_keeps_its_row_columns() {
        let g: GenAura = serde_json::from_value(serde_json::json!({
            "key": "g", "spell_id": 1776, "duration_ms": 4000, "modelled": true,
            "mechanic": "incapacitate", "max_stacks": 2, "aura_positive": false,
            "break_on": {"damage": true},
            "effects": [{"kind": "mod_run_speed", "params": {"pct": 30}}],
        }))
        .expect("an aura reads");
        let a = aura(&g).expect("it loads");
        assert_eq!(a.row.mechanic.as_deref(), Some("incapacitate"));
        assert_eq!(a.row.max_stacks, 2);
        assert!(!a.row.aura_positive);
        assert_eq!(a.row.break_on["damage"], true);
    }

    fn skill_applying(key: &str, kind: &str) -> Option<Skill> {
        let g: GenSkill = serde_json::from_value(serde_json::json!({
            "key": "hit", "spell_id": 5,
            "effects": [{"kind": kind, "params": {"apply_aura": key}}],
        }))
        .expect("a skill reads");
        skill(&g, &auras())
    }

    /// `apply_aura` expands into the aura row's effect: its params, the row's duration, and
    /// `break_on: damage` as `break_on_damage`.
    #[test]
    fn an_applied_aura_fills_in_the_skills_effect() {
        let hit = skill_applying("sleep", "incapacitate").expect("it loads");
        assert_eq!(
            hit.effects,
            [Effect::Incapacitate {
                duration_ms: 4000,
                break_on_damage: true,
            }]
        );
    }

    /// An aura that is unknown, unmodelled or has no effect of the skill's kind drops the skill.
    #[test]
    fn an_applied_aura_that_cannot_load_drops_the_skill() {
        assert!(skill_applying("ghost", "incapacitate").is_none());
        assert!(skill_applying("unbuilt", "stun").is_none());
        assert!(skill_applying("sleep", "stun").is_none());
    }
}

fn proc(ranks: &[GenRank], slot: usize) -> Option<TalentEffect> {
    let mut object = ranks[0].effects.get(slot)?.params.clone();
    // Each rank's own.
    for key in ["cooldown_ms", "splash_pct"] {
        if object.contains_key(key) {
            let sizes: Option<Vec<Value>> = ranks
                .iter()
                .map(|r| r.effects.get(slot)?.params.get(key).cloned())
                .collect();
            object.insert(key.into(), Value::Array(sizes?));
        }
    }
    object.remove("heal_pct");
    object.remove("apply_aura");
    object.remove("chance");
    object.remove("chance_per_stack");
    object.remove("energize");
    object.remove("add_combo_points");
    object.remove("block_value_pct");
    object.remove("apply_aura_area");
    object.remove("refund_cost_pct");
    object.remove("energize_pct_of_base");
    object.remove("apply_aura_target");
    object.remove("heal_allies_pct");
    let mut ally_heals = Vec::new();
    let mut refunds = Vec::new();
    let mut base_energize = Vec::new();
    let mut target_auras = Vec::new();
    // The ring is always centred where the caster stood (`center=origin`, the build's only one).
    object.remove("center");
    let mut area_auras = Vec::new();
    let mut heals = Vec::new();
    let mut block_value = Vec::new();
    let mut auras = Vec::new();
    let mut chances = Vec::new();
    let mut chances_per_stack = Vec::new();
    let mut energize = Vec::new();
    let mut combo_points = Vec::new();
    for rank in ranks {
        let params = &rank.effects.get(slot)?.params;
        heals.push(params.get("heal_pct").cloned().unwrap_or(0.into()));
        chances.push(params.get("chance").cloned().unwrap_or(100.into()));
        chances_per_stack.push(
            params
                .get("chance_per_stack")
                .cloned()
                .unwrap_or(0.0.into()),
        );
        energize.push(params.get("energize").cloned().unwrap_or(0.into()));
        combo_points.push(params.get("add_combo_points").cloned().unwrap_or(0.into()));
        block_value.push(params.get("block_value_pct").cloned().unwrap_or(0.into()));
        auras.push(match params.get("apply_aura") {
            Some(Value::Array(keys)) => Value::Array(keys.clone()),
            Some(key) => Value::Array(vec![key.clone()]),
            None => Value::Array(Vec::new()),
        });
        refunds.push(params.get("refund_cost_pct").cloned().unwrap_or(0.into()));
        ally_heals.push(params.get("heal_allies_pct").cloned().unwrap_or(0.into()));
        base_energize.push(
            params
                .get("energize_pct_of_base")
                .cloned()
                .unwrap_or(0.into()),
        );
        target_auras.push(match params.get("apply_aura_target") {
            Some(Value::Array(keys)) => Value::Array(keys.clone()),
            Some(key) => Value::Array(vec![key.clone()]),
            None => Value::Array(Vec::new()),
        });
        area_auras.push(match params.get("apply_aura_area") {
            Some(Value::Array(keys)) => Value::Array(keys.clone()),
            Some(key) => Value::Array(vec![key.clone()]),
            None => Value::Array(Vec::new()),
        });
    }
    object.insert("area_auras".into(), Value::Array(area_auras));
    object.insert("refund_cost_pct".into(), Value::Array(refunds));
    object.insert("energize_pct_of_base".into(), Value::Array(base_energize));
    object.insert("target_auras".into(), Value::Array(target_auras));
    object.insert("heal_allies_pct".into(), Value::Array(ally_heals));
    object.insert("per_rank".into(), Value::Array(heals));
    object.insert("auras".into(), Value::Array(auras));
    object.insert("chances".into(), Value::Array(chances));
    object.insert("chances_per_stack".into(), Value::Array(chances_per_stack));
    object.insert("energize".into(), Value::Array(energize));
    object.insert("combo_points".into(), Value::Array(combo_points));
    object.insert("block_value_pct".into(), Value::Array(block_value));
    object.insert("kind".into(), "proc".into());
    serde_json::from_value(Value::Object(object)).ok()
}
