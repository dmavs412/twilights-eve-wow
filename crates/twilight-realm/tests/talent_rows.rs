//! Talent Rows (ADR 0028, #500): the realm keeps each character's set choice per row, sends the
//! rows, sets one (free on an empty row, only at an innkeeper on a set one), swaps the spell book
//! and the bar, keeps the rows in the save and clears them on Advancement. The last four tests
//! play the effect params the row choices brought: `controlled_pct` (Backdraft),
//! `below_health_pct` (Finisher), `cleave_pct` (Cleave) and `skill=any` (Blindside). Played
//! through the realm's seam by a Basic Warrior at the Crossroads.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::messages::SpellDamageLog;
use benilla_protocol::{SessionEvent, TalentRowView};
use common::fight::*;
use common::*;
use quest_common::{accept, hand_in, save_with_quests};

static CHARGE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("basic_warrior", "charge"));
static HEROIC_STRIKE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("basic_warrior", "heroic_strike"));
static BULL_RUSH: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("basic_warrior", "charge", "bull_rush"));
static RALLYING_CHARGE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("basic_warrior", "charge", "rallying_charge"));
static BACKDRAFT: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("basic_warrior", "fire_blast", "backdraft"));
static BLINDSIDE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("basic_warrior", "gouge", "blindside"));
static CLEAVE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("basic_warrior", "heroic_strike", "cleave"));
static FINISHER: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| choice("basic_warrior", "heroic_strike", "finisher"));

const SEED: u64 = 0x5EED_0500;
/// Innkeeper Boorand Plainswind, in the Crossroads inn.
const INNKEEPER: u32 = 3934;
const RETHINK_LINE: &str = "I want to rethink my talents.";
/// Uzzek, the Crossroads' Warrior trainer, and his Advancement Quest into Vanguard.
const UZZEK: u32 = 5810;
const THE_WARRIORS_PATH: u32 = 64001;
const SPELL_FAILED_NOT_KNOWN: u8 = 0x38;
/// `SpellDamageLog::hit_info` of a crit.
const SPELL_HIT_TYPE_CRIT: u32 = 0x2;

/// A Basic Warrior of `level` at the Crossroads on `seed`'s dice: the realm, its guid, what the
/// client has seen, and the entry burst.
fn warrior(level: u8, seed: u64) -> (Realm, u64, View, Vec<SessionEvent>) {
    let (guid, blob) = save_with_quests(level, &[], 0, &[]);
    entered(guid, &blob, seed)
}

/// A realm holding just the save `blob`, entered as `guid`.
fn entered(guid: u64, blob: &str, seed: u64) -> (Realm, u64, View, Vec<SessionEvent>) {
    let mut realm = Realm::with_seed(seed);
    realm.import_character(blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let entry = view.drain(&mut realm);
    (realm, guid, view, entry)
}

/// The mob near the Crossroads, out of the entry burst.
fn the_mob(entry: &[SessionEvent]) -> Mob {
    entry
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
        .expect("a mob near the Crossroads")
}

/// Sets `row` to `spell_id`: the events it sent.
fn set(realm: &mut Realm, view: &mut View, row: &str, spell_id: u32) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::SetTalentRow {
        row: row.into(),
        spell_id,
    });
    view.drain(realm)
}

/// The last `TalentRows` in `events`: its Calling, its `at_innkeeper` and its rows.
fn rows_in(events: &[SessionEvent]) -> Option<(&str, bool, &[TalentRowView])> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::TalentRows {
            calling,
            at_innkeeper,
            rows,
        } => Some((calling.as_str(), *at_innkeeper, rows.as_slice())),
        _ => None,
    })
}

/// The last bar sent in `events`, as `(slot, spell)`.
fn bar_in(events: &[SessionEvent]) -> Vec<(u8, u32)> {
    events
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::ActionButtons { buttons } => {
                Some(buttons.iter().map(|b| (b.slot, b.action)).collect())
            }
            _ => None,
        })
        .expect("the bar is sent")
}

/// The spell book sent in `events`.
fn book_in(events: &[SessionEvent]) -> Vec<u32> {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::SpellBook { spell_ids, .. } => Some(spell_ids.clone()),
            _ => None,
        })
        .expect("the spell book is sent")
}

