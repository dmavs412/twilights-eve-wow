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
/// down each stair. Each step stays out of reach of the pack at its stairs' foot.
const CROSSING: [f32; 3] = [131.0, 3.0, -25.5229];
const EAST_STAIRS: [f32; 3] = [131.0, -12.0, -27.0];
const WEST_STAIRS: [f32; 3] = [131.0, 12.0, -27.0];

/// The Phase Tester, which neither wing holds any more.
const PHASE_TESTER: u32 = 97100;

/// The quiet wing's templates (`content/mobs.toml`): the Writ Warder, the Twilight Listener, the
/// Hushed, Sister Quill, the Hush and the four witnesses.
const WARDER: u32 = 97206;
const QUILL: u32 = 97209;
const HUSH: u32 = 97210;
const WITNESSES: [u32; 4] = [97211, 97212, 97213, 97214];
const QUIET_TEMPLATES: std::ops::RangeInclusive<u32> = 97206..=97214;
/// The drain grate, there once Quill is dead, and her creature row.
const GRATE: u32 = 983011;
const QUILL_GUID: i64 = 973111;
/// Where Quill stands in the scribe's room, and a spot in her reach.
const QUILL_AT: [f32; 3] = [148.5, 103.2, -35.1896];
const BY_QUILL: [f32; 3] = [147.0, 103.0, -35.1896];

/// The riot's templates (`content/mobs.toml`): the Defias Rioter, the Defias Ringleader, Varl
/// Cutter, the Bought Watchman, Captain Ansel Rook and Bertram Ash.
const RIOTER: u32 = 97200;
const CUTTER: u32 = 97202;
const ROOK: u32 = 97204;
const ASH: u32 = 97205;
const RIOT_TEMPLATES: std::ops::RangeInclusive<u32> = 97200..=97205;

/// The riot's levers and portcullises, and what a pull says.
const LEVER_A: u32 = 983001;
const PORTCULLIS_A: u32 = 983002;
const LEVER_B: u32 = 983003;
const LEVER_USED: &str = "The portcullis drops. That row is locked.";
/// In reach of Lever A, and beside Cutter.
const BY_LEVER_A: [f32; 3] = [121.0, -46.0, -34.7729];
const BY_CUTTER: [f32; 3] = [111.0, -78.5, -33.9396];
/// Where Cutter's calls out of row A and row B come in.
const ROW_A_CALL: [f32; 3] = [117.5, -48.0, -34.7729];
const ROW_B_CALL: [f32; 3] = [104.5, -94.0, -33.9396];
/// The first low guid of a creature a timed call has brought in.
const TIMED_LIVE_BASE: u64 = 0xC0_0000;

/// The Watch's Hub NPCs on map 34: Warden Thelwater, Quartermaster Mae Tollin, Sergeant Hale,
/// the Watchmen and the Convicts (`content/hub.toml`).
const THELWATER: u32 = 1719;
const WATCH: [u32; 5] = [THELWATER, 190078, 190079, 190080, 190081];
/// Sergeant Hale, who speaks the west door's refusal.
const HALE: u64 = 0xF130 << 48 | 190079 << 24 | 9300074;

/// The west wing's door, the item that opens it (Rook's Key, which The Quiet Wing gives) and
/// what Hale says to a character without it.
const WEST_DOOR: u64 = 0xF110 << 48 | 98004;
const KEY: u32 = 980129;
const SEALED: &str =
    "The west wing is closed by order of the Keep. The Warden holds no key to it, and neither do you.";
const PACK_1: u8 = 23;

/// A level-30 character standing in the city's jail door.
fn at_the_door() -> Party {
    door_party(false)
}

/// As [`at_the_door`], carrying the west wing's key when `key`.
fn door_party(key: bool) -> Party {
    door_party_at(30, key)
}

