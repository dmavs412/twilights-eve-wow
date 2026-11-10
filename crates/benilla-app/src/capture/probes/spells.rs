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
//! Some errors are the situation's, not the spell's, and earn a retry: no resource yet (wait; for
//! rage, with the swing that builds it), out of range, too close or not behind (move, through
//! the dev goto), a friendly-only spell at the dummy (cast on self), and a missing precondition
//! such as combo points or an aura state (its setup spell first, else once more at the end).
//!
//! `WOW_PROBE_SPELLS_FIRST=<spell id>,...` names spells to cast first, in that order, before the
//! probe's melee swing starts combat: Stealth, the stealth openers, the Seals a Judgement needs
//! (#523). `WOW_PROBE_SPELLS_SETUP=<spell id>:<setup id>,...` names the spell that makes each
//! one's precondition (a combo point builder for a finisher, the aura a copier copies); it is
//! cast, unreported, when the spell is refused for want of it. `WOW_PROBE_SPELLS_SKIP=<spell
//! id>=<reason>;...` names reactive spells whose event the dummy never makes (a block): one
//! still refused for its aura state at the end is reported as skipped, with the reason, not
//! failed. `scripts/smoke.ps1` builds all three from `server_spells.json`.

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
/// In melee behind the dummy, which faces west (`test_grounds.toml`), facing it.
const BEHIND: (f32, f32) = (-9638.0, 264.0);
/// The realm's aura-state refusal (`SPELL_FAILED_CASTER_AURASTATE`, `_TARGET_AURASTATE`): the
/// spell's precondition is missing, as `GlobalStrings.lua` words it.
const PRECONDITION_ERROR: &str = "can't do that yet";

