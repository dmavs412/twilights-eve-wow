//! Milestones (ADR 0034): facts about the whole server that the realm records once and never
//! forgets in play. The catalogue (`content/milestones.toml`) lists them; the state file,
//! `milestones.toml` in the realm's data folder, holds one table per Milestone that has a count or
//! is reached. Unlike `unlocks.toml`, the realm writes it.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::unlocks::{days_from_civil, unix_secs};

const DAY_SECS: u64 = 86_400;

/// One catalogue row.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Milestone {
    pub key: String,
    pub name: String,
    /// A Milestone that must be reached before this one can be.
    #[serde(default)]
    pub after: Option<String>,
    /// The count that reaches it: it makes the Milestone a Tally.
    #[serde(default)]
    pub goal: Option<u64>,
    /// The quests whose hand-ins add to the Tally's count.
    #[serde(default)]
    pub feeds: Vec<Feed>,
    /// The line every character in the world sees on `/1` when it is reached.
    #[serde(default)]
    pub announce: Option<String>,
    /// The Hub NPC line every character in the world hears when it is reached.
    #[serde(default)]
    pub yell: Option<Yell>,
}

/// A quest that feeds a Tally, and what each hand-in adds.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Feed {
    pub quest: u32,
    pub points: u64,
}

/// A Hub NPC's yell: its creature entry and its words.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Yell {
    pub npc: u32,
    pub text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalogue {
    milestone: Vec<Milestone>,
}

/// A catalogue's rows from its text (the shape of `content/milestones.toml`); a repeated key, an
/// `after` that names no row, `feeds` without a `goal`, a quest fed twice and a `goal` below 1
/// panic.
pub(crate) fn catalogue(text: &str) -> Vec<Milestone> {
    let file: Catalogue = toml::from_str(text).expect("content/milestones.toml parses");
    for (i, m) in file.milestone.iter().enumerate() {
        assert!(
            file.milestone[..i].iter().all(|n| n.key != m.key),
            "milestone {} is listed twice",
            m.key
        );
        if let Some(after) = &m.after {
            assert!(
                file.milestone.iter().any(|n| n.key == *after),
                "milestone {} comes after {after}, which is no Milestone",
                m.key
            );
        }
        assert!(
            m.feeds.is_empty() || m.goal.is_some(),
            "milestone {} has feeds and no goal",
            m.key
        );
        assert!(
            m.goal.is_none_or(|g| g >= 1),
            "milestone {} has a goal below 1",
            m.key
        );
        for (j, f) in m.feeds.iter().enumerate() {
            assert!(
                m.feeds[..j].iter().all(|g| g.quest != f.quest),
                "milestone {} feeds quest {} twice",
                m.key,
                f.quest
            );
        }
    }
    file.milestone
}

/// Panics on a catalogue row that names content the realm lacks: a fed quest, or a yell's Hub
/// NPC. Apart from [`catalogue`], whose shipped rows the quests and the Hub NPCs read as they load.
fn check_content(catalogue: &[Milestone]) {
    for m in catalogue {
        for f in &m.feeds {
            assert!(
                crate::quests::content::quest(f.quest).is_some(),
                "milestone {} feeds quest {}, which the content lacks",
                m.key,
                f.quest
            );
        }
        if let Some(yell) = &m.yell {
            assert!(
                crate::hub::npcs().iter().any(|n| n.entry == yell.npc),
                "milestone {}'s yell names npc {}, which is no Hub NPC entry",
                m.key,
                yell.npc
            );
        }
    }
}

/// The shipped catalogue's goal for Milestone `key`, if it is a Tally.
pub(crate) fn shipped_goal(key: &str) -> Option<u64> {
    shipped().iter().find(|m| m.key == key)?.goal
}

/// The shipped catalogue.
fn shipped() -> &'static [Milestone] {
    static FILE: OnceLock<Vec<Milestone>> = OnceLock::new();
    FILE.get_or_init(|| catalogue(include_str!("../content/milestones.toml")))
}

/// The two keys a content row may carry to wait on Milestones (ADR 0034), flattened into the row.
/// Each names a row of `content/milestones.toml`; a row with neither is always there.
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct Gate {
    /// Absent until this Milestone is reached, then there for good.
    #[serde(default)]
    pub after_milestone: Option<String>,
    /// There until this Milestone is reached, then gone for good.
    #[serde(default)]
    pub until_milestone: Option<String>,
}

impl Gate {
    /// Whether the row is there while the Milestones `reached` are.
    pub(crate) fn met<S: AsRef<str>>(&self, reached: &[S]) -> bool {
        let has = |key: &String| reached.iter().any(|r| r.as_ref() == key);
        self.after_milestone.as_ref().is_none_or(has) && !self.gone(reached)
    }

