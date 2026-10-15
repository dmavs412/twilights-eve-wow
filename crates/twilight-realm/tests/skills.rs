//! The Basic Warrior's eight skills, played through the realm's seam: the spellbook at login,
//! casts from the action bar, what they cost, what they refuse, and what they do to a mob.

mod common;

use benilla_protocol::SessionEvent;
use common::*;

const HEROIC_STRIKE: u32 = 78;
const CHARGE: u32 = 100;
const REND: u32 = 772;
const THUNDER_CLAP: u32 = 6343;
const BATTLE_SHOUT: u32 = 6673;
const HAMSTRING: u32 = 1715;
const OVERPOWER: u32 = 7384;
const BLOODRAGE: u32 = 2687;

const EIGHT: [u32; 8] = [
    HEROIC_STRIKE,
    CHARGE,
    REND,
    THUNDER_CLAP,
    BATTLE_SHOUT,
    HAMSTRING,
    OVERPOWER,
    BLOODRAGE,
];

#[test]
fn a_new_basic_warriors_spellbook_holds_exactly_the_eight_skills() {
    let mut realm = realm();
    let (_, events) = enter_new(&mut realm, "Garrosh", ORC);

    let books: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellBook {
                spell_ids,
                cooldowns,
            } => Some((spell_ids.clone(), cooldowns.len())),
            _ => None,
        })
        .collect();
    assert_eq!(books.len(), 1, "one spellbook at login");
    let (mut ids, cooldowns) = books[0].clone();
    ids.sort_unstable();
    let mut want = EIGHT.to_vec();
    want.sort_unstable();
    assert_eq!(ids, want);
    assert_eq!(cooldowns, 0, "a fresh character has nothing cooling down");
}

#[test]
fn a_warrior_stands_in_battle_stance_so_the_client_allows_its_stance_skills() {
    let mut realm = realm();
    let (guid, events) = enter_new(&mut realm, "Garrosh", ORC);
    assert_eq!(own_create(&events, guid).unit_shapeshift_form(), BATTLE_STANCE);
}

#[test]
fn a_new_character_finds_the_eight_on_its_battle_stance_bar() {
    let mut realm = realm();
    let (_, events) = enter_new(&mut realm, "Garrosh", ORC);
    let buttons = action_buttons(&events);
    // Battle Stance pages the main bar to slots 73-84 (0-based 72-83).
    let on_bar: Vec<(u8, u32)> = buttons.iter().map(|b| (b.slot, b.action)).collect();
    let want: Vec<(u8, u32)> = (72..).zip(EIGHT).collect();
    assert_eq!(on_bar, want);
    assert!(buttons.iter().all(|b| b.kind == 0), "spell buttons");
}

#[test]
fn the_bar_the_player_arranges_comes_back_at_the_next_login() {
    let mut realm = realm();
    let (guid, _) = enter_new(&mut realm, "Garrosh", ORC);
    // Charge moved to button 1 of the stance bar; Heroic Strike's old button cleared.
    realm.handle(RealmCommand::SetActionButton {
        slot: 72,
        packed: CHARGE,
    });
    realm.handle(RealmCommand::SetActionButton {
        slot: 73,
        packed: 0,
    });
    realm.handle(RealmCommand::Logout);
    realm.handle(RealmCommand::EnterWorld { guid });
    let events = drain(&mut realm);
    let on_bar: Vec<(u8, u32)> = action_buttons(&events)
        .iter()
        .map(|b| (b.slot, b.action))
        .collect();
    assert_eq!(&on_bar[..2], &[(72, CHARGE), (74, REND)]);

    // And it survives the save.
    let blob = realm.export_character(guid).unwrap();
    let mut other = Realm::new();
    other.import_character(&blob).unwrap();
    other.handle(RealmCommand::EnterWorld { guid });
    let events = drain(&mut other);
    assert_eq!(action_buttons(&events)[0].action, CHARGE);
}

