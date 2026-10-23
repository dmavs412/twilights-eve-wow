//! `WOW_PROBE=spawn_audit`: in Twilight's Eve, walks every unit we placed
//! ([`twilight_realm::audit_rows`], ADR 0021) with the real collision streamed in, measures what
//! the player would see, writes `target/spawn_audit.json` and `target/spawn_audit.md`, and exits.
//! Run on demand only; nothing schedules it.
//!
//! The walk visits the rows on the player's map tile by tile, and within a tile each cluster of
//! rows [`CLUSTER_YD`] across, so every row stands inside the realm's view of the spot it
//! teleports to. At each spot it waits for the terrain and for the collider set to stop changing
//! (or [`STREAM_LIMIT_SECS`]), then [`SETTLE_SECS`] more for the ground clamp. A Rumour's giver
//! and step spawns are started with the realm's dev commands and ended before the next.
//!
//! Per row, from the unit the client drew:
//! - `created`: the client holds an entity for it.
//! - `model`: its visual built a model, not the fallback cube; `unknown` while it is still pending.
//! - `sunk`: the ground clamp pulled it down past `ground_census::SUNK_YD`, or a walkable surface
//!   lies within [`ROOF_YD`] over its feet.
//! - `body`: a standing body fits at its feet (`WorldCollision::body_overlaps`, as `body_fits`).
//! - `water`: water over its feet deeper than [`DEEP_WATER_YD`], for a unit that does not swim.
//!
//! A row the walk cannot reach says why: `other map` (the dev teleport stays on the player's
//! map, so each continent takes its own run), `phase not entered`, `not streamed`, and for a
//! Rumour's unit nothing drawn near its point, `dropped` (`ground::spot` found no room).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use benilla_assets::coords::{bevy_to_wow, wow_to_bevy};
use benilla_protocol::EntityKind;
use benilla_world::collision::{ColliderEpoch, WorldCollision};
use benilla_world::liquid::{water_surface_at, LiquidClaim, WaterChunkInfo};
use benilla_world::world_map::CurrentMap;
use benilla_world::world_point::WorldPoint;
use bevy::prelude::*;
use twilight_realm::{audit_rows, AuditKind, AuditRow};

use super::ground_census::{lowest_surface_above, SUNK_YD};
use super::ProbeClock;
use crate::entities::{FallbackCube, VisualAttached};
use crate::net::{GroundClamped, Guid, NetEntity, SelfPlayer};
use crate::twilight::{DevGoto, DevRumour, DevRumourEnd, DevRumourStep};

/// Where the report goes, relative to the working directory.
const OUT_DIR: &str = "target";
/// Rows within this of a cluster's first row share its visit: well inside the realm's 100 yd view.
const CLUSTER_YD: f32 = 60.0;
/// How long a visit waits for its spot to stream in before its rows are `not streamed`.
const STREAM_LIMIT_SECS: f32 = 20.0;
/// The collider set unchanged this long counts as streamed: no signal says the doodads are in.
const QUIET_SECS: f32 = 3.0;
/// The wait after streaming for the ground clamp to settle the units.
const SETTLE_SECS: f32 = 2.0;
/// The wait after starting a Rumour for its units to be made and drawn.
const RUMOUR_SECS: f32 = 3.0;
/// The wait after ending a Rumour before the next starts, so the realm has dropped it.
const END_SECS: f32 = 1.5;
/// The teleport counts as arrived within this of the spot, horizontally.
const ARRIVED_YD: f32 = 15.0;
/// A Rumour unit drawn within this of its point is that point's; `ground::spot` nudges up to 5.
const FIND_YD: f32 = 6.0;
/// A walkable surface this close over the feet is a roof the unit is under.
const ROOF_YD: f32 = 3.0;
/// Water deeper than this over a land unit's feet fails it.
const DEEP_WATER_YD: f32 = 1.5;
/// A standing body, as `Host::body_fits` measures it.
const BODY_RADIUS_YD: f32 = 0.4;
const BODY_HEIGHT_YD: f32 = 2.0;

pub(crate) struct ProbeSpawnAuditPlugin;

