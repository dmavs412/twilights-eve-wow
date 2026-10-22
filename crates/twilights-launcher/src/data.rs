use std::path::{Path, PathBuf};

pub const REQUIRED: [&str; 5] = [
    "dbc.MPQ",
    "model.MPQ",
    "patch.MPQ",
    "terrain.MPQ",
    "texture.MPQ",
];

/// Accepts the WoW folder (uses its `Data` subfolder) or the `Data` folder itself. Returns the
/// resolved `Data` path, or the required files it lacks. File names match case-insensitively.
pub fn check(picked: &Path) -> Result<PathBuf, Vec<String>> {
    let data = match find_entry(picked, "Data") {
        Some(sub) if sub.is_dir() => sub,
        _ => picked.to_path_buf(),
    };
    let missing: Vec<String> = REQUIRED
        .iter()
        .filter(|name| find_entry(&data, name).is_none())
        .map(|name| name.to_string())
        .collect();
    if missing.is_empty() {
        Ok(data)
    } else {
        Err(missing)
    }
}

fn find_entry(dir: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swap::tests::scratch_dir;

    fn fill(dir: &Path, names: &[&str]) {
        std::fs::create_dir_all(dir).unwrap();
        for n in names {
            std::fs::write(dir.join(n), b"x").unwrap();
        }
    }

    #[test]
    fn wow_folder_resolves_to_data() {
        let root = scratch_dir("data-wow");
        fill(&root.join("Data"), &REQUIRED);
        assert_eq!(check(&root).unwrap(), root.join("Data"));
    }

    #[test]
    fn data_folder_itself() {
        let root = scratch_dir("data-itself");
        fill(&root, &REQUIRED);
        assert_eq!(check(&root).unwrap(), root);
    }

    #[test]
    fn lowercase_names() {
        let root = scratch_dir("data-lower");
        fill(
            &root,
            &[
                "dbc.mpq",
                "model.mpq",
                "patch.mpq",
                "terrain.mpq",
                "texture.mpq",
            ],
        );
        assert!(check(&root).is_ok());
    }

    #[test]
    fn missing_files_are_listed() {
        let root = scratch_dir("data-missing");
        fill(&root, &["dbc.MPQ", "patch.MPQ"]);
        let missing = check(&root).unwrap_err();
        assert_eq!(missing, vec!["model.MPQ", "terrain.MPQ", "texture.MPQ"]);
    }
}
