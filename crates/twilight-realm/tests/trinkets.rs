//! Trinkets (spec #458): clicking a worn trinket casts its Trinket Spell at once, off the global
//! cooldown, and the item cools down, kept in the save. One in a bag does nothing, and putting one
//! on starts a 30 s wait.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::fight::CAMP;
use common::vanguard::{scene_of, vanguard_save, Scene};
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

/// [`enter_at`] at `T0` on `seed`'s dice.
fn enter_seeded(seed: u64, me: u64, blob: &str) -> (Realm, Vec<SessionEvent>) {
    let mut realm = Realm::with_seed(seed);
    realm.set_wall_clock(T0);
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

/// The die's buffs: Quick Hands, Blood Luck and Sixes.
const BUFFS: [u32; 3] = [1960001, 1960002, 1960003];

/// Every `/random` line in `events`: (min, max, roll).
fn rolls(events: &[SessionEvent]) -> Vec<(u32, u32, u32)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::RandomRoll { min, max, roll, .. } => Some((*min, *max, *roll)),
            _ => None,
        })
        .collect()
}

/// `save` with the die worn.
fn with_the_die((me, blob): (u64, String)) -> (u64, String) {
    (
        me,
        format!("{blob}\n[[items]]\nslot = {TRINKET_SLOT}\nentry = {DIE}\ncount = 1\n"),
    )
}

/// The save entered on the first seed whose first click of the die rolls `face`, `prepare` run on
/// the realm before the click: the realm, the entry burst with what `prepare` sent, and the
/// click's events.
fn rolling(
    (me, blob): &(u64, String),
    face: u32,
    prepare: impl Fn(&mut Realm),
) -> (Realm, Vec<SessionEvent>, Vec<SessionEvent>) {
    for seed in 0..200 {
        let (mut realm, mut entry) = enter_seeded(seed, *me, blob);
        prepare(&mut realm);
        entry.extend(drain(&mut realm));
        let events = click(&mut realm, TRINKET_SLOT);
        if rolls(&events) == [(1, 6, face)] {
            return (realm, entry, events);
        }
    }
    panic!("no seed's first click rolls {face}");
}

/// The player's health in the last update of it in `events`.
fn health(events: &[SessionEvent], me: u64) -> Option<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectValues { guid, fields } if *guid == me => fields.unit_health(),
            _ => None,
        })
        .last()
}

/// The heals `spell_id` logged on `me`.
fn heals(events: &[SessionEvent], me: u64, spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellHealLog(l) if l.target == me && l.spell_id == spell_id => {
                Some(l.amount)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn every_click_of_the_die_rolls_one_to_six_in_chat() {
    let (mut realm, me, _) = wearing_the_die(&[]);
    let mut seen = Vec::new();
    for _ in 0..12 {
        let events = click(&mut realm, TRINKET_SLOT);
        let rolled = rolls(&events);
        assert_eq!(rolled.len(), 1, "{events:?}");
        let (min, max, roll) = rolled[0];
        assert_eq!((min, max), (1, 6));
        assert!((1..=6).contains(&roll));
        assert!(events
            .iter()
            .any(|e| matches!(e, SessionEvent::RandomRoll { guid, .. } if *guid == me)));
        seen.push(roll);
        // Past the five minute cooldown, and any buff it put up.
        wait(&mut realm, 301);
        realm.set_player_health(100_000);
        drain(&mut realm);
    }
    seen.sort_unstable();
    seen.dedup();
    assert!(seen.len() > 1, "twelve clicks all rolled {seen:?}");
}

#[test]
fn face_one_takes_30_percent_of_the_current_health_and_never_kills() {
    let save = with_the_die(save_with(10, 0, &[]));
    let me = save.0;
    let (_, _, events) = rolling(&save, 1, |r| r.set_player_health(100));
    let hurt: Vec<(u64, u64, u32)> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == DIE_SPELL => {
                Some((l.attacker, l.target, l.damage))
            }
            _ => None,
        })
        .collect();
    assert_eq!(hurt, [(me, me, 30)]);
    assert_eq!(health(&events, me), Some(70));

    let (_, _, events) = rolling(&save, 1, |r| r.set_player_health(1));
    assert_eq!(health(&events, me), Some(1));
    assert!(!events
        .iter()
        .any(|e| matches!(e, SessionEvent::SpellDamageLog(l) if l.damage > 0)));
}

#[test]
fn face_two_heals_a_fifth_of_the_maximum_health() {
    let save = with_the_die(save_with(10, 0, &[]));
    let me = save.0;
    let (_, entry, events) = rolling(&save, 2, |r| r.set_player_health(1));
    let max = descriptor(&entry, me).unit_max_health().unwrap();
    assert_eq!(heals(&events, me, DIE_SPELL), [max / 5]);
    assert_eq!(health(&events, me), Some(1 + max / 5));
}