impl Plugin for ProbeSpawnAuditPlugin {
    fn build(&self, app: &mut App) {
        if !crate::twilight::active() {
            warn!("probe-spawn-audit: WOW_PROBE=spawn_audit needs the twilights-eve launcher — ignored");
            return;
        }
        app.insert_resource(SpawnAudit::default())
            .add_systems(Update, (drive_spawn_audit, measure_spawn_audit).chain());
    }
}

/// What a visit starts before it measures.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Start {
    Giver { id: u32, point: usize },
    Step { id: u32, quest: u32, point: usize },
}

/// One teleport: its spot, the rows measured there, and the Rumour it starts.
#[derive(Debug, Clone, PartialEq)]
struct Visit {
    at: [f32; 2],
    rows: Vec<usize>,
    start: Option<Start>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Stage {
    Idle,
    Going {
        since: f32,
        quiet_since: f32,
        epoch: u64,
    },
    Settling {
        until: f32,
    },
    Measure,
    Ending {
        until: f32,
    },
    Done,
}

/// What one row measured, or why it was not.
#[derive(Debug, Clone, Default, PartialEq)]
struct Measures {
    skipped: Option<&'static str>,
    created: bool,
    /// `None` while its visual is pending.
    model: Option<bool>,
    sunk: bool,
    fits: bool,
    deep: bool,
    drawn_z: Option<f32>,
}

impl Measures {
    fn skipped(why: &'static str) -> Self {
        Self {
            skipped: Some(why),
            ..Default::default()
        }
    }

    /// Several drawn units for one row (a step's count): created if any, and failed where any is.
    fn merge(all: Vec<Measures>) -> Self {
        let Some(first) = all.first() else {
            return Self::skipped("dropped");
        };
        Self {
            skipped: None,
            created: true,
            model: if all.iter().any(|m| m.model == Some(false)) {
                Some(false)
            } else if all.iter().all(|m| m.model == Some(true)) {
                Some(true)
            } else {
                None
            },
            sunk: all.iter().any(|m| m.sunk),
            fits: all.iter().all(|m| m.fits),
            deep: all.iter().any(|m| m.deep),
            drawn_z: first.drawn_z,
        }
    }
}

/// The row's failures: why it was skipped, `not created`, or each failed measure. Empty is `ok`.
fn verdict(m: &Measures) -> Vec<&'static str> {
    if let Some(why) = m.skipped {
        return vec![why];
    }
    if !m.created {
        return vec!["not created"];
    }
    let mut out = Vec::new();
    if m.model == Some(false) {
        out.push("model");
    }
    if m.sunk {
        out.push("sunk");
    }
    if !m.fits {
        out.push("body");
    }
    if m.deep {
        out.push("water");
    }
    out
}

#[derive(Resource, Default)]
struct SpawnAudit {
    rows: Vec<AuditRow>,
    results: Vec<Option<Measures>>,
    visits: Vec<Visit>,
    next: usize,
    stage: Option<Stage>,
}

/// The rows' indexes by ADT tile `(map, tile x, tile y)`, in tile order.
fn by_tile(rows: &[(u32, [f32; 3])]) -> BTreeMap<(u32, u32, u32), Vec<usize>> {
    let mut out: BTreeMap<_, Vec<usize>> = BTreeMap::new();
    for (i, (map, p)) in rows.iter().enumerate() {
        let (tx, ty) = benilla_formats::world_to_tile(p[0], p[1]);
        out.entry((*map, tx, ty)).or_default().push(i);
    }
    out
}

