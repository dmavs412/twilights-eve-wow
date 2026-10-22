//! The quest content: the Hunting, Repeatable and Advancement Quests the Hub's givers offer
//! (`content/quests.toml`), and the items those quests use (`content/quest_items.toml`).

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use serde::Deserialize;

use crate::{items, rules};

/// A collect objective: `count` of `item`, dropping `from` these creatures at `chance` percent
/// while the quest wants more.
#[derive(Deserialize)]
pub(crate) struct Collect {
    pub(crate) item: u32,
    pub(crate) count: u32,
    /// None for a Quest Starter, whose own drop rule ([`StarterDrop`]) brings it.
    #[serde(default)]
    pub(crate) chance: u32,
    #[serde(default)]
    pub(crate) from: Vec<u32>,
    /// Drops that differ by creature, each with its own chance and count; a creature named here
    /// drops by its row instead of by `from` and `chance`.
    #[serde(default)]
    pub(crate) sources: Vec<Source>,
}

/// One kind of creature's drop of a collect objective's item.
#[derive(Deserialize)]
pub(crate) struct Source {
    pub(crate) from: Vec<u32>,
    pub(crate) chance: u32,
    /// How many one corpse holds.
    #[serde(default = "one")]
    pub(crate) count: u32,
}

fn one() -> u32 {
    1
}

/// A Quest Starter's drop (`content/quest_items.toml`'s `starter_drop`): `item` drops `from`
/// these creatures at `chance` percent, but only while the player has neither taken nor
/// completed the quest it starts and carries none already.
#[derive(Deserialize)]
pub(crate) struct StarterDrop {
    pub(crate) item: u32,
    pub(crate) from: Vec<u32>,
    pub(crate) chance: u32,
    /// An item that is no Quest Starter but a quest's own to hold ahead of time (the Dig Orders):
    /// it drops while this quest is not done and none is carried, taken or not.
    #[serde(default)]
    pub(crate) until_done: Option<u32>,
}

/// An explore objective: a spot on a map, met when the player walks into it.
#[derive(Deserialize)]
pub(crate) struct Explore {
    /// What the log calls the place.
    pub(crate) place: String,
    /// The map the spot is on: Kalimdor unless the content says otherwise.
    #[serde(default = "kalimdor")]
    map: u32,
    /// WoW `[x, y]`.
    pub(crate) position: [f32; 2],
    /// Yards.
    pub(crate) radius: f32,
}

fn kalimdor() -> u32 {
    crate::KALIMDOR
}

impl Explore {
    /// Whether the player at `at` on `map` stands in the spot.
    pub(crate) fn contains(&self, map: u32, at: [f32; 3]) -> bool {
        let [x, y] = self.position;
        map == self.map && (at[0] - x).hypot(at[1] - y) <= self.radius
    }

    /// The log's exploration line.
    pub(crate) fn text(&self) -> String {
        format!("{} explored", self.place)
    }
}

/// A quest's explore objectives: one spot (`explore = { ... }`), whose credit is the quest's
/// `explored` flag, or a list (`explore = [{ ... }, { ... }]`), each spot a log row of its own
/// with a credit in the counter after the use objectives.
#[derive(Default)]
pub(crate) struct Explores {
    spots: Vec<Explore>,
    listed: bool,
}

impl<'de> Deserialize<'de> for Explores {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            One(Explore),
            Many(Vec<Explore>),
        }
        Ok(match Written::deserialize(d)? {
            Written::One(e) => Explores {
                spots: vec![e],
                listed: false,
            },
            Written::Many(spots) => Explores {
                spots,
                listed: true,
            },
        })
    }
}

impl Explores {
    pub(crate) fn is_empty(&self) -> bool {
        self.spots.is_empty()
    }

    /// The lone spot of the unlisted form.
    pub(crate) fn single(&self) -> Option<&Explore> {
        self.spots.first().filter(|_| !self.listed)
    }

    /// The spots of the list form; none for the single form.
    pub(crate) fn listed(&self) -> &[Explore] {
        if self.listed {
            &self.spots
        } else {
            &[]
        }
    }
}

/// How often a quest can be done (`GLOSSARY.md`).
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    /// A Hub giver's: once per character, like [`Kind::Once`]; the kind names where it comes from.
    Hunting,
    /// A Dungeon's: taken again as soon as it is handed in.
    Repeatable,
    /// A field giver's: taken again as soon as it is handed in, paying half a level's XP, a
    /// Token and its money. Advancement never forgets it.
    Bounty,
    /// Into another Calling, with an [`Advancement`].
    Advancement,
    /// Once per character: Advancement never forgets it.
    Once,
    /// A Story Quest: once per character, Advancement never forgets it, and it pays XP as a
    /// Hunting Quest does, with no Token.
    Story,
}

/// Who offers a quest: an NPC's creature entry (a Hub NPC or a Field Giver), or `"item"` for a
/// Quest Starter's quest, which the item naming it in its `start_quest` offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Giver {
    Npc(u32),
    Item,
    /// Written as no `giver`: the quest names a `giver_object` instead.
    #[default]
    Unset,
}

