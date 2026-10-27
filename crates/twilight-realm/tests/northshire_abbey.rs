//! Northshire Abbey, the first Phased Dungeon (#78): a character inside its footprint sees and
//! fights only its own monsters (#94; Brother Neals stands there too, and never fights), and its
//! Run resets when the last one leaves or dies.

mod common;

use std::collections::{HashMap, HashSet};

use benilla_protocol::messages::AttackSwingError;
use benilla_protocol::{ObjectFields, SessionEvent, START_ELWYNN};
use common::*;

/// `HIGHGUID_UNIT`, in a creature guid's top 16 bits.
const HIGHGUID_UNIT: u64 = 0xF130;
/// The Abbey's survivor, a Field Giver (#85).
const BROTHER_NEALS: u64 = HIGHGUID_UNIT << 48 | 952 << 24 | 780005;
/// His row's x and y: the nave floor at the back of the side chapel, model (-0.5, 26.4) (#349).
const NEALS_AT: [f32; 2] = [-8887.8, -153.5];
/// The cage around him (#349), a GameObject guid.
const NEALS_CAGE: u64 = 0xF110 << 48 | 790902;
/// `FactionTemplate.dbc` 35, friendly to every race.
const FRIENDLY_TO_ALL: u32 = 35;
/// The Abbot at the top of the bell tower.
const ABBOT: u64 = HIGHGUID_UNIT << 48 | 96005 << 24 | 780015;
/// Where the Abbot stands, in the test's flat Abbey.
const ABBOT_AT: [f32; 2] = [-8900.5, -181.8];
/// The middle of the bell tower's top platform (#315): the floor where the tower stair comes up,
/// the south arm of the cross-shaped floor at nsabbey.wmo's z 33.1 (group 10), model x -7 to 3
/// and y -9 to 0, whose middle (-2, -4.5) goes through the content file's transform.
const PLATFORM_MIDDLE: [f32; 2] = [-8900.5, -181.8];
/// Staff of the Friar (item 3415), in his main hand.
const STAFF_OF_THE_FRIAR_DISPLAY: u32 = 20339;
/// Inside the nave, and out in the yard before the front doors.
const INSIDE: [f32; 2] = [-8890.0, -172.0];
const OUTSIDE: [f32; 2] = [-8950.0, -120.0];

/// The Abbey's floor everywhere, so a teleport lands inside the footprint's height.
struct AbbeyFloor(f32);

impl Host for AbbeyFloor {
    fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
        Some(self.0)
    }
    fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
        true
    }
    fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
        None
    }
}

/// The Abbey's monsters at a Run's start (#94): their low guids and entries.
const MONSTERS: [(u64, u64); 10] = [
    (780006, 96003),
    (780007, 96003),
    (780008, 96003),
    (780009, 96003),
    (780010, 96004),
    (780011, 96003),
    (780012, 96004),
    (780013, 96003),
    (780014, 96004),
    (780015, 96005),
];

fn monsters() -> HashSet<u64> {
    MONSTERS
        .iter()
        .map(|&(g, entry)| HIGHGUID_UNIT << 48 | entry << 24 | g)
        .collect()
}

fn is_unit(guid: u64) -> bool {
    guid >> 48 == HIGHGUID_UNIT
}

/// The realm and the creatures its client holds.
struct Abbey {
    realm: Realm,
    seen: HashSet<u64>,
    /// Each monster's health in its latest create.
    created_health: Vec<(u32, u32)>,
    /// The character.
    me: u64,
    /// Each creature's latest create: where it stands and its fields.
    created: HashMap<u64, ([f32; 3], ObjectFields)>,
    /// The height the host's ground reports.
    floor: f32,
    /// Every notification since last cleared.
    notes: Vec<String>,
    /// How many times the client was shown the tower door, and had its state updated.
    door_creates: u32,
    door_updates: u32,
    /// Where the client was shown Brother Neals's cage.
    cage: Option<[f32; 3]>,
    /// The counter of the teleport the client has yet to answer.
    teleport: Option<u32>,
}

impl Abbey {
    fn new() -> Self {
        Self::at_level(1)
    }

    /// A human warrior of `level` in the yard.
    fn at_level(level: u8) -> Self {
        Self::made(level, false)
    }

    /// A Champion of `level` (`common::as_champion`), for a fight the test only passes through.
    fn champion(level: u8) -> Self {
        Self::made(level, true)
    }

