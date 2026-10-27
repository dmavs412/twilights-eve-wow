//! A root, a ground patch and an absorb shield (#240), tried on the Gaze Hall's Vile Familiar
//! (`content/gaze_hall.toml`), which each test gives one Creature Spell.

mod common;

use benilla_protocol::messages::MoveMode;
use benilla_protocol::{EntityKind, SessionEvent, START_ELWYNN};
use common::party::*;
use common::*;
use twilight_realm::{Audience, CROSSROADS};

/// The Vile Familiar's entry, which the Gaze Hall places at (-9650, 20).
const FAMILIAR: u32 = 3101;
/// In melee reach of the familiar, so it never leaves its spot.
const EXPOSED: [f32; 3] = [-9650.0, 21.5, 47.2];
/// 8 yd south of the familiar.
const NEAR: [f32; 3] = [-9650.0, 12.0, 47.2];
/// In the Phase but 16 yd north.
const FAR: [f32; 3] = [-9650.0, 36.0, 47.2];
/// 3 yd from `EXPOSED`.
const BESIDE: [f32; 3] = [-9650.0, 24.5, 47.2];
/// Out in Elwynn, clear of the box.
const OUTSIDE: [f32; 3] = [-9720.0, 20.0, 47.2];
/// The first entry a ground patch's GameObject takes, clear of the dungeons' own.
const PATCH_ENTRY: u64 = 0x00f0_0000;
/// `UNIT_FIELD_AURA`, the first of the aura slots.
const UNIT_FIELD_AURA: u16 = 47;

struct Seen {
    at: f32,
    player: usize,
    event: SessionEvent,
}

struct Fight {
    party: Party,
    log: Vec<Seen>,
}

impl Fight {
    fn of(&self, i: usize) -> impl Iterator<Item = &Seen> {
        self.log.iter().filter(move |s| s.player == i)
    }

    /// When player `i` was rooted (`true`) or let go (`false`).
    fn roots(&self, i: usize) -> Vec<(f32, bool)> {
        let guid = self.party.guids[i];
        self.of(i)
            .filter_map(|s| match &s.event {
                SessionEvent::MoveMode {
                    guid: g,
                    mode: MoveMode::Root,
                    apply,
                    ..
                } if *g == guid => Some((s.at, *apply)),
                _ => None,
            })
            .collect()
    }

    /// The damage, with the time, the spell `id` did to player `i`.
    fn hits(&self, i: usize, id: u32) -> Vec<(f32, u32)> {
        let guid = self.party.guids[i];
        self.of(i)
            .filter_map(|s| match &s.event {
                SessionEvent::SpellDamageLog(l) if l.target == guid && l.spell_id == id => {
                    Some((s.at, l.damage))
                }
                _ => None,
            })
            .collect()
    }

    /// Whether player `i` was told that character `of` stands at `to`.
    fn saw_move(&self, i: usize, of: usize, to: [f32; 3]) -> bool {
        let guid = self.party.guids[of];
        self.party.seen(i).iter().any(|e| {
            matches!(e, SessionEvent::UnitMove { guid: g, position, .. }
                if *g == guid && *position == to)
        })
    }

    /// When player `i` was shown a GameObject, and its guid and position.
    fn scenery(&self, i: usize) -> Vec<(f32, u64, [f32; 3])> {
        self.of(i)
            .filter_map(|s| match &s.event {
                SessionEvent::ObjectCreate {
                    guid,
                    kind: EntityKind::GameObject,
                    position,
                    ..
                } if *guid & 0xffff_ffff >= PATCH_ENTRY => Some((s.at, *guid, *position)),
                _ => None,
            })
            .collect()
    }

    /// When player `i` was told that `guid` is gone.
    fn destroyed(&self, i: usize, guid: u64) -> Option<f32> {
        self.of(i).find_map(|s| match &s.event {
            SessionEvent::ObjectDestroyed(g) if *g == guid => Some(s.at),
            _ => None,
        })
    }
}

/// One character per place, the first pulling the familiar, which `spell` it casts. `act` is
/// called before every 0.1 s step with the party and the seconds into the fight.
fn fight(
    spell: &str,
    places: &[[f32; 3]],
    secs: f32,
    mut act: impl FnMut(&mut Party, f32),
) -> Fight {
    let realm = realm().with_creature_spells(FAMILIAR, &[spell]);
    let mut party = Party::starting(
        realm,
        &vec![20; places.len()],
        START_ELWYNN,
        Box::new(NoGround),
    );
    let mut log = Vec::new();
    let mut at = 0.0;
    party.go(0, places[0], 0.0);
    let step = |party: &mut Party, log: &mut Vec<Seen>, at: &mut f32| {
        for (player, events) in party.run(0.1).into_iter().enumerate() {
            log.extend(events.into_iter().map(|event| Seen {
                at: *at,
                player,
                event,
            }));
        }
        *at += 0.1;
    };
    for _ in 0..2 {
        step(&mut party, &mut log, &mut at);
    }
    for (i, &place) in places.iter().enumerate().skip(1) {
        party.go(i, place, 0.0);
    }
    let start = at;
    while at - start < secs {
        act(&mut party, at - start);
        step(&mut party, &mut log, &mut at);
    }
    // Times in the log are seconds into the fight.
    for s in &mut log {
        s.at -= start;
    }
    Fight { party, log }
}

