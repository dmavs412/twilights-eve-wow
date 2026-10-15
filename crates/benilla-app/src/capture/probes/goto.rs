//! `WOW_PROBE_GOTO="x,y[,facing][@secs]"`: in Twilight's Eve, puts the player at WoW-space
//! `(x, y)` on the current map once in-world. The Local Realm grounds it (its host's ground
//! height, with the spot's ADT tile fetched first when it lies beyond the stream window: see
//! `twilight::goto`) and sends the same teleport the Hearthstone does; a spot with no ground is
//! refused with a notification. Without `@secs` it goes as soon as the world settles: in-world,
//! with the terrain under the player streamed in, or [`SETTLE_LIMIT_SECS`] after entry.

use benilla_world::world_point::WorldPoint;
use bevy::prelude::*;

use super::ProbeClock;
use crate::twilight::DevGoto;

/// How long the default schedule waits for the player's ground after world entry before it asks
/// anyway.
const SETTLE_LIMIT_SECS: f32 = 30.0;

pub(crate) struct ProbeGotoPlugin;

impl Plugin for ProbeGotoPlugin {
    fn build(&self, app: &mut App) {
        let spec = std::env::var("WOW_PROBE_GOTO").unwrap_or_default();
        if !crate::twilight::active() {
            warn!("probe-goto: WOW_PROBE_GOTO needs Twilight's Eve (WOW_TWILIGHT=1) — ignored");
            return;
        }
        let Some(goto) = parse(&spec) else {
            warn!("probe-goto: unparseable {spec:?} (want x,y[,facing][@secs]) — ignored");
            return;
        };
        app.insert_resource(ProbeGoto {
            goto,
            entered_at: None,
            sent: false,
        })
        .add_systems(Update, fire_probe_goto);
    }
}

/// The parsed spec: where, and when.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Goto {
    x: f32,
    y: f32,
    facing: Option<f32>,
    /// [`ProbeClock`] seconds; `None` for as soon as the world settles.
    at: Option<f32>,
}

#[derive(Resource)]
struct ProbeGoto {
    goto: Goto,
    /// When the world entry was first seen, on [`ProbeClock`].
    entered_at: Option<f32>,
    sent: bool,
}

/// `"x,y[,facing][@secs]"`; `None` when any number is missing or does not parse.
fn parse(spec: &str) -> Option<Goto> {
    let (place, at) = match spec.split_once('@') {
        Some((place, at)) => (place, Some(at.trim().parse::<f32>().ok()?)),
        None => (spec, None),
    };
    let numbers = place
        .split(',')
        .map(|n| n.trim().parse::<f32>().ok())
        .collect::<Option<Vec<f32>>>()?;
    let (x, y, facing) = match numbers[..] {
        [x, y] => (x, y, None),
        [x, y, facing] => (x, y, Some(facing)),
        _ => return None,
    };
    Some(Goto { x, y, facing, at })
}

/// Asks the realm for the teleport once, when it is due.
fn fire_probe_goto(
    mut probe: ResMut<ProbeGoto>,
    time: ProbeClock,
    state: Res<State<crate::char_select::ClientState>>,
    self_player: Query<&GlobalTransform, With<crate::net::SelfPlayer>>,
    world: WorldPoint,
    mut out: MessageWriter<DevGoto>,
) {
    if probe.sent {
        return;
    }
    let Ok(me) = self_player.single() else {
        return;
    };
    if *state.get() != crate::char_select::ClientState::InWorld {
        return;
    }
    let now = time.elapsed_secs();
    let entered_at = *probe.entered_at.get_or_insert(now);
    let Goto { x, y, facing, at } = probe.goto;
    let due = match at {
        Some(at) => now >= at,
        None => {
            let ground = world.terrain_height_under(me.translation());
            ground.is_some() || now >= entered_at + SETTLE_LIMIT_SECS
        }
    };
    if !due {
        return;
    }
    probe.sent = true;
    info!("probe-goto: teleport to ({x}, {y}) facing {facing:?} at {now:.1}s");
    out.write(DevGoto { x, y, facing });
}

#[cfg(test)]
mod tests {
    use super::{parse, Goto};

    #[test]
    fn the_spec_parses_a_spot_an_optional_facing_and_an_optional_time() {
        assert_eq!(
            parse("-848.7,-2050"),
            Some(Goto {
                x: -848.7,
                y: -2050.0,
                facing: None,
                at: None
            })
        );
        assert_eq!(
            parse(" -848.7, -2050 ,3.0@25"),
            Some(Goto {
                x: -848.7,
                y: -2050.0,
                facing: Some(3.0),
                at: Some(25.0)
            })
        );
        assert_eq!(parse("1,2@x"), None);
        assert_eq!(parse("1"), None);
        assert_eq!(parse("1,2,3,4"), None);
        assert_eq!(parse(""), None);
    }
}
