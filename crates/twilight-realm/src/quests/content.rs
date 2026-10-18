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
}

/// A Quest Starter's drop (`content/quest_items.toml`'s `starter_drop`): `item` drops `from`
/// these creatures at `chance` percent, but only while the player has neither taken nor
/// completed the quest it starts and carries none already.
#[derive(Deserialize)]
pub(crate) struct StarterDrop {
    pub(crate) item: u32,
    pub(crate) from: Vec<u32>,
    pub(crate) chance: u32,
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

/// How often a quest can be done (`GLOSSARY.md`).
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    /// Once per Tier: Advancement forgets it.
    Hunting,
    /// A Dungeon's: taken again as soon as it is handed in.
    Repeatable,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Giver {
    Npc(u32),
    Item,
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
            Giver::Item => None,
        }
    }
}

/// A quest.
#[derive(Deserialize)]
pub(crate) struct Quest {
    pub(crate) id: u32,
    pub(crate) kind: Kind,
    pub(crate) giver: Giver,
    /// Who takes it in, when not its giver: a person found in the field.
    #[serde(default)]
    ender: Option<u32>,
    /// The quest that must be handed in before this one is offered.
    #[serde(default)]
    pub(crate) after: Option<u32>,
    pub(crate) level: u8,
    min_level: Option<u8>,
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
    /// `(creature entry, count)`.
    #[serde(default)]
    pub(crate) kill: Vec<(u32, u32)>,
    #[serde(default)]
    pub(crate) collect: Vec<Collect>,
    #[serde(default)]
    pub(crate) explore: Option<Explore>,
    /// A discover objective: these `AreaTable.dbc` areas (content/areas.toml), met once the
    /// character has discovered every one.
    #[serde(default)]
    pub(crate) discover: Vec<u32>,
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
            .expect("the content load checks a Quest Starter's quest names its ender")
    }

    /// The quest its ender offers next, once it is handed in (`NextQuestInChain`).
    pub(crate) fn next_in_chain(&self) -> Option<&'static Quest> {
        quests()
            .into_iter()
            .find(|n| n.after == Some(self.id) && n.giver.npc() == Some(self.ender()))
    }

    /// The quest log's header the quest goes under.
    pub(crate) fn zone_or_sort(&self) -> i32 {
        self.zone_or_sort.unwrap_or(crate::THE_BARRENS as i32)
    }

    /// The XP a player of level `player` earns handing it in, before the multipliers: an
    /// Advancement Quest pays in a new Calling instead.
    pub(crate) fn reward_xp(&self, player: u8) -> u32 {
        match self.kind {
            Kind::Hunting => rules::quest::hunting_reward_xp(player, self.level),
            Kind::Repeatable => rules::quest::reward_xp(player, self.level),
            // Once per character, at the Hunting Quest's rate.
            Kind::Story => rules::quest::hunting_reward_xp(player, self.level),
            Kind::Advancement | Kind::Once => 0,
        }
    }

    /// Whether a character who has handed it in this Tier may take it again.
    pub(crate) fn retakable(&self) -> bool {
        self.kind == Kind::Repeatable
    }

    /// The lowest level it is offered at: four below its own unless the content says otherwise.
    pub(crate) fn min_level(&self) -> u8 {
        self.min_level
            .unwrap_or_else(|| self.level.saturating_sub(4).max(1))
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
}

/// At most this many kill and this many collect objectives (`QUEST_OBJECTIVES_COUNT`).
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
        }
        for q in &quests {
            check(q, &|id| quests.iter().any(|o| o.id == id));
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
        quests
    })
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
    for q in parsed.quest {
        if quest(q.id).is_some() {
            continue;
        }
        check(&q, &|id| quest(id).is_some() || id == q.id);
        added().write().unwrap().push(Box::leak(Box::new(q)));
    }
}

/// What the content load refuses in `q`; `known` says whether a quest id exists.
fn check(q: &Quest, known: &dyn Fn(u32) -> bool) {
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
    for &item in &q.choices {
        assert!(
            crate::items::template(item).is_some(),
            "quest {} offers item {item}, which the content does not have",
            q.id
        );
    }
    if let Some(p) = q.after {
        assert!(
            known(p),
            "quest {} comes after quest {p}, which the content does not have",
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
                && q.collect.is_empty()
                && q.explore.is_none()
                && q.discover.is_empty(),
            "Advancement Quest {} asks for more than a word",
            q.id
        );
    }
    assert!(
        q.kill.len() <= OBJECTIVES && q.collect.len() <= OBJECTIVES,
        "quest {} has more than {OBJECTIVES} objectives of a kind",
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
