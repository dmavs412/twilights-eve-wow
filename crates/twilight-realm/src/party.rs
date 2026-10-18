//! **Everyone grouped** (#149): every player in the world is in the one party, in the order they
//! came in, the first its leader. Each client is sent the roster when it changes and the others'
//! health, power, level and place each second, for the stock party frames.

use benilla_protocol::messages::{
    member_status, GroupLootInfo, GroupMemberEntry, PartyMemberStatsInfo,
};
use benilla_protocol::SessionEvent;

use crate::{Realm, SessionId};

/// Seconds between one round of member stats and the next.
const STATS_EVERY_SECS: f32 = 1.0;

/// The party: its players in the order they came in, and the clock of the stats rounds.
#[derive(Default)]
pub(crate) struct Party {
    members: Vec<u64>,
    since_stats: f32,
}

/// One player in the world, as the party sees them.
struct Member {
    session: SessionId,
    guid: u64,
    name: String,
    stats: PartyMemberStatsInfo,
}

impl Realm {
    /// Everyone in the world, the served session included.
    fn members(&self) -> Vec<Member> {
        let worlds = std::iter::once((self.active, &self.world))
            .chain(self.parked.iter().map(|(id, s)| (*id, &s.world)));
        worlds
            .filter_map(|(session, w)| {
                let w = w.as_ref()?;
                let c = self.characters.get(w.guid)?;
                Some(Member {
                    session,
                    guid: c.guid,
                    name: c.name.clone(),
                    stats: w.party_stats(c),
                })
            })
            .collect()
    }

    /// The party's tick: the roster to everyone when someone came or went, and each second the
    /// others' stats.
    pub(crate) fn party_tick(&mut self, dt: f32) {
        let mut all = self.members();
        // Those who came in on the same tick join in the order they connected.
        all.sort_by_key(|m| m.session);
        let mut members = self.party.members.clone();
        members.retain(|g| all.iter().any(|m| m.guid == *g));
        for m in &all {
            if !members.contains(&m.guid) {
                members.push(m.guid);
            }
        }
        let changed = members != self.party.members;
        let was = self.party.members.len();
        self.party.members = members;
        if changed {
            let leader = self.party.members.first().copied().unwrap_or(0);
            for me in &all {
                let event = if all.len() < 2 {
                    // Alone again: the group is gone.
                    if was < 2 {
                        continue;
                    }
                    SessionEvent::GroupDestroyed
                } else {
                    roster(&all, me.guid, leader, &self.party.members)
                };
                self.send_to(me.session, event);
            }
        }
        self.party.since_stats += dt;
        if self.party.since_stats < STATS_EVERY_SECS && !changed {
            return;
        }
        self.party.since_stats = 0.0;
        for me in &all {
            for other in all.iter().filter(|o| o.guid != me.guid) {
                let event = SessionEvent::PartyMemberStats {
                    guid: other.guid,
                    full: true,
                    info: Box::new(other.stats.clone()),
                };
                self.send_to(me.session, event);
            }
        }
    }

    /// The party's line from the served player (`/p`): to everyone in the world.
    pub(crate) fn party_line(&mut self, line: SessionEvent) {
        for m in self.members() {
            self.send_to(m.session, line.clone());
        }
    }
}

/// `SMSG_GROUP_LIST` as `me` gets it: everyone else, in party order.
fn roster(all: &[Member], me: u64, leader: u64, order: &[u64]) -> SessionEvent {
    let members = order
        .iter()
        .filter(|&&g| g != me)
        .filter_map(|g| all.iter().find(|m| m.guid == *g))
        .map(|m| GroupMemberEntry {
            name: m.name.clone(),
            guid: m.guid,
            status: m.stats.status.unwrap_or(member_status::ONLINE),
            flags: 0,
        })
        .collect();
    SessionEvent::GroupList {
        // A party, not a raid.
        group_type: 0,
        own_flags: 0,
        members,
        leader,
        // Free-for-all: each player's loot is their own anyway (#148).
        loot: Some(GroupLootInfo {
            method: 0,
            master: 0,
            threshold: 2,
        }),
    }
}
