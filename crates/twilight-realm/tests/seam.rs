//! The seam between the realm's spells and the client's: `server_spells.json` and
//! `client_dbc_patch.json`, the two files `build_spells.py` writes, read against each other, and
//! the realm's creature content read against its quests and spawns. No game data needed.
//!
//! Each check is a small function over the parsed files, so a test can feed it a fixture that
//! rebuilds the bug it catches. `build_spells.py` keeps the halves that need `Spell.dbc`
//! (`check_seam`).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

const SERVER_SPELLS: &str = include_str!("../../../data/twilights-eve/server_spells.json");
const CLIENT_PATCH: &str = include_str!("../../../data/twilights-eve/client_dbc_patch.json");

fn shipped() -> (Value, Value) {
    (
        serde_json::from_str(SERVER_SPELLS).expect("server_spells.json parses"),
        serde_json::from_str(CLIENT_PATCH).expect("client_dbc_patch.json parses"),
    )
}

fn list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

/// Every realm row that has a spell id, as `(where, row)`: a Calling's skills, its auras and each
/// rank of each talent.
fn realm_rows(server: &Value) -> Vec<(String, &Value)> {
    let mut rows = Vec::new();
    for calling in server.as_array().into_iter().flatten() {
        let c = text(calling, "key");
        for kind in ["skills", "auras"] {
            for row in list(calling, kind) {
                rows.push((format!("{c}/{}", text(row, "key")), row));
            }
        }
        for tab in list(calling, "talent_tabs") {
            for talent in list(tab, "talents") {
                for rank in list(talent, "ranks") {
                    let n = rank.get("rank").and_then(Value::as_u64).unwrap_or(0);
                    rows.push((format!("{c}/{} rank {n}", text(talent, "key")), rank));
                }
            }
        }
    }
    rows
}

/// The client's spell rows by spell id.
fn client_rows(patch: &Value) -> BTreeMap<u64, &Value> {
    list(patch, "spells")
        .iter()
        .filter_map(|s| Some((s.get("spell_id")?.as_u64()?, s)))
        .collect()
}

/// The first implicit target numbers (Spell.dbc `Targets`) a client row may carry for a realm
/// target: the single target, or the area at the caster (22).
fn client_targets(realm_target: &str) -> &'static [u64] {
    match realm_target {
        "enemy" => &[6, 22],
        "ally" => &[21, 22],
        "dest" => &[16, 22],
        _ => &[1, 22],
    }
}

/// A patched spell's client target agrees with what the realm effect needs (2a6bf0d: a Charge
/// with no target, so the client never asked for one).
fn check_targets(server: &Value, patch: &Value) -> Vec<String> {
    let client = client_rows(patch);
    let mut bad = Vec::new();
    for (at, row) in realm_rows(server) {
        let Some(target) = row.get("target").and_then(Value::as_str) else {
            continue;
        };
        let Some(client_row) = row
            .get("spell_id")
            .and_then(Value::as_u64)
            .and_then(|id| client.get(&id))
        else {
            continue;
        };
        let first = client_row["fields"]["implicit_targets"][0].as_u64();
        let wanted = client_targets(target);
        if !first.is_some_and(|t| wanted.contains(&t)) {
            bad.push(format!(
                "{at} ({}): the realm target is {target:?} but the client row's first implicit \
                 target is {first:?}, want one of {wanted:?}",
                client_row["spell_id"]
            ));
        }
    }
    bad
}

/// Whether a key names a reagent, a required tool or a totem.
fn is_requirement_key(key: &str) -> bool {
    ["reagent", "tool", "totem"].iter().any(|w| key.contains(w))
}