#[test]
fn a_root_holds_every_character_in_its_radius_and_no_one_beyond() {
    let rooted_step = [-9650.0, 22.5, 47.2];
    let free_step = [-9650.0, 38.0, 47.2];
    let near_step = [-9650.0, 10.0, 47.2];
    let after_step = [-9650.0, 22.0, 47.2];
    let (mut rooted_at, mut tried, mut after) = (None, false, false);
    let f = fight(
        "test_root",
        &[EXPOSED, NEAR, FAR, OUTSIDE],
        14.0,
        |p, at| {
            let hit = p.seen(0).iter().any(|e| {
                matches!(
                    e,
                    SessionEvent::MoveMode {
                        apply: true,
                        mode: MoveMode::Root,
                        ..
                    }
                )
            });
            if hit && rooted_at.is_none() {
                rooted_at = Some(at);
            }
            let Some(r) = rooted_at else {
                return;
            };
            if !tried && at >= r + 1.0 {
                tried = true;
                p.go(0, rooted_step, 0.0);
                p.go(1, near_step, 0.0);
                p.go(2, free_step, 0.0);
            }
            if !after && at >= r + 6.0 {
                after = true;
                p.go(0, after_step, 0.0);
            }
        },
    );
    for (i, what) in [(0, "in reach"), (1, "8 yd off")] {
        let roots = f.roots(i);
        let [(rooted, true), (freed, false)] = roots[..] else {
            panic!("{what}: rooted once and let go once: {roots:?}");
        };
        assert!(
            (freed - rooted - 4.0).abs() < 0.3,
            "{what}: 4 s of root, {rooted} to {freed}"
        );
    }
    assert!(f.roots(2).is_empty(), "16 yd out: {:?}", f.roots(2));
    assert!(f.roots(3).is_empty(), "outside the Phase");
    assert!(
        !f.saw_move(1, 0, rooted_step),
        "a rooted character's step is not taken"
    );
    assert!(
        !f.saw_move(0, 1, near_step),
        "a rooted character's step is not taken"
    );
    assert!(f.saw_move(0, 2, free_step), "one outside the radius moves");
    assert!(f.saw_move(1, 0, after_step), "and the root wears off");
}

#[test]
fn a_ground_patch_hurts_those_standing_in_it_until_its_time_is_up() {
    let patch = creature_spell("test_ground_patch");
    let mut shown = None;
    let mut stepped = false;
    let f = fight(
        "test_ground_patch",
        &[EXPOSED, NEAR, BESIDE, OUTSIDE],
        18.0,
        |p, at| {
            let hit = p.seen(2).iter().any(|e| {
                matches!(
                    e,
                    SessionEvent::ObjectCreate {
                        kind: EntityKind::GameObject,
                        ..
                    }
                )
            });
            if hit && shown.is_none() {
                shown = Some(at);
            }
            if let Some(s) = shown {
                if !stepped && at >= s + 4.5 {
                    stepped = true;
                    p.go(2, FAR, 0.0);
                }
            }
        },
    );
    for i in 0..3 {
        let made = f.scenery(i);
        assert_eq!(made.len(), 1, "player {i} sees one patch: {made:?}");
        let (_, _, at) = made[0];
        assert!(
            flat_distance(at, EXPOSED) < 0.1,
            "at the target's feet: {at:?}"
        );
    }
    assert!(f.scenery(3).is_empty(), "not outside the Phase");
    let (shown, guid, _) = f.scenery(0)[0];
    let gone = f.destroyed(0, guid).expect("the patch goes");
    assert!(
        (gone - shown - 10.0).abs() < 0.3,
        "10 s on the ground: {shown} to {gone}"
    );

    let standing = f.hits(0, patch);
    assert!(
        (9..=10).contains(&standing.len()),
        "a tick a second for 10 s: {standing:?}"
    );
    assert!(standing.iter().all(|&(_, d)| d == 5), "{standing:?}");
    assert!(standing.iter().all(|&(at, _)| at <= gone + 0.2));
    assert!(f.hits(1, patch).is_empty(), "9.5 yd from it");
    assert!(f.hits(3, patch).is_empty(), "outside the Phase");

    let stepped_out = f.hits(2, patch);
    assert!(
        (3..=5).contains(&stepped_out.len()),
        "ticks until they step out at 4.5 s: {stepped_out:?}"
    );
    assert!(
        stepped_out.iter().all(|&(at, _)| at <= shown + 5.0),
        "none after: {stepped_out:?}"
    );
}

