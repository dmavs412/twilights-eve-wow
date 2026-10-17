//! Twilight's Eve's rewrites of the spell catalog, made when the client loads its spell data
//! (ADR 0003). The realm's content names the spells; each function here is one kind of rewrite.
//! A Calling skill is one 1.12.1 spell with no ranks, so the spellbook, the tooltip and the learn
//! line must not show the borrowed spell's "Rank 1". A talent the realm does not model yet reads
//! "<Name> (WIP)". The compiled-in client patch rewrites the rows it lists, among them the
//! spells a Calling borrows from another Class.

use super::{SpellCatalog, SpellDisplay};

/// What a WIP talent's rank spells add to their names.
const WIP_SUFFIX: &str = " (WIP)";

impl SpellCatalog {
    /// Blanks the rank text of every spell in `ids`; an id the catalog lacks is skipped.
    pub fn clear_ranks(&mut self, ids: impl IntoIterator<Item = u32>) {
        for id in ids {
            if let Some(display) = self.spells.get_mut(&id) {
                display.rank = None;
            }
        }
    }

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

    /// Applies the compiled-in client patch (`client_dbc_patch.json`, ADR 0003) to the rows it
    /// lists: power type, cost, cooldown, global cooldown, stances, the borrowed family, and the
    /// sheet's description (an empty one keeps the borrowed text). A field the patch lacks leaves
    /// the row's own value; an id the catalog lacks is skipped. Returns how many rows changed.
    pub fn apply_dbc_patch(&mut self, json: &str) -> Result<usize, serde_json::Error> {
        let patch: serde_json::Value = serde_json::from_str(json)?;
        let mut applied = 0;
        for entry in patch["spells"].as_array().into_iter().flatten() {
            let Some(d) = entry["spell_id"]
                .as_u64()
                .and_then(|id| self.spells.get_mut(&(id as u32)))
            else {
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
            if f["borrowed_from_class"].is_number() {
                d.stances = 0;
                d.stances_not = 0;
                d.spell_family = CLASS_SPELL_FAMILY;
                if let Some(bit) = num("borrowed_family_bit") {
                    d.spell_family_flags = 1 << bit as u32;
                }
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
            // The stock effects stay; a sheet effect only sets the numbers of the stock effect
            // it lines up with, so `$s1` and the spell mods read the sheet's damage.
            for effect in f["effects"].as_array().into_iter().flatten() {
                overwrite_damage(d, effect);
            }
            if let Some(text) = f["description"].as_str().filter(|t| !t.is_empty()) {
                d.description = Some(text.to_owned());
            }
            applied += 1;
        }
        Ok(applied)
    }
}

/// `SPELLFAMILY_WARRIOR`: every Calling is of the Warrior line.
const CLASS_SPELL_FAMILY: u32 = 4;

/// The compiled-in client patch the build script wrote from the master CSV.
pub const CLIENT_DBC_PATCH: &str =
    include_str!("../../../../data/twilights-eve/client_dbc_patch.json");

/// `SPELL_EFFECT_SCHOOL_DAMAGE`.
const EFFECT_SCHOOL_DAMAGE: u32 = 2;
/// `SPELL_EFFECT_WEAPON_DAMAGE_NOSCHOOL`, `WEAPON_DAMAGE` and `NORMALIZED_WEAPON_DMG`: the flat
/// bonus on top of a weapon swing.
const EFFECTS_WEAPON_BONUS: [u32; 3] = [17, 58, 121];

/// Sets the first stock effect a sheet damage effect lines up with (`school_damage` with a school
/// damage effect, `weapon_strike` with a weapon bonus) to the sheet's number: a one-sided die on
/// `base - 1`, and the per-level term. No stock effect of that kind leaves the row alone.
fn overwrite_damage(d: &mut SpellDisplay, effect: &serde_json::Value) {
    let (stock, formula): (&[u32], _) = match effect["kind"].as_str() {
        Some("school_damage") => (&[EFFECT_SCHOOL_DAMAGE], &effect["params"]["amount"]),
        Some("weapon_strike") => (&EFFECTS_WEAPON_BONUS, &effect["params"]["bonus"]),
        _ => return,
    };
    let (Some(base), Some(slot)) = (
        formula["base"].as_f64(),
        d.effects.iter().position(|e| stock.contains(e)),
    ) else {
        return;
    };
    d.effect_base_points[slot] = base.round() as i32 - 1;
    d.effect_base_dice[slot] = 1;
    d.effect_die_sides[slot] = 1;
    d.effect_dice_per_level[slot] = 0;
    d.effect_real_points_per_level[slot] = formula["per_level"].as_f64().unwrap_or(0.0) as f32;
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
    fn clear_ranks_blanks_only_the_named_spells() {
        let ranked = |rank: &str| SpellDisplay {
            rank: Some(rank.into()),
            ..SpellDisplay::default()
        };
        let mut catalog = SpellCatalog::from_displays(HashMap::from([
            (78, ranked("Rank 1")),
            (284, ranked("Rank 2")),
        ]));
        catalog.clear_ranks([78, 999]);
        assert_eq!(catalog.get(78).unwrap().rank, None);
        assert_eq!(catalog.get(284).unwrap().rank.as_deref(), Some("Rank 2"));
    }

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
         "gcd_ms": 1500, "borrowed_from_class": 8, "borrowed_family_bit": 37,
         "description": "Blasts the enemy."}},
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

