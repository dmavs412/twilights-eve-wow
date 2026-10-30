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

// Combustion Engine (#506), played by a shipped Initiate at 24 on the level 8 or 9 mob south of
// the Crossroads.

use common::cutthroat::damage_of;
use common::initiate::*;
use common::vanguard::STURDY;

/// The hits `spell_id` dealt that did not crit, across seeds, cast at the mob after
/// `first` (when set) put Searing Ember on it, with `rows` set.
fn plain_hits(rows: &[(&str, u32)], first: Option<u32>, spell_id: u32) -> Vec<u32> {
    let mut hits = Vec::new();
    for seed in 0..8 {
        let mut s = initiate_scene(seed, rows, STURDY);
        let mob = s.mob.guid;
        if let Some(ember) = first {
            cast_through(&mut s.realm, &mut s.view, ember, mob);
        }
        let events = cast_through(&mut s.realm, &mut s.view, spell_id, mob);
        hits.extend(
            damage_of(&events, spell_id)
                .into_iter()
                .filter(|&(target, _, crit)| target == mob && !crit)
                .map(|(_, dealt, _)| dealt),
        );
    }
    assert!(!hits.is_empty(), "a hit that did not crit");
    hits
}

/// Combustion Engine hits a target burning from Searing Ember for 1.45 times plain Biting Cold's
/// hit (rounded), and a clean target for plain Biting Cold's hit; plain Biting Cold hits a burning
/// target for its plain hit.
#[test]
fn combustion_engine_deals_1_45_times_plain_biting_colds_hit_to_a_burning_target() {
    let engine = choice("initiate", "biting_cold", "combustion_engine");
    let (cold, ember) = (initiate("biting_cold"), initiate("searing_ember"));
    let rows = [("biting_cold", engine)];
    let plain = plain_hits(&[], None, cold);
    let hit = plain[0];
    assert!(plain.iter().all(|&h| h == hit), "{plain:?}");
    let burning = plain_hits(&rows, Some(ember), engine);
    let want = (hit as f32 * 1.45).round() as u32;
    assert!(
        burning.iter().all(|&h| h == want),
        "{burning:?}, want {want}"
    );
    let clean = plain_hits(&rows, None, engine);
    assert!(clean.iter().all(|&h| h == hit), "{clean:?}, want {hit}");
    let plain_burning = plain_hits(&[], Some(ember), cold);
    assert!(plain_burning.iter().all(|&h| h == hit), "{plain_burning:?}");
}
