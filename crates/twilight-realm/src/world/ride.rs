//! Cart seats (spec #560): a player sits in a standing wagon, rides it as its timetable runs, and
//! stands up beside it at their next move. Seated, no mob holds or sees them, and they cannot act.

use benilla_protocol::{MoverState, SessionEvent, TransportPose};

use super::InWorld;
use crate::carts;

/// A taken seat: the route's wagon (its index in `carts::routes()`) and the seat's index in
/// `seat_offsets`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rider {
    pub(crate) route: usize,
    pub(crate) seat: usize,
    /// A move or a second click asked to stand up: the next tick, which has the host's ground,
    /// sets them down.
    pub(crate) leaving: bool,
}

impl Rider {
    /// The seat's pose in the wagon's frame.
    pub(crate) fn pose(&self) -> TransportPose {
        let route = &carts::routes()[self.route];
        carts::transport_pose(route.wagon_guid(), carts::cart().seat_offsets[self.seat])
    }

    /// Where the seat is in the world, in a wagon at `pose`.
    pub(crate) fn world_at(&self, pose: &carts::Pose) -> [f32; 3] {
        carts::world_at(pose, carts::cart().seat_offsets[self.seat])
    }
}

impl InWorld {
    /// Every mob drops the player from its threat list: an Invisibility Draught's first effect,
    /// and a cart seat's.
    pub(super) fn leave_threat_lists(&mut self) {
        let me = self.guid;
        for mob in &mut self.mobs {
            mob.threat.remove(me);
        }
    }

    /// The player sits in `seat` of route `route`'s wagon: their swing and cast stop, every mob
    /// lets go of them, and the client is told the seat.
    pub(crate) fn sit(&mut self, route: usize, seat: usize, events: &mut Vec<SessionEvent>) {
        self.interrupt_cast(events);
        self.stop_shooting(events);
        self.stop_attack(events);
        self.leave_threat_lists();
        let rider = Rider {
            route,
            seat,
            leaving: false,
        };
        self.rider = Some(rider);
        events.push(SessionEvent::Seated {
            wagon_guid: carts::routes()[route].wagon_guid(),
            pose: rider.pose(),
        });
    }

    /// The player stands up out of their seat at `position`: mobs may see them again and their
    /// run speed is sent afresh. The seat they left; `None` when they sat in none.
    pub(crate) fn stand_up(
        &mut self,
        position: [f32; 3],
        events: &mut Vec<SessionEvent>,
    ) -> Option<Rider> {
        let rider = self.rider.take()?;
        events.push(SessionEvent::Unseated { position });
        self.update_speed(self.guid, events);
        Some(rider)
    }

    /// `create`, the player's as other clients see it, in their seat's frame while they ride.
    pub(super) fn seated(&self, create: &mut SessionEvent) {
        let (
            Some(rider),
            SessionEvent::ObjectCreate {
                transport, mover, ..
            },
        ) = (self.rider, create)
        else {
            return;
        };
        *transport = Some(rider.pose());
        let flags = mover.map_or(0, |m| m.flags) | carts::ON_TRANSPORT;
        *mover = Some(MoverState { flags, pitch: 0.0 });
    }
}
