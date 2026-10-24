//! The Fishing Journal (`assets/ui/FishingJournal.xml`) over pushed journals: the fish slots, the
//! bait slot and its commands, and the key that toggles the window. It needs no game data, so a
//! missing font only warns.

use benilla_ui::script::{
    ContainerSlot, ContainerState, GearRequest, JournalLegend, JournalPage, JournalView,
    JournalZone, UiScript,
};

use super::test_ui::load_ui as load_xml;

fn journal() -> UiScript {
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1024.0, 768.0);
    load_xml(&s, "FishingJournal.xml");
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

/// Two zones: the first with one caught and one uncaught fish, the second with one fish.
fn two_zones() -> JournalView {
    JournalView {
        bait: Some(6529),
        zones: vec![
            JournalZone {
                name: "Elwynn Forest".into(),
                fish: vec![(6291, 1), (6308, 0)],
            },
            JournalZone {
                name: "Westfall".into(),
                fish: vec![(6289, 2)],
            },
        ],
        legends: vec![JournalLegend {
            item: 6317,
            zone: "Elwynn Forest".into(),
            hint_found: true,
            caught: false,
        }],
        ..Default::default()
    }
}

/// `two_zones` with a pole, a Shiny Bauble lure at 14 / 20 and a page for the caught fish: 12
/// catches, 3 perfect, one place, tier 1 unlocked and the next tier 10 catches away.
fn with_gear_and_page() -> JournalView {
    JournalView {
        pole: Some(6256),
        lure: Some((6529, 14, 20)),
        pages: vec![JournalPage {
            item: 6291,
            caught: 12,
            perfect: 3,
            places: vec![("Elwynn Forest".into(), "Northshire River".into(), 8)],
            revealed: 1,
            next_reveal: 10,
            lives_in: vec![("Elwynn Forest".into(), "Northshire River".into())],
            ..Default::default()
        }],
        ..two_zones()
    }
}

/// Put an item from bag 0, slot 2 on the cursor.
fn pick_up_from_bags(s: &mut UiScript, item_id: u32) {
    let mut slots = std::collections::HashMap::new();
    slots.insert(
        2,
        ContainerSlot {
            item_id,
            count: 1,
            ..Default::default()
        },
    );
    s.set_container(
        0,
        Some(ContainerState {
            name: Some("Backpack".into()),
            num_slots: 16,
            slots,
        }),
    );
    s.run("PickupContainerItem(0, 2)").unwrap();
}

fn click(s: &UiScript, frame: &str, button: &str) {
    s.run(&format!(
        "this = {frame} BenillaFishingJournalSlot_OnClick('{button}')"
    ))
    .unwrap();
}

/// The shown fish slots whose label reads `label`, and those whose slot holds a caught fish.
fn fish_slots(s: &UiScript) -> (i64, i64) {
    s.eval::<(i64, i64)>(
        "local unknown, caught = 0, 0
         for n = 1, BENILLA_JOURNAL_FISH_SLOTS do
             local slot = getglobal('BenillaFishingJournalFish' .. n)
             if slot:IsVisible() then
                 if getglobal(slot:GetName() .. 'Label'):GetText() == '???' then
                     unknown = unknown + 1
                 end
                 if slot.caught then
                     caught = caught + 1
                 end
             end
         end
         return unknown, caught",
    )
    .unwrap()
}

#[test]
fn a_pushed_journal_shows_one_icon_and_one_unknown() {
    let mut s = journal();
    s.set_fishing_journal(two_zones());
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s, 0.05);

    assert!(shown(&s, "BenillaFishingJournal"));
    assert_eq!(text(&s, "BenillaFishingJournalZoneName"), "Elwynn Forest");
    assert_eq!(fish_slots(&s), (1, 1), "one ???, one caught icon");
    assert!(
        !shown(&s, "BenillaFishingJournalFish3"),
        "a slot with no fish"
    );
    assert!(
        shown(&s, "BenillaFishingJournalFish1Pip1"),
        "a caught fish shows its pips"
    );
    assert!(
        !shown(&s, "BenillaFishingJournalFish2Pip1"),
        "an uncaught fish shows none"
    );
}

