//! Herbalism (#438, ADR 0024): Brother Ansel teaches it, each character who knows it has Herb
//! Patches of their own standing in the herb zones, and Find Herbs draws them on the minimap.

use std::collections::HashMap;

use benilla_protocol::messages::{buy_result, GossipOption};
use benilla_protocol::SessionEvent;

use crate::audience::Audience;
use crate::characters::Char;
use crate::content::EASTERN_KINGDOMS;
use crate::equipment;
use crate::fields::values;
use crate::fish::Block;
use crate::herbs;
use crate::herbs::{Effect, Skill};
use crate::hub::Npc;
use crate::items::Profession;
use crate::objects::{ObjectKind, ObjectSpec};
use crate::rng::Rng;
use crate::rules::forage::{self, Cell};
use crate::rules::game_clock;
use crate::space;
use crate::{HerbAction, Host};
use benilla_protocol::{HerbBasketRow, HerbKnown, HerbLog, HerbTile};

/// `HerbGridClosed` reasons.
const CLOSED_OUT_OF_PULLS: u8 = 0;
const CLOSED_EVERY_TILE: u8 = 1;
const CLOSED_LEFT: u8 = 2;
const CLOSED_DAMAGE: u8 = 3;
const CLOSED_WALKED_AWAY: u8 = 4;

/// Brother Ansel's menu lines: the merchant window and the lesson.
const OPTION_BROWSE: u32 = 1;
const OPTION_TEACH: u32 = 2;
/// Hedda Thornwick's perk reset.
const OPTION_RESET: u32 = 3;
/// `GOSSIP_ICON_VENDOR` and `GOSSIP_ICON_TRAINER`.
const ICON_VENDOR: u8 = 1;
const ICON_TRAINER: u8 = 3;

/// The Herbalist's Sickle, which Brother Ansel gives with the lesson.
const SICKLE: u32 = 961101;

/// `Herbalist`: the character's Herbalist Level, XP and perk slots, for login and every change.
pub(crate) fn herbalist(c: &Char) -> SessionEvent {
    let h = herbs::herbalist();
    let level = u32::from(c.herbalist_level);
    let mut perks = c.herb_perks.clone();
    perks.resize((level / h.perk_every) as usize, String::new());
    SessionEvent::Herbalist {
        knows: c.knows_herbalism,
        level: c.herbalist_level,
        xp: c.herbalist_xp,
        to_next: if level < h.cap {
            h.xp_to_next(level)
        } else {
            0
        },
        title: h.title(level).to_owned(),
        perks,
        tour_seen: c.herb_tour_seen,
    }
}

/// `HerbalismJournal`: the slots, the Satchel's herbalism items, the pick counts, each zone's plants
/// and the perks, for login and every change.
pub(crate) fn journal(c: &Char) -> SessionEvent {
    let mut picks = c.herb_picks.clone();
    picks.sort_unstable_by_key(|p| p.0);
    SessionEvent::HerbalismJournal {
        tool: c.herb_tool.unwrap_or(0),
        consumable: c.herb_consumable.unwrap_or(0),
        satchel: c.inventory.satchel.rows(Profession::Herbalism),
        picks,
        zones: herbs::zones()
            .into_iter()
            .map(|z| {
                let (legendary, rest): (Vec<_>, Vec<_>) = herbs::plants_in(&z.key)
                    .into_iter()
                    .partition(|p| p.rarity == herbs::Rarity::Legendary);
                (
                    z.zone,
                    rest.into_iter().chain(legendary).map(|p| p.item).collect(),
                )
            })
            .collect(),
        reveal_at: herbs::herbalist().reveal_at,
        perks: herbs::perks()
            .into_iter()
            .map(|p| {
                (
                    p.key.clone(),
                    p.name.clone(),
                    p.kind == herbs::PerkKind::Boon,
                    p.text.clone(),
                )
            })
            .collect(),
    }
}

/// The Tool Skill the character's Tool slot gives: its item must still be in the Satchel.
fn tool_skill(c: &Char) -> Option<Skill> {
    c.herb_tool
        .filter(|&item| c.inventory.satchel.count(item) > 0)
        .and_then(herbs::tool)
        .map(|t| t.skill)
}

/// `HerbTracking`: whether Find Herbs is on, for login and every toggle.
pub(crate) fn tracking(c: &Char) -> SessionEvent {
    SessionEvent::HerbTracking {
        patch_entry: herbs::patch().entry,
        on: c.herb_tracking,
    }
}

/// What `c`'s next perk reset costs, in copper: the base doubled once per reset so far.
fn reset_cost(c: &Char) -> u32 {
    herbs::herbalist()
        .reset_base_copper
        .saturating_mul(1u32.checked_shl(c.perk_resets).unwrap_or(u32::MAX))
}

