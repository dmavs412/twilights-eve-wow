//! The talent frame follows the current Calling: whenever the character in the world has a
//! different Calling than the catalog was built for (login, and Advancement later), the client's
//! talent catalog is replaced with that Calling's Talent Tree. `feed_talents` rebuilds the frame
//! when the resource changes. The tree is the realm's data; this only reshapes it.

use benilla_formats::{Talent, TalentCatalog, TalentTabInfo, MAX_TALENT_RANK};
use bevy::prelude::*;
use twilight_realm::callings::CallingDef;

use super::LocalRealm;
use crate::ui_talent::Talents;

/// Every race's bit (`ChrRaces.dbc` 1..9), as the shipped tabs carry it.
const ALL_RACES: u32 = 511;
/// The Warrior's class bit.
const WARRIOR: u32 = 1;

/// Puts the current Calling's tree in the talent catalog when it changes.
pub(super) fn follow_calling(
    local: Res<LocalRealm>,
    mut commands: Commands,
    mut built_for: Local<Option<&'static str>>,
) {
    let Some(calling) = local.realm.current_calling() else {
        return;
    };
    if *built_for == Some(calling.key.as_str()) {
        return;
    }
    *built_for = Some(calling.key.as_str());
    let catalog = catalog(calling);
    info!(
        "twilight: the talent frame shows {}'s tree ({} talents)",
        calling.key,
        catalog.len()
    );
    commands.insert_resource(Talents { catalog });
}

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
pub(super) fn unlock_talent_frame(
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

/// A Calling's Talent Tree as the client's catalog: its tabs in order, each talent on its 0-based
/// seat with its rank spells, and a prerequisite as the talent id and 0-based rank it needs.
fn catalog(calling: &CallingDef) -> TalentCatalog {
    let tabs = calling
        .talent_tabs
        .iter()
        .map(|tab| TalentTabInfo {
            id: tab.id,
            name: tab.name.clone(),
            race_mask: ALL_RACES,
            class_mask: WARRIOR,
            background: tab.background.clone(),
        })
        .collect();
    let talents = calling
        .talent_tabs
        .iter()
        .flat_map(|tab| {
            tab.talents.iter().map(move |t| {
                let mut ranks = [0; MAX_TALENT_RANK];
                for (slot, &spell) in ranks.iter_mut().zip(&t.rank_spells) {
                    *slot = spell;
                }
                let prereq = t.prereq.as_ref().and_then(|p| {
                    let needed = tab.talents.iter().find(|o| o.key == p.key)?;
                    Some((needed.talent_id, u32::from(p.rank.saturating_sub(1))))
                });
                Talent {
                    id: t.talent_id,
                    tab: tab.id,
                    row: u32::from(t.tier.saturating_sub(1)),
                    col: u32::from(t.column.saturating_sub(1)),
                    ranks,
                    prereq_talent: prereq.map_or(0, |(id, _)| id),
                    prereq_rank: prereq.map_or(0, |(_, rank)| rank),
                    required_spell: 0,
                    exceptional: t.exceptional,
                }
            })
        })
        .collect();
    TalentCatalog::from_parts(tabs, talents)
}
