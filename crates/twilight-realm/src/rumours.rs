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
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Grade {
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
    pub(crate) name: String,
    #[allow(dead_code)]
    pub(crate) grade: Grade,
    #[serde(default)]
    pub(crate) kind: Class,
    #[serde(default)]
    pub(crate) calling: Option<Span>,
    pub(crate) map: u32,
    /// `[x0, y0, x1, y1]` boxes on `map`.
    #[serde(default)]
    pub(crate) bounds: Vec<[f32; 4]>,
    /// Percent per kill, before the rank factor.
    pub(crate) chance: f32,
    #[serde(default)]
    pub(crate) droppers: Vec<Dropper>,
    pub(crate) giver: u32,
    pub(crate) points: Vec<Point>,
    pub(crate) quests: Vec<u32>,
    #[serde(default)]
    timer_min: Option<u32>,
}

impl Def {
    /// Minutes the instance lasts: the content's, else 30 plus 15 per step after the first.
    /// `None` for a calling Rumour.
    fn minutes(&self) -> Option<u32> {
        match self.kind {
            Class::Calling => None,
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
    rumour: Vec<Def>,
}

fn shipped() -> &'static File {
    static FILE: OnceLock<File> = OnceLock::new();
    FILE.get_or_init(|| {
        let file: File = toml::from_str(include_str!("../content/rumours.toml"))
            .expect("content/rumours.toml parses");
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
        let all: Vec<&Def> = defs().into_iter().chain(std::iter::once(&d)).collect();
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
    // A zero base chance is fine for a Rumour that only its droppers roll.
    assert!(
        ((0.01..=100.0).contains(&d.chance) || (d.chance == 0.0 && !d.droppers.is_empty()))
            && d.droppers
                .iter()
                .all(|r| (0.01..=100.0).contains(&r.chance)),
        "Rumour {id}'s chances must be 0.01 to 100"
    );
    match d.kind {
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
        // TODO(#290): require non-empty `pages` as well once readable items land.
        assert_eq!(
            note.max_count, 1,
            "Rumour {id}'s note {} needs max_count = 1",
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
        assert!(
            q.giver == Giver::Npc(d.giver) && q.ender() == d.giver,
            "quest {qid} of Rumour {id} must have the Rumour's giver as giver and ender"
        );
        assert_eq!(
            q.after.ids(),
            after.as_slice(),
            "quest {qid} of Rumour {id} must come after the step before it"
        );
        after = vec![qid];
    }
    let mine = |o: &&Def| o.id != id && o.quests.iter().any(|q| d.quests.contains(q));
    assert!(
        !all.iter().any(mine),
        "a quest of Rumour {id} belongs to another Rumour too"
    );
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
}

impl Rumour {
    fn done(&self) -> bool {
        self.audience.0.iter().all(|g| self.handed_in.contains(g))
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
                    && !d.quests.iter().all(|q| c.quests.completed.contains(q))
            }
            _ => self.held(c.guid) < ACTIVE_LIMIT,
        }
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

    /// Notes that `guid` handed in quest `quest`; the last step of a Rumour counts them in.
    pub(crate) fn hand_in(&mut self, guid: u64, quest: u32) {
        for r in &mut self.list {
            let Some(d) = def(r.id) else { continue };
            let Some(step) = d.quests.iter().position(|&q| q == quest) else {
                continue;
            };
            if !r.audience.admits(guid) {
                continue;
            }
            r.step = r.step.max(step + 1);
            if step + 1 == d.quests.len() {
                r.handed_in.insert(guid);
            }
        }
    }

    /// Removes and returns the instances that ended: the timer ran out or everyone handed in.
    pub(crate) fn take_ended(&mut self, now: f64) -> Vec<Rumour> {
        let (ended, kept) = std::mem::take(&mut self.list)
            .into_iter()
            .partition(|r| r.done() || r.ends.is_some_and(|t| now >= t));
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