/// `copper` as the menu shows it: gold, then silver and copper when there are any.
fn money_text(copper: u32) -> String {
    let (gold, silver, rest) = (copper / 10_000, copper / 100 % 100, copper % 100);
    let mut parts = vec![format!("{gold} gold")];
    if silver > 0 {
        parts.push(format!("{silver} silver"));
    }
    if rest > 0 {
        parts.push(format!("{rest} copper"));
    }
    parts.join(" ")
}

/// A herbalist's gossip lines for `c`: the goods, the lesson while they do not know Herbalism
/// (`teaches_herbalism`), and the perk reset once they hold a perk (`resets_perks`).
pub(crate) fn herbalist_options(npc: &Npc, c: &Char) -> Vec<GossipOption> {
    let mut options = vec![GossipOption {
        index: OPTION_BROWSE,
        icon: ICON_VENDOR,
        coded: false,
        message: "Let me browse your goods.".into(),
    }];
    if npc.teaches_herbalism && !c.knows_herbalism {
        options.push(GossipOption {
            index: OPTION_TEACH,
            icon: ICON_TRAINER,
            coded: false,
            message: "Teach me Herbalism.".into(),
        });
    }
    if npc.resets_perks && c.herb_perks.iter().any(|k| !k.is_empty()) {
        options.push(GossipOption {
            index: OPTION_RESET,
            icon: ICON_TRAINER,
            coded: false,
            message: format!("Forget my perks ({}).", money_text(reset_cost(c))),
        });
    }
    options
}

/// One standing patch of a character's.
struct Live {
    zone: usize,
    spot: usize,
    guid: u64,
}

/// The Forager's Grid a character has open: the patch it came from and its rolled tiles.
struct Open {
    patch: u64,
    zone: usize,
    grid: forage::Grid,
    /// The consumable taken from the Satchel for this patch, 0 for none, and what it does.
    consumable: u32,
    effect: Option<Effect>,
    /// One line per start-of-patch effect.
    notes: Vec<String>,
    /// The Herbalist level the patch was rolled at.
    grown_for: u8,
    /// What the patch has given so far, merged by item; the Lost Coin Purse adds no row.
    basket: Vec<HerbBasketRow>,
    /// Thorns pulled and Herbalist XP gained so far.
    thorns: u32,
    xp: u32,
}

/// One character's patches: those standing, and the replacements waiting on their delay.
#[derive(Default)]
struct Patches {
    live: Vec<Live>,
    /// The open grid, if any. Assumed: it lives here, beside the patches, not on the session.
    open: Option<Open>,
    /// `(zone, due wall secs)`.
    pending: Vec<(usize, f64)>,
}

/// Every character's patches, keyed by player guid. Not saved: a login makes a fresh set.
#[derive(Default)]
pub(crate) struct HerbPatches(HashMap<u64, Patches>);

impl crate::Realm {
    /// A line of a herbalist's menu: the goods open the merchant window; the lesson teaches
    /// Herbalism; the reset forgets the perks for money. Each closes the menu.
    pub(crate) fn herbalist_select(&mut self, npc: &Npc, option: u32) {
        if !self.near(npc) || !matches!(option, OPTION_BROWSE | OPTION_TEACH | OPTION_RESET) {
            return;
        }
        let Some(guid) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        let Some(c) = self.characters.get_mut(guid) else {
            return;
        };
        self.events.push(SessionEvent::GossipComplete);
        if option == OPTION_BROWSE {
            crate::vendor::list(npc.guid(), c, &mut self.events);
            return;
        }
        if option == OPTION_RESET {
            if !npc.resets_perks || !c.herb_perks.iter().any(|k| !k.is_empty()) {
                return;
            }
            let cost = reset_cost(c);
            if c.inventory.money < cost {
                self.events.push(SessionEvent::VendorBuyFailed {
                    vendor: npc.guid(),
                    item_entry: 0,
                    reason: buy_result::NOT_ENOUGH_MONEY,
                });
                return;
            }
            c.inventory.money -= cost;
            self.events
                .push(values(c.guid, &[equipment::money_pair(&c.inventory)]));
            c.herb_perks.clear();
            c.perk_resets += 1;
            self.events.push(herbalist(c));
            self.save(guid);
            return;
        }
        if option == OPTION_TEACH && (!npc.teaches_herbalism || c.knows_herbalism) {
            return;
        }
        c.knows_herbalism = true;
        match equipment::receive(c, SICKLE, 1, &mut self.events) {
            Ok(()) => c.herb_tool = Some(SICKLE),
            Err(reason) => self.events.push(equipment::inventory_failure(reason, 0)),
        }
        let spell = herbs::spell();
        self.events.push(SessionEvent::SpellLearned {
            spell_id: spell.learn,
        });
        self.events.push(SessionEvent::SpellLearned {
            spell_id: spell.find,
        });
        self.events.push(herbalist(c));
        self.events.push(journal(c));
        self.save(guid);
    }