/// As [`door_party`], the character at `level`.
fn door_party_at(level: u8, key: bool) -> Party {
    let mut party = Party::edited(
        Realm::with_seed(1),
        &[level],
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
    carrying(blob, &[KEY])
}

/// `blob` carrying one of each of `items` in its backpack, from its first slot, and nothing else
/// of its items.
fn carrying(blob: String, items: &[u32]) -> String {
    let mut blob = blob;
    if let Some(at) = blob.find("[[items]]") {
        blob.truncate(at);
    }
    let mut blob = blob.replace("items = []\n", "");
    for (slot, entry) in (PACK_1..).zip(items) {
        blob.push_str(&format!(
            "\n[[items]]\nslot = {slot}\nentry = {entry}\ncount = 1\n"
        ));
    }
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

/// How many of the quiet wing's creatures are created in `events`.
fn quiet_ones(events: &[SessionEvent]) -> usize {
    unit_creates(events)
        .iter()
        .filter(|u| QUIET_TEMPLATES.contains(&u.1))
        .count()
}

/// The Writ Warders created in `events`.
fn warders(events: &[SessionEvent]) -> usize {
    unit_creates(events)
        .iter()
        .filter(|u| u.1 == WARDER)
        .count()
}

/// How many of the riot's creatures are created in `events`.
fn rioters(events: &[SessionEvent]) -> usize {
    unit_creates(events)
        .iter()
        .filter(|u| RIOT_TEMPLATES.contains(&u.1))
        .count()
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
    assert!(rioters(&events) > 0, "none of the riot in view");
    assert_eq!(quiet_ones(&events), 0, "the quiet wing in the riot");
    party.run(2.0);

    let events = walk(&mut party, CROSSING);
    assert!(!in_phase(&party), "back up in the corridor");
    assert!(unit_creates(&events).iter().any(|u| u.1 == THELWATER));

    // No one is left in the riot: the next entry starts a fresh Run.
    let events = walk(&mut party, EAST_STAIRS);
    assert!(rioters(&events) > 0, "a fresh riot");
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
    assert!(warders(&events) > 0, "none of the quiet wing in view");
    assert_eq!(rioters(&events), 0, "the riot in the quiet wing");
    party.run(2.0);

    walk(&mut party, CROSSING);
    assert!(!in_phase(&party));

    // The door opened for this character once, so it opens again; no one is left inside, so the
    // Run is fresh.
    click_west_door(&mut party);
    let events = walk(&mut party, WEST_STAIRS);
    assert!(warders(&events) > 0, "a fresh quiet wing");
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

// The riot (#578): Varl Cutter, Captain Rook, the two rows and their levers.

fn riot() -> toml::Table {
    include_str!("../content/stockade_riot.toml")
        .parse()
        .unwrap()
}

fn rows<'a>(file: &'a toml::Table, kind: &str) -> &'a [toml::Value] {
    file.get(kind)
        .and_then(|v| v.as_array())
        .map_or(&[][..], |a| a.as_slice())
}

fn point(v: &toml::Value) -> [f32; 3] {
    let a = v.as_array().unwrap();
    let f = |x: &toml::Value| x.as_float().or(x.as_integer().map(|i| i as f64)).unwrap() as f32;
    [f(&a[0]), f(&a[1]), f(&a[2])]
}

fn int(v: &toml::Value, key: &str) -> Option<i64> {
    v.get(key).and_then(|x| x.as_integer())
}

#[test]
fn every_creature_and_gameobject_of_the_riot_stands_in_its_footprint() {
    let file = riot();
    let footprint = boxes(include_str!("../content/stockade_riot.toml"));
    let creatures = rows(&file, "creature");
    assert!(creatures.len() >= 26, "{} creatures", creatures.len());
    assert!(
        creatures
            .iter()
            .all(|c| int(c, "entry") != Some(PHASE_TESTER.into())),
        "the Phase Tester is still in the riot"
    );
    for c in creatures {
        let guid = int(c, "guid").unwrap();
        let at = point(&c["position"]);
        assert!(inside(&footprint, at), "creature {guid} at {at:?}");
        for call in rows(c.as_table().unwrap(), "every") {
            let at = point(&call["position"]);
            assert!(inside(&footprint, at), "creature {guid}'s call at {at:?}");
        }
    }
    for o in rows(&file, "gameobject") {
        let at = point(&o["position"]);
        assert!(
            inside(&footprint, at),
            "GameObject {:?} at {at:?}",
            o["guid"]
        );
    }
}

#[test]
fn each_lever_of_the_riot_drops_exactly_one_portcullis() {
    let file = riot();
    let objects = rows(&file, "gameobject");
    let levers: Vec<i64> = objects
        .iter()
        .filter(|o| o.get("use").and_then(|u| u.as_str()) == Some("lever"))
        .map(|o| int(o, "guid").unwrap())
        .collect();
    assert_eq!(levers, [i64::from(LEVER_A), i64::from(LEVER_B)]);
    for lever in levers {
        let portcullises = objects
            .iter()
            .filter(|o| int(o, "after_used") == Some(lever))
            .count();
        assert_eq!(portcullises, 1, "lever {lever}");
    }
}

#[test]
fn cutter_calls_rioters_from_row_a_until_lever_a_and_from_row_b_until_lever_b() {
    let file = riot();
    let cutters: Vec<&toml::Value> = rows(&file, "creature")
        .iter()
        .filter(|c| int(c, "entry") == Some(CUTTER.into()))
        .collect();
    assert_eq!(cutters.len(), 1);
    let calls = rows(cutters[0].as_table().unwrap(), "every");
    let stops: Vec<Option<i64>> = calls.iter().map(|c| int(c, "until_used")).collect();
    assert_eq!(stops, [Some(LEVER_A.into()), Some(LEVER_B.into())]);
    for call in calls {
        assert_eq!(int(call, "entry"), Some(RIOTER.into()));
        assert_eq!(int(call, "count"), Some(2));
    }
    assert_eq!(point(&calls[0]["position"]), ROW_A_CALL);
    assert_eq!(point(&calls[1]["position"]), ROW_B_CALL);
}

#[test]
fn rook_raises_the_boss_bar_and_ash_is_friendly() {
    assert_eq!(mob_template(ROOK)["boss"].as_bool(), Some(true));
    let ash = mob_template(ASH);
    assert_eq!(ash["hostile"].as_bool(), Some(false));
    assert_eq!(ash["friendly"].as_bool(), Some(true));
}

#[test]
fn every_hostile_creature_of_the_riot_has_an_aggro_line() {
    let entries: std::collections::BTreeSet<u32> = rows(&riot(), "creature")
        .iter()
        .map(|c| int(c, "entry").unwrap() as u32)
        .collect();
    assert_eq!(
        entries.iter().copied().collect::<Vec<_>>(),
        RIOT_TEMPLATES.collect::<Vec<_>>()
    );
    for entry in entries {
        let t = mob_template(entry);
        if t.get("hostile").and_then(|h| h.as_bool()) == Some(false) {
            continue;
        }
        let lines = t
            .get("say")
            .and_then(|s| s.get("aggro"))
            .and_then(|a| a.as_array())
            .map_or(0, |a| a.len());
        assert!(lines > 0, "{entry} has no aggro line");
    }
    let rioter_lines = mob_template(RIOTER)["say"]["aggro"].as_array().unwrap();
    let distinct: std::collections::BTreeSet<&str> =
        rioter_lines.iter().map(|l| l.as_str().unwrap()).collect();
    assert_eq!(distinct.len(), 3, "the Rioters shout three lines");
}

/// A level-60 character in the riot, at the foot of the east stairs.
fn in_the_riot() -> Party {
    let mut party = door_party_at(60, false);
    enter(&mut party);
    walk(&mut party, CROSSING);
    walk(&mut party, EAST_STAIRS);
    assert!(in_phase(&party));
    party
}

/// Pulls Cutter and lets him fight for 21 s, past his first calls; where the Rioters they
/// brought came in.
fn cutter_s_calls(party: &mut Party) -> Vec<[f32; 3]> {
    let (cutter, at) = party.unit(0, CUTTER, BY_CUTTER);
    party.go_facing(0, BY_CUTTER, at);
    party.run(0.2);
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::AttackSwing { guid: cutter });
    party.run(0.2);
    party.cmd(0, RealmCommand::AttackStop);
    party.run(21.0);
    unit_creates(&since(party, from))
        .into_iter()
        .filter(|u| u.1 == RIOTER && u.0 & 0xFF_FFFF >= TIMED_LIVE_BASE)
        .map(|u| u.2)
        .collect()
}

