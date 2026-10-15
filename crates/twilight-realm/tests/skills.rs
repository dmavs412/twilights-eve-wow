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
    assert_eq!(
        own_create(&events, guid).unit_shapeshift_form(),
        BATTLE_STANCE
    );
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
    assert_eq!(view.rage(me), 10);
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
    assert_eq!(view.rage(me), 20);
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
    assert!(events.iter().any(|e| matches!(
        e,
        SessionEvent::SpellGo {
            spell_id: BLOODRAGE,
            ..
        }
    )));
}

#[test]
fn battle_shout_needs_its_rage_and_raises_attack_power_for_two_minutes() {
    let (mut realm, me, _, mut view) = warrior_at(1);
    realm.handle(cast(BATTLE_SHOUT, None));
    assert_eq!(
        refusal(&view.drain(&mut realm), BATTLE_SHOUT),
        Some(NO_POWER)
    );

    realm.handle(cast(BLOODRAGE, None));
    let min_before = view.fields(me).unit_min_damage().unwrap();
    realm.handle(cast(BATTLE_SHOUT, None));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, BATTLE_SHOUT), None);
    assert_eq!(view.rage(me), 0, "10 rage spent");
    let aura = view.aura(me, BATTLE_SHOUT).expect("the shout's buff");
    assert!(aura.is_cancelable());
    assert_eq!(view.fields(me).unit_attack_power_mods(), (15, 0));
    // 15 attack power is 15 / 14 damage a second, over a 2 s unarmed swing.
    let gained = view.fields(me).unit_min_damage().unwrap() - min_before;
    assert!((gained - 15.0 / 14.0 * 2.0).abs() < 1e-3, "{gained}");

    view.run(&mut realm, 120.5);
    assert_eq!(view.aura(me, BATTLE_SHOUT), None, "faded");
    assert_eq!(view.fields(me).unit_attack_power_mods(), (0, 0));
}

#[test]
fn a_skill_grows_with_level_without_being_relearned() {
    let shout = |level| {
        let (mut realm, me, _, mut view) = warrior_at(level);
        realm.handle(cast(BLOODRAGE, None));
        realm.handle(cast(BATTLE_SHOUT, None));
        view.drain(&mut realm);
        view.fields(me).unit_attack_power_mods().0
    };
    assert_eq!(shout(1), 15);
    assert_eq!(shout(5), 17);
    assert_eq!(shout(10), 20);
}

#[test]
fn rend_bleeds_the_mob_three_times_over_nine_seconds() {
    let (mut realm, me, mob, mut view) = warrior_at(1);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(BLOODRAGE, None));
    let full = view.health(mob.guid);
    realm.handle(cast(REND, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, REND), None);
    assert!(
        events.iter().any(|e| matches!(e, SessionEvent::AttackStart { attacker, victim } if *attacker == mob.guid && *victim == me)),
        "the mob turns on the player"
    );
    let aura = view
        .aura(mob.guid, REND)
        .expect("the bleed on the mob's frame");
    assert!(aura.slot >= 32 && !aura.is_cancelable(), "a debuff");
    assert_eq!(aura.level, 1, "the caster's level");

    let events = view.run(&mut realm, 9.5);
    // 4 + 0.24 + 0.01 × 29 attack power.
    assert_eq!(periodic_damage(&events, REND), vec![5, 5, 5]);
    assert_eq!(view.health(mob.guid), full - 15);
    assert_eq!(view.aura(mob.guid, REND), None, "run out");
}

#[test]
fn a_global_cooldown_follows_rend_and_the_tank_skills() {
    let (mut realm, _, mob, mut view) = warrior_at(10);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(BLOODRAGE, None));
    view.run(&mut realm, 8.0);
    realm.handle(cast(REND, Some(mob.guid)));
    realm.handle(cast(HAMSTRING, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, REND), None);
    assert_eq!(refusal(&events, HAMSTRING), Some(NOT_READY));
    view.run(&mut realm, 1.5);
    realm.handle(cast(HAMSTRING, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), HAMSTRING), None);
}

