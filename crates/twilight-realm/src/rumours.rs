//! Rumours (spec #284, ADR 0014): a kill rolls one, the winner's loot gains a note, and the
//! credited players (its Audience) are shown a giver standing at one of the Rumour's points. The
//! content is `content/rumours.toml`; the instances live on the realm, are not saved, and end
//! when their timer runs out or every Audience member has handed the last step in.

use std::collections::HashSet;
use std::sync::{OnceLock, RwLock};

use serde::Deserialize;

use crate::audience::Audience;
use crate::characters::Char;
use crate::quests::content::{self as quest_content, Giver, Kind};
use crate::rng::Rng;
use crate::world::Kill;

/// A character holds at most this many ordinary Rumours at once.
const ACTIVE_LIMIT: usize = 3;

/// `grade`: how hard the Rumour's steps run. The data ticket gives it XP.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Grade {
    #[default]
    Easy,
    Hard,
    Deadly,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Class {
    #[default]
    Ordinary,
    Calling,
    /// A Legend Rumour (#331): a note that hints at a Legendary Fish. It has no giver, points,
    /// quests or timer, and rolls on a catch in the fish's pool or a kill of a dropper.
    Legend,
}

/// The Callings a calling Rumour moves a character between.
#[derive(Deserialize)]
pub(crate) struct Span {
    pub(crate) from: String,
    pub(crate) to: String,
}

/// A creature entry that rolls the Rumour at its own chance.
#[derive(Deserialize)]
pub(crate) struct Dropper {
    pub(crate) entry: u32,
    pub(crate) chance: f32,
}

/// A spot the giver may stand at, with the note the looter gets for it.
#[derive(Deserialize)]
pub(crate) struct Point {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) landmark: String,
    pub(crate) note: u32,
}

/// One Rumour of the content.
#[derive(Deserialize)]
pub(crate) struct Def {
    pub(crate) id: u32,
    #[allow(dead_code)]
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) grade: Grade,
    #[serde(default)]
    pub(crate) kind: Class,
    #[serde(default)]
    pub(crate) calling: Option<Span>,
    #[serde(default)]
    pub(crate) map: u32,
    /// `[x0, y0, x1, y1]` boxes on `map`.
    #[serde(default)]
    pub(crate) bounds: Vec<[f32; 4]>,
    /// Percent per kill, before the rank factor.
    #[serde(default)]
    pub(crate) chance: f32,
    #[serde(default)]
    pub(crate) droppers: Vec<Dropper>,
    #[serde(default)]
    pub(crate) giver: u32,
    #[serde(default)]
    pub(crate) points: Vec<Point>,
    #[serde(default)]
    pub(crate) quests: Vec<u32>,
    #[serde(default)]
    timer_min: Option<u32>,
    #[serde(default)]
    pub(crate) spawn: Vec<Spawn>,
    /// A legend Rumour's Legendary Fish, by trophy item.
    #[serde(default)]
    pub(crate) legend: Option<u32>,
    /// A legend Rumour's note, a quest item with `pages`.
    #[serde(default)]
    pub(crate) note: Option<u32>,
    /// A legend Rumour's percent per catch in its fish's pool.
    #[serde(default)]
    pub(crate) catch_chance: f32,
}

