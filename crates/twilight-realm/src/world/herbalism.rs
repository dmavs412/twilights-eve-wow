//! Herbalism (#438, ADR 0024): Brother Ansel teaches it, each character who knows it has Herb
//! Patches of their own standing in the herb zones, and Find Herbs draws them on the minimap.

use std::collections::HashMap;

use benilla_protocol::messages::GossipOption;
use benilla_protocol::SessionEvent;

use crate::audience::Audience;
use crate::characters::Char;
use crate::content::EASTERN_KINGDOMS;
use crate::equipment;
use crate::herbs;
use crate::hub::Npc;
use crate::objects::{ObjectKind, ObjectSpec};
use crate::Host;

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

/// One character's patches: those standing, and the replacements waiting on their delay.
#[derive(Default)]
struct Patches {
    live: Vec<Live>,
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

    /// A patch is finished (a test seam until the grid lands in #439): it goes, and its
    /// replacement is queued `respawn_secs` on.
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

    /// Logging out takes the character's patches with it.
    pub(crate) fn forget_herb_patches(&mut self, guid: u64) {
        if let Some(p) = self.herb_patches.0.remove(&guid) {
            for l in p.live {
                self.objects.remove(l.guid);
            }
        }
    }
}