#[test]
fn an_absorb_shield_goes_up_on_the_caster_for_its_duration() {
    let shield = creature_spell("test_absorb");
    let f = fight("test_absorb", &[EXPOSED], 16.0, |_, _| {});
    let caster = f
        .log
        .iter()
        .find_map(|s| match &s.event {
            SessionEvent::SpellGo {
                caster, spell_id, ..
            } if *spell_id == shield => Some(*caster),
            _ => None,
        })
        .expect("the familiar casts it");
    assert_ne!(caster, f.party.guids[0]);
    // The shield is a buff on the familiar, in an aura slot, until it times out.
    let slots: Vec<(f32, bool)> = f
        .of(0)
        .filter_map(|s| match &s.event {
            SessionEvent::ObjectValues { guid, fields } if *guid == caster => fields
                .raw_fields()
                .find(|&(field, _)| (UNIT_FIELD_AURA..UNIT_FIELD_AURA + 56).contains(&field))
                .map(|(_, spell)| (s.at, spell == shield)),
            _ => None,
        })
        .collect();
    let [(up, true), (down, false)] = slots[..] else {
        panic!("up once and down once: {slots:?}");
    };
    assert!(
        (down - up - 10.0).abs() < 0.3,
        "10 s of shield: {up} to {down}"
    );
}

/// `UNIT_FIELD_RESISTANCES`, whose first is armour.
const UNIT_FIELD_RESISTANCES: u16 = 155;
/// Far from Elwynn's creatures, so nothing fights the character there.
const NOWHERE: [f32; 3] = [-5000.0, -5000.0, 47.2];

#[test]
fn an_armor_debuff_stacks_to_its_most_and_all_of_it_goes_when_it_runs_out() {
    let sunder = creature_spell("sunder_kobold");
    // The fourth cast lands, then the character leaves for somewhere no creature stands, where
    // the debuff runs out.
    let mut left_at = None;
    let f = fight("sunder_kobold", &[EXPOSED], 66.0, |p, at| {
        let casts = p
            .seen(0)
            .iter()
            .filter(|e| matches!(e, SessionEvent::SpellGo { spell_id, .. } if *spell_id == sunder))
            .count();
        if casts == 4 && left_at.is_none() {
            left_at = Some(at);
            p.go(0, NOWHERE, 0.0);
        }
    });
    let left_at = left_at.expect("four casts before the fight ends");
    let guid = f.party.guids[0];
    // The character's armour as each create or update said it, with the time.
    let armor: Vec<(f32, u32)> = f
        .log
        .iter()
        .filter(|s| s.player == 0)
        .filter_map(|s| match &s.event {
            SessionEvent::ObjectCreate {
                guid: g, fields, ..
            }
            | SessionEvent::ObjectValues { guid: g, fields }
                if *g == guid =>
            {
                fields
                    .raw_fields()
                    .find_map(|(field, v)| (field == UNIT_FIELD_RESISTANCES).then_some((s.at, v)))
            }
            _ => None,
        })
        .collect();
    let gos: Vec<f32> = f
        .of(0)
        .filter(
            |s| matches!(&s.event, SessionEvent::SpellGo { spell_id, .. } if *spell_id == sunder),
        )
        .map(|s| s.at)
        .collect();
    assert_eq!(gos.len(), 4, "four casts and no more: {gos:?}");
    let at = |t: f32| {
        armor
            .iter()
            .rev()
            .find(|(at, _)| *at <= t)
            .map(|&(_, v)| v)
            .unwrap_or_else(|| panic!("no armour sent by {t}: {armor:?}"))
    };
    // What the first stack took away, the expiry gives back.
    let base = at(gos[0] + 0.05) + 10;
    for (n, &go) in gos.iter().enumerate() {
        let lost = 10 * (n as u32 + 1).min(3);
        assert_eq!(
            at(go + 0.05),
            base - lost,
            "after cast {}: {armor:?}",
            n + 1
        );
    }
    // 30 s after the last cast, every stack goes at once.
    let last = gos[3];
    assert!(left_at < last + 1.0);
    assert!(!died(f.party.seen(0), guid), "alive to the end");
    assert_eq!(at(last + 29.8), base - 30, "{armor:?}");
    assert_eq!(at(last + 30.3), base, "{armor:?}");
}

/// `UNIT_FIELD_HEALTH`.
const UNIT_FIELD_HEALTH: u16 = 22;

/// The familiar's guid, from the cast of `spell` it made.
fn caster_of(f: &Fight, spell: u32) -> u64 {
    f.log
        .iter()
        .find_map(|s| match &s.event {
            SessionEvent::SpellGo {
                caster, spell_id, ..
            } if *spell_id == spell => Some(*caster),
            _ => None,
        })
        .expect("the familiar casts it")
}

