//! `content/hub.toml` against what each NPC is in the town: its NPC flags fit its role, the quest
//! givers face their Hunting Grounds, and the townsfolk only talk.

use std::collections::HashSet;

use serde::Deserialize;

#[derive(Deserialize)]
struct Npc {
    guid: u32,
    entry: u32,
    name: String,
    role: String,
    npc_flags: u32,
    position: [f32; 3],
    orientation: f32,
    #[serde(default)]
    stock: Vec<u32>,
    #[serde(default)]
    tokens: bool,
    #[serde(default)]
    blood_coin: bool,
    #[serde(default)]
    greeting: String,
    text_id: Option<u32>,
    #[serde(default)]
    wander: f32,
    #[serde(default)]
    respec: bool,
    /// The `AreaTable` id an innkeeper binds to.
    area: Option<u32>,
    /// An innkeeper's rest area, in yards.
    rest_radius: Option<f32>,
}

#[derive(Deserialize)]
struct Hub {
    npc: Vec<Npc>,
}

#[derive(Deserialize)]
struct Collect {
    from: Vec<u32>,
}

#[derive(Deserialize)]
struct Quest {
    giver: u32,
    #[serde(default)]
    kill: Vec<(u32, u32)>,
    #[serde(default)]
    collect: Vec<Collect>,
}

#[derive(Deserialize)]
struct Quests {
    quest: Vec<Quest>,
}

/// `UNIT_NPC_FLAG_*`.
const GOSSIP: u32 = 0x1;
const QUEST_GIVER: u32 = 0x2;
const VENDOR: u32 = 0x4;
const TRAINER: u32 = 0x10;
const INNKEEPER: u32 = 0x80;
const REPAIR: u32 = 0x4000;

fn hub() -> Vec<Npc> {
    toml::from_str::<Hub>(include_str!("../content/hub.toml"))
        .expect("hub.toml parses")
        .npc
}

fn quests() -> Vec<Quest> {
    toml::from_str::<Quests>(include_str!("../content/quests.toml"))
        .expect("quests.toml parses")
        .quest
}

/// Every creature spawn in The Barrens, `(entry, x, y)`. A Wailing Caverns creature counts as
/// standing at the Caverns' entrance: its Hunting Ground, seen from Kalimdor, is that mouth.
fn spawns() -> Vec<(u32, f32, f32)> {
    let barrens: toml::Table = include_str!("../content/barrens.toml").parse().unwrap();
    let caverns: toml::Table = include_str!("../content/wailing_caverns.toml")
        .parse()
        .unwrap();
    let mouth = caverns["entrance"]["position"].as_array().unwrap();
    let mouth = |i: usize| mouth[i].as_float().unwrap() as f32;
    barrens["spawns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let row = row.as_array().unwrap();
            let f = |i: usize| row[i].as_float().unwrap() as f32;
            (row[1].as_integer().unwrap() as u32, f(4), f(5))
        })
        .chain(
            caverns["template"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| (t["entry"].as_integer().unwrap() as u32, mouth(0), mouth(1))),
        )
        .collect()
}

#[test]
fn every_npcs_flags_fit_its_role() {
    let quests = quests();
    for n in hub() {
        let gives = quests.iter().any(|q| q.giver == n.entry);
        let talks = n.text_id.is_some() && !n.greeting.is_empty();
        let ok = match n.role.as_str() {
            "goods" => n.npc_flags == VENDOR | REPAIR && !n.stock.is_empty() && !n.tokens,
            "innkeeper" => {
                n.npc_flags == GOSSIP | VENDOR | INNKEEPER
                    && talks
                    && !gives
                    && !n.stock.is_empty()
                    && !n.tokens
                    && n.area.is_some()
            }
            "tokens" => n.npc_flags == VENDOR && n.tokens && n.stock.is_empty(),
            "blood_coin" => {
                n.npc_flags == VENDOR && n.blood_coin && !n.tokens && n.stock.is_empty()
            }
            "trainer" => n.npc_flags == GOSSIP | QUEST_GIVER | TRAINER && talks,
            // The flight master only stands there: no flags, no menu, no words.
            "flight" => n.npc_flags == 0 && n.text_id.is_none() && n.stock.is_empty() && !n.tokens,
            "giver" => n.npc_flags == QUEST_GIVER && gives && !n.greeting.is_empty(),
            "ambient" => {
                n.npc_flags == GOSSIP
                    && talks
                    && !gives
                    && n.stock.is_empty()
                    && !n.tokens
                    && !n.greeting.contains('\n')
            }
            other => panic!("{}: no role {other:?}", n.name),
        };
        assert!(ok, "{} ({}): flags {:#x}", n.name, n.role, n.npc_flags);
        assert!(
            n.area.is_none() || n.role == "innkeeper",
            "{}: only an innkeeper binds",
            n.name
        );
        // Only the townsfolk walk about, and only a few yards.
        assert!(
            n.wander == 0.0 || (n.role == "ambient" && n.wander <= 6.0),
            "{} wanders {} yd",
            n.name,
            n.wander
        );
    }
}

