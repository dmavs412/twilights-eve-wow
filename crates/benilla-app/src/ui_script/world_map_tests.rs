//! The world map, the reference's own `WorldMapFrame.xml` off the player's chain, engine-only: the
//! POI pool `GetMapLandmarkInfo` feeds (the `AreaPOI.dbc` landmarks and the guard's directions
//! marker, [`crate::poi_marker`]), the unit blips, the player arrow and the full-screen quads.

use benilla_ui::script::{
    BattlefieldFlagView, BattlefieldPositionView, QuadContent, UiScript, WorldMapLandmarkView,
    ARROW_MODEL,
};

/// The map plus everything its OnLoad touches, in the shipped list's own order.
fn harness() -> UiScript {
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1024.0, 768.0);
    // A player for `WorldMapUnit_OnEnter`'s tooltip, which a cursor over the arrow's button opens.
    s.set_unit(
        "player",
        Some(benilla_ui::script::UnitState {
            exists: true,
            name: Some("Probefour".into()),
            level: 60,
            ..Default::default()
        }),
    );
    for file in [
        "Interface\\FrameXML\\GlobalStrings.lua",
        "Interface\\FrameXML\\Fonts.xml",
        "Interface\\FrameXML\\BasicControls.xml",
        "Interface\\FrameXML\\LocaleProperties.lua",
        r"Interface\FrameXML\UIParent.xml",
        r"Interface\FrameXML\MoneyFrame.lua",
        r"Interface\FrameXML\MoneyFrame.xml",
        "Interface\\FrameXML\\GameTooltip.xml",
        "Interface\\FrameXML\\UIDropDownMenu.xml", // the map's continent/zone pickers initialize into it at OnLoad
        r"Interface\FrameXML\UIPanelTemplates.lua",
        r"Interface\FrameXML\UIPanelTemplates.xml",
        "Interface\\FrameXML\\StaticPopup.xml",
        // The stock update walks `MAX_PARTY_MEMBERS` and `MAX_RAID_MEMBERS`, set by these two.
        r"Interface\FrameXML\PartyMemberFrame.lua",
        r"Interface\FrameXML\RaidFrame.lua",
        // The reference's own map, which <Include>s its blip templates itself.
        r"Interface\FrameXML\WorldMapFrame.xml",
        "ScrollTemplates.xml",
        "WorldMapMarks.xml",
        "WorldMapCanvas.xml",
        "WorldMapMist.xml",
    ] {
        super::test_ui::load_ui(&s, file);
    }
    // The map's OnShow calls `UpdateMicroButtons`, and the ping's fade reads Minimap.lua's
    // `MINIMAPPING_FADE_TIMER`; this kit loads neither the main menu bar nor the minimap.
    s.run("function UpdateMicroButtons() end MINIMAPPING_FADE_TIMER = 0.5")
        .unwrap();
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    s
}

/// The map arrow's file facts, as the app hands them over: `MinimapArrow.m2`, one looping 3.333 s
/// Stand keying no bone, header box x in [-0.0127, 0.0135], y in [-0.0118, 0.0145].
fn arrow_facts(s: &mut UiScript) {
    use benilla_ui::widget::{ModelFileFacts, SequenceFacts};
    s.set_model_facts(
        ARROW_MODEL,
        ModelFileFacts {
            sequences: vec![SequenceFacts {
                anim_id: 0,
                duration_ms: 3333,
                looping: true,
            }],
            bbox: ([-0.0127, -0.0118, 0.0], [0.0135, 0.0145, 0.0]),
            cameras: 0,
        },
    );
}

fn update(s: &mut UiScript) {
    s.run("this = WorldMapButton WorldMapButton_OnUpdate(0.1) this = nil")
        .unwrap();
}

fn landmark(name: &str, icon: u32, uv: (f32, f32)) -> WorldMapLandmarkView {
    WorldMapLandmarkView {
        name: name.into(),
        description: String::new(),
        texture_index: icon,
        uv,
    }
}

/// A landmark shows as a POI frame at its UV in its cell of the 8x8 `POIIcons` atlas. Icon 6 is
/// `ICON_POI_REDFLAG`, the flag vmangos's `points_of_interest` rows carry: the guard's directions.
#[test]
fn a_landmark_draws_its_poi_icon_at_its_map_position() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = harness();
    s.set_world_map_landmarks(vec![landmark("Stormwind Warrior Trainer", 6, (0.25, 0.5))]);
    s.run("WorldMapFrame_Update()").unwrap();

    assert_eq!(
        s.eval::<i64>("return GetNumMapLandmarks()").unwrap(),
        1,
        "the host reports the one landmark"
    );
    assert!(
        s.eval::<bool>("return WorldMapFramePOI1:IsShown()")
            .unwrap(),
        "the pool grew a frame for it and showed it"
    );

    // Cell 6 is column 6, row 0. `GetTexCoord` answers eight values (UL, LL, UR, LR as x,y
    // pairs), so the `(l, r, t, b)` rect is positions 1, 5, 2, 4.
    let (l, t, _, b, r, ..): (f32, f32, f32, f32, f32, f32, f32, f32) = s
        .eval("return WorldMapFramePOI1Texture:GetTexCoord()")
        .unwrap();
    assert_eq!(
        (l, r, t, b),
        (0.75, 0.875, 0.0, 0.125),
        "POIIcons cell 6 — the red flag"
    );

    // Seated at UV times the 1002x668 detail frame, from its TOPLEFT, y down.
    let (x, y): (f32, f32) = s
        .eval(
            "local _, _, _, ox, oy = WorldMapFramePOI1:GetPoint(1) \
             return ox, oy",
        )
        .unwrap();
    assert_eq!((x, y), (0.25 * 1002.0, -0.5 * 668.0));
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

