//! The Kitchen window's host (ADR 0025, spec #447, `assets/ui/Kitchen.xml`): the realm's
//! `Kitchen`, `KitchenPrediction` and `KitchenCooked` become the views Lua reads,
//! `KitchenClosed` hides the window, and every `Kitchen*` call the window queued goes back as a
//! `RealmCommand::Kitchen`. The client computes no result.

use bevy::prelude::*;

use benilla_protocol::SessionEvent;
use benilla_ui::script::{
    KitchenCall, KitchenCookedView, KitchenIngredientView, KitchenMode, KitchenOrderView,
    KitchenPredictionView, KitchenRecipeView, KitchenView, UiScript,
};
use twilight_realm::{KitchenAction, RealmCommand};

use crate::ui_script::UiFeed;

/// The realm's kitchen events, which the Twilight pump hands over instead of the net layer.
#[derive(Resource, Default)]
pub(crate) struct KitchenInbox(pub(crate) Vec<SessionEvent>);

/// The window's choices, which the Twilight pump sends to the realm.
#[derive(Resource, Default)]
pub(crate) struct KitchenOutbox(pub(crate) Vec<RealmCommand>);

/// The open kitchen, and what the feed owes Lua.
#[derive(Resource, Default)]
struct Kitchen {
    /// The last kitchen the realm sent, or `None` once it closed.
    view: Option<KitchenView>,
    prediction: Option<KitchenPredictionView>,
    cooked: Option<KitchenCookedView>,
    /// Any of the three changed since the last push.
    changed: bool,
    /// Events for Lua, oldest first.
    events: Vec<&'static str>,
}

/// The view of a `Kitchen` event, field for field; `None` for any other event.
fn view_of(event: &SessionEvent) -> Option<KitchenView> {
    let SessionEvent::Kitchen {
        name,
        cook,
        book,
        pantry,
        lessons,
        orders,
        cuttable,
        ..
    } = event
    else {
        return None;
    };
    Some(KitchenView {
        name: name.clone(),
        cook: cook.clone(),
        book: book
            .iter()
            .map(|r| KitchenRecipeView {
                key: r.key.clone(),
                kind: r.kind,
                effect: r.effect.clone(),
                family: r.family.clone(),
                band: r.band,
                known: r.known,
                name: r.name.clone(),
                meal: r.meal,
                slots: r.slots.clone(),
            })
            .collect(),
        pantry: pantry
            .iter()
            .map(|p| KitchenIngredientView {
                item: p.item,
                count: p.count,
                level: p.level,
                category: p.category.clone(),
                untried: p.untried,
            })
            .collect(),
        lessons: *lessons,
        orders: orders
            .iter()
            .map(|o| KitchenOrderView {
                label: o.label.clone(),
                kind: o.kind,
                copper: o.copper,
                qty: o.qty,
                have: o.have,
                done: o.done,
                can_deliver: o.can_deliver,
                bonus: o.bonus.clone(),
                bonus_rare: o.bonus_rare,
            })
            .collect(),
        cuttable: cuttable.clone(),
    })
}

/// The view of a `KitchenPrediction` event; `None` for any other event.
fn prediction_of(event: &SessionEvent) -> Option<KitchenPredictionView> {
    let SessionEvent::KitchenPrediction {
        ready,
        meal,
        note,
        why,
    } = event
    else {
        return None;
    };
    Some(KitchenPredictionView {
        ready: *ready,
        meal: *meal,
        note: note.clone(),
        why: why.clone(),
    })
}

/// The view of a `KitchenCooked` event; `None` for any other event.
fn cooked_of(event: &SessionEvent) -> Option<KitchenCookedView> {
    let SessionEvent::KitchenCooked {
        meal,
        mush,
        close,
        learned,
        revealed,
        line,
    } = event
    else {
        return None;
    };
    Some(KitchenCookedView {
        meal: *meal,
        mush: *mush,
        close: *close,
        learned: learned.clone(),
        revealed: revealed.clone(),
        line: line.clone(),
    })
}

fn mode_of(mode: KitchenMode) -> twilight_realm::KitchenMode {
    match mode {
        KitchenMode::Recipe => twilight_realm::KitchenMode::Recipe,
        KitchenMode::Discover => twilight_realm::KitchenMode::Discover,
        KitchenMode::Experiment => twilight_realm::KitchenMode::Experiment,
    }
}

/// The realm command a queued call stands for.
fn command_of(call: KitchenCall) -> RealmCommand {
    RealmCommand::Kitchen(match call {
        KitchenCall::Predict {
            mode,
            recipe,
            slots,
        } => KitchenAction::Predict {
            mode: mode_of(mode),
            recipe,
            slots,
        },
        KitchenCall::Cook {
            mode,
            recipe,
            slots,
        } => KitchenAction::Cook {
            mode: mode_of(mode),
            recipe,
            slots,
        },
        KitchenCall::Deliver(i) => KitchenAction::Deliver(i),
        KitchenCall::Cut(fish) => KitchenAction::Cut(fish),
        KitchenCall::Close => KitchenAction::Close,
    })
}