    /// Find Herbs: toggles the served character's tracking. False when they do not know
    /// Herbalism, so the cast takes its usual course.
    pub(crate) fn find_herbs(&mut self) -> bool {
        let Some(guid) = self.world.as_ref().map(|w| w.guid) else {
            return false;
        };
        let Some(c) = self.characters.get_mut(guid) else {
            return false;
        };
        if !c.knows_herbalism {
            return false;
        }
        c.herb_tracking = !c.herb_tracking;
        self.events.push(tracking(c));
        self.save(guid);
        true
    }

    /// Fills the served character's patches on their first tick knowing Herbalism, then spawns
    /// each replacement that has come due.
    pub(crate) fn tick_herbs(&mut self, host: &dyn Host) {
        let Some(guid) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        if !self.characters.get(guid).is_some_and(|c| c.knows_herbalism) {
            return;
        }
        self.check_open_herb_grid(guid);
        if !self.herb_patches.0.contains_key(&guid) {
            self.herb_patches.0.insert(guid, Patches::default());
            for zone in 0..herbs::zones().len() {
                for _ in 0..herbs::zones()[zone].live {
                    self.grow_herb_patch(host, guid, zone);
                }
            }
        }
        let wall = self.wall;
        let due: Vec<usize> = {
            let Some(p) = self.herb_patches.0.get_mut(&guid) else {
                return;
            };
            let (due, waiting): (Vec<_>, Vec<_>) =
                p.pending.drain(..).partition(|&(_, at)| at <= wall);
            p.pending = waiting;
            due.into_iter().map(|(zone, _)| zone).collect()
        };
        for zone in due {
            self.grow_herb_patch(host, guid, zone);
        }
    }

    /// Spawns one patch for `guid` at a random spot of `zone` that holds none of theirs. A spot the
    /// ground refuses is skipped and another tried.
    fn grow_herb_patch(&mut self, host: &dyn Host, guid: u64, zone: usize) {
        let zones = herbs::zones();
        let Some(z) = zones.get(zone) else {
            return;
        };
        let patch = herbs::patch();
        let mut free: Vec<usize> = (0..z.points.len())
            .filter(|&spot| {
                !self
                    .herb_patches
                    .0
                    .get(&guid)
                    .is_some_and(|p| p.live.iter().any(|l| l.zone == zone && l.spot == spot))
            })
            .collect();
        while !free.is_empty() {
            let spot = free.swap_remove(self.rng.below(free.len() as u32) as usize);
            let spec = ObjectSpec {
                entry: patch.entry,
                display: patch.display,
                kind: ObjectKind::HerbPatch { zone, spot },
                map: EASTERN_KINGDOMS,
                position: [z.points[spot].x, z.points[spot].y, 0.0],
                orientation: 0.0,
            };
            if let Some(object) = self.spawn_object(host, spec, Audience::of(&[guid])) {
                if let Some(p) = self.herb_patches.0.get_mut(&guid) {
                    p.live.push(Live {
                        zone,
                        spot,
                        guid: object,
                    });
                }
                return;
            }
        }
    }

    /// A patch is finished: it goes, and its replacement is queued `respawn_secs` on.
    pub fn finish_herb_patch(&mut self, patch: u64) {
        let due = self.wall + f64::from(herbs::patch().respawn_secs);
        for p in self.herb_patches.0.values_mut() {
            if let Some(i) = p.live.iter().position(|l| l.guid == patch) {
                let gone = p.live.remove(i);
                p.pending.push((gone.zone, due));
                self.objects.remove(patch);
                return;
            }
        }
    }

