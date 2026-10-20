//! Twilight's Eve's carry on the talent catalog (the fork's own file; its `mod` line is the one
//! upstream line it touches). A Calling's Talent Tree is its own TalentTab and Talent rows (ADR
//! 0011), carried in the compiled-in client patch, so the stock frame reads a catalog built from
//! them instead of the DBC tables.

use std::collections::HashMap;

use super::{Talent, TalentCatalog, TalentTabInfo, MAX_TALENT_RANK};

/// Every race's bit (`ChrRaces.dbc` 1..9), as the shipped tabs carry it.
const ALL_RACES: u32 = 511;
/// Every class's bit (`ChrClasses.dbc` 1..11): a Calling's tabs are the Calling's, whatever
/// Class it sits on.
const ALL_CLASSES: u32 = 2047;

impl TalentCatalog {
    /// `calling`'s Talent Tree from the client patch's `talent_tabs` and `talents`: its tabs in
    /// order, each talent on its 0-based seat (the sheet's are 1-based) with its rank spells, and
    /// a prerequisite as the talent id and 0-based rank it needs. `None` when the patch holds no
    /// tab of that Calling.
    pub fn from_dbc_patch(json: &str, calling: &str) -> Result<Option<Self>, serde_json::Error> {
        let patch: serde_json::Value = serde_json::from_str(json)?;
        let of_calling = |list: &str| -> Vec<serde_json::Value> {
            patch[list]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|e| e["calling"] == calling)
                .cloned()
                .collect()
        };
        let unsigned = |v: &serde_json::Value| v.as_u64().unwrap_or(0) as u32;
        let mut tab_rows = of_calling("talent_tabs");
        if tab_rows.is_empty() {
            return Ok(None);
        }
        tab_rows.sort_by_key(|t| unsigned(&t["order"]));
        let tabs = tab_rows
            .iter()
            .map(|t| TalentTabInfo {
                id: unsigned(&t["id"]),
                name: t["name"].as_str().unwrap_or_default().to_owned(),
                race_mask: ALL_RACES,
                class_mask: ALL_CLASSES,
                background: t["background"].as_str().unwrap_or_default().to_owned(),
            })
            .collect();
        let seats = of_calling("talents");
        let id_of = |key: &str| {
            seats
                .iter()
                .find(|s| s["key"] == key)
                .map_or(0, |s| unsigned(&s["talent_id"]))
        };
        let talents = seats
            .iter()
            .map(|s| {
                let mut ranks = [0; MAX_TALENT_RANK];
                for (slot, spell) in ranks
                    .iter_mut()
                    .zip(s["rank_spell_ids"].as_array().into_iter().flatten())
                {
                    *slot = unsigned(spell);
                }
                let prereq = &s["prereq"];
                Talent {
                    id: unsigned(&s["talent_id"]),
                    tab: unsigned(&s["tab_id"]),
                    row: unsigned(&s["row"]).saturating_sub(1),
                    col: unsigned(&s["column"]).saturating_sub(1),
                    ranks,
                    prereq_talent: prereq["key"].as_str().map_or(0, id_of),
                    prereq_rank: unsigned(&prereq["rank"]).saturating_sub(1),
                    required_spell: 0,
                    exceptional: s["exceptional"].as_bool().unwrap_or(false),
                }
            })
            .collect();
        Ok(Some(Self::from_parts(tabs, talents)))
    }

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

    /// The realm's half of the sheet, which carries every talent's seat as the sheet writes it.
    const SERVER_SPELLS: &str = include_str!("../../../../data/twilights-eve/server_spells.json");

    #[test]
    fn every_callings_tabs_and_talents_sit_where_the_sheet_seats_them() {
        let sheet: serde_json::Value = serde_json::from_str(SERVER_SPELLS).unwrap();
        let mut checked = 0;
        for calling in sheet.as_array().unwrap() {
            let key = calling["key"].as_str().unwrap();
            let catalog = TalentCatalog::from_dbc_patch(crate::CLIENT_DBC_PATCH, key)
                .unwrap()
                .unwrap_or_else(|| panic!("{key}: the patch has its tabs"));
            const HUMAN: u8 = 1;
            for class in [1, 4, 8] {
                let tabs = catalog.tabs_for_class(HUMAN, class);
                let mut want: Vec<&serde_json::Value> =
                    calling["talent_tabs"].as_array().unwrap().iter().collect();
                want.sort_by_key(|t| t["order"].as_u64());
                let names: Vec<&str> = tabs.iter().map(|t| t.name.as_str()).collect();
                let want_names: Vec<&str> =
                    want.iter().map(|t| t["name"].as_str().unwrap()).collect();
                assert_eq!(names, want_names, "{key}: tabs in order, for any Class");
                for (tab, want) in tabs.iter().zip(&want) {
                    assert_eq!(u64::from(tab.id), want["id"].as_u64().unwrap(), "{key}");
                    assert_eq!(tab.background, want["background"].as_str().unwrap());
                    let talents = catalog.talents_in_tab(tab.id);
                    let want_talents = want["talents"].as_array().unwrap();
                    assert_eq!(talents.len(), want_talents.len(), "{key}/{}", tab.name);
                    for (t, w) in talents.iter().zip(want_talents) {
                        let ranks = w["ranks"].as_array().unwrap();
                        let rank_ids: Vec<u32> = ranks
                            .iter()
                            .map(|r| r["spell_id"].as_u64().unwrap() as u32)
                            .collect();
                        assert!(t.id >= 1_000_000, "{key}: a talent id of the sheet's own");
                        assert_eq!(u64::from(t.id), ranks[0]["talent_id"].as_u64().unwrap());
                        assert_eq!(
                            (u64::from(t.row) + 1, u64::from(t.col) + 1),
                            (
                                w["seat"]["row"].as_u64().unwrap(),
                                w["seat"]["column"].as_u64().unwrap()
                            ),
                            "{key}/{}",
                            w["key"]
                        );
                        assert_eq!(&t.ranks[..rank_ids.len()], &rank_ids[..], "{}", w["key"]);
                        assert_eq!(t.max_rank() as usize, rank_ids.len());
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked >= 3 * 100, "{checked} talents checked");
    }

    #[test]
    fn a_prerequisite_names_its_talent_and_zero_based_rank() {
        let patch = r#"{"talent_tabs": [{"calling": "c", "key": "t", "id": 7, "name": "T",
            "background": "Twilight\\art", "order": 1}],
            "talents": [
              {"calling": "c", "key": "a", "tab_id": 7, "row": 1, "column": 2, "talent_id": 1000000,
               "rank_spell_ids": [1000010, 1000011]},
              {"calling": "c", "key": "b", "tab_id": 7, "row": 2, "column": 2, "talent_id": 1000001,
               "rank_spell_ids": [1000012], "exceptional": true, "prereq": {"key": "a", "rank": 2}},
              {"calling": "other", "key": "x", "tab_id": 7, "row": 1, "column": 1,
               "talent_id": 1000002, "rank_spell_ids": [1]}]}"#;
        let catalog = TalentCatalog::from_dbc_patch(patch, "c").unwrap().unwrap();
        assert_eq!(catalog.len(), 2, "another Calling's talent stays out");
        let b = catalog.talent(7, 1_000_001).unwrap();
        assert_eq!(
            (b.row, b.col, b.prereq_talent, b.prereq_rank),
            (1, 1, 1_000_000, 1)
        );
        assert!(b.exceptional);
        assert!(TalentCatalog::from_dbc_patch(patch, "nobody")
            .unwrap()
            .is_none());
    }
}