    /// Whether its `until_milestone` is among `reached`.
    pub(crate) fn gone<S: AsRef<str>>(&self, reached: &[S]) -> bool {
        self.until_milestone
            .as_ref()
            .is_some_and(|key| reached.iter().any(|r| r.as_ref() == key))
    }

    /// Panics naming `row` and a key the shipped catalogue does not list.
    pub(crate) fn check(&self, row: &str) {
        for key in [&self.after_milestone, &self.until_milestone]
            .into_iter()
            .flatten()
        {
            assert!(
                shipped().iter().any(|m| m.key == *key),
                "{row} names Milestone {key}, which content/milestones.toml does not list"
            );
        }
    }
}

/// The catalogue and what the state file holds: the Milestones reached, the Tallies' counts.
#[derive(Clone, Debug)]
pub(crate) struct Milestones {
    catalogue: Vec<Milestone>,
    /// Each reached Milestone and when, in seconds since the Unix epoch.
    reached: HashMap<String, u64>,
    /// Each Tally's count (ticket 04); absent is 0.
    counts: HashMap<String, u64>,
    /// What the state file named wrong, for the realm's log.
    warnings: Vec<String>,
    /// Changed since the state file was last handed to the host.
    dirty: bool,
}

impl Default for Milestones {
    /// The shipped catalogue with nothing reached.
    fn default() -> Self {
        Self::new(shipped().to_vec())
    }
}

impl Milestones {
    /// `catalogue` with nothing reached: there is no state file. A row naming a quest or a Hub NPC
    /// the content lacks panics.
    pub(crate) fn new(catalogue: Vec<Milestone>) -> Self {
        check_content(&catalogue);
        Self {
            catalogue,
            reached: HashMap::new(),
            counts: HashMap::new(),
            warnings: Vec::new(),
            dirty: false,
        }
    }

    /// The state file's `text` read against `catalogue`. A file that does not parse reaches
    /// nothing; a table naming no Milestone is skipped, and a `reached` that is no RFC 3339
    /// date-time leaves its Milestone unreached. Each is a warning, and so is a reached Milestone
    /// whose `after` is not, which stays reached.
    pub(crate) fn parse(catalogue: Vec<Milestone>, text: &str) -> Self {
        let mut milestones = Self::new(catalogue);
        let table: toml::Table = match text.parse() {
            Ok(t) => t,
            Err(e) => {
                milestones.warnings.push(format!(
                    "milestones.toml does not parse, so nothing is reached: {e}"
                ));
                return milestones;
            }
        };
        for (key, value) in table {
            if !milestones.catalogue.iter().any(|m| m.key == key) {
                milestones.warnings.push(format!(
                    "milestones.toml names {key}, which is no Milestone"
                ));
                continue;
            }
            let Some(row) = value.as_table() else {
                milestones.warnings.push(format!(
                    "milestones.toml's {key} is not a table, so it is not reached"
                ));
                continue;
            };
            if let Some(v) = row.get("reached") {
                match unix_secs(v) {
                    Some(s) => {
                        milestones.reached.insert(key.clone(), s);
                    }
                    None => milestones.warnings.push(format!(
                        "milestones.toml's {key} has reached = {v}, no RFC 3339 date-time, so it \
                         is not reached"
                    )),
                }
            }
            if let Some(v) = row.get("count") {
                match v.as_integer().and_then(|n| u64::try_from(n).ok()) {
                    Some(n) => {
                        milestones.counts.insert(key.clone(), n);
                    }
                    None => milestones.warnings.push(format!(
                        "milestones.toml's {key} has count = {v}, no whole number, so it counts 0"
                    )),
                }
            }
        }
        for m in &milestones.catalogue {
            let Some(after) = &m.after else { continue };
            if milestones.reached(&m.key) && !milestones.reached(after) {
                milestones.warnings.push(format!(
                    "milestones.toml has {} reached but not {after}, which it comes after",
                    m.key
                ));
            }
        }
        milestones
    }

    /// Whether Milestone `key` is reached. A key the catalogue does not list is not.
    pub(crate) fn reached(&self, key: &str) -> bool {
        self.reached.contains_key(key)
    }

    /// Every reached Milestone's key, in catalogue order.
    pub(crate) fn reached_keys(&self) -> Vec<String> {
        self.catalogue
            .iter()
            .filter(|m| self.reached(&m.key))
            .map(|m| m.key.clone())
            .collect()
    }

    /// Every reached Milestone's key, borrowed, in no order: what [`crate::closed_at`] takes.
    pub(crate) fn reached_refs(&self) -> Vec<&str> {
        self.reached.keys().map(String::as_str).collect()
    }

    /// The catalogue row of `key`.
    pub(crate) fn get(&self, key: &str) -> Option<&Milestone> {
        self.catalogue.iter().find(|m| m.key == key)
    }

