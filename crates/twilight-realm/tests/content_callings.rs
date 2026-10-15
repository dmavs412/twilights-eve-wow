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
    let reference = reference();
    let want = &reference["callings"]["basic_warrior"];
    let got = callings::calling("basic_warrior").expect("the Basic Warrior");
    assert_eq!(u64::from(got.tier), want["tier"].as_u64().unwrap());
    assert_eq!(u64::from(got.cap), want["cap"].as_u64().unwrap());
    assert_eq!(
        u64::from(got.shapeshift_form),
        want["client_shapeshift_form"].as_u64().unwrap()
    );

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
        // The client refuses a cast it cannot afford at the Spell.dbc cost.
        assert!(
            f64::from(got.rage_cost) >= want["client"]["cost"].as_f64().unwrap(),
            "{key}: realm cost under the client's"
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
        assert_eq!(
            got.requires_target_dodged,
            realm.get("requires").is_some(),
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
    }
}

#[test]
fn every_skill_id_is_listed_for_rank_blanking() {
    let mut ids: Vec<u32> = callings::skill_spell_ids().collect();
    ids.sort_unstable();
    assert_eq!(ids, [78, 100, 772, 1715, 2687, 6343, 6673, 7384]);
}

#[test]
fn the_basic_warrior_talent_tree_matches_the_reference() {
    let reference = reference();
    let want = &reference["callings"]["basic_warrior"]["talent_tree"]["tabs"];
    let got = &callings::calling("basic_warrior").unwrap().talent_tabs;
    let want = want.as_array().unwrap();
    assert_eq!(got.len(), want.len(), "one tab");
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
            match (&t.client_spellmod, want.get("client_spellmod")) {
                (None, None) => {}
                (Some(m), Some(w)) => {
                    assert_eq!(m.flat, w["type"] == "flat", "{key}");
                    // vmangos `SpellModOp`.
                    let op = match w["op"].as_str().unwrap() {
                        "COST" => 14,
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
                (m, w) => panic!("{key}: spell mod {m:?} against {w:?}"),
            }
            assert!(
                !t.effects.is_empty(),
                "{key}: the realm decides what it does"
            );
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
