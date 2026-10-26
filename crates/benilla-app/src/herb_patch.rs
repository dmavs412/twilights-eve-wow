//! The Forager's Grid window's host (spec #435, `assets/ui/HerbPatch.xml`): the realm's `HerbGrid`
//! becomes the view Lua reads, `HerbGridClosed` hides the window, and every `Herb*` call the
//! window queued goes back as a `RealmCommand::Herb`. The client holds no rules.
//!
//! The Herbalism Journal tab (`assets/ui/FishingJournal.xml`, #443) rides the same feed: `Herbalist`
//! and `HerbalismJournal` become the views its getters read, and its slot, perk and withdraw calls
//! go back as the realm's `SetHerbTool`, `SetHerbConsumable`, `PickPerk` and `WithdrawFromJournal`.

use bevy::prelude::*;

use benilla_protocol::{SatchelPick, SessionEvent};
use benilla_ui::script::{
    HerbCall, HerbGridView, HerbJournalCall, HerbJournalView, HerbKnownView, HerbPerkView,
    HerbTileView, HerbalistView, SatchelSlot, ScriptValue, UiScript,
};
use twilight_realm::{HerbAction, RealmCommand};

use crate::ui_script::UiFeed;

/// The realm's herbalism events, which the Twilight pump hands over instead of the net layer.
#[derive(Resource, Default)]
pub(crate) struct HerbInbox(pub(crate) Vec<SessionEvent>);

/// The window's clicks, which the Twilight pump sends to the realm.
#[derive(Resource, Default)]
pub(crate) struct HerbOutbox(pub(crate) Vec<RealmCommand>);