#[test]
fn a_stance_goes_up_once_and_drops_when_the_creature_leaves_combat() {
    let stance = creature_spell("defensive_stance_padfoot");
    // Out in the open: leaving the Gaze Hall's Phase takes every aura off its creatures anyway,
    // and coming back meets a fresh familiar, so neither shows the stance drop. A Kobold Vermin
    // alone past The Barrens' camps takes the stance when a swing pulls it; the character then
    // steps 70 yd off, past its leash, and it gives up the fight.
    let realm = realm().with_creature_spells(KOBOLD_VERMIN, &["defensive_stance_padfoot"]);
    let mut party = Party::new(realm, &[20]);
    party.go(0, away(0.0), 0.0);
    party.run(0.5);
    let map = party.realm.player_positions()[0].0;
    let kobold = party
        .realm
        .spawn_for(
            party.host.as_ref(),
            KOBOLD_VERMIN,
            2,
            away(3.0),
            map,
            Audience::of(&[party.guids[0]]),
        )
        .expect("a kobold");
    party.run(0.5);
    party.cmd(0, RealmCommand::AttackSwing { guid: kobold });
    party.run_until(3.0, |got| {
        got[0]
            .iter()
            .any(|e| matches!(e, SessionEvent::AttackStart { attacker, .. } if *attacker == kobold))
    });
    party.cmd(0, RealmCommand::AttackStop);
    party.run(10.0);
    let left = party.seen(0).len();
    party.go(0, away(70.0), 0.0);
    party.run(20.0);
    let seen = party.seen(0);
    assert!(!died(seen, kobold), "the kobold lives");
    let gos = seen
        .iter()
        .filter(|e| matches!(e, SessionEvent::SpellGo { spell_id, .. } if *spell_id == stance))
        .count();
    assert_eq!(gos, 1, "cast once, and not again while it is up");
    // Where in what the character saw the kobold's first aura slot showed the stance or not.
    let slots: Vec<(usize, bool)> = seen
        .iter()
        .enumerate()
        .filter_map(|(n, e)| match e {
            SessionEvent::ObjectValues { guid, fields } if *guid == kobold => fields
                .raw_fields()
                .find(|&(field, _)| field == UNIT_FIELD_AURA)
                .map(|(_, v)| (n, v == stance)),
            _ => None,
        })
        .collect();
    let [(up, true), (down, false)] = slots[..] else {
        panic!("up once and down once: {slots:?}");
    };
    assert!(
        up < left && down > left,
        "held through the fight, gone once it ends: {up} and {down}, the character leaving at {left}"
    );
}

const KOBOLD_VERMIN: u32 = 6;

/// `dx` yd east of a spot far out past The Barrens' camps, where nothing else streams in.
fn away(dx: f32) -> [f32; 3] {
    [CROSSROADS[0] + 3000.0 + dx, CROSSROADS[1], 0.0]
}

#[test]
fn a_stance_takes_a_tenth_off_every_blow_the_creature_takes() {
    let stance = creature_spell("defensive_stance_padfoot");
    let mut swinging = false;
    let f = fight("defensive_stance_padfoot", &[EXPOSED], 12.0, |p, _| {
        if !swinging {
            swinging = true;
            let (guid, at) = p.unit(0, FAMILIAR, EXPOSED);
            p.go_facing(0, EXPOSED, at);
            p.cmd(0, RealmCommand::AttackSwing { guid });
        }
    });
    let familiar = caster_of(&f, stance);
    let me = f.party.guids[0];
    // Each landed swing of the character's after the stance went up, with the health the
    // familiar lost to it.
    let (mut up, mut health, mut swing) = (false, None, None);
    let mut blows = Vec::new();
    for s in f.of(0) {
        match &s.event {
            SessionEvent::SpellGo { spell_id, .. } if *spell_id == stance => up = true,
            SessionEvent::AttackerState(a) if a.attacker == me && a.victim == familiar => {
                swing = (a.damage > 0).then_some(a.damage);
            }
            SessionEvent::ObjectCreate { guid, fields, .. }
            | SessionEvent::ObjectValues { guid, fields }
                if *guid == familiar =>
            {
                let Some((_, now)) = fields
                    .raw_fields()
                    .find(|&(field, _)| field == UNIT_FIELD_HEALTH)
                else {
                    continue;
                };
                if let (true, Some(was), Some(d)) = (up, health, swing.take()) {
                    blows.push((d, was - now));
                }
                health = Some(now);
            }
            _ => {}
        }
    }
    assert!(blows.len() >= 3, "several swings land: {blows:?}");
    for (dealt, lost) in blows {
        assert_eq!(lost, dealt * 9 / 10, "a swing of {dealt}");
    }
}