fn removed(events: &[SessionEvent], spell: u32) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::SpellRemoved { spell_id } if *spell_id == spell))
}

fn learned(events: &[SessionEvent], spell: u32) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::SpellLearned { spell_id } if *spell_id == spell))
}

/// The damage `spell` dealt in `events`.
fn hits(events: &[SessionEvent], spell: u32) -> Vec<SpellDamageLog> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == spell => Some(l.clone()),
            _ => None,
        })
        .collect()
}

// ── The rows ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn login_sends_the_rows_after_the_spell_book_with_only_the_charge_row_open_at_level_2() {
    let (_, _, _, entry) = warrior(2, SEED);
    let book = entry
        .iter()
        .position(|e| matches!(e, SessionEvent::SpellBook { .. }))
        .expect("the spell book");
    let sent: Vec<usize> = (0..entry.len())
        .filter(|&i| matches!(entry[i], SessionEvent::TalentRows { .. }))
        .collect();
    assert_eq!(sent.len(), 1, "one TalentRows");
    assert!(sent[0] > book, "after the spell book");
    let (calling, at_innkeeper, rows) = rows_in(&entry).unwrap();
    assert_eq!(calling, "Basic Warrior");
    assert!(!at_innkeeper);
    let shape: Vec<(&str, u8, bool, u32)> = rows
        .iter()
        .map(|r| (r.key.as_str(), r.level, r.open, r.set))
        .collect();
    assert_eq!(
        shape,
        [
            ("charge", 2, true, 0),
            ("fire_blast", 4, false, 0),
            ("gouge", 6, false, 0),
            ("heroic_strike", 8, false, 0),
        ]
    );
    assert_eq!(rows[0].skill, "Charge");
    let choices: Vec<u32> = rows[0].choices.iter().map(|c| c.spell_id).collect();
    assert_eq!(choices.len(), 3);
    assert!(choices.contains(&*BULL_RUSH), "{choices:?}");
}

#[test]
fn at_level_1_a_set_on_the_charge_row_sends_nothing() {
    let (mut realm, me, mut view, _) = warrior(1, SEED);
    let events = set(&mut realm, &mut view, "charge", *BULL_RUSH);
    assert!(events.is_empty(), "{events:?}");
    realm.handle(RealmCommand::OpenTalentRows);
    let events = view.drain(&mut realm);
    let (_, _, rows) = rows_in(&events).expect("the rows");
    assert!(rows.iter().all(|r| r.set == 0), "{rows:?}");
    let blob = realm.export_character(me).unwrap();
    assert!(!blob.contains("\"charge\""), "{blob}");
}

#[test]
fn setting_bull_rush_swaps_charge_out_of_the_book_and_the_bar() {
    let (mut realm, _, mut view, entry) = warrior(2, SEED);
    let (slot, _) = bar_in(&entry)
        .into_iter()
        .find(|(_, spell)| *spell == *CHARGE)
        .expect("Charge on the bar");

    let events = set(&mut realm, &mut view, "charge", *BULL_RUSH);
    assert!(removed(&events, *CHARGE), "{events:?}");
    assert!(learned(&events, *BULL_RUSH), "{events:?}");
    let bar = bar_in(&events);
    assert!(bar.contains(&(slot, *BULL_RUSH)), "{bar:?}");
    assert!(bar.iter().all(|(_, spell)| *spell != *CHARGE), "{bar:?}");
    let (_, _, rows) = rows_in(&events).expect("the rows");
    assert_eq!(rows[0].set, *BULL_RUSH);

    let mob = the_mob(&entry);
    stand_at(&mut realm, &mob, 15.0);
    realm.handle(cast(*CHARGE, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, *CHARGE), Some(SPELL_FAILED_NOT_KNOWN));
    realm.handle(cast(*BULL_RUSH, Some(mob.guid)));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, *BULL_RUSH), None, "Bull Rush is cast");
    assert!(
        events.iter().any(
            |e| matches!(e, SessionEvent::SpellGo { spell_id, .. } if *spell_id == *BULL_RUSH)
        ),
        "{events:?}"
    );
}

