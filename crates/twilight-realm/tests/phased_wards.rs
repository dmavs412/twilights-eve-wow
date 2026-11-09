//! Ward stones, creatures and GameObjects that appear after something, and timed calls for help
//! (#242), and levers (#577), tried on the Ward Test (`content/ward_test.toml`): a Phased Dungeon
//! with two stops, "water" (open) and "fire" (dark until the ward stone at "water" is used).

mod common;

use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

/// `HIGHGUID_GAMEOBJECT` and `HIGHGUID_UNIT`, in a guid's top 16 bits.
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
const HIGHGUID_UNIT: u64 = 0xF130;
/// The Corrupted Brother every creature of the Ward Test is.
const BROTHER: u64 = 96003;
/// The first low guid of a creature a timed call has brought in.
const TIMED_LIVE_BASE: u64 = 0xC0_0000;

fn unit(low: u64) -> u64 {
    HIGHGUID_UNIT << 48 | BROTHER << 24 | low
}

fn object(entry: u64) -> u64 {
    HIGHGUID_GAMEOBJECT << 48 | entry
}

/// The water stop's guard, and the creatures that come after it or its stop.
const GUARD: u64 = 800001;
const AFTER_PINNED: u64 = 800002;
const AFTER_DEAD: u64 = 800003;
/// The creature that calls two adds every 20 seconds of a fight.
const CALLER: u64 = 800004;
/// The ward stone, and the scenery that appears after the stop is pinned, after the guard dies,
/// and stands until it dies.
const WARD: u64 = 810001;
const SCENERY_PINNED: u64 = 810002;
const SCENERY_DEAD: u64 = 810003;
const SCENERY_UNTIL: u64 = 810004;
/// The inner portals to "water" and to "fire": `0x0F000000 | dungeon << 8 | stop << 1 | side`.
const INNER_WATER: u64 = HIGHGUID_GAMEOBJECT << 48 | 0x0F00_0000 | 90 << 8;
const INNER_FIRE: u64 = INNER_WATER | 2;

const HELD: &str = "Gresh's magic holds the stone.";
/// In the building by the inner portals, at the water stop by the guard and by the ward stone,
/// and in the building by the creature that calls.
const BY_INNER_WATER: [f32; 3] = [-9790.0, 565.0, 47.2];
const BY_INNER_FIRE: [f32; 3] = [-9790.0, 545.0, 47.2];
const BY_GUARD: [f32; 3] = [-9688.5, 545.0, 47.2];
const BY_WARD: [f32; 3] = [-9692.0, 560.0, 47.2];
const BY_CALLER: [f32; 3] = [-9808.5, 570.0, 47.2];
const GUARD_AT: [f32; 3] = [-9690.0, 545.0, 47.2];
const CALLER_AT: [f32; 3] = [-9810.0, 570.0, 47.2];
/// Where the inner portals put a character back, and where "fire" lands one.
const BACK_WATER: [f32; 3] = [-9788.0, 565.0, 47.2];
const ARRIVAL_FIRE: [f32; 3] = [-9590.0, 555.0, 47.2];
/// The lever at the water stop, which holds until the guard dies; the scenery and creatures that
/// come after its pull and stand until it; and a spot in its reach.
const LEVER: u64 = 810005;
const SCENERY_AFTER_PULL: u64 = 810006;
const SCENERY_UNTIL_PULL: u64 = 810007;
const AFTER_PULL: u64 = 800005;
const UNTIL_PULL: u64 = 800006;
const BY_LEVER: [f32; 3] = [-9684.0, 560.0, 47.2];
const LEVER_HELD: &str = "The guard's chain holds the lever.";
const LEVER_USED: &str = "The bars drop.";
/// A quest that wants one use of the ward stone, and one that wants a pull of the lever.
const TOUCH_THE_WARD: u32 = 99302;
const PULL_THE_LEVER: u32 = 99303;

const TEST_QUESTS: &str = r#"
[[quest]]
id = 99302
kind = "story"
giver = 197
level = 5
title = "Touch the Ward"
objectives = "Use the ward stone."
details = "-"
progress = "-"
completion = "-"
money = 0
use = [[810001, 1]]