/// A patched spell carries no reagent, tool or totem unless the realm row asks for one (7774c8e:
/// a borrowed row's Symbol of Kings made the client refuse the cast).
fn check_reagents(server: &Value, patch: &Value) -> Vec<String> {
    let realm: BTreeMap<u64, &Value> = realm_rows(server)
        .into_iter()
        .filter_map(|(_, row)| Some((row.get("spell_id")?.as_u64()?, row)))
        .collect();
    let mut bad = Vec::new();
    for (id, client_row) in client_rows(patch) {
        let asked = realm
            .get(&id)
            .and_then(|r| r.as_object())
            .is_some_and(|r| r.keys().any(|k| is_requirement_key(k)));
        if asked {
            continue;
        }
        let Some(fields) = client_row["fields"].as_object() else {
            continue;
        };
        for (key, value) in fields.iter().filter(|(k, _)| is_requirement_key(k)) {
            let empty = value.is_null()
                || value.as_array().is_some_and(Vec::is_empty)
                || value.as_u64() == Some(0);
            if !empty {
                bad.push(format!(
                    "spell {id} ({}): the client row has {key} = {value} and the realm row asks \
                     for none",
                    text(&client_row["fields"], "name")
                ));
            }
        }
    }
    bad
}

/// One aura's effects share one buff slot, keyed by the aura's spell id; with none they each take
/// their own (0a96886: two Seal of Iron buffs from one cast).
fn check_buff_slots(server: &Value) -> Vec<String> {
    let mut bad = Vec::new();
    for calling in server.as_array().into_iter().flatten() {
        for aura in list(calling, "auras") {
            let effects = list(aura, "effects").len();
            if effects > 1 && aura.get("spell_id").is_none_or(Value::is_null) {
                bad.push(format!(
                    "{}/{}: {effects} effects and no spell id to share one buff slot",
                    text(calling, "key"),
                    text(aura, "key")
                ));
            }
        }
    }
    bad
}

/// Every rank of every talent resolves to a spell on both sides: the realm's rank row has a spell
/// id, the client's seat lists the same ids, and each is a row of the client's spells.
fn check_talent_ranks(server: &Value, patch: &Value) -> Vec<String> {
    let client = client_rows(patch);
    let mut realm: BTreeMap<(String, String), Vec<Option<u64>>> = BTreeMap::new();
    for calling in server.as_array().into_iter().flatten() {
        for tab in list(calling, "talent_tabs") {
            for talent in list(tab, "talents") {
                let ids = list(talent, "ranks")
                    .iter()
                    .map(|r| r.get("spell_id").and_then(Value::as_u64))
                    .collect();
                realm.insert(
                    (text(calling, "key").into(), text(talent, "key").into()),
                    ids,
                );
            }
        }
    }
    let mut bad = Vec::new();
    for seat in list(patch, "talents") {
        if seat.get("talent_id").is_none() {
            continue;
        }
        let at = format!("{}/{}", text(seat, "calling"), text(seat, "key"));
        let ids: Vec<Option<u64>> = list(seat, "rank_spell_ids")
            .iter()
            .map(Value::as_u64)
            .collect();
        for (i, id) in ids.iter().enumerate() {
            match id {
                None => bad.push(format!("{at}: rank {} resolves to no spell", i + 1)),
                Some(id) if !client.contains_key(id) => bad.push(format!(
                    "{at}: rank {} is spell {id}, which is not a row of the client patch",
                    i + 1
                )),
                _ => {}
            }
        }
        match realm.get(&(text(seat, "calling").into(), text(seat, "key").into())) {
            None => bad.push(format!(
                "{at}: the client seat has no talent on the realm side"
            )),
            Some(realm_ids) => {
                for (i, id) in realm_ids.iter().enumerate() {
                    if id.is_none() {
                        bad.push(format!("{at}: realm rank {} has no spell id", i + 1));
                    }
                }
                if *realm_ids != ids {
                    bad.push(format!(
                        "{at}: the realm's rank spells {realm_ids:?} differ from the client's {ids:?}"
                    ));
                }
            }
        }
    }
    bad
}

/// The creatures the realm's content defines: those with a template, and those placed in the
/// world by a spawn, a hand-written creature or a dungeon.
#[derive(Default)]
struct Creatures {
    templates: BTreeSet<u64>,
    /// Every placement as `(where, entry)`.
    spawns: Vec<(String, u64)>,
}

impl Creatures {
    fn spawned(&self, entry: u64) -> bool {
        self.spawns.iter().any(|(_, e)| *e == entry)
    }
}

fn toml_file(src: &str) -> toml::Table {
    src.parse().expect("content parses")
}