/// A spot a step's spawn may stand at.
#[derive(Deserialize)]
pub(crate) struct Spot {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

/// A thing to use that a step spawns.
#[derive(Deserialize)]
pub(crate) struct UseObject {
    pub(crate) entry: u32,
    pub(crate) display: u32,
}

/// What accepting a step makes for the Audience: `count` of a creature or of a use object at one
/// of `points`, rolled once.
#[derive(Deserialize)]
pub(crate) struct Spawn {
    pub(crate) quest: u32,
    #[serde(default)]
    pub(crate) creature: Option<u32>,
    #[serde(default)]
    pub(crate) object: Option<UseObject>,
    #[serde(default = "one")]
    pub(crate) count: u32,
    /// The creature's level, when not the giver's.
    #[serde(default)]
    pub(crate) level: Option<u8>,
    pub(crate) points: Vec<Spot>,
}

fn one() -> u32 {
    1
}

/// The quest XP multiplier by Grade.
#[derive(Deserialize, Clone, Copy)]
struct GradeXp {
    easy: u32,
    hard: u32,
    deadly: u32,
}

impl Default for GradeXp {
    fn default() -> Self {
        Self {
            easy: 1,
            hard: 2,
            deadly: 3,
        }
    }
}

/// Where the chest's items come from.
#[derive(Deserialize)]
struct ChestPools {
    /// Names the green pool; only `"northshire_world"` exists.
    green: String,
    /// The Dungeon pool: gear rows.
    blue: Vec<u32>,
}

impl Def {
    /// Minutes the instance lasts: the content's, else 30 plus 15 per step after the first.
    /// `None` for a calling Rumour.
    fn minutes(&self) -> Option<u32> {
        match self.kind {
            Class::Calling | Class::Legend => None,
            Class::Ordinary => Some(
                self.timer_min
                    .unwrap_or(30 + 15 * (self.quests.len() as u32 - 1)),
            ),
        }
    }

    fn holds(&self, map: u32, at: [f32; 3]) -> bool {
        map == self.map
            && self.bounds.iter().any(|&[x0, y0, x1, y1]| {
                (x0.min(x1)..=x0.max(x1)).contains(&at[0])
                    && (y0.min(y1)..=y0.max(y1)).contains(&at[1])
            })
    }

    /// The notes of its points.
    fn notes(&self) -> Vec<u32> {
        self.points.iter().map(|p| p.note).collect()
    }
}

/// The chance multipliers by creature rank.
#[derive(Deserialize, Clone, Copy)]
struct RankFactor {
    normal: f32,
    elite: f32,
    boss: f32,
}

impl Default for RankFactor {
    fn default() -> Self {
        Self {
            normal: 1.0,
            elite: 3.0,
            boss: 3.0,
        }
    }
}

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    rank_factor: RankFactor,
    #[serde(default)]
    grade_xp: GradeXp,
    /// Required in the shipped file; a test's added Rumours leave it out.
    #[serde(default)]
    chest_pools: Option<ChestPools>,
    #[serde(default)]
    rumour: Vec<Def>,
}

fn shipped() -> &'static File {
    static FILE: OnceLock<File> = OnceLock::new();
    FILE.get_or_init(|| {
        let file: File = toml::from_str(include_str!("../content/rumours.toml"))
            .expect("content/rumours.toml parses");
        let pools = file
            .chest_pools
            .as_ref()
            .expect("content/rumours.toml has a [chest_pools] table");
        assert_eq!(
            pools.green, "northshire_world",
            "[chest_pools] green names a pool the realm does not have"
        );
        assert!(
            pools.blue.len() >= 2
                && pools
                    .blue
                    .iter()
                    .all(|&e| crate::items::template(e).is_some()),
            "[chest_pools] blue needs two or more gear rows"
        );
        let all: Vec<&Def> = file.rumour.iter().collect();
        for d in &file.rumour {
            check(d, &all);
        }
        for q in quest_content::shipped()
            .iter()
            .filter(|q| q.kind == Kind::Rumour)
        {
            assert!(
                file.rumour.iter().any(|d| d.quests.contains(&q.id)),
                "quest {} is a rumour quest and no Rumour in content/rumours.toml names it",
                q.id
            );
        }
        file
    })
}

fn added() -> &'static RwLock<Vec<&'static Def>> {
    static ADDED: RwLock<Vec<&'static Def>> = RwLock::new(Vec::new());
    &ADDED
}

/// Adds the Rumours in `toml` (the shape of `content/rumours.toml`) alongside the shipped ones,
/// for the rest of the process; an id already present is left as it is. A test seam
/// (`Realm::with_rumours`).
pub(crate) fn add(toml: &str) {
    let parsed: File = toml::from_str(toml).expect("the added rumours parse");
    for d in parsed.rumour {
        if def(d.id).is_some() {
            continue;
        }
        // A parallel test may have added `d` between the check above and here: leave it out.
        let all: Vec<&Def> = defs()
            .into_iter()
            .filter(|o| o.id != d.id)
            .chain(std::iter::once(&d))
            .collect();
        check(&d, &all);
        // Parallel tests add the same Rumours; the push rechecks under the lock.
        let mut extra = added().write().unwrap();
        if extra.iter().all(|a| a.id != d.id) {
            extra.push(Box::leak(Box::new(d)));
        }
    }
}