[[quest]]
id = 99303
kind = "story"
giver = 197
level = 5
title = "Pull the Lever"
objectives = "Pull the lever."
details = "-"
progress = "-"
completion = "-"
money = 0
use = [[810005, 1]]
"#;

/// `blob` with the quests to use the ward stone and pull the lever under way.
fn with_quest(blob: String) -> String {
    format!(
        "{blob}\n[[quests]]\nid = {TOUCH_THE_WARD}\nslot = 0\nkills = [0, 0, 0, 0]\n\
         \n[[quests]]\nid = {PULL_THE_LEVER}\nslot = 1\nkills = [0, 0, 0, 0]\n"
    )
}

/// Level 60 warriors: the quests under way for the first two.
fn party(n: usize) -> Party {
    let realm = Realm::with_seed(1).with_quests(TEST_QUESTS);
    let mut p = Party::edited(
        realm,
        &vec![60; n],
        START_ELWYNN,
        Box::new(NoGround),
        |i, blob| if i < 2 { with_quest(blob) } else { blob },
    );
    p.run(1.0);
    p
}

fn holds(p: &Party, i: usize, guid: u64) -> bool {
    p.units_held(i).contains(&guid)
}

/// Whether player `i`'s client holds the GameObject `guid`, by what it has been sent.
fn shows(p: &Party, i: usize, guid: u64) -> bool {
    let mut shown = false;
    for e in p.seen(i) {
        match e {
            SessionEvent::ObjectCreate { guid: g, .. } if *g == guid => shown = true,
            SessionEvent::ObjectsRemoved(gone) if gone.contains(&guid) => shown = false,
            SessionEvent::ObjectDestroyed(g) if *g == guid => shown = false,
            _ => {}
        }
    }
    shown
}

fn teleports(p: &Party, i: usize, from: usize) -> Vec<[f32; 3]> {
    p.seen(i)[from..]
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Teleport { position, .. } => Some(*position),
            _ => None,
        })
        .collect()
}

fn ack(p: &mut Party, i: usize) {
    let counter = p
        .seen(i)
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::Teleport { counter, .. } => Some(*counter),
            _ => None,
        })
        .expect("a teleport to answer");
    p.cmd(i, RealmCommand::TeleportAck { counter });
}

fn held_notes(p: &Party, i: usize) -> usize {
    notes(p, i, HELD)
}

/// The notifications saying `said` that player `i` has been shown.
fn notes(p: &Party, i: usize, said: &str) -> usize {
    p.seen(i)
        .iter()
        .filter(|e| matches!(e, SessionEvent::Notification { text } if text == said))
        .count()
}

/// Objective credits player `i` has been shown for the ward stone's quest.
fn credits(p: &Party, i: usize) -> usize {
    quest_credits(p, i, TOUCH_THE_WARD)
}

/// Objective credits player `i` has been shown for `quest`.
fn quest_credits(p: &Party, i: usize, quest: u32) -> usize {
    p.seen(i)
        .iter()
        .filter(|e| {
            matches!(e, SessionEvent::QuestObjectiveKill { quest_id, .. }
                if *quest_id == quest)
        })
        .count()
}

/// Player `i` stands at `at` facing `toward` and fights the creature `target` until it dies.
fn kill(p: &mut Party, i: usize, at: [f32; 3], toward: [f32; 3], target: u64) {
    p.go_facing(i, at, toward);
    p.run(0.2);
    p.cmd(i, RealmCommand::AttackSwing { guid: target });
    for _ in 0..120 {
        p.run(0.5);
        if died(p.seen(i), target) {
            return;
        }
    }
    let health = p.seen(i).iter().rev().find_map(|e| match e {
        SessionEvent::ObjectValues { guid, fields, .. } if *guid == target => fields.unit_health(),
        _ => None,
    });
    panic!("{target:#x} did not die, last health {health:?}");
}

fn click(p: &mut Party, i: usize, guid: u64) {
    p.cmd(i, RealmCommand::UseGameObject { guid });
    p.run(0.3);
}

