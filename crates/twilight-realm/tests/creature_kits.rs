//! Creature kits (#470): a `[[kit]]` lays spells, a say table and an idle emote over a creature's
//! template. A creature turning from idle to a fight may say one of its aggro lines to the
//! characters within 25 yd, and an idle one plays its emote now and then to those within 40 yd.

mod common;

use std::collections::HashMap;

use benilla_protocol::messages::{ChatMessage, CHAT_MSG_MONSTER_SAY};
use benilla_protocol::{EntityKind, SessionEvent};
use common::*;
use twilight_realm::SessionId;

/// The Vile Familiars' camp of five by the Crossroads (`content/barrens.toml`, 9000015).
const FAMILIAR: u32 = 3101;
const FAMILIAR_HOME: [f32; 3] = [-731.0, -2518.0, 0.0];
/// The one creature of its entry in a camp of six north of the Crossroads (9000020): the others
/// carry no kit, so they never speak.
const TALKER: u32 = 2954;
const TALKER_HOME: [f32; 3] = [-302.0, -2387.0, 0.0];
const LINES: [&str; 2] = ["First line.", "Second line."];
/// `Emotes.dbc` 1, talk.
const TALK: u32 = 1;

/// Every test lays the same kits, which last for the process, so the order they run in does not
/// matter.
const KITS: &str = r#"
[[kit]]
entry = 3101
spells = ["corruption_creature"]
say = { aggro = ["Never said."], aggro_chance = 0 }

[[kit]]
entry = 2954
say = { aggro = ["First line.", "Second line."], aggro_chance = 100 }
emote_ooc = { emote = 1, every_ms = [2000, 3000] }
"#;

#[test]
fn a_kits_spells_replace_the_templates() {
    let (mut realm, s) = players(1, 10);
    let (familiar, at) = find(&mut realm, s[0], FAMILIAR, FAMILIAR_HOME);
    // Close enough that it notices and fights.
    go(&mut realm, s[0], [at[0] + 2.0, at[1], 0.0]);
    let got = run(&mut realm, 10.0);
    let corruption = creature_spell("corruption_creature");
    assert!(
        got[&s[0]].iter().any(|e| matches!(e,
            SessionEvent::SpellStart { caster, spell_id, .. }
                if *caster == familiar && *spell_id == corruption)),
        "the familiar casts the kit's spell"
    );
}

#[test]
#[should_panic(expected = "[[kit]] for entry 4242424 names no template")]
fn a_kit_for_an_unknown_entry_fails_content_loading() {
    let _ = Realm::with_seed(1).with_kits("[[kit]]\nentry = 4242424\nflee_at_pct = 15\n");
}

#[test]
fn an_engage_says_one_line_to_a_character_10_yd_away() {
    let (got, puller, watcher) = pull_watched(10.0);
    let heard = lines(&got[&watcher]);
    assert_eq!(heard.len(), 1, "{heard:?}");
    let line = &heard[0];
    assert_eq!(line.sender_name.as_deref(), Some("Bristleback Battleboar"));
    assert!(LINES.contains(&line.text.as_str()), "{}", line.text);
    assert_eq!(lines(&got[&puller]).len(), 1, "and to the puller beside it");
}

#[test]
fn an_engage_says_nothing_to_a_character_40_yd_away() {
    let (got, puller, watcher) = pull_watched(40.0);
    assert_eq!(lines(&got[&puller]).len(), 1, "the puller beside it hears");
    assert!(lines(&got[&watcher]).is_empty());
}

#[test]
fn an_aggro_chance_of_0_says_nothing() {
    let (mut realm, s) = players(1, 20);
    let (familiar, at) = find(&mut realm, s[0], FAMILIAR, FAMILIAR_HOME);
    let got = pull(&mut realm, s[0], familiar, at);
    assert!(engaged(&got, familiar), "the familiar turned on the puller");
    let mut events = got;
    events.extend(run(&mut realm, 2.0).remove(&s[0]).unwrap_or_default());
    let said: Vec<&ChatMessage> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Chat(c) if c.chat_type == CHAT_MSG_MONSTER_SAY => Some(c),
            _ => None,
        })
        .collect();
    assert!(said.is_empty(), "{said:?}");
}

#[test]
fn an_idle_creature_emotes_within_its_range_and_not_while_it_fights() {
    let (mut realm, s) = players(1, 20);
    let (talker, at) = find(&mut realm, s[0], TALKER, TALKER_HOME);
    // Its timer armed while `find` ran, 0.5 s; the range's upper bound is 3 s.
    let idle = run(&mut realm, 3.2).remove(&s[0]).unwrap_or_default();
    assert!(
        idle.iter().any(|e| matches!(e,
            SessionEvent::Emote { guid, emote_id } if *guid == talker && *emote_id == TALK)),
        "the idle talker talks"
    );
    let mut fight = pull(&mut realm, s[0], talker, at);
    assert!(engaged(&fight, talker), "the talker turned on the puller");
    fight.extend(run(&mut realm, 4.0).remove(&s[0]).unwrap_or_default());
    let died = fight.iter().any(|e| matches!(e,
        SessionEvent::ObjectValues { guid, fields } if *guid == talker && fields.unit_health() == Some(0)));
    assert!(!died, "it lives, and fights, the whole while");
    assert!(
        !fight
            .iter()
            .any(|e| matches!(e, SessionEvent::Emote { guid, .. } if *guid == talker)),
        "no emote in a fight"
    );
}

