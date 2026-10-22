//! Quest XP (spec #1, #11, #28, ADR 0012) and the Tier a quest is offered from. A Repeatable Quest
//! pays the XP the player needs for their next level, any other quest half the XP from its own
//! level to the next. Neither shrinks as the player outlevels the quest.

use super::xp::xp_to_next;

/// The lowest Tier a character's Calling must be at to be offered a quest of level `level`:
/// below 10 is Tier 0, 10 to 29 Tier 1, 30 to 49 Tier 2, 50 and up Tier 3.
pub fn min_tier_for_level(level: u8) -> u8 {
    match level {
        0..=9 => 0,
        10..=29 => 1,
        30..=49 => 2,
        _ => 3,
    }
}

/// The XP a player of level `player` earns handing in a Repeatable Quest: the XP to their next
/// level; none at the end of the game.
pub fn reward_xp(player: u8) -> u32 {
    xp_to_next(player).unwrap_or(0)
}

/// The XP a player of level `player` earns handing in a Hunting Quest, Story Quest or Bounty of
/// level `quest`: half of `quest`'s level's XP, rounded up; none at the end of the game.
pub fn hunting_reward_xp(player: u8, quest: u8) -> u32 {
    if xp_to_next(player).is_none() {
        return 0;
    }
    xp_to_next(quest).map_or(0, |full| full.div_ceil(2))
}
