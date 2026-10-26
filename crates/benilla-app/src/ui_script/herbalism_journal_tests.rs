//! The Journal's Herbalism tab (`assets/ui/FishingJournal.xml`, #443) over pushed views: the
//! bookmark, the plant grid and its Plant Page tooltip, the Satchel's clicks and the perk list. It
//! needs no game data, so a missing font only warns.

use benilla_ui::script::{
    HerbJournalCall, HerbJournalView, HerbPerkView, HerbalistView, ItemTemplateView, SatchelSlot,
    UiScript,
};

use super::test_ui::load_ui as load_xml;

const SICKLE: u32 = 961101;
const ELIXIR: u32 = 961202;
const PEACEBLOOM: u32 = 2447;
const SILVERLEAF: u32 = 765;

fn journal() -> UiScript {
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1024.0, 768.0);
    load_xml(&s, "FishingJournal.xml");
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    s.run(
        "GameTooltip = { lines = {} }
         function GameTooltip:SetOwner() self.lines = {} end
         function GameTooltip:SetHyperlink(link) self.lines = { link } end
         function GameTooltip:SetText(text) self.lines = { text } end
         function GameTooltip:AddLine(text) table.insert(self.lines, text) end
         function GameTooltip:Show() end
         function GameTooltip:Hide() end
         popups = {}
         function StaticPopup_Show(which, a, b, data)
             table.insert(popups, { which = which, name = a, data = data })
         end",
    )
    .unwrap();
    for (item, name) in [
        (SICKLE, "Herbalist's Sickle"),
        (ELIXIR, "Elixir of Herb Sight"),
        (PEACEBLOOM, "Peacebloom"),
        (SILVERLEAF, "Silverleaf"),
    ] {
        s.set_item_template(
            item,
            ItemTemplateView {
                name: name.into(),
                ..Default::default()
            },
        );
    }
    s
}

