//! One copy of a place, shared by every player in it: an open-world map's mobs, which spawns are
//! in the world and which wait to come back, and the Hub's NPCs; or a Phased Dungeon's Run, its
//! creatures and its clock (ADR 0006). A player's own state (stats, auras, loot, quests) stays in
//! their session; the realm lends a session the shard's mobs while it serves that session, and
//! sends each client what comes into and leaves its view.

use std::collections::{HashMap, HashSet};

use benilla_protocol::SessionEvent;

use crate::audience::Audience;
use crate::fields::values;
use crate::looting::lootable_pair;
use crate::mobs::{self, Mob, State};
use crate::phased::{Heard, Meter, Phase, Run};
use crate::population::{Population, VIEW_YD};
use crate::rules::threat;
use crate::space::flat_distance;
use crate::Host;

/// Where a shard is: an open-world map, in the open world or one Phased Dungeon's Phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Place {
    pub(crate) map: u32,
    pub(crate) phase: Phase,
}

/// One place's shared state.
pub(crate) struct Shard {
    place: Place,
    pub(crate) mobs: Vec<Mob>,
    population: Population,
    hub: crate::hub::InView,
    /// A Phased Dungeon's Run: it starts with the shard, when the first player steps in, and
    /// resets with it, once no living player is inside.
    run: Option<Run>,
    /// How many creatures have been made here for an Audience, which numbers their guids.
    audience_spawned: u32,
    /// The guids of the Audience's creatures made before the host had their ground.
    settling: Vec<u64>,
    /// The open world's cart wagons (spec #560); none in a Phase.
    pub(crate) carts: crate::carts::Fleet,
}

/// A player standing on a shard, as the realm sees them for a tick.
pub(crate) struct OnShard<S> {
    pub(crate) session: S,
    pub(crate) guid: u64,
    pub(crate) position: [f32; 3],
    pub(crate) alive: bool,
    /// The share of their health they have left.
    pub(crate) health: f32,
    /// Their create as other clients are sent it.
    pub(crate) create: SessionEvent,
    /// Their Calling's key, which other clients read in place of the class byte (#207).
    pub(crate) calling: String,
    /// Stealthed: other clients are not shown them.
    pub(crate) hidden: bool,
    /// Their patches on the ground: each DynamicObject's guid, centre and create. Another client
    /// sees one in view of its centre, stealthed caster or not.
    pub(crate) patches: Vec<(u64, [f32; 3], SessionEvent)>,
}

/// Whether `guid` is a player's: `HIGHGUID_PLAYER` is 0.
pub(crate) fn is_player(guid: u64) -> bool {
    guid >> 48 == 0
}

/// Whether another client may be sent a player's descriptor `field`: the object and unit block
/// short of the stats, the player's flags and looks, and what they wear in sight. Never the
/// quest log, skills, inventory, XP or money.
pub(crate) fn public_player_field(field: u16) -> bool {
    use crate::fields::{PLAYER_FLAGS, UNIT_FIELD_STAT0};
    /// `UNIT_FIELD_BYTES_2`: the sheath and the PvP flag.
    const UNIT_FIELD_BYTES_2: u16 = 164;
    /// `PLAYER_QUEST_LOG_1_1`, where the private block after the looks starts.
    const PLAYER_QUEST_LOG_1_1: u16 = 198;
    /// `PLAYER_VISIBLE_ITEM_1_CREATOR` and `PLAYER_FIELD_INV_SLOT_HEAD`: the gear in sight.
    const VISIBLE_ITEMS: std::ops::Range<u16> = 258..486;
    field < UNIT_FIELD_STAT0
        || field == UNIT_FIELD_BYTES_2
        || (PLAYER_FLAGS - 2..PLAYER_QUEST_LOG_1_1).contains(&field)
        || VISIBLE_ITEMS.contains(&field)
}

