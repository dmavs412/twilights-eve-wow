//! Writes `content/elwynn.toml`, the realm's copy of Elwynn Forest's spawns, from
//! `data/twilights-eve/elwynn_spawns.json` (every creature spawn of the zone, friendly NPCs
//! included, imported from the vmangos world DB by `tools/twilights-eve/import_barrens_spawns.py
//! --all-creatures`).
//!
//! Unlike The Barrens there are no rings and no camp regrouping: every spawn keeps its vmangos
//! place and levels and is a camp of its own. Combat stats come from the realm's tables by level,
//! as for every mob. The conversion is `import_zone.rs`'s, shared with Duskwood, Redridge and
//! Stormwind City.
//!
//! ```text
//! cargo run -p twilight-realm --example import_elwynn
//! ```
//!
//! `tests/content_elwynn.rs` fails when the committed file no longer matches the dump.
//! `docs/reference/elwynn-spawns.md` has the whole chain.

#[path = "import_zone.rs"]
#[allow(dead_code)]
mod import_zone;

use std::path::PathBuf;

use import_zone::Zone;
use serde_json::Value;

/// Elwynn Forest, whose header names its own example, test and doc.
const ELWYNN: Zone = Zone {
    name: "Elwynn Forest",
    dump: "elwynn_spawns.json",
    toml: "elwynn.toml",
    example: "import_elwynn",
    test: "tests/content_elwynn.rs",
    doc: "docs/reference/elwynn-spawns.md",
};

/// Reads the dump.
pub fn dump() -> Value {
    import_zone::dump(&ELWYNN)
}

/// The committed file's path.
pub fn output_path() -> PathBuf {
    import_zone::output_path(&ELWYNN)
}

/// `elwynn.toml` from the dump.
pub fn convert(elwynn: &Value) -> String {
    import_zone::convert(&ELWYNN, elwynn, HUB_NPCS)
}

/// The entries whose spawn moved to `content/hub.toml`, where they do more than a creature
/// template can: Lee Brown (1651) teaches Fishing and sells its goods (#327), Innkeeper Farley
/// (295) hints at a Legendary Fish (#331), and the rest are Goldshire's inn and shops (#459):
/// Melika Isenstrider (6778) keeps the inn, five shopkeepers sell and Andrew Krighton repairs.
/// After them come Goldshire's townsfolk (#459): Smith Argus (514) and sixteen others who speak a
/// line under a title the game has a use for.
const HUB_NPCS: &[u64] = &[
    1651, 295, 6778, 151, 465, 3935, 3937, 66, 2046, 514, 54, 74, 913, 927, 917, 377, 328, 2329,
    253, 906, 6374, 6121, 1430, 1632, 6306, 6749,
];

#[allow(dead_code)]
fn main() {
    let out = output_path();
    std::fs::write(&out, convert(&dump())).expect("elwynn.toml writes");
    println!("wrote {}", out.display());
}
