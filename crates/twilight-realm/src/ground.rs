//! Where a placed unit stands when it is created: on the host's ground when its content row says
//! so (the default) and the host has that ground loaded, else at the height the row wrote.
//!
//! Every creature, Hub NPC and GameObject that content places goes through here; a create site
//! that reads a row's Z straight is the bug #253 fixed. Spell-made objects (`world::ground`,
//! `duel`) place themselves.

use crate::Host;

/// How far above a row's Z the ground lookup starts: a row written a little under its floor still
/// finds it, and a floor just over the terrain (a building's) wins over the terrain beneath.
const HINT_ABOVE_YD: f32 = 2.0;

/// The ground under `position` on `map`, when `grounded` asks for it and the host has it loaded.
pub(crate) fn ground_under(
    host: &dyn Host,
    map: u32,
    position: [f32; 3],
    grounded: bool,
) -> Option<f32> {
    if !grounded {
        return None;
    }
    let [x, y, z] = position;
    host.ground_height(map, x, y, z + HINT_ABOVE_YD)
}

/// The Z a unit placed at `position` on `map` stands at: the ground's when `grounded` and the
/// host has it, else the row's own.
pub(crate) fn grounded_z(host: &dyn Host, map: u32, position: [f32; 3], grounded: bool) -> f32 {
    ground_under(host, map, position, grounded).unwrap_or(position[2])
}

/// `position` with its Z grounded (see [`grounded_z`]).
pub(crate) fn grounded_at(
    host: &dyn Host,
    map: u32,
    position: [f32; 3],
    grounded: bool,
) -> [f32; 3] {
    [
        position[0],
        position[1],
        grounded_z(host, map, position, grounded),
    ]
}

/// The `grounded` field's default: a placed unit stands on the ground unless its row says not.
pub(crate) fn yes() -> bool {
    true
}

/// How far a settled unit's ground may differ from where it stands before it is moved, in yards.
const SETTLE_YD: f32 = 0.05;

/// What a unit created before its ground streamed in finds when it asks again.
pub(crate) enum Settle {
    /// The host still has no ground there; ask again later.
    Waiting,
    /// The ground is loaded and the unit already stands on it.
    Level,
    /// The ground is loaded at this height, not where the unit stands.
    Lift(f32),
}

/// Asks the host again for the ground under `at`, for a unit that was created without it.
pub(crate) fn settle(host: &dyn Host, map: u32, at: [f32; 3]) -> Settle {
    match ground_under(host, map, at, true) {
        None => Settle::Waiting,
        Some(ground) if (ground - at[2]).abs() < SETTLE_YD => Settle::Level,
        Some(ground) => Settle::Lift(ground),
    }
}

/// The one-millisecond move that lifts creature `guid` from `at` to `to`.
pub(crate) fn lift_event(
    guid: u64,
    at: [f32; 3],
    to: [f32; 3],
    facing: f32,
) -> benilla_protocol::SessionEvent {
    use benilla_protocol::messages::MonsterMoveFacing;
    use benilla_protocol::SessionEvent;
    SessionEvent::MonsterMove {
        guid,
        transport: None,
        start: at,
        spline_id: 0,
        path: vec![at, to],
        facing: MonsterMoveFacing::Angle(facing),
        stop: false,
        duration_ms: 1,
        flying: false,
        run_mode: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Shelf(Option<f32>);
    impl Host for Shelf {
        fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
            self.0
        }
        fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
            true
        }
        fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
            None
        }
    }

    #[test]
    fn a_grounded_unit_takes_the_hosts_ground() {
        assert_eq!(
            grounded_z(&Shelf(Some(10.0)), 0, [1.0, 2.0, 3.0], true),
            10.0
        );
        assert_eq!(
            grounded_at(&Shelf(Some(10.0)), 0, [1.0, 2.0, 3.0], true),
            [1.0, 2.0, 10.0]
        );
    }

    #[test]
    fn an_ungrounded_unit_keeps_its_written_z() {
        assert_eq!(
            grounded_z(&Shelf(Some(10.0)), 0, [1.0, 2.0, 3.0], false),
            3.0
        );
        assert_eq!(
            ground_under(&Shelf(Some(10.0)), 0, [1.0, 2.0, 3.0], false),
            None
        );
    }

    #[test]
    fn no_ground_loaded_keeps_the_written_z() {
        assert_eq!(grounded_z(&Shelf(None), 0, [1.0, 2.0, 3.0], true), 3.0);
    }
}
