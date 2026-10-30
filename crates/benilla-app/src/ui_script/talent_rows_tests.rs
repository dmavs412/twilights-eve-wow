//! The Talents panel (`assets/ui/Talents.xml`, ADR 0028) over pushed rows: the level labels, the
//! cell states and their clicks, the footer, the scroll child past ten rows, the empty Calling and
//! when the window opens. It needs no game data, so a missing font only warns.

use benilla_ui::script::{
    TalentChoiceView, TalentRowView, TalentRowsCall, TalentRowsView, UiScript,
};

use super::test_ui::load_ui as load_xml;

fn window() -> UiScript {
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1024.0, 768.0);
    load_xml(&s, "Talents.xml");
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    s
}

fn shown(s: &UiScript, frame: &str) -> bool {
    s.eval::<bool>(&format!("return {frame}:IsVisible() and true or false"))
        .unwrap()
}

fn plain(s: &UiScript, frame: &str) -> String {
    s.eval::<String>(&format!("return {frame}:GetText() or ''"))
        .unwrap()
}

fn click(s: &UiScript, frame: &str) {
    s.run(&format!("{frame}:Click()")).unwrap();
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// Row `key` at `level`, its three choices `spell + 1` to `spell + 3`, set to `set` (0 for none).
fn row(key: &str, level: u8, open: bool, skill: &str, spell: u32, set: u32) -> TalentRowView {
    TalentRowView {
        key: key.into(),
        level,
        open,
        skill: skill.into(),
        set,
        choices: (1..=3)
            .map(|n| TalentChoiceView {
                spell_id: spell + n,
                name: format!("{key} {n}"),
                icon: None,
            })
            .collect(),
    }
}

/// Basic Warrior at level 5: Charge open and empty, Fire Blast open and set to its second choice,
/// Gouge and Heroic Strike locked.
fn warrior(at_innkeeper: bool) -> TalentRowsView {
    TalentRowsView {
        calling: "Basic Warrior".into(),
        at_innkeeper,
        rows: vec![
            row("charge", 2, true, "Charge", 100, 0),
            row("fire_blast", 4, true, "Fire Blast", 200, 202),
            row("gouge", 6, false, "Gouge", 300, 0),
            row("heroic_strike", 8, false, "Heroic Strike", 400, 0),
        ],
    }
}

/// Push `view` the way the host does: set the model, then fire the update event.
fn push(s: &mut UiScript, view: TalentRowsView) {
    s.set_talent_rows(view);
    s.fire_event("TALENT_ROWS_UPDATE", Vec::new());
    s.resolve();
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
}

/// The window open over `view`, with no call queued.
fn open(view: TalentRowsView) -> UiScript {
    let mut s = window();
    push(&mut s, view);
    s.run("BenillaTalents:Show()").unwrap();
    s.take_talent_rows_calls();
    s
}

#[test]
fn four_rows_render_their_levels() {
    let s = open(warrior(false));
    assert_eq!(plain(&s, "BenillaTalentsTitle"), "Talents");
    assert_eq!(plain(&s, "BenillaTalentsCalling"), "Basic Warrior");
    for (i, level) in [(1, "2"), (2, "4"), (3, "6"), (4, "8")] {
        assert_eq!(plain(&s, &format!("BenillaTalentsRow{i}Level")), level);
    }
    assert_eq!(plain(&s, "BenillaTalentsRow1Skill"), "Charge");
    assert_eq!(plain(&s, "BenillaTalentsRow1Cell2Name"), "charge 2");
    // An open row's level is gold, a locked one's grey.
    let colour = |frame: &str| {
        s.eval::<(f32, f32, f32)>(&format!(
            "local r, g, b = {frame}:GetTextColor() return r, g, b"
        ))
        .unwrap()
    };
    assert_eq!(colour("BenillaTalentsRow1Level"), (1.0, 0.82, 0.0));
    assert_eq!(colour("BenillaTalentsRow3Level"), (0.5, 0.5, 0.5));
    assert!(
        s.eval::<bool>("return BenillaTalentsRow5 == nil").unwrap(),
        "a strip per row"
    );
}

#[test]
fn a_locked_cell_sends_nothing() {
    let mut s = open(warrior(false));
    click(&s, "BenillaTalentsRow3Cell1");
    assert_eq!(s.take_talent_rows_calls(), []);
    // At an innkeeper a locked row stays locked.
    push(&mut s, warrior(true));
    s.take_talent_rows_calls();
    click(&s, "BenillaTalentsRow4Cell3");
    assert_eq!(s.take_talent_rows_calls(), []);
}

#[test]
fn clicking_an_open_empty_cell_sets_its_row() {
    let mut s = open(warrior(false));
    click(&s, "BenillaTalentsRow1Cell2");
    assert_eq!(
        s.take_talent_rows_calls(),
        [TalentRowsCall::Set {
            row: "charge".into(),
            spell_id: 102,
        }]
    );
}

#[test]
fn a_set_rows_other_cells_send_nothing_away_from_an_innkeeper() {
    let mut s = open(warrior(false));
    click(&s, "BenillaTalentsRow2Cell1");
    click(&s, "BenillaTalentsRow2Cell2");
    assert_eq!(s.take_talent_rows_calls(), []);
    push(&mut s, warrior(true));
    s.take_talent_rows_calls();
    click(&s, "BenillaTalentsRow2Cell1");
    click(&s, "BenillaTalentsRow2Cell2");
    assert_eq!(
        s.take_talent_rows_calls(),
        [TalentRowsCall::Set {
            row: "fire_blast".into(),
            spell_id: 201,
        }],
        "the set choice itself sends nothing"
    );
}

#[test]
fn the_footer_follows_at_innkeeper() {
    let mut s = open(warrior(false));
    assert!(shown(&s, "BenillaTalentsFooter"));
    assert_eq!(
        plain(&s, "BenillaTalentsFooter"),
        "Set rows change at an innkeeper."
    );
    push(&mut s, warrior(true));
    assert_eq!(
        plain(&s, "BenillaTalentsFooter"),
        "At the innkeeper: any open row can be changed."
    );
}

#[test]
fn the_eleventh_row_is_inside_the_scroll_child() {
    let rows = (1..=11u8)
        .map(|n| row(&format!("row{n}"), n * 2, true, "", u32::from(n) * 100, 0))
        .collect();
    let s = open(TalentRowsView {
        calling: "Tester".into(),
        at_innkeeper: false,
        rows,
    });
    assert_eq!(
        s.eval::<String>("return BenillaTalentsRow11:GetParent():GetName()")
            .unwrap(),
        "BenillaTalentsScrollChild"
    );
    assert_eq!(plain(&s, "BenillaTalentsRow11Level"), "22");
    assert_eq!(
        s.eval::<f64>("return BenillaTalentsScrollChild:GetHeight()")
            .unwrap(),
        11.0 * 80.0
    );
}

#[test]
fn a_calling_with_no_rows_says_so_and_hides_the_footer() {
    let mut s = open(warrior(false));
    push(
        &mut s,
        TalentRowsView {
            calling: "Zealot".into(),
            at_innkeeper: false,
            rows: Vec::new(),
        },
    );
    assert!(shown(&s, "BenillaTalentsEmpty"));
    assert_eq!(plain(&s, "BenillaTalentsEmpty"), "No talents yet.");
    assert!(!shown(&s, "BenillaTalentsFooter"));
    assert!(!shown(&s, "BenillaTalentsRow1"));
}

#[test]
fn the_window_opens_on_the_update_after_it_asks_and_on_the_innkeepers() {
    let mut s = window();
    // Login's rows open nothing.
    push(&mut s, warrior(false));
    assert!(!shown(&s, "BenillaTalents"));
    // The micro button asks the realm and opens on its answer.
    s.run("BenillaTalents_Toggle()").unwrap();
    assert_eq!(s.take_talent_rows_calls(), [TalentRowsCall::Open]);
    assert!(!shown(&s, "BenillaTalents"));
    push(&mut s, warrior(false));
    assert!(shown(&s, "BenillaTalents"));
    s.run("BenillaTalents_Toggle()").unwrap();
    assert!(!shown(&s, "BenillaTalents"));
    assert_eq!(s.take_talent_rows_calls(), []);
    // The innkeeper's line sends the rows at the innkeeper, which open the window.
    push(&mut s, warrior(true));
    assert!(shown(&s, "BenillaTalents"));
    click(&s, "BenillaTalentsClose");
    assert!(!shown(&s, "BenillaTalents"));
}