impl<'de> Deserialize<'de> for Giver {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            Npc(u32),
            Word(String),
        }
        match Written::deserialize(d)? {
            Written::Npc(entry) => Ok(Giver::Npc(entry)),
            Written::Word(w) if w == "item" => Ok(Giver::Item),
            Written::Word(w) => Err(serde::de::Error::custom(format!(
                "a quest's giver is an NPC entry or \"item\", not {w:?}"
            ))),
        }
    }
}

impl Giver {
    /// The NPC entry that offers it; `None` for a Quest Starter's quest.
    pub(crate) fn npc(self) -> Option<u32> {
        match self {
            Giver::Npc(entry) => Some(entry),
            Giver::Item | Giver::Unset => None,
        }
    }
}

/// The quests that must be handed in before a quest is offered: one id (`after = 94001`) or a
/// list (`after = [94201, 94202]`), none by default.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct After(Vec<u32>);

impl<'de> Deserialize<'de> for After {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            One(u32),
            Many(Vec<u32>),
        }
        Ok(After(match Written::deserialize(d)? {
            Written::One(id) => vec![id],
            Written::Many(ids) => ids,
        }))
    }
}

impl After {
    pub(crate) fn ids(&self) -> &[u32] {
        &self.0
    }
}

/// A talk-to objective: while the quest is active and the credit not yet given, `npc`'s gossip
/// menu shows `option`; choosing it shows `reply` and gives the credit.
#[derive(Deserialize)]
pub(crate) struct Talk {
    pub(crate) npc: u32,
    pub(crate) option: String,
    pub(crate) reply: String,
}

/// A GameObject standing out in the open world for a use objective: it despawns for a character
/// who uses it and returns `respawn` seconds later.
#[derive(Deserialize)]
pub(crate) struct QuestObject {
    /// Distinguishes this placement from every other GameObject's guid.
    pub(crate) guid: u32,
    pub(crate) entry: u32,
    pub(crate) display: u32,
    #[serde(default = "kalimdor")]
    pub(crate) map: u32,
    pub(crate) position: [f32; 3],
    /// It stands on the host's ground (`crate::ground`); `false` keeps its written Z.
    #[serde(default = "crate::ground::yes")]
    pub(crate) grounded: bool,
    #[serde(default)]
    pub(crate) orientation: f32,
    /// Seconds until it returns once used.
    #[serde(default = "a_minute")]
    pub(crate) respawn: f32,
}

fn a_minute() -> f32 {
    60.0
}

impl QuestObject {
    pub(crate) fn object_guid(&self) -> u64 {
        crate::phased::HIGHGUID_GAMEOBJECT << 48 | u64::from(self.guid)
    }

    /// Its create, standing on `host`'s ground.
    pub(crate) fn create(&self, host: &dyn crate::Host) -> benilla_protocol::SessionEvent {
        crate::phased::gameobject_create(&crate::phased::GameObject {
            guid: self.object_guid(),
            entry: self.entry,
            display_id: self.display,
            type_id: crate::phased::GENERIC,
            flags: 0,
            faction: 0,
            position: crate::ground::grounded_at(host, self.map, self.position, self.grounded),
            orientation: self.orientation,
        })
    }
}

/// A quest.
#[derive(Deserialize)]
pub(crate) struct Quest {
    pub(crate) id: u32,
    pub(crate) kind: Kind,
    #[serde(default)]
    pub(crate) giver: Giver,
    /// The entry of an outdoor quest GameObject that offers it, in place of `giver`. It must name
    /// a `gameobject` row, and the quest names its `ender`.
    #[serde(default)]
    pub(crate) giver_object: Option<u32>,
    /// Who takes it in, when not its giver: a person found in the field.
    #[serde(default)]
    ender: Option<u32>,
    /// The quests that must all be handed in before this one is offered.
    #[serde(default)]
    pub(crate) after: After,
    /// A quest that must be in the log or handed in before this one is offered: `after` asks for
    /// the hand-in alone.
    #[serde(default)]
    pub(crate) after_taken: Option<u32>,
    pub(crate) level: u8,
    /// The lowest Tier it is offered at, when the Tier of its `level` is not the one.
    #[serde(default)]
    min_tier: Option<u8>,
    pub(crate) title: String,
    pub(crate) objectives: String,
    pub(crate) details: String,
    pub(crate) progress: String,
    pub(crate) completion: String,
    /// Copper on hand-in.
    pub(crate) money: u32,
    /// Tokens on hand-in.
    pub(crate) tokens: u32,
    /// The reward panel's "You will be able to choose one of these": item entries, one of the
    /// chosen one on hand-in. At most six (`QUEST_REWARD_CHOICES_COUNT`).
    #[serde(default)]
    pub(crate) choices: Vec<u32>,
    /// The reward panel's "You will receive": `[item entry, count]`, all of them on hand-in, beside
    /// any Tokens. Ammo among them is set as the ammo when none is (the Ranger's arrows).
    #[serde(default)]
    pub(crate) rewards: Vec<(u32, u32)>,
    /// `[item entry, count]` the giver hands over when the quest is taken: a quest item to carry
    /// to the ender, which a `collect` objective then takes at the hand-in.
    #[serde(default)]
    pub(crate) gives: Vec<(u32, u32)>,
    /// `(creature entry, count)`.
    #[serde(default)]
    pub(crate) kill: Vec<(u32, u32)>,
    /// Talk-to objectives, after the kills in the log's four creature/GameObject slots.
    #[serde(default)]
    pub(crate) talk: Vec<Talk>,
    /// Use-GameObject objectives, `(GameObject entry, count)`, after the talk objectives.
    #[serde(default, rename = "use")]
    pub(crate) use_object: Vec<(u32, u32)>,
    #[serde(default)]
    pub(crate) collect: Vec<Collect>,
    #[serde(default)]
    pub(crate) explore: Explores,
    /// A discover objective: these `AreaTable.dbc` areas (content/areas.toml), met once the
    /// character has discovered every one.
    #[serde(default)]
    pub(crate) discover: Vec<u32>,
    /// The quest fails, and this text goes to the character as a system message, when a creature
    /// enters combat with them while it is active.
    #[serde(default)]
    pub(crate) fail_if_seen: Option<String>,
    /// The quest log's header: a negative `QuestSort.dbc` id, or a zone; The Barrens by default.
    #[serde(default)]
    zone_or_sort: Option<i32>,
    /// An Advancement Quest's Callings: offered at the Tier Cap of one, it advances into the
    /// other. It asks for nothing but a word with its giver.
    #[serde(default)]
    pub(crate) advancement: Option<Advancement>,
}

