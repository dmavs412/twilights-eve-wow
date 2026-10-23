//! Closed Zones (ADR 0015): the map chunks no one may stand on, read from
//! `data/twilights-eve/closed_chunks.json`. The wall that keeps characters off them is
//! [`InWorld::closed_off`](super::InWorld::closed_off); content placed on one is a bug the
//! `closed_zones` test names.

use std::collections::HashSet;
use std::sync::OnceLock;

use serde::Deserialize;

/// `benilla-formats`' `TILE_SIZE`: the side of one ADT tile, in yards.
const TILE_SIZE: f32 = 533.333_3;
/// `benilla-formats`' `CHUNK_SIZE`: the side of one map chunk, a sixteenth of a tile.
const CHUNK_SIZE: f32 = TILE_SIZE / 16.0;
/// Chunks across a map, both ways: 64 tiles of 16.
const CHUNKS: i64 = 64 * 16;

#[derive(Deserialize)]
struct File {
    map: u32,
    chunks: Vec<[u32; 2]>,
}

struct Closed {
    map: u32,
    chunks: HashSet<(u32, u32)>,
}

fn closed() -> &'static Closed {
    static CLOSED: OnceLock<Closed> = OnceLock::new();
    CLOSED.get_or_init(|| {
        let file: File = serde_json::from_str(include_str!(
            "../../../../data/twilights-eve/closed_chunks.json"
        ))
        .expect("data/twilights-eve/closed_chunks.json parses");
        Closed {
            map: file.map,
            chunks: file.chunks.into_iter().map(|[gx, gy]| (gx, gy)).collect(),
        }
    })
}

/// Whether `(x, y)` on `map` lies on a closed chunk. A point off the map's grid, or on another
/// map than the file's, is open.
pub fn closed_at(map: u32, x: f32, y: f32) -> bool {
    let c = closed();
    if map != c.map {
        return false;
    }
    let index = |v: f32| ((32.0 * TILE_SIZE - v) / CHUNK_SIZE).floor() as i64;
    let (gx, gy) = (index(y), index(x));
    (0..CHUNKS).contains(&gx)
        && (0..CHUNKS).contains(&gy)
        && c.chunks.contains(&(gx as u32, gy as u32))
}

/// A point the realm's content puts something on, named for a test's failure message.
#[derive(Debug, Clone)]
pub struct Placed {
    pub what: String,
    pub map: u32,
    pub position: [f32; 2],
}

/// The values from `range[0]` to `range[1]` at most `step` apart, both ends included.
pub(crate) fn steps([lo, hi]: [f32; 2], step: f32) -> Vec<f32> {
    let (lo, hi) = (lo.min(hi), lo.max(hi));
    let n = ((hi - lo) / step).ceil().max(1.0) as usize;
    (0..=n)
        .map(|i| lo + (hi - lo) * i as f32 / n as f32)
        .collect()
}

/// How far apart the points a test samples over an area or a box are.
const SAMPLE_YD: f32 = 8.0;