#[test]
fn the_ward_stone_does_nothing_while_its_creature_lives() {
    let mut p = party(2);
    p.go(0, BY_WARD, 0.0);
    p.go(1, BY_INNER_WATER, 0.0);
    p.run(0.5);
    assert!(shows(&p, 0, object(WARD)), "the stone is not shown");
    let from = [p.seen(0).len(), p.seen(1).len()];
    click(&mut p, 0, object(WARD));
    assert_eq!(held_notes(&p, 0), 1, "no held text");
    assert!(teleports(&p, 0, from[0]).is_empty(), "sent back");
    assert_eq!(credits(&p, 0), 0);
    assert!(teleports(&p, 1, from[1]).is_empty());
}

#[test]
fn after_the_kill_a_ward_stone_sends_the_stop_home_darkens_it_opens_the_next_and_credits_each() {
    let mut p = party(3);
    p.go(0, BY_WARD, 0.0);
    p.go(1, [-9695.0, 570.0, 47.2], 0.0);
    p.go(2, BY_INNER_WATER, 0.0);
    p.run(0.5);
    kill(&mut p, 0, BY_GUARD, GUARD_AT, unit(GUARD));
    p.go(0, BY_WARD, 0.0);
    p.run(0.2);

    let from = [p.seen(0).len(), p.seen(1).len(), p.seen(2).len()];
    click(&mut p, 0, object(WARD));
    assert_eq!(held_notes(&p, 0), 0, "held after the kill");
    for i in [0, 1] {
        assert_eq!(
            teleports(&p, i, from[i]),
            [BACK_WATER],
            "player {i} not sent back"
        );
        assert_eq!(credits(&p, i), 1, "player {i} credit");
    }
    assert!(
        teleports(&p, 2, from[2]).is_empty(),
        "the one in the building moved"
    );
    assert_eq!(credits(&p, 2), 0);

    // The portal to "water" is dark, the one to "fire" is open.
    ack(&mut p, 0);
    p.go(0, BY_INNER_WATER, 0.0);
    p.run(0.3);
    let from = p.seen(0).len();
    click(&mut p, 0, INNER_WATER);
    assert!(
        teleports(&p, 0, from).is_empty(),
        "the pinned stop took them"
    );
    p.go(0, BY_INNER_FIRE, 0.0);
    p.run(0.3);
    let from = p.seen(0).len();
    click(&mut p, 0, INNER_FIRE);
    assert_eq!(teleports(&p, 0, from), [ARRIVAL_FIRE], "fire did not open");
}

#[test]
fn a_second_click_on_a_pinned_ward_stone_does_nothing() {
    let mut p = party(2);
    p.go(0, BY_WARD, 0.0);
    p.run(0.5);
    kill(&mut p, 0, BY_GUARD, GUARD_AT, unit(GUARD));
    p.go(0, BY_WARD, 0.0);
    p.run(0.2);
    click(&mut p, 0, object(WARD));
    ack(&mut p, 0);
    p.go(0, BY_WARD, 0.0);
    p.run(0.3);
    let from = p.seen(0).len();
    click(&mut p, 0, object(WARD));
    assert!(teleports(&p, 0, from).is_empty(), "sent back twice");
    assert_eq!(credits(&p, 0), 1, "credited twice");
}

#[test]
fn things_appear_after_their_condition_and_leave_when_their_creature_dies() {
    let mut p = party(3);
    p.go(0, BY_INNER_FIRE, 0.0);
    p.go(1, BY_WARD, 0.0);
    p.run(1.0);
    assert!(!holds(&p, 0, unit(AFTER_PINNED)), "after_pinned creature");
    assert!(!holds(&p, 0, unit(AFTER_DEAD)), "after_dead creature");
    assert!(!shows(&p, 0, object(SCENERY_PINNED)));
    assert!(!shows(&p, 0, object(SCENERY_DEAD)));
    assert!(shows(&p, 0, object(SCENERY_UNTIL)), "until_dead scenery");

    kill(&mut p, 1, BY_GUARD, GUARD_AT, unit(GUARD));
    p.run(0.5);
    assert!(holds(&p, 0, unit(AFTER_DEAD)), "after_dead creature absent");
    assert!(shows(&p, 0, object(SCENERY_DEAD)), "after_dead scenery");
    assert!(!shows(&p, 0, object(SCENERY_UNTIL)), "until_dead scenery");
    assert!(!holds(&p, 0, unit(AFTER_PINNED)), "before the pin");
    assert!(!shows(&p, 0, object(SCENERY_PINNED)), "before the pin");

    p.go(1, BY_WARD, 0.0);
    p.run(0.2);
    click(&mut p, 1, object(WARD));
    p.run(0.5);
    assert!(holds(&p, 0, unit(AFTER_PINNED)), "after_pinned creature");
    assert!(shows(&p, 0, object(SCENERY_PINNED)), "after_pinned scenery");
}

