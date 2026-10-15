//! The Local Realm's player create against a real vanilla server's.
//!
//! The client was written against real servers, so a field it reads from the self player
//! (`UNIT_FIELD_FLAGS`'s `PLAYER_CONTROLLED` bit, say, on which `CanAttack` picks its arm) is one
//! a real server sets. The realm builds that create by hand, and a field it forgets reads 0: no
//! error, just a client that behaves as on no real server (every neutral mob green, in the
//! playtest that prompted this check).
//!
//! No wire capture of a real player create is in the repository; the closest thing is the
//! `vplates` capture fixture's self player, four fields modelled on one. [`REAL_PLAYER`] widens
//! it to everything vmangos (`core@development`) sets on a level-1 human male warrior's create
//! with the default look: `Player::LoadFromDB`, the `Player::InitStatsForLevel` it calls, and the
//! stat updates that follow. Its rows are cited inline; indices are `UpdateFields_1_12_1.h`'s.

use benilla_protocol::{CharCreateReq, EntityKind, SessionEvent};
use std::collections::BTreeMap;
use twilight_realm::{Host, Realm, RealmCommand};

const fn float(v: f32) -> u32 {
    v.to_bits()
}

/// Every field a real server's create carries for a level-1 human male warrior with the default
/// look, at its real value. A create carries only nonzero fields, so a field absent here is one
/// the real server leaves 0. Inventory, visible items, quest log and further skill lines are left
/// out: all per-character, they have tests of their own in the realm. The values of the
/// per-character rows [`ALLOWED`] lists are illustrative; only their presence is checked.
const REAL_PLAYER: &[(u16, u32)] = &[
    (0, 0x1),                   // OBJECT_FIELD_GUID, low word
    (2, 0x19),                  // OBJECT_FIELD_TYPE: object | unit | player
    (4, float(1.0)),            // OBJECT_FIELD_SCALE_X (InitPlayerDisplayIds)
    (22, 60),                   // UNIT_FIELD_HEALTH: base 20 + stamina 22
    (28, 60),                   // UNIT_FIELD_MAXHEALTH
    (30, 1000),                 // UNIT_FIELD_MAXPOWER2: rage
    (34, 1),                    // UNIT_FIELD_LEVEL
    (35, 1),                    // UNIT_FIELD_FACTIONTEMPLATE: human
    (36, 1 | 1 << 8 | 1 << 24), // UNIT_FIELD_BYTES_0: human, warrior, male, rage
    (46, 0x8),                  // UNIT_FIELD_FLAGS: UNIT_FLAG_PLAYER_CONTROLLED
    (126, 2000),                // UNIT_FIELD_BASEATTACKTIME, main hand
    (129, float(0.306)),        // UNIT_FIELD_BOUNDINGRADIUS (human male model data)
    (130, float(1.5)),          // UNIT_FIELD_COMBATREACH
    (131, 49),                  // UNIT_FIELD_DISPLAYID: human male
    (132, 49),                  // UNIT_FIELD_NATIVEDISPLAYID
    (134, float(5.0)),          // UNIT_FIELD_MINDAMAGE
    (135, float(7.0)),          // UNIT_FIELD_MAXDAMAGE
    (138, 0xEE << 8),           // UNIT_FIELD_BYTES_1: pet loyalty 0xEE (LoadFromDB)
    (145, float(1.0)),          // UNIT_MOD_CAST_SPEED (InitStatsForLevel)
    (150, 23),                  // UNIT_FIELD_STAT0: strength
    (151, 20),                  // agility
    (152, 22),                  // stamina
    (153, 20),                  // intellect
    (154, 21),                  // spirit
    (155, 40),                  // UNIT_FIELD_RESISTANCES[0]: armor, agility × 2
    (163, 20),                  // UNIT_FIELD_BASE_HEALTH
    (164, (0x08 | 0x20) << 8),  // UNIT_FIELD_BYTES_2: misc flags UNK3 | UNK5 (LoadFromDB)
    (165, 26),                  // UNIT_FIELD_ATTACK_POWER: strength × 2 − 20
    (194, 0xEE << 8 | 1 << 24), // PLAYER_BYTES_2: unk 0xEE, REST_STATE_NORMAL
    (717, 400),                 // PLAYER_NEXT_LEVEL_XP
    (718, 162),                 // PLAYER_SKILL_INFO_1_1: the first skill line
    (719, 5 | 5 << 16),         // its value and maximum
    (1103, 2),                  // PLAYER_CHARACTER_POINTS2: free primary professions
    (1215, float(1.0)),         // PLAYER_FIELD_MOD_DAMAGE_DONE_PCT, seven schools
    (1216, float(1.0)),
    (1217, float(1.0)),
    (1218, float(1.0)),
    (1219, float(1.0)),
    (1220, float(1.0)),
    (1221, float(1.0)),
    (1261, u32::MAX), // PLAYER_FIELD_WATCHED_FACTION_INDEX: -1, none
];

