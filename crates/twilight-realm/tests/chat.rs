//! Chat and emotes between players (#145): say carries 25 yd and yell 300, a whisper finds its
//! addressee by name anywhere, the World channel reaches everyone, and a text emote reaches the
//! players nearby with its target's name.

mod common;

use benilla_protocol::messages::{self as m, ChatMessage};
use benilla_protocol::SessionEvent;
use common::party::*;
use common::*;
use twilight_realm::chat::WORLD_CHANNEL;
use twilight_realm::CROSSROADS;

const FAR_AWAY: [f32; 3] = [CROSSROADS[0] + 3000.0, CROSSROADS[1], 0.0];

/// Varian and Anduin 10 yd apart, Jaina 100 yd off.
fn three() -> Party {
    let mut party = Party::new(realm(), &[5, 5, 5]);
    party.go(0, FAR_AWAY, 0.0);
    party.go(1, [FAR_AWAY[0] + 10.0, FAR_AWAY[1], 0.0], 0.0);
    party.go(2, [FAR_AWAY[0] + 100.0, FAR_AWAY[1], 0.0], 0.0);
    party.run(0.0);
    party.log.clear();
    party
}

fn say(party: &mut Party, i: usize, chat_type: u32, target: Option<&str>, text: &str) {
    party.cmd(
        i,
        RealmCommand::Chat {
            chat_type,
            target: target.map(Into::into),
            text: text.into(),
        },
    );
}

fn lines(party: &Party, i: usize) -> Vec<ChatMessage> {
    party
        .seen(i)
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Chat(c) => Some(c.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn say_carries_25_yards_and_yell_300() {
    let mut party = three();
    say(&mut party, 0, m::CHAT_TYPE_SAY, None, "hello");
    assert_eq!(lines(&party, 0).len(), 1, "the speaker hears its own line");
    let heard = lines(&party, 1);
    assert_eq!(heard.len(), 1);
    assert_eq!(heard[0].chat_type, m::CHAT_MSG_SAY);
    assert_eq!(heard[0].sender_guid, party.guids[0]);
    assert!(lines(&party, 2).is_empty(), "100 yd is out of earshot");

    say(&mut party, 0, m::CHAT_TYPE_YELL, None, "HELLO");
    assert!(lines(&party, 2)
        .iter()
        .any(|l| l.chat_type == m::CHAT_MSG_YELL));
}

#[test]
fn a_whisper_goes_to_its_addressee_by_name() {
    let mut party = three();
    say(&mut party, 0, m::CHAT_TYPE_WHISPER, Some("jaina"), "psst");
    let got = lines(&party, 2);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].chat_type, m::CHAT_MSG_WHISPER);
    let told = lines(&party, 0);
    assert_eq!(told[0].chat_type, m::CHAT_MSG_WHISPER_INFORM);
    assert_eq!(told[0].sender_guid, party.guids[2]);
    assert!(lines(&party, 1).is_empty());

    say(&mut party, 0, m::CHAT_TYPE_WHISPER, Some("Nobody"), "?");
    assert!(party
        .seen(0)
        .iter()
        .any(|e| matches!(e, SessionEvent::ChatPlayerNotFound { name } if name == "Nobody")));
}

#[test]
fn the_world_channel_reaches_everyone() {
    let mut party = three();
    say(
        &mut party,
        1,
        m::CHAT_TYPE_CHANNEL,
        Some(WORLD_CHANNEL),
        "lfg",
    );
    for i in 0..3 {
        let got = lines(&party, i);
        assert_eq!(got.len(), 1, "player {i}");
        assert_eq!(got[0].channel.as_deref(), Some(WORLD_CHANNEL));
    }
}

#[test]
fn a_text_emote_names_its_target_to_the_players_nearby() {
    let mut party = three();
    let anduin = party.guids[1];
    party.cmd(
        0,
        RealmCommand::TextEmote {
            text_id: 1,
            target: anduin,
        },
    );
    let emoted = |party: &Party, i: usize| {
        party.seen(i).iter().any(
            |e| matches!(e, SessionEvent::TextEmote { target_name, .. } if target_name == "Anduin"),
        )
    };
    assert!(emoted(&party, 0) && emoted(&party, 1));
    assert!(!emoted(&party, 2));
}

#[test]
fn a_name_query_names_the_player() {
    let mut party = three();
    let jaina = party.guids[2];
    party.cmd(0, RealmCommand::NameQuery { guid: jaina });
    assert!(party.seen(0).iter().any(
        |e| matches!(e, SessionEvent::PlayerName { guid, name, .. } if *guid == jaina && name == "Jaina")
    ));
}