impl Shard {
    /// A fresh shard: an open world streams in around its players; a Phased Dungeon's Run starts
    /// with all its creatures at full health.
    pub(crate) fn new(place: Place, seed: u64) -> Self {
        let dungeon = place.phase != Phase::OpenWorld;
        let run = match place.phase {
            Phase::Dungeon(id) => Some(Run::new(id, seed)),
            Phase::OpenWorld => None,
        };
        Self {
            place,
            mobs: if dungeon {
                crate::world::creatures(place.phase)
            } else {
                Vec::new()
            },
            population: Population::default(),
            hub: Default::default(),
            run,
            audience_spawned: 0,
            settling: Vec::new(),
            carts: if dungeon {
                Default::default()
            } else {
                crate::carts::Fleet::on_map(place.map)
            },
        }
    }

    /// A creature of `template` at `level`, made at `position` for `audience` (ADR 0014): it
    /// stands at the [`crate::ground::spot`] for the point, and no spawn timer ever revives it.
    /// Its guid, or `None` when no spot within reach fits it.
    pub(crate) fn spawn_for(
        &mut self,
        host: &dyn Host,
        template: &'static mobs::Template,
        level: u8,
        position: [f32; 3],
        audience: Audience,
    ) -> Option<u64> {
        let at = crate::ground::spot(host, self.place.map, position, template.swims)?;
        self.audience_spawned = self.audience_spawned.wrapping_add(1);
        let low = mobs::AUDIENCE_GUID_BASE | self.audience_spawned & 0xF_FFFF;
        let guid = mobs::unit_guid(template.entry, low);
        if crate::ground::ground_under(host, self.place.map, position, true).is_none() {
            self.settling.push(guid);
        }
        let zone = mobs::zone_factor_at(host, self.place.phase, self.place.map, at);
        self.mobs
            .push(Mob::for_audience(guid, template, level, at, audience, zone));
        Some(guid)
    }

    /// Removes every creature made for exactly `audience`. Each admitted client has it destroyed
    /// when its view next lacks it.
    pub(crate) fn despawn_audience(&mut self, audience: &Audience) {
        self.mobs.retain(|m| m.audience.as_ref() != Some(audience));
    }

    /// The stops whose inner portal is open in the Run; none with no Run.
    pub(crate) fn portals_open(&self) -> Vec<String> {
        self.run
            .as_ref()
            .map_or(Vec::new(), |r| r.portals_open().to_vec())
    }

    /// Opens or closes the inner portal of stop `key` in the Run.
    pub(crate) fn set_portal(&mut self, key: &str, open: bool) {
        if let Some(run) = &mut self.run {
            run.set_portal(key, open);
        }
    }

    /// The stops pinned, the creatures dead and the levers used in the Run; none with no Run.
    pub(crate) fn run_facts(&self) -> (Vec<String>, Vec<u32>, Vec<u32>) {
        self.run
            .as_ref()
            .map_or((Vec::new(), Vec::new(), Vec::new()), |r| {
                (r.pinned.clone(), r.dead.clone(), r.used.clone())
            })
    }

    /// The creature that carries the Run's key and the locked doors unlocked in it; none with no
    /// Run.
    pub(crate) fn key_facts(&self) -> (Option<u32>, Vec<u32>) {
        self.run
            .as_ref()
            .map_or((None, Vec::new()), |r| (r.key_holder, r.unlocked.clone()))
    }

    /// Unlocks the locked door of `guid` for the rest of the Run. Whether it was locked.
    pub(crate) fn unlock(&mut self, guid: u32) -> bool {
        let Some(run) = &mut self.run else {
            return false;
        };
        let new = !run.unlocked.contains(&guid);
        if new {
            run.unlocked.push(guid);
        }
        new
    }

    /// Whether the dungeon creature of `guid` has died in the Run.
    pub(crate) fn is_dead(&self, guid: u32) -> bool {
        self.run.as_ref().is_some_and(|r| r.dead.contains(&guid))
    }

    /// Marks the lever of `guid` used in the Run. Whether it was newly used.
    pub(crate) fn pull(&mut self, guid: u32) -> bool {
        self.run.as_mut().is_some_and(|r| r.pull(guid))
    }

    /// Pins stop `key` in the Run, darkens its inner portal and opens the one of `opens`. Whether
    /// the stop was newly pinned.
    pub(crate) fn pin(&mut self, key: &str, opens: Option<&str>) -> bool {
        let Some(run) = &mut self.run else {
            return false;
        };
        if !run.pin(key) {
            return false;
        }
        run.set_portal(key, false);
        if let Some(next) = opens {
            run.set_portal(next, true);
        }
        true
    }

