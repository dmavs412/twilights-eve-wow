//! Server Unlocks (#561, ADR 0031): a free unlock is on with nothing bought, a bought one is on
//! until its `until`, and the realm logs each change at midnight UTC, at login and when the state
//! file is read again.

mod common;

use common::*;
use twilight_realm::Realm;

/// 2026-12-01T12:00:00Z.
const NOON_MS: u64 = 1_796_126_400_000;
/// 2026-12-31T23:59:59Z, a second before 2027.
const NEW_YEARS_EVE_MS: u64 = 1_798_761_599_000;

/// A realm at [`NOON_MS`] whose catalogue is the shipped one with its first row, `open_road`, for
/// sale.
fn open_road_for_sale() -> Realm {
    let text =
        include_str!("../content/unlocks.toml").replacen("\nfree = true", "\nfree = false", 1);
    let mut realm = Realm::new().with_unlock_catalogue(&text);
    realm.set_wall_clock(NOON_MS);
    realm
}

#[test]
fn with_no_state_file_the_free_unlocks_are_on() {
    let mut realm = Realm::new();
    realm.set_wall_clock(NOON_MS);
    realm.set_unlocks(None);
    assert!(realm.unlock_on("open_road"));
    assert!(realm.unlock_on("carts"));
    assert!(realm.drain_log().is_empty());
}

#[test]
fn an_unlock_for_sale_and_not_bought_is_off() {
    let mut realm = open_road_for_sale();
    realm.set_unlocks(None);
    assert!(!realm.unlock_on("open_road"));
    assert!(realm.unlock_on("carts"), "the other row stays free");
}

#[test]
fn a_bought_unlock_is_on_until_its_date() {
    let cases = [
        ("[open_road]\nuntil = \"2026-11-30T00:00:00Z\"\n", false),
        (
            "[open_road]\nuntil = \"2026-12-02T00:00:00Z\"\nbought = \"2026-11-02T18:04:00Z\"\n",
            true,
        ),
        ("[open_road]\n", true),
        // 11:00 and 13:00 UTC, either side of noon.
        (
            "[open_road]\nuntil = \"2026-12-01T13:00:00+02:00\"\n",
            false,
        ),
        ("[open_road]\nuntil = \"2026-12-01T11:00:00-02:00\"\n", true),
        // A bare TOML date-time reads as the quoted one does.
        ("[open_road]\nuntil = 2026-12-02T00:00:00Z\n", true),
    ];
    for (state, on) in cases {
        let mut realm = open_road_for_sale();
        realm.set_unlocks(Some(state));
        assert_eq!(realm.unlock_on("open_road"), on, "{state}");
        assert!(realm.drain_log().is_empty(), "{state}");
    }
}

#[test]
fn lapsing_across_midnight_utc_turns_it_off_and_logs_one_line() {
    let mut realm = open_road_for_sale();
    realm.set_wall_clock(NEW_YEARS_EVE_MS);
    realm.set_unlocks(Some("[open_road]\nuntil = \"2027-01-01T00:00:00Z\"\n"));
    assert!(realm.unlock_on("open_road"));
    assert!(realm.drain_log().is_empty());

    realm.tick(2.0, &FlatGround);
    assert!(!realm.unlock_on("open_road"));
    let log = realm.drain_log();
    assert_eq!(log.len(), 1, "{log:?}");
    assert!(log[0].contains("now off"), "{log:?}");

    realm.tick(2.0, &FlatGround);
    assert!(
        realm.drain_log().is_empty(),
        "the next check is the next midnight"
    );
}

#[test]
fn a_login_after_the_lapse_logs_it() {
    let mut realm = open_road_for_sale();
    realm.set_unlocks(Some("[open_road]\nuntil = \"2026-12-01T18:00:00Z\"\n"));
    // 19:00 the same day: no midnight crossed, so the tick checks nothing.
    realm.set_wall_clock(NOON_MS + 7 * 3_600_000);
    realm.tick(0.0, &FlatGround);
    assert!(realm.drain_log().is_empty());

    enter_new(&mut realm, "Mara", HUMAN);
    let log = realm.drain_log();
    assert_eq!(log.len(), 1, "{log:?}");
    assert!(log[0].contains("now off"), "{log:?}");
}

#[test]
fn a_purchase_read_again_takes_effect_and_logs() {
    let mut realm = open_road_for_sale();
    realm.set_unlocks(None);
    assert!(!realm.unlock_on("open_road"));

    realm.set_unlocks(Some("[open_road]\nbought = \"2026-12-01T12:00:00Z\"\n"));
    assert!(realm.unlock_on("open_road"));
    let log = realm.drain_log();
    assert_eq!(log.len(), 1, "{log:?}");
    assert!(log[0].contains("now on"), "{log:?}");
}

#[test]
fn an_unknown_key_warns_and_is_ignored() {
    let mut realm = open_road_for_sale();
    realm.set_unlocks(Some("[gold_rush]\n\n[open_road]\n"));
    assert!(realm.unlock_on("open_road"));
    assert!(!realm.unlock_on("gold_rush"));
    let log = realm.drain_log();
    assert_eq!(log.len(), 1, "{log:?}");
    assert!(log[0].contains("gold_rush"), "{log:?}");
}

#[test]
fn a_bad_date_or_file_warns_and_buys_nothing() {
    for state in [
        "[open_road]\nuntil = \"next year\"\n",
        "[open_road]\nuntil = \"2027-01-01\"\n",
        "[open_road\n",
    ] {
        let mut realm = open_road_for_sale();
        realm.set_unlocks(Some(state));
        assert!(!realm.unlock_on("open_road"), "{state}");
        let log = realm.drain_log();
        assert_eq!(log.len(), 1, "{state}: {log:?}");
    }
}
