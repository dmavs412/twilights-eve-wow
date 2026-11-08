//! Server Unlocks (ADR 0031): features a server buys, each on until a date. The catalogue
//! (`content/unlocks.toml`) lists them and which are free; the state file, `unlocks.toml` in the
//! realm's data folder, holds one table per bought unlock with its `until`.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;
use toml::value::{Datetime, Offset};

const DAY_SECS: u64 = 86_400;

/// One catalogue row.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Unlock {
    pub key: String,
    pub name: String,
    /// What it adds, for whatever sells it: no feature shows it yet.
    #[allow(dead_code)]
    pub blurb: String,
    /// On with nothing bought: the test build's state.
    pub free: bool,
}

#[derive(Deserialize)]
struct Catalogue {
    unlock: Vec<Unlock>,
}

/// A catalogue's rows from its text (the shape of `content/unlocks.toml`); a repeated key panics.
pub(crate) fn catalogue(text: &str) -> Vec<Unlock> {
    let file: Catalogue = toml::from_str(text).expect("content/unlocks.toml parses");
    for (i, u) in file.unlock.iter().enumerate() {
        assert!(
            file.unlock[..i].iter().all(|v| v.key != u.key),
            "unlock {} is listed twice",
            u.key
        );
    }
    file.unlock
}

/// The shipped catalogue.
fn shipped() -> &'static [Unlock] {
    static FILE: OnceLock<Vec<Unlock>> = OnceLock::new();
    FILE.get_or_init(|| catalogue(include_str!("../content/unlocks.toml")))
}

/// One bought unlock: a state file table.
#[derive(Clone, Debug)]
pub(crate) struct Bought {
    /// When it lapses, in seconds since the Unix epoch; `None` is for good.
    until: Option<u64>,
}

/// The catalogue and what the state file says was bought.
#[derive(Clone, Debug)]
pub(crate) struct Unlocks {
    catalogue: Vec<Unlock>,
    bought: HashMap<String, Bought>,
    /// What the state file named wrong, for the realm's log.
    warnings: Vec<String>,
}

impl Default for Unlocks {
    /// The shipped catalogue with nothing bought.
    fn default() -> Self {
        Self::new(shipped().to_vec())
    }
}

impl Unlocks {
    /// `catalogue` with nothing bought: there is no state file.
    pub(crate) fn new(catalogue: Vec<Unlock>) -> Self {
        Self {
            catalogue,
            bought: HashMap::new(),
            warnings: Vec::new(),
        }
    }

    /// The state file's `text` read against `catalogue`. A file that does not parse buys nothing;
    /// a table naming no unlock, or whose `until` is no RFC 3339 date-time, is skipped. Each is a
    /// warning.
    pub(crate) fn parse(catalogue: Vec<Unlock>, text: &str) -> Self {
        let mut unlocks = Self::new(catalogue);
        let table: toml::Table = match text.parse() {
            Ok(t) => t,
            Err(e) => {
                unlocks.warnings.push(format!(
                    "unlocks.toml does not parse, so nothing is bought: {e}"
                ));
                return unlocks;
            }
        };
        for (key, value) in table {
            if !unlocks.catalogue.iter().any(|u| u.key == key) {
                unlocks.warnings.push(format!(
                    "unlocks.toml names {key}, which is no Server Unlock"
                ));
                continue;
            }
            let Some(row) = value.as_table() else {
                unlocks.warnings.push(format!(
                    "unlocks.toml's {key} is not a table, so it is not bought"
                ));
                continue;
            };
            let until = match row.get("until") {
                None => None,
                Some(v) => match unix_secs(v) {
                    Some(s) => Some(s),
                    None => {
                        unlocks.warnings.push(format!(
                            "unlocks.toml's {key} has until = {v}, no RFC 3339 date-time, so it \
                             is not bought"
                        ));
                        continue;
                    }
                },
            };
            if let Some(v) = row.get("bought").filter(|v| unix_secs(v).is_none()) {
                unlocks.warnings.push(format!(
                    "unlocks.toml's {key} has bought = {v}, no RFC 3339 date-time"
                ));
            }
            unlocks.bought.insert(key, Bought { until });
        }
        unlocks
    }

