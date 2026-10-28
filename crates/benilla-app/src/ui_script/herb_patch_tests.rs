//! The Forager's Grid window (`assets/ui/HerbPatch.xml`) over pushed grids: the tile art, the
//! clicks that queue `Herb*` calls, skill mode and the close. It needs no game data, so a missing
//! font only warns.

use benilla_ui::script::{
    HerbCall, HerbGridView, HerbKnownView, HerbLogView, HerbTileView, ItemTemplateView,
    ScriptValue, UiScript,
};

use super::test_ui::load_ui as load_xml;

const ART: &str = "Interface\\Benilla\\herbalism\\";

fn window() -> UiScript {
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1024.0, 768.0);
    load_xml(&s, "HerbPatch.xml");
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
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
    assert_eq!(texture(&s, "BenillaHerbTile1Bg"), format!("{ART}soil"));
    assert_eq!(text(&s, "BenillaHerbTile1Name"), "Peacebloom");
    assert_eq!(text(&s, "BenillaHerbTile1Count"), "2");
    assert_eq!(text(&s, "BenillaHerbTile1Bloom"), "Bloom");
    assert_eq!(texture(&s, "BenillaHerbTile2Bg"), format!("{ART}soil"));
    assert_eq!(
        texture(&s, "BenillaHerbTile2Part"),
        format!("{ART}part_weed")
    );
    assert!(shown(&s, "BenillaHerbTile2Stake"));
    assert_eq!(text(&s, "BenillaHerbTile2StakeText"), "2");
    assert_eq!(
        texture(&s, "BenillaHerbTile3Bg"),
        format!("{ART}bush_thorn")
    );
    assert_eq!(
        texture(&s, "BenillaHerbTile4Bg"),
        format!("{ART}bush_hidden")
    );
    assert!(!shown(&s, "BenillaHerbTile4Stake"));
    assert_eq!(
        text(&s, "BenillaHerbKnown1"),
        "Peacebloom: in this patch, rows 2, 4"
    );
    assert_eq!(text(&s, "BenillaHerbKnown2"), "Silverleaf: not here");
    assert!(shown(&s, "BenillaHerbPip7"));
    assert!(!shown(&s, "BenillaHerbPip8"));
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
    assert_eq!(texture(&s, "BenillaHerbTile1Bg"), format!("{ART}bush_glow"));
    assert_eq!(
        texture(&s, "BenillaHerbTile2Bg"),
        format!("{ART}bush_wilted")
    );
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
    assert_eq!(text(&s, "BenillaHerbSkill"), "Sweep (2)");
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
fn the_pull_log_prints_each_entry_title_and_place_newest_first() {
    let mut s = window();
    let entry = |title: &str, place: &str| {
        grid(|g| {
            g.log = Some(HerbLogView {
                kind: 0,
                title: title.into(),
                place: place.into(),
                xp: 5,
            })
        })
    };
    push(&mut s, entry("Peacebloom x2", "B3"));
    push(&mut s, entry("Sweep", "A1-C1"));
    push(&mut s, grid(|_| {}));
    assert_eq!(text(&s, "BenillaHerbLog1"), "Sweep A1-C1");
    assert_eq!(text(&s, "BenillaHerbLog2"), "Peacebloom x2 B3");
    assert_eq!(text(&s, "BenillaHerbLog3"), "");
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
