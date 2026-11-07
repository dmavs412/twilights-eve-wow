//! Carts (spec #560): one wagon per route in `content/carts.toml`, back and forth all day on the
//! realm's wall clock. Its pose is a function of the time counted from the Unix epoch, so every
//! realm and every client agree, nothing is saved, and it runs with nobody aboard (ADR 0009).

use std::sync::OnceLock;

use benilla_protocol::guid::{HIGH_TRANSPORT, HIGH_UNIT};
use benilla_protocol::messages::ObjectType;
use benilla_protocol::wire::Vector3d;
use benilla_protocol::{
    CartStop, EntityKind, MoveSpeeds, MoverState, ObjectFields, RelayVerb, SessionEvent,
    TransportPose,
};
use serde::Deserialize;

use crate::fields::*;
use crate::phased::{gameobject_create, GameObject};

/// `GAMEOBJECT_TYPE_TRANSPORT`: the wagon is a transport its riders' poses are local to.
pub(crate) const TRANSPORT: u32 = 11;
/// `MOVEFLAG_FORWARD`.
const MOVE_FORWARD: u32 = 0x1;
/// `MOVEFLAG_ONTRANSPORT`: the unit's pose is local to the transport it rides.
pub(crate) const ON_TRANSPORT: u32 = 0x0200_0000;
/// The friendly faction the Hub NPCs carry: nobody can attack the horse or the driver.
const FRIENDLY: u32 = 35;
/// The horse's and driver's health: nothing ever takes any.
const HEALTH: u32 = 1500;
/// How far to the wagon's right a rider stands up, in yards.
const STEP_DOWN_YD: f32 = 2.0;
/// `UNIT_FIELD_BYTES_1`: its low byte is the stand state.
const UNIT_FIELD_BYTES_1: u16 = 138;
/// `UNIT_STAND_STATE_SIT`: the driver sits on the front bench.
const SITTING: u32 = 1;

/// The `[cart]` table: what every route's wagon shares.
#[derive(Deserialize)]
pub struct Cart {
    pub speed_yd_s: f32,
    pub dwell_s: f32,
    pub seats: u32,
    pub use_range_yd: f32,
    pub wagon_entry: u32,
    pub wagon_display: u32,
    pub horse_entry: u32,
    pub horse_display: u32,
    pub horse_name: String,
    pub horse_ahead_yd: f32,
    pub driver_entry: u32,
    pub driver_display: u32,
    pub driver_name: String,
    /// The driver's seat in the wagon's frame: x forward, y left, z up.
    pub driver_offset: [f32; 3],
    /// The riders' seats in the wagon's frame, one per seat.
    pub seat_offsets: Vec<[f32; 3]>,
    /// What a locked server says, at the wagon and on the map.
    pub locked_line: String,
}

#[derive(Deserialize)]
struct StopRow {
    key: String,
    name: String,
    position: [f32; 3],
}

#[derive(Deserialize)]
struct RouteRow {
    key: String,
    name: String,
    /// The map it runs on; every shipped route is in the Eastern Kingdoms.
    #[serde(default)]
    map: u32,
    stops: Vec<StopRow>,
    path: Vec<[f32; 3]>,
}

#[derive(Deserialize)]
struct File {
    cart: Cart,
    route: Vec<RouteRow>,
}

/// One route and its timetable.
pub struct Route {
    /// Its place in the file, from 0: the counter of its units' guids, less one.
    pub index: usize,
    pub key: String,
    pub name: String,
    pub map: u32,
    pub stops: Vec<CartStop>,
    pub path: Vec<[f32; 3]>,
    /// The yards along the path to each point.
    along: Vec<f32>,
    pub length_yd: f32,
    /// One way along the path.
    pub leg_ms: u64,
    /// Standing at each end.
    pub dwell_ms: u64,
    /// Out and back: `2 * (leg_ms + dwell_ms)`.
    pub period_ms: u64,
}

/// Where a wagon is in its day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartState {
    /// Standing at the route's stop of this index.
    Standing(usize),
    Rolling,
}

/// A wagon's pose at a moment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub point: [f32; 3],
    /// The way it faces, radians from north (WoW's x) toward the west.
    pub yaw: f32,
    pub state: CartState,
}