    /// Stands the Run's starting creatures on `host`'s ground, once: [`Shard::new`] has no host,
    /// and the first tick comes before any client is shown them.
    fn ground_run(&mut self, host: &dyn Host) {
        let Some(run) = &mut self.run else {
            return;
        };
        if run.grounded {
            return;
        }
        run.grounded = true;
        let map = self.place.map;
        for m in &mut self.mobs {
            let Some(s) = m.spawn_row() else {
                continue;
            };
            m.ground_to(crate::ground::grounded_z(host, map, s.position, s.grounded));
        }
    }

    /// The Run's creatures come and go on its facts: a death is recorded, a creature whose
    /// `until_dead` has died or whose `until_used` lever is pulled is gone, and one whose
    /// `after_pinned`, `after_dead` or `after_used` has come true appears, once, standing on
    /// `host`'s ground.
    fn run_changes(&mut self, host: &dyn Host) {
        let Phase::Dungeon(_) = self.place.phase else {
            return;
        };
        let Some(run) = &mut self.run else {
            return;
        };
        let content = mobs::content();
        for m in &self.mobs {
            if m.state == State::Dead {
                if let Some(s) = m.spawn_row().filter(|s| s.appears.is_some()) {
                    if !run.dead.contains(&s.guid) {
                        run.dead.push(s.guid);
                    }
                }
            }
        }
        let gone = |s: &mobs::Spawn| s.appears.is_some_and(|a| a.gone(&run.dead, &run.used));
        self.mobs
            .retain(|m| !m.spawn_row().is_some_and(|s| gone(s)));
        for (i, s) in content.spawn.iter().enumerate() {
            let Some(a) = s.appears.filter(|_| s.phase == self.place.phase) else {
                continue;
            };
            if a.comes_later()
                && !run.appeared.contains(&s.guid)
                && a.met(&run.pinned, &run.dead, &run.used)
            {
                run.appeared.push(s.guid);
                let z = crate::ground::grounded_z(host, s.map, s.position, s.grounded);
                let zone = mobs::zone_factor_at(host, s.phase, s.map, s.position);
                if let Some(mob) = Mob::spawn(i as u32, s.level_min, z, zone) {
                    self.mobs.push(mob);
                }
            }
        }
    }

    /// The stacks on the Run's meter; 0 with none.
    pub(crate) fn noise(&self) -> u32 {
        self.run.as_ref().map_or(0, |r| r.noise)
    }

    /// The Run's meter hears what happened this tick: the creatures first hurt or dead since it
    /// last looked, `cast` more from casts and `drain` from hiding, and the clock. What each
    /// source added; nothing with no Run.
    pub(crate) fn hear_noise(&mut self, meter: &Meter, cast: u32, drain: u32) -> Heard {
        let Some(run) = &mut self.run else {
            return Heard::default();
        };
        let mut heard = Heard {
            cast,
            ..Heard::default()
        };
        for m in &mut self.mobs {
            if !m.noised && m.health < m.stats.max_health {
                m.noised = true;
                heard.first_damage += meter.on_first_damage;
            }
            if !m.death_noised && m.state == State::Dead {
                m.death_noised = true;
                heard.death += m.template.noise_on_death.unwrap_or(meter.on_death);
            }
        }
        run.hear(meter, heard, drain)
    }

    /// Scales a Phased Dungeon's creatures to the Head Count of the living players `alive` in
    /// its Phase (ADR 0029): an idle creature follows the count, as does one not yet scaled (a
    /// wave or an Audience's creature made straight into a fight); one in a fight keeps the scale
    /// it was pulled at. The new count when it changed since the last, which the Phase is told,
    /// and the values the clients that see the creatures are sent. Nothing with no Run or no one
    /// alive.
    pub(crate) fn scale_to_head_count(
        &mut self,
        alive: &[u64],
    ) -> (Option<u32>, Vec<SessionEvent>) {
        let Some(run) = &mut self.run else {
            return (None, Vec::new());
        };
        if alive.is_empty() {
            return (None, Vec::new());
        }
        let n = alive.len() as u32;
        let changed = (run.head_count != 0 && run.head_count != n).then_some(n);
        run.head_count = n;
        let events = self
            .mobs
            .iter_mut()
            .filter(|m| m.state == State::Idle || (m.unscaled.is_none() && m.state != State::Dead))
            .filter_map(|m| {
                let own = m.audience.as_ref().map_or(n, |a| {
                    alive.iter().filter(|&&g| a.admits(g)).count().max(1) as u32
                });
                m.scale_to(own)
            })
            .collect();
        (changed, events)
    }