/// The five Tier 1 trainers stand together in the training yard, and Uzzek alone sells the respec.
#[test]
fn five_trainers_stand_in_the_yard_and_one_sells_the_respec() {
    let trainers: Vec<Npc> = hub().into_iter().filter(|n| n.role == "trainer").collect();
    assert_eq!(trainers.len(), 5);
    for a in &trainers {
        for b in &trainers {
            let d = (a.position[0] - b.position[0]).hypot(a.position[1] - b.position[1]);
            assert!(d <= 15.0, "{} and {} stand {d} yd apart", a.name, b.name);
        }
    }
    let respec: Vec<(String, String)> = hub()
        .into_iter()
        .filter(|n| n.respec)
        .map(|n| (n.name, n.role))
        .collect();
    assert_eq!(respec, [("Uzzek".to_string(), "trainer".to_string())]);
}

/// The middle of the Crossroads and of Camp Taurajo, and how far each town reaches from it.
const CROSSROADS: [f32; 2] = [-430.0, -2640.0];
const TAURAJO: [f32; 2] = [-2370.0, -1950.0];
const TOWN_RADIUS: f32 = 150.0;

/// The town's NPCs.
fn in_town(middle: [f32; 2]) -> Vec<Npc> {
    hub()
        .into_iter()
        .filter(|n| (n.position[0] - middle[0]).hypot(n.position[1] - middle[1]) <= TOWN_RADIUS)
        .collect()
}

#[test]
fn about_twenty_townsfolk_live_in_the_crossroads_and_some_wander() {
    let ambient: Vec<Npc> = in_town(CROSSROADS)
        .into_iter()
        .filter(|n| n.role == "ambient")
        .collect();
    assert!(
        (18..=24).contains(&ambient.len()),
        "{} townsfolk",
        ambient.len()
    );
    let wanderers = ambient.iter().filter(|n| n.wander > 0.0).count();
    assert!(
        (2..ambient.len() / 2).contains(&wanderers),
        "{wanderers} wander"
    );
}

/// Camp Taurajo, the Outpost: an innkeeper who binds to it, a goods vendor, about eight
/// townsfolk, and no trainer or Token vendor.
#[test]
fn camp_taurajo_has_an_inn_a_goods_vendor_and_its_own_folk_but_no_trainers() {
    const CAMP_TAURAJO: u32 = 378;
    let camp = in_town(TAURAJO);
    let count = |role: &str| camp.iter().filter(|n| n.role == role).count();
    assert_eq!(count("innkeeper"), 1);
    assert!(camp
        .iter()
        .any(|n| n.role == "innkeeper" && n.area == Some(CAMP_TAURAJO) && n.rest_radius.is_some()));
    assert_eq!(count("goods"), 1);
    assert!(
        (7..=10).contains(&count("ambient")),
        "{} townsfolk",
        count("ambient")
    );
    assert_eq!(count("trainer") + count("tokens"), 0);
}

#[test]
fn each_npc_is_its_own_spawn_and_each_line_its_own_text() {
    let npcs = hub();
    let mut guids = HashSet::new();
    let mut texts = HashSet::new();
    for n in &npcs {
        assert!(guids.insert(n.guid), "{}: guid {:#x} again", n.name, n.guid);
        if let Some(t) = n.text_id {
            assert!(texts.insert(t), "{}: text {t} again", n.name);
        }
        // An entry names one creature: the client caches its name and title by entry.
        for m in &npcs {
            assert!(
                m.entry != n.entry || m.name == n.name,
                "{} and {}",
                n.name,
                m.name
            );
        }
    }
}

/// How far a giver may face from its Hunting Ground and still be looking at it.
const FACING_SLACK_DEG: f32 = 45.0;

#[test]
fn each_quest_giver_faces_its_hunting_ground() {
    let quests = quests();
    let spawns = spawns();
    for g in hub().into_iter().filter(|n| n.role == "giver") {
        let targets: HashSet<u32> = quests
            .iter()
            .filter(|q| q.giver == g.entry)
            .flat_map(|q| {
                q.kill
                    .iter()
                    .map(|k| k.0)
                    .chain(q.collect.iter().flat_map(|c| c.from.iter().copied()))
            })
            .collect();
        // A giver whose quests hunt nothing ("Know the Land") has no ground to face.
        if targets.is_empty() {
            continue;
        }
        let at: Vec<(f32, f32)> = spawns
            .iter()
            .filter(|s| targets.contains(&s.0))
            .map(|s| (s.1, s.2))
            .collect();
        assert!(!at.is_empty(), "{}: no spawn of its targets", g.name);
        let n = at.len() as f32;
        let (cx, cy) = at
            .iter()
            .fold((0.0, 0.0), |(x, y), p| (x + p.0 / n, y + p.1 / n));
        let bearing = (cy - g.position[1]).atan2(cx - g.position[0]);
        let off = (bearing - g.orientation).rem_euclid(std::f32::consts::TAU);
        let off = off.min(std::f32::consts::TAU - off).to_degrees();
        assert!(
            off <= FACING_SLACK_DEG,
            "{} faces {:.2} rad; its Hunting Ground lies at {:.2}",
            g.name,
            g.orientation,
            bearing.rem_euclid(std::f32::consts::TAU)
        );
    }
}