#[test]
fn bloodrage_trades_a_fifth_of_base_health_for_rage_now_and_over_ten_seconds() {
    let (mut realm, me, _, mut view) = warrior_at(1);
    realm.handle(cast(BLOODRAGE, None));
    let events = view.drain(&mut realm);

    // The instant's packets, in a real server's order.
    let kinds: Vec<&str> = events.iter().filter_map(cast_packet).collect();
    assert_eq!(kinds, ["start", "result ok", "go"]);
    // 9.6 + 0.25 rage at level 1.
    assert_eq!(energized(&events, BLOODRAGE), vec![10]);
    assert_eq!(view.rage(me), 100);
    // A level-1 warrior's base health is 20 of its 60: 20% is 4.
    assert_eq!(view.health(me), 56);
    let aura = view.aura(me, 29131).expect("the rage-over-time aura");
    assert!(aura.is_cancelable(), "a buff");
    assert!(aura.slot < 32, "in a buff slot");
    assert!(
        events.iter().any(|e| matches!(e, SessionEvent::AuraDuration { slot, remaining_ms: 10_000 } if *slot == aura.slot)),
        "the buff bar's timer"
    );

    // One rage a second for ten seconds; Bloodrage holds the warrior in combat, so none decays.
    let events = view.run(&mut realm, 10.5);
    assert_eq!(periodic_energize(&events, 29131), vec![1; 10]);
    assert_eq!(view.rage(me), 200);
    assert_eq!(view.aura(me, 29131), None, "faded");
}

#[test]
fn bloodrage_cannot_be_cast_again_for_a_minute() {
    let (mut realm, _, _, mut view) = warrior_at(1);
    realm.handle(cast(BLOODRAGE, None));
    view.drain(&mut realm);
    view.run(&mut realm, 59.0);
    realm.handle(cast(BLOODRAGE, None));
    assert_eq!(refusal(&view.drain(&mut realm), BLOODRAGE), Some(NOT_READY));
    view.run(&mut realm, 1.1);
    realm.handle(cast(BLOODRAGE, None));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, BLOODRAGE), None);
    assert!(events.iter().any(|e| matches!(e, SessionEvent::SpellGo { spell_id: BLOODRAGE, .. })));
}

// ── Helpers ──────────────────────────────────────────────────────────────────────────────────

/// `UNIT_FIELD_BYTES_1` byte 2's Battle Stance.
const BATTLE_STANCE: u8 = 17;

fn own_create(events: &[SessionEvent], guid: u64) -> benilla_protocol::ObjectFields {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate { guid: g, fields, .. } if *g == guid => {
                Some(fields.clone())
            }
            _ => None,
        })
        .expect("the player's own create")
}

fn action_buttons(events: &[SessionEvent]) -> Vec<benilla_protocol::messages::ActionButton> {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ActionButtons { buttons } => Some(buttons.clone()),
            _ => None,
        })
        .expect("the action bar at login")
}

// `SpellCastResult` (vmangos `SpellDefines.h`), as the client's red error line reads them.
const AFFECTING_COMBAT: u8 = 0x00;
const BAD_TARGETS: u8 = 0x0a;
const CASTER_AURASTATE: u8 = 0x12;
const NOT_READY: u8 = 0x3c;
const NO_POWER: u8 = 0x4d;
const OUT_OF_RANGE: u8 = 0x59;
const TOO_CLOSE: u8 = 0x76;
const UNIT_NOT_INFRONT: u8 = 0x7c;

const SEED: u64 = 0x5EED_0008;
const CROSSROADS_XY: [f32; 3] = [-441.8, -2648.6, 0.0];

/// A mob, as its create shows it.
#[derive(Clone)]
struct Mob {
    guid: u64,
    position: [f32; 3],
}

/// What the client knows: every object's fields, merged as the packets arrive.
#[derive(Default)]
struct View(std::collections::HashMap<u64, benilla_protocol::ObjectFields>);

impl View {
    fn see(&mut self, events: &[SessionEvent]) {
        for e in events {
            match e {
                SessionEvent::ObjectCreate { guid, fields, .. } => {
                    self.0.insert(*guid, fields.clone());
                }
                SessionEvent::ObjectValues { guid, fields } => {
                    self.0.entry(*guid).or_default().merge(fields.clone());
                }
                _ => {}
            }
        }
    }

