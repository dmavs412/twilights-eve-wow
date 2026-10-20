//! Health regeneration and `mod_stat`: the Spirit-based tick out of combat, and Field Medic, the
//! talent that raises it and Stamina. Played through the realm's seam by human warriors.

mod common;

use benilla_protocol::SessionEvent;
use common::fight::*;
use common::*;
use twilight_realm::rules::regen;
use twilight_realm::{Realm, RealmCommand};

/// The Recruit tree's Field Medic: its borrowed `Talent.dbc` id.
static FIELD_MEDIC: std::sync::LazyLock<u32> =
    std::sync::LazyLock::new(|| talent("basic_warrior", "field_medic"));
const STAMINA: u8 = 2;

#[test]
fn the_tick_is_spirit_based_and_a_regen_percent_raises_it() {
    assert_eq!(regen::tick_heal(0, 0), 1, "never less than a point");
    assert!(regen::tick_heal(100, 0) > regen::tick_heal(50, 0));
    let plain = regen::tick_heal(200, 0);
    assert_eq!(regen::tick_heal(200, 100), 2 * plain, "+100% doubles it");
}

#[test]
fn health_regenerates_out_of_combat_and_not_in_it() {
    let (mut realm, me, mob, mut view) = warrior_at(1, &[]);
    hurt(&mut realm, &mut view, &mob);
    // In combat, nothing healed: health only fell.
    let fight = fight_until_dead(&mut realm, &mut view, me, &mob);
    assert!(
        fight.windows(2).all(|w| w[1] <= w[0]),
        "no health came back in the fight: {fight:?}"
    );
    view.run(&mut realm, 0.5);
    let max = view.fields(me).unit_max_health().unwrap();
    let after = view.health(me);
    assert!(after < max, "the fight left the warrior hurt");
    view.run(&mut realm, 10.0);
    assert!(view.health(me) > after, "10 s out of combat healed");
}

#[test]
fn field_medic_raises_regen_and_stamina_by_its_percents() {
    let (mut plain, p_me, p_mob, mut p_view) = warrior_at(6, &[]);
    let (mut medic, m_me, m_mob, mut m_view) = warrior_at(6, &[(*FIELD_MEDIC, 5)]);
    let sta = p_view.fields(p_me).unit_stat(STAMINA).unwrap();
    assert_eq!(
        m_view.fields(m_me).unit_stat(STAMINA).unwrap(),
        sta + sta * 15 / 100,
        "Stamina +15%"
    );
    let mut gained = Vec::new();
    for (realm, view, me, mob) in [
        (&mut plain, &mut p_view, p_me, &p_mob),
        (&mut medic, &mut m_view, m_me, &m_mob),
    ] {
        hurt(realm, view, mob);
        fight_until_dead(realm, view, me, mob);
        view.run(realm, 0.5);
        let before = view.health(me);
        view.run(realm, 2.1);
        gained.push(view.health(me) - before);
    }
    assert!(
        gained[1] > gained[0],
        "Field Medic heals more in one tick: {gained:?}"
    );
}

#[test]
fn learning_a_stamina_percent_raises_current_health_with_max_health() {
    let (mut realm, me, _, mut view) = warrior_at(6, &[]);
    let (hp, max) = (view.health(me), view.fields(me).unit_max_health().unwrap());
    for rank in 0..5 {
        realm.handle(RealmCommand::LearnTalent {
            talent_id: *FIELD_MEDIC,
            rank,
        });
    }
    view.drain(&mut realm);
    let (hp2, max2) = (view.health(me), view.fields(me).unit_max_health().unwrap());
    assert!(max2 > max, "max health rose");
    assert_eq!(hp2 - hp, max2 - max, "current health rose by the same");
}

/// Stands by the mob and lets it hit the warrior for a few seconds.
fn hurt(realm: &mut Realm, view: &mut View, mob: &Mob) {
    stand_at(realm, mob, 2.0);
    view.run(realm, 25.0);
}

/// Fights the mob to its death; the warrior's health at each step.
fn fight_until_dead(realm: &mut Realm, view: &mut View, me: u64, mob: &Mob) -> Vec<u32> {
    realm.handle(RealmCommand::AttackSwing { guid: mob.guid });
    let mut seen = vec![view.health(me)];
    for _ in 0..1200 {
        view.run(realm, 0.1);
        seen.push(view.health(me));
        assert!(view.health(me) > 0, "died");
        if view.health(mob.guid) == 0 {
            return seen;
        }
    }
    panic!("the mob outlived two minutes");
}

/// A human warrior of `level` in the world at the Crossroads with `talents` (`(id, rank)`) in
/// its save, the mob near it, and what the client has seen.
fn warrior_at(level: u8, talents: &[(u32, u8)]) -> (Realm, u64, Mob, View) {
    let mut maker = realm();
    assert_eq!(create(&mut maker, "Varian", HUMAN, 0), CHAR_CREATE_SUCCESS);
    let guid = roster(&mut maker)[0].guid;
    let mut blob = maker
        .export_character(guid)
        .unwrap()
        .replace("level = 1\n", &format!("level = {level}\n"));
    if !talents.is_empty() {
        let rows: Vec<String> = talents.iter().map(|(t, r)| format!("[{t}, {r}]")).collect();
        blob = blob.replace(
            "money = 0\n",
            &format!("money = 0\ntalents = [{}]\n", rows.join(", ")),
        );
    }
    let mut realm = Realm::with_seed(0x5EED_0056);
    realm.import_character(&blob).unwrap();
    realm.handle(RealmCommand::EnterWorld { guid });
    realm.tick(0.0, &FlatGround);
    let mut view = View::default();
    let mob = view
        .drain(&mut realm)
        .iter()
        .find_map(|e| match e {
            SessionEvent::ObjectCreate {
                guid,
                kind: benilla_protocol::EntityKind::Unit,
                position,
                ..
            } => Some(Mob {
                guid: *guid,
                position: *position,
            }),
            _ => None,
        })
        .expect("a mob near the Crossroads");
    (realm, guid, mob, view)
}