#[test]
fn a_run_that_resets_puts_them_back_as_they_were() {
    let mut p = party(2);
    p.go(0, BY_INNER_FIRE, 0.0);
    p.go(1, BY_WARD, 0.0);
    p.run(1.0);
    kill(&mut p, 1, BY_GUARD, GUARD_AT, unit(GUARD));
    p.run(0.5);
    assert!(holds(&p, 0, unit(AFTER_DEAD)));
    // Everyone leaves (a stop's edge only drags back to the building), the Run resets, and the
    // next one starts as the first did.
    p.go(1, BY_INNER_FIRE, 0.0);
    p.run(0.5);
    p.go(0, [-9900.0, 555.0, 47.2], 0.0);
    p.go(1, [-9900.0, 555.0, 47.2], 0.0);
    p.run(1.0);
    let n = p.seen(0).len();
    p.go(0, BY_INNER_FIRE, 0.0);
    p.run(1.0);
    let made: Vec<u64> = unit_creates(&p.seen(0)[n..])
        .into_iter()
        .map(|(guid, _, _)| guid)
        .collect();
    assert!(!made.contains(&unit(AFTER_DEAD)), "made {made:x?}");
    assert!(holds(&p, 0, unit(GUARD)), "the guard is back");
    assert!(shows(&p, 0, object(SCENERY_UNTIL)));
}

/// Creatures a timed call has brought in that player `i` has been created.
fn adds(p: &Party, i: usize) -> Vec<u64> {
    unit_creates(p.seen(i))
        .into_iter()
        .map(|(guid, _, _)| guid)
        .filter(|g| g & 0xFF_FFFF >= TIMED_LIVE_BASE)
        .collect()
}

#[test]
fn a_creature_with_timed_calls_brings_its_adds_on_the_clock_and_they_go_when_it_dies() {
    let mut p = party(1);
    p.go_facing(0, BY_CALLER, CALLER_AT);
    p.run(0.5);
    assert!(adds(&p, 0).is_empty());
    p.cmd(0, RealmCommand::AttackSwing { guid: unit(CALLER) });
    p.run(0.2);
    p.cmd(0, RealmCommand::AttackStop);
    p.run(17.0);
    assert!(adds(&p, 0).is_empty(), "a call before its time");
    p.run(5.0);
    assert_eq!(adds(&p, 0).len(), 2, "no call at 20 s: {:?}", adds(&p, 0));
    p.run(18.0);
    assert_eq!(adds(&p, 0).len(), 4, "no second call at 40 s");
    let brought = adds(&p, 0);
    assert!(
        brought.iter().all(|g| holds(&p, 0, *g)),
        "an add is not in view"
    );
    let engaged = p.seen(0).iter().any(
        |e| matches!(e, SessionEvent::AttackStart { attacker, .. } if brought.contains(attacker)),
    );
    assert!(engaged, "the adds did not attack");

    kill(&mut p, 0, BY_CALLER, CALLER_AT, unit(CALLER));
    p.run(0.5);
    for g in &brought {
        assert!(!holds(&p, 0, *g), "{g:#x} outlived its caller");
    }
}

/// Player `i` kills the guard and walks back to the lever.
fn free_the_lever(p: &mut Party, i: usize) {
    kill(p, i, BY_GUARD, GUARD_AT, unit(GUARD));
    p.go(i, BY_LEVER, 0.0);
    p.run(0.2);
}

