//! The Alchemy window (`assets/ui/Alchemy.xml`) over pushed tables: the make picker's open and
//! dark makes, a tray click that lists a herb and asks for a preview, the ready preview that
//! enables Brew and names the product, the target, the made result on the vials, Buy rumour and the
//! close. It needs no game data, so a missing font only warns and the dropdown kit is absent.

use benilla_ui::script::{
    AlchemyBenchView, AlchemyCall, AlchemyGradeView, AlchemyMadeView, AlchemyMakeView,
    AlchemyMaterialView, AlchemyPreviewView, AlchemyRumourView, AlchemyToxicView, AlchemyVialView,
    ItemTemplateView, UiScript,
};

use super::test_ui::load_ui as load_xml;

const PEACEBLOOM: u32 = 2447;
const MINOR_HEALING: u32 = 963100;

fn window() -> UiScript {
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1024.0, 768.0);
    load_xml(&s, "Alchemy.xml");
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    s.set_item_template(
        PEACEBLOOM,
        ItemTemplateView {
            name: "Peacebloom".into(),
            quality: 1,
            ..Default::default()
        },
    );
    s.set_item_template(
        MINOR_HEALING,
        ItemTemplateView {
            name: "Minor Healing Potion".into(),
            quality: 1,
            description: "Use: Heals 70 to 90 at once. Strength x1.".into(),
            ..Default::default()
        },
    );
    s
}

fn shown(s: &UiScript, frame: &str) -> bool {
    s.eval::<bool>(&format!("return {frame}:IsVisible() and true or false"))
        .unwrap()
}