fn came_in_at(calls: &[[f32; 3]], at: [f32; 3]) -> usize {
    calls
        .iter()
        .filter(|&&p| flat_distance(p, at) < 1.0)
        .count()
}

#[test]
fn with_both_rows_unbarred_cutter_calls_two_rioters_out_of_each() {
    let mut party = in_the_riot();
    let calls = cutter_s_calls(&mut party);
    assert_eq!(calls.len(), 4, "{calls:?}");
    assert_eq!(came_in_at(&calls, ROW_A_CALL), 2, "{calls:?}");
    assert_eq!(came_in_at(&calls, ROW_B_CALL), 2, "{calls:?}");
}

#[test]
fn with_lever_a_pulled_cutter_calls_rioters_only_out_of_row_b() {
    let mut party = in_the_riot();
    let portcullis = 0xF110 << 48 | u64::from(PORTCULLIS_A);
    walk(&mut party, BY_LEVER_A);
    let from = party.seen(0).len();
    party.cmd(
        0,
        RealmCommand::UseGameObject {
            guid: 0xF110 << 48 | u64::from(LEVER_A),
        },
    );
    party.run(0.5);
    let events = since(&party, from);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SessionEvent::Notification { text } if text == LEVER_USED)),
        "no pull"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SessionEvent::ObjectCreate { guid, .. } if *guid == portcullis)),
        "the portcullis did not drop"
    );

    let calls = cutter_s_calls(&mut party);
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert_eq!(came_in_at(&calls, ROW_B_CALL), 2, "{calls:?}");
}

// The quiet wing (#582): the witnesses, Sister Quill, the Hush and the drain grate.

fn quiet_wing() -> toml::Table {
    include_str!("../content/stockade_quiet_wing.toml")
        .parse()
        .unwrap()
}

#[test]
fn every_creature_and_gameobject_of_the_quiet_wing_stands_in_its_footprint() {
    let file = quiet_wing();
    let footprint = boxes(include_str!("../content/stockade_quiet_wing.toml"));
    let creatures = rows(&file, "creature");
    assert!(creatures.len() >= 23, "{} creatures", creatures.len());
    for c in creatures {
        let guid = int(c, "guid").unwrap();
        assert_ne!(
            int(c, "entry"),
            Some(PHASE_TESTER.into()),
            "the Phase Tester is still in the quiet wing"
        );
        assert!((973101..=973199).contains(&guid), "guid {guid}");
        let at = point(&c["position"]);
        assert!(inside(&footprint, at), "creature {guid} at {at:?}");
    }
    let objects = rows(&file, "gameobject");
    assert!(!objects.is_empty(), "no grate");
    for o in objects {
        let at = point(&o["position"]);
        assert!(
            inside(&footprint, at),
            "GameObject {:?} at {at:?}",
            o["guid"]
        );
    }
}

