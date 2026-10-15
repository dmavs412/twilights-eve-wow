//! Talent Points and the Basic Warrior's Talent Tree, played through the realm's seam: the points
//! the client shows, `CMSG_LEARN_TALENT` and its refusals, what learned talents do in a fight,
//! the respec at the Crossroads' Warrior trainer, and the save.

mod common;

use benilla_protocol::{EntityKind, ObjectFields, SessionEvent};
use common::*;
use twilight_realm::DevMultipliers;

// `Talent.dbc` ids and their rank spells (`content/callings.toml`).
const IMPROVED_HEROIC_STRIKE: u32 = 124;
const IHS_RANKS: [u32; 3] = [12282, 12663, 12664];
const CRUELTY: u32 = 157;
const CRUELTY_RANKS: [u32; 3] = [12320, 12852, 12853];
const IMPROVED_REND: u32 = 127;
const IMPROVED_BATTLE_SHOUT: u32 = 154;
const IMPROVED_CHARGE: u32 = 126;
const DEEP_WOUNDS: u32 = 121;
const IMPROVED_THUNDER_CLAP: u32 = 128;
/// A real Warrior talent that is not in the Basic Warrior's tree: Mortal Strike.
const MORTAL_STRIKE: u32 = 135;

const OVERPOWER_RANKS_TALENT: u32 = 131;

// The skills (`content/callings.toml`).
const HEROIC_STRIKE: u32 = 78;
const CHARGE: u32 = 100;
const REND: u32 = 772;
const THUNDER_CLAP: u32 = 6343;
const BATTLE_SHOUT: u32 = 6673;
const OVERPOWER: u32 = 7384;
const BLOODRAGE: u32 = 2687;
/// Deep Wounds' bleed (vmangos 12721, "Deep Wound").
const DEEP_WOUND: u32 = 12721;

// vmangos `SpellModOp`.
const SPELLMOD_ALL_EFFECTS: u8 = 8;
const SPELLMOD_COST: u8 = 14;

const NO_POWER: u8 = 0x4d;

// `NPCFlags`.
const NPC_GOSSIP: u32 = 0x01;
const NPC_QUESTGIVER: u32 = 0x02;
const NPC_TRAINER: u32 = 0x10;

const SEED: u64 = 0x5EED_0014;
const CROSSROADS_XY: [f32; 2] = [-441.8, -2648.6];

#[test]
fn the_client_shows_the_tree_of_the_calling_in_the_world() {
    let mut realm = realm();
    assert!(realm.current_calling().is_none(), "nobody at select");
    enter_new(&mut realm, "Garrosh", ORC);
    let calling = realm.current_calling().expect("in the world");
    assert_eq!(calling.key, "basic_warrior");
    assert_eq!(calling.talent_tabs[0].name, "Basic Warrior");
    realm.handle(RealmCommand::Logout);
    assert!(realm.current_calling().is_none(), "back at select");
}

#[test]
fn a_new_character_has_no_talent_points() {
    let (_, me, _, view) = warrior_at(1);
    assert_eq!(view.points(me), 0);
}

#[test]
fn a_character_logs_in_with_a_point_for_every_level_from_two() {
    let (_, me, _, view) = warrior_at(6);
    assert_eq!(view.points(me), 5);
}

#[test]
fn a_level_up_brings_a_talent_point() {
    let dev = DevMultipliers {
        xp: 100.0,
        ..DevMultipliers::default()
    };
    let (mut realm, me, mob, mut view) = warrior_with(5, dev, &[]);
    assert_eq!(view.points(me), 4);
    kill(&mut realm, &mut view, &mob);
    assert_eq!(view.level(me), 6);
    assert_eq!(view.points(me), 5);
}

#[test]
fn learning_a_talent_spends_a_point_and_teaches_its_rank_spell() {
    let (mut realm, me, _, mut view) = warrior_at(4);
    learn(&mut realm, CRUELTY, 0);
    let events = view.drain(&mut realm);
    assert!(events.iter().any(
        |e| matches!(e, SessionEvent::SpellLearned { spell_id } if *spell_id == CRUELTY_RANKS[0])
    ));
    assert_eq!(view.points(me), 2);
    assert_eq!(view.talent_rank(&CRUELTY_RANKS), 1);

    learn(&mut realm, CRUELTY, 1);
    view.drain(&mut realm);
    assert_eq!(view.points(me), 1);
    assert_eq!(view.talent_rank(&CRUELTY_RANKS), 2);
}

