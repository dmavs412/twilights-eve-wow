//! The Ranger's movement (#125): Vault's leap back, and the talents that read how long the Ranger
//! has been moving (Guerilla Tactics) or standing still (Steadfast Aim). Played through the
//! realm's seam by a level 10 Ranger with a Cracked Shortbow near the Crossroads; a step is a
//! `Move` to a new spot.

mod common;
mod gear_common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::{cast, refusal, stand_at, Mob, View};
use common::*;
use gear_common::save_shooting;
use twilight_realm::{Realm, RealmCommand};

const RANGED: u8 = 17;
const CRACKED_SHORTBOW: u32 = 2773;

static VAULT: std::sync::LazyLock<u32> = std::sync::LazyLock::new(|| spell("ranger", "vault"));
static SNAPSHOT: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("ranger", "snapshot"));
/// `Talent.dbc` ids, and the auras' spell ids (each rank's own).
const GUERILLA_TACTICS: u32 = 1311;
static GUERILLA_TACTICS_2: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent_spell("ranger", "guerilla_tactics", 2));
const STEADFAST_AIM: u32 = 1384;
static ENTRENCHED_1: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent_spell("ranger", "steadfast_aim", 1));

const SPELL_FAILED_NOT_READY: u8 = 0x3c;

/// A level 10 Ranger with a bow and `talents`, in the world at `seed`: the realm, its guid, the
/// first mob in view, the client's view.
fn ranger(talents: &[(u32, u8)], seed: u64) -> (Realm, u64, Mob, View) {
    let (guid, mut blob) = save_shooting(10, &[(RANGED, CRACKED_SHORTBOW, 1)]);
    blob = blob.replace("calling = \"basic_warrior\"", "calling = \"ranger\"");
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        let line = format!("money = 0\ntalents = [{}]\n", rows.join(", "));
        blob = blob.replace("money = 0\n", &line);
    }
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let mob = view
        .drain(&mut realm)
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

/// Walks east from `from` for `secs`, a step every 0.1 s, as a moving client reports.
fn walk(realm: &mut Realm, view: &mut View, from: [f32; 3], secs: f32) -> [f32; 3] {
    let mut at = from;
    for _ in 0..(secs / 0.1).round() as u32 {
        at[0] += 0.7;
        realm.handle(RealmCommand::Move {
            motion: Default::default(),
            position: at,
            orientation: 0.0,
        });
        view.run(realm, 0.1);
    }
    at
}

/// Where the player was teleported to in `events`.
fn teleported(events: &[SessionEvent]) -> Option<[f32; 3]> {
    events.iter().find_map(|e| match e {
        SessionEvent::Teleport { position, .. } => Some(*position),
        _ => None,
    })
}

#[test]
fn vault_leaps_15_yards_back_and_waits_15_seconds() {
    let (mut realm, _, _, mut view) = ranger(&[], 0xB0_0001);
    let start = [-441.8, -2648.6, 0.0];
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: start,
        orientation: 0.0,
    });
    view.run(&mut realm, 1.0);
    realm.handle(cast(*VAULT, None));
    let events = view.run(&mut realm, 0.1);
    assert_eq!(refusal(&events, *VAULT), None);
    let to = teleported(&events).expect("Vault moves the Ranger");
    // Facing east (orientation 0), so straight back is west.
    assert!((to[0] - (start[0] - 15.0)).abs() < 0.01, "to {to:?}");
    assert!((to[1] - start[1]).abs() < 0.01, "to {to:?}");

    view.run(&mut realm, 14.0);
    realm.handle(cast(*VAULT, None));
    assert_eq!(
        refusal(&view.drain(&mut realm), *VAULT),
        Some(SPELL_FAILED_NOT_READY)
    );
    view.run(&mut realm, 1.0);
    realm.handle(cast(*VAULT, None));
    assert_eq!(refusal(&view.drain(&mut realm), *VAULT), None);
}

/// Guerilla Tactics rank 2: 2 s of moving put one stack on (20%), 6 s three, and no more.
#[test]
fn guerilla_tactics_stacks_every_2_seconds_of_moving() {
    let (mut realm, me, _, mut view) = ranger(&[(GUERILLA_TACTICS, 2)], 0xB0_0002);
    let at = walk(&mut realm, &mut view, [-441.8, -2648.6, 0.0], 1.8);
    assert!(
        view.aura(me, *GUERILLA_TACTICS_2).is_none(),
        "not before 2 s"
    );
    let at = walk(&mut realm, &mut view, at, 0.4);
    let one = view.aura(me, *GUERILLA_TACTICS_2).expect("on at 2 s");
    assert_eq!(one.stacks, 1);
    walk(&mut realm, &mut view, at, 6.0);
    assert_eq!(view.aura(me, *GUERILLA_TACTICS_2).unwrap().stacks, 3);
}

/// The next Snapshot after 2 s of moving deals 20% more with rank 2, then the buff is gone. The
/// same seed with no talent is the yardstick.
#[test]
fn the_next_snapshot_after_moving_deals_more_and_uses_the_buff() {
    let mut compared = 0;
    for seed in 0..20 {
        let mut dealt = Vec::new();
        for talents in [&[][..], &[(GUERILLA_TACTICS, 2)][..]] {
            let (mut realm, me, mob, mut view) = ranger(talents, 0xB1_0000 + seed);
            walk(&mut realm, &mut view, [-441.8, -2648.6, 0.0], 2.2);
            stand_at(&mut realm, &mob, 20.0);
            view.drain(&mut realm);
            realm.handle(cast(*SNAPSHOT, Some(mob.guid)));
            let events = view.run(&mut realm, 0.1);
            dealt.push(events.iter().find_map(|e| match e {
                SessionEvent::SpellDamageLog(l) if l.spell_id == *SNAPSHOT && l.hit_info == 0 => {
                    Some(l.damage)
                }
                _ => None,
            }));
            let landed = events
                .iter()
                .any(|e| matches!(e, SessionEvent::SpellDamageLog(l) if l.spell_id == *SNAPSHOT));
            if !talents.is_empty() && landed {
                assert!(
                    view.aura(me, *GUERILLA_TACTICS_2).is_none(),
                    "Snapshot used it"
                );
            }
        }
        if let [Some(plain), Some(more)] = dealt[..] {
            let want = plain as f32 * 1.2;
            assert!(
                (more as f32 - want).abs() <= 1.5,
                "seed {seed}: {plain} with no talent, {more} with a stack"
            );
            compared += 1;
        }
    }
    assert!(compared >= 5, "enough plain hits to compare: {compared}");
}

#[test]
fn entrenched_goes_on_after_2_seconds_still_and_off_at_the_first_step() {
    let (mut realm, me, _, mut view) = ranger(&[(STEADFAST_AIM, 1)], 0xB0_0003);
    let at = walk(&mut realm, &mut view, [-441.8, -2648.6, 0.0], 0.5);
    view.run(&mut realm, 2.0);
    assert!(
        view.aura(me, *ENTRENCHED_1).is_none(),
        "not before 2 s still"
    );
    view.run(&mut realm, 1.0);
    assert!(view.aura(me, *ENTRENCHED_1).is_some(), "on after 2 s still");
    walk(&mut realm, &mut view, at, 0.1);
    assert!(
        view.aura(me, *ENTRENCHED_1).is_none(),
        "the first step takes it off"
    );
}
