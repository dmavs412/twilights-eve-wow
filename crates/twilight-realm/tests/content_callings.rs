//! `content/callings.toml` still says what `data/twilights-eve/callings.json` says: the realm half
//! of every skill the TOML carries, the JSON being the checked research (`extract_spells.py
//! verify`).

use std::path::PathBuf;

use serde_json::Value;
use twilight_realm::callings::{self, Effect, Target, Weapon};
use twilight_realm::rules::skills::Linear;

fn reference() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/twilights-eve/callings.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn linear(v: &Value) -> Linear {
    let f = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    Linear {
        base: f("base"),
        per_level: f("per_level"),
        per_ap: f("per_ap"),
        min: v.get("min").and_then(Value::as_u64).unwrap_or(0) as u32,
    }
}

fn ms(v: &Value, k: &str) -> u32 {
    v[k].as_u64().unwrap_or_else(|| panic!("{k}")) as u32
}

#[test]
fn the_basic_warrior_matches_the_reference() {
    check_calling("basic_warrior");
}

#[test]
fn the_vanguard_matches_the_reference() {
    check_calling("vanguard");
}

#[test]
fn the_zealot_matches_the_reference() {
    check_calling("zealot");
}

/// Spell.dbc as `extract_spells.py verify` dumped it.
fn extracted_spells() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/twilights-eve/spells-extracted.json");
    serde_json::from_str::<Value>(&std::fs::read_to_string(path).unwrap()).unwrap()["spells"]
        .clone()
}

/// `SpellFamilyName` 4.
const WARRIOR_FAMILY: u64 = 4;

/// A skill is marked borrowed exactly when Spell.dbc files it outside the Warrior family, and each
/// borrowed skill has its own family bit that no Warrior spell uses (bits 0..=32 are taken).
#[test]
fn every_skill_borrowed_from_another_class_is_marked() {
    let dbc = extracted_spells();
    let mut bits = Vec::new();
    for key in ["basic_warrior", "vanguard", "zealot"] {
        for s in &callings::calling(key).unwrap().skills {
            let row = &dbc[s.spell_id.to_string()];
            let family = row["family"]
                .as_u64()
                .unwrap_or_else(|| panic!("{key}.{}: not in the dump", s.key));
            assert_eq!(
                s.borrowed.is_some(),
                family != WARRIOR_FAMILY,
                "{key}.{}: Spell.dbc family {family}",
                s.key
            );
            if let Some(b) = s.borrowed {
                assert!(
                    callings::BORROWED_FAMILY_BITS.contains(&b.family_bit),
                    "{}",
                    s.key
                );
                bits.push(b.family_bit);
            }
        }
    }
    let n = bits.len();
    assert_eq!(n, 4, "Zealot's Seals, Judgement and Hammer of Justice");
    bits.sort_unstable();
    bits.dedup();
    assert_eq!(bits.len(), n, "one bit each");
}

/// The client rewrites these once (ADR 0003): each borrowed spell once, at its rage cost.
#[test]
fn the_borrowed_skills_are_zealots_four_at_their_rage_costs() {
    let got: Vec<(u32, u32)> = callings::borrowed_skills()
        .map(|s| (s.spell_id, s.rage_cost))
        .collect();
    assert_eq!(got, [(20375, 10), (20305, 10), (20271, 5), (10308, 15)]);
}