fn flat(a: [f32; 3], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// The walk over `rows` from map `map`: the visits, tile by tile, and the rows it cannot reach
/// with why.
fn plan(rows: &[AuditRow], map: u32) -> (Vec<Visit>, Vec<(usize, &'static str)>) {
    let mut skipped = Vec::new();
    let mut reachable = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        if r.phased {
            skipped.push((i, "phase not entered"));
        } else if r.map != map || !matches!(r.map, 0 | 1) {
            skipped.push((i, "other map"));
        } else {
            reachable.push(i);
        }
    }
    let placed: Vec<(u32, [f32; 3])> = reachable
        .iter()
        .map(|&i| (rows[i].map, rows[i].position))
        .collect();
    let mut visits = Vec::new();
    let mut rumours: BTreeMap<String, Visit> = BTreeMap::new();
    for (_, members) in by_tile(&placed) {
        let mut left: Vec<usize> = members.iter().map(|&k| reachable[k]).collect();
        // A Rumour's rows each start their own Rumour; a step's rows at one point share it.
        left.retain(|&i| {
            let r = &rows[i];
            let at = [r.position[0], r.position[1]];
            let (key, start) = match r.kind {
                AuditKind::RumourGiver { rumour, point } => (
                    format!("{rumour} giver {point}"),
                    Start::Giver { id: rumour, point },
                ),
                AuditKind::RumourStep {
                    rumour,
                    quest,
                    point,
                } => (
                    format!("{rumour} step {quest} {point}"),
                    Start::Step {
                        id: rumour,
                        quest,
                        point,
                    },
                ),
                _ => return true,
            };
            rumours
                .entry(key)
                .or_insert_with(|| Visit {
                    at,
                    rows: Vec::new(),
                    start: Some(start),
                })
                .rows
                .push(i);
            false
        });
        while let Some(&seed) = left.first() {
            let at = [rows[seed].position[0], rows[seed].position[1]];
            let (near, far): (Vec<usize>, Vec<usize>) = left
                .iter()
                .partition(|&&i| flat(rows[i].position, at) <= CLUSTER_YD);
            visits.push(Visit {
                at,
                rows: near,
                start: None,
            });
            left = far;
        }
    }
    visits.extend(rumours.into_values());
    (visits, skipped)
}

/// Starts the walk once in-world, moves it from visit to visit, and writes the report at the end.
#[allow(clippy::too_many_arguments)]
fn drive_spawn_audit(
    mut probe: ResMut<SpawnAudit>,
    time: ProbeClock,
    state: Res<State<crate::char_select::ClientState>>,
    map: Option<Res<CurrentMap>>,
    me: Query<&Transform, With<SelfPlayer>>,
    world: WorldPoint,
    epoch: Res<ColliderEpoch>,
    mut out: (
        MessageWriter<DevGoto>,
        MessageWriter<DevRumour>,
        MessageWriter<DevRumourStep>,
    ),
    mut exit: MessageWriter<AppExit>,
) {
    if *state.get() != crate::char_select::ClientState::InWorld {
        return;
    }
    let (Some(map), Ok(me)) = (map, me.single()) else {
        return;
    };
    let now = time.elapsed_secs();
    let probe = &mut *probe;
    let Some(stage) = probe.stage else {
        probe.rows = audit_rows();
        probe.results = vec![None; probe.rows.len()];
        let (visits, skipped) = plan(&probe.rows, map.0);
        for (i, why) in skipped {
            probe.results[i] = Some(Measures::skipped(why));
        }
        info!(
            "probe-spawn-audit: {} rows, {} visits on map {}",
            probe.rows.len(),
            visits.len(),
            map.0
        );
        probe.visits = visits;
        probe.stage = Some(Stage::Idle);
        return;
    };
    match stage {
        Stage::Idle => {
            let Some(visit) = probe.visits.get(probe.next) else {
                write_report(&probe.rows, &probe.results);
                probe.stage = Some(Stage::Done);
                exit.write(AppExit::Success);
                return;
            };
            out.0.write(DevGoto {
                x: visit.at[0],
                y: visit.at[1],
                facing: None,
            });
            probe.stage = Some(Stage::Going {
                since: now,
                quiet_since: now,
                epoch: epoch.get(),
            });
        }
        Stage::Going {
            since,
            mut quiet_since,
            epoch: seen,
        } => {
            if epoch.get() != seen {
                quiet_since = now;
            }
            let visit = &probe.visits[probe.next];
            let here = bevy_to_wow(me.translation);
            let arrived = flat(here, visit.at) <= ARRIVED_YD;
            let spot = wow_to_bevy([visit.at[0], visit.at[1], here[2]]);
            let streamed = arrived && world.terrain_height_under(spot).is_some();
            let quiet = now - quiet_since >= QUIET_SECS;
            if streamed && (quiet || now - since >= STREAM_LIMIT_SECS) {
                match visit.start {
                    Some(Start::Giver { id, point }) => {
                        out.1.write(DevRumour { id, point });
                    }
                    Some(Start::Step { id, quest, point }) => {
                        out.2.write(DevRumourStep { id, quest, point });
                    }
                    None => {}
                }
                let wait = if visit.start.is_some() {
                    RUMOUR_SECS
                } else {
                    SETTLE_SECS
                };
                probe.stage = Some(Stage::Settling { until: now + wait });
            } else if now - since >= STREAM_LIMIT_SECS {
                for i in visit.rows.clone() {
                    probe.results[i] = Some(Measures::skipped("not streamed"));
                }
                probe.next += 1;
                probe.stage = Some(Stage::Idle);
            } else {
                probe.stage = Some(Stage::Going {
                    since,
                    quiet_since,
                    epoch: epoch.get(),
                });
            }
        }
        Stage::Settling { until } if now >= until => probe.stage = Some(Stage::Measure),
        Stage::Ending { until } if now >= until => {
            probe.next += 1;
            probe.stage = Some(Stage::Idle);
        }
        _ => {}
    }
}

/// What the measure reads per drawn unit.
type UnitQuery = (
    Entity,
    &'static Guid,
    &'static NetEntity,
    &'static Transform,
    Option<&'static GroundClamped>,
    Has<VisualAttached>,
    Option<&'static Children>,
);

/// Measures the current visit's rows, then ends its Rumour.
fn measure_spawn_audit(
    mut probe: ResMut<SpawnAudit>,
    time: ProbeClock,
    units: Query<UnitQuery>,
    cubes: Query<(), With<FallbackCube>>,
    collision: WorldCollision,
    water: Query<&'static WaterChunkInfo>,
    mut end: MessageWriter<DevRumourEnd>,
) {
    if probe.stage != Some(Stage::Measure) {
        return;
    }
    let probe = &mut *probe;
    let visit = probe.visits[probe.next].clone();
    type Unit<'a> = (
        Entity,
        &'a Guid,
        &'a NetEntity,
        &'a Transform,
        Option<&'a GroundClamped>,
        bool,
        Option<&'a Children>,
    );
    let measure = |(entity, _, net, t, clamped, attached, children): Unit<'_>, swims: bool| {
        let feet = t.translation;
        let wow = bevy_to_wow(feet);
        let seat = clamped.map_or(feet.y, |c| c.seat_y);
        let roofed = lowest_surface_above(&collision, feet).is_some_and(|y| y - feet.y <= ROOF_YD);
        let skip = matches!(net.kind, EntityKind::GameObject).then_some(entity);
        let surface = water_surface_at(
            water.iter(),
            [wow[0], wow[1], f32::INFINITY],
            LiquidClaim::Unknown,
        );
        Measures {
            skipped: None,
            created: true,
            model: attached.then(|| !children.is_some_and(|c| c.iter().any(|k| cubes.contains(k)))),
            sunk: seat - feet.y > SUNK_YD || roofed,
            fits: !collision.body_overlaps_except(feet, BODY_RADIUS_YD, BODY_HEIGHT_YD, skip),
            deep: !swims && surface.is_some_and(|w| w - wow[2] > DEEP_WATER_YD),
            drawn_z: Some(wow[2]),
        }
    };
    for &i in &visit.rows {
        let row = &probe.rows[i];
        let m = match row.guid {
            Some(guid) => units
                .iter()
                .find(|u| u.1 .0 == guid)
                .map_or_else(Measures::default, |u| measure(u, row.swims)),
            None => Measures::merge(
                units
                    .iter()
                    .filter(|u| {
                        u.2.display_id == Some(row.display)
                            && matches!(u.2.kind, EntityKind::Unit | EntityKind::GameObject)
                            && flat(
                                bevy_to_wow(u.3.translation),
                                [row.position[0], row.position[1]],
                            ) <= FIND_YD
                    })
                    .map(|u| measure(u, row.swims))
                    .collect(),
            ),
        };
        probe.results[i] = Some(m);
    }
    let now = time.elapsed_secs();
    probe.stage = Some(match visit.start {
        Some(Start::Giver { id, .. } | Start::Step { id, .. }) => {
            end.write(DevRumourEnd { id });
            Stage::Ending {
                until: now + END_SECS,
            }
        }
        None => Stage::Ending { until: now },
    });
}

