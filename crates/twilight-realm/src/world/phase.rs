//! Crossing between Phases: stepping into a Phased Dungeon's footprint swaps the open world's
//! mobs for a fresh set of the dungeon's and starts its Run; stepping out, or dying, ends the
//! Run and swaps them back.

use benilla_protocol::messages::{ChatMessage, CHAT_MSG_MONSTER_SAY};
use benilla_protocol::{PhaseMeterState, SessionEvent};

use super::InWorld;
use crate::characters::Char;
use crate::dungeon::Place;
use crate::ground::{ground_under, grounded_at, settle, Settle};
use crate::mobs::{self, Mob};
use crate::phased::{self, GameObjectUse, Phase, StopPortal, UseKind};
use crate::population::VIEW_YD;
use crate::quests;
use crate::space::flat_distance;
use crate::Host;

/// The longest step between two movement reports that counts as walking; a longer one is a
/// teleport, which the wall lets through.
const WALK_STEP_YD: f32 = 40.0;

/// Seconds of realm time between two "You cannot go that way." messages to one character.
pub(super) const CLOSED_HUSH_SECS: f32 = 3.0;

/// What a click on a GameObject did with a Phased Dungeon's locked door.
pub(crate) enum LockedClick {
    /// The GameObject is no locked door.
    NotLocked,
    /// A door that stays as it is: out of reach, already unlocked or no key in the bags.
    Stays,
    /// The key is taken and the Run's locked door `door` of `dungeon` unlocks.
    Unlock { dungeon: u32, door: u32 },
}

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

