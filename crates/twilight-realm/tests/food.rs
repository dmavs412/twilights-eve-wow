//! Food and drink: used from the bags out of combat, the character sits and is restored over the
//! item's duration; combat, a hit or a step ends it (vmangos `SPELL_AURA_MOD_REGEN`).

mod common;
mod gear_common;

use benilla_protocol::{ObjectFields, SessionEvent};
use common::*;
use gear_common::*;

const TOUGH_JERKY: u32 = 117; // 61 health over 18 s
const REFRESHING_SPRING_WATER: u32 = 159; // 151 mana over 18 s
const FOOD: u32 = 433; // the spell Tough Jerky casts
const WILD_HOG_SHANK: u32 = 3771; // required level 25
const PACK_1: u8 = 23;

struct Script {
    realm: Realm,
    me: u64,
    events: Vec<SessionEvent>,
}

impl Script {
    /// A level 10 warrior with five pieces of jerky.
    fn new() -> Self {
        let (realm, me, events) = warrior_with(
            10,
            0,
            &[(PACK_1, TOUGH_JERKY, 5), (24, REFRESHING_SPRING_WATER, 2)],
        );
        Self { realm, me, events }
    }

    fn send(&mut self, command: RealmCommand) -> Vec<SessionEvent> {
        self.realm.handle(command);
        let new = drain(&mut self.realm);
        self.events.extend(new.clone());
        new
    }

    fn run(&mut self, secs: f32) {
        for _ in 0..(secs * 10.0).round() as u32 {
            self.realm.tick(0.1, &FlatGround);
            let new = drain(&mut self.realm);
            self.events.extend(new);
        }
    }

    fn me(&self) -> ObjectFields {
        descriptor(&self.events, self.me)
    }

    fn health(&self) -> u32 {
        self.me().unit_health().unwrap()
    }

    fn eat(&mut self) -> Vec<SessionEvent> {
        self.send(RealmCommand::UseItem {
            bag: 255,
            slot: PACK_1,
        })
    }

    fn jerky(&self) -> u32 {
        let guid = self.me().player_pack_slot(0).unwrap();
        descriptor(&self.events, guid).item_stack_count().unwrap()
    }

    /// Lets the nearest mob's camp beat on the warrior for `secs`, then steps well away until
    /// they give up: hurt, and out of combat.
    fn get_hurt(&mut self, secs: f32) {
        let mob = the_mob(&self.events);
        stand_by(&mut self.realm, &mob);
        self.send(RealmCommand::AttackSwing { guid: mob.guid });
        self.send(RealmCommand::AttackStop);
        self.run(secs);
        let me = self.me();
        assert!(me.unit_health() < me.unit_max_health());
        // Toward the Crossroads and on past it: past every leash.
        let hub = twilight_realm::CROSSROADS;
        let (dx, dy) = (hub[0] - mob.position[0], hub[1] - mob.position[1]);
        let len = (dx * dx + dy * dy).sqrt();
        let away = [
            mob.position[0] + dx / len * 90.0,
            mob.position[1] + dy / len * 90.0,
            mob.position[2],
        ];
        self.send(RealmCommand::Move {
            motion: Default::default(),
            position: away,
            orientation: 0.0,
        });
        // Stop at the first out-of-combat moment: natural regeneration starts there.
        for _ in 0..300 {
            if self.me().unit_flags() & 0x0008_0000 == 0 {
                break;
            }
            self.run(0.1);
        }
        assert_eq!(self.me().unit_flags() & 0x0008_0000, 0, "out of combat");
    }
}

fn eating(me: &ObjectFields) -> bool {
    me.unit_aura_ids().any(|id| id == FOOD)
}

#[test]
fn food_restores_its_health_over_its_duration_while_sitting() {
    let mut s = Script::new();
    s.get_hurt(30.0);
    let max = s.me().unit_max_health().unwrap();
    let before = s.health();
    assert!(
        max - before >= 61,
        "hurt by more than a meal: {before} of {max}"
    );

    s.eat();
    assert_eq!(s.jerky(), 4, "one piece eaten");
    assert!(eating(&s.me()), "the Food aura");
    assert_eq!(s.me().unit_stand_state(), 1, "sitting");

    s.run(18.0);
    // Tough Jerky: 61 health over 18 s, on top of the natural regeneration out of combat.
    let spirit = s.me().unit_stat(4).unwrap();
    let natural = 10 * twilight_realm::rules::regen::tick_heal(spirit, 0);
    let gained = s.health() - before;
    assert!(
        (61..=61 + natural).contains(&gained),
        "the meal's 61 plus at most {natural}: {gained}"
    );
    assert!(!eating(&s.me()), "done");
    assert_eq!(s.me().unit_stand_state(), 0, "standing again");
}