/// The rows the client rewrites: rage (`powerType` 1) in tenths, filed as Warrior spells
/// (`SpellFamilyName` 4) on each one's own bit.
#[test]
fn the_borrowed_rewrites_cost_rage_as_warrior_spells() {
    let got: Vec<(u32, u32, u32, u32, u8)> = callings::borrowed_spell_rewrites()
        .map(|r| {
            (
                r.spell_id,
                r.power_type,
                r.cost,
                r.spell_family,
                r.family_bit,
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            (20375, 1, 100, 4, 33),
            (20305, 1, 100, 4, 34),
            (20271, 1, 50, 4, 35),
            (10308, 1, 150, 4, 36),
        ]
    );
}

/// `callings::skill` answers with a spell's first table, so a skill two Callings share must be
/// the same in both.
#[test]
fn a_skill_two_callings_share_is_the_same_in_both() {
    let all: Vec<&callings::Skill> = ["basic_warrior", "vanguard", "zealot"]
        .iter()
        .flat_map(|c| &callings::calling(c).unwrap().skills)
        .collect();
    for s in &all {
        let first = callings::skill(s.spell_id).unwrap();
        assert_eq!(format!("{first:?}"), format!("{s:?}"), "{}", s.key);
    }
}

fn check_calling(calling: &str) {
    let reference = reference();
    let want = &reference["callings"][calling];
    let got = callings::calling(calling).expect(calling);
    let bonus = &reference["tier_bonus"][got.tier.to_string()];
    let bonus = ["strength", "agility", "stamina", "intellect", "spirit"]
        .map(|s| bonus[s].as_u64().unwrap() as u32);
    assert_eq!(got.tier_bonus.stats(), bonus, "{calling}: the Tier Bonus");
    assert_eq!(got.name, want["display_name"].as_str().unwrap());
    assert_eq!(u64::from(got.tier), want["tier"].as_u64().unwrap());
    assert_eq!(u64::from(got.cap), want["cap"].as_u64().unwrap());
    assert_eq!(
        u64::from(got.shapeshift_form),
        want["client_shapeshift_form"].as_u64().unwrap()
    );
    let armour: Vec<String> = got
        .armour
        .iter()
        .map(|a| format!("{a:?}").to_lowercase())
        .collect();
    let want_armour: Vec<&str> = want["armour"]
        .as_array()
        .unwrap_or_else(|| panic!("{calling}: the reference lists its armour"))
        .iter()
        .map(|a| a.as_str().unwrap())
        .collect();
    assert_eq!(armour, want_armour, "{calling}: armour");

    let skills = want["skills"].as_array().unwrap();
    assert_eq!(got.skills.len(), skills.len());
    for (got, want) in got.skills.iter().zip(skills) {
        let key = want["key"].as_str().unwrap();
        assert_eq!(got.key, key);
        assert_eq!(
            u64::from(got.spell_id),
            want["spell_id"].as_u64().unwrap(),
            "{key}"
        );
        let realm = &want["realm"];
        assert_eq!(got.rage_cost, ms(realm, "rage_cost"), "{key} rage");
        assert_eq!(got.cooldown_ms, ms(realm, "cooldown_ms"), "{key} cooldown");
        assert_eq!(got.gcd_ms, ms(realm, "gcd_ms"), "{key} gcd");
        // The client refuses a cast it cannot afford at the Spell.dbc cost; a borrowed spell's
        // cost there is mana, which the client rewrites to the realm's rage cost.
        assert!(
            got.borrowed.is_some()
                || f64::from(got.rage_cost) >= want["client"]["cost"].as_f64().unwrap(),
            "{key}: realm cost under the client's"
        );
        assert_eq!(
            got.borrowed.map(|b| u64::from(b.family_bit)),
            want["borrowed"]["family_bit"].as_u64(),
            "{key}: borrowed"
        );
        assert_eq!(
            u64::from(got.health_cost_pct_of_base_health),
            realm["health_cost_pct_of_base_health"]
                .as_u64()
                .unwrap_or(0),
            "{key} health cost"
        );
        let target = match realm["target"].as_str().unwrap() {
            "enemy" => Target::Enemy,
            "self" | "self_party" => Target::Caster,
            "self_area" => Target::SelfArea,
            t => panic!("{key}: target {t}"),
        };
        assert_eq!(got.target, target, "{key} target");
        match &realm["range_yd"] {
            Value::Array(r) => assert_eq!(
                got.range_yd.map(f64::from).to_vec(),
                r.iter().map(|v| v.as_f64().unwrap()).collect::<Vec<_>>(),
                "{key} range"
            ),
            Value::Number(max) => assert_eq!(
                got.range_yd,
                [0.0, max.as_f64().unwrap() as f32],
                "{key} range"
            ),
            _ => assert_eq!(got.range_yd, [0.0, 0.0], "{key} range"),
        }
        // A party shout's radius means nothing without a party: it lands on the caster alone.
        if let (Target::SelfArea, Some(r)) = (target, realm["radius_yd"].as_f64()) {
            assert_eq!(f64::from(got.radius_yd), r, "{key} radius");
            assert_eq!(
                u64::from(got.max_targets),
                realm["max_targets"].as_u64().unwrap()
            );
        }
        assert_eq!(
            got.requires_out_of_combat,
            realm["requires_out_of_combat"].as_bool().unwrap_or(false),
            "{key}"
        );
        let requires = realm["requires"].as_str().unwrap_or("");
        assert_eq!(
            got.requires_target_dodged,
            requires.contains("dodge"),
            "{key}"
        );
        assert_eq!(
            got.requires_target_health_pct,
            if requires.contains("20% health") {
                20
            } else {
                0
            },
            "{key}"
        );
        assert_eq!(
            u64::from(got.extra_targets),
            realm["extra_targets"].as_u64().unwrap_or(0),
            "{key}"
        );
        assert_eq!(
            f64::from(got.extra_target_radius_yd),
            realm["extra_target_radius_yd"].as_f64().unwrap_or(0.0),
            "{key}"
        );
        assert_eq!(
            u64::from(got.puts_in_combat_ms),
            realm["puts_in_combat_ms"].as_u64().unwrap_or(0),
            "{key}"
        );

        let effects = realm["effects"].as_array().unwrap();
        assert_eq!(got.effects.len(), effects.len(), "{key} effects");
        for (got, want) in got.effects.iter().zip(effects) {
            check_effect(key, got, want);
        }
    }
}

fn check_effect(key: &str, got: &Effect, want: &Value) {
    let kind = want["kind"].as_str().unwrap();
    match *got {
        Effect::WeaponStrike {
            weapon,
            bonus,
            on_next_swing,
            unavoidable,
        } => {
            assert_eq!(kind, "weapon_strike", "{key}");
            let w = match want["weapon"].as_str().unwrap() {
                "W" => Weapon::Normal,
                "Wn" => Weapon::Normalized,
                w => panic!("{key}: weapon {w}"),
            };
            assert_eq!(weapon, w, "{key}");
            assert_eq!(bonus, linear(&want["bonus"]), "{key}");
            assert_eq!(
                on_next_swing,
                want["on_next_swing"].as_bool().unwrap_or(false)
            );
            assert_eq!(unavoidable, want.get("unavoidable").is_some(), "{key}");
        }
        Effect::SchoolDamage {
            amount,
            melee_hit_table,
        } => {
            assert_eq!(kind, "school_damage", "{key}");
            assert_eq!(amount, linear(&want["amount"]), "{key}");
            assert_eq!(
                melee_hit_table,
                want["melee_hit_table"].as_bool().unwrap_or(false)
            );
        }
        Effect::PeriodicDamage {
            tick_ms,
            duration_ms,
            tick,
        } => {
            assert_eq!(kind, "periodic_damage", "{key}");
            assert_eq!(tick_ms, ms(want, "tick_ms"));
            assert_eq!(duration_ms, ms(want, "duration_ms"));
            assert_eq!(tick, linear(&want["tick"]), "{key}");
        }
        Effect::Charge { speed_yd_s } => {
            assert_eq!(kind, "charge", "{key}");
            assert_eq!(f64::from(speed_yd_s), want["speed_yd_s"].as_f64().unwrap());
        }
        Effect::EnergizeRage { amount } => {
            assert_eq!(kind, "energize_rage", "{key}");
            assert_eq!(amount, linear(&want["amount"]), "{key}");
        }
        Effect::PeriodicRage {
            aura_spell_id,
            per_tick,
            tick_ms,
            duration_ms,
        } => {
            assert_eq!(kind, "periodic_rage", "{key}");
            assert_eq!(aura_spell_id, ms(want, "aura_spell_id"));
            assert_eq!(per_tick, ms(want, "per_tick"));
            assert_eq!(tick_ms, ms(want, "tick_ms"));
            assert_eq!(duration_ms, ms(want, "duration_ms"));
        }
        Effect::Stun {
            aura_spell_id,
            duration_ms,
        } => {
            assert_eq!(kind, "stun", "{key}");
            assert_eq!(aura_spell_id, ms(want, "aura_spell_id"));
            assert_eq!(duration_ms, ms(want, "duration_ms"));
        }
        Effect::AttackSpeedSlow { pct, duration_ms } => {
            assert_eq!(kind, "attack_speed_slow", "{key}");
            assert_eq!(pct, ms(want, "pct"));
            assert_eq!(duration_ms, ms(want, "duration_ms"));
        }
        Effect::MovementSlow { pct, duration_ms } => {
            assert_eq!(kind, "movement_slow", "{key}");
            assert_eq!(pct, ms(want, "pct"));
            assert_eq!(duration_ms, ms(want, "duration_ms"));
        }
        Effect::AuraAttackPower {
            duration_ms,
            amount,
        } => {
            assert_eq!(kind, "aura_attack_power", "{key}");
            assert_eq!(duration_ms, ms(want, "duration_ms"));
            assert_eq!(amount, linear(&want["amount"]), "{key}");
        }
        Effect::HealingReduction { pct, duration_ms } => {
            assert_eq!(kind, "healing_reduction", "{key}");
            assert_eq!(pct, ms(want, "pct"));
            assert_eq!(duration_ms, ms(want, "duration_ms"));
        }
        Effect::Execute {
            base,
            per_extra_rage,
        } => {
            assert_eq!(kind, "execute", "{key}");
            assert_eq!(want["consumes_all_rage"], true, "{key}");
            assert_eq!(base, linear(&want["base"]), "{key}");
            assert_eq!(per_extra_rage, linear(&want["per_extra_rage"]), "{key}");
        }
        Effect::Seal { .. } | Effect::Judgement => {
            assert!(matches!(kind, "seal" | "judgement"), "{key}");
            let want: Effect =
                serde_json::from_value(want.clone()).unwrap_or_else(|e| panic!("{key}: {e}"));
            assert_eq!(*got, want, "{key}");
        }
        // Only active talents' skills do these, and the reference has no realm half for them.
        Effect::AuraMaxHealthPct { .. }
        | Effect::AuraDamagePct { .. }
        | Effect::SweepingStrikes { .. } => panic!("{key}: a talent's effect on a Calling skill"),
    }
}

#[test]
fn every_skill_id_is_listed_for_rank_blanking() {
    let mut ids: Vec<u32> = callings::skill_spell_ids().collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids,
        [78, 100, 6178, 6192, 6546, 7887, 10308, 12294, 20271, 20305, 20375]
    );
}

