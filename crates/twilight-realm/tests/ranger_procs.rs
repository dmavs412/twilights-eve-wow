//! The Ranger's procs (#127), each talent at its top rank: Rhythmic Draw, Twin Arrows, Adrenaline
//! Rush, Flurry, Rapid Assault, Jagged Barbs, Shrapnel Payload and Shattered Defenses. Played
//! through the realm's seam by a level 10 Ranger with a Cracked Shortbow at the scorpid camp
//! south of the Crossroads. Point Blank 2/2 rides along, so the Ranger keeps shooting when the
//! pulled scorpids reach melee.

mod common;
mod gear_common;

use benilla_protocol::{EntityKind, SessionEvent};
use common::fight::{cast, flat, refusal, Mob, View, CAMP};
use common::*;
use gear_common::save_shooting;
use twilight_realm::{Realm, RealmCommand};

const RANGED: u8 = 17;
const CRACKED_SHORTBOW: u32 = 2773;

const SNAPSHOT: u32 = 3044;
const PUNCTURING_SHAFT: u32 = 19434;
const BARBED_ARROW: u32 = 1978;
const VAULT: u32 = 781;
const RAPID_FIRE: u32 = 3045;
const RAPID_ASSAULT: u32 = 19263;

/// `Talent.dbc` ids.
const POINT_BLANK: u32 = 1624;
const RHYTHMIC_DRAW: u32 = 1304;
const TWIN_ARROWS: u32 = 1301;
const ADRENALINE_RUSH: u32 = 1310;
const FLURRY: u32 = 1322;
const RAPID_ASSAULT_TALENT: u32 = 1308;
const JAGGED_BARBS: u32 = 1348;
const SHRAPNEL_PAYLOAD: u32 = 1347;
const SHATTERED_DEFENSES: u32 = 1352;

/// Aura and proc spell ids at the top ranks.
const RHYTHMIC_DRAW_5: u32 = 19390;
const SUNDERED_4: u32 = 19467;
const SUNDERED_5: u32 = 19468;
const SHRAPNEL_PAYLOAD_3: u32 = 24691;

const SPELL_HIT_TYPE_CRIT: u32 = 0x2;

/// Where the Ranger stands: 15 yd east of the camp's middle.
const STAND: [f32; 3] = [CAMP[0] + 15.0, CAMP[1], CAMP[2]];

/// A level 10 Ranger with a bow, Point Blank and `talents`, at the scorpid camp on `seed`: the
/// realm, its guid, the camp's mobs, the client's view.
fn at_camp(talents: &[(u32, u8)], seed: u64) -> (Realm, u64, Vec<Mob>, View) {
    let (guid, mut blob) = save_shooting(10, &[(RANGED, CRACKED_SHORTBOW, 1)]);
    blob = blob.replace("calling = \"basic_warrior\"", "calling = \"ranger\"");
    let mut rows = vec![format!("[{POINT_BLANK}, 2]")];
    rows.extend(talents.iter().map(|(t, r)| format!("[{t}, {r}]")));
    let line = format!("money = 0\ntalents = [{}]\n", rows.join(", "));
    blob = blob.replace("money = 0\n", &line);
    let mut realm = Realm::with_seed(seed);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    view.drain(&mut realm);
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: STAND,
        orientation: std::f32::consts::PI,
    });
    let seen = view.run(&mut realm, 1.0);
    let camp = seen
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: EntityKind::Unit,
                position,
                ..
            } if flat(*position, CAMP) < 20.0 => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(camp.len() >= 3, "the camp in view");
    (realm, guid, camp, view)
}

/// Casts `spell_id` at `mob`, facing it from where the Ranger stands: the events of the cast,
/// or `None` when it was refused.
fn shoot(
    realm: &mut Realm,
    view: &mut View,
    spell_id: u32,
    mob: &Mob,
) -> Option<Vec<SessionEvent>> {
    realm.handle(RealmCommand::Move {
        motion: Default::default(),
        position: STAND,
        orientation: (mob.position[1] - STAND[1]).atan2(mob.position[0] - STAND[0]),
    });
    realm.handle(cast(spell_id, Some(mob.guid)));
    let events = view.drain(realm);
    refusal(&events, spell_id).is_none().then_some(events)
}

/// Casts `spell_id` at the first of `mobs` that takes it.
fn shoot_any(
    realm: &mut Realm,
    view: &mut View,
    spell_id: u32,
    mobs: &[Mob],
) -> Option<(Mob, Vec<SessionEvent>)> {
    mobs.iter()
        .find_map(|m| shoot(realm, view, spell_id, m).map(|e| (m.clone(), e)))
}

/// Each landed hit of `spell_id` in `events`: `(target, damage, crit)`.
fn landed(events: &[SessionEvent], spell_id: u32) -> Vec<(u64, u32, bool)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpellDamageLog(l) if l.spell_id == spell_id && l.damage > 0 => {
                Some((l.target, l.damage, l.hit_info & SPELL_HIT_TYPE_CRIT != 0))
            }
            _ => None,
        })
        .collect()
}

