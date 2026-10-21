//! **The build id** (#144): which content a binary carries, so a server refuses a client whose
//! rules differ from its own. The commit is the launcher's stamp; the content is fingerprinted
//! here, every content file and the generated spells the realm compiles in.

/// Every file of content the realm compiles in. A content file added without a line here is
/// caught by the test below.
const CONTENT: &[&str] = &[
    include_str!("../content/areas.toml"),
    include_str!("../content/barrens.toml"),
    include_str!("../content/callings.toml"),
    include_str!("../content/creature_spells.toml"),
    include_str!("../content/duel.toml"),
    include_str!("../content/encounters.toml"),
    include_str!("../content/durability.toml"),
    include_str!("../content/elwynn.toml"),
    include_str!("../content/elwynn_bands.toml"),
    include_str!("../content/elwynn_overrides.toml"),
    include_str!("../content/forest_edge.toml"),
    include_str!("../content/gear.toml"),
    include_str!("../content/jasperlode.toml"),
    include_str!("../content/ward_test.toml"),
    include_str!("../content/azora.toml"),
    include_str!("../content/group.toml"),
    include_str!("../content/given_items.toml"),
    include_str!("../content/goods.toml"),
    include_str!("../content/hub.toml"),
    include_str!("../content/items.toml"),
    include_str!("../content/level_overrides.toml"),
    include_str!("../content/levels.toml"),
    include_str!("../content/mob_class_stats.toml"),
    include_str!("../content/mobs.toml"),
    include_str!("../content/northshire_abbey.toml"),
    include_str!("../content/northshire_loot.toml"),
    include_str!("../content/notoriety.toml"),
    include_str!("../content/quest_items.toml"),
    include_str!("../content/quests.toml"),
    include_str!("../content/quests_elwynn.toml"),
    include_str!("../content/retired_items.toml"),
    include_str!("../content/sealed_yard.toml"),
    include_str!("../content/gaze_hall.toml"),
    include_str!("../content/starting_outfits.toml"),
    include_str!("../content/test_grounds.toml"),
    include_str!("../content/token_vendor.toml"),
    include_str!("../content/wailing_caverns.toml"),
    include_str!("../../../data/twilights-eve/server_spells.json"),
    include_str!("../../../data/twilights-eve/tuning.json"),
];

/// FNV-1a over [`CONTENT`], each file's length first so moving text between files counts.
pub fn content_fingerprint() -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for &b in bytes {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for file in CONTENT {
        // Line endings are a checkout's, not the content's.
        let text = file.replace("\r\n", "\n");
        eat(&(text.len() as u64).to_le_bytes());
        eat(text.as_bytes());
    }
    h
}

/// The id a client and a server compare: `<commit>+<content fingerprint>`.
pub fn build_id(commit: &str) -> String {
    let commit = if commit.is_empty() { "unknown" } else { commit };
    format!("{commit}+{:016x}", content_fingerprint())
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_content_file_is_fingerprinted() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/content");
        let on_disk = std::fs::read_dir(dir).unwrap().count();
        assert_eq!(
            super::CONTENT.len(),
            on_disk + 2,
            "a content file is missing from CONTENT"
        );
    }
}
