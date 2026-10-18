//! Padfoot's End: Marshal McBride's once-per-character bounty on the elite Garrick Padfoot, which
//! pays one weapon of four, the player's pick.

mod common;
mod gear_common;
mod quest_common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::*;
use quest_common::*;

const HIGHGUID_UNIT: u64 = 0xF130;
const MCBRIDE: u64 = HIGHGUID_UNIT << 48 | 197 << 24 | 79970;
const MCBRIDE_AT: [f32; 3] = [-8912.0, -140.5, 82.0];
const PADFOOTS_END: u32 = 94003;
/// The staff, the dagger, the bow and the two-handed axe.
const CHOICES: [u32; 4] = [900101, 900102, 900103, 900104];

struct Floor;

impl Host for Floor {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(82.0)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// A level 10 human warrior beside McBride with Garrick already dead: Padfoot's End done but not
/// handed in.
fn ready_to_hand_in() -> (Realm, u64) {
    let mut maker = realm();
    let RealmCommand::CreateCharacter(mut req) = create_cmd("Pilgrim", HUMAN, 0, WARRIOR) else {
        unreachable!()
    };
    req.start = START_ELWYNN;
    maker.handle(RealmCommand::CreateCharacter(req));
    let me = roster(&mut maker)[0].guid;
    let blob = maker
        .export_character(me)
        .unwrap()
        .replace("level = 1\n", "level = 10\n")
        + &format!("\n[[quests]]\nid = {PADFOOTS_END}\nslot = 0\nkills = [1, 0, 0, 0]\n");
    let mut realm = Realm::with_seed(1);
    realm
        .import_character(&blob)
        .unwrap_or_else(|e| panic!("{e:?}: {blob}"));
    realm.handle(RealmCommand::EnterWorld { guid: me });
    realm.tick(0.0, &Floor);
    realm.handle(RealmCommand::DevTeleport {
        x: MCBRIDE_AT[0] + 2.0,
        y: MCBRIDE_AT[1],
        facing: None,
    });
    realm.tick(0.1, &Floor);
    drain(&mut realm);
    (realm, me)
}

fn choose(realm: &mut Realm, choice: u32) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::QuestGiverCompleteQuest {
        npc: MCBRIDE,
        quest: PADFOOTS_END,
    });
    realm.handle(RealmCommand::QuestGiverRequestReward {
        npc: MCBRIDE,
        quest: PADFOOTS_END,
    });
    realm.handle(RealmCommand::QuestGiverChooseReward {
        npc: MCBRIDE,
        quest: PADFOOTS_END,
        choice,
    });
    drain(realm)
}

#[test]
fn the_reward_panel_offers_all_four_weapons() {
    let (mut realm, _) = ready_to_hand_in();
    realm.handle(RealmCommand::QuestGiverCompleteQuest {
        npc: MCBRIDE,
        quest: PADFOOTS_END,
    });
    let offer = drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::QuestOffer(o) => Some(o),
            _ => None,
        })
        .expect("the reward panel");
    let offered: Vec<u32> = offer.choices.iter().map(|c| c.item_id).collect();
    assert_eq!(offered, CHOICES);
}

#[test]
fn handing_in_pays_the_chosen_weapon_once() {
    let (mut realm, _) = ready_to_hand_in();
    let done = completed(&choose(&mut realm, 2)).expect("the hand-in went through");
    assert!(
        done.items.contains(&(CHOICES[2], 1)),
        "the bow: {:?}",
        done.items
    );
    assert!(!done
        .items
        .iter()
        .any(|&(i, _)| i == CHOICES[0] || i == CHOICES[1]));
    // Once per character: a second hand-in pays nothing.
    assert!(completed(&choose(&mut realm, 3)).is_none());
}

#[test]
fn a_choice_past_the_list_is_refused() {
    let (mut realm, _) = ready_to_hand_in();
    assert!(completed(&choose(&mut realm, 4)).is_none());
    // The quest is still there to hand in properly.
    assert!(completed(&choose(&mut realm, 0)).is_some());
}
