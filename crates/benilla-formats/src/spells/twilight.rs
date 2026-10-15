//! Twilight's Eve's rewrites of the spell catalog, made when the client loads its spell data
//! (ADR 0003). The realm's content names the spells; each function here is one kind of rewrite.
//! A Calling skill is one 1.12.1 spell with no ranks, so the spellbook, the tooltip and the learn
//! line must not show the borrowed spell's "Rank 1". A talent the realm does not model yet reads
//! "<Name> (WIP)".

use super::SpellCatalog;

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
}