    fn made(level: u8, champion: bool) -> Self {
        let mut maker = realm();
        let RealmCommand::CreateCharacter(mut req) = create_cmd("Pilgrim", HUMAN, 0, WARRIOR)
        else {
            unreachable!()
        };
        req.start = START_ELWYNN;
        maker.handle(RealmCommand::CreateCharacter(req));
        let me = roster(&mut maker)[0].guid;
        let blob = maker
            .export_character(me)
            .unwrap()
            .replace("level = 1\n", &format!("level = {level}\n"));
        let (mut realm, blob) = if champion {
            (realm().with_callings(CHAMPION_CALLINGS), as_champion(&blob))
        } else {
            (realm(), blob)
        };
        realm.import_character(&blob).unwrap();
        realm.handle(RealmCommand::EnterWorld { guid: me });
        let mut a = Self {
            realm,
            seen: HashSet::new(),
            created_health: Vec::new(),
            me,
            created: HashMap::new(),
            floor: 82.0,
            notes: Vec::new(),
            door_creates: 0,
            door_updates: 0,
            cage: None,
            teleport: None,
        };
        a.go(OUTSIDE, 1.0);
        a
    }

    fn tick(&mut self, secs: f32) {
        for _ in 0..(secs * 10.0).round().max(1.0) as u32 {
            self.realm.tick(0.1, &AbbeyFloor(self.floor));
            self.read();
        }
    }

    fn read(&mut self) {
        for e in drain(&mut self.realm) {
            match e {
                SessionEvent::ObjectCreate {
                    guid,
                    position,
                    fields,
                    ..
                } if is_unit(guid) => {
                    self.seen.insert(guid);
                    self.created.insert(guid, (position, fields.clone()));
                    if monsters().contains(&guid) {
                        let h = (
                            fields.unit_health().unwrap(),
                            fields.unit_max_health().unwrap(),
                        );
                        self.created_health.push(h);
                    }
                }
                SessionEvent::ObjectCreate { guid, .. } if guid == TOWER_DOOR => {
                    self.door_creates += 1;
                }
                SessionEvent::ObjectCreate { guid, position, .. } if guid == NEALS_CAGE => {
                    self.cage = Some(position);
                }
                SessionEvent::ObjectValues { guid, .. } if guid == TOWER_DOOR => {
                    self.door_updates += 1;
                }
                SessionEvent::Notification { text } => self.notes.push(text),
                SessionEvent::Teleport { counter, .. } => self.teleport = Some(counter),
                SessionEvent::ObjectsRemoved(gone) => gone.iter().for_each(|g| {
                    self.seen.remove(g);
                }),
                SessionEvent::ObjectDestroyed(g) => {
                    self.seen.remove(&g);
                }
                _ => {}
            }
        }
    }

    fn go(&mut self, xy: [f32; 2], secs: f32) {
        self.realm.handle(RealmCommand::DevTeleport {
            x: xy[0],
            y: xy[1],
            facing: None,
        });
        self.tick(secs);
        if let Some(counter) = self.teleport.take() {
            self.realm.handle(RealmCommand::TeleportAck { counter });
        }
    }

    fn refused(&mut self, guid: u64) -> bool {
        self.realm.handle(RealmCommand::AttackSwing { guid });
        drain(&mut self.realm).into_iter().any(|e| {
            matches!(
                e,
                SessionEvent::AttackSwingError(AttackSwingError::DeadOrUnattackable)
            )
        })
    }
}

#[test]
fn the_abbeys_cultists_cast_and_the_abbot_is_unchanged() {
    // The Corrupted Brother heals himself and swings 10% softer to pay for it (#469, #474).
    assert_eq!(mob_spells(96003), ["brother_heal"]);
    assert_eq!(
        mob_template(96003)["damage_multiplier"].as_float(),
        Some(0.9)
    );
    assert_eq!(
        mob_spells(96004),
        ["shadow_word_pain_creature", "shadow_bolt_creature"]
    );
    assert_eq!(mob_spells(96005), ["abbot_corruption", "abbot_heal"]);
}

