//! The Forager's Grid window (`assets/ui/HerbPatch.xml`) over pushed grids: the header, the tile
//! art, the chips, the basket, the end card, the pull log, the clicks that queue `Herb*` calls,
//! skill mode and the close. It needs no game data, so a missing font only warns.

use benilla_ui::script::{
    HerbBasketView, HerbCall, HerbGridView, HerbKnownView, HerbLogView, HerbTileView,
    HerbalistView, ItemTemplateView, ScriptValue, UiScript,
};

use super::test_ui::load_ui as load_xml;

const ART: &str = "Interface\\Benilla\\herbalism\\";

/// The window for a character who has seen the guided tour, so it stays out of the way.
fn window() -> UiScript {
    toured(true)
}

/// The window for a character whose `tourSeen` is `seen`.
fn toured(seen: bool) -> UiScript {
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1024.0, 768.0);
    load_xml(&s, "HerbPatch.xml");
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    s.set_herbalist(HerbalistView {
        knows: true,
        tour_seen: seen,
        ..Default::default()
    });
    s
}

fn shown(s: &UiScript, frame: &str) -> bool {
    s.eval::<bool>(&format!("return {frame}:IsVisible() and true or false"))
        .unwrap()
}

fn text(s: &UiScript, frame: &str) -> String {
    s.eval::<String>(&format!("return {frame}:GetText() or ''"))
        .unwrap()
}

/// `text` with its `|cffrrggbb` and `|r` colour codes taken out.
fn plain(s: &UiScript, frame: &str) -> String {
    s.eval::<String>(&format!(
        "local t = string.gsub({frame}:GetText() or '', '|c%x%x%x%x%x%x%x%x', '') \
         t = string.gsub(t, '|r', '') return t"
    ))
    .unwrap()
}

fn texture(s: &UiScript, frame: &str) -> String {
    s.eval::<String>(&format!("return {frame}:GetTexture() or ''"))
        .unwrap()
}

fn click(s: &UiScript, tile: u32) {
    s.run(&format!(
        "this = BenillaHerbTile{tile} BenillaHerbTile_OnClick()"
    ))
    .unwrap();
}

/// Twenty-five hidden tiles, with `edit` applied to the grid.
fn grid(edit: impl FnOnce(&mut HerbGridView)) -> HerbGridView {
    let mut view = HerbGridView {
        size: 5,
        pulls_left: 7,
        tiles: vec![
            HerbTileView {
                stake: -1,
                ..Default::default()
            };
            25
        ],
        ..Default::default()
    };
    edit(&mut view);
    view
}