#[test]
fn the_arrows_page_to_the_next_zone() {
    let mut s = journal();
    s.set_fishing_journal(two_zones());
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    s.run("BenillaFishingJournal_Turn(1)").unwrap();
    frame(&mut s, 0.05);

    assert_eq!(text(&s, "BenillaFishingJournalZoneName"), "Westfall");
    assert_eq!(text(&s, "BenillaFishingJournalPage"), "2 / 2");
    assert_eq!(fish_slots(&s), (0, 1));
}

#[test]
fn a_found_hint_marks_the_legend_slot_and_tooltips_the_note() {
    let mut s = journal();
    s.set_fishing_journal(two_zones());
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s, 0.05);

    assert!(s
        .eval::<bool>("return BenillaFishingJournalLegend1.hint ~= nil")
        .unwrap());
    assert!(s
        .eval::<bool>("return BenillaFishingJournalLegend2.hint == nil")
        .unwrap());
    assert!(!s
        .eval::<bool>("return BenillaFishingJournalLegend1.caught")
        .unwrap());
}

#[test]
fn setting_the_bait_queues_the_command_and_a_right_click_clears_it() {
    let mut s = journal();
    s.set_fishing_journal(two_zones());
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s, 0.05);

    s.run("SetFishingBait(6529)").unwrap();
    assert_eq!(s.take_bait_requests(), vec![Some(6529)]);

    s.run("this = BenillaFishingJournalBait BenillaFishingJournalSlot_OnClick('RightButton')")
        .unwrap();
    assert_eq!(s.take_bait_requests(), vec![None]);

    s.run("SetFishingBait(nil)").unwrap();
    assert_eq!(s.take_bait_requests(), vec![None]);
    assert!(s.take_bait_requests().is_empty(), "the queue drains");
}

#[test]
fn a_left_click_with_nothing_on_the_cursor_queues_nothing() {
    let mut s = journal();
    s.run("this = BenillaFishingJournalBait BenillaFishingJournalSlot_OnClick('LeftButton')")
        .unwrap();
    assert!(s.take_bait_requests().is_empty());
}

#[test]
fn the_lure_slot_shows_its_icon_and_durability() {
    let mut s = journal();
    s.set_fishing_journal(with_gear_and_page());
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s, 0.05);

    assert!(shown(&s, "BenillaFishingJournalPoleIcon"));
    assert!(shown(&s, "BenillaFishingJournalLureIcon"));
    assert_eq!(text(&s, "BenillaFishingJournalLureCount"), "14/20");
    assert_eq!(text(&s, "BenillaFishingJournalPoleHint"), "");
    assert_eq!(
        s.eval::<(Option<f64>, Option<f64>, Option<f64>, Option<f64>)>("return GetFishingGear()")
            .unwrap(),
        (Some(6256.0), Some(6529.0), Some(14.0), Some(20.0))
    );

    s.set_fishing_journal(two_zones());
    s.run("BenillaFishingJournal_Paint()").unwrap();
    assert!(!shown(&s, "BenillaFishingJournalLureIcon"));
    assert_eq!(text(&s, "BenillaFishingJournalLureCount"), "");
    assert_eq!(
        text(&s, "BenillaFishingJournalLureHint"),
        "Drop a lure here"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalPoleHint"),
        "Drop a pole here"
    );
}

#[test]
fn dropping_on_the_pole_slot_queues_the_gear_and_a_right_click_clears_it() {
    let mut s = journal();
    s.set_fishing_journal(two_zones());
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s, 0.05);

    pick_up_from_bags(&mut s, 6256);
    click(&s, "BenillaFishingJournalPole", "LeftButton");
    assert_eq!(
        s.take_gear_requests(),
        vec![GearRequest {
            lure: false,
            from: Some((0, 2))
        }]
    );
    assert!(s.cursor_payload().is_none(), "the drop clears the cursor");

    click(&s, "BenillaFishingJournalPole", "RightButton");
    click(&s, "BenillaFishingJournalLure", "RightButton");
    assert_eq!(
        s.take_gear_requests(),
        vec![
            GearRequest {
                lure: false,
                from: None
            },
            GearRequest {
                lure: true,
                from: None
            }
        ]
    );

    click(&s, "BenillaFishingJournalLure", "LeftButton");
    assert!(
        s.take_gear_requests().is_empty(),
        "an empty cursor queues nothing"
    );
}

