//! Marked (#179): dying to your own Bounty Hunter Marks you, with a tenth off your base stats and
//! no new hunter, while the victor roams where it won; killing it or two hours of logged-in play
//! ends it, and a save brings the victor back at its home spot.

mod common;
mod gear_common;

use benilla_protocol::messages::LootItem;
use benilla_protocol::{EntityKind, SessionEvent};
use common::*;
use twilight_realm::rules::notoriety::{
    blood_coin, coin_drop, heat_level, hunter_secs, marked_secs, marked_stat_pct,
};
use twilight_realm::{Host, CROSSROADS};

/// Notoriety that stays at Heat 4 through every drain here.
const HOT: f32 = 5000.0;
const HUNTER_ENTRIES: std::ops::Range<u32> = 980_000..990_000;
/// Out in the Barrens, well clear of the Hub.
const OUT: [f32; 2] = [CROSSROADS[0], CROSSROADS[1] + 300.0];
/// The floor's height.
const FLOOR: f32 = 82.0;
/// A two-hander that fells a level 2 hunter in a few blows.
const MEGACHOPPER: u32 = 9490;
const MAIN_HAND: u8 = 15;
/// Strength, as `unit_stat` numbers it.
const STRENGTH: u8 = 0;

/// Flat ground, open sight everywhere.
struct Floor;

impl Host for Floor {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(FLOOR)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// A character's save at `level` with `notoriety`, holding the Megachopper when `armed`.
fn save(level: u8, notoriety: f32, armed: bool) -> (u64, String) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Quarry", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let me = roster(&mut maker)[0].guid;
    let blob = maker.export_character(me).unwrap();
    (me, outfit(&blob, level, notoriety, armed))
}

/// `blob` at `level` with `notoriety`, its starting items swapped for the Megachopper when
/// `armed`.
fn outfit(blob: &str, level: u8, notoriety: f32, armed: bool) -> String {
    let mut blob = blob.to_string();
    let at = blob.find("\nlevel = ").unwrap() + 1;
    let end = at + blob[at..].find('\n').unwrap();
    blob.replace_range(at..end, &format!("level = {level}"));
    let blob = blob.replacen(
        "xp = 0\n",
        &format!("xp = 0\nnotoriety = {notoriety:?}\n"),
        1,
    );
    if !armed {
        return blob;
    }
    let mut blob = blob;
    if let Some(at) = blob.find("[[items]]") {
        blob.truncate(at);
    }
    blob.replace("items = []\n", "")
        + &format!("\n[[items]]\nslot = {MAIN_HAND}\nentry = {MEGACHOPPER}\ncount = 1\n")
}

/// One player in a realm of their own, and what their client has been told.
struct Quarry {
    realm: Realm,
    me: u64,
    /// Where the player last stood.
    at: [f32; 3],
    events: Vec<SessionEvent>,
    /// Seconds of play so far.
    clock: f32,
    /// Each unit's latest move: when it began, its path and its length in seconds.
    moves: std::collections::HashMap<u64, (f32, Vec<[f32; 3]>, f32)>,
}

impl Quarry {
    fn enter(me: u64, blob: &str) -> Self {
        let mut realm = realm();
        realm.import_character(blob).unwrap();
        realm.handle(RealmCommand::EnterWorld { guid: me });
        let mut q = Self {
            realm,
            me,
            at: [CROSSROADS[0], CROSSROADS[1], FLOOR],
            events: Vec::new(),
            clock: 0.0,
            moves: Default::default(),
        };
        q.run(0.1);
        q
    }

    fn run(&mut self, secs: f32) {
        self.run_by(secs, 0.1);
    }

    /// `secs` of play in steps of `dt`.
    fn run_by(&mut self, secs: f32, dt: f32) {
        for _ in 0..(secs / dt).round().max(1.0) as u32 {
            self.realm.tick(dt, &Floor);
            self.clock += dt;
            let got = drain(&mut self.realm);
            self.note(got);
        }
    }

    /// Keeps `events`, and the moves among them; acknowledges a teleport, as the client does.
    fn note(&mut self, events: Vec<SessionEvent>) {
        for e in &events {
            if let SessionEvent::Teleport { counter, .. } = e {
                self.realm
                    .handle(RealmCommand::TeleportAck { counter: *counter });
            }
            if let SessionEvent::MonsterMove {
                guid,
                path,
                duration_ms,
                ..
            } = e
            {
                let secs = *duration_ms as f32 / 1000.0;
                self.moves.insert(*guid, (self.clock, path.clone(), secs));
            }
        }
        self.events.extend(events);
    }

    fn cmd(&mut self, command: RealmCommand) -> Vec<SessionEvent> {
        self.realm.handle(command);
        let got = drain(&mut self.realm);
        self.note(got.clone());
        got
    }