fn entries(t: &toml::Table, key: &str) -> Vec<toml::Value> {
    t.get(key)
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn int(v: &toml::Value, key: &str) -> u64 {
    v.get(key)
        .and_then(toml::Value::as_integer)
        .unwrap_or_else(|| panic!("{v}: no integer `{key}`")) as u64
}

fn shipped_creatures() -> Creatures {
    let mut c = Creatures::default();
    let barrens = toml_file(include_str!("../content/barrens.toml"));
    let elwynn = toml_file(include_str!("../content/elwynn.toml"));
    let hand = toml_file(include_str!("../content/mobs.toml"));
    let overrides = toml_file(include_str!("../content/elwynn_overrides.toml"));
    let caverns = toml_file(include_str!("../content/wailing_caverns.toml"));
    for file in [&barrens, &elwynn, &hand, &caverns] {
        for t in entries(file, "template") {
            c.templates.insert(int(&t, "entry"));
        }
    }
    // A generated row is [guid, entry, ...]; the overrides delete some Elwynn entries outright.
    let removed: BTreeSet<u64> = entries(&overrides, "removed")
        .iter()
        .map(|v| v.as_integer().unwrap() as u64)
        .collect();
    for (file, rows) in [("barrens.toml", &barrens), ("elwynn.toml", &elwynn)] {
        for row in entries(rows, "spawns") {
            let row = row.as_array().unwrap();
            let (guid, entry) = (
                row[0].as_integer().unwrap(),
                row[1].as_integer().unwrap() as u64,
            );
            if file == "elwynn.toml" && removed.contains(&entry) {
                continue;
            }
            c.spawns.push((format!("{file} guid {guid}"), entry));
        }
    }
    for (file, rows, key) in [
        ("mobs.toml", &hand, "spawn"),
        ("elwynn_overrides.toml", &overrides, "added"),
        ("wailing_caverns.toml", &caverns, "spawn"),
    ] {
        for s in entries(rows, key) {
            c.spawns
                .push((format!("{file} guid {}", int(&s, "guid")), int(&s, "entry")));
        }
    }
    for (file, src) in [
        (
            "northshire_abbey.toml",
            include_str!("../content/northshire_abbey.toml"),
        ),
        (
            "test_grounds.toml",
            include_str!("../content/test_grounds.toml"),
        ),
        (
            "forest_edge.toml",
            include_str!("../content/forest_edge.toml"),
        ),
        (
            "jasperlode.toml",
            include_str!("../content/jasperlode.toml"),
        ),
    ] {
        for s in entries(&toml_file(src), "creature") {
            c.spawns
                .push((format!("{file} guid {}", int(&s, "guid")), int(&s, "entry")));
        }
    }
    // An Encounter's waves bring creatures in when a quest step starts it (#238).
    for e in entries(
        &toml_file(include_str!("../content/encounters.toml")),
        "encounter",
    ) {
        for wave in e["wave"].as_array().unwrap() {
            for creature in wave["creatures"].as_array().unwrap() {
                c.spawns.push((
                    format!("encounters.toml {}", e["key"].as_str().unwrap()),
                    int(creature, "entry"),
                ));
            }
        }
    }
    c
}

/// Every spawn names a creature with a template; the realm skips a spawn with none, so the place
/// is empty (#186, Echo Ridge Mine).
fn check_spawn_templates(c: &Creatures) -> Vec<String> {
    c.spawns
        .iter()
        .filter(|(_, entry)| !c.templates.contains(entry))
        .map(|(at, entry)| format!("{at}: creature {entry} has no template"))
        .collect()
}

/// The creatures a quest asks the player to kill or take drops from.
fn quest_creatures(quest: &toml::Value) -> Vec<u64> {
    let mut out = Vec::new();
    for k in quest
        .get("kill")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
    {
        out.push(k[0].as_integer().unwrap() as u64);
    }
    for c in quest
        .get("collect")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
    {
        let from = |v: &toml::Value| -> Vec<u64> {
            v.get("from")
                .and_then(toml::Value::as_array)
                .into_iter()
                .flatten()
                .map(|e| e.as_integer().unwrap() as u64)
                .collect()
        };
        out.extend(from(c));
        for s in c
            .get("sources")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            out.extend(from(s));
        }
    }
    out
}