#[test]
fn a_lever_holds_while_its_creature_lives_and_its_pull_brings_and_takes_things_once() {
    let mut p = party(2);
    p.go(0, BY_INNER_FIRE, 0.0);
    p.go(1, BY_LEVER, 0.0);
    p.run(1.0);
    assert!(shows(&p, 1, object(LEVER)), "the lever is not shown");
    assert!(
        shows(&p, 0, object(SCENERY_UNTIL_PULL)),
        "until_used scenery"
    );
    assert!(
        !shows(&p, 0, object(SCENERY_AFTER_PULL)),
        "after_used scenery"
    );
    assert!(holds(&p, 0, unit(UNTIL_PULL)), "until_used creature");
    assert!(!holds(&p, 0, unit(AFTER_PULL)), "after_used creature");

    // Out of reach, nothing; in reach while the guard lives, it holds and sets nothing.
    click(&mut p, 0, object(LEVER));
    assert_eq!(notes(&p, 0, LEVER_HELD) + notes(&p, 0, LEVER_USED), 0);
    click(&mut p, 1, object(LEVER));
    assert_eq!(notes(&p, 1, LEVER_HELD), 1, "no held text");
    assert_eq!(notes(&p, 1, LEVER_USED), 0, "pulled while held");
    p.run(0.5);
    assert!(
        shows(&p, 0, object(SCENERY_UNTIL_PULL)),
        "held, and it went"
    );
    assert!(
        !shows(&p, 0, object(SCENERY_AFTER_PULL)),
        "held, and it came"
    );
    assert_eq!(quest_credits(&p, 1, PULL_THE_LEVER), 0);

    free_the_lever(&mut p, 1);
    click(&mut p, 1, object(LEVER));
    assert_eq!(notes(&p, 1, LEVER_USED), 1, "no used text");
    assert_eq!(notes(&p, 1, LEVER_HELD), 1, "held after the kill");
    assert_eq!(quest_credits(&p, 1, PULL_THE_LEVER), 1, "the pull's credit");
    assert_eq!(
        quest_credits(&p, 0, PULL_THE_LEVER),
        0,
        "credit to a bystander"
    );
    p.run(0.5);
    assert!(
        shows(&p, 0, object(SCENERY_AFTER_PULL)),
        "after_used scenery"
    );
    assert!(
        !shows(&p, 0, object(SCENERY_UNTIL_PULL)),
        "until_used scenery"
    );
    assert!(holds(&p, 0, unit(AFTER_PULL)), "after_used creature");
    assert!(!holds(&p, 0, unit(UNTIL_PULL)), "until_used creature");

    // A second pull does nothing.
    click(&mut p, 1, object(LEVER));
    assert_eq!(notes(&p, 1, LEVER_USED), 1, "pulled twice");
    assert_eq!(notes(&p, 1, LEVER_HELD), 1);
    assert_eq!(quest_credits(&p, 1, PULL_THE_LEVER), 1, "credited twice");
}

#[test]
fn a_timed_call_until_used_brings_its_adds_before_the_pull_and_none_after() {
    let mut p = party(2);
    p.go_facing(0, BY_CALLER, CALLER_AT);
    p.go(1, BY_LEVER, 0.0);
    p.run(0.5);
    p.cmd(0, RealmCommand::AttackSwing { guid: unit(CALLER) });
    p.run(0.2);
    p.cmd(0, RealmCommand::AttackStop);
    p.run(21.0);
    assert_eq!(adds(&p, 0).len(), 2, "no call at 20 s: {:?}", adds(&p, 0));

    free_the_lever(&mut p, 1);
    click(&mut p, 1, object(LEVER));
    assert_eq!(notes(&p, 1, LEVER_USED), 1);
    // Well past the second call's 40 s.
    p.run(25.0);
    assert_eq!(adds(&p, 0).len(), 2, "a call after the pull");
}

/// The boss in the building's south-west corner, and the two frozen beside him until his 75% and
/// 40% marks (#637); a spot beside them.
const BOSS: u64 = 800010;
const FROZEN_75: u64 = 800011;
const FROZEN_40: u64 = 800012;
const FROZEN_75_AT: [f32; 3] = [-9815.0, 537.0, 47.2];
const BY_FROZEN: [f32; 3] = [-9813.5, 537.0, 47.2];
/// Frozen in the Mist, the ice a frozen creature wears.
const ICE: u32 = 1900057;
/// `UNIT_FIELD_FLAGS` and `UNIT_FLAG_NOT_SELECTABLE`.
const UNIT_FIELD_FLAGS: u16 = 46;
const NOT_SELECTABLE: u32 = 0x0200_0000;

