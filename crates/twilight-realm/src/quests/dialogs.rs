//! The stock quest dialogs (vmangos `QuestHandler.cpp`, `GossipDef.cpp`): the `!`/`?` markers,
//! the greeting list, the accept, progress and reward panels, the hand-in, and the quest template
//! the log asks for.

use benilla_protocol::messages::{
    dialog_status, QuestComplete, QuestDetails, QuestGiverList, QuestListEntry, QuestObjective,
    QuestOfferReward, QuestRequestItems, QuestRewardItem, QuestTemplate,
};
use benilla_protocol::SessionEvent;

use super::content::{self, Quest};
use super::{complete, slot_pairs, Active};
use crate::characters::Char;
use crate::fields::values;
use crate::{equipment, items, Realm, RealmCommand};

/// `CMSG_QUEST_QUERY`'s answer for `id`; nothing for a quest the realm does not have.
pub(crate) fn template_response(id: u32) -> Option<SessionEvent> {
    let q = content::quest(id)?;
    let objectives = std::array::from_fn(|i| {
        // Kills, then talk targets (count 1), then GameObjects (a negative entry).
        let (creature, kills) = if i < q.kill.len() {
            q.kill[i]
        } else if i < q.use_slot(0) {
            (q.talk[i - q.kill.len()].npc, 1)
        } else if i < q.explore_slot(0) {
            let (entry, count) = q.use_object[i - q.use_slot(0)];
            ((-(entry as i32)) as u32, count)
        } else if i - q.explore_slot(0) < q.explore.listed().len() {
            (super::spot_entry(i), 1)
        } else {
            (0, 0)
        };
        // A listed explore row names its place in the objective text.
        let text = q
            .explore
            .listed()
            .get(i.wrapping_sub(q.explore_slot(0)))
            .map(|e| e.place.clone())
            .unwrap_or_default();
        let (item, items) = q.collect.get(i).map_or((0, 0), |o| (o.item, o.count));
        QuestObjective {
            creature_or_go: creature,
            required_count: kills,
            item_id: item,
            item_count: items,
            text,
        }
    });
    let given = fixed_rewards(q);
    let rewards = std::array::from_fn(|i| given.get(i).copied().unwrap_or((0, 0)));
    Some(SessionEvent::QuestTemplate(Box::new(QuestTemplate {
        quest_id: q.id,
        // `QUEST_METHOD_ENABLED`.
        method: 2,
        level: u32::from(q.level),
        zone_or_sort: q.zone_or_sort(),
        quest_type: 0,
        rep_objective_faction: 0,
        rep_objective_value: 0,
        next_quest_in_chain: q.next_in_chain().map_or(0, |n| n.id),
        money: q.money as i32,
        money_max_level: 0,
        reward_spell: 0,
        src_item_id: 0,
        flags: 0,
        rewards,
        choices: std::array::from_fn(|i| q.choices.get(i).map_or((0, 0), |&item| (item, 1))),
        point_map_id: 0,
        point_x: 0.0,
        point_y: 0.0,
        point_opt: 0,
        title: q.title.clone(),
        objectives_text: q.objectives.clone(),
        details: q.details.clone(),
        // The log's exploration line.
        end_text: q.explore.single().map(|e| e.text()).unwrap_or_default(),
        objectives,
    })))
}

impl RealmCommand {
    /// The quest giver commands, which [`Realm::handle_quest`] takes.
    pub(crate) fn is_quest(&self) -> bool {
        matches!(
            self,
            Self::QuestGiverStatusQuery { .. }
                | Self::QuestGiverHello { .. }
                | Self::QuestGiverQueryQuest { .. }
                | Self::QuestGiverAcceptQuest { .. }
                | Self::QuestGiverCompleteQuest { .. }
                | Self::QuestGiverRequestReward { .. }
                | Self::QuestGiverChooseReward { .. }
                | Self::QuestLogRemoveQuest { .. }
        )
    }
}

