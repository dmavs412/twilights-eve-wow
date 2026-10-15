//! Twilight's Eve's feed into the stock UI's Lua: the edits to FrameXML's own functions that a
//! rule of the Local Realm needs, run once per VM.

use bevy::prelude::*;

/// The stock UI keeps talents shut below level 10: `ToggleTalentFrame` returns early
/// (`UIParent.lua:206`) and `UpdateTalentButton` hides the micro button
/// (`MainMenuBarMicroButtons.lua:133`). A Talent Point comes at level 2 here, so both lose the
/// gate; the rest of each is the stock body. Run once per VM, once FrameXML has defined them.
const UNLOCK_TALENT_FRAME: &str = r#"
if type(ToggleTalentFrame) ~= "function" or type(UpdateTalentButton) ~= "function" then
    return false
end
function ToggleTalentFrame()
    TalentFrame_LoadUI();
    if ( TalentFrame_Toggle ) then
        TalentFrame_Toggle();
    end
end
function UpdateTalentButton()
    TalentMicroButton:Show();
    QuestLogMicroButton:SetPoint("BOTTOMLEFT", "TalentMicroButton", "BOTTOMRIGHT", -2, 0);
end
UpdateTalentButton();
return true
"#;

/// Opens the talent frame below level 10, in every UI VM.
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
