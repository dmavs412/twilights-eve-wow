//! `WOW_PROBE_RUMOUR="id:point[@secs]"`: in Twilight's Eve, starts Rumour `id` of
//! `content/rumours.toml` for the player once in-world, its giver at `point` (counted from 1 in
//! the row's `points`) and that point's note in the backpack. A dev aid for walking to a giver
//! without farming the roll; pair it with `WOW_PROBE_GOTO` to stand at the point. Without `@secs`
//! it goes as soon as the player is in-world.

use bevy::prelude::*;

use super::ProbeClock;
use crate::twilight::DevRumour;

pub(crate) struct ProbeRumourPlugin;

impl Plugin for ProbeRumourPlugin {
    fn build(&self, app: &mut App) {
        if !crate::twilight::active() {
            warn!("probe-rumour: WOW_PROBE_RUMOUR needs the twilights-eve launcher — ignored");
            return;
        }
        let spec = std::env::var("WOW_PROBE_RUMOUR").unwrap_or_default();
        let Some(rumour) = parse(&spec) else {
            warn!("probe-rumour: unparseable {spec:?} (want id:point[@secs]) — ignored");
            return;
        };
        app.insert_resource(ProbeRumour {
            rumour,
            sent: false,
        })
        .add_systems(Update, fire_probe_rumour);
    }
}

/// The parsed spec: which Rumour, which point (from 0), and when.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Rumour {
    id: u32,
    point: usize,
    /// [`ProbeClock`] seconds; `None` for as soon as the player is in-world.
    at: Option<f32>,
}

#[derive(Resource)]
struct ProbeRumour {
    rumour: Rumour,
    sent: bool,
}

/// `"id:point[@secs]"` with `point` from 1; `None` when a number is missing, does not parse, or
/// the point is 0.
fn parse(spec: &str) -> Option<Rumour> {
    let (which, at) = match spec.split_once('@') {
        Some((which, at)) => (which, Some(at.trim().parse::<f32>().ok()?)),
        None => (spec, None),
    };
    let (id, point) = which.split_once(':')?;
    let point = point.trim().parse::<usize>().ok()?.checked_sub(1)?;
    Some(Rumour {
        id: id.trim().parse().ok()?,
        point,
        at,
    })
}

/// Asks the realm for the Rumour once, when it is due.
fn fire_probe_rumour(
    mut probe: ResMut<ProbeRumour>,
    time: ProbeClock,
    state: Res<State<crate::char_select::ClientState>>,
    mut out: MessageWriter<DevRumour>,
) {
    if probe.sent || *state.get() != crate::char_select::ClientState::InWorld {
        return;
    }
    let Rumour { id, point, at } = probe.rumour;
    if at.is_some_and(|at| time.elapsed_secs() < at) {
        return;
    }
    probe.sent = true;
    info!("probe-rumour: Rumour {id} at point {}", point + 1);
    out.write(DevRumour { id, point });
}

#[cfg(test)]
mod tests {
    use super::{parse, Rumour};

    #[test]
    fn the_spec_parses_a_rumour_a_point_from_one_and_an_optional_time() {
        assert_eq!(
            parse("3:2"),
            Some(Rumour {
                id: 3,
                point: 1,
                at: None
            })
        );
        assert_eq!(
            parse(" 3 : 1 @20"),
            Some(Rumour {
                id: 3,
                point: 0,
                at: Some(20.0)
            })
        );
        assert_eq!(parse("3:0"), None);
        assert_eq!(parse("3"), None);
        assert_eq!(parse("3:x"), None);
        assert_eq!(parse(""), None);
    }
}
