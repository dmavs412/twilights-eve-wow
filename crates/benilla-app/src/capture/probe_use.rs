//! The use probe (`WOW_PROBE_USE=<x>,<y>,<z>[,<map>]`): an unattended run cannot click a
//! GameObject, so this one goes to a point and uses the clickable GameObject there. It is how a
//! probe rings the Deeprun Tram's station bell (#594):
//! `WOW_PROBE_USE=-18.5,2484.0,-4.29,369`.
//!
//! 1. It sends `.go xyz` to the point and waits [`SETTLE_SECS`] for the world to stream in.
//! 2. It takes the nearest streamed `GOOBER` (type 10) within [`USE_RANGE`] of the point and uses
//!    it through [`crate::target::click::resolve_go_action`], sending the arm it names, as
//!    `probe_chest.rs` does for a chest.
//! 3. It logs `PROBE_USE: used <guid>` or `PROBE_USE: none within 5 yd`, then does nothing more.
//!    It never exits, so `WOW_CAPTURE` or another probe can watch what follows.
//!
//! Grep `PROBE_USE:` for the outcome. `WOW_PROBE=list` lists the variable.

use bevy::prelude::*;

use benilla_assets::coords::wow_to_bevy;
use benilla_protocol::EntityKind;

use super::probes::ProbeClock;
use crate::net::{ChatKind, ClientCommand, Guid, NetCommands, NetEntity, ObjectStore, SelfPlayer};
use crate::target::click::{resolve_go_action, GoAction};
use crate::target::lock::GoLockInputs;

/// The map a three-number value lands on.
const DEFAULT_MAP: f32 = 0.0;
/// `GAMEOBJECT_TYPE_GOOBER`: a lever, a button, a bell.
const GO_TYPE_GOOBER: i32 = 10;
/// How far from the point the used object may stand, in yards.
const USE_RANGE: f32 = 5.0;
const SETTLE_SECS: f64 = 5.0;

/// The streamed-object columns the scan and the lock resolve read.
type NearbyObject = (
    &'static Guid,
    &'static NetEntity,
    &'static ObjectStore,
    &'static Transform,
    Option<&'static crate::go_anim::GoAnim>,
);

pub(crate) struct ProbeUsePlugin;

impl Plugin for ProbeUsePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UseProbe>()
            .add_systems(Update, use_probe);
    }
}

#[derive(Resource, Default)]
struct UseProbe {
    phase: Phase,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum Phase {
    #[default]
    Wait,
    /// `.go` issued; letting the world stream the object in.
    Settling { sent_at: f64, at: [f32; 4] },
    /// The use went out, or there was nothing to use. The probe stays idle from here.
    Done,
}

/// The point from a `WOW_PROBE_USE` value: `<x>,<y>,<z>` on map 0, or `<x>,<y>,<z>,<map>`.
/// Anything else, or no variable at all, is `None`, and the probe does nothing.
fn target(raw: Option<&str>) -> Option<[f32; 4]> {
    let parts = raw?
        .split(',')
        .map(|s| s.trim().parse::<f32>().ok())
        .collect::<Option<Vec<f32>>>()?;
    match parts.len() {
        3 => Some([parts[0], parts[1], parts[2], DEFAULT_MAP]),
        4 => Some([parts[0], parts[1], parts[2], parts[3]]),
        _ => None,
    }
}

fn use_probe(
    time: ProbeClock,
    mut probe: ResMut<UseProbe>,
    self_store: Query<&ObjectStore, With<SelfPlayer>>,
    objects: Query<NearbyObject, Without<SelfPlayer>>,
    mut go_inputs: GoLockInputs,
    actions: Res<crate::ui_action::PlayerActions>,
    net: Res<NetCommands>,
) {
    let Ok(me_store) = self_store.single() else {
        return; // not in-world yet
    };
    let now = time.elapsed_secs_f64();

    match probe.phase {
        Phase::Wait => {
            let Some(at) = target(std::env::var("WOW_PROBE_USE").ok().as_deref()) else {
                error!("PROBE_USE: WOW_PROBE_USE is not <x>,<y>,<z>[,<map>]; doing nothing");
                probe.phase = Phase::Done;
                return;
            };
            let [x, y, z, map] = at;
            info!("PROBE_USE: heading to ({x} {y} {z} map {map}) to use the GOOBER there");
            let _ = net.0.send(ClientCommand::Chat {
                kind: ChatKind::Say,
                target: None,
                text: format!(".go xyz {x} {y} {z} {map}"),
                language: None,
            });
            probe.phase = Phase::Settling { sent_at: now, at };
        }
        Phase::Settling { sent_at, at } => {
            if now - sent_at < SETTLE_SECS {
                return;
            }
            probe.phase = Phase::Done;
            let point = wow_to_bevy([at[0], at[1], at[2]]);
            let nearest = objects
                .iter()
                .filter(|(_, net_e, store, ..)| {
                    net_e.kind == EntityKind::GameObject
                        && store.0.gameobject_type_id() == GO_TYPE_GOOBER
                })
                .map(|(guid, _, store, tf, anim)| {
                    (guid.0, store, anim, tf.translation.distance(point))
                })
                .filter(|(.., d)| *d <= USE_RANGE)
                .min_by(|a, b| a.3.total_cmp(&b.3));
            let Some((guid, store, anim, _)) = nearest else {
                info!("PROBE_USE: none within {USE_RANGE} yd");
                return;
            };
            // Send exactly what the right-click's lock chain would send.
            let go = Some((store, crate::go_anim::go_state(anim, store)));
            match resolve_go_action(
                guid,
                &mut go_inputs,
                &actions.spells,
                go,
                Some(me_store),
                &net,
            ) {
                GoAction::Use => {
                    let _ = net.0.send(ClientCommand::GameObjUse { guid });
                    info!("PROBE_USE: used {guid:#x}");
                }
                GoAction::OpenLock(spell_id) => {
                    let _ = net.0.send(ClientCommand::CastSpellGameObject {
                        spell_id,
                        go_guid: guid,
                    });
                    info!("PROBE_USE: used {guid:#x} (opener cast {spell_id})");
                }
                GoAction::OpenByKey { .. } | GoAction::Refuse(_) => {
                    error!(
                        "PROBE_USE: refused {guid:#x}: the lock chain wants a key or refuses \
                         this character"
                    );
                }
            }
        }
        Phase::Done => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_reads_the_point_and_its_map() {
        assert_eq!(
            target(Some("-18.5,2484.0,-4.29,369")),
            Some([-18.5, 2484.0, -4.29, 369.0])
        );
    }

    #[test]
    fn target_puts_three_numbers_on_map_0() {
        assert_eq!(
            target(Some("-18.5, 2484.0, -4.29")),
            Some([-18.5, 2484.0, -4.29, 0.0])
        );
    }

    #[test]
    fn target_is_none_for_anything_else() {
        for raw in ["", "1", "1,2", "1,2,3,4,5", "a,b,c", "1,2,x,4"] {
            assert_eq!(target(Some(raw)), None, "{raw:?}");
        }
        assert_eq!(target(None), None);
    }
}
