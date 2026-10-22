use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Error, Result};

pub const GAME_EXE: &str = "twilights-eve.exe";
pub const LAUNCHER_EXE: &str = "twilights-launcher.exe";
pub const SERVER_EXE: &str = "twilight-server.exe";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub tag: String,
    pub commit: String,
    pub files: BTreeMap<String, FileEntry>,
}

impl Manifest {
    /// Rejects a manifest that lacks an entry for any of the three executables.
    pub fn parse(json: &str) -> Result<Manifest> {
        let m: Manifest = serde_json::from_str(json).map_err(|e| Error::Parse(e.to_string()))?;
        for name in [GAME_EXE, LAUNCHER_EXE, SERVER_EXE] {
            if !m.files.contains_key(name) {
                return Err(Error::Parse(format!("manifest has no entry for {name}")));
            }
        }
        Ok(m)
    }

    pub fn entry(&self, name: &str) -> Result<&FileEntry> {
        self.files
            .get(name)
            .ok_or_else(|| Error::Parse(format!("manifest has no entry for {name}")))
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hex(&hasher.finalize()))
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn sample(tag: &str, launcher_sha: &str) -> String {
        format!(
            r#"{{"tag": "{tag}", "commit": "abc", "files": {{
                "twilights-eve.exe": {{"sha256": "aa", "size": 1}},
                "twilights-launcher.exe": {{"sha256": "{launcher_sha}", "size": 2}},
                "twilight-server.exe": {{"sha256": "cc", "size": 3}}
            }}}}"#
        )
    }

    #[test]
    fn parses() {
        let m = Manifest::parse(&sample("b-1", "bb")).unwrap();
        assert_eq!(m.tag, "b-1");
        assert_eq!(m.entry(LAUNCHER_EXE).unwrap().size, 2);
    }

    #[test]
    fn rejects_a_missing_file_entry() {
        let json = r#"{"tag": "b-1", "commit": "abc", "files": {
            "twilights-eve.exe": {"sha256": "aa", "size": 1},
            "twilights-launcher.exe": {"sha256": "bb", "size": 2}
        }}"#;
        assert!(Manifest::parse(json).is_err());
    }

    #[test]
    fn hex_is_lowercase() {
        assert_eq!(hex(&[0x0a, 0xff]), "0aff");
    }
}