#[test]
fn cruelty_raises_the_crit_chance_on_the_character_sheet() {
    let (mut realm, me, _, mut view) = warrior_at(4);
    let before = view.crit(me);
    learn(&mut realm, CRUELTY, 0);
    learn(&mut realm, CRUELTY, 1);
    learn(&mut realm, CRUELTY, 2);
    view.drain(&mut realm);
    assert!(
        (view.crit(me) - before - 3.0).abs() < 1e-4,
        "{before} -> {}",
        view.crit(me)
    );
}

#[test]
fn a_talent_cannot_be_learned_without_a_point() {
    let (mut realm, me, _, mut view) = warrior_at(2);
    learn(&mut realm, CRUELTY, 0);
    learn(&mut realm, CRUELTY, 1);
    view.drain(&mut realm);
    assert_eq!(view.points(me), 0);
    assert_eq!(view.talent_rank(&CRUELTY_RANKS), 1, "one point, one rank");
}

#[test]
fn only_the_callings_own_tree_can_be_learned() {
    let (mut realm, me, _, mut view) = warrior_at(10);
    learn(&mut realm, MORTAL_STRIKE, 0);
    let events = view.drain(&mut realm);
    assert!(!events
        .iter()
        .any(|e| matches!(e, SessionEvent::SpellLearned { .. })));
    assert_eq!(view.points(me), 9);
}

#[test]
fn a_talent_has_no_rank_past_its_last() {
    let (mut realm, me, _, mut view) = warrior_at(10);
    for rank in 0..4 {
        learn(&mut realm, CRUELTY, rank);
    }
    view.drain(&mut realm);
    assert_eq!(view.talent_rank(&CRUELTY_RANKS), 3);
    assert_eq!(view.points(me), 6);
}

#[test]
fn the_second_tier_opens_after_five_points_in_the_tab() {
    let (mut realm, me, _, mut view) = warrior_at(10);
    learn(&mut realm, IMPROVED_CHARGE, 0);
    view.drain(&mut realm);
    assert_eq!(view.points(me), 9, "tier 2 is shut with nothing spent");

    for rank in 0..3 {
        learn(&mut realm, CRUELTY, rank);
    }
    learn(&mut realm, IMPROVED_HEROIC_STRIKE, 0);
    learn(&mut realm, IMPROVED_CHARGE, 0);
    view.drain(&mut realm);
    assert_eq!(view.points(me), 5, "still shut at four");

    learn(&mut realm, IMPROVED_HEROIC_STRIKE, 1);
    learn(&mut realm, IMPROVED_CHARGE, 0);
    view.drain(&mut realm);
    assert_eq!(view.points(me), 3, "open at five");
    assert_eq!(view.talent_rank(&[12285, 12697]), 1);
}

#[test]
fn deep_wounds_needs_improved_rend_at_three_of_three() {
    let (mut realm, me, _, mut view) = warrior_at(10);
    for rank in 0..3 {
        learn(&mut realm, CRUELTY, rank);
    }
    learn(&mut realm, IMPROVED_REND, 0);
    learn(&mut realm, IMPROVED_REND, 1);
    learn(&mut realm, DEEP_WOUNDS, 0);
    view.drain(&mut realm);
    assert_eq!(
        view.points(me),
        4,
        "five spent, but Improved Rend is at 2/3"
    );

    learn(&mut realm, IMPROVED_REND, 2);
    learn(&mut realm, DEEP_WOUNDS, 0);
    view.drain(&mut realm);
    assert_eq!(view.points(me), 2);
    assert_eq!(view.talent_rank(&[12834, 12849, 12867]), 1);
}

// ── The respec at the Warrior trainer ───────────────────────────────────────────────────────────

#[test]
fn the_warrior_trainer_stands_in_the_crossroads() {
    let (mut realm, me, _, mut view) = warrior_at(1);
    let trainer = view.trainer();
    let fields = view.fields(trainer.guid);
    assert_eq!(
        fields.unit_npc_flags(),
        NPC_GOSSIP | NPC_QUESTGIVER | NPC_TRAINER,
        "a class trainer who talks and gives the Advancement Quests"
    );
    // Friendly to every race the player may be (`FactionTemplate.dbc` 35).
    assert_eq!(fields.unit_faction_template(), Some(35));
    let [x, y, _] = trainer.position;
    let d = ((x - CROSSROADS_XY[0]).powi(2) + (y - CROSSROADS_XY[1]).powi(2)).sqrt();
    assert!(d < 8.0, "{d} yd from where a character starts");

    let entry = fields.object_entry().unwrap();
    realm.handle(RealmCommand::CreatureQuery { entry });
    let named = view.drain(&mut realm).into_iter().find_map(|e| match e {
        SessionEvent::CreatureName { name, subname, .. } => Some((name, subname)),
        _ => None,
    });
    assert_eq!(
        named,
        Some((Some("Uzzek".into()), Some("Warrior Trainer".into())))
    );
    let _ = me;
}

