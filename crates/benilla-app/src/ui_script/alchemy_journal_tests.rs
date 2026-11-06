//! The Journal's Alchemy bookmark (`assets/ui/FishingJournal.xml`, #546) over pushed views: the
//! bookmark greys with an empty journal, its frame replaces the bottom tabs, and the Discoveries
//! strip, Herb Notes, grade ladders, Satchel and Brew Log draw from the journal. It needs no game
//! data, so a missing font only warns.

use benilla_ui::script::{
    AlchemyCall, AlchemyFlaskView, AlchemyGradePageView, AlchemyJournalView, AlchemyNoteView,
    BrewLogView, ItemTemplateView, SatchelSlot, UiScript,
};

use super::test_ui::load_ui as load_xml;

const PEACEBLOOM: u32 = 2447;
const SILVERLEAF: u32 = 765;
const TINCTURE: u32 = 990101;
const MINOR_HEALING: u32 = 990201;

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
         function GameTooltip:Hide() end",
    )
    .unwrap();
    for (item, name) in [
        (PEACEBLOOM, "Peacebloom"),
        (SILVERLEAF, "Silverleaf"),
        (TINCTURE, "Restorative Tincture"),
        (MINOR_HEALING, "Minor Healing Potion"),
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

fn live(s: &UiScript) -> bool {
    s.eval::<bool>("return BenillaFishingJournalProf4.live and true or false")
        .unwrap()
}

/// One grade page of the Restorative potion ladder.
fn grade(grade: u8, learned: bool, threshold: u32, steps: [u32; 5]) -> AlchemyGradePageView {
    AlchemyGradePageView {
        family: "potion".into(),
        key: "Restorative".into(),
        grade,
        name: format!("Grade {grade} Healing Potion"),
        base: "Healing Potion".into(),
        learned,
        rumoured: false,
        threshold,
        steps,
        effect: String::new(),
    }
}

/// A journal one potion in: Peacebloom unused, Silverleaf filled, the Minor grade learned.
fn one_brew() -> AlchemyJournalView {
    AlchemyJournalView {
        satchel: vec![SatchelSlot {
            item: TINCTURE,
            count: 3,
            ..Default::default()
        }],
        notes: vec![
            AlchemyNoteView {
                item: PEACEBLOOM,
                uses: 0,
                level: 1,
                rarity: 0,
                zones: "Elwynn".into(),
                values: Vec::new(),
                budget: 0,
            },
            AlchemyNoteView {
                item: SILVERLEAF,
                uses: 5,
                level: 3,
                rarity: 1,
                zones: "Elwynn, Redridge".into(),
                values: vec![("Restorative".into(), Some(2)), ("Calming".into(), Some(1))],
                budget: 3,
            },
        ],
        grades: vec![
            grade(0, true, 4, [2, 0, 1, 0, 0]),
            grade(1, false, 0, [0; 5]),
            grade(2, false, 0, [0; 5]),
            grade(3, false, 0, [0; 5]),
        ],
        flasks: vec![AlchemyFlaskView {
            key: "Fortifying+Volatile".into(),
            name: "Flask of the Bulwark".into(),
            stats: "Armour and Attack power".into(),
            made: 0,
        }],
        log: vec![BrewLogView {
            when: "Day 2, 08:00".into(),
            make: "potion".into(),
            inputs: vec![(PEACEBLOOM, 2), (SILVERLEAF, 2)],
            sums: "Restorative 4, Toxic 2".into(),
            toxic: "2 · 33%".into(),
            result: MINOR_HEALING,
            note: "Learned Minor Healing Potion".into(),
        }],
        open: vec!["potion".into()],
        last: "Minor Healing Potion".into(),
    }
}

fn open(s: &mut UiScript, view: AlchemyJournalView) {
    s.set_alchemy_journal(view);
    s.fire_event("ALCHEMY_JOURNAL_UPDATE", Vec::new());
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(s);
}

fn choose_alchemy(s: &mut UiScript) {
    s.run("this = BenillaFishingJournalProf4 BenillaJournalProfession_OnClick()")
        .unwrap();
    frame(s);
}

/// Opens the sub-page with `label`, by its button.
fn sub(s: &mut UiScript, label: &str) {
    s.run(&format!(
        "for i, row in ipairs(BENILLA_JOURNAL_ALCHEMY_SUBS) do
             if row.label == '{label}' then BenillaAlchemyJournal_ShowSub(i) end
         end"
    ))
    .unwrap();
    frame(s);
}

#[test]
fn the_alchemy_bookmark_is_grey_when_empty_and_live_with_a_brew_log_row() {
    let mut s = journal();
    open(&mut s, AlchemyJournalView::default());
    assert!(shown(&s, "BenillaFishingJournalProf4"));
    assert!(!live(&s));
    choose_alchemy(&mut s);
    assert!(
        s.eval::<bool>("return BenillaFishingJournalProf1.selected")
            .unwrap(),
        "a click on a grey Alchemy bookmark changes nothing"
    );

    let mut view = AlchemyJournalView::default();
    view.log = one_brew().log;
    s.set_alchemy_journal(view);
    s.fire_event("ALCHEMY_JOURNAL_UPDATE", Vec::new());
    frame(&mut s);
    assert!(live(&s));
}

#[test]
fn choosing_alchemy_hides_the_tabs_and_draws_the_discoveries_strip() {
    let mut s = journal();
    open(&mut s, one_brew());
    choose_alchemy(&mut s);

    assert_eq!(text(&s, "BenillaFishingJournalTitle"), "Alchemy Journal");
    assert!(shown(&s, "BenillaFishingJournalAlchemy"));
    for frame in [
        "BenillaFishingJournalTab1",
        "BenillaFishingJournalTab2",
        "BenillaFishingJournalTab3",
        "BenillaFishingJournalZones",
        "BenillaFishingJournalHerbZones",
        "BenillaFishingJournalCookPages",
    ] {
        assert!(!shown(&s, frame), "{frame} hides");
    }
    assert_eq!(
        text(&s, "BenillaFishingJournalAlchemyDisc1"),
        "Makes open 1 of 5"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalAlchemyDisc2"),
        "Herbs fully noted 1 of 2"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalAlchemyDisc3"),
        "Grades learned 1 of 4"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalAlchemyDisc4"),
        "Flask kinds made 0 of 1"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalAlchemyLast"),
        "Last learned: Minor Healing Potion"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalAlchemyTry"),
        "Try next: Brew any potion to open the Still."
    );

    // The Fishing bookmark brings the tabs back.
    s.run("this = BenillaFishingJournalProf1 BenillaJournalProfession_OnClick()")
        .unwrap();
    frame(&mut s);
    assert!(shown(&s, "BenillaFishingJournalTab1"));
    assert!(!shown(&s, "BenillaFishingJournalAlchemy"));
}

#[test]
fn the_try_next_line_moves_on_as_the_makes_open() {
    let mut s = journal();
    let mut view = one_brew();
    view.open = ["potion", "tincture", "essence", "oil", "flask"]
        .map(String::from)
        .to_vec();
    open(&mut s, view.clone());
    choose_alchemy(&mut s);
    assert_eq!(
        text(&s, "BenillaFishingJournalAlchemyTry"),
        "Try next: Join two Essences into a flask."
    );

    view.flasks[0].made = 2;
    s.set_alchemy_journal(view);
    s.fire_event("ALCHEMY_JOURNAL_UPDATE", Vec::new());
    frame(&mut s);
    assert_eq!(
        text(&s, "BenillaFishingJournalAlchemyTry"),
        "Try next: 1 herbs still have blanks in their notes. Five uses fill a note."
    );
}

#[test]
fn the_herb_notes_page_draws_an_unknown_herb_and_a_filled_note() {
    let mut s = journal();
    open(&mut s, one_brew());
    choose_alchemy(&mut s);
    sub(&mut s, "Herb Notes");

    assert!(shown(&s, "BenillaFishingJournalAlchemyHerbs"));
    assert!(!shown(&s, "BenillaFishingJournalAlchemySatchel"));
    assert!(shown(&s, "BenillaFishingJournalAlchemyNote1"));
    assert!(shown(&s, "BenillaFishingJournalAlchemyNote2"));
    assert!(!shown(&s, "BenillaFishingJournalAlchemyNote3"));
    assert_eq!(
        text(&s, "BenillaFishingJournalAlchemyNote1Name"),
        "Peacebloom"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalAlchemyNote1Unknown"),
        "Unknown. Use it in any craft to find out what it holds."
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalAlchemyNote1Uses"),
        "0 of 5 uses noted"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalAlchemyNote2Kind"),
        "level 3 · uncommon · Elwynn, Redridge"
    );
    assert_eq!(text(&s, "BenillaFishingJournalAlchemyNote2Unknown"), "");
    assert_eq!(
        format!(
            "{} {}",
            text(&s, "BenillaFishingJournalAlchemyNote2Value1Name"),
            text(&s, "BenillaFishingJournalAlchemyNote2Value1Points")
        ),
        "Restorative 2"
    );
    assert!(shown(&s, "BenillaFishingJournalAlchemyNote2Value1Fill"));
    assert!(!shown(&s, "BenillaFishingJournalAlchemyNote2Value3Track"));
    assert_eq!(
        text(&s, "BenillaFishingJournalAlchemyNote2Uses"),
        "5 of 5 uses noted · budget 3"
    );
}

#[test]
fn the_potions_page_draws_a_learned_minor_and_an_unlearned_lesser() {
    let mut s = journal();
    open(&mut s, one_brew());
    choose_alchemy(&mut s);
    sub(&mut s, "Potions");

    let card = "BenillaFishingJournalAlchemyRecipe1";
    assert!(shown(&s, card));
    assert!(!shown(&s, "BenillaFishingJournalAlchemyRecipe2"));
    assert_eq!(text(&s, &format!("{card}Base")), "Healing Potion");
    assert_eq!(
        text(&s, &format!("{card}Line")),
        "Restorative · Mortar · 2 to 4 herbs · rolls against Toxic"
    );
    assert_eq!(text(&s, &format!("{card}Grade1Label")), "Minor");
    assert_eq!(text(&s, &format!("{card}Grade1Threshold")), "≥ 4");
    assert!(shown(&s, &format!("{card}Grade1Fill")));
    assert!(shown(&s, &format!("{card}Grade1Pip1")));
    assert!(!shown(&s, &format!("{card}Grade1Pip2")));
    assert_eq!(text(&s, &format!("{card}Grade1Pip1Count")), "2");
    assert_eq!(text(&s, &format!("{card}Grade1Pip3Count")), "");
    assert_eq!(
        text(&s, &format!("{card}Grade1Effect")),
        "Made: x1 ×2, x1.5 ×1"
    );
    assert_eq!(text(&s, &format!("{card}Grade2Label")), "Lesser");
    assert_eq!(text(&s, &format!("{card}Grade2Threshold")), "?");
    assert!(!shown(&s, &format!("{card}Grade2Fill")));
    assert_eq!(text(&s, &format!("{card}Grade2Effect")), "Not yet made");
    assert_eq!(text(&s, &format!("{card}Grade3Label")), "Plain");
    // With no Alchemy window open there is no table to target.
    assert!(!shown(&s, &format!("{card}Grade1Make")));
}

#[test]
fn make_this_shows_while_the_alchemy_window_is_open_and_sets_its_target() {
    let mut s = journal();
    s.run(
        "BenillaAlchemy = CreateFrame('Frame', 'BenillaAlchemy', UIParent)
         targets = {}
         function BenillaAlchemy_SetTarget(family, key, grade)
             table.insert(targets, family .. '|' .. key .. '|' .. grade)
         end",
    )
    .unwrap();
    open(&mut s, one_brew());
    choose_alchemy(&mut s);
    sub(&mut s, "Potions");

    let make = "BenillaFishingJournalAlchemyRecipe1Grade2Make";
    assert!(shown(&s, make));
    s.run(&format!(
        "this = {make} BenillaAlchemyJournalMake_OnClick()"
    ))
    .unwrap();
    assert_eq!(
        s.eval::<String>("return table.concat(targets, ',')")
            .unwrap(),
        "potion|Restorative|1"
    );

    s.run("BenillaAlchemy:Hide()").unwrap();
    s.fire_event("ALCHEMY_CLOSED", Vec::new());
    frame(&mut s);
    assert!(!shown(&s, make));
}

#[test]
fn a_satchel_right_click_queues_a_withdraw() {
    let mut s = journal();
    open(&mut s, one_brew());
    choose_alchemy(&mut s);

    assert!(shown(&s, "BenillaFishingJournalAlchemySatchel"));
    assert_eq!(text(&s, "BenillaFishingJournalAlchemySlot1Stack"), "3");
    s.run(
        "this = BenillaFishingJournalAlchemySlot1 BenillaFishingJournalSlot_OnClick('RightButton')",
    )
    .unwrap();
    s.run("IsShiftKeyDown = function() return 1 end").unwrap();
    s.run(
        "this = BenillaFishingJournalAlchemySlot1 BenillaFishingJournalSlot_OnClick('RightButton')",
    )
    .unwrap();
    // An empty slot withdraws nothing.
    s.run(
        "this = BenillaFishingJournalAlchemySlot2 BenillaFishingJournalSlot_OnClick('RightButton')",
    )
    .unwrap();
    assert_eq!(
        s.take_alchemy_calls(),
        vec![
            AlchemyCall::Withdraw {
                item: TINCTURE,
                count: 3
            },
            AlchemyCall::Withdraw {
                item: TINCTURE,
                count: 1
            },
        ]
    );
}

#[test]
fn the_brew_log_page_draws_the_rows_summed_values() {
    let mut s = journal();
    open(&mut s, one_brew());
    choose_alchemy(&mut s);
    sub(&mut s, "Brew Log");

    let row = "BenillaFishingJournalAlchemyLog1";
    assert!(shown(&s, row));
    assert!(!shown(&s, "BenillaFishingJournalAlchemyLog2"));
    assert!(!shown(&s, "BenillaFishingJournalAlchemyLogEmpty"));
    assert_eq!(text(&s, &format!("{row}Sums")), "Restorative 4, Toxic 2");
    assert_eq!(text(&s, &format!("{row}Make")), "Potion");
    assert_eq!(
        text(&s, &format!("{row}In")),
        "Peacebloom ×2, Silverleaf ×2"
    );
    assert_eq!(text(&s, &format!("{row}Toxic")), "2 · 33%");
    assert_eq!(text(&s, &format!("{row}Out")), "Minor Healing Potion");
    assert_eq!(
        text(&s, &format!("{row}Note")),
        "Learned Minor Healing Potion"
    );

    // The Flasks page lists the kind, dimmed and unmade.
    sub(&mut s, "Flasks");
    assert_eq!(
        text(&s, "BenillaFishingJournalAlchemyFlask1Name"),
        "Flask of the Bulwark"
    );
    assert_eq!(text(&s, "BenillaFishingJournalAlchemyFlask1Made"), "—");
}

#[test]
fn an_empty_brew_log_says_so() {
    let mut s = journal();
    let mut view = one_brew();
    view.log.clear();
    open(&mut s, view);
    choose_alchemy(&mut s);
    sub(&mut s, "Brew Log");
    assert!(shown(&s, "BenillaFishingJournalAlchemyLogEmpty"));
    assert_eq!(
        text(&s, "BenillaFishingJournalAlchemyLogEmpty"),
        "Nothing brewed yet."
    );
    assert!(!shown(&s, "BenillaFishingJournalAlchemyLog1"));
}
