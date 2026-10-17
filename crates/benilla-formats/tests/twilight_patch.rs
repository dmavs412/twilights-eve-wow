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
