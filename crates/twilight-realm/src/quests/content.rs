//! The quest content: the Hub's givers and their Repeatable Quests (`content/quests.toml`), and the
//! items those quests use (`content/quest_items.toml`).

use std::collections::HashMap;
use std::sync::OnceLock;

use benilla_protocol::SessionEvent;
use serde::Deserialize;

use crate::items;

/// A quest giver at the Hub, tied to one Hunting Ground.
#[derive(Deserialize)]
pub(crate) struct Giver {
    pub(crate) entry: u32,
    /// Its spawn's low guid (24 bits).
    pub(crate) guid: u32,
    pub(crate) name: String,
    /// The Hunting Ground, under the name.
    pub(crate) subname: String,
    pub(crate) display_id: u32,
    pub(crate) level: u8,
    /// WoW `[x, y, z]`; the host's ground replaces z.
    pub(crate) position: [f32; 3],
    pub(crate) orientation: f32,
    pub(crate) greeting: String,
}

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
}

impl Quest {
    /// The lowest level it is offered at: four below its own unless the content says otherwise.
    pub(crate) fn min_level(&self) -> u8 {
        self.min_level
            .unwrap_or_else(|| self.level.saturating_sub(4).max(1))
    }
}

#[derive(Deserialize)]
struct QuestFile {
    giver: Vec<Giver>,
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
                file.giver.iter().any(|g| g.entry == q.giver),
                "quest {} names giver {}, who is not in content/quests.toml",
                q.id,
                q.giver
            );
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

pub(crate) fn givers() -> &'static [Giver] {
    &file().giver
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

/// `SMSG_CREATURE_QUERY_RESPONSE` for a quest giver; `None` for any other entry.
pub(crate) fn creature_name(entry: u32) -> Option<SessionEvent> {
    let g = givers().iter().find(|g| g.entry == entry)?;
    Some(SessionEvent::CreatureName {
        entry,
        name: Some(g.name.clone()),
        subname: Some(g.subname.clone()),
        // `CREATURE_TYPE_HUMANOID`.
        creature_type: Some(7),
        pet_family: 0,
        rank: 0,
        type_flags: 0,
        display_id: g.display_id,
        civilian: false,
        racial_leader: false,
    })
}
