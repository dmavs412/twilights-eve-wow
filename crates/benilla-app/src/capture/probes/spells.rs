//! `WOW_PROBE_SPELLS=<report.jsonl>`: in Twilight's Eve, casts every spell in the spellbook on a
//! training dummy through the UI's own `CastSpell`, and writes one JSON line per spell: whether
//! the cast went through or the error the client showed, the damage its combat-log line names,
//! and whether an aura changed. Passive spells are listed, not cast. Then it exits, with an error
//! status if any cast failed. `scripts/smoke.ps1` drives it from the Test Grounds (#200).
//!
//! The verdict comes from the FrameXML event stream (the VM's event tap), the same events the
//! cast bar and the error frame show a player:
//! - an error (`UI_ERROR_MESSAGE`, `SPELLCAST_FAILED`, `SPELLCAST_INTERRUPTED`) fails the cast;
//! - otherwise the cast counts as through once [`QUIET_SECS`] pass with no cast bar running.
//!
//! Some errors are the situation's, not the spell's, and earn a retry: no resource yet (wait),
//! out of range or too close (move, through the dev goto), a friendly-only spell at the dummy
//! (cast on self), and a "requires" error such as combo points (once more, at the end).

use std::io::Write;

use benilla_ui::script::{ScriptValue, UiScript};
use bevy::prelude::*;

use super::ProbeClock;
use crate::twilight::DevGoto;

/// Seconds after world entry before the first cast: the Test Grounds teleport, the dummies'
/// creates and the UI's first draw land first.
const START_AFTER_SECS: f32 = 20.0;
/// A cast with no error and no cast bar after this long went through.
const QUIET_SECS: f32 = 2.0;
/// The longest any one attempt is watched.
const ATTEMPT_LIMIT_SECS: f32 = 15.0;
/// How long a retry for a resource (rage, energy, mana) waits, and how often.
const RESOURCE_WAIT_SECS: f32 = 3.0;
const RESOURCE_TRIES: u32 = 10;
/// How long after a move before the retry.
const MOVE_WAIT_SECS: f32 = 2.0;
/// The dummy the probe works on (the Test Grounds' level-5 one stands at (-9640, 264)), and the
/// spots in front of it: in melee, and out at range.
const DUMMY_NAME: &str = "Training Dummy";
/// The melee swing's own errors (`GlobalStrings.lua`), which never judge a cast.
const SWING_ERRORS: [&str; 3] = [
    "You are too far away!",
    "You are facing the wrong way!",
    "There is nothing to attack.",
];
const CLOSE: (f32, f32) = (-9642.0, 264.0);
const FAR: (f32, f32) = (-9655.0, 264.0);

pub(crate) struct ProbeSpellsPlugin;

impl Plugin for ProbeSpellsPlugin {
    fn build(&self, app: &mut App) {
        if !crate::twilight::active() {
            warn!("probe-spells: WOW_PROBE_SPELLS needs the twilights-eve launcher — ignored");
            return;
        }
        let path = std::env::var("WOW_PROBE_SPELLS").unwrap_or_default();
        if path.is_empty() {
            return;
        }
        let calling = std::env::var("WOW_PROBE_SPELLS_CALLING").unwrap_or_default();
        app.insert_resource(ProbeSpells {
            path: path.into(),
            calling,
            ..default()
        })
        .add_systems(Update, drive_probe_spells);
    }
}

/// One spellbook entry to try.
#[derive(Debug, Clone)]
struct Entry {
    slot: usize,
    spell_id: u32,
    name: String,
    rank: String,
}

/// What became of one spell, as its report line says.
#[derive(Debug, Clone, PartialEq, Default)]
struct Outcome {
    ok: bool,
    error: Option<String>,
    damage: u32,
    aura: bool,
}

/// Why an error earns another try.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Retry {
    Resource,
    MoveClose,
    MoveFar,
    OnSelf,
    Later,
}

/// The error text's retry, if it has one. The texts are the stock `GlobalStrings.lua` ones.
fn retry_for(error: &str) -> Option<Retry> {
    let e = error.to_ascii_lowercase();
    if e.starts_with("not enough")
        || e.contains("not ready yet")
        || e.contains("can't do that yet")
        || e.contains("another action is in progress")
    {
        // A resource, a cooldown or a repeating shot in the way: all pass with time.
        Some(Retry::Resource)
    } else if e.contains("too close") {
        Some(Retry::MoveFar)
    } else if e.contains("out of range") || e.contains("too far") {
        Some(Retry::MoveClose)
    } else if e.contains("invalid target")
        || e.contains("target is friendly")
        || e.contains("can only use") && e.contains("friendly")
    {
        Some(Retry::OnSelf)
    } else if e.contains("requires") || e.contains("must be") {
        Some(Retry::Later)
    } else {
        None
    }
}

