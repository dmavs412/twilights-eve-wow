//! `content/callings.toml` still says what `data/twilights-eve/callings.json` says: the realm half
//! of every skill the TOML carries, the JSON being the checked research (`extract_spells.py
//! verify`).

use std::path::PathBuf;

use serde_json::Value;
use twilight_realm::callings::{self, Effect, Target, Weapon};
use twilight_realm::rules::power::PowerKind;
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

// Basic Warrior and Vanguard are not checked here: they play from the master CSV
// (`server_spells.json`), the sheet being their truth.

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
    // The Basic Warrior's borrowed spells are the client patch's, not marked here.
    for key in ["zealot"] {
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
        .map(|s| (s.spell_id, s.power_cost))
        .collect();
    assert_eq!(got, [(20375, 10), (20305, 10), (20271, 5), (10308, 15)]);
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
        assert_eq!(got.power_cost, ms(realm, "rage_cost"), "{key} cost");
        assert_eq!(got.cooldown_ms, ms(realm, "cooldown_ms"), "{key} cooldown");
        assert_eq!(got.gcd_ms, ms(realm, "gcd_ms"), "{key} gcd");
        // The client refuses a cast it cannot afford at the Spell.dbc cost; a borrowed spell's
        // cost there is mana, which the client rewrites to the realm's rage cost.
        assert!(
            got.borrowed.is_some()
                || f64::from(got.power_cost) >= want["client"]["cost"].as_f64().unwrap(),
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
            per_combo,
            ..
        } => {
            assert_eq!(kind, "weapon_strike", "{key}");
            assert_eq!(per_combo, want.get("per_combo").map(linear), "{key}");
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
            per_combo,
            ..
        } => {
            assert_eq!(kind, "school_damage", "{key}");
            assert_eq!(per_combo, want.get("per_combo").map(linear), "{key}");
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
            per_combo,
            duration_per_combo,
            ..
        } => {
            assert_eq!(kind, "periodic_damage", "{key}");
            assert_eq!(per_combo, want.get("per_combo").map(linear), "{key}");
            assert_eq!(
                duration_per_combo,
                want.get("duration_per_combo").map(linear),
                "{key}"
            );
            assert_eq!(tick_ms, ms(want, "tick_ms"));
            assert_eq!(duration_ms, ms(want, "duration_ms"));
            assert_eq!(tick, linear(&want["tick"]), "{key}");
        }
        Effect::Charge { speed_yd_s } => {
            assert_eq!(kind, "charge", "{key}");
            assert_eq!(f64::from(speed_yd_s), want["speed_yd_s"].as_f64().unwrap());
        }
        Effect::Energize { power, amount, .. } => {
            assert_eq!(kind, "energize", "{key}");
            assert_eq!(
                power,
                serde_json::from_value::<PowerKind>(want["power"].clone()).unwrap(),
                "{key}"
            );
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
            per_combo,
        } => {
            assert_eq!(kind, "stun", "{key}");
            assert_eq!(per_combo, want.get("per_combo").map(linear), "{key}");
            assert_eq!(aura_spell_id, ms(want, "aura_spell_id"));
            assert_eq!(duration_ms, ms(want, "duration_ms"));
        }
        Effect::Incapacitate {
            duration_ms,
            break_on_damage,
            drop_combat,
        } => {
            assert_eq!(kind, "incapacitate", "{key}");
            assert_eq!(duration_ms, ms(want, "duration_ms"));
            assert_eq!(break_on_damage, want["break_on_damage"] == true);
            assert_eq!(drop_combat, want["drop_combat"] == true);
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
        Effect::AddComboPoints { amount } => {
            assert_eq!(kind, "add_combo_points", "{key}");
            assert_eq!(amount, ms(want, "amount"));
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
        // The Vanguard's stacks live in the master CSV only; tests/vanguard_conviction.rs reads them.
        Effect::AddAuraStacks { .. } | Effect::ConsumeAuraStacks { .. } => {
            panic!("{key}: the reference has no realm half for stacks")
        }
        // The Initiate's and the Acolyte's newer kinds live in the master CSV only;
        // tests/initiate_*.rs and tests/acolyte_*.rs read them.
        Effect::ModRunSpeed { .. }
        | Effect::ConsumeAura { .. }
        | Effect::Blink { .. }
        | Effect::ResetCooldowns { .. }
        | Effect::Heal { .. }
        | Effect::PeriodicHeal { .. }
        | Effect::Drain { .. }
        | Effect::GroundArea { .. } => {
            panic!("{key}: the reference has no realm half for a snare or a consume")
        }
        // Consecration lives in the master CSV only; tests/vanguard_consecration.rs reads it.
        Effect::GroundEffect { .. } => panic!("{key}: the reference has no realm half for a patch"),
        // Evasion lives in the master CSV only; tests/cutthroat_stats.rs reads it.
        // Shield of the Penitent's and Seal of Iron's lasting stats live in the master CSV only;
        // tests/vanguard_defense.rs reads them.
        Effect::GrantAura { .. } => {
            panic!("{key}: the reference has no realm half for an aura row")
        }
        Effect::ParryPct { .. }
        | Effect::BlockPct { .. }
        | Effect::BlockValuePct { .. }
        | Effect::ArmorItemsPct { .. } => {
            panic!("{key}: the reference has no realm half for a defense aura")
        }
        Effect::DodgePct { .. } => {
            panic!("{key}: the reference has no realm half for a dodge aura")
        }
        // Stealth lives in the master CSV only; tests/stealth.rs reads it.
        Effect::Stealth { .. } => panic!("{key}: the reference has no realm half for stealth"),
    }
}

#[test]
fn every_skill_id_is_listed_for_rank_blanking() {
    let mut ids: Vec<u32> = callings::skill_spell_ids().collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids,
        [
            78, 100, 116, 133, 589, 689, 845, 1752, 1776, 1784, 1943, 1953, 2061, 2098, 2120, 2136,
            2912, 5277, 6178, 8092, 8921, 10308, 11366, 12043, 12294, 12809, 20165, 20271, 20305,
            20375, 25899
        ]
    );
}

