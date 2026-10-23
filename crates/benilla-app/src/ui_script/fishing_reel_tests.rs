//! The Reeling Bar (`assets/ui/FishingReel.xml`) over pushed reel views: the bar shows while a
//! view is pushed and hides without one, the end event shows its banner, and the Angler badge
//! lingers after the bar. It needs no game data, so a missing font only warns.

use benilla_ui::script::{ReelView, ScriptValue, UiScript};

use super::test_ui::load_ui as load_xml;

fn reel() -> UiScript {
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1024.0, 768.0);
    load_xml(&s, "FishingReel.xml");
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    s
}

/// One app tick of `dt` seconds: OnUpdate, then resolve.
fn frame(s: &mut UiScript, dt: f32) {
    s.tick(dt);
    s.resolve();
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

fn shown(s: &UiScript, frame: &str) -> bool {
    s.eval::<bool>(&format!("return {frame}:IsVisible() and true or false"))
        .unwrap()
}

fn text(s: &UiScript, frame: &str) -> String {
    s.eval::<String>(&format!("return {frame}:GetText() or ''"))
        .unwrap()
}

fn view() -> ReelView {
    ReelView {
        zone_bottom: 0.1,
        zone_height: 0.17,
        fish_pos: 0.18,
        progress: 0.5,
        chest_shown: true,
        chest_pos: 0.6,
        chest_fill: 0.25,
        legendary: false,
        fish_item: 6291,
        fish_in_zone: true,
        reeling: true,
    }
}

#[test]
fn a_pushed_view_shows_the_bar_and_no_view_hides_it() {
    let mut s = reel();
    frame(&mut s, 0.05);
    assert!(!shown(&s, "BenillaFishingReel"), "no bar, yet the frame");

    s.fire_event("FISHING_REEL_START", Vec::new());
    s.set_reel(Some(view()));
    frame(&mut s, 0.05);
    assert!(shown(&s, "BenillaFishingReel"), "a bar, but no frame");
    assert!(shown(&s, "BenillaFishingReelChest"), "the chest shows");
    assert!(
        shown(&s, "BenillaFishingAngler"),
        "the badge shows with the bar"
    );

    s.set_reel(None);
    frame(&mut s, 0.05);
    assert!(
        !shown(&s, "BenillaFishingReel"),
        "the bar closed, yet the frame"
    );
}

#[test]
fn the_end_event_shows_the_success_banner_and_the_badge_lingers() {
    let mut s = reel();
    s.set_angler(3, 40, 310);
    s.fire_event("ANGLER_UPDATE", Vec::new());
    s.set_reel(Some(view()));
    frame(&mut s, 0.05);
    assert_eq!(text(&s, "BenillaFishingAnglerText"), "Angler 3 · 40 / 310");

    s.set_reel(None);
    s.fire_event(
        "FISHING_REEL_END",
        vec![
            ScriptValue::Int(6291),
            ScriptValue::Bool(true),
            ScriptValue::Bool(false),
        ],
    );
    frame(&mut s, 0.05);
    assert!(shown(&s, "BenillaFishingBanner"), "no banner on the catch");
    let line = text(&s, "BenillaFishingBannerText");
    assert!(
        line.starts_with("Caught: ") && line.ends_with("Perfect!"),
        "{line}"
    );
    assert!(
        s.eval::<bool>("return BenillaFishingBanner.success")
            .unwrap(),
        "the success art"
    );
    assert!(shown(&s, "BenillaFishingAngler"), "the badge lingers");

    frame(&mut s, 2.6);
    assert!(
        !shown(&s, "BenillaFishingBanner"),
        "the banner outlived 2.5 s"
    );
    frame(&mut s, 0.5);
    assert!(!shown(&s, "BenillaFishingAngler"), "the badge outlived 3 s");
}

#[test]
fn a_lost_fish_shows_the_fail_banner() {
    let mut s = reel();
    s.fire_event(
        "FISHING_REEL_END",
        vec![
            ScriptValue::Nil,
            ScriptValue::Bool(false),
            ScriptValue::Bool(false),
        ],
    );
    frame(&mut s, 0.05);
    assert_eq!(text(&s, "BenillaFishingBannerText"), "It got away");
    assert!(!s
        .eval::<bool>("return BenillaFishingBanner.success")
        .unwrap());
}