#[test]
fn the_trainer_offers_to_unlearn_talents() {
    let (mut realm, _, _, mut view) = warrior_at(4);
    let trainer = view.trainer();
    realm.handle(RealmCommand::GossipHello { guid: trainer.guid });
    let (text_id, options) = view
        .drain(&mut realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu {
                npc,
                text_id,
                options,
                ..
            } if npc == trainer.guid => Some((text_id, options)),
            _ => None,
        })
        .expect("the trainer's menu");
    assert!(options
        .iter()
        .any(|o| o.message == "I wish to unlearn my talents."));

    // The greeting a menu waits on before it opens.
    realm.handle(RealmCommand::NpcTextQuery { text_id });
    let greeting = view.drain(&mut realm).into_iter().find_map(|e| match e {
        SessionEvent::NpcGreeting { text_id: t, blocks } if t == text_id => Some(blocks),
        _ => None,
    });
    let blocks = greeting.expect("the greeting");
    assert!(!blocks[0].male.is_empty());
}

#[test]
fn asking_to_unlearn_closes_the_menu_and_names_the_price() {
    let (mut realm, _, _, mut view) = warrior_at(4);
    learn(&mut realm, CRUELTY, 0);
    let trainer = view.trainer();
    let events = ask_for_respec(&mut realm, &mut view, trainer.guid);
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::GossipComplete)));
    assert_eq!(wipe_price(&events, trainer.guid), Some(10_000), "1 gold");
}

#[test]
fn with_nothing_learned_there_is_nothing_to_unlearn() {
    let (mut realm, _, _, mut view) = warrior_at(4);
    let trainer = view.trainer();
    let events = ask_for_respec(&mut realm, &mut view, trainer.guid);
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::GossipComplete)));
    assert_eq!(wipe_price(&events, trainer.guid), None);
}

#[test]
fn a_respec_costs_gold_returns_every_point_and_clears_the_talents() {
    let (mut realm, me, _, mut view) = warrior_with(
        10,
        DevMultipliers::default(),
        &[("money = 0", "money = 200000")],
    );
    let crit_before = view.crit(me);
    for rank in 0..3 {
        learn(&mut realm, CRUELTY, rank);
        learn(&mut realm, IMPROVED_HEROIC_STRIKE, rank);
    }
    view.drain(&mut realm);
    assert_eq!(view.points(me), 3);
    let trainer = view.trainer();
    ask_for_respec(&mut realm, &mut view, trainer.guid);
    realm.handle(RealmCommand::TalentWipeConfirm {
        trainer: trainer.guid,
    });
    let events = view.drain(&mut realm);

    assert_eq!(view.money(me), 190_000, "1 gold paid");
    assert_eq!(view.points(me), 9, "every point back");
    let removed: std::collections::BTreeSet<u32> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellRemoved { spell_id } => Some(*spell_id),
            _ => None,
        })
        .collect();
    let mut every_rank: Vec<u32> = CRUELTY_RANKS.into_iter().chain(IHS_RANKS).collect();
    every_rank.sort_unstable();
    assert_eq!(removed.into_iter().collect::<Vec<_>>(), every_rank);
    assert_eq!(view.talent_rank(&CRUELTY_RANKS), 0);
    assert_eq!(view.talent_rank(&IHS_RANKS), 0);
    assert_eq!(spell_mod(&events, true, 6, SPELLMOD_COST), Some(0));
    assert!(
        (view.crit(me) - crit_before).abs() < 1e-4,
        "Cruelty's crit gone"
    );

    // The next one costs 5 gold.
    learn(&mut realm, CRUELTY, 0);
    let events = ask_for_respec(&mut realm, &mut view, trainer.guid);
    assert_eq!(wipe_price(&events, trainer.guid), Some(50_000));
}

#[test]
fn a_respec_without_the_gold_changes_nothing() {
    let (mut realm, me, _, mut view) = warrior_with(
        4,
        DevMultipliers::default(),
        &[("money = 0", "money = 9999")],
    );
    learn(&mut realm, CRUELTY, 0);
    let trainer = view.trainer();
    ask_for_respec(&mut realm, &mut view, trainer.guid);
    realm.handle(RealmCommand::TalentWipeConfirm {
        trainer: trainer.guid,
    });
    view.drain(&mut realm);
    assert_eq!(view.money(me), 9999);
    assert_eq!(view.talent_rank(&CRUELTY_RANKS), 1);
    assert_eq!(view.points(me), 2);
}

