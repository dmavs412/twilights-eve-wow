//! Twilight's Eve's rewrites of the spell catalog, made when the client loads its spell data
//! (ADR 0003). The realm's content names the spells; each function here is one kind of rewrite.
//! A Calling skill is one 1.12.1 spell with no ranks, so the spellbook, the tooltip and the learn
//! line must not show the borrowed spell's "Rank 1". A talent the realm does not model yet reads
//! "<Name> (WIP)". A spell a Calling borrows from another Class costs what the realm charges, in
//! the Calling's power, with no stance, as one of the character's Class family.

use super::SpellCatalog;

/// The values a borrowed spell's row is rewritten to (ADR 0003).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorrowedSpellRow {
    pub spell_id: u32,
    /// The Calling's `powerType`: 1, rage.
    pub power_type: u32,
    /// The flat cost in that power's client unit (tenths of rage).
    pub cost: u32,
    /// The character's Class's `SpellFamilyName`: 4, Warrior.
    pub spell_family: u32,
    /// The one `SpellFamilyFlags` bit it gets, which no spell of that family uses, so the realm's
    /// talent spell modifiers reach it and nothing else.
    pub family_bit: u8,
}

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

    /// Rewrites every spell in `spells` as its [`BorrowedSpellRow`] says: that power and flat cost
    /// and no other cost term, no stance requirement, and that family with its own flag alone
    /// (the lending family's flags would name the borrower's spells). An id the catalog lacks is
    /// skipped.
    pub fn borrow_spells(&mut self, spells: impl IntoIterator<Item = BorrowedSpellRow>) {
        for b in spells {
            if let Some(d) = self.spells.get_mut(&b.spell_id) {
                d.power_type = b.power_type;
                d.mana_cost = b.cost;
                d.mana_cost_pct = 0;
                d.mana_cost_per_level = 0;
                d.mana_per_second = 0;
                d.stances = 0;
                d.stances_not = 0;
                d.spell_family = b.spell_family;
                d.spell_family_flags = 1 << b.family_bit;
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

    #[test]
    fn a_borrowed_spell_costs_rage_needs_no_stance_and_joins_the_warrior_family() {
        // Judgement 20271 as Spell.dbc has it: 6% of base mana, Paladin family bit 23 (which the
        // Warrior family spends on its stances).
        let judgement = SpellDisplay {
            name: "Judgement".into(),
            power_type: 0,
            mana_cost: 0,
            mana_cost_pct: 6,
            mana_cost_per_level: 2,
            mana_per_second: 1,
            stances: 1 << 16,
            stances_not: 1 << 1,
            spell_family: 10,
            spell_family_flags: 1 << 23,
            ..SpellDisplay::default()
        };
        let charge = SpellDisplay {
            name: "Charge".into(),
            power_type: 1,
            stances: 1 << 16,
            spell_family: 4,
            spell_family_flags: 1,
            ..SpellDisplay::default()
        };
        let mut catalog =
            SpellCatalog::from_displays(HashMap::from([(20271, judgement), (6178, charge)]));
        catalog.borrow_spells([
            BorrowedSpellRow {
                spell_id: 20271,
                power_type: 1,
                cost: 50,
                spell_family: 4,
                family_bit: 35,
            },
            BorrowedSpellRow {
                spell_id: 999,
                power_type: 1,
                cost: 10,
                spell_family: 4,
                family_bit: 36,
            },
        ]);
        let j = catalog.get(20271).unwrap();
        assert_eq!(j.name, "Judgement", "the real name");
        assert_eq!((j.power_type, j.mana_cost), (1, 50), "5 rage, in tenths");
        assert_eq!(
            (j.mana_cost_pct, j.mana_cost_per_level, j.mana_per_second),
            (0, 0, 0),
            "no cost but the flat one"
        );
        assert_eq!((j.stances, j.stances_not), (0, 0), "any stance");
        assert_eq!(j.spell_family, 4);
        assert_eq!(j.spell_family_flags, 1 << 35, "only its own bit");
        let charge = catalog.get(6178).unwrap();
        assert_eq!(
            (charge.stances, charge.spell_family_flags),
            (1 << 16, 1),
            "Charge untouched"
        );
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
}
