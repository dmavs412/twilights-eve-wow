//! **Repeatable Quests** (`GLOSSARY.md`): the Hub's givers, the character's quest log and its
//! progress. A quest is data (`content/quests.toml`): who gives it, what to kill or collect, and
//! what it pays. The log lives on the character and is saved; the client reads it from the
//! `PLAYER_QUEST_LOG` slots, and the stock quest dialogs drive it ([`dialogs`]).
//!
//! Kill objectives count in the log. Collect objectives count the quest items in the bags, as the
//! client does: their drops only fall while a quest wants more.

mod content;
mod dialogs;

use benilla_protocol::SessionEvent;

use crate::characters::Char;
use crate::fields::values;
use crate::rng::Rng;

pub(crate) use content::item_template;
pub(crate) use dialogs::{gossip_rows, template_response};

/// The Token's item entry. Tokens are stacks of it in the bags: the quest dialogs pay them there
/// and the Token vendor takes them from there.
pub(crate) const TOKEN: u32 = 90001;

/// `MAX_QUEST_LOG_SIZE`.
pub(crate) const LOG_SLOTS: u8 = 20;

/// `PLAYER_QUEST_LOG_1_1`: per slot `[quest id, counters | state << 24, timer]`.
const PLAYER_QUEST_LOG_1_1: u16 = 198;
/// `QUEST_STATE_COMPLETE`, in the slot's state byte.
const QUEST_STATE_COMPLETE: u32 = 0x01;

/// A quest in the character's log.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Active {
    pub(crate) id: u32,
    /// Its `PLAYER_QUEST_LOG` slot.
    pub(crate) slot: u8,
    /// Kills so far, by kill objective.
    pub(crate) kills: [u8; 4],
    /// Whether the client was last told it is complete; worked out again at load.
    done: bool,
}

impl Active {
    pub(crate) fn new(id: u32, slot: u8, kills: [u8; 4]) -> Self {
        Self {
            id,
            slot,
            kills,
            done: false,
        }
    }
}

/// The character's quest log.
#[derive(Debug, Clone, Default)]
pub(crate) struct QuestLog {
    pub(crate) active: Vec<Active>,
    /// Every quest ever handed in, once each, in the order first finished.
    pub(crate) completed: Vec<u32>,
}

impl QuestLog {
    pub(crate) fn get(&self, id: u32) -> Option<&Active> {
        self.active.iter().find(|a| a.id == id)
    }

    /// The lowest empty slot.
    fn free_slot(&self) -> Option<u8> {
        (0..LOG_SLOTS).find(|s| self.active.iter().all(|a| a.slot != *s))
    }
}

/// One quest under way in a save. Field names are the format.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct SavedQuest {
    id: u32,
    slot: u8,
    /// Kills so far, by kill objective.
    #[serde(default)]
    kills: [u8; 4],
}

/// The log as a save writes it, in slot order.
pub(crate) fn saved(log: &QuestLog) -> Vec<SavedQuest> {
    let mut out: Vec<SavedQuest> = log
        .active
        .iter()
        .map(|a| SavedQuest {
            id: a.id,
            slot: a.slot,
            kills: a.kills,
        })
        .collect();
    out.sort_by_key(|q| q.slot);
    out
}

/// The log a save describes. A quest the realm no longer has is dropped; a bad slot, or a slot or
/// quest named twice, makes the save unreadable.
pub(crate) fn load(saved: &[SavedQuest]) -> Result<QuestLog, crate::save::SaveError> {
    let mut log = QuestLog::default();
    for s in saved {
        if s.slot >= LOG_SLOTS || log.active.iter().any(|a| a.slot == s.slot || a.id == s.id) {
            return Err(crate::save::SaveError::Unreadable);
        }
        if content::quest(s.id).is_some() {
            log.active.push(Active::new(s.id, s.slot, s.kills));
        }
    }
    Ok(log)
}

/// How many of `item` the character carries.
fn carried(c: &Char, item: u32) -> u32 {
    c.inventory
        .iter()
        .filter(|(_, i)| i.entry == item)
        .map(|(_, i)| i.count)
        .sum()
}