#[test]
fn the_trainer_must_be_close_to_respec() {
    let (mut realm, me, mob, mut view) = warrior_with(
        4,
        DevMultipliers::default(),
        &[("money = 0", "money = 20000")],
    );
    learn(&mut realm, CRUELTY, 0);
    let trainer = view.trainer();
    ask_for_respec(&mut realm, &mut view, trainer.guid);
    // Off to the mob, out of the trainer's reach.
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::TalentWipeConfirm {
        trainer: trainer.guid,
    });
    view.drain(&mut realm);
    assert_eq!(view.money(me), 20_000);
    assert_eq!(view.talent_rank(&CRUELTY_RANKS), 1);
}

#[test]
fn the_respec_price_is_saved() {
    let (mut realm, me, _, mut view) = warrior_with(
        4,
        DevMultipliers::default(),
        &[("money = 0", "money = 20000")],
    );
    learn(&mut realm, CRUELTY, 0);
    let trainer = view.trainer();
    ask_for_respec(&mut realm, &mut view, trainer.guid);
    realm.handle(RealmCommand::TalentWipeConfirm {
        trainer: trainer.guid,
    });
    view.drain(&mut realm);
    let blob = realm.export_character(me).unwrap();

    let mut other = Realm::with_seed(SEED);
    other.import_character(&blob).unwrap();
    other.handle(RealmCommand::EnterWorld { guid: me });
    other.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut other);
    assert_eq!(view.points(me), 3);
    learn(&mut other, CRUELTY, 0);
    let events = ask_for_respec(&mut other, &mut view, trainer.guid);
    assert_eq!(wipe_price(&events, trainer.guid), Some(50_000));
}

// ── The save ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn learning_a_talent_saves_the_character() {
    let (mut realm, me, _, _) = warrior_at(4);
    realm.drain_storage();
    learn(&mut realm, CRUELTY, 0);
    let writes: Vec<_> = realm
        .drain_storage()
        .into_iter()
        .filter(|s| matches!(s, twilight_realm::Storage::Write { guid, .. } if *guid == me))
        .collect();
    assert_eq!(writes.len(), 1);
}

#[test]
fn talents_and_points_come_back_from_the_save() {
    let (mut realm, me, _, mut view) = warrior_at(6);
    for rank in 0..3 {
        learn(&mut realm, IMPROVED_HEROIC_STRIKE, rank);
    }
    learn(&mut realm, CRUELTY, 0);
    view.drain(&mut realm);
    let blob = realm.export_character(me).unwrap();

    let mut other = Realm::with_seed(SEED);
    other.import_character(&blob).unwrap();
    other.handle(RealmCommand::EnterWorld { guid: me });
    other.tick(0.0, &FlatGround);
    let mut view = View::default();
    let events = view.drain(&mut other);
    assert_eq!(view.points(me), 1);
    assert_eq!(view.talent_rank(&IHS_RANKS), 3);
    assert_eq!(view.talent_rank(&CRUELTY_RANKS), 1);
    assert_eq!(
        spell_mod(&events, true, 6, SPELLMOD_COST),
        Some(-30),
        "the client's cost comes back with the talent"
    );
    // And the stats with them.
    let (_, fresh, _, fresh_view) = warrior_at(6);
    assert!((view.crit(me) - fresh_view.crit(fresh) - 1.0).abs() < 1e-4);
}

#[test]
fn a_save_from_before_talents_loads_with_none() {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = maker.export_character(guid).unwrap();
    assert!(
        !blob.contains("talents ="),
        "nothing learned, nothing written"
    );
    let mut other = Realm::new();
    other.import_character(&blob).unwrap();
}

// ── What the talents do ─────────────────────────────────────────────────────────────────────────

#[test]
fn improved_heroic_strike_lowers_its_rage_cost_for_the_client_and_the_realm() {
    let (mut realm, me, mob, mut view) = warrior_at(10);
    for rank in 0..3 {
        learn(&mut realm, IMPROVED_HEROIC_STRIKE, rank);
    }
    let events = view.drain(&mut realm);
    // The client checks its cost before it sends: the cost cell on Heroic Strike's family bit.
    assert_eq!(
        spell_mod(&events, true, 6, SPELLMOD_COST),
        Some(-30),
        "3 rage less, in tenths"
    );

    // Bloodrage's 12 rage at level 10 buys a 12-rage strike, where a 15-rage one is refused.
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(BLOODRAGE, None));
    view.drain(&mut realm);
    assert_eq!(view.rage(me), 12);
    realm.handle(cast(HEROIC_STRIKE, Some(mob.guid)));
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = view.run(&mut realm, 0.1);
    assert_eq!(refusal(&events, HEROIC_STRIKE), None);
    assert!(went_off(&events, HEROIC_STRIKE));
}

