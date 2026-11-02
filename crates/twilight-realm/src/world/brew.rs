//! Using alchemy (#543, ADR 0025 amended 2026-11-01), as the Alchemy prototype's `drink`,
//! `applyOil`, `useFlask`, `swing`, `giveShield` and `expire` do: a potion or hybrid runs its
//! effects and starts the shared potion cooldown, an oil coats a hand and runs its effects on a
//! landed swing, and a flask adds its two stats, through death. The runner matches effect kinds
//! (`content/alchemy.toml`), never a product.

use benilla_protocol::SessionEvent;

use super::food::use_up_one;
use super::{cast, InWorld, Ticked};
use crate::alchemy::{self, Grade, Product, Row, Step, UseEffect};
use crate::characters::{Char, Worn};
use crate::items;
use crate::rng::Rng;
use crate::rules::power::PowerKind;
use crate::rules::school::School;
use crate::space::distance;

/// The two hands an oil coats, as `Char::oils` holds them.
const MAIN: usize = 0;
const OFF: usize = 1;
const HANDS: [&str; 2] = ["main hand", "off hand"];

#[cfg(test)]
thread_local! {
    /// Makes every oil proc, for the tests.
    static ALWAYS_PROC: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A product being used: its row's grade and step, the spell its effects show as, and the mob
/// they act on (the selected one for a potion, the one hit for an oil).
#[derive(Clone, Copy)]
struct Used {
    product: &'static Product,
    grade: Grade,
    step: Step,
    spell_id: u32,
    target: Option<u64>,
}

impl InWorld {
    /// `CMSG_USE_ITEM` on the item in player slot `slot`, if it is a potion, hybrid, oil or
    /// flask: `None` for any other item, else whether one was used up.
    pub(crate) fn use_alchemy(
        &mut self,
        slot: crate::inventory::Pos,
        c: &mut Char,
        wall: f64,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) -> Option<bool> {
        let entry = c.inventory.get(slot)?.entry;
        let row = alchemy::row(entry)?;
        // A row another content file holds at the entry (a test's meal) is not alchemy's.
        let t = items::template(entry)?;
        if !alchemy::item(entry).is_some_and(|a| std::ptr::eq(t, a)) {
            return None;
        }
        let spell_id = t.use_spell;
        let used = |product, grade, step| Used {
            product,
            grade,
            step,
            spell_id,
            target: None,
        };
        let usable = matches!(
            row,
            Row::Potion { .. } | Row::Hybrid { .. } | Row::Oil { .. } | Row::Flask { .. }
        );
        if !usable {
            return None;
        }
        if !self.player.alive() {
            events.push(cast::refused(spell_id, cast::SPELL_FAILED_CASTER_DEAD));
            return Some(false);
        }
        Some(match row {
            Row::Potion {
                product,
                grade,
                step,
            }
            | Row::Hybrid {
                product,
                grade,
                step,
            } => self.drink(slot, used(product, grade, step), c, wall, rng, events),
            Row::Oil { product, .. } => self.coat(slot, entry, product, spell_id, c, events),
            _ => self.use_flask(slot, entry, spell_id, c, events),
        })
    }

    /// A potion or hybrid: refused while the potion cooldown runs, or a throw without a target;
    /// else one is used up, the cooldown starts and its effects run in order.
    fn drink(
        &mut self,
        slot: crate::inventory::Pos,
        mut used: Used,
        c: &mut Char,
        wall: f64,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        if c.potion_ready - wall > f64::from(cast::READY_SLACK_SECS) {
            events.push(cast::refused(used.spell_id, cast::SPELL_FAILED_NOT_READY));
            return false;
        }
        used.target = match used
            .product
            .effects
            .iter()
            .find(|e| e.kind == "thrown_damage")
        {
            Some(e) => match self.throw_target(e, c) {
                Ok(target) => Some(target),
                Err(reason) => {
                    events.push(cast::refused(used.spell_id, reason));
                    return false;
                }
            },
            None => self.selected_mob(),
        };
        let Some(item_guid) = use_up_one(slot, c, events) else {
            return false;
        };
        let thrown = used
            .product
            .effects
            .iter()
            .any(|e| e.kind == "thrown_damage");
        let at = if thrown { used.target } else { None };
        self.went(used.spell_id, item_guid, at, events);
        c.potion_ready = wall + f64::from(alchemy::use_().potion_cooldown_secs);
        self.run_effects(used, c, rng, events, &mut Ticked::default());
        true
    }

    /// The selected mob a throw lands on: a living hostile one within the effect's `range_yd`.
    fn throw_target(&self, e: &UseEffect, c: &Char) -> Result<u64, u8> {
        let selected = self.skills.selection;
        let mob = self
            .mobs
            .iter()
            .find(|m| selected != 0 && m.guid == selected)
            .filter(|m| !m.template.friendly && m.alive())
            .ok_or(cast::SPELL_FAILED_BAD_TARGETS)?;
        let range = e
            .params
            .get("range_yd")
            .and_then(|v| v.as_integer())
            .unwrap_or(0) as f32;
        if !mob.admits(self.guid) || distance(mob.position, c.position) > range {
            return Err(cast::SPELL_FAILED_OUT_OF_RANGE);
        }
        Ok(mob.guid)
    }

    /// The selected mob, if one is.
    fn selected_mob(&self) -> Option<u64> {
        let selected = self.skills.selection;
        self.mobs
            .iter()
            .any(|m| selected != 0 && m.guid == selected)
            .then_some(selected)
    }

    /// The use went off: `SMSG_CAST_RESULT` (ok) and `SMSG_SPELL_GO` naming the item, at
    /// `target`, else at the player.
    fn went(
        &self,
        spell_id: u32,
        item_guid: u64,
        target: Option<u64>,
        events: &mut Vec<SessionEvent>,
    ) {
        let at = target.unwrap_or(self.guid);
        events.push(SessionEvent::CastResult {
            spell_id,
            success: true,
            reason: None,
            arg: None,
        });
        events.push(SessionEvent::SpellGo {
            caster: self.guid,
            spell_id,
            cast_flags: 0,
            hits: vec![at],
            misses: Vec::new(),
            target: Some(at),
            go_target: None,
            dest: None,
            ammo_display_id: None,
            item_caster: Some(item_guid),
        });
    }

    /// An oil: one is used up and coats the main hand if it wears none, else an off-hand weapon
    /// that wears none, else it replaces the main hand's.
    fn coat(
        &mut self,
        slot: crate::inventory::Pos,
        entry: u32,
        product: &Product,
        spell_id: u32,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        let Some(item_guid) = use_up_one(slot, c, events) else {
            return false;
        };
        self.went(spell_id, item_guid, None, events);
        let hand = if c.oils[MAIN].is_none() {
            MAIN
        } else if self.player.stats.off_hand.is_some() && c.oils[OFF].is_none() {
            OFF
        } else {
            MAIN
        };
        let secs = alchemy::use_().oil_secs;
        c.oils[hand] = Some(Worn {
            item: entry,
            left_secs: secs as f32,
        });
        let line = format!(
            "{} on your {} for {} min.",
            product.name,
            HANDS[hand],
            secs / 60
        );
        events.push(crate::quests::system_line(line));
        true
    }

    /// A flask: one is used up and replaces the flask worn; its stats join the character's.
    fn use_flask(
        &mut self,
        slot: crate::inventory::Pos,
        entry: u32,
        spell_id: u32,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        let Some(item_guid) = use_up_one(slot, c, events) else {
            return false;
        };
        self.went(spell_id, item_guid, None, events);
        if let Some(was) = c.flask {
            let line = format!(
                "{} is replaced; only one flask is active.",
                name_of(was.item)
            );
            events.push(crate::quests::system_line(line));
        }
        let secs = alchemy::use_().flask_secs;
        c.flask = Some(Worn {
            item: entry,
            left_secs: secs as f32,
        });
        let line = format!(
            "{} for {} min. It survives death.",
            name_of(entry),
            secs / 60
        );
        events.push(crate::quests::system_line(line));
        self.refresh_gear(c, events);
        self.flask_shown = false;
        self.show_flask(c, events);
        true
    }

    /// Runs `used`'s effects in order. The hybrid-only kinds are #544's.
    fn run_effects(
        &mut self,
        used: Used,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let me = self.guid;
        for e in &used.product.effects {
            match e.kind.as_str() {
                "heal" => {
                    let amount = roll(used, e, "amount", rng);
                    self.heal_from(me, c, used.spell_id, amount, events);
                }
                "restore_power" => {
                    let key = if self.player.power.kind == PowerKind::Mana {
                        "mana"
                    } else {
                        "power"
                    };
                    let amount = roll(used, e, key, rng);
                    self.energize_player(used.spell_id, amount, events);
                }
                "shield" => {
                    let amount = roll(used, e, "amount", rng);
                    let secs = e
                        .params
                        .get("secs")
                        .and_then(|v| v.as_integer())
                        .unwrap_or(i64::from(alchemy::use_().potion_shield_secs));
                    let kept = self.alchemy_shield(used.spell_id, amount, secs as f32, events);
                    if let Some(larger) = kept {
                        let line = format!(
                            "{}: a {amount} shield, but your {larger} shield is larger and stays.",
                            used.product.name
                        );
                        events.push(crate::quests::system_line(line));
                    }
                }
                "thrown_damage" | "damage" => {
                    let Some(target) = used.target else {
                        continue;
                    };
                    let amount = roll(used, e, "amount", rng);
                    let school = school_of(e);
                    events.push(SessionEvent::SpellDamageLog(cast::damage_log(
                        me,
                        target,
                        used.spell_id,
                        amount,
                        school,
                        false,
                    )));
                    self.hurt_school(target, amount, school, c, rng, events, ticked);
                }
                "drop_threat" => {
                    let amount = roll(used, e, "amount", rng);
                    if let Some(mob) = self.mobs.iter_mut().find(|m| Some(m.guid) == used.target) {
                        mob.threat.lower(me, amount as f32);
                    }
                }
                _ => {}
            }
        }
    }

    /// A white swing of the player's landed on `target`: the oil on that hand runs its effects
    /// at `[use] oil_proc_chance`, or on every landed swing for an oil that procs on every hit.
    pub(super) fn oil_swing_landed(
        &mut self,
        target: u64,
        off_hand: bool,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let Some(oil) = c.oils[usize::from(off_hand)] else {
            return;
        };
        let Some(Row::Oil {
            product,
            grade,
            step,
        }) = alchemy::row(oil.item)
        else {
            return;
        };
        if !oil_procs(product.every_hit, rng) {
            return;
        }
        let spell_id = items::template(oil.item).map_or(0, |t| t.use_spell);
        let used = Used {
            product,
            grade,
            step,
            spell_id,
            target: Some(target),
        };
        self.run_effects(used, c, rng, events, ticked);
    }

    /// The oils and the flask run down by `dt`; at nothing each wears off.
    pub(super) fn brew_tick(&mut self, dt: f32, c: &mut Char, events: &mut Vec<SessionEvent>) {
        for hand in [MAIN, OFF] {
            let Some(oil) = &mut c.oils[hand] else {
                continue;
            };
            oil.left_secs -= dt;
            if oil.left_secs <= 0.0 {
                let line = format!("{} has worn off your {}.", name_of(oil.item), HANDS[hand]);
                c.oils[hand] = None;
                events.push(crate::quests::system_line(line));
            }
        }
        if let Some(flask) = &mut c.flask {
            flask.left_secs -= dt;
            if flask.left_secs <= 0.0 {
                let line = format!("{} has worn off.", name_of(flask.item));
                c.flask = None;
                events.push(crate::quests::system_line(line));
                self.refresh_gear(c, events);
            }
        }
        self.show_flask(c, events);
    }

    /// The flask's aura brought in line with the flask: up with its time left while one is on
    /// and the player lives (so after a login or a release too), down otherwise.
    pub(super) fn show_flask(&mut self, c: &Char, events: &mut Vec<SessionEvent>) {
        let spell_id = alchemy::use_().flask_aura_spell;
        match c.flask.filter(|_| self.player.alive()) {
            Some(flask) if !self.flask_shown => {
                self.show_timed_marker(spell_id, flask.left_secs, events);
                self.flask_shown = true;
            }
            None if self.flask_shown => {
                self.hide_marker(spell_id, events);
                self.flask_shown = false;
            }
            _ => {}
        }
    }
}

/// Whether an oil procs on a landed swing: every time for one that procs on every hit, else one
/// roll under `[use] oil_proc_chance`.
fn oil_procs(every_hit: bool, rng: &mut Rng) -> bool {
    #[cfg(test)]
    if ALWAYS_PROC.with(std::cell::Cell::get) {
        return true;
    }
    every_hit || rng.below(1000) < (alchemy::use_().oil_proc_chance * 1000.0).round() as u32
}

/// Effect `e`'s number `param` for `used`: its product's number at the grade, times the step,
/// rounded half up; a range rolls between its ends. 0 when the effect leaves it out.
fn roll(used: Used, e: &UseEffect, param: &str, rng: &mut Rng) -> u32 {
    e.params
        .get(param)
        .and_then(|v| v.as_str())
        .and_then(|key| alchemy::num(used.product, key, used.grade, used.step))
        .map_or(0, |[lo, hi]| rng.urand(lo, hi))
}

/// Effect `e`'s `school`; physical when it names none.
fn school_of(e: &UseEffect) -> School {
    match e.params.get("school").and_then(|v| v.as_str()) {
        Some("fire") => School::Fire,
        Some("frost") => School::Frost,
        Some("nature") => School::Nature,
        Some("shadow") => School::Shadow,
        Some("arcane") => School::Arcane,
        Some("holy") => School::Holy,
        _ => School::Physical,
    }
}

/// The product name of alchemy item `item`: "Burning Oil", "Flask of Stone and Ember".
fn name_of(item: u32) -> &'static str {
    match alchemy::row(item) {
        Some(
            Row::Potion { product, .. }
            | Row::Oil { product, .. }
            | Row::Hybrid { product, .. }
            | Row::Flask { product, .. },
        ) => &product.name,
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Host, Realm, RealmCommand};

    const MINOR_BURNING_OIL: u32 = 963_240;
    const MINOR_QUIETING_OIL: u32 = 963_280;
    /// `SPELL_SCHOOL_FIRE` in a damage log.
    const FIRE: u8 = 2;

    struct Flat;
    impl Host for Flat {
        fn ground_height(&self, _: u32, _: f32, _: f32, _: f32) -> Option<f32> {
            Some(0.0)
        }
        fn line_of_sight(&self, _: u32, _: [f32; 3], _: [f32; 3]) -> bool {
            true
        }
        fn area_at(&self, _: u32, _: f32, _: f32) -> Option<u32> {
            None
        }
    }

    /// A new warrior in the world, the mobs about them lent to it, under `oil` on the main hand.
    fn oiled(oil: u32) -> (Realm, u64) {
        let mut realm = Realm::with_seed(4);
        realm.handle(RealmCommand::CreateCharacter(
            benilla_protocol::messages::CharCreateReq {
                name: "Oiler".into(),
                race: 1,
                class: 1,
                gender: 0,
                skin: 0,
                face: 0,
                hair_style: 0,
                hair_color: 0,
                facial_hair: 0,
                start: 0,
            },
        ));
        let me = realm.characters.newest().unwrap().guid;
        realm.characters.get_mut(me).unwrap().oils[MAIN] = Some(Worn {
            item: oil,
            left_secs: 1800.0,
        });
        realm.handle(RealmCommand::EnterWorld { guid: me });
        realm.tick(0.0, &Flat);
        realm.lend();
        (realm, me)
    }

    /// The first hostile mob's index in the world.
    fn hostile(w: &InWorld) -> usize {
        w.mobs
            .iter()
            .position(|m| m.alive() && !m.template.friendly)
            .expect("a hostile mob about")
    }

    /// Main-hand swings at the first hostile mob, kept at full health, until one lands: that
    /// swing's events, and the mob's guid.
    fn swing_until_landed(realm: &mut Realm, me: u64, rng: &mut Rng) -> (Vec<SessionEvent>, u64) {
        let w = realm.world.as_mut().unwrap();
        let c = realm.characters.get_mut(me).unwrap();
        let i = hostile(w);
        for _ in 0..50 {
            w.mobs[i].health = w.mobs[i].stats.max_health;
            let mut events = Vec::new();
            w.white_swing(
                i,
                true,
                false,
                false,
                c,
                rng,
                &mut events,
                &mut Ticked::default(),
            );
            let landed = events
                .iter()
                .any(|e| matches!(e, SessionEvent::AttackerState(s) if s.damage > 0));
            if landed {
                return (events, w.mobs[i].guid);
            }
        }
        panic!("no swing landed in 50");
    }

    #[test]
    fn a_landed_swing_under_burning_oil_also_burns_10_to_14_fire_when_it_procs() {
        ALWAYS_PROC.with(|p| p.set(true));
        let (mut realm, me) = oiled(MINOR_BURNING_OIL);
        let (events, mob) = swing_until_landed(&mut realm, me, &mut Rng::new(1));
        let burns: Vec<(u32, u8, u32)> = events
            .iter()
            .filter_map(|e| match e {
                SessionEvent::SpellDamageLog(l) if l.attacker == me && l.target == mob => {
                    Some((l.damage, l.school, l.spell_id))
                }
                _ => None,
            })
            .collect();
        assert_eq!(burns.len(), 1, "{burns:?}");
        let (damage, school, spell_id) = burns[0];
        assert!((10..=14).contains(&damage), "{damage}");
        assert_eq!((school, spell_id), (FIRE, alchemy::use_().oil_spell));
    }

    #[test]
    fn a_quieting_oil_lowers_threat_on_every_landed_swing() {
        let (mut realm, me) = oiled(MINOR_QUIETING_OIL);
        let threat = |realm: &Realm| {
            let w = realm.world.as_ref().unwrap();
            w.mobs[hostile(w)].threat.of(me)
        };
        {
            let w = realm.world.as_mut().unwrap();
            let i = hostile(w);
            w.mobs[i].threat.add(me, 1000.0);
        }
        let mut rng = Rng::new(1);
        swing_until_landed(&mut realm, me, &mut rng);
        assert_eq!(threat(&realm), 990.0);
        swing_until_landed(&mut realm, me, &mut rng);
        assert_eq!(threat(&realm), 980.0);
    }
}