// ── Helpers ───────────────────────────────────────────────────────────────────────────────────

/// A puller swings at the talker while a second character stands `yards` south of it: what each
/// session got from the swing to a second after the talker turned, and their sessions.
fn pull_watched(yards: f32) -> (HashMap<SessionId, Vec<SessionEvent>>, SessionId, SessionId) {
    let (mut realm, s) = players(2, 20);
    let (talker, at) = find(&mut realm, s[0], TALKER, TALKER_HOME);
    go(&mut realm, s[1], [at[0], at[1] - yards, 0.0]);
    run(&mut realm, 0.1);
    go(&mut realm, s[0], [at[0] + 2.0, at[1], 0.0]);
    realm.handle_as(s[0], RealmCommand::AttackSwing { guid: talker });
    let mut got: HashMap<SessionId, Vec<SessionEvent>> = HashMap::new();
    for _ in 0..50 {
        for (s, e) in run(&mut realm, 0.1) {
            got.entry(s).or_default().extend(e);
        }
        if got.get(&s[0]).is_some_and(|e| engaged(e, talker)) {
            break;
        }
    }
    assert!(
        engaged(&got[&s[0]], talker),
        "the talker turned on the puller"
    );
    realm.handle_as(s[0], RealmCommand::AttackStop);
    for (s, e) in run(&mut realm, 1.0) {
        got.entry(s).or_default().extend(e);
    }
    (got, s[0], s[1])
}

/// The creature lines in `events`.
fn lines(events: &[SessionEvent]) -> Vec<ChatMessage> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Chat(c) if c.chat_type == CHAT_MSG_MONSTER_SAY => Some(c.clone()),
            _ => None,
        })
        .collect()
}

/// Whether `mob` turned on someone in `events`.
fn engaged(events: &[SessionEvent], mob: u64) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::AttackStart { attacker, .. } if *attacker == mob))
}

/// `n` warriors of `level`, each in the world in a session of their own, in a realm with
/// [`KITS`].
fn players(n: usize, level: u8) -> (Realm, Vec<SessionId>) {
    let mut maker = realm();
    for name in &["Varian", "Anduin"][..n] {
        assert_eq!(create(&mut maker, name, HUMAN, 0), CHAR_CREATE_SUCCESS);
    }
    let mut realm = Realm::with_seed(7).with_kits(KITS);
    let mut sessions = Vec::new();
    for c in roster(&mut maker) {
        let blob = maker
            .export_character(c.guid)
            .unwrap()
            .replace("level = 1\n", &format!("level = {level}\n"));
        realm.import_character(&blob).unwrap();
        let s = realm.connect();
        realm.handle_as(s, RealmCommand::EnterWorld { guid: c.guid });
        sessions.push(s);
    }
    run(&mut realm, 0.1);
    (realm, sessions)
}

/// Brings session `s`'s character 30 yd south of `home`, out of reach of its camp, and returns
/// the creature of `entry` nearest it: its guid and where it stands.
fn find(realm: &mut Realm, s: SessionId, entry: u32, home: [f32; 3]) -> (u64, [f32; 3]) {
    go(realm, s, [home[0], home[1] - 30.0, 0.0]);
    let got = run(realm, 0.5).remove(&s).unwrap_or_default();
    got.iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } if fields.object_entry() == Some(entry) => Some((*guid, *position)),
            _ => None,
        })
        .min_by(|(_, a), (_, b)| {
            let d = |p: &[f32; 3]| (p[0] - home[0]).powi(2) + (p[1] - home[1]).powi(2);
            d(a).total_cmp(&d(b))
        })
        .unwrap_or_else(|| panic!("creature {entry} is in view"))
}

/// Session `s`'s character swings at `mob`, which stands at `at`, from 2 yd until it turns, then
/// stops swinging: what the session saw meanwhile.
fn pull(realm: &mut Realm, s: SessionId, mob: u64, at: [f32; 3]) -> Vec<SessionEvent> {
    go(realm, s, [at[0] + 2.0, at[1], 0.0]);
    realm.handle_as(s, RealmCommand::AttackSwing { guid: mob });
    let mut events = Vec::new();
    for _ in 0..50 {
        events.extend(run(realm, 0.1).remove(&s).unwrap_or_default());
        if engaged(&events, mob) {
            break;
        }
    }
    realm.handle_as(s, RealmCommand::AttackStop);
    events
}

/// Moves session `s`'s character to `to`, facing west.
fn go(realm: &mut Realm, s: SessionId, to: [f32; 3]) {
    realm.handle_as(
        s,
        RealmCommand::Move {
            motion: Default::default(),
            position: to,
            orientation: std::f32::consts::PI,
        },
    );
}

/// Ticks `secs` in 0.1 s steps on flat ground: what each session got.
fn run(realm: &mut Realm, secs: f32) -> HashMap<SessionId, Vec<SessionEvent>> {
    let mut got: HashMap<SessionId, Vec<SessionEvent>> = HashMap::new();
    for _ in 0..(secs / 0.1).round() as u32 {
        realm.tick(0.1, &FlatGround);
        for (s, e) in realm.drain_session_events() {
            got.entry(s).or_default().push(e);
        }
    }
    got
}
