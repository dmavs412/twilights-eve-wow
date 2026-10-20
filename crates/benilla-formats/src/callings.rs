//! The Callings as the client knows them (ADR 0011, #207): the `callings` table of the compiled-in
//! client patch, written by `build_spells.py` from `data/twilights-eve/callings.csv`. Everything
//! the stock client keyed on the unit's class byte reads the character's Calling from here; the
//! realm names each character's Calling by key (`SessionEvent::Callings`).

use std::sync::OnceLock;

use crate::CLIENT_DBC_PATCH;

/// One Calling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallingInfo {
    /// Its key, as the realm names it.
    pub key: String,
    /// What every screen calls it, in place of a class name.
    pub name: String,
    /// Its icon path.
    pub icon: String,
    /// Its spell book tab's name.
    pub spellbook: String,
    /// `Powers` of its bar: mana 0, rage 1, focus 2, energy 3.
    pub power_type: u8,
    /// The `ChrClasses.dbc` id of the vanilla class table it follows (its `stat_profile`): what the
    /// stock client's class-keyed tables read (skill lines, reputation bases, the relic slot).
    pub class: u8,
    /// The `ChrClasses.dbc` id whose `AllowableClass` bit an item it uses must carry.
    pub item_class: u8,
    /// The `SpellFamilyName` its spells are filed under, the family its spell mods reach.
    pub spell_family: u32,
    /// `ITEM_CLASS_WEAPON` and `ITEM_CLASS_ARMOR` subclass masks of what it may use.
    pub weapon_mask: u32,
    pub armor_mask: u32,
    /// Whether it wears a one-hander in the off hand too.
    pub dual_wield: bool,
}

impl CallingInfo {
    /// The `classToken` the stock UI keys colours and art on: its class table's.
    pub fn class_token(&self) -> &'static str {
        match self.class {
            2 => "PALADIN",
            3 => "HUNTER",
            4 => "ROGUE",
            5 => "PRIEST",
            7 => "SHAMAN",
            8 => "MAGE",
            9 => "WARLOCK",
            11 => "DRUID",
            _ => "WARRIOR",
        }
    }

    /// The `SMSG_SET_PROFICIENCY` mask of `item_class` (2 weapons, 4 armour); every subclass of
    /// any other class.
    pub fn proficiency(&self, item_class: u32) -> u32 {
        match item_class {
            2 => self.weapon_mask,
            // Subclass 0 (rings, necks, trinkets) needs no armour kind.
            4 => self.armor_mask | 1,
            _ => u32::MAX,
        }
    }
}

/// The Callings of a client patch's `callings` table.
pub fn parse_callings(json: &str) -> Result<Vec<CallingInfo>, serde_json::Error> {
    let patch: serde_json::Value = serde_json::from_str(json)?;
    let text = |v: &serde_json::Value| v.as_str().unwrap_or_default().to_owned();
    let number = |v: &serde_json::Value| v.as_u64().unwrap_or(0) as u32;
    Ok(patch["callings"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| CallingInfo {
            key: text(&c["key"]),
            name: text(&c["name"]),
            icon: text(&c["icon"]),
            spellbook: text(&c["spellbook"]),
            power_type: number(&c["power_type"]) as u8,
            class: number(&c["class"]) as u8,
            item_class: number(&c["item_class"]) as u8,
            spell_family: number(&c["spell_family"]),
            weapon_mask: number(&c["weapon_mask"]),
            armor_mask: number(&c["armor_mask"]),
            dual_wield: c["dual_wield"].as_bool().unwrap_or(false),
        })
        .collect())
}

/// The compiled-in patch's Calling with `key`.
pub fn calling(key: &str) -> Option<&'static CallingInfo> {
    static CALLINGS: OnceLock<Vec<CallingInfo>> = OnceLock::new();
    CALLINGS
        .get_or_init(|| parse_callings(CLIENT_DBC_PATCH).unwrap_or_default())
        .iter()
        .find(|c| c.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shipped_calling_is_in_the_patch_with_its_bar_and_tables() {
        let cutthroat = calling("cutthroat").expect("the Cutthroat");
        assert_eq!((cutthroat.power_type, cutthroat.class), (3, 4));
        assert_eq!(cutthroat.class_token(), "ROGUE");
        assert!(cutthroat.dual_wield);
        let initiate = calling("initiate").expect("the Initiate");
        assert_eq!(
            (initiate.power_type, initiate.name.as_str()),
            (0, "Initiate")
        );
        assert_eq!(
            initiate.spell_family, 4,
            "every Calling's spells are one family"
        );
        assert_eq!(initiate.item_class, 1);
        assert_eq!(initiate.proficiency(4) & (1 << 4), 0, "no plate");
        assert!(calling("nobody").is_none());
    }
}