    /// Seconds into its Run, for a Phased Dungeon's.
    pub(crate) fn run_elapsed(&self) -> Option<f32> {
        self.run.as_ref().map(|r| r.elapsed)
    }

    /// Streams spawns and Hub NPCs in around `players` and out once none is near, brings back
    /// the dead whose time is up, walks the Hub's wanderers, and runs the cart wagons to
    /// `wall_ms` with the Carts unlock `carts_on`. Nothing streams in on land closed while the
    /// Milestones `reached` are. The destroys and moves it makes; the creates go to each client as
    /// its view takes them in.
    pub(crate) fn update(
        &mut self,
        dt: f32,
        players: &[[f32; 3]],
        host: &dyn Host,
        wall_ms: u64,
        carts_on: bool,
        reached: &[&str],
    ) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        if let Some(run) = &mut self.run {
            run.elapsed += dt;
            self.ground_run(host);
            self.run_changes(host);
            events.extend(self.tick_tram(dt));
            return events;
        }
        let map = self.place.map;
        self.population
            .update(dt, players, map, reached, host, &mut self.mobs, &mut events);
        self.settle_audience(host, &mut events);
        self.hub
            .update(dt, players, map, reached, host, &mut events);
        events.extend(self.carts.update(wall_ms, carts_on));
        events.retain(|e| !matches!(e, SessionEvent::ObjectCreate { .. }));
        events
    }

    /// The Run's tram line (ADR 0033): it starts on the tick after the dungeon's bell is pulled,
    /// and from then on runs `dt` on, told whether any creature a tram brought is alive. The
    /// creatures of each car that stopped step off at its door into the mobs, and the line runs
    /// its creatures' own turn. The moves the clients that see them are sent.
    fn tick_tram(&mut self, dt: f32) -> Vec<SessionEvent> {
        let Phase::Dungeon(id) = self.place.phase else {
            return Vec::new();
        };
        let Some(run) = &mut self.run else {
            return Vec::new();
        };
        let Some(line) = &mut run.tram else {
            let rung = crate::phased::dungeon(id)
                .and_then(|d| d.tram.as_ref())
                .filter(|t| run.used.contains(&t.bell));
            if let Some(tram) = rung {
                run.tram = Some(crate::tram::Line::new(tram, self.place.map, run.seed));
            }
            return Vec::new();
        };
        let alive = self.mobs.iter().any(|m| m.alive() && line.brought(m.guid));
        line.tick(dt, alive);
        let zone = mobs::zone_factor(crate::phased::zone(self.place.phase));
        for s in line.take_stepping() {
            let Some(row) = mobs::tram_row(id, s.entry) else {
                continue;
            };
            let [x, y, z] = s.at;
            let Some(mut mob) = Mob::tram(row, s.level, [x, y], z, zone, s.seq) else {
                continue;
            };
            mob.orientation = s.facing;
            mob.home_orientation = s.facing;
            mob.wears = s.wears;
            mob.frozen = s.frozen;
            mob.strengthen(s.health_pct, s.damage_pct);
            line.bring(mob.guid, s.kind, mob.position);
            self.mobs.push(mob);
        }
        line.run_creatures(&mut self.mobs)
    }

    /// The tram's fighters, Conductor and thawed passengers awake since the last call (ADR 0033),
    /// each turned on the closest living player of `players`, as an Encounter's wave is: what to
    /// tell that player, by guid.
    pub(crate) fn wake_tram(
        &mut self,
        players: &[crate::tram::Standing],
    ) -> Vec<(u64, SessionEvent)> {
        let Some(line) = self.run.as_mut().and_then(|r| r.tram.as_mut()) else {
            return Vec::new();
        };
        let mut told = Vec::new();
        for guid in line.take_waking() {
            let Some(mob) = self.mobs.iter_mut().find(|m| m.guid == guid) else {
                continue;
            };
            if mob.state != State::Idle {
                continue;
            }
            let Some(target) = players.iter().filter(|p| p.alive).min_by(|a, b| {
                flat_distance(a.position, mob.position)
                    .total_cmp(&flat_distance(b.position, mob.position))
            }) else {
                continue;
            };
            let mut events = Vec::new();
            crate::world::engage(mob, target.guid, &mut events);
            if mob.state == State::Engaged {
                line.engaged(guid);
            }
            told.extend(events.into_iter().map(|e| (target.guid, e)));
        }
        told
    }

    /// The Run's tram line, once its bell is rung.
    pub(crate) fn tram(&self) -> Option<&crate::tram::Line> {
        self.run.as_ref().and_then(|r| r.tram.as_ref())
    }

    pub(crate) fn tram_mut(&mut self) -> Option<&mut crate::tram::Line> {
        self.run.as_mut().and_then(|r| r.tram.as_mut())
    }

    /// The route of the tram car `guid`, which its create follows; `None` for anything else.
    pub(crate) fn route_of(&self, guid: u64) -> Option<SessionEvent> {
        self.tram().and_then(|l| l.route_of(guid))
    }

    /// An Audience's creature made before the host had its ground stood at the point's own
    /// height, often under the terrain (a Rumour row writes none); once the ground is there, an
    /// idle one is lifted onto it, once. One that has left the world has nothing to settle.
    fn settle_audience(&mut self, host: &dyn Host, events: &mut Vec<SessionEvent>) {
        use crate::ground::{lift_event, settle, Settle};
        let (map, mobs) = (self.place.map, &mut self.mobs);
        self.settling.retain(|&guid| {
            let Some(m) = mobs.iter_mut().find(|m| m.guid == guid) else {
                return false;
            };
            if m.state != State::Idle || m.walk.is_some() {
                return true;
            }
            match settle(host, map, m.position) {
                Settle::Waiting => true,
                Settle::Level => false,
                Settle::Lift(ground) => {
                    let (at, to) = (m.position, [m.position[0], m.position[1], ground]);
                    events.push(lift_event(m.guid, at, to, m.orientation));
                    m.ground_to(ground);
                    false
                }
            }
        });
    }

    /// Every mob hunting a player who is not on this shard gives up and leaves: its owner went
    /// into a Phase or a Dungeon, or out of the world.
    pub(crate) fn drop_hunts(&mut self, present: &[u64]) {
        self.mobs
            .retain(|m| m.hunts.is_none_or(|owner| present.contains(&owner)));
    }

    /// The weakening by stacks of a player who has left lifts from the mobs: no session of
    /// theirs counts it any more.
    pub(crate) fn drop_withering(&mut self, present: &[u64]) {
        for m in &mut self.mobs {
            m.withered.retain(|by, _| present.contains(by));
        }
    }

    /// A mob of `spawn` died: it comes back once every player has been away `wait` seconds.
    pub(crate) fn died(&mut self, spawn: u32, wait: f32) {
        // An Encounter's creatures never come back by themselves.
        if mobs::content()
            .spawn
            .get(spawn as usize)
            .is_some_and(|s| s.encounter.is_some())
        {
            return;
        }
        self.population.died(spawn, wait);
    }

    /// Everything a player at `me` sees: each mob and Hub NPC within view, and its create.
    /// A mob whose Audience does not admit `viewer` is never in it.
    pub(crate) fn in_view(&self, viewer: u64, me: [f32; 3]) -> Vec<(u64, SessionEvent)> {
        // A Run is there all at once.
        let dungeon = self.run.is_some();
        let near = |m: &&Mob| {
            m.admits(viewer)
                && (dungeon
                    || flat_distance(m.position, me) <= VIEW_YD
                    || flat_distance(m.home, me) <= VIEW_YD)
        };
        let mut seen: Vec<(u64, SessionEvent)> = self
            .mobs
            .iter()
            .filter(near)
            .map(|m| (m.guid, m.create()))
            .collect();
        seen.extend(self.hub.in_view(me));
        seen.extend(self.carts.in_view());
        // The tram's cars and lit lamps, which the whole Phase sees.
        seen.extend(self.tram().map_or(Vec::new(), |l| l.in_view()));
        seen
    }

    /// Whether the client that sees `guid` keeps it out of view: a mob in a fight or running home
    /// stays until it is done, as it stays in the world.
    pub(crate) fn keeps(&self, guid: u64) -> bool {
        self.mobs
            .iter()
            .any(|m| m.guid == guid && !matches!(m.state, State::Idle | State::Dead))
    }

    /// Which player runs each mob's fight and AI this tick: the one it fights, while that player
    /// is here; otherwise the nearest.
    pub(crate) fn owners<S: Copy + Eq + std::hash::Hash>(
        &self,
        players: &[OnShard<S>],
    ) -> HashMap<S, HashSet<u64>> {
        let mut owners: HashMap<S, HashSet<u64>> = HashMap::new();
        for m in &self.mobs {
            let fighting = matches!(m.state, State::Engaged | State::Evading);
            // Only a character its Audience admits runs it.
            let owner = players
                .iter()
                .find(|p| fighting && p.guid == m.target && m.admits(p.guid))
                .or_else(|| {
                    players.iter().filter(|p| m.admits(p.guid)).min_by(|a, b| {
                        flat_distance(a.position, m.position)
                            .total_cmp(&flat_distance(b.position, m.position))
                    })
                });
            if let Some(p) = owner {
                owners.entry(p.session).or_default().insert(m.guid);
            }
        }
        owners
    }
}