#[test]
fn a_set_row_changes_only_at_an_innkeeper() {
    let (mut realm, _, mut view, _) = warrior(2, SEED);
    set(&mut realm, &mut view, "charge", *BULL_RUSH);
    let events = set(&mut realm, &mut view, "charge", *RALLYING_CHARGE);
    assert!(events.is_empty(), "away from any inn: {events:?}");

    walk_up_to(&mut realm, hub_guid(INNKEEPER));
    view.drain(&mut realm);
    let events = set(&mut realm, &mut view, "charge", *RALLYING_CHARGE);
    assert!(removed(&events, *BULL_RUSH), "{events:?}");
    assert!(learned(&events, *RALLYING_CHARGE), "{events:?}");
    let (_, at_innkeeper, rows) = rows_in(&events).expect("the rows");
    assert!(at_innkeeper);
    assert_eq!(rows[0].set, *RALLYING_CHARGE);
    let again = set(&mut realm, &mut view, "charge", *RALLYING_CHARGE);
    assert!(again.is_empty(), "the same choice again: {again:?}");
}

#[test]
fn the_innkeepers_rethink_line_opens_the_rows_at_the_innkeeper() {
    let (mut realm, _, mut view, _) = warrior(2, SEED);
    let inn = hub_guid(INNKEEPER);
    walk_up_to(&mut realm, inn);
    realm.handle(RealmCommand::GossipHello { guid: inn });
    let option = view
        .drain(&mut realm)
        .iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { npc, options, .. } if *npc == inn => options
                .iter()
                .find(|o| o.message == RETHINK_LINE)
                .map(|o| o.index),
            _ => None,
        })
        .expect("the rethink line");
    realm.handle(RealmCommand::GossipSelectOption { guid: inn, option });
    let events = view.drain(&mut realm);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SessionEvent::GossipComplete)),
        "the menu closes: {events:?}"
    );
    let (_, at_innkeeper, rows) = rows_in(&events).expect("the rows");
    assert!(at_innkeeper);
    assert_eq!(rows.len(), 4);
}

#[test]
fn the_save_keeps_a_set_row_and_a_version_5_saves_talents_load_as_empty_rows() {
    let (mut realm, me, mut view, _) = warrior(2, SEED);
    set(&mut realm, &mut view, "charge", *BULL_RUSH);
    let blob = realm.export_character(me).unwrap();
    let (_, _, _, entry) = entered(me, &blob, SEED);
    let (_, _, rows) = rows_in(&entry).expect("the rows");
    assert_eq!(rows[0].set, *BULL_RUSH);
    let book = book_in(&entry);
    assert!(
        book.contains(&*BULL_RUSH) && !book.contains(&*CHARGE),
        "{book:?}"
    );

    // Version 5 kept the tree's talents and had no rows.
    let (guid, blob) = save_with_quests(2, &[], 0, &[]);
    let old = blob
        .replacen("version = 6\n", "version = 5\n", 1)
        .replace("money = 0\n", "money = 0\ntalents = [[90001, 3]]\n");
    assert_ne!(old, blob);
    let (_, _, _, entry) = entered(guid, &old, SEED);
    let (_, _, rows) = rows_in(&entry).expect("the rows");
    assert!(rows.iter().all(|r| r.set == 0), "{rows:?}");
    assert!(book_in(&entry).contains(&*CHARGE));
}

#[test]
fn advancement_clears_the_rows_and_sends_the_vanguards() {
    let (mut realm, me, mut view, _) = warrior(10, SEED);
    set(&mut realm, &mut view, "charge", *BULL_RUSH);
    let uzzek = hub_guid(UZZEK);
    accept(&mut realm, uzzek, THE_WARRIORS_PATH);
    let events = hand_in(&mut realm, uzzek, THE_WARRIORS_PATH);
    assert_eq!(realm.current_calling().unwrap().key, "vanguard");
    let (calling, _, rows) = rows_in(&events).expect("the rows");
    assert_eq!(calling, "Vanguard");
    // Every row empty, whatever rows the Vanguard has.
    assert!(rows.iter().all(|r| r.set == 0), "{rows:?}");
    let blob = realm.export_character(me).unwrap();
    assert!(!blob.contains("\"charge\""), "{blob}");
}

