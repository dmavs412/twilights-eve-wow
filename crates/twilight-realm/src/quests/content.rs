//! The quest content: the Repeatable Quests the Hub's givers offer (`content/quests.toml`), and the
//! items those quests use (`content/quest_items.toml`).

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::items;

/// A collect objective: `count` of `item`, dropping `from` these creatures at `chance` percent
/// while the quest wants more.
#[derive(Deserialize)]
pub(crate) struct Collect {
    pub(crate) item: u32,
    pub(crate) count: u32,
    pub(crate) chance: u32,
    pub(crate) from: Vec<u32>,
}

/// A Repeatable Quest.
#[derive(Deserialize)]
pub(crate) struct Quest {
    pub(crate) id: u32,
    pub(crate) giver: u32,
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
    /// `(creature entry, count)`.
    #[serde(default)]
    pub(crate) kill: Vec<(u32, u32)>,
    #[serde(default)]
    pub(crate) collect: Vec<Collect>,
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
    /// The quest log's header the quest goes under.
    pub(crate) fn zone_or_sort(&self) -> i32 {
        self.zone_or_sort.unwrap_or(crate::THE_BARRENS as i32)
    }

    /// The lowest level it is offered at: four below its own unless the content says otherwise.
    pub(crate) fn min_level(&self) -> u8 {
        self.min_level
            .unwrap_or_else(|| self.level.saturating_sub(4).max(1))
    }
}

#[derive(Deserialize)]
struct QuestFile {
    quest: Vec<Quest>,
}

/// At most this many kill and this many collect objectives (`QUEST_OBJECTIVES_COUNT`).
const OBJECTIVES: usize = 4;

fn file() -> &'static QuestFile {
    static FILE: OnceLock<QuestFile> = OnceLock::new();
    FILE.get_or_init(|| {
        let file: QuestFile = toml::from_str(include_str!("../../content/quests.toml"))
            .expect("content/quests.toml parses");
        for q in &file.quest {
            assert!(
                crate::hub::npcs()
                    .iter()
                    .any(|n| n.entry == q.giver && n.gives_quests()),
                "quest {} names giver {}, who is no quest giver in content/hub.toml",
                q.id,
                q.giver
            );
            if let Some(a) = &q.advancement {
                assert!(
                    crate::callings::calling(&a.from).is_some()
                        && (!a.completable || crate::content::Calling::from_key(&a.to).is_some()),
                    "quest {}'s Advancement names a Calling the realm does not have",
                    q.id
                );
                assert!(
                    q.kill.is_empty() && q.collect.is_empty(),
                    "Advancement Quest {} asks for more than a word",
                    q.id
                );
            }
            assert!(
                q.kill.len() <= OBJECTIVES && q.collect.len() <= OBJECTIVES,
                "quest {} has more than {OBJECTIVES} objectives of a kind",
                q.id
            );
            for c in &q.collect {
                assert!(
                    item_template(c.item).is_some(),
                    "quest {}'s item {} is not in content/quest_items.toml",
                    q.id,
                    c.item
                );
            }
        }
        file
    })
}

pub(crate) fn quests() -> &'static [Quest] {
    &file().quest
}

pub(crate) fn quest(id: u32) -> Option<&'static Quest> {
    quests().iter().find(|q| q.id == id)
}

/// The quests `giver` offers, in the content's order.
pub(crate) fn offered_by(giver: u32) -> impl Iterator<Item = &'static Quest> {
    quests().iter().filter(move |q| q.giver == giver)
}

/// A quest item's (or the Token's) template.
pub(crate) fn item_template(entry: u32) -> Option<&'static items::Template> {
    #[derive(Deserialize)]
    struct Table {
        item: Vec<items::Template>,
    }
    static ITEMS: OnceLock<HashMap<u32, items::Template>> = OnceLock::new();
    ITEMS
        .get_or_init(|| {
            let table: Table = toml::from_str(include_str!("../../content/quest_items.toml"))
                .expect("content/quest_items.toml parses");
            table.item.into_iter().map(|t| (t.entry, t)).collect()
        })
        .get(&entry)
}
