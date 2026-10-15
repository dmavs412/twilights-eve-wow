//! The Dungeon (`GLOSSARY.md`): the Wailing Caverns' way in and out, its boss and her loot, and
//! the route graph that keeps a chase inside its tunnels (`content/wailing_caverns.toml`; the
//! creatures in it are [`crate::mobs`]'). There is no navmesh: a unit runs straight where the
//! way is clear and along the graph where it is not.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::space::distance;
use crate::Host;

/// A place to be sent to.
#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) struct Place {
    pub(crate) map: u32,
    pub(crate) position: [f32; 3],
    pub(crate) orientation: f32,
}

/// An `AreaTrigger.dbc` sphere and where it sends the player.
#[derive(Debug, Deserialize)]
pub(crate) struct Trigger {
    pub(crate) trigger: u32,
    pub(crate) map: u32,
    pub(crate) position: [f32; 3],
    pub(crate) radius: f32,
    pub(crate) to: Place,
}

impl Trigger {
    /// How far outside its sphere a claim to stand in it is still believed (vmangos re-checks
    /// `CMSG_AREATRIGGER` with 5 yd of slop).
    const SLOP_YD: f32 = 5.0;

    /// Whether a player on `map` at `at` may be standing in it.
    pub(crate) fn holds(&self, map: u32, at: [f32; 3]) -> bool {
        map == self.map && distance(self.position, at) <= self.radius + Self::SLOP_YD
    }
}

#[derive(Deserialize)]
struct Node {
    id: String,
    at: [f32; 3],
}

#[derive(Deserialize)]
struct RouteFile {
    nodes: Vec<Node>,
    edges: Vec<[String; 2]>,
}

#[derive(Deserialize)]
struct File {
    map: u32,
    zone: u32,
    boss: u32,
    boss_loot: Vec<u32>,
    entrance: Trigger,
    exit: Trigger,
    route: RouteFile,
}

/// The one Dungeon.
pub(crate) struct Dungeon {
    pub(crate) map: u32,
    pub(crate) zone: u32,
    /// The boss's spawn guid.
    pub(crate) boss: u32,
    /// The blues her kill draws one of.
    pub(crate) boss_loot: Vec<u32>,
    pub(crate) entrance: Trigger,
    pub(crate) exit: Trigger,
    pub(crate) route: Route,
}

pub(crate) fn content() -> &'static Dungeon {
    static DUNGEON: OnceLock<Dungeon> = OnceLock::new();
    DUNGEON.get_or_init(|| {
        let file: File = toml::from_str(include_str!("../content/wailing_caverns.toml"))
            .expect("content/wailing_caverns.toml parses");
        Dungeon {
            map: file.map,
            zone: file.zone,
            boss: file.boss,
            boss_loot: file.boss_loot,
            entrance: file.entrance,
            exit: file.exit,
            route: Route::new(file.route),
        }
    })
}

/// The route graph of `map`, if it is the Dungeon's.
pub(crate) fn route_on(map: u32) -> Option<&'static Route> {
    let d = content();
    (d.map == map).then_some(&d.route)
}

/// How far from the graph a straight run may stray: the tunnels' half width, about.
const CORRIDOR_YD: f32 = 8.0;
/// The step a straight run is checked at against the corridor.
const SAMPLE_YD: f32 = 2.0;

/// Walkable points and the straight walks between them that are known to stay inside.
pub(crate) struct Route {
    nodes: Vec<[f32; 3]>,
    /// Per node, its neighbours and the distance to each.
    links: Vec<Vec<(usize, f32)>>,
    edges: Vec<(usize, usize)>,
}