/// Every Rumour: the shipped ones, then any a test added.
pub(crate) fn defs() -> Vec<&'static Def> {
    shipped()
        .rumour
        .iter()
        .chain(added().read().unwrap().iter().copied())
        .collect()
}

pub(crate) fn def(id: u32) -> Option<&'static Def> {
    defs().into_iter().find(|d| d.id == id)
}

/// What the content load refuses in `d`; `all` is every Rumour, `d` among them.
fn check(d: &Def, all: &[&Def]) {
    let id = d.id;
    assert_eq!(
        all.iter().filter(|o| o.id == id).count(),
        1,
        "Rumour {id} is defined twice"
    );
    if d.kind == Class::Legend {
        check_legend(d);
        return;
    }
    // A zero base chance is fine for a Rumour that only its droppers roll.
    assert!(
        ((0.01..=100.0).contains(&d.chance) || (d.chance == 0.0 && !d.droppers.is_empty()))
            && d.droppers
                .iter()
                .all(|r| (0.01..=100.0).contains(&r.chance)),
        "Rumour {id}'s chances must be 0.01 to 100"
    );
    match d.kind {
        Class::Legend => unreachable!("checked above"),
        Class::Ordinary => {
            assert!(!d.bounds.is_empty(), "Rumour {id} has no bounds");
            assert!(
                d.calling.is_none(),
                "Rumour {id} is ordinary and names a calling"
            );
        }
        Class::Calling => {
            let span = d
                .calling
                .as_ref()
                .unwrap_or_else(|| panic!("calling Rumour {id} has no `calling` table"));
            assert!(
                crate::content::Calling::from_key(&span.from).is_some()
                    && crate::content::Calling::from_key(&span.to).is_some(),
                "calling Rumour {id} names a Calling the realm does not have"
            );
            assert!(d.timer_min.is_none(), "calling Rumour {id} has a timer");
        }
    }
    assert!(
        (2..=4).contains(&d.points.len()),
        "Rumour {id} needs two to four points"
    );
    assert!(!d.quests.is_empty(), "Rumour {id} has no quests");
    assert!(
        crate::mobs::template(d.giver).is_some_and(|t| t.friendly),
        "Rumour {id}'s giver {} is no friendly creature template",
        d.giver
    );
    for p in &d.points {
        let note = quest_content::item_template(p.note)
            .or_else(|| crate::items::template(p.note))
            .unwrap_or_else(|| panic!("Rumour {id}'s note {} is no item", p.note));
        assert_eq!(
            note.max_count, 1,
            "Rumour {id}'s note {} needs max_count = 1",
            p.note
        );
        assert!(
            !note.pages.is_empty(),
            "Rumour {id}'s note {} needs pages",
            p.note
        );
        assert_eq!(
            all.iter()
                .flat_map(|o| o.points.iter())
                .filter(|o| o.note == p.note)
                .count(),
            1,
            "note {} is the note of more than one point",
            p.note
        );
        assert!(
            note.description.contains(&p.landmark),
            "Rumour {id}'s note {} does not name its landmark {:?}",
            p.note,
            p.landmark
        );
    }
    let mut after: Vec<u32> = Vec::new();
    for &qid in &d.quests {
        let q = quest_content::quest(qid)
            .unwrap_or_else(|| panic!("Rumour {id} names quest {qid}, which does not exist"));
        assert!(
            q.kind == Kind::Rumour,
            "quest {qid} of Rumour {id} is no rumour quest"
        );
        // A calling Rumour's last quest may be handed in to the trainer who gives the Advancement
        // into `calling.to`.
        let trainer = d.kind == Class::Calling
            && qid == *d.quests.last().expect("checked above")
            && d.calling.as_ref().is_some_and(|span| {
                quest_content::quests().iter().any(|a| {
                    a.advancement.as_ref().is_some_and(|adv| adv.to == span.to)
                        && a.giver.npc() == Some(q.ender())
                })
            });
        assert!(
            q.giver == Giver::Npc(d.giver) && (q.ender() == d.giver || trainer),
            "quest {qid} of Rumour {id} must have the Rumour's giver as giver and ender"
        );
        assert_eq!(
            q.after.ids(),
            after.as_slice(),
            "quest {qid} of Rumour {id} must come after the step before it"
        );
        after = vec![qid];
    }
    for s in &d.spawn {
        assert!(
            d.quests.contains(&s.quest),
            "a spawn of Rumour {id} names quest {}, which is not one of its quests",
            s.quest
        );
        assert!(
            s.creature.is_some() != s.object.is_some(),
            "a spawn of Rumour {id} needs exactly one of `creature` and `object`"
        );
        assert!(
            !s.points.is_empty() && s.count > 0,
            "a spawn of Rumour {id} needs points and a count"
        );
        if let Some(entry) = s.creature {
            assert!(
                crate::mobs::template(entry).is_some(),
                "a spawn of Rumour {id} names creature {entry}, which is no template"
            );
        }
        if let Some(o) = &s.object {
            let wanted = quest_content::quest(s.quest)
                .is_some_and(|q| q.use_object.iter().any(|&(e, _)| e == o.entry));
            assert!(
                wanted,
                "a spawn of Rumour {id} makes object {}, which quest {} does not ask to use",
                o.entry, s.quest
            );
        }
    }
    let mine = |o: &&Def| o.id != id && o.quests.iter().any(|q| d.quests.contains(q));
    assert!(
        !all.iter().any(mine),
        "a quest of Rumour {id} belongs to another Rumour too"
    );
}