#[test]
fn the_poi_pool_grows_and_parks_its_tail() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = harness();
    s.set_world_map_landmarks(vec![
        landmark("The Bank", 6, (0.1, 0.1)),
        landmark("The Inn", 6, (0.2, 0.2)),
        landmark("The Auction House", 6, (0.3, 0.3)),
    ]);
    s.run("WorldMapFrame_Update()").unwrap();
    assert_eq!(s.eval::<i64>("return NUM_WORLDMAP_POIS").unwrap(), 3);
    assert!(s
        .eval::<bool>("return WorldMapFramePOI3:IsShown()")
        .unwrap());

    s.set_world_map_landmarks(vec![landmark("The Inn", 6, (0.2, 0.2))]);
    s.run("WorldMapFrame_Update()").unwrap();
    assert_eq!(
        s.eval::<i64>("return NUM_WORLDMAP_POIS").unwrap(),
        3,
        "the pool never shrinks"
    );
    assert!(s
        .eval::<bool>("return WorldMapFramePOI1:IsShown()")
        .unwrap());
    assert!(
        !s.eval::<bool>("return WorldMapFramePOI2:IsShown()")
            .unwrap(),
        "slot 2 parked hidden"
    );
    assert!(!s
        .eval::<bool>("return WorldMapFramePOI3:IsShown()")
        .unwrap());
    assert_eq!(
        s.eval::<String>("return WorldMapFramePOI1.name").unwrap(),
        "The Inn",
        "slot 1 was re-seated, not left on the stale landmark"
    );
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// `WorldMapPOI_OnEnter` names the POI and sets its status line, or "" when it has none
/// (WorldMapFrame.lua:156-165), which reads back as nil.
#[test]
fn hovering_a_poi_names_it_and_adds_a_status_line_only_when_there_is_one() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = harness();
    s.set_world_map_landmarks(vec![landmark("Lion's Pride Inn", 6, (0.5, 0.5))]);
    s.run("WorldMapFrame_Update()").unwrap();
    s.run("this = WorldMapFramePOI1 this:GetScript(\"OnEnter\")() this = nil")
        .unwrap();
    assert_eq!(
        s.eval::<String>("return WorldMapFrameAreaLabel:GetText()")
            .unwrap(),
        "Lion's Pride Inn"
    );
    assert_eq!(
        s.eval::<Option<String>>("return WorldMapFrameAreaDescription:GetText()")
            .unwrap(),
        None,
        "no description → a blank line that reads back NIL: `FontString:GetText 0x79d690` \
         substitutes nil for an empty string, and Cartographer 2.02's world-map \
         hover reads exactly this as \"this POI has no status line\""
    );

    let mut with_status = landmark("Stables", 6, (0.5, 0.5));
    with_status.description = "In Conflict".into();
    s.set_world_map_landmarks(vec![with_status]);
    s.run("WorldMapFrame_Update()").unwrap();
    s.run("this = WorldMapFramePOI1 this:GetScript(\"OnEnter\")() this = nil")
        .unwrap();
    assert_eq!(
        s.eval::<String>("return WorldMapFrameAreaDescription:GetText()")
            .unwrap(),
        "In Conflict"
    );
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// `GetMapLandmarkInfo` answers the reference's five values, nil for no description (`0x4a8740`).
#[test]
fn the_landmark_getter_returns_the_references_five_values() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = harness();
    let mut with_status = landmark("Stables", 9, (0.25, 0.75));
    with_status.description = "In Conflict".into();
    s.set_world_map_landmarks(vec![landmark("Woo Ping", 6, (0.5, 0.5)), with_status]);

    let (name, desc_is_nil, icon, x, y): (String, bool, i64, f32, f32) = s
        .eval("local n, d, t, x, y = GetMapLandmarkInfo(1) return n, d == nil, t, x, y")
        .unwrap();
    assert_eq!(
        (name.as_str(), desc_is_nil, icon, x, y),
        ("Woo Ping", true, 6, 0.5, 0.5)
    );

    let desc: String = s
        .eval("local _, d = GetMapLandmarkInfo(2) return d")
        .unwrap();
    assert_eq!(
        desc, "In Conflict",
        "a landmark that HAS one still answers it"
    );
    assert!(
        s.eval::<bool>("return GetMapLandmarkInfo(3) == nil")
            .unwrap(),
        "past the end is nil"
    );
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

