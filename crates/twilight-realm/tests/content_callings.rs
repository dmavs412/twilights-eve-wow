//! `content/callings.toml` still says what `data/twilights-eve/callings.json` says: the realm half
//! of every skill the TOML carries, the JSON being the checked research (`extract_spells.py
//! verify`).

mod common;

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

/// The one-time stock-to-sheet table (#204), `spells`, by name.
fn old_ids() -> &'static std::collections::HashMap<String, std::collections::HashMap<u64, u64>> {
    static TABLES: std::sync::OnceLock<
        std::collections::HashMap<String, std::collections::HashMap<u64, u64>>,
    > = std::sync::OnceLock::new();
    TABLES.get_or_init(|| {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/twilights-eve/spell_id_remap.json");
        let raw: std::collections::HashMap<String, std::collections::HashMap<String, u64>> =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        raw.into_iter()
            .map(|(name, table)| {
                let table = table
                    .into_iter()
                    .map(|(old, new)| (old.parse().unwrap(), new))
                    .collect();
                (name, table)
            })
            .collect()
    })
}

/// A stock spell id the reference names, as the content names it now: the sheet's own id through
/// the one-time table (#204), or the stock id where the sheet has no row for it.
fn ours(stock: u64) -> u64 {
    old_ids()["spells"].get(&stock).copied().unwrap_or(stock)
}

/// The stock id the content's `id` stands for, for a lookup in the stock dump.
fn stock(id: u32) -> u64 {
    let id = u64::from(id);
    (1..100_000).find(|&s| ours(s) == id).unwrap_or(id)
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
            let row = &dbc[stock(s.spell_id).to_string()];
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
    let keys = [
        "seal_of_command",
        "seal_of_the_crusader",
        "judgement",
        "hammer_of_justice",
    ];
    let want: Vec<(u32, u32)> = common::spells("zealot", &keys)
        .into_iter()
        .zip([10, 10, 5, 15])
        .collect();
    assert_eq!(got, want);
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
            ours(want["spell_id"].as_u64().unwrap()),
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
            assert_eq!(
                u64::from(aura_spell_id),
                ours(want["aura_spell_id"].as_u64().unwrap())
            );
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
            assert_eq!(
                u64::from(aura_spell_id),
                ours(want["aura_spell_id"].as_u64().unwrap())
            );
            assert_eq!(duration_ms, ms(want, "duration_ms"));
        }
        Effect::Incapacitate {
            duration_ms,
            break_on_damage,
        } => {
            assert_eq!(kind, "incapacitate", "{key}");
            assert_eq!(duration_ms, ms(want, "duration_ms"));
            assert_eq!(break_on_damage, want["break_on_damage"] == true);
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
        Effect::AddComboPoints { amount, .. } => {
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
        | Effect::GroundArea { .. }
        | Effect::GrowAura { .. }
        | Effect::Absorb { .. } => {
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
        // Kicks live in the master CSV only; tests/kicks.rs reads them.
        Effect::Interrupt { .. } => panic!("{key}: the reference has no realm half for a kick"),
        // Taunts live in the master CSV only; tests/bulwark_taunt.rs reads them.
        Effect::Knockback { .. } => {
            panic!("{key}: the reference has no realm half for a knockback")
        }
        Effect::Taunt { .. } => panic!("{key}: the reference has no realm half for a taunt"),
        // Initiate's Frostbite and Ley-Line Tap live in the master CSV only (#506);
        // tests/initiate_blink.rs and tests/initiate_procs.rs read them.
        Effect::Root { .. } => panic!("{key}: the reference has no realm half for a root"),
        Effect::RefundCost { .. } => {
            panic!("{key}: the reference has no realm half for a cost refund")
        }
        // Ranger's Parting Shot lives in the master CSV only (#507); tests/ranger_movement.rs
        // reads it.
        Effect::CastSkill { .. } => {
            panic!("{key}: the reference has no realm half for a cast of another skill")
        }
    }
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
            "seal_of_iron",
            "rebuke",
            "bulwark_taunt"
        ]
    );
}

