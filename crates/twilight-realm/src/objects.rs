//! GameObjects made at run time for an Audience (ADR 0014): a thing to use for a quest's use
//! objective, or a chest already filled. The realm keeps them; each session streams in the ones
//! its character is admitted to (`InWorld::show_runtime_objects`).

use crate::audience::Audience;
use crate::looting::Loot;
use crate::phased::{
    gameobject_create, GameObject, CHEST, FISHINGNODE, GENERIC, HIGHGUID_GAMEOBJECT,
};
use benilla_protocol::SessionEvent;

/// The first low guid of a GameObject made at run time, above every content row's (790000 to
/// 799999).
const FIRST_GUID: u64 = 800_000;

/// Vanilla's small wooden chest in `GameObjectDisplayInfo`.
pub const CHEST_DISPLAY: u32 = 259;

/// What a run-time GameObject is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectKind {
    /// A click counts for a quest's `use_object` objective by `entry`, then it is gone for
    /// everyone.
    Use,
    /// A chest filled with these item entries, one of each, and no money. A click opens the
    /// stock loot window; taking the last item removes it.
    Chest(Vec<u32>),
    /// A Fishing cast's bobber (ADR 0018), floating on the water: the client draws the line to
    /// it. A click does nothing; the Reeling Bar is the catch.
    Bobber,
}

/// Where and what a run-time GameObject is.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectSpec {
    pub entry: u32,
    pub display: u32,
    pub kind: ObjectKind,
    pub map: u32,
    pub position: [f32; 3],
    pub orientation: f32,
}

/// One run-time GameObject.
pub(crate) struct Object {
    pub(crate) guid: u64,
    pub(crate) spec: ObjectSpec,
    pub(crate) audience: Audience,
    /// What is left in a chest.
    pub(crate) loot: Option<Loot>,
}

impl Object {
    /// Its create, at the grounded position it was spawned at.
    pub(crate) fn create(&self) -> SessionEvent {
        gameobject_create(&GameObject {
            guid: self.guid,
            entry: self.spec.entry,
            display_id: self.spec.display,
            type_id: match self.spec.kind {
                ObjectKind::Use => GENERIC,
                ObjectKind::Chest(_) => CHEST,
                ObjectKind::Bobber => FISHINGNODE,
            },
            flags: 0,
            faction: 0,
            position: self.spec.position,
            orientation: self.spec.orientation,
        })
    }
}

/// Every run-time GameObject in the realm.
#[derive(Default)]
pub(crate) struct Objects {
    list: Vec<Object>,
    made: u64,
}

impl Objects {
    /// Adds an object standing at `spec.position` (already grounded); its guid.
    pub(crate) fn spawn(&mut self, spec: ObjectSpec, audience: Audience) -> u64 {
        let guid = HIGHGUID_GAMEOBJECT << 48 | (FIRST_GUID + self.made);
        self.made += 1;
        let loot = match &spec.kind {
            ObjectKind::Use | ObjectKind::Bobber => None,
            ObjectKind::Chest(entries) => Some(Loot {
                money: 0,
                items: entries.iter().map(|&e| Some((e, 1))).collect(),
            }),
        };
        self.list.push(Object {
            guid,
            spec,
            audience,
            loot,
        });
        guid
    }

    pub(crate) fn get(&self, guid: u64) -> Option<&Object> {
        self.list.iter().find(|o| o.guid == guid)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Object> {
        self.list.iter()
    }

    pub(crate) fn remove(&mut self, guid: u64) {
        self.list.retain(|o| o.guid != guid);
    }

    /// Removes every object made for exactly `audience`.
    pub(crate) fn despawn_audience(&mut self, audience: &Audience) {
        self.list.retain(|o| o.audience != *audience);
    }

    /// Records what a session left in chest `guid`.
    pub(crate) fn set_loot(&mut self, guid: u64, loot: Loot) {
        if let Some(o) = self.list.iter_mut().find(|o| o.guid == guid) {
            o.loot = Some(loot);
        }
    }
}