    fn go(&mut self, xy: [f32; 2]) {
        self.cmd(RealmCommand::DevTeleport {
            x: xy[0],
            y: xy[1],
            facing: None,
        });
        self.at = [xy[0], xy[1], FLOOR];
        self.run(0.5);
    }

    /// The Bounty Hunters this client was shown, by guid, each once.
    fn hunters(&self) -> Vec<u64> {
        let mut guids = Vec::new();
        for e in &self.events {
            if let SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                fields,
                ..
            } = e
            {
                if fields
                    .object_entry()
                    .is_some_and(|e| HUNTER_ENTRIES.contains(&e))
                    && !guids.contains(guid)
                {
                    guids.push(*guid);
                }
            }
        }
        guids
    }

    /// The first hunter's create: guid, entry and where it stood.
    fn first_hunter(&self) -> Option<(u64, u32, [f32; 3])> {
        self.events.iter().find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                fields,
                position,
                ..
            } => fields
                .object_entry()
                .filter(|e| HUNTER_ENTRIES.contains(e))
                .map(|e| (*guid, e, *position)),
            _ => None,
        })
    }

    /// The last value this client was told for the player's field, through `read`.
    fn mine(&self, read: impl Fn(&benilla_protocol::ObjectFields) -> Option<u32>) -> Option<u32> {
        self.events.iter().rev().find_map(|e| match e {
            SessionEvent::ObjectValues { guid, fields } if *guid == self.me => read(fields),
            SessionEvent::ObjectCreate { guid, fields, .. } if *guid == self.me => read(fields),
            _ => None,
        })
    }

    fn dead(&self) -> bool {
        self.mine(|f| f.unit_health()) == Some(0)
    }

    /// Waits at `OUT` for a hunter, then until it kills the player.
    fn die_to_a_hunter(&mut self) {
        self.go(OUT);
        let wait = hunter_secs(heat_level(HOT)).unwrap();
        self.run(wait + 5.0);
        assert!(self.first_hunter().is_some(), "a hunter came");
        self.die_where_standing();
    }

    fn die_where_standing(&mut self) {
        for _ in 0..600 {
            if self.dead() {
                return;
            }
            self.run(1.0);
        }
        panic!("ten minutes and still alive");
    }

    /// Release Spirit, back at the Hub.
    fn release(&mut self) {
        self.cmd(RealmCommand::RepopRequest);
        self.at = [CROSSROADS[0], CROSSROADS[1], FLOOR];
        self.run(0.5);
    }

    fn saved(&self) -> String {
        self.realm.export_character(self.me).unwrap()
    }

    /// The Marked time left in the save; `None` when not Marked.
    fn marked_left(&self) -> Option<f32> {
        let blob = self.saved();
        let at = blob.find("[marked]")?;
        let line = blob[at..].lines().find(|l| l.starts_with("secs_left = "))?;
        Some(line["secs_left = ".len()..].parse().unwrap())
    }

    fn rank(&self) -> u32 {
        self.saved()
            .lines()
            .find_map(|l| l.strip_prefix("bounty_rank = "))
            .map_or(0, |v| v.parse().unwrap())
    }

    /// Where this client last saw `unit`: its create, or the end of its latest move.
    fn seen_at(&self, unit: u64) -> Option<[f32; 3]> {
        self.events.iter().rev().find_map(|e| match e {
            SessionEvent::MonsterMove { guid, path, .. } if *guid == unit => path.last().copied(),
            SessionEvent::ObjectCreate { guid, position, .. } if *guid == unit => Some(*position),
            _ => None,
        })
    }

    /// Where `unit` stands now, as a client draws it: along its latest move by the time since
    /// it began, else where it was created.
    fn now_at(&self, unit: u64) -> Option<[f32; 3]> {
        let Some((began, path, secs)) = self.moves.get(&unit) else {
            return self.seen_at(unit);
        };
        let legs: Vec<f32> = path
            .windows(2)
            .map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]))
            .collect();
        let total: f32 = legs.iter().sum();
        let done = if *secs > 0.0 {
            ((self.clock - began) / secs).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let mut left = total * done;
        for (w, len) in path.windows(2).zip(&legs) {
            if left <= *len && *len > 0.0 {
                let t = left / len;
                return Some(std::array::from_fn(|i| w[0][i] + (w[1][i] - w[0][i]) * t));
            }
            left -= len;
        }
        path.last().copied()
    }

    /// Steps up to `unit` and swings at it until it falls.
    fn kill(&mut self, unit: u64) {
        for _ in 0..1200 {
            if let Some(them) = self.now_at(unit) {
                let (dx, dy) = (them[0] - self.at[0], them[1] - self.at[1]);
                let len = dx.hypot(dy).max(0.01);
                // At a run, 7 yd a second: no further than that in the step's fifth of one.
                let reach = (len - 1.5).clamp(0.0, 1.4) / len;
                self.at = [self.at[0] + dx * reach, self.at[1] + dy * reach, them[2]];
                self.cmd(RealmCommand::Move {
                    motion: Default::default(),
                    position: self.at,
                    orientation: dy.atan2(dx).rem_euclid(std::f32::consts::TAU),
                });
            }
            self.cmd(RealmCommand::AttackSwing { guid: unit });
            self.run(0.2);
            let fell = self.events.iter().any(|e| {
                matches!(e, SessionEvent::ObjectValues { guid, fields }
                    if *guid == unit && fields.unit_health() == Some(0))
            });
            if fell {
                return;
            }
        }
        let mut tally = std::collections::BTreeMap::<String, u32>::new();
        for e in &self.events {
            let k = match e {
                SessionEvent::AttackSwingError(x) => format!("{x:?}"),
                SessionEvent::AttackerState(a) if a.attacker == self.me => {
                    format!("hit {}", a.damage.min(1))
                }
                _ => continue,
            };
            *tally.entry(k).or_default() += 1;
        }
        panic!(
            "it outlived ten minutes of fighting: {tally:?} dead {}",
            self.dead()
        );
    }

    fn loot(&mut self, corpse: u64) -> Vec<LootItem> {
        self.cmd(RealmCommand::Loot { guid: corpse })
            .into_iter()
            .find_map(|e| match e {
                SessionEvent::LootResponse { items, .. } => Some(items),
                _ => None,
            })
            .expect("a loot window")
    }

    /// Whether this client was told `unit` is gone.
    fn removed(&self, unit: u64) -> bool {
        self.events.iter().any(|e| match e {
            SessionEvent::ObjectsRemoved(gone) => gone.contains(&unit),
            SessionEvent::ObjectDestroyed(g) => *g == unit,
            _ => false,
        })
    }
}