/// Adds the threat `events` show `player` making (#147): their damage to each mob 1:1, at
/// `threat_pct` percent of it (Steady Stance, #544), and their heals at half, split across the
/// living mobs that hate the one healed.
pub(crate) fn note_threat(mobs: &mut [Mob], player: u64, threat_pct: u32, events: &[SessionEvent]) {
    use benilla_protocol::messages::PeriodicTick;
    for e in events {
        let (victim, damage, healed, who) = match e {
            SessionEvent::AttackerState(s) if s.attacker == player => {
                (s.victim, s.damage, 0, player)
            }
            SessionEvent::SpellDamageLog(l) if l.attacker == player => {
                (l.target, l.damage, 0, player)
            }
            SessionEvent::PeriodicAuraLog(l) if l.caster == player => {
                let (mut damage, mut healed) = (0, 0);
                for t in &l.ticks {
                    match t {
                        PeriodicTick::Damage { amount, .. } => damage += amount,
                        PeriodicTick::Heal { amount } => healed += amount,
                        _ => {}
                    }
                }
                (l.target, damage, healed, player)
            }
            // A heal from another player is shown in the healed one's session.
            SessionEvent::SpellHealLog(l) => (l.target, 0, l.amount, l.healer),
            _ => continue,
        };
        if let Some(m) = mobs.iter_mut().find(|m| m.guid == victim) {
            // Even a miss puts the attacker on the table.
            m.threat
                .add(player, damage as f32 * threat_pct as f32 / 100.0);
            continue;
        }
        if healed > 0 {
            let hating = |m: &Mob| m.alive() && m.threat.contains(victim);
            let share = threat::heal_share(healed, mobs.iter().filter(|m| hating(m)).count());
            for m in mobs.iter_mut().filter(|m| hating(m)) {
                m.threat.add(who, share);
            }
        }
    }
}

