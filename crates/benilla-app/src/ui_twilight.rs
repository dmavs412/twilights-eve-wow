//! Twilight's Eve's feed into the stock UI's Lua: the edits to FrameXML's own functions that a
//! rule of the Local Realm needs, run once per VM.

use bevy::prelude::*;

/// The stock UI keeps talents shut below level 10: `ToggleTalentFrame` returns early
/// (`UIParent.lua:206`) and `UpdateTalentButton` hides the micro button
/// (`MainMenuBarMicroButtons.lua:133`). Here the Talents panel (`Talents.xml`, ADR 0028) replaces
/// the stock frame, and a Calling's first Talent Row opens at level 2. So `ToggleTalentFrame`
/// toggles `BenillaTalents` and never loads `Blizzard_TalentUI`, and the micro button shows from
/// level 1; the rest of `UpdateTalentButton` is the stock body. Run once per VM, once FrameXML has
/// defined them.
pub(crate) const UNLOCK_TALENT_FRAME: &str = r#"
if type(ToggleTalentFrame) ~= "function" or type(UpdateTalentButton) ~= "function" then
    return false
end
function ToggleTalentFrame()
    if ( BenillaTalents_Toggle ) then
        BenillaTalents_Toggle();
    end
end
function UpdateTalentButton()
    TalentMicroButton:Show();
    QuestLogMicroButton:SetPoint("BOTTOMLEFT", "TalentMicroButton", "BOTTOMRIGHT", -2, 0);
end
UpdateTalentButton();
return true
"#;

/// Puts the Talents panel behind the talent key and micro button, in every UI VM.
pub(crate) fn unlock_talent_frame(
    script: Option<NonSendMut<benilla_ui::script::UiScript>>,
    mut memo: Local<crate::ui_script::VmMemo<bool>>,
) {
    let Some(script) = script else {
        return;
    };
    let done = memo.get(&script);
    if *done {
        return;
    }
    match script.eval::<bool>(UNLOCK_TALENT_FRAME) {
        Ok(unlocked) => *done = unlocked,
        Err(e) => {
            warn!("twilight: unlocking the talent frame: {e}");
            *done = true;
        }
    }
}