/// What makes a quest an Advancement Quest.
#[derive(Deserialize)]
pub(crate) struct Advancement {
    /// The Calling it is offered to, at that Calling's Tier Cap.
    pub(crate) from: String,
    /// The Calling it advances into.
    pub(crate) to: String,
    /// `false` for a quest that is offered and never finished: its Calling has no kit.
    #[serde(default = "yes")]
    pub(crate) completable: bool,
}

fn yes() -> bool {
    true
}

impl Quest {
    /// The NPC entry that takes it in.
    pub(crate) fn ender(&self) -> u32 {
        self.ender
            .or(self.giver.npc())
            .expect("the content load checks an item's or a GameObject's quest names its ender")
    }

    /// The quests its ender may offer next, once it is handed in: those with this quest among
    /// their `after` ids. Whether the others are done is the offer's check.
    pub(crate) fn followers(&self) -> impl Iterator<Item = &'static Quest> + '_ {
        quests()
            .into_iter()
            .filter(|n| n.after.ids().contains(&self.id) && n.giver.npc() == Some(self.ender()))
    }

    /// The quest its ender offers next, once it is handed in (`NextQuestInChain`).
    pub(crate) fn next_in_chain(&self) -> Option<&'static Quest> {
        self.followers().next()
    }

    /// The log slot of talk objective `i`.
    pub(crate) fn talk_slot(&self, i: usize) -> usize {
        self.kill.len() + i
    }

    /// The log slot of use objective `i`.
    pub(crate) fn use_slot(&self, i: usize) -> usize {
        self.kill.len() + self.talk.len() + i
    }

    /// The log slot of listed explore spot `i`.
    pub(crate) fn explore_slot(&self, i: usize) -> usize {
        self.use_slot(self.use_object.len()) + i
    }

    /// The quest log's header the quest goes under.
    pub(crate) fn zone_or_sort(&self) -> i32 {
        self.zone_or_sort.unwrap_or(crate::THE_BARRENS as i32)
    }

    /// The XP a player of level `player` earns handing it in, before the multipliers: an
    /// Advancement Quest pays in a new Calling instead.
    pub(crate) fn reward_xp(&self, player: u8) -> u32 {
        match self.kind {
            Kind::Bounty | Kind::Hunting | Kind::Story => {
                rules::quest::hunting_reward_xp(player, self.level)
            }
            Kind::Repeatable => rules::quest::reward_xp(player),
            Kind::Advancement | Kind::Once => 0,
        }
    }

    /// Whether a character who has handed it in may take it again.
    pub(crate) fn retakable(&self) -> bool {
        matches!(self.kind, Kind::Repeatable | Kind::Bounty)
    }

    /// Whether the quest is offered to a character whose Calling is at Tier `tier`: from its
    /// minimum Tier up, with no maximum. A copy accepted earlier is handed in at any Tier.
    pub(crate) fn offered_at_tier(&self, tier: u8) -> bool {
        tier >= self.min_tier()
    }

    /// The lowest Tier it is offered at: the content's `min_tier`, else the Tier of its level.
    pub(crate) fn min_tier(&self) -> u8 {
        self.min_tier
            .unwrap_or_else(|| rules::quest::min_tier_for_level(self.level))
    }
}