/// Watches one attempt's events to a verdict.
#[derive(Debug, Default)]
struct Watch {
    started_at: f32,
    /// When the last event that bears on the cast arrived.
    last_at: f32,
    casting: bool,
    outcome: Outcome,
    done: bool,
}

impl Watch {
    fn new(now: f32) -> Self {
        Self {
            started_at: now,
            last_at: now,
            ..default()
        }
    }

    /// Folds in one tapped event. `name` is the spell's, to pick its combat-log lines.
    fn see(&mut self, event: &str, args: &[String], name: &str, now: f32) {
        if self.done {
            return;
        }
        match event {
            "SPELLCAST_START" | "SPELLCAST_CHANNEL_START" => {
                self.casting = true;
                self.last_at = now;
            }
            "SPELLCAST_STOP" | "SPELLCAST_CHANNEL_STOP" => {
                self.casting = false;
                self.last_at = now;
            }
            "UI_ERROR_MESSAGE" => {
                let text = args.first().cloned().unwrap_or_default();
                // The probe's own melee swing (`AttackTarget`) reports range and facing through
                // the same frame; those are the swing's, not the spell's.
                if !SWING_ERRORS.contains(&text.as_str()) {
                    self.fail(text);
                }
            }
            "SPELLCAST_FAILED" => self.fail("SPELLCAST_FAILED".into()),
            "SPELLCAST_INTERRUPTED" => self.fail("SPELLCAST_INTERRUPTED".into()),
            "CHAT_MSG_SPELL_SELF_DAMAGE" => {
                if let Some(line) = args.first() {
                    if line.contains(name) {
                        self.outcome.damage += damage_in(line);
                    }
                }
            }
            "UNIT_AURA" | "PLAYER_AURAS_CHANGED" => self.outcome.aura = true,
            _ => {}
        }
    }

    fn fail(&mut self, text: String) {
        self.outcome.ok = false;
        self.outcome.error = Some(text);
        self.done = true;
    }

    /// Settles the attempt once it has been quiet long enough, or watched too long.
    fn tick(&mut self, now: f32) {
        if self.done {
            return;
        }
        let quiet = !self.casting && now - self.last_at >= QUIET_SECS;
        if quiet {
            self.outcome.ok = true;
            self.done = true;
        } else if now - self.started_at >= ATTEMPT_LIMIT_SECS {
            self.fail("no result: the cast never finished".into());
        }
    }
}

