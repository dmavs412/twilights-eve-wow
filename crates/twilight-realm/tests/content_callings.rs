//! The Callings' content against `data/twilights-eve/callings.json`, the checked research
//! (`extract_spells.py verify`): every Calling plays from the master CSV since #686, and the
//! Advancement Quests and trainers say what the reference says.

mod common;

use std::path::PathBuf;

use serde_json::Value;
use twilight_realm::callings;

fn reference() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/twilights-eve/callings.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Since #686 no shipped Calling plays from `content/callings.toml`, so none borrows another class's
/// spell (ADR 0003): the client rewrites no row. The old Zealot's borrowed kit is the test Zealot
/// (`common::TEST_ZEALOT`), and tests/zealot_skills.rs plays it.
#[test]
fn no_shipped_calling_borrows_a_spell() {
    let borrowed: Vec<u32> = callings::borrowed_skills().map(|s| s.spell_id).collect();
    assert!(borrowed.is_empty(), "{borrowed:?}");
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

/// A Calling's source is data: the sheet's `calling` row moves it out of the TOML, and since the
/// Zealot's (#686) none is left there.
#[test]
fn no_calling_keeps_a_table_in_the_toml() {
    let tables: toml::Table = include_str!("../content/callings.toml").parse().unwrap();
    assert!(tables.get("calling").is_none(), "{tables:?}");
    for key in ["basic_warrior", "vanguard", "zealot"] {
        assert!(callings::calling(key).is_some(), "{key}");
    }
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
