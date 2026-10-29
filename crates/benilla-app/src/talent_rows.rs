//! The Talents panel's host (ADR 0028, spec #498, `assets/ui/Talents.xml`): the realm's
//! `TalentRows` becomes the view Lua reads, each choice wearing the icon of its own Spell.dbc row
//! in the client's catalog, and every call the panel queued goes back as `SetTalentRow` or
//! `OpenTalentRows`. The client decides nothing.

use bevy::prelude::*;

use benilla_protocol::SessionEvent;
use benilla_ui::script::{
    TalentChoiceView, TalentRowView, TalentRowsCall, TalentRowsView, UiScript,
};
use twilight_realm::RealmCommand;

use crate::ui_action::Spells;
use crate::ui_script::UiFeed;

/// The realm's `TalentRows` events, which the Twilight pump hands over instead of the net layer.
#[derive(Resource, Default)]
pub(crate) struct TalentRowsInbox(pub(crate) Vec<SessionEvent>);

/// The panel's calls, which the Twilight pump sends to the realm.
#[derive(Resource, Default)]
pub(crate) struct TalentRowsOutbox(pub(crate) Vec<RealmCommand>);

/// The last rows the realm sent, and what the feed owes Lua.
#[derive(Resource, Default)]
struct TalentRows {
    view: TalentRowsView,
    /// The view changed since the last push.
    changed: bool,
    /// `TALENT_ROWS_UPDATE`s owed, one per `TalentRows` event.
    updates: usize,
}

/// The view of a `TalentRows` event, field for field; `None` for any other event. The icons are
/// the feed's, from the spell catalog.
fn view_of(event: &SessionEvent) -> Option<TalentRowsView> {
    let SessionEvent::TalentRows {
        calling,
        at_innkeeper,
        rows,
    } = event
    else {
        return None;
    };
    Some(TalentRowsView {
        calling: calling.clone(),
        at_innkeeper: *at_innkeeper,
        rows: rows
            .iter()
            .map(|r| TalentRowView {
                key: r.key.clone(),
                level: r.level,
                open: r.open,
                skill: r.skill.clone(),
                set: r.set,
                choices: r
                    .choices
                    .iter()
                    .map(|c| TalentChoiceView {
                        spell_id: c.spell_id,
                        name: c.name.clone(),
                        icon: None,
                    })
                    .collect(),
            })
            .collect(),
    })
}

/// The realm command a queued call stands for.
fn command_of(call: TalentRowsCall) -> RealmCommand {
    match call {
        TalentRowsCall::Set { row, spell_id } => RealmCommand::SetTalentRow { row, spell_id },
        TalentRowsCall::Open => RealmCommand::OpenTalentRows,
    }
}

/// `view` with each choice's icon from its spell's own row, the path the spell book takes.
fn with_icons(mut view: TalentRowsView, spells: &Spells) -> TalentRowsView {
    for choice in view.rows.iter_mut().flat_map(|r| r.choices.iter_mut()) {
        choice.icon = spells
            .catalog
            .get(choice.spell_id)
            .and_then(|d| d.icon.clone());
    }
    view
}

/// Reads the realm's rows.
fn run_talent_rows(mut inbox: ResMut<TalentRowsInbox>, mut rows: ResMut<TalentRows>) {
    for event in inbox.0.drain(..) {
        match &event {
            SessionEvent::TalentRows { .. } => {
                rows.view = view_of(&event).unwrap_or_default();
                rows.updates += 1;
            }
            SessionEvent::LoggedOut => rows.view = TalentRowsView::default(),
            _ => continue,
        }
        rows.changed = true;
    }
}

/// Pushes the rows into Lua, fires `TALENT_ROWS_UPDATE` for each the realm sent and sends the
/// panel's calls. Gated on the in-game UI; a new VM, or icons that landed late, get the rows again.
fn feed_talent_rows(
    script: Option<NonSendMut<UiScript>>,
    mut rows: ResMut<TalentRows>,
    mut outbox: ResMut<TalentRowsOutbox>,
    spells: Option<Res<Spells>>,
    mut pushed: Local<crate::ui_script::VmMemo<()>>,
) {
    let Some(mut script) = script else {
        return;
    };
    let (_, new_vm) = pushed.get_reset(&script);
    let icons_moved = spells.as_ref().is_some_and(|s| s.is_changed());
    if new_vm || rows.changed || icons_moved {
        rows.changed = false;
        let view = match spells.as_deref() {
            Some(spells) => with_icons(rows.view.clone(), spells),
            None => rows.view.clone(),
        };
        script.set_talent_rows(view);
    }
    for call in script.take_talent_rows_calls() {
        outbox.0.push(command_of(call));
    }
    for _ in 0..std::mem::take(&mut rows.updates) {
        script.fire_event("TALENT_ROWS_UPDATE", Vec::new());
    }
}

pub(crate) struct TalentRowsPlugin;

