//! Players see each other: a player near another is sent their create with the public fields
//! only, sees them move and cast, and is told when they leave view or log out.

mod common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::party::*;
use common::*;
use twilight_realm::CROSSROADS;

/// Far out past The Barrens' camps: nothing else streams in.
const FAR_AWAY: [f32; 3] = [CROSSROADS[0] + 3000.0, CROSSROADS[1], 0.0];
const HEARTH_SPELL: u32 = 8690;
const PLAYER_BAG: u8 = 255;
/// The backpack's first slot, where a new character's Hearthstone sits.
const FIRST_PACK_SLOT: u8 = 23;

/// Two players standing together, far out.
fn together() -> Party {
    let mut party = Party::new(realm(), &[5, 5]);
    party.go(0, FAR_AWAY, 0.0);
    party.go(1, [FAR_AWAY[0] + 3.0, FAR_AWAY[1], 0.0], 0.0);
    party.run(0.0);
    party
}

/// The create of player `guid` that `events` hold.
fn player_create(events: &[SessionEvent], guid: u64) -> Option<&SessionEvent> {
    events.iter().rev().find(|e| {
        matches!(e, SessionEvent::ObjectCreate { guid: g, kind: EntityKind::Player, .. } if *g == guid)
    })
}

fn removed(events: &[SessionEvent], guid: u64) -> bool {
    events.iter().any(|e| match e {
        SessionEvent::ObjectsRemoved(gone) => gone.contains(&guid),
        SessionEvent::ObjectDestroyed(g) => *g == guid,
        _ => false,
    })
}

#[test]
fn each_is_sent_the_other_with_public_fields_only() {
    let party = together();
    for (me, other) in [(0, 1), (1, 0)] {
        let Some(SessionEvent::ObjectCreate { fields, .. }) =
            player_create(party.seen(me), party.guids[other])
        else {
            panic!("player {me} is not sent player {other}");
        };
        assert_eq!(fields.unit_level(), Some(5));
        assert!(fields.unit_health().is_some());
        // Nothing of the quest log, the bags, skills, XP or money.
        let private: Vec<u16> = fields
            .raw_fields()
            .map(|(f, _)| f)
            .filter(|&f| (198..258).contains(&f) || f >= 486)
            .collect();
        assert!(private.is_empty(), "private fields sent: {private:?}");
    }
}

#[test]
fn a_move_reaches_the_other() {
    let mut party = together();
    let a = party.guids[0];
    let to = [FAR_AWAY[0] + 6.0, FAR_AWAY[1] + 2.0, 0.0];
    party.go(0, to, 1.0);
    let moved = party.seen(1).iter().any(|e| {
        matches!(e, SessionEvent::UnitMove { guid, position, .. } if *guid == a && *position == to)
    });
    assert!(moved, "the other is shown the move");
}

#[test]
fn a_cast_is_seen_and_leaving_view_takes_the_player_away() {
    let mut party = together();
    let a = party.guids[0];
    party.cmd(
        0,
        RealmCommand::UseItem {
            bag: PLAYER_BAG,
            slot: FIRST_PACK_SLOT,
        },
    );
    party.run(11.0);
    let seen = party.seen(1);
    assert!(seen.iter().any(|e| matches!(e,
        SessionEvent::SpellStart { caster, spell_id: HEARTH_SPELL, .. } if *caster == a)));
    assert!(seen.iter().any(|e| matches!(e,
        SessionEvent::SpellGo { caster, spell_id: HEARTH_SPELL, .. } if *caster == a)));
    // The Hearthstone took them home, out of view.
    party.run(0.1);
    assert!(removed(party.seen(1), a), "the other's client lets them go");
}

#[test]
fn logging_out_takes_the_player_away() {
    let mut party = together();
    let a = party.guids[0];
    party.realm.disconnect(party.sessions[0]);
    party.run(0.1);
    assert!(removed(party.seen(1), a));
}