    /// Fire Blast as Spell.dbc has it: effect 1 is `SCHOOL_DAMAGE` (2), 8 to 10 fire damage.
    fn stock_fire_blast() -> SpellDisplay {
        SpellDisplay {
            effects: [2, 0, 0],
            effect_base_points: [7, 0, 0],
            effect_base_dice: [1, 0, 0],
            effect_die_sides: [3, 0, 0],
            ..SpellDisplay::default()
        }
    }

    #[test]
    fn the_sheets_damage_overwrites_the_stock_effect_it_lines_up_with() {
        let mut catalog = SpellCatalog::from_displays(HashMap::from([
            (2136, stock_fire_blast()),
            (
                78,
                SpellDisplay {
                    effects: [17, 0, 0],
                    ..SpellDisplay::default()
                },
            ),
            (
                1776,
                SpellDisplay {
                    effects: [6, 0, 0],
                    effect_base_points: [4, 0, 0],
                    ..SpellDisplay::default()
                },
            ),
        ]));
        catalog
            .apply_dbc_patch(
                r#"{"spells": [
                {"spell_id": 2136, "fields": {"effects": [{"kind": "school_damage",
                    "params": {"school": "fire", "amount": {"base": 20, "per_level": 2.5}}}]}},
                {"spell_id": 78, "fields": {"effects": [{"kind": "weapon_strike",
                    "params": {"bonus": {"base": 9.5, "per_level": 1.4}}}]}},
                {"spell_id": 1776, "fields": {"effects": [{"kind": "school_damage",
                    "params": {"amount": {"base": 10, "per_level": 1.5}}}]}}
            ]}"#,
            )
            .unwrap();
        let fb = catalog.get(2136).unwrap();
        // Value = base points + one one-sided die: 19 + 1 = 20, plus 2.5 per level.
        assert_eq!(
            (
                fb.effect_base_points[0],
                fb.effect_base_dice[0],
                fb.effect_die_sides[0]
            ),
            (19, 1, 1)
        );
        assert_eq!(fb.effect_real_points_per_level[0], 2.5);
        assert_eq!(catalog.get(78).unwrap().effect_base_points[0], 9);
        let gouge = catalog.get(1776).unwrap();
        assert_eq!(
            gouge.effect_base_points[0], 4,
            "no stock damage effect: left alone"
        );
    }
}
