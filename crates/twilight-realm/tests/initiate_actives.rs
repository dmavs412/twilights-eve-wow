//! The shipped Initiate's active talents, read from its talent tabs: Mana Surge refills half the
//! bar and resets Blink and Catalyst; Elemental Convergence leaves the Searing Ember and Biting
//! Cold auras on its target.

mod common;

use common::*;

#[test]
fn the_shipped_actives_name_the_real_auras_and_cooldowns() {
    use twilight_realm::callings::{self, Effect};
    let initiate = callings::calling("initiate").unwrap();
    let skill = |key: &str| {
        initiate
            .talent_tabs
            .iter()
            .flat_map(|t| &t.talents)
            .find(|t| t.key == key)
            .and_then(|t| t.skill.as_ref())
            .unwrap_or_else(|| panic!("{key} teaches a skill"))
    };
    let id = |key: &str| spell("initiate", key);
    let surge = &skill("mana_surge").effects;
    assert!(surge
        .iter()
        .any(|e| matches!(e, Effect::Energize { pct_of_max: 50, .. })));
    assert!(surge.iter().any(
        |e| matches!(e, Effect::ResetCooldowns { spell_ids } if spell_ids[..2] == [id("blink"), id("catalyst")])
    ));
    let convergence = &skill("elemental_convergence").effects;
    assert!(convergence.iter().any(|e| matches!(
        e,
        Effect::PeriodicDamage {
            aura_spell_id: Some(a),
            max_stacks: 3,
            ..
        } if *a == id("aura_searing_ember")
    )));
    assert!(convergence.iter().any(|e| matches!(
        e,
        Effect::ModRunSpeed {
            aura_spell_id: Some(a),
            ..
        } if *a == id("aura_biting_cold")
    )));
}