// ── The effect params ───────────────────────────────────────────────────────────────────────────

/// A level 10 Basic Warrior on `seed`'s dice with `choice` set on `row`, beside the nearest mob,
/// swinging at it until there is `rage`, then stopped: the realm, its guid, the mob and the view.
fn raged(row: &str, choice: u32, rage: u32, seed: u64) -> (Realm, u64, Mob, View) {
    let (mut realm, me, mut view, entry) = warrior(10, seed);
    set(&mut realm, &mut view, row, choice);
    let mob = the_mob(&entry);
    stand_at(&mut realm, &mob, 15.0);
    realm.handle(cast(*CHARGE, Some(mob.guid)));
    assert_eq!(
        refusal(&view.drain(&mut realm), *CHARGE),
        None,
        "the charge"
    );
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    for _ in 0..600 {
        if view.rage(me) >= rage {
            realm.handle(RealmCommand::AttackStop);
            view.drain(&mut realm);
            return (realm, me, mob, view);
        }
        view.run(&mut realm, 0.1);
        assert!(view.health(me) > 0, "died waiting for {rage} rage");
        assert!(view.health(mob.guid) > 0, "killed it before {rage} rage");
    }
    panic!("a minute of swings built no {rage} rage");
}

#[test]
fn backdraft_deals_twice_fire_blasts_roll_at_a_stunned_target() {
    let (mut plain, mut crits) = (Vec::new(), Vec::new());
    for seed in 0..60 {
        let (mut realm, _, mut view, entry) = warrior(10, seed);
        set(&mut realm, &mut view, "fire_blast", *BACKDRAFT);
        let mob = the_mob(&entry);
        stand_at(&mut realm, &mob, 15.0);
        // Charge's stun and its 10 rage, Backdraft's cost.
        realm.handle(cast(*CHARGE, Some(mob.guid)));
        assert_eq!(
            refusal(&view.drain(&mut realm), *CHARGE),
            None,
            "the charge"
        );
        realm.handle(cast(*BACKDRAFT, Some(mob.guid)));
        let events = view.drain(&mut realm);
        assert_eq!(refusal(&events, *BACKDRAFT), None, "Backdraft is cast");
        for l in hits(&events, *BACKDRAFT) {
            if l.hit_info & SPELL_HIT_TYPE_CRIT == 0 {
                plain.push(l.damage);
            } else {
                crits.push(l.damage);
            }
        }
    }
    assert!(
        !plain.is_empty() && !crits.is_empty(),
        "{plain:?} {crits:?}"
    );
    // Fire Blast's 20 + 2.5 x 10 is 45 (tests/fire_blast.rs); a crit is 150% of the 90.
    assert!(plain.iter().all(|d| *d == 90), "{plain:?}");
    assert!(crits.iter().all(|d| *d == 135), "{crits:?}");
}

#[test]
fn finisher_hits_half_again_as_hard_at_a_target_under_30_percent() {
    let (mut healthy, mut low) = (Vec::new(), Vec::new());
    for seed in 0..8 {
        let (mut realm, me, mut view, entry) = warrior(10, seed);
        set(&mut realm, &mut view, "heroic_strike", *FINISHER);
        let mob = the_mob(&entry);
        stand_at(&mut realm, &mob, 2.0);
        realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
        view.run(&mut realm, 0.1);
        let max = view.fields(mob.guid).unit_max_health().unwrap();
        for _ in 0..1200 {
            let before = view.health(mob.guid);
            if view.rage(me) >= 15 {
                realm.handle(cast(*FINISHER, Some(mob.guid)));
            }
            let events = view.run(&mut realm, 0.1);
            let after = view.health(mob.guid);
            for l in hits(&events, *FINISHER) {
                // A crit or a killing blow says nothing about the bonus.
                if l.hit_info & SPELL_HIT_TYPE_CRIT != 0 || after == 0 {
                    continue;
                }
                if before * 100 < max * 30 {
                    low.push(l.damage);
                } else {
                    healthy.push(l.damage);
                }
            }
            if after == 0 {
                break;
            }
            assert!(view.health(me) > 0, "died");
        }
    }
    assert!(
        !healthy.is_empty() && !low.is_empty(),
        "{healthy:?} {low:?}"
    );
    let most = *healthy.iter().max().unwrap();
    assert!(low.iter().all(|d| *d > most), "{healthy:?} {low:?}");
    let mean = |v: &[u32]| v.iter().sum::<u32>() as f32 / v.len() as f32;
    let ratio = mean(&low) / mean(&healthy);
    assert!((1.4..1.6).contains(&ratio), "{ratio}: {healthy:?} {low:?}");
}