/// Whether player `i`'s client was last shown `guid` as not selectable: by its create, or by a
/// later change of its flags.
fn frozen(p: &Party, i: usize, guid: u64) -> bool {
    let mut frozen = false;
    for e in p.seen(i) {
        match e {
            SessionEvent::ObjectCreate {
                guid: g, fields, ..
            } if *g == guid => frozen = fields.unit_flags() & NOT_SELECTABLE != 0,
            SessionEvent::ObjectValues { guid: g, fields } if *g == guid => {
                if let Some((_, v)) = fields.raw_fields().find(|(f, _)| *f == UNIT_FIELD_FLAGS) {
                    frozen = v & NOT_SELECTABLE != 0;
                }
            }
            _ => {}
        }
    }
    frozen
}

/// Whether player `i`'s client was shown `guid` wearing the aura `spell_id`.
fn wears(p: &Party, i: usize, guid: u64, spell_id: u32) -> bool {
    p.seen(i).iter().any(|e| match e {
        SessionEvent::ObjectValues { guid: g, fields } if *g == guid => {
            fields.unit_auras().any(|a| a.spell_id == spell_id)
        }
        _ => false,
    })
}

/// Whether player `i`'s client was shown `guid` below the maximum health of its create.
fn hurt(p: &Party, i: usize, guid: u64) -> bool {
    let max = p.seen(i).iter().find_map(|e| match e {
        SessionEvent::ObjectCreate {
            guid: g, fields, ..
        } if *g == guid => fields.unit_max_health(),
        _ => None,
    });
    let max = max.expect("a create with its maximum health");
    p.seen(i).iter().any(|e| {
        matches!(e, SessionEvent::ObjectValues { guid: g, fields }
            if *g == guid && fields.unit_health().is_some_and(|h| h < max))
    })
}

/// Whether `guid` turned on player `i`, as their client was shown.
fn turned_on(p: &Party, i: usize, guid: u64) -> bool {
    let me = p.guids[i];
    p.seen(i).iter().any(|e| {
        matches!(e, SessionEvent::AttackStart { attacker, victim }
            if *attacker == guid && *victim == me)
    })
}

#[test]
fn creatures_frozen_until_a_boss_s_mark_start_frozen_wear_the_ice_and_take_no_damage() {
    let mut p = party(1);
    p.go_facing(0, BY_FROZEN, FROZEN_75_AT);
    p.run(0.5);
    for g in [FROZEN_75, FROZEN_40] {
        assert!(holds(&p, 0, unit(g)), "{g} is not in view");
        assert!(frozen(&p, 0, unit(g)), "{g} is selectable");
        assert!(wears(&p, 0, unit(g), ICE), "{g} wears no ice");
    }
    assert!(!frozen(&p, 0, unit(BOSS)), "the boss is frozen");
    p.cmd(
        0,
        RealmCommand::AttackSwing {
            guid: unit(FROZEN_75),
        },
    );
    p.run(4.0);
    assert!(!hurt(&p, 0, unit(FROZEN_75)), "damage to it landed");
    for g in [FROZEN_75, FROZEN_40] {
        assert!(!turned_on(&p, 0, unit(g)), "{g} acted while frozen");
        assert!(frozen(&p, 0, unit(g)), "{g} thawed on its own");
    }
}