/// Whether every objective of `a` is met.
fn complete(c: &Char, a: &Active) -> bool {
    let Some(q) = content::quest(a.id) else {
        return false;
    };
    // Ironclad's quest: offered, and never finished.
    if q.advancement.as_ref().is_some_and(|adv| !adv.completable) {
        return false;
    }
    q.kill
        .iter()
        .zip(a.kills)
        .all(|(&(_, count), killed)| u32::from(killed) >= count)
        && q.collect.iter().all(|o| carried(c, o.item) >= o.count)
}

/// One slot's three fields: `a`'s quest, or empty.
fn slot_pairs(slot: u8, a: Option<&Active>) -> [(u16, u32); 3] {
    let base = PLAYER_QUEST_LOG_1_1 + 3 * u16::from(slot);
    let (id, counters) = a.map_or((0, 0), |a| {
        let mut counters = a
            .kills
            .iter()
            .enumerate()
            .fold(0, |acc, (i, &k)| acc | u32::from(k.min(63)) << (6 * i));
        if a.done {
            counters |= QUEST_STATE_COMPLETE << 24;
        }
        (a.id, counters)
    });
    [(base, id), (base + 1, counters), (base + 2, 0)]
}

/// The log's slots for the player's own create.
pub(crate) fn log_pairs(c: &Char) -> Vec<(u16, u32)> {
    c.quests
        .active
        .iter()
        .flat_map(|a| slot_pairs(a.slot, Some(a)))
        .collect()
}

/// Works out which quests are complete, for a character just read from a save.
pub(crate) fn settle(c: &mut Char) {
    let done: Vec<bool> = c.quests.active.iter().map(|a| complete(c, a)).collect();
    for (a, done) in c.quests.active.iter_mut().zip(done) {
        a.done = done;
    }
}

/// The player killed a creature of `entry`: every quest that wants it counts the kill.
pub(crate) fn credit_kill(c: &mut Char, entry: u32, events: &mut Vec<SessionEvent>) {
    for i in 0..c.quests.active.len() {
        let a = &c.quests.active[i];
        let Some(q) = content::quest(a.id) else {
            continue;
        };
        let Some(o) = q
            .kill
            .iter()
            .zip(a.kills)
            .position(|(&(e, count), killed)| e == entry && u32::from(killed) < count)
        else {
            continue;
        };
        let a = &mut c.quests.active[i];
        a.kills[o] += 1;
        events.push(SessionEvent::QuestObjectiveKill {
            quest_id: a.id,
            entry,
            count: u32::from(a.kills[o]),
            required: q.kill[o].1,
        });
        update_slot(c, i, events);
    }
}

/// The bags changed: tell the client which quests that completed or undid.
pub(crate) fn refresh(c: &mut Char, events: &mut Vec<SessionEvent>) {
    for i in 0..c.quests.active.len() {
        if complete(c, &c.quests.active[i]) != c.quests.active[i].done {
            update_slot(c, i, events);
        }
    }
}

/// Sends quest `i`'s slot, with its complete flag worked out afresh.
fn update_slot(c: &mut Char, i: usize, events: &mut Vec<SessionEvent>) {
    let now = complete(c, &c.quests.active[i]);
    let a = &mut c.quests.active[i];
    let finished = now && !a.done;
    a.done = now;
    events.push(values(c.guid, &slot_pairs(a.slot, Some(a))));
    if finished {
        events.push(SessionEvent::QuestObjectivesComplete { quest_id: a.id });
    }
}

/// The quest items a creature of `entry` drops for the player: one of each item a quest in the
/// log still wants, at its chance.
pub(crate) fn drops(c: &Char, entry: u32, rng: &mut Rng) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    for a in &c.quests.active {
        let Some(q) = content::quest(a.id) else {
            continue;
        };
        for o in &q.collect {
            if o.from.contains(&entry)
                && carried(c, o.item) < o.count
                && !out.iter().any(|&(item, _)| item == o.item)
                && rng.below(100) < o.chance
            {
                out.push((o.item, 1));
            }
        }
    }
    out
}
