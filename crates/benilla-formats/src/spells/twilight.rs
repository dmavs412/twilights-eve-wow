//! Twilight's Eve's rewrites of the spell catalog, made when the client loads its spell data
//! (ADR 0011). The compiled-in client patch inserts every Calling spell as a complete row of its
//! own, under an id no stock spell uses. A talent the realm does not model yet reads
//! "<Name> (WIP)".

use super::{
    EffectRow, SpellCatalog, SpellDisplay, ATTR_EX2_AUTO_REPEAT, ATTR_EX3_NORMAL_RANGED_ATTACK,
    ATTR_EX_CHANNELED, ATTR_NOT_IN_COMBAT, ATTR_ON_NEXT_SWING, ATTR_PASSIVE, ATTR_RANGED,
};

/// What a WIP talent's rank spells add to their names.
const WIP_SUFFIX: &str = " (WIP)";

impl SpellCatalog {
    /// Appends [`WIP_SUFFIX`] to the name of every spell in `ids`, once; an id the catalog lacks
    /// is skipped.
    pub fn mark_wip(&mut self, ids: impl IntoIterator<Item = u32>) {
        for id in ids {
            if let Some(display) = self.spells.get_mut(&id) {
                if !display.name.ends_with(WIP_SUFFIX) {
                    display.name.push_str(WIP_SUFFIX);
                }
            }
        }
    }

    /// Applies the compiled-in client patch (`client_dbc_patch.json`, ADR 0011) to the rows it
    /// lists. An id from [`FROM_SCRATCH_MIN_ID`] up is inserted as a complete row: name, icon,
    /// look, cost, cooldowns, stances, family, school, cast shape, targets and every effect from
    /// the entry. A field the patch lacks leaves the row's own value; a stock id the catalog lacks
    /// is skipped. Returns how many rows changed.
    pub fn apply_dbc_patch(&mut self, json: &str) -> Result<usize, serde_json::Error> {
        let patch: serde_json::Value = serde_json::from_str(json)?;
        let mut applied = 0;
        for entry in patch["spells"].as_array().into_iter().flatten() {
            let Some(id) = entry["spell_id"].as_u64().map(|id| id as u32) else {
                continue;
            };
            // A from-scratch row (ADR 0011) has no stock row to borrow: it starts from safe
            // defaults and the entry sets every field the client reads.
            if id >= FROM_SCRATCH_MIN_ID {
                self.spells
                    .entry(id)
                    .or_insert_with(|| from_scratch_display(id));
            }
            let Some(d) = self.spells.get_mut(&id) else {
                continue;
            };
            let f = &entry["fields"];
            let num = |key: &str| f[key].as_f64();
            if let Some(power) = num("power_type") {
                d.power_type = power as u32;
            }
            if let Some(cost) = num("cost") {
                // Rage is stored in tenths; every other power in whole units.
                let unit = if d.power_type == 1 { 10.0 } else { 1.0 };
                d.mana_cost = (cost * unit).round() as u32;
                d.mana_cost_pct = 0;
                d.mana_cost_per_level = 0;
                d.mana_per_second = 0;
            }
            // A percent-of-base or per-second cost replaces the flat one the same way.
            if let Some(pct) = num("cost_percent") {
                d.mana_cost = 0;
                d.mana_cost_pct = pct.round() as u32;
                d.mana_cost_per_level = 0;
                d.mana_per_second = 0;
            }
            if let Some(per_second) = num("cost_per_second") {
                let unit = if d.power_type == 1 { 10.0 } else { 1.0 };
                d.mana_cost = 0;
                d.mana_cost_pct = 0;
                d.mana_cost_per_level = 0;
                d.mana_per_second = (per_second * unit).round() as u32;
            }
            apply_gates(d, f);
            if let Some(ms) = num("cooldown_ms") {
                d.recovery_ms = ms as u32;
            }
            if let Some(ms) = num("gcd_ms") {
                d.start_recovery_ms = ms as u32;
            }
            if let Some(names) = f["stances"].as_array() {
                d.stances = names
                    .iter()
                    .filter_map(|n| n.as_str())
                    .filter_map(stance_form)
                    .fold(0, |mask, form| mask | 1 << (form - 1));
                d.stances_not = 0;
            }
            // The family a Calling's spell mods name its skills by.
            if let Some(family) = f["spell_family"].as_u64() {
                d.spell_family = family as u32;
            }
            if let Some(flags) = f["family_flags"].as_u64() {
                d.spell_family_flags = flags;
            }
            // The stock Spell.dbc rows the sheet's range, cast time and duration select.
            for (key, slot) in [
                ("range_index", &mut d.range_index),
                ("cast_time_index", &mut d.casting_time_index),
                ("duration_index", &mut d.duration_index),
            ] {
                if let Some(index) = f[key].as_u64() {
                    *slot = index as u32;
                }
            }
            // The sheet's name replaces the borrowed one; a WIP mark made before stays on it.
            if let Some(name) = f["name"].as_str().filter(|n| !n.is_empty()) {
                let wip = d.name.ends_with(WIP_SUFFIX);
                d.name = name.to_owned();
                if wip {
                    d.name.push_str(WIP_SUFFIX);
                }
            }
            if let Some(text) = f["description"].as_str().filter(|t| !t.is_empty()) {
                d.description = Some(text.to_owned());
            }
            if let Some(icon) = f["icon"].as_str().filter(|i| !i.is_empty()) {
                d.icon = Some(icon.to_owned());
            }
            if let Some(visual) = num("visual") {
                d.visual = visual as u32;
            }
            if let Some(speed) = num("speed") {
                d.speed = speed as f32;
            }
            if let Some(facing) = f["needs_facing"].as_bool() {
                d.needs_facing = facing;
            }
            // A from-scratch row's effects, as many as the sheet has; slots past three are the
            // display's extras.
            for (slot, effect) in f["dbc_effects"]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
            {
                d.set_effect_row(slot, effect_row(effect));
            }
            // Every effect takes the row's implicit targets: the cast asks for a unit, a ground
            // spot or nothing by them.
            if let Some(targets) = f["implicit_targets"].as_array() {
                let at = |i: usize| targets.get(i).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                d.implicit_target_a1 = at(0);
                d.effect_implicit_target_a = [at(0); 3];
                d.effect_implicit_target_b = [at(1); 3];
            }
            applied += 1;
        }
        Ok(applied)
    }
}

