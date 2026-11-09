//! The Hub's friendly NPCs (`content/hub.toml`): units that live in the Crossroads, or out in a
//! Hunting Ground for a quest to find them, each opening the stock window its NPC flags name,
//! and stream in and out with the player's view as the mobs do. Some wander a few yards about
//! where they stand, as vmangos's random movement walks a creature about its spawn point.

use std::sync::OnceLock;

use benilla_protocol::messages::{MonsterMoveFacing, NpcTextBlock, ObjectType, NPC_TEXT_BLOCKS};
use benilla_protocol::{EntityKind, MoveSpeeds, ObjectFields, SessionEvent};
use serde::Deserialize;

use crate::fields::*;
use crate::ground::{ground_under, lift_event, Settling};
use crate::rng::Rng;
use crate::space::flat_distance;
use crate::Host;

/// `HIGHGUID_UNIT`, in a creature guid's top 16 bits.
const HIGHGUID_UNIT: u64 = 0xF130;
/// `UNIT_NPC_FLAGS`.
const UNIT_NPC_FLAGS: u16 = 147;
/// A Hub NPC's health: it is friendly, so nothing ever takes any.
const HEALTH: u32 = 1500;

#[derive(Deserialize)]
pub(crate) struct Npc {
    guid: u32,
    pub(crate) entry: u32,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) subname: String,
    /// What it does for the player (`giver`, `trainer`, `goods`, ...); `tests/content_hub.rs`
    /// checks the set.
    #[serde(default)]
    pub(crate) role: String,
    level: u8,
    pub(crate) display_id: u32,
    faction_template: u32,
    npc_flags: u32,
    /// The map it stands on: Kalimdor unless the content says otherwise.
    #[serde(default = "kalimdor")]
    pub(crate) map: u32,
    pub(crate) position: [f32; 3],
    /// It stands on the host's ground (`crate::ground`); `false` keeps its written Z.
    #[serde(default = "crate::ground::yes")]
    grounded: bool,
    orientation: f32,
    /// A spot (`[x, y]`) it stands facing; when set, it replaces `orientation`.
    #[serde(default)]
    faces: Option<[f32; 2]>,
    /// How far it wanders from `position`, in yards; 0 stands still.
    #[serde(default)]
    wander: f32,
    /// A vendor's list, in the merchant window's order.
    #[serde(default)]
    pub(crate) stock: Vec<u32>,
    /// The Blood Coin vendor: it sells its stock for Blood Coin (`crate::currency_vendor`).
    #[serde(default)]
    pub(crate) blood_coin: bool,
    /// Its gossip menu teaches Fishing (ADR 0018) to a character who does not know it.
    #[serde(default)]
    pub(crate) teaches_fishing: bool,
    /// Its gossip menu teaches Herbalism (ADR 0024) to a character who does not know it.
    #[serde(default)]
    pub(crate) teaches_herbalism: bool,
    /// Its gossip menu opens its kitchen, `cooking::kitchen_of_cook(entry)` (ADR 0025).
    #[serde(default)]
    pub(crate) cook: bool,
    /// Its gossip menu opens its alchemy table, `alchemy::table_of_alchemist(entry)` (ADR 0025).
    #[serde(default)]
    pub(crate) alchemist: bool,
    /// Its gossip menu sells a Herbalist the forgetting of their perks, for money that doubles
    /// with each reset (ADR 0024).
    #[serde(default)]
    pub(crate) resets_perks: bool,
    /// The Legendary Fish it hints at in its gossip menu (#331), to those at an Angler Level.
    #[serde(default)]
    pub(crate) angler_hints: Vec<AnglerHint>,
    /// What its quest list, or its gossip menu, opens with.
    #[serde(default)]
    pub(crate) greeting: String,
    /// A gossip NPC's greeting's `npc_text` id, which `CMSG_NPC_TEXT_QUERY` asks for.
    #[serde(default)]
    pub(crate) text_id: Option<u32>,
    /// An innkeeper's inn: the `AreaTable` id a bind there names.
    #[serde(default)]
    pub(crate) area: Option<u32>,
    /// An innkeeper's inn as a rest area: the yards around `position` it covers.
    #[serde(default)]
    pub(crate) rest_radius: Option<f32>,
    /// The Milestones it waits on (ADR 0034): out of the world, and serving no one, while they
    /// are not met.
    #[serde(flatten)]
    pub(crate) milestones: crate::milestones::Gate,
    /// Its gossip greeting says how far a Tally has come (ADR 0034), in place of `greeting`.
    #[serde(default)]
    pub(crate) readout: Option<Readout>,
}