    /// The served character clicks one of their patches: rolls its grid and sends it. Nothing when
    /// they know no Herbalism, a grid is already open, or the patch is not one of theirs. The chosen
    /// consumable, if the Satchel holds one, is used up and applies to this patch.
    pub(crate) fn open_herb_grid(&mut self, patch: u64, zone: usize) {
        let Some(w) = self.world.as_mut() else {
            return;
        };
        let guid = w.guid;
        let Some(c) = self.characters.get_mut(guid) else {
            return;
        };
        let Some(p) = self.herb_patches.0.get_mut(&guid) else {
            return;
        };
        if !c.knows_herbalism || p.open.is_some() || !p.live.iter().any(|l| l.guid == patch) {
            return;
        }
        let zones = herbs::zones();
        let Some(z) = zones.get(zone) else {
            return;
        };
        let tuning = herbs::grid();
        let minute = game_clock::minute_of_day(self.wall);
        let grid = forage::roll(
            &mut self.rng,
            &herbs::plants_in(&z.key),
            u32::from(c.herbalist_level),
            minute,
            tuning,
            z.thorns.unwrap_or(tuning.thorns),
            forage::Perks::of(&c.herb_perks),
        );
        let mut open = Open {
            patch,
            zone,
            grid,
            consumable: 0,
            effect: None,
            notes: Vec::new(),
            grown_for: c.herbalist_level,
            basket: Vec::new(),
            thorns: 0,
            xp: 0,
        };
        let chosen = c
            .herb_consumable
            .filter(|&item| c.inventory.satchel.count(item) > 0)
            .and_then(herbs::consumable);
        if let Some(row) = chosen {
            equipment::take(c, row.item, 1, &mut self.events);
            open.consumable = row.item;
            open.effect = Some(row.effect);
            apply_consumable(&mut open, &mut self.rng, &z.key);
        }
        w.hurt = false;
        self.events.push(grid_event(c, &open, minute, None, None));
        p.open = Some(open);
    }

    /// What the served character does on their open grid.
    pub(crate) fn herb_action(&mut self, action: HerbAction) {
        let Some(guid) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        let t = herbs::grid();
        match action {
            HerbAction::TourSeen => {
                let Some(c) = self.characters.get_mut(guid) else {
                    return;
                };
                c.herb_tour_seen = true;
                self.events.push(herbalist(c));
                self.save(guid);
            }
            HerbAction::Leave => self.close_herb_grid(guid, CLOSED_LEFT),
            HerbAction::Pull(tile) => self.act(guid, |c, open, rng, ev| {
                let tile = usize::from(tile);
                let pulled = forage::pull(&mut open.grid, tile)?;
                Some(collect(c, rng, ev, open, tile, pulled.cell, pulled.cost))
            }),
            HerbAction::Sweep { tile, across } => self.act(guid, |c, open, rng, ev| {
                if tool_skill(c) != Some(Skill::Sweep) || usize::from(tile) >= open.grid.tiles.len()
                {
                    return None;
                }
                let reach = forage::Perks::of(&c.herb_perks).reach;
                let length = t.sweep_length as usize + usize::from(reach);
                let line = forage::sweep_line(open.grid.size, usize::from(tile), across, length);
                skill_reveal(c, rng, ev, open, &line, t.sweep_cost, 0, "Sweep")
            }),
            HerbAction::Snip(tile) => self.act(guid, |c, open, rng, ev| {
                if tool_skill(c) != Some(Skill::Snip) {
                    return None;
                }
                skill_reveal(
                    c,
                    rng,
                    ev,
                    open,
                    &[usize::from(tile)],
                    t.snip_cost,
                    t.snip_bonus,
                    "Snip",
                )
            }),
            HerbAction::Probe(tile) => self.act(guid, |c, open, _, _| {
                let tile = usize::from(tile);
                let g = &mut open.grid;
                if tool_skill(c) != Some(Skill::Probe)
                    || tile >= g.tiles.len()
                    || g.pulls_left < t.probe_cost
                {
                    return None;
                }
                let reach = forage::Perks::of(&c.herb_perks).reach;
                let square = forage::square(g, tile);
                let hidden: Vec<usize> = square
                    .iter()
                    .copied()
                    .filter(|&i| !g.tiles[i].pulled)
                    .collect();
                if hidden.is_empty() {
                    return None;
                }
                for i in hidden {
                    g.known[i] = forage::kind_of(g.tiles[i].cell);
                    g.probed[i] = reach;
                }
                g.pulls_left -= t.probe_cost;
                Some(HerbLog {
                    kind: LOG_SKILL,
                    title: "Probe".to_owned(),
                    place: span(g.size, &square),
                    xp: 0,
                })
            }),
            HerbAction::Sight(tiles) => self.act(guid, |_, open, _, _| {
                let g = &mut open.grid;
                if open.effect != Some(Effect::Sight) || !g.ready {
                    return None;
                }
                let mut marked = Vec::new();
                for tile in tiles.map(usize::from) {
                    if g.tiles.get(tile).is_some_and(|t| !t.pulled) && !marked.contains(&tile) {
                        marked.push(tile);
                    }
                }
                if marked.is_empty() {
                    return None;
                }
                for &i in &marked {
                    g.known[i] = forage::kind_of(g.tiles[i].cell);
                }
                g.ready = false;
                let place = marked
                    .iter()
                    .map(|&i| tile_name(g.size, i))
                    .collect::<Vec<_>>()
                    .join(", ");
                Some(consumable_log(open.consumable, place))
            }),
            HerbAction::Twine(tile) => self.act(guid, |_, open, _, _| {
                let g = &mut open.grid;
                let tile = usize::from(tile);
                if open.effect != Some(Effect::Twine) || !g.ready || tile >= g.tiles.len() {
                    return None;
                }
                let square = forage::square(g, tile);
                for &i in &square {
                    if !g.tiles[i].pulled && g.tiles[i].cell == Cell::Weed {
                        g.known[i] = 2;
                    }
                }
                g.ready = false;
                Some(consumable_log(open.consumable, span(g.size, &square)))
            }),
            HerbAction::ArmCharm => self.act(guid, |_, open, _, _| {
                let g = &mut open.grid;
                if open.effect != Some(Effect::Charm) || !g.ready {
                    return None;
                }
                g.charm_armed = !g.charm_armed;
                Some(consumable_log(open.consumable, String::new()))
            }),
        }
    }

