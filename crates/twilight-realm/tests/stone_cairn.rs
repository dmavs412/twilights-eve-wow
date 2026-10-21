//! Stone Cairn Lake (#243): using the fifth Defaced Cairn of The Vigil Defaced (94501) starts the
//! wave of six Twilight Daubers, three seconds later.

mod common;

use std::collections::HashSet;

use benilla_protocol::START_ELWYNN;
use common::party::*;
use common::*;

const DAUBER: u32 = 96023;
const VIGIL_DEFACED: u32 = 94501;
const HIGHGUID_GAMEOBJECT: u64 = 0xF110;
const CAIRN: u64 = HIGHGUID_GAMEOBJECT << 48 | 790501;
/// Beside the first cairn, which stands at the Vigil's centre.
const BY_CAIRN: [f32; 3] = [-9268.0, -1100.0, 72.0];

/// A level 16 character on the quest with four cairns already cleaned. The kill objective takes
/// the first of the log's slots and the cairns the second.
fn party() -> Party {
    Party::edited(
        realm(),
        &[16],
        START_ELWYNN,
        Box::new(NoGround),
        |_, mut blob| {
            blob.push_str(&format!(
                "\n[[quests]]\nid = {VIGIL_DEFACED}\nslot = 0\nkills = [0, 4, 0, 0]\n"
            ));
            blob
        },
    )
}

/// The Daubers player 0's client has been sent.
fn daubers(p: &Party) -> HashSet<u64> {
    unit_creates(p.seen(0))
        .into_iter()
        .filter(|(_, entry, _)| *entry == DAUBER)
        .map(|(guid, _, _)| guid)
        .collect()
}

#[test]
fn the_fifth_cleaned_cairn_brings_six_daubers() {
    let mut p = party();
    p.go(0, BY_CAIRN, 0.0);
    p.run(15.0);
    assert!(daubers(&p).is_empty(), "nothing comes before the fifth");
    p.cmd(0, RealmCommand::UseGameObject { guid: CAIRN });
    p.run(1.0);
    assert!(daubers(&p).is_empty(), "the wave is 3 s out");
    p.run(4.0);
    assert_eq!(daubers(&p).len(), 6);
}
