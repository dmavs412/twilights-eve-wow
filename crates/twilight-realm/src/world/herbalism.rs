//! Herbalism (#438, ADR 0024): Brother Ansel teaches it, each character who knows it has Herb
//! Patches of their own standing in the herb zones, and Find Herbs draws them on the minimap.

use std::collections::HashMap;

use benilla_protocol::messages::GossipOption;
use benilla_protocol::SessionEvent;

use crate::audience::Audience;
use crate::characters::Char;
use crate::content::EASTERN_KINGDOMS;
use crate::equipment;
use crate::fields::values;
use crate::herbs;
use crate::hub::Npc;
use crate::objects::{ObjectKind, ObjectSpec};
use crate::rules::forage::{self, Cell};
use crate::rules::game_clock;
use crate::space;
use crate::{HerbAction, Host};
use benilla_protocol::{HerbKnown, HerbTile};

/// `HerbGridClosed` reasons.
const CLOSED_OUT_OF_PULLS: u8 = 0;
const CLOSED_EVERY_TILE: u8 = 1;
const CLOSED_LEFT: u8 = 2;
const CLOSED_DAMAGE: u8 = 3;
const CLOSED_WALKED_AWAY: u8 = 4;

/// Brother Ansel's menu lines: the merchant window and the lesson.
const OPTION_BROWSE: u32 = 1;
const OPTION_TEACH: u32 = 2;
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
    }
}

/// `HerbTracking`: whether Find Herbs is on, for login and every toggle.
pub(crate) fn tracking(c: &Char) -> SessionEvent {
    SessionEvent::HerbTracking {
        patch_entry: herbs::patch().entry,
        on: c.herb_tracking,
    }
}