/// A character in Cover in a Phased Dungeon (#538).
pub(super) struct Hide {
    /// The aura they wear for it (`[meter] cover_aura`), if the meter names one.
    aura: Option<u32>,
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
        // A dungeon's key never leaves it: a living character outside its Phase carries none.
        if self.player.alive() {
            for d in phased::content()
                .iter()
                .filter(|d| now != Phase::Dungeon(d.id))
            {
                if let Some(key) = &d.key {
                    let held = c.inventory.count(key.item);
                    if held > 0 {
                        crate::equipment::take(c, key.item, held, events);
                    }
                }
            }
        }
        if now == self.phase {
            return;
        }
        if let Phase::Dungeon(id) = now {
            let door = phased::dungeon(id).and_then(|d| d.portal.door.as_ref());
            if let Some(door) = door.filter(|_| !self.doors_open.contains(&id)) {
                events.push(self.refusal(door));
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
        // The Phase's meter and its Cover stay behind.
        self.leave_cover(events);
        self.hide_noise(events);
        self.phase = now;
        // The client walks through the new Phase's Cover, told before its creates come.
        let guids = match now {
            Phase::Dungeon(id) => phased::dungeon(id).map_or(Vec::new(), |d| d.walk_through()),
            Phase::OpenWorld => Vec::new(),
        };
        if self.walk_through_sent || !guids.is_empty() {
            self.walk_through_sent = !guids.is_empty();
            events.push(SessionEvent::WalkThrough { guids });
        }
    }

    /// The player's report of a step from `from` to `to`: if it walks through the wall of a
    /// Phased Dungeon's footprint, the client is snapped back to `from` and the step is refused.
    /// The dead walk through, and a jump longer than a walker's step is a teleport. Whether the
    /// step was refused.
    pub(crate) fn walled_off(
        &mut self,
        from: [f32; 3],
        to: [f32; 3],
        orientation: f32,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        if !self.player.alive()
            || self.teleport_pending()
            || self.transferring()
            || flat_distance(from, to) > WALK_STEP_YD
        {
            return false;
        }
        if phased::content()
            .iter()
            .any(|d| d.wall_blocks(self.map, from, to))
        {
            self.snap_client(from, orientation, events);
            return true;
        }
        let locked = phased::content()
            .iter()
            .filter(|d| self.phase == Phase::Dungeon(d.id))
            .find_map(|d| d.locked_door_crossed(self.map, from, to, &self.run_unlocked));
        let Some(door) = locked else {
            return false;
        };
        self.snap_client(from, orientation, events);
        if self.closed_hush <= 0.0 {
            self.closed_hush = CLOSED_HUSH_SECS;
            events.push(SessionEvent::Notification {
                text: door.locked_text.clone(),
            });
        }
        true
    }

    /// The text id of the greeting creature `guid` gives in the Run's dungeon, when it is the
    /// dungeon's `hint_npc`: it names the creature that carries the key.
    pub(crate) fn key_hint_text(&self, guid: u64) -> Option<u32> {
        let Phase::Dungeon(id) = self.phase else {
            return None;
        };
        let key = phased::dungeon(id)?.key.as_ref()?;
        let holder = self.run_key_holder?;
        let mob = self.mobs.iter().find(|m| m.guid == guid)?;
        (mob.spawn_row()?.guid == key.hint_npc).then_some(quests::HINT_TEXT + holder)
    }

    /// The player clicks the GameObject of `guid`, which may be a Phased Dungeon's locked door.
    /// In reach, alive and in the Phase, with the key in the bags, the key is taken; the Run
    /// unlocks the door. Without it the click only says the door is locked.
    pub(crate) fn click_locked_door(
        &mut self,
        guid: u64,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) -> LockedClick {
        let Some((d, door)) = phased::locked_door_with(guid) else {
            return LockedClick::NotLocked;
        };
        if self.phase != Phase::Dungeon(d.id)
            || !self.player.alive()
            || self.run_unlocked.contains(&door.guid)
            || flat_distance(door.position, c.position) > crate::space::INTERACTION_YD
        {
            return LockedClick::Stays;
        }
        if c.inventory.count(door.opens_with) == 0 {
            events.push(SessionEvent::Notification {
                text: door.locked_text.clone(),
            });
            return LockedClick::Stays;
        }
        crate::equipment::take(c, door.opens_with, 1, events);
        self.run_unlocked.push(door.guid);
        LockedClick::Unlock {
            dungeon: d.id,
            door: door.guid,
        }
    }

    /// Streams `d`'s locked doors in and out with the view, for the Phase only, and shows each
    /// locked or open as the Run has it.
    fn show_locked_doors(
        &mut self,
        d: &phased::Dungeon,
        me: [f32; 3],
        events: &mut Vec<SessionEvent>,
    ) {
        for door in &d.locked_door {
            let d_yd = if d.map == self.map && self.phase == Phase::Dungeon(d.id) {
                flat_distance(door.position, me)
            } else {
                f32::INFINITY
            };
            let open = self.run_unlocked.contains(&door.guid);
            match self.locked_shown.get(&door.guid).copied() {
                None if d_yd <= VIEW_YD => {
                    events.push(door.create());
                    self.locked_shown.insert(door.guid, false);
                }
                Some(_) if d_yd > VIEW_YD + 10.0 => {
                    self.locked_shown.remove(&door.guid);
                    events.push(SessionEvent::ObjectsRemoved(vec![door.object_guid()]));
                    continue;
                }
                Some(_) => {}
                None => continue,
            }
            if self.locked_shown.insert(door.guid, open) != Some(open) {
                events.push(door.state(open));
            }
        }
    }

    /// The player's report of a step from `from` to `to`: if it ends on a Closed Zone's chunk
    /// (ADR 0015), the client is snapped back to `from`, told so at most once every
    /// [`CLOSED_HUSH_SECS`], and the step is refused. Unlike [`Self::walled_off`], it holds the
    /// dead too and any step length; only the realm's own teleports pass. Whether the step was
    /// refused.
    pub(crate) fn closed_off(
        &mut self,
        from: [f32; 3],
        to: [f32; 3],
        orientation: f32,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        if self.teleport_pending()
            || self.transferring()
            || !super::closed::closed_at(self.map, to[0], to[1])
        {
            return false;
        }
        self.snap_client(from, orientation, events);
        if self.closed_hush <= 0.0 {
            self.closed_hush = CLOSED_HUSH_SECS;
            events.push(SessionEvent::Notification {
                text: "You cannot go that way.".to_string(),
            });
        }
        true
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
            // A return portal stands on the stop's ground. An inner portal stands on a floor of
            // the building, whose Z the row wrote; the terrain under the tower is lower and would
            // bury it.
            o.object.position = grounded_at(host, d.map, o.object.position, o.gate.is_none());
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

    /// The player's Noise Bar shows `stacks` of the Phase's meter: put up on a living
    /// character, even empty, and sent again on every change. `line` names what raised the
    /// stacks this tick; the bar carries it only when they rose.
    pub(crate) fn show_noise(
        &mut self,
        stacks: u32,
        line: Option<&str>,
        events: &mut Vec<SessionEvent>,
    ) {
        let Some(meter) = self.meter().filter(|_| self.player.alive()) else {
            return;
        };
        if self.noise_bar && stacks == self.noise_shown {
            return;
        }
        let rose = line.filter(|_| self.noise_bar && stacks > self.noise_shown);
        events.push(SessionEvent::PhaseMeter {
            meter: Some(PhaseMeterState {
                name: meter.name.clone(),
                stacks,
                max: meter.max,
                end_icon: meter.end_icon.clone(),
                rose: rose.map(str::to_string),
            }),
        });
        self.noise_shown = stacks;
        self.noise_bar = true;
    }

    /// The player's Noise Bar comes down, if it is up.
    pub(super) fn hide_noise(&mut self, events: &mut Vec<SessionEvent>) {
        if self.noise_bar {
            events.push(SessionEvent::PhaseMeter { meter: None });
        }
        self.noise_shown = 0;
        self.noise_bar = false;
    }

    /// What the player did to the Run's meter since it last asked: stacks added and drained.
    pub(crate) fn take_noise(&mut self) -> (u32, u32) {
        (
            std::mem::take(&mut self.noise_gain),
            std::mem::take(&mut self.noise_drain),
        )
    }

    /// Whether the player is in Cover (#538): alive in a Phased Dungeon, within `cover_yd` of one
    /// of its Cover GameObjects, and not fighting, attacking or casting.
    fn in_cover_now(&self, c: &Char) -> bool {
        let Phase::Dungeon(id) = self.phase else {
            return false;
        };
        self.player.alive()
            && !self.player.in_combat
            && self.player.attacking.is_none()
            && !self.skills.casting()
            && phased::dungeon(id).is_some_and(|d| d.in_cover(c.position))
    }

    /// Puts the player in Cover or takes them out of it, as where they stand and what they do
    /// say. In Cover they wear the meter's `cover_aura` and take a stack off the Run's meter
    /// every drain period, counted from when they came in; moving inside the radius keeps it.
    pub(super) fn cover_tick(&mut self, dt: f32, c: &Char, events: &mut Vec<SessionEvent>) {
        if !self.in_cover_now(c) {
            self.leave_cover(events);
            return;
        }
        let meter = self.meter();
        if self.hiding.is_none() {
            let aura = meter.and_then(|m| m.cover_aura);
            if let Some(aura) = aura {
                self.show_marker(aura, true, 1, events);
            }
            self.hiding = Some(Hide { aura, timer: 0.0 });
        }
        let Some(every) = meter
            .map(|m| m.hide_drain_every_s)
            .filter(|every| *every > 0.0)
        else {
            return;
        };
        let Some(hide) = &mut self.hiding else {
            return;
        };
        hide.timer += dt;
        while hide.timer >= every {
            hide.timer -= every;
            self.noise_drain += 1;
        }
    }

    /// The player is out of Cover, if they were in it: its aura comes off.
    pub(super) fn leave_cover(&mut self, events: &mut Vec<SessionEvent>) {
        if let Some(Hide {
            aura: Some(aura), ..
        }) = self.hiding.take()
        {
            self.hide_marker(aura, events);
        }
    }

    /// Whether the player is in Cover, where idle creatures do not notice them.
    pub(super) fn in_cover(&self) -> bool {
        self.hiding.is_some()
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
            self.show_locked_doors(d, me, events);
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
            events.push(self.refusal(door));
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

    /// What tells the player `door` stays shut: its speaker says the sealed text when the
    /// player sees them, else a notification shows it.
    fn refusal(&self, door: &phased::Door) -> SessionEvent {
        let speaker = door.speaker.and_then(|low| {
            self.mobs.iter().find(|m| {
                m.spawn_row()
                    .is_some_and(|s| s.guid == low && s.phase == Phase::OpenWorld)
            })
        });
        let Some(speaker) = speaker else {
            return SessionEvent::Notification {
                text: door.sealed_text.clone(),
            };
        };
        SessionEvent::Chat(ChatMessage {
            chat_type: CHAT_MSG_MONSTER_SAY,
            language: 0,
            sender_guid: speaker.guid,
            target_guid: 0,
            sender_name: Some(speaker.template.name.clone()),
            channel: None,
            text: door.sealed_text.clone(),
            chat_tag: 0,
        })
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
                    if o.grounded && ground_under(host, o.map, o.position, true).is_none() {
                        self.quest_settling.insert(guid);
                    }
                    events.push(o.create(host));
                }
            } else if spent || d_yd > VIEW_YD + 10.0 {
                self.quest_objects.remove(&guid);
                self.quest_settling.remove(&guid);
                gone.push(guid);
            } else if self.quest_settling.contains(&guid) {
                // Created before the ground streamed in. The client cannot move a GameObject in
                // place, so the lift destroys it and creates it again on the ground.
                match settle(host, o.map, o.position) {
                    Settle::Waiting => {}
                    Settle::Level => {
                        self.quest_settling.remove(&guid);
                    }
                    Settle::Lift(_) => {
                        self.quest_settling.remove(&guid);
                        events.push(SessionEvent::ObjectsRemoved(vec![guid]));
                        events.push(o.create(host));
                    }
                }
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

    /// Streams the run-time GameObjects this character is admitted to in and out with the view,
    /// and destroys those that were removed.
    pub(crate) fn show_runtime_objects(
        &mut self,
        objects: &crate::objects::Objects,
        c: &Char,
        events: &mut Vec<SessionEvent>,
    ) {
        let mut gone: Vec<u64> = self
            .runtime_objects
            .iter()
            .copied()
            .filter(|&g| objects.get(g).is_none_or(|o| !o.audience.admits(self.guid)))
            .collect();
        for o in objects.iter().filter(|o| o.audience.admits(self.guid)) {
            let d_yd = if o.spec.map == c.map && self.phase == Phase::OpenWorld {
                flat_distance(o.spec.position, c.position)
            } else {
                f32::INFINITY
            };
            if !self.runtime_objects.contains(&o.guid) {
                if d_yd <= VIEW_YD {
                    self.runtime_objects.insert(o.guid);
                    events.push(o.create());
                }
            } else if d_yd > VIEW_YD + 10.0 {
                gone.push(o.guid);
            }
        }
        for g in &gone {
            self.runtime_objects.remove(g);
        }
        if !gone.is_empty() {
            events.push(SessionEvent::ObjectsRemoved(gone));
        }
    }

    /// Whether the client has the run-time GameObject `guid` and the player is alive to click it.
    pub(crate) fn sees_runtime_object(&self, guid: u64) -> bool {
        self.player.alive() && self.runtime_objects.contains(&guid)
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
    let zone = mobs::zone_factor(crate::phased::zone(phase));
    mobs::content()
        .spawn
        .iter()
        .enumerate()
        .filter(|(_, s)| s.phase == phase && s.called_by.is_none() && s.timed_by.is_none())
        .filter(|(_, s)| s.appears.is_none_or(|a| a.met(&[], &[])))
        .filter_map(|(i, s)| Mob::spawn(i as u32, s.level_min, s.position[2], zone))
        .collect()
}