#[test]
fn no_landmarks_draws_nothing() {
    let _data = benilla_formats::wow_data_or_skip!();
    let s = harness();
    s.run("WorldMapFrame_Update()").unwrap();
    assert_eq!(s.eval::<i64>("return GetNumMapLandmarks()").unwrap(), 0);
    assert_eq!(s.eval::<i64>("return NUM_WORLDMAP_POIS").unwrap(), 0);
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// One blip per `party1..4`, placed from `GetPlayerMapPosition` and hidden at (0, 0): `CENTER`
/// against the detail frame's `TOPLEFT`, y by minus its height, because UV v runs down the sheet.
#[test]
fn party_blips_sit_at_their_map_positions_and_hide_when_absent() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = harness();
    // `IsShown`, not `IsVisible`: the map itself is closed in this harness.
    let shown = |s: &UiScript, i: u32| -> bool {
        s.eval::<bool>(&format!("return WorldMapParty{i}:IsShown()"))
            .unwrap()
    };

    update(&mut s);
    for i in 1..=4 {
        assert!(!shown(&s, i), "slot {i} hides while the party is empty");
    }

    // Two members on the map and one between them off it: the app pushes None for no position.
    s.set_world_map_feed(
        None,
        Some((0.5, 0.5)),
        0.0,
        None,
        vec![Some((0.25, 0.75)), None, Some((0.5, 0.125))],
        Vec::new(),
    );
    update(&mut s);
    let diag = s
        .eval::<String>(
            r#"return "p1="..tostring(GetPlayerMapPosition("party1")).." p3="..tostring(GetPlayerMapPosition("party3"))"#,
        )
        .unwrap();
    assert!(
        shown(&s, 1) && shown(&s, 3),
        "the two placed members show ({diag})"
    );
    assert!(!shown(&s, 2), "a member with no position stays hidden");
    assert!(!shown(&s, 4), "a slot past the roster's end stays hidden");

    let (w, h) = s
        .eval::<(f64, f64)>(
            "return WorldMapDetailFrame:GetWidth(), WorldMapDetailFrame:GetHeight()",
        )
        .unwrap();
    let (blip_x, blip_y) = s
        .eval::<(f64, f64)>("return WorldMapParty1:GetCenter()")
        .unwrap();
    let (sheet_left, sheet_top) = s
        .eval::<(f64, f64)>("return WorldMapDetailFrame:GetLeft(), WorldMapDetailFrame:GetTop()")
        .unwrap();
    assert!(
        (blip_x - (sheet_left + 0.25 * w)).abs() < 0.5,
        "u scales by the sheet's width"
    );
    assert!(
        (blip_y - (sheet_top - 0.75 * h)).abs() < 0.5,
        "v runs DOWN from the sheet's top"
    );

    s.set_world_map_feed(None, Some((0.5, 0.5)), 0.0, None, Vec::new(), Vec::new());
    update(&mut s);
    for i in 1..=4 {
        assert!(!shown(&s, i), "slot {i} hides when the party is gone");
    }
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// `WorldMapFrame_OnLoad` creates the arrow, an anonymous `Model` (WorldMapFrame.lua:15); the
/// update seats it with `WorldMapPlayer`, turns it to the facing and hides it off the map.
#[test]
fn the_player_arrow_is_the_stock_model_pane_seated_and_turned_by_the_update() {
    let _data = benilla_formats::wow_data_or_skip!();
    // The whole manifest: the arrow's quad needs the map visible, through `ShowUIPanel`.
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1600.0, 900.0);
    s.set_unit(
        "player",
        Some(benilla_ui::script::UnitState {
            exists: true,
            name: Some("Probefour".into()),
            level: 60,
            ..Default::default()
        }),
    );
    let failures = super::load_default_ui(&s);
    assert!(failures.is_empty(), "manifest load errors: {failures:#?}");
    arrow_facts(&mut s);
    s.resolve();
    s.run("ShowUIPanel(WorldMapFrame)").unwrap();
    s.resolve();
    s.set_world_map_feed(None, Some((0.25, 0.75)), 1.25, None, Vec::new(), Vec::new());
    update(&mut s);
    s.resolve();
    let arrow = |s: &mut UiScript| {
        s.extract().into_iter().find(|q| {
            matches!(&q.content, QuadContent::ModelPane { model: Some(m), .. } if m == ARROW_MODEL)
        })
    };
    let pane = arrow(&mut s).expect("the arrow pane is in the render list");
    assert!(
        matches!(&pane.content, QuadContent::ModelPane { facing, .. } if (*facing - 1.25).abs() < 1e-6),
        "turned to the facing: {:?}",
        pane.content
    );
    let rect = pane.rect.expect("…with a resolved rect");
    // The implicit rect is the file's box in layout units: at 16:9 a layout unit is
    // 768·√((16/9)²+1) = 1566.4 FrameXML units, so 0.0262 x 0.0263 reads 41.0 x 41.2.
    assert!(
        (rect.right - rect.left - 0.0262 * 1566.4).abs() < 0.2
            && (rect.top - rect.bottom - 0.0263 * 1566.4).abs() < 0.2,
        "sized by the file's box: {rect:?}"
    );
    // The model is re-centred every update, at half the width and height (`0x4a7b20`).
    assert!(
        matches!(&pane.content, QuadContent::ModelPane { position, .. }
            if (position.0 - 0.0131).abs() < 1e-4 && (position.1 - 0.01315).abs() < 1e-4),
        "{:?}",
        pane.content
    );
    // The arrow is anonymous: found among the map's children by kind, against `WorldMapPlayer`.
    let (dx, dy) = s
        .eval::<(f64, f64)>(
            r#"local px, py = WorldMapPlayer:GetCenter()
               for _, child in ipairs({ WorldMapFrame:GetChildren() }) do
                   if child:GetObjectType() == "Model" and child:IsShown() then
                       local ax, ay = child:GetCenter()
                       return ax - px, ay - py
                   end
               end
               return 1e9, 1e9"#,
        )
        .unwrap();
    assert!(
        dx.abs() < 0.5 && dy.abs() < 0.5,
        "seated with WorldMapPlayer ({dx}, {dy})"
    );

    // Off the displayed map: the (0, 0) sentinel hides it.
    s.set_world_map_feed(None, None, 0.0, None, Vec::new(), Vec::new());
    update(&mut s);
    s.resolve();
    assert!(arrow(&mut s).is_none(), "off-map hides the arrow");
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// Battleground teammates take the `WorldMapRaid` frames past the raid's own, named for the
/// tooltip; the flag carrier takes `WorldMapFlag1` in the token's texture; (0, 0) hides.
#[test]
fn battleground_teammates_and_the_flag_draw_from_the_position_family() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = harness();
    s.set_world_map_feed(None, Some((0.5, 0.5)), 0.0, None, Vec::new(), Vec::new());
    s.set_battlefield_positions(
        vec![
            BattlefieldPositionView {
                uv: (0.25, 0.75),
                name: Some("Probe".into()),
            },
            BattlefieldPositionView {
                uv: (0.0, 0.0),
                name: None,
            },
        ],
        Some(BattlefieldFlagView {
            uv: (0.4, 0.6),
            token: Some("HordeFlag".into()),
        }),
        1.0,
    );
    update(&mut s);
    assert!(
        s.eval::<bool>(r#"return WorldMapRaid1:IsShown() and WorldMapRaid1.name == "Probe""#)
            .unwrap(),
        "the placed teammate shows on the first raid frame with its name"
    );
    assert!(
        !s.eval::<bool>("return WorldMapRaid2:IsShown()").unwrap(),
        "the (0,0) teammate hides"
    );
    assert!(s.eval::<bool>("return WorldMapFlag1:IsShown()").unwrap());
    assert_eq!(
        s.eval::<String>("return WorldMapFlag1Texture:GetTexture()")
            .unwrap(),
        r"Interface\WorldStateFrame\HordeFlag"
    );
    assert!(!s.eval::<bool>("return WorldMapFlag2:IsShown()").unwrap());
    let (w, h) = s
        .eval::<(f64, f64)>(
            "return WorldMapDetailFrame:GetWidth(), WorldMapDetailFrame:GetHeight()",
        )
        .unwrap();
    let (sheet_left, sheet_top) = s
        .eval::<(f64, f64)>("return WorldMapDetailFrame:GetLeft(), WorldMapDetailFrame:GetTop()")
        .unwrap();
    let (bx, by) = s
        .eval::<(f64, f64)>("return WorldMapRaid1:GetCenter()")
        .unwrap();
    assert!(
        (bx - (sheet_left + 0.25 * w)).abs() < 0.5,
        "u scales by the sheet's width"
    );
    assert!(
        (by - (sheet_top - 0.75 * h)).abs() < 0.5,
        "v runs DOWN from the sheet's top"
    );

    s.set_battlefield_positions(Vec::new(), None, 1.0);
    update(&mut s);
    assert!(
        !s.eval::<bool>("return WorldMapRaid1:IsShown() or WorldMapFlag1:IsShown()")
            .unwrap(),
        "leaving the battleground hides them"
    );
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// `WorldMapFrame` is an `area = "full"` panel, so `ShowUIPanel` hides `UIParent`. What the map
/// needs while up hangs elsewhere: `BlackoutWorld` is inside the map, `WorldMapTooltip` is parented
/// to it (WorldMapFrame.xml:636), and the dropdown lists are `toplevel` with no parent.
#[test]
fn the_maps_own_furniture_survives_the_hide_that_showing_it_performs() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1600.0, 900.0);
    // A player exists before the manifest loads: the UI loads on world entry, and the macro
    // window's OnLoad formats `UnitName("player")`.
    s.set_unit(
        "player",
        Some(benilla_ui::script::UnitState {
            exists: true,
            name: Some("Probefour".into()),
            level: 60,
            ..Default::default()
        }),
    );
    let failures = super::load_default_ui(&s);
    assert!(failures.is_empty(), "manifest load errors: {failures:#?}");
    s.resolve();

    let visible = |s: &UiScript, f: &str| {
        s.eval::<i64>(&format!(
            "local x = getglobal('{f}') if not x then return -1 end return x:IsVisible() and 1 or 0"
        ))
        .unwrap()
    };

    s.run("ShowUIPanel(WorldMapFrame)").unwrap();
    s.resolve();

    assert_eq!(visible(&s, "WorldMapFrame"), 1, "the map itself is up");
    assert_eq!(
        visible(&s, "UIParent"),
        0,
        "and it took the screen from the HUD"
    );
    assert_eq!(
        visible(&s, "BlackoutWorld"),
        1,
        "the blackout is up WITH it — it is what stops the 3D world showing through the margins \
         beside the 4:3 sheet, and a blackout that hides when the map opens is no blackout"
    );

    // The map's pickers open on the shared `DropDownList1`; `ToggleDropDownMenu` ends in its
    // `Show()`. Asserted at the `Show`: the toggle bails early on an empty list.
    s.run("DropDownList1:Show()").unwrap();
    s.resolve();
    assert_eq!(
        visible(&s, "DropDownList1"),
        1,
        "the shared dropdown list can open over a full-screen panel — the continent and zone \
         pickers in the map's own header have no other seat"
    );

    s.run("CloseDropDownMenus()").unwrap();
    s.run("HideUIPanel(WorldMapFrame)").unwrap();
    s.resolve();
    assert_eq!(visible(&s, "UIParent"), 1, "and the HUD comes back after");
    assert_eq!(visible(&s, "BlackoutWorld"), 0, "with the blackout down");
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// Narrower than 4:3, `SetupFullscreenScale` scales the map under 1 (0.802 here), and
/// `WorldMapUnit_OnEnter` asks `MouseIsOver(WorldMapPlayer)` for the tooltip text
/// (WorldMapFrame.lua:473), which must divide the cursor by the scale as the reference does.
#[test]
fn hovering_the_player_blip_on_a_scaled_map_names_the_player() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(821.0, 768.0);
    s.set_unit(
        "player",
        Some(benilla_ui::script::UnitState {
            exists: true,
            name: Some("Probefour".into()),
            level: 60,
            ..Default::default()
        }),
    );
    let failures = super::load_default_ui(&s);
    assert!(failures.is_empty(), "manifest load errors: {failures:#?}");
    s.resolve();
    s.run("ShowUIPanel(WorldMapFrame)").unwrap();
    s.resolve();
    assert!(
        s.eval::<f64>("return WorldMapFrame:GetScale()").unwrap() < 0.99,
        "the narrow window scales the map under 1"
    );
    s.set_world_map_feed(None, Some((0.5, 0.5)), 0.0, None, Vec::new(), Vec::new());
    update(&mut s);
    s.resolve();
    let (px, py, eff) = s
        .eval::<(f64, f64, f64)>(
            "local x, y = WorldMapPlayer:GetCenter() return x, y, WorldMapPlayer:GetEffectiveScale()",
        )
        .unwrap();
    s.mouse_move((px * eff) as f32, (py * eff) as f32);
    assert_eq!(
        s.hit_test_name((px * eff) as f32, (py * eff) as f32)
            .as_deref(),
        Some("WorldMapPlayer")
    );
    assert!(s.eval::<bool>("return WorldMapTooltip:IsShown()").unwrap());
    assert_eq!(
        s.eval::<String>("return WorldMapTooltipTextLeft1:GetText()")
            .unwrap(),
        "Probefour"
    );
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// The zone overlays are ARTWORK textures of `WorldMapDetailFrame` (WorldMapFrame.lua:108) and the
/// arrow a `Model` child of `WorldMapFrame` (WorldMapFrame.lua:15), at the same level; a model
/// draws last in its bucket's ARTWORK batch (`0x76fb00`), so the arrow paints over them.
#[test]
fn the_player_arrow_draws_over_the_zones_explored_overlays() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1600.0, 900.0);
    s.set_unit(
        "player",
        Some(benilla_ui::script::UnitState {
            exists: true,
            name: Some("Probefour".into()),
            level: 60,
            ..Default::default()
        }),
    );
    assert!(super::load_default_ui(&s).is_empty());
    s.resolve();
    arrow_facts(&mut s);
    s.run("ShowUIPanel(WorldMapFrame)").unwrap();
    s.resolve();
    s.set_world_map_feed(None, Some((0.5, 0.5)), 0.0, None, Vec::new(), Vec::new());
    update(&mut s);
    // An overlay made the stock file's way, covering the whole detail frame.
    s.run(
        r#"local o = WorldMapDetailFrame:CreateTexture("WorldMapOverlay1", "ARTWORK")
           o:SetTexture("Interface\\WorldMap\\Elwynn\\ElwynnForest1")
           o:SetAllPoints(WorldMapDetailFrame)
           o:Show()"#,
    )
    .unwrap();
    s.resolve();

    let quads = s.extract();
    let arrow = quads
        .iter()
        .find(|q| {
            matches!(&q.content, QuadContent::ModelPane { model: Some(m), .. } if m == ARROW_MODEL)
        })
        .expect("the arrow pane is in the render list");
    let overlay = quads
        .iter()
        .find(|q| {
            matches!(
                &q.content,
                QuadContent::Texture { path: Some(p), .. } if p.contains("ElwynnForest1")
            )
        })
        .expect("the overlay texture is in the render list");
    assert!(
        arrow.z > overlay.z,
        "the arrow must paint after the zone overlay (arrow {:#x}, overlay {:#x})",
        arrow.z,
        overlay.z
    );
}