/// The number after " for " in a combat-log line (`Your Heroic Strike hits X for 123.`).
fn damage_in(line: &str) -> u32 {
    line.split(" for ")
        .nth(1)
        .map(|rest| {
            rest.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
        })
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// The report's JSON line; hand-written, since the shipped app carries no serde_json.
fn report_line(calling: &str, e: &Entry, passive: bool, o: &Outcome) -> String {
    format!(
        "{{\"calling\":{},\"spell_id\":{},\"name\":{},\"rank\":{},\"passive\":{},\"ok\":{},\"error\":{},\"damage\":{},\"aura\":{}}}",
        json_str(calling),
        e.spell_id,
        json_str(&e.name),
        json_str(&e.rank),
        passive,
        o.ok,
        o.error.as_deref().map_or("null".into(), json_str),
        o.damage,
        o.aura,
    )
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Where the probe is in its run.
#[derive(Debug, Default)]
enum Phase {
    #[default]
    Waiting,
    /// Between attempts; the next one starts at `at`.
    Next {
        at: f32,
    },
    Casting {
        entry: Entry,
        watch: Watch,
        on_self: bool,
    },
    Done,
}

#[derive(Resource, Default)]
struct ProbeSpells {
    path: std::path::PathBuf,
    calling: String,
    entered_at: Option<f32>,
    queue: std::collections::VecDeque<Entry>,
    /// Spells already put back once for a "requires" error.
    deferred: std::collections::HashSet<u32>,
    /// Spells already moved for once, for a range error.
    moved: std::collections::HashSet<u32>,
    /// Attempts per spell, for the resource retries.
    tries: std::collections::HashMap<u32, u32>,
    phase: Phase,
    exited: bool,
    failed: u32,
    written: u32,
}

impl ProbeSpells {
    fn write(&mut self, line: &str) {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path);
        match file {
            Ok(mut f) => {
                let _ = writeln!(f, "{line}");
                self.written += 1;
            }
            Err(e) => error!("probe-spells: writing {}: {e}", self.path.display()),
        }
    }
}

fn run_lua(script: &UiScript, chunk: &str) {
    if let Err(e) = script.run(chunk) {
        warn!("probe-spells: lua {chunk:?}: {e}");
    }
}

fn arg_text(v: &ScriptValue) -> String {
    match v {
        ScriptValue::Str(s) => s.clone(),
        ScriptValue::Int(n) => n.to_string(),
        ScriptValue::Number(n) => n.to_string(),
        ScriptValue::Bool(b) => b.to_string(),
        ScriptValue::Nil => String::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn drive_probe_spells(
    mut probe: ResMut<ProbeSpells>,
    time: ProbeClock,
    script: Option<NonSendMut<UiScript>>,
    self_player: Query<(), With<crate::net::SelfPlayer>>,
    state: Res<State<crate::char_select::ClientState>>,
    entry_ui_pending: Option<Res<crate::ui_script::PendingEntryUiLoad>>,
    mut goto: MessageWriter<DevGoto>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut script) = script else { return };
    let in_world = *state.get() == crate::char_select::ClientState::InWorld;
    if !in_world || entry_ui_pending.is_some() || self_player.is_empty() {
        return;
    }
    let now = time.elapsed_secs();
    let entered = *probe.entered_at.get_or_insert(now);
    let probe = &mut *probe;
    // Taken out and put back by every arm, so the arms can reach the rest of `probe`.
    match std::mem::take(&mut probe.phase) {
        Phase::Waiting => {
            if now < entered + START_AFTER_SECS {
                return;
            }
            script.set_event_tap(true);
            let book = script.spellbook();
            let calling = probe.calling.clone();
            for (i, slot) in book.slots.iter().enumerate() {
                let entry = Entry {
                    slot: i + 1,
                    spell_id: slot.spell_id,
                    name: slot.name.clone(),
                    rank: slot.rank.clone().unwrap_or_default(),
                };
                if slot.passive {
                    let ok = Outcome {
                        ok: true,
                        ..default()
                    };
                    probe.write(&report_line(&calling, &entry, true, &ok));
                } else {
                    probe.queue.push_back(entry);
                }
            }
            info!(
                "probe-spells: {} spells to cast, {} passive",
                probe.queue.len(),
                probe.written
            );
            goto.write(DevGoto {
                x: CLOSE.0,
                y: CLOSE.1,
                facing: Some(0.0),
            });
            probe.phase = Phase::Next {
                at: now + MOVE_WAIT_SECS,
            };
        }
        Phase::Next { at } => {
            if now < at {
                probe.phase = Phase::Next { at };
                return;
            }
            let Some(entry) = probe.queue.pop_front() else {
                probe.phase = Phase::Done;
                return;
            };
            run_lua(&script, &format!("TargetByName({})", lua_str(DUMMY_NAME)));
            // Melee swings build rage for the Warrior Callings; a no-op once swinging.
            run_lua(&script, "AttackTarget()");
            script.take_tapped_events();
            run_lua(&script, &format!("CastSpell({}, \"spell\")", entry.slot));
            probe.phase = Phase::Casting {
                entry,
                watch: Watch::new(now),
                on_self: false,
            };
        }
        Phase::Casting {
            entry,
            mut watch,
            on_self,
        } => {
            for (event, args) in script.take_tapped_events() {
                let args: Vec<String> = args.iter().map(arg_text).collect();
                watch.see(&event, &args, &entry.name, now);
            }
            watch.tick(now);
            if !watch.done {
                probe.phase = Phase::Casting {
                    entry,
                    watch,
                    on_self,
                };
                return;
            }
            let outcome = watch.outcome.clone();
            let tries = probe.tries.entry(entry.spell_id).or_insert(0);
            *tries += 1;
            let tries = *tries;
            match outcome.error.as_deref().and_then(retry_for) {
                Some(Retry::Resource) if tries < RESOURCE_TRIES => {
                    probe.queue.push_front(entry);
                    probe.phase = Phase::Next {
                        at: now + RESOURCE_WAIT_SECS,
                    };
                }
                Some(retry @ (Retry::MoveClose | Retry::MoveFar))
                    if !probe.moved.contains(&entry.spell_id) =>
                {
                    let (x, y) = if retry == Retry::MoveFar { FAR } else { CLOSE };
                    goto.write(DevGoto {
                        x,
                        y,
                        facing: Some(0.0),
                    });
                    probe.moved.insert(entry.spell_id);
                    probe.queue.push_front(entry);
                    probe.phase = Phase::Next {
                        at: now + MOVE_WAIT_SECS,
                    };
                }
                Some(Retry::OnSelf) if !on_self => {
                    script.take_tapped_events();
                    run_lua(&script, &format!("CastSpell({}, \"spell\", 1)", entry.slot));
                    probe.phase = Phase::Casting {
                        entry,
                        watch: Watch::new(now),
                        on_self: true,
                    };
                }
                Some(Retry::Later) if probe.deferred.insert(entry.spell_id) => {
                    probe.queue.push_back(entry);
                    probe.phase = Phase::Next { at: now + 0.5 };
                }
                _ => {
                    if !outcome.ok {
                        probe.failed += 1;
                        warn!(
                            "probe-spells: {} ({}) failed: {}",
                            entry.name,
                            entry.spell_id,
                            outcome.error.as_deref().unwrap_or("?")
                        );
                    }
                    let calling = probe.calling.clone();
                    probe.write(&report_line(&calling, &entry, false, &outcome));
                    probe.phase = Phase::Next { at: now + 0.2 };
                }
            }
        }
        Phase::Done => {
            probe.phase = Phase::Done;
            if probe.exited {
                return;
            }
            probe.exited = true;
            info!(
                "probe-spells: done, {} lines, {} failed — {}",
                probe.written,
                probe.failed,
                probe.path.display()
            );
            script.set_event_tap(false);
            exit.write(if probe.failed == 0 {
                AppExit::Success
            } else {
                AppExit::error()
            });
        }
    }
}

fn lua_str(s: &str) -> String {
    format!("{s:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_quiet_cast_goes_through_and_names_its_damage() {
        let mut w = Watch::new(0.0);
        w.see(
            "CHAT_MSG_SPELL_SELF_DAMAGE",
            &args(&["Your Heroic Strike hits Training Dummy for 123."]),
            "Heroic Strike",
            0.5,
        );
        w.see(
            "CHAT_MSG_SPELL_SELF_DAMAGE",
            &args(&["Your Rend hits Training Dummy for 9."]),
            "Heroic Strike",
            0.6,
        );
        w.tick(1.0);
        assert!(!w.done, "settled before the quiet window");
        w.tick(2.1);
        assert!(w.done);
        assert_eq!(
            w.outcome,
            Outcome {
                ok: true,
                error: None,
                damage: 123,
                aura: false
            }
        );
    }

    #[test]
    fn a_cast_bar_holds_the_verdict_until_it_stops() {
        let mut w = Watch::new(0.0);
        w.see(
            "SPELLCAST_START",
            &args(&["Fireball", "3000"]),
            "Fireball",
            0.1,
        );
        w.tick(2.5);
        assert!(!w.done, "settled while the bar ran");
        w.see("SPELLCAST_STOP", &[], "Fireball", 3.1);
        w.see("UNIT_AURA", &args(&["target"]), "Fireball", 3.2);
        w.tick(5.2);
        assert!(w.done && w.outcome.ok && w.outcome.aura);
    }

    #[test]
    fn an_error_fails_the_cast_with_its_text() {
        let mut w = Watch::new(0.0);
        w.see("UI_ERROR_MESSAGE", &args(&["Not enough rage"]), "Rend", 0.1);
        assert!(w.done);
        assert_eq!(w.outcome.error.as_deref(), Some("Not enough rage"));
        assert!(!w.outcome.ok);
        let mut w = Watch::new(0.0);
        w.see(
            "UI_ERROR_MESSAGE",
            &args(&["You are too far away!"]),
            "Auto Shot",
            0.1,
        );
        assert!(!w.done, "the swing's error judged the cast");
        let mut w = Watch::new(0.0);
        w.see("SPELLCAST_START", &[], "Fireball", 0.1);
        w.tick(16.0);
        assert!(
            w.done && !w.outcome.ok,
            "a bar that never ends is a failure"
        );
    }

    #[test]
    fn errors_of_the_situation_earn_their_retry() {
        assert_eq!(retry_for("Not enough rage"), Some(Retry::Resource));
        assert_eq!(retry_for("Not enough energy"), Some(Retry::Resource));
        assert_eq!(retry_for("Spell is not ready yet."), Some(Retry::Resource));
        assert_eq!(
            retry_for("Another action is in progress"),
            Some(Retry::Resource)
        );
        assert_eq!(retry_for("Target too close"), Some(Retry::MoveFar));
        assert_eq!(retry_for("Out of range."), Some(Retry::MoveClose));
        assert_eq!(retry_for("Invalid target"), Some(Retry::OnSelf));
        assert_eq!(
            retry_for("That ability requires combo points"),
            Some(Retry::Later)
        );
        assert_eq!(retry_for("SPELLCAST_FAILED"), None);
    }

    #[test]
    fn the_report_line_is_json_with_every_field() {
        let e = Entry {
            slot: 3,
            spell_id: 78,
            name: "Heroic \"Strike\"".into(),
            rank: "Rank 1".into(),
        };
        let o = Outcome {
            ok: false,
            error: Some("Not enough rage".into()),
            damage: 0,
            aura: false,
        };
        assert_eq!(
            report_line("vanguard", &e, false, &o),
            r#"{"calling":"vanguard","spell_id":78,"name":"Heroic \"Strike\"","rank":"Rank 1","passive":false,"ok":false,"error":"Not enough rage","damage":0,"aura":false}"#
        );
        assert_eq!(damage_in("Your Rend hits X for 9."), 9);
        assert_eq!(damage_in("Your Rend was dodged."), 0);
    }
}