/// The open grid, and what the feed owes Lua.
#[derive(Resource, Default)]
struct HerbPatch {
    /// The last grid the realm sent, or `None` once it closed.
    grid: Option<HerbGridView>,
    /// The grid changed since the last push.
    changed: bool,
    /// The last `Herbalist` and `HerbalismJournal` the realm sent, and whether either changed
    /// since the last push.
    herbalist: Option<HerbalistView>,
    journal: Option<HerbJournalView>,
    journal_changed: bool,
    /// Events for Lua, oldest first.
    events: Vec<(&'static str, Vec<ScriptValue>)>,
}

/// The view of a `HerbGrid` event, field for field; `None` for any other event.
fn view_of(event: &SessionEvent) -> Option<HerbGridView> {
    let SessionEvent::HerbGrid {
        size,
        pulls_left,
        tiles,
        known,
        notes,
        log,
        tool,
        skill_cost,
        consumable,
        consumable_ready,
        charm_armed,
        ..
    } = event
    else {
        return None;
    };
    Some(HerbGridView {
        size: *size,
        pulls_left: *pulls_left,
        tiles: tiles
            .iter()
            .map(|t| HerbTileView {
                state: t.state,
                known: t.known,
                stake: t.stake,
                item: t.item,
                count: t.count,
                bloom: t.bloom,
            })
            .collect(),
        known: known
            .iter()
            .map(|k| HerbKnownView {
                item: k.item,
                present: k.present,
                rows: k.rows.clone(),
            })
            .collect(),
        notes: notes.clone(),
        log: log.clone(),
        tool: *tool,
        skill_cost: *skill_cost,
        consumable: *consumable,
        consumable_ready: *consumable_ready,
        charm_armed: *charm_armed,
    })
}

/// The view of a `Herbalist` event; `None` for any other event.
fn herbalist_of(event: &SessionEvent) -> Option<HerbalistView> {
    let SessionEvent::Herbalist {
        knows,
        level,
        xp,
        to_next,
        title,
        perks,
    } = event
    else {
        return None;
    };
    Some(HerbalistView {
        knows: *knows,
        level: u32::from(*level),
        xp: *xp,
        to_next: *to_next,
        title: title.clone(),
        perks: perks.clone(),
    })
}

/// The view of a `HerbalismJournal` event, field for field; `None` for any other event.
fn journal_of(event: &SessionEvent) -> Option<HerbJournalView> {
    let SessionEvent::HerbalismJournal {
        tool,
        consumable,
        satchel,
        picks,
        zones,
        reveal_at,
        perks,
    } = event
    else {
        return None;
    };
    Some(HerbJournalView {
        tool: *tool,
        consumable: *consumable,
        satchel: satchel
            .iter()
            .map(|row| SatchelSlot {
                item: row.item,
                count: row.count,
                wear: row.wear,
                tackle: row.tackle,
            })
            .collect(),
        picks: picks.clone(),
        zones: zones.clone(),
        reveal_at: *reveal_at,
        perks: perks
            .iter()
            .map(|(key, name, boon, text)| HerbPerkView {
                key: key.clone(),
                name: name.clone(),
                boon: *boon,
                text: text.clone(),
            })
            .collect(),
    })
}

/// The realm command a Journal call stands for.
fn journal_command_of(call: HerbJournalCall) -> RealmCommand {
    match call {
        HerbJournalCall::SetTool(item) => RealmCommand::SetHerbTool { item: Some(item) },
        HerbJournalCall::SetConsumable(item) => RealmCommand::SetHerbConsumable { item },
        HerbJournalCall::PickPerk { slot, key } => RealmCommand::PickPerk { slot, perk: key },
        HerbJournalCall::Withdraw { item, count } => RealmCommand::WithdrawFromJournal {
            pick: SatchelPick {
                item,
                durability: None,
            },
            count,
        },
    }
}

/// The realm command a queued call stands for.
fn command_of(call: HerbCall) -> RealmCommand {
    RealmCommand::Herb(match call {
        HerbCall::Pull(tile) => HerbAction::Pull(tile),
        HerbCall::Sweep { tile, across } => HerbAction::Sweep { tile, across },
        HerbCall::Probe(tile) => HerbAction::Probe(tile),
        HerbCall::Snip(tile) => HerbAction::Snip(tile),
        HerbCall::Sight(tiles) => HerbAction::Sight(tiles),
        HerbCall::Twine(tile) => HerbAction::Twine(tile),
        HerbCall::ArmCharm => HerbAction::ArmCharm,
        HerbCall::Leave => HerbAction::Leave,
    })
}

/// Reads the realm's herbalism events.
fn run_herbs(mut inbox: ResMut<HerbInbox>, mut patch: ResMut<HerbPatch>) {
    let p = &mut *patch;
    for event in inbox.0.drain(..) {
        match event {
            SessionEvent::HerbGridClosed { reason, .. } => {
                p.grid = None;
                p.changed = true;
                p.events.push((
                    "HERB_GRID_CLOSED",
                    vec![ScriptValue::Int(i64::from(reason))],
                ));
            }
            SessionEvent::LoggedOut => {
                p.grid = None;
                p.changed = true;
            }
            SessionEvent::Herbalist { .. } => {
                p.herbalist = herbalist_of(&event);
                p.journal_changed = true;
                p.events.push(("HERBALIST_UPDATE", Vec::new()));
            }
            SessionEvent::HerbalismJournal { .. } => {
                p.journal = journal_of(&event);
                p.journal_changed = true;
                p.events.push(("HERBALISM_JOURNAL_UPDATE", Vec::new()));
            }
            grid => {
                if let Some(view) = view_of(&grid) {
                    p.grid = Some(view);
                    p.changed = true;
                    p.events.push(("HERB_GRID_UPDATE", Vec::new()));
                }
            }
        }
    }
}

/// Pushes the grid into Lua, fires what happened since the last frame and sends the window's
/// clicks. Gated on the in-game UI; a new VM gets the open grid again.
fn feed_herbs(
    script: Option<NonSendMut<UiScript>>,
    mut patch: ResMut<HerbPatch>,
    mut outbox: ResMut<HerbOutbox>,
    mut pushed: Local<crate::ui_script::VmMemo<()>>,
) {
    let Some(mut script) = script else {
        return;
    };
    let (_, new_vm) = pushed.get_reset(&script);
    if new_vm || patch.changed {
        patch.changed = false;
        script.set_herb_grid(patch.grid.clone());
    }
    if new_vm || patch.journal_changed {
        patch.journal_changed = false;
        if let Some(herbalist) = patch.herbalist.clone() {
            script.set_herbalist(herbalist);
        }
        if let Some(journal) = patch.journal.clone() {
            script.set_herb_journal(journal);
        }
    }
    for call in script.take_herb_calls() {
        outbox.0.push(command_of(call));
    }
    for call in script.take_herb_journal_calls() {
        outbox.0.push(journal_command_of(call));
    }
    for (name, args) in std::mem::take(&mut patch.events) {
        script.fire_event(name, args);
    }
}

pub(crate) struct HerbPatchPlugin;

impl Plugin for HerbPatchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HerbInbox>()
            .init_resource::<HerbOutbox>()
            .init_resource::<HerbPatch>()
            .add_systems(
                Update,
                (run_herbs, feed_herbs.run_if(crate::ui_script::ingame_ui_up))
                    .chain()
                    .in_set(UiFeed),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use benilla_protocol::{HerbKnown, HerbTile};

    fn grid_event() -> SessionEvent {
        SessionEvent::HerbGrid {
            patch: 7,
            size: 5,
            pulls_left: 9,
            tiles: vec![
                HerbTile {
                    state: 1,
                    known: 0,
                    stake: -1,
                    item: 2447,
                    count: 2,
                    bloom: true,
                },
                HerbTile {
                    state: 0,
                    known: 3,
                    stake: 4,
                    item: 0,
                    count: 0,
                    bloom: false,
                },
            ],
            known: vec![HerbKnown {
                item: 2447,
                present: true,
                rows: vec![2, 4],
            }],
            notes: vec!["Bloom Draught: one plant here is now blooming.".into()],
            log: "Pulled B3: Peacebloom x2.".into(),
            tool: 5001,
            skill_cost: 2,
            consumable: 5002,
            consumable_ready: true,
            charm_armed: true,
        }
    }

    #[test]
    fn a_grid_event_becomes_the_view_field_for_field() {
        let view = view_of(&grid_event()).unwrap();
        assert_eq!(view.size, 5);
        assert_eq!(view.pulls_left, 9);
        assert_eq!(
            view.tiles[0],
            HerbTileView {
                state: 1,
                known: 0,
                stake: -1,
                item: 2447,
                count: 2,
                bloom: true,
            }
        );
        assert_eq!((view.tiles[1].known, view.tiles[1].stake), (3, 4));
        assert_eq!(
            view.known,
            vec![HerbKnownView {
                item: 2447,
                present: true,
                rows: vec![2, 4],
            }]
        );
        assert_eq!(view.notes.len(), 1);
        assert_eq!(view.log, "Pulled B3: Peacebloom x2.");
        assert_eq!(
            (view.tool, view.skill_cost, view.consumable),
            (5001, 2, 5002)
        );
        assert!(view.consumable_ready && view.charm_armed);
        assert!(view_of(&SessionEvent::LoggedOut).is_none());
    }

    #[test]
    fn every_queued_call_becomes_its_realm_command() {
        let cases = [
            (HerbCall::Pull(3), HerbAction::Pull(3)),
            (
                HerbCall::Sweep {
                    tile: 6,
                    across: false,
                },
                HerbAction::Sweep {
                    tile: 6,
                    across: false,
                },
            ),
            (HerbCall::Probe(1), HerbAction::Probe(1)),
            (HerbCall::Snip(24), HerbAction::Snip(24)),
            (HerbCall::Sight([0, 5, 9]), HerbAction::Sight([0, 5, 9])),
            (HerbCall::Twine(12), HerbAction::Twine(12)),
            (HerbCall::ArmCharm, HerbAction::ArmCharm),
            (HerbCall::Leave, HerbAction::Leave),
        ];
        for (call, action) in cases {
            assert!(matches!(command_of(call), RealmCommand::Herb(a) if a == action));
        }
    }

    #[test]
    fn every_journal_call_becomes_its_realm_command() {
        assert!(matches!(
            journal_command_of(HerbJournalCall::SetTool(961101)),
            RealmCommand::SetHerbTool { item: Some(961101) }
        ));
        assert!(matches!(
            journal_command_of(HerbJournalCall::SetConsumable(None)),
            RealmCommand::SetHerbConsumable { item: None }
        ));
        assert!(matches!(
            journal_command_of(HerbJournalCall::PickPerk { slot: 0, key: "reach".into() }),
            RealmCommand::PickPerk { slot: 0, perk } if perk == "reach"
        ));
        assert!(matches!(
            journal_command_of(HerbJournalCall::Withdraw { item: 961202, count: 3 }),
            RealmCommand::WithdrawFromJournal { pick, count: 3 }
                if pick.item == 961202 && pick.durability.is_none()
        ));
    }

    #[test]
    fn the_herbalist_and_journal_events_become_their_views() {
        let herbalist = herbalist_of(&SessionEvent::Herbalist {
            knows: true,
            level: 12,
            xp: 30,
            to_next: 90,
            title: "Herb Gatherer".into(),
            perks: vec!["reach".into()],
        })
        .unwrap();
        assert_eq!((herbalist.level, herbalist.perks.len()), (12, 1));
        assert!(herbalist_of(&SessionEvent::LoggedOut).is_none());
        let journal = journal_of(&SessionEvent::HerbalismJournal {
            tool: 961101,
            consumable: 0,
            satchel: Vec::new(),
            picks: vec![(2447, 7)],
            zones: vec![(12, vec![2447])],
            reveal_at: [5, 10, 25],
            perks: vec![("reach".into(), "Long Reach".into(), true, "text".into())],
        })
        .unwrap();
        assert_eq!((journal.tool, journal.picks[0].1), (961101, 7));
        assert!(journal.perks[0].boon);
    }

    #[test]
    fn the_feed_pushes_the_grid_and_sends_the_clicks() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.init_resource::<HerbOutbox>();
        world.insert_resource(HerbPatch {
            grid: view_of(&grid_event()),
            changed: true,
            ..HerbPatch::default()
        });
        world.insert_non_send_resource(UiScript::new().unwrap());
        world.run_system_once(feed_herbs).unwrap();
        let open: bool = world
            .non_send_resource::<UiScript>()
            .eval("return (GetHerbGrid())")
            .unwrap();
        assert!(open);
        world
            .non_send_resource_mut::<UiScript>()
            .run("HerbPull(4) HerbLeave()")
            .unwrap();
        world.run_system_once(feed_herbs).unwrap();
        let sent = &world.resource::<HerbOutbox>().0;
        assert_eq!(sent.len(), 2);
        assert!(matches!(sent[0], RealmCommand::Herb(HerbAction::Pull(3))));
    }
}