/// A Tally's stages in words: one greeting before its `after` is reached, one per band of the
/// count while it runs, and one once it is reached. Each stage has its own text id, because the
/// client asks for a text only on its first visit to that id and keeps it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Readout {
    milestone: String,
    before: Stage,
    /// From the lowest `from_pct` up: the last whose `from_pct` the count has passed shows.
    progress: Vec<Stage>,
    reached: Stage,
}

/// One of a readout's greetings.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stage {
    /// In `progress`: the count, in whole percent of the goal, this stage shows from.
    #[serde(default)]
    from_pct: u64,
    text_id: u32,
    text: String,
}

impl Readout {
    fn stages(&self) -> impl Iterator<Item = &Stage> {
        [&self.before, &self.reached]
            .into_iter()
            .chain(&self.progress)
    }

    /// The stage's text id while the Milestones are as `state` holds them.
    fn text_id(&self, state: &crate::milestones::Milestones) -> u32 {
        let Some(m) = state.get(&self.milestone) else {
            return self.before.text_id;
        };
        if state.reached(&m.key) {
            return self.reached.text_id;
        }
        if m.after.as_ref().is_some_and(|a| !state.reached(a)) {
            return self.before.text_id;
        }
        let (count, goal) = state.tally(&m.key).unwrap_or((0, 1));
        let pct = count * 100 / goal;
        self.progress
            .iter()
            .rfind(|s| s.from_pct <= pct)
            .unwrap_or(&self.before)
            .text_id
    }

    /// Panics naming `npc` on a readout of no Tally, or one whose `progress` does not start at 0
    /// or does not rise.
    fn check(&self, npc: &str) {
        assert!(
            crate::milestones::shipped_goal(&self.milestone).is_some(),
            "{npc}'s readout names {}, which has no goal",
            self.milestone
        );
        assert!(
            self.progress.first().is_some_and(|s| s.from_pct == 0),
            "{npc}'s readout progress does not start at 0"
        );
        assert!(
            self.progress
                .windows(2)
                .all(|w| w[0].from_pct < w[1].from_pct),
            "{npc}'s readout progress does not rise"
        );
    }
}

/// A hint at a Legendary Fish (`angler_hints`): its trophy item, and the Angler Level that earns it.
#[derive(Deserialize)]
pub(crate) struct AnglerHint {
    pub(crate) legend: u32,
    pub(crate) min_angler_level: u32,
}

fn kalimdor() -> u32 {
    crate::KALIMDOR
}

#[derive(Deserialize)]
struct File {
    npc: Vec<Npc>,
}

pub(crate) fn npcs() -> &'static [Npc] {
    static NPCS: OnceLock<Vec<Npc>> = OnceLock::new();
    NPCS.get_or_init(|| {
        let file: File =
            toml::from_str(include_str!("../content/hub.toml")).expect("content/hub.toml parses");
        for n in &file.npc {
            n.milestones
                .check(&format!("Hub NPC {} ({})", n.guid, n.name));
        }
        check_readouts(&file.npc);
        file.npc
    })
}

