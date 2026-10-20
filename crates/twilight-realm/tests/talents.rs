//! Talent Points and the Basic Warrior's "Recruit" Talent Tree, played through the realm's seam:
//! the points the client shows, `CMSG_LEARN_TALENT` and its refusals, what learned talents do in a
//! fight, the respec at the Crossroads' Vanguard Trainer, and the save. Deep Wounds, which the Recruit
//! tree dropped, is played on the Zealot's Blade tab, which still has it.

mod common;

use benilla_protocol::{EntityKind, ObjectFields, SessionEvent};
use common::*;
use twilight_realm::DevMultipliers;

// The Recruit tree's borrowed `Talent.dbc` ids and their rank spells (`master.csv`).
static FIELD_MEDIC: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("basic_warrior", "field_medic"));
static FIELD_MEDIC_RANKS: std::sync::LazyLock<Vec<u32>> =
    std::sync::LazyLock::new(|| talent_spells("basic_warrior", "field_medic"));
static INITIATES_SPARK: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("basic_warrior", "initiates_spark"));
static INITIATES_SPARK_RANKS: std::sync::LazyLock<Vec<u32>> =
    std::sync::LazyLock::new(|| talent_spells("basic_warrior", "initiates_spark"));
static SAVAGE_STRIKES: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("basic_warrior", "savage_strikes"));
static SAVAGE_STRIKES_RANKS: std::sync::LazyLock<Vec<u32>> =
    std::sync::LazyLock::new(|| talent_spells("basic_warrior", "savage_strikes"));
static VANGUARDS_RESOLVE: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("basic_warrior", "vanguards_resolve"));
static VANGUARDS_RESOLVE_RANKS: std::sync::LazyLock<Vec<u32>> =
    std::sync::LazyLock::new(|| talent_spells("basic_warrior", "vanguards_resolve"));
/// A real Warrior talent that is not in the Recruit tree: Mortal Strike.
const MORTAL_STRIKE: u32 = 135;

// vmangos `SpellModOp`.

// `NPCFlags`.
const NPC_GOSSIP: u32 = 0x01;
const NPC_QUESTGIVER: u32 = 0x02;
const NPC_TRAINER: u32 = 0x10;
/// Uzzek, the Vanguard Trainer, who sells the respec.
const UZZEK: u32 = 5810;

const SEED: u64 = 0x5EED_0014;
const CROSSROADS_XY: [f32; 2] = [-441.8, -2648.6];

#[test]
fn the_client_shows_the_tree_of_the_calling_in_the_world() {
    let mut realm = realm();
    assert!(realm.current_calling().is_none(), "nobody at select");
    enter_new(&mut realm, "Garrosh", ORC);
    let calling = realm.current_calling().expect("in the world");
    assert_eq!(calling.key, "basic_warrior");
    let tabs: Vec<&str> = calling
        .talent_tabs
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(tabs, ["Recruit"]);
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
fn a_basic_warrior_at_level_10_has_nine_talent_points() {
    let (_, me, _, view) = warrior_at(10);
    assert_eq!(view.points(me), 9);
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
    learn(&mut realm, *FIELD_MEDIC, 0);
    let events = view.drain(&mut realm);
    assert!(events.iter().any(
        |e| matches!(e, SessionEvent::SpellLearned { spell_id } if *spell_id == FIELD_MEDIC_RANKS[0])
    ));
    assert_eq!(view.points(me), 2);
    assert_eq!(view.talent_rank(&*FIELD_MEDIC_RANKS), 1);

    learn(&mut realm, *FIELD_MEDIC, 1);
    view.drain(&mut realm);
    assert_eq!(view.points(me), 1);
    assert_eq!(view.talent_rank(&*FIELD_MEDIC_RANKS), 2);
}

#[test]
fn a_talent_cannot_be_learned_without_a_point() {
    let (mut realm, me, _, mut view) = warrior_at(2);
    learn(&mut realm, *FIELD_MEDIC, 0);
    learn(&mut realm, *FIELD_MEDIC, 1);
    view.drain(&mut realm);
    assert_eq!(view.points(me), 0);
    assert_eq!(
        view.talent_rank(&*FIELD_MEDIC_RANKS),
        1,
        "one point, one rank"
    );
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
        learn(&mut realm, *INITIATES_SPARK, rank);
    }
    view.drain(&mut realm);
    assert_eq!(view.talent_rank(&*INITIATES_SPARK_RANKS), 3);
    assert_eq!(view.points(me), 6);
}