#[test]
fn without_the_talent_heroic_strike_still_costs_fifteen() {
    let (mut realm, _, mob, mut view) = warrior_at(10);
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(BLOODRAGE, None));
    realm.handle(cast(HEROIC_STRIKE, Some(mob.guid)));
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let events = view.run(&mut realm, 0.1);
    assert_eq!(refusal(&events, HEROIC_STRIKE), Some(NO_POWER));
}

#[test]
fn improved_thunder_clap_lowers_its_rage_cost() {
    let (mut realm, me, _, mut view) = warrior_at(10);
    for rank in 0..3 {
        learn(&mut realm, CRUELTY, rank);
    }
    learn(&mut realm, IMPROVED_HEROIC_STRIKE, 0);
    learn(&mut realm, IMPROVED_HEROIC_STRIKE, 1);
    for rank in 0..3 {
        learn(&mut realm, IMPROVED_THUNDER_CLAP, rank);
    }
    let events = view.drain(&mut realm);
    assert_eq!(spell_mod(&events, true, 7, SPELLMOD_COST), Some(-40));

    // 12 rage now and one a second: 16 after four seconds, Thunder Clap's 20 less 4.
    realm.handle(cast(BLOODRAGE, None));
    view.run(&mut realm, 4.05);
    assert_eq!(view.rage(me), 16);
    realm.handle(cast(THUNDER_CLAP, None));
    let events = view.drain(&mut realm);
    assert_eq!(refusal(&events, THUNDER_CLAP), None);
    assert_eq!(view.rage(me), 0);
}

#[test]
fn improved_rend_makes_every_tick_bleed_more() {
    let ticks = |talent: bool| {
        let (mut realm, _, mob, mut view) = warrior_at(4);
        if talent {
            for rank in 0..3 {
                learn(&mut realm, IMPROVED_REND, rank);
            }
            let events = view.drain(&mut realm);
            assert_eq!(
                spell_mod(&events, false, 5, SPELLMOD_ALL_EFFECTS),
                Some(35),
                "the tooltip's +35%"
            );
        }
        stand_at(&mut realm, &mob, 2.0);
        realm.handle(cast(BLOODRAGE, None));
        realm.handle(cast(REND, Some(mob.guid)));
        periodic_damage(&view.run(&mut realm, 9.5), REND)
    };
    // 4 + 0.24 × 4 + 0.01 × 33 attack power = 5.29: 5 a tick, and 35% more is 6.75.
    assert_eq!(ticks(false), [5, 5, 5]);
    assert_eq!(ticks(true), [7, 7, 7]);
}

#[test]
fn improved_battle_shout_raises_the_shouts_attack_power() {
    let (mut realm, me, _, mut view) = warrior_at(4);
    for rank in 0..3 {
        learn(&mut realm, IMPROVED_BATTLE_SHOUT, rank);
    }
    let events = view.drain(&mut realm);
    assert_eq!(
        spell_mod(&events, false, 16, SPELLMOD_ALL_EFFECTS),
        Some(15)
    );
    realm.handle(cast(BLOODRAGE, None));
    realm.handle(cast(BATTLE_SHOUT, None));
    view.drain(&mut realm);
    // 14.5 + 0.5 × 4 = 16.5, so 17 attack power; 15% more is 19.55.
    assert_eq!(view.fields(me).unit_attack_power_mods(), (20, 0));
}

#[test]
fn improved_charge_gives_more_rage() {
    let (mut realm, me, mob, mut view) = warrior_at(10);
    for rank in 0..3 {
        learn(&mut realm, CRUELTY, rank);
    }
    learn(&mut realm, IMPROVED_HEROIC_STRIKE, 0);
    learn(&mut realm, IMPROVED_HEROIC_STRIKE, 1);
    learn(&mut realm, IMPROVED_CHARGE, 0);
    learn(&mut realm, IMPROVED_CHARGE, 1);
    view.drain(&mut realm);
    stand_at(&mut realm, &mob, 15.0);
    realm.handle(cast(CHARGE, Some(mob.guid)));
    let events = view.drain(&mut realm);
    // 8.5 + 0.14 × 10 = 9.9, so 10 rage; and 6 more.
    assert_eq!(energized(&events, CHARGE), vec![16]);
    assert_eq!(view.rage(me), 16);
}

