//! The Launcher's logic with no window: settings, channel lookup, manifest, hash check, file
//! swaps, data folder check and launch arguments. `github.rs` is the only module that touches the
//! network.

pub mod data;
pub mod github;
pub mod launch;
pub mod manifest;
pub mod plan;
pub mod settings;
pub mod status;
pub mod swap;
pub mod update;

use std::fmt;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Network(String),
    Parse(String),
    /// The download's size or SHA-256 differs from the manifest entry.
    Mismatch(String),
    /// The target file is in use, so the rename failed.
    InUse,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Network(e) => write!(f, "network: {e}"),
            Error::Parse(e) => write!(f, "parse: {e}"),
            Error::Mismatch(e) => write!(f, "download does not match the manifest: {e}"),
            Error::InUse => write!(f, "Close Twilight's Eve to finish patching"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