#[derive(Deserialize)]
struct QuestFile {
    #[serde(default)]
    quest: Vec<Quest>,
    /// Only through the test seam: quest items, as `content/quest_items.toml` writes them.
    #[serde(default)]
    item: Vec<items::Template>,
    #[serde(default)]
    starter_drop: Vec<StarterDrop>,
    /// The GameObjects the file's use objectives click.
    #[serde(default)]
    gameobject: Vec<QuestObject>,
}

/// At most this many kill, talk, use and listed explore objectives together, and as many collect
/// objectives (`QUEST_OBJECTIVES_COUNT`).
const OBJECTIVES: usize = 4;

/// Whether the creature `entry` is a Field Giver's: a friendly spawn the realm places anywhere,
/// on the open world or in a Phased Dungeon.
fn friendly_spawn(entry: u32) -> bool {
    crate::mobs::template(entry).is_some_and(|t| t.friendly)
        && crate::mobs::content()
            .spawn
            .iter()
            .any(|s| s.entry == entry)
}

/// The shipped quests: `content/quests.toml`, then `content/quests_elwynn.toml`.
fn shipped() -> &'static [Quest] {
    static FILE: OnceLock<Vec<Quest>> = OnceLock::new();
    FILE.get_or_init(|| {
        let mut quests = Vec::new();
        let mut objects = Vec::new();
        for (name, text) in [
            ("quests.toml", include_str!("../../content/quests.toml")),
            (
                "quests_elwynn.toml",
                include_str!("../../content/quests_elwynn.toml"),
            ),
        ] {
            let file: QuestFile =
                toml::from_str(text).unwrap_or_else(|e| panic!("content/{name} parses: {e}"));
            quests.extend(file.quest);
            objects.extend(file.gameobject);
        }
        for q in &quests {
            check(q, &|id| quests.iter().any(|o| o.id == id), &|entry| {
                objects.iter().any(|o| o.entry == entry) || crate::phased::is_door(entry)
            });
            assert_eq!(
                quests.iter().filter(|o| o.id == q.id).count(),
                1,
                "quest id {} is used twice in the quest files",
                q.id
            );
        }
        for t in shipped_items().item.iter().filter(|t| t.start_quest != 0) {
            assert!(
                quests
                    .iter()
                    .any(|q| q.id == t.start_quest && q.giver == Giver::Item),
                "item {} starts quest {}, which is no quest with giver = \"item\"",
                t.entry,
                t.start_quest
            );
        }
        let _ = SHIPPED_OBJECTS.set(objects);
        quests
    })
}

static SHIPPED_OBJECTS: OnceLock<Vec<QuestObject>> = OnceLock::new();

fn added_objects() -> &'static RwLock<Vec<&'static QuestObject>> {
    static ADDED: RwLock<Vec<&'static QuestObject>> = RwLock::new(Vec::new());
    &ADDED
}

/// Every GameObject the use objectives click: the shipped ones, then any a test added.
pub(crate) fn objects() -> Vec<&'static QuestObject> {
    shipped();
    SHIPPED_OBJECTS
        .get()
        .into_iter()
        .flatten()
        .chain(added_objects().read().unwrap().iter().copied())
        .collect()
}

fn added() -> &'static RwLock<Vec<&'static Quest>> {
    static ADDED: RwLock<Vec<&'static Quest>> = RwLock::new(Vec::new());
    &ADDED
}

/// Adds the quests in `toml` (the shape of `content/quests.toml`) alongside the shipped ones, for
/// the rest of the process; an id already present is left as it is. A test seam
/// (`Realm::with_quests`), so a player script can try a quest the content does not ship.
pub(crate) fn add(toml: &str) {
    let parsed: QuestFile = toml::from_str(toml).expect("the added quests parse");
    {
        // Looked up before the write lock, which a lookup would wait on.
        let new: Vec<_> = parsed
            .item
            .into_iter()
            .filter(|t| item_template(t.entry).is_none())
            .collect();
        let mut extra = added_items().write().unwrap();
        for t in new {
            extra.item.push(Box::leak(Box::new(t)));
        }
        for d in parsed.starter_drop {
            extra.starter_drop.push(Box::leak(Box::new(d)));
        }
    }
    for o in parsed.gameobject {
        shipped();
        let shipped_here = SHIPPED_OBJECTS
            .get()
            .into_iter()
            .flatten()
            .any(|s| s.guid == o.guid);
        let mut extra = added_objects().write().unwrap();
        if !shipped_here && extra.iter().all(|known| known.guid != o.guid) {
            extra.push(Box::leak(Box::new(o)));
        }
    }
    for q in parsed.quest {
        if quest(q.id).is_some() {
            continue;
        }
        check(&q, &|id| quest(id).is_some() || id == q.id, &|entry| {
            objects().iter().any(|o| o.entry == entry) || crate::phased::is_door(entry)
        });
        // Parallel tests add the same quests; the check and the push share one lock.
        let mut extra = added().write().unwrap();
        if extra.iter().all(|a| a.id != q.id) {
            extra.push(Box::leak(Box::new(q)));
        }
    }
}