#[test]
fn no_creature_guid_repeats_across_the_two_wings() {
    let mut seen = std::collections::BTreeSet::new();
    for file in [riot(), quiet_wing()] {
        for c in rows(&file, "creature") {
            let guid = int(c, "guid").unwrap();
            assert!(seen.insert(guid), "guid {guid} in both");
        }
    }
}

#[test]
fn no_template_of_the_quiet_wing_has_an_aggro_line() {
    let entries: std::collections::BTreeSet<u32> = rows(&quiet_wing(), "creature")
        .iter()
        .map(|c| int(c, "entry").unwrap() as u32)
        .collect();
    assert_eq!(
        entries.iter().copied().collect::<Vec<_>>(),
        QUIET_TEMPLATES.collect::<Vec<_>>()
    );
    for entry in entries {
        let aggro = mob_template(entry).get("say").and_then(|s| s.get("aggro"));
        assert!(aggro.is_none(), "{entry} has an aggro line: {aggro:?}");
    }
}

#[test]
fn the_hush_raises_the_boss_bar_and_the_four_witnesses_are_friendly() {
    assert_eq!(mob_template(HUSH)["boss"].as_bool(), Some(true));
    let file = quiet_wing();
    for w in WITNESSES {
        let t = mob_template(w);
        assert_eq!(t["hostile"].as_bool(), Some(false), "{w}");
        assert_eq!(t["friendly"].as_bool(), Some(true), "{w}");
        let cells = rows(&file, "creature")
            .iter()
            .filter(|c| int(c, "entry") == Some(w.into()))
            .count();
        assert_eq!(cells, 1, "{w} stands in one cell");
    }
}

/// Whether player 0's client holds the GameObject `guid`, by what it has been sent.
fn shows(p: &Party, guid: u64) -> bool {
    let mut shown = false;
    for e in p.seen(0) {
        match e {
            SessionEvent::ObjectCreate { guid: g, .. } if *g == guid => shown = true,
            SessionEvent::ObjectsRemoved(gone) if gone.contains(&guid) => shown = false,
            SessionEvent::ObjectDestroyed(g) if *g == guid => shown = false,
            _ => {}
        }
    }
    shown
}

#[test]
fn the_drain_grate_is_there_only_once_quill_is_dead() {
    let file = quiet_wing();
    let grate = rows(&file, "gameobject")
        .iter()
        .find(|o| int(o, "guid") == Some(GRATE.into()))
        .expect("the grate");
    assert_eq!(int(grate, "after_dead"), Some(QUILL_GUID));
    let quill_row = rows(&file, "creature")
        .iter()
        .find(|c| int(c, "guid") == Some(QUILL_GUID))
        .expect("Quill's row");
    assert_eq!(int(quill_row, "entry"), Some(QUILL.into()));

    let mut party = door_party_at(60, true);
    enter(&mut party);
    click_west_door(&mut party);
    walk(&mut party, WEST_STAIRS);
    assert!(in_phase(&party));
    let grate = 0xF110 << 48 | u64::from(GRATE);
    let (quill, at) = party.unit(0, QUILL, QUILL_AT);
    party.go_facing(0, BY_QUILL, at);
    party.run(0.5);
    assert!(!shows(&party, grate), "the grate while Quill lives");

    party.realm.set_creature_health_pct(quill, 1);
    party.cmd(0, RealmCommand::AttackSwing { guid: quill });
    for _ in 0..40 {
        party.run(0.5);
        if died(party.seen(0), quill) {
            break;
        }
    }
    assert!(died(party.seen(0), quill), "Quill did not die");
    party.run(0.5);
    assert!(shows(&party, grate), "the grate once Quill is dead");
}

// Quill's spells, tried on the Gaze Hall's Vile Familiar (`content/gaze_hall.toml`), as
// `tests/creature_nova_fear.rs` tries the test spells.

const FAMILIAR: u32 = 3101;
const CASTER: [f32; 3] = [-9650.0, 20.0, 47.2];
/// In melee reach of the familiar, so it never leaves its spot.
const EXPOSED: [f32; 3] = [-9650.0, 21.5, 47.2];
/// 6 yd south of the familiar, inside Ink's 8.
const SIX_YD: [f32; 3] = [-9650.0, 14.0, 47.2];
/// In the Phase but 16 yd north, outside Ink's 8.
const SIXTEEN_YD: [f32; 3] = [-9650.0, 36.0, 47.2];
const SPELL_FAILED_SILENCED: u8 = 0x60;
const INK_WARNING: &str = "Quill's ink runs across the floor!";

