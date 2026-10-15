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

const SEED: u64 = 0x5EED_0014;
const CROSSROADS_XY: [f32; 2] = [-441.8, -2648.6];

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

// ── Helpers ─────────────────────────────────────────────────────────────────────────────────────

/// What the client knows: every object's fields as the packets left them, and the spells known.
#[derive(Default)]
struct View {
    objects: std::collections::HashMap<u64, ObjectFields>,
    spells: std::collections::BTreeSet<u32>,
}

impl View {
    fn see(&mut self, events: &[SessionEvent]) {
        for e in events {
            match e {
                SessionEvent::ObjectCreate { guid, fields, .. } => {
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
    let mut realm = Realm::with_seed(SEED).with_dev(dev);
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
