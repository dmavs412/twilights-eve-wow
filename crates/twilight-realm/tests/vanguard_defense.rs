//! The Vanguard's defensive stats: parry and block chance from talents, and Shield of the
//! Penitent's block aura. Played through the realm's seam by a level 10 Vanguard with the talents
//! in its save, near the Crossroads, in the starting outfit's sword and shield (or without the
//! shield).

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::*;
use common::*;
use twilight_realm::callings;
use twilight_realm::{Realm, RealmCommand};

const SEED: u64 = 0x5EED_0099;
const LEVEL: u8 = 10;
const OFF_HAND: u8 = 16;

fn vanguard() -> &'static callings::CallingDef {
    callings::calling("vanguard").expect("the Vanguard is a shipped Calling")
}

/// The `Talent.dbc` id of the Vanguard talent `key`.
fn talent(key: &str) -> u32 {
    vanguard()
        .talent_tabs
        .iter()
        .flat_map(|t| &t.talents)
        .find(|t| t.key == key)
        .unwrap_or_else(|| panic!("the Vanguard has {key}"))
        .talent_id
}

/// The spell id of the Vanguard skill `key`.
fn skill(key: &str) -> u32 {
    vanguard()
        .skills
        .iter()
        .find(|s| s.key == key)
        .unwrap_or_else(|| panic!("the Vanguard has {key} on the bar"))
        .spell_id
}

/// A level 10 Vanguard with `talents`, the shield worn when `shield`, in the world: the realm,
/// its guid, a mob near it and what the client has seen.
fn vanguard_in_world(talents: &[(u32, u8)], shield: bool) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Bulwark", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {LEVEL}\n"))
        .replace("calling = \"basic_warrior\"", "calling = \"vanguard\"");
    if !shield {
        blob = drop_slot(&blob, OFF_HAND);
    }
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
    }
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let entry = view.drain(&mut realm);
    let mob = entry
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
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

fn block(view: &View, me: u64) -> f32 {
    view.fields(me).player_block_percentage().unwrap()
}

fn parry(view: &View, me: u64) -> f32 {
    view.fields(me).player_parry_percentage().unwrap()
}

#[test]
fn shield_specialization_adds_block_only_with_a_shield() {
    let spec = [(talent("shield_specialization"), 5)];
    let (_, me, _, bare) = vanguard_in_world(&[], true);
    let (_, me2, _, trained) = vanguard_in_world(&spec, true);
    let (_, me3, _, unshielded) = vanguard_in_world(&spec, false);
    assert!(block(&bare, me) > 0.0, "a shield blocks");
    assert!(
        (block(&trained, me2) - block(&bare, me) - 5.0).abs() < 1e-3,
        "block {} -> {}",
        block(&bare, me),
        block(&trained, me2)
    );
    assert_eq!(block(&unshielded, me3), 0.0, "no shield, no block");
}

#[test]
fn reactive_strike_adds_parry() {
    let (_, me, _, bare) = vanguard_in_world(&[], true);
    let (_, me2, _, trained) = vanguard_in_world(&[(talent("reactive_strike"), 5)], true);
    assert!(
        (parry(&trained, me2) - parry(&bare, me) - 5.0).abs() < 1e-3,
        "parry {} -> {}",
        parry(&bare, me),
        parry(&trained, me2)
    );
}

#[test]
fn shield_of_the_penitent_adds_seventy_five_block_for_ten_seconds() {
    let penitent = skill("shield_of_penitent");
    let (mut realm, me, mob, mut view) = vanguard_in_world(&[], true);
    let base = block(&view, me);
    // Rage for the cast, from trading blows with the mob.
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut waited = 0.0;
    while view.rage(me) < 10 {
        view.run(&mut realm, 0.1);
        waited += 0.1;
        assert!(waited < 30.0, "never reached 10 rage");
    }
    realm.handle(cast(penitent, None));
    assert_eq!(refusal(&view.drain(&mut realm), penitent), None, "the cast");
    assert!((block(&view, me) - base - 75.0).abs() < 1e-3, "on");
    view.run(&mut realm, 9.0);
    assert!(
        (block(&view, me) - base - 75.0).abs() < 1e-3,
        "still on at 9 s"
    );
    view.run(&mut realm, 2.0);
    assert!((block(&view, me) - base).abs() < 1e-3, "off after 11 s");
}

/// The spell id of the Vanguard talent `key`'s rank `rank`.
fn rank_spell(key: &str, rank: usize) -> u32 {
    vanguard()
        .talent_tabs
        .iter()
        .flat_map(|t| &t.talents)
        .find(|t| t.key == key)
        .unwrap_or_else(|| panic!("the Vanguard has {key}"))
        .rank_spells[rank - 1]
}

