//! The Stockade (spec #575, ADR 0032 amended 2026-11-04): an instance map, 34
//! (`content/instance_maps.toml`), holding two Phased Dungeons with the Watch's open-world
//! corridor between them. Trigger 101 in the city's jail door loads the jail behind a loading
//! screen and trigger 503 inside sends the character back out to the door. The east stairs lead
//! down into the riot (dungeon 16, `content/stockade_riot.toml`), the west stairs, behind a
//! sealed door, into the quiet wing (dungeon 17, `content/stockade_quiet_wing.toml`). Dying
//! inside and a save made inside both come out at the door.

mod common;

use benilla_protocol::messages::CHAT_MSG_MONSTER_SAY;
use benilla_protocol::{SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;

/// `AreaTrigger.dbc` 101, "Stormwind Stockades - Entrance": a box in the city's jail door.
const ENTRANCE: u32 = 101;
const DOOR: [f32; 3] = [-8761.85, 848.557, 87.805];
/// `AreaTrigger.dbc` 503, "Stormwind Stockades - Exit": an 8 yd sphere inside, by the gate.
const EXIT: u32 = 503;
/// In the exit's sphere, on the gate hall's floor.
const AT_THE_EXIT: [f32; 3] = [45.0, 0.8, -16.4];

/// `Map.dbc` 34, and Eastern Kingdoms.
const STOCKADE: u32 = 34;
const EASTERN_KINGDOMS: u32 = 0;
/// Where the entrance puts the player inside, and the exit outside (`areatrigger_teleport`).
const ARRIVAL: [f32; 3] = [49.0, 0.5, -16.4];
const OUTSIDE: [f32; 3] = [-8766.1, 845.5, 88.0];
const OUTSIDE_FACING: f32 = 3.84;

/// Where the corridor crosses the hall, Thelwater's spot and the sealed door's outside; a step
/// down each stair.
const CROSSING: [f32; 3] = [131.0, 3.0, -25.5229];
const EAST_STAIRS: [f32; 3] = [131.0, -20.0, -29.0];
const WEST_STAIRS: [f32; 3] = [131.0, 20.0, -29.0];

/// Each dungeon's one creature, the Phase Tester, and the side of the hall it stands on.
const PHASE_TESTER: u32 = 97100;
const RIOT_TESTER_Y: f32 = -35.0;
const QUIET_TESTER_Y: f32 = 35.0;

/// The Watch's Hub NPCs on map 34: Warden Thelwater, Quartermaster Mae Tollin, Sergeant Hale,
/// the Watchmen and the Convicts (`content/hub.toml`).
const THELWATER: u32 = 1719;
const WATCH: [u32; 5] = [THELWATER, 190078, 190079, 190080, 190081];
/// Sergeant Hale, who speaks the west door's refusal.
const HALE: u64 = 0xF130 << 48 | 190079 << 24 | 9300074;

/// The west wing's door, the item that opens it (a stand-in until #580 makes Rook's Key) and
/// what Hale says to a character without it.
const WEST_DOOR: u64 = 0xF110 << 48 | 98004;
const KEY: u32 = 980122;
const SEALED: &str =
    "The west wing is closed by order of the Keep. The Warden holds no key to it, and neither do you.";
const PACK_1: u8 = 23;

/// A level-30 character standing in the city's jail door.
fn at_the_door() -> Party {
    door_party(false)
}

/// As [`at_the_door`], carrying the west wing's key when `key`.
fn door_party(key: bool) -> Party {
    let mut party = Party::edited(
        Realm::with_seed(1),
        &[30],
        START_ELWYNN,
        Box::new(NoGround),
        |_, blob| if key { with_key(blob) } else { blob },
    );
    party.go(0, DOOR, 0.66);
    party.run(0.5);
    party
}

/// `blob` carrying the west wing's key in its backpack and nothing else of its items.
fn with_key(blob: String) -> String {
    let mut blob = blob;
    if let Some(at) = blob.find("[[items]]") {
        blob.truncate(at);
    }
    let mut blob = blob.replace("items = []\n", "");
    blob.push_str(&format!(
        "\n[[items]]\nslot = {PACK_1}\nentry = {KEY}\ncount = 1\n"
    ));
    blob
}

/// What the client has been sent since its `from`th event.
fn since(party: &Party, from: usize) -> Vec<SessionEvent> {
    party.seen(0)[from..].to_vec()
}

/// Walks into the entrance and loads the jail, as the client does; the events from the ack on.
fn enter(party: &mut Party) -> Vec<SessionEvent> {
    party.go(0, DOOR, 0.66);
    party.cmd(0, RealmCommand::AreaTrigger { id: ENTRANCE });
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::WorldportAck);
    party.run(0.5);
    since(party, from)
}

