//! The legendary and plant meals' specials (#493, ADR 0025): what each Well Fed special does in a
//! fight, eaten from the bags as in `tests/food.rs`, or carried in the save for a party script.

mod common;
mod gear_common;

use benilla_protocol::{ObjectFields, SessionEvent, START_ELWYNN};
use common::party::{NoGround, Party};
use common::*;
use gear_common::*;

const PACK_1: u8 = 23;
const WELL_FED: u32 = 19705;
const FED_SLOT: usize = 30;
const STOCKADE_SUCKER_POT: u32 = 962207; // cc_half 50
const HARBOURMASTERS_EEL_PIE: u32 = 962208; // kill_heal 3
const BLOODFIN_SEARING: u32 = 962211; // damage_stack 1, max 10
const LEVIATHAN_FEAST: u32 = 962212; // revive 30
const STITCHJAW_STEW: u32 = 962215; // melee_reflect 10
const MOONLESS_MUSKIE_SUPPER: u32 = 962216; // night_aggro_half 50
const WIDOWS_LILY_TEA: u32 = 962303; // shadow_resist 25, fear_half 50
const STRIKER_SUPPER: u32 = 969811;
const EMBER_SUPPER: u32 = 969812;
/// `SpellDamageLog::school` of fire and nature.
const FIRE: u8 = 2;
const NATURE: u8 = 3;

/// Test meals whose specials go off at every chance.
const SURE_MEALS: &str = r#"
item = [
    { entry = 969811, name = "Striker's Test Supper", use_spell = 25695 },
    { entry = 969812, name = "Ember Test Supper", use_spell = 25695 },
]

[[recipe]]
key = "test_extra_strike"
kind = "legend"
meal = 969811
ingredients = [960401, 960401, 960401, 960401, 960402]
unique = { key = "extra_strike", chance_pct = 100 }

[[recipe]]
key = "test_fire_proc"
kind = "plant"
meal = 969812
ingredients = [960401, 960401, 960401, 960402, 960402]
specials = [{ key = "fire_proc", chance_pct = 100, damage = 20 }]
"#;

struct Script {
    realm: Realm,
    me: u64,
    events: Vec<SessionEvent>,
}

impl Script {
    /// A warrior of `level` (holding Thug's Cutlass, for `armed`) who has eaten `meal`.
    fn fed(level: u8, armed: bool, meal: u32) -> Self {
        let _ = Realm::new().with_cooking(SURE_MEALS);
        let items = [(PACK_1, meal, 1)];
        let (realm, me, events) = if armed {
            fighter_with(level, 0, &items)
        } else {
            warrior_with(level, 0, &items)
        };
        let mut s = Self { realm, me, events };
        s.realm.handle(RealmCommand::UseItem {
            bag: 255,
            slot: PACK_1,
        });
        s.events.extend(drain(&mut s.realm));
        assert_eq!(s.fed_aura(), WELL_FED, "eaten");
        s
    }

    /// One 0.1 s tick: its events, also kept.
    fn tick(&mut self) -> Vec<SessionEvent> {
        self.realm.tick(0.1, &FlatGround);
        let new = drain(&mut self.realm);
        self.events.extend(new.clone());
        new
    }

    fn me(&self) -> ObjectFields {
        descriptor(&self.events, self.me)
    }

    fn fed_aura(&self) -> u32 {
        self.me().unit_aura_ids().nth(FED_SLOT).unwrap_or(0)
    }

    /// The nearest mob is pulled onto the warrior, who does not swing back.
    fn pull(&mut self) -> Mob {
        let mob = the_mob(&self.events);
        stand_by(&mut self.realm, &mob);
        self.realm
            .handle(RealmCommand::AttackSwing { guid: mob.guid });
        self.realm.handle(RealmCommand::AttackStop);
        self.events.extend(drain(&mut self.realm));
        mob
    }
}

/// The swings `attacker` landed on `victim` in `events`, by damage.
fn landed(events: &[SessionEvent], attacker: u64, victim: u64) -> Vec<u32> {
    swings(events, attacker, victim)
        .into_iter()
        .filter(|&d| d > 0)
        .collect()
}

/// Every swing `attacker` took at `victim` in `events`, by damage (0 for one that missed).
fn swings(events: &[SessionEvent], attacker: u64, victim: u64) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::AttackerState(s) if s.attacker == attacker && s.victim == victim => {
                Some(s.damage)
            }
            _ => None,
        })
        .collect()
}