#[test]
fn the_second_row_opens_after_five_points_in_the_tab() {
    let (mut realm, me, _, mut view) = warrior_at(10);
    for second_row in [*SAVAGE_STRIKES, *VANGUARDS_RESOLVE] {
        learn(&mut realm, second_row, 0);
    }
    view.drain(&mut realm);
    assert_eq!(view.points(me), 9, "row 2 is shut with nothing spent");

    for rank in 0..4 {
        learn(&mut realm, *FIELD_MEDIC, rank);
    }
    learn(&mut realm, *SAVAGE_STRIKES, 0);
    view.drain(&mut realm);
    assert_eq!(view.points(me), 5, "still shut at four");

    learn(&mut realm, *FIELD_MEDIC, 4);
    learn(&mut realm, *SAVAGE_STRIKES, 0);
    view.drain(&mut realm);
    assert_eq!(view.points(me), 3, "open at five");
    assert_eq!(view.talent_rank(&*FIELD_MEDIC_RANKS), 5);
    assert_eq!(view.talent_rank(&*SAVAGE_STRIKES_RANKS), 1);
}

#[test]
fn a_talent_the_realm_does_not_model_yet_is_still_learned() {
    let (mut realm, me, _, mut view) = warrior_at(10);
    open_the_second_row(&mut realm);
    learn(&mut realm, *VANGUARDS_RESOLVE, 0);
    view.drain(&mut realm);
    assert_eq!(view.talent_rank(&*VANGUARDS_RESOLVE_RANKS), 1);
    assert_eq!(view.points(me), 3);
}

// ── The respec at the Vanguard Trainer ──────────────────────────────────────────────────────────

#[test]
fn the_vanguard_trainer_stands_in_the_crossroads() {
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
    // In the training yard, in view of the inn.
    assert!(d < 60.0, "{d} yd from where a character starts");

    let entry = fields.object_entry().unwrap();
    realm.handle(RealmCommand::CreatureQuery { entry });
    let named = view.drain(&mut realm).into_iter().find_map(|e| match e {
        SessionEvent::CreatureName { name, subname, .. } => Some((name, subname)),
        _ => None,
    });
    assert_eq!(
        named,
        Some((Some("Uzzek".into()), Some("Vanguard Trainer".into())))
    );
    let _ = me;
}

#[test]
fn the_trainer_offers_to_unlearn_talents() {
    let (mut realm, _, _, mut view) = warrior_at(4);
    let trainer = view.trainer();
    walk_up_to(&mut realm, trainer.guid);
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
    // Both are the trainer's row in content/hub.toml.
    let hub: toml::Table = include_str!("../content/hub.toml").parse().unwrap();
    let row = hub["npc"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"].as_str() == Some("Uzzek"))
        .expect("the trainer's row");
    assert_eq!(i64::from(text_id), row["text_id"].as_integer().unwrap());
    assert_eq!(Some(blocks[0].male.as_str()), row["greeting"].as_str());
}

#[test]
fn asking_to_unlearn_closes_the_menu_and_names_the_price() {
    let (mut realm, _, _, mut view) = warrior_at(4);
    learn(&mut realm, *FIELD_MEDIC, 0);
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
    open_the_second_row(&mut realm);
    for rank in 0..2 {
        learn(&mut realm, *SAVAGE_STRIKES, rank);
    }
    view.drain(&mut realm);
    assert_eq!(view.points(me), 2);
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
    let mut every_rank: Vec<u32> = SAVAGE_STRIKES_RANKS
        .iter()
        .chain(FIELD_MEDIC_RANKS.iter())
        .copied()
        .collect();
    every_rank.sort_unstable();
    assert_eq!(removed.into_iter().collect::<Vec<_>>(), every_rank);
    assert_eq!(view.talent_rank(&*SAVAGE_STRIKES_RANKS), 0);
    assert_eq!(view.talent_rank(&*FIELD_MEDIC_RANKS), 0);

    // The next one costs 5 gold.
    learn(&mut realm, *FIELD_MEDIC, 0);
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
    learn(&mut realm, *FIELD_MEDIC, 0);
    let trainer = view.trainer();
    ask_for_respec(&mut realm, &mut view, trainer.guid);
    realm.handle(RealmCommand::TalentWipeConfirm {
        trainer: trainer.guid,
    });
    view.drain(&mut realm);
    assert_eq!(view.money(me), 9999);
    assert_eq!(view.talent_rank(&*FIELD_MEDIC_RANKS), 1);
    assert_eq!(view.points(me), 2);
}

#[test]
fn the_trainer_must_be_close_to_respec() {
    let (mut realm, me, mob, mut view) = warrior_with(
        4,
        DevMultipliers::default(),
        &[("money = 0", "money = 20000")],
    );
    learn(&mut realm, *FIELD_MEDIC, 0);
    let trainer = view.trainer();
    ask_for_respec(&mut realm, &mut view, trainer.guid);
    // Off to the mob, out of the trainer's reach.
    stand_at(&mut realm, &mob, 2.0);
    realm.handle(RealmCommand::TalentWipeConfirm {
        trainer: trainer.guid,
    });
    view.drain(&mut realm);
    assert_eq!(view.money(me), 20_000);
    assert_eq!(view.talent_rank(&*FIELD_MEDIC_RANKS), 1);
}

#[test]
fn the_respec_price_is_saved() {
    let (mut realm, me, _, mut view) = warrior_with(
        4,
        DevMultipliers::default(),
        &[("money = 0", "money = 20000")],
    );
    learn(&mut realm, *FIELD_MEDIC, 0);
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
    learn(&mut other, *FIELD_MEDIC, 0);
    let events = ask_for_respec(&mut other, &mut view, trainer.guid);
    assert_eq!(wipe_price(&events, trainer.guid), Some(50_000));
}

// ── The save ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn learning_a_talent_saves_the_character() {
    let (mut realm, me, _, _) = warrior_at(4);
    realm.drain_storage();
    learn(&mut realm, *FIELD_MEDIC, 0);
    let writes: Vec<_> = realm
        .drain_storage()
        .into_iter()
        .filter(|s| matches!(s, twilight_realm::Storage::Write { guid, .. } if *guid == me))
        .collect();
    assert_eq!(writes.len(), 1);
}