/// Every POI icon is a child of `WorldMapButton` (WorldMapFrame.lua:178), as is the player's blip,
/// so a click over one still answers the map's UV.
#[test]
fn a_click_over_a_blip_still_answers_the_maps_uv() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1600.0, 900.0);
    s.set_unit(
        "player",
        Some(benilla_ui::script::UnitState {
            exists: true,
            name: Some("Probefour".into()),
            level: 60,
            ..Default::default()
        }),
    );
    assert!(super::load_default_ui(&s).is_empty());
    s.resolve();
    s.run("ShowUIPanel(WorldMapFrame)").unwrap();
    s.resolve();
    s.set_world_map_feed(None, Some((0.4, 0.6)), 0.0, None, Vec::new(), Vec::new());
    update(&mut s);
    s.resolve();

    let (px, py, eff) = s
        .eval::<(f64, f64, f64)>(
            "local x, y = WorldMapPlayer:GetCenter() return x, y, WorldMapPlayer:GetEffectiveScale()",
        )
        .unwrap();
    let (x, y) = ((px * eff) as f32, (py * eff) as f32);
    assert_eq!(
        s.hit_test_name(x, y).as_deref(),
        Some("WorldMapPlayer"),
        "the blip is what the mouse is over"
    );
    let (u, v) = s
        .world_map_uv_at(x, y)
        .expect("the click still lands on the map");
    assert!(
        (u - 0.4).abs() < 0.01 && (v - 0.6).abs() < 0.01,
        "the UV under the blip is the blip's own: {u}, {v}"
    );
    // The map's own chrome is not the map.
    let (cx, cy, ceff) = s
        .eval::<(f64, f64, f64)>(
            "local x, y = WorldMapFrameCloseButton:GetCenter() return x, y, WorldMapFrameCloseButton:GetEffectiveScale()",
        )
        .unwrap();
    assert!(
        s.world_map_uv_at((cx * ceff) as f32, (cy * ceff) as f32)
            .is_none(),
        "the close button is not the map"
    );
}

/// The real catalog, [`crate::ui_world_map::build_catalog`] over `WorldMapArea`, `AreaTable` and
/// `WorldMapOverlay`, under a live cursor. Feralas is `WorldMapArea` 121; its `DIREMAUL` overlay's
/// hit rect is (top 235, left 525, bottom 325, right 655) px of the 1002x668 detail frame, naming
/// `AreaTable` 2577 "Dire Maul" behind explore bit 953, so UV (58.8 %, 42.6 %) lies inside it.
#[test]
fn the_real_feralas_catalog_names_dire_maul_under_the_cursor() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut s = harness();

    let mut chain = benilla_formats::open_chain(&data).expect("chain");
    let areas = benilla_formats::load_area_table_catalog(&mut chain).expect("AreaTable");
    let maps =
        benilla_assets::MapCatalogRes(benilla_formats::load_map_catalog(&mut chain).expect("Map"));
    let (views, _) = crate::ui_world_map::build_catalog(&mut chain, &areas, &maps, false)
        .expect("the real catalog");
    // Feralas' place in the pushed catalog, found the way Lua would.
    let (ci, zi) = views
        .iter()
        .enumerate()
        .find_map(|(ci, c)| {
            c.zones
                .iter()
                .position(|z| z.name == "Feralas")
                .map(|zi| (ci + 1, zi + 1))
        })
        .expect("Feralas is in the catalog");
    s.set_world_map_catalog(views);
    s.set_world_map_explored(vec![u32::MAX; 64]); // a fully-discovered character
                                                  // After the toggle, whose `SetMapToCurrentZone` lands on the world with no player zone.
    s.run("ToggleWorldMap()").unwrap();
    s.run(&format!("SetMapZoom({ci}, {zi})")).unwrap();
    s.resolve();

    // (58.8 %, 42.6 %) of the detail frame in screen units: what `WorldMapButton_OnUpdate` inverts.
    let (l, r, t, b, eff) = s
        .eval::<(f32, f32, f32, f32, f32)>(
            "return WorldMapButton:GetLeft(), WorldMapButton:GetRight(), WorldMapButton:GetTop(), \
             WorldMapButton:GetBottom(), WorldMapButton:GetEffectiveScale()",
        )
        .unwrap();
    let (x, y) = ((l + (r - l) * 0.588) * eff, (t - (t - b) * 0.426) * eff);
    s.mouse_move(x, y);
    update(&mut s);

    assert_eq!(
        s.eval::<String>("return WorldMapFrameAreaLabel:GetText()")
            .unwrap(),
        "Dire Maul",
        "the stock OnUpdate puts the hovered sub-area's name in the label"
    );
}