/// The client names these " (WIP)": the rank spells of every talent with no effect and no skill,
/// Anger Management's and Flurry's (in every tree that has them), and the Zealot's unmodelled
/// Paladin talents'.
#[test]
fn the_wip_rank_spells_are_the_talents_with_no_effect() {
    let mut ids: Vec<u32> = callings::wip_rank_spell_ids().collect();
    ids.sort_unstable();
    ids.dedup();
    let mut want = vec![
        // Anger Management, Flurry.
        12296, 12319, 12971, 12972, 12973, 12974,
        // Improved Judgement, Vindication, Eye for an Eye.
        25956, 25957, 9452, 26016, 26021, 9799, 25988,
        // Pursuit of Justice, Improved Hammer of Justice, Vengeance.
        26022, 26023, 20487, 20488, 20489, 20049, 20056, 20057, 20058, 20059,
        // Holy Power, Divine Strength.
        5923, 5924, 5925, 5926, 25829, 20262, 20263, 20264, 20265, 20266,
        // Precision, Redoubt, Reckoning.
        20189, 20192, 20193, 20127, 20130, 20135, 20136, 20137, 20177, 20179, 20181, 20180, 20182,
    ];
    want.sort_unstable();
    assert_eq!(ids, want);
}

#[test]
fn the_vanguard_kit_is_mortal_strike_overpower_charge_rend_and_battle_shout() {
    let kit: Vec<&str> = callings::calling("vanguard")
        .unwrap()
        .skills
        .iter()
        .map(|s| s.key.as_str())
        .collect();
    assert_eq!(
        kit,
        [
            "mortal_strike",
            "overpower",
            "charge",
            "rend",
            "battle_shout"
        ]
    );
}

