//! The Hub's friendly NPCs (`content/hub.toml`): units that live in the Crossroads, or out in a
//! Hunting Ground for a quest to find them, each opening the stock window its NPC flags name,
//! and stream in and out with the player's view as the mobs do. Some wander a few yards about
//! where they stand, as vmangos's random movement walks a creature about its spawn point.

use std::sync::OnceLock;

use benilla_protocol::messages::{MonsterMoveFacing, NpcTextBlock, ObjectType, NPC_TEXT_BLOCKS};
use benilla_protocol::{EntityKind, MoveSpeeds, ObjectFields, SessionEvent};
use serde::Deserialize;

use crate::fields::*;
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
    name: String,
    #[serde(default)]
    subname: String,
    level: u8,
    display_id: u32,
    faction_template: u32,
    npc_flags: u32,
    pub(crate) position: [f32; 3],
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
    /// The trainer who sells the talent respec (`crate::trainer`).
    #[serde(default)]
    respec: bool,
    /// The Token vendor: it sells the buyer's band of greens for Tokens (`crate::token_vendor`).
    #[serde(default)]
    pub(crate) tokens: bool,
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
        file.npc
    })
}

impl Npc {
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

    /// Whether it trains (`UNIT_NPC_FLAG_TRAINER`): one of the four Tier 1 trainers.
    pub(crate) fn trains(&self) -> bool {
        self.npc_flags & 0x10 != 0
    }

    /// Whether it sells the talent respec: Uzzek, the Vanguard Trainer, alone.
    pub(crate) fn sells_respec(&self) -> bool {
        self.trains() && self.respec
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

/// The NPC with object guid `guid`.
pub(crate) fn find(guid: u64) -> Option<&'static Npc> {
    npcs().iter().find(|n| n.guid() == guid)
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

/// A Hub NPC's gossip greeting, for `CMSG_NPC_TEXT_QUERY`; `None` for any other text.
pub(crate) fn greeting(text_id: u32) -> Option<SessionEvent> {
    let npc = npcs().iter().find(|n| n.text_id == Some(text_id))?;
    let mut blocks = vec![NpcTextBlock::default(); NPC_TEXT_BLOCKS];
    blocks[0] = NpcTextBlock {
        probability: 1.0,
        male: npc.greeting.clone(),
        female: npc.greeting.clone(),
    };
    Some(SessionEvent::NpcGreeting { text_id, blocks })
}

impl crate::Realm {
    /// `CMSG_GOSSIP_HELLO` at a Hub NPC with a gossip greeting: its menu. A trainer's carries
    /// his lines and his Advancement Quests, as vmangos's gossip lists a quest giver's quests;
    /// an innkeeper's, the bind and the merchant window; a townsperson's is the greeting alone.
    pub(crate) fn gossip_hello(&mut self, guid: u64) {
        let Some(npc) = find(guid).filter(|n| n.gossips() && n.text_id.is_some() && self.near(n))
        else {
            return;
        };
        let quests = self
            .world
            .as_ref()
            .and_then(|w| self.characters.get(w.guid))
            .map(|c| crate::quests::gossip_rows(c, npc.entry))
            .unwrap_or_default();
        self.events.push(SessionEvent::GossipMenu {
            npc: npc.guid(),
            text_id: npc.text_id.unwrap_or_default(),
            options: if npc.is_innkeeper() {
                crate::inn::options()
            } else {
                crate::trainer::options(npc)
            },
            quests,
        });
    }

    /// `CMSG_GOSSIP_SELECT_OPTION`: a line of a trainer's or an innkeeper's menu.
    pub(crate) fn gossip_select(&mut self, guid: u64, option: u32) {
        match find(guid) {
            Some(npc) if npc.is_innkeeper() => self.inn_select(npc, option),
            Some(_) => self.trainer_select(guid, option),
            None => {}
        }
    }

    /// Whether the character in the world can reach `npc`.
    pub(crate) fn near(&self, npc: &Npc) -> bool {
        self.world
            .as_ref()
            .and_then(|w| self.characters.get(w.guid))
            .is_some_and(|c| c.map == crate::KALIMDOR && within_reach(npc, c.position))
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
    /// Whether `at` is on the host's ground; false when it spawned before the terrain streamed in.
    grounded: bool,
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
    /// seconds. They stand on Kalimdor only.
    pub(crate) fn update(
        &mut self,
        dt: f32,
        players: &[[f32; 3]],
        map: u32,
        host: &dyn Host,
        events: &mut Vec<SessionEvent>,
    ) {
        let rng = &mut self.rng;
        let all = npcs();
        self.present.resize_with(all.len(), || None);
        let mut gone = Vec::new();
        for (npc, present) in all.iter().zip(&mut self.present) {
            let d = if map == crate::KALIMDOR {
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
                    let ground = host.ground_height(map, x, y, z + 2.0);
                    let at = [x, y, ground.unwrap_or(z)];
                    events.push(npc.create(at, npc.facing()));
                    *present = Some(Standing {
                        at,
                        grounded: ground.is_some(),
                        walk: None,
                        idle: rng.frand(WANDER_PAUSE.0, WANDER_PAUSE.1),
                    });
                }
                Some(_) if d > FORGET_YD => {
                    gone.push(npc.guid());
                    *present = None;
                }
                Some(s) => {
                    s.settle(npc, map, host, events);
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
    /// An NPC that spawned before the ground streamed in stood at its spawn's height, often under
    /// the terrain; once the ground is there, lifts it onto it.
    fn settle(&mut self, npc: &Npc, map: u32, host: &dyn Host, events: &mut Vec<SessionEvent>) {
        if self.grounded || self.walk.is_some() {
            return;
        }
        let [x, y, z] = self.at;
        let Some(ground) = host.ground_height(map, x, y, z + 2.0) else {
            return;
        };
        self.grounded = true;
        if (ground - z).abs() < 0.05 {
            return;
        }
        let to = [x, y, ground];
        events.push(SessionEvent::MonsterMove {
            guid: npc.guid(),
            transport: None,
            start: self.at,
            spline_id: 0,
            path: vec![self.at, to],
            facing: MonsterMoveFacing::Angle(npc.facing()),
            stop: false,
            duration_ms: 1,
            flying: false,
            run_mode: false,
        });
        self.at = to;
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