#[test]
fn a_melee_skill_needs_a_target_in_reach_and_in_front() {
    let (mut realm, _, mob, mut view) = warrior_at(10);
    realm.handle(cast(BLOODRAGE, None));
    view.drain(&mut realm);

    realm.handle(cast(REND, None));
    assert_eq!(refusal(&view.drain(&mut realm), REND), Some(BAD_TARGETS));

    stand_at(&mut realm, &mob, 7.0);
    realm.handle(cast(REND, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), REND), Some(OUT_OF_RANGE));

    stand_at(&mut realm, &mob, 2.0);
    turn_around(&mut realm, &mob, 2.0);
    realm.handle(cast(REND, Some(mob.guid)));
    assert_eq!(
        refusal(&view.drain(&mut realm), REND),
        Some(UNIT_NOT_INFRONT)
    );

    // Fireball is not a Basic Warrior's.
    realm.handle(cast(133, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), 133), Some(NOT_KNOWN));
}

#[test]
fn hamstring_hits_and_slows_the_mobs_chase() {
    let (mut realm, _, mob, mut view) = warrior_at(10);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(BLOODRAGE, None));
    realm.handle(cast(HAMSTRING, Some(mob.guid)));
    let events = view.drain(&mut realm);
    // A level-10 warrior cannot miss a level-3 mob.
    assert_eq!(spell_damage(&events, HAMSTRING).len(), 1);
    assert!(view.aura(mob.guid, HAMSTRING).is_some());

    // Step away: the mob's chase runs at 60% of its 8 yd/s.
    stand_at(&mut realm, &mob, 15.0);
    let events = view.run(&mut realm, 1.0);
    let speed = chase_speed(&events, mob.guid).expect("the mob gives chase");
    assert!((speed - 7.0 * 1.143 * 0.6).abs() < 0.05, "{speed} yd/s");
}

#[test]
fn thunder_clap_hits_what_stands_close_and_slows_its_swings() {
    let (mut realm, me, mob, mut view) = warrior_at(10);
    realm.handle(cast(BLOODRAGE, None));
    view.run(&mut realm, 8.0);

    // Out of the clap's 8 yd: it goes off, on no one.
    stand_at(&mut realm, &mob, 12.0);
    realm.handle(cast(THUNDER_CLAP, None));
    let events = view.drain(&mut realm);
    assert_eq!(go_hits(&events, THUNDER_CLAP), Some(vec![]));
    assert_eq!(view.rage(me), 0, "paid all the same");

    let (mut realm, _, mob, mut view) = warrior_at(10);
    realm.handle(cast(BLOODRAGE, None));
    view.run(&mut realm, 8.0);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(THUNDER_CLAP, None));
    let events = view.drain(&mut realm);
    assert_eq!(go_hits(&events, THUNDER_CLAP), Some(vec![mob.guid]));
    assert_eq!(spell_damage(&events, THUNDER_CLAP).len(), 1);
    assert!(view.aura(mob.guid, THUNDER_CLAP).is_some());
    realm.handle(cast(THUNDER_CLAP, None));
    assert_eq!(
        refusal(&view.drain(&mut realm), THUNDER_CLAP),
        Some(NOT_READY)
    );

    // The mob swings every 2.2 s instead of 2.
    let gaps = swing_gaps(&mut realm, &mut view, mob.guid, 7.0);
    assert!(!gaps.is_empty());
    assert!(gaps.iter().all(|&g| (g - 2.2).abs() < 0.11), "{gaps:?}");
}

#[test]
fn charge_runs_the_warrior_to_the_mob_for_rage_and_a_stun() {
    let (mut realm, me, mob, mut view) = warrior_at(1);
    stand_at(&mut realm, &mob, 3.0);
    realm.handle(cast(CHARGE, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), CHARGE), Some(TOO_CLOSE));
    stand_at(&mut realm, &mob, 30.0);
    realm.handle(cast(CHARGE, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), CHARGE), Some(OUT_OF_RANGE));

    stand_at(&mut realm, &mob, 15.0);
    realm.handle(cast(CHARGE, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, CHARGE), None);
    let path = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::MonsterMove { guid, path, .. } if *guid == me => Some(path.clone()),
            _ => None,
        })
        .expect("the charge's run");
    let end = *path.last().unwrap();
    assert!(dist(end, mob.position) < 2.5, "ends at the mob");
    assert_eq!(energized(&events, CHARGE), vec![9]);
    assert_eq!(view.rage(me), 9);
    assert!(view.aura(mob.guid, 7922).is_some(), "stunned");

    // Stunned for a second: the mob does not swing until it is over.
    let first = first_swing_at(&mut realm, &mut view, mob.guid, 3.0).expect("it swings at last");
    assert!(first >= 1.0, "swung {first} s in");

    // In combat now, and on cooldown anyway.
    realm.handle(cast(CHARGE, Some(mob.guid)));
    assert_eq!(refusal(&view.drain(&mut realm), CHARGE), Some(NOT_READY));
}

