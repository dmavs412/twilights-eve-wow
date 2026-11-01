//! Twilight's Eve's feed into the stock UI's Lua: the edits to FrameXML's own functions that a
//! rule of the Local Realm needs, run once per VM, and the Noise Bar's meter.

use bevy::prelude::*;

use benilla_protocol::PhaseMeterState;
use benilla_ui::script::{PhaseMeterView, ScriptValue, UiScript};

/// The event the Noise Bar frame hears on every change of its meter (spec #533): `arg1` is the
/// cause line when the stacks rose, nil otherwise.
const PHASE_METER_EVENT: &str = "BENILLA_PHASE_METER";

/// Pushes the realm's meter into Lua and fires [`PHASE_METER_EVENT`] on every change. A fresh VM
/// gets the bar that is up, without the line of the rise it missed.
pub(crate) fn feed_phase_meter(
    script: Option<NonSendMut<UiScript>>,
    meter: Option<Res<crate::twilight::PhaseMeter>>,
    mut pushed: Local<crate::ui_script::VmMemo<Option<PhaseMeterState>>>,
) {
    let Some(mut script) = script else {
        return;
    };
    let now = meter.and_then(|m| m.0.clone());
    let (last, new_vm) = pushed.get_reset(&script);
    if *last == now {
        return;
    }
    *last = now.clone();
    let rose = now
        .as_ref()
        .and_then(|m| m.rose.clone())
        .filter(|_| !new_vm);
    script.set_phase_meter(now.map(|m| PhaseMeterView {
        name: m.name,
        stacks: m.stacks,
        max: m.max,
        end_icon: m.end_icon,
    }));
    script.fire_event(
        PHASE_METER_EVENT,
        vec![rose.map_or(ScriptValue::Nil, ScriptValue::Str)],
    );
}

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
    script: Option<NonSendMut<UiScript>>,
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
