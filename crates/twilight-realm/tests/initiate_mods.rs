//! The shipped Searing Ember aura row loads under the Initiate's spell id for it. Reads the
//! Calling's data.

mod common;

use common::*;

/// Combustion Engine looks the burn up by its aura row; the shipped row must load.
#[test]
fn the_shipped_ember_aura_loads_for_combustion_engine_to_find() {
    let ember = twilight_realm::callings::aura("aura_searing_ember").expect("the aura row loads");
    assert_eq!(ember.spell_id, spell("initiate", "aura_searing_ember"));
}