#[test]
fn talent_tab_ids_are_unique_across_callings() {
    let mut ids: Vec<u32> = ["basic_warrior", "vanguard", "zealot"]
        .iter()
        .flat_map(|c| {
            callings::calling(c)
                .unwrap()
                .talent_tabs
                .iter()
                .map(|t| t.id)
        })
        .collect();
    let all = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), all, "{ids:?}");
}

/// The Recruit talents the realm shows and lets the player learn, but does not model yet: Anger
/// Management's slower rage decay has no effect kind.
const RECRUIT_NOT_MODELLED: &[&str] = &["anger_management"];

#[test]
fn the_basic_warrior_talent_tree_matches_the_reference() {
    check_tree("basic_warrior", RECRUIT_NOT_MODELLED);
    let tree = &callings::calling("basic_warrior").unwrap().talent_tabs;
    let names: Vec<&str> = tree.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["Recruit"]);
}

#[test]
fn the_basic_warrior_kit_is_heroic_strike_and_charge() {
    let kit: Vec<&str> = callings::calling("basic_warrior")
        .unwrap()
        .skills
        .iter()
        .map(|s| s.key.as_str())
        .collect();
    assert_eq!(kit, ["heroic_strike", "charge"]);
}

/// The Vanguard talents the realm shows and lets the player learn, but does not model yet: Anger
/// Management's slower rage decay and Flurry's haste after a crit have no effect kind.
const VANGUARD_NOT_MODELLED: &[&str] = &["anger_management", "flurry"];