/// The units an event shows doing or suffering something, for the other clients that see them;
/// empty for what only its own client is told.
pub(crate) fn shown_units(e: &SessionEvent) -> Vec<u64> {
    use SessionEvent as E;
    match e {
        E::ObjectValues { guid, .. }
        | E::MonsterMove { guid, .. }
        | E::UnitMove { guid, .. }
        | E::ObjectMove { guid, .. }
        | E::Emote { guid, .. }
        | E::PlaySpellVisual { unit: guid, .. } => vec![*guid],
        E::AttackStart { attacker, victim } | E::AttackStop { attacker, victim } => {
            vec![*attacker, *victim]
        }
        E::AttackerState(s) => vec![s.attacker, s.victim],
        E::SpellStart { caster, target, .. } => {
            [Some(*caster), *target].into_iter().flatten().collect()
        }
        E::SpellGo { caster, hits, .. } => std::iter::once(*caster)
            .chain(hits.iter().copied())
            .collect(),
        E::SpellDamageLog(l) => vec![l.target, l.attacker],
        E::PeriodicAuraLog(l) => vec![l.target, l.caster],
        E::SpellHealLog(l) => vec![l.target, l.healer],
        E::SpellLogMiss(l) => vec![l.caster],
        _ => Vec::new(),
    }
}

