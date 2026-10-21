//! Crossing between Phases: stepping into a Phased Dungeon's footprint swaps the open world's
//! mobs for a fresh set of the dungeon's and starts its Run; stepping out, or dying, ends the
//! Run and swaps them back.

use benilla_protocol::SessionEvent;

use super::InWorld;
use crate::characters::Char;
use crate::mobs::{self, Mob};
use crate::phased::{self, GameObjectUse, Phase, UseKind};
use crate::population::VIEW_YD;
use crate::quests;
use crate::space::flat_distance;

/// How near a hiding spot a character must stand to crouch at it.
const HIDE_REACH_YD: f32 = 5.0;
/// How far a hiding character may drift before the hide ends.
const HIDE_DRIFT_YD: f32 = 1.0;

/// A character crouched at a hiding spot of a Phased Dungeon.
pub(super) struct Hide {
    dungeon: u32,
    /// Where they crouched.
    at: [f32; 3],
    /// Their health a moment ago: a drop ends the hide.
    health: u32,
    /// Seconds since the meter last lost a stack to them.
    timer: f32,
}

impl InWorld {
    /// Puts the player in the Phase where they stand, unless a worldport is under way.
    pub(crate) fn step_into_phase(&mut self, c: &Char, events: &mut Vec<SessionEvent>) {
        if !self.transfer {
            self.cross_phase(c, events);
        }
    }

    /// Puts the player in the Phase where they stand; the dead are in none of a dungeon's.
    pub(super) fn cross_phase(&mut self, c: &Char, events: &mut Vec<SessionEvent>) {
        let now = if self.player.alive() {
            phased::at(self.map, c.position)
        } else {
            Phase::OpenWorld
        };
        if now == self.phase {
            return;
        }
        // The client is sent only the creatures of its own Phase: the old ones go, with the
        // fight and every aura on them. A Bounty Hunter gives up the chase.
        self.recall_hunter(events);
        self.stop_attack(events);
        let units: Vec<u64> = self.mobs.iter().map(|m| m.guid).collect();
        for &unit in &units {
            self.auras_die_with(unit, events);
        }
        // The new Phase's creatures are its Run's, in its shard: they come with the view.
        self.drop_view(events);
        // The Phase's meter and the hiding spot stay behind.
        self.hiding = None;
        if self.noise_shown > 0 {
            if let Some(meter) = self.meter() {
                self.hide_marker(meter.client_spell, events);
            }
            self.noise_shown = 0;
        }
        self.phase = now;
    }