/// Panics on a readout that [`Readout::check`] refuses, or whose text ids repeat within it or
/// clash with any other Hub NPC's.
fn check_readouts(all: &[Npc]) {
    let ids = |n: &Npc| -> Vec<u32> {
        let stages = n.readout.iter().flat_map(Readout::stages);
        n.text_id
            .into_iter()
            .chain(stages.map(|s| s.text_id))
            .collect()
    };
    for (i, n) in all.iter().enumerate() {
        let Some(readout) = &n.readout else { continue };
        readout.check(&n.name);
        let own = ids(n);
        for (j, id) in own.iter().enumerate() {
            assert!(
                !own[..j].contains(id),
                "{}'s readout has text {id} twice",
                n.name
            );
            let other = all
                .iter()
                .enumerate()
                .find(|&(k, m)| k != i && ids(m).contains(id));
            assert!(
                other.is_none(),
                "{}'s readout text {id} is {}'s too",
                n.name,
                other.map_or("", |(_, m)| m.name.as_str())
            );
        }
    }
}

/// The Hub NPC of spawn `low` (its low guid) standing on `map`.
pub(crate) fn spawned(low: u32, map: u32) -> Option<&'static Npc> {
    npcs().iter().find(|n| n.guid == low && n.map == map)
}

/// Whether a player at `at` on the Hub NPC's map has it in view.
pub(crate) fn in_view(npc: &Npc, at: [f32; 3]) -> bool {
    flat_distance(npc.position, at) <= VIEW_YD
}

impl Npc {
    /// What it takes in place of copper, if it is a vendor of that kind.
    pub(crate) fn currency(&self) -> Option<&'static crate::currency_vendor::Currency> {
        if self.blood_coin {
            Some(&crate::currency_vendor::BLOOD_COIN)
        } else {
            None
        }
    }

    pub(crate) fn guid(&self) -> u64 {
        HIGHGUID_UNIT << 48
            | u64::from(self.entry & 0xFF_FFFF) << 24
            | u64::from(self.guid & 0xFF_FFFF)
    }

    /// The way it stands: toward `faces` when set, else `orientation`.
    fn facing(&self) -> f32 {
        match self.faces {
            Some([x, y]) => (y - self.position[1])
                .atan2(x - self.position[0])
                .rem_euclid(std::f32::consts::TAU),
            None => self.orientation,
        }
    }

    /// Whether a right-click opens its gossip menu (`UNIT_NPC_FLAG_GOSSIP`).
    pub(crate) fn gossips(&self) -> bool {
        self.npc_flags & 0x1 != 0
    }

    /// Whether it trades through the merchant window (`UNIT_NPC_FLAG_VENDOR`).
    pub(crate) fn is_vendor(&self) -> bool {
        self.npc_flags & 0x4 != 0
    }

    /// Whether it gives quests (`UNIT_NPC_FLAG_QUESTGIVER`).
    pub(crate) fn gives_quests(&self) -> bool {
        self.npc_flags & 0x2 != 0
    }

    /// Whether it binds the Hearthstone (`UNIT_NPC_FLAG_INNKEEPER`).
    pub(crate) fn is_innkeeper(&self) -> bool {
        self.npc_flags & 0x80 != 0
    }

    /// Whether it repairs gear (`UNIT_NPC_FLAG_REPAIR`).
    pub(crate) fn repairs(&self) -> bool {
        self.npc_flags & 0x4000 != 0
    }

    /// The NPC's create at `position`, facing `orientation`: a friendly unit with its NPC flags.
    fn create(&self, position: [f32; 3], orientation: f32) -> SessionEvent {
        let mut pairs = vec![
            (OBJECT_FIELD_TYPE, TYPE_UNIT),
            (OBJECT_FIELD_ENTRY, self.entry),
            (OBJECT_FIELD_SCALE_X, float(1.0)),
            (UNIT_FIELD_HEALTH, HEALTH),
            (UNIT_FIELD_MAXHEALTH, HEALTH),
            (UNIT_FIELD_LEVEL, u32::from(self.level)),
            (UNIT_FIELD_FACTIONTEMPLATE, self.faction_template),
            // Race 0, warrior, male, mana.
            (UNIT_FIELD_BYTES_0, 1 << 8),
            (UNIT_FIELD_BOUNDINGRADIUS, float(0.389)),
            (UNIT_FIELD_COMBATREACH, float(1.5)),
            (UNIT_FIELD_DISPLAYID, self.display_id),
            (UNIT_FIELD_NATIVEDISPLAYID, self.display_id),
            (UNIT_NPC_FLAGS, self.npc_flags),
        ];
        pairs.extend(guid_pairs(OBJECT_FIELD_GUID, self.guid()));
        SessionEvent::ObjectCreate {
            guid: self.guid(),
            kind: EntityKind::Unit,
            display_id: Some(self.display_id),
            position,
            orientation,
            scale: 1.0,
            speeds: Some(MoveSpeeds {
                walk: WALK_SPEED,
                run: 7.0,
                run_back: 4.5,
                swim: 4.722_222,
                swim_back: 2.5,
                turn_rate: std::f32::consts::PI,
            }),
            mover: None,
            transport_progress: None,
            transport: None,
            spline: None,
            fields: ObjectFields::from_pairs(&pairs).into_created(ObjectType::Unit),
        }
    }
}