/// What the content load refuses in legend Rumour `d`.
fn check_legend(d: &Def) {
    let id = d.id;
    let legend = d
        .legend
        .unwrap_or_else(|| panic!("legend Rumour {id} names no `legend`"));
    assert!(
        crate::fish::legend(legend).is_some(),
        "legend Rumour {id} names {legend}, which is no [[legend]]"
    );
    let note = d
        .note
        .unwrap_or_else(|| panic!("legend Rumour {id} has no `note`"));
    let item = quest_content::item_template(note)
        .or_else(|| crate::items::template(note))
        .unwrap_or_else(|| panic!("legend Rumour {id}'s note {note} is no item"));
    assert!(
        item.max_count == 1 && !item.pages.is_empty(),
        "legend Rumour {id}'s note {note} needs max_count = 1 and pages"
    );
    assert!(
        (0.01..=100.0).contains(&d.catch_chance)
            && d.droppers
                .iter()
                .all(|r| (0.01..=100.0).contains(&r.chance)),
        "legend Rumour {id}'s chances must be 0.01 to 100"
    );
    assert!(
        d.calling.is_none() && d.giver == 0 && d.points.is_empty() && d.quests.is_empty(),
        "legend Rumour {id} has a calling, giver, points or quests"
    );
}

/// The legend Rumours a roll of `chance_of` hit for `c`, who has neither the hint nor the fish:
/// each `(legend, note)`, in file order.
fn legend_hits(
    c: &Char,
    rng: &mut Rng,
    chance_of: impl Fn(&Def) -> Option<f32>,
) -> Vec<(u32, u32)> {
    let mut hits = Vec::new();
    for d in defs().into_iter().filter(|d| d.kind == Class::Legend) {
        let (Some(legend), Some(note), Some(chance)) = (d.legend, d.note, chance_of(d)) else {
            continue;
        };
        if c.legends_caught.contains(&legend) || c.legend_hints.contains(&legend) {
            continue;
        }
        if rng.frand(0.0, 100.0) < chance {
            hits.push((legend, note));
        }
    }
    hits
}

/// The legend Rumours a catch in pool `pool` hits for `c`.
pub(crate) fn legend_catch_hits(c: &Char, pool: &str, rng: &mut Rng) -> Vec<(u32, u32)> {
    legend_hits(c, rng, |d| {
        let l = crate::fish::legend(d.legend?)?;
        (l.pool == pool).then_some(d.catch_chance)
    })
}