    /// Records `key` reached at `now_unix_s`, and marks the state for writing. The caller checks
    /// that the catalogue lists it and that it is not reached yet.
    pub(crate) fn reach(&mut self, key: &str, now_unix_s: u64) {
        self.reached.insert(key.to_string(), now_unix_s);
        self.dirty = true;
    }

    /// Tally `key`'s count (0 when it has none) and goal; `None` for a Milestone that is no Tally.
    pub(crate) fn tally(&self, key: &str) -> Option<(u64, u64)> {
        let goal = self.get(key)?.goal?;
        Some((self.counts.get(key).copied().unwrap_or(0), goal))
    }

    /// A hand-in of `quest`: adds its points to each Tally it feeds whose `after` is reached and
    /// which is not, up to the goal, and marks the state for writing. The keys whose count it
    /// brought to their goal, which the caller reaches.
    pub(crate) fn feed(&mut self, quest: u32) -> Vec<String> {
        let mut full = Vec::new();
        for m in &self.catalogue {
            let (Some(goal), Some(f)) = (m.goal, m.feeds.iter().find(|f| f.quest == quest)) else {
                continue;
            };
            let open = m
                .after
                .as_ref()
                .is_none_or(|a| self.reached.contains_key(a));
            if !open || self.reached.contains_key(&m.key) {
                continue;
            }
            let count = self.counts.entry(m.key.clone()).or_insert(0);
            if *count >= goal {
                continue;
            }
            *count = (*count + f.points).min(goal);
            self.dirty = true;
            if *count >= goal {
                full.push(m.key.clone());
            }
        }
        full
    }

    /// The Tallies whose count is at their goal, whose `after` is reached and which are not: what
    /// a state file may hold that the realm reaches when it reads it.
    pub(crate) fn full(&self) -> Vec<String> {
        self.catalogue
            .iter()
            .filter(|m| {
                let at_goal = m
                    .goal
                    .is_some_and(|g| self.counts.get(&m.key).is_some_and(|&n| n >= g));
                at_goal && !self.reached(&m.key) && m.after.as_ref().is_none_or(|a| self.reached(a))
            })
            .map(|m| m.key.clone())
            .collect()
    }

    /// The whole state file, when it changed since the last call.
    pub(crate) fn take_changed(&mut self) -> Option<String> {
        std::mem::take(&mut self.dirty).then(|| self.render())
    }

    /// The state file: a header, then a table per Milestone with a count or reached, in
    /// catalogue order.
    pub(crate) fn render(&self) -> String {
        let mut out = String::from(
            "# Milestones (ADR 0034). The realm writes this file: edit it only while the realm is\n\
             # stopped, and delete a Milestone's table to reset it.\n",
        );
        for m in &self.catalogue {
            let (reached, count) = (self.reached.get(&m.key), self.counts.get(&m.key));
            if reached.is_none() && count.is_none() {
                continue;
            }
            out.push_str(&format!("\n[{}]\n", m.key));
            if let Some(&at) = reached {
                out.push_str(&format!("reached = \"{}\"\n", rfc3339(at)));
            }
            if let Some(n) = count {
                out.push_str(&format!("count = {n}\n"));
            }
        }
        out
    }

    /// The catalogue's rows.
    pub(crate) fn catalogue(&self) -> &[Milestone] {
        &self.catalogue
    }

    /// What the state file named wrong.
    pub(crate) fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

/// `unix_s` as an RFC 3339 date-time in UTC, to the second.
fn rfc3339(unix_s: u64) -> String {
    let (days, secs) = (unix_s / DAY_SECS, unix_s % DAY_SECS);
    let (y, m, d) = civil_from_days(days as i64);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}

/// The date `days` after 1970-01-01, by Howard Hinnant's civil-from-days: the inverse of
/// [`days_from_civil`].
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    debug_assert_eq!(days_from_civil(y, m, d), days);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_catalogue_loads_with_both_rows() {
        let keys: Vec<_> = shipped().iter().map(|m| m.key.as_str()).collect();
        assert_eq!(keys, ["ilgalar_anchor_broken", "steppes_open"]);
        assert_eq!(shipped()[1].after.as_deref(), Some("ilgalar_anchor_broken"));
    }

    #[test]
    #[should_panic(expected = "steppes_open is listed twice")]
    fn a_repeated_key_panics() {
        catalogue(
            "[[milestone]]\nkey = \"steppes_open\"\nname = \"a\"\n\n\
             [[milestone]]\nkey = \"steppes_open\"\nname = \"b\"\n",
        );
    }

    #[test]
    #[should_panic(expected = "steppes_open comes after nope")]
    fn an_after_naming_no_row_panics() {
        catalogue("[[milestone]]\nkey = \"steppes_open\"\nname = \"a\"\nafter = \"nope\"\n");
    }

