//! **Hunting Quests** and **Repeatable Quests** (`GLOSSARY.md`): the Hub's givers, the character's
//! quest log and its progress. A quest is data (`content/quests.toml`): who gives it, how often
//! it can be done, what to kill or collect, and what it pays. The log lives on the character and
//! is saved; the client reads it from the `PLAYER_QUEST_LOG` slots, and the stock quest dialogs
//! drive it ([`dialogs`]).
//!
//! Kill objectives count in the log, an explore objective is met when the player walks into its
//! spot, and a discover objective once the character has discovered all its areas. Collect
//! objectives count the quest items in the bags, as the client does: their drops only fall while
//! a quest wants more.

pub(crate) mod content;
mod dialogs;

use benilla_protocol::messages::ChatMessage;
use benilla_protocol::SessionEvent;

use crate::characters::Char;
use crate::fields::values;
use crate::rng::Rng;

pub(crate) use content::item_template;
pub(crate) use dialogs::{
    gossip_rows, is_complete, object_hello, offered, take, template_response, use_starter,
};

/// The Token's item entry. Tokens are stacks of it in the bags: the quest dialogs pay them there
/// and the Token vendor takes them from there.
pub(crate) const TOKEN: u32 = 90001;

/// `MAX_QUEST_LOG_SIZE`.
pub(crate) const LOG_SLOTS: u8 = 20;

/// `PLAYER_QUEST_LOG_1_1`: per slot `[quest id, counters | state << 24, timer]`.
const PLAYER_QUEST_LOG_1_1: u16 = 198;
/// `QUEST_STATE_COMPLETE`, in the slot's state byte.
const QUEST_STATE_COMPLETE: u32 = 0x01;
/// `QUEST_STATE_FAILED`, in the slot's state byte.
const QUEST_STATE_FAILED: u32 = 0x02;
/// The system chat line type (`CHAT_MSG_SYSTEM`).
const CHAT_MSG_SYSTEM: u8 = 0x0A;

/// A quest in the character's log.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Active {
    pub(crate) id: u32,
    /// Its `PLAYER_QUEST_LOG` slot.
    pub(crate) slot: u8,
    /// Kills so far, by kill objective.
    pub(crate) kills: [u8; 4],
    /// Whether the player has walked into its explore objective's spot.
    pub(crate) explored: bool,
    /// Whether the client was last told it is complete; worked out again at load.
    done: bool,
    /// Whether it has failed: it gains no credit and cannot be handed in, until it is abandoned.
    pub(crate) failed: bool,
}

impl Active {
    pub(crate) fn new(id: u32, slot: u8, kills: [u8; 4]) -> Self {
        Self {
            id,
            slot,
            kills,
            explored: false,
            done: false,
            failed: false,
        }
    }
}

/// The character's quest log.
#[derive(Debug, Clone, Default)]
pub(crate) struct QuestLog {
    pub(crate) active: Vec<Active>,
    /// The quests handed in, once each, in the order first finished: every quest the character
    /// has finished, Advancement leaves it alone.
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
    /// Its explore objective met.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    explored: bool,
    /// Its `fail_if_seen` tripped.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    failed: bool,
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
            explored: a.explored,
            failed: a.failed,
        })
        .collect();
    out.sort_by_key(|q| q.slot);
    out
}

/// The log a save describes, with the quests it has handed in, and the quest ids the realm no
/// longer has, which are dropped. A bad slot, or a slot or quest named twice, makes the save
/// unreadable.
pub(crate) fn load(
    saved: &[SavedQuest],
    completed: &[u32],
) -> Result<(QuestLog, Vec<u32>), crate::save::SaveError> {
    let mut log = QuestLog::default();
    let mut dropped = Vec::new();
    for s in saved {
        if s.slot >= LOG_SLOTS || log.active.iter().any(|a| a.slot == s.slot || a.id == s.id) {
            return Err(crate::save::SaveError::Unreadable);
        }
        if content::quest(s.id).is_some() {
            let mut a = Active::new(s.id, s.slot, s.kills);
            a.explored = s.explored;
            a.failed = s.failed;
            log.active.push(a);
        } else {
            dropped.push(s.id);
        }
    }
    for &id in completed {
        if content::quest(id).is_none() {
            dropped.push(id);
        } else if !log.completed.contains(&id) {
            log.completed.push(id);
        }
    }
    Ok((log, dropped))
}