#[test]
fn charge_is_refused_in_combat() {
    let (mut realm, _, mob, mut view) = warrior_at(10);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    view.run(&mut realm, 0.5);
    stand_at(&mut realm, &mob, 15.0);
    realm.handle(cast(CHARGE, Some(mob.guid)));
    assert_eq!(
        refusal(&view.drain(&mut realm), CHARGE),
        Some(AFFECTING_COMBAT)
    );
}

#[test]
fn heroic_strike_waits_for_the_next_swing_and_replaces_it() {
    let (mut realm, me, mob, mut view) = warrior_at(10);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(BLOODRAGE, None));
    view.run(&mut realm, 3.0);
    let rage = view.rage(me);
    assert!(rage >= 15);

    realm.handle(cast(HEROIC_STRIKE, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert!(
        events.iter().all(|e| cast_packet(e).is_none()),
        "nothing until the swing"
    );
    assert_eq!(view.rage(me), rage, "paid at the swing");

    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = view.run(&mut realm, 0.1);
    assert_eq!(go_hits(&events, HEROIC_STRIKE), Some(vec![mob.guid]));
    let damage = spell_damage(&events, HEROIC_STRIKE);
    // The white roll plus 9.5 + 14, through a level-3 mob's armour: no white swing comes close.
    assert!(damage[0] >= 20, "{damage:?}");
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == me)),
        "the strike took the white swing's place"
    );
    assert!(view.rage(me) <= rage - 15 + 1);

    // The swing after is white again.
    let events = view.run(&mut realm, 2.0);
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == me)));
    assert_eq!(go_hits(&events, HEROIC_STRIKE), None);
}

#[test]
fn a_queued_heroic_strike_without_its_rage_at_the_swing_fails_and_swings_white() {
    let (mut realm, me, mob, mut view) = warrior_at(1);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(BLOODRAGE, None));
    view.run(&mut realm, 5.0);
    realm.handle(cast(HEROIC_STRIKE, Some(mob.guid)));
    // The shout spends the rage the strike was waiting on.
    realm.handle(cast(BATTLE_SHOUT, None));
    view.drain(&mut realm);
    assert!(view.rage(me) < 15);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = view.run(&mut realm, 0.1);
    assert_eq!(refusal(&events, HEROIC_STRIKE), Some(NO_POWER));
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == me)));
}

#[test]
fn overpower_opens_only_after_the_mob_dodges() {
    let (mut realm, _, mob, mut view) = warrior_at(1);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(BLOODRAGE, None));
    realm.handle(cast(OVERPOWER, Some(mob.guid)));
    assert_eq!(
        refusal(&view.drain(&mut realm), OVERPOWER),
        Some(CASTER_AURASTATE)
    );

    // The dodge leaves a combo point on the mob, which the client's Overpower button needs.
    let (mut realm, me, mob, mut view) = dodged_fight();
    let f = view.fields(me);
    assert_eq!(f.player_combo_points(), Some(1));
    assert_eq!(f.player_combo_target(), mob.guid);

    realm.handle(cast(OVERPOWER, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, OVERPOWER), None);
    let missed: Vec<u8> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellGo {
                spell_id: OVERPOWER,
                misses,
                ..
            } => Some(misses.iter().map(|m| m.1)),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(missed.iter().all(|&m| m == 1), "only a miss: {missed:?}");
    assert_eq!(view.fields(me).player_combo_points(), Some(0), "spent");
}

#[test]
fn the_dodge_window_closes_after_five_seconds() {
    let (mut realm, me, mob, mut view) = dodged_fight();
    realm.handle(RealmCommand::AttackStop);
    view.run(&mut realm, 5.1);
    assert_eq!(view.fields(me).player_combo_points(), Some(0));
    realm.handle(cast(OVERPOWER, Some(mob.guid)));
    assert_eq!(
        refusal(&view.drain(&mut realm), OVERPOWER),
        Some(CASTER_AURASTATE)
    );
}

// ── Helpers ──────────────────────────────────────────────────────────────────────────────────

/// `UNIT_FIELD_BYTES_1` byte 2's Battle Stance.
const BATTLE_STANCE: u8 = 17;