#[test]
fn each_frozen_creature_thaws_at_its_mark_of_the_boss_s_health_and_turns_on_the_player() {
    let mut p = party(1);
    p.go_facing(0, BY_FROZEN, FROZEN_75_AT);
    p.run(0.5);
    p.realm.set_creature_health_pct(unit(BOSS), 70);
    p.run(0.5);
    assert!(
        !frozen(&p, 0, unit(FROZEN_75)),
        "70% did not thaw the 75% one"
    );
    assert!(
        turned_on(&p, 0, unit(FROZEN_75)),
        "the thawed one did not turn"
    );
    assert!(frozen(&p, 0, unit(FROZEN_40)), "70% thawed the 40% one");
    assert!(!turned_on(&p, 0, unit(FROZEN_40)));

    p.realm.set_creature_health_pct(unit(BOSS), 40);
    p.run(0.5);
    assert!(
        !frozen(&p, 0, unit(FROZEN_40)),
        "40% did not thaw the 40% one"
    );
    assert!(
        turned_on(&p, 0, unit(FROZEN_40)),
        "the thawed one did not turn"
    );
}

#[test]
fn a_boss_killed_before_its_marks_leaves_them_frozen() {
    let mut p = party(1);
    p.go_facing(0, BY_FROZEN, FROZEN_75_AT);
    p.run(0.5);
    p.realm.kill_creature(unit(BOSS));
    p.run(3.0);
    for g in [FROZEN_75, FROZEN_40] {
        assert!(frozen(&p, 0, unit(g)), "{g} thawed after the boss died");
        assert!(!turned_on(&p, 0, unit(g)), "{g} turned after the boss died");
    }
}

#[test]
fn a_run_that_resets_freezes_them_again() {
    let mut p = party(1);
    p.go_facing(0, BY_FROZEN, FROZEN_75_AT);
    p.run(0.5);
    p.realm.set_creature_health_pct(unit(BOSS), 30);
    p.run(0.5);
    for g in [FROZEN_75, FROZEN_40] {
        assert!(!frozen(&p, 0, unit(g)), "{g} did not thaw");
    }
    // The player leaves, the Run resets, and the next one starts as the first did.
    p.go(0, [-9900.0, 555.0, 47.2], 0.0);
    p.run(1.0);
    let n = p.seen(0).len();
    p.go_facing(0, BY_FROZEN, FROZEN_75_AT);
    p.run(1.0);
    let made: Vec<u64> = unit_creates(&p.seen(0)[n..])
        .into_iter()
        .map(|(guid, _, _)| guid)
        .collect();
    for g in [FROZEN_75, FROZEN_40] {
        assert!(made.contains(&unit(g)), "{g} not made again: {made:x?}");
        assert!(frozen(&p, 0, unit(g)), "{g} is not frozen in the new Run");
    }
}

#[test]
fn a_run_that_resets_puts_the_lever_back() {
    let mut p = party(2);
    p.go(0, BY_INNER_FIRE, 0.0);
    p.go(1, BY_LEVER, 0.0);
    p.run(1.0);
    free_the_lever(&mut p, 1);
    click(&mut p, 1, object(LEVER));
    p.run(0.5);
    assert!(holds(&p, 0, unit(AFTER_PULL)));
    assert!(!shows(&p, 0, object(SCENERY_UNTIL_PULL)));
    // Everyone leaves, as in `a_run_that_resets_puts_them_back_as_they_were`.
    p.go(1, BY_INNER_FIRE, 0.0);
    p.run(0.5);
    p.go(0, [-9900.0, 555.0, 47.2], 0.0);
    p.go(1, [-9900.0, 555.0, 47.2], 0.0);
    p.run(1.0);
    p.go(0, BY_INNER_FIRE, 0.0);
    p.go(1, BY_LEVER, 0.0);
    p.run(1.0);
    assert!(
        shows(&p, 0, object(SCENERY_UNTIL_PULL)),
        "until_used scenery"
    );
    assert!(
        !shows(&p, 0, object(SCENERY_AFTER_PULL)),
        "after_used scenery"
    );
    assert!(holds(&p, 0, unit(UNTIL_PULL)), "until_used creature");
    assert!(!holds(&p, 0, unit(AFTER_PULL)), "after_used creature");
    // The guard is back and holds it; after the kill it pulls again.
    click(&mut p, 1, object(LEVER));
    assert_eq!(notes(&p, 1, LEVER_HELD), 1, "the guard is not back");
    free_the_lever(&mut p, 1);
    click(&mut p, 1, object(LEVER));
    assert_eq!(notes(&p, 1, LEVER_USED), 2, "the lever stayed pulled");
}