/// How many of `item` the character carries.
fn carried(c: &Char, item: u32) -> u32 {
    c.inventory.count(item)
}

/// Whether every objective of `a` is met.
fn complete(c: &Char, a: &Active) -> bool {
    let Some(q) = content::quest(a.id) else {
        return false;
    };
    if a.failed {
        return false;
    }
    // An Advancement into a Calling with no kit: offered, and never finished.
    if q.advancement.as_ref().is_some_and(|adv| !adv.completable) {
        return false;
    }
    q.kill
        .iter()
        .zip(a.kills)
        .all(|(&(_, count), killed)| u32::from(killed) >= count)
        && (0..q.talk.len()).all(|i| a.kills[q.talk_slot(i)] >= 1)
        && q.use_object
            .iter()
            .enumerate()
            .all(|(i, &(_, count))| u32::from(a.kills[q.use_slot(i)]) >= count)
        && q.collect.iter().all(|o| carried(c, o.item) >= o.count)
        && (q.explore.single().is_none() || a.explored)
        && (0..q.explore.listed().len()).all(|i| a.kills[q.explore_slot(i)] >= 1)
        && q.discover
            .iter()
            .all(|id| c.discovered.binary_search(id).is_ok())
}

/// Whether `a`'s quest has use objectives and every one is met.
pub(crate) fn uses_done(a: &Active) -> bool {
    content::quest(a.id).is_some_and(|q| {
        !q.use_object.is_empty()
            && q.use_object
                .iter()
                .enumerate()
                .all(|(i, &(_, count))| u32::from(a.kills[q.use_slot(i)]) >= count)
    })
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
        if a.failed {
            counters |= QUEST_STATE_FAILED << 24;
        } else if a.done {
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
        let Some(q) = content::quest(a.id).filter(|_| !a.failed) else {
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

/// The first talk-line index in a gossip menu; the trainer's and the innkeeper's lines sit below.
pub(crate) const TALK_OPTION: u32 = 100;
/// `NpcText` ids for a talk NPC's greeting (`GREETING_TEXT` + its entry) and a talk reply
/// (`REPLY_TEXT` + quest id x 4 + talk index), answered by [`talk_text`].
const GREETING_TEXT: u32 = 0x4000_0000;
/// `NpcText` ids for a dungeon hint (`HINT_TEXT` + the guid of the creature that carries the
/// Run's key), answered by [`talk_text`] with that holder's `hint`.
pub(crate) const HINT_TEXT: u32 = 0x3000_0000;
const REPLY_TEXT: u32 = 0x5000_0000;

/// The talk lines NPC `entry` offers `c` now: `(quest, talk index)` of each active quest that has
/// a talk objective with that NPC whose credit is not given.
fn talk_lines(c: &Char, entry: u32) -> Vec<(&'static content::Quest, usize)> {
    let mut out = Vec::new();
    for a in c.quests.active.iter().filter(|a| !a.failed) {
        let Some(q) = content::quest(a.id) else {
            continue;
        };
        for (t, talk) in q.talk.iter().enumerate() {
            if talk.npc == entry && a.kills[q.talk_slot(t)] == 0 {
                out.push((q, t));
            }
        }
    }
    out
}

/// The gossip lines of NPC `entry` that give talk credit, indexed from [`TALK_OPTION`].
pub(crate) fn talk_options(c: &Char, entry: u32) -> Vec<benilla_protocol::messages::GossipOption> {
    talk_lines(c, entry)
        .into_iter()
        .zip(TALK_OPTION..)
        .map(|((q, t), index)| benilla_protocol::messages::GossipOption {
            index,
            icon: 0,
            coded: false,
            message: q.talk[t].option.clone(),
        })
        .collect()
}

/// The text a talk NPC's greeting or reply shows, for `CMSG_NPC_TEXT_QUERY`.
pub(crate) fn talk_text(text_id: u32) -> Option<String> {
    if (HINT_TEXT..GREETING_TEXT).contains(&text_id) {
        return crate::phased::hint_text(text_id - HINT_TEXT).map(str::to_string);
    }
    if text_id >= REPLY_TEXT {
        let n = text_id - REPLY_TEXT;
        let q = content::quest(n / 4)?;
        return Some(q.talk.get((n % 4) as usize)?.reply.clone());
    }
    (text_id >= GREETING_TEXT).then(|| "Greetings.".to_string())
}

/// The text id of the greeting of the NPC `entry`.
pub(crate) fn greeting_text(entry: u32) -> u32 {
    GREETING_TEXT + entry
}

/// The player chose gossip line `option` of NPC `entry`: if it is a talk line, the credit is
/// given. The reply's text id.
pub(crate) fn credit_talk(
    c: &mut Char,
    entry: u32,
    option: u32,
    events: &mut Vec<SessionEvent>,
) -> Option<u32> {
    let (q, t) = *talk_lines(c, entry).get(option.checked_sub(TALK_OPTION)? as usize)?;
    let i = c.quests.active.iter().position(|a| a.id == q.id)?;
    c.quests.active[i].kills[q.talk_slot(t)] = 1;
    events.push(SessionEvent::QuestObjectiveKill {
        quest_id: q.id,
        entry,
        count: 1,
        required: 1,
    });
    update_slot(c, i, events);
    Some(REPLY_TEXT + q.id * 4 + t as u32)
}

/// The player used a GameObject of `entry`: every quest that wants one more counts it. Whether
/// any did.
pub(crate) fn credit_use(c: &mut Char, entry: u32, events: &mut Vec<SessionEvent>) -> bool {
    let mut any = false;
    for i in 0..c.quests.active.len() {
        let a = &c.quests.active[i];
        let Some(q) = content::quest(a.id).filter(|_| !a.failed) else {
            continue;
        };
        let Some(o) = q
            .use_object
            .iter()
            .enumerate()
            .position(|(o, &(e, count))| e == entry && u32::from(a.kills[q.use_slot(o)]) < count)
        else {
            continue;
        };
        let slot = q.use_slot(o);
        let a = &mut c.quests.active[i];
        a.kills[slot] += 1;
        events.push(SessionEvent::QuestObjectiveKill {
            quest_id: a.id,
            // A GameObject is `(-entry)|0x80000000` in the announce line.
            entry: (-(entry as i32)) as u32 | 0x8000_0000,
            count: u32::from(a.kills[slot]),
            required: q.use_object[o].1,
        });
        update_slot(c, i, events);
        any = true;
    }
    any
}

/// The creature-or-GameObject field of a listed explore row in log slot `slot`: a GameObject id
/// no content uses, so the client shows the row's `ObjectiveText` (the spot's place).
pub(crate) fn spot_entry(slot: usize) -> u32 {
    (-(slot as i32 + 1)) as u32
}

/// The player moved: every quest whose explore spot they now stand in has it explored. Whether
/// any was.
pub(crate) fn explore(c: &mut Char, events: &mut Vec<SessionEvent>) -> bool {
    let mut any = false;
    for i in 0..c.quests.active.len() {
        let a = &c.quests.active[i];
        let Some(q) = content::quest(a.id).filter(|_| !a.failed) else {
            continue;
        };
        let mut changed = q
            .explore
            .single()
            .is_some_and(|e| !a.explored && e.contains(c.map, c.position));
        if changed {
            c.quests.active[i].explored = true;
        }
        for (s, spot) in q.explore.listed().iter().enumerate() {
            let slot = q.explore_slot(s);
            let a = &mut c.quests.active[i];
            if a.kills[slot] == 0 && spot.contains(c.map, c.position) {
                a.kills[slot] = 1;
                events.push(SessionEvent::QuestObjectiveKill {
                    quest_id: a.id,
                    entry: spot_entry(slot),
                    count: 1,
                    required: 1,
                });
                changed = true;
            }
        }
        if changed {
            update_slot(c, i, events);
            any = true;
        }
    }
    any
}

/// A system chat line.
pub(crate) fn system_line(text: String) -> SessionEvent {
    SessionEvent::Chat(ChatMessage {
        chat_type: CHAT_MSG_SYSTEM,
        language: 0,
        sender_guid: 0,
        target_guid: 0,
        sender_name: None,
        channel: None,
        text,
        chat_tag: 0,
    })
}

/// A Rumour ended: its `quests` in the log fail and leave it, and its `notes` leave the bags.
pub(crate) fn end_rumour(
    c: &mut Char,
    quests: &[u32],
    notes: &[u32],
    events: &mut Vec<SessionEvent>,
) {
    for &id in quests {
        if let Some(i) = c.quests.active.iter().position(|a| a.id == id) {
            let a = c.quests.active.remove(i);
            events.push(SessionEvent::QuestFailed {
                quest_id: id,
                timed: true,
            });
            events.push(values(c.guid, &slot_pairs(a.slot, None)));
        }
    }
    for &note in notes {
        let held = carried(c, note);
        if held > 0 {
            crate::equipment::take(c, note, held, events);
        }
    }
    refresh(c, events);
}

/// A creature is in combat with the character: every active quest with a `fail_if_seen` text
/// fails, and its text goes to the character once.
pub(crate) fn seen(c: &mut Char, events: &mut Vec<SessionEvent>) {
    for i in 0..c.quests.active.len() {
        let a = &c.quests.active[i];
        let Some(text) = content::quest(a.id).and_then(|q| q.fail_if_seen.as_ref()) else {
            continue;
        };
        if a.failed {
            continue;
        }
        let a = &mut c.quests.active[i];
        a.failed = true;
        a.done = false;
        events.push(SessionEvent::QuestFailed {
            quest_id: a.id,
            timed: false,
        });
        events.push(SessionEvent::Chat(ChatMessage {
            chat_type: CHAT_MSG_SYSTEM,
            language: 0,
            sender_guid: 0,
            target_guid: 0,
            sender_name: None,
            channel: None,
            text: text.clone(),
            chat_tag: 0,
        }));
        events.push(values(c.guid, &slot_pairs(a.slot, Some(a))));
    }
}

/// Whether the character holds a quest with a `fail_if_seen` text that has not failed.
pub(crate) fn watches_for_sight(c: &Char) -> bool {
    c.quests
        .active
        .iter()
        .any(|a| !a.failed && content::quest(a.id).is_some_and(|q| q.fail_if_seen.is_some()))
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
    for a in c.quests.active.iter().filter(|a| !a.failed) {
        let Some(q) = content::quest(a.id) else {
            continue;
        };
        for o in &q.collect {
            let (chance, count) = match o.sources.iter().find(|s| s.from.contains(&entry)) {
                Some(s) => (s.chance, s.count),
                None if o.from.contains(&entry) => (o.chance, 1),
                None => continue,
            };
            let have = carried(c, o.item);
            if have < o.count
                && !out.iter().any(|&(item, _)| item == o.item)
                && rng.below(100) < chance
            {
                out.push((o.item, count.min(o.count - have)));
            }
        }
    }
    // A Quest Starter drops only while its quest is neither taken nor done, and none is carried.
    for d in content::starter_drops() {
        if let Some(quest) = d.until_done {
            if d.from.contains(&entry)
                && !c.quests.completed.contains(&quest)
                && carried(c, d.item) == 0
                && !out.iter().any(|&(item, _)| item == d.item)
                && rng.below(100) < d.chance
            {
                out.push((d.item, 1));
            }
            continue;
        }
        let Some(quest) = crate::items::template(d.item).map(|t| t.start_quest) else {
            continue;
        };
        if d.from.contains(&entry)
            && c.quests.get(quest).is_none()
            && !c.quests.completed.contains(&quest)
            && carried(c, d.item) == 0
            && !out.iter().any(|&(item, _)| item == d.item)
            && rng.below(100) < d.chance
        {
            out.push((d.item, 1));
        }
    }
    out
}
