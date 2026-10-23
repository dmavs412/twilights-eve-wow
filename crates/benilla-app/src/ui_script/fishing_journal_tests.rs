//! The Fishing Journal (`assets/ui/FishingJournal.xml`) over pushed journals: the fish slots, the
//! bait slot and its commands, and the key that toggles the window. It needs no game data, so a
//! missing font only warns.

use benilla_ui::script::{JournalLegend, JournalView, JournalZone, UiScript};

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
    }
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
