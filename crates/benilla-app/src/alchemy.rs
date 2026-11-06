//! The Alchemy window's host (ADR 0025, spec #522, `assets/ui/Alchemy.xml`): the realm's
//! `AlchemyBench`, `AlchemyPreview` and `AlchemyMade` become the views Lua reads, `AlchemyClosed`
//! hides the window, and every `Alchemy*` call the window queued goes back as a
//! `RealmCommand::Alchemy`. The client computes no result.

use bevy::prelude::*;

use benilla_protocol::{AlchemyToxic, AlchemyVial, SessionEvent};
use benilla_ui::script::{
    AlchemyBenchView, AlchemyCall, AlchemyGradeView, AlchemyMadeView, AlchemyMakeView,
    AlchemyMaterialView, AlchemyPreviewView, AlchemyRumourView, AlchemyToxicView, AlchemyVialView,
    UiScript,
};
use twilight_realm::{AlchemyAction, RealmCommand};

use crate::ui_script::UiFeed;

/// The realm's alchemy table events, which the Twilight pump hands over instead of the net layer.
#[derive(Resource, Default)]
pub(crate) struct AlchemyInbox(pub(crate) Vec<SessionEvent>);

/// The window's choices, which the Twilight pump sends to the realm.
#[derive(Resource, Default)]
pub(crate) struct AlchemyOutbox(pub(crate) Vec<RealmCommand>);

/// The open table, and what the feed owes Lua.
#[derive(Resource, Default)]
struct Alchemy {
    /// The last table the realm sent, or `None` once it closed.
    view: Option<AlchemyBenchView>,
    preview: Option<AlchemyPreviewView>,
    made: Option<AlchemyMadeView>,
    /// Any of the three changed since the last push.
    changed: bool,
    /// Events for Lua, oldest first.
    events: Vec<&'static str>,
}

/// The view of an `AlchemyBench` event, field for field; `None` for any other event.
fn view_of(event: &SessionEvent) -> Option<AlchemyBenchView> {
    let SessionEvent::AlchemyBench {
        table,
        alchemist,
        line,
        tiers,
        makes,
        tray,
        targets,
        rumours,
        sludge,
        ..
    } = event
    else {
        return None;
    };
    Some(AlchemyBenchView {
        table: table.clone(),
        alchemist: alchemist.clone(),
        line: line.clone(),
        tiers: tiers.clone(),
        makes: makes
            .iter()
            .map(|m| AlchemyMakeView {
                key: m.key.clone(),
                name: m.name.clone(),
                station: m.station.clone(),
                verb: m.verb.clone(),
                rule: m.rule.clone(),
                open: m.open,
                herbs: m.herbs,
                tinctures: m.tinctures,
                essences: m.essences,
                fish: m.fish,
                wash: m.wash,
            })
            .collect(),
        tray: tray
            .iter()
            .map(|t| AlchemyMaterialView {
                item: t.item,
                count: t.count,
                kind: t.kind,
                level: t.level,
                values: t
                    .values
                    .iter()
                    .map(|v| (v.name.clone(), v.points))
                    .collect(),
                prop: t.prop.clone(),
                points: t.points,
                toxic: t.toxic,
                step: t.step,
            })
            .collect(),
        targets: targets
            .iter()
            .map(|t| AlchemyGradeView {
                family: t.family.clone(),
                key: t.key.clone(),
                grade: t.grade,
                name: t.name.clone(),
                learned: t.learned,
                rumoured: t.rumoured,
                threshold: t.threshold,
            })
            .collect(),
        rumours: rumours
            .iter()
            .map(|r| AlchemyRumourView {
                family: r.family.clone(),
                key: r.key.clone(),
                grade: r.grade,
                name: r.name.clone(),
                price: r.price,
            })
            .collect(),
        sludge: *sludge,
    })
}

fn vials_of(vials: &[AlchemyVial]) -> Vec<AlchemyVialView> {
    vials
        .iter()
        .map(|v| AlchemyVialView {
            prop: v.prop.clone(),
            value: v.value,
            unknown: v.unknown,
            main: v.main,
            target: v.target,
            bonus: v.bonus,
            tick: v.tick,
        })
        .collect()
}

fn toxic_of(t: &AlchemyToxic) -> AlchemyToxicView {
    AlchemyToxicView {
        raw: t.raw,
        purify: t.purify,
        net: t.net,
        unknown: t.unknown,
        fail_pct: t.fail_pct,
        bonus: t.bonus,
    }
}