/// The NPC with object guid `guid`, while the Milestones `reached` meet its keys.
pub(crate) fn find(guid: u64, reached: &[&str]) -> Option<&'static Npc> {
    npcs()
        .iter()
        .find(|n| n.guid() == guid)
        .filter(|n| n.milestones.met(reached))
}

/// `SMSG_CREATURE_QUERY_RESPONSE` for a Hub NPC's `entry`; `None` for any other entry.
pub(crate) fn creature_name(entry: u32) -> Option<SessionEvent> {
    let npc = npcs().iter().find(|n| n.entry == entry)?;
    Some(SessionEvent::CreatureName {
        entry,
        name: Some(npc.name.clone()),
        subname: Some(npc.subname.clone()),
        // `CREATURE_TYPE_HUMANOID`.
        creature_type: Some(7),
        pet_family: 0,
        rank: 0,
        type_flags: 0,
        display_id: npc.display_id,
        civilian: false,
        racial_leader: false,
    })
}

/// Whether the player at `at` can reach `npc`, wherever within its wander it has walked to.
pub(crate) fn within_reach(npc: &Npc, at: [f32; 3]) -> bool {
    flat_distance(npc.position, at) <= crate::space::INTERACTION_YD + npc.wander
}

/// A Hub NPC's gossip greeting, or a readout's stage, for `CMSG_NPC_TEXT_QUERY`; `None` for any
/// other text.
pub(crate) fn greeting(text_id: u32) -> Option<SessionEvent> {
    let stage = || {
        npcs()
            .iter()
            .filter_map(|n| n.readout.as_ref())
            .flat_map(Readout::stages)
            .find(|s| s.text_id == text_id)
            .map(|s| s.text.clone())
    };
    let text = match npcs().iter().find(|n| n.text_id == Some(text_id)) {
        Some(npc) => npc.greeting.clone(),
        None => stage().or_else(|| crate::quests::talk_text(text_id))?,
    };
    let mut blocks = vec![NpcTextBlock::default(); NPC_TEXT_BLOCKS];
    blocks[0] = NpcTextBlock {
        probability: 1.0,
        male: text.clone(),
        female: text,
    };
    Some(SessionEvent::NpcGreeting { text_id, blocks })
}

