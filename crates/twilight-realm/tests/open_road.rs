//! Open Road (#563, spec #560): on a road or street, out of combat, run speed is +30%, shown as
//! the Open Road buff in aura slot 29. The road is the terrain texture the host names under the
//! player; `content/roads.toml` lists which names are roads.

mod common;
mod gear_common;

use benilla_protocol::{ObjectFields, SessionEvent};
use common::*;
use gear_common::*;
use twilight_realm::{Host, Realm, RealmCommand};

/// vanilla's base run speed.
const RUN_SPEED: f32 = 7.0;
const PACK_1: u8 = 23;
/// Runner's Glassfin Wrap, the top tier speed dish: move speed +30%.
const GLASSFIN_WRAP: u32 = 962032;
const GLASSFIN_PCT: u32 = 30;
const ROAD_SLOT: usize = 29;
/// `UNIT_FLAG_IN_COMBAT`.
const IN_COMBAT: u32 = 0x0008_0000;

/// `content/roads.toml` with its map 0 rows on map 1, where the test warrior stands.
fn roads_on_kalimdor() -> String {
    include_str!("../content/roads.toml").replacen("id = 0", "id = 1", 1)
}

fn roads() -> toml::Table {
    include_str!("../content/roads.toml").parse().unwrap()
}

/// The first road texture `roads.toml` lists, as the host names it.
fn road_texture() -> String {
    roads()["map"].as_array().unwrap()[0]["textures"]
        .as_array()
        .unwrap()[0]
        .as_str()
        .unwrap()
        .to_owned()
}

/// `[open_road] speed_pct`.
fn road_pct() -> u32 {
    roads()["open_road"]["speed_pct"].as_integer().unwrap() as u32
}

/// The Open Road buff's own spell id, from `content/buffs.toml`.
fn open_road_spell() -> u32 {
    let buffs: toml::Table = include_str!("../content/buffs.toml").parse().unwrap();
    let row = buffs["spell"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["key"].as_str() == Some("open_road"))
        .expect("buffs.toml lists open_road");
    row["spell_id"].as_integer().unwrap() as u32
}

/// Flat ground whose top texture is a road inside the box `min..max` and a field outside it.
/// `sight` off, no creature notices the warrior, so no fight starts by chance.
struct Painted {
    min: [f32; 2],
    max: [f32; 2],
    road: Option<String>,
    sight: bool,
}

impl Painted {
    /// A road 10 yd east of `at`, 4 yd square.
    fn beside(at: [f32; 3]) -> Self {
        Self {
            min: [at[0] + 8.0, at[1] - 2.0],
            max: [at[0] + 12.0, at[1] + 2.0],
            road: Some(road_texture()),
            sight: false,
        }
    }

    /// The road everywhere, and sight between every two points.
    fn everywhere() -> Self {
        Self {
            min: [f32::MIN, f32::MIN],
            max: [f32::MAX, f32::MAX],
            road: Some(road_texture()),
            sight: true,
        }
    }

    /// A spot on the road.
    fn on(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) / 2.0,
            (self.min[1] + self.max[1]) / 2.0,
            0.0,
        ]
    }
}

impl Host for Painted {
    fn ground_height(&self, _map: u32, _x: f32, _y: f32, _z_hint: f32) -> Option<f32> {
        Some(0.0)
    }
    fn line_of_sight(&self, _map: u32, _from: [f32; 3], _to: [f32; 3]) -> bool {
        self.sight
    }
    fn area_at(&self, _map: u32, _x: f32, _y: f32) -> Option<u32> {
        None
    }
    fn ground_texture(&self, _map: u32, x: f32, y: f32) -> Option<String> {
        let inside =
            (self.min[0]..=self.max[0]).contains(&x) && (self.min[1]..=self.max[1]).contains(&y);
        if inside {
            self.road.clone()
        } else {
            Some("Tileset\\Elwynn\\ElwynnGrassBase".to_owned())
        }
    }
}

struct Script {
    realm: Realm,
    me: u64,
    events: Vec<SessionEvent>,
}

impl Script {
    /// A warrior on map 1 carrying `items`, whose realm reads `roads.toml` as map 1's.
    fn new(items: &[Carried]) -> Self {
        let (realm, me, events) = warrior_with(10, 0, items);
        Self {
            realm: realm.with_roads(&roads_on_kalimdor()),
            me,
            events,
        }
    }

    /// [`Self::new`] with the Open Road unlock for sale and not bought.
    fn unlock_off() -> Self {
        let mut s = Self::new(&[]);
        let catalogue =
            include_str!("../content/unlocks.toml").replacen("\nfree = true", "\nfree = false", 1);
        s.realm = s.realm.with_unlock_catalogue(&catalogue);
        assert!(!s.realm.unlock_on("open_road"));
        s
    }

    /// Where the warrior entered the world.
    fn start(&self) -> [f32; 3] {
        self.events
            .iter()
            .find_map(|e| match e {
                SessionEvent::Worldport { position, .. } => Some(*position),
                _ => None,
            })
            .expect("a worldport")
    }

    fn step(&mut self, to: [f32; 3]) {
        self.realm.handle(RealmCommand::Move {
            motion: Default::default(),
            position: to,
            orientation: 0.0,
        });
    }

    /// One 0.1 s tick on `host`: its events, also kept.
    fn tick(&mut self, host: &dyn Host) -> Vec<SessionEvent> {
        self.realm.tick(0.1, host);
        let new = drain(&mut self.realm);
        self.events.extend(new.clone());
        new
    }

