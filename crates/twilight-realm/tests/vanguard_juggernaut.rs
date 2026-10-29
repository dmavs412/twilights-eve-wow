//! Iron Juggernaut: a skill's strike adds a percent of the shield's Block Value. Reads the
//! Vanguard's rows.

use twilight_realm::callings::{self, TalentEffect};

#[test]
fn the_vanguards_iron_juggernaut_feeds_both_strikes() {
    let t = callings::calling("vanguard")
        .unwrap()
        .talent_tabs
        .iter()
        .flat_map(|t| &t.talents)
        .find(|t| t.key == "iron_juggernaut")
        .unwrap();
    let skills: Vec<(&str, &[u32])> = t
        .effects
        .iter()
        .filter_map(|e| match e {
            TalentEffect::SkillBlockValuePct { skill, per_rank } => {
                Some((skill.as_str(), per_rank.as_slice()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        skills,
        [
            ("righteous_smash", &[50, 100, 150][..]),
            ("zealous_cleave", &[50, 100, 150][..])
        ]
    );
}
