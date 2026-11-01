//! The Noise Bar (`assets/ui/NoiseBar.xml`, spec #533) over a pushed meter: the bar and its
//! colour, the cause line, and the Boss Bar below it. It needs no game data, so a missing font only
//! warns.

use benilla_ui::script::{PhaseMeterView, ScriptValue, UiScript, UnitState};

use super::test_ui::load_ui as load_xml;

fn bar() -> UiScript {
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1024.0, 768.0);
    // A bare VM has no stock `UIParent`; both bars anchor to it, so a stand-in covers the screen.
    s.run(r#"local up = CreateFrame("Frame", "UIParent") up:SetAllPoints()"#)
        .unwrap();
    load_xml(&s, "BossBar.xml");
    load_xml(&s, "NoiseBar.xml");
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    s
}

/// The app's push: the meter, then `BENILLA_PHASE_METER` with the cause line or nil.
fn push(s: &mut UiScript, meter: Option<(u32, u32)>, rose: Option<&str>) {
    s.set_phase_meter(meter.map(|(stacks, max)| PhaseMeterView {
        name: "Noise".into(),
        stacks,
        max,
        end_icon: r"Interface\Benilla\noise\hogger_head".into(),
    }));
    let arg = rose.map_or(ScriptValue::Nil, |r| ScriptValue::Str(r.into()));
    s.fire_event("BENILLA_PHASE_METER", vec![arg]);
}

/// One app tick of `dt` seconds: OnUpdate, then resolve.
fn frame(s: &mut UiScript, dt: f32) {
    s.tick(dt);
    s.resolve();
}

fn visible(s: &UiScript, frame: &str) -> bool {
    s.eval::<bool>(&format!("return {frame}:IsVisible() and true or false"))
        .unwrap()
}

/// The meter bar's `(value, min, max)`.
fn meter(s: &UiScript) -> (f64, f64, f64) {
    let v: Vec<f64> = s
        .eval(
            "local lo, hi = BenillaNoiseBarMeter:GetMinMaxValues() \
             return { BenillaNoiseBarMeter:GetValue(), lo, hi }",
        )
        .unwrap();
    (v[0], v[1], v[2])
}

fn colour(s: &UiScript) -> [f64; 3] {
    let c: Vec<f64> = s
        .eval(
            "local r, g, b = BenillaNoiseBarMeter:GetStatusBarColor() \
             return { r, g, b }",
        )
        .unwrap();
    [c[0], c[1], c[2]]
}

fn assert_colour(actual: [f64; 3], wanted: [f64; 3], what: &str) {
    for (a, w) in actual.iter().zip(wanted) {
        assert!(
            (a - w).abs() < 1e-3,
            "{what}: {actual:?}, expected {wanted:?}"
        );
    }
}

/// Every FontString's text under `frame`, its children's included.
fn font_texts(s: &UiScript, frame: &str) -> Vec<String> {
    s.eval(&format!(
        "local out = {{}} \
         local function walk(f) \
           for _, r in ipairs({{ f:GetRegions() }}) do \
             if r:GetObjectType() == 'FontString' then table.insert(out, r:GetText() or '') end \
           end \
           for _, c in ipairs({{ f:GetChildren() }}) do walk(c) end \
         end \
         walk({frame}) \
         return out"
    ))
    .unwrap()
}

fn boss(name: &str) -> UnitState {
    UnitState {
        exists: true,
        has_object: true,
        name: Some(name.to_string()),
        health: 100,
        max_health: 100,
        is_connected: true,
        ..UnitState::default()
    }
}

/// A meter at 10 of 40 shows the bar at 10 of 40, and nothing on the frame is a number.
#[test]
fn a_meter_shows_its_stacks_with_no_number() {
    let mut s = bar();
    push(&mut s, Some((10, 40)), None);
    frame(&mut s, 0.05);
    assert!(visible(&s, "BenillaNoiseBar"), "a meter is up, yet no bar");
    assert_eq!(meter(&s), (10.0, 0.0, 40.0));
    let texts = font_texts(&s, "BenillaNoiseBar");
    assert!(
        texts.iter().any(|t| t == "Noise"),
        "the meter's name is not on the frame: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.chars().any(|c| c.is_ascii_digit())),
        "a FontString of the Noise Bar holds a number: {texts:?}"
    );
}