fn file() -> &'static (Cart, Vec<Route>) {
    static FILE: OnceLock<(Cart, Vec<Route>)> = OnceLock::new();
    FILE.get_or_init(|| {
        let file: File = toml::from_str(include_str!("../content/carts.toml"))
            .expect("content/carts.toml parses");
        let cart = file.cart;
        let routes = file
            .route
            .into_iter()
            .enumerate()
            .map(|(index, r)| route(&cart, index, r))
            .collect();
        (cart, routes)
    })
}

fn route(cart: &Cart, index: usize, r: RouteRow) -> Route {
    assert!(r.path.len() >= 2, "route {} has no path", r.key);
    let along = along(&r.path);
    let length_yd = along[along.len() - 1];
    let leg_ms = (f64::from(length_yd) / f64::from(cart.speed_yd_s) * 1000.0).round() as u64;
    let dwell_ms = (f64::from(cart.dwell_s) * 1000.0).round() as u64;
    let stops = r
        .stops
        .into_iter()
        .map(|s| {
            let point_index = (0..r.path.len())
                .min_by(|&a, &b| {
                    dist(r.path[a], s.position).total_cmp(&dist(r.path[b], s.position))
                })
                .unwrap_or(0);
            CartStop {
                key: s.key,
                name: s.name,
                position: s.position,
                point_index,
            }
        })
        .collect();
    Route {
        index,
        key: r.key,
        name: r.name,
        map: r.map,
        stops,
        path: r.path,
        along,
        length_yd,
        leg_ms,
        dwell_ms,
        period_ms: 2 * (leg_ms + dwell_ms),
    }
}

/// The cart numbers every route shares.
pub fn cart() -> &'static Cart {
    &file().0
}

/// The shipped routes, in file order.
pub fn routes() -> &'static [Route] {
    &file().1
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The yards along `path` to each of its points, from 0.
pub fn along(path: &[[f32; 3]]) -> Vec<f32> {
    let mut along = vec![0.0];
    for w in path.windows(2) {
        along.push(along[along.len() - 1] + dist(w[0], w[1]));
    }
    along
}