/// What the content load refuses in `q`; `known` says whether a quest id exists and `object`
/// whether a use objective's GameObject entry is declared.
fn check(q: &Quest, known: &dyn Fn(u32) -> bool, object: &dyn Fn(u32) -> bool) {
    assert!(
        (q.giver == Giver::Unset) != q.giver_object.is_none(),
        "quest {} needs exactly one of `giver` and `giver_object`",
        q.id
    );
    if let Some(entry) = q.giver_object {
        assert!(
            object(entry),
            "quest {} is given by GameObject {entry}, which no `gameobject` row declares",
            q.id
        );
        assert!(
            q.ender.is_some(),
            "quest {} is given by a GameObject and names no ender",
            q.id
        );
    }
    if q.giver == Giver::Item {
        assert!(
            q.ender.is_some(),
            "quest {} starts from an item and names no ender",
            q.id
        );
        assert!(
            starter_of(q.id).is_some(),
            "quest {} starts from an item, and no item in content/quest_items.toml starts it",
            q.id
        );
    }
    for npc in [q.giver.npc(), q.ender].into_iter().flatten() {
        let hub = crate::hub::npcs()
            .iter()
            .any(|n| n.entry == npc && n.gives_quests());
        assert!(
            hub || friendly_spawn(npc),
            "quest {} names {npc}, who is no quest giver in content/hub.toml and no \
                     friendly spawn anywhere",
            q.id,
        );
    }
    assert!(
        q.choices.len() <= 6,
        "quest {} offers more than six reward choices",
        q.id
    );
    assert!(
        q.rewards.len() + usize::from(q.tokens > 0) <= 4,
        "quest {} gives more than four reward items",
        q.id
    );
    for &item in q
        .choices
        .iter()
        .chain(q.rewards.iter().chain(&q.gives).map(|(e, _)| e))
    {
        assert!(
            crate::items::template(item).is_some(),
            "quest {} offers item {item}, which the content does not have",
            q.id
        );
    }
    for &p in q.after.ids() {
        assert!(
            known(p),
            "quest {} comes after quest {p}, which the content does not have",
            q.id
        );
    }
    for t in &q.talk {
        let hub = crate::hub::npcs()
            .iter()
            .any(|n| n.entry == t.npc && n.gossips());
        assert!(
            hub || friendly_spawn(t.npc),
            "quest {} talks to {}, who is no gossiping Hub NPC and no friendly spawn",
            q.id,
            t.npc
        );
    }
    for &(entry, count) in &q.use_object {
        assert!(
            count > 0 && object(entry),
            "quest {} uses GameObject {entry}, which no `gameobject` row declares (or asks for 0)",
            q.id
        );
    }
    assert_eq!(
        q.kind == Kind::Advancement,
        q.advancement.is_some(),
        "quest {}'s kind and its `advancement` table disagree",
        q.id
    );
    if let Some(a) = &q.advancement {
        assert!(
            crate::callings::calling(&a.from).is_some()
                && (!a.completable || crate::content::Calling::from_key(&a.to).is_some()),
            "quest {}'s Advancement names a Calling the realm does not have",
            q.id
        );
        assert!(
            q.kill.is_empty()
                && q.talk.is_empty()
                && q.use_object.is_empty()
                && q.collect.is_empty()
                && q.explore.is_empty()
                && q.discover.is_empty(),
            "Advancement Quest {} asks for more than a word",
            q.id
        );
    }
    assert!(
        q.kill.len() + q.talk.len() + q.use_object.len() + q.explore.listed().len() <= OBJECTIVES
            && q.collect.len() <= OBJECTIVES,
        "quest {} has more than {OBJECTIVES} kill, talk and use objectives (listed explore \
         spots count too), or collect objectives",
        q.id
    );
    for &area in &q.discover {
        assert!(
            crate::areas::get(area).is_some(),
            "quest {} discovers area {area}, which is not in content/areas.toml",
            q.id
        );
    }
    for c in &q.collect {
        assert!(
            item_template(c.item).is_some(),
            "quest {}'s item {} is not in content/quest_items.toml",
            q.id,
            c.item
        );
    }
}

/// Every quest: the shipped ones, then any a test added.
pub(crate) fn quests() -> Vec<&'static Quest> {
    shipped()
        .iter()
        .chain(added().read().unwrap().iter().copied())
        .collect()
}

pub(crate) fn quest(id: u32) -> Option<&'static Quest> {
    shipped()
        .iter()
        .find(|q| q.id == id)
        .or_else(|| added().read().unwrap().iter().copied().find(|q| q.id == id))
}

/// The quests `giver` offers, in the content's order.
pub(crate) fn offered_by(giver: u32) -> impl Iterator<Item = &'static Quest> {
    quests()
        .into_iter()
        .filter(move |q| q.giver.npc() == Some(giver))
}

/// The quests the GameObject with entry `object` offers, in the content's order.
pub(crate) fn offered_by_object(object: u32) -> impl Iterator<Item = &'static Quest> {
    quests()
        .into_iter()
        .filter(move |q| q.giver_object == Some(object))
}

