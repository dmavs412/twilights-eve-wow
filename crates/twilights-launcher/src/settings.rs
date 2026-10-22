use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Live,
    Testing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Solo,
    Join,
}

/// `launcher.toml` beside the Launcher exe. A missing file or a missing key takes the default.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The 1.12.1 Data folder; empty until picked.
    pub data: String,
    pub channel: Channel,
    pub mode: Mode,
    /// `host:port`, used when `mode` is `join`.
    pub server: String,
    pub join_password: String,
}

impl Settings {
    pub fn load(path: &Path) -> Result<Settings> {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|e| Error::Parse(e.to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self).map_err(|e| Error::Parse(e.to_string()))?;
        std::fs::write(path, text)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swap::tests::scratch_dir;

    #[test]
    fn round_trip() {
        let dir = scratch_dir("settings-round");
        let path = dir.join("launcher.toml");
        let s = Settings {
            data: r"C:\Games\WoW\Data".into(),
            channel: Channel::Testing,
            mode: Mode::Join,
            server: "10.0.0.2:8085".into(),
            join_password: "pw".into(),
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), s);
    }

    #[test]
    fn missing_file_is_default() {
        let dir = scratch_dir("settings-missing");
        let s = Settings::load(&dir.join("launcher.toml")).unwrap();
        assert_eq!(s, Settings::default());
        assert_eq!(s.channel, Channel::Live);
        assert_eq!(s.mode, Mode::Solo);
    }

    #[test]
    fn partial_file_fills_defaults() {
        let dir = scratch_dir("settings-partial");
        let path = dir.join("launcher.toml");
        std::fs::write(&path, "channel = \"testing\"\n").unwrap();
        let s = Settings::load(&path).unwrap();
        assert_eq!(s.channel, Channel::Testing);
        assert_eq!(s.mode, Mode::Solo);
        assert_eq!(s.data, "");
    }
}
