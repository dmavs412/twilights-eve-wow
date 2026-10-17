//! The compiled-in client patch applied to the real `Spell.dbc` (ADR 0003): Fire Blast, a Mage
//! spell that costs mana in the stock data, costs the sheet's 10 rage.

use benilla_formats::{load_spell_catalog, open_chain, CLIENT_DBC_PATCH};

#[test]
fn fire_blast_costs_ten_rage_once_the_patch_is_applied() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut chain = open_chain(&data).expect("open vanilla patch chain");
    let mut catalog = load_spell_catalog(&mut chain).expect("load spell catalog");
    assert_eq!(
        catalog.get(2136).expect("Fire Blast").power_type,
        0,
        "mana in the stock data"
    );

    let rows = catalog
        .apply_dbc_patch(CLIENT_DBC_PATCH)
        .expect("patch parses");
    assert!(
        rows > 100,
        "the patch lists about 200 rows, {rows} found in Spell.dbc"
    );

    let fire_blast = catalog.get(2136).unwrap();
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

/// Zealot's Paladin spells, formerly rewritten from the realm's content: now the patch does it.
#[test]
fn zealots_borrowed_spells_cost_rage_as_warrior_spells() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut chain = open_chain(&data).expect("open vanilla patch chain");
    let mut catalog = load_spell_catalog(&mut chain).expect("load spell catalog");
    catalog
        .apply_dbc_patch(CLIENT_DBC_PATCH)
        .expect("patch parses");
    for (id, tenths, bit) in [
        (20375, 100, 33),
        (20305, 100, 34),
        (20271, 50, 35),
        (10308, 150, 36),
    ] {
        let d = catalog.get(id).expect("borrowed spell");
        assert_eq!(
            (
                d.power_type,
                d.mana_cost,
                d.spell_family,
                d.spell_family_flags,
                d.stances
            ),
            (1, tenths, 4, 1u64 << bit, 0),
            "spell {id}"
        );
    }
}

/// Every gate field the patch lists lands in its Spell.dbc column on the real catalog.
#[test]
fn every_patched_gate_field_lands_in_the_real_catalog() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut chain = open_chain(&data).expect("open vanilla patch chain");
    let mut catalog = load_spell_catalog(&mut chain).expect("load spell catalog");
    catalog
        .apply_dbc_patch(CLIENT_DBC_PATCH)
        .expect("patch parses");
    let patch: serde_json::Value = serde_json::from_str(CLIENT_DBC_PATCH).unwrap();
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