impl crate::Realm {
    /// `CMSG_GOSSIP_HELLO` at a Hub NPC with a gossip greeting: its menu. A trainer's carries
    /// his lines and his Advancement Quests, as vmangos's gossip lists a quest giver's quests;
    /// an innkeeper's, the bind and the merchant window; a townsperson's is the greeting alone,
    /// and a readout's greeting is its Tally's stage.
    pub(crate) fn gossip_hello(&mut self, guid: u64) {
        let reached = self.milestones.reached_refs();
        let talks = |n: &Npc| n.text_id.is_some() || n.readout.is_some();
        let Some(npc) = find(guid, &reached).filter(|n| n.gossips() && talks(n) && self.near(n))
        else {
            self.field_gossip_hello(guid);
            return;
        };
        let player = self
            .world
            .as_ref()
            .and_then(|w| self.characters.get(w.guid));
        let window = crate::writs::window(self.wall as u64);
        let quests = player
            .map(|c| crate::quests::gossip_rows(c, npc.entry, window, &reached))
            .unwrap_or_default();
        let mut options = if npc.is_innkeeper() {
            crate::inn::options()
        } else if npc.teaches_fishing {
            player.map_or_else(Vec::new, crate::world::angler_options)
        } else if npc.teaches_herbalism || npc.resets_perks {
            player.map_or_else(Vec::new, |c| crate::world::herbalist_options(npc, c))
        } else if npc.cook {
            crate::world::kitchen_options()
        } else if npc.alchemist {
            crate::world::alchemy_options()
        } else {
            crate::trainer::options(npc)
        };
        options.extend(player.map_or_else(Vec::new, |c| crate::world::hint_options(npc, c)));
        options.extend(player.map_or_else(Vec::new, |c| crate::quests::talk_options(c, npc.entry)));
        let text_id = match &npc.readout {
            Some(readout) => readout.text_id(&self.milestones),
            None => npc.text_id.unwrap_or_default(),
        };
        self.events.push(SessionEvent::GossipMenu {
            npc: npc.guid(),
            text_id,
            options,
            quests,
        });
    }

    /// `CMSG_GOSSIP_SELECT_OPTION`: a line of a trainer's or an innkeeper's menu.
    pub(crate) fn gossip_select(&mut self, guid: u64, option: u32) {
        let npc = find(guid, &self.milestones.reached_refs());
        match npc {
            Some(npc) if option >= crate::quests::TALK_OPTION => {
                if self.near(npc) {
                    self.talk_select(guid, npc.entry, option);
                }
            }
            Some(npc) if crate::world::is_hint_option(option) => self.hint_select(npc, option),
            Some(npc) if npc.is_innkeeper() => self.inn_select(npc, option),
            Some(npc) if npc.teaches_fishing => self.angler_select(npc, option),
            Some(npc) if npc.teaches_herbalism || npc.resets_perks => {
                self.herbalist_select(npc, option)
            }
            Some(npc) if npc.cook => self.kitchen_select(npc, option),
            Some(npc) if npc.alchemist => self.alchemy_select(npc, option),
            Some(_) => self.trainer_select(guid, option),
            None => self.field_gossip_select(guid, option),
        }
    }

    /// Whether the character in the world can reach `npc`, and the Milestones meet its keys.
    pub(crate) fn near(&self, npc: &Npc) -> bool {
        npc.milestones.met(&self.milestones.reached_refs())
            && self
                .world
                .as_ref()
                .and_then(|w| self.characters.get(w.guid))
                .is_some_and(|c| c.map == npc.map && within_reach(npc, c.position))
    }
}

/// A Hub NPC's walking speed, yards per second (the stock 2.5).
const WALK_SPEED: f32 = 2.5;
/// How long a wanderer stands between walks, in seconds: a few, as vmangos's random movement
/// pauses at each point.
const WANDER_PAUSE: (f32, f32) = (5.0, 12.0);

/// How far the player sees a Hub NPC, and how far before it leaves the world (as the mobs).
const VIEW_YD: f32 = crate::population::VIEW_YD;
const FORGET_YD: f32 = VIEW_YD + 10.0;

/// Which Hub NPCs are in the world, and where each wanderer has walked to.
#[derive(Default)]
pub(crate) struct InView {
    present: Vec<Option<Standing>>,
    /// The wanderers' own dice, so their walks never shift the realm's fights.
    rng: Rng,
}

/// An NPC in the world: where it is, and what it is doing there.
struct Standing {
    at: [f32; 3],
    /// While it still asks for its ground; `None` once the window has passed, or for one that
    /// keeps its written Z.
    settling: Option<Settling>,
    walk: Option<Walk>,
    /// Seconds until a wanderer sets off again.
    idle: f32,
}

/// A walk under way: from, to, how long it takes and how far along it is.
struct Walk {
    from: [f32; 3],
    to: [f32; 3],
    duration: f32,
    elapsed: f32,
}