/// Whether `e` is `mob`'s swing at `me` that the player blocked.
fn blocked(e: &SessionEvent, mob: u64, me: u64) -> bool {
    matches!(e, SessionEvent::AttackerState(s)
        if s.attacker == mob && s.victim == me && (s.blocked > 0 || s.victim_state == 5))
}

/// The damage `me` dealt under `spell_id` in `events`, one entry a hit.
fn dealt(events: &[SessionEvent], me: u64, spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.attacker == me && l.spell_id == spell_id => {
                Some(l.damage)
            }
            _ => None,
        })
        .collect()
}

/// Pulls `mob` onto the player, who stands facing it and does not swing back, so the fight
/// lasts: every event of the next `secs`.
fn tank(realm: &mut Realm, view: &mut View, mob: &Mob, secs: f32) -> Vec<SessionEvent> {
    stand_at(realm, mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    realm.handle(RealmCommand::AttackStop);
    let mut seen = Vec::new();
    let mut t = 0.0;
    while t < secs {
        seen.extend(view.run(realm, 0.1));
        t += 0.1;
    }
    seen
}

#[test]
fn shield_specialization_gives_two_rage_on_every_block() {
    let spell = rank_spell("shield_specialization", 5);
    let (mut realm, me, mob, mut view) =
        vanguard_in_world(&[(talent("shield_specialization"), 5)], true);
    let seen = tank(&mut realm, &mut view, &mob, 60.0);
    let blocks = seen.iter().filter(|e| blocked(e, mob.guid, me)).count();
    let energizes: Vec<u32> = seen
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellEnergizeLog(l) if l.spell_id == spell => Some(l.amount),
            _ => None,
        })
        .collect();
    assert!(blocks > 0, "the seed blocks");
    assert_eq!(energizes.len(), blocks, "one energize a block");
    assert!(energizes.iter().all(|&a| a == 2), "{energizes:?}");
}

#[test]
fn a_block_under_shield_of_the_penitent_burns_the_attacker() {
    let penitent = skill("shield_of_penitent");
    let (mut realm, me, mob, mut view) = vanguard_in_world(&[], true);
    // Rage for the cast from trading blows, then the player stops swinging.
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut waited = 0;
    while view.rage(me) < 10 {
        view.run(&mut realm, 0.1);
        waited += 1;
        assert!(waited < 300, "never reached 10 rage");
    }
    realm.handle(RealmCommand::AttackStop);
    realm.handle(cast(penitent, None));
    let mut seen = view.drain(&mut realm);
    assert_eq!(refusal(&seen, penitent), None, "the cast");
    for _ in 0..95 {
        seen.extend(view.run(&mut realm, 0.1));
    }
    let blocks = seen.iter().filter(|e| blocked(e, mob.guid, me)).count();
    let burns = dealt(&seen, me, penitent);
    assert!(blocks > 0, "the seed blocks under the aura");
    assert_eq!(burns.len(), blocks, "one burn a block");
    // 30 + 3 a level, Holy: no armour, no roll.
    let want = 30 + 3 * u32::from(LEVEL);
    assert!(burns.iter().all(|&d| d == want), "{burns:?}, want {want}");
}

#[test]
fn iron_retribution_hits_back_for_half_the_block_value() {
    let spell = rank_spell("iron_retribution", 5);
    let (mut realm, me, mob, mut view) =
        vanguard_in_world(&[(talent("iron_retribution"), 5)], true);
    let seen = tank(&mut realm, &mut view, &mob, 60.0);
    let blocks = seen.iter().filter(|e| blocked(e, mob.guid, me)).count();
    let hits = dealt(&seen, me, spell);
    assert!(blocks > 0, "the seed blocks");
    assert_eq!(hits.len(), blocks, "one hit back a block");
    // Half of the Block Value; physical, so the attacker's armour takes a little off.
    let first = hits[0];
    assert!(first > 0, "{hits:?}");
    assert!(hits.iter().all(|&d| d == first), "{hits:?}");
}

#[test]
fn tempered_steel_raises_block_after_a_crit_taken() {
    let aura = vanguard()
        .auras
        .iter()
        .find(|a| a.key == "aura_tempered_steel_5")
        .expect("Tempered Steel's rank 5 buff")
        .spell_id;
    let (mut realm, me, mob, mut view) = vanguard_in_world(&[(talent("tempered_steel"), 5)], true);
    let base = block(&view, me);
    let mut seen = tank(&mut realm, &mut view, &mob, 0.1);
    for _ in 0..1200 {
        let crit = seen.iter().any(|e| {
            matches!(e, SessionEvent::AttackerState(s)
            if s.attacker == mob.guid && s.hit_info & 0x80 != 0)
        });
        if crit {
            assert!(
                view.aura(me, aura).is_some(),
                "the buff is up after the crit"
            );
            assert!((block(&view, me) - base - 50.0).abs() < 1e-3, "+50% block");
            return;
        }
        seen = view.run(&mut realm, 0.1);
    }
    panic!("no crit taken in two minutes");
}
