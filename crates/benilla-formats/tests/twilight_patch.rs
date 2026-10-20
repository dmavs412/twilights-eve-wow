//! The compiled-in client patch applied to the real `Spell.dbc` (ADR 0011): every Calling spell is
//! a row of its own, inserted beside the stock ones, never a rewrite of one.

use benilla_formats::{load_spell_catalog, open_chain, SpellCatalog, CLIENT_DBC_PATCH};

/// The real catalog with the patch applied, and the patch; `None` with no WoW install.
fn patched() -> Option<(SpellCatalog, serde_json::Value)> {
    let data = benilla_formats::wow_data_or_skip!(None);
    let mut chain = open_chain(&data).expect("open vanilla patch chain");
    let mut catalog = load_spell_catalog(&mut chain).expect("load spell catalog");
    let stock = catalog.len();
    let rows = catalog
        .apply_dbc_patch(CLIENT_DBC_PATCH)
        .expect("patch parses");
    let patch: serde_json::Value = serde_json::from_str(CLIENT_DBC_PATCH).unwrap();
    assert_eq!(rows, patch["spells"].as_array().unwrap().len());
    assert_eq!(catalog.len(), stock + rows, "every row is a new one");
    Some((catalog, patch))
}

/// The patch entry of a Calling's spell, found by its sheet key.
fn entry<'a>(patch: &'a serde_json::Value, calling: &str, key: &str) -> &'a serde_json::Value {
    patch["spells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["calling"] == calling && e["key"] == key)
        .unwrap_or_else(|| panic!("{calling}/{key} in the patch"))
}

#[test]
fn fire_blast_is_its_own_row_and_costs_ten_rage() {
    let Some((catalog, patch)) = patched() else {
        return;
    };
    let id = entry(&patch, "basic_warrior", "fire_blast")["spell_id"]
        .as_u64()
        .unwrap() as u32;
    assert!(id >= 1_000_000);
    let fire_blast = catalog.get(id).unwrap();
    assert_eq!(
        (fire_blast.power_type, fire_blast.mana_cost),
        (1, 100),
        "10 rage, in tenths"
    );
    assert_eq!(fire_blast.stances, 0);
    let text = fire_blast.description.as_deref().unwrap_or("");
    assert!(
        text.starts_with("Blasts the enemy with fire damage"),
        "sheet text, got {text:?}"
    );
}

/// Zealot's spells, once borrowed from the Paladin, cost rage and carry a family bit of their own.
#[test]
fn zealots_spells_cost_rage_as_warrior_spells() {
    let Some((catalog, patch)) = patched() else {
        return;
    };
    for (key, tenths) in [
        ("seal_of_command", 100),
        ("seal_of_the_crusader", 100),
        ("judgement", 50),
        ("hammer_of_justice", 150),
    ] {
        let e = entry(&patch, "zealot", key);
        let d = catalog.get(e["spell_id"].as_u64().unwrap() as u32).unwrap();
        let flags = e["fields"]["family_flags"].as_u64().unwrap();
        assert_eq!(flags.count_ones(), 1, "{key}: one bit");
        assert!(flags >= 1 << 33, "{key}: above every stock Warrior bit");
        assert_eq!(
            (
                d.power_type,
                d.mana_cost,
                d.spell_family,
                d.spell_family_flags,
                d.stances
            ),
            (1, tenths, 4, flags, 0),
            "{key}"
        );
    }
}

/// Every gate field the patch lists lands in its Spell.dbc column on the real catalog.
#[test]
fn every_patched_gate_field_lands_in_the_real_catalog() {
    let Some((catalog, patch)) = patched() else {
        return;
    };
    let mut checked = 0;
    for entry in patch["spells"].as_array().unwrap() {
        let f = &entry["fields"];
        let Some(d) = catalog.get(entry["spell_id"].as_u64().unwrap() as u32) else {
            continue;
        };
        let id = d.id;
        for (key, got) in [
            ("school", d.school),
            ("prevention_type", d.prevention_type),
            ("category", d.category),
            ("category_cooldown_ms", d.category_recovery_ms),
            ("equipped_subclass_mask", d.equipped_item_subclass_mask),
            (
                "equipped_inventory_mask",
                d.equipped_item_inventory_type_mask,
            ),
            ("caster_aura_state", d.caster_aura_state),
            ("target_aura_state", d.target_aura_state),
            ("interrupt_flags", d.interrupt_flags),
            ("channel_interrupt_flags", d.channel_interrupt_flags),
            ("dispel_type", d.dispel),
            ("mechanic", d.mechanic),
        ] {
            if let Some(want) = f[key].as_u64() {
                assert_eq!(got, want as u32, "spell {id} {key}");
                checked += 1;
            }
        }
        if f["on_next_swing"] == true {
            assert!(d.on_next_swing(), "spell {id} on_next_swing");
        }
        if f["auto_repeat"] == true {
            assert!(d.auto_repeat(), "spell {id} auto_repeat");
        }
    }
    assert!(checked > 100, "only {checked} gate fields checked");
}
