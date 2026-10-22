//! Audience (ADR 0014): the characters that can see a creature. A creature with one is created
//! only for them and acts only on them; a creature without one belongs to everyone.

/// The character guids that can see an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Audience(pub(crate) Vec<u64>);

impl Audience {
    pub fn of(guids: &[u64]) -> Self {
        Self(guids.to_vec())
    }

    /// Whether `viewer` is one of the characters it admits.
    pub fn admits(&self, viewer: u64) -> bool {
        self.0.contains(&viewer)
    }
}

/// Whether a creature with `audience` is seen by `viewer`: no Audience admits everyone.
pub(crate) fn admits(audience: &Option<Audience>, viewer: u64) -> bool {
    audience.as_ref().is_none_or(|a| a.admits(viewer))
}
