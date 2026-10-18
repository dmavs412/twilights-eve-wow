//! Procs: whether a `proc` effect's trigger matches what just happened. Its chance, and what it
//! does, are the world's.

use crate::callings::ProcEvent;
use crate::rules::school::School;

/// What a proc waits for: the event, and the filters that narrow it.
#[derive(Debug, Clone, Copy)]
pub struct Trigger<'a> {
    pub on: ProcEvent,
    /// Only a cast of this skill.
    pub skill: Option<&'a str>,
    /// Only a killing blow on a target that yields experience.
    pub yields_xp: bool,
    /// Only damage in this school (`damage_school=`).
    pub school: Option<School>,
    /// How long a `moving_for` or `still_for` waits.
    pub ms: u32,
}

/// What just happened.
#[derive(Debug, Clone, Copy)]
pub struct Happened<'a> {
    pub event: ProcEvent,
    /// The skill cast, for a cast.
    pub skill: Option<&'a str>,
    /// Whether the target of a kill yielded experience.
    pub yields_xp: bool,
    /// Who a combo point from this proc lands on: the target of a finisher, or the mob the
    /// character is fighting.
    pub target: Option<u64>,
    /// The damage done, for `damage_done` and `periodic_tick`; 0 otherwise.
    pub amount: u32,
    /// The school of that damage.
    pub school: Option<School>,
    /// For `moving_for` and `still_for`: how long the player had been moving or still before
    /// this tick; `amount` is how long after it, in ms.
    pub was_ms: u32,
}

impl Trigger<'_> {
    /// Whether `happened` sets this trigger off.
    pub fn fires(&self, happened: &Happened) -> bool {
        let waited = match self.on {
            // Each further `ms` of moving, so the n-th crossing fires as the first did.
            ProcEvent::MovingFor => {
                self.ms > 0 && happened.amount / self.ms > happened.was_ms / self.ms
            }
            ProcEvent::StillFor => happened.was_ms < self.ms && happened.amount >= self.ms,
            _ => true,
        };
        self.on == happened.event
            && waited
            && self.skill.is_none_or(|s| happened.skill == Some(s))
            && (!self.yields_xp || happened.yields_xp)
            && self.school.is_none_or(|s| happened.school == Some(s))
    }
}