impl Route {
    fn new(file: RouteFile) -> Self {
        let index: HashMap<&str, usize> = file
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.as_str(), i))
            .collect();
        let nodes: Vec<[f32; 3]> = file.nodes.iter().map(|n| n.at).collect();
        let mut links = vec![Vec::new(); nodes.len()];
        let mut edges = Vec::new();
        for [a, b] in &file.edges {
            let (a, b) = (index[a.as_str()], index[b.as_str()]);
            let d = distance(nodes[a], nodes[b]);
            links[a].push((b, d));
            links[b].push((a, d));
            edges.push((a, b));
        }
        Self {
            nodes,
            links,
            edges,
        }
    }

    /// The way from `from` to `to` on `map`: straight if that is clear, else to the graph, along
    /// it, and off it again, cutting every corner that is clear. Starts at `from`, ends at `to`.
    pub(crate) fn path(
        &self,
        from: [f32; 3],
        to: [f32; 3],
        map: u32,
        host: &dyn Host,
    ) -> Vec<[f32; 3]> {
        // The corridor first: it is arithmetic, sight a trace through the host's world.
        let clear =
            |a: [f32; 3], b: [f32; 3]| self.inside(a, b) && host.line_of_sight(map, eye(a), eye(b));
        if clear(from, to) {
            return vec![from, to];
        }
        let (Some(a), Some(b)) = (self.entry(from, &clear), self.entry(to, &clear)) else {
            return vec![from, to];
        };
        let mut points = vec![from];
        points.extend(self.shortest(a, b).into_iter().map(|n| self.nodes[n]));
        points.push(to);
        // Pull the string: from each point on to the farthest one in clear sight of it.
        let mut out = vec![from];
        let mut i = 0;
        while i + 1 < points.len() {
            let j = (i + 2..points.len())
                .rev()
                .find(|&j| clear(points[i], points[j]))
                .unwrap_or(i + 1);
            out.push(points[j]);
            i = j;
        }
        out.dedup_by(|a, b| distance(*a, *b) < 0.01);
        out
    }

    /// The node a unit at `at` joins the graph by: the nearest it can run to straight, else the
    /// nearest.
    fn entry(&self, at: [f32; 3], clear: &impl Fn([f32; 3], [f32; 3]) -> bool) -> Option<usize> {
        let mut by_distance: Vec<usize> = (0..self.nodes.len()).collect();
        by_distance
            .sort_by(|&a, &b| distance(self.nodes[a], at).total_cmp(&distance(self.nodes[b], at)));
        by_distance
            .iter()
            .copied()
            .find(|&n| clear(at, self.nodes[n]))
            .or(by_distance.first().copied())
    }

    /// The nodes from `a` to `b` along the graph, both included (Dijkstra); just `a` when `b`
    /// cannot be reached.
    fn shortest(&self, a: usize, b: usize) -> Vec<usize> {
        let n = self.nodes.len();
        let mut best = vec![f32::INFINITY; n];
        let mut prev = vec![usize::MAX; n];
        let mut done = vec![false; n];
        best[a] = 0.0;
        while let Some(u) = (0..n)
            .filter(|&i| !done[i] && best[i].is_finite())
            .min_by(|&x, &y| best[x].total_cmp(&best[y]))
        {
            if u == b {
                break;
            }
            done[u] = true;
            for &(v, d) in &self.links[u] {
                if best[u] + d < best[v] {
                    best[v] = best[u] + d;
                    prev[v] = u;
                }
            }
        }
        if !best[b].is_finite() {
            return vec![a];
        }
        let mut path = vec![b];
        while let Some(&last) = path.last() {
            if last == a {
                break;
            }
            path.push(prev[last]);
        }
        path.reverse();
        path
    }

    /// Whether the straight walk from `a` to `b` stays within the corridor round the graph.
    fn inside(&self, a: [f32; 3], b: [f32; 3]) -> bool {
        let steps = (distance(a, b) / SAMPLE_YD).ceil().max(1.0) as u32;
        (0..=steps).all(|s| {
            let t = s as f32 / steps as f32;
            let p = std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t);
            self.edges
                .iter()
                .any(|&(u, v)| to_segment(p, self.nodes[u], self.nodes[v]) <= CORRIDOR_YD)
        })
    }
}

/// A unit's eyes, for line of sight: two yards over its feet.
fn eye(p: [f32; 3]) -> [f32; 3] {
    [p[0], p[1], p[2] + 2.0]
}

/// The distance from `p` to the segment `ab`.
fn to_segment(p: [f32; 3], a: [f32; 3], b: [f32; 3]) -> f32 {
    let ab: [f32; 3] = std::array::from_fn(|i| b[i] - a[i]);
    let ap: [f32; 3] = std::array::from_fn(|i| p[i] - a[i]);
    let len2: f32 = ab.iter().map(|x| x * x).sum();
    let t = if len2 > 0.0 {
        (ab.iter().zip(&ap).map(|(x, y)| x * y).sum::<f32>() / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    distance(p, std::array::from_fn(|i| a[i] + ab[i] * t))
}
