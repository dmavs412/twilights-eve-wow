//! Our cart seat (spec #560): from `SessionEvent::Seated` to `Unseated` the wagon's frame times the
//! seat's pose is our pose, we sit, and the camera follows as usual. A movement key or a click on
//! the world sends one movement report in the wagon's frame, which the realm reads as a wish to
//! stand up; a right-click on the wagon sends its use through the ordinary interact path. Nothing
//! else goes out until `Unseated` sets us down beside the wagon and hands control back, the way a
//! ride ends with `CMSG_MOVE_SPLINE_DONE` ([`super::server_ride`]).

use benilla_assets::coords::{bevy_to_wow, wow_to_bevy};
use benilla_protocol::{SessionEvent, TransportPose};
use benilla_world::interact::WorldClick;
use bevy::prelude::*;

use crate::bindings::{BindingsState, Input};
use crate::creature_anim::{move_flags, MovementState};
use crate::net::{ClientCommand, Embodied, MoveKind, NetCommands};
use crate::transport::{deck_pose, Transport};

use super::state::PlayerSeat;
use super::Player;

/// `UNIT_STAND_STATE_SIT`, the posture a seat holds.
const SITTING: u8 = 1;

/// The realm's `Seated` and `Unseated`, queued by the Twilight session pump for [`drive_seat`].
#[derive(Resource, Default)]
pub(crate) struct SeatInbox(pub(crate) Vec<SessionEvent>);

/// The keys that ask a seated rider to stand up: any that would move or turn them.
const STAND_UP_KEYS: [Input; 8] = [
    Input::MoveForward,
    Input::MoveBackward,
    Input::StrafeLeft,
    Input::StrafeRight,
    Input::TurnLeft,
    Input::TurnRight,
    Input::Jump,
    Input::ToggleAutoRun,
];

/// Seat and unseat from the inbox, pose a seated body at its seat and send the one report that
/// asks to stand up. Runs just before `control`, which yields while [`Player::seat`] is set.
#[allow(clippy::type_complexity)] // a Bevy query's component tuple
pub(super) fn drive_seat(
    net: Res<NetCommands>,
    mut inbox: ResMut<SeatInbox>,
    mut player: ResMut<Player>,
    binds: Res<BindingsState>,
    mut clicks: MessageReader<WorldClick>,
    guid_index: Res<crate::net::GuidIndex>,
    wagons: Query<&Transform, (With<Transport>, Without<Embodied>)>,
    mut body: Query<(&mut Transform, Option<&mut MovementState>), With<Embodied>>,
) {
    let clicked = clicks.read().count() > 0;
    for event in inbox.0.drain(..) {
        match event {
            SessionEvent::Seated { wagon_guid, pose } => sit(&mut player, wagon_guid, &pose),
            SessionEvent::Unseated { position } => {
                stand_up(&mut player, position);
                if let Ok((mut transform, _)) = body.single_mut() {
                    transform.translation = player.pos;
                }
            }
            _ => {}
        }
    }
    let Some(seat) = player.seat else {
        return;
    };
    // A wagon not streamed in keeps our last pose; the realm stands us up if it goes.
    if let Some(wagon) = guid_index
        .0
        .get(&seat.wagon_guid)
        .and_then(|&e| wagons.get(e).ok())
    {
        let (world, yaw) = deck_pose(wagon, seat.local_pos, seat.local_orientation);
        player.pos = world;
        player.face_yaw = yaw;
        player.model_yaw = yaw;
    }
    if let Ok((mut transform, motion)) = body.single_mut() {
        transform.translation = player.pos;
        transform.rotation = Quat::from_rotation_y(player.model_yaw);
        if let Some(mut motion) = motion {
            motion.speed = 0.0;
            motion.vertical_speed = 0.0;
            motion.flags = 0;
            motion.stand_state = SITTING;
        }
    }
    let wants_up = clicked || STAND_UP_KEYS.iter().any(|&i| binds.just_pressed(i));
    if wants_up && !seat.asked {
        let _ = net.0.send(seated_report(&player, &seat));
        if let Some(s) = player.seat.as_mut() {
            s.asked = true;
        }
    }
}

/// `Seated`: the seat owns the pose from now on. The flags drop silently: the realm stands a rider
/// up at any move, so the stop the controller would send must not go out.
fn sit(player: &mut Player, wagon_guid: u64, pose: &TransportPose) {
    info!("cart: seated in wagon {wagon_guid:#x}");
    player.seat = Some(PlayerSeat {
        wagon_guid,
        local_pos: [pose.pos.x, pose.pos.y, pose.pos.z],
        local_orientation: pose.orientation,
        asked: false,
    });
    player.ride = None;
    player.autorun = false;
    player.move_flags = 0;
    player.airborne_since = None;
    player.vel_y = 0.0;
    player.horiz_vel = Vec3::ZERO;
}

/// `Unseated`: set down at `position`, standing, at rest, with control back.
fn stand_up(player: &mut Player, position: [f32; 3]) {
    info!(
        "cart: stood up at [{:.1}, {:.1}, {:.1}]",
        position[0], position[1], position[2]
    );
    player.seat = None;
    player.pos = wow_to_bevy(position);
    player.last_pos = position;
    player.move_flags = 0;
    player.airborne_since = None;
    player.vel_y = 0.0;
    player.horiz_vel = Vec3::ZERO;
}