/// The legend Rumours a kill of creature `entry` hits for `c`.
pub(crate) fn legend_kill_hits(c: &Char, entry: u32, rng: &mut Rng) -> Vec<(u32, u32)> {
    legend_hits(c, rng, |d| {
        d.droppers
            .iter()
            .find(|r| r.entry == entry)
            .map(|r| r.chance)
    })
}

/// The note of the legend Rumour for Legendary Fish `legend`.
pub(crate) fn legend_note(legend: u32) -> Option<u32> {
    defs()
        .into_iter()
        .find(|d| d.kind == Class::Legend && d.legend == Some(legend))
        .and_then(|d| d.note)
}

/// One Rumour under way.
pub(crate) struct Rumour {
    pub(crate) id: u32,
    pub(crate) audience: Audience,
    /// 0 until the next tick makes the giver.
    pub(crate) giver_guid: u64,
    /// Which of the Rumour's points the giver stands at.
    pub(crate) point: usize,
    /// The killed creature's level, the giver's.
    pub(crate) level: u8,
    /// The realm clock at the roll, in seconds.
    #[allow(dead_code)]
    pub(crate) started: f64,
    pub(crate) ends: Option<f64>,
    /// How many steps someone has handed in so far, at most.
    pub(crate) step: usize,
    pub(crate) handed_in: HashSet<u64>,
    /// The step quests whose spawns were made already.
    spawned: HashSet<u32>,
    /// The guids of the chests its hand-ins made.
    chests: Vec<u64>,
}

impl Rumour {
    pub(crate) fn chests(&self) -> &[u64] {
        &self.chests
    }

    fn done(&self) -> bool {
        self.audience.0.iter().all(|g| self.handed_in.contains(g))
    }
}

/// The GameObject entry of a Rumour's chest. It has no template: the display is the chest's.
pub(crate) const CHEST_ENTRY: u32 = 980199;

/// What the realm makes on its next tick, for the instance at an index of the list.
pub(crate) enum Order {
    /// The spawns of step quest `quest`, which someone accepted first.
    Step { rumour: usize, quest: u32 },
    /// A chest for `guid` alone, who handed in the last step.
    Chest { rumour: usize, guid: u64 },
}

/// The XP multiplier of the Rumour that owns quest `quest`: its Grade's, or 1 for a calling
/// Rumour or a quest that is none's.
pub(crate) fn xp_factor(quest: u32) -> u32 {
    let table = shipped().grade_xp;
    match defs().into_iter().find(|d| d.quests.contains(&quest)) {
        Some(d) if d.kind == Class::Ordinary => match d.grade {
            Grade::Easy => table.easy,
            Grade::Hard => table.hard,
            Grade::Deadly => table.deadly,
        },
        _ => 1,
    }
}

/// Draws `n` distinct entries of `pool`.
fn draw(pool: &[u32], n: usize, rng: &mut Rng) -> Vec<u32> {
    let mut left = pool.to_vec();
    (0..n.min(left.len()))
        .map(|_| left.swap_remove(rng.below(left.len() as u32) as usize))
        .collect()
}

/// What a Rumour's chest holds for a giver of `level`: an easy one a World green, a hard one a
/// Dungeon blue and a deadly one two. A calling Rumour has no chest.
pub(crate) fn chest_items(d: &Def, level: u8, rng: &mut Rng) -> Vec<u32> {
    let blue = || {
        &shipped()
            .chest_pools
            .as_ref()
            .expect("checked on load")
            .blue[..]
    };
    if d.kind == Class::Calling {
        return Vec::new();
    }
    match d.grade {
        Grade::Easy => draw(
            crate::rules::loot::northshire_greens(
                level.min(crate::rules::loot::NORTHSHIRE_MAX_LEVEL),
            ),
            1,
            rng,
        ),
        Grade::Hard => draw(blue(), 1, rng),
        Grade::Deadly => draw(blue(), 2, rng),
    }
}

/// What a won roll tells the realm to do.
pub(crate) struct Found {
    pub(crate) audience: Vec<u64>,
    /// Who gets the note in their loot.
    pub(crate) looter: u64,
    pub(crate) note: u32,
    /// The chat line the rest of the Audience gets.
    pub(crate) line: String,
}

