//! The Kitchen window (`assets/ui/Kitchen.xml`) over pushed kitchens: the recipe book's known and
//! "???" rows, the pot a row click fills, the prediction that enables Cook, a pantry click in
//! Experiment, the result panel, the close, and the Orders tab's Deliver. It needs no game data, so
//! a missing font only warns.

use benilla_ui::script::{
    ItemTemplateView, KitchenCall, KitchenCookedView, KitchenIngredientView, KitchenMode,
    KitchenOrderView, KitchenPredictionView, KitchenRecipeView, KitchenView, UiScript,
};

use super::test_ui::load_ui as load_xml;

const PERCH: u32 = 960101;
const ONION: u32 = 960201;
const STEW: u32 = 961001;
const BAIT: u32 = 960502;

fn window() -> UiScript {
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1024.0, 768.0);
    load_xml(&s, "Kitchen.xml");
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    for (item, name) in [
        (PERCH, "Goldshire Perch"),
        (ONION, "Wild Onion"),
        (STEW, "Hunter's Fish Stew"),
        (BAIT, "Cut Perch"),
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

fn shown(s: &UiScript, frame: &str) -> bool {
    s.eval::<bool>(&format!("return {frame}:IsVisible() and true or false"))
        .unwrap()
}

/// `GetText()` with its `|cffrrggbb` and `|r` colour codes taken out.
fn plain(s: &UiScript, frame: &str) -> String {
    s.eval::<String>(&format!(
        "local t = string.gsub({frame}:GetText() or '', '|c%x%x%x%x%x%x%x%x', '') \
         t = string.gsub(t, '|r', '') return t"
    ))
    .unwrap()
}

fn click(s: &UiScript, frame: &str) {
    s.run(&format!("{frame}:Click()")).unwrap();
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// A kitchen whose book is open, with one known and one unknown recipe, three perch and an onion
/// in the pantry, and one fish order the player can fill.
fn kitchen() -> KitchenView {
    KitchenView {
        name: "Lion's Pride kitchen".into(),
        cook: "Tomas".into(),
        book: vec![
            KitchenRecipeView {
                key: "stamina_1".into(),
                kind: 0,
                effect: "+Stamina".into(),
                family: "Combat Well Fed".into(),
                band: 1,
                known: true,
                name: "Hunter's Fish Stew".into(),
                meal: STEW,
                slots: vec![PERCH, ONION],
            },
            KitchenRecipeView {
                key: "stamina_10".into(),
                kind: 0,
                effect: "+Stamina".into(),
                family: "Combat Well Fed".into(),
                band: 10,
                known: false,
                name: String::new(),
                meal: 0,
                slots: vec![PERCH, 0, 0],
            },
        ],
        pantry: vec![
            KitchenIngredientView {
                item: PERCH,
                count: 3,
                level: 2,
                category: "fish".into(),
                untried: false,
            },
            KitchenIngredientView {
                item: ONION,
                count: 1,
                level: 1,
                category: "roots".into(),
                untried: true,
            },
        ],
        lessons: [true, false, false, false, false, false],
        orders: vec![KitchenOrderView {
            label: "3 Goldshire Perch".into(),
            kind: 0,
            copper: 84,
            qty: 3,
            have: 3,
            done: false,
            can_deliver: true,
            bonus: "a recipe Rumour".into(),
            bonus_rare: false,
        }],
        cuttable: vec![(PERCH, 5, BAIT)],
    }
}

/// Push `view` the way the host does: set the model, then fire the update event.
fn push(s: &mut UiScript, view: KitchenView) {
    s.set_kitchen(Some(view));
    s.fire_event("KITCHEN_UPDATE", Vec::new());
    s.resolve();
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

fn open() -> UiScript {
    let mut s = window();
    push(&mut s, kitchen());
    s
}

#[test]
fn the_book_draws_a_known_name_and_an_unknown_recipes_question_marks() {
    let mut s = open();
    assert!(shown(&s, "BenillaKitchen"));
    assert_eq!(plain(&s, "BenillaKitchenTitle"), "Lion's Pride kitchen");
    assert_eq!(plain(&s, "BenillaKitchenCookName"), "cook: Tomas");
    assert_eq!(plain(&s, "BenillaKitchenBookCount"), "1 of 2 known");
    assert_eq!(plain(&s, "BenillaKitchenHead1Effect"), "+Stamina");
    assert_eq!(plain(&s, "BenillaKitchenHead1Family"), "Combat Well Fed");
    assert_eq!(plain(&s, "BenillaKitchenRow1Name"), "Hunter's Fish Stew");
    assert_eq!(plain(&s, "BenillaKitchenRow1Band"), "Band 1");
    assert_eq!(plain(&s, "BenillaKitchenRow1Chip2"), "Wild Onion");
    assert!(!shown(&s, "BenillaKitchenRow1Chip3"));
    assert_eq!(plain(&s, "BenillaKitchenRow2Name"), "???");
    assert_eq!(plain(&s, "BenillaKitchenRow2Chip1"), "Goldshire Perch");
    assert_eq!(plain(&s, "BenillaKitchenRow2Chip2"), "???");
    assert_eq!(plain(&s, "BenillaKitchenRow2Chip3"), "???");
    // The first lesson is done, so the strip shows the second.
    assert_eq!(
        plain(&s, "BenillaKitchenLessonTitle"),
        "Cook from your recipe book"
    );
    assert_eq!(
        plain(&s, "BenillaKitchenLessonCount"),
        "Tomas's lessons · 1 of 6"
    );
    // Before the first cook the book says it is empty.
    let mut fresh = kitchen();
    fresh.lessons = [false; 6];
    push(&mut s, fresh);
    assert!(shown(&s, "BenillaKitchenBookEmpty"));
    assert!(plain(&s, "BenillaKitchenBookEmpty").starts_with("Empty. Cook anything once"));
}

#[test]
fn clicking_a_known_row_fills_its_slots_and_asks_for_a_prediction() {
    let mut s = open();
    s.take_kitchen_calls();
    click(&s, "BenillaKitchenRow1");
    assert_eq!(plain(&s, "BenillaKitchenPotTitle"), "Hunter's Fish Stew");
    assert_eq!(plain(&s, "BenillaKitchenSlot1Name"), "Goldshire Perch");
    assert_eq!(plain(&s, "BenillaKitchenSlot2Name"), "Wild Onion");
    assert!(!shown(&s, "BenillaKitchenSlot3"));
    assert_eq!(plain(&s, "BenillaKitchenRefill"), "Refill");
    assert_eq!(
        s.take_kitchen_calls(),
        [KitchenCall::Predict {
            mode: KitchenMode::Recipe,
            recipe: "stamina_1".into(),
            slots: vec![PERCH, ONION],
        }]
    );
    // An unknown row fills its revealed slot and leaves the guesses empty.
    click(&s, "BenillaKitchenRow2");
    assert_eq!(
        plain(&s, "BenillaKitchenPotTitle"),
        "Unknown recipe · Band 10"
    );
    assert_eq!(
        plain(&s, "BenillaKitchenSlot2Empty"),
        "Empty. Pick from the pantry."
    );
    assert_eq!(
        s.take_kitchen_calls(),
        [KitchenCall::Predict {
            mode: KitchenMode::Discover,
            recipe: "stamina_10".into(),
            slots: vec![PERCH, 0, 0],
        }]
    );
}

#[test]
fn a_ready_prediction_enables_cook() {
    let mut s = open();
    let enabled = |s: &UiScript| {
        s.eval::<i64>("return BenillaKitchenCook:IsEnabled()")
            .unwrap()
    };
    assert_eq!(enabled(&s), 0);
    s.set_kitchen_prediction(Some(KitchenPredictionView {
        ready: false,
        meal: 0,
        note: "Fill every slot first.".into(),
        why: Vec::new(),
    }));
    s.fire_event("KITCHEN_PREDICTION", Vec::new());
    assert_eq!(enabled(&s), 0);
    assert_eq!(
        plain(&s, "BenillaKitchenPredictUse"),
        "Fill every slot first."
    );
    s.set_kitchen_prediction(Some(KitchenPredictionView {
        ready: true,
        meal: STEW,
        note: String::new(),
        why: vec!["An exact match.".into()],
    }));
    s.fire_event("KITCHEN_PREDICTION", Vec::new());
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    assert_eq!(enabled(&s), 1);
    assert_eq!(plain(&s, "BenillaKitchenPredictMeal"), "Hunter's Fish Stew");
    assert_eq!(plain(&s, "BenillaKitchenPredictWhy"), "- An exact match.");
    s.take_kitchen_calls();
    click(&s, "BenillaKitchenCook");
    assert!(matches!(
        s.take_kitchen_calls()[..],
        [KitchenCall::Cook {
            mode: KitchenMode::Experiment,
            ..
        }]
    ));
}

#[test]
fn a_pantry_click_in_experiment_fills_the_first_slot() {
    let mut s = open();
    click(&s, "BenillaKitchenExperiment");
    assert_eq!(plain(&s, "BenillaKitchenPotTitle"), "Experiment");
    assert!(shown(&s, "BenillaKitchenSize4"));
    s.take_kitchen_calls();
    assert_eq!(plain(&s, "BenillaKitchenPantryGroup1"), "Fish");
    assert_eq!(plain(&s, "BenillaKitchenPantryGroup2"), "Roots");
    assert_eq!(plain(&s, "BenillaKitchenPantryItem1Meta"), "L2 · 3 left");
    assert!(shown(&s, "BenillaKitchenPantryItem2New"));
    click(&s, "BenillaKitchenPantryItem1");
    assert_eq!(plain(&s, "BenillaKitchenSlot1Name"), "Goldshire Perch");
    assert_eq!(plain(&s, "BenillaKitchenPantryItem1Meta"), "L2 · 2 left");
    assert_eq!(
        s.take_kitchen_calls(),
        [KitchenCall::Predict {
            mode: KitchenMode::Experiment,
            recipe: String::new(),
            slots: vec![PERCH, 0, 0],
        }]
    );
    // The one onion goes in, and then the onion button has nothing left to give.
    click(&s, "BenillaKitchenPantryItem2");
    assert_eq!(plain(&s, "BenillaKitchenSlot2Name"), "Wild Onion");
    assert_eq!(
        s.eval::<i64>("return BenillaKitchenPantryItem2:IsEnabled()")
            .unwrap(),
        0
    );
}

#[test]
fn a_cooked_pot_shows_the_result_panel() {
    let mut s = open();
    s.set_kitchen_cooked(Some(KitchenCookedView {
        meal: STEW,
        mush: false,
        close: false,
        learned: vec!["stamina_10".into(), "stamina_1".into()],
        revealed: vec![(ONION, 3)],
        line: "First time in my kitchen? Here, these are mine.".into(),
    }));
    s.fire_event("KITCHEN_COOKED", Vec::new());
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    assert!(shown(&s, "BenillaKitchenResult"));
    assert_eq!(plain(&s, "BenillaKitchenResultKick"), "You cooked");
    assert_eq!(plain(&s, "BenillaKitchenResultMeal"), "Hunter's Fish Stew");
    assert_eq!(
        plain(&s, "BenillaKitchenResultNews1"),
        "Tomas taught you 1 recipe. Your recipe book is open."
    );
    assert_eq!(
        plain(&s, "BenillaKitchenResultNews2"),
        "New recipe: Hunter's Fish Stew!"
    );
    assert_eq!(
        plain(&s, "BenillaKitchenResultNews3"),
        "Wild Onion revealed in 3 unknown recipes"
    );
    assert_eq!(
        plain(&s, "BenillaKitchenLog1"),
        "First time in my kitchen? Here, these are mine."
    );
    click(&s, "BenillaKitchenResultKeep");
    assert!(!shown(&s, "BenillaKitchenResult"));
}

#[test]
fn kitchen_closed_hides_the_window_and_the_close_button_closes_the_kitchen() {
    let mut s = open();
    s.set_kitchen(None);
    s.fire_event("KITCHEN_CLOSED", Vec::new());
    assert!(!shown(&s, "BenillaKitchen"));
    assert!(!s.take_kitchen_calls().contains(&KitchenCall::Close));
    push(&mut s, kitchen());
    assert!(shown(&s, "BenillaKitchen"));
    s.take_kitchen_calls();
    click(&s, "BenillaKitchenClose");
    assert!(!shown(&s, "BenillaKitchen"));
    assert_eq!(s.take_kitchen_calls(), [KitchenCall::Close]);
}

#[test]
fn the_orders_tab_draws_an_order_and_deliver_queues_it() {
    let s = open();
    assert_eq!(plain(&s, "BenillaKitchenTab2"), "Orders & bait (1)");
    click(&s, "BenillaKitchenTab2");
    assert!(shown(&s, "BenillaKitchenOrdersPage"));
    assert!(!shown(&s, "BenillaKitchenCookPage"));
    assert_eq!(plain(&s, "BenillaKitchenOrdersTitle"), "Orders from Tomas");
    assert_eq!(plain(&s, "BenillaKitchenOrder1Kind"), "Raw fish");
    assert_eq!(plain(&s, "BenillaKitchenOrder1Label"), "3 Goldshire Perch");
    assert_eq!(
        plain(&s, "BenillaKitchenOrder1Pays"),
        "Pays 84c and a recipe Rumour"
    );
    assert_eq!(plain(&s, "BenillaKitchenOrder1Progress"), "You have 3 / 3");
    assert!(!shown(&s, "BenillaKitchenOrder2"));
    assert_eq!(
        plain(&s, "BenillaKitchenCut1Text"),
        "Goldshire Perch (5) → 3 Cut Perch"
    );
    let mut s = s;
    s.take_kitchen_calls();
    click(&s, "BenillaKitchenOrder1Deliver");
    click(&s, "BenillaKitchenCutButton");
    assert_eq!(
        s.take_kitchen_calls(),
        [KitchenCall::Deliver(0), KitchenCall::Cut(PERCH)]
    );
}