/// The point `d` yards along `path` and the heading of the segment it is on; `along` is
/// [`along`] of `path`.
fn at(path: &[[f32; 3]], along: &[f32], d: f32) -> ([f32; 3], f32) {
    let i = along
        .windows(2)
        .position(|w| d <= w[1])
        .unwrap_or(along.len() - 2);
    let (a, b) = (path[i], path[i + 1]);
    let span = along[i + 1] - along[i];
    let f = if span > 0.0 {
        ((d - along[i]) / span).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let point = std::array::from_fn(|k| a[k] + (b[k] - a[k]) * f);
    (point, heading(a, b))
}

impl Route {
    /// The last stop's index.
    fn last_stop(&self) -> usize {
        self.stops.len().saturating_sub(1)
    }

    /// Its wagon's guid: a `HIGHGUID_TRANSPORT` GameObject.
    pub fn wagon_guid(&self) -> u64 {
        guid(HIGH_TRANSPORT, cart().wagon_entry, self.index)
    }

    /// Its horse's guid.
    pub fn horse_guid(&self) -> u64 {
        guid(HIGH_UNIT, cart().horse_entry, self.index)
    }

    /// Its driver's guid.
    pub fn driver_guid(&self) -> u64 {
        guid(HIGH_UNIT, cart().driver_entry, self.index)
    }
}

fn guid(high: u16, entry: u32, index: usize) -> u64 {
    u64::from(high) << 48 | u64::from(entry & 0xFF_FFFF) << 24 | (index as u64 + 1)
}

/// The heading from `a` to `b` on the ground, in `[0, TAU)`.
fn heading(a: [f32; 3], b: [f32; 3]) -> f32 {
    (b[1] - a[1])
        .atan2(b[0] - a[0])
        .rem_euclid(std::f32::consts::TAU)
}

/// `route`'s wagon at `wall_ms`, milliseconds since the Unix epoch. In each period it stands at
/// the first stop for `dwell_ms`, rolls the path forward, stands at the last stop and rolls it
/// back. Standing, it faces the way it leaves.
pub fn pose(route: &Route, wall_ms: u64) -> Pose {
    sample(
        &route.path,
        &route.along,
        route.dwell_ms,
        route.leg_ms,
        route.last_stop(),
        wall_ms % route.period_ms.max(1),
        false,
    )
}

/// A wagon's pose `t` ms into its period, `t < 2 * (leg_ms + dwell_ms)`: standing `dwell_ms` at
/// the first stop, rolling `path` in `leg_ms`, standing at the last stop (`last_stop`, its index)
/// and rolling back. `along` is [`along`] of `path`. The realm's [`pose`] and the client's road
/// drive (#565) both sample it, so a rider and a bystander agree where the wagon is. With
/// `backs_out` (a Deeprun Tram car, ADR 0033) it never turns round: standing at the last stop and
/// rolling back, it keeps facing the way it arrived.
pub fn sample(
    path: &[[f32; 3]],
    along: &[f32],
    dwell_ms: u64,
    leg_ms: u64,
    last_stop: usize,
    t: u64,
    backs_out: bool,
) -> Pose {
    let (dwell, leg) = (dwell_ms, leg_ms.max(1));
    let length_yd = along[along.len() - 1];
    let at = |d: f32| at(path, along, d);
    let back = |(point, yaw): ([f32; 3], f32)| {
        if backs_out {
            (point, yaw)
        } else {
            (point, (yaw + std::f32::consts::PI) % std::f32::consts::TAU)
        }
    };
    let (state, (point, yaw)) = if t < dwell {
        (CartState::Standing(0), at(0.0))
    } else if t < dwell + leg {
        let d = (t - dwell) as f32 / leg as f32 * length_yd;
        (CartState::Rolling, at(d))
    } else if t < 2 * dwell + leg {
        (CartState::Standing(last_stop), back(at(length_yd)))
    } else {
        let d = (1.0 - (t - 2 * dwell - leg) as f32 / leg as f32) * length_yd;
        (CartState::Rolling, back(at(d)))
    };
    Pose { point, yaw, state }
}

/// Whether the timetable runs: with the Carts unlock off every wagon stands at its first stop.
pub(crate) fn pose_now(route: &Route, wall_ms: u64, on: bool) -> Pose {
    if on {
        pose(route, wall_ms)
    } else {
        Pose {
            state: CartState::Standing(0),
            ..pose(route, 0)
        }
    }
}

/// `offset` in the frame of a wagon at `pose`, in world coordinates.
pub(crate) fn world_at(pose: &Pose, offset: [f32; 3]) -> [f32; 3] {
    let (sin, cos) = pose.yaw.sin_cos();
    [
        pose.point[0] + offset[0] * cos - offset[1] * sin,
        pose.point[1] + offset[0] * sin + offset[1] * cos,
        pose.point[2] + offset[2],
    ]
}

/// Where a rider stands up beside a wagon at `pose`: [`STEP_DOWN_YD`] to its right, at the
/// path's height for the host's ground to settle.
pub(crate) fn step_down(pose: &Pose) -> [f32; 3] {
    world_at(pose, [0.0, -STEP_DOWN_YD, 0.0])
}

/// `offset` as a pose in the frame of `wagon`.
pub(crate) fn transport_pose(wagon: u64, offset: [f32; 3]) -> TransportPose {
    TransportPose {
        guid: wagon,
        pos: Vector3d {
            x: offset[0],
            y: offset[1],
            z: offset[2],
        },
        orientation: 0.0,
    }
}

/// The wall clock as a 32-bit ms stamp, for relays.
fn stamp(wall_ms: u64) -> u32 {
    wall_ms as u32
}

/// `route`, for the client to sample the wagon's timetable from.
pub(crate) fn route_event(route: &Route) -> SessionEvent {
    SessionEvent::CartRoute {
        key: route.key.clone(),
        name: route.name.clone(),
        map: route.map,
        wagon_guid: route.wagon_guid(),
        horse_guid: route.horse_guid(),
        driver_guid: route.driver_guid(),
        points: route.path.clone(),
        speed_yd_s: cart().speed_yd_s,
        dwell_ms: route.dwell_ms as u32,
        period_ms: route.period_ms as u32,
        stops: route.stops.clone(),
        backs_out: false,
    }
}

/// The creates of `route`'s wagon, horse and driver at `wall_ms`. With `on` the wagon is a
/// transport anchored at its place in the period, and the horse and driver ride its frame; off,
/// all three stand at the first stop as plain objects.
pub(crate) fn creates(route: &Route, wall_ms: u64, on: bool) -> Vec<(u64, SessionEvent)> {
    let cart = cart();
    let pose = pose_now(route, wall_ms, on);
    let wagon = route.wagon_guid();
    let mut wagon_create = gameobject_create(&GameObject {
        guid: wagon,
        entry: cart.wagon_entry,
        display_id: cart.wagon_display,
        type_id: TRANSPORT,
        flags: 0,
        faction: 0,
        position: pose.point,
        orientation: pose.yaw,
    });
    if let (
        true,
        SessionEvent::ObjectCreate {
            transport_progress, ..
        },
    ) = (on, &mut wagon_create)
    {
        *transport_progress = Some((wall_ms % route.period_ms.max(1)) as u32);
    }
    let rolling = on && pose.state == CartState::Rolling;
    let horse = Rider {
        guid: route.horse_guid(),
        entry: cart.horse_entry,
        display: cart.horse_display,
        offset: [cart.horse_ahead_yd, 0.0, 0.0],
        forward: rolling,
        stand_state: 0,
    };
    let driver = Rider {
        guid: route.driver_guid(),
        entry: cart.driver_entry,
        display: cart.driver_display,
        offset: cart.driver_offset,
        forward: false,
        stand_state: SITTING,
    };
    vec![
        (wagon, wagon_create),
        (horse.guid, horse.create(&pose, wagon, on)),
        (driver.guid, driver.create(&pose, wagon, on)),
    ]
}

/// The horse's relay when its wagon starts or stops rolling: forward while it rolls, still
/// while it stands, so the client walks or rests it.
pub(crate) fn horse_relay(route: &Route, wall_ms: u64) -> SessionEvent {
    let cart = cart();
    let pose = pose(route, wall_ms);
    let offset = [cart.horse_ahead_yd, 0.0, 0.0];
    let forward = if pose.state == CartState::Rolling {
        MOVE_FORWARD
    } else {
        0
    };
    SessionEvent::UnitMove {
        guid: route.horse_guid(),
        position: world_at(&pose, offset),
        orientation: pose.yaw,
        flags: ON_TRANSPORT | forward,
        pitch: 0.0,
        time: stamp(wall_ms),
        verb: RelayVerb::Pose,
        fall_time: 0,
        jump: None,
        transport: Some(transport_pose(route.wagon_guid(), offset)),
    }
}

/// A unit that rides a wagon: its horse or its driver.
struct Rider {
    guid: u64,
    entry: u32,
    display: u32,
    offset: [f32; 3],
    forward: bool,
    /// Its `UNIT_FIELD_BYTES_1` stand state: the driver sits, the horse stands.
    stand_state: u32,
}

impl Rider {
    /// Its create beside the wagon at `pose`: in the frame of `wagon` while `on`, else where it
    /// stands in the world.
    fn create(&self, pose: &Pose, wagon: u64, on: bool) -> SessionEvent {
        let mut pairs = vec![
            (OBJECT_FIELD_TYPE, TYPE_UNIT),
            (OBJECT_FIELD_ENTRY, self.entry),
            (OBJECT_FIELD_SCALE_X, float(1.0)),
            (UNIT_FIELD_HEALTH, HEALTH),
            (UNIT_FIELD_MAXHEALTH, HEALTH),
            (UNIT_FIELD_LEVEL, 1),
            (UNIT_FIELD_FACTIONTEMPLATE, FRIENDLY),
            // Race 0, warrior, male, mana.
            (UNIT_FIELD_BYTES_0, 1 << 8),
            (UNIT_FIELD_BOUNDINGRADIUS, float(0.389)),
            (UNIT_FIELD_COMBATREACH, float(1.5)),
            (UNIT_FIELD_DISPLAYID, self.display),
            (UNIT_FIELD_NATIVEDISPLAYID, self.display),
            (UNIT_FIELD_BYTES_1, self.stand_state),
        ];
        pairs.extend(guid_pairs(OBJECT_FIELD_GUID, self.guid));
        let flags = if self.forward { MOVE_FORWARD } else { 0 };
        let (transport, flags) = if on {
            (
                Some(transport_pose(wagon, self.offset)),
                flags | ON_TRANSPORT,
            )
        } else {
            (None, flags)
        };
        SessionEvent::ObjectCreate {
            guid: self.guid,
            kind: EntityKind::Unit,
            display_id: Some(self.display),
            position: world_at(pose, self.offset),
            orientation: pose.yaw,
            scale: 1.0,
            speeds: Some(MoveSpeeds {
                walk: 2.5,
                run: cart().speed_yd_s,
                run_back: 4.5,
                swim: 4.722_222,
                swim_back: 2.5,
                turn_rate: std::f32::consts::PI,
            }),
            mover: Some(MoverState { flags, pitch: 0.0 }),
            transport_progress: None,
            transport,
            spline: None,
            fields: ObjectFields::from_pairs(&pairs).into_created(ObjectType::Unit),
        }
    }
}

/// The route whose wagon is `guid`.
pub(crate) fn by_wagon(guid: u64) -> Option<&'static Route> {
    routes().iter().find(|r| r.wagon_guid() == guid)
}