#[test]
fn food_above_the_characters_level_can_be_eaten() {
    let (mut realm, me, mut events) = warrior_with(1, 0, &[(PACK_1, WILD_HOG_SHANK, 2)]);
    realm.handle(RealmCommand::UseItem {
        bag: 255,
        slot: PACK_1,
    });
    let eaten = drain(&mut realm);
    assert!(
        !eaten
            .iter()
            .any(|e| matches!(e, SessionEvent::InventoryFailure { .. })),
        "{eaten:?}"
    );
    events.extend(eaten);
    let f = descriptor(&events, me);
    assert!(
        f.unit_aura_ids().any(|id| id == 1127),
        "Wild Hog Shank's Food"
    );
    let shank = f.player_pack_slot(0).unwrap();
    assert_eq!(descriptor(&events, shank).item_stack_count(), Some(1));
}

#[test]
fn eating_in_combat_is_refused() {
    let mut s = Script::new();
    let mob = the_mob(&s.events);
    stand_by(&mut s.realm, &mob);
    s.send(RealmCommand::AttackSwing { guid: mob.guid });
    s.run(0.1);
    let events = s.eat();
    let refusal = events.iter().find_map(|e| match e {
        SessionEvent::CastResult {
            spell_id,
            success: false,
            reason,
            ..
        } => Some((*spell_id, *reason)),
        _ => None,
    });
    assert_eq!(
        refusal,
        Some((FOOD, Some(0))),
        "SPELL_FAILED_AFFECTING_COMBAT"
    );
    assert_eq!(s.jerky(), 5);
    assert!(!eating(&s.me()));
}

#[test]
fn standing_up_ends_the_meal() {
    let mut s = Script::new();
    s.get_hurt(30.0);
    let before = s.health();
    s.eat();
    s.run(4.0);
    let mid = s.health();
    assert!(mid > before, "some of it went down");
    let mut at = twilight_realm::CROSSROADS;
    at[0] += 3.0;
    s.send(RealmCommand::Move {
        motion: Default::default(),
        position: [at[0], at[1], at[2]],
        orientation: 0.0,
    });
    s.run(0.1);
    assert!(!eating(&s.me()));
    s.run(14.0);
    let spirit = s.me().unit_stat(4).unwrap();
    let natural = 8 * twilight_realm::rules::regen::tick_heal(spirit, 0);
    assert!(
        s.health() <= mid + natural,
        "no more meal once up, only the natural regeneration"
    );
}

#[test]
fn a_warrior_can_drink_but_has_no_mana_to_restore() {
    let mut s = Script::new();
    s.send(RealmCommand::UseItem { bag: 255, slot: 24 });
    let guid = s.me().player_pack_slot(1).unwrap();
    assert_eq!(descriptor(&s.events, guid).item_stack_count(), Some(1));
    s.run(18.0);
    assert_eq!(s.me().unit_power_type(), 1, "still rage");
}

#[test]
fn the_tooltip_names_the_food_spell_and_the_bar_counts_the_stack() {
    let mut s = Script::new();
    let events = s.send(RealmCommand::ItemQuery { entry: TOUGH_JERKY });
    let info = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ItemTemplate { info, .. } => info.clone(),
            _ => None,
        })
        .expect("Tough Jerky's template");
    assert_eq!(info.name, "Tough Jerky");
    assert_eq!(info.use_spell.map(|u| u.spell_id), Some(FOOD));
    assert_eq!(info.use_spell_index(), Some(0));
    assert!(
        info.is_consumable(),
        "used up, so the action bar shows the count"
    );
    assert_eq!(
        (info.buy_price, info.sell_price, info.stackable),
        (25, 1, 20)
    );
}