/// The view of an `AlchemyPreview` event; `None` for any other event.
fn preview_of(event: &SessionEvent) -> Option<AlchemyPreviewView> {
    let SessionEvent::AlchemyPreview {
        ready,
        note,
        vials,
        toxic,
        unnoted,
        scale,
        rings,
        hybrid,
        known,
        product,
        sludge,
        carried,
    } = event
    else {
        return None;
    };
    Some(AlchemyPreviewView {
        ready: *ready,
        note: note.clone(),
        vials: vials_of(vials),
        toxic: toxic_of(toxic),
        unnoted: *unnoted,
        scale: *scale,
        rings: *rings,
        hybrid: *hybrid,
        known: *known,
        product: *product,
        sludge: *sludge,
        carried: *carried,
    })
}

/// The view of an `AlchemyMade` event; `None` for any other event.
fn made_of(event: &SessionEvent) -> Option<AlchemyMadeView> {
    let SessionEvent::AlchemyMade {
        product,
        failed,
        fail_pct,
        vials,
        toxic,
        scale,
        rings,
        lines,
    } = event
    else {
        return None;
    };
    Some(AlchemyMadeView {
        product: *product,
        failed: *failed,
        fail_pct: *fail_pct,
        vials: vials_of(vials),
        toxic: toxic_of(toxic),
        scale: *scale,
        rings: *rings,
        lines: lines.clone(),
    })
}

/// The realm command a queued call stands for.
fn command_of(call: AlchemyCall) -> RealmCommand {
    RealmCommand::Alchemy(match call {
        AlchemyCall::Preview {
            make,
            items,
            target,
        } => AlchemyAction::Preview {
            make,
            items,
            target,
        },
        AlchemyCall::Make {
            make,
            items,
            target,
        } => AlchemyAction::Make {
            make,
            items,
            target,
        },
        AlchemyCall::BuyRumour { family, key, grade } => {
            AlchemyAction::BuyRumour { family, key, grade }
        }
        AlchemyCall::SellSludge => AlchemyAction::SellSludge,
        AlchemyCall::Close => AlchemyAction::Close,
    })
}

/// Reads the realm's alchemy table events.
fn run_alchemy(mut inbox: ResMut<AlchemyInbox>, mut alchemy: ResMut<Alchemy>) {
    let a = &mut *alchemy;
    for event in inbox.0.drain(..) {
        match &event {
            SessionEvent::AlchemyBench { .. } => {
                a.view = view_of(&event);
                a.events.push("ALCHEMY_UPDATE");
            }
            SessionEvent::AlchemyPreview { .. } => {
                a.preview = preview_of(&event);
                a.events.push("ALCHEMY_PREVIEW");
            }
            SessionEvent::AlchemyMade { .. } => {
                a.made = made_of(&event);
                a.events.push("ALCHEMY_MADE");
            }
            SessionEvent::AlchemyClosed => {
                a.view = None;
                a.preview = None;
                a.made = None;
                a.events.push("ALCHEMY_CLOSED");
            }
            SessionEvent::LoggedOut => {
                a.view = None;
                a.preview = None;
                a.made = None;
            }
            _ => continue,
        }
        a.changed = true;
    }
}

/// Pushes the table into Lua, fires what happened since the last frame and sends the window's
/// choices. Gated on the in-game UI; a new VM gets the open table again.
fn feed_alchemy(
    script: Option<NonSendMut<UiScript>>,
    mut alchemy: ResMut<Alchemy>,
    mut outbox: ResMut<AlchemyOutbox>,
    mut pushed: Local<crate::ui_script::VmMemo<()>>,
) {
    let Some(mut script) = script else {
        return;
    };
    let (_, new_vm) = pushed.get_reset(&script);
    if new_vm || alchemy.changed {
        alchemy.changed = false;
        script.set_alchemy(alchemy.view.clone());
        script.set_alchemy_preview(alchemy.preview.clone());
        script.set_alchemy_made(alchemy.made.clone());
    }
    for call in script.take_alchemy_calls() {
        outbox.0.push(command_of(call));
    }
    for name in std::mem::take(&mut alchemy.events) {
        script.fire_event(name, Vec::new());
    }
}

pub(crate) struct AlchemyPlugin;