#[test]
fn the_vanguard_talent_tree_matches_the_reference() {
    check_tree("vanguard", VANGUARD_NOT_MODELLED);
    let tree = &callings::calling("vanguard").unwrap().talent_tabs;
    let names: Vec<&str> = tree.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["Weaponmaster", "Warlord"]);
}

/// The rows of each Vanguard tab, as the handoff lists them: talent keys by tier.
#[test]
fn the_vanguard_tabs_seat_the_handoffs_rows() {
    let rows = |tab: usize| -> Vec<Vec<String>> {
        let tab = &callings::calling("vanguard").unwrap().talent_tabs[tab];
        let last = tab.talents.iter().map(|t| t.tier).max().unwrap();
        (1..=last)
            .map(|tier| {
                let mut row: Vec<String> = tab
                    .talents
                    .iter()
                    .filter(|t| t.tier == tier)
                    .map(|t| t.key.clone())
                    .collect();
                row.sort();
                row
            })
            .collect()
    };
    assert_eq!(
        rows(0),
        [
            vec!["improved_overpower", "improved_rend"],
            vec!["anger_management", "deep_wounds", "improved_charge"],
            vec!["impale", "two_handed_weapon_specialization"],
            vec![
                "axe_specialization",
                "mace_specialization",
                "sword_specialization"
            ],
            vec!["flurry"],
        ]
    );
    assert_eq!(
        rows(1),
        [
            vec!["booming_voice", "cruelty"],
            vec!["improved_battle_shout", "unbridled_wrath"],
            vec!["blood_craze", "enrage"],
        ]
    );
}

/// The Zealot talents the realm shows and lets the player learn, but does not model yet: those
/// waiting on a kind no talent has yet (cooldowns, haste, hit, movement, Strength, holy crit) or
/// on a proc the realm does not roll.
const ZEALOT_NOT_MODELLED: &[&str] = &[
    "improved_judgement",
    "vindication",
    "eye_for_an_eye",
    "pursuit_of_justice",
    "improved_hammer_of_justice",
    "vengeance",
    "holy_power",
    "anger_management",
    "flurry",
    "divine_strength",
    "precision",
    "redoubt",
    "reckoning",
];

#[test]
fn the_zealot_talent_tree_matches_the_reference() {
    check_tree("zealot", ZEALOT_NOT_MODELLED);
    let tree = &callings::calling("zealot").unwrap().talent_tabs;
    let names: Vec<&str> = tree.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["Crusade", "Blade", "Faith"]);
}

/// A tab's talent keys, tier by tier, each row sorted.
fn rows(calling: &str, tab: usize) -> Vec<Vec<String>> {
    let tab = &callings::calling(calling).unwrap().talent_tabs[tab];
    let last = tab.talents.iter().map(|t| t.tier).max().unwrap();
    (1..=last)
        .map(|tier| {
            let mut row: Vec<String> = tab
                .talents
                .iter()
                .filter(|t| t.tier == tier)
                .map(|t| t.key.clone())
                .collect();
            row.sort();
            row
        })
        .collect()
}

