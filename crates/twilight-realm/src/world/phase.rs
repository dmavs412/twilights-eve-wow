//! Crossing between Phases: stepping into a Phased Dungeon's footprint swaps the open world's
//! mobs for a fresh set of the dungeon's and starts its Run; stepping out, or dying, ends the
//! Run and swaps them back.

use benilla_protocol::SessionEvent;

use super::InWorld;
use crate::characters::Char;
use crate::dungeon::Place;
use crate::ground::grounded_at;
use crate::mobs::{self, Mob};
use crate::phased::{self, GameObjectUse, Phase, StopPortal, UseKind};
use crate::population::VIEW_YD;
use crate::quests;
use crate::space::flat_distance;
use crate::Host;

/// How near a hiding spot a character must stand to crouch at it.
const HIDE_REACH_YD: f32 = 5.0;
/// How far a hiding character may drift before the hide ends.
const HIDE_DRIFT_YD: f32 = 1.0;

/// What a click on a GameObject did with a Phased Dungeon's door.
pub(crate) enum DoorClick {
    /// The GameObject is no door.
    NotADoor,
    /// A door that stays as it is: out of reach, or the character has no key.
    Ignored,
    /// The door of `dungeon` opens for `secs`; `first` when it is the character's first time.
    Opened {
        dungeon: u32,
        secs: f32,
        first: bool,
    },
}

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
    pub(crate) fn step_into_phase(&mut self, c: &mut Char, events: &mut Vec<SessionEvent>) {
        if !self.transfer {
            self.cross_phase(c, events);
        }
    }

    /// Puts the player in the Phase where they stand; the dead are in none of a dungeon's. A
    /// shut door keeps out a player who is not already in the Phase: they are put outside it.
    pub(super) fn cross_phase(&mut self, c: &mut Char, events: &mut Vec<SessionEvent>) {
        let now = if self.player.alive() {
            phased::at(self.map, c.position)
        } else {
            Phase::OpenWorld
        };
        // A character who walks out of a stop's box into no other box is dragged back to the
        // building. The dead are free of it, so Release Spirit works from a stop.
        if !self.player.alive() {
            self.in_stop = None;
        } else if now == Phase::OpenWorld {
            if let Some((id, i)) = self.in_stop.take() {
                if self.drag_back(id, i, c, events) {
                    return;
                }
            }
        } else {
            self.in_stop = phased::stop_at(self.map, c.position);
        }
        if now == self.phase {
            return;
        }
        if let Phase::Dungeon(id) = now {
            let door = phased::dungeon(id).and_then(|d| d.portal.door.as_ref());
            if let Some(door) = door.filter(|_| !self.doors_open.contains(&id)) {
                events.push(SessionEvent::Notification {
                    text: door.sealed_text.clone(),
                });
                self.teleport(c, door.outside, c.orientation, events);
                return;
            }
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

    /// Puts the player at the `back` of stop `stop`'s inner portal, in the same Phase and Run.
    /// Whether the stop has one.
    fn send_back(
        &mut self,
        dungeon: u32,
        stop: usize,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        let Some(d) = phased::dungeon(dungeon) else {
            return false;
        };
        let Some(portal) = d.stop.get(stop).and_then(|s| d.inner_portal_of(&s.key)) else {
            return false;
        };
        let [x, y, z, orientation] = portal.back;
        let to = Place {
            map: d.map,
            position: [x, y, z],
            orientation,
        };
        self.in_stop = None;
        self.move_to(c, to, events);
        true
    }

    /// The pull of the portal: the player stepped out of a stop's box and goes back to the
    /// building.
    fn drag_back(
        &mut self,
        dungeon: u32,
        stop: usize,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        if !self.send_back(dungeon, stop, c, events) {
            return false;
        }
        let name = phased::dungeon(dungeon).map_or("", |d| d.name.as_str());
        events.push(SessionEvent::Notification {
            text: format!("The portal's pull drags you back to {name}."),
        });
        true
    }

    /// Puts the player at the stop `stop` of `d` if they stand in its box: a ward stone's recall.
    /// Whether they were sent.
    pub(crate) fn recall_from_stop(
        &mut self,
        d: &phased::Dungeon,
        stop: usize,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        self.map == d.map
            && self.phase == Phase::Dungeon(d.id)
            && self.player.alive()
            && d.stop[stop].holds(c.position)
            && self.send_back(d.id, stop, c, events)
    }

    /// The player clicks the ward stone `used`: whether the click is theirs to make, in reach of a
    /// stone they see.
    pub(crate) fn ward_click(&self, used: GameObjectUse, c: &Char) -> bool {
        let stone = phased::dungeon(used.dungeon).and_then(|d| d.usable(used.guid));
        let Some((stone, UseKind::Ward)) = stone else {
            return false;
        };
        self.player.alive()
            && self.phase == Phase::Dungeon(used.dungeon)
            && self.dungeon_objects.contains(&used.guid)
            && flat_distance(stone.position, c.position) <= crate::space::INTERACTION_YD
    }

    /// The player clicks the GameObject of `guid`, which may be one of a stop's two portals. An
    /// open inner portal takes them to the stop's arrival and a return portal to the inner
    /// portal's back; a dark one, or one out of reach, does nothing. Whether the GameObject was a
    /// stop portal.
    pub(crate) fn click_stop_portal(
        &mut self,
        guid: u64,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        let Some((d, stop, kind)) = phased::Dungeon::with_stop_portal(guid) else {
            return false;
        };
        let Some(object) = d
            .stop_objects()
            .into_iter()
            .map(|o| o.object)
            .find(|o| o.guid == guid)
        else {
            return true;
        };
        if d.map != self.map
            || self.phase != Phase::Dungeon(d.id)
            || !self.player.alive()
            || flat_distance(object.position, c.position) > crate::space::INTERACTION_YD
        {
            return true;
        }
        match kind {
            StopPortal::Inner => {
                let key = &d.stop[stop].key;
                if self.inner_open.contains(key) {
                    let [x, y, z, orientation] = d.stop[stop].arrival;
                    let to = Place {
                        map: d.map,
                        position: [x, y, z],
                        orientation,
                    };
                    self.in_stop = Some((d.id, stop));
                    self.move_to(c, to, events);
                }
            }
            StopPortal::Return => {
                self.send_back(d.id, stop, c, events);
            }
        }
        true
    }

    /// Streams `d`'s stop portals in and out with the view, only while the player is in its
    /// Phase, and an inner portal only while its stop is open.
    fn show_stop_portals(
        &mut self,
        d: &phased::Dungeon,
        me: [f32; 3],
        host: &dyn Host,
        events: &mut Vec<SessionEvent>,
    ) {
        let mut gone = Vec::new();
        for mut o in d.stop_objects() {
            o.object.position = grounded_at(host, d.map, o.object.position, true);
            let open = o
                .gate
                .is_none_or(|key| self.inner_open.iter().any(|k| k == key));
            let seen = d.map == self.map && self.phase == Phase::Dungeon(d.id) && open;
            let d_yd = if seen {
                flat_distance(o.object.position, me)
            } else {
                f32::INFINITY
            };
            let guid = o.object.guid;
            if !self.dungeon_objects.contains(&guid) {
                if d_yd <= VIEW_YD {
                    self.dungeon_objects.insert(guid);
                    events.push(phased::gameobject_create(&o.object));
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

    /// Streams each Phased Dungeon's doorway portal in and out with the view, in every Phase,
    /// and with it the dungeon's GameObjects, stop portals and door, each standing on `host`'s
    /// ground.
    pub(super) fn show_portals(
        &mut self,
        me: [f32; 3],
        host: &dyn Host,
        events: &mut Vec<SessionEvent>,
    ) {
        for d in phased::content() {
            let d_yd = if d.map == self.map {
                flat_distance(d.portal.position, me)
            } else {
                f32::INFINITY
            };
            let shown = self.portals.contains(&d.id);
            if !shown && d_yd <= VIEW_YD {
                self.portals.push(d.id);
                events.push(d.portal_create(host));
            } else if shown && d_yd > VIEW_YD + 10.0 {
                self.portals.retain(|&id| id != d.id);
                events.push(SessionEvent::ObjectsRemoved(vec![d.portal_guid()]));
            }
            self.show_dungeon_objects(d, me, host, events);
            self.show_stop_portals(d, me, host, events);
            if let Some(door) = &d.portal.door {
                self.show_door(d, door, me, host, events);
            }
        }
    }

    /// Streams `door` in and out with the view, in every Phase, and shows it open or shut as the
    /// world has it.
    fn show_door(
        &mut self,
        d: &phased::Dungeon,
        door: &phased::Door,
        me: [f32; 3],
        host: &dyn Host,
        events: &mut Vec<SessionEvent>,
    ) {
        let d_yd = if d.map == self.map {
            flat_distance(door.position, me)
        } else {
            f32::INFINITY
        };
        let open = self.doors_open.contains(&d.id);
        match self.doors_shown.get(&d.id).copied() {
            None if d_yd <= VIEW_YD => {
                events.push(door.create(host, d.map));
                self.doors_shown.insert(d.id, false);
            }
            Some(_) if d_yd > VIEW_YD + 10.0 => {
                self.doors_shown.remove(&d.id);
                events.push(SessionEvent::ObjectsRemoved(vec![door.guid()]));
                return;
            }
            Some(_) => {}
            None => return,
        }
        if self.doors_shown.insert(d.id, open) != Some(open) {
            events.push(door.state(open));
        }
    }

    /// The player clicks the GameObject of `guid`, which may be a Phased Dungeon's door. A door
    /// opens for a character who has opened it before, or who has its key in their bags; the
    /// first opening is remembered for good and counted for the quests that want a use of it.
    pub(crate) fn click_door(
        &mut self,
        guid: u64,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) -> DoorClick {
        let Some((d, door)) = phased::Dungeon::with_door(guid) else {
            return DoorClick::NotADoor;
        };
        if d.map != self.map
            || !self.player.alive()
            || flat_distance(door.position, c.position) > crate::space::INTERACTION_YD
        {
            return DoorClick::Ignored;
        }
        let opened = c.doors_opened.contains(&door.entry);
        if !opened && c.inventory.count(door.opens_with) == 0 {
            events.push(SessionEvent::Notification {
                text: door.sealed_text.clone(),
            });
            return DoorClick::Ignored;
        }
        if !opened {
            c.doors_opened.push(door.entry);
            c.doors_opened.sort_unstable();
            quests::credit_use(c, door.entry, events);
        }
        DoorClick::Opened {
            dungeon: d.id,
            secs: door.open_secs,
            first: !opened,
        }
    }

    /// Streams `d`'s GameObjects in and out with the view, those for the Phase only while the
    /// player is in it.
    fn show_dungeon_objects(
        &mut self,
        d: &phased::Dungeon,
        me: [f32; 3],
        host: &dyn Host,
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
            let seen =
                o.seen_from(self.phase, d.id) && o.appears.met(&self.run_pinned, &self.run_dead);
            if !self.dungeon_objects.contains(&guid) {
                if seen && d_yd <= VIEW_YD {
                    self.dungeon_objects.insert(guid);
                    events.push(o.create(host, d.map));
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
        host: &dyn Host,
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
                    events.push(o.create(host));
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

    /// The entry of the quest GameObject `guid` when it gives quests, is in the player's view and
    /// stands within `at`'s interaction range.
    pub(crate) fn giver_object_within_reach(&self, guid: u64, at: [f32; 3]) -> Option<u32> {
        let (entry, position, in_view) = match quests::content::objects()
            .into_iter()
            .find(|o| o.object_guid() == guid)
        {
            Some(o) => (o.entry, o.position, self.quest_objects.contains(&guid)),
            None => {
                // A Phased Dungeon's GameObject that gives quests, in the Run's view.
                let o = phased::content()
                    .iter()
                    .flat_map(|d| &d.gameobject)
                    .find(|o| o.gives_quests && o.object_guid() == guid)?;
                (o.guid, o.position, self.dungeon_objects.contains(&guid))
            }
        };
        (in_view
            && self.player.alive()
            && flat_distance(position, at) <= crate::space::INTERACTION_YD
            && quests::content::offered_by_object(entry).next().is_some())
        .then_some(entry)
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

/// A fresh set of `phase`'s creatures, at full health where their content file puts them, at
/// their written heights: the shard stands them on the ground on its first tick with a host
/// (`crate::ground`).
pub(crate) fn creatures(phase: Phase) -> Vec<Mob> {
    mobs::content()
        .spawn
        .iter()
        .enumerate()
        .filter(|(_, s)| s.phase == phase && s.called_by.is_none() && s.timed_by.is_none())
        .filter(|(_, s)| s.appears.is_none_or(|a| a.met(&[], &[])))
        .filter_map(|(i, s)| Mob::spawn(i as u32, s.level_min, s.position[2]))
        .collect()
}