/// Westfall is the one Closed Zone (ADR 0015, `closed_chunks.json`): the real catalog marks it, and
/// the Eastern Kingdoms continent map draws its `WestfallHighlight` art as mist. Kalimdor has none.
#[test]
fn the_real_catalog_mists_westfall_on_the_eastern_kingdoms_only() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut s = harness();

    let mut chain = benilla_formats::open_chain(&data).expect("chain");
    let areas = benilla_formats::load_area_table_catalog(&mut chain).expect("AreaTable");
    let maps =
        benilla_assets::MapCatalogRes(benilla_formats::load_map_catalog(&mut chain).expect("Map"));
    let (views, _) = crate::ui_world_map::build_catalog(&mut chain, &areas, &maps, false)
        .expect("the real catalog");
    let continent = |name: &str| {
        views
            .iter()
            .position(|c| c.name == name)
            .map(|i| i + 1)
            .unwrap_or_else(|| panic!("{name} is in the catalog"))
    };
    let (ek, kalimdor) = (continent("Eastern Kingdoms"), continent("Kalimdor"));
    s.set_world_map_catalog(views);
    s.run("ToggleWorldMap()").unwrap();
    s.resolve();

    s.run(&format!("SetMapZoom({ek})")).unwrap();
    s.run("WorldMapFrame_Update()").unwrap();
    assert!(s.eval::<bool>("return WorldMapMist1:IsShown()").unwrap());
    for i in [1, 2] {
        assert_eq!(
            s.eval::<String>(&format!("return WorldMapMist{i}:GetBlendMode()"))
                .unwrap(),
            "ADD",
            "the Highlight art is a silhouette on black, drawn additively like WorldMapHighlight"
        );
    }
    assert_eq!(
        s.eval::<String>("return WorldMapMist1:GetTexture()")
            .unwrap(),
        "Interface\\WorldMap\\Westfall\\WestfallHighlight"
    );

    s.run(&format!("SetMapZoom({kalimdor})")).unwrap();
    s.run("WorldMapFrame_Update()").unwrap();
    assert!(
        !s.eval::<bool>("return WorldMapMist1 ~= nil and WorldMapMist1:IsShown()")
            .unwrap(),
        "Kalimdor has no Closed Zone"
    );
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// An addon that windows the map (Cartographer's `LookNFeel`) stamps userPlaced through a drag
/// (`0x7652b0`, at `0x7652e5`). The layout cache gates both ends on `movable|resizable` too, as the
/// reference does (writer `0x490e97`; apply `0x490600`, `0x490689`), and stock `WorldMapFrame` has
/// neither flag: without the addon the row is inert and falls out of the file at logout.
#[test]
fn a_stale_layout_row_cannot_seat_a_stock_frame() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1600.0, 900.0);
    s.set_unit(
        "player",
        Some(benilla_ui::script::UnitState {
            exists: true,
            name: Some("Probefour".into()),
            level: 60,
            ..Default::default()
        }),
    );
    let failures = super::load_default_ui(&s);
    assert!(failures.is_empty(), "manifest load errors: {failures:#?}");
    s.resolve();

    // The row as `benilla-config/layout/<realm>-<character>.txt` carries it: Cartographer's saved
    // `LookNFeel` offsets and the size it forces.
    let row = || benilla_ui::script::FrameLayout {
        name: "WorldMapFrame".into(),
        width: 1024.0,
        height: 768.0,
        points: vec![benilla_ui::script::LayoutPoint {
            point: "CENTER".into(),
            relative_to: Some("UIParent".into()),
            relative_point: "CENTER".into(),
            x: -0.6369222,
            y: 53.861614,
        }],
    };

    s.restore_user_placed_layouts([row()]);
    s.resolve();
    s.run("ShowUIPanel(WorldMapFrame)").unwrap();
    s.resolve();

    let (w, h, sw, sh) = s
        .eval::<(f64, f64, f64, f64)>(
            "return WorldMapFrame:GetWidth(), WorldMapFrame:GetHeight(), \
             GetScreenWidth(), GetScreenHeight()",
        )
        .unwrap();
    assert_eq!(
        (w, h),
        (sw, sh),
        "the stale row seated nothing — the map is still the screen, as its setAllPoints authored it"
    );
    assert!(
        !s.eval::<bool>("return WorldMapFrame:IsUserPlaced()")
            .unwrap(),
        "and the apply's userPlaced stamp lives inside the position arm it never took"
    );
    assert!(
        s.user_placed_layouts().is_empty(),
        "so the row falls out of the file at this logout instead of being written again"
    );

    // The control: with the addon's `SetMovable` and `SetResizable`, the same row seats.
    s.run("WorldMapFrame:SetMovable(true) WorldMapFrame:SetResizable(true)")
        .unwrap();
    s.restore_user_placed_layouts([row()]);
    s.resolve();
    let (w, h) = s
        .eval::<(f64, f64)>("return WorldMapFrame:GetWidth(), WorldMapFrame:GetHeight()")
        .unwrap();
    assert_eq!(
        (w, h),
        (1024.0, 768.0),
        "a movable+resizable frame takes it"
    );
    assert!(
        s.eval::<bool>("return WorldMapFrame:IsUserPlaced()")
            .unwrap(),
        "and the position arm stamps the bit it was written under"
    );
    let saved = s.user_placed_layouts();
    assert_eq!(saved.len(), 1, "which is what puts the row back: {saved:?}");
    assert_eq!(saved[0], row(), "unchanged through the round trip");
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// Stock `WorldMapFrame_OnLoad` and `CinematicFrame_OnLoad` size `BlackoutWorld` and the letterbox
/// bars once, from the screen size. Deviation: `manifest::on_screen_resized` re-sizes them, because
/// a resize here keeps the loaded UI and the quads would stop short of the new screen.
#[test]
fn the_fullscreen_quads_follow_a_resize() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1600.0, 900.0);
    // A player exists by the time the manifest loads.
    s.set_unit(
        "player",
        Some(benilla_ui::script::UnitState {
            exists: true,
            name: Some("Probefour".into()),
            level: 60,
            ..Default::default()
        }),
    );
    let failures = super::load_default_ui(&s);
    assert!(failures.is_empty(), "manifest load errors: {failures:#?}");
    s.resolve();

    let width = |s: &UiScript, f: &str| s.eval::<f64>(&format!("return {f}:GetWidth()")).unwrap();
    assert_eq!(width(&s, "BlackoutWorld"), 1600.0, "the load edge sized it");

    // Grown, still wider than 4:3, so all three take their recompute branch.
    assert!(s.set_screen_size(1920.0, 1080.0), "the size changed");
    super::manifest::on_screen_resized(&s);
    s.resolve();

    let screen = s.eval::<f64>("return GetScreenWidth()").unwrap();
    assert_eq!(screen, 1920.0);
    for quad in ["BlackoutWorld", "UpperBlackBar", "LowerBlackBar"] {
        assert_eq!(
            width(&s, quad),
            screen,
            "{quad} still spans the OLD screen after a resize — the world shows through beside it"
        );
    }
    assert_eq!(
        s.eval::<f64>("return BlackoutWorld:GetHeight()").unwrap(),
        1080.0,
        "…and the blackout is the new height too"
    );
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// A landmark past the `POIIcons` grid draws its own gossip icon, and a reused slot gets the grid
/// back (WorldMapMarks.xml).
#[test]
fn a_twilight_mark_swaps_the_icon_and_a_reused_slot_gets_the_grid_back() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = harness();
    s.set_world_map_landmarks(vec![landmark("Marshal Dughan", 64, (0.5, 0.5))]);
    s.run("WorldMapFrame_Update()").unwrap();
    let file: String = s
        .eval("return WorldMapFramePOI1Texture:GetTexture()")
        .unwrap();
    assert!(file.contains("AvailableQuestIcon"), "got {file}");

    s.set_world_map_landmarks(vec![landmark("Lion's Pride Inn", 6, (0.5, 0.5))]);
    s.run("WorldMapFrame_Update()").unwrap();
    let file: String = s
        .eval("return WorldMapFramePOI1Texture:GetTexture()")
        .unwrap();
    assert!(file.contains("POIIcons"), "got {file}");
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// A mark's POI shows `WorldMapTooltip` with the name and one line per description line, and the
/// stock label stays empty.
#[test]
fn hovering_a_twilight_mark_shows_the_world_map_tooltip() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = harness();
    let mut mark = landmark("Marshal Dughan", 65, (0.5, 0.5));
    mark.description = "line one\nline two".into();
    s.set_world_map_landmarks(vec![mark]);
    s.run("WorldMapFrame_Update()").unwrap();
    // The stock OnUpdate clears the label's XML placeholder to the (empty) area name first.
    update(&mut s);
    s.run("this = WorldMapFramePOI1 this:GetScript(\"OnEnter\")() this = nil")
        .unwrap();
    assert!(s.eval::<bool>("return WorldMapTooltip:IsShown()").unwrap());
    assert_eq!(
        s.eval::<i64>("return WorldMapTooltip:NumLines()").unwrap(),
        3
    );
    assert_eq!(
        s.eval::<String>("return WorldMapTooltipTextLeft1:GetText()")
            .unwrap(),
        "Marshal Dughan"
    );
    assert_eq!(
        s.eval::<Option<String>>("return WorldMapFrameAreaLabel:GetText()")
            .unwrap(),
        None,
        "the stock label stays empty"
    );
    s.run("this = WorldMapFramePOI1 this:GetScript(\"OnLeave\")() this = nil")
        .unwrap();
    assert!(!s.eval::<bool>("return WorldMapTooltip:IsShown()").unwrap());
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// The canvas rect of #410 over Duskwood, Redridge Mountains, Elwynn Forest and Stormwind City.
const CANVAS: (f32, f32, f32, f32) = (1579.69, -3785.94, -7939.58, -11516.67);