/// The client names these " (WIP)": the rank spells of every talent with no effect and no skill,
/// Anger Management's and Flurry's (in every tree that has them), the Zealot's unmodelled
/// Paladin talents' and the Vanguard's unmodelled talents'.
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
        // Vanguard's Bastion: nothing (every talent is modelled now).
        // Vanguard's Righteous: nothing (every talent is modelled now).
        // Cutthroat's Skirmisher: nothing (Lethality, Flurry of Blades, Reflexive Strike, Dual
        // Wield Specialization, Phantom Guile and Relentless Strikes are modelled now).
        // Cutthroat's Tracker: nothing (Scent of Blood, Bloodletting, Predator's Step, Serrated
        // Strikes, Ruthlessness and Executioner's Toll are modelled now).
        // Initiate's Weaver and Channeler: nothing (every talent is modelled now).
        // Ranger's Swiftbow: Guerilla Tactics, Rhythmic Draw, Twin Arrows, Parting Gift, Adrenaline
        // Rush, Flurry, Rapid Assault.
        19295, 19297, 19298, 19301, 19300, 19184, 19387, 19388, 19389, 19390, 19151, 19152, 19153,
        19239, 19245, 19290, 19294, 24283, 19376, 19377, 19263,
        // Ranger's Arbalest: Piercing Force, Jagged Barbs, Shrapnel Payload, Steadfast Aim,
        // Shattered Defenses, Point Blank, Explosive Payload.
        19426, 19427, 19429, 19430, 19431, 19464, 19465, 19466, 19467, 19468, 19461, 19462, 24691,
        19559, 19560, 19498, 19499, 19500, 19557, 19558, 19503,
        // Acolyte's Rot-Weaver: Virulence, Epidemic, Symbiotic Rot, Festering Wounds, Necrotic
        // Detonation.
        15270, 15335, 15336, 15337, 15338, 15318, 15272, 15320, 27811, 27815, 27816, 15274, 15311,
        15286,
        // Acolyte's Hemomancer: Vitality, Masochism, Coagulation, Crimson Surge, Will to Survive,
        // Blood Ties, Sanguine Shell.
        14889, 15008, 15009, 15010, 15011, 14898, 15349, 15354, 15355, 15356, 14912, 15013, 15014,
        14913, 15012, 14908, 15020, 17191, 27789, 27790, 15237,
    ];
    want.sort_unstable();
    assert_eq!(ids, want);
}

