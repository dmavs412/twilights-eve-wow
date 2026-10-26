//! The Calling Choice popup (spec #414): the realm sends `SessionEvent::CallingChoice` to a Basic
//! Warrior at the Tier Cap. This module keeps the latest one, resolves each skill id through the
//! client's `Spell.dbc` into the view `CallingChoice.xml` reads, and sends the pick back as
//! `RealmCommand::ChooseCalling`. It follows the Fishing Journal's path: the Twilight pump hands
//! the event over instead of the net layer, and the commands leave through [`ReelOutbox`].

use bevy::prelude::*;

use benilla_protocol::{CallingOption, SessionEvent};
use benilla_ui::script::{CallingChoiceView, CallingOptionView, CallingSkillView, UiScript};
use twilight_realm::RealmCommand;

use crate::fishing_reel::ReelOutbox;
use crate::ui_action::Spells;
use crate::ui_script::UiFeed;

/// The realm's Calling Choice events, which the Twilight pump hands over instead of the net layer.
#[derive(Resource, Default)]
pub(crate) struct CallingChoiceInbox(pub(crate) Vec<SessionEvent>);

/// The options of the latest event, kept until the player picks or logs out, so a new VM gets them
/// again.
#[derive(Resource, Default)]
struct CallingChoiceState {
    options: Option<Vec<CallingOption>>,
    /// The options changed since the last push.
    changed: bool,
}

/// The power bar's display name for a `Powers` value.
fn power_name(power: u8) -> &'static str {
    match power {
        0 => "Mana",
        1 => "Rage",
        2 => "Focus",
        _ => "Energy",
    }
}

/// The popup's options as Lua reads them, each skill named from `Spell.dbc` (empty with no row).
fn choice_view(options: &[CallingOption], spells: &Spells) -> CallingChoiceView {
    CallingChoiceView {
        options: options
            .iter()
            .map(|o| CallingOptionView {
                quest: o.quest,
                name: o.name.clone(),
                icon: o.icon.clone(),
                power: power_name(o.power).to_string(),
                blurb: o.blurb.clone(),
                objectives: o.objectives.clone(),
                skills: o
                    .skills
                    .iter()
                    .map(|&id| {
                        let row = spells.catalog.get(id);
                        CallingSkillView {
                            name: row.map(|d| d.name.clone()).unwrap_or_default(),
                            icon: row.and_then(|d| d.icon.clone()).unwrap_or_default(),
                            description: row
                                .and_then(|d| d.description.clone())
                                .unwrap_or_default(),
                        }
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// Keeps the latest event, pushes it into Lua and sends the picks. Gated on the in-game UI, so the
/// login burst waits in the inbox for a VM that stays. Waits for `Spell.dbc` too.
fn feed_calling_choice(
    script: Option<NonSendMut<UiScript>>,
    mut inbox: ResMut<CallingChoiceInbox>,
    mut state: ResMut<CallingChoiceState>,
    mut outbox: ResMut<ReelOutbox>,
    mut pushed: Local<crate::ui_script::VmMemo<()>>,
    spells: Option<Res<Spells>>,
) {
    let Some(mut script) = script else {
        return;
    };
    for event in inbox.0.drain(..) {
        match event {
            SessionEvent::CallingChoice { options } => {
                state.options = Some(options);
                state.changed = true;
            }
            SessionEvent::LoggedOut => state.options = None,
            _ => {}
        }
    }
    let (_, new_vm) = pushed.get_reset(&script);
    if let (Some(spells), Some(options)) = (spells.as_deref(), &state.options) {
        if new_vm || state.changed {
            script.set_calling_choice(Some(choice_view(options, spells)));
        }
        state.changed = false;
    }
    for quest in script.take_calling_choice_requests() {
        outbox.0.push(RealmCommand::ChooseCalling { quest });
        state.options = None;
    }
}

pub(crate) struct CallingChoicePlugin;

impl Plugin for CallingChoicePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CallingChoiceInbox>()
            .init_resource::<CallingChoiceState>()
            .add_systems(
                Update,
                feed_calling_choice
                    .run_if(crate::ui_script::ingame_ui_up)
                    .in_set(UiFeed),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_names_follow_the_powers_values() {
        assert_eq!(power_name(0), "Mana");
        assert_eq!(power_name(1), "Rage");
        assert_eq!(power_name(2), "Focus");
        assert_eq!(power_name(3), "Energy");
    }
}
