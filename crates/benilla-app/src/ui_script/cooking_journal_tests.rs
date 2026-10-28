//! The Journal's Cooking bookmark (`assets/ui/FishingJournal.xml`, #496) over pushed views: the
//! bookmark greys without pages, its Dish Pages replace the bottom tabs, and seven pages need two
//! screens. It needs no game data, so a missing font only warns.

use benilla_ui::script::{CookingJournalView, DishPageView, ItemTemplateView, UiScript};

use super::test_ui::load_ui as load_xml;

const CHOWDER: u32 = 962025;
const PERCH: u32 = 960401;
const DANDELION: u32 = 961001;
const ONION: u32 = 961002;

fn journal() -> UiScript {
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1024.0, 768.0);
    load_xml(&s, "FishingJournal.xml");
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    for (item, name) in [
        (CHOWDER, "Lion's Pride Chowder"),
        (PERCH, "Goldshire Perch"),
        (DANDELION, "Dandelion Greens"),
        (ONION, "Wild Onion"),
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
    s.eval::<bool>("return BenillaFishingJournalProf3.live and true or false")
        .unwrap()
}

/// The chowder's page, cooked once at Lion's Pride.
fn chowder() -> DishPageView {
    DishPageView {
        recipe: "regen_1".into(),
        meal: CHOWDER,
        kind: 0,
        effect: "Faster regen".into(),
        family: "Out-of-combat Well Fed".into(),
        band: 1,
        ingredients: vec![PERCH, DANDELION, ONION],
        cooked: 1,
        first: "Day 2, 08:00, Lion's Pride kitchen".into(),
    }
}

fn choose_cooking(s: &mut UiScript) {
    s.run("this = BenillaFishingJournalProf3 BenillaJournalProfession_OnClick()")
        .unwrap();
    frame(s);
}

#[test]
fn the_cooking_bookmark_is_grey_without_pages_and_live_with_one() {
    let mut s = journal();
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s);
    assert!(shown(&s, "BenillaFishingJournalProf3"));
    assert!(!live(&s));
    choose_cooking(&mut s);
    assert!(
        s.eval::<bool>("return BenillaFishingJournalProf1.selected")
            .unwrap(),
        "a click on a grey Cooking bookmark changes nothing"
    );

    s.set_cooking_journal(CookingJournalView {
        pages: vec![chowder()],
    });
    s.fire_event("COOKING_JOURNAL_UPDATE", Vec::new());
    frame(&mut s);
    assert!(live(&s));
}

#[test]
fn choosing_cooking_hides_the_tabs_and_draws_the_dish_page() {
    let mut s = journal();
    s.set_cooking_journal(CookingJournalView {
        pages: vec![chowder()],
    });
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s);
    choose_cooking(&mut s);

    assert!(s
        .eval::<bool>("return BenillaFishingJournalProf3.selected")
        .unwrap());
    assert_eq!(text(&s, "BenillaFishingJournalTitle"), "Cooking Journal");
    assert!(shown(&s, "BenillaFishingJournalCookPages"));
    for frame in [
        "BenillaFishingJournalTab1",
        "BenillaFishingJournalTab2",
        "BenillaFishingJournalTab3",
        "BenillaFishingJournalZones",
        "BenillaFishingJournalHerbZones",
    ] {
        assert!(!shown(&s, frame), "{frame} hides");
    }
    assert!(shown(&s, "BenillaFishingJournalDish1"));
    assert!(!shown(&s, "BenillaFishingJournalDish2"));
    assert_eq!(
        text(&s, "BenillaFishingJournalDish1Kind"),
        "Out-of-combat Well Fed · Band 1"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalDish1Name"),
        "Lion's Pride Chowder"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalDish1Needs"),
        "Faster regen · needs Goldshire Perch, Dandelion Greens, Wild Onion"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalDish1Cooked"),
        "Times cooked 1"
    );
    assert_eq!(
        text(&s, "BenillaFishingJournalDish1First"),
        "First cook Day 2, 08:00, Lion's Pride kitchen"
    );

    // The Fishing bookmark brings the tabs back.
    s.run("this = BenillaFishingJournalProf1 BenillaJournalProfession_OnClick()")
        .unwrap();
    frame(&mut s);
    assert!(shown(&s, "BenillaFishingJournalTab1"));
    assert!(shown(&s, "BenillaFishingJournalZones"));
    assert!(!shown(&s, "BenillaFishingJournalCookPages"));
}

#[test]
fn seven_dish_pages_page_to_a_second_screen() {
    let mut s = journal();
    let pages = (0..7)
        .map(|n| DishPageView {
            recipe: format!("dish_{n}"),
            cooked: 0,
            first: String::new(),
            ..chowder()
        })
        .collect();
    s.set_cooking_journal(CookingJournalView { pages });
    s.run("BenillaFishingJournal_Toggle()").unwrap();
    frame(&mut s);
    choose_cooking(&mut s);

    for n in 1..=6 {
        assert!(shown(&s, &format!("BenillaFishingJournalDish{n}")));
    }
    assert_eq!(text(&s, "BenillaFishingJournalCookScreen"), "1 / 2");
    assert_eq!(
        text(&s, "BenillaFishingJournalDish1First"),
        "Not cooked yet"
    );
    assert!(shown(&s, "BenillaFishingJournalCookNext"));

    s.run("BenillaCookJournal_Turn(1)").unwrap();
    frame(&mut s);
    assert_eq!(text(&s, "BenillaFishingJournalCookScreen"), "2 / 2");
    assert!(shown(&s, "BenillaFishingJournalDish1"));
    assert!(!shown(&s, "BenillaFishingJournalDish2"));
    assert_eq!(text(&s, "BenillaFishingJournalCookCount"), "7 known");
}