/// Walks to `at` and lets the realm tick; the events that came.
fn walk(party: &mut Party, at: [f32; 3]) -> Vec<SessionEvent> {
    let from = party.seen(0).len();
    party.go(0, at, 0.0);
    party.run(0.5);
    since(party, from)
}

fn in_phase(party: &Party) -> bool {
    party.realm.run_elapsed_as(party.sessions[0]).is_some()
}

/// The Phase Testers created in `events`, by the y they stand at.
fn testers(events: &[SessionEvent]) -> Vec<f32> {
    unit_creates(events)
        .into_iter()
        .filter(|u| u.1 == PHASE_TESTER)
        .map(|u| u.2[1])
        .collect()
}

fn near_y(ys: &[f32], y: f32) -> bool {
    ys.iter().any(|&t| (t - y).abs() < 0.5)
}

/// The worldport in `events`: `(map, position, orientation, needs_ack)`.
fn worldport(events: &[SessionEvent]) -> Option<(u32, [f32; 3], f32, bool)> {
    events.iter().find_map(|e| match e {
        SessionEvent::Worldport {
            map_id,
            position,
            orientation,
            needs_ack,
        } => Some((*map_id, *position, *orientation, *needs_ack)),
        _ => None,
    })
}

fn transfer_pending(events: &[SessionEvent]) -> Option<u32> {
    events.iter().find_map(|e| match e {
        SessionEvent::TransferPending { map_id, .. } => Some(*map_id),
        _ => None,
    })
}

fn near(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 0.01)
}

/// Asserts that `events` worldport the player out to the jail door on Eastern Kingdoms.
fn out_at_the_door(events: &[SessionEvent]) {
    assert_eq!(transfer_pending(events), Some(EASTERN_KINGDOMS));
    let (map, at, facing, needs_ack) = worldport(events).expect("a worldport out");
    assert_eq!((map, needs_ack), (EASTERN_KINGDOMS, true));
    assert!(near(at, OUTSIDE), "{at:?}");
    assert_eq!(facing, OUTSIDE_FACING);
}

/// Hale's refusals in `events`.
fn hale_lines(events: &[SessionEvent]) -> usize {
    events
        .iter()
        .filter(|e| {
            matches!(
                e,
                SessionEvent::Chat(c)
                    if c.chat_type == CHAT_MSG_MONSTER_SAY
                        && c.sender_guid == HALE
                        && c.sender_name.as_deref() == Some("Sergeant Hale")
                        && c.text == SEALED
            )
        })
        .count()
}

/// The west door's open and shut updates in `events`.
fn door_updates(events: &[SessionEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::ObjectValues { guid, .. } if *guid == WEST_DOOR))
        .count()
}

/// The last teleport in `events`.
fn last_teleport(events: &[SessionEvent]) -> Option<[f32; 3]> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Teleport { position, .. } => Some(*position),
            _ => None,
        })
        .last()
}

/// Clicks the west door from the crossing; the events that came.
fn click_west_door(party: &mut Party) -> Vec<SessionEvent> {
    party.go(0, CROSSING, 1.57);
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::UseGameObject { guid: WEST_DOOR });
    party.run(0.2);
    since(party, from)
}

#[test]
fn walking_into_the_jail_door_loads_the_stockade_behind_a_loading_screen() {
    let mut party = at_the_door();
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::AreaTrigger { id: ENTRANCE });
    let events = since(&party, from);
    assert_eq!(transfer_pending(&events), Some(STOCKADE));
    let (map, at, facing, needs_ack) = worldport(&events).expect("a worldport into the jail");
    assert_eq!(map, STOCKADE);
    assert!(near(at, ARRIVAL), "{at:?}");
    assert_eq!(facing, 0.0);
    assert!(
        needs_ack,
        "the realm waits for the client to have loaded the jail"
    );

    // Until the client says the jail is loaded, nothing of it is sent.
    let from = party.seen(0).len();
    party.run(2.0);
    assert!(unit_creates(&since(&party, from)).is_empty());
    assert!(!in_phase(&party));
}

#[test]
fn a_trigger_claimed_from_far_away_does_nothing() {
    let mut party = at_the_door();
    party.go(0, OUTSIDE.map(|v| v + 40.0), 0.0);
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::AreaTrigger { id: ENTRANCE });
    assert_eq!(worldport(&since(&party, from)), None);
}