/// At 0 of 40 the bar is up and empty; with no meter it is down.
#[test]
fn an_empty_meter_shows_and_no_meter_hides() {
    let mut s = bar();
    frame(&mut s, 0.05);
    assert!(!visible(&s, "BenillaNoiseBar"), "no meter, yet a bar");

    push(&mut s, Some((0, 40)), None);
    frame(&mut s, 0.05);
    assert!(visible(&s, "BenillaNoiseBar"), "an empty meter, yet no bar");
    assert_eq!(meter(&s).0, 0.0);

    push(&mut s, None, None);
    frame(&mut s, 0.05);
    assert!(
        !visible(&s, "BenillaNoiseBar"),
        "the meter came down, yet the bar shows"
    );
}

/// The bar is yellow at 0 and red at the meter's max.
#[test]
fn the_colour_runs_from_yellow_to_red() {
    let mut s = bar();
    push(&mut s, Some((0, 40)), None);
    frame(&mut s, 0.05);
    assert_colour(colour(&s), [1.0, 0.82, 0.0], "at 0");

    push(&mut s, Some((40, 40)), Some("A gnoll falls"));
    frame(&mut s, 0.05);
    assert_colour(colour(&s), [1.0, 0.1, 0.1], "at 40");

    push(&mut s, Some((20, 40)), None);
    frame(&mut s, 0.05);
    assert_colour(colour(&s), [1.0, 0.46, 0.05], "halfway");
}

/// A rise shows its cause line under the bar, which is gone 4 s later. The glow pulses for 0.6 s.
#[test]
fn a_rise_shows_its_cause_line_for_four_seconds() {
    let mut s = bar();
    push(&mut s, Some((4, 40)), None);
    frame(&mut s, 0.05);
    assert!(
        !visible(&s, "BenillaNoiseBarCause"),
        "no rise, yet a cause line"
    );
    assert!(!visible(&s, "BenillaNoiseBarGlow"), "no rise, yet a glow");

    push(&mut s, Some((5, 40)), Some("A gnoll falls"));
    frame(&mut s, 0.05);
    assert!(
        visible(&s, "BenillaNoiseBarCause"),
        "a rise, yet no cause line"
    );
    assert_eq!(
        s.eval::<String>("return BenillaNoiseBarCause:GetText()")
            .unwrap(),
        "A gnoll falls"
    );
    assert!(visible(&s, "BenillaNoiseBarGlow"), "a rise, yet no glow");

    frame(&mut s, 0.6);
    assert!(
        !visible(&s, "BenillaNoiseBarGlow"),
        "the glow outlived its 0.6 s"
    );
    frame(&mut s, 2.4);
    assert!(
        visible(&s, "BenillaNoiseBarCause"),
        "3 s after the rise, the line is gone"
    );
    frame(&mut s, 1.0);
    assert!(
        !visible(&s, "BenillaNoiseBarCause"),
        "4 s after the rise, the line still shows"
    );
}

/// The Boss Bar sits 4 px under the Noise Bar while it is up, and back at the top without it.
#[test]
fn the_boss_bar_sits_below_the_noise_bar() {
    let mut s = bar();
    s.set_unit("boss1", Some(boss("Hogger")));
    push(&mut s, Some((10, 40)), None);
    frame(&mut s, 0.05);
    assert!(visible(&s, "BenillaBossRow1"), "boss1 is up, yet no row");
    let (noise_bottom, boss_top): (f64, f64) = s
        .eval("return BenillaNoiseBar:GetBottom(), BenillaBossBar:GetTop()")
        .unwrap();
    assert!(
        boss_top < noise_bottom,
        "the Boss Bar's top {boss_top} is not below the Noise Bar's bottom {noise_bottom}"
    );
    assert!((noise_bottom - boss_top - 4.0).abs() < 0.01);

    push(&mut s, None, None);
    frame(&mut s, 0.05);
    let (screen_top, boss_top): (f64, f64) = s
        .eval("return UIParent:GetTop(), BenillaBossBar:GetTop()")
        .unwrap();
    assert!(
        (screen_top - boss_top - 40.0).abs() < 0.01,
        "with no Noise Bar the Boss Bar's top is {} below the screen top, not 40",
        screen_top - boss_top
    );
}
