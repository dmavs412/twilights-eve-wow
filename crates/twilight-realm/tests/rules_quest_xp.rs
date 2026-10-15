//! A Repeatable Quest's XP: the player's next level's worth, shrunk by vanilla's quest-level
//! falloff (`docs/reference/vanilla-combat-rules.md` §10).

use twilight_realm::rules::quest::reward_xp;

#[test]
fn a_quest_at_or_below_the_players_level_plus_five_is_worth_a_full_level() {
    // (player, quest, xp), §10's vector table.
    let vectors = [(10, 10, 7600), (8, 10, 5400), (15, 10, 14400), (2, 2, 900)];
    for (player, quest, xp) in vectors {
        assert_eq!(
            reward_xp(player, quest),
            xp,
            "player {player}, quest {quest}"
        );
    }
}

#[test]
fn above_five_levels_the_reward_falls_off() {
    let vectors = [
        (16, 10, 12800),
        (17, 10, 10620),
        (18, 10, 7760),
        (19, 10, 4260),
        (20, 10, 2320),
        (25, 10, 3400),
    ];
    for (player, quest, xp) in vectors {
        assert_eq!(
            reward_xp(player, quest),
            xp,
            "player {player}, quest {quest}"
        );
    }
}

#[test]
fn the_falloff_counts_levels_above_the_quest() {
    // The content's own quests: a level 9 on a level 2 quest (+7), a level 23 on a level 17 (+6).
    assert_eq!(reward_xp(9, 2), 3900);
    assert_eq!(reward_xp(23, 17), 23520);
    assert_eq!(reward_xp(12, 2), 1010);
}

#[test]
fn at_the_end_of_the_game_a_quest_gives_nothing() {
    assert_eq!(reward_xp(60, 60), 0);
}