#[test]
fn outside_sees_no_monster_and_inside_sees_only_the_abbeys_own() {
    let mut a = Abbey::new();
    assert!(a.seen.is_disjoint(&monsters()), "a monster shows outside");
    assert!(!a.seen.is_empty(), "the open world streamed in");
    let brother = HIGHGUID_UNIT << 48 | 96003 << 24 | 780009;
    assert!(a.refused(brother), "a monster is targetable from outside");

    a.go(INSIDE, 0.1);
    let mut abbeys = monsters();
    abbeys.insert(BROTHER_NEALS);
    assert_eq!(
        a.seen, abbeys,
        "inside holds the Abbey's monsters and Brother Neals, and nothing else"
    );
    assert!(!a.refused(brother), "a monster is not attackable inside");
}

#[test]
fn brother_neals_waits_friendly_in_a_cage_at_the_back_of_the_side_chapel() {
    let mut a = Abbey::new();
    assert!(
        !a.created.contains_key(&BROTHER_NEALS),
        "Brother Neals shows outside the Abbey"
    );
    assert_eq!(a.cage, None, "the cage shows outside the Abbey");
    a.go(INSIDE, 0.5);
    assert!(a.realm.run_elapsed().is_some(), "no Run");
    let (at, fields) = &a.created[&BROTHER_NEALS];
    let off = flat_off(*at, NEALS_AT);
    assert!(
        off <= 0.5,
        "Brother Neals stands at {at:?}, {off} yd off his row"
    );
    assert_eq!(fields.unit_faction_template(), Some(FRIENDLY_TO_ALL));
    let cage = a.cage.expect("the cage shows inside the Abbey");
    let off = flat_off(cage, NEALS_AT);
    assert!(
        off <= 1.0,
        "the cage stands at {cage:?}, {off} yd off Neals"
    );
}

fn flat_off(at: [f32; 3], to: [f32; 2]) -> f32 {
    ((at[0] - to[0]).powi(2) + (at[1] - to[1]).powi(2)).sqrt()
}

#[test]
fn the_abbot_stands_in_the_middle_of_the_tower_top() {
    let mut a = Abbey::new();
    a.go(INSIDE, 0.5);
    let (at, _) = &a.created[&ABBOT];
    let off = flat_off(*at, PLATFORM_MIDDLE);
    assert!(
        off <= 1.0,
        "the Abbot stands at {at:?}, {off} yd off the platform's middle"
    );
}

#[test]
fn the_abbot_carries_a_staff_and_nothing_else() {
    let mut a = Abbey::new();
    a.go(INSIDE, 0.5);
    let (_, fields) = &a.created[&ABBOT];
    assert_eq!(
        fields.unit_virtual_item_display(0),
        Some(STAFF_OF_THE_FRIAR_DISPLAY)
    );
    // Item class 2 (weapon), subclass 10 (staff).
    let (class, subclass, ..) = fields.unit_virtual_item_info(0).expect("a main hand");
    assert_eq!((class, subclass), (2, 10));
    for slot in [1, 2] {
        assert!(
            matches!(fields.unit_virtual_item_display(slot), None | Some(0)),
            "slot {slot} holds something"
        );
    }
}

#[test]
fn entering_starts_a_run_and_leaving_ends_it() {
    let mut a = Abbey::new();
    assert_eq!(a.realm.run_elapsed(), None);
    a.go(INSIDE, 0.5);
    assert!(a.realm.run_elapsed().is_some_and(|t| t > 0.0), "no Run");
    a.go(OUTSIDE, 0.1);
    assert_eq!(
        a.realm.run_elapsed(),
        None,
        "the Run outlived the last one out"
    );
    assert!(a.seen.is_disjoint(&monsters()));
}

#[test]
fn dying_inside_ends_the_run_and_the_monsters_come_back_at_full_health() {
    let mut a = Abbey::new();
    a.go(INSIDE, 0.1);
    let brother = HIGHGUID_UNIT << 48 | 96003 << 24 | 780009;
    a.realm.handle(RealmCommand::AttackSwing { guid: brother });
    // A level 8 Corrupted Brother against a level 1 warrior.
    for _ in 0..600 {
        a.tick(0.1);
        if a.realm.run_elapsed().is_none() {
            break;
        }
    }
    assert_eq!(a.realm.run_elapsed(), None, "dying did not end the Run");
    assert!(
        a.seen.is_disjoint(&monsters()),
        "the dead still see the monsters"
    );

    a.realm.handle(RealmCommand::RepopRequest);
    a.tick(0.1);
    a.created_health.clear();
    a.go(INSIDE, 0.1);
    assert!(
        a.realm.run_elapsed().is_some(),
        "re-entering starts a new Run"
    );
    assert_eq!(a.created_health.len(), MONSTERS.len());
    assert!(
        a.created_health.iter().all(|(h, max)| h == max),
        "{:?}",
        a.created_health
    );
}

