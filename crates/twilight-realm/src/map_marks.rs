//! What the world map marks for a character (spec #383): quest givers and hand-ins, service NPCs
//! and Phased Dungeons. The realm sends the whole list whenever it changes.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use benilla_protocol::{MapIcon, MapMark, MarkedQuest};

use crate::characters::Char;
use crate::mobs;
use crate::phased::{self, Phase};
use crate::quests::content::{self, Kind, Quest};
use crate::quests::{is_complete, offered};

/// Someone who may give or take in quests, with where and as whom the map shows them.
struct Giver {
    entry: u32,
    object: bool,
    map: u32,
    position: [f32; 2],
    guid: u64,
    name: String,
    subtitle: String,
}

/// The quest givers in marking order: Hub NPCs, Field Givers, then GameObjects.
fn givers() -> &'static [Giver] {
    static GIVERS: OnceLock<Vec<Giver>> = OnceLock::new();
    GIVERS.get_or_init(|| {
        let mut out = Vec::new();
        for n in crate::hub::npcs().iter().filter(|n| n.gives_quests()) {
            out.push(Giver {
                entry: n.entry,
                object: false,
                map: n.map,
                position: [n.position[0], n.position[1]],
                guid: n.guid(),
                name: n.name.clone(),
                subtitle: n.subname.clone(),
            });
        }
        for s in &mobs::content().spawn {
            if s.phase != Phase::OpenWorld || !content::field_giver(s.entry) {
                continue;
            }
            let Some(t) = mobs::template(s.entry) else {
                continue;
            };
            out.push(Giver {
                entry: s.entry,
                object: false,
                map: s.map,
                position: [s.position[0], s.position[1]],
                guid: mobs::unit_guid(s.entry, s.guid),
                name: t.name.clone(),
                subtitle: String::new(),
            });
        }
        for o in content::objects() {
            let named = content::quests()
                .into_iter()
                .find(|q| q.giver_object == Some(o.entry));
            let Some(q) = named else {
                continue;
            };
            out.push(Giver {
                entry: o.entry,
                object: true,
                map: o.map,
                position: [o.position[0], o.position[1]],
                guid: o.object_guid(),
                // The realm holds no GameObject name; the first quest it offers stands in.
                name: q.title.clone(),
                subtitle: String::new(),
            });
        }
        out
    })
}

fn kind_label(k: Kind) -> &'static str {
    match k {
        Kind::Hunting => "Hunting Quest",
        Kind::Repeatable => "Repeatable Quest",
        Kind::Bounty => "Bounty",
        Kind::Advancement => "Advancement Quest",
        Kind::Story => "Story Quest",
        Kind::Writ => "Writ",
        Kind::Once | Kind::Rumour => "Quest",
    }
}

/// The level range of `q`'s kill targets near the giver: those in the giver's level area, or all
/// of them when none is or the giver stands in no area.
fn monsters(g: &Giver, q: &Quest) -> Option<[u8; 2]> {
    static CACHE: OnceLock<Mutex<HashMap<(u64, u32), Option<[u8; 2]>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(r) = cache.lock().unwrap().get(&(g.guid, q.id)) {
        return *r;
    }
    let targets: Vec<&mobs::Spawn> = mobs::content()
        .spawn
        .iter()
        .filter(|s| s.phase == Phase::OpenWorld && q.kill.iter().any(|k| k.counts(s.entry)))
        .collect();
    let area = mobs::level_area(g.map, g.position[0], g.position[1]);
    let near: Vec<&mobs::Spawn> = match area {
        Some(a) => targets
            .iter()
            .copied()
            .filter(|s| mobs::level_area(s.map, s.position[0], s.position[1]) == Some(a))
            .collect(),
        None => Vec::new(),
    };
    let kept = if near.is_empty() { targets } else { near };
    let range = kept
        .iter()
        .map(|s| s.level_min)
        .min()
        .zip(kept.iter().map(|s| s.level_max).max())
        .map(|(lo, hi)| [lo, hi]);
    cache.lock().unwrap().insert((g.guid, q.id), range);
    range
}

fn row(g: &Giver, q: &Quest, ready: bool) -> MarkedQuest {
    MarkedQuest {
        title: q.title.clone(),
        kind: kind_label(q.kind).to_string(),
        level: q.level,
        monsters: monsters(g, q),
        ready,
    }
}

/// The quests `g` takes in that are ready, then those it offers `c` now, in the Writ Board's
/// `window`.
fn quest_rows(c: &Char, g: &Giver, window: u32) -> Vec<MarkedQuest> {
    let mut rows = Vec::new();
    if !g.object {
        for q in content::ended_by(g.entry) {
            if q.kind != Kind::Rumour && c.quests.get(q.id).is_some() && is_complete(c, q) {
                rows.push(row(g, q, true));
            }
        }
    }
    let offers: Vec<&Quest> = if g.object {
        content::offered_by_object(g.entry).collect()
    } else {
        content::offered_by(g.entry).collect()
    };
    for q in offers {
        if q.kind != Kind::Rumour && offered(c, q, window) {
            rows.push(row(g, q, false));
        }
    }
    rows
}

