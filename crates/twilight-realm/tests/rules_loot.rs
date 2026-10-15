//! What a kill drops: the quality spread, the items near the mob's level, and the money.

use twilight_realm::rules::loot::{money, pool, pool_level, quality, Quality, QUALITY_ROLL};

#[test]
fn most_drops_are_grey_or_white_and_one_in_ten_is_green() {
    let mut counts = [0u32; 4];
    for roll in 0..QUALITY_ROLL {
        counts[quality(roll) as usize] += 1;
    }
    assert_eq!(QUALITY_ROLL, 100);
    assert_eq!(
        counts,
        [60, 30, 10, 0],
        "grey, white, green, blue out of 100"
    );
    assert_eq!(quality(0), Quality::Poor);
    assert_eq!(quality(59), Quality::Poor);
    assert_eq!(quality(60), Quality::Common);
    assert_eq!(quality(89), Quality::Common);
    assert_eq!(quality(90), Quality::Uncommon);
    assert_eq!(quality(99), Quality::Uncommon);
}

#[test]
fn a_level_20_mob_drops_items_of_levels_18_to_22() {
    for q in [Quality::Poor, Quality::Common, Quality::Uncommon] {
        let items = pool(20, q);
        assert!(!items.is_empty(), "{q:?}");
        for entry in items {
            let level = pool_level(*entry).unwrap();
            assert!((18..=22).contains(&level), "{q:?} {entry} is level {level}");
        }
    }
}

#[test]
fn the_pool_is_every_item_of_the_window_at_that_quality() {
    // Level 3 greys: the reference's pools at levels 1 to 5.
    let mut items = pool(3, Quality::Poor).to_vec();
    items.sort_unstable();
    let mut want = vec![
        1366, 1378, 2210, 2649, 2654, 5364, // level 2
        755, 1367, 1372, 1377, 2650, 3363, 4865, 7098, // level 3
        1368, 1370, 1374, 1380, 2211, 2651, 2653, 7073, 7074, 7099, 7100, 7101, // level 4
    ];
    // Levels 1 and 5.
    for entry in items.iter().filter(|e| !want.contains(e)) {
        assert!(matches!(pool_level(*entry), Some(1 | 5)), "{entry}");
    }
    want.retain(|e| !items.contains(e));
    assert!(want.is_empty(), "missing {want:?}");
}

#[test]
fn a_window_with_none_of_a_quality_widens_up_to_six_levels() {
    // The lowest world-drop green is level 10: a level 5 mob's window widens to 0..=10 for it.
    let greens = pool(5, Quality::Uncommon);
    assert!(!greens.is_empty());
    for entry in greens {
        assert_eq!(pool_level(*entry), Some(10), "{entry}");
    }
}

#[test]
fn a_quality_with_none_near_the_level_falls_back_to_the_next_one_down() {
    // Six levels either side of 3 still reach no green, so the green roll drops a white.
    assert_eq!(pool(3, Quality::Uncommon), pool(3, Quality::Common));
}

#[test]
fn money_grows_with_the_mobs_level() {
    // Copper, (min, max): L + L²/4, twice that.
    assert_eq!(money(1), (1, 2));
    assert_eq!(money(3), (5, 10));
    assert_eq!(money(10), (35, 70));
    assert_eq!(money(20), (120, 240));
    assert_eq!(money(30), (255, 510));
}

#[test]
fn a_boss_drops_from_the_band_of_the_five_levels_up_to_the_runners() {
    use twilight_realm::rules::loot::boss_band;
    // (entry, required level)
    let blues = [(1, 15), (2, 17), (3, 20), (4, 21), (5, 26), (6, 30)];
    assert_eq!(boss_band(20, &blues), [2, 3]);
    assert_eq!(boss_band(25, &blues), [4]);
    assert_eq!(boss_band(30, &blues), [5, 6]);
    // Below every band: the lowest one there is, five levels from its first blue.
    assert_eq!(boss_band(5, &blues), [1, 2]);
    // Above every band: the highest, five levels down from its last blue.
    assert_eq!(boss_band(40, &blues), [5, 6]);
    assert!(boss_band(20, &[]).is_empty());
}