/// What a quest command did that the realm acts on.
#[derive(Default)]
struct Done {
    /// The log or the purse changed: save.
    save: bool,
    /// The quest handed in, for a Rumour to count.
    handed_in: Option<u32>,
    /// The quest taken, for a Rumour's spawns.
    accepted: Option<u32>,
}

impl Realm {
    /// A quest giver command: they need the character in the world.
    pub(crate) fn handle_quest(&mut self, command: RealmCommand) {
        let Some(w) = &mut self.world else {
            return;
        };
        let Some(c) = self.characters.get_mut(w.guid) else {
            return;
        };
        let events = &mut self.events;
        let mut done = Done::default();
        match command {
            RealmCommand::QuestGiverStatusQuery { npc } => {
                if let Some(g) = any_speaker(npc) {
                    events.push(SessionEvent::QuestGiverStatus {
                        npc,
                        status: status(c, g),
                    });
                }
            }
            RealmCommand::QuestLogRemoveQuest { slot } => {
                if let Some(i) = c.quests.active.iter().position(|a| a.slot == slot) {
                    c.quests.active.remove(i);
                    events.push(values(c.guid, &slot_pairs(slot, None)));
                    done.save = true;
                }
            }
            RealmCommand::QuestGiverHello { npc } => {
                if let Some(g) = talking_to(c, w, npc) {
                    if g.object {
                        object_hello(c, g.entry, npc, events);
                    } else {
                        hello(c, g, npc, events);
                    }
                }
            }
            RealmCommand::QuestGiverQueryQuest { npc, quest } => {
                let offered = talking_to(c, w, npc).and_then(|g| offer(c, g, quest));
                if let Some(q) = offered.or_else(|| starter_offer(c, npc, quest)) {
                    events.push(SessionEvent::QuestDetail(details(npc, q)));
                }
            }
            RealmCommand::QuestGiverAcceptQuest { npc, quest } => {
                let offered = talking_to(c, w, npc).and_then(|g| offer(c, g, quest));
                if let Some(q) = offered.or_else(|| starter_offer(c, npc, quest)) {
                    done.save = accept(c, q, events);
                    if done.save {
                        done.accepted = Some(q.id);
                    }
                }
            }
            RealmCommand::QuestGiverCompleteQuest { npc, quest } => {
                if let Some(q) = talking_to(c, w, npc).and_then(|g| in_log(c, g, quest)) {
                    turn_in(c, npc, q, events);
                }
            }
            RealmCommand::QuestGiverRequestReward { npc, quest } => {
                if let Some(q) = talking_to(c, w, npc).and_then(|g| in_log(c, g, quest)) {
                    if is_complete(c, q) {
                        events.push(SessionEvent::QuestOffer(offer_reward(npc, q)));
                    }
                }
            }
            RealmCommand::QuestGiverChooseReward { npc, quest, choice } => {
                if let Some(q) = talking_to(c, w, npc).and_then(|g| in_log(c, g, quest)) {
                    // A quest with choices takes one of them; vmangos refuses any other index.
                    let chosen = q.choices.get(choice as usize).copied();
                    if (q.choices.is_empty() || chosen.is_some())
                        && is_complete(c, q)
                        && reward(c, q, chosen, events)
                    {
                        let gained = w.reward_xp(c, q.reward_xp(c.level), events);
                        w.notorious(c, crate::rules::notoriety::quest_gain());
                        let mut items = fixed_rewards(q);
                        if let Some(item) = chosen {
                            items.push((item, 1));
                        }
                        events.push(SessionEvent::QuestComplete(QuestComplete {
                            quest_id: q.id,
                            xp: gained,
                            money: q.money,
                            items,
                        }));
                        if let Some(to) = q.advancement.as_ref().and_then(|a| {
                            crate::content::Calling::from_key(&a.to).filter(|_| a.completable)
                        }) {
                            w.advance(c, to, events);
                        }
                        // The next in the chain opens at once (vmangos `GetNextQuest`).
                        if let Some(next) = q
                            .followers()
                            .find_map(|n| talking_to(c, w, npc).and_then(|g| offer(c, g, n.id)))
                        {
                            events.push(SessionEvent::QuestDetail(details(npc, next)));
                        }
                        done.save = true;
                        done.handed_in = Some(q.id);
                    }
                }
            }
            _ => {}
        }
        if let Some(quest) = done.accepted {
            self.rumours.accept(w.guid, quest);
        }
        if let Some(quest) = done.handed_in {
            if let Some(c) = self.characters.get_mut(w.guid) {
                self.rumours.hand_in(c, quest);
            }
        }
        if done.save {
            let guid = w.guid;
            self.save(guid);
        }
    }
}

