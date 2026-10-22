//! Quest XP. A Repeatable Quest's is the player's next level's worth, any other quest's half of
//! its own level's. Neither shrinks as the player outlevels the quest (ADR 0012).

use twilight_realm::rules::quest::{hunting_reward_xp, min_tier_for_level, reward_xp};

#[test]
fn a_repeatable_quest_is_worth_the_players_next_level() {
    // (player, xp), the level table.
    let vectors = [(10, 7600), (8, 5400), (15, 14400), (2, 900), (9, 6500)];
    for (player, xp) in vectors {
        assert_eq!(reward_xp(player), xp, "player {player}");
    }
}

#[test]
fn at_the_end_of_the_game_a_quest_gives_nothing() {
    assert_eq!(reward_xp(60), 0);
}

#[test]
fn a_hunting_quest_is_worth_half_of_its_own_levels_xp() {
    // (player, quest, xp): level 4's 2100, level 10's 7600, level 2's 900, halved; a player below
    // the quest still earns the quest's level's half.
    let vectors = [
        (4, 4, 1050),
        (10, 10, 3800),
        (2, 2, 450),
        (8, 10, 3800),
        (15, 10, 3800),
    ];
    for (player, quest, xp) in vectors {
        assert_eq!(
            hunting_reward_xp(player, quest),
            xp,
            "player {player}, quest {quest}"
        );
    }
}

#[test]
fn an_outlevelled_hunting_quest_pays_in_full() {
    assert_eq!(
        hunting_reward_xp(25, 12),
        twilight_realm::rules::xp::xp_to_next(12)
            .unwrap()
            .div_ceil(2)
    );
    assert_eq!(hunting_reward_xp(16, 10), 3800);
    assert_eq!(hunting_reward_xp(25, 3), 700);
    assert_eq!(hunting_reward_xp(60, 50), 0);
}

#[test]
fn a_quests_minimum_tier_follows_its_level() {
    let vectors = [
        (1, 0),
        (9, 0),
        (10, 1),
        (29, 1),
        (30, 2),
        (49, 2),
        (50, 3),
        (60, 3),
    ];
    for (level, tier) in vectors {
        assert_eq!(min_tier_for_level(level), tier, "level {level}");
    }
}