#[test]
fn the_vanguard_plays_righteous_smash_bulwark_charge_and_zealous_cleave() {
    let kit: Vec<&str> = callings::calling("vanguard")
        .unwrap()
        .skills
        .iter()
        .map(|s| s.key.as_str())
        .collect();
    assert_eq!(
        kit,
        [
            "righteous_smash",
            "bulwark_charge",
            "shield_of_penitent",
            "zealous_cleave",
            "seal_of_iron"
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

/// The fields schema #63 added reach the content: Charge's gate and cast shape, an aura row's
/// columns, and a talent rank's spellmods. Loaded only; nothing enforces them yet.
#[test]
fn the_schema_fields_reach_the_content() {
    let warrior = callings::calling("basic_warrior").unwrap();

    let charge = warrior.skills.iter().find(|s| s.key == "charge").unwrap();
    assert_eq!(charge.meta.gates.requires["combat"], "out");
    assert_eq!(charge.meta.gates.school.as_deref(), Some("physical"));
    assert_eq!(charge.meta.gates.prevention.as_deref(), Some("pacify"));
    assert_eq!(charge.meta.shape.cost_kind.as_deref(), Some("flat"));
    assert_eq!(charge.meta.shape.cast_kind.as_deref(), Some("instant"));
    assert_eq!(charge.meta.shape.defense_type.as_deref(), Some("melee"));
    assert!(charge.meta.shape.on_cast.breaks_stealth);
    assert_eq!(charge.meta.shape.max_targets, Some(1));

    // An aura whose effect kind the realm lacks (the stun) is still dropped, as before.
    let buff = warrior
        .auras
        .iter()
        .find(|a| a.key == "momentum_crit_1")
        .unwrap();
    assert!(!buff.row.is_hidden());
    assert_eq!(buff.spell_id, 14177);
    assert_eq!(buff.row.max_stacks, 1);
    assert_eq!(buff.row.stack_rule.as_deref(), Some("refresh"));
    assert_eq!(buff.row.unique_per.as_deref(), Some("caster"));
    assert_eq!(buff.row.dispel_type.as_deref(), Some("none"));
    assert!(buff.row.aura_positive && buff.row.cancelable);
    assert!(!buff.row.aura_hidden && !buff.row.persist_death);

    let spark = warrior
        .talent_tabs
        .iter()
        .flat_map(|t| &t.talents)
        .find(|t| t.key == "initiates_spark")
        .unwrap();
    assert_eq!(spark.rank_spellmods.len(), spark.rank_spells.len());
    let first = &spark.rank_spellmods[0][0];
    assert_eq!((first.kind.as_str(), first.op.as_str()), ("pct", "DAMAGE"));
    assert_eq!(first.family_bit, Some(37));
    assert_eq!(first.value, 10.0);
    assert_eq!(first.target, ["fire_blast"]);
}

/// Basic Warrior's tree is the master CSV's, as `build_spells.py` wrote it: the realm shows each
/// talent on its borrowed `Talent.dbc` id and rank spells, and models only what the sheet marks
/// modelled and the realm has a kind for.
#[test]
fn the_basic_warrior_talent_tree_is_the_sheets() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/twilights-eve/server_spells.json");
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let want = file
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["key"] == "basic_warrior")
        .unwrap();
    let got = callings::calling("basic_warrior").unwrap();
    assert_eq!(got.name, want["name"].as_str().unwrap());
    assert_eq!(u64::from(got.cap), want["tier_cap"].as_u64().unwrap());
    let tabs = want["talent_tabs"].as_array().unwrap();
    assert_eq!(got.talent_tabs.len(), tabs.len());
    for (tab, want) in got.talent_tabs.iter().zip(tabs) {
        assert_eq!(u64::from(tab.id), want["id"].as_u64().unwrap());
        assert_eq!(tab.name, want["name"].as_str().unwrap());
        assert_eq!(tab.background, want["background"].as_str().unwrap());
        let talents = want["talents"].as_array().unwrap();
        assert_eq!(tab.talents.len(), talents.len());
        for (t, want) in tab.talents.iter().zip(talents) {
            let key = want["key"].as_str().unwrap();
            let ranks = want["ranks"].as_array().unwrap();
            assert_eq!(t.key, key);
            assert_eq!(
                u64::from(t.talent_id),
                ranks[0]["talent_id"].as_u64().unwrap(),
                "{key}"
            );
            assert_eq!(u64::from(t.tier), want["seat"]["row"].as_u64().unwrap());
            assert_eq!(
                u64::from(t.column),
                want["seat"]["column"].as_u64().unwrap()
            );
            let spells: Vec<u64> = ranks
                .iter()
                .map(|r| r["spell_id"].as_u64().unwrap())
                .collect();
            let got_spells: Vec<u64> = t.rank_spells.iter().map(|&s| u64::from(s)).collect();
            assert_eq!(got_spells, spells, "{key}");
            for e in &t.effects {
                assert_eq!(e.per_rank().len(), spells.len(), "{key}: a size per rank");
            }
        }
    }
}

/// A Calling's source is data: the sheet's `calling` row moves it out of the TOML.
#[test]
fn only_the_zealot_keeps_a_table_in_the_toml() {
    let tables: toml::Table = include_str!("../content/callings.toml").parse().unwrap();
    let keys: Vec<&str> = tables["calling"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, ["zealot"]);
    assert!(callings::calling("basic_warrior").is_some());
    assert!(callings::calling("vanguard").is_some());
}

/// Recruit keeps its id and art; every talent on the sheet is modelled.
#[test]
fn the_basic_warrior_tree_is_recruit_on_the_warrior_arms_art() {
    let tree = &callings::calling("basic_warrior").unwrap().talent_tabs;
    let tabs: Vec<(u32, &str, &str)> = tree
        .iter()
        .map(|t| (t.id, t.name.as_str(), t.background.as_str()))
        .collect();
    assert_eq!(tabs, [(900, "Recruit", "WarriorArms")]);
    let modelled: Vec<&str> = tree[0]
        .talents
        .iter()
        .filter(|t| !t.is_wip())
        .map(|t| t.key.as_str())
        .collect();
    assert_eq!(
        modelled,
        [
            "field_medic",
            "initiates_spark",
            "cutthroats_momentum",
            "savage_strikes",
            "vanguards_resolve"
        ]
    );
}

#[test]
fn the_basic_warrior_kit_is_heroic_strike_charge_gouge_and_fire_blast() {
    let kit: Vec<&str> = callings::calling("basic_warrior")
        .unwrap()
        .skills
        .iter()
        .map(|s| s.key.as_str())
        .collect();
    assert_eq!(kit, ["heroic_strike", "charge", "gouge", "fire_blast"]);
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

/// The client names a rank spell WIP once for every Calling, so a talent two Callings share (the
/// Vanguard's Righteous Fury borrows Cruelty's row, which the Zealot's Blade has too) is WIP in both
/// or neither.
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

/// Every Tier 1 Calling has a kit: none is locked, and each has its Advancement Quest at Basic
/// Warrior's cap.
#[test]
fn every_tier_1_calling_has_a_kit() {
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
        ["acolyte", "cutthroat", "initiate", "ranger", "vanguard"]
    );
    for key in tier_1 {
        let c = &callings[key];
        assert_eq!(c["cap"], 30, "{key}");
        assert!(c.get("locked").is_none(), "{key}");
        assert!(callings::calling(key).is_some(), "{key} has a kit");
        if key != "vanguard" {
            assert_eq!(
                c["advancement_quest"]["offered_to"]["calling"], "basic_warrior",
                "{key}"
            );
        }
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
        ["acolyte", "cutthroat", "initiate", "ranger", "vanguard"],
        "one trainer for each Tier 1 Calling"
    );
}