impl Realm {
    /// `CMSG_GOSSIP_HELLO` at a friendly creature in the field: a menu of its talk lines and
    /// quests, when it has any.
    pub(crate) fn field_gossip_hello(&mut self, guid: u64) {
        let Some(w) = &self.world else {
            return;
        };
        let Some(c) = self.characters.get(w.guid) else {
            return;
        };
        let Some(entry) = w.friendly_within_reach(guid, c.position) else {
            return;
        };
        let options = super::talk_options(c, entry);
        let quests = gossip_rows(c, entry);
        let hint = w.key_hint_text(guid);
        if options.is_empty() && quests.is_empty() && hint.is_none() {
            return;
        }
        self.events.push(SessionEvent::GossipMenu {
            npc: guid,
            text_id: hint.unwrap_or_else(|| super::greeting_text(entry)),
            options,
            quests,
        });
    }

    /// `CMSG_GOSSIP_SELECT_OPTION` at a friendly creature in the field.
    pub(crate) fn field_gossip_select(&mut self, guid: u64, option: u32) {
        let entry = self.world.as_ref().and_then(|w| {
            let c = self.characters.get(w.guid)?;
            w.friendly_within_reach(guid, c.position)
        });
        if let Some(entry) = entry {
            self.talk_select(guid, entry, option);
        }
    }

    /// A talk line of NPC `entry` (guid `guid`, in reach) was chosen: it gives its credit and the
    /// menu shows the reply.
    pub(crate) fn talk_select(&mut self, guid: u64, entry: u32, option: u32) {
        let Some(w) = &self.world else {
            return;
        };
        let player = w.guid;
        let Some(c) = self.characters.get_mut(player) else {
            return;
        };
        let Some(reply) = super::credit_talk(c, entry, option, &mut self.events) else {
            return;
        };
        self.events.push(SessionEvent::GossipMenu {
            npc: guid,
            text_id: reply,
            options: Vec::new(),
            quests: Vec::new(),
        });
        self.save(player);
    }
}

/// A quest giver: a Hub NPC with the quest giver flag (a Hunting Ground's giver, a person found
/// out in one, or a Tier 1 trainer, whose gossip menu lists their Line's Advancement Quests), or a
/// Field Giver, a friendly creature the realm places on any map or in a Phased Dungeon.
#[derive(Clone, Copy)]
struct Speaker {
    entry: u32,
    greeting: &'static str,
    /// `entry` is an outdoor quest GameObject's, not a creature's. It gives quests and takes none.
    object: bool,
}

/// `HIGHGUID_UNIT`, in a creature guid's top 16 bits, which carries its entry in bits 24-47.
const HIGHGUID_UNIT: u64 = 0xF130;

fn hub_speaker(npc: u64) -> Option<&'static crate::hub::Npc> {
    crate::hub::find(npc).filter(|n| n.gives_quests())
}

/// The Field Giver whose creature entry is `entry`.
fn field_speaker(entry: u32) -> Option<Speaker> {
    content::field_giver(entry).then_some(Speaker {
        entry,
        greeting: "",
        object: false,
    })
}