fn kind_name(kind: AuditKind) -> &'static str {
    match kind {
        AuditKind::Creature => "creature",
        AuditKind::HubNpc => "Hub NPC",
        AuditKind::GameObject => "GameObject",
        AuditKind::RumourGiver { .. } => "Rumour giver",
        AuditKind::RumourStep { .. } => "Rumour step",
    }
}

/// `s` as a JSON string.
fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The JSON report, one object per row, and the Markdown one, the failures by kind and a count.
fn reports(rows: &[AuditRow], results: &[Option<Measures>]) -> (String, String) {
    let mut json = String::from("[\n");
    let mut failed: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    let (mut ok, mut bad, mut skipped) = (0, 0, 0);
    for (n, (r, m)) in rows.iter().zip(results).enumerate() {
        let m = m
            .clone()
            .unwrap_or_else(|| Measures::skipped("not visited"));
        let v = verdict(&m);
        let verdict_text = if v.is_empty() {
            "ok".to_string()
        } else {
            v.join(" ")
        };
        match (m.skipped, v.is_empty()) {
            (Some(_), _) => skipped += 1,
            (None, true) => ok += 1,
            (None, false) => bad += 1,
        }
        if m.skipped.is_none() && !v.is_empty() {
            failed.entry(kind_name(r.kind)).or_default().push(format!(
                "- {} at ({:.1}, {:.1}): {verdict_text}",
                r.what, r.position[0], r.position[1]
            ));
        }
        let model = match m.model {
            Some(true) => "\"yes\"",
            Some(false) => "\"no\"",
            None => "\"unknown\"",
        };
        let _ = write!(
            json,
            "  {{\"what\": {}, \"kind\": {}, \"map\": {}, \"x\": {:.2}, \"y\": {:.2}, \"z\": {:.2}, \
             \"drawn_z\": {}, \"created\": {}, \"model\": {model}, \"sunk\": {}, \"body_fits\": {}, \
             \"deep_water\": {}, \"verdict\": {}}}{}\n",
            json_str(&r.what),
            json_str(kind_name(r.kind)),
            r.map,
            r.position[0],
            r.position[1],
            r.position[2],
            m.drawn_z.map_or_else(|| "null".to_string(), |z| format!("{z:.2}")),
            m.created,
            m.sunk,
            m.fits,
            m.deep,
            json_str(&verdict_text),
            if n + 1 == rows.len() { "" } else { "," },
        );
    }
    json.push_str("]\n");
    let mut md = String::from("# Spawn audit\n\n");
    for (kind, lines) in &failed {
        let _ = writeln!(md, "## {kind}\n");
        for line in lines {
            let _ = writeln!(md, "{line}");
        }
        md.push('\n');
    }
    let _ = writeln!(
        md,
        "{} rows: {ok} ok, {bad} failed, {skipped} not measured.",
        rows.len()
    );
    (json, md)
}

