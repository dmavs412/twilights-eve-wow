//! Hammer of the Vanguard, a talent that teaches a skill: 150% of a weapon roll as Holy damage
//! from 20 yd, always a critical strike on a target standing in the caster's Consecration. Reads
//! the Vanguard's rows.

use twilight_realm::callings::{self, CritIf, Effect};
use twilight_realm::rules::school::School;

fn vanguard() -> &'static callings::CallingDef {
    callings::calling("vanguard").expect("the Vanguard is a shipped Calling")
}

fn talent(key: &str) -> &'static callings::TalentDef {
    vanguard()
        .talent_tabs
        .iter()
        .flat_map(|t| &t.talents)
        .find(|t| t.key == key)
        .unwrap_or_else(|| panic!("the Vanguard has {key}"))
}

#[test]
fn the_vanguards_hammer_is_a_twenty_yard_holy_strike_at_half_again_a_weapon_roll() {
    let t = talent("hammer_of_vanguard");
    let skill = t.skill.as_ref().expect("the talent teaches a skill");
    assert_eq!(skill.spell_id, t.rank_spells[0]);
    assert_eq!(
        (skill.power_cost, skill.cooldown_ms, skill.gcd_ms),
        (30, 6000, 0)
    );
    assert_eq!(skill.range_yd, [0.0, 20.0]);
    assert!(matches!(
        skill.effects[..],
        [Effect::WeaponStrike {
            weapon_pct: 150,
            school: School::Holy,
            crit_if_target_in: Some(CritIf::GroundEffect),
            ..
        }]
    ));
}
