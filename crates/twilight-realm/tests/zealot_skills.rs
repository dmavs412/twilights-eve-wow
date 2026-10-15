//! The Zealot's borrowed Paladin skills, through the realm's seam: Hammer of Justice's stun, and
//! what the Seals and Judgement cost before their effects arrive.

mod common;

use benilla_protocol::SessionEvent;
use common::*;

const HAMMER_OF_JUSTICE: u32 = 10308;
const SEAL_OF_COMMAND: u32 = 20375;

#[test]
fn hammer_of_justice_stuns_for_six_seconds_and_the_mob_does_not_swing() {
    let (mut realm, me, mob, mut view) = tanking(20);
    stand_at(&mut realm, &mob, 8.0);
    let rage = view.rage(me);
    realm.handle(cast(HAMMER_OF_JUSTICE, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, HAMMER_OF_JUSTICE), None, "in 10 yd reach");
    assert_eq!(view.rage(me), rage - 15, "15 rage");
    let hits = go_hits(&events, HAMMER_OF_JUSTICE).expect("it went off");
    assert!(hits.contains(&mob.guid), "{hits:?}");
    let stun = view
        .aura(mob.guid, HAMMER_OF_JUSTICE)
        .expect("the stun shows");
    assert!(stun.slot >= 32, "a debuff");

    stand_at(&mut realm, &mob, 2.0);
    let stunned = view.run(&mut realm, 5.8);
    assert!(!swings(&stunned, mob.guid), "a stunned mob does not swing");
    assert!(view.aura(mob.guid, HAMMER_OF_JUSTICE).is_some());
    view.run(&mut realm, 0.4);
    assert!(
        view.aura(mob.guid, HAMMER_OF_JUSTICE).is_none(),
        "six seconds"
    );
    let after = view.run(&mut realm, 3.0);
    assert!(swings(&after, mob.guid), "and swings again after");

    realm.handle(cast(HAMMER_OF_JUSTICE, Some(mob.guid)));
    assert!(
        refusal(&view.drain(&mut realm), HAMMER_OF_JUSTICE).is_some(),
        "a 60 s cooldown"
    );
}

#[test]
fn a_seal_costs_ten_rage_in_battle_stance() {
    let (mut realm, me, _, mut view) = tanking(12);
    let rage = view.rage(me);
    realm.handle(cast(SEAL_OF_COMMAND, None));
    assert_eq!(refusal(&view.drain(&mut realm), SEAL_OF_COMMAND), None);
    assert_eq!(view.rage(me), rage - 10);
}

// ── Helpers ──────────────────────────────────────────────────────────────────────────────────

const SEED: u64 = 0x5EED_0023;
const CROSSROADS_XY: [f32; 3] = [-441.8, -2648.6, 0.0];
/// A camp of four level 3 mobs south of the Crossroads (`content/barrens.toml`, 9000011).
const CAMP: [f32; 3] = [-376.0, -2837.0, 0.0];

#[derive(Clone)]
struct Mob {
    guid: u64,
    position: [f32; 3],
}

/// What the client knows: every object's fields, merged as the packets arrive.
struct View {
    objects: std::collections::HashMap<u64, benilla_protocol::ObjectFields>,
}

impl View {
    fn see(&mut self, events: &[SessionEvent]) {
        for e in events {
            match e {
                SessionEvent::ObjectCreate { guid, fields, .. } => {
                    self.objects.insert(*guid, fields.clone());
                }
                SessionEvent::ObjectValues { guid, fields } => {
                    self.objects.entry(*guid).or_default().merge(fields.clone());
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

    fn run(&mut self, realm: &mut Realm, secs: f32) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        for _ in 0..(secs / 0.1).round() as u32 {
            realm.tick(0.1, &FlatGround);
            events.extend(self.drain(realm));
        }
        events
    }

    fn fields(&self, guid: u64) -> &benilla_protocol::ObjectFields {
        self.objects
            .get(&guid)
            .expect("an object the client has seen")
    }

    fn rage(&self, guid: u64) -> u32 {
        self.fields(guid).unit_power(1).unwrap_or(0) / 10
    }

    fn health(&self, guid: u64) -> u32 {
        self.fields(guid).unit_health().unwrap()
    }

    fn aura(&self, guid: u64, spell_id: u32) -> Option<benilla_protocol::messages::UnitAuraSlot> {
        self.fields(guid)
            .unit_auras()
            .find(|a| a.spell_id == spell_id)
    }
}

/// A level 1 human Zealot in the world at the Crossroads, and the view.
fn zealot_at(seed: u64) -> (Realm, u64, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("calling = \"basic_warrior\"", "calling = \"zealot\"");
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View {
        objects: Default::default(),
    };
    view.drain(&mut realm);
    // The world streams in over the first ticks.
    view.run(&mut realm, 1.0);
    (realm, guid, view)
}

/// A level 1 Zealot that pulled the camp south of the Crossroads without swinging back, and took
/// its blows until it had `rage`.
fn tanking(rage: u32) -> (Realm, u64, Mob, View) {
    let (mut realm, me, mut view) = zealot_at(SEED);
    // Out of reach of the four neutral scorpids, then each of them in turn.
    realm.handle(RealmCommand::Move {
        position: [CAMP[0] + 40.0, CAMP[1], 0.0],
        orientation: std::f32::consts::PI,
    });
    let streamed = view.run(&mut realm, 1.0);
    let camp: Vec<Mob> = streamed
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: benilla_protocol::EntityKind::Unit,
                position,
                ..
            } if flat(*position, CAMP) < 15.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect();
    let mob = camp.first().cloned().expect("the scorpid camp streams in");
    stand_at(&mut realm, &mob, 2.0);
    for other in camp.iter().rev() {
        realm.handle(RealmCommand::AttackSwing { guid: other.guid });
    }
    realm.handle(RealmCommand::AttackStop);
    for _ in 0..1800 {
        view.run(&mut realm, 0.1);
        assert!(view.health(me) > 0, "died waiting for {rage} rage");
        if view.rage(me) >= rage {
            return (realm, me, mob, view);
        }
    }
    panic!("two minutes of blows built no {rage} rage");
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
    realm.handle(RealmCommand::Move {
        position: at,
        orientation: (mob.position[1] - at[1]).atan2(mob.position[0] - at[0]),
    });
}

fn flat(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// Whether `mob` swung in `events`.
fn swings(events: &[SessionEvent], mob: u64) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == mob))
}

fn go_hits(events: &[SessionEvent], spell_id: u32) -> Option<Vec<u64>> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellGo {
            spell_id: s, hits, ..
        } if *s == spell_id => Some(hits.clone()),
        _ => None,
    })
}

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