/// Reads the realm's kitchen events.
fn run_kitchen(mut inbox: ResMut<KitchenInbox>, mut kitchen: ResMut<Kitchen>) {
    let k = &mut *kitchen;
    for event in inbox.0.drain(..) {
        match &event {
            SessionEvent::Kitchen { .. } => {
                k.view = view_of(&event);
                k.events.push("KITCHEN_UPDATE");
            }
            SessionEvent::KitchenPrediction { .. } => {
                k.prediction = prediction_of(&event);
                k.events.push("KITCHEN_PREDICTION");
            }
            SessionEvent::KitchenCooked { .. } => {
                k.cooked = cooked_of(&event);
                k.events.push("KITCHEN_COOKED");
            }
            SessionEvent::KitchenClosed => {
                k.view = None;
                k.prediction = None;
                k.cooked = None;
                k.events.push("KITCHEN_CLOSED");
            }
            SessionEvent::LoggedOut => {
                k.view = None;
                k.prediction = None;
                k.cooked = None;
            }
            _ => continue,
        }
        k.changed = true;
    }
}

/// Pushes the kitchen into Lua, fires what happened since the last frame and sends the window's
/// choices. Gated on the in-game UI; a new VM gets the open kitchen again.
fn feed_kitchen(
    script: Option<NonSendMut<UiScript>>,
    mut kitchen: ResMut<Kitchen>,
    mut outbox: ResMut<KitchenOutbox>,
    mut pushed: Local<crate::ui_script::VmMemo<()>>,
) {
    let Some(mut script) = script else {
        return;
    };
    let (_, new_vm) = pushed.get_reset(&script);
    if new_vm || kitchen.changed {
        kitchen.changed = false;
        script.set_kitchen(kitchen.view.clone());
        script.set_kitchen_prediction(kitchen.prediction.clone());
        script.set_kitchen_cooked(kitchen.cooked.clone());
    }
    for call in script.take_kitchen_calls() {
        outbox.0.push(command_of(call));
    }
    for name in std::mem::take(&mut kitchen.events) {
        script.fire_event(name, Vec::new());
    }
}

pub(crate) struct KitchenPlugin;