#[test]
fn quill_s_take_down_silences_a_character_for_4_s() {
    let realm = realm().with_creature_spells(FAMILIAR, &["take_down_quill"]);
    let mut p = Party::starting(realm, &[20], START_ELWYNN, Box::new(NoGround));
    p.go_facing(0, EXPOSED, CASTER);
    p.run(0.5);
    // A cast tried every 0.1 s for 10 s: when each was refused for the silence.
    let mut refused = Vec::new();
    for tick in 0..100 {
        let before = p.seen(0).len();
        p.cmd(
            0,
            RealmCommand::CastSpell {
                spell_id: 1,
                target: None,
            },
        );
        let silenced = p.seen(0)[before..].iter().any(|e| {
            matches!(e, SessionEvent::CastResult { spell_id: 1, reason: Some(r), .. }
                if *r == SPELL_FAILED_SILENCED)
        });
        if silenced {
            refused.push(tick as f32 * 0.1);
        }
        p.run(0.1);
    }
    let (Some(&first), Some(&last)) = (refused.first(), refused.last()) else {
        panic!("no cast was refused for the silence");
    };
    let secs = last - first + 0.1;
    assert!((secs - 4.0).abs() < 0.3, "4 s of silence: {secs} s");
    assert!(
        (refused.len() as f32 * 0.1 - secs).abs() < 0.05,
        "a cast went through mid-silence: {refused:?}"
    );
}

#[test]
fn quill_s_ink_roots_every_character_within_8_yd_of_her_for_3_s() {
    use benilla_protocol::messages::{MoveMode, CHAT_MSG_RAID_WARNING};
    let realm = realm().with_creature_spells(FAMILIAR, &["ink_quill"]);
    let places = [EXPOSED, SIX_YD, SIXTEEN_YD];
    let mut p = Party::starting(realm, &[20; 3], START_ELWYNN, Box::new(NoGround));
    p.go(0, EXPOSED, 0.0);
    p.run(0.2);
    for (i, &at) in places.iter().enumerate().skip(1) {
        p.go(i, at, 0.0);
    }
    let guids = p.guids.clone();
    // When each character was rooted (`true`) or let go (`false`).
    let mut roots = vec![Vec::new(); places.len()];
    for tick in 0..120 {
        let at = tick as f32 * 0.1;
        for (i, events) in p.run(0.1).into_iter().enumerate() {
            for e in events {
                if let SessionEvent::MoveMode {
                    guid,
                    mode: MoveMode::Root,
                    apply,
                    ..
                } = e
                {
                    if guid == guids[i] {
                        roots[i].push((at, apply));
                    }
                }
            }
        }
    }
    let warned = p.seen(0).iter().any(|e| {
        matches!(e, SessionEvent::Chat(c)
            if c.chat_type == CHAT_MSG_RAID_WARNING && c.text == INK_WARNING)
    });
    assert!(warned, "no warning");
    for (i, what) in [(0, "in reach"), (1, "6 yd out")] {
        let [(rooted, true), (freed, false)] = roots[i][..] else {
            panic!("{what}: rooted once and let go once: {:?}", roots[i]);
        };
        assert!(
            (freed - rooted - 3.0).abs() < 0.3,
            "{what}: 3 s of root, {rooted} to {freed}"
        );
    }
    assert!(roots[2].is_empty(), "16 yd out: {:?}", roots[2]);
}

// The quests (#580, `content/quests_stormwind.toml`): Dughan's hook, Thelwater's chain through
// the riot and the quiet wing, and the quest items.

const A_REPORT_FOR_THE_WATCH: u32 = 97301;
const THE_TURNKEY: u32 = 97304;
const THE_CAPTAINS_KEYS: u32 = 97305;
const ROOKS_LETTERS_QUEST: u32 = 97306;
const THE_QUIET_WING: u32 = 97308;
const THE_DRAIN: u32 = 97310;
const NOT_UP_THE_HILL: u32 = 97311;
const STOCKADE_QUESTS: std::ops::RangeInclusive<u32> = 97301..=97312;

const REPORT: u32 = 980123;
const PURSE: u32 = 980124;
const COIN: u32 = 980125;
const KEYRING: u32 = 980126;
const LETTERS: u32 = 980127;
const WRIT_OF_AUDIENCE: u32 = 980122;
const BOOK: u32 = 980130;
const WRIT_OF_NAMES: u32 = 980131;

/// Warden Thelwater at the crossing (`hub.toml` 9300072) and Marshal Dughan in Goldshire (Elwynn
/// spawn 80334).
const THELWATER_NPC: u64 = 0xF130 << 48 | 1719 << 24 | 9300072;
const DUGHAN: u64 = 0xF130 << 48 | 240 << 24 | 80334;
const BY_DUGHAN: [f32; 3] = [-9463.5, 74.0, 57.0];
/// Bertram Ash in his cell, and a step out in the hall from him.
const ASH_AT: [f32; 3] = [106.0, -126.0, -33.9396];
const BY_ASH: [f32; 3] = [104.5, -126.0, -33.9396];
/// The four witnesses in their cells: `(entry, where)`.
const WITNESSES_AT: [(u32, [f32; 3]); 4] = [
    (97211, [128.6, 49.0, -33.8562]),
    (97212, [160.1, 45.7, -34.7729]),
    (97213, [134.1, 71.0, -33.8562]),
    (97214, [162.7, 59.2, -34.8562]),
];
const WHY: &str = "Why are you here?";
/// Wender Cogg, and the sentence that ends his reply, sending the player to Station Master
/// Brannoc (#593).
const COGG: u32 = 97211;
const TO_BRANNOC: &str =
    " Brannoc still keeps the station. Ask him what he hears down there at night.";