/// The one movement report a seated rider sends: in the wagon's frame, so the realm and every
/// observer keep them in the wagon until the realm stands them up.
fn seated_report(player: &Player, seat: &PlayerSeat) -> ClientCommand {
    ClientCommand::Move {
        kind: MoveKind::Heartbeat,
        flags: move_flags::ON_TRANSPORT,
        pos: bevy_to_wow(player.pos),
        orientation: player.face_yaw.rem_euclid(std::f32::consts::TAU),
        pitch: 0.0,
        fall_time: 0,
        jump: None,
        transport: Some(TransportPose {
            guid: seat.wagon_guid,
            pos: benilla_protocol::wire::Vector3d {
                x: seat.local_pos[0],
                y: seat.local_pos[1],
                z: seat.local_pos[2],
            },
            orientation: seat.local_orientation,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    const WAGON: u64 = 0xF120_0000_0000_0001;
    /// A seat 1 yd back and 0.5 yd left of the wagon's centre, 1 yd up, facing forward.
    const SEAT: [f32; 3] = [-1.0, 0.5, 1.0];

    fn seat_pose() -> TransportPose {
        TransportPose {
            guid: WAGON,
            pos: benilla_protocol::wire::Vector3d {
                x: SEAT[0],
                y: SEAT[1],
                z: SEAT[2],
            },
            orientation: 0.0,
        }
    }

    /// One `drive_seat` app with a wagon at WoW `[100, 200, 50]` facing `yaw`, and our body.
    fn app(yaw: f32) -> (App, Entity, crossbeam_channel::Receiver<ClientCommand>) {
        let mut app = App::new();
        app.add_message::<WorldClick>()
            .init_resource::<BindingsState>()
            .init_resource::<SeatInbox>()
            .insert_resource(Player {
                active: true,
                ..Default::default()
            })
            .add_systems(Update, drive_seat);
        let (tx, rx) = crossbeam_channel::unbounded();
        app.insert_resource(NetCommands(tx));
        let wagon = app
            .world_mut()
            .spawn((
                Transform::from_translation(wow_to_bevy([100.0, 200.0, 50.0]))
                    .with_rotation(Quat::from_rotation_y(yaw)),
                crate::transport::test_road_transport(),
            ))
            .id();
        let mut index = crate::net::GuidIndex::default();
        index.0.insert(WAGON, wagon);
        app.insert_resource(index);
        let body = app
            .world_mut()
            .spawn((Transform::default(), MovementState::default(), Embodied))
            .id();
        app.world_mut()
            .resource_mut::<SeatInbox>()
            .0
            .push(SessionEvent::Seated {
                wagon_guid: WAGON,
                pose: seat_pose(),
            });
        (app, body, rx)
    }

    #[track_caller]
    fn assert_at(app: &App, body: Entity, want_wow: [f32; 3], want_yaw: f32) {
        let player = app.world().resource::<Player>();
        let got = bevy_to_wow(player.pos);
        let off = (0..3)
            .map(|k| (got[k] - want_wow[k]).abs())
            .fold(0.0, f32::max);
        assert!(off < 1e-3, "expected {want_wow:?}, got {got:?}");
        assert!((player.face_yaw - want_yaw).abs() < 1e-4);
        let t = app.world().get::<Transform>(body).unwrap();
        assert!(
            (t.translation - player.pos).length() < 1e-4,
            "the body is posed too"
        );
    }

    #[test]
    fn a_seat_in_a_wagon_at_yaw_0_is_the_wagon_plus_the_seat() {
        let (mut app, body, rx) = app(0.0);
        app.update();
        assert_at(&app, body, [99.0, 200.5, 51.0], 0.0);
        let motion = app.world().get::<MovementState>(body).unwrap();
        assert_eq!(motion.stand_state, SITTING, "a seat sits");
        assert!(rx.try_recv().is_err(), "sitting sends nothing");
    }

    /// Turned a quarter west, the wagon's forward is WoW +y and its left is WoW -x.
    #[test]
    fn a_seat_in_a_wagon_at_yaw_pi_over_2_turns_with_it() {
        let (mut app, body, _rx) = app(FRAC_PI_2);
        app.update();
        assert_at(&app, body, [99.5, 199.0, 51.0], FRAC_PI_2);
    }

    #[test]
    fn unseated_ends_the_seat_and_sets_the_pose() {
        let (mut app, body, rx) = app(0.0);
        app.update();
        app.world_mut()
            .resource_mut::<SeatInbox>()
            .0
            .push(SessionEvent::Unseated {
                position: [101.0, 198.0, 49.0],
            });
        app.update();
        let player = app.world().resource::<Player>();
        assert!(player.seat.is_none(), "control is ours again");
        assert_eq!(player.movement_suppressors(), "none");
        assert_at(&app, body, [101.0, 198.0, 49.0], 0.0);
        assert!(rx.try_recv().is_err(), "standing up owes no report");
    }

    #[test]
    fn a_movement_key_sends_one_report_in_the_wagons_frame() {
        let (mut app, _body, rx) = app(0.0);
        app.update();
        for _ in 0..3 {
            app.world_mut()
                .resource_mut::<BindingsState>()
                .press_for_test(Input::MoveForward);
            app.update();
        }
        let sent: Vec<_> = rx.try_iter().collect();
        assert_eq!(sent.len(), 1, "one report, then nothing until Unseated");
        match &sent[0] {
            ClientCommand::Move {
                flags, transport, ..
            } => {
                assert_ne!(flags & move_flags::ON_TRANSPORT, 0);
                let t = transport.expect("the transport tail");
                assert_eq!(t.guid, WAGON);
                assert_eq!([t.pos.x, t.pos.y, t.pos.z], SEAT);
            }
            _ => panic!("expected a movement report"),
        }
    }
}