/// The fields schema #63 added reach the content: Charge's gate and cast shape, and an aura row's
/// columns. Loaded only; nothing enforces them yet.
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
    assert_eq!(
        buff.spell_id,
        common::spell("basic_warrior", "momentum_crit_1")
    );
    assert_eq!(buff.row.max_stacks, 1);
    assert_eq!(buff.row.stack_rule.as_deref(), Some("refresh"));
    assert_eq!(buff.row.unique_per.as_deref(), Some("caster"));
    assert_eq!(buff.row.dispel_type.as_deref(), Some("none"));
    assert!(buff.row.aura_positive && buff.row.cancelable);
    assert!(!buff.row.aura_hidden && !buff.row.persist_death);
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
    /// The map it stands on: Kalimdor (1) unless written.
    #[serde(default = "kalimdor")]
    map: u32,
    #[serde(default)]
    greeting: String,
}

fn kalimdor() -> u32 {
    1
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

fn trainers_on(map: u32) -> Vec<Npc> {
    #[derive(serde::Deserialize)]
    struct File {
        npc: Vec<Npc>,
    }
    toml::from_str::<File>(include_str!("../content/hub.toml"))
        .expect("hub.toml parses")
        .npc
        .into_iter()
        .filter(|n| n.role == "trainer" && n.map == map)
        .collect()
}

fn trainers() -> Vec<Npc> {
    trainers_on(kalimdor())
}

#[test]
fn the_five_tier_1_callings_carry_a_calling_choice_blurb() {
    for key in ["vanguard", "cutthroat", "initiate", "ranger", "acolyte"] {
        let calling = callings::calling(key).unwrap_or_else(|| panic!("{key}: no Calling"));
        assert!(!calling.blurb.is_empty(), "{key}: no blurb");
    }
    assert!(callings::calling("basic_warrior").unwrap().blurb.is_empty());
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

/// Every Advancement Quest the reference names is in the content as it says, and the Crossroads
/// has no other. Northshire's trainers' quests (#216) are checked against the Hub's own below.
/// Commander Aldous Wray (#378) gives the Tier 2 and 3 quests the Crossroads trainers once did.
#[test]
fn every_advancement_quest_matches_the_reference() {
    let reference = reference();
    let mut crossroads: Vec<u32> = trainers().iter().map(|t| t.entry).collect();
    crossroads.push(190071);
    let quests: Vec<Quest> = advancement_quests()
        .into_iter()
        .filter(|q| crossroads.contains(&q.giver))
        .collect();
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

/// Each Northshire trainer (#216) has one Advancement Quest, the Tier 1 quest of their Line as the
/// Crossroads trainer gives it, in their own words and naming Northshire. There is no Tier 2 row.
#[test]
fn each_northshire_trainer_has_their_lines_tier_1_quest() {
    let quests = advancement_quests();
    let crossroads: Vec<u32> = trainers().iter().map(|t| t.entry).collect();
    let northshire = trainers_on(0);
    assert_eq!(northshire.len(), 5, "one trainer for each Tier 1 Calling");
    for t in &northshire {
        let mine: Vec<&Quest> = quests.iter().filter(|q| q.giver == t.entry).collect();
        let [q] = mine[..] else {
            panic!("{}: expected one quest, found {}", t.name, mine.len());
        };
        let adv = q.advancement.as_ref().unwrap();
        assert_eq!(adv.from, "basic_warrior", "{}: a Tier 1 quest", t.name);
        assert_ne!(adv.completable, Some(false), "{}", t.name);
        let hub = quests
            .iter()
            .find(|h| {
                crossroads.contains(&h.giver)
                    && h.advancement.as_ref().is_some_and(|a| a.to == adv.to)
            })
            .unwrap_or_else(|| panic!("{}: the Crossroads has no quest into {}", t.name, adv.to));
        assert_eq!(
            (q.level, q.zone_or_sort),
            (hub.level, hub.zone_or_sort),
            "{}",
            t.name
        );
        assert_ne!(q.id, hub.id, "{}: a row of its own", t.name);
        assert_ne!(
            q.details, hub.details,
            "{}: the trainer's own voice",
            t.name
        );
        assert!(q.objectives.contains("Northshire"), "{}", t.name);
    }
}