/// The player at Eastvale (-9450, -1300) in canvas UV.
const EASTVALE: (f32, f32) = (0.5367, 0.4222);

/// The `0x4a6ec0` cell law, as `benilla_ui`'s `area_grid_cell` runs it: un-lerp the UV through the
/// rect to world coordinates and index the 128x128 grid as `col - row * 128`.
fn grid_cell(loc: (f32, f32, f32, f32), u: f32, v: f32) -> usize {
    const K: f32 = 2.929_687_6e-5;
    let (left, right, top, bottom) = loc;
    let f1 = 0.5 - ((1.0 - u) * (left - right) + right) * K;
    let f2 = 0.5 - ((1.0 - v) * (top - bottom) + bottom) * K;
    let col = (f1 * 128.0) as i32;
    let row = (f2 * -128.0) as i32;
    usize::try_from(col - row * 128).unwrap()
}

/// The engine ticket's `push_canvas_catalog`: a bare Kalimdor, then Eastern Kingdoms with the real
/// loc rects of the four canvas zones and two neighbours, and the canvas over Duskwood, Redridge,
/// Elwynn and Stormwind. The grid names Elwynn at canvas UV (0.28, 0.43) and Westfall at
/// (0.05, 0.85).
fn canvas_catalog() -> Vec<benilla_ui::script::WorldMapContinentView> {
    use benilla_ui::script::{
        WorldMapCanvasView, WorldMapContinentView, WorldMapOverlayView, WorldMapZoneView,
    };
    let zone = |name: &str, area_id: u32, file: &str, loc_rect, closed| WorldMapZoneView {
        name: name.into(),
        area_id,
        map_file: file.into(),
        loc_rect,
        overlays: Vec::new(),
        closed,
    };
    let mut elwynn = zone(
        "Elwynn Forest",
        12,
        "Elwynn",
        (1535.42, -1935.42, -7939.58, -10254.17),
        false,
    );
    elwynn.overlays.push(WorldMapOverlayView {
        texture: r"Interface\WorldMap\Elwynn\GOLDSHIRE".into(),
        width: 256,
        height: 256,
        offset_x: 300,
        offset_y: 200,
        explore_bits: vec![5],
        hit_rect: (0, 0, 668, 1002),
        area_name: Some("Goldshire".into()),
    });
    let mut grid = vec![0u16; 128 * 128];
    grid[grid_cell(CANVAS, 0.28, 0.43)] = 1;
    grid[grid_cell(CANVAS, 0.05, 0.85)] = 5;
    vec![
        WorldMapContinentView {
            name: "Kalimdor".into(),
            map_file: "Kalimdor".into(),
            ..Default::default()
        },
        WorldMapContinentView {
            name: "Eastern Kingdoms".into(),
            map_file: "Azeroth".into(),
            world_rect: (0.62375, 0.02695, 0.92315, 0.87126),
            loc_rect: (16000.0, -19199.9, 7466.6, -16000.0),
            zone_grid: grid,
            zones: vec![
                elwynn,
                zone(
                    "Stormwind City",
                    1519,
                    "Stormwind",
                    (1380.97, 36.70, -8278.85, -9175.21),
                    false,
                ),
                zone(
                    "Duskwood",
                    10,
                    "Duskwood",
                    (833.33, -1866.67, -9716.67, -11516.67),
                    false,
                ),
                zone(
                    "Redridge Mountains",
                    44,
                    "Redridge",
                    (-1570.83, -3741.67, -8575.0, -10022.92),
                    false,
                ),
                zone(
                    "Westfall",
                    40,
                    "Westfall",
                    (3016.67, -483.33, -9400.0, -11733.33),
                    true,
                ),
                zone(
                    "Dun Morogh",
                    1,
                    "DunMorogh",
                    (1802.08, -3122.92, -3877.08, -7160.42),
                    false,
                ),
            ],
            canvas: Some(WorldMapCanvasView {
                zones: vec![3, 4, 1, 2],
                loc_rect: CANVAS,
            }),
        },
    ]
}

/// The canvas open in the 1024x768 harness, with the player at Eastvale.
fn canvas_harness() -> UiScript {
    let mut s = harness();
    arrow_facts(&mut s);
    s.set_world_map_catalog(canvas_catalog());
    s.set_world_map_feed(
        Some((2, 1)),
        Some(EASTVALE),
        0.0,
        None,
        Vec::new(),
        Vec::new(),
    );
    s.run("SetMapZoom(2)").unwrap();
    s.run("ToggleWorldMap()").unwrap();
    s.resolve();
    settle(&mut s);
    s
}

/// One map update and a layout pass, so every icon sits where this zoom puts it.
fn settle(s: &mut UiScript) {
    update(s);
    s.resolve();
}

/// The screen point at map UV `(u, v)` of `WorldMapDetailFrame`, at the current zoom and scroll.
fn map_point(s: &UiScript, u: f64, v: f64) -> (f32, f32) {
    let (l, r, t, b, eff) = s
        .eval::<(f64, f64, f64, f64, f64)>(
            "local d = WorldMapDetailFrame \
             return d:GetLeft(), d:GetRight(), d:GetTop(), d:GetBottom(), d:GetEffectiveScale()",
        )
        .unwrap();
    (
        ((l + (r - l) * u) * eff) as f32,
        ((t - (t - b) * v) * eff) as f32,
    )
}

/// The UV `WorldMapButton_OnUpdate` computes for the cursor, measured against the detail frame.
fn cursor_uv(s: &UiScript) -> (f64, f64) {
    s.eval::<(f64, f64)>(
        "local d = WorldMapDetailFrame \
         local scale = d:GetEffectiveScale() \
         local x, y = GetCursorPosition() \
         x = x / scale y = y / scale \
         local cx, cy = d:GetCenter() \
         local w, h = d:GetWidth(), d:GetHeight() \
         return (x - (cx - w / 2)) / w, ((cy + h / 2) - y) / h",
    )
    .unwrap()
}