#[test]
fn the_abbeys_npcs_are_gone_and_mcbride_waits_outside() {
    let removed: [u64; 14] = [
        198, 375, 911, 6373, 459, 925, 915, 152, 190, 1213, 78, 11940, 951, 952,
    ];
    let a = Abbey::new();
    let entries: HashSet<u64> = a.seen.iter().map(|g| g >> 24 & 0xFF_FFFF).collect();
    for entry in removed {
        assert!(!entries.contains(&entry), "{entry} still spawns");
    }
    assert!(entries.contains(&197), "Marshal McBride is gone");
}

/// The tower door (#348), a GameObject guid.
const TOWER_DOOR: u64 = 0xF110 << 48 | 790901;
/// The Abbey Tower Key.
const TOWER_KEY: u32 = 980121;
/// The key's holders, each with a phrase only its hint has.
const HOLDERS: [(u64, &str); 6] = [
    (780006, "back toward the front doors"),
    (780007, "far west end of the nave"),
    (780008, "just outside this chapel"),
    (780009, "south end of the nave"),
    (780011, "in the back wing"),
    (780013, "far end"),
];
/// The tower door's world position and the gallery's floor.
const DOOR_AT: [f32; 2] = [-8903.8, -184.5];
const GALLERY: f32 = 97.7;
/// A step across the doorway, and one along the gallery beside it.
const ACROSS: ([f32; 2], [f32; 2]) = ([-8904.73, -184.13], [-8902.87, -184.87]);
const BESIDE: ([f32; 2], [f32; 2]) = ([-8903.52, -181.06], [-8901.66, -181.80]);

impl Abbey {
    /// The low guid the key's holder has, read off Brother Neals's greeting.
    fn holder(&mut self) -> u32 {
        self.go([NEALS_AT[0] + 1.0, NEALS_AT[1]], 0.3);
        self.realm.handle(RealmCommand::GossipHello {
            guid: BROTHER_NEALS,
        });
        let events = drain(&mut self.realm);
        let text_id = events
            .iter()
            .find_map(|e| match e {
                SessionEvent::GossipMenu { text_id, .. } => Some(*text_id),
                _ => None,
            })
            .unwrap_or_else(|| panic!("Neals's menu, in {:?}", events.len()));
        text_id - 0x3000_0000
    }

    /// Kills the six brothers one at a time and loots each. The low guids of those whose corpse
    /// held the key; the key is taken into the bags.
    fn kill_the_brothers(&mut self) -> Vec<u64> {
        let mut with_key = Vec::new();
        for (low, _) in HOLDERS {
            let guid = HIGHGUID_UNIT << 48 | 96003 << 24 | low;
            let at = self.created[&guid].0;
            self.realm.handle(RealmCommand::DevTeleport {
                x: at[0] + 1.0,
                y: at[1],
                facing: Some(std::f32::consts::PI),
            });
            self.tick(0.2);
            self.realm.handle(RealmCommand::AttackSwing { guid });
            let mut items = None;
            // A corpse with nothing on it answers with a loot error; wait out the kill first.
            for _ in 0..60 {
                self.tick(0.5);
                self.realm.handle(RealmCommand::Loot { guid });
                items = drain(&mut self.realm).into_iter().find_map(|e| match e {
                    SessionEvent::LootResponse { items, .. } => Some(items),
                    _ => None,
                });
                if items.is_some() {
                    break;
                }
            }
            let items = items.unwrap_or_default();
            if let Some(key) = items.iter().find(|i| i.item_id == TOWER_KEY) {
                with_key.push(low);
                self.realm
                    .handle(RealmCommand::AutostoreLootItem { slot: key.slot });
                self.tick(0.1);
            }
            self.realm.handle(RealmCommand::LootRelease { guid });
        }
        with_key
    }

    fn holds_key(&self) -> bool {
        self.realm
            .export_character(self.me)
            .unwrap()
            .contains(&TOWER_KEY.to_string())
    }