fn frame(s: &mut UiScript) {
    s.tick(0.05);
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

fn herbalist(knows: bool, level: u32, perks: &[&str]) -> HerbalistView {
    HerbalistView {
        knows,
        level,
        xp: 40,
        to_next: 100,
        title: "Herb Gatherer".into(),
        perks: perks.iter().map(|k| k.to_string()).collect(),
    }
}

fn perk(key: &str, name: &str, boon: bool) -> HerbPerkView {
    HerbPerkView {
        key: key.into(),
        name: name.into(),
        boon,
        text: format!("{name} text."),
    }
}

/// One zone with Peacebloom unpicked and Silverleaf picked 7 times, a Sickle and three Elixirs in the
/// Satchel, and two perks.
fn view() -> HerbJournalView {
    HerbJournalView {
        satchel: vec![
            SatchelSlot {
                item: SICKLE,
                count: 1,
                ..Default::default()
            },
            SatchelSlot {
                item: ELIXIR,
                count: 3,
                ..Default::default()
            },
        ],
        picks: vec![(SILVERLEAF, 7)],
        zones: vec![(12, vec![PEACEBLOOM, SILVERLEAF])],
        reveal_at: [5, 10, 25],
        perks: vec![
            perk("gloves", "Thick Gloves", true),
            perk("bramble", "Bramble Bargain", false),
        ],
        ..Default::default()
    }
}

/// An open Journal on the Herbalism bookmark.
fn open(s: &mut UiScript, herbalist: HerbalistView) {
    s.set_herbalist(herbalist);
    s.set_herb_journal(view());
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    s.run("this = BenillaFishingJournalProf2 BenillaJournalProfession_OnClick()")
        .unwrap();
    frame(s);
}

#[test]
fn the_herbalism_bookmark_shows_the_plants_of_a_zone() {
    let mut s = journal();
    open(&mut s, herbalist(true, 1, &[]));

    assert!(s
        .eval::<bool>("return BenillaFishingJournalProf2.selected")
        .unwrap());
    assert!(shown(&s, "BenillaFishingJournalHerbZones"));
    assert!(!shown(&s, "BenillaFishingJournalZones"));
    assert_eq!(
        text(&s, "BenillaFishingJournalHerbZoneName"),
        "Elwynn Forest"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalHerbPlant1Name"),
        "Unknown plant"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalHerbPlant2Name"),
        "Silverleaf"
    );
    assert!(!shown(&s, "BenillaFishingJournalHerbPlant3"));

    s.run("this = BenillaFishingJournalHerbPlant2 BenillaFishingJournalSlot_OnEnter()")
        .unwrap();
    let lines = s.eval::<Vec<String>>("return GameTooltip.lines").unwrap();
    assert_eq!(
        lines,
        vec![
            "Silverleaf",
            "Picked 7 times",
            "At 10 picks: a patch names its rows."
        ]
    );
    s.run("this = BenillaFishingJournalHerbPlant1 BenillaFishingJournalSlot_OnEnter()")
        .unwrap();
    let lines = s.eval::<Vec<String>>("return GameTooltip.lines").unwrap();
    assert_eq!(lines[0], "Unknown plant");

    // The fishing frames come back with the Fishing bookmark.
    s.run("this = BenillaFishingJournalProf1 BenillaJournalProfession_OnClick()")
        .unwrap();
    assert!(shown(&s, "BenillaFishingJournalZones"));
    assert!(!shown(&s, "BenillaFishingJournalHerbZones"));
}

#[test]
fn a_plant_page_names_the_next_unlock_and_then_none() {
    let mut s = journal();
    open(&mut s, herbalist(true, 1, &[]));
    let lines = |picks: u32| {
        s.eval::<Vec<String>>(&format!("return BenillaHerbJournal_PageLines({picks})"))
            .unwrap()
    };
    assert_eq!(
        lines(1),
        vec![
            "Picked 1 time",
            "At 5 picks: a patch tells you it is there."
        ]
    );
    assert_eq!(lines(12)[1], "At 25 picks: its bush glows.");
    assert_eq!(lines(25)[1], "You know this plant by heart.");
}

#[test]
fn left_clicking_a_satchel_consumable_slots_it_and_a_tool_slots_the_tool() {
    let mut s = journal();
    open(&mut s, herbalist(true, 1, &[]));
    s.run("BenillaJournal_ShowPage('Inventory')").unwrap();
    frame(&mut s);

    assert!(shown(&s, "BenillaFishingJournalHerbTool"));
    s.run(
        "this = BenillaFishingJournalHerbSatchel2 BenillaFishingJournalSlot_OnClick('LeftButton')",
    )
    .unwrap();
    s.run(
        "this = BenillaFishingJournalHerbSatchel1 BenillaFishingJournalSlot_OnClick('LeftButton')",
    )
    .unwrap();
    assert_eq!(
        s.take_herb_journal_calls(),
        vec![
            HerbJournalCall::SetConsumable(Some(ELIXIR)),
            HerbJournalCall::SetTool(SICKLE)
        ]
    );

    // Right-click on a Satchel item withdraws the stack, with shift one.
    s.run(
        "this = BenillaFishingJournalHerbSatchel2 BenillaFishingJournalSlot_OnClick('RightButton')",
    )
    .unwrap();
    s.run("IsShiftKeyDown = function() return 1 end").unwrap();
    s.run(
        "this = BenillaFishingJournalHerbSatchel2 BenillaFishingJournalSlot_OnClick('RightButton')",
    )
    .unwrap();
    assert_eq!(
        s.take_herb_journal_calls(),
        vec![
            HerbJournalCall::Withdraw {
                item: ELIXIR,
                count: 3
            },
            HerbJournalCall::Withdraw {
                item: ELIXIR,
                count: 1
            }
        ]
    );

    // Right-click empties the Consumable slot; the Tool slot cannot be emptied.
    s.run("this = BenillaFishingJournalHerbConsumable BenillaFishingJournalSlot_OnClick('RightButton')")
        .unwrap();
    s.run("this = BenillaFishingJournalHerbTool BenillaFishingJournalSlot_OnClick('RightButton')")
        .unwrap();
    assert_eq!(
        s.take_herb_journal_calls(),
        vec![HerbJournalCall::SetConsumable(None)]
    );
}

#[test]
fn an_opened_perk_slot_lists_the_perks_and_a_confirmed_choice_picks_it() {
    let mut s = journal();
    open(&mut s, herbalist(true, 10, &[""]));
    s.run("BenillaJournal_ShowPage('Progression')").unwrap();
    frame(&mut s);

    assert_eq!(text(&s, "BenillaFishingJournalHerbProgressionLevel"), "10");
    assert_eq!(
        text(&s, "BenillaFishingJournalHerbProgressionTitle"),
        "Herb Gatherer"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalHerbPerk1Name"),
        "Choose a perk"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalHerbPerk2Text"),
        "Unlocks at Herbalist Level 20"
    );
    assert!(!shown(&s, "BenillaFishingJournalHerbPerkList"));

    // A locked slot opens nothing.
    s.run("this = BenillaFishingJournalHerbPerk2 BenillaHerbJournal_PerkRow_OnClick()")
        .unwrap();
    assert!(!shown(&s, "BenillaFishingJournalHerbPerkList"));

    s.run("this = BenillaFishingJournalHerbPerk1 BenillaHerbJournal_PerkRow_OnClick()")
        .unwrap();
    assert!(shown(&s, "BenillaFishingJournalHerbPerkList"));
    assert_eq!(
        text(&s, "BenillaFishingJournalHerbPerkChoice2Name"),
        "Bramble Bargain"
    );
    assert!(!shown(&s, "BenillaFishingJournalHerbPerkChoice3"));

    s.run("this = BenillaFishingJournalHerbPerkChoice2 BenillaHerbJournal_Choice_OnClick()")
        .unwrap();
    assert_eq!(s.eval::<i64>("return table.getn(popups)").unwrap(), 1);
    assert_eq!(
        s.eval::<String>("return popups[1].name").unwrap(),
        "Bramble Bargain"
    );
    assert_eq!(
        s.eval::<String>("return StaticPopupDialogs['BENILLA_HERB_PICK_PERK'].text")
            .unwrap(),
        "This choice is permanent. Choose %s?"
    );
    assert!(
        s.take_herb_journal_calls().is_empty(),
        "nothing before the confirm"
    );

    s.run("StaticPopupDialogs['BENILLA_HERB_PICK_PERK'].OnAccept(popups[1].data)")
        .unwrap();
    assert_eq!(
        s.take_herb_journal_calls(),
        vec![HerbJournalCall::PickPerk {
            slot: 0,
            key: "bramble".into()
        }]
    );
    assert!(!shown(&s, "BenillaFishingJournalHerbPerkList"));
}

#[test]
fn a_filled_perk_slot_shows_its_perk_and_cannot_be_changed() {
    let mut s = journal();
    open(&mut s, herbalist(true, 10, &["gloves"]));
    s.run("BenillaJournal_ShowPage('Progression')").unwrap();
    frame(&mut s);

    assert_eq!(
        text(&s, "BenillaFishingJournalHerbPerk1Name"),
        "Thick Gloves"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalHerbPerk1Text"),
        "Thick Gloves text."
    );
    s.run("this = BenillaFishingJournalHerbPerk1 BenillaHerbJournal_PerkRow_OnClick()")
        .unwrap();
    assert!(!shown(&s, "BenillaFishingJournalHerbPerkList"));
}

#[test]
fn without_herbalism_the_bookmark_does_nothing() {
    let mut s = journal();
    s.set_herbalist(herbalist(false, 0, &[]));
    s.set_herb_journal(view());
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s);

    s.run("this = BenillaFishingJournalProf2 BenillaJournalProfession_OnClick()")
        .unwrap();
    assert!(s
        .eval::<bool>("return BenillaFishingJournalProf1.selected")
        .unwrap());
    assert!(!s
        .eval::<bool>("return BenillaFishingJournalProf2.selected")
        .unwrap());
    assert!(shown(&s, "BenillaFishingJournalZones"));
    assert!(!shown(&s, "BenillaFishingJournalHerbZones"));
}