/// The quest giver with guid `npc`, wherever the player stands.
fn any_speaker(npc: u64) -> Option<Speaker> {
    if let Some(n) = hub_speaker(npc) {
        return Some(Speaker {
            entry: n.entry,
            greeting: &n.greeting,
            object: false,
        });
    }
    if npc >> 48 != HIGHGUID_UNIT {
        return None;
    }
    field_speaker((npc >> 24 & 0xFF_FFFF) as u32)
}

/// The quest giver `npc`, if the player stands near enough to talk to it.
fn talking_to(c: &Char, w: &crate::world::InWorld, npc: u64) -> Option<Speaker> {
    if let Some(n) = hub_speaker(npc) {
        // A Hub NPC stands on its own map only, not in a Dungeon.
        let near = c.map == n.map && crate::hub::within_reach(n, c.position);
        return near.then_some(Speaker {
            entry: n.entry,
            greeting: &n.greeting,
            object: false,
        });
    }
    if npc >> 48 == crate::phased::HIGHGUID_GAMEOBJECT {
        let entry = w.giver_object_within_reach(npc, c.position)?;
        return Some(Speaker {
            entry,
            greeting: "",
            object: true,
        });
    }
    field_speaker(w.friendly_within_reach(npc, c.position)?)
}

/// `quest`, if the Quest Starter the player carries as item `item` offers it now: the quest the
/// item begins, theirs to take. The item is the giver, as vmangos makes it one.
fn starter_offer(c: &Char, item: u64, quest: u32) -> Option<&'static Quest> {
    let (_, carried) = c.inventory.iter().find(|(_, i)| i.guid == item)?;
    items::template(carried.entry).filter(|t| t.start_quest == quest)?;
    content::quest(quest).filter(|q| q.giver == content::Giver::Item && offered(c, q))
}

// `SMSG_QUESTGIVER_QUEST_INVALID` reasons (vmangos `QuestFailedReasons`).
const INVALIDREASON_DONT_HAVE_REQ: u32 = 0;
const INVALIDREASON_QUEST_FAILED_LOW_LEVEL: u32 = 1;
/// "You have completed that quest." vmangos sends it; this client's handler shows its default
/// text for it.
const INVALIDREASON_QUEST_ALREADY_DONE: u32 = 7;
const INVALIDREASON_QUEST_ALREADY_ON: u32 = 13;

/// `CMSG_USE_ITEM` on the item at `slot`: a Quest Starter opens its quest's offer with the item as
/// the giver, or says why not. Whether the item was one.
pub(crate) fn use_starter(
    c: &Char,
    slot: crate::inventory::Pos,
    events: &mut Vec<SessionEvent>,
) -> bool {
    let Some(item) = c.inventory.get(slot) else {
        return false;
    };
    let Some(t) = items::template(item.entry).filter(|t| t.start_quest != 0) else {
        return false;
    };
    let Some(q) = content::quest(t.start_quest) else {
        return true;
    };
    let refusal = if c.quests.get(q.id).is_some() {
        Some(INVALIDREASON_QUEST_ALREADY_ON)
    } else if c.quests.completed.contains(&q.id) {
        Some(INVALIDREASON_QUEST_ALREADY_DONE)
    } else if !q.offered_at_tier(crate::callings::of(c.calling).tier) {
        Some(INVALIDREASON_QUEST_FAILED_LOW_LEVEL)
    } else if !offered(c, q) {
        Some(INVALIDREASON_DONT_HAVE_REQ)
    } else {
        None
    };
    events.push(match refusal {
        Some(reason) => SessionEvent::QuestGiverInvalid { reason },
        None => SessionEvent::QuestDetail(details(item.guid, q)),
    });
    true
}

/// `quest`, if `g` offers it to the player now: theirs to take, and not in the log already.
fn offer(c: &Char, g: Speaker, quest: u32) -> Option<&'static Quest> {
    let gives = |q: &Quest| {
        if g.object {
            q.giver_object == Some(g.entry)
        } else {
            q.giver.npc() == Some(g.entry)
        }
    };
    content::quest(quest).filter(|q| gives(q) && offered(c, q))
}