    /// Reports a step in the gallery, from `from` to `to`.
    fn step(&mut self, (from, to): ([f32; 2], [f32; 2])) {
        self.floor = GALLERY;
        self.go(from, 0.3);
        self.realm.handle(RealmCommand::Move {
            motion: Default::default(),
            position: [to[0], to[1], GALLERY],
            orientation: 0.0,
        });
        self.tick(0.1);
    }

    fn click_door(&mut self) {
        self.floor = GALLERY;
        self.go([DOOR_AT[0] + 0.5, DOOR_AT[1]], 0.3);
        self.realm
            .handle(RealmCommand::UseGameObject { guid: TOWER_DOOR });
        self.tick(0.2);
    }
}

#[test]
fn one_brother_drops_the_key_and_neals_names_him() {
    let mut a = Abbey::champion(40);
    a.go(INSIDE, 0.5);
    let holder = a.holder();
    let with_key = a.kill_the_brothers();
    assert_eq!(with_key, vec![u64::from(holder)], "who dropped the key");
    assert!(a.holds_key());
    let hint = HOLDERS.iter().find(|h| h.0 == u64::from(holder)).unwrap().1;
    a.go([NEALS_AT[0] + 1.0, NEALS_AT[1]], 0.3);
    a.realm.handle(RealmCommand::NpcTextQuery {
        text_id: 0x3000_0000 + holder,
    });
    let text = drain(&mut a.realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::NpcGreeting { blocks, .. } => Some(blocks[0].male.clone()),
            _ => None,
        })
        .expect("the hint");
    assert!(text.contains(hint), "{text}");
}

#[test]
fn fresh_runs_pick_different_holders() {
    let mut a = Abbey::champion(40);
    let mut seen = HashSet::new();
    for _ in 0..12 {
        a.go(INSIDE, 0.3);
        seen.insert(a.holder());
        a.go(OUTSIDE, 0.3);
    }
    assert!(seen.len() > 1, "{seen:?}");
    assert!(seen
        .iter()
        .all(|h| HOLDERS.iter().any(|x| x.0 == u64::from(*h))));
}

#[test]
fn a_step_across_a_locked_doorway_is_refused_once_and_a_step_beside_it_is_not() {
    let mut a = Abbey::champion(40);
    a.go(INSIDE, 0.3);
    a.step(BESIDE);
    assert!(a.notes.is_empty(), "{:?}", a.notes);
    a.step(ACROSS);
    a.step(ACROSS);
    let locked: Vec<_> = a.notes.iter().filter(|n| n.contains("locked")).collect();
    assert_eq!(locked.len(), 1, "{:?}", a.notes);
    assert!(a.door_creates >= 1, "the door was never shown");
}

#[test]
fn a_click_without_the_key_says_locked_and_changes_nothing() {
    let mut a = Abbey::champion(40);
    a.go(INSIDE, 0.3);
    a.click_door();
    assert!(
        a.notes.iter().any(|n| n.contains("locked")),
        "{:?}",
        a.notes
    );
    assert_eq!(a.door_updates, 0);
    a.notes.clear();
    a.step(ACROSS);
    assert_eq!(a.notes.len(), 1, "the doorway should still be shut");
}

#[test]
fn the_key_unlocks_the_door_for_the_run_and_never_leaves_the_abbey() {
    let mut a = Abbey::champion(40);
    a.go(INSIDE, 0.5);
    a.kill_the_brothers();
    assert!(a.holds_key());
    a.click_door();
    assert!(!a.holds_key(), "the click should take the key");
    assert!(a.door_updates >= 1, "no open state was sent");
    a.notes.clear();
    a.step(ACROSS);
    assert!(a.notes.is_empty(), "{:?}", a.notes);

    // A new Run locks it again.
    a.floor = 82.0;
    a.go(OUTSIDE, 0.3);
    a.go(INSIDE, 0.3);
    a.step(ACROSS);
    assert_eq!(a.notes.len(), 1, "the next Run's door should be locked");
}

#[test]
fn a_key_carried_out_is_taken() {
    let mut a = Abbey::champion(40);
    a.go(INSIDE, 0.5);
    a.kill_the_brothers();
    assert!(a.holds_key());
    a.floor = 82.0;
    a.go(OUTSIDE, 0.3);
    assert!(!a.holds_key(), "the key left the Abbey");
}
