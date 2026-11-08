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
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalogue {
    milestone: Vec<Milestone>,
}

/// A catalogue's rows from its text (the shape of `content/milestones.toml`); a repeated key, or
/// an `after` that names no row, panics.
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
    }
    file.milestone
}

/// The shipped catalogue.
fn shipped() -> &'static [Milestone] {
    static FILE: OnceLock<Vec<Milestone>> = OnceLock::new();
    FILE.get_or_init(|| catalogue(include_str!("../content/milestones.toml")))
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
    /// `catalogue` with nothing reached: there is no state file.
    pub(crate) fn new(catalogue: Vec<Milestone>) -> Self {
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
