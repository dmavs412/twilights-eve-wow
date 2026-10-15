//! Twilight's Eve's rewrites of the spell catalog, made when the client loads its spell data
//! (ADR 0003). The realm's content names the spells; each function here is one kind of rewrite.
//! A Calling skill is one 1.12.1 spell with no ranks, so the spellbook, the tooltip and the learn
//! line must not show the borrowed spell's "Rank 1". A talent the realm does not model yet reads
//! "<Name> (WIP)". A spell a Calling borrows from another Class costs what the realm charges, in
//! the Calling's power, with no stance, as one of the character's Class family.

use super::SpellCatalog;

/// How a borrowed spell's row is rewritten (ADR 0003).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Borrowed {
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

    /// Rewrites every spell in `spells` as its [`Borrowed`] says: that power and flat cost and
    /// no other cost term, no stance requirement, and that family with its own flag alone (the
    /// lending family's flags would name the borrower's spells). An id the catalog lacks is
    /// skipped.
    pub fn borrow_spells(&mut self, spells: impl IntoIterator<Item = Borrowed>) {
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
            Borrowed {
                spell_id: 20271,
                power_type: 1,
                cost: 50,
                spell_family: 4,
                family_bit: 35,
            },
            Borrowed {
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
}