    /// Runs `f` on the served character and their open grid. When it returns the action's log
    /// entry, its XP is added, the grid is sent, and the patch ends when it is out of pulls or
    /// tiles: a last grid shows the tiles left unpulled, then the grid closes. `None` changes
    /// nothing.
    fn act<F>(&mut self, guid: u64, f: F)
    where
        F: FnOnce(&mut Char, &mut Open, &mut Rng, &mut Vec<SessionEvent>) -> Option<HerbLog>,
    {
        let Some(c) = self.characters.get_mut(guid) else {
            return;
        };
        let Some(open) = self
            .herb_patches
            .0
            .get_mut(&guid)
            .and_then(|p| p.open.as_mut())
        else {
            return;
        };
        let Some(log) = f(c, open, &mut self.rng, &mut self.events) else {
            return;
        };
        let xp = log.xp;
        open.xp += xp;
        let levelled = gain_herbalist_xp(c, xp);
        if xp > 0 || levelled {
            self.events.push(herbalist(c));
        }
        let minute = game_clock::minute_of_day(self.wall);
        self.events
            .push(grid_event(c, open, minute, Some(log), None));
        let end = if open.grid.pulls_left == 0 {
            Some(CLOSED_OUT_OF_PULLS)
        } else if !open.grid.any_hidden() {
            Some(CLOSED_EVERY_TILE)
        } else {
            None
        };
        if let Some(reason) = end {
            self.events
                .push(grid_event(c, open, minute, None, Some(reason)));
        }
        match end {
            Some(reason) => self.close_herb_grid(guid, reason),
            None if levelled => self.save(guid),
            None => {}
        }
    }

    /// `SetHerbTool`: a `[[tool]]` item the Satchel holds becomes the Tool, and `None` clears it.
    /// Anything else is ignored.
    pub(crate) fn set_herb_tool(&mut self, item: Option<u32>) {
        self.set_herb_slot(item, true);
    }

    /// `SetHerbConsumable`: a `[[consumable]]` item the Satchel holds becomes the Consumable, and
    /// `None` clears it. Anything else is ignored.
    pub(crate) fn set_herb_consumable(&mut self, item: Option<u32>) {
        self.set_herb_slot(item, false);
    }

    fn set_herb_slot(&mut self, item: Option<u32>, tool: bool) {
        let Some(guid) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        let Some(c) = self.characters.get_mut(guid) else {
            return;
        };
        let fits = item.is_none_or(|i| {
            let known = if tool {
                herbs::tool(i).is_some()
            } else {
                herbs::consumable(i).is_some()
            };
            known && c.inventory.satchel.count(i) > 0
        });
        if !fits {
            return;
        }
        if tool {
            c.herb_tool = item;
        } else {
            c.herb_consumable = item;
        }
        self.events.push(journal(c));
        if tool {
            // An open grid shows the Tool's skill, so it follows.
            let minute = game_clock::minute_of_day(self.wall);
            if let Some(open) = self.herb_patches.0.get(&guid).and_then(|p| p.open.as_ref()) {
                self.events.push(grid_event(c, open, minute, None, None));
            }
        }
        self.save(guid);
    }

    /// `PickPerk`: fills an empty, unlocked perk slot with a perk not already picked. A filled slot
    /// never changes here.
    pub(crate) fn pick_perk(&mut self, slot: u8, perk: &str) {
        let Some(guid) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        let Some(c) = self.characters.get_mut(guid) else {
            return;
        };
        let h = herbs::herbalist();
        let slots = (u32::from(c.herbalist_level) / h.perk_every) as usize;
        let slot = usize::from(slot);
        if !c.knows_herbalism || slot >= slots || herbs::perk(perk).is_none() {
            return;
        }
        c.herb_perks.resize(slots, String::new());
        if !c.herb_perks[slot].is_empty() || c.herb_perks.iter().any(|k| k == perk) {
            return;
        }
        c.herb_perks[slot] = perk.to_owned();
        self.events.push(herbalist(c));
        self.save(guid);
    }

