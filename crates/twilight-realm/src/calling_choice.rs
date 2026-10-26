//! The **Calling Choice** (`GLOSSARY.md`): the popup a Basic Warrior gets at the Tier Cap, listing
//! each Tier 1 Calling. The quests it lists carry `calling_choice` in `content/quests.toml`;
//! picking one puts that Advancement Quest in the log.

use benilla_protocol::{CallingOption, SessionEvent};

use crate::characters::Char;
use crate::quests::content::{calling_choices, quest, Kind, Quest};

/// The popup for `c`, when it is due: a Basic Warrior at the Tier Cap who has not picked.
pub(crate) fn calling_choice(c: &Char) -> Option<SessionEvent> {
    let due = crate::callings::of(c.calling).tier == 0
        && crate::world::at_tier_cap(c)
        && !c.calling_chosen;
    due.then(|| SessionEvent::CallingChoice {
        options: calling_choices().map(option).collect(),
    })
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

/// `RealmCommand::ChooseCalling`: puts the picked quest in the log and marks the choice made.
/// Nothing happens unless the popup is due and `picked` is one of its options. A full log pushes
/// `QuestLogFull` and records nothing. Whether the pick was taken.
pub(crate) fn choose(c: &mut Char, picked: u32, events: &mut Vec<SessionEvent>) -> bool {
    if calling_choice(c).is_none() {
        return false;
    }
    let Some(q) = calling_choices().find(|q| q.id == picked) else {
        return false;
    };
    if !crate::quests::take(c, q, events) {
        return false;
    }
    c.calling_chosen = true;
    let giver = q
        .giver
        .npc()
        .and_then(|entry| crate::hub::npcs().iter().find(|n| n.entry == entry))
        .map_or("your trainer", |n| n.name.as_str());
    events.push(crate::quests::system_line(format!(
        "Your path is chosen. Speak with {giver} to begin."
    )));
    true
}
