//! Open Road (spec #560): on a road or street, out of combat, the player runs faster under the
//! Open Road buff. A move asks the host for the terrain texture under it at the next tick, and a
//! road seen in the last [`ROAD_GRACE_SECS`] counts, so a step across a patch of grass keeps the
//! speed.

use benilla_protocol::SessionEvent;

use super::InWorld;
use crate::buffs;
use crate::characters::Char;
use crate::fields::values;
use crate::roads::Roads;
use crate::Host;

/// How long a road counts after the last step on one: a smoothing window, not a rule to tune.
const ROAD_GRACE_SECS: f64 = 1.0;

/// The player's road state.
#[derive(Default)]
pub(super) struct OpenRoad {
    /// A move stood since the last tick: the ground under it is still to be asked about.
    probe: bool,
    /// The wall clock when the player last stood on a road with the unlock on.
    seen_at: Option<f64>,
    /// The run speed percent the road adds, from `roads.toml`.
    pct: u32,
    /// The wall clock at this tick.
    pub(super) now: f64,
    /// Whether the buff is up.
    shown: bool,
}

impl InWorld {
    /// A move stood: the next tick asks the host what lies under it.
    pub(crate) fn probe_road(&mut self) {
        self.road.probe = true;
    }

    /// The tick's road check. When a move stood since the last tick and the Open Road unlock is
    /// on, the host names the texture under the player, and a road stamps the time.
    pub(crate) fn step_on_road(
        &mut self,
        host: &dyn Host,
        c: &Char,
        roads: &Roads,
        open: bool,
        wall: f64,
    ) {
        self.road.now = wall;
        if !std::mem::take(&mut self.road.probe) || !open {
            return;
        }
        let [x, y, _] = c.position;
        let texture = host.ground_texture(c.map, x, y);
        if texture.is_some_and(|t| roads.is_road(c.map, &t)) {
            self.road.seen_at = Some(wall);
            self.road.pct = roads.speed_pct;
        }
    }

    /// Whether Open Road holds: a road seen in the last [`ROAD_GRACE_SECS`], out of combat.
    pub(super) fn on_road(&self) -> bool {
        let r = &self.road;
        !self.player.in_combat && r.seen_at.is_some_and(|t| r.now - t <= ROAD_GRACE_SECS)
    }

    /// The run speed percent Open Road adds while it holds; else 0.
    pub(super) fn road_pct(&self) -> u32 {
        if self.on_road() {
            self.road.pct
        } else {
            0
        }
    }

    /// The spell in the Open Road aura slot: the buff's while it is up, else none.
    pub(super) fn road_aura_spell(&self) -> u32 {
        if self.road.shown {
            buffs::spell_id("open_road")
        } else {
            0
        }
    }

    /// Open Road came on or went off since the last tick (the first road step, the grace running
    /// out, a fight starting or ending): the run speed and the buff follow.
    pub(super) fn road_tick(&mut self, events: &mut Vec<SessionEvent>) {
        let on = self.on_road();
        if on == self.road.shown {
            return;
        }
        self.road.shown = on;
        self.update_speed(self.guid, events);
        events.push(values(self.guid, &self.food_aura_pairs()));
    }
}