/// Every point the realm's content places something on: creature spawns (the open world's,
/// the Phased Dungeons', the called, timed and Encounter ones), Hub NPCs, quest GameObjects,
/// explore spots, Rumour boxes and points, Encounter anchors, the Phased Dungeons' footprints,
/// portals, doors, stops and objects, the Dungeon's triggers, and the respawn and bind points.
/// An area or a box comes as points [`SAMPLE_YD`] apart over it.
pub fn placed() -> Vec<Placed> {
    let mut out = Vec::new();
    let mut put = |what: String, map: u32, [x, y]: [f32; 2]| {
        out.push(Placed {
            what,
            map,
            position: [x, y],
        })
    };
    let xy = |p: &[f32]| [p[0], p[1]];
    for s in &crate::mobs::content().spawn {
        put(
            format!("creature {} (entry {})", s.guid, s.entry),
            s.map,
            xy(&s.position),
        );
    }
    for npc in crate::hub::npcs() {
        put(format!("Hub NPC {}", npc.entry), npc.map, xy(&npc.position));
    }
    for o in crate::quests::content::objects() {
        put(
            format!("quest GameObject {} (entry {})", o.guid, o.entry),
            o.map,
            xy(&o.position),
        );
    }
    for q in crate::quests::content::quests() {
        for e in q.explore.single().into_iter().chain(q.explore.listed()) {
            let [x, y] = e.position;
            for gx in steps([x - e.radius, x + e.radius], SAMPLE_YD) {
                for gy in steps([y - e.radius, y + e.radius], SAMPLE_YD) {
                    if (gx - x).hypot(gy - y) <= e.radius {
                        put(
                            format!("quest {} explore spot {:?}", q.id, e.place),
                            e.map,
                            [gx, gy],
                        );
                    }
                }
            }
        }
    }
    for r in crate::rumours::defs() {
        for &[x0, y0, x1, y1] in &r.bounds {
            for x in steps([x0, x1], SAMPLE_YD) {
                for y in steps([y0, y1], SAMPLE_YD) {
                    put(format!("Rumour {} bounds", r.id), r.map, [x, y]);
                }
            }
        }
        for p in &r.points {
            put(
                format!("Rumour {} point {:?}", r.id, p.landmark),
                r.map,
                [p.x, p.y],
            );
        }
        for s in &r.spawn {
            for p in &s.points {
                put(
                    format!("Rumour {} step {} point", r.id, s.quest),
                    r.map,
                    [p.x, p.y],
                );
            }
        }
    }
    for e in crate::encounters::content() {
        put(
            format!("Encounter {} anchor", e.key),
            crate::content::EASTERN_KINGDOMS,
            xy(&e.anchor),
        );
    }
    for d in crate::phased::content() {
        for p in d.footprint_points(SAMPLE_YD) {
            put(format!("Phased Dungeon {} footprint", d.id), d.map, p);
        }
        put(
            format!("Phased Dungeon {} portal", d.id),
            d.map,
            xy(&d.portal.position),
        );
        if let Some(door) = &d.portal.door {
            put(
                format!("Phased Dungeon {} door", d.id),
                d.map,
                xy(&door.position),
            );
            put(
                format!("Phased Dungeon {} door's outside", d.id),
                d.map,
                xy(&door.outside),
            );
        }
        for s in &d.stop {
            put(
                format!("Phased Dungeon {} stop {} arrival", d.id, s.key),
                d.map,
                xy(&s.arrival),
            );
            put(
                format!("Phased Dungeon {} stop {} return portal", d.id, s.key),
                d.map,
                xy(&s.return_portal),
            );
        }
        for p in &d.inner_portal {
            put(
                format!("Phased Dungeon {} inner portal to {}", d.id, p.stop),
                d.map,
                xy(&p.position),
            );
            put(
                format!("Phased Dungeon {} inner portal back from {}", d.id, p.stop),
                d.map,
                xy(&p.back),
            );
        }
        for o in &d.gameobject {
            put(
                format!("Phased Dungeon {} GameObject {}", d.id, o.guid),
                d.map,
                xy(&o.position),
            );
        }
    }
    let dungeon = crate::dungeon::content();
    for (name, t) in [("entrance", &dungeon.entrance), ("exit", &dungeon.exit)] {
        put(format!("Dungeon {name} trigger"), t.map, xy(&t.position));
        put(
            format!("Dungeon {name} destination"),
            t.to.map,
            xy(&t.to.position),
        );
    }
    for (name, r) in [
        ("Hub respawn", super::death::Respawn::HUB),
        ("Northshire respawn", super::death::Respawn::NORTHSHIRE),
    ] {
        put(name.to_string(), r.map, xy(&r.position));
    }
    for (name, b) in [
        ("Crossroads bind", crate::inn::Bind::crossroads_inn()),
        ("Northshire bind", crate::inn::Bind::northshire()),
    ] {
        put(name.to_string(), b.place.map, xy(&b.place.position));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_cover_both_ends_at_most_a_step_apart() {
        assert_eq!(steps([0.0, 16.0], 8.0), vec![0.0, 8.0, 16.0]);
        assert_eq!(steps([5.0, 5.0], 8.0), vec![5.0, 5.0]);
        let s = steps([10.0, -7.0], 8.0);
        assert_eq!((s[0], *s.last().unwrap()), (-7.0, 10.0));
        assert!(s.windows(2).all(|w| w[1] - w[0] <= 8.0));
    }
}