/// The lowest id of a from-scratch row (ADR 0011); stock Spell.dbc ids stay far below it.
const FROM_SCRATCH_MIN_ID: u32 = 1_000_000;

/// A from-scratch row before its patch entry: instant, on the global cooldown, no stock
/// requirements.
fn from_scratch_display(id: u32) -> SpellDisplay {
    SpellDisplay {
        id,
        casting_time_index: 1,
        start_recovery_category: 133,
        start_recovery_ms: 1500,
        proc_chance: 101,
        ..SpellDisplay::default()
    }
}

/// One `dbc_effects` entry: the effect and aura numbers, then the roll as `apply_dbc_patch`
/// writes a sheet damage number (a one-sided die on `base - 1`, plus the per-level term).
fn effect_row(effect: &serde_json::Value) -> EffectRow {
    let unsigned = |key: &str| effect[key].as_u64().unwrap_or(0) as u32;
    EffectRow {
        effect: unsigned("effect"),
        apply_aura: unsigned("aura"),
        amplitude: unsigned("amplitude_ms"),
        mechanic: unsigned("mechanic"),
        misc_value: effect["misc_value"].as_i64().unwrap_or(0) as i32,
        implicit_target_a: unsigned("target_a"),
        base_points: effect["base"].as_f64().unwrap_or(0.0).round() as i32 - 1,
        base_dice: 1,
        die_sides: 1,
        real_points_per_level: effect["per_level"].as_f64().unwrap_or(0.0) as f32,
        ..EffectRow::default()
    }
}

/// The compiled-in client patch the build script wrote from the master CSV.
pub const CLIENT_DBC_PATCH: &str =
    include_str!("../../../../data/twilights-eve/client_dbc_patch.json");

