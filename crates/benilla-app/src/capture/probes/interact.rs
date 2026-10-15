//! `WOW_PROBE_INTERACT="<secs>[;<secs>…]"`: at each time, interact with the current selection as
//! a right-click on it would, through [`crate::target::Interact`]: a vendor's merchant window, a
//! corpse's loot window, a quest giver's dialog, a trainer's gossip. Pair it with
//! `WOW_PROBE_LUA="TargetByName('Barg')"` (or a `WOW_PROBE_KEY` Tab) to pick the unit first.

use bevy::prelude::*;

use super::ProbeClock;

pub(crate) struct ProbeInteractPlugin;

impl Plugin for ProbeInteractPlugin {
    fn build(&self, app: &mut App) {
        let spec = std::env::var("WOW_PROBE_INTERACT").unwrap_or_default();
        let at = parse_times(&spec);
        app.insert_resource(ProbeInteract { at, next: 0 })
            .add_systems(Update, fire_probe_interact);
    }
}

#[derive(Resource)]
struct ProbeInteract {
    /// Fire times in [`ProbeClock`] seconds, ascending.
    at: Vec<f32>,
    /// The first time not yet fired.
    next: usize,
}

/// `"12; 20.5;x"` → `[12.0, 20.5]`, ascending; an unparseable entry is skipped with a warning.
fn parse_times(spec: &str) -> Vec<f32> {
    let mut at: Vec<f32> = spec
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| match s.parse::<f32>() {
            Ok(t) => Some(t),
            Err(_) => {
                warn!("probe-interact: unparseable time {s:?} (want e.g. 12;20) — skipped");
                None
            }
        })
        .collect();
    at.sort_by(f32::total_cmp);
    at
}

/// Sends one [`crate::target::Interact`] per due time, on whatever is selected then.
fn fire_probe_interact(
    mut probe: ResMut<ProbeInteract>,
    time: ProbeClock,
    selection: Res<crate::target::Selection>,
    mut interact: MessageWriter<crate::target::Interact>,
) {
    let now = time.elapsed_secs();
    while probe.at.get(probe.next).is_some_and(|&t| now >= t) {
        probe.next += 1;
        match selection.target {
            Some(entity) => {
                info!("probe-interact: {:?} ({now:.1}s)", selection.guid);
                interact.write(crate::target::Interact(entity));
            }
            None => warn!("probe-interact: nothing selected at {now:.1}s — skipped"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_times;

    #[test]
    fn times_parse_sorted_and_skip_junk() {
        assert_eq!(parse_times("20; 12;x;;7.5"), vec![7.5, 12.0, 20.0]);
        assert!(parse_times("").is_empty());
    }
}