/// Every creature a quest names has a template and a spawn, so the objective can be met.
fn check_quest_creatures(quests: &[toml::Value], c: &Creatures) -> Vec<String> {
    let mut bad = Vec::new();
    for q in quests {
        let id = int(q, "id");
        for entry in quest_creatures(q) {
            if !c.templates.contains(&entry) {
                bad.push(format!("quest {id}: creature {entry} has no template"));
            } else if !c.spawned(entry) {
                bad.push(format!("quest {id}: creature {entry} is never spawned"));
            }
        }
    }
    bad
}

fn shipped_quests() -> Vec<toml::Value> {
    let mut quests = entries(&toml_file(include_str!("../content/quests.toml")), "quest");
    quests.extend(entries(
        &toml_file(include_str!("../content/quests_elwynn.toml")),
        "quest",
    ));
    quests
}

fn assert_clean(what: &str, bad: Vec<String>) {
    assert!(bad.is_empty(), "{} {what}:\n{}", bad.len(), bad.join("\n"));
}

#[test]
fn shipped_client_targets_agree_with_the_realm() {
    let (server, patch) = shipped();
    assert_clean("target disagreements", check_targets(&server, &patch));
}

#[test]
fn shipped_patch_asks_for_no_reagent() {
    let (server, patch) = shipped();
    assert_clean("unasked requirements", check_reagents(&server, &patch));
}

#[test]
fn shipped_auras_share_one_buff_slot() {
    let (server, _) = shipped();
    assert_clean("aura rows", check_buff_slots(&server));
}

#[test]
fn shipped_talent_ranks_resolve_on_both_sides() {
    let (server, patch) = shipped();
    assert_clean("talent rank problems", check_talent_ranks(&server, &patch));
}

#[test]
fn shipped_spawns_and_quests_name_creatures_that_exist() {
    let c = shipped_creatures();
    assert!(c.templates.len() > 100 && c.spawns.len() > 1000);
    assert_clean("spawns without a template", check_spawn_templates(&c));
    assert_clean(
        "quest creatures",
        check_quest_creatures(&shipped_quests(), &c),
    );
}

// Fixtures: each rebuilds the bug its check catches.

fn fixture_server(row: Value) -> Value {
    json!([{ "key": "bulwark", "skills": [row], "auras": [], "talent_tabs": [] }])
}

fn fixture_patch(spell_id: u64, fields: Value) -> Value {
    json!({ "spells": [{ "spell_id": spell_id, "fields": fields }], "talents": [] })
}

#[test]
fn a_charge_with_no_client_target_fails() {
    // 2a6bf0d: Charge borrowed the trainer's learn-spell, one Learn Spell effect, no target.
    let server = fixture_server(json!({ "key": "charge", "spell_id": 1000010, "target": "enemy" }));
    let patch = fixture_patch(
        1000010,
        json!({ "name": "Charge", "implicit_targets": [0, 0], "dbc_effects": [{ "effect": 36 }] }),
    );
    let bad = check_targets(&server, &patch);
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(bad[0].contains("bulwark/charge") && bad[0].contains("1000010"));
    let fixed = fixture_patch(1000010, json!({ "implicit_targets": [6, 0] }));
    assert!(check_targets(&server, &fixed).is_empty());
}

#[test]
fn a_patched_spell_with_a_reagent_fails() {
    // 7774c8e: Greater Blessing of Sanctuary's Symbol of Kings stayed on the borrowed row.
    let server = fixture_server(json!({ "key": "shield", "spell_id": 1000020, "target": "self" }));
    let patch = fixture_patch(
        1000020,
        json!({ "name": "Shield of the Penitent", "reagent": [21177, 0], "implicit_targets": [1, 0] }),
    );
    let bad = check_reagents(&server, &patch);
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(bad[0].contains("1000020") && bad[0].contains("Shield of the Penitent"));
    // A realm row that asks for the reagent makes it the sheet's choice.
    let asked = fixture_server(json!({ "key": "shield", "spell_id": 1000020, "reagent": 21177 }));
    assert!(check_reagents(&asked, &patch).is_empty());
}