/// The Rumours under way.
#[derive(Default)]
pub(crate) struct Rumours {
    list: Vec<Rumour>,
    orders: Vec<Order>,
}

impl Rumours {
    /// The active ordinary Rumours that admit `guid`.
    fn held(&self, guid: u64) -> usize {
        self.list
            .iter()
            .filter(|r| def(r.id).is_some_and(|d| d.kind == Class::Ordinary))
            .filter(|r| r.audience.admits(guid))
            .count()
    }

    /// The kill's roll: the first Rumour, in file order, that holds the corpse (or names its
    /// entry) and wins its draw. `credited` are the characters sharing the kill, `killer` the one
    /// who struck it, and `map` the map it fell on.
    pub(crate) fn roll(
        &mut self,
        credited: &[&Char],
        killer: u64,
        kill: &Kill,
        map: u32,
        now: f64,
        rng: &mut Rng,
    ) -> Option<Found> {
        let factors = shipped().rank_factor;
        let factor = match crate::mobs::template(kill.entry).map_or(0, |t| t.rank) {
            0 => factors.normal,
            1 | 2 => factors.elite,
            _ => factors.boss,
        };
        for d in defs() {
            // A legend Rumour rolls apart, outside the active limit.
            if d.kind == Class::Legend {
                continue;
            }
            let dropper = d.droppers.iter().find(|r| r.entry == kill.entry);
            let calling = d.kind == Class::Calling;
            let inside = if calling {
                map == d.map
            } else {
                d.holds(map, kill.position)
            };
            let chance = match dropper {
                Some(r) => r.chance,
                None if inside => d.chance,
                None => continue,
            };
            let audience: Vec<u64> = credited
                .iter()
                .filter(|c| self.admits_new(d, c))
                .map(|c| c.guid)
                .collect();
            if audience.is_empty() || rng.frand(0.0, 100.0) >= chance * factor {
                continue;
            }
            let point = rng.below(d.points.len() as u32) as usize;
            let looter = if audience.contains(&killer) {
                killer
            } else {
                audience[0]
            };
            let p = &d.points[point];
            let giver = crate::mobs::template(d.giver).map_or("Someone", |t| t.name.as_str());
            let name = credited
                .iter()
                .find(|c| c.guid == looter)
                .map_or("Someone", |c| c.name.as_str());
            let line = format!("{name} found a rumour: {giver} waits at {}.", p.landmark);
            self.list.push(Rumour {
                id: d.id,
                audience: Audience::of(&audience),
                giver_guid: 0,
                point,
                level: kill.level,
                started: now,
                ends: d.minutes().map(|m| now + f64::from(m) * 60.0),
                step: 0,
                handed_in: HashSet::new(),
                spawned: HashSet::new(),
                chests: Vec::new(),
            });
            return Some(Found {
                audience,
                looter,
                note: p.note,
                line,
            });
        }
        None
    }

    /// Whether `c` may be told Rumour `d`: an ordinary one to anyone under the cap who does not
    /// hold it already, a calling one to those at the Tier Cap of its `from` who have not
    /// finished it.
    fn admits_new(&self, d: &Def, c: &Char) -> bool {
        if self
            .list
            .iter()
            .any(|r| r.id == d.id && r.audience.admits(c.guid))
        {
            return false;
        }
        match (&d.kind, &d.calling) {
            (Class::Calling, Some(span)) => {
                c.calling.key() == span.from
                    && crate::world::at_tier_cap(c)
                    && !c.finished_rumours.contains(&d.id)
            }
            _ => self.held(c.guid) < ACTIVE_LIMIT,
        }
    }