/// How long the cast bar `spell_id` started with in `events` ran, `None` with no bar.
fn bar(events: &[SessionEvent], spell_id: u32) -> Option<u32> {
    events.iter().find_map(|e| match e {
        SessionEvent::SpellStart {
            spell_id: s,
            cast_time_ms,
            ..
        } if *s == spell_id && *cast_time_ms > 0 => Some(*cast_time_ms),
        _ => None,
    })
}

/// Rhythmic Draw 5/5: two Snapshots take a second off the Shaft (3 s to 2 s), and six take all
/// three: an instant Shaft, the floor. (Seven would leave the camp dead.)
#[test]
fn rhythmic_draw_shortens_the_shaft_down_to_instant() {
    for (snapshots, want) in [(2, Some(2000)), (6, None)] {
        let (mut realm, me, camp, mut view) = at_camp(&[(RHYTHMIC_DRAW, 5)], 0xD0_0001);
        let mut cast = 0;
        while cast < snapshots {
            if shoot_any(&mut realm, &mut view, SNAPSHOT, &camp).is_some() {
                cast += 1;
            }
            view.run(&mut realm, 1.6);
        }
        let stacks = view
            .aura(me, RHYTHMIC_DRAW_5)
            .expect("Rhythmic Draw on")
            .stacks;
        assert_eq!(u32::from(stacks), snapshots);
        let (_, events) =
            shoot_any(&mut realm, &mut view, PUNCTURING_SHAFT, &camp).expect("a Shaft");
        assert_eq!(
            bar(&events, PUNCTURING_SHAFT),
            want,
            "{snapshots} Snapshots"
        );
        if want.is_none() {
            assert!(events.iter().any(
                |e| matches!(e, SessionEvent::SpellGo { spell_id, .. } if *spell_id == PUNCTURING_SHAFT)
            ));
        }
        view.run(&mut realm, 2.1);
        assert!(
            view.aura(me, RHYTHMIC_DRAW_5).is_none(),
            "the Shaft used them"
        );
    }
}

/// Twin Arrows 3/3: about 30% of Snapshots go off twice at once.
#[test]
fn twin_arrows_repeats_a_snapshot_30_pct_of_the_time() {
    let (mut twice, mut casts) = (0, 0);
    for seed in 0..60 {
        let (mut realm, _, camp, mut view) = at_camp(&[(TWIN_ARROWS, 3)], 0xD1_0000 + seed);
        let Some((_, events)) = shoot_any(&mut realm, &mut view, SNAPSHOT, &camp) else {
            continue;
        };
        casts += 1;
        let gone = events
            .iter()
            .filter(
                |e| matches!(e, SessionEvent::SpellGo { spell_id, .. } if *spell_id == SNAPSHOT),
            )
            .count();
        assert!(gone <= 2, "a repeat never repeats");
        twice += usize::from(gone == 2);
    }
    let rate = twice as f32 / casts as f32;
    assert!((0.15..0.45).contains(&rate), "{twice} of {casts}");
}

/// Adrenaline Rush 3/3: a Snapshot crit takes 3 s off Vault's cooldown.
#[test]
fn adrenaline_rush_cuts_vaults_cooldown_on_a_snapshot_crit() {
    let mut seen = 0;
    for seed in 0..80 {
        let (mut realm, _, camp, mut view) = at_camp(&[(ADRENALINE_RUSH, 3)], 0xD2_0000 + seed);
        realm.handle(cast(VAULT, None));
        view.run(&mut realm, 1.6);
        let Some((_, events)) = shoot_any(&mut realm, &mut view, SNAPSHOT, &camp) else {
            continue;
        };
        let crit = landed(&events, SNAPSHOT).iter().any(|(_, _, crit)| *crit);
        let cut = events.iter().find_map(|e| match e {
            SessionEvent::SpellCooldowns { cooldowns, .. } => cooldowns
                .iter()
                .find(|(id, _)| *id == VAULT)
                .map(|(_, ms)| *ms),
            _ => None,
        });
        if crit {
            let ms = cut.expect("a crit cuts Vault's cooldown");
            // 15 s, less the 1.6 s gone and the 3 s cut.
            assert!((10_300..=10_500).contains(&ms), "{ms} ms left");
            seen += 1;
        } else {
            assert_eq!(cut, None, "only a crit cuts it");
        }
    }
    assert!(seen >= 1, "a Snapshot crit in 80 tries");
}

/// Flurry 2/2: after Rapid Fire, two Snapshots cost no mana and crit; the third is plain.
#[test]
fn flurry_gives_two_free_certain_crit_snapshots_after_rapid_fire() {
    let (mut realm, me, camp, mut view) = at_camp(&[(FLURRY, 2)], 0xD3_0001);
    realm.handle(cast(RAPID_FIRE, None));
    view.run(&mut realm, 1.6);
    for n in 0..3 {
        let before = view.fields(me).unit_power(0).unwrap();
        let (_, events) = shoot_any(&mut realm, &mut view, SNAPSHOT, &camp).expect("a Snapshot");
        let after = view.fields(me).unit_power(0).unwrap();
        let hits = landed(&events, SNAPSHOT);
        if n < 2 {
            assert_eq!(after, before, "Snapshot {n} is free");
            assert!(
                hits.iter().all(|(_, _, crit)| *crit),
                "Snapshot {n} crits: {hits:?}"
            );
        } else {
            assert!(after < before, "the third costs mana");
        }
        view.run(&mut realm, 1.6);
    }
}