    /// Closes `guid`'s open grid for `reason`: the patch is finished and the character saved.
    fn close_herb_grid(&mut self, guid: u64, reason: u8) {
        let Some(open) = self
            .herb_patches
            .0
            .get_mut(&guid)
            .and_then(|p| p.open.take())
        else {
            return;
        };
        self.events.push(SessionEvent::HerbGridClosed {
            patch: open.patch,
            reason,
        });
        self.finish_herb_patch(open.patch);
        self.save(guid);
    }

    /// Closes the open grid when the character took damage, or stands beyond reach of its patch.
    fn check_open_herb_grid(&mut self, guid: u64) {
        let Some(patch) = self
            .herb_patches
            .0
            .get(&guid)
            .and_then(|p| p.open.as_ref())
            .map(|o| o.patch)
        else {
            return;
        };
        if self
            .world
            .as_mut()
            .is_some_and(|w| std::mem::take(&mut w.hurt))
        {
            return self.close_herb_grid(guid, CLOSED_DAMAGE);
        }
        let away = match (self.objects.get(patch), self.characters.get(guid)) {
            (Some(o), Some(c)) => {
                space::flat_distance(o.spec.position, c.position) > space::INTERACTION_YD
            }
            _ => true,
        };
        if away {
            self.close_herb_grid(guid, CLOSED_WALKED_AWAY);
        }
    }

    /// Logging out takes the character's patches with it.
    pub(crate) fn forget_herb_patches(&mut self, guid: u64) {
        if let Some(p) = self.herb_patches.0.remove(&guid) {
            for l in p.live {
                self.objects.remove(l.guid);
            }
        }
    }
}

/// Applies the consumable `open` took to its grid.
fn apply_consumable(open: &mut Open, rng: &mut Rng, zone_key: &str) {
    let g = &mut open.grid;
    match open.effect {
        Some(Effect::Tea) => g.pulls_left += 1,
        Some(Effect::Salve) => g.free_thorns += 1,
        Some(Effect::Draught) => {
            let fresh: Vec<usize> = (0..g.tiles.len())
                .filter(|&i| matches!(g.tiles[i].cell, Cell::Plant { bloom: false, .. }))
                .collect();
            if fresh.is_empty() {
                open.notes
                    .push("Bloom Draught: no plant here could bloom.".to_owned());
                return;
            }
            let i = fresh[rng.below(fresh.len() as u32) as usize];
            if let Cell::Plant { count, bloom, .. } = &mut g.tiles[i].cell {
                *bloom = true;
                *count = herbs::grid().bloom_factor.round() as u32;
            }
            open.notes
                .push("Bloom Draught: one plant here is now blooming.".to_owned());
        }
        Some(Effect::Moonwater) => {
            let legendary = herbs::plants_in(zone_key)
                .into_iter()
                .find(|p| p.rarity == herbs::Rarity::Legendary)
                .map(|p| p.item);
            let tile = (0..g.tiles.len()).find(
                |&i| matches!(g.tiles[i].cell, Cell::Plant { item, .. } if Some(item) == legendary),
            );
            open.notes.push(match tile {
                Some(i) => {
                    g.known[i] = 1;
                    "Moonwater: the zone's legendary grows here.".to_owned()
                }
                None => "Moonwater: the zone's legendary does not grow here.".to_owned(),
            });
        }
        Some(Effect::Sight | Effect::Twine | Effect::Charm) => g.ready = true,
        None => {}
    }
}

/// `HerbLog` kinds.
const LOG_PLANT: u8 = 0;
const LOG_WEED: u8 = 1;
const LOG_THORN: u8 = 2;
const LOG_SKILL: u8 = 3;
const LOG_CONSUMABLE: u8 = 4;

/// A tile's name as the grid labels it: its column letter and 1-based row, "B3".
fn tile_name(size: usize, tile: usize) -> String {
    format!(
        "{}{}",
        (b'A' + (tile % size) as u8) as char,
        tile / size + 1
    )
}

/// The span `tiles` cover, from the first to the last: "B3-D3", or "B3" for one tile.
fn span(size: usize, tiles: &[usize]) -> String {
    match (tiles.iter().min(), tiles.iter().max()) {
        (Some(&first), Some(&last)) if first != last => {
            format!("{}-{}", tile_name(size, first), tile_name(size, last))
        }
        (Some(&first), _) => tile_name(size, first),
        _ => String::new(),
    }
}

/// The log entry for using the open patch's consumable on `place`: titled with its item's name.
fn consumable_log(consumable: u32, place: String) -> HerbLog {
    HerbLog {
        kind: LOG_CONSUMABLE,
        title: crate::items::template(consumable).map_or_else(String::new, |t| t.name.clone()),
        place,
        xp: 0,
    }
}