    /// A dev aid (`WOW_PROBE_RUMOUR`): starts Rumour `id` for `c` alone with its giver at point
    /// `point`, as a won roll would. The point's note; `None` for an unknown Rumour or point, or
    /// one `c` holds already.
    pub(crate) fn force(&mut self, id: u32, point: usize, c: &Char, now: f64) -> Option<u32> {
        let d = def(id)?;
        let note = d.points.get(point)?.note;
        if self
            .list
            .iter()
            .any(|r| r.id == id && r.audience.admits(c.guid))
        {
            return None;
        }
        self.list.push(Rumour {
            id,
            audience: Audience::of(&[c.guid]),
            giver_guid: 0,
            point,
            level: c.level,
            started: now,
            ends: d.minutes().map(|m| now + f64::from(m) * 60.0),
            step: 0,
            handed_in: HashSet::new(),
            spawned: HashSet::new(),
            chests: Vec::new(),
        });
        Some(note)
    }

    /// The instances whose giver is not made yet: index, Rumour and level.
    pub(crate) fn unspawned(&self) -> Vec<usize> {
        (0..self.list.len())
            .filter(|&i| self.list[i].giver_guid == 0)
            .collect()
    }

    pub(crate) fn get(&self, i: usize) -> &Rumour {
        &self.list[i]
    }

    pub(crate) fn set_giver(&mut self, i: usize, guid: u64) {
        self.list[i].giver_guid = guid;
    }

    /// Notes that `c` accepted quest `quest`; the first of an Audience to take a step has its
    /// spawns made.
    pub(crate) fn accept(&mut self, guid: u64, quest: u32) {
        for (i, r) in self.list.iter_mut().enumerate() {
            let Some(d) = def(r.id) else { continue };
            if d.spawn.iter().any(|s| s.quest == quest)
                && r.audience.admits(guid)
                && r.spawned.insert(quest)
            {
                self.orders.push(Order::Step { rumour: i, quest });
            }
        }
    }

    /// Notes that `c` handed in quest `quest`. The last step of a Rumour counts them in: an
    /// ordinary one orders them a chest, a calling one is written on their save.
    pub(crate) fn hand_in(&mut self, c: &mut Char, quest: u32) {
        let guid = c.guid;
        for (i, r) in self.list.iter_mut().enumerate() {
            let Some(d) = def(r.id) else { continue };
            let Some(step) = d.quests.iter().position(|&q| q == quest) else {
                continue;
            };
            if !r.audience.admits(guid) {
                continue;
            }
            r.step = r.step.max(step + 1);
            if step + 1 == d.quests.len() && r.handed_in.insert(guid) {
                match d.kind {
                    Class::Ordinary => self.orders.push(Order::Chest { rumour: i, guid }),
                    Class::Calling if !c.finished_rumours.contains(&d.id) => {
                        c.finished_rumours.push(d.id);
                        c.finished_rumours.sort_unstable();
                    }
                    Class::Calling | Class::Legend => {}
                }
            }
        }
    }

    /// The orders the accepts and hand-ins since the last call left.
    pub(crate) fn take_orders(&mut self) -> Vec<Order> {
        std::mem::take(&mut self.orders)
    }

    /// Records that chest `guid` was made for the instance at `i`.
    pub(crate) fn add_chest(&mut self, i: usize, guid: u64) {
        self.list[i].chests.push(guid);
    }

    /// Removes and returns the instances that ended: the timer ran out, or everyone handed in
    /// and no chest it made (`holds` says which) still has items in it.
    pub(crate) fn take_ended(&mut self, now: f64, holds: impl Fn(u64) -> bool) -> Vec<Rumour> {
        let (ended, kept) = std::mem::take(&mut self.list).into_iter().partition(|r| {
            (r.done() && !r.chests.iter().any(|&g| holds(g))) || r.ends.is_some_and(|t| now >= t)
        });
        self.list = kept;
        ended
    }

    /// The giver guids of the Rumours under way.
    pub(crate) fn givers(&self) -> Vec<u64> {
        self.list
            .iter()
            .map(|r| r.giver_guid)
            .filter(|&g| g != 0 && g != u64::MAX)
            .collect()
    }
}

/// The quests of Rumour `id` and the notes of its points, for the end.
pub(crate) fn leftovers(id: u32) -> (Vec<u32>, Vec<u32>) {
    def(id).map_or_else(Default::default, |d| (d.quests.clone(), d.notes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_rumours_validate() {
        let f = shipped().rank_factor;
        assert!(f.normal > 0.0 && f.elite >= f.normal && f.boss >= f.normal);
    }
}