#[test]
fn improved_overpower_makes_overpower_crit_far_more_often() {
    let mut crits = 0;
    let mut casts = 0;
    for seed in 0..3000 {
        if casts == 20 {
            break;
        }
        let Some((mut realm, _, mob, mut view)) = dodged_fight(seed, true) else {
            continue;
        };
        realm.handle(cast(OVERPOWER, Some(mob.guid)));
        let events = view.drain(&mut realm);
        if !went_off(&events, OVERPOWER) {
            continue;
        }
        casts += 1;
        if crit_damage(&events, OVERPOWER) {
            crits += 1;
        }
    }
    assert_eq!(casts, 20, "twenty Overpowers");
    // About 5% without the talent; over 55% with it.
    assert!(crits >= 6, "{crits} of {casts} crit");
}

#[test]
fn deep_wounds_bleeds_the_target_after_a_melee_crit() {
    // The first seeded fight whose crit leaves the mob standing.
    for seed in 0..100 {
        let (mut realm, me, mob, mut view) =
            warrior_seeded(10, seed, DevMultipliers::default(), &[]);
        for rank in 0..3 {
            learn(&mut realm, CRUELTY, rank);
            learn(&mut realm, IMPROVED_REND, rank);
        }
        for rank in 0..3 {
            learn(&mut realm, DEEP_WOUNDS, rank);
        }
        view.drain(&mut realm);
        assert_eq!(view.points(me), 0);
        stand_at(&mut realm, &mob, 2.0);
        realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
        let mut critted = false;
        for _ in 0..300 {
            let events = view.run(&mut realm, 0.1);
            critted = events.iter().any(|e| {
                matches!(e, SessionEvent::AttackerState(s) if s.attacker == me && s.hit_info & 0x80 != 0)
            });
            if critted || view.health(mob.guid) == 0 {
                break;
            }
        }
        if !critted || view.health(mob.guid) == 0 {
            continue;
        }
        let aura = view
            .fields(mob.guid)
            .unit_auras()
            .find(|a| a.spell_id == DEEP_WOUND);
        assert!(aura.is_some(), "the bleed on the mob's frame");
        // Hands off: what hurts the mob now is the bleed alone, a tick every 3 s for 12 s.
        realm.handle(RealmCommand::AttackStop);
        let before = view.health(mob.guid);
        let events = view.run(&mut realm, 12.5);
        let bleeds = periodic_damage(&events, DEEP_WOUND);
        if view.health(mob.guid) > 0 {
            assert_eq!(bleeds.len(), 4, "{bleeds:?}");
        }
        assert!(bleeds.iter().all(|&t| t >= 1));
        assert_eq!(
            before - view.health(mob.guid),
            bleeds.iter().sum::<u32>().min(before)
        );
        return;
    }
    panic!("no seeded fight saw a survivable crit");
}

// ── Helpers ─────────────────────────────────────────────────────────────────────────────────────

/// What the client knows: every object's fields as the packets left them, and the spells known.
#[derive(Default)]
struct View {
    objects: std::collections::HashMap<u64, ObjectFields>,
    spells: std::collections::BTreeSet<u32>,
    positions: std::collections::HashMap<u64, [f32; 3]>,
}

impl View {
    fn see(&mut self, events: &[SessionEvent]) {
        for e in events {
            match e {
                SessionEvent::ObjectCreate {
                    guid,
                    fields,
                    position,
                    ..
                } => {
                    self.positions.insert(*guid, *position);
                    self.objects.insert(*guid, fields.clone());
                }
                SessionEvent::ObjectValues { guid, fields } => {
                    self.objects.entry(*guid).or_default().merge(fields.clone());
                }
                SessionEvent::SpellBook { spell_ids, .. } => {
                    self.spells = spell_ids.iter().copied().collect();
                }
                SessionEvent::SpellLearned { spell_id } => {
                    self.spells.insert(*spell_id);
                }
                SessionEvent::SpellRemoved { spell_id } => {
                    self.spells.remove(spell_id);
                }
                _ => {}
            }
        }
    }

    fn drain(&mut self, realm: &mut Realm) -> Vec<SessionEvent> {
        let events = drain(realm);
        self.see(&events);
        events
    }