/// The Well Fed buff's spell damage `attacker` logged on `target` in `events`: `(damage, school)`.
fn fed_hits(events: &[SessionEvent], attacker: u64, target: u64) -> Vec<(u32, u8)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l)
                if l.attacker == attacker && l.target == target && l.spell_id == WELL_FED =>
            {
                Some((l.damage, l.school))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn kill_heal_heals_three_percent_of_max_health_at_a_kill() {
    let mut s = Script::fed(5, true, HARBOURMASTERS_EEL_PIE);
    let mob = the_mob(&s.events);
    let fight = kill(&mut s.realm, &mob);
    let died = fight
        .iter()
        .position(|e| {
            matches!(e, SessionEvent::ObjectValues { guid, fields }
            if *guid == mob.guid && fields.unit_health() == Some(0))
        })
        .unwrap();
    s.events.extend(fight[..died].iter().cloned());
    let (before, max) = (
        s.me().unit_health().unwrap(),
        s.me().unit_max_health().unwrap(),
    );
    assert!(before < max, "hurt in the fight: {before} of {max}");
    let after = values_of(&fight[died..], s.me)
        .find_map(|f| f.unit_health())
        .expect("a heal at the kill");
    assert_eq!(after, (before + max * 3 / 100).min(max));
}

#[test]
fn revive_cancels_one_death_at_thirty_percent_and_ends_the_buff() {
    let mut s = Script::fed(1, false, LEVIATHAN_FEAST);
    let max = s.me().unit_max_health().unwrap();
    s.pull();
    let line = "Everstill Leviathan's meal pulls you back from death.";
    let mut revived = false;
    for _ in 0..12_000 {
        let new = s.tick();
        if format!("{new:?}").contains(line) {
            revived = true;
            break;
        }
        assert_ne!(s.me().unit_health(), Some(0), "died with the revive on");
    }
    assert!(revived, "the warrior outlived twenty minutes of beating");
    assert_eq!(s.me().unit_health(), Some(max * 30 / 100));
    assert_eq!(s.fed_aura(), 0, "the buff is gone");

    // The second death kills.
    s.pull();
    for _ in 0..12_000 {
        s.tick();
        if s.me().unit_health() == Some(0) {
            return;
        }
    }
    panic!("no second death");
}

#[test]
fn melee_reflect_returns_a_tenth_of_each_blow_as_nature_damage() {
    let mut s = Script::fed(10, false, STITCHJAW_STEW);
    let mob = s.pull();
    let from = s.events.len();
    for _ in 0..200 {
        s.tick();
    }
    let fight = &s.events[from..];
    let blows = landed(fight, mob.guid, s.me);
    assert!(!blows.is_empty(), "the mob landed blows");
    let owed: Vec<(u32, u8)> = blows
        .iter()
        .map(|d| d * 10 / 100)
        .filter(|&d| d > 0)
        .map(|d| (d, NATURE))
        .collect();
    assert_eq!(fed_hits(fight, s.me, mob.guid), owed);
}

#[test]
fn damage_stack_raises_damage_a_percent_a_hit_to_ten_and_resets_out_of_combat() {
    // Two warriors on one seed, one fed: the dice fall alike, so each fed swing is the plain
    // one's, raised by the hits landed before it in that fight.
    let mut plain = Script::fed(10, true, STITCHJAW_STEW);
    let mut fed = Script::fed(10, true, BLOODFIN_SEARING);
    let mob = the_mob(&fed.events);
    let mut fights: Vec<[Vec<u32>; 2]> = Vec::new();
    for round in 0..2 {
        let at = settled_at(&fed.events, mob.guid, mob.position);
        let here = Mob {
            guid: mob.guid,
            position: at,
            fields: mob.fields.clone(),
        };
        let marks = [plain.events.len(), fed.events.len()];
        for s in [&mut plain, &mut fed] {
            stand_by(&mut s.realm, &here);
            s.realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
        }
        for _ in 0..if round == 0 { 300 } else { 100 } {
            for s in [&mut plain, &mut fed] {
                s.realm.set_creature_health_pct(mob.guid, 100);
                s.tick();
            }
        }
        fights.push([
            swings(&plain.events[marks[0]..], plain.me, mob.guid),
            swings(&fed.events[marks[1]..], fed.me, mob.guid),
        ]);
        // Out of the fight: well away until the mob gives up, then back once it is home.
        for s in [&mut plain, &mut fed] {
            s.realm.handle(RealmCommand::AttackStop);
            let away = [mob.position[0] + 90.0, mob.position[1], mob.position[2]];
            s.realm.handle(RealmCommand::Move {
                motion: Default::default(),
                position: away,
                orientation: 0.0,
            });
        }
        for _ in 0..300 {
            for s in [&mut plain, &mut fed] {
                s.tick();
            }
        }
        assert_eq!(fed.me().unit_flags() & 0x0008_0000, 0, "out of combat");
    }
    for (round, [plain, fed]) in fights.iter().enumerate() {
        assert_eq!(plain.len(), fed.len(), "the same swings");
        // The first fight runs past the cap; the second shows the stack starting again.
        let need = if round == 0 { 11 } else { 2 };
        assert!(
            plain.iter().filter(|&&d| d > 0).count() >= need,
            "{plain:?}"
        );
        let mut hits = 0;
        for (&p, &f) in plain.iter().zip(fed) {
            assert_eq!(f, p * (100 + hits.min(10)) / 100, "after {hits} hits");
            if p > 0 {
                hits += 1;
            }
        }
    }
}

#[test]
fn extra_strike_swings_again_at_every_landed_swing() {
    let mut s = Script::fed(10, true, STRIKER_SUPPER);
    let mob = the_mob(&s.events);
    stand_by(&mut s.realm, &mob);
    s.realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut landed_ticks = 0;
    for _ in 0..200 {
        s.realm.set_creature_health_pct(mob.guid, 100);
        let tick = s.tick();
        let mine = swings(&tick, s.me, mob.guid);
        match mine.first() {
            Some(&d) if d > 0 => {
                landed_ticks += 1;
                assert_eq!(mine.len(), 2, "a landed swing and its strike: {mine:?}");
            }
            Some(_) => assert_eq!(mine.len(), 1, "a missed swing strikes no more"),
            None => {}
        }
    }
    assert!(landed_ticks > 3);
}

#[test]
fn fire_proc_burns_the_target_at_every_landed_swing() {
    let mut s = Script::fed(10, true, EMBER_SUPPER);
    let mob = the_mob(&s.events);
    stand_by(&mut s.realm, &mob);
    s.realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let from = s.events.len();
    for _ in 0..200 {
        s.realm.set_creature_health_pct(mob.guid, 100);
        s.tick();
    }
    let fight = &s.events[from..];
    let hits = landed(fight, s.me, mob.guid).len();
    assert!(hits > 3);
    assert_eq!(fed_hits(fight, s.me, mob.guid), vec![(20, FIRE); hits]);
}

/// The Gaze Hall's Vile Familiar (`content/gaze_hall.toml`), which these tests give one Creature
/// Spell, and a spot in its melee reach.
const FAMILIAR: u32 = 3101;
const EXPOSED: [f32; 3] = [-9650.0, 21.5, 47.2];

/// Level 20 characters at `EXPOSED` fighting the familiar, which casts `spell`, for `secs`: each
/// under `meals`' Well Fed buff (0 for none).
fn familiar_fight(spell: &str, meals: &[u32], secs: f32) -> Party {
    let realm = realm().with_creature_spells(FAMILIAR, &[spell]);
    let meals = meals.to_vec();
    let mut p = Party::edited(
        realm,
        &vec![20; meals.len()],
        START_ELWYNN,
        Box::new(NoGround),
        |i, blob| match meals[i] {
            0 => blob,
            meal => blob + &format!("\n[well_fed]\nmeal = {meal}\nleft_secs = 3600.0\n"),
        },
    );
    for i in 0..meals.len() {
        p.go(i, EXPOSED, 0.0);
    }
    p.run(secs);
    p
}

/// What each hit of spell `id` took off `me`'s health in `events`.
fn health_lost(events: &[SessionEvent], me: u64, id: u32) -> Vec<u32> {
    let mut health = None;
    let mut lost = Vec::new();
    let mut waiting = false;
    for e in events {
        match e {
            SessionEvent::SpellDamageLog(l) if l.target == me && l.spell_id == id => waiting = true,
            SessionEvent::ObjectCreate { guid, fields, .. }
            | SessionEvent::ObjectValues { guid, fields }
                if *guid == me =>
            {
                if let Some(h) = fields.unit_health() {
                    if waiting {
                        lost.push(health.unwrap_or(h) - h);
                        waiting = false;
                    }
                    health = Some(h);
                }
            }
            _ => {}
        }
    }
    lost
}

#[test]
fn shadow_resist_takes_its_share_off_shadow_damage_by_the_attackers_level() {
    let nova = creature_spell("test_nova");
    let p = familiar_fight("test_nova", &[0], 10.0);
    let plain = health_lost(p.seen(0), p.guids[0], nova);
    assert!(
        !plain.is_empty() && plain.iter().all(|&d| d == 30),
        "{plain:?}"
    );
    let p = familiar_fight("test_nova", &[WIDOWS_LILY_TEA], 10.0);
    let (familiar, _) = p.unit(0, FAMILIAR, EXPOSED);
    let level = descriptor(p.seen(0), familiar).unit_level().unwrap();
    let fed = health_lost(p.seen(0), p.guids[0], nova);
    // 25 / (25 + 5 x level) of the 30 is resisted, rounded down.
    let resisted = 30 * 25 / (25 + 5 * level);
    assert!(!fed.is_empty(), "the nova hit");
    assert!(
        fed.iter().all(|&d| d == 30 - resisted),
        "{fed:?}, level {level}"
    );
}

/// Each time `me` had the client's control taken (`false`) or given back (`true`) in `seen`.
fn control(seen: &[SessionEvent], me: u64) -> Vec<bool> {
    seen.iter()
        .filter_map(|e| match e {
            SessionEvent::ClientControl { mover, allow_move } if *mover == me => Some(*allow_move),
            _ => None,
        })
        .collect()
}

/// How long a level 20 warrior under `meal`'s Well Fed buff (0 for none) stays feared by the
/// familiar's `test_fear` (6 s), in seconds.
fn fear_secs(meal: u32) -> f32 {
    let realm = realm().with_creature_spells(FAMILIAR, &["test_fear"]);
    let mut p = Party::edited(realm, &[20], START_ELWYNN, Box::new(NoGround), |_, blob| {
        if meal == 0 {
            blob
        } else {
            blob + &format!("\n[well_fed]\nmeal = {meal}\nleft_secs = 3600.0\n")
        }
    });
    p.go(0, EXPOSED, 0.0);
    let me = p.guids[0];
    let mut times = Vec::new();
    for tick in 0..140 {
        for allow in control(&p.run(0.1)[0], me) {
            times.push((tick as f32 * 0.1, allow));
        }
    }
    let [(taken, false), (given, true), ..] = times[..] else {
        panic!("control taken and given back: {times:?}");
    };
    given - taken
}

#[test]
fn cc_half_halves_a_creature_fear() {
    assert!((fear_secs(0) - 6.0).abs() < 0.3, "the plain fear is 6 s");
    let half = fear_secs(STOCKADE_SUCKER_POT);
    assert!((half - 3.0).abs() < 0.3, "{half} s");
}

#[test]
fn fear_half_halves_a_creature_fear() {
    let half = fear_secs(WIDOWS_LILY_TEA);
    assert!((half - 3.0).abs() < 0.3, "{half} s");
}

/// A level-1 warrior's realm, as [`warrior_with`]'s, the warrior under `meal`'s Well Fed buff (0
/// for none) and the game clock at `hour`:00.
fn watcher(meal: u32, hour: u64) -> Realm {
    let (guid, mut blob) = save_with(1, 0, &[]);
    if meal != 0 {
        blob += &format!("\n[well_fed]\nmeal = {meal}\nleft_secs = 3600.0\n");
    }
    let mut realm = Realm::with_seed(SEED);
    realm.import_character(&blob).unwrap();
    realm.set_wall_clock(hour * 60 * 5 * 1000);
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    drain(&mut realm);
    realm
}

/// Moves the player to `at` and ticks once: the mobs that started attacking.
fn step_to(realm: &mut Realm, at: [f32; 3]) -> Vec<u64> {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: at,
        orientation: 0.0,
    });
    realm.tick(0.0, &FlatGround);
    drain(realm)
        .into_iter()
        .filter_map(|e| match e {
            SessionEvent::AttackStart { attacker, .. } if attacker >> 48 == 0xF130 => {
                Some(attacker)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn night_aggro_half_halves_the_notice_radius_at_night_only() {
    // Walk at the first mob of the entry burst from 40 yd north, a yard a tick: how near it
    // lets the warrior come.
    let mob = the_mob(&warrior_with(1, 0, &[]).2);
    let approach = |meal: u32, hour: u64| -> Option<f32> {
        let mut realm = watcher(meal, hour);
        for d in (1..=40).rev() {
            let at = [mob.position[0] + d as f32, mob.position[1], mob.position[2]];
            if step_to(&mut realm, at).contains(&mob.guid) {
                return Some(d as f32);
            }
        }
        None
    };
    let noon = approach(MOONLESS_MUSKIE_SUPPER, 12).expect("noticed at noon");
    let plain_night = approach(0, 23).expect("noticed at night without the meal");
    let night = approach(MOONLESS_MUSKIE_SUPPER, 23).expect("noticed at night");
    assert!(
        (noon - plain_night).abs() <= 1.0,
        "{noon} and {plain_night}"
    );
    assert!(
        (night - noon / 2.0).abs() <= 1.5,
        "half of {noon} yd at night: {night}"
    );
}
