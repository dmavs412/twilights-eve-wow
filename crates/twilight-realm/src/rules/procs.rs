//! Procs: whether a `proc` effect's trigger matches what just happened. Its chance, and what it
//! does, are the world's.

use crate::callings::ProcEvent;

/// What a proc waits for: the event, and the filters that narrow it.
#[derive(Debug, Clone, Copy)]
pub struct Trigger<'a> {
    pub on: ProcEvent,
    /// Only a cast of this skill.
    pub skill: Option<&'a str>,
    /// Only a killing blow on a target that yields experience.
    pub yields_xp: bool,
}

/// What just happened.
#[derive(Debug, Clone, Copy)]
pub struct Happened<'a> {
    pub event: ProcEvent,
    /// The skill cast, for a cast.
    pub skill: Option<&'a str>,
    /// Whether the target of a kill yielded experience.
    pub yields_xp: bool,
}

impl Trigger<'_> {
    /// Whether `happened` sets this trigger off.
    pub fn fires(&self, happened: &Happened) -> bool {
        self.on == happened.event
            && self.skill.is_none_or(|s| happened.skill == Some(s))
            && (!self.yields_xp || happened.yields_xp)
    }
}