/// How a field the real server sets may differ in the realm's create.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Allowed {
    /// The realm sends it, but the value is the character's own; only its presence is checked.
    PerCharacter,
    /// The realm may leave it out.
    Absent,
    /// The realm sends it, and may lack these bits of the real value.
    Bits(u32),
}

/// The one allow-list: each field of [`REAL_PLAYER`] the realm's create may disagree on, and why.
/// Every other field must be present with every bit the real server sets.
const ALLOWED: &[(u16, Allowed, &str)] = &[
    (0, Allowed::PerCharacter, "the guid is the character's"),
    (
        22,
        Allowed::PerCharacter,
        "health follows the Calling's stats, not vanilla's",
    ),
    (
        28,
        Allowed::PerCharacter,
        "max health follows the Calling's stats",
    ),
    (34, Allowed::PerCharacter, "the level is the character's"),
    (126, Allowed::PerCharacter, "the swing time is the weapon's"),
    (
        129,
        Allowed::PerCharacter,
        "the realm uses one bounding radius for every model",
    ),
    (134, Allowed::PerCharacter, "the damage is the weapon's"),
    (135, Allowed::PerCharacter, "the damage is the weapon's"),
    (150, Allowed::PerCharacter, "stats follow the Calling"),
    (151, Allowed::PerCharacter, "stats follow the Calling"),
    (152, Allowed::PerCharacter, "stats follow the Calling"),
    (153, Allowed::PerCharacter, "stats follow the Calling"),
    (154, Allowed::PerCharacter, "stats follow the Calling"),
    (155, Allowed::PerCharacter, "armor is the gear's"),
    (
        163,
        Allowed::PerCharacter,
        "base health follows the Calling",
    ),
    (
        165,
        Allowed::PerCharacter,
        "attack power follows the Calling",
    ),
    (
        717,
        Allowed::PerCharacter,
        "the XP to the next level is the level's",
    ),
    (
        718,
        Allowed::PerCharacter,
        "the skill lines are the character's",
    ),
    (
        719,
        Allowed::PerCharacter,
        "the skill lines are the character's",
    ),
    (
        138,
        Allowed::Absent,
        "pet loyalty on a player: a pre-2.x leftover vmangos itself questions; no client reads it",
    ),
    (
        145,
        Allowed::Absent,
        "only the stock spell tooltip scales cast times by it; this client's reads no field",
    ),
    (
        164,
        Allowed::Absent,
        "UNIT_BYTE2_FLAG_UNK3 and UNK5 have no known meaning in 1.12 and no reader here",
    ),
    (
        194,
        Allowed::Bits(0xEE << 8),
        "byte 1 of PLAYER_BYTES_2 is vmangos's `UNK1`, which no client reads",
    ),
    (
        1103,
        Allowed::Absent,
        "Twilight's Eve has no professions to learn",
    ),
];