/// Push `view` the way the host does: set the model, then fire the update event.
fn push(s: &mut UiScript, view: HerbGridView) {
    s.set_herb_grid(Some(view));
    s.fire_event("HERB_GRID_UPDATE", Vec::new());
    s.resolve();
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// A plant on tile 1, a weed with stake 2 on tile 2, a known thorn on tile 3.
fn mixed() -> HerbGridView {
    grid(|g| {
        g.tiles[0] = HerbTileView {
            state: 1,
            stake: -1,
            item: 2447,
            count: 2,
            bloom: true,
            part: "flower".into(),
            ..Default::default()
        };
        g.tiles[1] = HerbTileView {
            state: 2,
            stake: 2,
            ..Default::default()
        };
        g.tiles[2] = HerbTileView {
            known: 3,
            stake: -1,
            ..Default::default()
        };
        g.known = vec![
            HerbKnownView {
                item: 2447,
                present: true,
                rows: vec![2, 4],
            },
            HerbKnownView {
                item: 765,
                present: false,
                rows: Vec::new(),
            },
        ];
    })
}

fn named(s: &mut UiScript, item: u32, name: &str) {
    s.set_item_template(
        item,
        ItemTemplateView {
            name: name.into(),
            ..Default::default()
        },
    );
}

#[test]
fn a_pushed_grid_draws_plant_weed_and_thorn() {
    let mut s = window();
    named(&mut s, 2447, "Peacebloom");
    named(&mut s, 765, "Silverleaf");
    push(&mut s, mixed());
    assert!(shown(&s, "BenillaHerbPatch"));
    assert_eq!(texture(&s, "BenillaHerbTile1Bush"), format!("{ART}mound"));
    assert_eq!(text(&s, "BenillaHerbTile1Name"), "Peacebloom");
    assert_eq!(texture(&s, "BenillaHerbTile2Bush"), format!("{ART}mound"));
    assert_eq!(
        texture(&s, "BenillaHerbTile2Glyph"),
        format!("{ART}part_weed")
    );
    assert!(shown(&s, "BenillaHerbTile2Stake"));
    assert_eq!(text(&s, "BenillaHerbTile2StakeText"), "2");
    assert_eq!(
        texture(&s, "BenillaHerbTile3Bush"),
        format!("{ART}bush_thorn_3")
    );
    assert!(!shown(&s, "BenillaHerbTile4Stake"));
    assert_eq!(
        plain(&s, "BenillaHerbKnownChip1Text"),
        "Peacebloom row 2, 4"
    );
    assert_eq!(
        plain(&s, "BenillaHerbKnownChip2Text"),
        "Silverleaf not in this patch"
    );
    assert_eq!(texture(&s, "BenillaHerbPip7"), format!("{ART}pip_on"));
    assert!(s
        .eval::<bool>("return not (BenillaHerbPip8 and BenillaHerbPip8:IsVisible())")
        .unwrap());
}

#[test]
fn the_header_names_the_time_size_and_grown_for_level() {
    let mut s = window();
    push(
        &mut s,
        grid(|g| {
            g.time_block = "Dusk".into();
            g.grown_for = 12;
        }),
    );
    assert_eq!(
        text(&s, "BenillaHerbMeta"),
        "Dusk · 5×5 · grown for herbalism 12"
    );
    assert!(!shown(&s, "BenillaHerbBloomLine"));
}

#[test]
fn a_hidden_bush_takes_its_shape_and_facing_from_its_tile_number() {
    let mut s = window();
    push(&mut s, grid(|g| g.tiles[6].known = 1));
    let first_u = |s: &UiScript, frame: &str| {
        s.eval::<f32>(&format!("local u = {frame}:GetTexCoord() return u"))
            .unwrap()
    };
    assert_eq!(
        texture(&s, "BenillaHerbTile1Bush"),
        format!("{ART}bush_hidden_1")
    );
    assert_eq!(first_u(&s, "BenillaHerbTile1Bush"), 0.0);
    assert_eq!(
        texture(&s, "BenillaHerbTile5Bush"),
        format!("{ART}bush_hidden_2")
    );
    assert_eq!(
        first_u(&s, "BenillaHerbTile5Bush"),
        1.0,
        "tile 4 is mirrored"
    );
    assert!(shown(&s, "BenillaHerbTile7Glow"));
    assert!(!shown(&s, "BenillaHerbTile1Glow"));
}

#[test]
fn a_known_tile_shows_its_kind_of_bush() {
    let mut s = window();
    push(
        &mut s,
        grid(|g| {
            g.tiles[0].known = 1;
            g.tiles[1].known = 2;
        }),
    );
    assert_eq!(
        texture(&s, "BenillaHerbTile1Bush"),
        format!("{ART}bush_hidden_1")
    );
    assert!(shown(&s, "BenillaHerbTile1Glow"));
    assert_eq!(
        texture(&s, "BenillaHerbTile2Bush"),
        format!("{ART}bush_wilted_2")
    );
}

#[test]
fn a_pulled_plant_shows_its_part_glyph() {
    let mut s = window();
    named(&mut s, 2447, "Peacebloom");
    push(&mut s, mixed());
    assert_eq!(
        texture(&s, "BenillaHerbTile1Glyph"),
        format!("{ART}glyph_flower")
    );
    assert!(shown(&s, "BenillaHerbTile1Glyph"));
}

#[test]
fn the_basket_lists_its_rows_or_says_it_is_empty() {
    let mut s = window();
    named(&mut s, 2447, "Peacebloom");
    push(&mut s, grid(|_| {}));
    assert_eq!(
        text(&s, "BenillaHerbBasketEmpty"),
        "Nothing yet. Pull a tile to start."
    );
    assert!(!shown(&s, "BenillaHerbBasket1"));
    push(
        &mut s,
        grid(|g| {
            g.basket = vec![HerbBasketView {
                item: 2447,
                count: 3,
                part: "flower".into(),
            }]
        }),
    );
    assert!(shown(&s, "BenillaHerbBasket1"));
    assert_eq!(text(&s, "BenillaHerbBasket1Name"), "Peacebloom");
    assert_eq!(text(&s, "BenillaHerbBasket1Count"), "3");
    assert_eq!(text(&s, "BenillaHerbBasketEmpty"), "");
    assert!(!shown(&s, "BenillaHerbBasket2"));
}

#[test]
fn an_ended_patch_shows_its_end_card_and_close_until_closed() {
    let mut s = window();
    push(
        &mut s,
        grid(|g| {
            g.over = Some(0);
            g.xp = 25;
        }),
    );
    assert!(shown(&s, "BenillaHerbEnd"));
    assert_eq!(text(&s, "BenillaHerbEndReason"), "Out of pulls.");
    assert_eq!(
        text(&s, "BenillaHerbEndSummary"),
        "You gathered 0 plants · +25 XP"
    );
    assert!(shown(&s, "BenillaHerbClose"));
    assert!(!shown(&s, "BenillaHerbLeave"));
    // The realm closes the patch after its last grid; the window stays on its end card.
    s.fire_event("HERB_GRID_CLOSED", vec![ScriptValue::Int(0)]);
    s.set_herb_grid(None);
    s.tick(0.0);
    s.resolve();
    assert!(shown(&s, "BenillaHerbPatch"));
    click(&s, 1);
    s.run("this = BenillaHerbClose BenillaHerbClose:Click()")
        .unwrap();
    assert!(!shown(&s, "BenillaHerbPatch"));
    assert!(s.take_herb_calls().is_empty());
}

#[test]
fn hovering_a_stake_rings_the_eight_tiles_it_counts() {
    let mut s = window();
    push(&mut s, mixed());
    s.run("this = BenillaHerbTile2 BenillaHerbTile_OnEnter()")
        .unwrap();
    assert!(shown(&s, "BenillaHerbTile1Highlight"));
    assert!(shown(&s, "BenillaHerbTile8Highlight"));
    assert!(!shown(&s, "BenillaHerbTile2Highlight"));
    assert!(!shown(&s, "BenillaHerbTile5Highlight"));
}

#[test]
fn clicking_a_hidden_tile_queues_a_pull_with_its_zero_based_number() {
    let mut s = window();
    push(&mut s, mixed());
    click(&s, 4);
    click(&s, 1);
    assert_eq!(s.take_herb_calls(), vec![HerbCall::Pull(3)]);
}

#[test]
fn sweep_mode_sends_the_skill_across_or_down() {
    let mut s = window();
    named(&mut s, 961101, "Herbalist's Sickle");
    push(
        &mut s,
        grid(|g| {
            g.tool = 961101;
            g.skill_cost = 2;
        }),
    );
    assert_eq!(text(&s, "BenillaHerbSkill"), "Sweep · 2 pulls");
    assert_eq!(
        plain(&s, "BenillaHerbChip1Text"),
        "Tool Herbalist's Sickle · Sweep costs 2 pulls"
    );
    s.run("this = BenillaHerbSkill BenillaHerbSkill_OnClick()")
        .unwrap();
    assert!(shown(&s, "BenillaHerbAcross"));
    click(&s, 8);
    assert_eq!(
        s.take_herb_calls(),
        vec![HerbCall::Sweep {
            tile: 7,
            across: true
        }]
    );
    s.run("this = BenillaHerbSkill BenillaHerbSkill_OnClick()")
        .unwrap();
    s.run("this = BenillaHerbAcross BenillaHerbAcross_OnClick()")
        .unwrap();
    click(&s, 8);
    assert_eq!(
        s.take_herb_calls(),
        vec![HerbCall::Sweep {
            tile: 7,
            across: false
        }]
    );
}

#[test]
fn herb_sight_waits_for_three_hidden_tiles() {
    let mut s = window();
    named(&mut s, 961202, "Elixir of Herb Sight");
    push(
        &mut s,
        grid(|g| {
            g.consumable = 961202;
            g.consumable_ready = true;
        }),
    );
    assert_eq!(
        text(&s, "BenillaHerbConsumable"),
        "Use Elixir of Herb Sight"
    );
    assert_eq!(
        plain(&s, "BenillaHerbChip1Text"),
        "Item Elixir of Herb Sight · ready"
    );
    s.run("this = BenillaHerbConsumable BenillaHerbConsumable_OnClick()")
        .unwrap();
    click(&s, 1);
    click(&s, 5);
    assert!(s.take_herb_calls().is_empty());
    click(&s, 25);
    assert_eq!(s.take_herb_calls(), vec![HerbCall::Sight([0, 4, 24])]);
}

#[test]
fn the_buttons_queue_the_charm_and_the_leave() {
    let mut s = window();
    named(&mut s, 961206, "Forager's Charm");
    push(
        &mut s,
        grid(|g| {
            g.consumable = 961206;
            g.consumable_ready = true;
        }),
    );
    s.run("this = BenillaHerbConsumable BenillaHerbConsumable_OnClick()")
        .unwrap();
    s.run("this = BenillaHerbLeave BenillaHerbLeave:Click()")
        .unwrap();
    assert_eq!(
        s.take_herb_calls(),
        vec![HerbCall::ArmCharm, HerbCall::Leave]
    );
}

#[test]
fn the_pull_log_puts_the_newest_entry_at_the_top() {
    let mut s = window();
    let entry = |kind: u8, title: &str, place: &str, xp: u32| {
        grid(|g| {
            g.log = Some(HerbLogView {
                kind,
                title: title.into(),
                place: place.into(),
                xp,
            })
        })
    };
    push(&mut s, entry(0, "Peacebloom x2", "B3", 5));
    push(&mut s, entry(3, "Sweep", "A1-C1", 0));
    push(&mut s, grid(|_| {}));
    assert_eq!(text(&s, "BenillaHerbLogEntry1Title"), "Sweep");
    assert_eq!(text(&s, "BenillaHerbLogEntry1Place"), "A1-C1");
    assert_eq!(text(&s, "BenillaHerbLogEntry1Xp"), "");
    assert_eq!(text(&s, "BenillaHerbLogEntry2Title"), "Peacebloom x2");
    assert_eq!(text(&s, "BenillaHerbLogEntry2Xp"), "+5 XP");
    assert!(!shown(&s, "BenillaHerbLogEntry3"));
}

#[test]
fn the_title_sits_inside_the_margin() {
    let mut s = window();
    push(&mut s, grid(|_| {}));
    let inset = s
        .eval::<f32>("return BenillaHerbTitle:GetLeft() - BenillaHerbPatch:GetLeft()")
        .unwrap();
    assert!(inset >= 18.0, "the title is {inset} inside the frame");
}

#[test]
fn the_garden_and_sidebar_fit_small_screens() {
    for (width, height) in [(1024.0, 768.0), (800.0, 600.0)] {
        let mut s = window();
        s.set_screen_size(width, height);
        push(&mut s, grid(|_| {}));
        assert!(
            s.eval::<bool>(
                "local scale = BenillaHerbPatch:GetEffectiveScale() \
                 return BenillaHerbPatch:GetLeft() >= 0 \
                 and BenillaHerbLogPanel:GetRight() * scale <= GetScreenWidth() \
                 and BenillaHerbPatch:GetTop() * scale <= GetScreenHeight() \
                 and BenillaHerbPatch:GetBottom() >= 0 \
                 and BenillaHerbLogEntry12:GetBottom() >= BenillaHerbLegend:GetTop() \
                 and BenillaHerbBasket6:GetBottom() >= BenillaHerbLogPanel:GetBottom() \
                 and BenillaHerbSkill:GetTop() < BenillaHerbBed:GetBottom()",
            )
            .unwrap(),
            "garden overflow at {width} x {height}"
        );
    }
}

#[test]
fn closing_hides_the_window_and_pushing_none_does_too() {
    let mut s = window();
    push(&mut s, mixed());
    assert!(shown(&s, "BenillaHerbPatch"));
    s.fire_event("HERB_GRID_CLOSED", vec![ScriptValue::Int(1)]);
    s.resolve();
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    assert!(!shown(&s, "BenillaHerbPatch"));
    push(&mut s, mixed());
    assert!(shown(&s, "BenillaHerbPatch"));
    s.set_herb_grid(None);
    s.tick(0.0);
    s.resolve();
    assert!(!shown(&s, "BenillaHerbPatch"));
}

/// A real left click at the frame's centre, through the hit test.
fn mouse_click(s: &mut UiScript, frame: &str) {
    s.resolve();
    let (x, y) = s
        .eval::<(f64, f64)>(&format!("return {frame}:GetCenter()"))
        .unwrap();
    s.mouse_move(x as f32, y as f32);
    s.mouse_button(x as f32, y as f32, "LeftButton", true);
    s.mouse_button(x as f32, y as f32, "LeftButton", false);
}

fn press(s: &UiScript, button: &str) {
    s.run(&format!("this = {button} {button}:Click()")).unwrap();
}

/// The tour's step numbers, space separated.
fn tour_steps(s: &UiScript) -> String {
    s.eval::<String>(
        "return BenillaHerbState.tour and table.concat(BenillaHerbState.tour.steps, ' ') or ''",
    )
    .unwrap()
}

#[test]
fn a_first_grid_starts_the_tour_at_step_one() {
    let mut s = toured(false);
    push(&mut s, mixed());
    assert!(shown(&s, "BenillaHerbTour"));
    assert!(shown(&s, "BenillaHerbTourRing1"));
    assert_eq!(
        text(&s, "BenillaHerbTourText"),
        "Each pull spends one. The patch ends when they run out."
    );
    // No Tool and no consumable: the skill and Item steps drop out.
    assert_eq!(tour_steps(&s), "1 2 3 4 7 8");
    assert_eq!(text(&s, "BenillaHerbTourCount"), "1 / 6");
    assert_eq!(text(&s, "BenillaHerbTourNext"), "Next");
}

#[test]
fn the_tour_skips_the_steps_whose_regions_are_hidden() {
    let mut s = toured(false);
    named(&mut s, 961101, "Herbalist's Sickle");
    push(
        &mut s,
        grid(|g| {
            g.tool = 961101;
            g.skill_cost = 2;
        }),
    );
    assert_eq!(tour_steps(&s), "1 2 3 5 7 8");
    for _ in 0..3 {
        press(&s, "BenillaHerbTourNext");
    }
    assert_eq!(text(&s, "BenillaHerbTourCount"), "4 / 6");
    assert_eq!(
        text(&s, "BenillaHerbTourText"),
        "Your Tool's skill: Sweep pulls a line of 3 tiles for 2 pulls."
    );
    press(&s, "BenillaHerbTourNext");
    assert!(
        shown(&s, "BenillaHerbTourRing2"),
        "the Basket and the pull log"
    );
    press(&s, "BenillaHerbTourNext");
    assert_eq!(text(&s, "BenillaHerbTourCount"), "6 / 6");
    assert_eq!(text(&s, "BenillaHerbTourNext"), "Done");
    assert!(!shown(&s, "BenillaHerbTourRing2"));
    press(&s, "BenillaHerbTourNext");
    assert!(!shown(&s, "BenillaHerbTour"));
    assert_eq!(s.take_herb_calls(), vec![HerbCall::TourSeen]);
}

#[test]
fn the_item_step_reads_the_consumables_description() {
    let mut s = toured(false);
    s.set_item_template(
        961202,
        ItemTemplateView {
            name: "Elixir of Herb Sight".into(),
            description: "Shows what three hidden tiles hold.".into(),
            ..Default::default()
        },
    );
    push(
        &mut s,
        grid(|g| {
            g.consumable = 961202;
            g.consumable_ready = true;
        }),
    );
    assert_eq!(tour_steps(&s), "1 2 3 6 7 8");
    for _ in 0..3 {
        press(&s, "BenillaHerbTourNext");
    }
    assert_eq!(
        text(&s, "BenillaHerbTourText"),
        "Your consumable for this patch: Shows what three hidden tiles hold."
    );
}

#[test]
fn every_callout_sits_inside_the_window_and_off_its_outline() {
    let mut s = toured(false);
    named(&mut s, 961101, "Herbalist's Sickle");
    named(&mut s, 961202, "Elixir of Herb Sight");
    named(&mut s, 2447, "Peacebloom");
    push(
        &mut s,
        grid(|g| {
            g.tool = 961101;
            g.skill_cost = 2;
            g.consumable = 961202;
            g.consumable_ready = true;
            g.known = mixed().known;
        }),
    );
    assert_eq!(tour_steps(&s), "1 2 3 4 5 6 7 8");
    for step in 1..=8 {
        s.resolve();
        let fits = s
            .eval::<String>(
                "local c, p, r = BenillaHerbTourCallout, BenillaHerbPatch, BenillaHerbTourRing1 \
                 local inside = c:GetLeft() >= p:GetLeft() and c:GetRight() <= p:GetRight() \
                     and c:GetTop() <= p:GetTop() and c:GetBottom() >= p:GetBottom() \
                 local apart = c:GetLeft() >= r:GetRight() or c:GetRight() <= r:GetLeft() \
                     or c:GetBottom() >= r:GetTop() or c:GetTop() <= r:GetBottom() \
                 return (inside and 'in' or 'out') .. ' ' .. (apart and 'apart' or 'over')",
            )
            .unwrap();
        assert_eq!(fits, "in apart", "step {step}");
        press(&s, "BenillaHerbTourNext");
    }
    assert!(!shown(&s, "BenillaHerbTour"));
}

#[test]
fn skip_hides_the_tour_and_queues_one_tour_seen() {
    let mut s = toured(false);
    push(&mut s, mixed());
    press(&s, "BenillaHerbTourSkip");
    press(&s, "BenillaHerbTourSkip");
    assert!(!shown(&s, "BenillaHerbTour"));
    assert_eq!(s.take_herb_calls(), vec![HerbCall::TourSeen]);
}

#[test]
fn a_seen_tour_waits_for_the_help_button() {
    let mut s = toured(true);
    push(&mut s, mixed());
    assert!(!shown(&s, "BenillaHerbTour"));
    assert_eq!(text(&s, "BenillaHerbTourHelp"), "?");
    mouse_click(&mut s, "BenillaHerbTourHelp");
    assert!(shown(&s, "BenillaHerbTour"));
    assert_eq!(text(&s, "BenillaHerbTourCount"), "1 / 6");
    // A replay tells the realm too.
    press(&s, "BenillaHerbTourSkip");
    assert_eq!(s.take_herb_calls(), vec![HerbCall::TourSeen]);
}

#[test]
fn the_tour_blocks_clicks_on_the_grid() {
    let mut s = toured(false);
    push(&mut s, mixed());
    mouse_click(&mut s, "BenillaHerbTile4");
    mouse_click(&mut s, "BenillaHerbLeave");
    assert!(s.take_herb_calls().is_empty());
    mouse_click(&mut s, "BenillaHerbTourSkip");
    assert_eq!(s.take_herb_calls(), vec![HerbCall::TourSeen]);
    mouse_click(&mut s, "BenillaHerbTile4");
    assert_eq!(s.take_herb_calls(), vec![HerbCall::Pull(3)]);
}

#[test]
fn the_grid_closing_ends_the_tour_unseen() {
    let mut s = toured(false);
    push(&mut s, mixed());
    s.fire_event("HERB_GRID_CLOSED", vec![ScriptValue::Int(3)]);
    s.set_herb_grid(None);
    s.resolve();
    assert!(!shown(&s, "BenillaHerbTour"));
    assert!(s.take_herb_calls().is_empty());
    // The realm still says unseen, so the next patch tours again.
    push(&mut s, mixed());
    assert!(shown(&s, "BenillaHerbTour"));
}