    fn drain(&mut self, realm: &mut Realm) -> Vec<SessionEvent> {
        let events = drain(realm);
        self.see(&events);
        events
    }

    /// Ticks `secs` in 0.1 s steps, collecting the events.
    fn run(&mut self, realm: &mut Realm, secs: f32) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        let steps = (secs / 0.1).round() as u32;
        for _ in 0..steps {
            realm.tick(0.1, &FlatGround);
            events.extend(self.drain(realm));
        }
        events
    }

    fn fields(&self, guid: u64) -> &benilla_protocol::ObjectFields {
        self.0.get(&guid).expect("an object the client has seen")
    }

    /// Whole rage points.
    fn rage(&self, guid: u64) -> u32 {
        self.fields(guid).unit_power(1).unwrap_or(0) / 10 * 10
    }

    fn health(&self, guid: u64) -> u32 {
        self.fields(guid).unit_health().unwrap()
    }

    fn aura(&self, guid: u64, spell_id: u32) -> Option<benilla_protocol::messages::UnitAuraSlot> {
        self.fields(guid).unit_auras().find(|a| a.spell_id == spell_id)
    }
}

/// A human warrior of `level` in the world (on flat ground) at the Crossroads, the mob near it,
/// and what the client has seen so far.
fn warrior_at(level: u8) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"));
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let events = view.drain(&mut realm);
    let mob = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: benilla_protocol::EntityKind::Unit,
                position,
                ..
            } => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("a mob near the Crossroads");
    (realm, guid, mob, view)
}

fn cast(spell_id: u32, target: Option<u64>) -> RealmCommand {
    RealmCommand::CastSpell { spell_id, target }
}

/// Moves the player to `yards` from `mob`, on the Crossroads side, facing it.
fn stand_at(realm: &mut Realm, mob: &Mob, yards: f32) {
    let (dx, dy) = (
        CROSSROADS_XY[0] - mob.position[0],
        CROSSROADS_XY[1] - mob.position[1],
    );
    let len = (dx * dx + dy * dy).sqrt();
    let at = [
        mob.position[0] + dx / len * yards,
        mob.position[1] + dy / len * yards,
        mob.position[2],
    ];
    let facing = (mob.position[1] - at[1]).atan2(mob.position[0] - at[0]);
    realm.handle(RealmCommand::Move {
        position: at,
        orientation: facing,
    });
}

/// The cast's own packets, by kind.
fn cast_packet(e: &SessionEvent) -> Option<&'static str> {
    match e {
        SessionEvent::SpellStart { .. } => Some("start"),
        SessionEvent::CastResult { success: true, .. } => Some("result ok"),
        SessionEvent::CastResult { success: false, .. } => Some("result failed"),
        SessionEvent::SpellGo { .. } => Some("go"),
        _ => None,
    }
}

/// The reason `spell_id` was refused, if it was.
fn refusal(events: &[SessionEvent], spell_id: u32) -> Option<u8> {
    events.iter().find_map(|e| match e {
        SessionEvent::CastResult {
            spell_id: s,
            success: false,
            reason,
            ..
        } if *s == spell_id => *reason,
        _ => None,
    })
}

/// `SMSG_SPELLENERGIZELOG` amounts for `spell_id`, in rage points.
fn energized(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellEnergizeLog(l) if l.spell_id == spell_id => {
                assert_eq!(l.power, 1, "rage");
                Some(l.amount)
            }
            _ => None,
        })
        .collect()
}

/// `SMSG_PERIODICAURALOG` energize ticks for `spell_id`.
fn periodic_energize(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == spell_id => Some(&l.ticks),
            _ => None,
        })
        .flatten()
        .filter_map(|t| match t {
            benilla_protocol::messages::PeriodicTick::Energize { amount, .. } => Some(*amount),
            _ => None,
        })
        .collect()
}
