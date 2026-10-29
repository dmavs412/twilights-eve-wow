//! The Ranger's movement (#125): Vault's leap back and its cooldown. Played through the realm's
//! seam by a level 10 Ranger with a Cracked Shortbow near the Crossroads.

mod common;
mod gear_common;

use benilla_protocol::SessionEvent;
use common::fight::{cast, refusal, View};
use common::*;
use gear_common::save_shooting;
use twilight_realm::{Realm, RealmCommand};

const RANGED: u8 = 17;
const CRACKED_SHORTBOW: u32 = 2773;

static VAULT: std::sync::LazyLock<u32> = std::sync::LazyLock::new(|| spell("ranger", "vault"));

const SPELL_FAILED_NOT_READY: u8 = 0x3c;

/// A level 10 Ranger with a bow, in the world at `seed`: the realm and the client's view.
fn ranger(seed: u64) -> (Realm, View) {
    let (guid, blob) = save_shooting(10, &[(RANGED, CRACKED_SHORTBOW, 1)]);
    let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"ranger\"");
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    (realm, view)
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
    let (mut realm, mut view) = ranger(0xB0_0001);
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