/// Which way the player faces at a move's spot: east at the dummy from in front, west at it from
/// behind.
fn facing_at(spot: (f32, f32)) -> f32 {
    if spot == BEHIND {
        std::f32::consts::PI
    } else {
        0.0
    }
}

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
        let first_value = std::env::var("WOW_PROBE_SPELLS_FIRST").unwrap_or_default();
        let first = first_ids(&first_value);
        if first.is_empty() && !first_value.trim().is_empty() {
            warn!("probe-spells: WOW_PROBE_SPELLS_FIRST={first_value:?} is not a list of spell ids — ignored");
        }
        let setup_value = std::env::var("WOW_PROBE_SPELLS_SETUP").unwrap_or_default();
        let setup = setup_pairs(&setup_value);
        if setup.is_empty() && !setup_value.trim().is_empty() {
            warn!("probe-spells: WOW_PROBE_SPELLS_SETUP={setup_value:?} is not a list of <id>:<id> — ignored");
        }
        let skip_value = std::env::var("WOW_PROBE_SPELLS_SKIP").unwrap_or_default();
        let skip = skip_reasons(&skip_value);
        if skip.is_empty() && !skip_value.trim().is_empty() {
            warn!("probe-spells: WOW_PROBE_SPELLS_SKIP={skip_value:?} is not a list of <id>=<reason> — ignored");
        }
        app.insert_resource(ProbeSpells {
            path: path.into(),
            calling,
            first,
            setup,
            skip,
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
    /// Cast only to set up the next spell's precondition: logged, never reported.
    setup: bool,
}

/// `WOW_PROBE_SPELLS_SETUP`'s pairs, spell id to the id of the spell that sets it up. A value
/// that is not a comma list of `<id>:<id>` gives none.
fn setup_pairs(value: &str) -> std::collections::HashMap<u32, u32> {
    if value.trim().is_empty() {
        return Default::default();
    }
    value
        .split(',')
        .map(|pair| {
            let (spell, setup) = pair.split_once(':').ok_or(())?;
            let id = |s: &str| s.trim().parse::<u32>().map_err(|_| ());
            Ok((id(spell)?, id(setup)?))
        })
        .collect::<Result<_, ()>>()
        .unwrap_or_default()
}

/// `WOW_PROBE_SPELLS_SKIP`'s reasons by spell id. A value that is not a `;` list of
/// `<id>=<reason>` with a reason gives none.
fn skip_reasons(value: &str) -> std::collections::HashMap<u32, String> {
    if value.trim().is_empty() {
        return Default::default();
    }
    value
        .split(';')
        .map(|pair| {
            let (spell, reason) = pair.split_once('=').ok_or(())?;
            let spell = spell.trim().parse::<u32>().map_err(|_| ())?;
            let reason = reason.trim();
            if reason.is_empty() {
                return Err(());
            }
            Ok((spell, reason.to_string()))
        })
        .collect::<Result<_, ()>>()
        .unwrap_or_default()
}

/// `WOW_PROBE_SPELLS_FIRST`'s spell ids, in order. A value that is not a comma list of ids gives
/// none.
fn first_ids(value: &str) -> Vec<u32> {
    if value.trim().is_empty() {
        return Vec::new();
    }
    value
        .split(',')
        .map(|id| id.trim().parse::<u32>())
        .collect::<Result<_, _>>()
        .unwrap_or_default()
}

/// The cast queue: the spells `first` lists, in its order, then the rest of `book` in the book's
/// order. A listed spell the book lacks is skipped.
fn cast_order(mut book: Vec<Entry>, first: &[u32]) -> std::collections::VecDeque<Entry> {
    let mut queue = std::collections::VecDeque::with_capacity(book.len());
    for id in first {
        if let Some(i) = book.iter().position(|e| e.spell_id == *id) {
            queue.push_back(book.remove(i));
        }
    }
    queue.extend(book);
    queue
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
    MoveBehind,
    OnSelf,
    Later,
}

/// The error text's retry, if it has one. The texts are the stock `GlobalStrings.lua` ones.
fn retry_for(error: &str) -> Option<Retry> {
    let e = error.to_ascii_lowercase();
    if e.starts_with("not enough")
        || e.contains("not ready yet")
        || e.contains("not yet recovered")
        || e.contains("another action is in progress")
    {
        // A resource, a cooldown or a repeating shot in the way: all pass with time.
        Some(Retry::Resource)
    } else if e.contains("too close") {
        Some(Retry::MoveFar)
    } else if e.contains("out of range") || e.contains("too far") {
        Some(Retry::MoveClose)
    } else if e.contains("behind") {
        Some(Retry::MoveBehind)
    } else if e.contains("invalid target")
        || e.contains("target is friendly")
        || e.contains("can only use") && e.contains("friendly")
    {
        Some(Retry::OnSelf)
    } else if e.contains("requires") || e.contains("must be") || e.contains(PRECONDITION_ERROR) {
        Some(Retry::Later)
    } else {
        None
    }
}

/// What the probe does after an attempt that did not go through.
#[derive(Debug, Clone, PartialEq)]
enum Next {
    /// Wait and try again; `swing` starts the melee swing first, since rage comes from it.
    Wait { swing: bool },
    /// Go to the spot and try again.
    Move((f32, f32)),
    /// Try again on the player.
    OnSelf,
    /// Cast this setup spell, unreported, then the spell again.
    SetUp(u32),
    /// Put the spell at the back of the queue.
    Defer,
    /// Write its line: a failure, or a skip with its reason.
    Report { skipped: Option<String> },
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
/// `skipped` is the reason a reactive spell was not judged.
fn report_line(
    calling: &str,
    e: &Entry,
    passive: bool,
    o: &Outcome,
    skipped: Option<&str>,
) -> String {
    format!(
        "{{\"calling\":{},\"spell_id\":{},\"name\":{},\"rank\":{},\"passive\":{},\"ok\":{},\"error\":{},\"damage\":{},\"aura\":{},\"skipped\":{}}}",
        json_str(calling),
        e.spell_id,
        json_str(&e.name),
        json_str(&e.rank),
        passive,
        o.ok,
        o.error.as_deref().map_or("null".into(), json_str),
        o.damage,
        o.aura,
        skipped.map_or("null".into(), json_str),
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
    /// The spells cast first, before the swing starts combat (`WOW_PROBE_SPELLS_FIRST`).
    first: Vec<u32>,
    /// The spell that sets up each spell's precondition (`WOW_PROBE_SPELLS_SETUP`).
    setup: std::collections::HashMap<u32, u32>,
    /// Why each reactive spell may go unjudged (`WOW_PROBE_SPELLS_SKIP`).
    skip: std::collections::HashMap<u32, String>,
    entered_at: Option<f32>,
    /// The castable spellbook, for the setup casts.
    book: Vec<Entry>,
    queue: std::collections::VecDeque<Entry>,
    /// Spells already put back once for a "requires" error.
    deferred: std::collections::HashSet<u32>,
    /// Spells whose setup has been cast once.
    set_up: std::collections::HashSet<u32>,
    /// Opening spells that ran short of rage: they get the swing after all.
    swing: std::collections::HashSet<u32>,
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

    /// What follows an attempt at `spell` (a setup cast when `setup`) that ended in `error`, on
    /// its `tries`-th attempt; `on_self` when that attempt already went at the player.
    fn next_step(&self, spell: u32, setup: bool, error: &str, tries: u32, on_self: bool) -> Next {
        let report = Next::Report { skipped: None };
        match retry_for(error) {
            Some(Retry::Resource) if tries < RESOURCE_TRIES => Next::Wait {
                swing: error.to_ascii_lowercase().contains("rage"),
            },
            Some(retry @ (Retry::MoveClose | Retry::MoveFar | Retry::MoveBehind))
                if !self.moved.contains(&spell) =>
            {
                Next::Move(match retry {
                    Retry::MoveFar => FAR,
                    Retry::MoveBehind => BEHIND,
                    _ => CLOSE,
                })
            }
            Some(Retry::OnSelf) if !on_self => Next::OnSelf,
            // A setup cast that cannot go is dropped; the spell it was for tries anyway.
            Some(Retry::Later) if setup => report,
            Some(Retry::Later) => match self.setup.get(&spell) {
                Some(&by) if !self.set_up.contains(&spell) => Next::SetUp(by),
                _ if !self.deferred.contains(&spell) => Next::Defer,
                _ => Next::Report {
                    skipped: error
                        .to_ascii_lowercase()
                        .contains(PRECONDITION_ERROR)
                        .then(|| self.skip.get(&spell).cloned())
                        .flatten(),
                },
            },
            _ => report,
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
            let mut castable = Vec::new();
            for (i, slot) in book.slots.iter().enumerate() {
                let entry = Entry {
                    slot: i + 1,
                    spell_id: slot.spell_id,
                    name: slot.name.clone(),
                    rank: slot.rank.clone().unwrap_or_default(),
                    setup: false,
                };
                if slot.passive {
                    let ok = Outcome {
                        ok: true,
                        ..default()
                    };
                    probe.write(&report_line(&calling, &entry, true, &ok, None));
                } else {
                    castable.push(entry);
                }
            }
            // A setup the book lacks cannot be cast.
            probe
                .setup
                .retain(|_, by| castable.iter().any(|e| e.spell_id == *by));
            probe.book = castable.clone();
            probe.queue = cast_order(castable, &probe.first);
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
            // Melee swings build rage for the Warrior Callings; a no-op once swinging. The
            // opening spells go without: the swing puts the player in combat and breaks Stealth.
            // An opening spell short of rage (a Seal) gets it after all: no swing, no rage.
            if !probe.first.contains(&entry.spell_id) || probe.swing.contains(&entry.spell_id) {
                run_lua(&script, "AttackTarget()");
            }
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
            let next = match outcome.error.as_deref() {
                Some(error) if !outcome.ok => {
                    probe.next_step(entry.spell_id, entry.setup, error, tries, on_self)
                }
                _ => Next::Report { skipped: None },
            };
            match next {
                Next::Wait { swing } => {
                    if swing {
                        probe.swing.insert(entry.spell_id);
                    }
                    probe.queue.push_front(entry);
                    probe.phase = Phase::Next {
                        at: now + RESOURCE_WAIT_SECS,
                    };
                }
                Next::Move(spot) => {
                    goto.write(DevGoto {
                        x: spot.0,
                        y: spot.1,
                        facing: Some(facing_at(spot)),
                    });
                    probe.moved.insert(entry.spell_id);
                    probe.queue.push_front(entry);
                    probe.phase = Phase::Next {
                        at: now + MOVE_WAIT_SECS,
                    };
                }
                Next::OnSelf => {
                    script.take_tapped_events();
                    run_lua(&script, &format!("CastSpell({}, \"spell\", 1)", entry.slot));
                    probe.phase = Phase::Casting {
                        entry,
                        watch: Watch::new(now),
                        on_self: true,
                    };
                }
                Next::SetUp(by) => {
                    // The setup cast gets a fresh run of retries, then the spell goes again.
                    let setup = probe.book.iter().find(|e| e.spell_id == by).cloned();
                    probe.set_up.insert(entry.spell_id);
                    probe.queue.push_front(entry);
                    if let Some(setup) = setup {
                        probe.tries.remove(&by);
                        probe.moved.remove(&by);
                        probe.queue.push_front(Entry {
                            setup: true,
                            ..setup
                        });
                    }
                    probe.phase = Phase::Next { at: now + 0.5 };
                }
                Next::Defer => {
                    probe.deferred.insert(entry.spell_id);
                    probe.queue.push_back(entry);
                    probe.phase = Phase::Next { at: now + 0.5 };
                }
                Next::Report { .. } if entry.setup => {
                    info!(
                        "probe-spells: setup {} ({}): {}",
                        entry.name,
                        entry.spell_id,
                        outcome.error.as_deref().unwrap_or("went through")
                    );
                    probe.phase = Phase::Next { at: now + 0.2 };
                }
                Next::Report { skipped } => {
                    let outcome = if let Some(reason) = &skipped {
                        info!(
                            "probe-spells: {} ({}) skipped: {reason}",
                            entry.name, entry.spell_id
                        );
                        Outcome {
                            ok: true,
                            ..outcome
                        }
                    } else {
                        outcome
                    };
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
                    probe.write(&report_line(
                        &calling,
                        &entry,
                        false,
                        &outcome,
                        skipped.as_deref(),
                    ));
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
        // The realm's aura-state refusal (a Judgement with no Seal, a Kindle with no Ember on
        // the target) is a missing precondition, not a cooldown; a cooldown is "Not yet
        // recovered".
        assert_eq!(retry_for("You can't do that yet"), Some(Retry::Later));
        assert_eq!(retry_for("Not yet recovered"), Some(Retry::Resource));
        assert_eq!(
            retry_for("You must be behind your target"),
            Some(Retry::MoveBehind)
        );
    }

    #[test]
    fn the_setups_and_skips_parse_and_a_bad_value_gives_none() {
        assert_eq!(
            setup_pairs("1000773:1000774, 1000822:1000818"),
            [(1000773, 1000774), (1000822, 1000818)]
                .into_iter()
                .collect()
        );
        assert!(setup_pairs("").is_empty());
        assert!(setup_pairs("1000773").is_empty());
        assert!(setup_pairs("1000773:x").is_empty());
        assert_eq!(
            skip_reasons("1000625=usable only after a block; 1000731=needs Bloodlust stacks"),
            [
                (1000625, "usable only after a block".to_string()),
                (1000731, "needs Bloodlust stacks".to_string())
            ]
            .into_iter()
            .collect()
        );
        assert!(skip_reasons("").is_empty());
        assert!(skip_reasons("1000625").is_empty());
        assert!(skip_reasons("x=reason").is_empty());
        assert!(skip_reasons("1000625=").is_empty());
    }

    fn probe() -> ProbeSpells {
        ProbeSpells {
            setup: [(73, 74)].into_iter().collect(),
            skip: [(25, "usable only after a block".to_string())]
                .into_iter()
                .collect(),
            ..default()
        }
    }

    #[test]
    fn a_missing_precondition_casts_its_setup_once_then_defers_once_then_fails() {
        let mut p = probe();
        let combo = "That ability requires combo points";
        assert_eq!(p.next_step(73, false, combo, 1, false), Next::SetUp(74));
        p.set_up.insert(73);
        assert_eq!(p.next_step(73, false, combo, 2, false), Next::Defer);
        p.deferred.insert(73);
        assert_eq!(
            p.next_step(73, false, combo, 3, false),
            Next::Report { skipped: None }
        );
        // A setup's own failure is never set up or deferred: it is dropped.
        assert_eq!(
            p.next_step(74, true, "You must be behind your target", 1, false),
            Next::Move(BEHIND)
        );
        assert_eq!(
            p.next_step(74, true, combo, 1, false),
            Next::Report { skipped: None }
        );
    }

    #[test]
    fn a_reactive_spell_whose_event_never_came_is_skipped_with_its_reason() {
        let mut p = probe();
        let state = "You can't do that yet";
        assert_eq!(p.next_step(25, false, state, 1, false), Next::Defer);
        p.deferred.insert(25);
        assert_eq!(
            p.next_step(25, false, state, 2, false),
            Next::Report {
                skipped: Some("usable only after a block".into())
            }
        );
        // Any other error still fails it.
        assert_eq!(
            p.next_step(25, false, "SPELLCAST_FAILED", 3, false),
            Next::Report { skipped: None }
        );
    }

    #[test]
    fn a_rage_short_cast_waits_with_the_swing_and_others_wait_without() {
        let p = probe();
        assert_eq!(
            p.next_step(1, false, "Not enough rage", 1, false),
            Next::Wait { swing: true }
        );
        assert_eq!(
            p.next_step(1, false, "Not enough energy", 1, false),
            Next::Wait { swing: false }
        );
        assert_eq!(
            p.next_step(1, false, "Not enough rage", RESOURCE_TRIES, false),
            Next::Report { skipped: None }
        );
        assert_eq!(
            p.next_step(1, false, "Out of range.", 1, false),
            Next::Move(CLOSE)
        );
        assert_eq!(
            p.next_step(1, false, "Invalid target", 1, false),
            Next::OnSelf
        );
        assert_eq!(
            p.next_step(1, false, "Invalid target", 1, true),
            Next::Report { skipped: None }
        );
    }

    #[test]
    fn the_opening_order_parses_to_ids_and_a_bad_value_to_none() {
        assert_eq!(
            first_ids("1000195,1000404,1000413"),
            [1000195, 1000404, 1000413]
        );
        assert_eq!(first_ids(" 7 , 3 "), [7, 3]);
        assert!(first_ids("").is_empty());
        assert!(first_ids("  ").is_empty());
        assert!(first_ids("stealth").is_empty());
        assert!(first_ids("1000195,sap").is_empty());
        assert!(first_ids("1000195,,1000413").is_empty());
        assert!(first_ids("-5").is_empty());
    }

    #[test]
    fn the_opening_spells_lead_the_queue_and_the_rest_keep_the_book_order() {
        let entry = |slot: usize, spell_id: u32| Entry {
            slot,
            spell_id,
            name: format!("Spell {spell_id}"),
            rank: String::new(),
            setup: false,
        };
        let book: Vec<Entry> = [10, 20, 30, 40, 50]
            .iter()
            .enumerate()
            .map(|(i, &id)| entry(i + 1, id))
            .collect();
        let ids = |q: &std::collections::VecDeque<Entry>| -> Vec<u32> {
            q.iter().map(|e| e.spell_id).collect()
        };
        // A listed id the book lacks is skipped; one listed twice is cast once.
        let queue = cast_order(book.clone(), &[40, 20, 99, 40]);
        assert_eq!(ids(&queue), [40, 20, 10, 30, 50]);
        assert_eq!(queue[0].slot, 4, "an entry keeps its spellbook slot");
        assert_eq!(ids(&cast_order(book, &[])), [10, 20, 30, 40, 50]);
    }

    #[test]
    fn the_report_line_is_json_with_every_field() {
        let e = Entry {
            slot: 3,
            spell_id: 78,
            name: "Heroic \"Strike\"".into(),
            rank: "Rank 1".into(),
            setup: false,
        };
        let o = Outcome {
            ok: false,
            error: Some("Not enough rage".into()),
            damage: 0,
            aura: false,
        };
        assert_eq!(
            report_line("vanguard", &e, false, &o, None),
            r#"{"calling":"vanguard","spell_id":78,"name":"Heroic \"Strike\"","rank":"Rank 1","passive":false,"ok":false,"error":"Not enough rage","damage":0,"aura":false,"skipped":null}"#
        );
        // A skip is no failure: it goes through as ok, with its reason.
        let skip = Outcome {
            ok: true,
            error: Some("You can't do that yet".into()),
            ..o
        };
        assert_eq!(
            report_line(
                "ironclad",
                &e,
                false,
                &skip,
                Some("usable only after a block")
            ),
            r#"{"calling":"ironclad","spell_id":78,"name":"Heroic \"Strike\"","rank":"Rank 1","passive":false,"ok":true,"error":"You can't do that yet","damage":0,"aura":false,"skipped":"usable only after a block"}"#
        );
        assert_eq!(damage_in("Your Rend hits X for 9."), 9);
        assert_eq!(damage_in("Your Rend was dodged."), 0);
    }
}