/// Writes the patch's gate, cast-shape and aura columns into their `Spell.dbc` columns: equipped
/// item class and masks, category, school, prevention, aura states, `Targets`, interrupt flags,
/// the channeled, on-next-swing, auto-repeat, passive, ranged and out-of-combat attribute bits,
/// and the dispel type, mechanic and aura description. A key the patch lacks leaves the row's
/// own value.
fn apply_gates(d: &mut SpellDisplay, f: &serde_json::Value) {
    let unsigned = |key: &str| f[key].as_u64().map(|v| v as u32);
    if let Some(class) = f["equipped_item_class"].as_i64() {
        d.equipped_item_class = class as i32;
    }
    for (key, slot) in [
        ("equipped_subclass_mask", &mut d.equipped_item_subclass_mask),
        (
            "equipped_inventory_mask",
            &mut d.equipped_item_inventory_type_mask,
        ),
        ("category", &mut d.category),
        ("category_cooldown_ms", &mut d.category_recovery_ms),
        ("school", &mut d.school),
        ("prevention_type", &mut d.prevention_type),
        ("caster_aura_state", &mut d.caster_aura_state),
        ("target_aura_state", &mut d.target_aura_state),
        ("interrupt_flags", &mut d.interrupt_flags),
        ("channel_interrupt_flags", &mut d.channel_interrupt_flags),
        ("dispel_type", &mut d.dispel),
        ("mechanic", &mut d.mechanic),
        ("targets", &mut d.targets),
    ] {
        if let Some(v) = unsigned(key) {
            *slot = v;
        }
    }
    let set = |word: &mut u32, key: &str, bits: u32| match f[key].as_bool() {
        Some(true) => *word |= bits,
        Some(false) => *word &= !bits,
        None => {}
    };
    set(&mut d.attributes_ex, "channeled", ATTR_EX_CHANNELED);
    set(&mut d.attributes_ex2, "auto_repeat", ATTR_EX2_AUTO_REPEAT);
    for (key, bits) in [
        ("on_next_swing", ATTR_ON_NEXT_SWING),
        ("passive", ATTR_PASSIVE),
        ("ranged", ATTR_RANGED),
        ("not_in_combat", ATTR_NOT_IN_COMBAT),
    ] {
        set(&mut d.attributes, key, bits);
    }
    d.passive = d.attributes & ATTR_PASSIVE != 0;
    // An auto-repeat shot floats white damage, as Auto Shot does.
    if f["ranged"].as_bool() == Some(true) && f["auto_repeat"].as_bool() == Some(true) {
        d.attributes_ex3 |= ATTR_EX3_NORMAL_RANGED_ATTACK;
    }
    if let Some(text) = f["aura_description"].as_str().filter(|t| !t.is_empty()) {
        d.aura_description = Some(text.to_owned());
    }
}

