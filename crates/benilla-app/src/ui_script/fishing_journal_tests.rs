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
                zone: 12,
                fish: vec![(6291, 1), (6308, 0)],
                ..Default::default()
            },
            JournalZone {
                name: "Westfall".into(),
                zone: 40,
                fish: vec![(6289, 2)],
                ..Default::default()
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
                 if getglobal(slot:GetName() .. 'Name'):GetText() == '???' then
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
    s.run("BenillaJournal_ShowPage('Inventory')").unwrap();
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

/// A stand-in `GameTooltip` that records its lines, since the journal tests load no stock files.
fn stub_tooltip(s: &UiScript) {
    s.run(
        "GameTooltip = { lines = {} }
         function GameTooltip:SetOwner() self.lines = {} end
         function GameTooltip:SetHyperlink(link) self.lines = { link } end
         function GameTooltip:SetText(text) self.lines = { text } end
         function GameTooltip:AddLine(text) table.insert(self.lines, text) end
         function GameTooltip:Show() end
         function GameTooltip:Hide() end",
    )
    .unwrap();
}

fn tooltip_lines(s: &UiScript) -> Vec<String> {
    s.eval::<Vec<String>>("return GameTooltip.lines").unwrap()
}

fn hover(s: &UiScript, frame: &str, handler: &str) {
    s.run(&format!("this = {frame} {handler}()")).unwrap();
}

#[test]
fn the_journal_opens_on_zones_and_a_tab_swaps_the_page() {
    let mut s = journal();
    s.set_fishing_journal(two_zones());
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s, 0.05);

    for tab in 1..=3 {
        assert!(shown(&s, &format!("BenillaFishingJournalTab{tab}")));
    }
    assert_eq!(text(&s, "BenillaFishingJournalTab1"), "Zones");
    assert_eq!(text(&s, "BenillaFishingJournalTab2"), "Inventory");
    assert_eq!(text(&s, "BenillaFishingJournalTab3"), "Progression");
    assert!(shown(&s, "BenillaFishingJournalZones"));
    assert!(!shown(&s, "BenillaFishingJournalInventory"));
    assert!(!shown(&s, "BenillaFishingJournalProgression"));

    s.run("this = BenillaFishingJournalTab3 BenillaJournal_ShowPage('Progression')")
        .unwrap();
    assert!(!shown(&s, "BenillaFishingJournalZones"));
    assert!(shown(&s, "BenillaFishingJournalProgression"));

    // An update repaint keeps the chosen page.
    s.fire_event("FISHING_JOURNAL_UPDATE", Vec::new());
    frame(&mut s, 0.05);
    assert!(shown(&s, "BenillaFishingJournalProgression"));

    s.run("BenillaJournal_ShowPage('Inventory')").unwrap();
    assert!(shown(&s, "BenillaFishingJournalPole"));
}

#[test]
fn fishing_is_the_selected_bookmark_and_herbalism_is_inert() {
    let mut s = journal();
    stub_tooltip(&s);
    s.set_fishing_journal(two_zones());
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s, 0.05);

    assert!(shown(&s, "BenillaFishingJournalProf1"));
    assert!(shown(&s, "BenillaFishingJournalProf2"));
    assert!(
        s.eval::<bool>("return BenillaFishingJournalProf3 == nil")
            .unwrap(),
        "two bookmarks"
    );
    assert!(s
        .eval::<bool>("return BenillaFishingJournalProf1.selected")
        .unwrap());

    click_bookmark(&s, 2);
    assert!(
        s.eval::<bool>("return BenillaFishingJournalProf1.selected")
            .unwrap(),
        "a click on Herbalism changes nothing"
    );
    assert!(!s
        .eval::<bool>("return BenillaFishingJournalProf2.selected")
        .unwrap());

    hover(
        &s,
        "BenillaFishingJournalProf2",
        "BenillaJournalProfession_OnEnter",
    );
    assert_eq!(tooltip_lines(&s), vec!["Herbalism: coming soon"]);
}

fn click_bookmark(s: &UiScript, n: u32) {
    s.run(&format!(
        "this = BenillaFishingJournalProf{n} BenillaJournalProfession_OnClick()"
    ))
    .unwrap();
}