/// A Tool Skill named `skill` that reveals `tiles`: refused when the pulls left fall short of
/// `base` or nothing is hidden. Each tile collects as a pull would; a thorn costs its extra unless
/// the skill is a Snip (`bonus` above 0), and a Snipped plant yields `bonus` more.
#[allow(clippy::too_many_arguments)]
fn skill_reveal(
    c: &mut Char,
    rng: &mut Rng,
    events: &mut Vec<SessionEvent>,
    open: &mut Open,
    tiles: &[usize],
    base: u32,
    bonus: u32,
    skill: &str,
) -> Option<HerbLog> {
    if open.grid.pulls_left < base {
        return None;
    }
    let hidden: Vec<usize> = tiles
        .iter()
        .copied()
        .filter(|&i| open.grid.tiles.get(i).is_some_and(|t| !t.pulled))
        .collect();
    if hidden.is_empty() {
        return None;
    }
    let (mut cost, mut xp) = (base, 0);
    for tile in hidden {
        let spared = if bonus > 0 {
            std::mem::take(&mut open.grid.free_thorns)
        } else {
            0
        };
        let (mut cell, extra) = forage::reveal(&mut open.grid, tile)?;
        open.grid.free_thorns += spared;
        if bonus > 0 {
            if let Cell::Plant { count, .. } = &mut cell {
                *count += bonus;
            }
        } else {
            cost += extra;
        }
        open.grid.tiles[tile].cell = cell;
        xp += collect(c, rng, events, open, tile, cell, 1).xp;
    }
    open.grid.pulls_left = open.grid.pulls_left.saturating_sub(cost);
    Some(HerbLog {
        kind: LOG_SKILL,
        title: skill.to_owned(),
        place: span(open.grid.size, tiles),
        xp,
    })
}

/// What pulling `tile`, which held `cell` and cost `cost` pulls, gives `c`: the plant to the
/// Satchel, the basket and the pick count, a thorn to the patch's count, and the log entry with
/// its XP.
fn collect(
    c: &mut Char,
    rng: &mut Rng,
    events: &mut Vec<SessionEvent>,
    open: &mut Open,
    tile: usize,
    cell: Cell,
    cost: u32,
) -> HerbLog {
    let h = herbs::herbalist();
    let place = tile_name(open.grid.size, tile);
    let key = &herbs::zones()[open.zone].key;
    let level = u32::from(c.herbalist_level);
    match cell {
        Cell::Plant {
            item, count, money, ..
        } => {
            let row = herbs::plants_in(key).into_iter().find(|p| p.item == item);
            let plant_level = row.map_or(1, |p| p.level);
            let base = h.plant_xp_base + h.plant_xp_per_level * plant_level;
            let xp =
                (base as f32 * h.colour_factor(level.saturating_sub(plant_level))).round() as u32;
            match c.herb_picks.iter_mut().find(|(i, _)| *i == item) {
                Some(pick) => pick.1 += 1,
                None => c.herb_picks.push((item, 1)),
            }
            let item_name = crate::items::template(item).map_or("a plant", |t| t.name.as_str());
            let title = if let Some([lo, hi]) = money {
                let copper = rng.urand(lo, hi);
                c.inventory.money += copper;
                events.push(values(c.guid, &[equipment::money_pair(&c.inventory)]));
                format!("{item_name}, {copper} copper")
            } else {
                if let Err(reason) = equipment::receive(c, item, count, events) {
                    events.push(equipment::inventory_failure(reason, 0));
                }
                match open.basket.iter_mut().find(|r| r.item == item) {
                    Some(row) => row.count += count,
                    None => open.basket.push(HerbBasketRow {
                        item,
                        count,
                        part: herbs::part_of(item).to_owned(),
                    }),
                }
                format!("{item_name} x{count}")
            };
            HerbLog {
                kind: LOG_PLANT,
                title,
                place,
                xp,
            }
        }
        Cell::Weed => HerbLog {
            kind: LOG_WEED,
            title: "Weed".to_owned(),
            place,
            xp: h.weed_xp,
        },
        Cell::Thorn => {
            open.thorns += 1;
            HerbLog {
                kind: LOG_THORN,
                title: format!("Thorn, {cost} pull{}", if cost == 1 { "" } else { "s" }),
                place,
                xp: h.thorn_xp,
            }
        }
    }
}