fn write_report(rows: &[AuditRow], results: &[Option<Measures>]) {
    let (json, md) = reports(rows, results);
    let dir = std::path::Path::new(OUT_DIR);
    let written = std::fs::create_dir_all(dir)
        .and_then(|()| std::fs::write(dir.join("spawn_audit.json"), json))
        .and_then(|()| std::fs::write(dir.join("spawn_audit.md"), &md));
    match written {
        Ok(()) => info!("probe-spawn-audit: wrote {OUT_DIR}/spawn_audit.json and .md"),
        Err(e) => error!("probe-spawn-audit: could not write the report: {e}"),
    }
    println!("{}", md.lines().last().unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(what: &str, kind: AuditKind, map: u32, x: f32, y: f32) -> AuditRow {
        AuditRow {
            what: what.to_string(),
            kind,
            map,
            phased: false,
            position: [x, y, 0.0],
            guid: Some(1),
            display: 1,
            swims: false,
        }
    }

    #[test]
    fn rows_group_by_their_adt_tile() {
        // Northshire Abbey and the Goldshire inn share tile (32, 48); Stormwind's gate does not.
        let tiles = by_tile(&[
            (0, [-8913.0, -137.0, 0.0]),
            (0, [-9460.0, 40.0, 0.0]),
            (0, [-8913.0, -137.0, 0.0]),
            (1, [-8913.0, -137.0, 0.0]),
        ]);
        let groups: Vec<Vec<usize>> = tiles.values().cloned().collect();
        assert_eq!(tiles.len(), 3, "{tiles:?}");
        assert!(groups.contains(&vec![0, 2]));
        assert!(groups.contains(&vec![3]), "another map is another tile");
    }

    #[test]
    fn a_tile_splits_into_visits_its_rows_stand_near_and_skips_what_it_cannot_reach() {
        let mut far = row("far", AuditKind::Creature, 0, -8913.0, -137.0 + 100.0);
        far.guid = Some(2);
        let mut phased = row("phased", AuditKind::Creature, 0, -8913.0, -137.0);
        phased.phased = true;
        let rows = vec![
            row("near", AuditKind::Creature, 0, -8913.0, -137.0),
            far,
            phased,
            row("kalimdor", AuditKind::Creature, 1, -400.0, -2600.0),
            row(
                "giver",
                AuditKind::RumourGiver {
                    rumour: 3,
                    point: 0,
                },
                0,
                -8900.0,
                -140.0,
            ),
        ];
        let (visits, skipped) = plan(&rows, 0);
        assert_eq!(skipped, vec![(2, "phase not entered"), (3, "other map")]);
        let statics: Vec<&Visit> = visits.iter().filter(|v| v.start.is_none()).collect();
        assert_eq!(
            statics.len(),
            2,
            "the far row is past CLUSTER_YD: {visits:?}"
        );
        let giver = visits
            .iter()
            .find(|v| v.start.is_some())
            .expect("a Rumour visit");
        assert_eq!(giver.rows, vec![4]);
        assert_eq!(giver.start, Some(Start::Giver { id: 3, point: 0 }));
    }

    #[test]
    fn the_verdict_names_each_failed_measure() {
        let good = Measures {
            created: true,
            model: Some(true),
            fits: true,
            ..Default::default()
        };
        assert!(verdict(&good).is_empty(), "ok");
        assert_eq!(
            verdict(&Measures {
                model: Some(false),
                sunk: true,
                fits: false,
                deep: true,
                ..good.clone()
            }),
            vec!["model", "sunk", "body", "water"]
        );
        assert!(
            verdict(&Measures {
                model: None,
                ..good.clone()
            })
            .is_empty(),
            "unknown passes"
        );
        assert_eq!(verdict(&Measures::default()), vec!["not created"]);
        assert_eq!(verdict(&Measures::skipped("other map")), vec!["other map"]);
        assert_eq!(verdict(&Measures::merge(Vec::new())), vec!["dropped"]);
    }

    #[test]
    fn the_report_counts_and_lists_failures_by_kind() {
        let rows = vec![
            row("a", AuditKind::Creature, 0, 1.0, 2.0),
            row("b \"quoted\"", AuditKind::HubNpc, 0, 3.0, 4.0),
            row("c", AuditKind::GameObject, 0, 5.0, 6.0),
        ];
        let ok = Measures {
            created: true,
            model: Some(true),
            fits: true,
            drawn_z: Some(1.0),
            ..Default::default()
        };
        let sunk = Measures {
            sunk: true,
            ..ok.clone()
        };
        let (json, md) = reports(
            &rows,
            &[Some(ok), Some(sunk), Some(Measures::skipped("other map"))],
        );
        assert!(json.contains(r#""what": "b \"quoted\"""#), "{json}");
        assert!(json.contains(r#""verdict": "sunk""#), "{json}");
        assert!(md.contains("## Hub NPC"), "{md}");
        assert!(
            md.ends_with("3 rows: 1 ok, 1 failed, 1 not measured.\n"),
            "{md}"
        );
    }
}
