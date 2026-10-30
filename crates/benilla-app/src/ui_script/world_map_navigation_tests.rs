//! Navigation behavior exercised through the shipped M-key map scripts.
use super::*;

#[test]
fn surveyed_detail_appears_at_close_zoom_under_the_exploration_covers() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = canvas_harness();
    let at = centre(&s, "WorldMapZoomFrame");
    wheel(&mut s, at, 20, -1.0);
    assert!(!s
        .eval::<bool>("return BenillaMapTerrainDetail1:IsShown()")
        .unwrap());
    wheel(&mut s, at, 20, 1.0);
    assert_eq!(
        s.eval::<f64>("return BenillaMapTerrainDetail1:GetAlpha()")
            .unwrap(),
        1.0
    );
    assert!(s
        .eval::<bool>("return BenillaMapTerrainDetail280:IsShown()")
        .unwrap());
    assert!(s
        .eval::<bool>("return BenillaMapTerrainDetail1:GetParent() == BenillaCanvasPlate")
        .unwrap());
    assert_eq!(
        s.eval::<String>("return BenillaMapTerrainDetail280:GetTexture()")
            .unwrap(),
        "Interface\\Benilla\\map\\world-map-detail_280"
    );
    assert!(s.errors().is_empty(), "{:?}", s.errors());
}

#[test]
fn nearby_vendors_group_then_separate_at_their_exact_map_positions() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = canvas_harness();
    s.set_world_map_landmarks(vec![
        landmark("Smith", 67, (0.5, 0.5)),
        landmark("Provisioner", 67, (0.503, 0.5)),
    ]);
    s.run("WorldMapFrame_Update()").unwrap();
    settle(&mut s);
    assert!(s
        .eval::<bool>("return WorldMapFramePOI1:IsShown()")
        .unwrap());
    assert!(!s
        .eval::<bool>("return WorldMapFramePOI2:IsShown()")
        .unwrap());
    assert_eq!(
        s.eval::<String>("return WorldMapFramePOI1.benillaCount:GetText()")
            .unwrap(),
        "2"
    );
    s.run("this = WorldMapFramePOI1 WorldMapPOI_OnEnter() this = nil")
        .unwrap();
    assert_eq!(
        s.eval::<String>("return WorldMapTooltipTextLeft2:GetText()")
            .unwrap(),
        "Smith - Vendor"
    );
    assert_eq!(
        s.eval::<String>("return WorldMapTooltipTextLeft3:GetText()")
            .unwrap(),
        "Provisioner - Vendor"
    );
    let at = map_point(&s, 0.5, 0.5);
    wheel(&mut s, at, 20, 1.0);
    assert!(s
        .eval::<bool>("return WorldMapFramePOI2:IsShown()")
        .unwrap());
    for (pin, u) in [
        ("WorldMapFramePOI1", 0.5),
        ("WorldMapFramePOI2", f64::from(0.503_f32)),
    ] {
        assert!(near(centre(&s, pin), map_point(&s, u, 0.5)));
    }
    assert!(s.errors().is_empty(), "{:?}", s.errors());
}

#[test]
fn navigation_filters_toggle_live_quests_services_and_dungeons_independently() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = canvas_harness();
    s.set_world_map_landmarks(vec![
        landmark("Available quest", 64, (0.4, 0.4)),
        landmark("Vendor", 67, (0.5, 0.5)),
        landmark("Dungeon", 70, (0.6, 0.6)),
    ]);
    s.run("WorldMapFrame_Update()").unwrap();
    settle(&mut s);
    for n in 1..=3 {
        s.run(&format!(
            "this = BenillaMapFilter{n} this:GetScript('OnClick')() this = nil"
        ))
        .unwrap();
        for i in 1..=3 {
            let shown = s
                .eval::<bool>(&format!("return WorldMapFramePOI{i}:IsShown()"))
                .unwrap();
            assert_eq!(shown, i != n, "filter {n}, pin {i}");
        }
        s.run(&format!(
            "this = BenillaMapFilter{n} this:GetScript('OnClick')() this = nil"
        ))
        .unwrap();
    }
    // A fresh server list reuses pin frames and must not retain a previous cluster count.
    s.set_world_map_landmarks(vec![landmark("Ready to turn in", 65, (0.4, 0.4))]);
    s.run("WorldMapFrame_Update()").unwrap();
    assert!(s
        .eval::<bool>("return WorldMapFramePOI1:IsShown()")
        .unwrap());
    assert_eq!(
        s.eval::<String>("return WorldMapFramePOI1Texture:GetTexture()")
            .unwrap(),
        "Interface\\GossipFrame\\ActiveQuestIcon"
    );
    assert!(s.errors().is_empty(), "{:?}", s.errors());
}
