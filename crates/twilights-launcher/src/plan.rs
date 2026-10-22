use crate::manifest::{Manifest, LAUNCHER_EXE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    UpdateLauncher,
    PatchGame,
}

/// What the Launcher knows about the install, read from disk by the caller.
#[derive(Debug, Clone, Copy)]
pub struct Installed<'a> {
    pub game_exe_exists: bool,
    /// The installed `game/manifest.json`, `None` when missing or unreadable.
    pub manifest: Option<&'a Manifest>,
    /// SHA-256 of `std::env::current_exe()`.
    pub launcher_sha256: &'a str,
}

/// Compares the installed state with a release manifest. An empty list means up to date.
pub fn plan(installed: Installed, release: &Manifest) -> Vec<Step> {
    let mut steps = Vec::new();
    let launcher_current = release
        .files
        .get(LAUNCHER_EXE)
        .is_some_and(|e| e.sha256 == installed.launcher_sha256);
    if !launcher_current {
        steps.push(Step::UpdateLauncher);
    }
    let game_current =
        installed.game_exe_exists && installed.manifest.is_some_and(|m| m.tag == release.tag);
    if !game_current {
        steps.push(Step::PatchGame);
    }
    steps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::tests::sample;

    fn m(tag: &str) -> Manifest {
        Manifest::parse(&sample(tag, "bb")).unwrap()
    }

    #[test]
    fn game_missing() {
        let rel = m("b-1");
        let steps = plan(
            Installed {
                game_exe_exists: false,
                manifest: Some(&rel),
                launcher_sha256: "bb",
            },
            &rel,
        );
        assert_eq!(steps, vec![Step::PatchGame]);
    }

    #[test]
    fn manifest_missing() {
        let rel = m("b-1");
        let steps = plan(
            Installed {
                game_exe_exists: true,
                manifest: None,
                launcher_sha256: "bb",
            },
            &rel,
        );
        assert_eq!(steps, vec![Step::PatchGame]);
    }

    #[test]
    fn same_tag_is_nothing() {
        let rel = m("b-1");
        let steps = plan(
            Installed {
                game_exe_exists: true,
                manifest: Some(&rel),
                launcher_sha256: "bb",
            },
            &rel,
        );
        assert!(steps.is_empty());
    }

    #[test]
    fn different_tag_patches() {
        let old = m("b-1");
        let rel = m("b-2");
        let steps = plan(
            Installed {
                game_exe_exists: true,
                manifest: Some(&old),
                launcher_sha256: "bb",
            },
            &rel,
        );
        assert_eq!(steps, vec![Step::PatchGame]);
    }

    #[test]
    fn launcher_hash_differs() {
        let rel = m("b-1");
        let steps = plan(
            Installed {
                game_exe_exists: true,
                manifest: Some(&rel),
                launcher_sha256: "zz",
            },
            &rel,
        );
        assert_eq!(steps, vec![Step::UpdateLauncher]);
    }
}