#[test]
fn a_caught_fish_opens_its_page_and_back_returns_to_the_grid() {
    let mut s = journal();
    s.set_fishing_journal(with_gear_and_page());
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s, 0.05);

    click(&s, "BenillaFishingJournalFish1", "LeftButton");
    frame(&mut s, 0.05);

    assert!(!shown(&s, "BenillaFishingJournalFish1"), "the grid hides");
    assert!(!shown(&s, "BenillaFishingJournalLegend1"));
    assert!(!shown(&s, "BenillaFishingJournalPrev"));
    assert!(shown(&s, "BenillaFishingJournalBack"));
    assert_eq!(
        text(&s, "BenillaFishingJournalPageCaught"),
        "Caught 12, perfect 3"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalLine1"),
        "Northshire River, Elwynn Forest: 8"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalLine2"),
        "Lives in: Northshire River, Elwynn Forest"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalLine3"),
        "??? (catch 10 to learn)"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalLine4"),
        "??? (catch 25 to learn)"
    );
    assert_eq!(text(&s, "BenillaFishingJournalLine5"), "");

    // An update repaints the page that is open.
    s.fire_event("FISHING_JOURNAL_UPDATE", Vec::new());
    frame(&mut s, 0.05);
    assert!(shown(&s, "BenillaFishingJournalBack"));

    s.run("BenillaFishingJournal_Back()").unwrap();
    frame(&mut s, 0.05);
    assert!(shown(&s, "BenillaFishingJournalFish1"));
    assert!(shown(&s, "BenillaFishingJournalLegend1"));
    assert!(!shown(&s, "BenillaFishingJournalBack"));
    assert_eq!(text(&s, "BenillaFishingJournalZoneName"), "Elwynn Forest");
}

#[test]
fn unlocked_tiers_show_their_facts() {
    let mut s = journal();
    let mut view = with_gear_and_page();
    view.pages[0].revealed = 3;
    view.pages[0].next_reveal = 0;
    view.pages[0].kinds = vec!["river".into(), "lake".into(), "pond".into()];
    view.pages[0].peak_hours = Some((6, 10));
    view.pages[0].favourite_bait = Some(6529);
    s.set_fishing_journal(view.clone());
    s.run("BenillaFishingJournal_OpenPage(6291)").unwrap();
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s, 0.05);

    assert_eq!(
        text(&s, "BenillaFishingJournalLine3"),
        "Water: river, lake, pond"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalLine4"),
        "Bites most: 6:00 to 10:00"
    );
    assert!(text(&s, "BenillaFishingJournalLine5").starts_with("Favourite bait: "));

    view.pages[0].peak_hours = None;
    s.set_fishing_journal(view);
    s.run("BenillaFishingJournal_Paint()").unwrap();
    assert_eq!(text(&s, "BenillaFishingJournalLine4"), "Bites at any hour");
}

#[test]
fn the_binding_and_the_slash_command_toggle_the_window() {
    let mut s = journal();
    super::manifest::register_layer_bindings(&s);
    frame(&mut s, 0.05);
    assert!(!shown(&s, "BenillaFishingJournal"));

    assert!(s.execute_binding("TOGGLEFISHINGJOURNAL", true).unwrap());
    assert!(shown(&s, "BenillaFishingJournal"), "the binding opens it");
    assert!(s.execute_binding("TOGGLEFISHINGJOURNAL", true).unwrap());
    assert!(!shown(&s, "BenillaFishingJournal"), "and closes it");

    s.run("SlashCmdList['BENILLAJOURNAL']()").unwrap();
    assert!(shown(&s, "BenillaFishingJournal"), "/journal opens it");
}

#[test]
fn the_default_key_is_j_unless_the_chain_binds_it() {
    let keys = super::manifest::default_bindings();
    let bound = keys
        .iter()
        .find(|(_, command)| command == "TOGGLEFISHINGJOURNAL");
    let taken = keys
        .iter()
        .any(|(key, command)| key == "J" && command != "TOGGLEFISHINGJOURNAL");
    assert!(
        bound.is_some_and(|(key, _)| key == "J") || taken,
        "J opens the journal, or the chain already uses J: {keys:?}"
    );
}