#[test]
fn a_zone_card_counts_its_collected_fish_and_shows_its_description() {
    let mut s = journal();
    let mut view = two_zones();
    view.zones[0] = JournalZone {
        name: "Elwynn Forest".into(),
        zone: 12,
        description: "Slow rivers and a quiet lake.".into(),
        fish: vec![(6291, 1), (6308, 0), (6289, 2)],
    };
    s.set_fishing_journal(view);
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s, 0.05);

    assert_eq!(
        text(&s, "BenillaFishingJournalCommonCount"),
        "2 / 3 Collected"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalDescription"),
        "Slow rivers and a quiet lake."
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalLegendCount"),
        "0 / 4 Collected"
    );
    assert_eq!(text(&s, "BenillaFishingJournalFish2Name"), "???");
    assert_ne!(text(&s, "BenillaFishingJournalFish1Name"), "???");
    assert!(
        shown(&s, "BenillaFishingJournalScene"),
        "Elwynn has a scene"
    );

    s.run("BenillaFishingJournal_Turn(1)").unwrap();
    assert!(
        !shown(&s, "BenillaFishingJournalScene"),
        "a zone missing from the table shows none"
    );
}

#[test]
fn hovering_a_caught_fish_shows_its_page_as_tooltip_lines() {
    let mut s = journal();
    stub_tooltip(&s);
    s.set_fishing_journal(with_gear_and_page());
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s, 0.05);

    hover(
        &s,
        "BenillaFishingJournalFish1",
        "BenillaFishingJournalSlot_OnEnter",
    );
    let lines = tooltip_lines(&s);
    assert_eq!(lines[0], "item:6291:0:0:0", "the item's own tooltip first");
    assert!(lines.contains(&"Lives in: Northshire River, Elwynn Forest".to_string()));
    assert!(lines.contains(&"Northshire River, Elwynn Forest: 8".to_string()));
    assert!(lines.contains(&"??? (catch 10 to learn)".to_string()));

    hover(
        &s,
        "BenillaFishingJournalFish2",
        "BenillaFishingJournalSlot_OnEnter",
    );
    assert_eq!(tooltip_lines(&s), vec!["???", "Catch it to learn more."]);

    assert!(
        !s.eval::<bool>("return BenillaFishingJournalBack ~= nil")
            .unwrap(),
        "no page frame exists"
    );
    click(&s, "BenillaFishingJournalFish1", "LeftButton");
    assert!(
        shown(&s, "BenillaFishingJournalFish1"),
        "a click opens nothing"
    );
}

#[test]
fn unlocked_tiers_show_their_facts() {
    let mut s = journal();
    stub_tooltip(&s);
    let mut view = with_gear_and_page();
    view.pages[0].revealed = 3;
    view.pages[0].next_reveal = 0;
    view.pages[0].kinds = vec!["river".into(), "lake".into(), "pond".into()];
    view.pages[0].peak_hours = Some((6, 10));
    view.pages[0].favourite_bait = Some(6529);
    s.set_fishing_journal(view.clone());
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s, 0.05);

    hover(
        &s,
        "BenillaFishingJournalFish1",
        "BenillaFishingJournalSlot_OnEnter",
    );
    let lines = tooltip_lines(&s);
    assert!(lines.contains(&"Water: river, lake, pond".to_string()));
    assert!(lines.contains(&"Bites most: 6:00 to 10:00".to_string()));
    assert!(lines.iter().any(|l| l.starts_with("Favourite bait: ")));

    view.pages[0].peak_hours = None;
    s.set_fishing_journal(view);
    hover(
        &s,
        "BenillaFishingJournalFish1",
        "BenillaFishingJournalSlot_OnEnter",
    );
    assert!(tooltip_lines(&s).contains(&"Bites at any hour".to_string()));
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

fn progression(s: &mut UiScript) {
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    s.run("BenillaJournal_ShowPage('Progression')").unwrap();
    frame(s, 0.05);
}

