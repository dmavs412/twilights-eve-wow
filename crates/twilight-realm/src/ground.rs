//! Where a placed unit stands when it is created: on the host's ground when its content row says
//! so (the default) and the host has that ground loaded, else at the height the row wrote.
//!
//! Every creature, Hub NPC and GameObject that content places goes through here; a create site
//! that reads a row's Z straight is the bug #253 fixed. A spawn the realm places at run time (an
//! Audience creature, a Rumour's giver and step spawns, a Grade chest, a Bounty Hunter and its
//! adds) goes through [`spot`], which also tests the body against the host's collision and water
//! and nudges it out to [`NUDGE_YD`] (ADR 0021). Spell-made objects (`world::ground`, `duel`)
//! place themselves.

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

/// How far [`spot`] nudges a blocked spawn, in rings 1 yd apart.
pub(crate) const NUDGE_YD: f32 = 5.0;
/// The deepest water a land unit may stand in, surface over ground, in yards.
pub(crate) const DEEP_WATER_YD: f32 = 1.5;

/// Where a run-time spawn wanted at `wanted` on `map` stands: the first of `wanted` and the rings
/// at 1 to [`NUDGE_YD`] yd round it (8 headings each, from +x, as `ground_check.py --fix` tries
/// them) that has ground, water no deeper than [`DEEP_WATER_YD`] unless the unit `swims`, and
/// room for its body ([`Host::body_fits`]). With no ground loaded at `wanted`, `wanted` itself,
/// for the unit to settle on later; `None` when no candidate passes.
pub(crate) fn spot(host: &dyn Host, map: u32, wanted: [f32; 3], swims: bool) -> Option<[f32; 3]> {
    let rings = (1..=NUDGE_YD as u32).flat_map(|r| {
        (0..8).map(move |k| {
            let a = (45.0 * k as f32).to_radians();
            (r as f32 * a.cos(), r as f32 * a.sin())
        })
    });
    for (n, (dx, dy)) in std::iter::once((0.0, 0.0)).chain(rings).enumerate() {
        let [x, y] = [wanted[0] + dx, wanted[1] + dy];
        let Some(g) = ground_under(host, map, [x, y, wanted[2]], true) else {
            if n == 0 {
                return Some(wanted);
            }
            continue;
        };
        let shallow = swims
            || host
                .liquid_at(map, x, y)
                .is_none_or(|w| w - g <= DEEP_WATER_YD);
        if shallow && host.body_fits(map, [x, y, g]) {
            return Some([x, y, g]);
        }
    }
    None
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

    /// Flat ground at z 10 (or none), water at a fixed height, and room for a body where `fits`
    /// says so.
    struct Room {
        ground: Option<f32>,
        water: Option<f32>,
        fits: fn([f32; 3]) -> bool,
    }
    impl Host for Room {
        fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
            self.ground
        }
        fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
            true
        }
        fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
            None
        }
        fn liquid_at(&self, _: u32, _: f32, _: f32) -> Option<f32> {
            self.water
        }
        fn body_fits(&self, _: u32, at: [f32; 3]) -> bool {
            (self.fits)(at)
        }
    }

    fn room(fits: fn([f32; 3]) -> bool) -> Room {
        Room {
            ground: Some(10.0),
            water: None,
            fits,
        }
    }

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-4)
    }

    #[test]
    fn a_spot_that_fits_stands_on_its_ground() {
        let at = spot(&room(|_| true), 0, [1.0, 2.0, 3.0], false);
        assert_eq!(at, Some([1.0, 2.0, 10.0]));
    }

    #[test]
    fn a_blocked_spot_takes_the_free_point_a_yard_east() {
        // Only the point 1 yd along +x from (0, 0) is free.
        let host = room(|[x, y, _]| (x - 1.0).abs() < 0.01 && y.abs() < 0.01);
        let at = spot(&host, 0, [0.0, 0.0, 3.0], false).expect("a nudged spot");
        assert!(close(at, [1.0, 0.0, 10.0]), "{at:?}");
    }

    #[test]
    fn a_blocked_disc_wider_than_the_rings_has_no_spot() {
        let host = room(|[x, y, _]| x.hypot(y) > 5.5);
        assert_eq!(spot(&host, 0, [0.0, 0.0, 3.0], false), None);
    }

    #[test]
    fn deep_water_turns_away_a_land_unit_but_not_a_swimmer() {
        let host = Room {
            water: Some(12.0),
            ..room(|_| true)
        };
        assert_eq!(spot(&host, 0, [0.0, 0.0, 3.0], false), None);
        assert_eq!(
            spot(&host, 0, [0.0, 0.0, 3.0], true),
            Some([0.0, 0.0, 10.0])
        );
    }

    #[test]
    fn no_ground_loaded_keeps_the_wanted_spot() {
        let host = Room {
            ground: None,
            ..room(|_| false)
        };
        assert_eq!(
            spot(&host, 0, [1.0, 2.0, 3.0], false),
            Some([1.0, 2.0, 3.0])
        );
    }
}
