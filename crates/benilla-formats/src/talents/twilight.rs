//! Twilight's Eve's carry on the talent catalog (the fork's own file; its `mod` line is the one
//! upstream line it touches). A Calling's Talent Tree re-seats real `Talent.dbc` rows on its own
//! tabs, so the stock frame needs a catalog built from that data instead of the DBC tables.

use std::collections::HashMap;

use super::{Talent, TalentCatalog, TalentTabInfo};

impl TalentCatalog {
    /// A catalog of exactly these tabs, in this order, and these talents, each listed under its
    /// `tab` in the order given (the order `GetTalentInfo` indexes).
    pub fn from_parts(tabs: Vec<TalentTabInfo>, talents: Vec<Talent>) -> Self {
        let mut by_tab: HashMap<u32, Vec<Talent>> = HashMap::new();
        for t in talents {
            by_tab.entry(t.tab).or_default().push(t);
        }
        Self { tabs, by_tab }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn talent(id: u32, tab: u32) -> Talent {
        Talent {
            id,
            tab,
            row: 0,
            col: 0,
            ranks: [id * 10, 0, 0, 0, 0],
            prereq_talent: 0,
            prereq_rank: 0,
            required_spell: 0,
            exceptional: false,
        }
    }

    #[test]
    fn a_built_catalog_serves_its_tabs_and_talents_in_the_order_given() {
        let tab = TalentTabInfo {
            id: 900,
            name: "Basic Warrior".into(),
            race_mask: 511,
            class_mask: 1,
            background: "WarriorArms".into(),
        };
        let catalog =
            TalentCatalog::from_parts(vec![tab], vec![talent(124, 900), talent(121, 900)]);
        const ORC: u8 = 2;
        const WARRIOR: u8 = 1;
        const MAGE: u8 = 8;
        let tabs = catalog.tabs_for_class(ORC, WARRIOR);
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0].name, "Basic Warrior");
        assert!(catalog.tabs_for_class(ORC, MAGE).is_empty());
        let ids: Vec<u32> = catalog.talents_in_tab(900).iter().map(|t| t.id).collect();
        assert_eq!(ids, [124, 121]);
        assert_eq!(catalog.talent(900, 121).map(|t| t.ranks[0]), Some(1210));
        assert_eq!(catalog.len(), 2);
    }
}