#[test]
fn the_progression_tab_shows_the_level_title_and_xp() {
    let mut s = journal();
    s.set_fishing_journal(two_zones());
    s.set_angler(12, 420, 1000, "Apprentice Fisher".into());
    progression(&mut s);

    let p = "BenillaFishingJournalProgression";
    assert_eq!(text(&s, &format!("{p}Level")), "12");
    assert_eq!(text(&s, &format!("{p}Title")), "Apprentice Fisher");
    assert_eq!(text(&s, &format!("{p}XpText")), "420 / 1,000 XP");

    s.set_angler(30, 0, 0, "Master Angler".into());
    s.fire_event("ANGLER_UPDATE", Vec::new());
    frame(&mut s, 0.05);
    assert_eq!(text(&s, &format!("{p}XpText")), "Max level");
    assert_eq!(text(&s, &format!("{p}Title")), "Master Angler");
}

#[test]
fn the_progression_stats_count_each_fish_once() {
    let mut s = journal();
    // Fish 6291 is in both zones and caught in the first only.
    let mut view = two_zones();
    view.zones[1].fish.push((6291, 0));
    view.casts = 1234;
    view.caught = 7;
    s.set_fishing_journal(view);
    s.set_angler(1, 0, 100, String::new());
    progression(&mut s);

    let p = "BenillaFishingJournalProgression";
    assert_eq!(text(&s, &format!("{p}StatValue1")), "7");
    // Caught: 6291 (zone 1) and 6289 (best 2, zone 2). Distinct: 6291, 6308, 6289.
    assert_eq!(text(&s, &format!("{p}StatValue2")), "2 / 3");
    assert_eq!(text(&s, &format!("{p}StatValue3")), "0 / 1");
    assert_eq!(text(&s, &format!("{p}StatValue4")), "2 / 2");
    assert_eq!(text(&s, &format!("{p}StatValue5")), "1,234");
    assert_eq!(text(&s, &format!("{p}StatLabel5")), "Total Casts");
}

#[test]
fn a_zone_with_no_catch_is_not_discovered() {
    let mut s = journal();
    let mut view = two_zones();
    view.zones[1].fish = vec![(6289, 0)];
    s.set_fishing_journal(view);
    progression(&mut s);
    assert_eq!(
        text(&s, "BenillaFishingJournalProgressionStatValue4"),
        "1 / 2"
    );
}

#[test]
fn the_talent_tree_is_locked() {
    let mut s = journal();
    stub_tooltip(&s);
    s.set_fishing_journal(two_zones());
    progression(&mut s);

    let p = "BenillaFishingJournalProgression";
    assert_eq!(text(&s, &format!("{p}Points")), "0 / 0 Points Spent");
    assert_eq!(text(&s, &format!("{p}Caption")), "Coming later");
    for i in 1..=6 {
        assert_eq!(text(&s, &format!("{p}Slot{i}Rank")), "0/5");
    }
    hover(&s, &format!("{p}Slot4"), "BenillaJournalTalent_OnEnter");
    assert!(tooltip_lines(&s)
        .iter()
        .any(|l| l == "Fishing talents are coming later."));
}

#[test]
fn get_angler_info_returns_four_values() {
    let mut s = journal();
    s.set_angler(12, 420, 1000, "Apprentice Fisher".into());
    let n = s
        .eval::<i64>("local t = { GetAnglerInfo() } return table.getn(t)")
        .unwrap();
    assert_eq!(n, 4);
    assert_eq!(
        s.eval::<String>("local _, _, _, t = GetAnglerInfo() return t")
            .unwrap(),
        "Apprentice Fisher"
    );
}

#[test]
fn an_unknown_fish_gets_its_name_once_the_client_learns_the_item() {
    let mut s = journal();
    s.set_fishing_journal(two_zones());
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s, 0.05);
    assert_eq!(
        text(&s, "BenillaFishingJournalFish1Name"),
        "an unknown item"
    );

    s.set_item_template(
        6291,
        benilla_ui::script::ItemTemplateView {
            name: "Raw Brilliant Smallfish".into(),
            ..Default::default()
        },
    );
    frame(&mut s, 0.6);
    assert_eq!(
        text(&s, "BenillaFishingJournalFish1Name"),
        "Raw Brilliant Smallfish",
        "the open Journal repaints until it knows every item"
    );
}
