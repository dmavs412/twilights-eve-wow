//! Forest's Edge's creatures fill the whole hollow (#316): enough of them, every one inside the
//! footprint and clear of the Portal, some in each quarter, and no two Camps on top of each other.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// The quarters split here, about the footprint's middle.
const SPLIT: [f64; 2] = [-9995.0, 605.0];
/// No creature stands this close to the Portal, and no Camp's centre this close to another's.
const PORTAL_CLEAR_YD: f64 = 15.0;
const CAMP_APART_YD: f64 = 20.0;

fn forest_edge() -> toml::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("content/forest_edge.toml");
    toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn xy(v: &toml::Value) -> [f64; 2] {
    let a = v.as_array().unwrap();
    let f = |x: &toml::Value| x.as_float().or(x.as_integer().map(|i| i as f64)).unwrap();
    [f(&a[0]), f(&a[1])]
}

fn apart(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

#[test]
fn the_riverpaw_fill_the_hollow() {
    let fe = forest_edge();
    let footprint = &fe["footprint"][0];
    let (fx, fy) = (xy(&footprint["x"]), xy(&footprint["y"]));
    let portal = xy(&fe["portal"]["position"]);
    let creatures = fe["creature"].as_array().unwrap();
    assert!(creatures.len() >= 38, "{} creatures", creatures.len());

    let mut quarters = [0; 4];
    let mut camps: BTreeMap<i64, Vec<[f64; 2]>> = BTreeMap::new();
    for c in creatures {
        let at = xy(&c["position"]);
        let guid = c["guid"].as_integer().unwrap();
        assert!(
            (fx[0]..=fx[1]).contains(&at[0]) && (fy[0]..=fy[1]).contains(&at[1]),
            "{guid} at {at:?} is outside the footprint"
        );
        assert!(
            apart(at, portal) >= PORTAL_CLEAR_YD,
            "{guid} at {at:?} crowds the Portal"
        );
        quarters[usize::from(at[0] < SPLIT[0]) * 2 + usize::from(at[1] < SPLIT[1])] += 1;
        if let Some(camp) = c.get("camp") {
            camps
                .entry(camp.as_integer().unwrap())
                .or_default()
                .push(at);
        }
    }
    assert!(
        quarters.iter().all(|&n| n > 0),
        "creatures per quarter (east north, east south, west north, west south): {quarters:?}"
    );

    let centres: Vec<(i64, [f64; 2])> = camps
        .iter()
        .map(|(&camp, at)| {
            let n = at.len() as f64;
            let sum = at
                .iter()
                .fold([0.0, 0.0], |s, p| [s[0] + p[0], s[1] + p[1]]);
            (camp, [sum[0] / n, sum[1] / n])
        })
        .collect();
    for (i, &(a, at)) in centres.iter().enumerate() {
        for &(b, bt) in &centres[i + 1..] {
            let d = apart(at, bt);
            assert!(d >= CAMP_APART_YD, "Camps {a} and {b} are {d:.1} yd apart");
        }
    }
}