/// Brother Ansel's gossip lines for `c`: his goods, and the lesson while they do not know
/// Herbalism.
pub(crate) fn herbalist_options(c: &Char) -> Vec<GossipOption> {
    let mut options = vec![GossipOption {
        index: OPTION_BROWSE,
        icon: ICON_VENDOR,
        coded: false,
        message: "Let me browse your goods.".into(),
    }];
    if !c.knows_herbalism {
        options.push(GossipOption {
            index: OPTION_TEACH,
            icon: ICON_TRAINER,
            coded: false,
            message: "Teach me Herbalism.".into(),
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
    /// A line of Brother Ansel's menu: his goods open the merchant window; the lesson teaches
    /// Herbalism. Both close the menu.
    pub(crate) fn herbalist_select(&mut self, npc: &Npc, option: u32) {
        if !self.near(npc) || !matches!(option, OPTION_BROWSE | OPTION_TEACH) {
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
        if c.knows_herbalism {
            return;
        }
        c.knows_herbalism = true;
        if let Err(reason) = equipment::receive(c, SICKLE, 1, &mut self.events) {
            self.events.push(equipment::inventory_failure(reason, 0));
        }
        let spell = herbs::spell();
        self.events.push(SessionEvent::SpellLearned {
            spell_id: spell.learn,
        });
        self.events.push(SessionEvent::SpellLearned {
            spell_id: spell.find,
        });
        self.events.push(herbalist(c));
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
    /// they know no Herbalism, a grid is already open, or the patch is not one of theirs.
    pub(crate) fn open_herb_grid(&mut self, patch: u64, zone: usize) {
        let Some(w) = self.world.as_mut() else {
            return;
        };
        let guid = w.guid;
        let Some(c) = self.characters.get(guid) else {
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
        let grid = forage::roll(
            &mut self.rng,
            &herbs::plants_in(&z.key),
            u32::from(c.herbalist_level),
            game_clock::minute_of_day(self.wall),
            tuning,
            z.thorns.unwrap_or(tuning.thorns),
        );
        w.hurt = false;
        let open = Open { patch, zone, grid };
        self.events.push(grid_event(
            c,
            &open,
            game_clock::minute_of_day(self.wall),
            String::new(),
        ));
        p.open = Some(open);
    }

    /// What the served character does on their open grid.
    pub(crate) fn herb_action(&mut self, action: HerbAction) {
        let Some(guid) = self.world.as_ref().map(|w| w.guid) else {
            return;
        };
        match action {
            HerbAction::Leave => self.close_herb_grid(guid, CLOSED_LEFT),
            HerbAction::Pull(tile) => self.pull_herb(guid, usize::from(tile)),
        }
    }

    /// Pulls `tile` of `guid`'s open grid: the plant into the Satchel, the XP and pick count, the
    /// new grid sent, and the grid closed when it is out of pulls or tiles.
    fn pull_herb(&mut self, guid: u64, tile: usize) {
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
        let Some(pulled) = forage::pull(&mut open.grid, tile) else {
            return;
        };
        let h = herbs::herbalist();
        let size = open.grid.size;
        let name = format!(
            "{}{}",
            (b'A' + (tile % size) as u8) as char,
            tile / size + 1
        );
        let key = &herbs::zones()[open.zone].key;
        let level = u32::from(c.herbalist_level);
        let (xp, log) = match pulled.cell {
            Cell::Plant {
                item, count, money, ..
            } => {
                let row = herbs::plants_in(key).into_iter().find(|p| p.item == item);
                let plant_level = row.map_or(1, |p| p.level);
                let base = h.plant_xp_base + h.plant_xp_per_level * plant_level;
                let xp = (base as f32 * h.colour_factor(level.saturating_sub(plant_level))).round()
                    as u32;
                match c.herb_picks.iter_mut().find(|(i, _)| *i == item) {
                    Some(pick) => pick.1 += 1,
                    None => c.herb_picks.push((item, 1)),
                }
                let item_name = crate::items::template(item).map_or("a plant", |t| t.name.as_str());
                let log = if let Some([lo, hi]) = money {
                    let copper = self.rng.urand(lo, hi);
                    c.inventory.money += copper;
                    self.events
                        .push(values(c.guid, &[equipment::money_pair(&c.inventory)]));
                    format!("Pulled {name}: {item_name}, {copper} copper.")
                } else {
                    if let Err(reason) = equipment::receive(c, item, count, &mut self.events) {
                        self.events.push(equipment::inventory_failure(reason, 0));
                    }
                    format!("Pulled {name}: {item_name} x{count}.")
                };
                (xp, log)
            }
            Cell::Weed => (h.weed_xp, format!("Pulled {name}: a weed.")),
            Cell::Thorn => (
                h.thorn_xp,
                format!("Pulled {name}: a thorn. It cost {} pulls.", pulled.cost),
            ),
        };
        let levelled = gain_herbalist_xp(c, xp);
        if xp > 0 || levelled {
            self.events.push(herbalist(c));
        }
        let minute = game_clock::minute_of_day(self.wall);
        let end = match self.herb_patches.0.get(&guid).and_then(|p| p.open.as_ref()) {
            Some(open) => {
                self.events.push(grid_event(c, open, minute, log));
                if open.grid.pulls_left == 0 {
                    Some(CLOSED_OUT_OF_PULLS)
                } else if !open.grid.any_hidden() {
                    Some(CLOSED_EVERY_TILE)
                } else {
                    None
                }
            }
            None => None,
        };
        match end {
            Some(reason) => self.close_herb_grid(guid, reason),
            None if levelled => self.save(guid),
            None => {}
        }
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
/// knows of the zone's plants.
fn grid_event(c: &Char, open: &Open, minute: u32, log: String) -> SessionEvent {
    let h = herbs::herbalist();
    let grid = &open.grid;
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
                }
                (true, Cell::Weed) => tile.state = 2,
                (true, Cell::Thorn) => tile.state = 3,
                (false, Cell::Plant { item, .. }) if picks(c, item) >= h.reveal_at[2] => {
                    tile.known = 1;
                }
                _ => {}
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
    SessionEvent::HerbGrid {
        patch: open.patch,
        size: grid.size as u8,
        pulls_left: grid.pulls_left as u8,
        tiles,
        known,
        notes: Vec::new(),
        log,
        tool: 0,
        skill_cost: 0,
        consumable: 0,
        consumable_ready: false,
        charm_armed: false,
    }
}