/// Beside the drain grate, out of the Hush's reach.
const BY_GRATE: [f32; 3] = [166.0, 153.5, -33.9396];

fn stormwind_quests() -> toml::Table {
    include_str!("../content/quests_stormwind.toml")
        .parse()
        .unwrap()
}

fn quest_row(id: u32) -> toml::Value {
    rows(&stormwind_quests(), "quest")
        .iter()
        .find(|q| int(q, "id") == Some(id.into()))
        .unwrap_or_else(|| panic!("quest {id}"))
        .clone()
}

/// A Zealot (Tier 2, so the level-30 quests are offered) at `level` in the city's jail door,
/// having handed in `done` and carrying one of each of `items` and nothing else.
fn questing_party(level: u8, done: &[u32], items: &[u32]) -> Party {
    let done = done.to_vec();
    let items = items.to_vec();
    let mut party = Party::edited(
        Realm::with_seed(1),
        &[level],
        START_ELWYNN,
        Box::new(NoGround),
        move |_, blob| {
            let mut blob = carrying(blob, &items)
                .replace("calling = \"basic_warrior\"", "calling = \"zealot\"");
            if !done.is_empty() {
                blob = blob.replace("xp = 0\n", &format!("xp = 0\ncompleted = {done:?}\n"));
            }
            blob
        },
    );
    party.go(0, DOOR, 0.66);
    party.run(0.5);
    party
}

/// Takes `quest` from `npc` where the player stands; the events that came.
fn accept(party: &mut Party, npc: u64, quest: u32) -> Vec<SessionEvent> {
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::QuestGiverAcceptQuest { npc, quest });
    party.run(0.2);
    since(party, from)
}

/// Hands `quest` in at `npc` the stock way; the events that came.
fn hand_in(party: &mut Party, npc: u64, quest: u32) -> Vec<SessionEvent> {
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::QuestGiverCompleteQuest { npc, quest });
    party.cmd(0, RealmCommand::QuestGiverRequestReward { npc, quest });
    party.cmd(
        0,
        RealmCommand::QuestGiverChooseReward {
            npc,
            quest,
            choice: 0,
        },
    );
    party.run(0.2);
    since(party, from)
}

/// The text of the reply menu in `replied` (a gossip menu with no options), as the client asks
/// for it.
fn reply_text(party: &mut Party, replied: &[SessionEvent]) -> String {
    let text_id = replied
        .iter()
        .find_map(|e| match e {
            SessionEvent::GossipMenu {
                text_id, options, ..
            } if options.is_empty() => Some(*text_id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the reply menu: {replied:?}"));
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::NpcTextQuery { text_id });
    party.run(0.1);
    since(party, from)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::NpcGreeting { blocks, .. } => Some(blocks[0].male.clone()),
            _ => None,
        })
        .expect("the reply text")
}

fn completed(events: &[SessionEvent], quest: u32) -> bool {
    events
        .iter()
        .any(|e| matches!(e, SessionEvent::QuestComplete(c) if c.quest_id == quest))
}

fn offered(events: &[SessionEvent], quest: u32) -> bool {
    events.iter().any(|e| match e {
        SessionEvent::QuestDetail(d) => d.quest_id == quest,
        SessionEvent::QuestGreeting(list) => list.quests.iter().any(|q| q.quest_id == quest),
        _ => false,
    })
}

/// How many of item `entry` the character's save carries.
fn carries(party: &Party, entry: u32) -> u32 {
    let saved = party.realm.export_character(party.guids[0]).unwrap();
    let rows: toml::Table = saved.parse().unwrap();
    rows.get("items")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter(|i| int(i, "entry") == Some(entry.into()))
        .map(|i| int(i, "count").unwrap_or(1) as u32)
        .sum()
}

#[test]
fn the_stockade_s_quests_follow_one_another_as_the_ticket_chains_them() {
    let quests = stormwind_quests();
    let ids: Vec<u32> = rows(&quests, "quest")
        .iter()
        .map(|q| int(q, "id").unwrap() as u32)
        .filter(|id| STOCKADE_QUESTS.contains(id))
        .collect();
    assert_eq!(ids, STOCKADE_QUESTS.collect::<Vec<_>>());
    let chained = [
        (97302, 97301),
        (97303, 97302),
        (97304, 97303),
        (97306, 97305),
        (97307, 97306),
        (97308, 97307),
        (97311, 97310),
    ];
    for id in STOCKADE_QUESTS {
        let after = int(&quest_row(id), "after").map(|a| a as u32);
        let want = chained.iter().find(|c| c.0 == id).map(|c| c.1);
        assert_eq!(after, want, "quest {id}'s `after`");
        let kind = quest_row(id)["kind"].as_str().unwrap().to_string();
        let want = if id == 97312 { "repeatable" } else { "story" };
        assert_eq!(kind, want, "quest {id}");
    }
}