/// A frame's centre in screen units.
fn centre(s: &UiScript, frame: &str) -> (f32, f32) {
    let (x, y, eff) = s
        .eval::<(f64, f64, f64)>(&format!(
            "local f = {frame} local x, y = f:GetCenter() return x, y, f:GetEffectiveScale()"
        ))
        .unwrap();
    ((x * eff) as f32, (y * eff) as f32)
}

/// The player arrow, the anonymous `Model` the canvas moves into `WorldMapButton`, as a global.
fn find_arrow(s: &UiScript) {
    s.run(
        "for _, c in ipairs({ WorldMapButton:GetChildren() }) do \
             if c:GetObjectType() == 'Model' and not c:GetName() then BenillaTestArrow = c end \
         end",
    )
    .unwrap();
}

fn zoom_of(s: &UiScript) -> f64 {
    s.eval::<f64>("return BenillaMapCanvas_GetZoom()").unwrap()
}

fn near(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() < 1.0 && (a.1 - b.1).abs() < 1.0
}

fn wheel(s: &mut UiScript, at: (f32, f32), notches: u32, delta: f32) {
    s.mouse_move(at.0, at.1);
    for _ in 0..notches {
        s.mouse_wheel(at.0, at.1, delta);
    }
    settle(s);
}

/// The map opens on the player's zone at its stock size, centred on the player, with no continent
/// picker and a zoom-out that does nothing.
#[test]
fn the_canvas_opens_on_the_players_zone_centred_on_the_player() {
    let _data = benilla_formats::wow_data_or_skip!();
    let s = canvas_harness();
    assert!((zoom_of(&s) - 1.5459).abs() < 1e-3, "{}", zoom_of(&s));
    let view = centre(&s, "WorldMapZoomFrame");
    let player = centre(&s, "WorldMapPlayer");
    assert!(
        near(player, view),
        "player {player:?}, view centre {view:?}"
    );
    find_arrow(&s);
    let arrow = centre(&s, "BenillaTestArrow");
    assert!(near(arrow, view), "arrow {arrow:?}, view centre {view:?}");
    assert_eq!(
        s.eval::<String>("return WorldMapZoneDropDownText:GetText()")
            .unwrap(),
        "Elwynn Forest"
    );
    assert!(
        !s.eval::<bool>(
            "return WorldMapContinentDropDown:IsShown() or WorldMapZoomOutButton:IsShown()"
        )
        .unwrap(),
        "the canvas has no continent picker and no zoom-out button"
    );
    s.run("WorldMapZoomOutButton_OnClick()").unwrap();
    assert!((zoom_of(&s) - 1.5459).abs() < 1e-3, "{}", zoom_of(&s));
    assert_eq!(
        s.eval::<(i64, i64)>("return GetCurrentMapContinent(), GetCurrentMapZone()")
            .unwrap(),
        (2, 0),
        "the right-click's zoom-out does nothing"
    );
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// Zoom 1 fits the canvas and the deepest zoom is four times Elwynn's stock size.
#[test]
fn the_canvas_zooms_from_the_whole_canvas_to_four_times_elwynn() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = canvas_harness();
    let at = centre(&s, "WorldMapZoomFrame");
    wheel(&mut s, at, 20, -1.0);
    assert_eq!(zoom_of(&s), 1.0);
    assert_eq!(s.eval::<i64>("return GetCurrentMapContinent()").unwrap(), 2);
    wheel(&mut s, at, 20, 1.0);
    assert!((zoom_of(&s) - 6.184).abs() < 1e-3, "{}", zoom_of(&s));
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// The drift regression: through five notches in and five out, a landmark and the player arrow
/// stay on their map points at their starting size, and so does a landmark pushed mid-zoom.
#[test]
fn icons_stay_on_their_map_points_through_a_zoom_round_trip() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = canvas_harness();
    const MARK: (f64, f64) = (0.7063, 0.3675);
    let mark = (MARK.0 as f32, MARK.1 as f32);
    s.set_world_map_landmarks(vec![landmark("The Inn", 6, mark)]);
    s.run("WorldMapFrame_Update()").unwrap();
    settle(&mut s);
    find_arrow(&s);
    let size = |s: &UiScript, poi: &str| {
        s.eval::<f64>(&format!(
            "return {poi}:GetWidth() * {poi}:GetEffectiveScale()"
        ))
        .unwrap()
    };
    let start = size(&s, "WorldMapFramePOI1");
    let mut poi = "WorldMapFramePOI1";
    for step in 0..10 {
        let delta = if step < 5 { 1.0 } else { -1.0 };
        let at = map_point(&s, MARK.0, MARK.1);
        wheel(&mut s, at, 1, delta);
        if step == 2 {
            s.set_world_map_landmarks(vec![
                landmark("The Bank", 6, (0.2, 0.2)),
                landmark("The Inn", 6, mark),
            ]);
            s.run("WorldMapFrame_Update()").unwrap();
            settle(&mut s);
            poi = "WorldMapFramePOI2";
        }
        let got = centre(&s, poi);
        let want = map_point(&s, MARK.0, MARK.1);
        assert!(
            near(got, want),
            "step {step}: {poi} at {got:?}, map point {want:?}"
        );
        let got_size = size(&s, poi);
        assert!(
            (got_size - start).abs() < 1e-3,
            "step {step}: {start} became {got_size}"
        );
        let player = map_point(&s, f64::from(EASTVALE.0), f64::from(EASTVALE.1));
        for icon in ["WorldMapPlayer", "BenillaTestArrow"] {
            let got = centre(&s, icon);
            assert!(
                near(got, player),
                "step {step}: {icon} at {got:?}, map point {player:?}"
            );
        }
    }
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// At every zoom the cursor over canvas UV (0.28, 0.43) names Elwynn Forest in the area label.
#[test]
fn the_hover_names_the_zone_at_every_zoom() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = canvas_harness();
    for step in 0..16 {
        let delta = if step < 8 { 1.0 } else { -1.0 };
        let at = map_point(&s, 0.28, 0.43);
        s.mouse_move(at.0, at.1);
        settle(&mut s);
        let (u, v) = cursor_uv(&s);
        assert!(
            (u - 0.28).abs() < 0.002 && (v - 0.43).abs() < 0.002,
            "step {step}: ({u}, {v})"
        );
        assert_eq!(
            s.eval::<Option<String>>("return WorldMapFrameAreaLabel:GetText()")
                .unwrap()
                .as_deref(),
            Some("Elwynn Forest"),
            "step {step}, zoom {}",
            zoom_of(&s)
        );
        wheel(&mut s, at, 1, delta);
    }
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// The zone dropdown lists the canvas zones alphabetically, and a choice zooms to that zone's
/// stock size and centres it.
#[test]
fn choosing_a_zone_in_the_dropdown_zooms_to_it_and_centres_it() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = canvas_harness();
    s.run("ToggleDropDownMenu(1, nil, WorldMapZoneDropDown)")
        .unwrap();
    let names = s
        .eval::<(String, String, String, String)>(
            "return DropDownList1Button1:GetText(), DropDownList1Button2:GetText(), \
             DropDownList1Button3:GetText(), DropDownList1Button4:GetText()",
        )
        .unwrap();
    assert_eq!(
        names,
        (
            "Duskwood".into(),
            "Elwynn Forest".into(),
            "Redridge Mountains".into(),
            "Stormwind City".into()
        )
    );
    s.run("this = DropDownList1Button3 this:GetScript('OnClick')() this = nil")
        .unwrap();
    settle(&mut s);
    assert!((zoom_of(&s) - 2.4717).abs() < 1e-3, "{}", zoom_of(&s));
    let (u, v) = s
        .eval::<(f64, f64)>(
            "local e = GetMapCanvas()[2] return e.left + e.width / 2, e.top + e.height / 2",
        )
        .unwrap();
    let got = map_point(&s, u, v);
    let want = centre(&s, "WorldMapZoomFrame");
    assert!(
        near(got, want),
        "Redridge's centre {got:?}, view centre {want:?}"
    );
    assert_eq!(
        s.eval::<String>("return WorldMapZoneDropDownText:GetText()")
            .unwrap(),
        "Redridge Mountains"
    );
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// At the deepest zoom a drag far past the map's edge leaves the detail frame over the whole view.
#[test]
fn a_drag_past_the_edge_leaves_no_gap() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = canvas_harness();
    let (x, y) = centre(&s, "WorldMapZoomFrame");
    wheel(&mut s, (x, y), 20, 1.0);
    for (dx, dy) in [(900.0, 700.0), (-900.0, -700.0)] {
        s.mouse_button(x, y, "LeftButton", true);
        s.mouse_move(x + dx, y + dy);
        s.run("BenillaMapCanvas_Pan()").unwrap();
        s.resolve();
        let covers: bool = s
            .eval(
                "local d, z = WorldMapDetailFrame, WorldMapZoomFrame \
                 local de, ze, e = d:GetEffectiveScale(), z:GetEffectiveScale(), 0.01 \
                 return d:GetLeft() * de <= z:GetLeft() * ze + e \
                    and d:GetRight() * de >= z:GetRight() * ze - e \
                    and d:GetTop() * de >= z:GetTop() * ze - e \
                    and d:GetBottom() * de <= z:GetBottom() * ze + e",
            )
            .unwrap();
        assert!(covers, "a gap after dragging by ({dx}, {dy})");
        s.mouse_button(x + dx, y + dy, "LeftButton", false);
        s.mouse_move(x, y);
    }
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// Each canvas zone has a plate on its canvas rect, at zoom 1 and at the deepest zoom.
#[test]
fn each_canvas_zone_has_a_plate_on_its_rect() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = canvas_harness();
    let at = centre(&s, "WorldMapZoomFrame");
    for (notches, delta) in [(20, -1.0), (20, 1.0)] {
        wheel(&mut s, at, notches, delta);
        for i in 1..=4 {
            let (left, top, width) = s
                .eval::<(f64, f64, f64)>(&format!(
                    "local e = GetMapCanvas()[{i}] return e.left, e.top, e.width"
                ))
                .unwrap();
            let want = map_point(&s, left, top);
            let (pl, pt, pw, eff, dw, deff) = s
                .eval::<(f64, f64, f64, f64, f64, f64)>(&format!(
                    "local p, d = BenillaCanvasPlate{i}, WorldMapDetailFrame \
                     return p:GetLeft(), p:GetTop(), p:GetWidth(), p:GetEffectiveScale(), \
                        d:GetWidth(), d:GetEffectiveScale()"
                ))
                .unwrap();
            let got = ((pl * eff) as f32, (pt * eff) as f32);
            assert!(
                near(got, want),
                "plate {i} at zoom {}: {got:?} vs {want:?}",
                zoom_of(&s)
            );
            assert!(
                (pw * eff - width * dw * deff).abs() < 1.0,
                "plate {i} width {} vs {}",
                pw * eff,
                width * dw * deff
            );
        }
    }
    assert!(
        !s.eval::<bool>("return WorldMapDetailTile1:IsShown()")
            .unwrap(),
        "the stock tiles hide under the plates"
    );
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// At the deepest zoom `WorldMapButton` is larger than the view; outside the view the hit test
/// reaches neither it nor a POI seated there.
#[test]
fn outside_the_view_the_hit_test_misses_the_map() {
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = canvas_harness();
    let at = centre(&s, "WorldMapZoomFrame");
    wheel(&mut s, at, 20, 1.0);
    let (top, eff) = s
        .eval::<(f64, f64)>(
            "return WorldMapZoomFrame:GetTop(), WorldMapZoomFrame:GetEffectiveScale()",
        )
        .unwrap();
    let probe = (at.0, (top * eff) as f32 + 10.0);
    let (u, v) = s
        .eval::<(f64, f64)>(&format!(
            "local d = WorldMapDetailFrame local e = d:GetEffectiveScale() \
             return ({x} / e - d:GetLeft()) / d:GetWidth(), (d:GetTop() - {y} / e) / d:GetHeight()",
            x = probe.0,
            y = probe.1
        ))
        .unwrap();
    assert!(v > 0.0, "the button reaches past the view's top ({u}, {v})");
    s.set_world_map_landmarks(vec![landmark("The Inn", 6, (u as f32, v as f32))]);
    s.run("WorldMapFrame_Update()").unwrap();
    settle(&mut s);
    assert!(near(centre(&s, "WorldMapFramePOI1"), probe));
    let hit = s.hit_test_name(probe.0, probe.1);
    assert!(
        !matches!(hit.as_deref(), Some("WorldMapButton" | "WorldMapFramePOI1")),
        "{hit:?}"
    );
    assert!(s.world_map_uv_at(probe.0, probe.1).is_none());
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// Off the canvas the stock map stays as it is: a wheel notch changes nothing.
#[test]
fn off_the_canvas_the_wheel_does_nothing() {
    use benilla_ui::script::{WorldMapContinentView, WorldMapZoneView};
    let _data = benilla_formats::wow_data_or_skip!();
    let mut s = harness();
    s.set_world_map_catalog(vec![WorldMapContinentView {
        name: "Eastern Kingdoms".into(),
        map_file: "Azeroth".into(),
        world_rect: (0.62375, 0.02695, 0.92315, 0.87126),
        loc_rect: (16000.0, -19199.9, 7466.6, -16000.0),
        zone_grid: vec![1u16; 128 * 128],
        canvas: None,
        zones: vec![WorldMapZoneView {
            name: "Elwynn Forest".into(),
            area_id: 12,
            map_file: "Elwynn".into(),
            loc_rect: (-8000.0, -10000.0, -400.0, -1900.0),
            overlays: Vec::new(),
            closed: false,
        }],
    }]);
    // After the toggle, whose `SetMapToCurrentZone` lands on the world with no player zone.
    s.run("ToggleWorldMap()").unwrap();
    s.run("SetMapZoom(1, 1)").unwrap();
    s.run("WorldMapFrame_Update()").unwrap();
    s.resolve();
    let at = centre(&s, "WorldMapZoomFrame");
    wheel(&mut s, at, 1, 1.0);
    wheel(&mut s, at, 1, -1.0);
    assert_eq!(zoom_of(&s), 1.0);
    let sizes = s
        .eval::<(f64, f64, f64, f64)>(
            "return WorldMapDetailFrame:GetWidth(), WorldMapButton:GetWidth(), \
             WorldMapZoomFrame:GetHorizontalScroll(), WorldMapZoomFrame:GetVerticalScroll()",
        )
        .unwrap();
    assert_eq!(sizes, (1002.0, 1002.0, 0.0, 0.0));
    assert_eq!(
        s.eval::<(i64, i64)>("return GetCurrentMapContinent(), GetCurrentMapZone()")
            .unwrap(),
        (1, 1)
    );
    assert!(s
        .eval::<bool>("return WorldMapDetailTile1:IsShown() and WorldMapZoomOutButton:IsShown()")
        .unwrap());
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}