/// A level 1 player Marked by their first hunter, released at the Hub.
fn marked() -> Quarry {
    let (me, blob) = save(1, HOT, false);
    let mut q = Quarry::enter(me, &blob);
    q.die_to_a_hunter();
    q.release();
    q
}

#[test]
fn dying_to_your_own_hunter_marks_you() {
    let q = marked();
    let left = q.marked_left().expect("Marked");
    assert!(left > marked_secs() - 60.0 && left <= marked_secs());
}

#[test]
fn dying_to_anything_else_does_not_mark_you() {
    // A level 1 player pulls the mob by the Hub and lets it win.
    let (me, blob) = gear_common::save_with(1, 0, &[]);
    let (mut realm, _, events) = gear_common::enter_saved(me, &blob);
    let mob = gear_common::the_mob(&events);
    gear_common::stand_by(&mut realm, &mob);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    realm.handle(RealmCommand::AttackStop);
    let mut dead = false;
    for _ in 0..6000 {
        realm.tick(0.1, &FlatGround);
        dead |= drain(&mut realm).iter().any(|e| {
            matches!(e, SessionEvent::ObjectValues { guid, fields }
                if *guid == me && fields.unit_health() == Some(0))
        });
        if dead {
            break;
        }
    }
    assert!(dead, "the mob won");
    assert!(!realm.export_character(me).unwrap().contains("[marked]"));
}

impl Quarry {
    /// Swings at `mob` for a while, standing next to it, whoever falls.
    fn kill_or_be_killed(&mut self, mob: u64) {
        for _ in 0..120 {
            if self.dead() {
                return;
            }
            if let Some(them) = self.seen_at(mob) {
                self.at = [them[0] - 1.0, them[1], them[2]];
                self.cmd(RealmCommand::Move {
                    motion: Default::default(),
                    position: self.at,
                    orientation: 0.0,
                });
            }
            self.cmd(RealmCommand::AttackSwing { guid: mob });
            self.run(0.5);
        }
    }
}

#[test]
fn marked_takes_a_tenth_off_the_base_stats() {
    let (me, blob) = save(1, HOT, false);
    let mut q = Quarry::enter(me, &blob);
    let before = q.mine(|f| f.unit_stat(STRENGTH)).unwrap();
    q.die_to_a_hunter();
    q.release();
    let after = q.mine(|f| f.unit_stat(STRENGTH)).unwrap();
    assert_eq!(after, before * marked_stat_pct() / 100);
    assert!(after < before);
}