/// The rows of each Zealot tab, as the handoff lists them.
#[test]
fn the_zealot_tabs_seat_the_handoffs_rows() {
    assert_eq!(
        rows("zealot", 0),
        [
            vec!["benediction", "improved_seal_of_the_crusader"],
            vec!["deflection", "improved_judgement"],
            vec!["conviction", "vindication"],
            vec!["eye_for_an_eye", "pursuit_of_justice"],
            vec![
                "improved_hammer_of_justice",
                "two_handed_weapon_specialization"
            ],
            vec!["vengeance"],
            vec!["holy_power"],
        ]
    );
    // The handoff's first two rows share the first: Improved Charge and Anger Management alone
    // hold 3 ranks, short of the 5 that open a second row.
    assert_eq!(
        rows("zealot", 1),
        [
            vec![
                "anger_management",
                "deep_wounds",
                "impale",
                "improved_charge"
            ],
            vec![
                "axe_specialization",
                "mace_specialization",
                "sword_specialization"
            ],
            vec!["flurry"],
        ]
    );
    assert_eq!(
        rows("zealot", 2),
        [
            vec!["divine_strength", "toughness"],
            vec!["precision", "redoubt"],
            vec!["anticipation", "one_handed_weapon_specialization"],
            vec!["reckoning"],
        ]
    );
}

/// Blade's talents are Vanguard's Talent.dbc rows, doing for a Zealot what they do for a Vanguard,
/// so the Vanguard's fights (tests/tier1_talents_in_combat.rs) cover them. The client names a rank
/// spell WIP once for every Calling, so a talent two Callings share is WIP in both or neither.
#[test]
fn a_talent_two_callings_share_does_the_same_in_both() {
    let tree = |c: &str| -> Vec<&'static callings::TalentDef> {
        callings::calling(c)
            .unwrap()
            .talent_tabs
            .iter()
            .flat_map(|tab| &tab.talents)
            .collect()
    };
    let vanguard = tree("vanguard");
    let blade = &callings::calling("zealot").unwrap().talent_tabs[1].talents;
    for t in blade {
        let v = vanguard
            .iter()
            .find(|v| v.talent_id == t.talent_id)
            .unwrap_or_else(|| panic!("{}: not Vanguard's", t.key));
        assert_eq!(t.effects, v.effects, "{}", t.key);
    }
    let all: Vec<_> = ["basic_warrior", "vanguard", "zealot"]
        .iter()
        .flat_map(|c| tree(c))
        .collect();
    for a in &all {
        for b in all.iter().filter(|b| b.talent_id == a.talent_id) {
            assert_eq!(a.rank_spells, b.rank_spells, "{}", a.key);
            assert_eq!(a.is_wip(), b.is_wip(), "{}", a.key);
        }
    }
}