/// The real server's `field`, from [`REAL_PLAYER`]; the test's own lookups, and nothing else.
fn real(field: u16) -> Option<u32> {
    REAL_PLAYER
        .iter()
        .find_map(|&(f, v)| (f == field).then_some(v))
}

/// Flat ground at `z = 0` with nothing in the way.
struct FlatGround;

impl Host for FlatGround {
    fn ground_height(&self, _map: u32, _x: f32, _y: f32, _z_hint: f32) -> Option<f32> {
        Some(0.0)
    }
    fn line_of_sight(&self, _map: u32, _from: [f32; 3], _to: [f32; 3]) -> bool {
        true
    }
}

/// The nonzero fields of the self player's create, as the glue would push them: a human male
/// warrior created, listed and entered through the realm's commands.
fn realm_player_create() -> BTreeMap<u16, u32> {
    let mut realm = Realm::new();
    realm.handle(RealmCommand::CreateCharacter(CharCreateReq {
        name: "Benilla".into(),
        race: 1,
        class: 1,
        gender: 0,
        skin: 0,
        face: 0,
        hair_style: 0,
        hair_color: 0,
        facial_hair: 0,
    }));
    realm.handle(RealmCommand::ListCharacters);
    let guid = realm
        .drain_events()
        .into_iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::CharacterList { characters, .. } => {
                characters.into_iter().find(|c| c.name == "Benilla")
            }
            _ => None,
        })
        .expect("the created character is listed")
        .guid;
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    realm
        .drain_events()
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid: g,
                kind: EntityKind::Player,
                fields,
                ..
            } if g == guid => Some(fields),
            _ => None,
        })
        .expect("the self player's create")
        .raw_fields()
        .filter(|&(_, v)| v != 0)
        .collect()
}

#[test]
fn the_realm_creates_the_player_with_every_field_a_real_server_sets() {
    let ours = realm_player_create();
    let mut gaps = Vec::new();
    for &(field, theirs) in REAL_PLAYER {
        let allowed = ALLOWED
            .iter()
            .find_map(|&(f, a, _)| (f == field).then_some(a));
        let (v, may_lack) = match (ours.get(&field), allowed) {
            (_, Some(Allowed::Absent)) | (Some(_), Some(Allowed::PerCharacter)) => continue,
            (None, _) => {
                gaps.push(format!("field {field} missing (real: {theirs:#x})"));
                continue;
            }
            (Some(&v), Some(Allowed::Bits(bits))) => (v, bits),
            (Some(&v), None) => (v, 0),
        };
        let lacking = theirs & !v & !may_lack;
        if lacking != 0 {
            gaps.push(format!(
                "field {field} is {v:#x}, lacking bits {lacking:#x} of the real {theirs:#x}"
            ));
        }
    }
    assert!(
        gaps.is_empty(),
        "the realm's player create differs from a real server's:\n{}",
        gaps.join("\n")
    );
}

#[test]
fn every_allowance_names_a_real_field_once_and_says_why() {
    for (i, &(field, _, why)) in ALLOWED.iter().enumerate() {
        assert!(
            real(field).is_some(),
            "allowance for field {field}, which no real create sets"
        );
        assert!(
            !why.is_empty(),
            "allowance for field {field} gives no reason"
        );
        assert!(
            ALLOWED[..i].iter().all(|&(f, _, _)| f != field),
            "field {field} is allowed twice"
        );
    }
}

/// The `vplates` fixture's self player is copied from a real create: it must agree with
/// [`REAL_PLAYER`] on every field but the level, which the fixture raises to 2 for its yellow con.
#[cfg(feature = "dev")]
#[test]
fn the_vplates_fixture_agrees_with_the_real_player() {
    for &(field, value) in &crate::capture::VPLATES_SELF_PLAYER {
        let theirs = real(field).expect("a field the real create sets");
        if field != 34 {
            assert_eq!(
                value & !theirs,
                0,
                "field {field}: {value:#x} vs {theirs:#x}"
            );
        }
    }
}