#[test]
fn a_second_death_to_it_neither_stacks_nor_restarts_the_clock() {
    let mut q = marked();
    let first = q.marked_left().unwrap();
    let stat = q.mine(|f| f.unit_stat(STRENGTH)).unwrap();
    let (victor, _, _) = q.first_hunter().unwrap();
    q.go(OUT);
    q.run(5.0);
    // Walk up to it and let it win again.
    q.kill_or_be_killed(victor);
    q.die_where_standing();
    q.release();
    let second = q.marked_left().expect("still Marked");
    assert!(second < first, "the clock ran on: {first} then {second}");
    assert_eq!(q.mine(|f| f.unit_stat(STRENGTH)), Some(stat));
}

#[test]
fn no_new_hunter_comes_while_marked() {
    let q = marked();
    let (_, victor, _) = q.first_hunter().unwrap();
    // Death took their Notoriety: back at Heat 4, and still Marked.
    let blob = q.saved().replacen(
        "xp = 0
",
        &format!(
            "xp = 0
notoriety = {HOT:?}
"
        ),
        1,
    );
    let mut q = Quarry::enter(q.me, &blob);
    assert!(q.marked_left().is_some());
    q.go(OUT);
    let wait = hunter_secs(heat_level(HOT)).unwrap();
    q.run(wait * 2.0);
    let others: Vec<u32> = q
        .hunter_entries()
        .into_iter()
        .filter(|&e| e != victor)
        .collect();
    assert_eq!(others, Vec::<u32>::new(), "only the victor");
}

impl Quarry {
    /// The entries of the Bounty Hunters this client was shown.
    fn hunter_entries(&self) -> Vec<u32> {
        self.events
            .iter()
            .filter_map(|e| match e {
                SessionEvent::ObjectCreate {
                    kind: EntityKind::Unit,
                    fields,
                    ..
                } => fields.object_entry().filter(|e| HUNTER_ENTRIES.contains(e)),
                _ => None,
            })
            .collect()
    }
}

#[test]
fn killing_the_victor_lifts_the_mark_pays_its_coin_and_keeps_rank_0() {
    let q = marked();
    let (_, entry, _) = q.first_hunter().unwrap();
    // Back stronger: the same character at level 60 with the Megachopper.
    let blob = outfit(&q.saved(), 60, 0.0, true);
    let mut q = Quarry::enter(q.me, &blob);
    q.go(OUT);
    q.run(1.0);
    let victor = q
        .events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                fields,
                ..
            } if fields.object_entry() == Some(entry) => Some(*guid),
            _ => None,
        })
        .expect("the victor is back");
    q.kill(victor);
    q.run(1.0);
    assert_eq!(q.marked_left(), None);
    assert_eq!(q.rank(), 0);
    let coin = q
        .loot(victor)
        .iter()
        .filter(|i| i.item_id == blood_coin())
        .map(|i| i.count)
        .sum::<u32>();
    assert_eq!(coin, coin_drop(0));
}

#[test]
fn two_hours_of_play_end_it_and_take_the_victor() {
    let q = marked();
    let left = q.marked_left().unwrap();
    // Logged out, the clock stands still.
    let blob = q.saved();
    let mut q = Quarry::enter(q.me, &blob);
    // Entering ticks once.
    assert!((left - q.marked_left().unwrap()) < 0.5);
    q.go(OUT);
    q.run(1.0);
    let back = q.hunters();
    assert_eq!(back.len(), 1, "the victor came back");
    q.run_by(left - 30.0, 1.0);
    assert!(q.marked_left().is_some(), "half a minute still to go");
    q.run_by(60.0, 1.0);
    assert_eq!(q.marked_left(), None);
    assert!(q.removed(back[0]), "the victor left with it");
}

#[test]
fn a_save_brings_the_victor_back_at_its_home_spot() {
    let q = marked();
    let (_, entry, _) = q.first_hunter().unwrap();
    let blob = q.saved();
    let home = {
        let at = blob.find("home = [").unwrap() + "home = [".len();
        let end = at + blob[at..].find(']').unwrap();
        let xyz: Vec<f32> = blob[at..end]
            .split(',')
            .map(|v| v.trim().parse().unwrap())
            .collect();
        [xyz[0], xyz[1], xyz[2]]
    };
    let mut q = Quarry::enter(q.me, &blob);
    q.go([home[0], home[1]]);
    let back = q
        .events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                kind: EntityKind::Unit,
                fields,
                position,
                ..
            } if fields.object_entry() == Some(entry) => Some(*position),
            _ => None,
        })
        .expect("the victor is back");
    assert!(
        (back[0] - home[0]).hypot(back[1] - home[1]) < 0.5,
        "at {back:?}, home {home:?}"
    );
    let flagged = q
        .events
        .iter()
        .any(|e| matches!(e, SessionEvent::GossipPoi(p) if (p.pos[0] - home[0]).abs() < 50.0));
    assert!(flagged, "a map flag on it");
}
