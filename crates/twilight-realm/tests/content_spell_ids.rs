//! Every spell id the realm's content names is a row of the client's spell catalog (`Spell.dbc`
//! with the client patch applied, ADR 0011), under the name the content gives it beside the id. An id chosen from memory that is off by one names some
//! other spell, and the client shows that spell's name, icon and tooltip; this is what catches it.
//!
//! Each id key has its name key in the same table ([`ID_KEYS`]); a new key with "spell" in it
//! fails here until it is taught its pair. `cargo run -p benilla-formats --example spell_names --
//! <id>...` prints the name and rank an id has. Data-gated: it skips without a WoW install.

use std::collections::BTreeSet;
use std::path::PathBuf;

use benilla_formats::SpellCatalog;

/// An id key, the name key beside it, and how the ids it holds are laid out.
const ID_KEYS: &[(&str, &str, Ids)] = &[
    ("spell_id", "spell_name", Ids::One),
    ("aura_spell_id", "aura_spell_name", Ids::One),
    ("strike_spell_id", "strike_spell_name", Ids::One),
    ("proc_spell_id", "proc_spell_name", Ids::One),
    ("judgement_spell_id", "judgement_spell_name", Ids::One),
    ("use_spell", "use_spell_name", Ids::One),
    // Every rank's aura is the one spell's name.
    ("aura_spell_ids", "aura_spell_name", Ids::SameName),
    ("equip_spells", "equip_spell_names", Ids::Parallel),
];

/// Keys with "spell" in them that hold no spell id.
const NOT_IDS: &[&str] = &["client_spellmods"];

#[derive(Clone, Copy)]
enum Ids {
    /// One id, one name.
    One,
    /// A list of ids, all under the one name.
    SameName,
    /// A list of ids and a list of names, pairwise.
    Parallel,
}

/// One id the content names: where, the id and the name it says.
struct Named {
    at: String,
    id: u32,
    name: String,
}

#[test]
fn every_content_spell_id_is_the_spell_its_name_says() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut chain = benilla_formats::open_chain(&data).expect("open the patch chain");
    let mut spells = benilla_formats::load_spell_catalog(&mut chain).expect("load Spell.dbc");
    spells
        .apply_dbc_patch(benilla_formats::CLIENT_DBC_PATCH)
        .expect("the client patch parses");

    let named = content_spell_ids();
    let ids: BTreeSet<u32> = named.iter().map(|n| n.id).collect();
    // Callings' skills and procs, the Hearthstone, food and drink, equip spells.
    assert!(ids.len() > 170, "only {} ids found", ids.len());

    let wrong: Vec<String> = named.iter().filter_map(|n| mismatch(n, &spells)).collect();
    assert!(
        wrong.is_empty(),
        "{} of {} content spell ids disagree with Spell.dbc:\n{}",
        wrong.len(),
        named.len(),
        wrong.join("\n")
    );
}

/// What is wrong with one pair, or `None`.
fn mismatch(n: &Named, spells: &SpellCatalog) -> Option<String> {
    let Some(row) = spells.get(n.id) else {
        return Some(format!("{}: {} is not in Spell.dbc", n.at, n.id));
    };
    let rank = row.rank.as_deref().unwrap_or("");
    if row.name != n.name {
        return Some(format!(
            "{}: {} is {:?} {rank:?} in Spell.dbc, not {:?}",
            n.at, n.id, row.name, n.name
        ));
    }
    None
}

/// Every id in every `content/*.toml`, paired with its name.
fn content_spell_ids() -> Vec<Named> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("content");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for path in files {
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        let value: toml::Value = toml::from_str(&std::fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{file}: {e}"));
        walk(&value, &file, &mut out);
    }
    out
}

fn walk(v: &toml::Value, at: &str, out: &mut Vec<Named>) {
    match v {
        toml::Value::Table(t) => {
            pairs(t, at, out);
            for (k, v) in t {
                walk(v, &format!("{at}.{k}"), out);
            }
        }
        toml::Value::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                walk(v, &format!("{at}[{i}]"), out);
            }
        }
        _ => {}
    }
}

/// The id-name pairs of one table.
fn pairs(t: &toml::Table, at: &str, out: &mut Vec<Named>) {
    let at = match t.get("key").or(t.get("entry")).or(t.get("kind")) {
        Some(k) => format!("{at} ({k})"),
        None => at.to_owned(),
    };
    for key in t.keys() {
        let is_name = ID_KEYS.iter().any(|(_, name, _)| name == key);
        if !key.contains("spell") || is_name || NOT_IDS.contains(&key.as_str()) {
            continue;
        }
        // A table's name (`[spell]`, `[[spell]]`) holds no ids; [`walk`] reaches its rows.
        let tables = match &t[key] {
            toml::Value::Table(_) => true,
            toml::Value::Array(a) => !a.is_empty() && a.iter().all(toml::Value::is_table),
            _ => false,
        };
        if tables {
            continue;
        }
        let Some(&(_, name_key, layout)) = ID_KEYS.iter().find(|(id, _, _)| id == key) else {
            panic!("{at}: `{key}` is new; teach this test whether it holds spell ids");
        };
        let name = t
            .get(name_key)
            .unwrap_or_else(|| panic!("{at}: `{key}` has no `{name_key}` beside it"));
        let id = |v: &toml::Value| {
            u32::try_from(v.as_integer().unwrap_or_else(|| panic!("{at}: `{key}`"))).unwrap()
        };
        let text = |v: &toml::Value| {
            v.as_str()
                .unwrap_or_else(|| panic!("{at}: `{name_key}` is not a string"))
                .to_owned()
        };
        let one = |id: u32, name: String| Named {
            at: format!("{at}.{key}"),
            id,
            name,
        };
        match layout {
            Ids::One => out.push(one(id(&t[key]), text(name))),
            Ids::SameName => {
                for v in t[key].as_array().unwrap() {
                    out.push(one(id(v), text(name)));
                }
            }
            Ids::Parallel => {
                let (ids, names) = (t[key].as_array().unwrap(), name.as_array().unwrap());
                assert_eq!(ids.len(), names.len(), "{at}: `{key}` and `{name_key}`");
                for (v, n) in ids.iter().zip(names) {
                    out.push(one(id(v), text(n)));
                }
            }
        }
    }
}
