//! **Playing together** (`content/group.toml`): the tunables of threat and shared credit.

use std::sync::OnceLock;

#[derive(serde::Deserialize)]
pub(crate) struct Group {
    /// The pull seed's share of the mob's max health (#147).
    pub(crate) pull_threat_health_fraction: f32,
}

pub(crate) fn get() -> &'static Group {
    static GROUP: OnceLock<Group> = OnceLock::new();
    GROUP.get_or_init(|| {
        toml::from_str(include_str!("../content/group.toml")).expect("content/group.toml parses")
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_content_parses() {
        assert!(super::get().pull_threat_health_fraction > 0.0);
    }
}