impl Plugin for TalentRowsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TalentRowsInbox>()
            .init_resource::<TalentRowsOutbox>()
            .init_resource::<TalentRows>()
            .add_systems(
                Update,
                (
                    run_talent_rows,
                    feed_talent_rows.run_if(crate::ui_script::ingame_ui_up),
                )
                    .chain()
                    .in_set(UiFeed),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUSH: u32 = 1_009_101;
    const RALLY: u32 = 1_009_102;
    const RECKLESS: u32 = 1_009_103;

    fn rows_event() -> SessionEvent {
        SessionEvent::TalentRows {
            calling: "Basic Warrior".into(),
            at_innkeeper: true,
            rows: vec![benilla_protocol::TalentRowView {
                key: "charge".into(),
                level: 2,
                open: true,
                skill: "Charge".into(),
                set: RALLY,
                choices: vec![
                    benilla_protocol::TalentChoiceView {
                        spell_id: RUSH,
                        name: "Bull Rush".into(),
                    },
                    benilla_protocol::TalentChoiceView {
                        spell_id: RALLY,
                        name: "Rallying Charge".into(),
                    },
                    benilla_protocol::TalentChoiceView {
                        spell_id: RECKLESS,
                        name: "Reckless Charge".into(),
                    },
                ],
            }],
        }
    }

    #[test]
    fn a_talent_rows_event_becomes_the_view_field_for_field() {
        let view = view_of(&rows_event()).unwrap();
        let choice = |spell_id, name: &str| TalentChoiceView {
            spell_id,
            name: name.into(),
            icon: None,
        };
        assert_eq!(
            view,
            TalentRowsView {
                calling: "Basic Warrior".into(),
                at_innkeeper: true,
                rows: vec![TalentRowView {
                    key: "charge".into(),
                    level: 2,
                    open: true,
                    skill: "Charge".into(),
                    set: RALLY,
                    choices: vec![
                        choice(RUSH, "Bull Rush"),
                        choice(RALLY, "Rallying Charge"),
                        choice(RECKLESS, "Reckless Charge"),
                    ],
                }],
            }
        );
        assert!(view_of(&SessionEvent::LoggedOut).is_none());
    }

    #[test]
    fn every_queued_call_becomes_its_realm_command() {
        assert!(matches!(
            command_of(TalentRowsCall::Set {
                row: "charge".into(),
                spell_id: RUSH,
            }),
            RealmCommand::SetTalentRow { row, spell_id } if row == "charge" && spell_id == RUSH
        ));
        assert!(matches!(
            command_of(TalentRowsCall::Open),
            RealmCommand::OpenTalentRows
        ));
    }

    #[test]
    fn the_feed_pushes_the_rows_and_sends_the_choices() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.init_resource::<TalentRowsOutbox>();
        world.init_resource::<TalentRows>();
        world.insert_resource(TalentRowsInbox(vec![rows_event()]));
        let mut spells = Spells::empty_for_tests();
        spells.catalog = benilla_formats::SpellCatalog::from_displays(
            [(
                RUSH,
                benilla_formats::SpellDisplay {
                    name: "Bull Rush".into(),
                    icon: Some("Interface\\Icons\\Ability_BullRush".into()),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
        );
        world.insert_resource(spells);
        world.insert_non_send_resource(UiScript::new().unwrap());
        world
            .non_send_resource_mut::<UiScript>()
            .run(
                "Updates = 0 local f = CreateFrame('Frame') f:RegisterEvent('TALENT_ROWS_UPDATE') \
                 f:SetScript('OnEvent', function() Updates = Updates + 1 end)",
            )
            .unwrap();
        world.run_system_once(run_talent_rows).unwrap();
        world.run_system_once(feed_talent_rows).unwrap();
        let script = world.non_send_resource::<UiScript>();
        let said = |lua: &str| script.eval::<String>(lua).unwrap();
        assert_eq!(
            said(
                "local n, inn, calling = GetTalentRows() \
                 return n .. tostring(inn) .. calling .. Updates"
            ),
            "1trueBasic Warrior1"
        );
        assert_eq!(
            said(
                "local key, level, open, skill, set = GetTalentRow(1) \
                 return key .. level .. tostring(open) .. skill .. set"
            ),
            format!("charge2trueCharge{RALLY}")
        );
        assert_eq!(
            said("local id, name, icon = GetTalentChoice(1, 1) return id .. name .. icon"),
            format!("{RUSH}Bull RushInterface\\Icons\\Ability_BullRush")
        );
        assert_eq!(
            said(
                "local id, name, icon = GetTalentChoice(1, 3) return id .. name .. tostring(icon)"
            ),
            format!("{RECKLESS}Reckless Chargenil"),
            "a spell the catalog lacks has no icon"
        );
        world
            .non_send_resource_mut::<UiScript>()
            .run("SetTalentRowChoice(1, 3) OpenTalentRows() SetTalentRowChoice(2, 1)")
            .unwrap();
        world.run_system_once(feed_talent_rows).unwrap();
        let sent = &world.resource::<TalentRowsOutbox>().0;
        assert_eq!(sent.len(), 2, "the row out of range sends nothing");
        assert!(matches!(
            &sent[0],
            RealmCommand::SetTalentRow { row, spell_id } if row == "charge" && *spell_id == RECKLESS
        ));
        assert!(matches!(sent[1], RealmCommand::OpenTalentRows));
    }
}
