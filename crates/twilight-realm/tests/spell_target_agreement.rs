//! The realm's `target` and the client's Spell.dbc `implicit_targets` come from one CSV row but
//! travel in two files. When they disagree the client casts with no unit and the realm refuses,
//! or the reverse (Zealous Cleave, #249). This reads both generated files and compares them.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn load(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/twilights-eve")
        .join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The client target pairs a realm `target` may carry. `self` also allows the caster-centred area.
fn allowed(target: &str) -> Option<&'static [[i64; 2]]> {
    match target {
        "enemy" => Some(&[[6, 0]]),
        "ally" => Some(&[[21, 0]]),
        "dest" => Some(&[[16, 28]]),
        "self" => Some(&[[1, 0], [22, 15]]),
        _ => None,
    }
}

#[test]
fn realm_target_agrees_with_client_implicit_targets() {
    let client: BTreeMap<u64, (String, Vec<i64>)> = load("client_dbc_patch.json")["spells"]
        .as_array()
        .expect("client spells")
        .iter()
        .filter_map(|row| {
            let id = row["spell_id"].as_u64()?;
            let key = row["key"].as_str().unwrap_or("").to_string();
            let targets = row["fields"]["implicit_targets"]
                .as_array()?
                .iter()
                .map(|n| n.as_i64().unwrap_or(-1))
                .collect();
            Some((id, (key, targets)))
        })
        .collect();

    let mut checked = 0;
    let mut bad = Vec::new();
    for calling in load("server_spells.json")
        .as_array()
        .expect("realm callings")
    {
        for skill in calling["skills"].as_array().into_iter().flatten() {
            let (Some(id), Some(target)) = (skill["spell_id"].as_u64(), skill["target"].as_str())
            else {
                continue;
            };
            let Some(ok) = allowed(target) else {
                continue;
            };
            let key = skill["key"].as_str().unwrap_or("?");
            let Some((_, pair)) = client.get(&id) else {
                bad.push(format!(
                    "{id} {key}: realm target {target:?}, no client row"
                ));
                continue;
            };
            checked += 1;
            if !ok.iter().any(|a| a.as_slice() == pair.as_slice()) {
                bad.push(format!(
                    "{id} {key}: realm target {target:?}, client implicit_targets {pair:?}"
                ));
            }
        }
    }
    assert!(checked > 0, "no skill with a target was checked");
    assert!(
        bad.is_empty(),
        "{} mismatches:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