#[test]
fn after_the_ack_the_character_stands_in_the_watch_s_open_world() {
    let mut party = at_the_door();
    assert!(
        !unit_creates(party.seen(0)).is_empty(),
        "the city's people are in view at the door"
    );
    let events = enter(&mut party);
    assert!(!in_phase(&party), "the gate hall is open world");
    let entries: Vec<u32> = unit_creates(&events).iter().map(|u| u.1).collect();
    assert!(entries.contains(&THELWATER), "{entries:?}");
    assert!(
        entries.iter().all(|e| WATCH.contains(e)),
        "only the Watch's people, no Phase Tester and none of the city's or Goldshire's: \
         {entries:?}"
    );
}

#[test]
fn the_east_stairs_lead_into_the_riot_s_run_and_back_out() {
    let mut party = at_the_door();
    enter(&mut party);
    walk(&mut party, CROSSING);
    assert!(!in_phase(&party), "the crossing is open world");

    let events = walk(&mut party, EAST_STAIRS);
    assert!(in_phase(&party), "the east stairs are the riot's");
    let ys = testers(&events);
    assert!(near_y(&ys, RIOT_TESTER_Y), "{ys:?}");
    assert!(!near_y(&ys, QUIET_TESTER_Y), "{ys:?}");
    party.run(2.0);

    let events = walk(&mut party, CROSSING);
    assert!(!in_phase(&party), "back up in the corridor");
    assert!(unit_creates(&events).iter().any(|u| u.1 == THELWATER));

    // No one is left in the riot: the next entry starts a fresh Run.
    let events = walk(&mut party, EAST_STAIRS);
    assert!(
        near_y(&testers(&events), RIOT_TESTER_Y),
        "a fresh Phase Tester"
    );
    let second = party.realm.run_elapsed_as(party.sessions[0]);
    assert!(second.is_some_and(|s| s < 1.0), "{second:?}");
}

#[test]
fn the_west_stairs_lead_into_the_quiet_wing_s_run_with_the_key() {
    let mut party = door_party(true);
    enter(&mut party);
    let events = click_west_door(&mut party);
    assert_eq!(door_updates(&events), 1, "the door shows open");

    let events = walk(&mut party, WEST_STAIRS);
    assert!(in_phase(&party), "the west stairs are the quiet wing's");
    let ys = testers(&events);
    assert!(near_y(&ys, QUIET_TESTER_Y), "{ys:?}");
    assert!(!near_y(&ys, RIOT_TESTER_Y), "{ys:?}");
    party.run(2.0);

    walk(&mut party, CROSSING);
    assert!(!in_phase(&party));

    // The door opened for this character once, so it opens again; no one is left inside, so the
    // Run is fresh.
    click_west_door(&mut party);
    let events = walk(&mut party, WEST_STAIRS);
    assert!(
        near_y(&testers(&events), QUIET_TESTER_Y),
        "a fresh Phase Tester"
    );
    let second = party.realm.run_elapsed_as(party.sessions[0]);
    assert!(second.is_some_and(|s| s < 1.0), "{second:?}");
}

#[test]
fn the_west_door_turns_away_a_character_without_the_key_and_hale_says_why() {
    let mut party = at_the_door();
    enter(&mut party);
    walk(&mut party, CROSSING);
    assert!(party.units_held(0).contains(&HALE), "Hale is in view");

    let events = walk(&mut party, WEST_STAIRS);
    assert!(!in_phase(&party));
    assert_eq!(last_teleport(&events), Some(CROSSING), "put back outside");
    assert_eq!(hale_lines(&events), 1);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SessionEvent::Notification { text } if text == SEALED)),
        "Hale's line, not a notification"
    );

    let events = click_west_door(&mut party);
    assert_eq!(hale_lines(&events), 1, "a click gets the same line");
    assert_eq!(door_updates(&events), 0, "the door stays shut");
}

#[test]
fn the_west_door_opens_once_for_a_character_with_the_key() {
    let mut party = door_party(true);
    enter(&mut party);
    let events = click_west_door(&mut party);
    assert_eq!(hale_lines(&events), 0);
    assert_eq!(door_updates(&events), 1, "the door shows open");
    let from = party.seen(0).len();
    party.run(15.5);
    assert_eq!(door_updates(&since(&party, from)), 1, "and shuts again");
    let saved = party.realm.export_character(party.guids[0]).unwrap();
    assert!(saved.contains("doors_opened = [98004]"), "{saved}");
}