/// A warrior stance by its sheet name: its `SpellShapeshiftForm.dbc` id.
fn stance_form(name: &str) -> Option<u32> {
    match name {
        "Battle Stance" => Some(17),
        "Defensive Stance" => Some(18),
        "Berserker Stance" => Some(19),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::SpellDisplay;

    #[test]
    fn mark_wip_names_only_the_named_spells_once() {
        let named = |name: &str| SpellDisplay {
            name: name.into(),
            rank: Some("Rank 1".into()),
            ..SpellDisplay::default()
        };
        let mut catalog = SpellCatalog::from_displays(HashMap::from([
            (12300, named("Iron Will")),
            (12297, named("Anticipation")),
        ]));
        catalog.mark_wip([12300, 999]);
        catalog.mark_wip([12300]);
        let iron_will = catalog.get(12300).unwrap();
        assert_eq!(iron_will.name, "Iron Will (WIP)");
        assert_eq!(iron_will.rank.as_deref(), Some("Rank 1"));
        assert_eq!(catalog.get(12297).unwrap().name, "Anticipation");
    }

    fn patch_catalog() -> SpellCatalog {
        let stock = |name: &str| SpellDisplay {
            name: name.into(),
            power_type: 0,
            mana_cost_pct: 6,
            stances: 1 << 16,
            description: Some("Borrowed text.".into()),
            ..SpellDisplay::default()
        };
        SpellCatalog::from_displays(HashMap::from([
            (2136, stock("Fire Blast")),
            (78, stock("Heroic Strike")),
            (12300, stock("Talent")),
        ]))
    }

    const PATCH: &str = r#"{"spells": [
        {"spell_id": 2136, "fields": {"cost": 10, "power_type": 1, "cooldown_ms": 6000,
         "gcd_ms": 1500, "spell_family": 4, "family_flags": 137438953472, "stances": [],
         "name": "Scorch", "description": "Blasts the enemy."}},
        {"spell_id": 78, "fields": {"cost": 15, "power_type": 1, "stances": ["Battle Stance", "Berserker Stance"],
         "description": ""}},
        {"spell_id": 999999, "fields": {"cost": 1}}
    ], "talents": []}"#;

    #[test]
    fn a_patched_row_costs_rage_and_reads_the_sheet() {
        let mut catalog = patch_catalog();
        catalog.apply_dbc_patch(PATCH).unwrap();
        let fb = catalog.get(2136).unwrap();
        assert_eq!(
            (fb.power_type, fb.mana_cost),
            (1, 100),
            "10 rage, in tenths"
        );
        assert_eq!(
            (fb.mana_cost_pct, fb.stances),
            (0, 0),
            "no stock cost, any stance"
        );
        assert_eq!((fb.recovery_ms, fb.start_recovery_ms), (6000, 1500));
        assert_eq!((fb.spell_family, fb.spell_family_flags), (4, 1 << 37));
        assert_eq!(fb.description.as_deref(), Some("Blasts the enemy."));
        assert_eq!(fb.name, "Scorch", "the sheet's name replaces the row's");
    }

    #[test]
    fn targets_and_attribute_bits_land_on_a_from_scratch_row() {
        let mut catalog = patch_catalog();
        catalog
            .apply_dbc_patch(
                r#"{"spells": [
                {"spell_id": 1000003, "fields": {"implicit_targets": [6, 0], "ranged": true,
                 "auto_repeat": true, "speed": 40, "dbc_effects": [{"effect": 58}]}},
                {"spell_id": 1000004, "fields": {"implicit_targets": [16, 28], "targets": 64,
                 "not_in_combat": true}},
                {"spell_id": 1000005, "fields": {"implicit_targets": [1, 0], "passive": true}}]}"#,
            )
            .unwrap();
        let shot = catalog.get(1_000_003).unwrap();
        assert_eq!(shot.implicit_target_a1, 6);
        assert_eq!(shot.effect_implicit_target_a, [6; 3]);
        assert!(shot.ranged_attack() && shot.ranged_icon_substitution());
        assert!(
            shot.melee_white_damage(),
            "an auto-repeat shot floats white damage"
        );
        assert_eq!(shot.speed, 40.0);
        let ground = catalog.get(1_000_004).unwrap();
        assert_eq!((ground.implicit_target_a1, ground.targets), (16, 64));
        assert_eq!(ground.effect_implicit_target_b, [28; 3]);
        assert_ne!(ground.attributes & ATTR_NOT_IN_COMBAT, 0);
        let talent = catalog.get(1_000_005).unwrap();
        assert!(talent.passive && !talent.ranged_attack());
    }

    #[test]
    fn an_empty_description_keeps_the_borrowed_text_and_stances_become_a_mask() {
        let mut catalog = patch_catalog();
        catalog.apply_dbc_patch(PATCH).unwrap();
        let hs = catalog.get(78).unwrap();
        assert_eq!(hs.description.as_deref(), Some("Borrowed text."));
        assert_eq!(hs.stances, (1 << 16) | (1 << 18));
        assert_eq!(
            catalog.get(12300).unwrap().mana_cost_pct,
            6,
            "unlisted row untouched"
        );
    }

    #[test]
    fn row_indexes_overwrite_range_cast_time_and_duration() {
        let mut catalog = patch_catalog();
        catalog
            .apply_dbc_patch(
                r#"{"spells": [{"spell_id": 2136, "fields":
                {"range_index": 3, "cast_time_index": 1, "duration_index": 21}}]}"#,
            )
            .unwrap();
        let fb = catalog.get(2136).unwrap();
        assert_eq!(
            (fb.range_index, fb.casting_time_index, fb.duration_index),
            (3, 1, 21)
        );
    }

    #[test]
    fn gate_columns_land_in_their_spell_dbc_fields() {
        let mut catalog = patch_catalog();
        catalog
            .apply_dbc_patch(
                r#"{"spells": [{"spell_id": 2136, "fields": {
                "equipped_item_class": 2, "equipped_subclass_mask": 262156,
                "equipped_inventory_mask": 8192, "category": 10003,
                "category_cooldown_ms": 30000, "school": 4, "prevention_type": 1,
                "caster_aura_state": 2, "target_aura_state": 5,
                "interrupt_flags": 9, "channel_interrupt_flags": 17,
                "dispel_type": 1, "mechanic": 7, "aura_description": "Slowed.",
                "channeled": true, "on_next_swing": true, "auto_repeat": true}}]}"#,
            )
            .unwrap();
        let d = catalog.get(2136).unwrap();
        assert_eq!(
            (
                d.equipped_item_class,
                d.equipped_item_subclass_mask,
                d.equipped_item_inventory_type_mask
            ),
            (2, 262156, 8192)
        );
        assert_eq!((d.category, d.category_recovery_ms), (10003, 30000));
        assert_eq!((d.school, d.prevention_type), (4, 1));
        assert_eq!((d.caster_aura_state, d.target_aura_state), (2, 5));
        assert_eq!((d.interrupt_flags, d.channel_interrupt_flags), (9, 17));
        assert_eq!((d.dispel, d.mechanic), (1, 7));
        assert_eq!(d.aura_description.as_deref(), Some("Slowed."));
        assert_eq!(d.attributes_ex & 0x44, 0x44, "channeled");
        assert_eq!(d.attributes & 0x404, 0x404, "on next swing");
        assert_eq!(d.attributes_ex2 & 0x20, 0x20, "auto repeat");
    }

    #[test]
    fn percent_and_per_second_costs_replace_the_flat_cost() {
        let mut catalog = patch_catalog();
        catalog
            .apply_dbc_patch(
                r#"{"spells": [
                {"spell_id": 2136, "fields": {"power_type": 0, "cost_percent": 8}},
                {"spell_id": 78, "fields": {"power_type": 1, "cost_per_second": 2}},
                {"spell_id": 12300, "fields": {"power_type": 4294967294}}]}"#,
            )
            .unwrap();
        let fb = catalog.get(2136).unwrap();
        assert_eq!((fb.mana_cost, fb.mana_cost_pct), (0, 8));
        let hs = catalog.get(78).unwrap();
        assert_eq!(
            (hs.mana_cost_pct, hs.mana_per_second),
            (0, 20),
            "rage in tenths"
        );
        assert_eq!(catalog.get(12300).unwrap().power_type, 0xFFFF_FFFE);
    }

    #[test]
    fn a_patch_without_gate_keys_leaves_the_row_alone() {
        let mut catalog = patch_catalog();
        catalog
            .apply_dbc_patch(r#"{"spells": [{"spell_id": 78, "fields": {"cost": 15}}]}"#)
            .unwrap();
        let hs = catalog.get(78).unwrap();
        assert_eq!(hs.equipped_item_class, -1);
        assert_eq!((hs.school, hs.attributes, hs.attributes_ex2), (0, 0, 0));
    }

    #[test]
    fn a_from_scratch_entry_adds_a_complete_row_with_four_effects() {
        let mut catalog = patch_catalog();
        let applied = catalog
            .apply_dbc_patch(
                r#"{"spells": [{"spell_id": 1000001, "fields": {
                "name": "Ember Lash", "power_type": 0, "cost": 30, "cooldown_ms": 8000,
                "icon": "Interface\\Icons\\Twilight\\ember_lash", "visual": 4321,
                "school": 2, "channeled": true, "needs_facing": true, "description": "Lashes.",
                "dbc_effects": [
                    {"effect": 2, "base": 10, "per_level": 1.5},
                    {"effect": 6, "aura": 3, "base": 4, "amplitude_ms": 3000},
                    {"effect": 3, "base": 1},
                    {"effect": 3, "base": 7, "per_level": 0.5}]}}]}"#,
            )
            .unwrap();
        assert_eq!(applied, 1);
        let d = catalog.get(1_000_001).expect("the patch adds the id");
        assert_eq!(d.id, 1_000_001);
        assert_eq!(d.name, "Ember Lash");
        assert_eq!((d.power_type, d.mana_cost, d.recovery_ms), (0, 30, 8000));
        assert_eq!(
            d.icon.as_deref(),
            Some("Interface\\Icons\\Twilight\\ember_lash")
        );
        assert_eq!((d.visual, d.school, d.needs_facing), (4321, 2, true));
        assert_eq!(d.effect_count(), 4);
        assert_eq!(d.effect_row(0).effect, 2);
        assert_eq!(d.effect_row(1).apply_aura, 3);
        let fourth = d.effect_row(3);
        assert_eq!((fourth.effect, fourth.base_points), (3, 6));
        assert_eq!(fourth.real_points_per_level, 0.5);
        assert!(d.equipped_item_class == -1, "no item requirement");
    }

    #[test]
    fn effect_slots_past_the_last_read_as_empty() {
        let mut catalog = patch_catalog();
        catalog
            .apply_dbc_patch(
                r#"{"spells": [{"spell_id": 1000002, "fields": {"name": "Four",
                "dbc_effects": [{"effect": 3, "base": 1}, {"effect": 3, "base": 2},
                                {"effect": 3, "base": 3}, {"effect": 3, "base": 4}]}}]}"#,
            )
            .unwrap();
        let d = catalog.get(1_000_002).unwrap();
        assert_eq!(d.effect_row(3).base_points, 3);
        assert_eq!(d.effect_row(9), EffectRow::default());
    }
}