#[test]
fn talents_and_points_come_back_from_the_save() {
    let (mut realm, me, _, mut view) = warrior_at(7);
    open_the_second_row(&mut realm);
    learn(&mut realm, *SAVAGE_STRIKES, 0);
    view.drain(&mut realm);
    let blob = realm.export_character(me).unwrap();

    let mut other = Realm::with_seed(SEED);
    other.import_character(&blob).unwrap();
    other.handle(RealmCommand::EnterWorld { guid: me });
    other.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut other);
    assert_eq!(view.points(me), 0);
    assert_eq!(view.talent_rank(&*FIELD_MEDIC_RANKS), 5);
    assert_eq!(view.talent_rank(&*SAVAGE_STRIKES_RANKS), 1);
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

// ── A talent the Recruit tree dropped, on the Zealot ──────────────────────────────────────────

static T1_DEEP_WOUNDS: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("zealot", "deep_wounds"));

/// Deep Wounds' bleed (vmangos 12721, "Deep Wound").
const DEEP_WOUND: u32 = 12721;

#[test]
fn deep_wounds_bleeds_the_target_after_a_melee_crit() {
    // The first seeded fight whose crit leaves the mob standing.
    for seed in 0..100 {
        let (mut realm, me, mob, mut view) = tier1_at(10, seed, &[(*T1_DEEP_WOUNDS, 1)]);
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

    fn money(&self, me: u64) -> u32 {
        self.fields(me).player_money().unwrap_or(0)
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
    /// The trainer who sells the respec, as the client has been shown him: Uzzek, the Vanguard
    /// Trainer, one of the training yard's five.
    fn trainer(&self) -> Mob {
        let mut trainers = self
            .objects
            .iter()
            .filter(|(_, f)| f.unit_npc_flags() & NPC_TRAINER != 0);
        assert_eq!(trainers.clone().count(), 5, "the five Tier 1 trainers");
        let (&guid, _) = trainers
            .find(|(_, f)| f.object_entry() == Some(UZZEK))
            .expect("Uzzek in the world");
        Mob {
            guid,
            position: self.positions[&guid],
        }
    }
}

/// Talks to the trainer and picks its unlearn line, as the stock gossip frame does.
fn ask_for_respec(realm: &mut Realm, view: &mut View, trainer: u64) -> Vec<SessionEvent> {
    walk_up_to(realm, trainer);
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
    warrior_from(level, seed, dev, |mut blob| {
        for (from, to) in edits {
            blob = blob.replace(from, to);
        }
        blob
    })
}

/// A human Zealot of `level` with `talents` (`(talent id, rank)`) in its save, placed as
/// [`warrior_at`] places a Basic Warrior.
fn tier1_at(level: u8, seed: u64, talents: &[(u32, u8)]) -> (Realm, u64, Mob, View) {
    warrior_from(level, seed, DevMultipliers::default(), |blob| {
        let blob = blob.replace("calling = \"basic_warrior\"", "calling = \"zealot\"");
        if talents.is_empty() {
            return blob;
        }
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        )
    })
}

/// A human warrior of `level` from a fresh save that `edit` changes first, on `seed`'s dice.
fn warrior_from(
    level: u8,
    seed: u64,
    dev: DevMultipliers,
    edit: impl FnOnce(String) -> String,
) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let blob = edit(
        maker
            .export_character(guid)
            .unwrap()
            .replace("level = 1\n", &format!("level = {level}\n")),
    );
    let mut realm = Realm::with_seed(seed).with_dev(dev);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let events = view.drain(&mut realm);
    let mob = the_mob(&events);
    (realm, guid, mob, view)
}

/// Five points in the Recruit tree's first row: Field Medic 5/5.
fn open_the_second_row(realm: &mut Realm) {
    for rank in 0..5 {
        learn(realm, *FIELD_MEDIC, rank);
    }
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
        motion: Default::default(),
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