#[test]
fn the_exit_takes_the_character_out_of_the_door() {
    let mut party = at_the_door();
    enter(&mut party);
    walk(&mut party, EAST_STAIRS);
    assert!(in_phase(&party));

    walk(&mut party, AT_THE_EXIT);
    assert!(!in_phase(&party), "the gate hall is open world");
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::AreaTrigger { id: EXIT });
    out_at_the_door(&since(&party, from));
    party.cmd(0, RealmCommand::WorldportAck);
    party.run(0.3);
    assert!(!in_phase(&party));
}

#[test]
fn dying_in_a_wing_and_releasing_comes_out_at_the_door() {
    let mut party = at_the_door();
    enter(&mut party);
    walk(&mut party, EAST_STAIRS);
    assert!(in_phase(&party));
    party.realm.kill_player();
    party.run(0.2);
    assert!(!in_phase(&party), "the dead are in no dungeon's Phase");
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::RepopRequest);
    out_at_the_door(&since(&party, from));
}

#[test]
fn a_save_made_in_the_corridor_logs_back_in_at_the_door() {
    let mut party = at_the_door();
    enter(&mut party);
    walk(&mut party, CROSSING);
    let me = party.guids[0];
    let saved = party.realm.export_character(me).unwrap();
    assert!(saved.contains("map = 34\n"), "{saved}");

    // The save comes back into a realm with no Run, as after a crash.
    let mut realm = Realm::with_seed(2);
    realm.import_character(&saved).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid: me });
    let events = drain(&mut realm);
    let (map, at, facing, needs_ack) = worldport(&events).expect("the login's worldport");
    assert_eq!((map, needs_ack), (EASTERN_KINGDOMS, false));
    assert!(near(at, OUTSIDE), "{at:?}");
    assert_eq!(facing, OUTSIDE_FACING);
}

/// The 93 creature spawns of map 34 in the vmangos dump (`creature` rows, `map = 34`): guid,
/// x, y, z.
const STOCK: [(u32, f32, f32, f32); 93] = [
    (79035, 82.3245, -10.6106, -26.4396),
    (79036, 110.986, -29.1003, -26.4396),
    (79037, 77.8325, -88.7943, -33.8562),
    (79038, 129.042, -36.0354, -33.8562),
    (79039, 70.595, -83.9379, -33.8562),
    (79040, 99.3863, -37.6553, -34.7729),
    (79041, 117.7, -45.8883, -34.7729),
    (79042, 110.662, -0.797785, -25.5229),
    (79043, 88.573, -15.4623, -26.4396),
    (79044, 87.736, -28.9658, -26.4396),
    (79045, 102.687, -28.8744, -26.4396),
    (79046, 81.5075, -2.03578, -25.5229),
    (79047, 111.721, -41.2636, -34.7729),
    (79048, 110.512, -14.289, -26.4396),
    (79049, 110.796, -65.9973, -34.7729),
    (79050, 109.622, 21.0682, -26.4396),
    (79051, 110.493, 42.8924, -34.7729),
    (79052, 110.413, 30.1479, -26.4396),
    (79053, 129.414, 21.692, -29.216),
    (79054, 76.2288, 0.302129, -25.5229),
    (79055, 83.9548, 30.5974, -26.4396),
    (79056, 131.149, 3.33946, -25.5229),
    (79057, 86.8127, 21.0691, -26.4396),
    (79058, 128.622, 49.0336, -33.8562),
    (79059, 80.683, 16.7574, -26.4396),
    (79060, 131.167, 40.9354, -33.8562),
    (79061, 105.4, 70.095, -34.7729),
    (79062, 140.338, 3.08356, -25.5229),
    (79063, 129.687, -47.4302, -33.8562),
    (79064, 93.4684, 3.03369, -25.5229),
    (79065, 103.24, 12.9131, -26.4396),
    (79066, 100.916, 53.2517, -34.7729),
    (79067, 100.564, 44.3137, -34.7729),
    (79068, 127.145, -1.40234, -25.5229),
    (79069, 123.368, -61.7398, -33.8562),
    (79070, 94.8542, -60.1856, -34.7729),
    (79071, 66.9717, -90.2606, -33.8562),
    (79072, 96.4198, -54.7564, -34.7729),
    (79073, 115.026, 46.7235, -34.7729),
    (79074, 104.11, -55.8138, -34.7729),
    (79075, 124.691, -68.8523, -33.8562),
    (79076, 82.6119, -94.8407, -33.8562),
    (79077, 146.893, -42.7851, -34.8562),
    (79078, 138.566, -48.0856, -34.7729),
    (79079, 121.623, 74.722, -34.7729),
    (79080, 108.058, 80.0361, -34.7729),
    (79081, 153.705, 3.6997, -25.6062),
    (79082, 145.489, 43.3674, -34.7729),
    (79083, 150.993, 39.1095, -34.7729),
    (79084, 114.613, -86.3965, -33.9396),
    (79085, 140.158, -64.5333, -34.7729),
    (79086, 165.944, 1.0911, -25.6062),
    (79087, 134.054, 70.9823, -33.8562),
    (79088, 142.644, -71.8536, -34.8562),
    (79089, 155.148, -52.0394, -34.8562),
    (79090, 160.051, 45.709, -34.7729),
    (79091, 159.931, 39.9643, -34.8562),
    (79092, 151.641, 60.1077, -34.7729),
    (79093, 148.353, 65.0762, -34.7729),
    (79094, 161.133, 54.0252, -34.8562),
    (79095, 141.995, 84.1125, -33.9396),
    (79096, 162.726, 59.1677, -34.8562),
    (79097, 152.804, -69.5129, -34.7729),
    (79098, 105.523, -105.795, -35.1062),
    (79099, 95.4846, -105.947, -35.1062),
    (79100, 150.261, -76.7672, -34.8562),
    (79101, 109.097, -114.239, -35.1062),
    (79102, 148.538, 103.18, -35.1896),
    (79103, 171.119, 93.3214, -33.8562),
    (79104, 155.673, 99.6374, -35.1896),
    (79105, 123.941, 119.574, -33.8562),
    (79106, 129.951, 122.361, -33.8562),
    (79107, 147.452, 111.814, -35.1896),
    (79108, 156.316, 109.516, -35.1896),
    (79109, 189.206, 87.3162, -33.8562),
    (79110, 182.74, 95.2994, -33.8562),
    (79111, 186.274, 86.9216, -33.8562),
    (79112, 192.771, 91.34, -33.8562),
    (79113, 119.78, 128.785, -33.9396),
    (79114, 161.07, 129.292, -33.8562),
    (79115, 167.363, 127.883, -33.8562),
    (79116, 166.789, 134.787, -33.9396),
    (79117, 171.059, 142.025, -33.9396),
    (79118, 168.987, 147.931, -33.9396),
    (79119, 176.087, 142.688, -33.8562),
    (79120, 126.276, -116.828, -33.9396),
    (79121, 132.264, -123.557, -33.9396),
    (79122, 141.424, -120.264, -33.9396),
    (79123, 139.681, -128.899, -33.8562),
    (79124, 94.4828, -125.829, -33.9396),
    (79125, 91.7954, -126.047, -33.9396),
    (79126, 95.3684, -128.729, -33.9396),
    (84027, 159.582, 1.25311, -25.6062),
];