/// `SMSG_CREATURE_QUERY_RESPONSE` for the horse's or the driver's `entry`; `None` for any other.
pub(crate) fn creature_name(entry: u32) -> Option<SessionEvent> {
    let cart = cart();
    let (name, display, creature_type) = if entry == cart.horse_entry {
        // `CREATURE_TYPE_BEAST`.
        (&cart.horse_name, cart.horse_display, 1)
    } else if entry == cart.driver_entry {
        // `CREATURE_TYPE_HUMANOID`.
        (&cart.driver_name, cart.driver_display, 7)
    } else {
        return None;
    };
    Some(SessionEvent::CreatureName {
        entry,
        name: Some(name.clone()),
        subname: Some(String::new()),
        creature_type: Some(creature_type),
        pet_family: 0,
        rank: 0,
        type_flags: 0,
        display_id: display,
        civilian: false,
        racial_leader: false,
    })
}

/// The wagons on one open-world map: what the shard streams, and the horses' last state.
#[derive(Default)]
pub(crate) struct Fleet {
    routes: Vec<&'static Route>,
    /// The wall clock and the Carts unlock as of the last update; `None` before the first.
    now: Option<(u64, bool)>,
    /// Each route's wagon was rolling at the last update.
    rolling: Vec<bool>,
}