impl Plugin for KitchenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KitchenInbox>()
            .init_resource::<KitchenOutbox>()
            .init_resource::<Kitchen>()
            .add_systems(
                Update,
                (
                    run_kitchen,
                    feed_kitchen.run_if(crate::ui_script::ingame_ui_up),
                )
                    .chain()
                    .in_set(UiFeed),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use benilla_protocol::{KitchenIngredient, KitchenOrderRow, KitchenRecipe};

    fn kitchen_event() -> SessionEvent {
        SessionEvent::Kitchen {
            npc: 7,
            name: "Lion's Pride kitchen".into(),
            cook: "Tomas".into(),
            book: vec![KitchenRecipe {
                key: "stamina_10".into(),
                kind: 0,
                effect: "+Stamina".into(),
                family: "Combat Well Fed".into(),
                band: 10,
                known: false,
                name: String::new(),
                meal: 0,
                slots: vec![2447, 0, 0],
            }],
            pantry: vec![KitchenIngredient {
                item: 960101,
                count: 3,
                level: 12,
                category: "fish".into(),
                untried: true,
            }],
            lessons: [true, false, false, false, false, false],
            orders: vec![KitchenOrderRow {
                label: "3 Goldshire Perch".into(),
                kind: 0,
                copper: 84,
                qty: 3,
                have: 2,
                done: false,
                can_deliver: false,
                bonus: "a recipe Rumour".into(),
                bonus_rare: true,
            }],
            cuttable: vec![(960101, 5, 960502)],
        }
    }

    #[test]
    fn a_kitchen_event_becomes_the_view_field_for_field() {
        let view = view_of(&kitchen_event()).unwrap();
        assert_eq!(
            (view.name.as_str(), view.cook.as_str()),
            ("Lion's Pride kitchen", "Tomas")
        );
        assert_eq!(
            view.book,
            [KitchenRecipeView {
                key: "stamina_10".into(),
                kind: 0,
                effect: "+Stamina".into(),
                family: "Combat Well Fed".into(),
                band: 10,
                known: false,
                name: String::new(),
                meal: 0,
                slots: vec![2447, 0, 0],
            }]
        );
        assert_eq!(
            view.pantry,
            [KitchenIngredientView {
                item: 960101,
                count: 3,
                level: 12,
                category: "fish".into(),
                untried: true,
            }]
        );
        assert_eq!(view.lessons, [true, false, false, false, false, false]);
        assert_eq!(
            view.orders,
            [KitchenOrderView {
                label: "3 Goldshire Perch".into(),
                kind: 0,
                copper: 84,
                qty: 3,
                have: 2,
                done: false,
                can_deliver: false,
                bonus: "a recipe Rumour".into(),
                bonus_rare: true,
            }]
        );
        assert_eq!(view.cuttable, [(960101, 5, 960502)]);
        assert!(view_of(&SessionEvent::LoggedOut).is_none());
    }

    #[test]
    fn the_prediction_and_cooked_events_become_their_views() {
        let p = prediction_of(&SessionEvent::KitchenPrediction {
            ready: true,
            meal: 961001,
            note: String::new(),
            why: vec!["An exact match.".into()],
        })
        .unwrap();
        assert!(p.ready);
        assert_eq!((p.meal, p.why.len()), (961001, 1));
        let c = cooked_of(&SessionEvent::KitchenCooked {
            meal: 961001,
            mush: false,
            close: true,
            learned: vec!["vigil".into()],
            revealed: vec![(2447, 3)],
            line: "Well. It's food.".into(),
        })
        .unwrap();
        assert_eq!(
            c,
            KitchenCookedView {
                meal: 961001,
                mush: false,
                close: true,
                learned: vec!["vigil".into()],
                revealed: vec![(2447, 3)],
                line: "Well. It's food.".into(),
            }
        );
        assert!(prediction_of(&SessionEvent::KitchenClosed).is_none());
        assert!(cooked_of(&SessionEvent::KitchenClosed).is_none());
    }

    #[test]
    fn every_queued_call_becomes_its_realm_command() {
        let cases = [
            (
                KitchenCall::Predict {
                    mode: KitchenMode::Experiment,
                    recipe: String::new(),
                    slots: vec![2447, 0],
                },
                KitchenAction::Predict {
                    mode: twilight_realm::KitchenMode::Experiment,
                    recipe: String::new(),
                    slots: vec![2447, 0],
                },
            ),
            (
                KitchenCall::Cook {
                    mode: KitchenMode::Recipe,
                    recipe: "stew".into(),
                    slots: vec![1, 2, 3],
                },
                KitchenAction::Cook {
                    mode: twilight_realm::KitchenMode::Recipe,
                    recipe: "stew".into(),
                    slots: vec![1, 2, 3],
                },
            ),
            (
                KitchenCall::Predict {
                    mode: KitchenMode::Discover,
                    recipe: "vigil".into(),
                    slots: vec![4],
                },
                KitchenAction::Predict {
                    mode: twilight_realm::KitchenMode::Discover,
                    recipe: "vigil".into(),
                    slots: vec![4],
                },
            ),
            (KitchenCall::Deliver(0), KitchenAction::Deliver(0)),
            (KitchenCall::Cut(960101), KitchenAction::Cut(960101)),
            (KitchenCall::Close, KitchenAction::Close),
        ];
        for (call, action) in cases {
            assert!(matches!(command_of(call), RealmCommand::Kitchen(a) if a == action));
        }
    }

    #[test]
    fn the_feed_pushes_the_kitchen_and_sends_the_choices() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.init_resource::<KitchenOutbox>();
        world.init_resource::<Kitchen>();
        world.insert_resource(KitchenInbox(vec![
            kitchen_event(),
            SessionEvent::KitchenPrediction {
                ready: false,
                meal: 0,
                note: "Fill every slot first.".into(),
                why: Vec::new(),
            },
        ]));
        world.insert_non_send_resource(UiScript::new().unwrap());
        world.run_system_once(run_kitchen).unwrap();
        world.run_system_once(feed_kitchen).unwrap();
        let script = world.non_send_resource::<UiScript>();
        let said = |lua: &str| script.eval::<String>(lua).unwrap();
        assert_eq!(
            said("local n, c, b, p, o, k = GetKitchen() return n .. c .. b .. p .. o .. k"),
            "Lion's Pride kitchenTomas1111"
        );
        assert_eq!(
            said("local _, _, note = GetKitchenPrediction() return note"),
            "Fill every slot first."
        );
        assert_eq!(
            said("return GetKitchenRecipeSlot(1, 1) .. GetKitchenRecipeSlot(1, 2)"),
            "24470"
        );
        world
            .non_send_resource_mut::<UiScript>()
            .run("KitchenDeliver(1) KitchenClose()")
            .unwrap();
        world.run_system_once(feed_kitchen).unwrap();
        let sent = &world.resource::<KitchenOutbox>().0;
        assert_eq!(sent.len(), 2);
        assert!(matches!(
            sent[0],
            RealmCommand::Kitchen(KitchenAction::Deliver(0))
        ));
        assert!(matches!(
            sent[1],
            RealmCommand::Kitchen(KitchenAction::Close)
        ));
    }
}