/// Adds `xp` Herbalist XP, levelling up along the curve to the cap. True when a level came.
fn gain_herbalist_xp(c: &mut Char, xp: u32) -> bool {
    let h = herbs::herbalist();
    let before = c.herbalist_level;
    c.herbalist_xp += xp;
    while u32::from(c.herbalist_level) < h.cap
        && c.herbalist_xp >= h.xp_to_next(u32::from(c.herbalist_level))
    {
        c.herbalist_xp -= h.xp_to_next(u32::from(c.herbalist_level));
        c.herbalist_level += 1;
    }
    if u32::from(c.herbalist_level) >= h.cap {
        c.herbalist_xp = 0;
    }
    c.herbalist_level > before
}

/// How many times `c` has pulled `item`.
fn picks(c: &Char, item: u32) -> u32 {
    c.herb_picks
        .iter()
        .find(|(i, _)| *i == item)
        .map_or(0, |p| p.1)
}

/// The `HerbGrid` for `c`'s open grid: each tile as far as `c` may know it, and what the Journal
/// knows of the zone's plants. `over` is the reason a patch that ran out of pulls or tiles ended:
/// its unpulled tiles then show what they held, marked `missed`.
fn grid_event(
    c: &Char,
    open: &Open,
    minute: u32,
    log: Option<HerbLog>,
    over: Option<u8>,
) -> SessionEvent {
    let h = herbs::herbalist();
    let grid = &open.grid;
    let t = herbs::grid();
    let tool = tool_skill(c).zip(c.herb_tool).map(|(skill, item)| {
        let cost = match skill {
            Skill::Sweep => t.sweep_cost,
            Skill::Probe => t.probe_cost,
            Skill::Snip => t.snip_cost,
        };
        (item, cost)
    });
    let tiles = grid
        .tiles
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let mut tile = HerbTile {
                stake: forage::stake(grid, i).map_or(-1, |s| s as i8),
                ..Default::default()
            };
            match (t.pulled, t.cell) {
                (
                    true,
                    Cell::Plant {
                        item, count, bloom, ..
                    },
                ) => {
                    tile.state = 1;
                    tile.item = item;
                    tile.count = count;
                    tile.bloom = bloom;
                    tile.part = herbs::part_of(item).to_owned();
                }
                (true, Cell::Weed) => tile.state = 2,
                (true, Cell::Thorn) => tile.state = 3,
                (false, cell) if over.is_some() => {
                    tile.missed = true;
                    match cell {
                        Cell::Plant { item, .. } => {
                            tile.state = 1;
                            tile.item = item;
                            tile.part = herbs::part_of(item).to_owned();
                        }
                        Cell::Weed => tile.state = 2,
                        Cell::Thorn => tile.state = 3,
                    }
                }
                (false, cell) => {
                    tile.known = grid.known[i];
                    if matches!(cell, Cell::Plant { item, .. } if picks(c, item) >= h.reveal_at[2])
                    {
                        tile.known = 1;
                    }
                }
            }
            tile
        })
        .collect();
    let level = u32::from(c.herbalist_level);
    let key = &herbs::zones()[open.zone].key;
    let known = herbs::plants_in(key)
        .into_iter()
        .filter(|p| p.level <= level && (!p.timed || p.bloom.is_some_and(|b| b.holds(minute))))
        .filter(|p| picks(c, p.item) >= h.reveal_at[0])
        .map(|p| {
            let rows: Vec<u8> = (0..grid.size)
                .filter(|&row| {
                    (0..grid.size).any(|col| {
                        matches!(grid.tiles[row * grid.size + col].cell,
                            Cell::Plant { item, .. } if item == p.item)
                    })
                })
                .map(|row| row as u8 + 1)
                .collect();
            HerbKnown {
                item: p.item,
                present: !rows.is_empty(),
                rows: if picks(c, p.item) >= h.reveal_at[1] {
                    rows
                } else {
                    Vec::new()
                },
            }
        })
        .collect();
    let in_bloom = herbs::plants_in(key)
        .into_iter()
        .filter(|p| p.rarity != herbs::Rarity::Legendary && p.level <= level)
        .filter(|p| p.bloom.is_some_and(|b| b.holds(minute)))
        .map(|p| p.item)
        .collect();
    SessionEvent::HerbGrid {
        patch: open.patch,
        size: grid.size as u8,
        pulls_left: grid.pulls_left as u8,
        tiles,
        known,
        notes: open.notes.clone(),
        log,
        tool: tool.map_or(0, |(item, _)| item),
        skill_cost: tool.map_or(0, |(_, cost)| cost as u8),
        consumable: open.consumable,
        consumable_ready: grid.ready,
        charm_armed: grid.charm_armed,
        time_block: Block::of(minute).name().to_owned(),
        grown_for: open.grown_for,
        in_bloom,
        bloom_factor: t.bloom_factor,
        basket: open.basket.clone(),
        thorns: open.thorns.min(u32::from(u8::MAX)) as u8,
        xp: open.xp,
        over,
    }
}
