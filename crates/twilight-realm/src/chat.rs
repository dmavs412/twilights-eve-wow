//! **Chat and emotes** (#145): say, yell and `/emote` to the players in earshot, whispers by name,
//! the one server-wide channel every player is on, text emotes to the players nearby, and the
//! name queries that put names on the lines. Ranges are vanilla's (vmangos `sWorld` defaults).

use benilla_protocol::messages::ChannelNoticeTail;
use benilla_protocol::messages::{self as m, ChatMessage};
use benilla_protocol::SessionEvent;

use crate::{Realm, SessionId};

/// How far `/say`, a custom `/emote` and a text emote carry, in yards.
const SAY_YD: f32 = 25.0;
/// How far `/yell` carries.
const YELL_YD: f32 = 300.0;
/// `Languages.dbc` 0: every player reads every line, whatever its race.
const LANGUAGE_UNIVERSAL: u32 = 0;
/// The server-wide channel, joined on entering the world: `/1`.
pub const WORLD_CHANNEL: &str = "World";

/// A player in the world: its session, guid, name, map and position.
struct Listener {
    session: SessionId,
    guid: u64,
    name: String,
    map: u32,
    position: [f32; 3],
}

impl Realm {
    /// Everyone in the world, the served session included.
    fn listeners(&self) -> Vec<Listener> {
        let worlds = std::iter::once((self.active, &self.world))
            .chain(self.parked.iter().map(|(id, s)| (*id, &s.world)));
        worlds
            .filter_map(|(session, w)| {
                let c = self.characters.get(w.as_ref()?.guid)?;
                Some(Listener {
                    session,
                    guid: c.guid,
                    name: c.name.clone(),
                    map: c.map,
                    position: c.position,
                })
            })
            .collect()
    }

    /// Queues `event` for `session`, served or parked.
    pub(crate) fn send_to(&mut self, session: SessionId, event: SessionEvent) {
        if session == self.active {
            self.events.push(event);
        } else if let Some(s) = self.parked.get_mut(&session) {
            s.events.push(event);
        }
    }

    /// The served player's line of `chat_type` (`CMSG_MESSAGECHAT`'s); `target` names a whisper's
    /// addressee or a channel; `/p` reaches everyone, all in one party (#149). Any other type is
    /// dropped.
    pub(crate) fn chat(&mut self, chat_type: u32, target: Option<String>, text: String) {
        let all = self.listeners();
        let Some(me) = all.iter().find(|l| l.session == self.active) else {
            return;
        };
        let line = |chat_type: u8, sender_guid: u64, channel: Option<String>| {
            SessionEvent::Chat(ChatMessage {
                chat_type,
                language: LANGUAGE_UNIVERSAL,
                sender_guid,
                target_guid: 0,
                sender_name: None,
                channel,
                text: text.clone(),
                chat_tag: 0,
            })
        };
        let (kind, range) = match chat_type {
            m::CHAT_TYPE_SAY => (m::CHAT_MSG_SAY, SAY_YD),
            m::CHAT_TYPE_EMOTE => (m::CHAT_MSG_EMOTE, SAY_YD),
            m::CHAT_TYPE_YELL => (m::CHAT_MSG_YELL, YELL_YD),
            m::CHAT_TYPE_WHISPER => {
                let name = target.unwrap_or_default();
                let Some(to) = all
                    .iter()
                    .find(|l| l.name.eq_ignore_ascii_case(name.trim()))
                else {
                    self.events.push(SessionEvent::ChatPlayerNotFound { name });
                    return;
                };
                let (to_session, to_guid, from) = (to.session, to.guid, me.guid);
                self.send_to(to_session, line(m::CHAT_MSG_WHISPER, from, None));
                self.events
                    .push(line(m::CHAT_MSG_WHISPER_INFORM, to_guid, None));
                return;
            }
            m::CHAT_TYPE_PARTY => {
                let said = line(m::CHAT_MSG_PARTY, me.guid, None);
                self.party_line(said);
                return;
            }
            m::CHAT_TYPE_CHANNEL => {
                if !target.is_some_and(|t| t.eq_ignore_ascii_case(WORLD_CHANNEL)) {
                    return;
                }
                let from = me.guid;
                for l in &all {
                    let said = line(m::CHAT_MSG_CHANNEL, from, Some(WORLD_CHANNEL.into()));
                    self.send_to(l.session, said);
                }
                return;
            }
            _ => return,
        };
        let from = me.guid;
        for session in in_range(&all, me, range) {
            self.send_to(session, line(kind, from, None));
        }
    }

    /// The served player's text emote (`CMSG_TEXT_EMOTE`) at `target`, to the players nearby;
    /// each client plays the emote's animation and sound from it.
    pub(crate) fn text_emote(&mut self, text_id: u32, target: u64) {
        let all = self.listeners();
        let Some(me) = all.iter().find(|l| l.session == self.active) else {
            return;
        };
        let target_name = all
            .iter()
            .find(|l| l.guid == target)
            .map(|l| l.name.clone())
            .or_else(|| {
                let entry = benilla_protocol::guid::entry(target)?;
                crate::mobs::template(entry).map(|t| t.name.clone())
            })
            .unwrap_or_default();
        let emote = SessionEvent::TextEmote {
            guid: me.guid,
            text_emote: text_id,
            target_name,
        };
        for session in in_range(&all, me, SAY_YD) {
            self.send_to(session, emote.clone());
        }
    }

    /// `SMSG_NAME_QUERY_RESPONSE` for a character; an unknown guid answers with no name.
    pub(crate) fn name_query(&mut self, guid: u64) {
        let answer = match self.characters.get(guid) {
            Some(c) => SessionEvent::PlayerName {
                guid,
                name: c.name.clone(),
                race: u32::from(c.race),
                gender: u32::from(c.gender),
                class: u32::from(c.class()),
            },
            None => SessionEvent::PlayerName {
                guid,
                name: String::new(),
                race: 0,
                gender: 0,
                class: 0,
            },
        };
        self.events.push(answer);
    }
}

/// The server-wide channel's join notice, sent on entering the world.
pub(crate) fn joined_world_channel() -> SessionEvent {
    SessionEvent::ChannelNotify {
        notice: m::channel_notice::YOU_JOINED,
        channel: WORLD_CHANNEL.into(),
        tail: ChannelNoticeTail::YouJoined { flags: 0 },
    }
}

/// The sessions within `range` of `me` on its map, `me`'s own included.
fn in_range(all: &[Listener], me: &Listener, range: f32) -> Vec<SessionId> {
    all.iter()
        .filter(|l| l.map == me.map && crate::space::distance(l.position, me.position) <= range)
        .map(|l| l.session)
        .collect()
}