    /// Ticks `secs` in 0.1 s steps, collecting the events.
    fn run(&mut self, realm: &mut Realm, secs: f32) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        for _ in 0..(secs / 0.1).round() as u32 {
            realm.tick(0.1, &FlatGround);
            events.extend(self.drain(realm));
        }
        events
    }

    fn fields(&self, guid: u64) -> &ObjectFields {
        self.objects
            .get(&guid)
            .expect("an object the client has seen")
    }

    /// `PLAYER_CHARACTER_POINTS1`: the unspent Talent Points.
    fn points(&self, me: u64) -> u32 {
        self.fields(me).player_talent_points().unwrap_or(0)
    }

    fn level(&self, guid: u64) -> u32 {
        self.fields(guid).unit_level().unwrap()
    }

    fn health(&self, guid: u64) -> u32 {
        self.fields(guid).unit_health().unwrap()
    }

    /// The sheet's melee crit chance, percent.
    fn crit(&self, me: u64) -> f32 {
        self.fields(me).player_crit_percentage().unwrap()
    }

    fn money(&self, me: u64) -> u32 {
        self.fields(me).player_money().unwrap_or(0)
    }

    /// Whole rage points.
    fn rage(&self, guid: u64) -> u32 {
        self.fields(guid).unit_power(1).unwrap_or(0) / 10
    }

    /// A talent's rank as the stock frame reads it: the highest rank spell known.
    fn talent_rank(&self, ranks: &[u32]) -> usize {
        ranks
            .iter()
            .rposition(|s| self.spells.contains(s))
            .map_or(0, |i| i + 1)
    }
}

impl View {
    /// The trainer the client has been shown: the one unit with the trainer flag.
    fn trainer(&self) -> Mob {
        let mut trainers = self
            .objects
            .iter()
            .filter(|(_, f)| f.unit_npc_flags() & NPC_TRAINER != 0);
        let (&guid, _) = trainers.next().expect("a trainer in the world");
        assert!(trainers.next().is_none(), "one trainer");
        Mob {
            guid,
            position: self.positions[&guid],
        }
    }
}

/// Talks to the trainer and picks its unlearn line, as the stock gossip frame does.
fn ask_for_respec(realm: &mut Realm, view: &mut View, trainer: u64) -> Vec<SessionEvent> {
    realm.handle(RealmCommand::GossipHello { guid: trainer });
    let option = view
        .drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu { options, .. } => options
                .into_iter()
                .find(|o| o.message == "I wish to unlearn my talents.")
                .map(|o| o.index),
            _ => None,
        })
        .expect("the unlearn line");
    realm.handle(RealmCommand::GossipSelectOption {
        guid: trainer,
        option,
    });
    view.drain(realm)
}

/// `MSG_TALENT_WIPE_CONFIRM`'s price, if the trainer asked.
fn wipe_price(events: &[SessionEvent], trainer: u64) -> Option<u32> {
    events.iter().find_map(|e| match *e {
        SessionEvent::TalentWipeConfirm { trainer: t, cost } if t == trainer => Some(cost),
        _ => None,
    })
}

/// `CMSG_LEARN_TALENT` for the 0-based `rank`, as the stock frame sends it.
fn learn(realm: &mut Realm, talent_id: u32, rank: u32) {
    realm.handle(RealmCommand::LearnTalent { talent_id, rank });
}

#[derive(Clone)]
struct Mob {
    guid: u64,
    position: [f32; 3],
}

/// A human warrior of `level` in the world at the Crossroads, the mob near it, and the view.
fn warrior_at(level: u8) -> (Realm, u64, Mob, View) {
    warrior_with(level, DevMultipliers::default(), &[])
}

/// [`warrior_at`] with dev multipliers, and its save edited by `(from, to)` replacements first.
fn warrior_with(level: u8, dev: DevMultipliers, edits: &[(&str, &str)]) -> (Realm, u64, Mob, View) {
    warrior_seeded(level, SEED, dev, edits)
}

/// [`warrior_with`] on its own dice.
fn warrior_seeded(
    level: u8,
    seed: u64,
    dev: DevMultipliers,
    edits: &[(&str, &str)],
) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"));
    for (from, to) in edits {
        blob = blob.replace(from, to);
    }
    let mut realm = Realm::with_seed(seed).with_dev(dev);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let events = view.drain(&mut realm);
    let mob = the_mob(&events);
    (realm, guid, mob, view)
}

fn the_mob(events: &[SessionEvent]) -> Mob {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                fields,
                ..
            } if fields.unit_npc_flags() == 0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("a mob near the Crossroads")
}