#[test]
fn face_three_fills_a_quarter_of_a_vanguards_empty_rage_bar() {
    const RAGE: u8 = 1;
    let save = with_the_die(vanguard_save(10, true));
    let me = save.0;
    let (_, entry, events) = rolling(&save, 3, |_| {});
    assert_eq!(descriptor(&entry, me).unit_power(RAGE), Some(0));
    let gained: Vec<(u32, u32)> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellEnergizeLog(l) if l.spell_id == DIE_SPELL => {
                Some((l.power, l.amount))
            }
            _ => None,
        })
        .collect();
    assert_eq!(gained, [(u32::from(RAGE), 25)]);
    let mut all = entry;
    all.extend(events);
    // Rage is held in tenths.
    assert_eq!(descriptor(&all, me).unit_power(RAGE), Some(250));
}

#[test]
fn faces_four_to_six_put_their_buff_on_for_15_seconds() {
    let save = with_the_die(save_with(10, 0, &[]));
    let me = save.0;
    for (face, buff) in (4..=6).zip(BUFFS) {
        let (mut realm, entry, events) = rolling(&save, face, |_| {});
        let mut all = entry;
        all.extend(events);
        let on =
            |all: &[SessionEvent]| descriptor(all, me).unit_auras().any(|a| a.spell_id == buff);
        assert!(on(&all), "face {face}: {buff} on");
        for _ in 0..149 {
            realm.tick(0.1, &FlatGround);
            all.extend(drain(&mut realm));
        }
        assert!(on(&all), "face {face}: {buff} still on at 14.9 s");
        for _ in 0..2 {
            realm.tick(0.1, &FlatGround);
            all.extend(drain(&mut realm));
        }
        assert!(!on(&all), "face {face}: {buff} gone at 15.1 s");
    }
}

/// Bulwark Charge's row opens at 16; Hammer of the Vanguard ends it in a strike.
const VANGUARD_LEVEL: u8 = 16;

fn hammer() -> u32 {
    choice("vanguard", "bulwark_charge", "hammer_of_vanguard")
}

/// A Vanguard wearing the die, Hammer of the Vanguard set, 25 yd from the camp's first mob on
/// `seed`'s dice, after one click of the die: the scene and the face it rolled.
fn vanguard_clicked(seed: u64) -> (Scene, u32) {
    let (me, blob) = with_the_die(vanguard_save(VANGUARD_LEVEL, true));
    let mut s = scene_of(me, &blob, seed, &[("bulwark_charge", hammer())], CAMP);
    s.realm.handle(RealmCommand::UseItem {
        bag: PLAYER_BAG,
        slot: TRINKET_SLOT,
    });
    let events = s.view.drain(&mut s.realm);
    let face = rolls(&events).first().map_or(0, |r| r.2);
    (s, face)
}

/// The damage of each Hammer strike in `events`.
fn hammer_hits(events: &[SessionEvent]) -> Vec<u32> {
    let hammer = hammer();
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == hammer => Some(l.damage),
            _ => None,
        })
        .collect()
}

#[test]
fn under_sixes_the_same_seeded_skill_hit_deals_double() {
    let mut compared = 0;
    for seed in 0..80 {
        let (mut sixes, face) = vanguard_clicked(seed);
        if face != 6 {
            continue;
        }
        let doubled = hammer_hits(&sixes.cast_at_mob(hammer()));
        // The same dice, the buff taken off before the charge.
        let (mut plain, _) = vanguard_clicked(seed);
        plain
            .realm
            .handle(RealmCommand::CancelAura { spell_id: BUFFS[2] });
        plain.view.drain(&mut plain.realm);
        let single = hammer_hits(&plain.cast_at_mob(hammer()));
        if single.iter().all(|&d| d == 0) {
            continue;
        }
        let twice: Vec<u32> = single.iter().map(|d| d * 2).collect();
        assert_eq!(doubled, twice, "seed {seed}");
        compared += 1;
        if compared == 2 {
            return;
        }
    }
    panic!("{compared} seeds rolled a six and landed the strike");
}

#[test]
fn under_blood_luck_a_skill_hit_of_n_heals_n_over_5() {
    for seed in 0..80 {
        let (mut s, face) = vanguard_clicked(seed);
        if face != 5 {
            continue;
        }
        s.realm.set_player_health(10);
        s.view.drain(&mut s.realm);
        let events = s.cast_at_mob(hammer());
        let Some(&n) = hammer_hits(&events).first().filter(|&&n| n >= 5) else {
            continue;
        };
        assert_eq!(heals(&events, s.me, BUFFS[1]), [n / 5], "seed {seed}");
        return;
    }
    panic!("no seed rolled a five and landed the strike");
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
