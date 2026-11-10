//! The **Calling Choice** (`GLOSSARY.md`): the popup a character gets at its Calling's Tier Cap,
//! listing each Calling of the next Tier it may advance into. The quests it lists carry
//! `calling_choice` in `content/quests.toml`; picking one puts that Advancement Quest in the log.

use benilla_protocol::{CallingOption, SessionEvent};

use crate::characters::Char;
use crate::quests::content::{calling_choices, quest, Kind, Quest};

/// The quests `c`'s popup lists: the Calling Choice quests that advance from its Calling.
fn options(c: &Char) -> impl Iterator<Item = &'static Quest> {
    let from = &crate::callings::of(c.calling).key;
    calling_choices().filter(move |q| q.advancement.as_ref().is_some_and(|a| a.from == *from))
}

/// The popup for `c`, when it is due: at the Tier Cap, not yet picked at this Tier, and with at
/// least one quest to list.
pub(crate) fn calling_choice(c: &Char) -> Option<SessionEvent> {
    let tier = crate::callings::of(c.calling).tier;
    if !crate::world::at_tier_cap(c) || c.chosen_tiers.contains(&tier) {
        return None;
    }
    let options: Vec<CallingOption> = options(c).map(option).collect();
    (!options.is_empty()).then_some(SessionEvent::CallingChoice { options })
}

/// The popup at login: also not while an Advancement Quest is already in the log.
pub(crate) fn at_login(c: &Char) -> Option<SessionEvent> {
    let under_way = c
        .quests
        .active
        .iter()
        .any(|a| quest(a.id).is_some_and(|q| q.kind == Kind::Advancement));
    if under_way {
        return None;
    }
    calling_choice(c)
}

fn option(q: &Quest) -> CallingOption {
    let a = q
        .advancement
        .as_ref()
        .expect("the content load checks a Calling Choice quest advances");
    let def = crate::callings::calling(&a.to).expect("the content load checks the Calling exists");
    CallingOption {
        quest: q.id,
        calling: def.key.clone(),
        name: def.name.clone(),
        icon: def.icon.clone(),
        power: def.power.id() as u8,
        blurb: def.blurb.clone(),
        skills: def.skills.iter().map(|s| s.spell_id).collect(),
        objectives: q.objectives.clone(),
    }
}

/// `RealmCommand::ChooseCalling`: puts the picked quest in the log and records the pick at the
/// character's Tier. Nothing happens unless the popup is due and `picked` is one of its options.
/// A full log pushes `QuestLogFull` and records nothing. Whether the pick was taken.
pub(crate) fn choose(c: &mut Char, picked: u32, events: &mut Vec<SessionEvent>) -> bool {
    if calling_choice(c).is_none() {
        return false;
    }
    let Some(q) = options(c).find(|q| q.id == picked) else {
        return false;
    };
    if !crate::quests::take(c, q, events) {
        return false;
    }
    let tier = crate::callings::of(c.calling).tier;
    c.chosen_tiers.push(tier);
    // A Tier 0 pick names its trainer; a later Tier's trainers are hidden, so the quest log's
    // text is the only lead.
    let line = if tier == 0 {
        let giver = q
            .giver
            .npc()
            .and_then(|entry| crate::hub::npcs().iter().find(|n| n.entry == entry))
            .map_or("your trainer", |n| n.name.as_str());
        format!("Your path is chosen. Speak with {giver} to begin.")
    } else {
        "Your path is chosen. Your quest log tells you where to begin looking.".to_owned()
    };
    events.push(crate::quests::system_line(line));
    true
}