/// The click on the giver GameObject `guid`: the quest panel, or the list of its quests when it
/// has several, or nothing when it has none for the player.
pub(crate) fn object_hello(c: &Char, entry: u32, guid: u64, events: &mut Vec<SessionEvent>) {
    let g = Speaker {
        entry,
        greeting: "",
        object: true,
    };
    let open: Vec<_> = content::offered_by_object(entry)
        .filter(|q| offered(c, q))
        .collect();
    match open.as_slice() {
        [] => {}
        [q] => events.push(SessionEvent::QuestDetail(details(guid, q))),
        open => events.push(SessionEvent::QuestGreeting(QuestGiverList {
            npc: guid,
            greeting: g.greeting.to_string(),
            emote_delay: 0,
            emote: 0,
            quests: open
                .iter()
                .map(|q| QuestListEntry {
                    quest_id: q.id,
                    icon: dialog_status::AVAILABLE,
                    level: u32::from(q.level),
                    title: q.title.clone(),
                })
                .collect(),
        })),
    }
}

pub(crate) fn offered(c: &Char, q: &Quest) -> bool {
    if c.quests.get(q.id).is_some() || done_for_now(c, q) || !after_met(c, q) {
        return false;
    }
    if q.min_herbalist_level > 0
        && !(c.knows_herbalism && c.herbalist_level >= q.min_herbalist_level)
    {
        return false;
    }
    match &q.advancement {
        // At the Tier Cap of the Calling it advances from.
        Some(a) => c.calling.key() == a.from && c.level >= crate::callings::of(c.calling).cap,
        None => q.offered_at_tier(crate::callings::of(c.calling).tier),
    }
}

/// Whether the player has handed `q` in and may not take it again.
fn done_for_now(c: &Char, q: &Quest) -> bool {
    !q.retakable() && c.quests.completed.contains(&q.id)
}

/// Whether every quest `q` comes after is handed in.
fn after_met(c: &Char, q: &Quest) -> bool {
    q.after.ids().iter().all(|p| c.quests.completed.contains(p))
        && q.after_taken
            .is_none_or(|p| c.quests.get(p).is_some() || c.quests.completed.contains(&p))
}

/// `quest`, if it is in the player's log and `g` takes it in.
fn in_log(c: &Char, g: Speaker, quest: u32) -> Option<&'static Quest> {
    content::quest(quest)
        .filter(|q| !g.object && q.ender() == g.entry && c.quests.get(q.id).is_some())
}

pub(crate) fn is_complete(c: &Char, q: &Quest) -> bool {
    c.quests.get(q.id).is_some_and(|a| complete(c, a))
}

/// The giver's mark (`GetDialogStatus`): a yellow `?` for a quest to hand in, a yellow `!` for one
/// to take, a grey `?` for one under way. The `?` marks show over the quest's ender, the `!` over
/// its giver. A quest the player is too low for, an Advancement Quest that is not theirs yet, a
/// Hunting Quest done this Tier, and a quest whose earlier step is not handed in show nothing:
/// no grey `!`, so the world shows only the quests the player can take now.
fn status(c: &Char, g: Speaker) -> u32 {
    let to_hand_in = content::ended_by(g.entry).map(|q| match c.quests.get(q.id) {
        Some(a) if complete(c, a) => dialog_status::REWARD2,
        Some(_) => dialog_status::INCOMPLETE,
        None => dialog_status::NONE,
    });
    let to_take = content::offered_by(g.entry).map(|q| match c.quests.get(q.id) {
        Some(_) => dialog_status::NONE,
        None if offered(c, q) => dialog_status::AVAILABLE,
        None => dialog_status::NONE,
    });
    to_hand_in
        .chain(to_take)
        .max()
        .unwrap_or(dialog_status::NONE)
}