    /// Steps to `to` and ticks once: the tick's events.
    fn step_and_tick(&mut self, to: [f32; 3], host: &dyn Host) -> Vec<SessionEvent> {
        self.step(to);
        self.tick(host)
    }

    fn me(&self) -> ObjectFields {
        descriptor(&self.events, self.me)
    }

    fn road_aura(&self) -> u32 {
        self.me().unit_aura_ids().nth(ROAD_SLOT).unwrap_or(0)
    }

    /// The run speed the client was last told of, `None` before any change.
    fn speed(&self) -> Option<f32> {
        speeds(&self.events, self.me).last().copied()
    }
}

/// The run speeds the client was told of for `me` in `events`, in order.
fn speeds(events: &[SessionEvent], me: u64) -> Vec<f32> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ForceSpeedChange { guid, speed, .. } if *guid == me => Some(*speed),
            _ => None,
        })
        .collect()
}

fn assert_speed(s: &Script, mul: f32) {
    let speed = s.speed().expect("a speed change");
    assert!(
        (speed - RUN_SPEED * mul).abs() < 1e-3,
        "run speed {speed}, want {}",
        RUN_SPEED * mul
    );
}

fn road_mul(extra_pct: u32) -> f32 {
    1.0 + (road_pct() + extra_pct) as f32 / 100.0
}

#[test]
fn a_step_onto_the_road_out_of_combat_runs_faster_under_the_buff() {
    let mut s = Script::new(&[]);
    let host = Painted::beside(s.start());
    s.step_and_tick(host.on(), &host);
    assert_speed(&s, road_mul(0));
    assert_eq!(s.road_aura(), open_road_spell());
}

#[test]
fn the_top_speed_dish_adds_to_the_road() {
    let mut s = Script::new(&[(PACK_1, GLASSFIN_WRAP, 1)]);
    s.realm.handle(RealmCommand::UseItem {
        bag: 255,
        slot: PACK_1,
    });
    s.events.extend(drain(&mut s.realm));
    let host = Painted::beside(s.start());
    s.step_and_tick(host.on(), &host);
    assert_speed(&s, road_mul(GLASSFIN_PCT));
    assert_eq!(s.road_aura(), open_road_spell());
}

#[test]
fn off_the_road_the_speed_holds_a_second_then_drops_with_the_buff() {
    let mut s = Script::new(&[]);
    let start = s.start();
    let host = Painted::beside(start);
    s.step_and_tick(host.on(), &host);
    assert_speed(&s, road_mul(0));

    let changes = speeds(&s.events, s.me).len();
    let mut off = s.step_and_tick(start, &host);
    for _ in 0..7 {
        off.extend(s.tick(&host));
    }
    assert!(speeds(&off, s.me).is_empty(), "held for 0.8 s: {off:?}");
    assert_eq!(s.road_aura(), open_road_spell(), "the buff holds too");

    for _ in 0..5 {
        s.tick(&host);
    }
    assert_eq!(speeds(&s.events, s.me).len(), changes + 1);
    assert_speed(&s, 1.0);
    assert_eq!(s.road_aura(), 0, "the buff is gone");
}

#[test]
fn a_fight_on_the_road_ends_the_buff_and_its_end_brings_it_back() {
    let mut s = Script::new(&[]);
    let host = Painted::everywhere();
    let mob = the_mob(&s.events);
    stand_by(&mut s.realm, &mob);
    s.tick(&host);
    assert_speed(&s, road_mul(0));
    assert_eq!(s.road_aura(), open_road_spell());

    s.realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut fought = false;
    for _ in 0..50 {
        s.realm.set_creature_health_pct(mob.guid, 100);
        s.tick(&host);
        fought |= s.me().unit_flags() & IN_COMBAT != 0;
        if fought && s.road_aura() == 0 {
            break;
        }
    }
    assert!(fought, "the fight starts");
    assert_speed(&s, 1.0);
    assert_eq!(s.road_aura(), 0, "no buff in a fight");

    // Out of the fight, still on the road: well away until the mob gives up.
    s.realm.handle(RealmCommand::AttackStop);
    let away = [mob.position[0] + 90.0, mob.position[1], mob.position[2]];
    for _ in 0..300 {
        s.step_and_tick(away, &host);
        if s.me().unit_flags() & IN_COMBAT == 0 && s.road_aura() != 0 {
            break;
        }
    }
    assert_eq!(s.me().unit_flags() & IN_COMBAT, 0, "out of combat");
    assert_speed(&s, road_mul(0));
    assert_eq!(s.road_aura(), open_road_spell(), "the buff is back");
}

#[test]
fn with_the_unlock_off_a_road_does_nothing() {
    let mut s = Script::unlock_off();
    let host = Painted::beside(s.start());
    for _ in 0..3 {
        s.step_and_tick(host.on(), &host);
    }
    assert_eq!(s.speed(), None);
    assert_eq!(s.road_aura(), 0);
}

#[test]
fn a_host_that_names_no_texture_gives_no_road() {
    let mut s = Script::new(&[]);
    let start = s.start();
    let mut unloaded = Painted::beside(start);
    unloaded.road = None;
    let spot = unloaded.on();
    for _ in 0..3 {
        s.step_and_tick(spot, &unloaded);
        s.step_and_tick(spot, &FlatGround);
    }
    assert_eq!(s.speed(), None);
    assert_eq!(s.road_aura(), 0);
}
