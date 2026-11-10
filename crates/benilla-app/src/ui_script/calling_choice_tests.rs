//! The Calling Choice popup (`assets/ui/CallingChoice.xml`) over pushed options: the rows, the
//! Choose and close buttons, and the skill tooltips. It needs no game data, so a missing stock
//! template only warns.

use benilla_ui::script::{CallingChoiceView, CallingOptionView, CallingSkillView, UiScript};

use super::test_ui::load_ui as load_xml;

fn popup() -> UiScript {
    let mut s = UiScript::new().unwrap();
    s.set_screen_size(1024.0, 768.0);
    load_xml(&s, "CallingChoice.xml");
    assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    s
}

/// One app tick: OnUpdate, then resolve.
fn frame(s: &mut UiScript) {
    s.tick(0.0);
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

/// A stand-in `GameTooltip` that records its lines, since these tests load no stock files.
fn stub_tooltip(s: &UiScript) {
    s.run(
        "GameTooltip = { lines = {} }
         function GameTooltip:SetOwner() self.lines = {} end
         function GameTooltip:SetText(text) self.lines = { text } end
         function GameTooltip:AddLine(text) table.insert(self.lines, text) end
         function GameTooltip:Show() end
         function GameTooltip:Hide() end",
    )
    .unwrap();
}

fn skill(name: &str) -> CallingSkillView {
    CallingSkillView {
        name: name.into(),
        icon: "Interface\\Icons\\Ability_Warrior_Charge".into(),
        description: format!("{name} hits hard."),
    }
}

fn option(quest: u32, name: &str, power: &str) -> CallingOptionView {
    CallingOptionView {
        quest,
        name: name.into(),
        icon: "Interface\\Icons\\INV_Sword_04".into(),
        power: power.into(),
        blurb: format!("The {name} blurb."),
        objectives: format!("Slay ten foes for the {name}."),
        skills: vec![skill("Smash"), skill("Cleave")],
    }
}

/// Two options: a Vanguard on Rage and a Cutthroat on Energy.
fn two_options() -> CallingChoiceView {
    CallingChoiceView {
        options: vec![
            option(64010, "Vanguard", "Rage"),
            option(64012, "Cutthroat", "Energy"),
        ],
    }
}

fn shown_popup() -> UiScript {
    let mut s = popup();
    s.set_calling_choice(Some(two_options()));
    frame(&mut s);
    s
}

#[test]
fn pushing_options_shows_the_frame_with_each_row_painted() {
    let s = shown_popup();
    assert!(shown(&s, "BenillaCallingChoice"));
    for (row, view) in two_options().options.iter().enumerate() {
        let base = format!("BenillaCallingChoiceRow{}", row + 1);
        assert!(shown(&s, &base));
        assert_eq!(text(&s, &format!("{base}Name")), view.name);
        assert_eq!(text(&s, &format!("{base}Power")), view.power);
        assert_eq!(text(&s, &format!("{base}Blurb")), view.blurb);
        assert_eq!(text(&s, &format!("{base}Objectives")), view.objectives);
        assert!(shown(&s, &format!("{base}Skill2")));
        assert!(!shown(&s, &format!("{base}Skill3")));
    }
    assert!(!shown(&s, "BenillaCallingChoiceRow3"));
}

#[test]
fn choose_queues_that_rows_quest_and_hides_the_frame() {
    let mut s = shown_popup();
    s.run("BenillaCallingChoiceRow2Choose:Click()").unwrap();
    frame(&mut s);
    assert_eq!(s.take_calling_choice_requests(), vec![64012]);
    assert!(!shown(&s, "BenillaCallingChoice"));
}

#[test]
fn close_hides_the_frame_and_queues_nothing() {
    let mut s = shown_popup();
    s.run("BenillaCallingChoiceClose:Click()").unwrap();
    frame(&mut s);
    assert!(s.take_calling_choice_requests().is_empty());
    assert!(!shown(&s, "BenillaCallingChoice"));
}

/// A stand-in `UnitClass` that answers `calling` for the player, since these tests have no player
/// record.
fn stub_unit_class(s: &UiScript, calling: &str) {
    s.run(&format!(
        "function UnitClass(unit) return '{calling}', 'WARRIOR' end"
    ))
    .unwrap();
}

/// The popup shown over `view` for a player whose Calling is `calling`.
fn shown_as(calling: &str, view: CallingChoiceView) -> UiScript {
    let mut s = popup();
    stub_unit_class(&s, calling);
    s.set_calling_choice(Some(view));
    frame(&mut s);
    s
}

fn five_options() -> CallingChoiceView {
    CallingChoiceView {
        options: vec![
            option(64010, "Vanguard", "Rage"),
            option(64011, "Ranger", "Focus"),
            option(64012, "Cutthroat", "Energy"),
            option(64013, "Acolyte", "Mana"),
            option(64014, "Initiate", "Mana"),
        ],
    }
}

fn height(s: &UiScript) -> f64 {
    s.eval::<f64>("return BenillaCallingChoice:GetHeight()")
        .unwrap()
}

#[test]
fn the_subtitle_names_the_players_calling() {
    let s = shown_as("Vanguard", two_options());
    assert_eq!(
        text(&s, "BenillaCallingChoiceSubtitle"),
        "You have learned all a Vanguard can learn. Choose the path you will take."
    );
    let s = shown_as("Basic Warrior", five_options());
    assert_eq!(
        text(&s, "BenillaCallingChoiceSubtitle"),
        "You have learned all a Basic Warrior can learn. Choose the path you will take."
    );
}

#[test]
fn the_frame_is_as_tall_as_the_rows_shown() {
    let five = shown_as("Basic Warrior", five_options());
    assert_eq!(height(&five), 690.0);
    assert!(shown(&five, "BenillaCallingChoiceRow5"));
    let two = shown_as("Vanguard", two_options());
    assert_eq!(height(&two), 690.0 - 3.0 * 118.0);
}

#[test]
fn hovering_a_skill_icon_shows_its_name_and_description() {
    let mut s = shown_popup();
    stub_tooltip(&s);
    s.run("this = BenillaCallingChoiceRow1Skill2 BenillaCallingChoiceSkill_OnEnter()")
        .unwrap();
    frame(&mut s);
    let lines = s.eval::<Vec<String>>("return GameTooltip.lines").unwrap();
    assert_eq!(
        lines,
        vec!["Cleave".to_string(), "Cleave hits hard.".into()]
    );
}
