//! The Tier 1 Warrior's skills that the Basic Warrior has no like of, played through the realm's
//! seam: Mortal Strike's wound, Execute on a mob near death, and Cleave's second target.

mod common;

use benilla_protocol::SessionEvent;
use common::*;

const MORTAL_STRIKE: u32 = 12294;
const EXECUTE: u32 = 5308;
const CLEAVE: u32 = 845;

/// `SPELL_FAILED_TARGET_AURASTATE`: the target is not low enough for Execute.
const TARGET_AURASTATE: u8 = 0x67;
/// `UNIT_FIELD_AURASTATE`'s bit for a unit at or below 20% health (`AURA_STATE_HEALTHLESS_20_PERCENT`),
/// which the client's Execute button needs on the target.
const HEALTHLESS_20: u32 = 0x2;

#[test]
fn mortal_strike_strikes_and_wounds_the_target() {
    let (mut realm, _, mob, mut view) = tanking(30);
    realm.handle(cast(MORTAL_STRIKE, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, MORTAL_STRIKE), None);
    let landed = go_hits(&events, MORTAL_STRIKE).expect("it went off");
    if landed.is_empty() {
        return; // A miss or a dodge wounds nothing.
    }
    assert_eq!(spell_damage(&events, MORTAL_STRIKE).len(), 1);
    let wound = view.aura(mob.guid, MORTAL_STRIKE).expect("the wound");
    assert!(wound.slot >= 32, "a debuff");
    realm.handle(cast(MORTAL_STRIKE, Some(mob.guid)));
    let refused = refusal(&view.drain(&mut realm), MORTAL_STRIKE);
    assert!(refused.is_some(), "a 6 s cooldown, or no rage left");
}

#[test]
fn execute_needs_the_target_at_a_fifth_of_its_health_and_spends_all_the_rage() {
    let (mut realm, _, mob, mut view) = tanking(15);
    assert_eq!(view.fields(mob.guid).unit_aura_state() & HEALTHLESS_20, 0);
    realm.handle(cast(EXECUTE, Some(mob.guid)));
    assert_eq!(
        refusal(&view.drain(&mut realm), EXECUTE),
        Some(TARGET_AURASTATE)
    );

    // Beaten down to a fifth: the mob's aura state says so, and Execute goes.
    let Some((mut realm, me, mob, mut view)) = (0..100).find_map(low_mob_fight) else {
        panic!("no seeded fight left the mob low with rage to spend");
    };
    assert_ne!(view.fields(mob.guid).unit_aura_state() & HEALTHLESS_20, 0);
    let rage = view.rage(me);
    assert!(rage >= 15);
    realm.handle(cast(EXECUTE, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, EXECUTE), None);
    let hits = go_hits(&events, EXECUTE).expect("it went off");
    if hits.contains(&mob.guid) {
        assert_eq!(view.rage(me), 0, "all {rage} rage spent");
        assert_eq!(spell_damage(&events, EXECUTE).len(), 1);
    } else {
        assert_eq!(view.rage(me), rage - 15, "a miss costs only the cost");
    }
}

#[test]
fn cleave_strikes_a_second_enemy_beside_the_target() {
    // A camp pulled: its mobs gather round the warrior, a stride from the one it fights.
    let (mut realm, me, a, mut view) = tanking(20);
    realm.handle(cast(CLEAVE, Some(a.guid)));
    realm.handle(RealmCommand::AttackSwing { guid: a.guid });
    let mut events = Vec::new();
    for _ in 0..40 {
        events.extend(view.run(&mut realm, 0.1));
        if go_targets(&events, CLEAVE).is_some() {
            break;
        }
    }
    let targets = go_targets(&events, CLEAVE).expect("Cleave went with the swing");
    assert_eq!(targets[0], a.guid);
    assert_eq!(
        targets.len(),
        2,
        "the target and one beside it: {targets:?}"
    );
    assert_ne!(targets[1], me);
}

/// Who `spell_id`'s `SMSG_SPELL_GO` names, hit or missed, the cast's target first.
fn go_targets(events: &[SessionEvent], spell_id: u32) -> Option<Vec<u64>> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellGo {
            spell_id: s,
            hits,
            misses,
            target,
            ..
        } if *s == spell_id => {
            let mut all: Vec<u64> = hits
                .iter()
                .copied()
                .chain(misses.iter().map(|m| m.0))
                .collect();
            all.sort_by_key(|g| Some(*g) != *target);
            Some(all)
        }
        _ => None,
    })
}

// ── Helpers ──────────────────────────────────────────────────────────────────────────────────

const SEED: u64 = 0x5EED_0015;
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

    fn max_health(&self, guid: u64) -> u32 {
        self.fields(guid).unit_max_health().unwrap()
    }

    fn aura(&self, guid: u64, spell_id: u32) -> Option<benilla_protocol::messages::UnitAuraSlot> {
        self.fields(guid)
            .unit_auras()
            .find(|a| a.spell_id == spell_id)
    }
}

/// A human Tier 1 Warrior of `level` in the world at the Crossroads, the hostile mobs in view (the
/// nearest first), and the view.
fn tier1_at(level: u8, seed: u64) -> (Realm, u64, Vec<Mob>, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"warrior\"");
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View {
        objects: Default::default(),
    };
    let mut events = view.drain(&mut realm);
    // The world streams in over the first ticks.
    events.extend(view.run(&mut realm, 1.0));
    let mut mobs: Vec<Mob> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: benilla_protocol::EntityKind::Unit,
                position,
                fields,
                ..
            } if fields.unit_npc_flags() == 0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect();
    mobs.sort_by(|a, b| {
        flat(a.position, CROSSROADS_XY).total_cmp(&flat(b.position, CROSSROADS_XY))
    });
    (realm, guid, mobs, view)
}

/// A level 1 Tier 1 Warrior that pulled the nearest camp without swinging back, and took its
/// blows until it had `rage`.
fn tanking(rage: u32) -> (Realm, u64, Mob, View) {
    let (mut realm, me, _, mut view) = tier1_at(1, SEED);
    // Out of reach of the four level 3 scorpids south of the Crossroads, then each of them in
    // turn: neutral, they come only when attacked themselves.
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

/// A level 5 Tier 1 Warrior that beat the nearest mob down to a fifth of its health with 15 rage
/// left over; `None` when this seed's fight does not get there.
fn low_mob_fight(seed: u64) -> Option<(Realm, u64, Mob, View)> {
    let (mut realm, me, mobs, mut view) = tier1_at(5, seed);
    let mob = mobs[0].clone();
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    for _ in 0..600 {
        view.run(&mut realm, 0.1);
        let (hp, max) = (view.health(mob.guid), view.max_health(mob.guid));
        if view.health(me) == 0 || hp == 0 {
            return None;
        }
        if hp * 5 <= max {
            realm.handle(RealmCommand::AttackStop);
            view.drain(&mut realm);
            return (view.rage(me) >= 15).then_some((realm, me, mob, view));
        }
    }
    None
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

fn go_hits(events: &[SessionEvent], spell_id: u32) -> Option<Vec<u64>> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellGo {
            spell_id: s, hits, ..
        } if *s == spell_id => Some(hits.clone()),
        _ => None,
    })
}

fn spell_damage(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id => Some(l.damage),
            _ => None,
        })
        .collect()
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
