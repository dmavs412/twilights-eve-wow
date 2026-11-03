//! Trinkets (spec #458): clicking a worn trinket casts its Trinket Spell at once, off the global
//! cooldown, and the item cools down, kept in the save. One in a bag does nothing, and putting one
//! on starts a 30 s wait.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::*;
use gear_common::{descriptor, save_with, BACKPACK_1, SEED};

/// Morgan's Loaded Die and its Trinket Spell, 5 minutes a click.
const DIE: u32 = 906000;
const DIE_SPELL: u32 = 1960000;
const COOLDOWN_MS: u32 = 300_000;
const HEARTHSTONE: u32 = 6948;

const TRINKET_SLOT: u8 = 12;
const PLAYER_BAG: u8 = 255;

// `SpellCastResult`.
const SPELL_FAILED_NOT_READY: u8 = 0x3c;
const SPELL_FAILED_SPELL_IN_PROGRESS: u8 = 0x61;

/// A wall clock to start from, in Unix milliseconds.
const T0: u64 = 1_781_000_000_000;

/// A realm on the wall clock `at` holding the save `blob`, entered as `me`, with the entry burst.
fn enter_at(at: u64, me: u64, blob: &str) -> (Realm, Vec<SessionEvent>) {
    let mut realm = Realm::with_seed(SEED);
    realm.set_wall_clock(at);
    realm.import_character(blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &FlatGround);
    let events = drain(&mut realm);
    (realm, events)
}

/// A warrior wearing the die, entered at `T0`.
fn wearing_the_die(carried: &[(u8, u32, u32)]) -> (Realm, u64, Vec<SessionEvent>) {
    let mut items = vec![(TRINKET_SLOT, DIE, 1)];
    items.extend_from_slice(carried);
    let (me, blob) = save_with(10, 0, &items);
    let (realm, events) = enter_at(T0, me, &blob);
    (realm, me, events)
}

fn click(realm: &mut Realm, slot: u8) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::UseItem {
        bag: PLAYER_BAG,
        slot,
    });
    drain(realm)
}

/// Ticks `secs` in 1 s steps, collecting the events.
fn wait(realm: &mut Realm, secs: u32) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    for _ in 0..secs {
        realm.tick(1.0, &FlatGround);
        events.extend(drain(realm));
    }
    events
}

/// The die's `SpellGo`: its item caster.
fn die_go(events: &[SessionEvent]) -> Option<Option<u64>> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellGo {
            spell_id: DIE_SPELL,
            item_caster,
            ..
        } => Some(*item_caster),
        _ => None,
    })
}

fn refusal(events: &[SessionEvent]) -> Option<(u32, Option<u8>)> {
    events.iter().find_map(|e| match e {
        SessionEvent::CastResult {
            spell_id,
            success: false,
            reason,
            ..
        } => Some((*spell_id, *reason)),
        _ => None,
    })
}

/// The save's `item_ready_at` for `entry`, in Unix ms.
fn ready_at(blob: &str, entry: u32) -> Option<u64> {
    let save: toml::Table = toml::from_str(blob).unwrap();
    save.get("item_ready_at")?
        .as_array()?
        .iter()
        .filter_map(|pair| pair.as_array())
        .find(|pair| pair[0].as_integer() == Some(i64::from(entry)))
        .and_then(|pair| pair[1].as_integer())
        .map(|ms| ms as u64)
}

#[test]
fn a_worn_die_goes_off_at_once_and_cools_down_for_five_minutes() {
    let (mut realm, me, events) = wearing_the_die(&[]);
    let die = descriptor(&events, me)
        .player_inv_slot(TRINKET_SLOT)
        .expect("the die in its slot");
    let events = click(&mut realm, TRINKET_SLOT);
    assert_eq!(die_go(&events), Some(Some(die)), "{events:?}");
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SessionEvent::SpellStart { .. })),
        "no cast bar"
    );
    let blob = realm.export_character(me).unwrap();
    assert_eq!(ready_at(&blob, DIE), Some(T0 + u64::from(COOLDOWN_MS)));

    let events = click(&mut realm, TRINKET_SLOT);
    assert_eq!(die_go(&events), None);
    assert_eq!(
        refusal(&events),
        Some((DIE_SPELL, Some(SPELL_FAILED_NOT_READY)))
    );
}

#[test]
fn a_click_during_a_hearthstone_cast_is_refused() {
    let (mut realm, _, _) = wearing_the_die(&[(BACKPACK_1, HEARTHSTONE, 1)]);
    click(&mut realm, BACKPACK_1);
    let events = click(&mut realm, TRINKET_SLOT);
    assert_eq!(die_go(&events), None);
    assert_eq!(
        refusal(&events),
        Some((DIE_SPELL, Some(SPELL_FAILED_SPELL_IN_PROGRESS)))
    );
}

#[test]
fn the_die_in_the_backpack_does_nothing() {
    let (me, blob) = save_with(10, 0, &[(BACKPACK_1, DIE, 1)]);
    let (mut realm, _) = enter_at(T0, me, &blob);
    let events = click(&mut realm, BACKPACK_1);
    assert!(events.is_empty(), "{events:?}");
}

#[test]
fn putting_the_die_on_starts_a_thirty_second_wait() {
    let (me, blob) = save_with(10, 0, &[(BACKPACK_1, DIE, 1)]);
    let (mut realm, _) = enter_at(T0, me, &blob);
    realm.handle(RealmCommand::AutoEquipItem {
        bag: PLAYER_BAG,
        slot: BACKPACK_1,
    });
    let events = drain(&mut realm);
    assert!(
        events.iter().any(|e| matches!(
            e,
            SessionEvent::ItemCooldown {
                spell_id: DIE_SPELL,
                ..
            }
        )),
        "{events:?}"
    );

    wait(&mut realm, 29);
    let events = click(&mut realm, TRINKET_SLOT);
    assert_eq!(
        refusal(&events),
        Some((DIE_SPELL, Some(SPELL_FAILED_NOT_READY)))
    );
    wait(&mut realm, 2);
    assert!(die_go(&click(&mut realm, TRINKET_SLOT)).is_some());
}

#[test]
fn the_cooldown_comes_back_at_login_outside_the_spell_book() {
    let (mut realm, me, _) = wearing_the_die(&[]);
    click(&mut realm, TRINKET_SLOT);
    wait(&mut realm, 10);
    realm.handle(RealmCommand::Logout);
    drain(&mut realm);
    let blob = realm.export_character(me).unwrap();

    let (_, events) = enter_at(T0 + 10_000, me, &blob);
    let cooldowns = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellCooldowns { cooldowns, .. } => Some(cooldowns.clone()),
            _ => None,
        })
        .expect("the die's cooldown at login");
    let &(spell, left) = cooldowns.first().unwrap();
    assert_eq!(spell, DIE_SPELL);
    assert!((289_000..=290_100).contains(&left), "{left}");
    let book = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellBook { cooldowns, .. } => Some(cooldowns.clone()),
            _ => None,
        })
        .expect("the spell book");
    assert_eq!(book, Vec::new(), "no Trinket Spell in the u16 list");
}