#[test]
fn cleave_hits_a_second_enemy_in_reach_in_front_for_half_the_hit() {
    let (mut realm, me, mut view, entry) = warrior(10, SEED);
    set(&mut realm, &mut view, "heroic_strike", *CLEAVE);
    drop(entry);
    let west = [CAMP[0] - 25.0, CAMP[1], 0.0];
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: west,
        orientation: 0.0,
    });
    let mut camp: Vec<Mob> = view
        .run(&mut realm, 1.0)
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
    assert!(camp.len() >= 2, "the scorpid camp streams in");
    for mob in &camp {
        pull(&mut realm, &mut view, mob);
    }
    // Back west, facing east, while the camp comes to hand.
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: west,
        orientation: 0.0,
    });
    for _ in 0..300 {
        for e in view.run(&mut realm, 0.1) {
            if let SessionEvent::MonsterMove { guid, path, .. } = e {
                if let (Some(mob), Some(end)) =
                    (camp.iter_mut().find(|m| m.guid == guid), path.last())
                {
                    mob.position = *end;
                }
            }
        }
        if camp.iter().all(|m| flat(m.position, west) <= 4.0) {
            break;
        }
    }
    view.run(&mut realm, 3.0);
    assert!(view.health(me) > 0, "died waiting for the camp");
    let target = camp
        .iter()
        .find(|m| m.position[0] > west[0])
        .expect("a scorpid east of the warrior")
        .guid;
    realm.handle(RealmCommand::AttackSwing { guid: target });
    let mut logs = Vec::new();
    for _ in 0..300 {
        if view.rage(me) >= 15 {
            realm.handle(cast(*CLEAVE, Some(target)));
        }
        logs = hits(&view.run(&mut realm, 0.1), *CLEAVE);
        if !logs.is_empty() {
            break;
        }
    }
    let [first, second] = &logs[..] else {
        panic!("the hit and its cleave: {logs:?}");
    };
    assert_eq!(first.target, target);
    assert_ne!(second.target, target, "a second enemy");
    assert_eq!(second.damage, first.damage * 50 / 100);
}

#[test]
fn after_blindside_the_next_heroic_strike_crits() {
    let mut landed = Vec::new();
    for seed in 0..6 {
        // Blindside's 15 and Heroic Strike's 15.
        let (mut realm, _, mob, mut view) = raged("gouge", *BLINDSIDE, 30, seed);
        realm.handle(cast(*BLINDSIDE, Some(mob.guid)));
        let events = view.drain(&mut realm);
        assert_eq!(refusal(&events, *BLINDSIDE), None, "Blindside is cast");
        assert!(
            hits(&events, *BLINDSIDE)
                .iter()
                .all(|l| l.hit_info & SPELL_HIT_TYPE_CRIT == 0),
            "Blindside's own hit spends nothing"
        );
        realm.handle(cast(*HEROIC_STRIKE, Some(mob.guid)));
        assert_eq!(
            refusal(&view.drain(&mut realm), *HEROIC_STRIKE),
            None,
            "Heroic Strike is queued"
        );
        // The incapacitated mob is left alone until the warrior swings.
        realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
        for _ in 0..50 {
            let strikes = hits(&view.run(&mut realm, 0.1), *HEROIC_STRIKE);
            if !strikes.is_empty() {
                landed.extend(strikes);
                break;
            }
        }
    }
    assert!(!landed.is_empty(), "a Heroic Strike landed");
    assert!(
        landed.iter().all(|l| l.hit_info & SPELL_HIT_TYPE_CRIT != 0),
        "{landed:?}"
    );
}