/// `e` as another client sees it: a corpse is lootable only by its killer, and a player shows
/// only their public fields; `None` when nothing of it is for others.
pub(crate) fn as_seen_by_others(e: &SessionEvent) -> Option<SessionEvent> {
    match e {
        SessionEvent::ObjectValues { guid, fields } => {
            let (dynamic, _) = lootable_pair(false);
            let player = is_player(*guid);
            let pairs: Vec<(u16, u32)> = fields
                .raw_fields()
                .filter(|&(field, _)| !player || public_player_field(field))
                .map(|(field, v)| {
                    if field == dynamic && !player {
                        lootable_pair(false)
                    } else {
                        (field, v)
                    }
                })
                .collect();
            (!pairs.is_empty()).then(|| values(*guid, &pairs))
        }
        other => Some(other.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mobs::Template;
    use crate::phased::MeterLines;
    use crate::rules::stats::UnitClass;

    fn meter() -> Meter {
        Meter {
            name: "Noise".into(),
            max: 20,
            end_icon: String::new(),
            lines: MeterLines {
                first_damage: "hurt".into(),
                death: "died".into(),
                cast: "cast".into(),
                clock: "clock".into(),
            },
            clock_stack_every_s: 30.0,
            on_first_damage: 1,
            on_death: 1,
            hide_drain_every_s: 2.0,
            cover_aura: None,
            wake_base_yd: 3.0,
            wake_per_stack_yd: 1.5,
            sleep_aura: None,
        }
    }

    /// The Abbey's Run, a Phased Dungeon with creatures in it.
    fn run() -> Shard {
        let shard = Shard::new(
            Place {
                map: 0,
                phase: Phase::Dungeon(1),
            },
            7,
        );
        assert!(shard.mobs.len() >= 2, "the Abbey holds creatures");
        shard
    }

    #[test]
    fn damage_to_a_sleeper_adds_one_stack_once() {
        let mut shard = run();
        shard.hear_noise(&meter(), 0, 0);
        assert_eq!(shard.noise(), 0, "an untouched Run is quiet");
        shard.mobs[0].health -= 1;
        shard.hear_noise(&meter(), 0, 0);
        assert_eq!(shard.noise(), 1);
        shard.mobs[0].health -= 1;
        shard.hear_noise(&meter(), 0, 0);
        assert_eq!(shard.noise(), 1, "a second blow to the same creature");
        shard.mobs[1].health -= 1;
        shard.hear_noise(&meter(), 0, 0);
        assert_eq!(shard.noise(), 2, "the next creature's first blow");
    }

    #[test]
    fn a_death_adds_the_meters_stack_or_the_creatures_own() {
        let mut shard = run();
        let mut runt = Template::hunter(
            1,
            "Runt".into(),
            String::new(),
            1,
            1,
            UnitClass::Warrior,
            2000,
            1.0,
            Vec::new(),
            Vec::new(),
        );
        runt.noise_on_death = Some(2);
        shard.mobs[0].template = Box::leak(Box::new(runt));
        for i in [0, 1] {
            shard.mobs[i].noised = true;
            shard.mobs[i].health = 0;
            shard.mobs[i].state = State::Dead;
        }
        shard.hear_noise(&meter(), 0, 0);
        assert_eq!(shard.noise(), 3, "the Runt's 2 and the meter's 1");
        shard.hear_noise(&meter(), 0, 0);
        assert_eq!(shard.noise(), 3, "a death is heard once");
    }

    #[test]
    fn an_idle_creature_follows_the_head_count_and_a_fighting_one_keeps_its_own() {
        let mut shard = run();
        assert!(shard.mobs.len() >= 3);
        let made: Vec<u32> = shard.mobs.iter().map(|m| m.stats.max_health).collect();
        let (line, sent) = shard.scale_to_head_count(&[1]);
        assert_eq!(line, None, "the first count of a Run is not announced");
        assert_eq!(sent.len(), shard.mobs.len(), "each creature's values, once");
        assert!(shard.mobs.iter().all(|m| m.scaled_for == 1));
        shard.mobs[1].state = State::Engaged;
        shard.mobs[2].audience = Some(Audience::of(&[2]));

        let (line, _) = shard.scale_to_head_count(&[1, 2]);
        assert_eq!(line, Some(2));
        let want = (made[0] as f32 * 1.5).round() as u32;
        assert_eq!(shard.mobs[0].stats.max_health, want);
        assert_eq!(shard.mobs[0].scaled_for, 2);
        assert_eq!(shard.mobs[1].stats.max_health, made[1], "pulled at one");
        assert_eq!(
            shard.mobs[2].scaled_for, 1,
            "its Audience is one of the two"
        );
        assert_eq!(shard.mobs[2].stats.max_health, made[2]);

        let (line, sent) = shard.scale_to_head_count(&[2, 1]);
        assert_eq!(line, None, "the same count");
        assert!(sent.is_empty());
    }

    #[test]
    fn a_creature_made_into_a_fight_is_scaled_once_and_the_dead_never() {
        let mut shard = run();
        shard.scale_to_head_count(&[1]);
        // A wave's creature, made mid-Run straight into its fight.
        shard.mobs[1].unscaled = None;
        shard.mobs[1].state = State::Engaged;
        shard.mobs[0].state = State::Dead;
        shard.mobs[0].health = 0;
        shard.scale_to_head_count(&[1, 2, 3]);
        assert_eq!(shard.mobs[1].scaled_for, 3, "it takes the count it came at");
        assert_eq!(shard.mobs[0].scaled_for, 1, "the dead");
        let wave = shard.mobs[1].guid;
        let (_, sent) = shard.scale_to_head_count(&[1]);
        assert!(
            !sent
                .iter()
                .any(|e| matches!(e, SessionEvent::ObjectValues { guid, .. } if *guid == wave)),
            "and keeps it through its fight"
        );
        assert_eq!(shard.mobs[1].scaled_for, 3);
    }

    #[test]
    fn the_open_world_and_an_empty_phase_are_never_scaled() {
        let mut open = Shard::new(
            Place {
                map: 0,
                phase: Phase::OpenWorld,
            },
            7,
        );
        open.mobs = run().mobs;
        let (line, sent) = open.scale_to_head_count(&[1, 2]);
        assert!(line.is_none() && sent.is_empty());
        assert!(open.mobs.iter().all(|m| m.unscaled.is_none()));
        let mut shard = run();
        let (line, sent) = shard.scale_to_head_count(&[]);
        assert!(line.is_none() && sent.is_empty());
        assert!(shard.mobs.iter().all(|m| m.unscaled.is_none()));
    }

    #[test]
    fn the_meter_reports_what_each_source_added_and_the_most_names_the_rise() {
        let mut shard = run();
        let m = meter();
        shard.mobs[0].health -= 1;
        let heard = shard.hear_noise(&m, 2, 0);
        let want = Heard {
            first_damage: 1,
            cast: 2,
            ..Heard::default()
        };
        assert_eq!(heard, want);
        assert_eq!(heard.line(&m.lines), Some("cast"));
        shard.run.as_mut().unwrap().elapsed = 30.0;
        let heard = shard.hear_noise(&m, 0, 0);
        assert_eq!(heard.line(&m.lines), Some("clock"));
        let tie = Heard {
            death: 1,
            clock: 1,
            ..Heard::default()
        };
        assert_eq!(tie.line(&m.lines), Some("died"), "a tie goes to the death");
        assert_eq!(Heard::default().line(&m.lines), None);
    }

    #[test]
    fn the_meter_clamps_at_its_max_and_drains_to_nothing() {
        let mut shard = run();
        shard.hear_noise(&meter(), 50, 0);
        assert_eq!(shard.noise(), 20);
        shard.hear_noise(&meter(), 0, 3);
        assert_eq!(shard.noise(), 17);
        shard.hear_noise(&meter(), 0, 99);
        assert_eq!(shard.noise(), 0);
    }
}