/// A Hub NPC's service icon and its subtitle when the content gives it none.
fn service(role: &str) -> Option<(MapIcon, &'static str)> {
    Some(match role {
        "angler" => (MapIcon::Trainer, "Angler"),
        "fisherman" => (MapIcon::Trainer, "Fisherman"),
        "herb_trainer" => (MapIcon::Trainer, "Herbalism Trainer"),
        "herbalist" => (MapIcon::Trainer, "Herbalist"),
        "cook" => (MapIcon::Trainer, "Cook"),
        "alchemist" => (MapIcon::Trainer, "Alchemist"),
        "goods" | "shop" => (MapIcon::Vendor, "Vendor"),
        "repairer" => (MapIcon::Vendor, "Repairs"),
        "blood_coin" => (MapIcon::Vendor, "Blood Coin Broker"),
        "innkeeper" => (MapIcon::Innkeeper, "Innkeeper"),
        "flight" => (MapIcon::FlightMaster, "Flight Master"),
        _ => return None,
    })
}

/// Everything the map marks for `c`: quest marks, then service marks, dungeon marks, and the cart
/// stops on their map, grey while `carts_locked`. `window` is the Writ Board's now. None stands on
/// land closed while the Milestones `reached` are.
pub(crate) fn marks(c: &Char, carts_locked: bool, window: u32, reached: &[&str]) -> Vec<MapMark> {
    let mut out = Vec::new();
    let mut quest_marked = Vec::new();
    for g in givers() {
        let quests = quest_rows(c, g, window);
        if quests.is_empty() {
            continue;
        }
        if !g.object {
            quest_marked.push(g.guid);
        }
        let icon = if quests.iter().any(|q| q.ready) {
            MapIcon::QuestReady
        } else {
            MapIcon::QuestAvailable
        };
        out.push(MapMark {
            map: g.map,
            position: g.position,
            guid: g.guid,
            icon,
            name: g.name.clone(),
            subtitle: g.subtitle.clone(),
            quests,
        });
    }
    for n in crate::hub::npcs() {
        let Some((icon, label)) = service(&n.role) else {
            continue;
        };
        if quest_marked.contains(&n.guid()) {
            continue;
        }
        out.push(MapMark {
            map: n.map,
            position: [n.position[0], n.position[1]],
            guid: n.guid(),
            icon,
            name: n.name.clone(),
            subtitle: if n.subname.is_empty() {
                label.to_string()
            } else {
                n.subname.clone()
            },
            quests: Vec::new(),
        });
    }
    // An instance map is marked at its door on the open world, by its own name and levels; a
    // dungeon on it makes no mark of its own (ADR 0032).
    for m in crate::instance_map::content() {
        let [lo, hi] = m.levels;
        out.push(MapMark {
            map: m.entrance.map,
            position: [m.entrance.position[0], m.entrance.position[1]],
            guid: 0,
            icon: MapIcon::Dungeon,
            name: m.name.clone(),
            subtitle: format!("Phased Dungeon, levels {lo}-{hi}"),
            quests: Vec::new(),
        });
    }
    for d in phased::content() {
        let Some([lo, hi]) = d
            .levels
            .filter(|_| crate::instance_map::of_map(d.map).is_none())
        else {
            continue;
        };
        out.push(MapMark {
            map: d.map,
            position: [d.portal.position[0], d.portal.position[1]],
            guid: 0,
            icon: MapIcon::Dungeon,
            name: d.name.clone(),
            subtitle: format!("Phased Dungeon, levels {lo}-{hi}"),
            quests: Vec::new(),
        });
    }
    for r in crate::carts::routes().iter().filter(|r| r.map == c.map) {
        for (i, stop) in r.stops.iter().enumerate() {
            // The stop at the route's other end; a middle stop names the far end.
            let other = if i + 1 == r.stops.len() {
                &r.stops[0]
            } else {
                &r.stops[r.stops.len() - 1]
            };
            let (icon, subtitle) = if carts_locked {
                (
                    MapIcon::CartStopLocked,
                    crate::carts::cart().locked_line.clone(),
                )
            } else {
                (MapIcon::CartStop, format!("Cart to {}", other.name))
            };
            out.push(MapMark {
                map: r.map,
                position: [stop.position[0], stop.position[1]],
                guid: 0,
                icon,
                name: stop.name.clone(),
                subtitle,
                quests: Vec::new(),
            });
        }
    }
    out.retain(|m| !crate::closed_at(m.map, m.position[0], m.position[1], reached));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_areas_name_northshire_and_goldshire() {
        let eagan = mobs::content()
            .spawn
            .iter()
            .find(|s| mobs::template(s.entry).is_some_and(|t| t.name == "Eagan Peltskinner"))
            .expect("Peltskinner spawns");
        assert_eq!(
            mobs::level_area(eagan.map, eagan.position[0], eagan.position[1]),
            Some("Northshire Valley")
        );
        assert_eq!(mobs::level_area(0, -9462.0, 16.0), Some("Goldshire"));
    }
}