#[test]
fn every_stockade_quest_is_loaded_with_its_template() {
    let mut realm = realm();
    for id in STOCKADE_QUESTS {
        realm.handle(RealmCommand::QuestQuery { quest: id });
        let title = drain(&mut realm).into_iter().find_map(|e| match e {
            SessionEvent::QuestTemplate(t) => Some(t.title),
            _ => None,
        });
        assert_eq!(
            title.as_deref(),
            quest_row(id)["title"].as_str(),
            "quest {id}"
        );
    }
}

#[test]
fn a_report_for_the_watch_goes_from_dughan_and_stays_with_thelwater() {
    let mut party = questing_party(30, &[], &[]);
    party.go(0, BY_DUGHAN, 3.14);
    party.run(0.5);
    accept(&mut party, DUGHAN, A_REPORT_FOR_THE_WATCH);
    assert_eq!(carries(&party, REPORT), 1, "Dughan hands over his report");

    enter(&mut party);
    walk(&mut party, CROSSING);
    let events = hand_in(&mut party, THELWATER_NPC, A_REPORT_FOR_THE_WATCH);
    assert!(completed(&events, A_REPORT_FOR_THE_WATCH), "{events:?}");
    assert_eq!(
        carries(&party, REPORT),
        0,
        "the report stays with Thelwater"
    );
}

#[test]
fn the_turnkey_is_handed_in_at_ash_with_nothing_to_collect() {
    let row = quest_row(THE_TURNKEY);
    for objective in ["kill", "collect", "talk", "use"] {
        assert!(
            row.get(objective).is_none(),
            "The Turnkey has a {objective}"
        );
    }
    let mut party = questing_party(50, &[97301, 97302, 97303], &[]);
    enter(&mut party);
    walk(&mut party, CROSSING);
    accept(&mut party, THELWATER_NPC, THE_TURNKEY);
    let events = hand_in(&mut party, THELWATER_NPC, THE_TURNKEY);
    assert!(!completed(&events, THE_TURNKEY), "Thelwater took it in");

    walk(&mut party, EAST_STAIRS);
    assert!(in_phase(&party));
    walk(&mut party, BY_ASH);
    let (ash, _) = party.unit(0, ASH, ASH_AT);
    let events = hand_in(&mut party, ash, THE_TURNKEY);
    assert!(completed(&events, THE_TURNKEY), "{events:?}");
    // Ash offers The Captain's Keys next.
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::QuestGiverHello { npc: ash });
    party.run(0.2);
    assert!(offered(&since(&party, from), THE_CAPTAINS_KEYS));
}

#[test]
fn the_quiet_wing_gives_rook_s_key_and_the_west_door_opens_for_its_carrier() {
    // The Writ of Audience was the door's stand-in; it opens nothing now.
    let done = [97301, 97302, 97303, 97304, 97305, 97306, 97307];
    let mut party = questing_party(37, &done, &[WRIT_OF_AUDIENCE]);
    enter(&mut party);
    walk(&mut party, CROSSING);
    let events = click_west_door(&mut party);
    assert_eq!(door_updates(&events), 0, "the writ opened the door");
    assert_eq!(hale_lines(&events), 1);

    party.go(0, CROSSING, 1.57);
    accept(&mut party, THELWATER_NPC, THE_QUIET_WING);
    assert_eq!(carries(&party, KEY), 1, "Thelwater hands over Rook's Key");
    let events = click_west_door(&mut party);
    assert_eq!(hale_lines(&events), 0);
    assert_eq!(door_updates(&events), 1, "the door shows open");
    let events = walk(&mut party, WEST_STAIRS);
    assert!(
        in_phase(&party),
        "the key let the character into the quiet wing"
    );
    assert!(warders(&events) > 0);
    assert_eq!(carries(&party, KEY), 1, "the door keeps no key");
}