/// The quests `ender` takes in, in the content's order.
pub(crate) fn ended_by(ender: u32) -> impl Iterator<Item = &'static Quest> {
    quests().into_iter().filter(move |q| q.ender() == ender)
}

/// Whether the creature `entry` gives or takes in any quest: a Field Giver, whose create carries
/// the quest giver flag.
pub(crate) fn field_giver(entry: u32) -> bool {
    friendly_spawn(entry)
        && quests()
            .iter()
            .any(|q| q.giver.npc() == Some(entry) || q.ender() == entry)
}

/// Whether any quest has a talk objective with the creature `entry`.
pub(crate) fn talk_target(entry: u32) -> bool {
    friendly_spawn(entry)
        && quests()
            .iter()
            .any(|q| q.talk.iter().any(|t| t.npc == entry))
}

#[derive(Deserialize)]
struct ItemTable {
    item: Vec<items::Template>,
    #[serde(default)]
    starter_drop: Vec<StarterDrop>,
}

fn shipped_items() -> &'static ItemTable {
    static ITEMS: OnceLock<ItemTable> = OnceLock::new();
    ITEMS.get_or_init(|| {
        toml::from_str(include_str!("../../content/quest_items.toml"))
            .expect("content/quest_items.toml parses")
    })
}

/// The quest items and starter drops a test added.
#[derive(Default)]
struct AddedItems {
    item: Vec<&'static items::Template>,
    starter_drop: Vec<&'static StarterDrop>,
}

fn added_items() -> &'static RwLock<AddedItems> {
    static ADDED: OnceLock<RwLock<AddedItems>> = OnceLock::new();
    ADDED.get_or_init(Default::default)
}

/// A quest item's (or the Token's) template.
pub(crate) fn item_template(entry: u32) -> Option<&'static items::Template> {
    static BY_ENTRY: OnceLock<HashMap<u32, &'static items::Template>> = OnceLock::new();
    BY_ENTRY
        .get_or_init(|| shipped_items().item.iter().map(|t| (t.entry, t)).collect())
        .get(&entry)
        .copied()
        .or_else(|| {
            let added = added_items().read().unwrap();
            added.item.iter().copied().find(|t| t.entry == entry)
        })
}

/// The Quest Starter that begins `quest`.
pub(crate) fn starter_of(quest: u32) -> Option<&'static items::Template> {
    shipped_items()
        .item
        .iter()
        .find(|t| t.start_quest == quest)
        .or_else(|| {
            let added = added_items().read().unwrap();
            added.item.iter().copied().find(|t| t.start_quest == quest)
        })
}

