//! What the window shows for each event, and the `--check` output line. Pure functions.

/// One thing that happened during an update run. The worker thread sends these to the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Checking,
    /// The channel's release was found; carries its tag and patch notes.
    Release {
        tag: String,
        notes: String,
    },
    /// A download is running. `what` is the tag or "launcher".
    Downloading {
        what: String,
        done: u64,
        total: Option<u64>,
    },
    /// The Launcher would update itself. `--check` reports it and carries on.
    LauncherUpdate(String),
    /// The game is current.
    Ready(String),
    /// The check failed: no network, an HTTP error or a bad manifest.
    CheckFailed(String),
    /// The target exe is running, so the rename failed.
    InUse,
    /// The download's size or hash differs from the manifest.
    Damaged,
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Normal,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub text: String,
    pub tone: Tone,
    pub play_enabled: bool,
    pub retry: bool,
}

fn status(text: impl Into<String>, tone: Tone, play_enabled: bool, retry: bool) -> Status {
    Status {
        text: text.into(),
        tone,
        play_enabled,
        retry,
    }
}

/// Status text and Play state for an event. `game_installed` is whether `game/twilights-eve.exe`
/// exists. `Release` carries no status of its own; it returns `None`.
pub fn status_for(event: &Event, game_installed: bool) -> Option<Status> {
    Some(match event {
        Event::Release { .. } => return None,
        Event::Checking => status("Checking for updates…", Tone::Normal, false, false),
        Event::Downloading { what, .. } if what == "launcher" => {
            status("Updating launcher…", Tone::Normal, false, false)
        }
        Event::Downloading { what, .. } => {
            status(format!("Downloading {what}…"), Tone::Normal, false, false)
        }
        Event::LauncherUpdate(_) => status("Updating launcher…", Tone::Normal, false, false),
        Event::Ready(_) => status("Ready", Tone::Normal, true, false),
        Event::CheckFailed(_) => status(
            "Couldn't check for updates",
            Tone::Warning,
            game_installed,
            false,
        ),
        Event::InUse => status(
            "Close Twilight's Eve to finish patching",
            Tone::Error,
            false,
            true,
        ),
        Event::Damaged => status("Download was damaged, try again", Tone::Error, false, true),
        Event::Failed(e) => status(e.clone(), Tone::Error, false, true),
    })
}

/// The one line `--check` prints and its exit code. `None` for an event that is not final.
pub fn check_line(event: &Event, game_installed: bool) -> Option<(String, i32)> {
    Some(match event {
        Event::Ready(tag) => (format!("ready {tag}"), 0),
        Event::CheckFailed(_) => ("offline".into(), if game_installed { 0 } else { 1 }),
        Event::InUse => ("error Close Twilight's Eve to finish patching".into(), 1),
        Event::Damaged => ("error download was damaged".into(), 1),
        Event::Failed(e) => (format!("error {e}"), 1),
        _ => return None,
    })
}

/// "12.3 MB" or "12.3 / 45.6 MB".
pub fn progress_text(done: u64, total: Option<u64>) -> String {
    let mb = |n: u64| n as f64 / 1_048_576.0;
    match total {
        Some(t) => format!("{:.1} / {:.1} MB", mb(done), mb(t)),
        None => format!("{:.1} MB", mb(done)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_enables_play() {
        let s = status_for(&Event::Ready("b-1".into()), false).unwrap();
        assert_eq!(s.text, "Ready");
        assert!(s.play_enabled);
        assert!(!s.retry);
    }

    #[test]
    fn offline_with_a_game_keeps_play() {
        let e = Event::CheckFailed("dns".into());
        let s = status_for(&e, true).unwrap();
        assert_eq!(s.text, "Couldn't check for updates");
        assert_eq!(s.tone, Tone::Warning);
        assert!(s.play_enabled);
    }

    #[test]
    fn offline_without_a_game_disables_play() {
        let s = status_for(&Event::CheckFailed("dns".into()), false).unwrap();
        assert!(!s.play_enabled);
    }

    #[test]
    fn in_use_offers_retry() {
        let s = status_for(&Event::InUse, true).unwrap();
        assert_eq!(s.text, "Close Twilight's Eve to finish patching");
        assert!(s.retry);
        assert!(!s.play_enabled);
    }

    #[test]
    fn damaged_download_offers_retry() {
        let s = status_for(&Event::Damaged, true).unwrap();
        assert_eq!(s.text, "Download was damaged, try again");
        assert!(s.retry);
    }

    #[test]
    fn launcher_update_and_game_download_text() {
        let s = status_for(&Event::LauncherUpdate("b-2".into()), true).unwrap();
        assert_eq!(s.text, "Updating launcher…");
        let d = Event::Downloading {
            what: "b-2".into(),
            done: 0,
            total: None,
        };
        assert_eq!(status_for(&d, true).unwrap().text, "Downloading b-2…");
    }

    #[test]
    fn check_lines() {
        let ready = check_line(&Event::Ready("b-1".into()), false);
        assert_eq!(ready, Some(("ready b-1".into(), 0)));
        let off = Event::CheckFailed("x".into());
        assert_eq!(check_line(&off, true), Some(("offline".into(), 0)));
        assert_eq!(check_line(&off, false), Some(("offline".into(), 1)));
        let err = check_line(&Event::Failed("boom".into()), true);
        assert_eq!(err, Some(("error boom".into(), 1)));
        assert_eq!(check_line(&Event::Checking, true), None);
    }

    #[test]
    fn progress_in_megabytes() {
        assert_eq!(progress_text(1_572_864, Some(3_145_728)), "1.5 / 3.0 MB");
        assert_eq!(progress_text(1_048_576, None), "1.0 MB");
    }
}