/// A box of a footprint: `[lo, hi]` on x, y and z.
type Box3 = [[f32; 2]; 3];

/// The footprint boxes of a dungeon's content file.
fn boxes(file: &str) -> Vec<Box3> {
    let d: toml::Table = file.parse().unwrap();
    let pair = |v: &toml::Value| {
        let a = v.as_array().unwrap();
        [
            a[0].as_float().unwrap() as f32,
            a[1].as_float().unwrap() as f32,
        ]
    };
    d["footprint"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| [pair(&b["x"]), pair(&b["y"]), pair(&b["z"])])
        .collect()
}

fn inside(boxes: &[Box3], p: [f32; 3]) -> bool {
    boxes
        .iter()
        .any(|b| (0..3).all(|i| (b[i][0]..=b[i][1]).contains(&p[i])))
}

#[test]
fn the_stock_spawns_fall_in_the_wings_and_the_corridor_in_neither() {
    let riot = boxes(include_str!("../content/stockade_riot.toml"));
    let quiet = boxes(include_str!("../content/stockade_quiet_wing.toml"));
    let (mut east, mut west, mut corridor) = (0, 0, 0);
    for (guid, x, y, z) in STOCK {
        let p = [x, y, z];
        // The corridor and its cells are the level above the hall; a point there counts as the
        // corridor's, wherever its y.
        if y.abs() <= 34.0 && z > -28.0 {
            corridor += 1;
            assert!(!inside(&riot, p) && !inside(&quiet, p), "{guid}: corridor");
        } else if y < -30.0 {
            east += 1;
            assert!(inside(&riot, p) && !inside(&quiet, p), "{guid}: east wing");
        } else if y > 30.0 {
            west += 1;
            assert!(inside(&quiet, p) && !inside(&riot, p), "{guid}: west wing");
        }
    }
    assert_eq!(
        (east, west, corridor),
        (33, 37, 22),
        "one more on the west stairs"
    );
}
