//! Quest XP (spec #1, #11, #28). A Repeatable Quest pays the XP the player needs for their next
//! level, a Hunting Quest half the XP from its own level to the next. Both shrink by vanilla's
//! quest-level falloff (vmangos `Quest::XPValue`) once the player is more than five levels above
//! the quest.

use super::xp::xp_to_next;

/// The share of a quest's full XP, in percent, that a player `player - quest` levels above it
/// earns: all of it up to five levels above (and at any level below), then 80/60/40/20/10%.
pub fn falloff_pct(player: u8, quest: u8) -> u32 {
    match player.saturating_sub(quest) {
        0..=5 => 100,
        6 => 80,
        7 => 60,
        8 => 40,
        9 => 20,
        _ => 10,
    }
}

/// The XP a player of level `player` earns handing in a Repeatable Quest of level `quest`, rounded
/// up; none at the end of the game.
pub fn reward_xp(player: u8, quest: u8) -> u32 {
    xp_to_next(player).map_or(0, |full| (full * falloff_pct(player, quest)).div_ceil(100))
}

/// The XP a player of level `player` earns handing in a Hunting Quest of level `quest`: half of
/// `quest`'s level's XP, with the falloff, rounded up; none at the end of the game.
pub fn hunting_reward_xp(player: u8, quest: u8) -> u32 {
    if xp_to_next(player).is_none() {
        return 0;
    }
    xp_to_next(quest).map_or(0, |full| (full * falloff_pct(player, quest)).div_ceil(200))
}