/// Every Quest Starter's drop.
pub(crate) fn starter_drops() -> Vec<&'static StarterDrop> {
    shipped_items()
        .starter_drop
        .iter()
        .chain(added_items().read().unwrap().starter_drop.iter().copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn northshires_bounties_and_the_abbey_relics_have_their_range_giver_and_objective() {
        // id, kind, giver, Tier offered from, kills, collected item.
        let rows: [(u32, Kind, u32, u8, &[u32], Option<u32>); 5] = [
            (94005, Kind::Bounty, 196, 0, &[299, 69], None),
            (94006, Kind::Bounty, 196, 0, &[], Some(980002)),
            (94007, Kind::Bounty, 823, 0, &[6, 257], None),
            (94008, Kind::Bounty, 197, 0, &[38], None),
            (94009, Kind::Repeatable, 197, 1, &[], Some(980003)),
        ];
        for (id, kind, giver, tier, kills, item) in rows {
            let q = super::quest(id).unwrap_or_else(|| panic!("no quest {id}"));
            assert!(q.kind == kind, "{id}'s kind");
            assert!(
                matches!(q.giver, Giver::Npc(g) if g == giver),
                "{id}'s giver"
            );
            assert_eq!(q.min_tier(), tier, "{id}'s Tier");
            let killed: Vec<u32> = q.kill.iter().map(|&(e, _)| e).collect();
            assert_eq!(killed, kills, "{id}'s kills");
            assert_eq!(q.collect.first().map(|c| c.item), item, "{id}'s item");
        }
        let relics = &super::quest(94009).unwrap().collect[0];
        let source = |entry| {
            relics
                .sources
                .iter()
                .find(|s| s.from.contains(&entry))
                .unwrap()
        };
        assert_eq!((source(96003).chance, source(96003).count), (35, 1));
        assert_eq!((source(96004).chance, source(96004).count), (100, 1));
        assert_eq!((source(96005).chance, source(96005).count), (100, 3));
    }

    #[test]
    fn the_eyes_corpse_holds_three_resonant_ore_and_the_dig_orders() {
        let ore = &super::quest(94404).unwrap().collect[0];
        let source = |entry| {
            ore.sources
                .iter()
                .find(|s| s.from.contains(&entry))
                .unwrap()
        };
        assert_eq!(ore.item, 980017);
        assert_eq!((source(96017).chance, source(96017).count), (35, 1));
        assert_eq!((source(96018).chance, source(96018).count), (35, 1));
        assert_eq!((source(96019).chance, source(96019).count), (100, 1));
        assert_eq!((source(96022).chance, source(96022).count), (100, 3));
        let orders = starter_drops()
            .into_iter()
            .find(|d| d.item == 980018)
            .expect("the Dig Orders drop");
        assert_eq!((orders.from.as_slice(), orders.chance), (&[96022][..], 100));
        assert_eq!(orders.until_done, Some(94403));
        let wanted = &super::quest(94403).unwrap().collect;
        assert_eq!((wanted[0].item, wanted[0].count), (980018, 1));
    }

    #[test]
    fn jasperlodes_quests_chain_from_the_keystone_to_the_orders() {
        let q = |id| super::quest(id).unwrap();
        assert_eq!(q(94401).after.ids(), [94303]);
        assert_eq!(q(94401).use_object, [(98002, 1)]);
        assert_eq!(q(94401).ender(), 96015);
        assert_eq!(q(94402).after.ids(), [94401]);
        assert_eq!(q(94402).kill, [(96022, 1)]);
        assert_eq!((q(94402).money, q(94402).tokens), (1000, 3));
        assert!(q(94402).choices.is_empty());
        assert_eq!(q(94403).after.ids(), [94402]);
        assert!(matches!(q(94403).giver, Giver::Npc(240)));
        let ore = q(94404);
        assert!(matches!(ore.giver, Giver::Npc(514)));
        assert!(ore.after.ids().is_empty());
        assert!(!ore.offered_at_tier(0) && ore.offered_at_tier(1) && ore.offered_at_tier(3));
        assert_eq!((ore.money, ore.tokens), (400, 3));
    }

    #[test]
    fn stone_cairns_story_quests_chain_and_serane_drops_the_shard() {
        let q = |id| super::quest(id).unwrap();
        assert!(q(94501).after.ids().is_empty());
        assert_eq!(q(94501).use_object, [(980104, 5)]);
        assert_eq!(q(94501).kill, [(96023, 6)]);
        assert_eq!(q(94502).after.ids(), [94501, 94403]);
        assert_eq!(q(94502).kill, [(96025, 1)]);
        assert_eq!((q(94502).money, q(94502).tokens), (800, 3));
        assert!(q(94502).choices.is_empty());
        assert_eq!(q(94503).after.ids(), [94502]);
        assert!(matches!(q(94503).giver, Giver::Item));
        assert_eq!(q(94503).ender(), 190062);
        let shard = starter_drops()
            .into_iter()
            .find(|d| d.item == 980019)
            .expect("the Resonant Shard drops");
        assert_eq!((shard.from.as_slice(), shard.chance), (&[96025][..], 100));
        assert_eq!(crate::items::template(980019).unwrap().start_quest, 94503);
    }

    #[test]
    fn stone_cairns_encounters_have_their_waves() {
        let all = crate::encounters::content();
        let by = |key: &str| all.iter().find(|e| e.key == key).unwrap();
        let daubers = by("vigil_daubers");
        assert_eq!(daubers.quest, 94501);
        assert!(matches!(daubers.start, crate::encounters::Start::UseDone));
        assert_eq!(daubers.wave.len(), 1);
        assert_eq!(daubers.wave[0].creatures[0].count, 6);
        let rites = by("cairn_rites");
        assert_eq!(rites.quest, 94502);
        assert!(matches!(rites.start, crate::encounters::Start::Accepted));
        assert_eq!(rites.wave.len(), 3);
        let last: Vec<u32> = rites.wave[2].creatures.iter().map(|c| c.entry).collect();
        assert!(last.contains(&96025), "Serane leads the last wave");
        assert_eq!(rites.wave[0].creatures[0].count, 3);
    }

    #[test]
    fn stone_cairns_bounties_have_their_range_giver_and_objective() {
        // id, giver, kills, uses, collected item; all offered from Tier 1.
        let rows: [(u32, u32, &[u32], &[u32], Option<u32>); 4] = [
            (94505, 190061, &[96026, 96027], &[], None),
            (94506, 190061, &[], &[980105], None),
            (94507, 190061, &[96028], &[], None),
            (94508, 190060, &[], &[], Some(980020)),
        ];
        for (id, giver, kills, uses, item) in rows {
            let q = super::quest(id).unwrap_or_else(|| panic!("no quest {id}"));
            assert!(q.kind == Kind::Bounty, "{id}'s kind");
            assert!(
                matches!(q.giver, Giver::Npc(g) if g == giver),
                "{id}'s giver"
            );
            assert_eq!(q.min_tier(), 1, "{id}'s Tier");
            assert_eq!(q.tokens, 1, "{id}'s Token");
            let killed: Vec<u32> = q.kill.iter().map(|&(e, _)| e).collect();
            assert_eq!(killed, kills, "{id}'s kills");
            let used: Vec<u32> = q.use_object.iter().map(|&(e, _)| e).collect();
            assert_eq!(used, uses, "{id}'s uses");
            assert_eq!(q.collect.first().map(|c| c.item), item, "{id}'s item");
        }
        assert_eq!(super::quest(94506).unwrap().use_object, [(980105, 6)]);
        let marks = &super::quest(94508).unwrap().collect[0];
        assert_eq!(
            (marks.count, marks.chance, marks.from.as_slice()),
            (8, 50, &[96028][..])
        );
    }

    #[test]
    fn goldshires_bounties_and_wrecked_cart_have_their_range_giver_and_objective() {
        // id, giver, kills, collected item; all offered from Tier 1.
        let rows: [(u32, u32, &[u32], Option<u32>); 4] = [
            (94105, 963, &[118, 30], None),
            (94106, 514, &[], Some(980006)),
            (94107, 963, &[116], None),
            (94108, 241, &[], Some(980007)),
        ];
        for (id, giver, kills, item) in rows {
            let q = super::quest(id).unwrap_or_else(|| panic!("no quest {id}"));
            assert!(q.kind == Kind::Bounty, "{id}'s kind");
            assert!(
                matches!(q.giver, Giver::Npc(g) if g == giver),
                "{id}'s giver"
            );
            assert_eq!(q.min_tier(), 1, "{id}'s Tier");
            assert_eq!(q.tokens, 1, "{id}'s Token");
            let killed: Vec<u32> = q.kill.iter().map(|&(e, _)| e).collect();
            assert_eq!(killed, kills, "{id}'s kills");
            assert_eq!(q.collect.first().map(|c| c.item), item, "{id}'s item");
        }
        let cart = super::quest(94103).unwrap();
        assert_eq!(
            (cart.giver, cart.ender(), cart.money),
            (Giver::Npc(295), 240, 400)
        );
        assert_eq!(cart.collect[0].from, [96011]);
        assert_eq!(cart.collect[0].chance, 100);
        assert!(crate::mobs::template(96011).is_some_and(|t| t.rank == 0));
        assert_eq!(super::quest(94101).unwrap().gives, [(980004, 1)]);
    }

    /// A quest of `kind` at level 10, as the content would write it.
    fn quest(kind: &str) -> Quest {
        toml::from_str(&format!(
            r#"
            id = 1
            kind = "{kind}"
            giver = 1
            level = 10
            title = ""
            objectives = ""
            details = ""
            progress = ""
            completion = ""
            money = 0
            tokens = 3
            "#
        ))
        .expect("a quest")
    }

    #[test]
    fn a_repeatable_quest_can_be_retaken_and_pays_a_full_level() {
        let q = quest("repeatable");
        assert!(q.retakable());
        // Level 10's whole 7600 XP; level 15's 14400 for a level 15 player.
        assert_eq!((q.reward_xp(10), q.reward_xp(15)), (7600, 14400));
    }

    #[test]
    fn a_hunting_quest_cannot_be_retaken_and_pays_half_its_level() {
        let q = quest("hunting");
        assert!(!q.retakable());
        assert_eq!((q.reward_xp(10), q.reward_xp(15)), (3800, 3800));
    }

    #[test]
    fn a_bounty_can_be_retaken_and_pays_half_a_level() {
        let q = quest("bounty");
        assert!(q.retakable());
        assert_eq!((q.reward_xp(10), q.reward_xp(15)), (3800, 3800));
    }

    #[test]
    fn a_quest_is_offered_from_its_levels_tier_up_unless_the_content_sets_one() {
        let q: Quest =
            toml::from_str(&format!("kind = \"bounty\"\n{}", quest_body())).expect("a quest");
        // Level 10 is Tier 1; there is no maximum.
        assert!(!q.offered_at_tier(0) && q.offered_at_tier(1) && q.offered_at_tier(3));
        let later: Quest = toml::from_str(&format!(
            "kind = \"bounty\"\nmin_tier = 2\n{}",
            quest_body()
        ))
        .expect("a quest");
        assert!(!later.offered_at_tier(1) && later.offered_at_tier(2));
        // A Tier 2 character is offered a level 3 quest.
        let low: Quest = toml::from_str(&format!(
            "kind = \"hunting\"\n{}",
            quest_body().replace("level = 10", "level = 3")
        ))
        .expect("a quest");
        assert!(low.offered_at_tier(0) && low.offered_at_tier(2));
        // Unshrunk however far above it the player stands.
        assert_eq!(q.reward_xp(11), q.reward_xp(40));
    }

    fn quest_body() -> &'static str {
        r#"id = 1
        giver = 1
        level = 10
        title = ""
        objectives = ""
        details = ""
        progress = ""
        completion = ""
        money = 0
        tokens = 1"#
    }

    #[test]
    fn a_quest_without_a_kind_is_no_quest() {
        let no_kind = r#"
            id = 1
            giver = 1
            level = 10
            title = ""
            objectives = ""
            details = ""
            progress = ""
            completion = ""
            money = 0
            tokens = 1
        "#;
        assert!(toml::from_str::<Quest>(no_kind).is_err());
    }
}
