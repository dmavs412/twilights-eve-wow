//! The Ranger's Field Dressing (#295): a self cast that heals 20% of maximum health over 10 s in
//! five ticks of 4%, rounded down, usable in combat, on a 45 s cooldown. Played through the
//! realm's seam by a level 10 Ranger hurt by a camp scorpid.

mod common;
mod gear_common;

use benilla_protocol::messages::PeriodicTick;
use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::{cast, refusal, stand_at, Mob, View};
use common::*;
use gear_common::save_shooting;
use twilight_realm::{Realm, RealmCommand};

const RANGED: u8 = 17;
const CRACKED_SHORTBOW: u32 = 2773;
const SPELL_FAILED_NOT_READY: u8 = 0x3c;

static FIELD_DRESSING: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| spell("ranger", "field_dressing"));

/// A level 6 Ranger left standing 2 yd from the first creature near the Crossroads for 40 s: the
/// realm, their guid and the view.
fn hurt_ranger() -> (Realm, u64, View) {
    let (guid, mut blob) = save_shooting(6, &[(RANGED, CRACKED_SHORTBOW, 1)]);
    blob = blob.replace("calling = \"basic_warrior\"", "calling = \"ranger\"");
    let mut realm = Realm::with_seed(0x5EED_0056);
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
    stand_at(&mut realm, &mob, 2.0);
    view.run(&mut realm, 40.0);
    (realm, guid, view)
}

/// The amounts of `spell_id`'s heal ticks on `me` in `events`.
fn heal_ticks(events: &[SessionEvent], spell_id: u32, me: u64) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == spell_id && l.target == me => {
                match l.ticks[0] {
                    PeriodicTick::Heal { amount } => Some(amount),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect()
}

#[test]
fn field_dressing_heals_four_percent_of_max_health_five_times_in_combat() {
    let (mut realm, me, mut view) = hurt_ranger();
    let max = view.fields(me).unit_max_health().unwrap();
    assert!(
        view.health(me) + 4 * max / 100 < max,
        "the scorpid hurt the Ranger: {} of {max}",
        view.health(me)
    );
    realm.handle(cast(*FIELD_DRESSING, None));
    let first = view.drain(&mut realm);
    assert_eq!(refusal(&first, *FIELD_DRESSING), None);
    let mut ticks = heal_ticks(&first, *FIELD_DRESSING, me);
    ticks.extend(heal_ticks(&view.run(&mut realm, 10.1), *FIELD_DRESSING, me));
    assert_eq!(
        ticks,
        vec![max * 4 / 100; 5],
        "five ticks of 4%, rounded down"
    );
}

#[test]
fn field_dressing_is_on_cooldown_for_45_seconds() {
    let (mut realm, me, mut view) = hurt_ranger();
    realm.handle(cast(*FIELD_DRESSING, None));
    assert_eq!(refusal(&view.drain(&mut realm), *FIELD_DRESSING), None);
    view.run(&mut realm, 20.0);
    assert!(view.health(me) > 0, "the Ranger lived");
    realm.handle(cast(*FIELD_DRESSING, None));
    assert_eq!(
        refusal(&view.drain(&mut realm), *FIELD_DRESSING),
        Some(SPELL_FAILED_NOT_READY),
        "inside 45 s"
    );
}