impl InView {
    /// Brings the NPCs within view of any of `players` into the world, standing on the host's
    /// ground, lets go those far from them all, and walks the wanderers in view on by `dt`
    /// seconds. Each stands on its own map only, none on land closed while the Milestones
    /// `reached` are, and none whose keys they do not meet; one that stops meeting them leaves.
    pub(crate) fn update(
        &mut self,
        dt: f32,
        players: &[[f32; 3]],
        map: u32,
        reached: &[&str],
        host: &dyn Host,
        events: &mut Vec<SessionEvent>,
    ) {
        self.update_among(npcs(), dt, players, map, reached, host, events);
    }

    /// [`Self::update`] over the NPC rows `all`.
    #[allow(clippy::too_many_arguments)]
    fn update_among(
        &mut self,
        all: &[Npc],
        dt: f32,
        players: &[[f32; 3]],
        map: u32,
        reached: &[&str],
        host: &dyn Host,
        events: &mut Vec<SessionEvent>,
    ) {
        let rng = &mut self.rng;
        self.present.resize_with(all.len(), || None);
        let mut gone = Vec::new();
        for (npc, present) in all.iter().zip(&mut self.present) {
            let [x, y, _] = npc.position;
            let d = if map == npc.map
                && !crate::closed_at(map, x, y, reached)
                && npc.milestones.met(reached)
            {
                players
                    .iter()
                    .map(|&p| flat_distance(npc.position, p))
                    .fold(f32::INFINITY, f32::min)
            } else {
                f32::INFINITY
            };
            match present {
                None if d <= VIEW_YD => {
                    let [x, y, z] = npc.position;
                    let ground = ground_under(host, map, npc.position, npc.grounded);
                    let at = [x, y, ground.unwrap_or(z)];
                    events.push(npc.create(at, npc.facing()));
                    *present = Some(Standing {
                        at,
                        // One that keeps its written Z has nothing to settle onto.
                        settling: npc.grounded.then(|| Settling::new(ground.is_some())),
                        walk: None,
                        idle: rng.frand(WANDER_PAUSE.0, WANDER_PAUSE.1),
                    });
                }
                Some(_) if d > FORGET_YD => {
                    gone.push(npc.guid());
                    *present = None;
                }
                Some(s) => {
                    s.settle(npc, dt, map, host, events);
                    if npc.wander > 0.0 {
                        s.wander(npc, dt, map, host, rng, events);
                    }
                }
                None => {}
            }
        }
        if !gone.is_empty() {
            events.push(SessionEvent::ObjectsRemoved(gone));
        }
    }
}

impl InView {
    /// The NPCs in the world within view of `me`: each one's guid and its create where it stands.
    pub(crate) fn in_view(&self, me: [f32; 3]) -> Vec<(u64, SessionEvent)> {
        npcs()
            .iter()
            .zip(&self.present)
            .filter_map(|(npc, s)| s.as_ref().map(|s| (npc, s)))
            .filter(|(_, s)| flat_distance(s.at, me) <= VIEW_YD)
            .map(|(npc, s)| (npc.guid(), npc.create(s.at, npc.facing())))
            .collect()
    }
}

impl Standing {
    /// A grounded NPC asks again for its ground from its content height for a while after it is
    /// placed ([`Settling`]): one that spawned before the ground streamed in, often under the
    /// terrain, is set onto it, and one placed on the terrain before a building's floor over it
    /// streamed in is lifted onto the floor (#547).
    fn settle(
        &mut self,
        npc: &Npc,
        dt: f32,
        map: u32,
        host: &dyn Host,
        events: &mut Vec<SessionEvent>,
    ) {
        let Some(settling) = &mut self.settling else {
            return;
        };
        if !settling.tick(dt) {
            self.settling = None;
            return;
        }
        if self.walk.is_some() {
            return;
        }
        if let Some(ground) = settling.ask(host, map, self.at, npc.position[2]) {
            let to = [self.at[0], self.at[1], ground];
            events.push(lift_event(npc.guid(), self.at, to, npc.facing()));
            self.at = to;
        }
    }

