//! The quest content: the Hunting, Repeatable and Advancement Quests the Hub's givers offer
//! (`content/quests.toml`), and the items those quests use (`content/quest_items.toml`).

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::{items, rules};

/// A collect objective: `count` of `item`, dropping `from` these creatures at `chance` percent
/// while the quest wants more.
#[derive(Deserialize)]
pub(crate) struct Collect {
    pub(crate) item: u32,
    pub(crate) count: u32,
    pub(crate) chance: u32,
    pub(crate) from: Vec<u32>,
}

/// An explore objective: a spot on Kalimdor, met when the player walks into it.
#[derive(Deserialize)]
pub(crate) struct Explore {
    /// What the log calls the place.
    pub(crate) place: String,
    /// WoW `[x, y]`.
    pub(crate) position: [f32; 2],
    /// Yards.
    pub(crate) radius: f32,
}

impl Explore {
    /// Whether the player at `at` on `map` stands in the spot.
    pub(crate) fn contains(&self, map: u32, at: [f32; 3]) -> bool {
        let [x, y] = self.position;
        map == crate::KALIMDOR && (at[0] - x).hypot(at[1] - y) <= self.radius
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
}

/// A quest.
#[derive(Deserialize)]
pub(crate) struct Quest {
    pub(crate) id: u32,
    pub(crate) kind: Kind,
    pub(crate) giver: u32,
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
        self.ender.unwrap_or(self.giver)
    }

    /// The quest its ender offers next, once it is handed in (`NextQuestInChain`).
    pub(crate) fn next_in_chain(&self) -> Option<&'static Quest> {
        quests()
            .iter()
            .find(|n| n.after == Some(self.id) && n.giver == self.ender())
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
            for npc in [q.giver, q.ender.unwrap_or(q.giver)] {
                assert!(
                    crate::hub::npcs()
                        .iter()
                        .any(|n| n.entry == npc && n.gives_quests()),
                    "quest {} names {npc}, who is no quest giver in content/hub.toml",
                    q.id,
                );
            }
            if let Some(p) = q.after {
                assert!(
                    file.quest.iter().any(|o| o.id == p),
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

/// The quests `ender` takes in, in the content's order.
pub(crate) fn ended_by(ender: u32) -> impl Iterator<Item = &'static Quest> {
    quests().iter().filter(move |q| q.ender() == ender)
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