impl Fleet {
    /// The wagons of the routes on `map`.
    pub(crate) fn on_map(map: u32) -> Self {
        let routes: Vec<&'static Route> = routes().iter().filter(|r| r.map == map).collect();
        Self {
            rolling: vec![false; routes.len()],
            routes,
            now: None,
        }
    }

    /// Moves the wagons to `wall_ms` with the Carts unlock `on`: a horse whose wagon started or
    /// stopped is relayed; an unlock that changed removes every unit, so each client re-creates
    /// them in their new form.
    pub(crate) fn update(&mut self, wall_ms: u64, on: bool) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        let was = self.now.replace((wall_ms, on));
        if was.is_some_and(|(_, was_on)| was_on != on) {
            let guids = self
                .routes
                .iter()
                .flat_map(|r| [r.wagon_guid(), r.horse_guid(), r.driver_guid()])
                .collect();
            events.push(SessionEvent::ObjectsRemoved(guids));
        }
        for (route, rolling) in self.routes.iter().zip(&mut self.rolling) {
            let now = on && pose(route, wall_ms).state == CartState::Rolling;
            if was.is_some_and(|(_, was_on)| was_on == on) && now != *rolling && on {
                events.push(horse_relay(route, wall_ms));
            }
            *rolling = now;
        }
        events
    }

    /// Every wagon's units and their creates: a client on the map sees them wherever they are,
    /// as a ship is. The realm sends each wagon's route just before its create.
    pub(crate) fn in_view(&self) -> Vec<(u64, SessionEvent)> {
        let Some((wall_ms, on)) = self.now else {
            return Vec::new();
        };
        self.routes
            .iter()
            .flat_map(|r| creates(r, wall_ms, on))
            .collect()
    }
}