impl Plugin for AlchemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AlchemyInbox>()
            .init_resource::<AlchemyOutbox>()
            .init_resource::<Alchemy>()
            .add_systems(
                Update,
                (
                    run_alchemy,
                    feed_alchemy.run_if(crate::ui_script::ingame_ui_up),
                )
                    .chain()
                    .in_set(UiFeed),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use benilla_protocol::{
        AlchemyGrade, AlchemyMakeRow, AlchemyMaterial, AlchemyRumour, AlchemyValue,
    };

    const PEACEBLOOM: u32 = 2447;

    fn bench_event() -> SessionEvent {
        SessionEvent::AlchemyBench {
            npc: 7,
            table: "Goldshire".into(),
            alchemist: "Alwen Carrow".into(),
            line: "Dirty herbs are a gamble.".into(),
            tiers: vec!["Minor".into(), "Lesser".into()],
            makes: vec![AlchemyMakeRow {
                key: "potion".into(),
                name: "Potion".into(),
                station: "Mortar".into(),
                verb: "Brew".into(),
                rule: "2 to 4 herbs".into(),
                open: true,
                herbs: [2, 4],
                tinctures: 0,
                essences: 0,
                fish: 0,
                wash: 0,
            }],
            tray: vec![AlchemyMaterial {
                item: PEACEBLOOM,
                count: 7,
                kind: 0,
                level: 1,
                values: vec![
                    AlchemyValue {
                        name: "Restorative".into(),
                        points: Some(2),
                    },
                    AlchemyValue {
                        name: "Toxic".into(),
                        points: None,
                    },
                ],
                prop: String::new(),
                points: 0.0,
                toxic: 0,
                step: 0.0,
            }],
            targets: vec![AlchemyGrade {
                family: "potion".into(),
                key: "Restorative".into(),
                grade: 0,
                name: "Minor Healing Potion".into(),
                learned: true,
                rumoured: false,
                threshold: 4,
            }],
            rumours: vec![AlchemyRumour {
                family: "potion".into(),
                key: "Restorative".into(),
                grade: 1,
                name: "Lesser Healing Potion".into(),
                price: 6000,
            }],
            sludge: 3,
        }
    }

    #[test]
    fn an_alchemy_bench_event_becomes_the_view_field_for_field() {
        let view = view_of(&bench_event()).unwrap();
        assert_eq!(
            (
                view.table.as_str(),
                view.alchemist.as_str(),
                view.line.as_str()
            ),
            ("Goldshire", "Alwen Carrow", "Dirty herbs are a gamble.")
        );
        assert_eq!(view.tiers, ["Minor", "Lesser"]);
        assert_eq!(
            view.makes,
            [AlchemyMakeView {
                key: "potion".into(),
                name: "Potion".into(),
                station: "Mortar".into(),
                verb: "Brew".into(),
                rule: "2 to 4 herbs".into(),
                open: true,
                herbs: [2, 4],
                tinctures: 0,
                essences: 0,
                fish: 0,
                wash: 0,
            }]
        );
        assert_eq!(
            view.tray,
            [AlchemyMaterialView {
                item: PEACEBLOOM,
                count: 7,
                kind: 0,
                level: 1,
                values: vec![("Restorative".into(), Some(2)), ("Toxic".into(), None)],
                prop: String::new(),
                points: 0.0,
                toxic: 0,
                step: 0.0,
            }]
        );
        assert_eq!(
            view.targets,
            [AlchemyGradeView {
                family: "potion".into(),
                key: "Restorative".into(),
                grade: 0,
                name: "Minor Healing Potion".into(),
                learned: true,
                rumoured: false,
                threshold: 4,
            }]
        );
        assert_eq!(
            view.rumours,
            [AlchemyRumourView {
                family: "potion".into(),
                key: "Restorative".into(),
                grade: 1,
                name: "Lesser Healing Potion".into(),
                price: 6000,
            }]
        );
        assert_eq!(view.sludge, 3);
        assert!(view_of(&SessionEvent::LoggedOut).is_none());
    }

    #[test]
    fn the_preview_and_made_events_become_their_views() {
        let vial = AlchemyVial {
            prop: "Restorative".into(),
            value: 4.0,
            unknown: 0,
            main: true,
            target: false,
            bonus: 1.0,
            tick: 4,
        };
        let toxic = AlchemyToxic {
            raw: 2,
            purify: 1,
            net: 1,
            unknown: false,
            fail_pct: 17,
            bonus: 1.0,
        };
        let p = preview_of(&SessionEvent::AlchemyPreview {
            ready: true,
            note: String::new(),
            vials: vec![vial.clone()],
            toxic: toxic.clone(),
            unnoted: 2,
            scale: 24.0,
            rings: [4, 12, 24, 40],
            hybrid: false,
            known: true,
            product: 118,
            sludge: false,
            carried: 0,
        })
        .unwrap();
        assert!(p.ready);
        assert_eq!((p.unnoted, p.rings, p.product), (2, [4, 12, 24, 40], 118));
        assert_eq!(
            p.vials,
            [AlchemyVialView {
                prop: "Restorative".into(),
                value: 4.0,
                unknown: 0,
                main: true,
                target: false,
                bonus: 1.0,
                tick: 4,
            }]
        );
        assert_eq!(
            p.toxic,
            AlchemyToxicView {
                raw: 2,
                purify: 1,
                net: 1,
                unknown: false,
                fail_pct: 17,
                bonus: 1.0,
            }
        );
        let m = made_of(&SessionEvent::AlchemyMade {
            product: 118,
            failed: false,
            fail_pct: 17,
            vials: vec![vial],
            toxic,
            scale: 24.0,
            rings: [4, 12, 24, 40],
            lines: vec![(3, "Minor Healing Potion x1, into your bags.".into())],
        })
        .unwrap();
        assert_eq!((m.product, m.failed, m.fail_pct), (118, false, 17));
        assert_eq!(m.lines.len(), 1);
        assert!(preview_of(&SessionEvent::AlchemyClosed).is_none());
        assert!(made_of(&SessionEvent::AlchemyClosed).is_none());
    }

    #[test]
    fn every_queued_call_becomes_its_realm_command() {
        let cases = [
            (
                AlchemyCall::Preview {
                    make: "potion".into(),
                    items: vec![PEACEBLOOM],
                    target: String::new(),
                },
                AlchemyAction::Preview {
                    make: "potion".into(),
                    items: vec![PEACEBLOOM],
                    target: String::new(),
                },
            ),
            (
                AlchemyCall::Make {
                    make: "potion".into(),
                    items: vec![PEACEBLOOM, PEACEBLOOM],
                    target: "potion|Restorative|Minor".into(),
                },
                AlchemyAction::Make {
                    make: "potion".into(),
                    items: vec![PEACEBLOOM, PEACEBLOOM],
                    target: "potion|Restorative|Minor".into(),
                },
            ),
            (
                AlchemyCall::BuyRumour {
                    family: "potion".into(),
                    key: "Restorative".into(),
                    grade: 1,
                },
                AlchemyAction::BuyRumour {
                    family: "potion".into(),
                    key: "Restorative".into(),
                    grade: 1,
                },
            ),
            (AlchemyCall::SellSludge, AlchemyAction::SellSludge),
            (AlchemyCall::Close, AlchemyAction::Close),
        ];
        for (call, action) in cases {
            assert!(matches!(command_of(call), RealmCommand::Alchemy(a) if a == action));
        }
    }

    #[test]
    fn the_feed_pushes_the_table_and_sends_the_choices() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.init_resource::<AlchemyOutbox>();
        world.init_resource::<Alchemy>();
        world.insert_resource(AlchemyInbox(vec![bench_event()]));
        world.insert_non_send_resource(UiScript::new().unwrap());
        world.run_system_once(run_alchemy).unwrap();
        world.run_system_once(feed_alchemy).unwrap();
        let script = world.non_send_resource::<UiScript>();
        let said = |lua: &str| script.eval::<String>(lua).unwrap();
        assert_eq!(
            said(
                "local t, a, _, m, tr, tg, r, s = GetAlchemyBench() \
                 return t .. a .. m .. tr .. tg .. r .. s"
            ),
            "GoldshireAlwen Carrow11113"
        );
        assert_eq!(
            said("local n, p = GetAlchemyTrayValue(1, 1) return n .. p"),
            "Restorative2"
        );
        assert_eq!(
            said("local _, p = GetAlchemyTrayValue(1, 2) return tostring(p)"),
            "nil"
        );
        world
            .non_send_resource_mut::<UiScript>()
            .run("AlchemyPreview('potion', nil, 2447) AlchemySellSludge() AlchemyClose()")
            .unwrap();
        world.run_system_once(feed_alchemy).unwrap();
        let sent = &world.resource::<AlchemyOutbox>().0;
        assert_eq!(sent.len(), 3);
        assert!(matches!(
            &sent[0],
            RealmCommand::Alchemy(AlchemyAction::Preview { make, items, target })
                if make == "potion" && items == &[PEACEBLOOM] && target.is_empty()
        ));
        assert!(matches!(
            sent[1],
            RealmCommand::Alchemy(AlchemyAction::SellSludge)
        ));
        assert!(matches!(
            sent[2],
            RealmCommand::Alchemy(AlchemyAction::Close)
        ));
    }
}