/// Rapid Assault: for 8 s each landed Snapshot strikes twice, the second for half a bow roll.
#[test]
fn rapid_assault_adds_a_half_strike_to_each_snapshot() {
    let mut doubled = 0;
    for seed in 0..10 {
        let (mut realm, _, camp, mut view) =
            at_camp(&[(RAPID_ASSAULT_TALENT, 1)], 0xD4_0000 + seed);
        realm.handle(cast(RAPID_ASSAULT, None));
        view.run(&mut realm, 1.6);
        let Some((_, events)) = shoot_any(&mut realm, &mut view, SNAPSHOT, &camp) else {
            continue;
        };
        let hits = landed(&events, SNAPSHOT);
        if !hits.is_empty() {
            assert_eq!(hits.len(), 2, "{hits:?}");
            assert!(
                hits[1].1 < hits[0].1,
                "the extra arrow is the smaller: {hits:?}"
            );
            doubled += 1;
        }
    }
    assert!(doubled >= 3);
}

/// Jagged Barbs 5/5: Barbed Arrow's ticks put Sundered on its target, 20% a tick, never past 3.
#[test]
fn jagged_barbs_sunders_the_bleeding_target() {
    let mut sundered = 0;
    for seed in 0..10 {
        let (mut realm, _, camp, mut view) = at_camp(&[(JAGGED_BARBS, 5)], 0xD5_0000 + seed);
        let (mob, _) = shoot_any(&mut realm, &mut view, BARBED_ARROW, &camp).expect("a bleed");
        let mut most = 0;
        for _ in 0..90 {
            view.run(&mut realm, 0.1);
            if let Some(slot) = view
                .objects
                .get(&mob.guid)
                .and_then(|_| view.aura(mob.guid, SUNDERED_5))
            {
                most = most.max(slot.stacks);
            }
        }
        assert!(most <= 3);
        sundered += usize::from(most > 0);
    }
    assert!((2..=9).contains(&sundered), "{sundered} of 10 Sundered");
}

/// Shrapnel Payload 3/3: a Shaft on a bleeding target sends 30% of its damage into the bleeding
/// scorpids packed around it.
#[test]
fn shrapnel_payload_splashes_a_share_of_the_shaft() {
    let mut splashed = 0;
    for seed in 0..10 {
        let (mut realm, _, camp, mut view) = at_camp(&[(SHRAPNEL_PAYLOAD, 3)], 0xD6_0000 + seed);
        // Three bleed and come running.
        for mob in &camp[..3] {
            shoot(&mut realm, &mut view, BARBED_ARROW, mob);
            view.run(&mut realm, 1.6);
        }
        let Some((target, mut events)) =
            shoot_any(&mut realm, &mut view, PUNCTURING_SHAFT, &camp[..3])
        else {
            continue;
        };
        events.extend(view.run(&mut realm, 3.1));
        let Some(&(_, shaft, _)) = landed(&events, PUNCTURING_SHAFT).first() else {
            continue;
        };
        let splash = landed(&events, SHRAPNEL_PAYLOAD_3);
        assert!(splash
            .iter()
            .all(|(guid, dealt, _)| *guid != target.guid && *dealt <= shaft * 30 / 100 + 1));
        splashed += usize::from(!splash.is_empty());
    }
    assert!(splashed >= 2, "{splashed} Shafts splashed");
}

/// Shattered Defenses 3/3: a Snapshot on a Sundered target deals 15% more than the same roll
/// without the talent. Jagged Barbs 4/5 does the sundering, keeping the talents within level 10's
/// nine points.
#[test]
fn shattered_defenses_hits_sundered_targets_harder() {
    let mut compared = 0;
    for seed in 0..30 {
        let mut dealt = Vec::new();
        for talents in [
            &[(JAGGED_BARBS, 4)][..],
            &[(JAGGED_BARBS, 4), (SHATTERED_DEFENSES, 3)][..],
        ] {
            let (mut realm, _, camp, mut view) = at_camp(talents, 0xD7_0000 + seed);
            let (mob, _) = shoot_any(&mut realm, &mut view, BARBED_ARROW, &camp).expect("a bleed");
            view.run(&mut realm, 8.1);
            if view.aura(mob.guid, SUNDERED_4).is_none() {
                dealt.push(None);
                continue;
            }
            let hit = shoot(&mut realm, &mut view, SNAPSHOT, &mob)
                .and_then(|events| landed(&events, SNAPSHOT).first().copied());
            dealt.push(hit.filter(|(_, _, crit)| !crit).map(|(_, d, _)| d));
        }
        if let [Some(plain), Some(more)] = dealt[..] {
            let want = plain as f32 * 1.15;
            assert!(
                (more as f32 - want).abs() <= 1.5,
                "seed {seed}: {plain} then {more}"
            );
            compared += 1;
        }
    }
    assert!(compared >= 2, "{compared} compared");
}