/// A Calling's tree against the reference; the talents in `unmodelled` have no effect yet.
fn check_tree(calling: &str, unmodelled: &[&str]) {
    let reference = reference();
    let want = &reference["callings"][calling]["talent_tree"]["tabs"];
    let got = &callings::calling(calling).unwrap().talent_tabs;
    let want = want.as_array().unwrap();
    assert_eq!(got.len(), want.len(), "{calling}: the tabs");
    for (tab, want) in got.iter().zip(want) {
        assert_eq!(u64::from(tab.id), want["catalog_tab_id"].as_u64().unwrap());
        assert_eq!(tab.name, want["name"].as_str().unwrap());
        assert_eq!(tab.background, want["background"].as_str().unwrap());
        let talents = want["talents"].as_array().unwrap();
        assert_eq!(tab.talents.len(), talents.len());
        for (t, want) in tab.talents.iter().zip(talents) {
            let key = want["key"].as_str().unwrap();
            assert_eq!(t.key, key);
            assert_eq!(
                u64::from(t.talent_id),
                want["talent_id"].as_u64().unwrap(),
                "{key}"
            );
            assert_eq!(u64::from(t.tier), want["tier"].as_u64().unwrap(), "{key}");
            assert_eq!(
                u64::from(t.column),
                want["column"].as_u64().unwrap(),
                "{key}"
            );
            let ranks: Vec<u32> = want["rank_spells"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect();
            assert_eq!(t.rank_spells, ranks, "{key}");
            assert_eq!(
                t.exceptional,
                want["exceptional"].as_bool().unwrap_or(false),
                "{key}"
            );
            match (&t.prereq, want.get("prereq")) {
                (None, None) => {}
                (Some(p), Some(w)) => {
                    assert_eq!(p.key, w["key"].as_str().unwrap(), "{key}");
                    assert_eq!(u64::from(p.rank), w["rank"].as_u64().unwrap(), "{key}");
                }
                (p, w) => panic!("{key}: prereq {p:?} against {w:?}"),
            }
            let none = Vec::new();
            let want_mods = want
                .get("client_spellmods")
                .map_or(&none, |w| w.as_array().unwrap());
            assert_eq!(
                t.client_spellmods.len(),
                want_mods.len(),
                "{key}: spell mods"
            );
            for (m, w) in t.client_spellmods.iter().zip(want_mods) {
                assert_eq!(m.flat, w["type"] == "flat", "{key}");
                // vmangos `SpellModOp`.
                let op = match w["op"].as_str().unwrap() {
                    "COST" => 14,
                    "EFFECT1" => 3,
                    "ALL_EFFECTS" => 8,
                    op => panic!("{key}: op {op}"),
                };
                assert_eq!(m.op, op, "{key}");
                assert_eq!(u64::from(m.family_bit), w["family_bit"].as_u64().unwrap());
                let values: Vec<i32> = w["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_i64().unwrap() as i32)
                    .collect();
                assert_eq!(m.per_rank, values, "{key}");
            }
            assert_eq!(
                t.is_wip(),
                unmodelled.contains(&key),
                "{key}: the realm decides what it does"
            );
            // An active talent's one rank is the skill it teaches.
            if let Some(skill) = &t.skill {
                assert!(t.exceptional, "{key}: a skill from a passive talent");
                assert_eq!(t.rank_spells, [skill.spell_id], "{key}");
            }
            for e in &t.effects {
                assert_eq!(
                    e.per_rank().len(),
                    t.rank_spells.len(),
                    "{key}: an effect for every rank"
                );
            }
        }
    }
}

// ── The Advancement Quests and the Tier 1 trainers ──────────────────────────────────────────────

/// An Advancement Quest as `content/quests.toml` writes it.
#[derive(serde::Deserialize)]
struct Quest {
    id: u32,
    giver: u32,
    level: u8,
    title: String,
    objectives: String,
    details: String,
    progress: String,
    completion: String,
    zone_or_sort: Option<i32>,
    advancement: Option<Advancement>,
}

#[derive(serde::Deserialize)]
struct Advancement {
    from: String,
    to: String,
    completable: Option<bool>,
}

/// A Hub NPC as `content/hub.toml` writes it.
#[derive(serde::Deserialize)]
struct Npc {
    entry: u32,
    name: String,
    #[serde(default)]
    subname: String,
    role: String,
    #[serde(default)]
    greeting: String,
}

fn advancement_quests() -> Vec<Quest> {
    #[derive(serde::Deserialize)]
    struct File {
        quest: Vec<Quest>,
    }
    toml::from_str::<File>(include_str!("../content/quests.toml"))
        .expect("quests.toml parses")
        .quest
        .into_iter()
        .filter(|q| q.advancement.is_some())
        .collect()
}

fn trainers() -> Vec<Npc> {
    #[derive(serde::Deserialize)]
    struct File {
        npc: Vec<Npc>,
    }
    toml::from_str::<File>(include_str!("../content/hub.toml"))
        .expect("hub.toml parses")
        .npc
        .into_iter()
        .filter(|n| n.role == "trainer")
        .collect()
}

/// The Tier 1 Calling an Advancement Quest's Calling `to` branches from: its Line.
fn line_of(quests: &[Quest], to: &str) -> String {
    fn adv(q: &Quest) -> &Advancement {
        q.advancement.as_ref().unwrap()
    }
    let q = quests
        .iter()
        .find(|q| adv(q).to == to)
        .unwrap_or_else(|| panic!("no Advancement Quest into {to}"));
    match adv(q).from.as_str() {
        "basic_warrior" => to.to_string(),
        from => line_of(quests, from),
    }
}

/// Every Advancement Quest the reference names is in the content as it says, and the content has
/// no other.
#[test]
fn every_advancement_quest_matches_the_reference() {
    let reference = reference();
    let quests = advancement_quests();
    let mut named = Vec::new();
    for (key, calling) in reference["callings"].as_object().unwrap() {
        let want = &calling["advancement_quest"];
        if want.is_null() {
            continue;
        }
        let id = want["quest_id"].as_u64().unwrap() as u32;
        named.push(id);
        let q = quests
            .iter()
            .find(|q| q.id == id)
            .unwrap_or_else(|| panic!("{key}: quest {id} is not in content/quests.toml"));
        let adv = q.advancement.as_ref().unwrap();
        let from = want["offered_to"]["calling"].as_str().unwrap();
        assert_eq!((adv.from.as_str(), adv.to.as_str()), (from, key.as_str()));
        assert_eq!(
            want["offered_to"]["level"].as_u64(),
            reference["callings"][from]["cap"].as_u64(),
            "{key}: offered at {from}'s Tier Cap"
        );
        assert_eq!(
            adv.completable.unwrap_or(true),
            want["completable"].as_bool().unwrap_or(true),
            "{key}"
        );
        assert_eq!(
            adv.completable.unwrap_or(true),
            !calling["locked"].as_bool().unwrap_or(false),
            "{key}: a locked Calling's quest never finishes"
        );
        assert_eq!(u64::from(q.level), want["quest_level"].as_u64().unwrap());
        assert_eq!(
            q.zone_or_sort.map(i64::from),
            want["zone_or_sort"].as_i64(),
            "{key}"
        );
        assert_eq!(q.title, want["title"].as_str().unwrap(), "{key}");
        assert_eq!(q.details, want["details"].as_str().unwrap(), "{key}");
        assert_eq!(q.objectives, want["objectives"].as_str().unwrap(), "{key}");
        if let Some(t) = want["offer_reward_text"].as_str() {
            assert_eq!(q.completion, t, "{key}");
        }
        if let Some(t) = want["request_items_text"].as_str() {
            assert_eq!(q.progress, t, "{key}");
        }
    }
    let mut have: Vec<u32> = quests.iter().map(|q| q.id).collect();
    have.sort_unstable();
    named.sort_unstable();
    assert_eq!(have, named);
}

/// Cutthroat, Initiate and Witch Hunter are Tier 1 Callings with no kit and no Talent Tree yet:
/// their quests are offered, and nothing can enter them.
#[test]
fn the_three_new_tier_1_callings_are_locked_with_no_kit() {
    let reference = reference();
    let callings = reference["callings"].as_object().unwrap();
    let mut tier_1: Vec<&str> = callings
        .iter()
        .filter(|(_, c)| c["tier"] == 1)
        .map(|(k, _)| k.as_str())
        .collect();
    tier_1.sort_unstable();
    assert_eq!(
        tier_1,
        ["cutthroat", "initiate", "vanguard", "witch_hunter"]
    );
    for (key, name) in [
        ("cutthroat", "Cutthroat"),
        ("initiate", "Initiate"),
        ("witch_hunter", "Witch Hunter"),
    ] {
        let c = &callings[key];
        assert_eq!(c["display_name"], name);
        assert_eq!(c["cap"], 30, "{key}");
        assert_eq!(c["locked"], true, "{key}");
        assert_eq!(c["skills"], serde_json::json!([]), "{key}");
        assert!(c["talent_tree"].is_null(), "{key}");
        assert!(callings::calling(key).is_none(), "{key} has no kit");
        assert_eq!(
            c["advancement_quest"]["offered_to"]["calling"], "basic_warrior",
            "{key}"
        );
    }
}

/// Each trainer offers the Advancement Quests of one Line, every Tier 1 Calling has its own
/// trainer, and each trainer's title and greeting name the Line.
#[test]
fn each_tier_1_line_has_its_own_trainer() {
    let reference = reference();
    let quests = advancement_quests();
    let mut lines = Vec::new();
    for t in trainers() {
        let mut line: Vec<String> = quests
            .iter()
            .filter(|q| q.giver == t.entry)
            .map(|q| line_of(&quests, &q.advancement.as_ref().unwrap().to))
            .collect();
        line.sort();
        line.dedup();
        let [line] = <[String; 1]>::try_from(line)
            .unwrap_or_else(|l| panic!("{}: the quests of {l:?}", t.name));
        let name = reference["callings"][&line]["display_name"]
            .as_str()
            .unwrap();
        assert_eq!(t.subname, format!("{name} Trainer"), "{}", t.name);
        assert!(
            t.greeting.contains(name),
            "{}: the greeting names the {name} line",
            t.name
        );
        lines.push(line);
    }
    lines.sort();
    assert_eq!(
        lines,
        ["cutthroat", "initiate", "vanguard", "witch_hunter"],
        "one trainer for each Tier 1 Calling"
    );
}