fn enabled(s: &UiScript, frame: &str) -> bool {
    s.eval::<i64>(&format!("return {frame}:IsEnabled()"))
        .unwrap()
        == 1
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

fn make(key: &str, name: &str, station: &str, verb: &str, open: bool) -> AlchemyMakeView {
    AlchemyMakeView {
        key: key.into(),
        name: name.into(),
        station: station.into(),
        verb: verb.into(),
        rule: String::new(),
        open,
        ..Default::default()
    }
}

/// Goldshire's table with only the Potion open, seven Peacebloom in the tray, the Minor Healing
/// Potion learned and one Rumour for sale.
fn bench() -> AlchemyBenchView {
    let mut potion = make("potion", "Potion", "Mortar", "Brew", true);
    potion.rule = "2 to 4 herbs".into();
    potion.herbs = [2, 4];
    AlchemyBenchView {
        table: "Goldshire".into(),
        alchemist: "Alwen Carrow".into(),
        line: "Dirty herbs are a gamble.".into(),
        tiers: vec!["Minor".into(), "Lesser".into()],
        makes: vec![
            potion,
            make("tincture", "Tincture", "Still", "Distill", false),
            make("essence", "Essence", "Still", "Condense", false),
            make("oil", "Weapon Oil", "Oil Tin", "Mix", false),
            make("flask", "Flask", "Flask Rack", "Join", false),
        ],
        tray: vec![AlchemyMaterialView {
            item: PEACEBLOOM,
            count: 7,
            kind: 0,
            level: 1,
            values: vec![("Restorative".into(), Some(2)), ("Toxic".into(), None)],
            ..Default::default()
        }],
        targets: vec![AlchemyGradeView {
            family: "potion".into(),
            key: "Restorative".into(),
            grade: 0,
            name: "Minor Healing Potion".into(),
            learned: true,
            rumoured: false,
            threshold: 4,
        }],
        rumours: vec![AlchemyRumourView {
            family: "potion".into(),
            key: "Restorative".into(),
            grade: 1,
            name: "Lesser Healing Potion".into(),
            price: 6000,
        }],
        sludge: 3,
    }
}

/// Push `view` the way the host does: set the model, then fire the update event.
fn push(s: &mut UiScript, view: AlchemyBenchView) {
    s.set_alchemy(Some(view));
    s.fire_event("ALCHEMY_UPDATE", Vec::new());
    s.resolve();
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

fn open() -> UiScript {
    let mut s = window();
    push(&mut s, bench());
    s
}

fn vials(main: bool) -> Vec<AlchemyVialView> {
    ["Restorative", "Fortifying", "Volatile", "Arcane", "Calming"]
        .iter()
        .enumerate()
        .map(|(i, p)| AlchemyVialView {
            prop: (*p).into(),
            value: if i == 0 { 4.0 } else { 0.0 },
            main: main && i == 0,
            ..Default::default()
        })
        .collect()
}

/// A ready preview of the Minor Healing Potion, as the realm answers four Peacebloom.
fn ready_preview() -> AlchemyPreviewView {
    AlchemyPreviewView {
        ready: true,
        note: String::new(),
        vials: vials(true),
        toxic: AlchemyToxicView::default(),
        unnoted: 0,
        scale: 24.0,
        rings: [4, 12, 24, 40],
        hybrid: false,
        known: true,
        product: MINOR_HEALING,
        sludge: false,
        carried: 0,
    }
}

fn push_preview(s: &mut UiScript, view: AlchemyPreviewView) {
    s.set_alchemy_preview(Some(view));
    s.fire_event("ALCHEMY_PREVIEW", Vec::new());
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

#[test]
fn a_bench_with_only_the_potion_open_draws_five_makes_and_the_tincture_dark() {
    let s = open();
    assert!(shown(&s, "BenillaAlchemy"));
    assert_eq!(plain(&s, "BenillaAlchemyTitle"), "Goldshire");
    assert_eq!(plain(&s, "BenillaAlchemySub"), "Alwen Carrow's table");
    for i in 1..=5 {
        assert!(shown(&s, &format!("BenillaAlchemyMake{i}")));
    }
    assert_eq!(plain(&s, "BenillaAlchemyMake1Name"), "Potion");
    assert_eq!(plain(&s, "BenillaAlchemyMake1Station"), "Mortar");
    assert!(enabled(&s, "BenillaAlchemyMake1"));
    assert_eq!(plain(&s, "BenillaAlchemyMake2Name"), "Tincture");
    assert_eq!(plain(&s, "BenillaAlchemyMake2Station"), "Dark");
    assert!(!enabled(&s, "BenillaAlchemyMake2"));
    // The Potion is picked first, and its workspace names its station and rule.
    assert_eq!(plain(&s, "BenillaAlchemyStation"), "Mortar");
    assert_eq!(plain(&s, "BenillaAlchemyRule"), "Potion · 2 to 4 herbs");
    assert_eq!(plain(&s, "BenillaAlchemyVial6Name"), "Toxic");
    assert_eq!(plain(&s, "BenillaAlchemyTrayGroup1"), "Herbs");
    assert_eq!(plain(&s, "BenillaAlchemyTray1Name"), "Peacebloom");
    assert_eq!(plain(&s, "BenillaAlchemyTray1Chips"), "Rest 2  Tox ?");
    assert_eq!(plain(&s, "BenillaAlchemyTray1Fill"), "+4");
    assert_eq!(plain(&s, "BenillaAlchemySell"), "Sell Murky Sludge (3)");
    assert_eq!(
        plain(&s, "BenillaAlchemyRumourHint"),
        "A Rumour gives one grade's exact threshold. Alwen Carrow knows the Minor, Lesser grades."
    );
}

#[test]
fn clicking_a_peacebloom_lists_it_and_asks_for_a_preview() {
    let mut s = open();
    s.take_alchemy_calls();
    click(&s, "BenillaAlchemyTray1");
    assert_eq!(plain(&s, "BenillaAlchemyTray1In"), "1 in");
    assert_eq!(plain(&s, "BenillaAlchemyTray1Count"), "× 6");
    assert!(shown(&s, "BenillaAlchemyTray1Less"));
    assert_eq!(
        s.take_alchemy_calls(),
        [AlchemyCall::Preview {
            make: "potion".into(),
            items: vec![PEACEBLOOM],
            target: String::new(),
        }]
    );
    click(&s, "BenillaAlchemyTray1Less");
    assert!(!shown(&s, "BenillaAlchemyTray1In"));
    assert_eq!(
        s.take_alchemy_calls(),
        [AlchemyCall::Preview {
            make: "potion".into(),
            items: Vec::new(),
            target: String::new(),
        }]
    );
}

#[test]
fn a_ready_preview_enables_brew_and_names_the_product() {
    let mut s = open();
    assert_eq!(plain(&s, "BenillaAlchemyGo"), "Brew");
    assert!(!enabled(&s, "BenillaAlchemyGo"));
    push_preview(
        &mut s,
        AlchemyPreviewView {
            ready: false,
            note: "Two to four herbs.".into(),
            vials: vials(false),
            scale: 24.0,
            rings: [4, 12, 24, 40],
            ..Default::default()
        },
    );
    assert!(!enabled(&s, "BenillaAlchemyGo"));
    assert_eq!(plain(&s, "BenillaAlchemyVial1Note"), "Two to four herbs.");
    assert_eq!(plain(&s, "BenillaAlchemyVial6Note"), "no roll");
    push_preview(&mut s, ready_preview());
    assert!(enabled(&s, "BenillaAlchemyGo"));
    assert_eq!(
        plain(&s, "BenillaAlchemyVial1Note"),
        "Minor Healing Potion x1"
    );
    assert_eq!(plain(&s, "BenillaAlchemyVial1Value"), "4");
    assert_eq!(plain(&s, "BenillaAlchemyVial1Ring1Label"), "4");
    // A Toxic that can take the brew says its chance.
    let mut toxic = ready_preview();
    toxic.toxic = AlchemyToxicView {
        raw: 2,
        net: 2,
        fail_pct: 33,
        ..Default::default()
    };
    toxic.unnoted = 2;
    push_preview(&mut s, toxic);
    assert_eq!(plain(&s, "BenillaAlchemyVial6Note"), "33% fail");
    assert_eq!(
        plain(&s, "BenillaAlchemyFooter"),
        "2 unnoted herbs not shown"
    );
}

#[test]
fn choosing_a_target_queues_a_preview_with_it() {
    let mut s = open();
    s.take_alchemy_calls();
    s.run("BenillaAlchemy_SetTarget('potion', 'Restorative', 0)")
        .unwrap();
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    assert_eq!(
        s.take_alchemy_calls(),
        [AlchemyCall::Preview {
            make: "potion".into(),
            items: Vec::new(),
            target: "potion|Restorative|Minor".into(),
        }]
    );
    s.run("BenillaAlchemy_SetTarget(nil)").unwrap();
    assert!(matches!(
        &s.take_alchemy_calls()[..],
        [AlchemyCall::Preview { target, .. }] if target.is_empty()
    ));
}

#[test]
fn a_made_view_shows_the_result_until_the_list_changes() {
    let mut s = open();
    click(&s, "BenillaAlchemyTray1Fill");
    push_preview(&mut s, ready_preview());
    s.take_alchemy_calls();
    click(&s, "BenillaAlchemyGo");
    assert_eq!(
        s.take_alchemy_calls(),
        [AlchemyCall::Make {
            make: "potion".into(),
            items: vec![PEACEBLOOM; 4],
            target: String::new(),
        }]
    );
    s.set_alchemy_made(Some(AlchemyMadeView {
        product: MINOR_HEALING,
        failed: false,
        fail_pct: 0,
        vials: vials(true),
        toxic: AlchemyToxicView::default(),
        scale: 24.0,
        rings: [4, 12, 24, 40],
        lines: vec![
            (3, "Minor Healing Potion x1, into your bags.".into()),
            (1, "New grade learned: Minor Healing Potion.".into()),
        ],
    }));
    s.fire_event("ALCHEMY_MADE", Vec::new());
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    assert_eq!(
        plain(&s, "BenillaAlchemyFooter"),
        "Result · Minor Healing Potion x1"
    );
    assert_eq!(
        plain(&s, "BenillaAlchemyLog1"),
        "Minor Healing Potion x1, into your bags."
    );
    assert!(shown(&s, "BenillaAlchemyAgain"));
    // The realm takes the four herbs; the next bench keeps the result.
    let mut after = bench();
    after.tray[0].count = 3;
    push(&mut s, after);
    assert_eq!(
        plain(&s, "BenillaAlchemyFooter"),
        "Result · Minor Healing Potion x1"
    );
    // Same again is short of Peacebloom and says so.
    click(&s, "BenillaAlchemyAgain");
    assert_eq!(
        plain(&s, "BenillaAlchemyLog3"),
        "Not enough Peacebloom for that again (3 of 4)."
    );
    // A herb in the list clears the result.
    click(&s, "BenillaAlchemyTray1");
    assert_eq!(plain(&s, "BenillaAlchemyFooter"), "");
}

#[test]
fn buy_rumour_queues_the_rumour() {
    let mut s = open();
    s.take_alchemy_calls();
    click(&s, "BenillaAlchemyBuy");
    assert_eq!(
        s.take_alchemy_calls(),
        [AlchemyCall::BuyRumour {
            family: "potion".into(),
            key: "Restorative".into(),
            grade: 1,
        }]
    );
    let mut none = bench();
    none.rumours.clear();
    none.sludge = 0;
    push(&mut s, none);
    assert!(!shown(&s, "BenillaAlchemyBuy"));
    assert!(!shown(&s, "BenillaAlchemySell"));
    assert_eq!(
        plain(&s, "BenillaAlchemyNoRumours"),
        "Nothing left to tell at this table."
    );
}

#[test]
fn alchemy_closed_hides_the_window_and_the_close_button_closes_the_table() {
    let mut s = open();
    s.set_alchemy(None);
    s.fire_event("ALCHEMY_CLOSED", Vec::new());
    assert!(!shown(&s, "BenillaAlchemy"));
    assert!(!s.take_alchemy_calls().contains(&AlchemyCall::Close));
    push(&mut s, bench());
    assert!(shown(&s, "BenillaAlchemy"));
    s.take_alchemy_calls();
    click(&s, "BenillaAlchemyClose");
    assert!(!shown(&s, "BenillaAlchemy"));
    assert_eq!(s.take_alchemy_calls(), [AlchemyCall::Close]);
}