    #[test]
    #[should_panic(expected = "parses")]
    fn an_unknown_field_panics() {
        catalogue("[[milestone]]\nkey = \"steppes_open\"\nname = \"a\"\ngaol = 540\n");
    }

    #[test]
    #[should_panic(expected = "steppes_open has feeds and no goal")]
    fn feeds_without_a_goal_panic() {
        catalogue(
            "[[milestone]]\nkey = \"steppes_open\"\nname = \"a\"\n\
             feeds = [{ quest = 1, points = 15 }]\n",
        );
    }

    #[test]
    #[should_panic(expected = "steppes_open feeds quest 1 twice")]
    fn a_quest_fed_twice_panics() {
        catalogue(
            "[[milestone]]\nkey = \"steppes_open\"\nname = \"a\"\ngoal = 30\n\
             feeds = [{ quest = 1, points = 15 }, { quest = 1, points = 120 }]\n",
        );
    }

    #[test]
    #[should_panic(expected = "steppes_open has a goal below 1")]
    fn a_goal_of_nothing_panics() {
        catalogue("[[milestone]]\nkey = \"steppes_open\"\nname = \"a\"\ngoal = 0\n");
    }

    #[test]
    fn the_shipped_tally_opens_the_steppes_at_540() {
        assert_eq!(shipped_goal("steppes_open"), Some(540));
        assert_eq!(shipped_goal("ilgalar_anchor_broken"), None);
        assert!(shipped()[1].announce.is_some());
    }

    #[test]
    fn a_feed_counts_only_after_its_after_and_stops_at_the_goal() {
        let rows = catalogue(
            "[[milestone]]\nkey = \"a\"\nname = \"a\"\n\n\
             [[milestone]]\nkey = \"b\"\nname = \"b\"\nafter = \"a\"\ngoal = 30\n\
             feeds = [{ quest = 7, points = 20 }]\n",
        );
        let mut state = Milestones {
            catalogue: rows,
            ..Milestones::new(Vec::new())
        };
        assert!(state.feed(7).is_empty());
        assert_eq!(state.tally("b"), Some((0, 30)));
        assert!(state.take_changed().is_none());
        state.reach("a", 0);
        assert!(state.feed(7).is_empty());
        assert_eq!(state.tally("b"), Some((20, 30)));
        assert_eq!(state.feed(7), ["b"]);
        assert_eq!(state.tally("b"), Some((30, 30)));
        assert_eq!(state.full(), ["b"]);
        state.reach("b", 0);
        let _ = state.take_changed();
        assert!(state.feed(7).is_empty());
        assert!(state.take_changed().is_none());
    }

    #[test]
    fn a_gate_waits_on_its_keys() {
        let gate = |text: &str| -> Gate { toml::from_str(text).unwrap() };
        let after = gate("after_milestone = \"ilgalar_anchor_broken\"");
        let until = gate("until_milestone = \"ilgalar_anchor_broken\"");
        let both =
            gate("after_milestone = \"ilgalar_anchor_broken\"\nuntil_milestone = \"steppes_open\"");
        let none: [&str; 0] = [];
        let anchor = ["ilgalar_anchor_broken"];
        let all = ["ilgalar_anchor_broken", "steppes_open"];
        assert!(Gate::default().met(&none) && Gate::default().met(&all));
        assert!(!after.met(&none) && after.met(&anchor));
        assert!(until.met(&none) && !until.met(&anchor) && until.gone(&anchor));
        assert!(!both.met(&none) && both.met(&anchor) && !both.met(&all));
        both.check("test row");
    }

    #[test]
    #[should_panic(expected = "test row names Milestone nope")]
    fn a_gate_naming_no_milestone_panics() {
        let gate: Gate = toml::from_str("until_milestone = \"nope\"").unwrap();
        gate.check("test row");
    }

    #[test]
    fn a_date_renders_and_reads_back() {
        // 2026-11-20T21:14:03Z.
        let at = 1_795_209_243;
        assert_eq!(rfc3339(at), "2026-11-20T21:14:03Z");
        assert_eq!(unix_secs(&toml::Value::String(rfc3339(at))), Some(at));
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn counts_and_dates_round_trip_in_catalogue_order() {
        let text = "[steppes_open]\ncount = 315\n\n[ilgalar_anchor_broken]\n\
                    reached = \"2026-11-20T21:14:03Z\"\n";
        let state = Milestones::parse(shipped().to_vec(), text);
        assert!(state.warnings().is_empty(), "{:?}", state.warnings());
        let out = state.render();
        let anchor = out.find("[ilgalar_anchor_broken]").unwrap();
        let steppes = out.find("[steppes_open]").unwrap();
        assert!(anchor < steppes, "{out}");
        assert!(out.contains("count = 315\n"), "{out}");
        let again = Milestones::parse(shipped().to_vec(), &out);
        assert_eq!(again.render(), out);
    }
}
