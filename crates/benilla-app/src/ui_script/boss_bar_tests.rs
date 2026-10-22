//! The Boss Bar (`assets/ui/BossBar.xml`) over synthetic `boss1` to `boss3` snapshots: the rows,
//! the cast bar and the kill linger. It needs no game data, so a missing font only warns.

use benilla_ui::script::{ScriptValue, UiScript, UnitState};

use super::test_ui::load_ui as load_xml;

fn bar() -> UiScript {
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1024.0, 768.0);
    load_xml(&s, "BossBar.xml");
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    s
}

fn boss(name: &str, health: u32, max_health: u32) -> UnitState {
    UnitState {
        exists: true,
        has_object: true,
        name: Some(name.to_string()),
        health,
        max_health,
        dead: health == 0,
        is_connected: true,
        ..UnitState::default()
    }
}

/// One app tick of `dt` seconds: OnUpdate, then resolve.
fn frame(s: &mut UiScript, dt: f32) {
    s.tick(dt);
    s.resolve();
}

fn shown(s: &UiScript, row: u32) -> bool {
    s.eval::<bool>(&format!(
        "return BenillaBossRow{row}:IsVisible() and true or false"
    ))
    .unwrap()
}

fn text(s: &UiScript, frame: &str) -> String {
    s.eval::<String>(&format!("return {frame}:GetText()"))
        .unwrap()
}

fn cast_shown(s: &UiScript, row: u32) -> bool {
    s.eval::<bool>(&format!(
        "return BenillaBossRow{row}Cast:IsVisible() and true or false"
    ))
    .unwrap()
}

fn start_cast(s: &mut UiScript, token: &str, spell: &str, ms: i64) {
    s.fire_event(
        "BENILLA_BOSS_CAST",
        vec![
            ScriptValue::Str(token.into()),
            ScriptValue::Str(spell.into()),
            ScriptValue::Int(ms),
        ],
    );
}

/// A row shows while its token exists, with the name and `floor(health / max * 100)`.
#[test]
fn a_row_shows_while_its_token_exists_with_name_and_percent() {
    let mut s = bar();
    frame(&mut s, 0.05);
    assert!(
        !shown(&s, 1) && !shown(&s, 2) && !shown(&s, 3),
        "no boss, yet a row"
    );

    s.set_unit("boss1", Some(boss("Hogger", 1, 3)));
    s.set_unit("boss2", Some(boss("Brute", 250, 1000)));
    frame(&mut s, 0.05);
    assert!(shown(&s, 1) && shown(&s, 2), "two bosses, two rows");
    assert!(!shown(&s, 3), "no third boss, yet a third row");
    assert_eq!(text(&s, "BenillaBossRow1Name"), "Hogger");
    assert_eq!(text(&s, "BenillaBossRow1HealthText"), "33%");
    assert_eq!(text(&s, "BenillaBossRow2Name"), "Brute");
    assert_eq!(text(&s, "BenillaBossRow2HealthText"), "25%");
}

/// A cast pushed for `boss1` shows its cast bar with the spell name, and the bar hides when the
/// cast runs out or the feed ends it.
#[test]
fn a_cast_shows_its_bar_with_the_spell_name() {
    let mut s = bar();
    s.set_unit("boss1", Some(boss("Hogger", 100, 100)));
    s.set_unit("boss2", Some(boss("Brute", 100, 100)));
    frame(&mut s, 0.05);
    assert!(!cast_shown(&s, 1), "no cast, yet a cast bar");

    start_cast(&mut s, "boss1", "Fireball", 3000);
    frame(&mut s, 0.5);
    assert!(cast_shown(&s, 1), "a cast started, yet no cast bar");
    assert_eq!(text(&s, "BenillaBossRow1CastText"), "Fireball");
    assert!(!cast_shown(&s, 2), "boss2 never cast");

    frame(&mut s, 3.0);
    assert!(!cast_shown(&s, 1), "the cast ran out, yet its bar shows");

    start_cast(&mut s, "boss2", "Frostbolt", 3000);
    frame(&mut s, 0.1);
    assert!(cast_shown(&s, 2));
    start_cast(&mut s, "boss2", "", 0);
    frame(&mut s, 0.1);
    assert!(
        !cast_shown(&s, 2),
        "the feed ended the cast, yet its bar shows"
    );
}

/// A token that clears while its last snapshot was alive (evade, out of view) hides at once.
#[test]
fn a_boss_that_goes_away_alive_hides_at_once() {
    let mut s = bar();
    s.set_unit("boss1", Some(boss("Hogger", 100, 100)));
    s.set_unit("boss2", Some(boss("Brute", 100, 100)));
    frame(&mut s, 0.05);
    s.set_unit("boss2", None);
    frame(&mut s, 0.05);
    assert!(!shown(&s, 2), "boss2 left alive, yet its row lingers");
    assert!(shown(&s, 1));
}

/// A token that clears after a dead snapshot keeps its row two seconds at 0%.
#[test]
fn a_boss_that_goes_away_dead_lingers_two_seconds_at_zero() {
    let mut s = bar();
    s.set_unit("boss1", Some(boss("Hogger", 100, 100)));
    frame(&mut s, 0.05);
    s.set_unit("boss1", Some(boss("Hogger", 0, 100)));
    frame(&mut s, 0.05);
    assert!(shown(&s, 1));
    assert_eq!(text(&s, "BenillaBossRow1HealthText"), "0%");

    s.set_unit("boss1", None);
    frame(&mut s, 0.05);
    assert!(shown(&s, 1), "gone dead, yet the row hid at once");
    assert_eq!(text(&s, "BenillaBossRow1Name"), "Hogger");
    assert_eq!(text(&s, "BenillaBossRow1HealthText"), "0%");
    frame(&mut s, 1.5);
    assert!(shown(&s, 1), "1.6 s after the kill, the row is still up");
    frame(&mut s, 0.6);
    assert!(!shown(&s, 1), "2.1 s after the kill, the row is still up");
}