#[test]
fn speaking_with_the_four_witnesses_credits_the_quiet_wing() {
    let done = [97301, 97302, 97303, 97304, 97305, 97306, 97307];
    let mut party = questing_party(50, &done, &[]);
    enter(&mut party);
    walk(&mut party, CROSSING);
    accept(&mut party, THELWATER_NPC, THE_QUIET_WING);
    click_west_door(&mut party);
    walk(&mut party, WEST_STAIRS);
    assert!(in_phase(&party));

    for (entry, at) in WITNESSES_AT {
        walk(&mut party, [at[0] + 1.5, at[1], at[2]]);
        let (witness, _) = party.unit(0, entry, at);
        let from = party.seen(0).len();
        party.cmd(0, RealmCommand::GossipHello { guid: witness });
        party.run(0.1);
        let menu = since(&party, from);
        let option = menu
            .iter()
            .find_map(|e| match e {
                SessionEvent::GossipMenu { options, .. } => {
                    options.iter().find(|o| o.message == WHY).map(|o| o.index)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("{entry} offers no {WHY:?}: {menu:?}"));
        let from = party.seen(0).len();
        party.cmd(
            0,
            RealmCommand::GossipSelectOption {
                guid: witness,
                option,
            },
        );
        party.run(0.1);
        let replied = since(&party, from);
        let credited = replied.iter().any(|e| {
            matches!(e, SessionEvent::QuestObjectiveKill { quest_id, entry: e, .. }
                if *quest_id == THE_QUIET_WING && *e == entry)
        });
        assert!(credited, "{entry} gave no credit");
        if entry == COGG {
            let reply = reply_text(&mut party, &replied);
            assert!(reply.ends_with(TO_BRANNOC), "{reply:?}");
        }
    }

    walk(&mut party, CROSSING);
    assert!(!in_phase(&party));
    let events = hand_in(&mut party, THELWATER_NPC, THE_QUIET_WING);
    assert!(completed(&events, THE_QUIET_WING), "{events:?}");
}

#[test]
fn the_drain_grate_offers_the_drain_only_once_quill_is_dead() {
    let mut party = questing_party(50, &[], &[KEY]);
    enter(&mut party);
    click_west_door(&mut party);
    walk(&mut party, WEST_STAIRS);
    assert!(in_phase(&party));
    let grate = 0xF110 << 48 | u64::from(GRATE);

    walk(&mut party, BY_GRATE);
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::UseGameObject { guid: grate });
    party.run(0.3);
    assert!(
        !offered(&since(&party, from), THE_DRAIN),
        "offered while Quill lives"
    );

    walk(&mut party, BY_QUILL);
    let (quill, at) = party.unit(0, QUILL, QUILL_AT);
    party.go_facing(0, BY_QUILL, at);
    party.realm.set_creature_health_pct(quill, 1);
    party.cmd(0, RealmCommand::AttackSwing { guid: quill });
    for _ in 0..40 {
        party.run(0.5);
        if died(party.seen(0), quill) {
            break;
        }
    }
    assert!(died(party.seen(0), quill), "Quill did not die");

    walk(&mut party, BY_GRATE);
    assert!(shows(&party, grate));
    let from = party.seen(0).len();
    party.cmd(0, RealmCommand::UseGameObject { guid: grate });
    party.run(0.3);
    assert!(
        offered(&since(&party, from), THE_DRAIN),
        "the grate did not offer The Drain"
    );
}

#[test]
fn rook_s_letters_and_not_up_the_hill_put_their_papers_back_in_the_bag() {
    let done = [97301, 97302, 97303, 97304, 97305, 97309, 97310];
    let mut party = questing_party(50, &done, &[]);
    enter(&mut party);
    walk(&mut party, CROSSING);
    accept(&mut party, THELWATER_NPC, ROOKS_LETTERS_QUEST);
    assert_eq!(carries(&party, LETTERS), 1, "Rook's Letters");
    accept(&mut party, THELWATER_NPC, NOT_UP_THE_HILL);
    assert_eq!(carries(&party, BOOK), 1, "the Book of Takings");
    assert_eq!(carries(&party, WRIT_OF_NAMES), 1, "the Writ of Names");
}

/// The `drop` list of the creature row `guid` in `file`.
fn drop_of(file: &toml::Table, guid: i64) -> Vec<u32> {
    rows(file, "creature")
        .iter()
        .find(|c| int(c, "guid") == Some(guid))
        .and_then(|c| c.get("drop"))
        .and_then(|d| d.as_array())
        .map_or(Vec::new(), |d| {
            d.iter().map(|i| i.as_integer().unwrap() as u32).collect()
        })
}

#[test]
fn the_bosses_hold_their_quest_items_and_the_riot_s_officers_a_coin() {
    let riot = riot();
    for c in rows(&riot, "creature") {
        let (guid, entry) = (int(c, "guid").unwrap(), int(c, "entry").unwrap() as u32);
        let mut want = match entry {
            97201 | 97203 => vec![COIN],
            CUTTER => vec![PURSE, COIN],
            ROOK => vec![KEYRING, LETTERS, COIN],
            _ => Vec::new(),
        };
        let mut got = drop_of(&riot, guid);
        want.sort_unstable();
        got.sort_unstable();
        assert_eq!(got, want, "riot creature {guid} ({entry})");
    }
    let quiet = quiet_wing();
    for c in rows(&quiet, "creature") {
        let (guid, entry) = (int(c, "guid").unwrap(), int(c, "entry").unwrap() as u32);
        let want = match entry {
            QUILL => vec![BOOK],
            HUSH => vec![WRIT_OF_NAMES],
            _ => Vec::new(),
        };
        assert_eq!(
            drop_of(&quiet, guid),
            want,
            "quiet wing creature {guid} ({entry})"
        );
    }
    let grate = rows(&quiet, "gameobject")
        .iter()
        .find(|o| int(o, "guid") == Some(GRATE.into()))
        .unwrap();
    assert_eq!(grate["gives_quests"].as_bool(), Some(true));
    assert_eq!(
        int(&quiet["portal"]["door"], "opens_with"),
        Some(KEY.into()),
        "the west door opens with Rook's Key"
    );
}