    /// Walks on, or after a pause sets off for another spot within `npc`'s wander.
    fn wander(
        &mut self,
        npc: &Npc,
        dt: f32,
        map: u32,
        host: &dyn Host,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) {
        if let Some(w) = &mut self.walk {
            w.elapsed += dt;
            let t = (w.elapsed / w.duration).min(1.0);
            self.at = std::array::from_fn(|i| w.from[i] + (w.to[i] - w.from[i]) * t);
            if t >= 1.0 {
                self.walk = None;
                self.idle = rng.frand(WANDER_PAUSE.0, WANDER_PAUSE.1);
            }
            return;
        }
        self.idle -= dt;
        if self.idle > 0.0 {
            return;
        }
        self.idle = rng.frand(WANDER_PAUSE.0, WANDER_PAUSE.1);
        // A spot spread evenly over the wander's disc.
        let angle = rng.frand(0.0, std::f32::consts::TAU);
        let r = npc.wander * rng.frand(0.0, 1.0).sqrt();
        let [hx, hy, hz] = npc.position;
        let (x, y) = (hx + r * angle.cos(), hy + r * angle.sin());
        let to = [x, y, host.ground_height(map, x, y, hz + 2.0).unwrap_or(hz)];
        // Never walk through a catapult, a wall or a cart: a blocked spot waits for the next try.
        let eye = |p: [f32; 3]| [p[0], p[1], p[2] + 1.0];
        if !host.line_of_sight(map, eye(self.at), eye(to)) {
            return;
        }
        let duration = flat_distance(self.at, to) / WALK_SPEED;
        if duration <= 0.0 {
            return;
        }
        events.push(SessionEvent::MonsterMove {
            guid: npc.guid(),
            transport: None,
            start: self.at,
            spline_id: 0,
            path: vec![self.at, to],
            facing: MonsterMoveFacing::None,
            stop: false,
            duration_ms: crate::clock::ms(duration),
            flying: false,
            run_mode: false,
        });
        self.walk = Some(Walk {
            from: self.at,
            to,
            duration,
            elapsed: 0.0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Flat;
    impl Host for Flat {
        fn ground_height(&self, _: u32, _: f32, _: f32, z: f32) -> Option<f32> {
            Some(z)
        }
        fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
            true
        }
        fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
            None
        }
    }

    /// Two test NPCs on the Abbey steps: one after the Anchor breaks, one until it does.
    fn rows() -> Vec<Npc> {
        let row = |guid: u32, key: &str| {
            format!(
                "[[npc]]\nguid = {guid}\nentry = {guid}\nname = \"Test\"\nlevel = 1\n\
                 display_id = 1\nfaction_template = 35\nnpc_flags = 0\nmap = 0\n\
                 position = [-8912.0, -140.0, 82.0]\norientation = 0.0\n\
                 {key} = \"ilgalar_anchor_broken\"\n"
            )
        };
        let text = row(990901, "after_milestone") + &row(990902, "until_milestone");
        toml::from_str::<File>(&text).unwrap().npc
    }

    /// The guids of the creates and removes one look at the steps sends.
    fn look(view: &mut InView, all: &[Npc], reached: &[&str]) -> (Vec<u64>, Vec<u64>) {
        let mut events = Vec::new();
        let at = [-8910.0, -140.0, 82.0];
        view.update_among(all, 0.1, &[at], 0, reached, &Flat, &mut events);
        let mut made = Vec::new();
        let mut gone = Vec::new();
        for e in events {
            match e {
                SessionEvent::ObjectCreate { guid, .. } => made.push(guid),
                SessionEvent::ObjectsRemoved(g) => gone.extend(g),
                _ => {}
            }
        }
        (made, gone)
    }

    #[test]
    fn an_npc_comes_and_goes_with_its_milestone() {
        let all = rows();
        let (after, until) = (all[0].guid(), all[1].guid());
        let mut view = InView::default();
        assert_eq!(look(&mut view, &all, &[]), (vec![until], vec![]));
        let anchor = ["ilgalar_anchor_broken"];
        assert_eq!(look(&mut view, &all, &anchor), (vec![after], vec![until]));
        assert_eq!(look(&mut view, &all, &anchor), (vec![], vec![]));
    }
}