fn own_create(events: &[SessionEvent], guid: u64) -> benilla_protocol::ObjectFields {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g, fields, ..
            } if *g == guid => Some(fields.clone()),
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
const NOT_KNOWN: u8 = 0x38;
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

/// A human warrior of `level` in the world (on flat ground) at the Crossroads, the mob near it,
/// and what the client has seen so far.
fn warrior_at(level: u8) -> (Realm, u64, Mob, View) {
    warrior_seeded(level, SEED)
}

/// [`warrior_at`] with its own dice.
fn warrior_seeded(level: u8, seed: u64) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"));
    let mut realm = Realm::with_seed(seed);
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

/// A level-1 warrior with Bloodrage's rage, swinging at the level-3 mob until it dodges: about
/// one swing in fifty, so the dice of the first seeded fight that gets one.
fn dodged_fight() -> (Realm, u64, Mob, View) {
    for seed in 0..200 {
        let (mut realm, me, mob, mut view) = warrior_seeded(1, seed);
        stand_at(&mut realm, &mob, 2.0);
        realm.handle(cast(BLOODRAGE, None));
        realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
        for _ in 0..600 {
            realm.tick(0.1, &FlatGround);
            let events = view.drain(&mut realm);
            let dodge = events.iter().any(|e| {
                matches!(e, SessionEvent::AttackerState(s) if s.attacker == me && s.victim_state == 2)
            });
            if dodge && view.health(me) > 0 && view.health(mob.guid) > 0 {
                return (realm, me, mob, view);
            }
            if view.health(me) == 0 || view.health(mob.guid) == 0 {
                break;
            }
        }
    }
    panic!("no seeded fight saw a dodge");
}

/// Stands `yards` from `mob` as [`stand_at`] does, but with the back to it.
fn turn_around(realm: &mut Realm, mob: &Mob, yards: f32) {
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
        orientation: dy.atan2(dx),
    });
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// `SMSG_SPELL_GO`'s hit list for `spell_id`, if it went off.
fn go_hits(events: &[SessionEvent], spell_id: u32) -> Option<Vec<u64>> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellGo {
            spell_id: s, hits, ..
        } if *s == spell_id => Some(hits.clone()),
        _ => None,
    })
}

/// `SMSG_SPELLNONMELEEDAMAGELOG` amounts for `spell_id`.
fn spell_damage(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id => Some(l.damage),
            _ => None,
        })
        .collect()
}

/// `SMSG_PERIODICAURALOG` damage ticks for `spell_id`.
fn periodic_damage(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == spell_id => Some(&l.ticks),
            _ => None,
        })
        .flatten()
        .filter_map(|t| match t {
            benilla_protocol::messages::PeriodicTick::Damage { amount, .. } => Some(*amount),
            _ => None,
        })
        .collect()
}

/// A mob's run speed in its first chase leg, yd/s.
fn chase_speed(events: &[SessionEvent], guid: u64) -> Option<f32> {
    events.iter().find_map(|e| match e {
        SessionEvent::MonsterMove {
            guid: g,
            path,
            duration_ms,
            ..
        } if *g == guid && *duration_ms > 0 => {
            Some(dist(path[0], *path.last().unwrap()) / (*duration_ms as f32 / 1000.0))
        }
        _ => None,
    })
}

/// The seconds into the next `secs` at which `guid` first swings.
fn first_swing_at(realm: &mut Realm, view: &mut View, guid: u64, secs: f32) -> Option<f32> {
    swing_times(realm, view, guid, secs).first().copied()
}

/// The seconds between `guid`'s swings over the next `secs`.
fn swing_gaps(realm: &mut Realm, view: &mut View, guid: u64, secs: f32) -> Vec<f32> {
    swing_times(realm, view, guid, secs)
        .windows(2)
        .map(|w| w[1] - w[0])
        .collect()
}

fn swing_times(realm: &mut Realm, view: &mut View, guid: u64, secs: f32) -> Vec<f32> {
    let mut times = Vec::new();
    for step in 0..(secs / 0.1).round() as u32 {
        realm.tick(0.1, &FlatGround);
        let events = view.drain(realm);
        if events
            .iter()
            .any(|e| matches!(e, SessionEvent::AttackerState(s) if s.attacker == guid))
        {
            times.push((step + 1) as f32 * 0.1);
        }
    }
    times
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