/// The quests the NPC `giver` lists for the player, `(quest, icon)`: the ones under way that it
/// takes in first, as vmangos's menu lists them (4 for one to hand in, 3 for one under way), then
/// the ones it has on offer.
fn rows(c: &Char, giver: u32) -> Vec<(&'static Quest, u32)> {
    let active = content::ended_by(giver)
        .filter(|q| c.quests.get(q.id).is_some())
        .map(|q| (q, if is_complete(c, q) { 4 } else { 3 }));
    let open = content::offered_by(giver)
        .filter(|q| offered(c, q))
        .map(|q| (q, dialog_status::AVAILABLE));
    active.chain(open).collect()
}

/// The quest rows of a gossip menu (`SMSG_GOSSIP_MESSAGE`), `(quest, icon, level, title)`: what
/// the NPC with entry `giver` has for the player.
pub(crate) fn gossip_rows(c: &Char, giver: u32) -> Vec<(u32, u32, u32, String)> {
    rows(c, giver)
        .into_iter()
        .map(|(q, icon)| (q.id, icon, u32::from(q.level), q.title.clone()))
        .collect()
}

/// `CMSG_QUESTGIVER_HELLO`: a lone quest opens its own panel, several open the greeting list.
fn hello(c: &Char, g: Speaker, npc: u64, events: &mut Vec<SessionEvent>) {
    match rows(c, g.entry).as_slice() {
        [(q, icon)] if *icon != dialog_status::AVAILABLE => turn_in(c, npc, q, events),
        [(q, _)] => events.push(SessionEvent::QuestDetail(details(npc, q))),
        rows => {
            let quests = rows
                .iter()
                .map(|(q, icon)| QuestListEntry {
                    quest_id: q.id,
                    icon: *icon,
                    level: u32::from(q.level),
                    title: q.title.clone(),
                })
                .collect();
            events.push(SessionEvent::QuestGreeting(QuestGiverList {
                npc,
                greeting: g.greeting.to_string(),
                emote_delay: 0,
                emote: 0,
                quests,
            }));
        }
    }
}

/// The items to choose one of.
fn choice_rewards(q: &Quest) -> Vec<QuestRewardItem> {
    q.choices
        .iter()
        .map(|&item| QuestRewardItem {
            item_id: item,
            count: 1,
            display_id: display(item),
        })
        .collect()
}

/// What every hand-in gives, `(entry, count)`: the quest's reward items.
fn fixed_rewards(q: &Quest) -> Vec<(u32, u32)> {
    q.rewards.clone()
}

/// The reward panel's "You will receive" rows.
fn reward_rows(q: &Quest) -> Vec<QuestRewardItem> {
    fixed_rewards(q)
        .into_iter()
        .map(|(item_id, count)| QuestRewardItem {
            item_id,
            count,
            display_id: display(item_id),
        })
        .collect()
}

fn display(item: u32) -> u32 {
    items::template(item).map_or(0, |t| t.display_id)
}

/// The accept panel.
fn details(npc: u64, q: &Quest) -> QuestDetails {
    QuestDetails {
        npc,
        quest_id: q.id,
        title: q.title.clone(),
        details: q.details.clone(),
        objectives: q.objectives.clone(),
        auto_finish: 0,
        choices: choice_rewards(q),
        rewards: reward_rows(q),
        money: q.money as i32,
        reward_spell: 0,
    }
}

/// The reward panel.
fn offer_reward(npc: u64, q: &Quest) -> QuestOfferReward {
    QuestOfferReward {
        npc,
        quest_id: q.id,
        title: q.title.clone(),
        offer_text: q.completion.clone(),
        auto_finish: 0,
        choices: choice_rewards(q),
        rewards: reward_rows(q),
        money: q.money as i32,
        quest_flags: 0,
        reward_spell: 0,
    }
}

/// `CMSG_QUESTGIVER_ACCEPT_QUEST`: the quest takes the lowest free log slot, and the dialog
/// closes. Whether it went in.
fn accept(c: &mut Char, q: &Quest, events: &mut Vec<SessionEvent>) -> bool {
    let taken = take(c, q, events);
    if taken {
        events.push(SessionEvent::GossipComplete);
    }
    taken
}

