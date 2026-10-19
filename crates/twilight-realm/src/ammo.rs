//! Ammo (#157), as in vanilla: the arrows or bullets stay in the bags, and `PLAYER_AMMO_ID` names
//! the entry the ranged weapon shoots (ADR 0007's Ammo Gear Slot). A bow or crossbow shoots
//! arrows, a gun bullets, a thrown weapon itself. Each ranged attack spends one, and its DPS times
//! the weapon's speed adds to the shot (vmangos `Player::UpdateDamagePhysical(RANGED_ATTACK)`).

use benilla_protocol::SessionEvent;

use crate::characters::Char;
use crate::fields::values;
use crate::items;

/// `PLAYER_AMMO_ID`: the set ammo's entry, 0 for none.
pub(crate) const PLAYER_AMMO_ID: u16 = 1223;
/// `ITEM_CLASS_PROJECTILE`.
const CLASS_PROJECTILE: u32 = 6;
/// `ItemSubclassProjectile`.
const ARROW: u32 = 2;
const BULLET: u32 = 3;
/// `EQUIP_INVENTORY_SLOT_RANGED`.
const SLOT_RANGED: u8 = 17;
/// `EQUIP_ERR_ONLY_AMMO_CAN_GO_HERE`: no ammo, or not the kind the ranged weapon shoots.
const EQUIP_ERR_ONLY_AMMO: u8 = 7;
/// `EQUIP_ERR_ITEM_NOT_FOUND`: none of it in the bags.
const EQUIP_ERR_ITEM_NOT_FOUND: u8 = 23;

/// The ammo subclass the worn ranged weapon shoots: arrows for a bow (2) or crossbow (18),
/// bullets for a gun (3); `None` without one, or for a thrown weapon, which needs none.
fn wanted(c: &Char) -> Option<u32> {
    let worn = c.inventory.get(SLOT_RANGED)?;
    match items::template(worn.entry)?.subclass {
        2 | 18 => Some(ARROW),
        3 => Some(BULLET),
        _ => None,
    }
}

/// Whether a ranged attack needs ammo: the worn weapon shoots arrows or bullets.
pub(crate) fn needed(c: &Char) -> bool {
    wanted(c).is_some()
}

/// The set ammo, when the bags hold some and the ranged weapon shoots its kind.
fn loaded(c: &Char) -> Option<&'static items::Template> {
    let entry = c.inventory.ammo;
    let t = items::template(entry).filter(|_| entry != 0)?;
    (Some(t.subclass) == wanted(c) && c.inventory.count(entry) > 0).then_some(t)
}

/// Whether a ranged attack may go: no ammo needed, or some set that fits.
pub(crate) fn ready(c: &Char) -> bool {
    !needed(c) || loaded(c).is_some()
}

/// What the loaded ammo adds to the ranged shot, in damage per second: the middle of its roll.
pub(crate) fn dps(c: &Char) -> f32 {
    loaded(c)
        .and_then(|t| t.damage.first())
        .map_or(0.0, |&[min, max, _]| (min + max) / 2.0)
}

/// Whether `entry` is ammo the ranged weapon shoots; with none worn, any arrows or bullets.
fn fits(c: &Char, entry: u32) -> bool {
    items::template(entry)
        .is_some_and(|t| t.class == CLASS_PROJECTILE && wanted(c).is_none_or(|w| w == t.subclass))
}

/// `CMSG_SET_AMMO`: set `entry` as the ammo, if the bags hold it and the ranged weapon shoots its
/// kind (with none worn, any arrows or bullets go). Whether it changed.
pub(crate) fn set(c: &mut Char, entry: u32, events: &mut Vec<SessionEvent>) -> bool {
    if !fits(c, entry) {
        events.push(crate::equipment::inventory_failure(EQUIP_ERR_ONLY_AMMO, 0));
        return false;
    }
    if c.inventory.count(entry) == 0 {
        events.push(crate::equipment::inventory_failure(
            EQUIP_ERR_ITEM_NOT_FOUND,
            0,
        ));
        return false;
    }
    if c.inventory.ammo == entry {
        return false;
    }
    c.inventory.ammo = entry;
    events.push(values(c.guid, &[(PLAYER_AMMO_ID, entry)]));
    true
}

/// [`set`] for ammo just received, refusing nothing: anything but arrows or bullets the ranged
/// weapon shoots is left alone.
pub(crate) fn set_quietly(c: &mut Char, entry: u32, events: &mut Vec<SessionEvent>) {
    if fits(c, entry) && c.inventory.count(entry) > 0 {
        c.inventory.ammo = entry;
        events.push(values(c.guid, &[(PLAYER_AMMO_ID, entry)]));
    }
}

/// Spend `n` of the loaded ammo, one per ranged attack. The last one spent clears the ammo.
/// Whether it ran out, so the sheet's ranged damage must drop its DPS.
pub(crate) fn spend(c: &mut Char, n: u32, events: &mut Vec<SessionEvent>) -> bool {
    let Some(t) = loaded(c) else {
        return false;
    };
    let n = n.min(c.inventory.count(t.entry));
    crate::equipment::take(c, t.entry, n, events);
    if c.inventory.count(t.entry) > 0 {
        return false;
    }
    c.inventory.ammo = 0;
    events.push(values(c.guid, &[(PLAYER_AMMO_ID, 0)]));
    true
}
