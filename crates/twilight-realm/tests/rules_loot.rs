//! What a kill drops: the quality spread, the items near the mob's level, and the money.

use twilight_realm::rules::loot::{
    band_greens, band_of, band_roll, money, pool, pool_level, quality, Pick, Quality, BAND_ROLL,
    QUALITY_ROLL,
};

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
    let greys = pool(20, Quality::Poor);
    assert!(!greys.is_empty());
    for entry in greys {
        let level = pool_level(*entry).unwrap();
        assert!((18..=22).contains(&level), "{entry} is level {level}");
    }
    // Every Gear row left the drop file (spec #448): the white pool holds only the bags and
    // goods of items.toml, and the green roll falls back to it.
    // Its window widens (to 25 today) to find any.
    let whites = pool(20, Quality::Common);
    assert!(!whites.is_empty());
    for entry in whites {
        let level = pool_level(*entry).unwrap();
        assert!((14..=26).contains(&level), "{entry} is level {level}");
    }
    assert_eq!(pool(20, Quality::Uncommon), whites);
}

#[test]
fn the_pool_is_every_item_of_the_window_at_that_quality() {
    // Level 3 greys: the reference's pools at levels 1 to 5, less the grey Gear (2210, 1380 and
    // 2211): every Gear row of pool level 12 and under left the drop file (spec #272).
    let mut items = pool(3, Quality::Poor).to_vec();
    items.sort_unstable();
    let mut want = vec![
        1366, 1378, 2649, 2654, 5364, // level 2
        755, 1367, 1372, 1377, 2650, 3363, 4865, 7098, // level 3
        1368, 1370, 1374, 2651, 2653, 7073, 7074, 7099, 7100, 7101, // level 4
    ];
    // Gear for a body part outside the ten Gear Slots is in no pool (ADR 0007).
    want.retain(|e| pool_level(*e).is_some());
    // Levels 1 and 5.
    for entry in items.iter().filter(|e| !want.contains(e)) {
        assert!(matches!(pool_level(*entry), Some(1 | 5)), "{entry}");
    }
    want.retain(|e| !items.contains(e));
    assert!(want.is_empty(), "missing {want:?}");
}

#[test]
fn a_window_with_none_of_a_quality_widens_up_to_six_levels() {
    // No Gear row is in the drop file (spec #272 for pool level 12 and under, spec #448 for the
    // rest: loot is hand-authored), so a level 10 mob's green roll finds no green Gear to widen
    // into and falls back to the white pool of items.toml.
    assert_eq!(pool(10, Quality::Uncommon), pool(10, Quality::Common));
}

#[test]
fn a_quality_with_none_near_the_level_falls_back_to_the_next_one_down() {
    // Six levels either side of 3 still reach no green, so the green roll drops a white: with no
    // white Gear under pool level 13 (spec #272), the bags of items.toml.
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

#[test]
fn the_band_roll_is_8_white_4_green_and_nothing_else() {
    let picks: Vec<Option<Pick>> = (0..BAND_ROLL).map(band_roll).collect();
    assert_eq!(picks.iter().filter(|p| **p == Some(Pick::White)).count(), 8);
    assert_eq!(picks.iter().filter(|p| **p == Some(Pick::Green)).count(), 4);
    assert_eq!(picks.iter().filter(|p| p.is_none()).count(), 88);
}

#[test]
fn northshire_greens_come_from_within_a_level_of_the_mobs_clamped_to_5_to_8() {
    let rows = band_of(6).green_rows();
    assert_eq!(rows.len(), 21);
    let levels = |level: u8| -> Vec<u8> {
        let greens = band_greens(level);
        assert!(!greens.is_empty(), "level {level}");
        let mut levels: Vec<u8> = greens
            .iter()
            .map(|e| rows.iter().find(|(entry, _)| entry == e).unwrap().1)
            .collect();
        levels.sort_unstable();
        levels.dedup();
        levels
    };
    // A level 2 mob clamps to 5 and draws the level 5 and 6 greens; a level 10 mob clamps to 8
    // and draws 7 and 8.
    assert_eq!(levels(2), [5, 6]);
    assert_eq!(levels(10), [7, 8]);
    // A level 6 mob draws levels 5, 6 and 7.
    assert_eq!(levels(6), [5, 6, 7]);
    // Every green of those levels is in the list.
    let want: Vec<u32> = rows
        .iter()
        .filter(|(_, l)| (5..=7).contains(l))
        .map(|(e, _)| *e)
        .collect();
    assert_eq!(band_greens(6), want);
}