/// Moves the player to `yards` from `mob`, on the Crossroads side, facing it.
fn stand_at(realm: &mut Realm, mob: &Mob, yards: f32) {
    let (dx, dy) = (
        CROSSROADS_XY[0] - mob.position[0],
        CROSSROADS_XY[1] - mob.position[1],
    );
    let len = (dx * dx + dy * dy).sqrt();
    let at = [
        mob.position[0] + dx / len * yards,
        mob.position[1] + dy / len * yards,
        mob.position[2],
    ];
    realm.handle(RealmCommand::Move {
        position: at,
        orientation: (mob.position[1] - at[1]).atan2(mob.position[0] - at[0]),
    });
}

/// Stands by `mob` and auto-attacks it until it dies, collecting the events.
fn kill(realm: &mut Realm, view: &mut View, mob: &Mob) -> Vec<SessionEvent> {
    stand_at(realm, mob, 2.0);
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut events = Vec::new();
    for _ in 0..1200 {
        events.extend(view.run(realm, 0.1));
        if view.health(mob.guid) == 0 {
            return events;
        }
    }
    panic!("the mob outlived two minutes of fighting");
}

/// A level-10 warrior on `seed`'s dice, with Improved Overpower at 2/2 when `talent`, swinging
/// at the mob until it dodges; `None` when this fight never sees a dodge both survive.
fn dodged_fight(seed: u64, talent: bool) -> Option<(Realm, u64, Mob, View)> {
    let (mut realm, me, mob, mut view) = warrior_seeded(10, seed, DevMultipliers::default(), &[]);
    if talent {
        for rank in 0..3 {
            learn(&mut realm, CRUELTY, rank);
        }
        learn(&mut realm, IMPROVED_HEROIC_STRIKE, 0);
        learn(&mut realm, IMPROVED_HEROIC_STRIKE, 1);
        learn(&mut realm, OVERPOWER_RANKS_TALENT, 0);
        learn(&mut realm, OVERPOWER_RANKS_TALENT, 1);
        view.drain(&mut realm);
        assert_eq!(view.points(me), 2);
    }
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(cast(BLOODRAGE, None));
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    for _ in 0..600 {
        realm.tick(0.1, &FlatGround);
        let events = view.drain(&mut realm);
        let dodge = events.iter().any(|e| {
            matches!(e, SessionEvent::AttackerState(s) if s.attacker == me && s.victim_state == 2)
        });
        if dodge && view.health(me) > 0 && view.health(mob.guid) > 0 {
            return Some((realm, me, mob, view));
        }
        if view.health(me) == 0 || view.health(mob.guid) == 0 {
            return None;
        }
    }
    None
}

fn cast(spell_id: u32, target: Option<u64>) -> RealmCommand {
    RealmCommand::CastSpell { spell_id, target }
}

/// The reason `spell_id` was refused, if it was.
fn refusal(events: &[SessionEvent], spell_id: u32) -> Option<u8> {
    events.iter().find_map(|e| match e {
        SessionEvent::CastResult {
            spell_id: s,
            success: false,
            reason,
            ..
        } if *s == spell_id => *reason,
        _ => None,
    })
}

/// Whether `spell_id` went off (`SMSG_SPELL_GO`).
fn went_off(events: &[SessionEvent], spell_id: u32) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::SpellGo { spell_id: s, .. } if *s == spell_id))
}

/// Whether `spell_id`'s damage log was a crit.
fn crit_damage(events: &[SessionEvent], spell_id: u32) -> bool {
    events.iter().any(|e| {
        matches!(e, SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id && l.hit_info & 0x2 != 0)
    })
}

/// The last spell-mod cell `(flat, family bit, op)` was set to.
fn spell_mod(events: &[SessionEvent], flat: bool, bit: u8, op: u8) -> Option<i32> {
    events
        .iter()
        .filter_map(|e| match *e {
            SessionEvent::SpellModifier {
                flat: f,
                mask_bit,
                op: o,
                value,
            } if (f, mask_bit, o) == (flat, bit, op) => Some(value),
            _ => None,
        })
        .last()
}

/// `SMSG_PERIODICAURALOG` damage ticks for `spell_id`.
fn periodic_damage(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PeriodicAuraLog(l) if l.spell_id == spell_id => Some(&l.ticks),
            _ => None,
        })
        .flatten()
        .filter_map(|t| match t {
            benilla_protocol::messages::PeriodicTick::Damage { amount, .. } => Some(*amount),
            _ => None,
        })
        .collect()
}

/// `SMSG_SPELLENERGIZELOG` rage for `spell_id`.
fn energized(events: &[SessionEvent], spell_id: u32) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellEnergizeLog(l) if l.spell_id == spell_id => Some(l.amount),
            _ => None,
        })
        .collect()
}