    /// Whether unlock `key` is on at `now_unix_s`: free, or bought with no `until` or one after
    /// `now_unix_s`. A key the catalogue does not list is off.
    pub(crate) fn on(&self, key: &str, now_unix_s: u64) -> bool {
        let Some(unlock) = self.catalogue.iter().find(|u| u.key == key) else {
            return false;
        };
        unlock.free
            || self
                .bought
                .get(key)
                .is_some_and(|b| b.until.is_none_or(|until| until > now_unix_s))
    }

    /// The catalogue's rows.
    pub(crate) fn catalogue(&self) -> &[Unlock] {
        &self.catalogue
    }

    /// What the state file named wrong.
    pub(crate) fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

/// Each unlock's state at the last check, and the UTC day that check ran on.
#[derive(Default)]
pub(crate) struct Seen {
    day: Option<u64>,
    on: HashMap<String, bool>,
}

impl Seen {
    /// Whether `now_unix_s` falls on a UTC day the last check did not run on: midnight has passed.
    pub(crate) fn new_day(&self, now_unix_s: u64) -> bool {
        self.day != Some(now_unix_s / DAY_SECS)
    }

    /// Checks every unlock at `now_unix_s` and logs each whose state changed since the last check.
    /// An unlock's first check is not a change.
    pub(crate) fn check(&mut self, unlocks: &Unlocks, now_unix_s: u64, log: &mut Vec<String>) {
        self.day = Some(now_unix_s / DAY_SECS);
        for u in unlocks.catalogue() {
            let on = unlocks.on(&u.key, now_unix_s);
            if self
                .on
                .insert(u.key.clone(), on)
                .is_some_and(|was| was != on)
            {
                let state = if on { "on" } else { "off" };
                log.push(format!("Server Unlock {} is now {state}.", u.name));
            }
        }
    }
}

/// An RFC 3339 date-time (a string, or a bare TOML date-time), in seconds since the Unix epoch.
/// A date or time alone, a time with no offset, or a moment before 1970 is none.
pub(crate) fn unix_secs(value: &toml::Value) -> Option<u64> {
    let at = match value {
        toml::Value::String(s) => s.parse::<Datetime>().ok()?,
        toml::Value::Datetime(d) => d.clone(),
        _ => return None,
    };
    let (date, time, offset) = (at.date?, at.time?, at.offset?);
    let offset_mins = match offset {
        Offset::Z => 0,
        Offset::Custom { minutes } => i64::from(minutes),
    };
    let days = days_from_civil(
        i64::from(date.year),
        i64::from(date.month),
        i64::from(date.day),
    );
    let secs = days * DAY_SECS as i64
        + i64::from(time.hour) * 3600
        + i64::from(time.minute) * 60
        + i64::from(time.second)
        - offset_mins * 60;
    u64::try_from(secs).ok()
}

/// Days from 1970-01-01 to the date `y`-`m`-`d`, by Howard Hinnant's days-from-civil.
pub(crate) fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_from_civil_counts_from_1970() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(days_from_civil(2026, 1, 1), 20_454);
    }

    #[test]
    fn the_shipped_catalogue_is_free() {
        let unlocks = Unlocks::default();
        assert!(unlocks.catalogue().iter().all(|u| u.free));
        assert!(unlocks.warnings().is_empty());
    }

    #[test]
    fn a_bad_bought_date_warns_and_keeps_the_table() {
        let unlocks = Unlocks::parse(shipped().to_vec(), "[carts]\nbought = \"yesterday\"\n");
        assert_eq!(unlocks.warnings().len(), 1);
        assert!(unlocks.bought.contains_key("carts"));
    }
}
