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
/// A Skill Choice row's choices carry a Change Line; a Talent row's (`skill` "") carry "".
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
                change: change_line(key, skill, n),
                icon: None,
            })
            .collect(),
    }
}

fn change_line(key: &str, skill: &str, n: u32) -> String {
    if skill.is_empty() {
        String::new()
    } else {
        format!("{skill} changes the {key} {n} way, at length enough to wrap on the tooltip.")
    }
}

/// Hover choice `j` of row `i` the way the cell's OnEnter does, and read the tooltip's left lines.
/// The stock GameTooltip and UIParent are game data, so the test makes a bare tooltip that the
/// cell owns.
fn hover(s: &UiScript, i: usize, j: usize) -> Vec<String> {
    s.run(&format!(
        "if not GameTooltip then CreateFrame('GameTooltip', 'GameTooltip') end \
         GameTooltip:SetOwner(BenillaTalentsRow{i}Cell{j}, 'ANCHOR_NONE') \
         GameTooltip:SetTalent({i}, {j})"
    ))
    .unwrap();
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    let n = s.eval::<i64>("return GameTooltip:NumLines()").unwrap();
    (1..=n)
        .map(|k| plain(s, &format!("GameTooltipTextLeft{k}")))
        .collect()
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

/// `frame`'s left and right edges.
fn span(s: &UiScript, frame: &str) -> (f64, f64) {
    s.eval::<(f64, f64)>(&format!("return {frame}:GetLeft(), {frame}:GetRight()"))
        .unwrap()
}

#[test]
fn the_rows_sit_centred_inside_the_painted_sheet() {
    let s = open(warrior(false));
    let (sheet_left, sheet_right) = span(&s, "BenillaTalentsMiddle");
    let inside = |frame: &str, left: f64, right: f64| {
        let (l, r) = span(&s, frame);
        assert!(
            l >= left && r <= right,
            "{frame} runs {l} to {r}, outside {left} to {right}"
        );
    };
    for i in 1..=4 {
        inside(
            &format!("BenillaTalentsRow{i}Level"),
            sheet_left,
            sheet_right,
        );
        for j in 1..=3 {
            let cell = format!("BenillaTalentsRow{i}Cell{j}");
            inside(&cell, sheet_left, sheet_right);
            let (cell_left, cell_right) = span(&s, &cell);
            inside(&format!("{cell}Icon"), cell_left, cell_right);
            inside(&format!("{cell}Name"), cell_left, cell_right);
        }
    }
    // Fire Blast's second choice is set: its gold edges frame the whole cell.
    let set = span(&s, "BenillaTalentsRow2Cell2");
    for side in ["Top", "Bottom"] {
        assert_eq!(span(&s, &format!("BenillaTalentsRow2Cell2Set{side}")), set);
    }
    let centre = |frame: &str| {
        let (l, r) = span(&s, frame);
        (l + r) / 2.0
    };
    let off = centre("BenillaTalentsScroll") - centre("BenillaTalents");
    assert!(off.abs() <= 0.5, "the rows sit {off} px off centre");
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
        11.0 * 64.0
    );
}

/// `frame`'s top and bottom edges.
fn rise(s: &UiScript, frame: &str) -> (f64, f64) {
    s.eval::<(f64, f64)>(&format!("return {frame}:GetTop(), {frame}:GetBottom()"))
        .unwrap()
}

#[test]
fn ten_rows_paint_and_the_tenth_scrolls_into_view() {
    // A Tier 2 Calling: rows at levels 5 to 50, the first three open.
    let rows = (1..=10u8)
        .map(|n| row(&format!("row{n}"), n * 5, n <= 3, "", u32::from(n) * 100, 0))
        .collect();
    let mut s = open(TalentRowsView {
        calling: "Vanguard".into(),
        at_innkeeper: false,
        rows,
    });
    for i in 1..=10u8 {
        let strip = format!("BenillaTalentsRow{i}");
        assert!(shown(&s, &strip), "{strip} is shown");
        assert_eq!(
            plain(&s, &format!("{strip}Level")),
            (u32::from(i) * 5).to_string()
        );
        assert_eq!(plain(&s, &format!("{strip}Cell1Name")), format!("row{i} 1"));
    }
    assert!(
        s.eval::<bool>("return BenillaTalentsRow11 == nil").unwrap(),
        "no eleventh strip"
    );
    // The tenth strip starts below the scroll frame's bottom edge, and the wheel brings it in.
    let (top, bottom) = rise(&s, "BenillaTalentsScroll");
    assert!(rise(&s, "BenillaTalentsRow10").1 < bottom);
    for _ in 0..10 {
        s.run("BenillaTalents_Wheel(BenillaTalentsScroll, -1)")
            .unwrap();
    }
    s.resolve();
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    let (row_top, row_bottom) = rise(&s, "BenillaTalentsRow10");
    assert!(
        row_top <= top && row_bottom >= bottom - 0.5,
        "the tenth strip runs {row_top} to {row_bottom}, the scroll frame {top} to {bottom}"
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
fn hovering_a_skill_choice_shows_its_name_skill_and_change_line() {
    let s = open(warrior(false));
    assert_eq!(
        hover(&s, 1, 2),
        [
            "charge 2".to_string(),
            "Upgrades Charge".to_string(),
            change_line("charge", "Charge", 2),
        ],
        "no cost, range, cast time or cooldown lines"
    );
}

#[test]
fn hovering_a_talent_shows_its_spell_tooltip() {
    let mut view = warrior(false);
    view.rows.push(row("tough", 10, true, "", 500, 0));
    let s = open(view);
    // A Skill Choice's lines first, so the Talent's hover must clear them.
    hover(&s, 1, 1);
    let lines = hover(&s, 5, 1);
    assert_eq!(lines.first().map(String::as_str), Some("tough 1"));
    assert!(
        !lines.iter().any(|l| l.starts_with("Upgrades")),
        "a Talent names no skill: {lines:?}"
    );
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
