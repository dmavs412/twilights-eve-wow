//! Death and the way back: dying costs a tenth of the money and of each worn item's durability,
//! and Release Spirit puts the player back in the Crossroads at full health.

mod common;
mod gear_common;

use benilla_protocol::{ObjectFields, SessionEvent};
use common::*;
use gear_common::*;
use twilight_realm::CROSSROADS;

const GNOLL_WAR_HARNESS: u32 = 1211; // 65 durability
const FINE_SCIMITAR: u32 = 4560; // 30 durability
const CHEST: u8 = 4;
const MAIN_HAND: u8 = 15;

struct Script {
    realm: Realm,
    me: u64,
    events: Vec<SessionEvent>,
}

impl Script {
    /// A level 1 warrior with 1234c, wearing the harness and the scimitar.
    fn new() -> Self {
        let (realm, me, events) = warrior_with(
            1,
            1234,
            &[(CHEST, GNOLL_WAR_HARNESS, 1), (MAIN_HAND, FINE_SCIMITAR, 1)],
        );
        Self { realm, me, events }
    }

    fn send(&mut self, command: RealmCommand) -> Vec<SessionEvent> {
        self.realm.handle(command);
        let new = drain(&mut self.realm);
        self.events.extend(new.clone());
        new
    }

    fn tick(&mut self, dt: f32) -> Vec<SessionEvent> {
        self.realm.tick(dt, &FlatGround);
        let new = drain(&mut self.realm);
        self.events.extend(new.clone());
        new
    }

    fn me(&self) -> ObjectFields {
        descriptor(&self.events, self.me)
    }

    fn durability(&self, slot: u8) -> u32 {
        let guid = self.me().player_inv_slot(slot).unwrap();
        descriptor(&self.events, guid).item_durability().unwrap()
    }

    /// Lets the nearest mob beat the warrior to death; the events of the fight.
    fn die(&mut self) -> Vec<SessionEvent> {
        let mob = the_mob(&self.events);
        stand_by(&mut self.realm, &mob);
        let mut events = self.send(RealmCommand::AttackSwing { guid: mob.guid });
        events.extend(self.send(RealmCommand::AttackStop));
        for _ in 0..12_000 {
            events.extend(self.tick(0.1));
            if self.me().unit_health() == Some(0) {
                return events;
            }
        }
        panic!("the warrior outlived twenty minutes of beating");
    }
}

#[test]
fn dying_costs_a_tenth_of_the_money_and_of_each_worn_items_durability() {
    let mut s = Script::new();
    let events = s.die();
    assert_eq!(s.me().player_money(), Some(1111), "1234c less 123c");
    assert_eq!(s.durability(CHEST), 59, "65 less 6");
    assert_eq!(s.durability(MAIN_HAND), 27, "30 less 3");
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::DurabilityDamageDeath)));
}

#[test]
fn release_spirit_respawns_the_player_in_the_crossroads_at_full_health() {
    let mut s = Script::new();
    s.die();
    let events = s.send(RealmCommand::RepopRequest);
    let (position, orientation) = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::Teleport {
                guid,
                position,
                orientation,
                ..
            } if *guid == s.me => Some((*position, *orientation)),
            _ => None,
        })
        .expect("a teleport to the Hub");
    assert_eq!(position, [CROSSROADS[0], CROSSROADS[1], CROSSROADS[2]]);
    assert_eq!(orientation, CROSSROADS[3]);
    let me = s.me();
    assert_eq!(me.unit_health(), me.unit_max_health());
    assert!(me.unit_health().unwrap() > 0);

    // Alive again, and the penalty was taken once, at death.
    s.tick(1.0);
    assert_eq!(s.me().player_money(), Some(1111));
    assert!(
        s.send(RealmCommand::RepopRequest).is_empty(),
        "only the dead release"
    );
}

#[test]
fn the_penalty_and_the_respawn_are_saved() {
    let mut s = Script::new();
    s.die();
    s.send(RealmCommand::RepopRequest);
    let saved = s
        .realm
        .drain_storage()
        .into_iter()
        .filter_map(|op| match op {
            twilight_realm::Storage::Write { blob, .. } => Some(blob),
            _ => None,
        })
        .last()
        .expect("the release saves");

    let mut fresh = Realm::new();
    fresh.import_character(&saved).unwrap();
    fresh.handle(RealmCommand::EnterWorld { guid: s.me });
    fresh.tick(0.0, &FlatGround);
    let events = drain(&mut fresh);
    let me = descriptor(&events, s.me);
    assert_eq!(me.player_money(), Some(1111));
    let chest = me.player_inv_slot(CHEST).unwrap();
    assert_eq!(descriptor(&events, chest).item_durability(), Some(59));
    let at = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::Worldport { position, .. } => Some(*position),
            _ => None,
        })
        .unwrap();
    assert_eq!(at, [CROSSROADS[0], CROSSROADS[1], CROSSROADS[2]]);
}

#[test]
fn logging_out_dead_comes_back_alive_in_the_crossroads() {
    let mut s = Script::new();
    s.die();
    s.send(RealmCommand::Logout);
    let me = s.me;
    s.events = s.send(RealmCommand::EnterWorld { guid: me });
    let at = s
        .events
        .iter()
        .find_map(|e| match e {
            SessionEvent::Worldport { position, .. } => Some(*position),
            _ => None,
        })
        .unwrap();
    assert_eq!(at, [CROSSROADS[0], CROSSROADS[1], CROSSROADS[2]]);
    assert_eq!(s.me().player_money(), Some(1111), "no second toll");
}