#[test]
fn an_aura_of_two_effects_without_a_spell_id_fails() {
    // 0a96886: Seal of Iron's two procs each took their own buff slot.
    let aura =
        json!({ "key": "seal_of_iron", "effects": [{ "kind": "proc" }, { "kind": "proc" }] });
    let server = json!([{ "key": "vanguard", "skills": [], "talent_tabs": [], "auras": [aura] }]);
    let bad = check_buff_slots(&server);
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(bad[0].contains("vanguard/seal_of_iron"));
    let with_id = json!([{ "key": "vanguard", "auras": [{
        "key": "seal_of_iron", "spell_id": 1000300, "effects": [{ "kind": "proc" }, { "kind": "proc" }]
    }]}]);
    assert!(check_buff_slots(&with_id).is_empty());
}

fn talent_fixture(realm_ids: &[Value], client_ids: Value) -> (Value, Value) {
    let ranks: Vec<Value> = realm_ids
        .iter()
        .enumerate()
        .map(|(i, id)| json!({ "rank": i + 1, "spell_id": id }))
        .collect();
    let server = json!([{ "key": "bulwark", "skills": [], "auras": [], "talent_tabs": [{
        "talents": [{ "key": "iron_will", "ranks": ranks }]
    }]}]);
    let patch = json!({
        "spells": [{ "spell_id": 1000400, "fields": {} }],
        "talents": [{ "calling": "bulwark", "key": "iron_will", "talent_id": 1000500,
                      "rank_spell_ids": client_ids }]
    });
    (server, patch)
}

#[test]
fn a_talent_rank_with_no_spell_fails() {
    let (server, patch) = talent_fixture(&[json!(1000400), json!(null)], json!([1000400, null]));
    let bad = check_talent_ranks(&server, &patch);
    assert!(
        bad.iter().all(|b| b.starts_with("bulwark/iron_will")),
        "{bad:?}"
    );
    assert!(bad
        .iter()
        .any(|b| b.contains("rank 2 resolves to no spell")));
    assert!(bad
        .iter()
        .any(|b| b.contains("realm rank 2 has no spell id")));
    let (server, patch) = talent_fixture(&[json!(1000400)], json!([1000400]));
    assert!(check_talent_ranks(&server, &patch).is_empty());
}

#[test]
fn a_talent_rank_the_client_does_not_hold_fails() {
    let (server, patch) = talent_fixture(&[json!(1000401)], json!([1000401]));
    let bad = check_talent_ranks(&server, &patch);
    assert!(
        bad.iter()
            .any(|b| b.contains("1000401") && b.contains("not a row of the client patch")),
        "{bad:?}"
    );
}

#[test]
fn a_spawn_with_no_template_fails() {
    // #186: Echo Ridge Mine's creatures had spawns and no template, so the mine was empty.
    let c = Creatures {
        templates: BTreeSet::from([80]),
        spawns: vec![
            ("elwynn_overrides.toml guid 960001".into(), 80),
            ("elwynn_overrides.toml guid 960002".into(), 96001),
        ],
    };
    let bad = check_spawn_templates(&c);
    assert_eq!(
        bad,
        ["elwynn_overrides.toml guid 960002: creature 96001 has no template"]
    );
}

#[test]
fn a_quest_naming_a_missing_creature_fails() {
    let quests: Vec<toml::Value> = entries(
        &toml_file(
            "[[quest]]\nid = 94001\nkill = [[80, 5], [96001, 1]]\n\
             [[quest]]\nid = 94002\ncollect = [{ item = 1, count = 1, sources = [{ from = [80, 96002] }] }]\n\
             [[quest]]\nid = 94003\nkill = [[81, 1]]\n",
        ),
        "quest",
    );
    let c = Creatures {
        templates: BTreeSet::from([80, 81]),
        spawns: vec![("mobs.toml guid 1".into(), 80)],
    };
    let bad = check_quest_creatures(&quests, &c);
    assert_eq!(
        bad,
        [
            "quest 94001: creature 96001 has no template",
            "quest 94002: creature 96002 has no template",
            "quest 94003: creature 81 is never spawned",
        ]
    );
}