    /// The meter of the Phase the player is in, if it has one.
    pub(super) fn meter(&self) -> Option<&'static phased::Meter> {
        match self.phase {
            Phase::Dungeon(id) => phased::dungeon(id)?.meter.as_ref(),
            Phase::OpenWorld => None,
        }
    }

    /// The player's aura shows `stacks` of the Phase's meter: put up, raised or lowered in
    /// place, and taken off at 0.
    pub(crate) fn show_noise(&mut self, stacks: u32, events: &mut Vec<SessionEvent>) {
        let Some(meter) = self.meter() else {
            return;
        };
        if stacks == self.noise_shown {
            return;
        }
        if stacks == 0 {
            self.hide_marker(meter.client_spell, events);
        } else {
            self.show_marker(meter.client_spell, false, stacks, events);
        }
        self.noise_shown = stacks;
    }

    /// What the player did to the Run's meter since it last asked: stacks added and drained.
    pub(crate) fn take_noise(&mut self) -> (u32, u32) {
        (
            std::mem::take(&mut self.noise_gain),
            std::mem::take(&mut self.noise_drain),
        )
    }

    /// The player clicks a hiding spot: they crouch there if it is within reach and they are
    /// out of combat.
    pub(crate) fn begin_hide(&mut self, used: GameObjectUse, c: &Char) {
        if used.kind != UseKind::Hide
            || !self.player.alive()
            || self.player.in_combat
            || self.phase != Phase::Dungeon(used.dungeon)
        {
            return;
        }
        let spot = phased::dungeon(used.dungeon).and_then(|d| d.usable(used.guid));
        let Some((spot, _)) = spot else {
            return;
        };
        if flat_distance(spot.position, c.position) > HIDE_REACH_YD {
            return;
        }
        self.hiding = Some(Hide {
            dungeon: used.dungeon,
            at: c.position,
            health: self.player.health,
            timer: 0.0,
        });
    }

    /// A hiding player takes a stack off the Run's meter every drain period; moving, attacking,
    /// taking damage or a fight ends it.
    pub(super) fn hide_tick(&mut self, dt: f32, c: &Char) {
        let Some(hide) = &mut self.hiding else {
            return;
        };
        let still = flat_distance(c.position, hide.at) <= HIDE_DRIFT_YD;
        let hurt = self.player.health < hide.health;
        if !self.player.alive()
            || self.player.in_combat
            || self.player.attacking.is_some()
            || hurt
            || !still
        {
            self.hiding = None;
            return;
        }
        hide.health = self.player.health;
        let Some(every) = phased::dungeon(hide.dungeon)
            .and_then(|d| d.meter.as_ref())
            .map(|m| m.hide_drain_every_s)
            .filter(|every| *every > 0.0)
        else {
            return;
        };
        hide.timer += dt;
        while hide.timer >= every {
            hide.timer -= every;
            self.noise_drain += 1;
        }
    }

    /// Streams each Phased Dungeon's doorway portal in and out with the view, in every Phase.
    pub(super) fn show_portals(&mut self, me: [f32; 3], events: &mut Vec<SessionEvent>) {
        for d in phased::content() {
            let d_yd = if d.map == self.map {
                flat_distance(d.portal.position, me)
            } else {
                f32::INFINITY
            };
            let shown = self.portals.contains(&d.id);
            if !shown && d_yd <= VIEW_YD {
                self.portals.push(d.id);
                events.push(d.portal_create());
            } else if shown && d_yd > VIEW_YD + 10.0 {
                self.portals.retain(|&id| id != d.id);
                events.push(SessionEvent::ObjectsRemoved(vec![d.portal_guid()]));
            }
            self.show_dungeon_objects(d, me, events);
        }
    }

    /// Streams `d`'s GameObjects in and out with the view, those for the Phase only while the
    /// player is in it.
    fn show_dungeon_objects(
        &mut self,
        d: &phased::Dungeon,
        me: [f32; 3],
        events: &mut Vec<SessionEvent>,
    ) {
        let mut gone = Vec::new();
        for o in &d.gameobject {
            let d_yd = if d.map == self.map {
                flat_distance(o.position, me)
            } else {
                f32::INFINITY
            };
            let guid = o.object_guid();
            let seen = o.seen_from(self.phase, d.id);
            if !self.dungeon_objects.contains(&guid) {
                if seen && d_yd <= VIEW_YD {
                    self.dungeon_objects.insert(guid);
                    events.push(o.create());
                }
            } else if !seen || d_yd > VIEW_YD + 10.0 {
                self.dungeon_objects.remove(&guid);
                gone.push(guid);
            }
        }
        if !gone.is_empty() {
            events.push(SessionEvent::ObjectsRemoved(gone));
        }
    }

    /// Streams the open world's quest GameObjects in and out with the view, brings back those
    /// whose respawn has run out, and shows none while the player is inside a Phased Dungeon.
    pub(super) fn show_quest_objects(
        &mut self,
        dt: f32,
        me: [f32; 3],
        events: &mut Vec<SessionEvent>,
    ) {
        for (_, left) in &mut self.quest_spent {
            *left -= dt;
        }
        self.quest_spent.retain(|&(_, left)| left > 0.0);
        let mut gone = Vec::new();
        for o in quests::content::objects() {
            let guid = o.object_guid();
            let d_yd = if o.map == self.map && self.phase == Phase::OpenWorld {
                flat_distance(o.position, me)
            } else {
                f32::INFINITY
            };
            let spent = self.quest_spent.iter().any(|&(g, _)| g == guid);
            if !self.quest_objects.contains(&guid) {
                if !spent && d_yd <= VIEW_YD {
                    self.quest_objects.insert(guid);
                    events.push(o.create());
                }
            } else if spent || d_yd > VIEW_YD + 10.0 {
                self.quest_objects.remove(&guid);
                gone.push(guid);
            }
        }
        if !gone.is_empty() {
            events.push(SessionEvent::ObjectsRemoved(gone));
        }
    }

    /// The player clicks the quest GameObject of `guid`: if it stands in reach, in view and not
    /// used, and a quest wants one, the quest counts it and the object leaves the player's view
    /// until its `respawn` runs out. Whether the click was a quest object's.
    pub(crate) fn use_quest_object(
        &mut self,
        guid: u64,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        let Some(o) = quests::content::objects()
            .into_iter()
            .find(|o| o.object_guid() == guid)
        else {
            return false;
        };
        if !self.quest_objects.contains(&guid)
            || !self.player.alive()
            || flat_distance(o.position, c.position) > crate::space::INTERACTION_YD
        {
            return true;
        }
        if quests::credit_use(c, o.entry, events) {
            self.quest_spent.push((guid, o.respawn));
            self.quest_objects.remove(&guid);
            events.push(SessionEvent::ObjectsRemoved(vec![guid]));
        }
        true
    }

    /// The player clicks the GameObject of `guid`: what it asks for, if it is one of the Phase's
    /// to use.
    pub(crate) fn use_gameobject(&self, guid: u64) -> Option<GameObjectUse> {
        let (d, (_, kind)) = phased::content()
            .iter()
            .filter(|d| d.map == self.map && self.phase == Phase::Dungeon(d.id))
            .find_map(|d| d.usable(guid).map(|u| (d, u)))?;
        Some(GameObjectUse {
            dungeon: d.id,
            guid,
            kind,
        })
    }
}

/// A fresh set of `phase`'s creatures, at full health where their content file puts them.
pub(crate) fn creatures(phase: Phase) -> Vec<Mob> {
    mobs::content()
        .spawn
        .iter()
        .enumerate()
        .filter(|(_, s)| s.phase == phase && s.called_by.is_none())
        .filter_map(|(i, s)| {
            // The authored height: the ground lookup finds the terrain under a building's floor.
            Mob::spawn(i as u32, s.level_min, s.position[2])
        })
        .collect()
}
