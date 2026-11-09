//! The Milestones reached (ADR 0034), from the realm's `SessionEvent::Milestones`: whole at login,
//! before the character's world, and again whenever one is reached. The set goes to
//! [`ReachedMilestones`], which the ADT loader cuts tiles by; a set that adds keys also writes
//! [`ZonesOpened`], which loads the opened land and fades the mist on its border.

use benilla_assets::ReachedMilestones;
use benilla_formats::closed_zones::Reached;
use benilla_protocol::SessionEvent;
use benilla_world::mist::ZonesOpened;
use bevy::prelude::*;

/// The realm's `SessionEvent::Milestones`, as [`super::pump`] drains them.
#[derive(Resource, Default)]
pub(crate) struct MilestonesInbox(pub(crate) Vec<SessionEvent>);

/// Sets [`ReachedMilestones`] to each event's set and, when it adds keys, writes [`ZonesOpened`].
/// At login nothing is loaded yet, so the first tiles load open and nothing fades.
pub(crate) fn apply_milestones(
    mut inbox: ResMut<MilestonesInbox>,
    reached: Option<Res<ReachedMilestones>>,
    mut opened: MessageWriter<ZonesOpened>,
) {
    for event in inbox.0.drain(..) {
        let SessionEvent::Milestones { reached: keys } = event else {
            continue;
        };
        // No game data, so no loader and no world to open.
        let Some(reached) = reached.as_deref() else {
            continue;
        };
        let now = Reached::new(keys);
        let before = reached.get();
        let added = reached.set(now.clone());
        if !added.is_empty() {
            info!("twilight: Milestones reached {added:?}, opening their land");
            opened.write(ZonesOpened { before, now });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::message::Messages;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<MilestonesInbox>()
            .init_resource::<ReachedMilestones>()
            .add_message::<ZonesOpened>()
            .add_systems(Update, apply_milestones);
        app
    }

    fn send(app: &mut App, keys: &[&str]) -> Vec<ZonesOpened> {
        app.world_mut()
            .resource_mut::<MilestonesInbox>()
            .0
            .push(SessionEvent::Milestones {
                reached: keys.iter().map(|k| k.to_string()).collect(),
            });
        app.update();
        app.world_mut()
            .resource_mut::<Messages<ZonesOpened>>()
            .drain()
            .collect()
    }

    #[test]
    fn milestones_set_the_reached_set_and_only_new_keys_start_a_fade() {
        let mut app = app();
        let first = send(&mut app, &["steppes_open"]);
        let reached = app.world().resource::<ReachedMilestones>().get();
        assert!(reached.contains("steppes_open"));
        assert_eq!(first.len(), 1, "the key is new: the zone opens");
        assert_eq!(first[0].before, Reached::default());
        assert_eq!(first[0].now, Reached::new(["steppes_open"]));
        assert!(
            send(&mut app, &["steppes_open"]).is_empty(),
            "the same set again opens nothing and starts no fade"
        );
    }
}
