//! Twilight's Eve's carry on the spell catalog (the fork's own file; its `mod` line is the one
//! upstream line it touches). A Calling skill is one 1.12.1 spell with no ranks, so the spellbook,
//! the tooltip and the learn line must not show the borrowed spell's "Rank 1".

use super::SpellCatalog;

impl SpellCatalog {
    /// Blanks the rank text of every spell in `ids`; an id the catalog lacks is skipped.
    pub fn clear_ranks(&mut self, ids: impl IntoIterator<Item = u32>) {
        for id in ids {
            if let Some(display) = self.spells.get_mut(&id) {
                display.rank = None;
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
}