/// Puts `q` in the log, with its goods and the log's update fields, and no dialog to close. A full
/// log or full bags push their refusal and put nothing in. Whether it went in.
pub(crate) fn take(c: &mut Char, q: &Quest, events: &mut Vec<SessionEvent>) -> bool {
    let Some(slot) = c.quests.free_slot() else {
        events.push(SessionEvent::QuestLogFull);
        return false;
    };
    let mut room = c.inventory.clone();
    if q.gives.iter().any(|&(e, n)| room.store(e, n).is_err()) {
        events.push(equipment::inventory_failure(
            equipment::EQUIP_ERR_INVENTORY_FULL,
            0,
        ));
        return false;
    }
    for &(entry, count) in &q.gives {
        // Checked above.
        let _ = equipment::receive_from(c, entry, count, true, events);
    }
    c.quests.active.push(Active::new(q.id, slot, [0; 4]));
    let i = c.quests.active.len() - 1;
    // The bags may already hold what it asks for.
    super::update_slot(c, i, events);
    true
}

/// `CMSG_QUESTGIVER_COMPLETE_QUEST` on a quest in the log: the progress panel, or straight to the
/// reward panel for a finished quest with nothing to bring.
fn turn_in(c: &Char, npc: u64, q: &Quest, events: &mut Vec<SessionEvent>) {
    let finished = is_complete(c, q);
    if finished && q.collect.is_empty() {
        events.push(SessionEvent::QuestOffer(offer_reward(npc, q)));
        return;
    }
    events.push(SessionEvent::QuestProgress(QuestRequestItems {
        npc,
        quest_id: q.id,
        title: q.title.clone(),
        request_text: q.progress.clone(),
        emote: 0,
        close_on_cancel: 0,
        required_money: 0,
        required_items: q
            .collect
            .iter()
            .map(|o| QuestRewardItem {
                item_id: o.item,
                count: o.count,
                display_id: display(o.item),
            })
            .collect(),
        is_complete: finished,
    }));
}

/// The hand-in's goods: the collected items go, the money, reward items and `chosen` item come, and the
/// slot clears. The XP is the world's to pay. Goods that would not fit in the bags refuse the
/// hand-in before anything changes, as vmangos `Player::CanRewardQuest` does; whether it went
/// through.
fn reward(c: &mut Char, q: &Quest, chosen: Option<u32>, events: &mut Vec<SessionEvent>) -> bool {
    let given: Vec<(u32, u32)> = fixed_rewards(q)
        .into_iter()
        .chain(chosen.map(|item| (item, 1)))
        .collect();
    let mut room = c.inventory.clone();
    if given.iter().any(|&(e, n)| room.store(e, n).is_err()) {
        events.push(equipment::inventory_failure(
            equipment::EQUIP_ERR_INVENTORY_FULL,
            0,
        ));
        return false;
    }
    for o in &q.collect {
        equipment::take(c, o.item, o.count, events);
    }
    c.inventory.money = c.inventory.money.saturating_add(q.money);
    for &(entry, count) in &given {
        // Checked above.
        let _ = equipment::receive_from(c, entry, count, true, events);
        if c.inventory.ammo == 0 {
            crate::ammo::set_quietly(c, entry, events);
        }
    }
    if !c.quests.completed.contains(&q.id) {
        c.quests.completed.push(q.id);
    }
    if let Some(i) = c.quests.active.iter().position(|a| a.id == q.id) {
        let a = c.quests.active.remove(i);
        let mut pairs = slot_pairs(a.slot, None).to_vec();
        pairs.push(equipment::money_pair(&c.inventory));
        events.push(values(c.guid, &pairs));
    }
    // Handing in may undo another quest that wanted the same items.
    super::refresh(c, events);
    true
}
