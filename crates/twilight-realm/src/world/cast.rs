//! The character's skills in the world: the spellbook at login, casting them, and the auras they
//! leave. Every skill is cast from its data ([`crate::callings`]); nothing here names a skill.
//!
//! An instant cast answers as vmangos's does: `SMSG_SPELL_START`, `SMSG_CAST_RESULT` (ok),
//! `SMSG_SPELL_GO` with its hits and misses, then what each effect did (damage, energize, the aura
//! fields). A refusal is a lone failed `SMSG_CAST_RESULT` with the stock reason. An on-next-swing
//! skill answers nothing until the swing it replaces.

use benilla_protocol::messages::ObjectType;
use benilla_protocol::messages::{
    ActionButton, MonsterMoveFacing, PeriodicAuraLog, PeriodicTick, SpeedKind, SpellCooldown,
    SpellDamageLog, SpellEnergizeLog,
};
use benilla_protocol::{EntityKind, ObjectFields, SessionEvent};

use super::power::Combo;
use super::{eye, in_front, melee_reach, InWorld, Ticked};
use crate::callings::{self, CritBleedOf, Effect, IfWorn, Seal, Skill, StackRule, Target, Weapon};
use crate::characters::Char;
use crate::clock;
use crate::fields::*;
use crate::items;
use crate::mobs::State;
use crate::rng::Rng;
use crate::rules::haste::{haste_cut_damage, hasted_attack_time};
use crate::rules::melee::{Outcome, PlayerOffense, ROLL};
use crate::rules::power::PowerKind;
use crate::rules::school::{crit_scaled, mitigated, School};
use crate::rules::seals;
use crate::rules::skills::{self as formulas, ability_table};
use crate::rules::stats::{CharacterStats, OFF_HAND_DAMAGE_PCT};
use crate::rules::talents::{crit_damage, deep_wound_tick, heal_tick, scaled, scaled_share};
use crate::space::{distance, flat_distance};

/// `UNIT_FIELD_BYTES_1`: stand state, loyalty, shapeshift form, visibility flags.
const UNIT_FIELD_BYTES_1: u16 = 138;
const UNIT_FIELD_AURA: u16 = 47;
/// Four bits a slot, eight slots a dword.
const UNIT_FIELD_AURAFLAGS: u16 = 95;
/// A byte a slot: the caster's level.
const UNIT_FIELD_AURALEVELS: u16 = 101;
/// A byte a slot: stacks - 1.
const UNIT_FIELD_AURAAPPLICATIONS: u16 = 113;
/// Two i16s: the flat positive and negative attack power mods.
const UNIT_FIELD_ATTACK_POWER_MODS: u16 = 166;
const PLAYER_FIELD_COMBO_TARGET: u16 = 714;
/// Byte 1: combo points.
const PLAYER_FIELD_BYTES: u16 = 1222;
/// The aura-state bits, and the one for a unit at or below 20% health
/// (`AURA_STATE_HEALTHLESS_20_PERCENT`, bit 2 of `TargetAuraState`).
const UNIT_FIELD_AURASTATE: u16 = 125;
const AURA_STATE_HEALTHLESS_20: u32 = 0x2;

/// `AFLAG_CANCELABLE`: a buff the player may click off.
const AFLAG_CANCELABLE: u32 = 0x01;
/// `AFLAG_EFF_INDEX_0`: the slot is live.
const AFLAG_EFF_INDEX_0: u32 = 0x02;
/// Slots below this hold buffs, the rest debuffs.
const POSITIVE_SLOTS: u8 = 32;
const AURA_SLOTS: u8 = 48;

// `SpellCastResult` (vmangos `SpellDefines.h`).
const SPELL_FAILED_AFFECTING_COMBAT: u8 = 0x00;
const SPELL_FAILED_BAD_TARGETS: u8 = 0x0a;
const SPELL_FAILED_CASTER_AURASTATE: u8 = 0x12;
const SPELL_FAILED_CASTER_DEAD: u8 = 0x13;
pub(super) const SPELL_FAILED_EQUIPPED_ITEM_CLASS: u8 = 0x19;
const SPELL_FAILED_FLEEING: u8 = 0x1e;
/// No arrows or bullets for the ranged weapon (#157).
const SPELL_FAILED_NO_AMMO: u8 = 0x43;
pub(super) const SPELL_FAILED_NOT_KNOWN: u8 = 0x38;
pub(super) const SPELL_FAILED_NOT_READY: u8 = 0x3c;
const SPELL_FAILED_NO_COMBO_POINTS: u8 = 0x46;
const SPELL_FAILED_NO_POWER: u8 = 0x4d;
const SPELL_FAILED_ONLY_STEALTHED: u8 = 0x57;
const SPELL_FAILED_OUT_OF_RANGE: u8 = 0x59;
const SPELL_FAILED_TARGET_AFFECTING_COMBAT: u8 = 0x66;
const SPELL_FAILED_TARGET_AURASTATE: u8 = 0x67;
const SPELL_FAILED_TARGETS_DEAD: u8 = 0x65;
const SPELL_FAILED_TOO_CLOSE: u8 = 0x76;
const SPELL_FAILED_UNIT_NOT_INFRONT: u8 = 0x7c;

// `SpellMissInfo`, as `SMSG_SPELL_GO`'s miss list carries it.
const SPELL_MISS_MISS: u8 = 1;
const SPELL_MISS_DODGE: u8 = 3;
const SPELL_MISS_PARRY: u8 = 4;
const SPELL_MISS_BLOCK: u8 = 5;

/// `SPELL_HIT_TYPE_CRIT` in a spell damage log.
const SPELL_HIT_TYPE_CRIT: u32 = 0x2;

/// How long a dodge leaves Overpower open, and the combo point that marks it (vmangos).
const DODGE_WINDOW_SECS: f32 = 5.0;
/// A cooldown counts as over this close to its end: the client's clock and the realm's tick
/// differ by up to a frame, and the client sends the cast the moment its own clock says ready.
pub(super) const READY_SLACK_SECS: f32 = 0.1;
/// Seconds of float drift an aura clock forgives, so a tick and the fade due on the same tick
/// both land.
const CLOCK_EPSILON: f32 = 1e-3;
/// Seconds left on an aura with no duration, which lasts until something breaks it (Stealth).
const UNTIL_BROKEN_SECS: f32 = 1.0e9;
/// How far short of the target a charge stops.
const CHARGE_STOP_YD: f32 = 2.0;

/// The skills' lasting state: cooldowns, the global cooldown, the queued strike, the auras.
#[derive(Default)]
pub(super) struct Skills {
    /// `(spell, seconds left)`.
    cooldowns: Vec<(u32, f32)>,
    gcd: f32,
    /// The on-next-swing skill waiting for the next white swing.
    queued: Option<u32>,
    /// The mob that last dodged, and the seconds Overpower stays open on it.
    dodged: Option<(u64, f32)>,
    /// Whether that dodge shows the client its marker point.
    dodge_marked: bool,
    /// The combo points a builder put on a target; the finisher that lands spends them.
    pub(super) combo: Combo,
    /// Seconds the caster is held in combat (Bloodrage).
    combat_hold: f32,
    auras: Vec<Aura>,
    /// The patches on the ground, one at most for each caster.
    grounds: Vec<Ground>,
    /// The blood droplets lying about, waiting for the player to walk over them.
    drops: Vec<Drop>,
    /// The timed cast under way: an item's on-use spell (the Hearthstone's).
    casting: Option<TimedCast>,
    /// The mobs whose aura state says they are at or below a fifth of their health.
    low_health: Vec<u64>,
    /// The player's guid, for the aura packets only the player's own auras get.
    me: u64,
    /// The auras put on other players (#150), for their own sessions to wear.
    sent: Vec<Aura>,
    /// Speed changes sent to the client so far: the counter its ack echoes.
    speed_changes: u32,
    /// The aura a `consume_aura` just took, and its row's full duration in ms, while the consume
    /// procs run: what a spread copies.
    consumed: Option<(Aura, u32)>,
    /// A Blink cast and not yet landed: the next tick, which has the ground, moves the caster.
    blink: Option<PendingBlink>,
    /// Mobs a knockback cast hit and not yet moved: the next tick, which has the ground, slides them.
    knocks: Vec<PendingKnock>,
    /// Shields that broke and burst, not yet paid: the world tick pays them after the skills.
    bursts: Vec<Burst>,
    /// Seconds since the player's last step; `None` before the first.
    since_step: Option<f32>,
    /// Seconds the player has been moving without stopping, or standing still.
    moving_secs: f32,
    still_secs: f32,
    /// The player's auras that their next step takes off (`break_on=move`), by spell id.
    break_on_move: Vec<u32>,
    /// The unit target of the cast under way, for a proc that repeats it or reads the target as
    /// it stood before the cast.
    pub(super) cast_target: Option<u64>,
    /// A free cast is under way (a repeat, a `cast_skill`): it pays nothing and starts no
    /// cooldown.
    free: bool,
    /// A free repeat is under way (Twin Arrows): it repeats nothing.
    repeating: bool,
    /// The unit the player last selected (`CMSG_SET_SELECTION`); 0 for none.
    pub(super) selection: u64,
    /// `cast_skill`s cast and not yet gone off: the tick after the cast, which has the ground and
    /// lands any Blink first, casts them (Parting Shot).
    follow_ups: Vec<FollowUp>,
    /// The channel under way, if any.
    channel: Option<Channel>,
    /// The ground areas up, and the point the cast under way was aimed at.
    areas: Vec<Area>,
    dest: Option<[f32; 3]>,
    /// Patches put on the ground so far, which numbers their DynamicObjects.
    patches_made: u16,
    /// The spell-mod cells the player's `next_cast` buffs last set on the client, with values.
    sent_mods: Vec<(bool, u8, u8, i32)>,
    /// The aura whose tick is running its procs: what a spread copies.
    ticking: Option<Aura>,
    /// What the cast under way has dealt and healed so far: a share of either (`pct_of`) reads it.
    cast_dealt: u32,
    cast_healed: u32,
    /// What the cast's heals would have healed with no maximum health to stop them.
    cast_full_healed: u32,
    /// The health the last cast cost.
    pub(super) last_health_cost: u32,
    /// What the last cast cost, and whether the bar was full before it paid: its procs read them.
    pub(super) last_cost: u32,
    pub(super) was_full: bool,
    /// The last cast's target as it stood before the cast landed: whether it was snared, and the
    /// spell ids of its auras. A cast proc's `requires_target_aura` reads it.
    pub(super) before: Option<(bool, Vec<u32>)>,
    /// Diminishing returns on mobs: `(unit, kind, controls landed, seconds since the last)`.
    control_clocks: Vec<(u64, ControlKind, u32, f32)>,
}

/// The kinds of control that share a diminishing-returns clock on a mob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ControlKind {
    Stun,
    Incapacitate,
}

/// Seconds without a control after which a mob's diminishing returns reset.
const CONTROL_RESET_SECS: f32 = 15.0;

/// A mob's speed multiplier after its slows: a boss feels half of any slow.
fn felt_speed_mul(mul: f32, boss: bool) -> f32 {
    if boss && mul < 1.0 {
        1.0 - (1.0 - mul) / 2.0
    } else {
        mul
    }
}

/// The client's `SpellModOp`s a `next_cast` buff moves: the cast bar and the cost.
const SPELLMOD_CASTING_TIME: u8 = 10;
const SPELLMOD_COST: u8 = 14;

/// A ground area: who cast it, where, how wide, seconds left, the aura rows it puts on the
/// enemies and the friendly players inside, and who holds them now.
#[derive(Debug, Clone)]
struct Area {
    /// The DynamicObject that shows it.
    object: u64,
    caster: u64,
    spell_id: u32,
    centre: [f32; 3],
    radius_yd: f32,
    left: f32,
    /// Seconds before anyone counts as inside.
    arming: f32,
    /// Only the first enemy that is not a boss is caught, and the area ends with it.
    first_only: bool,
    enemy_aura: Option<&'static callings::AuraDef>,
    ally_aura: Option<&'static callings::AuraDef>,
    inside: Vec<u64>,
    /// The caster's damage over time each enemy inside that wears it gains a stack of, the
    /// seconds between, and until the next (Breeding Ground).
    grow: Option<(u32, f32, f32)>,
    /// Centred on the caster at each check rather than where it was cast; `centre` is then
    /// where its DynamicObject stands (Shroud of Rot).
    follow: bool,
}

/// How far the caster walks from a following area's DynamicObject before it is shown again at
/// their feet: the object itself cannot move.
const FOLLOW_RESHOW_YD: f32 = 2.0;

/// A channelled skill under way: its spell and target, seconds left, and its tick clock.
#[derive(Debug, Clone, Copy)]
struct Channel {
    spell_id: u32,
    target: u64,
    left: f32,
    period: f32,
    next_tick: f32,
    /// A step ends it: its skill is interrupted on `move` (not Unbroken Siphon's).
    step_ends: bool,
}

/// Where a Blink starts, the way it faces, and how far it goes.
#[derive(Debug, Clone, Copy)]
struct PendingBlink {
    from: [f32; 3],
    /// The way it goes, and the way the caster faces after.
    orientation: f32,
    faces: f32,
    distance_yd: f32,
}

/// A `cast_skill` waiting for its cast's Blink to land: the plain skill's spell id, whose version
/// set on its row goes off, and the unit it goes off at.
#[derive(Debug, Clone, Copy)]
struct FollowUp {
    skill_spell_id: u32,
    target: Option<u64>,
}

/// A mob a knockback hit, where the caster stood, and how far it slides.
#[derive(Debug, Clone, Copy)]
struct PendingKnock {
    mob: u64,
    from: [f32; 3],
    distance_yd: f32,
}

/// How fast a knocked mob slides, in yards a second.
const KNOCKBACK_SPEED_YD_S: f32 = 20.0;

/// An aura row's most stacks: its `max_stacks` with `stack_rule=stack`, one otherwise.
fn stack_limit(def: &callings::AuraDef) -> u32 {
    if def.row.stack_rule.as_deref() == Some("stack") {
        def.row.max_stacks.max(1)
    } else {
        1
    }
}

/// How long after a step the player still counts as moving: the client reports a moving
/// player at least every half second.
const MOVING_GRACE_SECS: f32 = 0.75;

/// How far a Blink steps between ground checks, and the most the ground may rise or fall over one
/// step before the Blink stops short.
const BLINK_STEP_YD: f32 = 1.0;
const BLINK_MAX_CLIMB_YD: f32 = 1.5;

/// A cast with a cast bar: an item's, or a skill's.
#[derive(Debug, Clone, Copy)]
struct TimedCast {
    spell_id: u32,
    /// The item it is cast from, and that item's template entry; 0 for a skill.
    item: u64,
    entry: u32,
    /// A skill's cast, at this target, rather than an item's.
    skill: Option<Option<u64>>,
    /// Seconds until it goes off.
    left: f32,
}

/// A blood droplet on the ground (`bleed_drop`): the player who walks within `radius_yd` takes the
/// `grants` aura, and it fades after `left` seconds.
#[derive(Debug, Clone, Copy)]
struct Drop {
    /// The DynamicObject that shows it.
    object: u64,
    spell_id: u32,
    centre: [f32; 3],
    radius_yd: f32,
    left: f32,
    grants: &'static str,
}

/// A patch on the ground (a `GroundEffect`): each tick it strikes every enemy inside it.
#[derive(Debug, Clone, Copy)]
struct Ground {
    /// The DynamicObject that shows it.
    object: u64,
    caster: u64,
    /// The spell its DynamicObject and its damage are logged under.
    spell_id: u32,
    centre: [f32; 3],
    radius_yd: f32,
    school: School,
    /// What each enemy inside takes a tick, talents folded in.
    damage: u32,
    /// Seconds left, between ticks, and until the next.
    left: f32,
    period: f32,
    next_tick: f32,
    /// The aura row the caster wears while they stand inside (Consecrated Ground), and whether
    /// they stood inside at the last look.
    caster_aura: Option<&'static callings::AuraDef>,
    caster_inside: bool,
}

impl Aura {
    /// The unit that wears it.
    pub(crate) fn unit(&self) -> u64 {
        self.unit
    }

    pub(crate) fn spell_id(&self) -> u32 {
        self.spell_id
    }
}

impl Skills {
    /// Whether `position` is inside a patch `caster` left on the ground.
    pub(super) fn in_ground_effect(&self, caster: u64, position: [f32; 3]) -> bool {
        self.grounds
            .iter()
            .any(|g| g.caster == caster && distance(g.centre, position) <= g.radius_yd)
    }

    /// A new patch's DynamicObject guid: the caster's low guid and a count, so no two sessions'
    /// patches share one, and a re-cast is a new object.
    fn next_patch(&mut self, caster: u64) -> u64 {
        self.patches_made = self.patches_made.wrapping_add(1);
        HIGHGUID_DYNAMICOBJECT << 48 | (caster & 0xFFFF_FFFF) << 16 | u64::from(self.patches_made)
    }

    /// Every patch on the ground as its DynamicObject's guid, centre and create: what another
    /// client in view of it is sent.
    pub(super) fn patches(&self) -> Vec<(u64, [f32; 3], SessionEvent)> {
        let grounds = self
            .grounds
            .iter()
            .map(|g| (g.object, g.caster, g.spell_id, g.centre, g.radius_yd));
        let areas = self
            .areas
            .iter()
            .map(|a| (a.object, a.caster, a.spell_id, a.centre, a.radius_yd));
        let drops = self
            .drops
            .iter()
            .map(|d| (d.object, self.me, d.spell_id, d.centre, d.radius_yd));
        grounds
            .chain(areas)
            .chain(drops)
            .map(|(guid, caster, spell_id, centre, radius_yd)| {
                let create = patch_create(guid, caster, spell_id, centre, radius_yd);
                (guid, centre, create)
            })
            .collect()
    }
}

/// `HIGHGUID_DYNAMICOBJECT`, `TYPEMASK_OBJECT | TYPEMASK_DYNAMICOBJECT`, and the
/// `DYNAMICOBJECT_*` fields (`UpdateFields_1_12_1.h`).
const HIGHGUID_DYNAMICOBJECT: u64 = 0xF100;
const TYPE_DYNAMICOBJECT: u32 = 0x01 | 0x40;
const DYNAMICOBJECT_CASTER: u16 = 6;
const DYNAMICOBJECT_BYTES: u16 = 8;
const DYNAMICOBJECT_SPELLID: u16 = 9;
const DYNAMICOBJECT_RADIUS: u16 = 10;
const DYNAMICOBJECT_POS_X: u16 = 11;
/// `DYNAMIC_OBJECT_AREA_SPELL`, the bytes' type.
const DYNAMIC_OBJECT_AREA_SPELL: u32 = 1;

/// The DynamicObject that shows a patch on the ground, its fields as vmangos's
/// `DynamicObject::Create` fills them: the client draws `spell_id`'s area visual at `centre`.
fn patch_create(
    guid: u64,
    caster: u64,
    spell_id: u32,
    centre: [f32; 3],
    radius_yd: f32,
) -> SessionEvent {
    let [x, y, z] = centre;
    let mut pairs = vec![
        (OBJECT_FIELD_TYPE, TYPE_DYNAMICOBJECT),
        (OBJECT_FIELD_ENTRY, spell_id),
        (OBJECT_FIELD_SCALE_X, float(1.0)),
        (DYNAMICOBJECT_BYTES, DYNAMIC_OBJECT_AREA_SPELL),
        (DYNAMICOBJECT_SPELLID, spell_id),
        (DYNAMICOBJECT_RADIUS, float(radius_yd)),
        (DYNAMICOBJECT_POS_X, float(x)),
        (DYNAMICOBJECT_POS_X + 1, float(y)),
        (DYNAMICOBJECT_POS_X + 2, float(z)),
    ];
    pairs.extend(guid_pairs(OBJECT_FIELD_GUID, guid));
    pairs.extend(guid_pairs(DYNAMICOBJECT_CASTER, caster));
    SessionEvent::ObjectCreate {
        guid,
        kind: EntityKind::DynamicObject,
        display_id: None,
        position: centre,
        orientation: 0.0,
        scale: 1.0,
        speeds: None,
        mover: None,
        transport_progress: None,
        transport: None,
        spline: None,
        fields: ObjectFields::from_pairs(&pairs).into_created(ObjectType::DynamicObject),
    }
}

/// One aura on a unit.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Aura {
    unit: u64,
    spell_id: u32,
    slot: u8,
    level: u8,
    /// Seconds left.
    left: f32,
    /// Seconds between ticks, 0 for none, and until the next.
    period: f32,
    next_tick: f32,
    kind: AuraKind,
    /// Uses left before the aura is spent; 0 for no limit.
    charges: u32,
}

/// A damage over time's riders (#506, #507): after each tick, `aura_chance` percent of the time
/// `aura` goes on the caster, or on the ticking unit when `aura_on_target`, and the caster gains
/// `energize_pct_of_max` percent of their maximum power and `energize_pct_of_base` percent of
/// their base mana; while it is on a unit, that unit's blows at the caster are `weaken_pct`
/// percent less. Acolyte's (#508): `aura_chance_per_stack` times the tick's stacks in place of
/// `aura_chance`; `spread_chance` percent of the time a copy goes on another enemy within
/// `spread_yd`; while it is on a unit, that unit's hits at anyone are `weaken_per_stack_pct` a
/// stack less, at most `weaken_max_pct`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct TickRiders {
    aura: Option<&'static callings::AuraDef>,
    aura_chance: u32,
    aura_chance_per_stack: f32,
    aura_on_target: bool,
    energize_pct_of_max: u32,
    energize_pct_of_base: u32,
    weaken_pct: u32,
    spread_chance: u32,
    spread_yd: f32,
    weaken_per_stack_pct: u32,
    weaken_max_pct: u32,
}

/// What a shield of the player's does besides soaking (#506): `energize_pct` percent of each soak
/// back as power, `attacker_aura` on the enemy whose damage it soaks, and when it breaks, its whole
/// `amount` in `burst_school` at every enemy within `burst_radius_yd`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(super) struct Ward {
    amount: u32,
    energize_pct: u32,
    attacker_aura: Option<&'static callings::AuraDef>,
    burst_radius_yd: f32,
    burst_school: School,
}

/// What one of the player's shields soaked of a blow: its spell, level and riders, how much, and
/// whether that broke it.
#[derive(Debug, Clone, Copy)]
pub(super) struct Soak {
    spell_id: u32,
    level: u8,
    soaked: u32,
    ward: Ward,
    broke: bool,
}

/// A broken shield's burst, waiting for the world tick to pay it with the player's position.
#[derive(Debug, Clone, Copy)]
struct Burst {
    spell_id: u32,
    amount: u32,
    radius_yd: f32,
    school: School,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AuraKind {
    /// `per_tick` damage of `school` a tick, from `caster`: a bleed, or a creature's curse on
    /// the player.
    PeriodicDamage {
        per_tick: u32,
        /// The first ticks that deal one more, so a total that does not divide still adds up.
        carry: u32,
        school: School,
        caster: u64,
        /// Applications stacked so far, each tick dealing `per_tick` once for each, and the most.
        stacks: u32,
        max_stacks: u32,
        /// Percent of each tick's damage that heals its caster.
        leech_pct: u32,
        /// `Replace` starts the aura over at the new application's stacks; any other rule adds.
        rule: StackRule,
        /// Stacks gained after each tick deals its damage.
        stacks_per_tick: u32,
        /// The percent chance a tick spills the caster's blood droplet, in place of their
        /// Calling's own.
        drop_chance: Option<u32>,
        /// What each tick gives the caster, and how much less the unit hits them for.
        riders: TickRiders,
    },
    Rage {
        per_tick: u32,
    },
    AttackPower(u32),
    AttackSlow {
        pct: u32,
    },
    MoveSlow {
        pct: u32,
    },
    Stun,
    /// The player flees a creature's fear (`fear.rs` moves them); this is the debuff the client
    /// shows. Damage does not break it.
    Fear,
    /// The unit cannot move; it still attacks and casts, and damage does not break it.
    Root,
    /// A stun that damage ends when `break_on_damage` (Gouge).
    Incapacitate {
        break_on_damage: bool,
    },
    /// Healing on the unit is cut (Mortal Strike's wound). Nothing heals a mob in a fight yet,
    /// so the debuff is all it does.
    HealingReduced,
    /// The player's maximum health is `pct`% higher (Last Stand).
    MaxHealthPct(u32),
    /// The player deals `pct`% more physical damage, with `armor_loss_pct`% less armour; for
    /// `swings` more white swings when it counts them (Death Wish, Enrage).
    DamagePct {
        pct: u32,
        armor_loss_pct: u32,
        swings: Option<u32>,
    },
    /// The player's next `strikes` landed melee blows also strike another enemy within
    /// `radius_yd` (Sweeping Strikes).
    Sweeping {
        strikes: u32,
        radius_yd: f32,
        strike_spell_id: u32,
    },
    /// The player heals `per_tick` a tick (Blood Craze).
    Heal {
        per_tick: u32,
    },
    /// The player's one active Seal, and the attack power it adds (Seal of the Crusader's, sized
    /// when cast).
    Seal {
        seal: Seal,
        attack_power: u32,
    },
    /// The unit takes `pct`% more holy damage (Judgement of the Crusader); the player's landed
    /// swings at it put it back to its full `duration_ms`.
    HolyTaken {
        pct: u32,
        duration_ms: u32,
    },
    /// The player's `skill` has `pct`% more critical strike chance (a buff from a `proc`).
    SkillCrit {
        skill: &'static str,
        pct: u32,
    },
    /// The player runs `pct`% faster (a buff from a `proc`).
    RunSpeed {
        pct: u32,
    },
    /// The player's dodge chance is `pct` percent higher.
    Dodge {
        pct: u32,
    },
    /// A shield on the player that soaks `left` more damage, of `school` only when set.
    Absorb {
        left: u32,
        school: Option<School>,
        /// What it does as it soaks and when it breaks.
        ward: Ward,
    },
    /// The player's casts are `pct` percent faster; `bits` carry it to the client's cast bars.
    CastSpeed {
        pct: u32,
        bits: &'static [u8],
    },
    /// The unit takes `pct` percent more `school` damage from others than the aura's caster, less
    /// when `pct` is negative; `None` is every school.
    DamageTaken {
        pct: i32,
        school: Option<School>,
    },
    /// The unit's chance to hit is `pct` points higher (lower when negative).
    HitChance {
        pct: i32,
    },
    /// The healing the unit receives is `pct` percent higher.
    HealingTaken {
        pct: u32,
    },
    /// The amounts the player's `skill` deals are `pct` percent higher (a buff from a `proc`).
    SkillAmount {
        skill: &'static str,
        pct: u32,
        /// Applications so far, each adding `pct`, and the most (the row's `max_stacks`).
        stacks: u32,
        max_stacks: u32,
    },
    /// The player deals `pct` percent more physical damage for each stack (Bloodlust).
    DamageDone {
        pct: u32,
        stacks: u32,
        max_stacks: u32,
    },
    /// The player's melee swings are `pct` percent faster for each stack (Bloodlust).
    MeleeHaste {
        pct: u32,
        stacks: u32,
        max_stacks: u32,
    },
    /// The player's ranged weapon shoots `pct` percent faster.
    RangedHaste {
        pct: u32,
    },
    /// The player's ranged attacks crit `pct` percent more often.
    RangedCrit {
        pct: u32,
    },
    /// The player's non-physical skills deal `pct` percent more damage.
    SpellDamagePct {
        pct: u32,
    },
    /// A buff's proc, live while the buff is (Shield of the Penitent's burst).
    Proc(&'static callings::AuraProc),
    /// The player's next cast of a skill changes; the buffs of one `group` go together.
    NextCast {
        next: &'static callings::NextCast,
        group: Option<&'static str>,
        /// Applications so far, each adding the buff's amounts again, and the most.
        stacks: u32,
        max_stacks: u32,
    },
    /// The unit's armour is `amount` lower for each stack (Sundered).
    ArmorLoss {
        amount: u32,
        stacks: u32,
        max_stacks: u32,
    },
    /// Each cast of `skill` also strikes for `weapon_pct` of a `weapon` roll (Rapid Assault).
    ExtraStrike {
        skill: &'static str,
        weapon: Weapon,
        weapon_pct: u32,
    },
    /// A count that grows with each new application; at `max` the aura row `then` replaces it.
    StackTo {
        count: u32,
        max: u32,
        then: &'static str,
    },
    /// The player's `skill` costs `amount` less power (a buff from a `proc`).
    SkillCost {
        skill: &'static str,
        amount: u32,
    },
    /// The player's `skill` adds `stacks` of its aura, if more than its own (a buff from a `proc`).
    SkillStacks {
        skill: &'static str,
        stacks: u32,
    },
    /// The player's `stat` is `pct` percent higher.
    Defense {
        stat: DefenseStat,
        pct: u32,
    },
    /// The player is stealthed; `opener_crit`: their first attack from it is a critical strike.
    Stealth {
        opener_crit: bool,
    },
    /// Moving does not hold the player's `skill` (Running Fire).
    CastWhileMoving {
        skill: &'static str,
    },
    /// A count of stacks that does nothing itself; a skill that consumes it reads the count
    /// (Conviction). The aura row's `max_stacks` and `stack_rule` govern a new application.
    Stacks {
        count: u32,
        max: u32,
        rule: StackRule,
    },
    /// Does nothing: it shows a state the realm keeps elsewhere (a Heat Level, Marked, a
    /// dungeon's Noise), a buff when `helpful`, a debuff otherwise, with `stacks` on its icon.
    Marker {
        helpful: bool,
        stacks: u32,
    },
}

/// A defensive number an aura raises by a percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DefenseStat {
    /// Parry chance, in points.
    Parry,
    /// Block chance with a shield worn, in points.
    Block,
    /// The shield's Block Value, in percent of it.
    BlockValue,
    /// The armour from items, in percent of it.
    ArmorItems,
}

impl AuraKind {
    /// The aura a defensive effect (talent-free: a skill's or an aura row's) puts up, if it is one.
    fn defense(effect: &Effect) -> Option<(DefenseStat, u32, u32)> {
        match *effect {
            Effect::ParryPct { pct, duration_ms } => Some((DefenseStat::Parry, pct, duration_ms)),
            Effect::BlockPct { pct, duration_ms } => Some((DefenseStat::Block, pct, duration_ms)),
            Effect::BlockValuePct { pct, duration_ms } => {
                Some((DefenseStat::BlockValue, pct, duration_ms))
            }
            Effect::ArmorItemsPct { pct, duration_ms } => {
                Some((DefenseStat::ArmorItems, pct, duration_ms))
            }
            _ => None,
        }
    }

    /// Whether `other` is this same effect (a reapplication replaces it): the same kind, and for
    /// a skill's crit the same skill, since one spell may carry a crit effect per skill.
    fn same_effect(self, other: AuraKind) -> bool {
        match (self, other) {
            (AuraKind::SkillCrit { skill: a, .. }, AuraKind::SkillCrit { skill: b, .. }) => a == b,
            (AuraKind::Defense { stat: a, .. }, AuraKind::Defense { stat: b, .. }) => a == b,
            (AuraKind::SkillCost { skill: a, .. }, AuraKind::SkillCost { skill: b, .. }) => a == b,
            (AuraKind::SkillAmount { skill: a, .. }, AuraKind::SkillAmount { skill: b, .. }) => {
                a == b
            }
            (AuraKind::SkillStacks { skill: a, .. }, AuraKind::SkillStacks { skill: b, .. }) => {
                a == b
            }
            (AuraKind::Proc(a), AuraKind::Proc(b)) => std::ptr::eq(a, b),
            (AuraKind::NextCast { next: a, .. }, AuraKind::NextCast { next: b, .. }) => {
                std::ptr::eq(a, b)
            }
            (AuraKind::CastWhileMoving { skill: a }, AuraKind::CastWhileMoving { skill: b }) => {
                a == b
            }
            _ => std::mem::discriminant(&self) == std::mem::discriminant(&other),
        }
    }

    fn helpful(self) -> bool {
        matches!(
            self,
            AuraKind::Rage { .. }
                | AuraKind::AttackPower(_)
                | AuraKind::MaxHealthPct(_)
                | AuraKind::DamagePct { .. }
                | AuraKind::Sweeping { .. }
                | AuraKind::Heal { .. }
                | AuraKind::Seal { .. }
                | AuraKind::SkillCrit { .. }
                | AuraKind::RunSpeed { .. }
                | AuraKind::Dodge { .. }
                | AuraKind::Absorb { .. }
                | AuraKind::HealingTaken { .. }
                | AuraKind::CastSpeed { .. }
                | AuraKind::SkillAmount { .. }
                | AuraKind::DamageDone { .. }
                | AuraKind::MeleeHaste { .. }
                | AuraKind::SpellDamagePct { .. }
                | AuraKind::Defense { .. }
                | AuraKind::Proc(_)
                | AuraKind::NextCast { .. }
                | AuraKind::StackTo { .. }
                | AuraKind::SkillCost { .. }
                | AuraKind::SkillStacks { .. }
                | AuraKind::Stealth { .. }
                | AuraKind::CastWhileMoving { .. }
                | AuraKind::Stacks { .. }
                | AuraKind::Marker { helpful: true, .. }
                // Less damage taken (Consecrated Ground) is a buff; more is a debuff.
                | AuraKind::DamageTaken {
                    pct: i32::MIN..=-1,
                    ..
                }
        )
    }

    /// The stacks the aura shows on its icon.
    fn stacks(self) -> u32 {
        match self {
            AuraKind::Stacks { count, .. } => count,
            AuraKind::PeriodicDamage { stacks, .. } => stacks,
            AuraKind::StackTo { count, .. } => count,
            AuraKind::SkillAmount { stacks, .. }
            | AuraKind::DamageDone { stacks, .. }
            | AuraKind::MeleeHaste { stacks, .. }
            | AuraKind::NextCast { stacks, .. }
            | AuraKind::ArmorLoss { stacks, .. }
            | AuraKind::Marker { stacks, .. } => stacks,
            _ => 1,
        }
    }
}

impl Skills {
    pub(super) fn new(me: u64) -> Self {
        Self {
            me,
            ..Self::default()
        }
    }

    /// The flat attack power the player's buffs add.
    fn attack_power_bonus(&self) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .map(|a| match a.kind {
                AuraKind::AttackPower(n) => n,
                AuraKind::Seal { attack_power, .. } => attack_power,
                _ => 0,
            })
            .sum()
    }

    /// How much faster the player's buffs make their swings, in percent (Seal of the
    /// Crusader's).
    fn haste_pct(&self) -> u32 {
        let seal = match self.seal() {
            Some(Seal::Crusader { haste_pct, .. }) => haste_pct,
            _ => 0,
        };
        let stacked: u32 = self
            .auras
            .iter()
            .filter(|a| a.unit == self.me)
            .map(|a| match a.kind {
                AuraKind::MeleeHaste { pct, stacks, .. } => pct * stacks,
                _ => 0,
            })
            .sum();
        seal + stacked
    }

    /// The percent more holy damage `unit` takes (Judgement of the Crusader).
    fn holy_taken_pct(&self, unit: u64) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == unit)
            .map(|a| match a.kind {
                AuraKind::HolyTaken { pct, .. } => pct,
                _ => 0,
            })
            .sum()
    }

    /// A landed swing at `unit` puts its judgement back to its full duration ("Your melee
    /// strikes will refresh the spell's duration").
    fn refresh_judgement(&mut self, unit: u64) {
        for a in self.auras.iter_mut().filter(|a| a.unit == unit) {
            if let AuraKind::HolyTaken { duration_ms, .. } = a.kind {
                a.left = clock::secs(duration_ms);
            }
        }
    }

    /// The percents the player's buffs raise their maximum health by, raise the physical damage
    /// they deal by, and take off their armour.
    fn buff_pcts(&self) -> (u32, u32, u32) {
        self.auras.iter().filter(|a| a.unit == self.me).fold(
            (0, 0, 0),
            |(health, damage, armor), a| match a.kind {
                AuraKind::MaxHealthPct(pct) => (health + pct, damage, armor),
                AuraKind::DamagePct {
                    pct,
                    armor_loss_pct,
                    ..
                } => (health, damage + pct, armor + armor_loss_pct),
                AuraKind::DamageDone { pct, stacks, .. } => (health, damage + pct * stacks, armor),
                _ => (health, damage, armor),
            },
        )
    }

    /// The percent the player's buffs raise their dodge chance by.
    fn dodge_pct(&self) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .map(|a| match a.kind {
                AuraKind::Dodge { pct } => pct,
                _ => 0,
            })
            .sum()
    }

    /// The percent the player's buffs raise the damage of their non-physical skills by.
    fn spell_damage_pct(&self) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .map(|a| match a.kind {
                AuraKind::SpellDamagePct { pct } => pct,
                _ => 0,
            })
            .sum()
    }

    /// Takes the harmful aura `spell_id` off `unit`, if it is there.
    fn take_aura(
        &mut self,
        unit: u64,
        spell_id: u32,
        events: &mut Vec<SessionEvent>,
    ) -> Option<Aura> {
        self.remove_where(
            |a| a.unit == unit && a.spell_id == spell_id && !a.kind.helpful(),
            events,
        )
        .into_iter()
        .next()
    }

    /// The percent the player's buffs raise `stat` by.
    fn defense_pct(&self, stat: DefenseStat) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .map(|a| match a.kind {
                AuraKind::Defense { stat: s, pct } if s == stat => pct,
                _ => 0,
            })
            .sum()
    }

    /// Whether the player is stealthed.
    pub(crate) fn stealthed(&self) -> bool {
        self.auras
            .iter()
            .any(|a| a.unit == self.me && matches!(a.kind, AuraKind::Stealth { .. }))
    }

    /// Takes the player's stealth off (an attack, a cast, a blow taken). `Some(opener_crit)` when
    /// they were stealthed, whether the attack that broke it is a guaranteed critical strike.
    pub(super) fn break_stealth(&mut self, events: &mut Vec<SessionEvent>) -> Option<bool> {
        let me = self.me;
        let gone = self.remove_where(
            |a| a.unit == me && matches!(a.kind, AuraKind::Stealth { .. }),
            events,
        );
        gone.iter().find_map(|a| match a.kind {
            AuraKind::Stealth { opener_crit } => Some(opener_crit),
            _ => None,
        })
    }

    /// Whether `skill` is the first blow from stealth with an opener crit; a skill that breaks
    /// stealth takes it off either way.
    pub(super) fn opens_from_stealth(
        &mut self,
        skill: &Skill,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        skill.meta.shape.on_cast.breaks_stealth && self.break_stealth(events) == Some(true)
    }

    /// The percent the player's buffs raise the physical damage they deal by.
    pub(super) fn damage_pct(&self) -> u32 {
        self.buff_pcts().1
    }

    /// A white swing went: the buffs that count swings count one, and go at their last. What
    /// went.
    pub(super) fn count_swing(&mut self, events: &mut Vec<SessionEvent>) -> Vec<Aura> {
        let me = self.me;
        for a in self.auras.iter_mut().filter(|a| a.unit == me) {
            if let AuraKind::DamagePct {
                swings: Some(n), ..
            } = &mut a.kind
            {
                *n = n.saturating_sub(1);
            }
        }
        self.remove_where(
            |a| {
                a.unit == me
                    && matches!(
                        a.kind,
                        AuraKind::DamagePct {
                            swings: Some(0),
                            ..
                        }
                    )
            },
            events,
        )
    }

    /// The player's active Seal.
    fn seal(&self) -> Option<Seal> {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .find_map(|a| match a.kind {
                AuraKind::Seal { seal, .. } => Some(seal),
                _ => None,
            })
    }

    /// Whether a skill holds the player in combat.
    pub(super) fn holds_combat(&self) -> bool {
        self.combat_hold > 0.0
    }

    /// Whether `unit` is stunned.
    pub(super) fn stunned(&self, unit: u64) -> bool {
        self.auras.iter().any(|a| {
            a.unit == unit && matches!(a.kind, AuraKind::Stun | AuraKind::Incapacitate { .. })
        })
    }

    /// Whether `unit` is incapacitated.
    pub(super) fn incapacitated(&self, unit: u64) -> bool {
        self.auras
            .iter()
            .any(|a| a.unit == unit && matches!(a.kind, AuraKind::Incapacitate { .. }))
    }

    /// The player's `next_cast` buffs for `skill`, or for the skill a Skill Choice replaces.
    fn next_casts<'a>(
        &'a self,
        skill: &'a Skill,
    ) -> impl Iterator<Item = (&'static callings::NextCast, i32)> + 'a {
        let me = self.me;
        self.auras.iter().filter_map(move |a| match a.kind {
            AuraKind::NextCast { next, stacks, .. }
                if a.unit == me && next.skill.iter().any(|s| skill.is(s)) =>
            {
                Some((next, stacks as i32))
            }
            _ => None,
        })
    }

    /// How long `skill`'s cast bar runs now, the player's `next_cast` buffs applied.
    fn cast_time_ms(&self, skill: &Skill) -> u32 {
        let base = skill.meta.shape.cast_time_ms;
        if base == 0 || self.next_casts(skill).any(|(n, _)| n.instant) {
            return 0;
        }
        // Never below an instant cast.
        let change: i32 = self
            .next_casts(skill)
            .map(|(n, stacks)| n.cast_time_ms * stacks)
            .sum();
        let ms = (base as i32 + change).max(0) as u32;
        ms * 100 / (100 + self.cast_speed_pct())
    }

    /// The percent faster the player's buffs make their casts.
    fn cast_speed_pct(&self) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .map(|a| match a.kind {
                AuraKind::CastSpeed { pct, .. } => pct,
                _ => 0,
            })
            .sum()
    }

    /// The percent the player's `next_cast` buffs change `skill`'s cost by, -100 at least.
    fn next_cost_pct(&self, skill: &Skill) -> i32 {
        self.next_casts(skill)
            .map(|(n, stacks)| n.cost_pct * stacks)
            .sum::<i32>()
            .max(-100)
    }

    /// The milliseconds the player's `next_cast` buffs add to the auras `skill` applies.
    fn next_duration_ms(&self, skill: &Skill) -> i32 {
        self.next_casts(skill)
            .map(|(n, stacks)| n.duration_ms * stacks)
            .sum()
    }

    /// `skill` went off: each `next_cast` buff for it loses a charge (one with none lasts its
    /// duration), and the buffs that share a group with one used go too. What went.
    fn use_next_casts(&mut self, skill: &Skill, events: &mut Vec<SessionEvent>) -> Vec<Aura> {
        let me = self.me;
        let mut groups: Vec<&'static str> = Vec::new();
        for a in &mut self.auras {
            let AuraKind::NextCast { next, group, .. } = a.kind else {
                continue;
            };
            if a.unit != me || !next.skill.iter().any(|s| skill.is(s)) {
                continue;
            }
            groups.extend(group);
            if a.charges > 0 {
                a.charges -= 1;
                if a.charges == 0 {
                    a.left = 0.0;
                }
            }
        }
        for a in &mut self.auras {
            if let AuraKind::NextCast { group: Some(g), .. } = a.kind {
                if a.unit == me && groups.contains(&g) {
                    a.left = 0.0;
                }
            }
        }
        self.remove_where(|a| a.left <= CLOCK_EPSILON, events)
    }

    /// The spell-mod cells the player's `next_cast` buffs set: `(flat, bit, op, value)`.
    fn next_cast_mods(&self) -> Vec<(bool, u8, u8, i32)> {
        let mut cells: Vec<(bool, u8, u8, i32)> = Vec::new();
        let mut add = |flat: bool, bit: u8, op: u8, value: i32| match cells
            .iter_mut()
            .find(|(f, b, o, _)| (*f, *b, *o) == (flat, bit, op))
        {
            Some(cell) => cell.3 += value,
            None => cells.push((flat, bit, op, value)),
        };
        for a in self.auras.iter().filter(|a| a.unit == self.me) {
            if let AuraKind::CastSpeed { pct, bits } = a.kind {
                // The client's percent shortens the bar: 10% faster is about 9% shorter.
                let shorter = (pct * 100 / (100 + pct)) as i32;
                for &bit in bits {
                    add(false, bit, SPELLMOD_CASTING_TIME, -shorter);
                }
            }
            let AuraKind::NextCast { next, stacks, .. } = a.kind else {
                continue;
            };
            let stacks = stacks as i32;
            for &bit in &next.family_bits {
                if next.instant {
                    add(false, bit, SPELLMOD_CASTING_TIME, -100);
                } else if next.cast_time_ms != 0 {
                    add(true, bit, SPELLMOD_CASTING_TIME, next.cast_time_ms * stacks);
                }
                if next.cost_pct != 0 {
                    add(
                        false,
                        bit,
                        SPELLMOD_COST,
                        (next.cost_pct * stacks).max(-100),
                    );
                }
            }
        }
        cells
    }

    /// The armour `unit` has lost to the player's debuffs (Sundered).
    pub(super) fn armor_loss(&self, unit: u64) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == unit)
            .map(|a| match a.kind {
                AuraKind::ArmorLoss { amount, stacks, .. } => amount * stacks,
                _ => 0,
            })
            .sum()
    }

    /// The extra strikes the player's buffs add to each cast of `skill`, or of a Skill Choice of
    /// it: `(weapon, pct)`.
    fn extra_strikes(&self, skill: &Skill) -> Vec<(Weapon, u32)> {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .filter_map(|a| match a.kind {
                AuraKind::ExtraStrike {
                    skill: s,
                    weapon,
                    weapon_pct,
                } if skill.is(s) => Some((weapon, weapon_pct)),
                _ => None,
            })
            .collect()
    }

    /// Whether a buff of the player's lets them move while `skill`, or a Skill Choice of it,
    /// goes on (Running Fire's Auto Shot).
    fn casts_while_moving(&self, skill: &Skill) -> bool {
        self.auras.iter().any(|a| {
            a.unit == self.me
                && matches!(a.kind, AuraKind::CastWhileMoving { skill: s } if skill.is(s))
        })
    }

    /// `spell_id`'s cooldown changes by `ms` (shorter when negative), and the client is told.
    pub(super) fn change_cooldown(
        &mut self,
        spell_id: u32,
        ms: i32,
        events: &mut Vec<SessionEvent>,
    ) {
        let Some(at) = self.cooldowns.iter().position(|(id, _)| *id == spell_id) else {
            return;
        };
        let left = (self.cooldowns[at].1 + ms as f32 / 1000.0).max(0.0);
        self.cooldowns[at].1 = left;
        events.push(if left > 0.0 {
            SessionEvent::SpellCooldowns {
                caster: self.me,
                cooldowns: vec![(spell_id, clock::ms(left))],
            }
        } else {
            SessionEvent::ClearCooldown {
                spell_id,
                caster: self.me,
            }
        });
    }

    /// The skill the player's next-cast buff turns their next `skill` into, if one does.
    fn becomes(&self, skill: &str) -> Option<&'static str> {
        self.auras.iter().find_map(|a| match a.kind {
            AuraKind::NextCast { next, .. }
                if a.unit == self.me && next.skill.iter().any(|s| s == skill) =>
            {
                next.becomes.as_deref()
            }
            _ => None,
        })
    }

    /// The percent the player's buffs make their ranged weapon shoot faster.
    fn ranged_haste_pct(&self) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .map(|a| match a.kind {
                AuraKind::RangedHaste { pct } => pct,
                _ => 0,
            })
            .sum()
    }

    /// The percent the player's buffs add to their ranged crit chance.
    fn ranged_crit_pct(&self) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .map(|a| match a.kind {
                AuraKind::RangedCrit { pct } => pct,
                _ => 0,
            })
            .sum()
    }

    /// The percent the player's buffs raise `skill`'s amounts by.
    pub(super) fn skill_amount_pct(&self, skill: &Skill) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .map(|a| match a.kind {
                AuraKind::SkillAmount {
                    skill: s,
                    pct,
                    stacks,
                    ..
                } if skill.is(s) => pct * stacks,
                _ => 0,
            })
            .sum()
    }

    /// `damage` coming at the player, less what their shields soak; a spent shield breaks. The
    /// world reads what each shield soaked through [`Self::soak_on`].
    #[cfg(test)]
    pub(super) fn absorb(
        &mut self,
        damage: u32,
        school: School,
        events: &mut Vec<SessionEvent>,
    ) -> u32 {
        self.absorb_on(self.me, damage, school, events)
    }

    /// `damage` coming at `unit`, less what its shields soak; a spent shield breaks.
    pub(super) fn absorb_on(
        &mut self,
        me: u64,
        damage: u32,
        school: School,
        events: &mut Vec<SessionEvent>,
    ) -> u32 {
        self.soak_on(me, damage, school, events).0
    }

    /// [`Self::absorb_on`], and what each shield soaked.
    pub(super) fn soak_on(
        &mut self,
        me: u64,
        damage: u32,
        school: School,
        events: &mut Vec<SessionEvent>,
    ) -> (u32, Vec<Soak>) {
        let mut left = damage;
        let mut soaks = Vec::new();
        for a in &mut self.auras {
            if let AuraKind::Absorb {
                left: shield,
                school: only,
                ward,
            } = &mut a.kind
            {
                if a.unit == me && left > 0 && only.is_none_or(|s| s == school) {
                    let soaked = left.min(*shield);
                    *shield -= soaked;
                    left -= soaked;
                    if *shield == 0 {
                        a.left = 0.0;
                    }
                    soaks.push(Soak {
                        spell_id: a.spell_id,
                        level: a.level,
                        soaked,
                        ward: *ward,
                        broke: *shield == 0,
                    });
                }
            }
        }
        self.remove_where(
            |a| a.unit == me && matches!(a.kind, AuraKind::Absorb { left: 0, .. }),
            events,
        );
        (left, soaks)
    }

    /// What the player's shield of `spell_id` has left to soak, if they wear one.
    fn shield_left(&self, spell_id: u32) -> Option<u32> {
        self.auras.iter().find_map(|a| match a.kind {
            AuraKind::Absorb { left, .. } if a.unit == self.me && a.spell_id == spell_id => {
                Some(left)
            }
            _ => None,
        })
    }

    /// The percent less `unit` hits the player for from the largest flat cut of the player's
    /// damage over time on it (Smoldering Ember).
    pub(super) fn burn_weaken_pct(&self, unit: u64) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == unit)
            .map(|a| match a.kind {
                AuraKind::PeriodicDamage { caster, riders, .. } if caster == self.me => {
                    riders.weaken_pct
                }
                _ => 0,
            })
            .max()
            .unwrap_or(0)
    }

    /// The percent less `unit` hits anyone for from the largest cut by stacks of a damage over
    /// time this session holds on it (Withering Blight).
    pub(super) fn withered_pct(&self, unit: u64) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == unit)
            .map(|a| match a.kind {
                AuraKind::PeriodicDamage { stacks, riders, .. } => {
                    (riders.weaken_per_stack_pct * stacks).min(riders.weaken_max_pct)
                }
                _ => 0,
            })
            .max()
            .unwrap_or(0)
    }

    /// The stacks of the player's damage over time `spell_id` on `unit`; 0 without it.
    pub(super) fn stacks_on(&self, unit: u64, spell_id: u32) -> u32 {
        self.auras
            .iter()
            .find_map(|a| match a.kind {
                AuraKind::PeriodicDamage { caster, stacks, .. }
                    if a.unit == unit && a.spell_id == spell_id && caster == self.me =>
                {
                    Some(stacks)
                }
                _ => None,
            })
            .unwrap_or(0)
    }

    /// The points more of `unit`'s swings miss, from its hit-chance debuffs.
    pub(super) fn miss_bonus(&self, unit: u64) -> u32 {
        let pct: i32 = self
            .auras
            .iter()
            .filter(|a| a.unit == unit)
            .map(|a| match a.kind {
                AuraKind::HitChance { pct } => pct,
                _ => 0,
            })
            .sum();
        (-pct).max(0) as u32
    }

    /// What the cast under way dealt, healed or cost in health.
    pub(super) fn of(&self, of: callings::PctOf) -> u32 {
        match of {
            callings::PctOf::Hit => self.cast_dealt,
            callings::PctOf::Heal => self.cast_healed,
            callings::PctOf::FullHeal => self.cast_full_healed,
            callings::PctOf::HealthCost => self.last_health_cost,
            // The caster's maximums live on the player; `of_pct` reads them.
            callings::PctOf::MaxMana | callings::PctOf::MaxHealth => 0,
        }
    }

    /// The percent more healing `unit` receives.
    fn healing_taken_pct(&self, unit: u64) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == unit)
            .map(|a| match a.kind {
                AuraKind::HealingTaken { pct } => pct,
                _ => 0,
            })
            .sum()
    }

    /// The percent more `school` damage `unit` takes from its own auras, negative for less
    /// (Consecrated Ground).
    pub(super) fn damage_taken_pct(&self, unit: u64, school: School) -> i32 {
        self.auras
            .iter()
            .filter(|a| a.unit == unit)
            .map(|a| match a.kind {
                AuraKind::DamageTaken { pct, school: of } if of.is_none_or(|s| s == school) => pct,
                _ => 0,
            })
            .sum()
    }

    /// Whether `unit` wears an aura of `spell_id`.
    pub(super) fn on(&self, unit: u64, spell_id: u32) -> bool {
        self.auras
            .iter()
            .any(|a| a.unit == unit && a.spell_id == spell_id)
    }

    /// Whether `unit` is snared.
    pub(super) fn slowed(&self, unit: u64) -> bool {
        self.auras
            .iter()
            .any(|a| a.unit == unit && matches!(a.kind, AuraKind::MoveSlow { pct } if pct > 0))
    }

    /// Whether `unit` is rooted.
    pub(super) fn rooted(&self, unit: u64) -> bool {
        self.auras
            .iter()
            .any(|a| a.unit == unit && a.kind == AuraKind::Root)
    }

    /// The duration a `kind` of control of `duration_ms` has on mob `unit` after diminishing
    /// returns: full, then a half, then a quarter, then `None`. Every control that lands resets
    /// the 15 s. The player is never diminished.
    pub(super) fn control(
        &mut self,
        unit: u64,
        kind: ControlKind,
        duration_ms: u32,
    ) -> Option<u32> {
        if unit == self.me {
            return Some(duration_ms);
        }
        let live = self
            .control_clocks
            .iter()
            .position(|&(u, k, _, since)| u == unit && k == kind && since < CONTROL_RESET_SECS);
        let Some(i) = live else {
            self.control_clocks
                .retain(|&(u, k, _, _)| !(u == unit && k == kind));
            self.control_clocks.push((unit, kind, 1, 0.0));
            return Some(duration_ms);
        };
        let count = self.control_clocks[i].2;
        if count >= 3 {
            return None;
        }
        self.control_clocks[i].2 = count + 1;
        self.control_clocks[i].3 = 0.0;
        Some(duration_ms >> count)
    }

    /// What `unit`'s swing time is multiplied by.
    pub(super) fn attack_time_mul(&self, unit: u64) -> f32 {
        self.multiplier(unit, |kind| match kind {
            AuraKind::AttackSlow { pct } => Some(1.0 + pct as f32 / 100.0),
            _ => None,
        })
    }

    /// What `unit`'s run speed is multiplied by: its strongest snare only, and every sprint.
    pub(super) fn speed_mul(&self, unit: u64) -> f32 {
        self.speed_mul_at_least(unit, 1.0)
    }

    /// [`Self::speed_mul`] with the sprints' part at least `floor`: a Well Fed speed, which
    /// takes the larger of itself and the sprints rather than adding to them.
    pub(super) fn speed_mul_at_least(&self, unit: u64, floor: f32) -> f32 {
        let snare = self
            .auras
            .iter()
            .filter(|a| a.unit == unit)
            .filter_map(|a| match a.kind {
                AuraKind::MoveSlow { pct } => Some(pct),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        let sprint = self.multiplier(unit, |kind| match kind {
            AuraKind::RunSpeed { pct } => Some(1.0 + pct as f32 / 100.0),
            _ => None,
        });
        (1.0 - snare.min(100) as f32 / 100.0) * sprint.max(floor)
    }

    /// The extra critical strike chance, in percent, the player's buffs give `skill`: theirs for
    /// it, and those for `any` skill (Blindside's edge).
    pub(super) fn skill_crit_pct(&self, skill: &Skill) -> u32 {
        self.crit_pct_for(|s| s == "any" || skill.is(s))
    }

    /// The extra critical strike chance a white swing gets: the buffs' for `any` skill.
    pub(super) fn white_crit_pct(&self) -> u32 {
        self.crit_pct_for(|s| s == "any")
    }

    fn crit_pct_for(&self, wanted: impl Fn(&str) -> bool) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .map(|a| match a.kind {
                AuraKind::SkillCrit { skill: s, pct } if wanted(s) => pct,
                _ => 0,
            })
            .sum()
    }

    /// Whether the player has an aura of `spell_id` on them.
    pub(super) fn wears(&self, spell_id: u32) -> bool {
        self.auras
            .iter()
            .any(|a| a.unit == self.me && a.spell_id == spell_id)
    }

    /// The power the player's buffs take off `skill`'s cost.
    pub(super) fn cost_less(&self, skill: &Skill) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .map(|a| match a.kind {
                AuraKind::SkillCost { skill: s, amount } if skill.is(s) => amount,
                _ => 0,
            })
            .sum()
    }

    /// The most stacks the player's buffs make `skill` add, 0 for none.
    fn skill_stacks(&self, skill: &Skill) -> u32 {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .map(|a| match a.kind {
                AuraKind::SkillStacks { skill: s, stacks } if skill.is(s) => stacks,
                _ => 0,
            })
            .max()
            .unwrap_or(0)
    }

    /// The procs the player's buffs carry, each with the buff's spell id.
    /// The stacks of the aura whose tick is running its procs; 0 outside a tick.
    pub(super) fn ticking_stacks(&self) -> u32 {
        self.ticking.map_or(0, |a| a.kind.stacks())
    }

    pub(super) fn aura_procs(&self) -> Vec<(u32, &'static callings::AuraProc)> {
        self.auras
            .iter()
            .filter(|a| a.unit == self.me)
            .filter_map(|a| match a.kind {
                AuraKind::Proc(p) => Some((a.spell_id, p)),
                _ => None,
            })
            .collect()
    }

    /// `skill` landed (`None`: a white swing): each of the player's buffs for it, or for `any`
    /// skill, with charges loses one, and a spent one goes. A white swing spends only a buff for
    /// `any` skill, and a skill never spends a buff of its own spell, which its cast just granted.
    pub(super) fn spend_charges(
        &mut self,
        skill: Option<&Skill>,
        events: &mut Vec<SessionEvent>,
    ) -> Vec<Aura> {
        for a in &mut self.auras {
            let for_skill = match a.kind {
                AuraKind::SkillCrit { skill: s, .. }
                | AuraKind::SkillAmount { skill: s, .. }
                | AuraKind::SkillCost { skill: s, .. }
                | AuraKind::SkillStacks { skill: s, .. } => match skill {
                    Some(k) => k.is(s) || (s == "any" && a.spell_id != k.spell_id),
                    None => s == "any",
                },
                _ => false,
            };
            if a.unit == self.me && a.charges > 0 && for_skill {
                a.charges -= 1;
                if a.charges == 0 {
                    // Spent: it runs out now.
                    a.left = 0.0;
                }
            }
        }
        self.remove_where(|a| a.left <= CLOCK_EPSILON, events)
    }

    /// The product of the factors `factor` reads off `unit`'s auras; 1 with none.
    fn multiplier(&self, unit: u64, factor: impl Fn(AuraKind) -> Option<f32>) -> f32 {
        self.auras
            .iter()
            .filter(|a| a.unit == unit)
            .filter_map(|a| factor(a.kind))
            .product()
    }

    fn cooldown_left(&self, spell_id: u32) -> f32 {
        self.cooldowns
            .iter()
            .find(|(s, _)| *s == spell_id)
            .map_or(0.0, |(_, left)| *left)
    }

    /// The queued on-next-swing skill, taken off the queue.
    pub(super) fn take_queued(&mut self) -> Option<u32> {
        self.queued.take()
    }

    /// `mob` dodged the player: Overpower opens on it, marked with a combo point when the
    /// player's Calling has a skill that needs the dodge. Any other Calling's client would show
    /// the marker as a combo point the realm never added.
    pub(super) fn dodged_by(&mut self, mob: u64, marked: bool, events: &mut Vec<SessionEvent>) {
        self.dodged = Some((mob, DODGE_WINDOW_SECS));
        self.dodge_marked = marked;
        if marked {
            self.send_combo(events);
        }
    }

    fn close_dodge_window(&mut self, events: &mut Vec<SessionEvent>) {
        if self.dodged.take().is_some() && self.dodge_marked {
            self.send_combo(events);
        }
    }

    /// The combo fields as the client should see them: the points a builder put on a target,
    /// else the point that marks the Overpower window, else none.
    fn combo_pairs(&self) -> Vec<(u16, u32)> {
        let (count, target) = if self.combo.count > 0 {
            (u32::from(self.combo.count), self.combo.target)
        } else if let Some((mob, _)) = self.dodged.filter(|_| self.dodge_marked) {
            (1, mob)
        } else {
            (0, 0)
        };
        let mut pairs = vec![(PLAYER_FIELD_BYTES, count << 8)];
        pairs.extend(guid_pairs(PLAYER_FIELD_COMBO_TARGET, target));
        pairs
    }

    pub(super) fn send_combo(&self, events: &mut Vec<SessionEvent>) {
        events.push(values(self.me, &self.combo_pairs()));
    }

    /// `mob` died: the points on it go with it.
    pub(super) fn target_died(&mut self, mob: u64, events: &mut Vec<SessionEvent>) {
        if self.combo.count > 0 && self.combo.target == mob {
            self.combo.clear();
            self.send_combo(events);
        }
    }

    /// Puts `aura` on its unit, in the slot its spell already holds there or a free one.
    fn apply(&mut self, mut aura: Aura, events: &mut Vec<SessionEvent>) -> Option<Aura> {
        // Another player wears it in their own session.
        if aura.unit != self.me && crate::shard::is_player(aura.unit) {
            self.sent.push(aura);
            return None;
        }
        if let Some(old) = self.auras.iter_mut().find(|a| {
            a.unit == aura.unit && a.spell_id == aura.spell_id && a.kind.same_effect(aura.kind)
        }) {
            aura.slot = old.slot;
            if let (AuraKind::Stacks { count: held, .. }, AuraKind::Stacks { count, max, rule }) =
                (old.kind, aura.kind)
            {
                let count = match rule {
                    StackRule::Ignore => held,
                    StackRule::Replace => count,
                    StackRule::Refresh | StackRule::Independent | StackRule::Stack => held + count,
                }
                .min(max);
                aura.kind = AuraKind::Stacks { count, max, rule };
                if rule == StackRule::Ignore {
                    aura = *old;
                }
            }
            if let (AuraKind::StackTo { count: held, .. }, AuraKind::StackTo { count, max, .. }) =
                (old.kind, &mut aura.kind)
            {
                *count = (held + *count).min(*max);
            }
            if let (
                AuraKind::SkillAmount { stacks: held, .. },
                AuraKind::SkillAmount {
                    stacks, max_stacks, ..
                },
            )
            | (
                AuraKind::DamageDone { stacks: held, .. },
                AuraKind::DamageDone {
                    stacks, max_stacks, ..
                },
            )
            | (
                AuraKind::MeleeHaste { stacks: held, .. },
                AuraKind::MeleeHaste {
                    stacks, max_stacks, ..
                },
            )
            | (
                AuraKind::NextCast { stacks: held, .. },
                AuraKind::NextCast {
                    stacks, max_stacks, ..
                },
            )
            | (
                AuraKind::ArmorLoss { stacks: held, .. },
                AuraKind::ArmorLoss {
                    stacks, max_stacks, ..
                },
            ) = (old.kind, &mut aura.kind)
            {
                *stacks = (held + *stacks).min(*max_stacks);
            }
            if let (
                AuraKind::PeriodicDamage { stacks: held, .. },
                AuraKind::PeriodicDamage {
                    stacks,
                    max_stacks,
                    rule,
                    ..
                },
            ) = (old.kind, &mut aura.kind)
            {
                if *rule != StackRule::Replace {
                    *stacks = (held + *stacks).min(*max_stacks);
                }
            }
            *old = aura;
        } else if let Some(sibling) = self
            .auras
            .iter()
            .find(|a| a.unit == aura.unit && a.spell_id == aura.spell_id)
        {
            // Another effect of the same spell: one buff on the bar, as a Spell.dbc aura with
            // several effects is (Seal of Iron's two procs).
            aura.slot = sibling.slot;
            self.auras.push(aura);
        } else {
            let range = if aura.kind.helpful() {
                0..POSITIVE_SLOTS
            } else {
                POSITIVE_SLOTS..AURA_SLOTS
            };
            let taken: Vec<u8> = self
                .auras
                .iter()
                .filter(|a| a.unit == aura.unit)
                .map(|a| a.slot)
                .collect();
            aura.slot = range.into_iter().find(|s| !taken.contains(s))?;
            self.auras.push(aura);
        }
        if aura.unit == self.me {
            events.push(SessionEvent::AuraDuration {
                slot: aura.slot,
                remaining_ms: clock::ms(aura.left),
            });
        }
        events.push(self.slot_fields(aura.unit, aura.slot));
        Some(aura)
    }

    /// Adds `n` stacks, up to its `max_stacks`, to the player's periodic damage aura `spell_id` on
    /// `unit`, leaving its timer alone. Returns whether the unit wears one.
    pub(super) fn grow(
        &mut self,
        unit: u64,
        spell_id: u32,
        n: u32,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        let me = self.me;
        let Some(aura) = self.auras.iter_mut().find(|a| {
            a.unit == unit
                && a.spell_id == spell_id
                && matches!(a.kind, AuraKind::PeriodicDamage { caster, .. } if caster == me)
        }) else {
            return false;
        };
        if let AuraKind::PeriodicDamage {
            stacks, max_stacks, ..
        } = &mut aura.kind
        {
            *stacks = (*stacks + n).min(*max_stacks);
        }
        let slot = aura.slot;
        events.push(self.slot_fields(unit, slot));
        true
    }

    /// Takes every stack of `spell_id` off `unit`: the count it held.
    pub(super) fn take_stacks(
        &mut self,
        unit: u64,
        spell_id: u32,
        events: &mut Vec<SessionEvent>,
    ) -> u32 {
        self.remove_where(
            |a| {
                a.unit == unit
                    && a.spell_id == spell_id
                    && matches!(a.kind, AuraKind::Stacks { .. })
            },
            events,
        )
        .iter()
        .map(|a| a.kind.stacks())
        .sum()
    }

    /// Takes `unit`'s [`AuraKind::Marker`] of `spell_id` off.
    pub(super) fn hide_marker_on(
        &mut self,
        unit: u64,
        spell_id: u32,
        events: &mut Vec<SessionEvent>,
    ) {
        self.remove_where(
            |a| {
                a.unit == unit
                    && a.spell_id == spell_id
                    && matches!(a.kind, AuraKind::Marker { .. })
            },
            events,
        );
    }

    /// Takes every aura `keep` refuses off, clearing their slots.
    fn remove_where(
        &mut self,
        mut drop: impl FnMut(&Aura) -> bool,
        events: &mut Vec<SessionEvent>,
    ) -> Vec<Aura> {
        let (gone, kept): (Vec<Aura>, Vec<Aura>) = std::mem::take(&mut self.auras)
            .into_iter()
            .partition(|a| drop(a));
        self.auras = kept;
        for a in &gone {
            events.push(self.slot_fields(a.unit, a.slot));
        }
        gone
    }

    /// The `UNIT_FIELD_AURA*` dwords that hold `slot` on `unit`, as its auras now stand.
    fn slot_fields(&self, unit: u64, slot: u8) -> SessionEvent {
        let on_unit = |s: u8| self.auras.iter().find(|a| a.unit == unit && a.slot == s);
        let flags_word = slot / 8;
        let flags = (0..8).fold(0u32, |w, i| {
            let s = flags_word * 8 + i;
            let nibble = on_unit(s).map_or(0, |a| {
                AFLAG_EFF_INDEX_0
                    | if a.kind.helpful() {
                        AFLAG_CANCELABLE
                    } else {
                        0
                    }
            });
            w | nibble << (4 * u32::from(i))
        });
        let byte_word = slot / 4;
        let levels = (0..4).fold(0u32, |w, i| {
            let level = on_unit(byte_word * 4 + i).map_or(0, |a| u32::from(a.level));
            w | level << (8 * u32::from(i))
        });
        // A byte a slot: stacks - 1.
        let applications = (0..4).fold(0u32, |w, i| {
            let stacks = on_unit(byte_word * 4 + i).map_or(1, |a| a.kind.stacks());
            w | stacks.saturating_sub(1).min(0xFF) << (8 * u32::from(i))
        });
        values(
            unit,
            &[
                (
                    UNIT_FIELD_AURA + u16::from(slot),
                    on_unit(slot).map_or(0, |a| a.spell_id),
                ),
                (UNIT_FIELD_AURAFLAGS + u16::from(flags_word), flags),
                (UNIT_FIELD_AURALEVELS + u16::from(byte_word), levels),
                (
                    UNIT_FIELD_AURAAPPLICATIONS + u16::from(byte_word),
                    applications,
                ),
            ],
        )
    }
}

/// `SMSG_INITIAL_SPELLS`: the current Calling's skills, a set Skill Choice in its skill's place,
/// and the set choices' spells, and the
/// item cooldowns still running at the wall clock's `wall` (the skills' own start afresh each
/// session).
pub(super) fn spell_book(c: &Char, wall: f64) -> SessionEvent {
    SessionEvent::SpellBook {
        spell_ids: callings::of(c.calling)
            .skills
            .iter()
            .filter(|s| u32::from(s.learn_level) <= u32::from(c.level))
            .filter(|s| !crate::talents::replaced(c).any(|k| k == s.key))
            .map(|s| s.spell_id)
            .chain(crate::talents::known_spells(c))
            .chain(c.knows_fishing.then_some(crate::fish::spell().id))
            .collect(),
        cooldowns: c
            .item_cooldowns
            .iter()
            .filter(|&&(_, ready)| ready > wall)
            .filter_map(|&(entry, ready)| {
                let t = items::template(entry)?;
                Some(SpellCooldown {
                    spell_id: t.use_spell as u16,
                    item_id: entry as u16,
                    category: 0,
                    spell_cd_ms: clock::ms((ready - wall) as f32),
                    category_cd_ms: 0,
                })
            })
            .collect(),
    }
}

/// `SMSG_ACTION_BUTTONS`: the bar as the player left it, or the Calling's default.
pub(crate) fn action_buttons(c: &Char) -> SessionEvent {
    let bar = c
        .action_bar
        .clone()
        .unwrap_or_else(|| callings::default_bar(c.calling));
    SessionEvent::ActionButtons {
        buttons: bar
            .into_iter()
            .map(|(slot, packed)| ActionButton {
                slot,
                action: packed & 0x00FF_FFFF,
                kind: (packed >> 24) as u8,
            })
            .collect(),
    }
}

/// The Calling's stance in `UNIT_FIELD_BYTES_1` byte 2: the client checks a skill's stances
/// against it before it sends the cast.
pub(super) fn stance(c: &Char) -> (u16, u32) {
    (
        UNIT_FIELD_BYTES_1,
        u32::from(callings::of(c.calling).shapeshift_form) << 16,
    )
}

/// The player's stats with their buffs: what strikes roll from. White swings roll from
/// [`white`], which adds the haste.
pub(super) fn buffed(stats: &CharacterStats, skills: &Skills) -> CharacterStats {
    let bonus = skills.attack_power_bonus();
    let mut s = *stats;
    let (health_pct, _, armor_loss_pct) = skills.buff_pcts();
    s.max_health += s.max_health * health_pct / 100;
    // Agility's armour is not the items'.
    let items_armor = s.armor.saturating_sub(2 * s.base[1]);
    s.armor += scaled(items_armor, skills.defense_pct(DefenseStat::ArmorItems)) - items_armor;
    s.armor -= s.armor * armor_loss_pct.min(100) / 100;
    // A creature's sunder on the player.
    s.armor = s.armor.saturating_sub(skills.armor_loss(skills.me));
    s.dodge_pct = (s.dodge_pct + f64::from(skills.dodge_pct())).min(100.0);
    s.parry_pct = (s.parry_pct + f64::from(skills.defense_pct(DefenseStat::Parry))).min(100.0);
    // Blocking takes a shield; the roll table never passes 100%.
    if s.shield_block.is_some() {
        s.block_pct = (s.block_pct + f64::from(skills.defense_pct(DefenseStat::Block))).min(100.0);
        s.block_value = scaled(s.block_value, skills.defense_pct(DefenseStat::BlockValue));
    }
    if bonus > 0 {
        s.attack_power += bonus as i32;
        let extra = formulas::ap_damage(bonus as i32, clock::secs(s.attack_time_ms));
        s.min_damage += extra;
        s.max_damage += extra;
    }
    s
}

/// [`buffed`] with the haste: what white swings roll from and time by, and what the sheet shows.
/// Faster swings, each white hit cut to match, so the haste alone adds no damage a second; a
/// strike (Mortal Strike, Heroic Strike) keeps its full weapon roll.
pub(super) fn white(stats: &CharacterStats, skills: &Skills) -> CharacterStats {
    let mut s = buffed(stats, skills);
    let haste = skills.haste_pct();
    if haste > 0 {
        s.attack_time_ms = hasted_attack_time(s.attack_time_ms, haste);
        s.min_damage = haste_cut_damage(s.min_damage, haste);
        s.max_damage = haste_cut_damage(s.max_damage, haste);
    }
    s
}

/// [`white`] for an off-hand swing: the off-hand weapon's damage at half, and Dual Wield
/// Specialization's share more, on the off-hand weapon's own swing time; `None` without a weapon
/// in the off hand. The buffs and the haste land as they do on the main hand.
pub(super) fn off_hand_white(stats: &CharacterStats, skills: &Skills) -> Option<CharacterStats> {
    let off = stats.off_hand?;
    let bonus = skills.attack_power_bonus();
    let extra = if bonus > 0 {
        formulas::ap_damage(bonus as i32, clock::secs(off.attack_time_ms))
    } else {
        0.0
    };
    let halved = OFF_HAND_DAMAGE_PCT as f32 / 100.0;
    let share = halved * (100 + stats.offhand_damage_pct) as f32 / 100.0;
    let mut s = buffed(stats, skills);
    s.attack_time_ms = off.attack_time_ms;
    s.min_damage = (off.min_damage + extra) * share;
    s.max_damage = (off.max_damage + extra) * share;
    let haste = skills.haste_pct();
    if haste > 0 {
        s.attack_time_ms = hasted_attack_time(s.attack_time_ms, haste);
        s.min_damage = haste_cut_damage(s.min_damage, haste);
        s.max_damage = haste_cut_damage(s.max_damage, haste);
    }
    Some(s)
}

/// The sheet fields buffs move: the attack power mods, the damage they add and the swing time.
fn buff_sheet(stats: &CharacterStats, skills: &Skills) -> Vec<(u16, u32)> {
    let s = white(stats, skills);
    let off = off_hand_white(stats, skills);
    let off_time = off.map_or(s.attack_time_ms, |o| o.attack_time_ms);
    vec![
        (
            UNIT_FIELD_ATTACK_POWER_MODS,
            skills.attack_power_bonus() & 0xFFFF,
        ),
        (UNIT_FIELD_MINDAMAGE, float(s.min_damage)),
        (UNIT_FIELD_MAXDAMAGE, float(s.max_damage)),
        (UNIT_FIELD_BASEATTACKTIME, s.attack_time_ms),
        (UNIT_FIELD_BASEATTACKTIME + 1, off_time),
    ]
}

/// How one strike against one target went.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Landed {
    Hit,
    Crit,
    Missed(u8),
}

pub(super) fn refused(spell_id: u32, reason: u8) -> SessionEvent {
    SessionEvent::CastResult {
        spell_id,
        success: false,
        reason: Some(reason),
        arg: None,
    }
}

/// The `SpellMissInfo` a melee spell's target goes into `SMSG_SPELL_GO`'s miss list with, when the
/// roll on the attack table was a miss, a dodge or a parry. A block still hits.
pub(super) fn missed_by(outcome: Outcome) -> Option<u8> {
    match outcome {
        Outcome::Miss => Some(SPELL_MISS_MISS),
        Outcome::Dodge => Some(SPELL_MISS_DODGE),
        Outcome::Parry => Some(SPELL_MISS_PARRY),
        _ => None,
    }
}

/// `SMSG_SPELLNONMELEEDAMAGELOG` for `damage` of `school` from `attacker`'s `spell_id`: nothing
/// absorbed, resisted or blocked, flagged as a crit when it was one.
pub(super) fn damage_log(
    attacker: u64,
    target: u64,
    spell_id: u32,
    damage: u32,
    school: School,
    crit: bool,
) -> SpellDamageLog {
    SpellDamageLog {
        target,
        attacker,
        spell_id,
        damage,
        school: school.index(),
        absorb: 0,
        resist: 0,
        periodic: false,
        blocked: 0,
        hit_info: if crit { SPELL_HIT_TYPE_CRIT } else { 0 },
    }
}

impl InWorld {
    /// `CMSG_CAST_SPELL`: refused with the stock reason, queued on the next swing, or cast.
    pub(crate) fn cast(
        &mut self,
        spell_id: u32,
        target: Option<u64>,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) -> Ticked {
        let ticked = Ticked::default();
        if self.fear.is_some() {
            events.push(refused(spell_id, SPELL_FAILED_FLEEING));
            return ticked;
        }
        // Casting ends a hide, and loses a fish.
        self.hiding = None;
        self.lose_fish(events);
        if spell_id == crate::fish::spell().id {
            self.cast_fishing(c, events);
            return ticked;
        }
        let Some(skill) = crate::talents::skill(c, spell_id) else {
            events.push(refused(spell_id, SPELL_FAILED_NOT_KNOWN));
            return ticked;
        };
        let targets = match self.check(skill, target, true, c) {
            Ok(targets) => targets,
            Err(reason) => {
                events.push(refused(spell_id, reason));
                return ticked;
            }
        };
        if skill.effects.iter().any(|e| {
            matches!(
                e,
                Effect::WeaponStrike {
                    on_next_swing: true,
                    ..
                }
            )
        }) {
            // Paid and answered when the swing comes (`CURRENT_MELEE_SPELL`).
            self.skills.queued = Some(spell_id);
            return ticked;
        }
        if self.skills.casting.is_some() {
            events.push(refused(spell_id, SPELL_FAILED_SPELL_IN_PROGRESS));
            return ticked;
        }
        let shape = &skill.meta.shape;
        // An auto-repeat shot starts shooting; each shot comes on its own timer.
        if shape.cast_kind.as_deref() == Some("auto_repeat") {
            self.player.shooting = target.map(|t| (spell_id, t));
            return ticked;
        }
        // A new cast ends a channel under way.
        self.end_channel(true, events);
        if shape.cast_kind.as_deref() == Some("channel") && shape.channel_ms > 0 {
            self.start_channel(skill, target, &targets, c, events);
            return ticked;
        }
        // A strike that is instant at a low target has no bar there (Killshot).
        let cast_ms = if self.instant_below(skill, target) {
            0
        } else {
            self.skills.cast_time_ms(skill)
        };
        if cast_ms > 0 {
            // A cast bar: the global cooldown starts now, the rest when it fills.
            if skill.gcd_ms > 0 {
                self.skills.gcd = clock::secs(skill.gcd_ms);
            }
            self.skills.casting = Some(TimedCast {
                spell_id,
                item: 0,
                entry: 0,
                skill: Some(target),
                left: clock::secs(cast_ms),
            });
            events.push(SessionEvent::SpellStart {
                caster: self.guid,
                spell_id,
                cast_flags: 0,
                cast_time_ms: cast_ms,
                target,
                ammo_display_id: None,
            });
            return ticked;
        }
        self.go(skill, target, targets, true, c, rng, events)
    }

    /// Whether `skill` cast at `target` has no cast bar because the target is under its strike's
    /// `below_health_pct` (Killshot), read as the cast starts.
    fn instant_below(&self, skill: &Skill, target: Option<u64>) -> bool {
        let Some(pct) = skill.effects.iter().find_map(|e| match *e {
            Effect::WeaponStrike {
                below_instant: true,
                below_health_pct,
                ..
            } => Some(below_health_pct),
            _ => None,
        }) else {
            return false;
        };
        self.mobs.iter().any(|m| {
            Some(m.guid) == target
                && m.alive()
                && u64::from(m.health) * 100 < u64::from(m.stats.max_health) * u64::from(pct)
        })
    }

    /// A proc's free repeat of the skill `key` cast (the version of it the player casts now) at
    /// the cast's target (Twin Arrows' old proc).
    pub(super) fn repeat_cast(
        &mut self,
        key: Option<&str>,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let (Some(key), Some(target)) = (key, self.skills.cast_target) else {
            return;
        };
        if let Some(skill) = crate::talents::current(c, key) {
            self.free_cast(skill, target, true, c, rng, events, ticked);
        }
    }

    /// `skill` goes off at once at `target` if it stands in reach: no cost, no cooldown, no global
    /// cooldown and no cast bar. A `repeat` (Twin Arrows) repeats nothing itself, and nothing
    /// repeats inside one; a `cast_skill`'s cast (Parting Shot) may be repeated.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn free_cast(
        &mut self,
        skill: &'static Skill,
        target: u64,
        repeat: bool,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        if repeat && self.skills.repeating {
            return;
        }
        let (free, repeating) = (self.skills.free, self.skills.repeating);
        self.skills.free = true;
        self.skills.repeating |= repeat;
        if let Ok(targets) = self.check(skill, Some(target), false, c) {
            let done = self.go(skill, Some(target), targets, true, c, rng, events);
            ticked.leveled |= done.leveled;
            ticked.discovered |= done.discovered;
        }
        self.skills.free = free;
        self.skills.repeating = repeating;
    }

    /// Casts the skill each `cast_skill` of the last tick's casts names, now that their Blinks
    /// have landed: the version set on its row, free, at the cast's target, if it is alive, in
    /// the skill's range and in sight of where the caster stands (Parting Shot).
    pub(super) fn land_follow_ups(
        &mut self,
        host: &dyn crate::Host,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        for f in std::mem::take(&mut self.skills.follow_ups) {
            let Some(target) = f.target else {
                continue;
            };
            let Some(skill) = crate::talents::current_of(c, f.skill_spell_id) else {
                continue;
            };
            let Some(mob) = self.mobs.iter().find(|m| m.guid == target) else {
                continue;
            };
            if !host.line_of_sight(self.map, eye(c.position), eye(mob.position)) {
                continue;
            }
            self.free_cast(skill, target, false, c, rng, events, ticked);
        }
    }

    /// The player stepped: they are moving, and their `break_on=move` auras come off.
    pub(crate) fn stepped(&mut self, events: &mut Vec<SessionEvent>) {
        self.lose_fish(events);
        let s = &mut self.skills;
        s.since_step = Some(0.0);
        let gone = std::mem::take(&mut s.break_on_move);
        let me = self.guid;
        if !gone.is_empty() {
            let faded = s.remove_where(|a| a.unit == me && gone.contains(&a.spell_id), events);
            self.faded(&faded, events);
        }
    }

    /// Whether the caster is moving: a step came in the last [`MOVING_GRACE_SECS`].
    fn moving(&self) -> bool {
        self.skills
            .since_step
            .is_some_and(|t| t < MOVING_GRACE_SECS)
    }

    /// The movement clocks: moving while a step came in the last [`MOVING_GRACE_SECS`], still
    /// otherwise; `moving_for` and `still_for` procs as they run.
    pub(super) fn movement_tick(
        &mut self,
        dt: f32,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let moving = self.moving();
        let s = &mut self.skills;
        s.since_step = s.since_step.map(|t| t + dt);
        let (clock, other, event) = if moving {
            (
                &mut s.moving_secs,
                &mut s.still_secs,
                callings::ProcEvent::MovingFor,
            )
        } else {
            (
                &mut s.still_secs,
                &mut s.moving_secs,
                callings::ProcEvent::StillFor,
            )
        };
        *other = 0.0;
        let was = clock::ms(*clock);
        *clock += dt;
        let now = clock::ms(*clock);
        self.proc(
            crate::rules::procs::Happened {
                event,
                skill: None,
                plain: None,
                yields_xp: false,
                target: None,
                amount: now,
                school: None,
                was_ms: was,
            },
            c,
            rng,
            events,
            ticked,
        );
    }

    /// `CMSG_CANCEL_AUTO_REPEAT_SPELL`, or a shot that cannot go on: the shooting stops.
    pub(crate) fn stop_shooting(&mut self, events: &mut Vec<SessionEvent>) {
        if self.player.shooting.take().is_some() {
            events.push(SessionEvent::CancelAutoRepeat);
        }
    }

    /// The auto-repeat shot, when its timer is up: at the ranged weapon's speed, held while a cast
    /// bar runs, the target is behind or, for a skill interrupted on `move`, the caster moves (the
    /// timer runs on, so a shot that came due goes as they stop), stopped when the target dies or leaves the shot's range.
    pub(super) fn auto_shot(
        &mut self,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let Some((spell_id, target)) = self.player.shooting else {
            return;
        };
        let Some(skill) = crate::talents::skill(c, spell_id) else {
            self.stop_shooting(events);
            return;
        };
        if self.player.shot_timer > 0.0 || self.skills.casting.is_some() || self.fear.is_some() {
            return;
        }
        // A buff may let the shot go on the move (Running Fire).
        if self.moving()
            && skill.meta.shape.interrupt_on.iter().any(|i| i == "move")
            && !self.skills.casts_while_moving(skill)
        {
            return;
        }
        // A next-cast buff may make this shot another skill (Explosive Payload).
        let auto = skill;
        let skill = self
            .skills
            .becomes(&auto.key)
            .and_then(|key| callings::of(c.calling).skill_by_key(key))
            .unwrap_or(auto);
        let targets = match self.check(skill, Some(target), false, c) {
            Ok(targets) => targets,
            Err(SPELL_FAILED_UNIT_NOT_INFRONT) => return,
            Err(_) => {
                self.stop_shooting(events);
                return;
            }
        };
        let speed = self.player.stats.ranged.map_or(0, |r| r.attack_time_ms);
        // A quiver's haste and the buffs' multiply.
        let speed = hasted_attack_time(speed, self.player.stats.quiver_haste_pct);
        let speed = hasted_attack_time(speed, self.skills.ranged_haste_pct());
        self.player.shot_timer = clock::secs(speed);
        let done = self.go(skill, Some(target), targets, true, c, rng, events);
        ticked.leveled |= done.leveled;
        ticked.discovered |= done.discovered;
        if !std::ptr::eq(skill, auto) {
            let gone = self.skills.use_next_casts(auto, events);
            self.faded(&gone, events);
        }
        if !self.mobs.iter().any(|m| m.guid == target && m.alive()) {
            self.stop_shooting(events);
        }
    }

    /// A channelled `skill` starts at its first target: paid now, its bar on the client, its
    /// effects each tick until it runs out or breaks.
    fn start_channel(
        &mut self,
        skill: &Skill,
        target: Option<u64>,
        targets: &[u64],
        c: &Char,
        events: &mut Vec<SessionEvent>,
    ) {
        let Some(&at) = targets.first() else {
            events.push(refused(skill.spell_id, SPELL_FAILED_BAD_TARGETS));
            return;
        };
        self.pay(skill, true, c, events);
        let shape = &skill.meta.shape;
        events.push(SessionEvent::SpellStart {
            caster: self.guid,
            spell_id: skill.spell_id,
            cast_flags: 0,
            cast_time_ms: 0,
            target,
            ammo_display_id: None,
        });
        events.push(SessionEvent::CastResult {
            spell_id: skill.spell_id,
            success: true,
            reason: None,
            arg: None,
        });
        events.push(SessionEvent::SpellGo {
            caster: self.guid,
            spell_id: skill.spell_id,
            cast_flags: 0,
            hits: vec![at],
            misses: Vec::new(),
            target,
            go_target: None,
            dest: None,
            ammo_display_id: None,
            item_caster: None,
        });
        events.push(SessionEvent::ChannelStart {
            spell_id: skill.spell_id,
            duration_ms: shape.channel_ms,
        });
        // The caster's channel fields: every client that sees them draws the beam to `at`.
        let mut pairs = vec![(UNIT_CHANNEL_SPELL, skill.spell_id)];
        pairs.extend(guid_pairs(UNIT_FIELD_CHANNEL_OBJECT, at));
        events.push(values(self.guid, &pairs));
        if let Some(mob) = self.mobs.iter_mut().find(|m| m.guid == at) {
            if mob.state == State::Idle {
                super::engage(mob, self.guid, events);
            }
        }
        let period = clock::secs(shape.channel_tick_ms.max(1));
        self.skills.channel = Some(Channel {
            spell_id: skill.spell_id,
            target: at,
            left: clock::secs(shape.channel_ms),
            period,
            next_tick: period,
            step_ends: shape.interrupt_on.iter().any(|i| i == "move"),
        });
        let p = &self.player;
        events.push(values(self.guid, &[p.power.field()]));
    }

    /// The channel's clock: each tick runs the skill's effects at its target, rolled afresh; it
    /// ends when it runs out, or when its target dies or leaves its range.
    #[allow(clippy::too_many_arguments)]
    fn channel_tick(
        &mut self,
        dt: f32,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let Some(ch) = &mut self.skills.channel else {
            return;
        };
        ch.left -= dt;
        ch.next_tick -= dt;
        let due = ch.next_tick <= CLOCK_EPSILON;
        if due {
            ch.next_tick += ch.period;
        }
        let ch = *ch;
        if due {
            let Some(skill) = crate::talents::skill(c, ch.spell_id) else {
                self.end_channel(true, events);
                return;
            };
            let [lo, hi] = skill.range_yd;
            let in_reach = self.mobs.iter().any(|m| {
                m.guid == ch.target && m.alive() && {
                    let d = distance(m.position, c.position);
                    d >= lo && (hi == 0.0 || d <= hi)
                }
            });
            if !in_reach {
                self.end_channel(true, events);
                return;
            }
            let how = self.roll(skill, ch.target, false, c, rng);
            if !matches!(how, Landed::Missed(_)) {
                for effect in &skill.effects {
                    self.effect(
                        skill,
                        effect,
                        ch.target,
                        how,
                        c.power_level(),
                        0,
                        c,
                        rng,
                        events,
                        ticked,
                    );
                }
            }
        }
        if ch.left <= CLOCK_EPSILON {
            self.end_channel(false, events);
        }
    }

    /// The point a ground-targeted cast is aimed at, for the cast that follows.
    pub(crate) fn set_dest(&mut self, dest: Option<[f32; 3]>) {
        self.skills.dest = dest;
    }

    /// The ground areas' clock: whoever stepped in takes the area's aura for their side, whoever
    /// left or outlived it loses it, and a spent area goes.
    fn area_tick(&mut self, dt: f32, c: &Char, events: &mut Vec<SessionEvent>) {
        let mut areas = std::mem::take(&mut self.skills.areas);
        for area in &mut areas {
            area.left -= dt;
            area.arming = (area.arming - dt).max(0.0);
            // A following area is centred on its caster; its DynamicObject, which cannot move, is
            // shown again at their feet once they have walked off it (Shroud of Rot).
            let centre = if area.follow {
                let off = distance(area.centre, c.position) >= FOLLOW_RESHOW_YD;
                if off && area.left > CLOCK_EPSILON {
                    events.push(SessionEvent::ObjectDestroyed(area.object));
                    area.object = self.skills.next_patch(self.guid);
                    area.centre = c.position;
                    events.push(patch_create(
                        area.object,
                        area.caster,
                        area.spell_id,
                        area.centre,
                        area.radius_yd,
                    ));
                }
                c.position
            } else {
                area.centre
            };
            let now: Vec<u64> = if area.left <= CLOCK_EPSILON || area.arming > CLOCK_EPSILON {
                Vec::new()
            } else {
                let mut now: Vec<u64> = self
                    .mobs
                    .iter()
                    .filter(|m| {
                        m.alive()
                            && distance(m.position, centre) <= area.radius_yd
                            && !(area.first_only && !m.remote && m.template.raises_boss_bar())
                    })
                    .map(|m| m.guid)
                    .collect();
                if !area.first_only {
                    now.extend(
                        self.friendly_players(c)
                            .into_iter()
                            .filter(|(_, p, _)| distance(*p, centre) <= area.radius_yd)
                            .map(|(guid, _, _)| guid),
                    );
                }
                now
            };
            if area.first_only {
                // The first enemy inside takes the aura and spends the area; the aura runs out
                // on its own duration.
                if let (Some(&unit), Some(def)) = (now.first(), area.enemy_aura) {
                    self.put_aura(def, unit, c.power_level(), events);
                    area.left = 0.0;
                    events.push(SessionEvent::ObjectDestroyed(area.object));
                } else if area.left <= CLOCK_EPSILON {
                    events.push(SessionEvent::ObjectDestroyed(area.object));
                }
                continue;
            }
            let left: Vec<u64> = area
                .inside
                .iter()
                .copied()
                .filter(|u| !now.contains(u))
                .collect();
            self.leave_area(area, &left, events);
            for &unit in now.iter().filter(|u| !area.inside.contains(u)) {
                let row = if unit == self.guid {
                    area.ally_aura
                } else {
                    area.enemy_aura
                };
                if let Some(def) = row {
                    self.put_aura(def, unit, c.power_level(), events);
                }
            }
            // Each enemy inside wearing the caster's damage over time gains a stack on the
            // clock; its timer stays (Breeding Ground).
            if let Some((id, every, next)) = &mut area.grow {
                *next -= dt;
                while *next <= CLOCK_EPSILON {
                    *next += *every;
                    for &unit in now.iter().filter(|&&u| u != self.guid) {
                        self.skills.grow(unit, *id, 1, events);
                    }
                }
            }
            area.inside = now;
            if area.left <= CLOCK_EPSILON {
                events.push(SessionEvent::ObjectDestroyed(area.object));
            }
        }
        areas.retain(|a| a.left > CLOCK_EPSILON);
        self.skills.areas = areas;
    }

    /// `units` are out of `area`: its auras come off them.
    fn leave_area(&mut self, area: &Area, units: &[u64], events: &mut Vec<SessionEvent>) {
        for &unit in units {
            let row = if unit == self.guid {
                area.ally_aura
            } else {
                area.enemy_aura
            };
            if let Some(def) = row {
                let gone = self
                    .skills
                    .remove_where(|a| a.unit == unit && a.spell_id == def.spell_id, events);
                self.faded(&gone, events);
            }
        }
    }

    /// Ends the channel under way, if any: `broken` when it stops short.
    pub(super) fn end_channel(&mut self, broken: bool, events: &mut Vec<SessionEvent>) {
        let Some(ch) = self.skills.channel.take() else {
            return;
        };
        self.channel_closed(ch.spell_id, broken, events);
    }

    /// A channel of `spell_id` is over: the client's bar empties and the caster's channel fields
    /// clear. `broken` when it stopped short.
    pub(super) fn channel_closed(
        &self,
        spell_id: u32,
        broken: bool,
        events: &mut Vec<SessionEvent>,
    ) {
        if broken {
            events.push(SessionEvent::SpellFailedOther {
                caster: self.guid,
                spell_id,
            });
        }
        events.push(SessionEvent::ChannelUpdate { remaining_ms: 0 });
        let mut pairs = vec![(UNIT_CHANNEL_SPELL, 0)];
        pairs.extend(guid_pairs(UNIT_FIELD_CHANNEL_OBJECT, 0));
        events.push(values(self.guid, &pairs));
    }

    /// `skill` goes off at `targets` (`target` the client's pick): paid, rolled, landed. `instant`
    /// for a cast with no bar, which starts the global cooldown and announces its own start.
    #[allow(clippy::too_many_arguments)]
    fn go(
        &mut self,
        skill: &'static Skill,
        target: Option<u64>,
        mut targets: Vec<u64>,
        instant: bool,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) -> Ticked {
        let mut ticked = Ticked::default();
        let spell_id = skill.spell_id;
        self.pay(skill, instant, c, events);
        // A ranged attack spends an arrow or bullet, and each extra ranged strike another.
        if skill.wants_ranged() {
            let extra = self
                .skills
                .extra_strikes(skill)
                .iter()
                .filter(|(w, _)| *w == Weapon::Ranged)
                .count() as u32;
            if crate::ammo::spend(c, 1 + extra, events) {
                self.refresh_gear(c, events);
            }
        }
        self.skills.cast_target = target;
        self.skills.cast_dealt = 0;
        self.skills.cast_healed = 0;
        self.skills.cast_full_healed = 0;
        // A cast that breaks stealth does so as it goes; its first blow from stealth is a crit.
        let opener = self.skills.opens_from_stealth(skill, events);

        // Flurry of Blades: a chance of one more enemy in melee reach in front of the caster.
        let chance = crate::talents::extra_target_pct(c, &skill.key);
        let mut rider = None;
        if chance > 0 && rng.below(100) < chance {
            rider = self
                .mobs
                .iter()
                .filter(|m| {
                    m.attackable_by(self.guid)
                        && !targets.contains(&m.guid)
                        && distance(m.position, c.position) <= melee_reach()
                        && in_front(c.orientation, c.position, m.position)
                })
                .min_by(|a, b| {
                    distance(a.position, c.position).total_cmp(&distance(b.position, c.position))
                })
                .map(|m| m.guid);
            targets.extend(rider);
        }

        // Roll who it lands on first: `SMSG_SPELL_GO` lists the misses.
        let level = c.power_level();
        let landed: Vec<(u64, Landed)> = targets
            .iter()
            .map(|&t| (t, self.roll(skill, t, opener, c, rng)))
            .collect();
        let hits: Vec<u64> = landed
            .iter()
            .filter(|(_, l)| !matches!(l, Landed::Missed(_)))
            .map(|(t, _)| *t)
            .collect();
        let crits: Vec<(u64, Landed)> = landed
            .iter()
            .filter(|(_, l)| *l == Landed::Crit)
            .copied()
            .collect();
        let misses: Vec<(u64, u8)> = landed
            .iter()
            .filter_map(|(t, l)| match l {
                Landed::Missed(why) => Some((*t, *why)),
                _ => None,
            })
            .collect();
        self.skills.before = target.map(|t| {
            let auras = self
                .skills
                .auras
                .iter()
                .filter(|a| a.unit == t)
                .map(|a| a.spell_id)
                .collect();
            (self.skills.slowed(t), auras)
        });
        if instant {
            events.push(SessionEvent::SpellStart {
                caster: self.guid,
                spell_id,
                cast_flags: 0,
                cast_time_ms: 0,
                target,
                ammo_display_id: None,
            });
        }
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
            hits: hits.clone(),
            misses: misses.clone(),
            target,
            go_target: None,
            dest: None,
            ammo_display_id: None,
            item_caster: None,
        });
        for &(guid, why) in &misses {
            if why == SPELL_MISS_DODGE {
                self.skills
                    .dodged_by(guid, crate::callings::opens_on_dodge(c.calling), events);
            }
        }
        // Talents that give rage back on a skill with no rage effect of its own.
        let extra = crate::talents::extra_rage(c, &skill.key);
        let energizes = skill.effects.iter().any(|e| {
            matches!(
                e,
                Effect::Energize {
                    power: PowerKind::Rage,
                    ..
                }
            )
        });
        let any_landed = landed
            .iter()
            .any(|(_, how)| !matches!(how, Landed::Missed(_)));
        let mut spent = false;
        let mut finisher_target = None;
        // A ground skill's effects happen at its point, once.
        if skill.target == Target::Ground {
            for effect in &skill.effects {
                self.effect(
                    skill,
                    effect,
                    self.guid,
                    Landed::Hit,
                    level,
                    0,
                    c,
                    rng,
                    events,
                    &mut ticked,
                );
            }
        }
        for (guid, how) in landed {
            if !matches!(how, Landed::Missed(_)) {
                let combo = self.skills.combo.on(guid);
                for (i, effect) in skill.effects.iter().enumerate() {
                    // The rider's blow gives no combo point of its own.
                    if rider == Some(guid) && matches!(effect, Effect::AddComboPoints { .. }) {
                        continue;
                    }
                    // An effect that asks what the target wore before the cast (Frostbite).
                    if !self.target_held(skill, i, guid) {
                        continue;
                    }
                    // A Kick: the effects after it run only on a target whose cast it stopped.
                    if let Effect::Interrupt { lockout_ms, bosses } = *effect {
                        let stopped = self
                            .mobs
                            .iter_mut()
                            .find(|m| m.guid == guid && m.state != State::Dead)
                            .is_some_and(|mob| {
                                (bosses || !mob.template.raises_boss_bar())
                                    && super::kick_cast(mob, clock::secs(lockout_ms), events)
                            });
                        if !stopped {
                            break;
                        }
                        continue;
                    }
                    self.effect(
                        skill,
                        effect,
                        guid,
                        how,
                        level,
                        combo,
                        c,
                        rng,
                        events,
                        &mut ticked,
                    );
                }
                // The player's buffs that add a strike to each cast of the skill (Rapid Assault).
                for (weapon, weapon_pct) in self.skills.extra_strikes(skill) {
                    let strike = Effect::WeaponStrike {
                        weapon,
                        bonus: formulas::Linear {
                            base: 0.0,
                            per_level: 0.0,
                            per_ap: 0.0,
                            min: 0,
                        },
                        on_next_swing: false,
                        unavoidable: false,
                        per_combo: None,
                        controlled_pct: 0,
                        cleave_pct: 0,
                        below_health_pct: 0,
                        below_bonus_pct: 0,
                        block_value_pct: 0,
                        heal_pct: 0,
                        bleeding_pct: 0,
                        kill_heal_pct: 0,
                        kill_energize: 0,
                        repeat_chance: 0,
                        crit_cooldown_spell_id: None,
                        crit_cooldown_ms: 0,
                        crit_damage_pct: 0,
                        below_instant: false,
                        weapon_pct,
                        school: School::Physical,
                        crit_if_target_in: None,
                        armor_ignore: None,
                    };
                    self.effect(
                        skill,
                        &strike,
                        guid,
                        how,
                        level,
                        combo,
                        c,
                        rng,
                        events,
                        &mut ticked,
                    );
                }
                if skill.is_finisher() {
                    spent = true;
                    finisher_target.get_or_insert(guid);
                }
            }
            // A cast at a mob, landed or not, turns it on the player. A skill that needs an idle
            // target (Sap) leaves it idle, so no threat, no call for help and no pack aggro.
            if let Some(mob) = self.mobs.iter_mut().find(|m| m.guid == guid) {
                if mob.state == State::Idle && !skill.requires_target_out_of_combat {
                    super::engage(mob, self.guid, events);
                }
            }
        }
        for effect in &skill.effects {
            if let Effect::ConsumeAuraStacks {
                aura_spell_id,
                per_stack,
                school,
                heal_pct,
                refund_per_stack,
            } = *effect
            {
                let stacks = self.skills.take_stacks(self.guid, aura_spell_id, events);
                if stacks == 0 {
                    continue;
                }
                // Power back for every stack taken, landed or not (Tithe of Zeal).
                self.energize_player(skill.spell_id, refund_per_stack * stacks, events);
                let ap = buffed(&self.player.stats, &self.skills).attack_power;
                let pct = crate::talents::amount_pct(c, &skill.key);
                let each = scaled(per_stack.at(level, ap), pct) * stacks;
                let mut dealt = 0;
                for &guid in &hits {
                    dealt += self.bonus_hit(
                        skill.spell_id,
                        guid,
                        each,
                        school,
                        c,
                        rng,
                        events,
                        &mut ticked,
                    );
                }
                let heal =
                    dealt * (heal_pct + crate::talents::consume_heal_pct(c, &skill.key)) / 100;
                if heal > 0 {
                    self.heal_player(c, skill.spell_id, heal, events);
                }
            }
        }
        if extra > 0 && !energizes && any_landed {
            self.gain_rage(skill.spell_id, extra, events);
        }
        if any_landed {
            let gone = self.skills.spend_charges(Some(skill), events);
            self.faded(&gone, events);
        }
        if self.skills.cast_dealt > 0 {
            if let Some(&first) = hits.first() {
                self.proc(
                    crate::rules::procs::Happened {
                        was_ms: 0,
                        event: crate::callings::ProcEvent::DamageDone,
                        skill: Some(&skill.key),
                        plain: skill.replaces.as_deref(),
                        yields_xp: false,
                        target: Some(first),
                        amount: self.skills.cast_dealt,
                        school: Some(skill_school(skill)),
                    },
                    c,
                    rng,
                    events,
                    &mut ticked,
                );
            }
        }
        // The next-cast buffs this cast used, landed or not.
        let gone = self.skills.use_next_casts(skill, events);
        self.faded(&gone, events);
        if let Some(&(first, _)) = crits.first() {
            self.proc(
                crate::rules::procs::Happened {
                    event: crate::callings::ProcEvent::Crit,
                    skill: Some(&skill.key),
                    plain: skill.replaces.as_deref(),
                    yields_xp: false,
                    target: Some(first),
                    amount: 0,
                    school: Some(skill_school(skill)),
                    was_ms: 0,
                },
                c,
                rng,
                events,
                &mut ticked,
            );
        }
        self.proc(
            crate::rules::procs::Happened {
                was_ms: 0,
                event: crate::callings::ProcEvent::Cast,
                skill: Some(&skill.key),
                plain: skill.replaces.as_deref(),
                yields_xp: false,
                target: None,
                amount: 0,
                school: None,
            },
            c,
            rng,
            events,
            &mut ticked,
        );
        if spent {
            // A finisher that landed spends every point.
            self.skills.combo.clear();
            self.skills.send_combo(events);
            // Its procs come after, so a point they add survives the spend.
            self.proc(
                crate::rules::procs::Happened {
                    was_ms: 0,
                    event: crate::callings::ProcEvent::Finisher,
                    skill: Some(&skill.key),
                    plain: skill.replaces.as_deref(),
                    yields_xp: false,
                    target: finisher_target,
                    amount: 0,
                    school: None,
                },
                c,
                rng,
                events,
                &mut ticked,
            );
        }
        let p = &self.player;
        events.push(values(
            self.guid,
            &[(UNIT_FIELD_HEALTH, p.health), p.power.field()],
        ));
        self.update_aura_states(events);
        self.sync_next_cast_mods(c, events);
        self.skills.cast_target = None;
        // A strike that landed may go off again at once, free; a repeat never repeats (Twin
        // Arrows).
        let repeat = skill.effects.iter().find_map(|e| match *e {
            Effect::WeaponStrike { repeat_chance, .. } if repeat_chance > 0 => Some(repeat_chance),
            _ => None,
        });
        if let (Some(chance), Some(again), true) = (repeat, target, any_landed) {
            if !self.skills.repeating && rng.below(100) < chance {
                self.free_cast(skill, again, true, c, rng, events, &mut ticked);
            }
        }
        ticked
    }

    /// Whether effect `i` of `skill` lands on `target`: what its `requires_target_aura` asks, read
    /// off the cast's target as it stood before the cast landed, and off any other target as it
    /// stands now (#506).
    fn target_held(&self, skill: &Skill, i: usize, target: u64) -> bool {
        let Some(Some(wanted)) = skill.requires_target_aura.get(i).copied() else {
            return true;
        };
        let before = self
            .skills
            .before
            .as_ref()
            .filter(|_| self.skills.cast_target == Some(target));
        match (wanted, before) {
            (callings::TargetAura::Slowed, Some((slowed, _))) => *slowed,
            (callings::TargetAura::Aura(id), Some((_, auras))) => auras.contains(&id),
            (callings::TargetAura::Slowed, None) => self.skills.slowed(target),
            (callings::TargetAura::Aura(id), None) => self.skills.on(target, id),
        }
    }

    /// What a share is of: the cast under way's totals, or the caster's maximum mana or health.
    fn of_pct(&self, of: callings::PctOf) -> u32 {
        match of {
            callings::PctOf::MaxMana => buffed(&self.player.stats, &self.skills).max_mana,
            callings::PctOf::MaxHealth => buffed(&self.player.stats, &self.skills).max_health,
            _ => self.skills.of(of),
        }
    }

    /// `pct` percent of what the cast under way dealt or healed (`pct_of`), a tick's share over
    /// `duration_ms` in `tick_ms` ticks; 0 with no share.
    fn share(
        &self,
        pct_of: Option<callings::PctOf>,
        pct: u32,
        duration_ms: u32,
        tick_ms: u32,
    ) -> u32 {
        let Some(of) = pct_of else {
            return 0;
        };
        let amount = self.of_pct(of);
        let ticks = (duration_ms / tick_ms.max(1)).max(1);
        scaled_share(amount, pct) / ticks
    }

    /// The friendly players in the world, each with where they stand and the share of their
    /// health they have left: the player, and the other living players on the shard, all one
    /// party (#149).
    fn friendly_players(&self, c: &Char) -> Vec<(u64, [f32; 3], f32)> {
        if !self.player.alive() {
            return Vec::new();
        }
        let mut all = vec![(self.guid, c.position, self.health_share())];
        all.extend(self.others.iter().copied());
        all
    }

    /// The share of the player's health they have left.
    /// This player as their stand-in in a foe's session shows them: level, health and armour.
    pub(crate) fn stand_in_stats(&self) -> (crate::rules::stats::MobStats, u32) {
        let s = buffed(&self.player.stats, &self.skills);
        let stats = crate::rules::stats::MobStats {
            level: self.player.stats.level,
            max_health: s.max_health,
            armor: s.armor,
            min_damage: 0.0,
            max_damage: 0.0,
        };
        (stats, self.player.health)
    }

    /// The fight with the player `foe` is over: this player stops swinging at them and casting
    /// at them, the auras of `spells` that `foe` put on this player come off, and `foe`'s
    /// stand-in goes.
    pub(crate) fn part_from(&mut self, foe: u64, spells: &[u32], events: &mut Vec<SessionEvent>) {
        if self.player.attacking == Some(foe) {
            self.player.attacking = None;
            events.push(SessionEvent::AttackStop {
                attacker: self.guid,
                victim: foe,
            });
        }
        if self
            .skills
            .casting
            .is_some_and(|c| c.skill == Some(Some(foe)))
        {
            self.interrupt_cast(events);
        }
        let me = self.guid;
        let gone = self
            .skills
            .remove_where(|a| a.unit == me && spells.contains(&a.spell_id), events);
        self.faded(&gone, events);
        if !gone.is_empty() {
            events.push(values(me, &self.sheet_now()));
            self.update_speed(me, events);
        }
        self.mobs.retain(|m| !(m.remote && m.guid == foe));
        self.foe = None;
        self.beaten_by = None;
    }

    pub(crate) fn health_share(&self) -> f32 {
        let max = buffed(&self.player.stats, &self.skills).max_health.max(1);
        self.player.health as f32 / max as f32
    }

    /// The friendly player within `radius_yd` of `at` with the smallest share of health left.
    fn lowest_health_ally(&self, c: &Char, at: [f32; 3], radius_yd: f32) -> Option<u64> {
        self.friendly_players(c)
            .into_iter()
            .filter(|(_, p, _)| distance(*p, at) <= radius_yd)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(guid, _, _)| guid)
    }

    /// The percent more healing the player receives: their buffs', and their talents' at the
    /// health they have now out of `max`.
    fn healing_taken(&self, c: &Char, max: u32) -> u32 {
        let health_pct = self.player.health * 100 / max.max(1);
        self.skills.healing_taken_pct(self.guid) + crate::talents::healing_taken_pct(c, health_pct)
    }

    /// `total` healing split evenly among the friendly players within `radius_yd` of `at`.
    pub(super) fn heal_allies(
        &mut self,
        c: &Char,
        at: [f32; 3],
        radius_yd: f32,
        total: u32,
        spell_id: u32,
        events: &mut Vec<SessionEvent>,
    ) {
        let allies: Vec<u64> = self
            .friendly_players(c)
            .into_iter()
            .filter(|(_, p, _)| distance(*p, at) <= radius_yd)
            .map(|(guid, _, _)| guid)
            .collect();
        if allies.is_empty() || total == 0 {
            return;
        }
        let each = total / allies.len() as u32;
        for guid in allies {
            if guid == self.guid {
                self.heal_player(c, spell_id, each, events);
            } else {
                self.helps.push(super::Help::Heal {
                    to: guid,
                    healer: self.guid,
                    spell_id,
                    amount: each,
                    burst: None,
                });
            }
        }
    }

    /// A copy of the aura whose tick is running its procs, with its time left and the stacks the
    /// source aura holds after this tick's growth (the tick's own if the source is gone), goes
    /// on the living enemy nearest its unit within `radius_yd` that does not already have it.
    pub(super) fn spread_ticking(&mut self, radius_yd: f32, events: &mut Vec<SessionEvent>) {
        let Some(tick) = self.skills.ticking else {
            return;
        };
        let Some(at) = self
            .mobs
            .iter()
            .find(|m| m.guid == tick.unit)
            .map(|m| m.position)
        else {
            return;
        };
        let Some(other) = enemies_within(&self.mobs, self.guid, at, radius_yd)
            .filter(|m| m.guid != tick.unit && !self.skills.on(m.guid, tick.spell_id))
            .min_by(|a, b| distance(a.position, at).total_cmp(&distance(b.position, at)))
            .map(|m| m.guid)
        else {
            return;
        };
        let mut copy = tick;
        copy.unit = other;
        copy.slot = 0;
        if let AuraKind::PeriodicDamage { caster, .. } = tick.kind {
            let held = self.skills.auras.iter().find_map(|a| match a.kind {
                AuraKind::PeriodicDamage {
                    caster: c, stacks, ..
                } if a.unit == tick.unit && a.spell_id == tick.spell_id && c == caster => {
                    Some(stacks)
                }
                _ => None,
            });
            if let (Some(held), AuraKind::PeriodicDamage { stacks, .. }) = (held, &mut copy.kind) {
                *stacks = held;
            }
        }
        self.skills.apply(copy, events);
    }

    /// What the party frames show of the player (#149): health, power, level, zone and place.
    pub(crate) fn party_stats(&self, c: &Char) -> benilla_protocol::messages::PartyMemberStatsInfo {
        use benilla_protocol::messages::member_status;
        let p = &self.player;
        let max = buffed(&p.stats, &self.skills).max_health;
        let mut status = member_status::ONLINE;
        if !p.alive() {
            status |= member_status::DEAD;
        }
        let clamp = |v: u32| v.min(u32::from(u16::MAX)) as u16;
        benilla_protocol::messages::PartyMemberStatsInfo {
            status: Some(status),
            cur_hp: Some(clamp(p.health)),
            max_hp: Some(clamp(max)),
            power_type: Some(p.power.kind.id() as u8),
            cur_power: Some(clamp(p.power.current)),
            max_power: Some(clamp(p.power.max)),
            level: Some(u16::from(c.level)),
            zone: Some(clamp(c.zone)),
            position: Some((c.position[0] as i16, c.position[1] as i16)),
            ..Default::default()
        }
    }

    /// Whom an ally skill lands on (#150): the other living player targeted, in its range; the
    /// caster when nothing friendly is targeted (vanilla's auto self-cast).
    fn ally(&self, skill: &Skill, target: Option<u64>, c: &Char) -> Result<u64, u8> {
        let Some(g) = target.filter(|&g| g != self.guid && crate::shard::is_player(g)) else {
            return Ok(self.guid);
        };
        let Some(&(_, at, _)) = self.others.iter().find(|o| o.0 == g) else {
            return Err(SPELL_FAILED_TARGETS_DEAD);
        };
        let [_, hi] = skill.range_yd;
        if hi > 0.0 && distance(c.position, at) > hi {
            return Err(SPELL_FAILED_OUT_OF_RANGE);
        }
        Ok(g)
    }

    /// The auras this session put on other players since it was last asked.
    pub(crate) fn take_sent(&mut self) -> Vec<Aura> {
        std::mem::take(&mut self.skills.sent)
    }

    /// Wears an aura another player put on this one, and shows what it changes.
    pub(crate) fn wear(&mut self, aura: Aura, events: &mut Vec<SessionEvent>) {
        let mut aura = aura;
        aura.unit = self.guid;
        self.skills.apply(aura, events);
        events.push(values(self.guid, &self.sheet_now()));
        self.update_speed(self.guid, events);
    }

    /// Heals the player `amount` under `spell_id`, up to their maximum: what was healed.
    fn heal_player(
        &mut self,
        c: &Char,
        spell_id: u32,
        amount: u32,
        events: &mut Vec<SessionEvent>,
    ) -> u32 {
        let healer = self.guid;
        self.heal_from(healer, c, spell_id, amount, events)
    }

    /// Heals the player `amount` of `healer`'s `spell_id`, up to their maximum: what was healed.
    pub(crate) fn heal_from(
        &mut self,
        healer: u64,
        c: &Char,
        spell_id: u32,
        amount: u32,
        events: &mut Vec<SessionEvent>,
    ) -> u32 {
        self.heal_full_from(healer, c, spell_id, amount, events).0
    }

    /// [`Self::heal_from`]: what was healed, and what would have been with no maximum to stop
    /// it (#508).
    pub(crate) fn heal_full_from(
        &mut self,
        healer: u64,
        c: &Char,
        spell_id: u32,
        amount: u32,
        events: &mut Vec<SessionEvent>,
    ) -> (u32, u32) {
        let max = buffed(&self.player.stats, &self.skills).max_health;
        let amount = scaled(amount, self.healing_taken(c, max));
        let p = &mut self.player;
        if !p.alive() {
            return (0, 0);
        }
        let healed = amount.min(max.saturating_sub(p.health));
        p.health += healed;
        events.push(SessionEvent::SpellHealLog(
            benilla_protocol::messages::SpellHealLog {
                target: self.guid,
                healer,
                spell_id,
                amount: healed,
                critical: false,
            },
        ));
        events.push(values(self.guid, &[(UNIT_FIELD_HEALTH, p.health)]));
        (healed, amount)
    }

    /// The player's shield of `spell_id` grows by `more`, up to `max_pct` percent of their maximum
    /// health; when it grew, it lasts `duration_ms` from now (Gorging Siphon).
    fn grow_shield(
        &mut self,
        spell_id: u32,
        more: u32,
        max_pct: u32,
        duration_ms: u32,
        level: u8,
        events: &mut Vec<SessionEvent>,
    ) {
        let cap = buffed(&self.player.stats, &self.skills).max_health * max_pct / 100;
        let now = self.skills.shield_left(spell_id).unwrap_or(0);
        let grown = (now + more).min(cap);
        if grown > now {
            self.shield(spell_id, grown, duration_ms, None, level, events);
        }
    }

    /// Sends the spell-mod cells the player's `next_cast` buffs move, each with the talents'
    /// value added (a packet sets a cell outright), and puts back the ones they no longer move.
    pub(super) fn sync_next_cast_mods(&mut self, c: &Char, events: &mut Vec<SessionEvent>) {
        let now = self.skills.next_cast_mods();
        if now == self.skills.sent_mods {
            return;
        }
        let gone: Vec<(bool, u8, u8, i32)> = self
            .skills
            .sent_mods
            .iter()
            .filter(|(f, b, o, _)| !now.iter().any(|n| (n.0, n.1, n.2) == (*f, *b, *o)))
            .map(|&(f, b, o, _)| (f, b, o, 0))
            .collect();
        for &(flat, mask_bit, op, value) in now.iter().chain(&gone) {
            events.push(SessionEvent::SpellModifier {
                flat,
                mask_bit,
                op,
                value: value + crate::talents::spell_mod_value(c, flat, mask_bit, op),
            });
        }
        self.skills.sent_mods = now;
    }

    /// What `skill` costs now: the talents' price, less what the player's buffs take off.
    fn power_cost(&self, c: &Char, skill: &Skill) -> u32 {
        // The row's price is a share of base mana for `pct_base`, and health for `health`.
        let base = match skill.meta.shape.cost_kind.as_deref() {
            Some("pct_base") => {
                let s = &self.player.stats;
                crate::rules::power::base_mana(s.class, s.level) * skill.power_cost / 100
            }
            Some("health") | Some("health_current") => 0,
            _ => skill.power_cost,
        };
        let cost = crate::talents::power_cost_from(c, skill, base)
            .saturating_sub(self.skills.cost_less(skill));
        (cost as i32 * (100 + self.skills.next_cost_pct(skill)) / 100) as u32
    }

    /// The health `skill` costs: its row's share of the caster's maximum health, for a
    /// `cost_kind` of `health`.
    fn health_cost(&self, skill: &Skill) -> u32 {
        match skill.meta.shape.cost_kind.as_deref() {
            Some("health") => {
                buffed(&self.player.stats, &self.skills).max_health * skill.power_cost / 100
            }
            // A share of the health left, which never kills.
            Some("health_current") => (self.player.health * skill.power_cost / 100)
                .min(self.player.health.saturating_sub(1)),
            _ => 0,
        }
    }

    /// Whether `skill` can go off now at `target`, and on whom; `gcd`: whether the global
    /// cooldown holds it back.
    fn check(
        &self,
        skill: &Skill,
        target: Option<u64>,
        gcd: bool,
        c: &Char,
    ) -> Result<Vec<u64>, u8> {
        let p = &self.player;
        if !p.alive() {
            return Err(SPELL_FAILED_CASTER_DEAD);
        }
        let free = self.skills.free;
        if !free
            && (self.skills.cooldown_left(skill.spell_id) > READY_SLACK_SECS
                || (gcd && skill.gcd_ms > 0 && self.skills.gcd > READY_SLACK_SECS))
        {
            return Err(SPELL_FAILED_NOT_READY);
        }
        if !free && !p.power.has(self.power_cost(c, skill)) {
            return Err(SPELL_FAILED_NO_POWER);
        }
        // A health cost that would kill is refused.
        let blood = self.health_cost(skill);
        if blood > 0 && blood >= p.health {
            return Err(SPELL_FAILED_NO_POWER);
        }
        // A shot needs a bow, gun, crossbow or thrown weapon in the ranged slot.
        let wants_ranged = skill.wants_ranged();
        if wants_ranged && p.stats.ranged.is_none() {
            return Err(SPELL_FAILED_EQUIPPED_ITEM_CLASS);
        }
        if wants_ranged && !crate::ammo::ready(c) {
            return Err(SPELL_FAILED_NO_AMMO);
        }
        if skill.requires_out_of_combat && p.in_combat {
            return Err(SPELL_FAILED_AFFECTING_COMBAT);
        }
        if skill.requires_stealth && !self.skills.stealthed() {
            return Err(SPELL_FAILED_ONLY_STEALTHED);
        }
        if skill.is_finisher() && target.is_none_or(|t| self.skills.combo.on(t) == 0) {
            return Err(SPELL_FAILED_NO_COMBO_POINTS);
        }
        // Judgement needs a Seal to judge.
        if skill.effects.contains(&Effect::Judgement) && self.skills.seal().is_none() {
            return Err(SPELL_FAILED_CASTER_AURASTATE);
        }
        match skill.target {
            Target::Caster => Ok(vec![self.guid]),
            Target::Ally => self.ally(skill, target, c).map(|g| vec![g]),
            // A point on the ground, within the skill's range.
            Target::Ground => {
                let dest = self.skills.dest.ok_or(SPELL_FAILED_BAD_TARGETS)?;
                let [_, hi] = skill.range_yd;
                if hi > 0.0 && distance(dest, c.position) > hi {
                    return Err(SPELL_FAILED_OUT_OF_RANGE);
                }
                Ok(Vec::new())
            }
            Target::SelfArea => {
                Ok(
                    enemies_within(&self.mobs, self.guid, c.position, skill.radius_yd)
                        .take(skill.max_targets as usize)
                        .map(|m| m.guid)
                        .collect(),
                )
            }
            Target::Enemy => {
                let guid = target.ok_or(SPELL_FAILED_BAD_TARGETS)?;
                let i = self
                    .mobs
                    .iter()
                    .position(|m| m.guid == guid)
                    .ok_or(SPELL_FAILED_BAD_TARGETS)?;
                let mob = &self.mobs[i];
                if mob.template.friendly {
                    return Err(SPELL_FAILED_BAD_TARGETS);
                }
                if !mob.alive() {
                    return Err(SPELL_FAILED_TARGETS_DEAD);
                }
                // A creature this character is not an Audience of is out of reach, as if far off.
                if !mob.admits(self.guid) {
                    return Err(SPELL_FAILED_OUT_OF_RANGE);
                }
                let d = distance(mob.position, c.position);
                let [min, max] = skill.range_yd;
                let ranged = skill.meta.shape.defense_type.as_deref() == Some("ranged");
                let min = if crate::talents::no_min_range(c, &skill.key, ranged) {
                    0.0
                } else {
                    min
                };
                // A 5 yd skill is a melee one: it reaches as far as a swing does.
                let max = if max <= 5.0 { melee_reach() } else { max };
                if d > max {
                    return Err(SPELL_FAILED_OUT_OF_RANGE);
                }
                if d < min {
                    return Err(SPELL_FAILED_TOO_CLOSE);
                }
                if !in_front(c.orientation, c.position, mob.position) {
                    return Err(SPELL_FAILED_UNIT_NOT_INFRONT);
                }
                if skill.requires_target_dodged
                    && !self.skills.dodged.is_some_and(|(m, _)| m == guid)
                {
                    return Err(SPELL_FAILED_CASTER_AURASTATE);
                }
                if skill.requires_target_out_of_combat && !matches!(mob.state, State::Idle) {
                    return Err(SPELL_FAILED_TARGET_AFFECTING_COMBAT);
                }
                if skill.requires_target_health_pct > 0
                    && u64::from(mob.health) * 100
                        > u64::from(mob.stats.max_health)
                            * u64::from(skill.requires_target_health_pct)
                {
                    return Err(SPELL_FAILED_TARGET_AURASTATE);
                }
                let mut targets = vec![guid];
                targets.extend(self.beside(skill, i, c));
                Ok(targets)
            }
        }
    }

    /// The enemies a sheet skill with an area shape lands on besides its target `first` (an index
    /// into `mobs`): the nearest within `radius_yd` of the shape's centre, up to `max_targets`
    /// in all. A cone also keeps to the caster's front half-circle.
    fn beside(&self, skill: &Skill, first: usize, c: &Char) -> Vec<u64> {
        let shape = &skill.meta.shape;
        let Some(area) = shape.area_shape.as_deref() else {
            return Vec::new();
        };
        let centre = match shape.area_center.as_deref() {
            Some("caster") => c.position,
            _ => self.mobs[first].position,
        };
        // A 5 yd reach is a melee one, as for the target's range.
        let radius = if skill.radius_yd <= 5.0 {
            melee_reach()
        } else {
            skill.radius_yd
        };
        let mut near: Vec<_> = self
            .mobs
            .iter()
            .enumerate()
            .filter(|&(j, m)| {
                j != first
                    && m.attackable_by(self.guid)
                    && distance(m.position, centre) <= radius
                    && (area != "cone" || in_front(c.orientation, c.position, m.position))
            })
            .map(|(_, m)| m)
            .collect();
        near.sort_by(|a, b| distance(a.position, centre).total_cmp(&distance(b.position, centre)));
        near.into_iter()
            .take((skill.max_targets as usize).saturating_sub(1))
            .map(|m| m.guid)
            .collect()
    }

    /// Takes the cast's costs and starts its cooldowns.
    fn pay(&mut self, skill: &Skill, gcd: bool, c: &Char, events: &mut Vec<SessionEvent>) {
        // A free cast pays nothing and starts no cooldown.
        if self.skills.free {
            return;
        }
        let cost = self.power_cost(c, skill);
        self.skills.last_cost = cost;
        self.skills.was_full = self.player.power.current >= self.player.power.max;
        let blood = self.health_cost(skill);
        let p = &mut self.player;
        p.power.spend(cost);
        let health_cost = p.stats.base_health * skill.health_cost_pct_of_base_health / 100 + blood;
        self.skills.last_health_cost = health_cost.min(p.health.saturating_sub(1));
        // A cost never kills.
        p.health = p.health.saturating_sub(health_cost).max(1);
        let s = &mut self.skills;
        if skill.cooldown_ms > 0 {
            s.cooldowns.retain(|(id, _)| *id != skill.spell_id);
            s.cooldowns
                .push((skill.spell_id, clock::secs(skill.cooldown_ms)));
        }
        if gcd && skill.gcd_ms > 0 {
            s.gcd = clock::secs(skill.gcd_ms);
        }
        if skill.puts_in_combat_ms > 0 {
            s.combat_hold = s.combat_hold.max(clock::secs(skill.puts_in_combat_ms));
        }
        if skill.requires_target_dodged {
            s.close_dodge_window(events);
        }
    }

    /// The melee table's verdict on `skill` at `target`: a strike or a hit-table skill rolls
    /// avoidance, then crit (the two-roll yellow model); anything else lands.
    /// An `opener` (the first blow from stealth) skips the table and is a critical strike.
    fn roll(&self, skill: &Skill, target: u64, opener: bool, c: &Char, rng: &mut Rng) -> Landed {
        let mut rolls = false;
        let mut unavoidable = false;
        let mut crit_if = None;
        let mut mana_crit_pct = 0.0;
        for e in &skill.effects {
            if let Effect::SchoolDamage {
                crit_pct_at_full_mana: Some(pct),
                ..
            } = *e
            {
                mana_crit_pct += pct;
            }
            match *e {
                Effect::WeaponStrike {
                    unavoidable: u,
                    crit_if_target_in,
                    ..
                } => {
                    rolls = true;
                    unavoidable |= u;
                    crit_if = crit_if.or(crit_if_target_in);
                }
                Effect::SchoolDamage {
                    melee_hit_table: true,
                    ..
                }
                | Effect::Execute { .. } => rolls = true,
                // Magic misses on the level-based chance alone.
                Effect::SchoolDamage { school, .. } | Effect::Drain { school, .. }
                    if school.is_magic() =>
                {
                    rolls = true;
                    unavoidable = true;
                }
                _ => {}
            }
        }
        let Some(mob) = self.mobs.iter().find(|m| m.guid == target) else {
            return Landed::Hit;
        };
        if !rolls {
            return Landed::Hit;
        }
        let stands_in = match crit_if {
            Some(callings::CritIf::GroundEffect) => {
                self.skills.in_ground_effect(self.guid, mob.position)
            }
            None => false,
        };
        if opener || stands_in {
            return Landed::Crit;
        }
        let stats = buffed(&self.player.stats, &self.skills);
        let from_front = in_front(mob.orientation, mob.position, c.position);
        // The mana-fill crit, in basis points; only a mana bar has a fill to read.
        let bar = &self.player.power;
        let mana_crit = if bar.kind == PowerKind::Mana && bar.max > 0 {
            (mana_crit_pct * 100.0 * bar.current as f32 / bar.max as f32) as u32
        } else {
            0
        };
        let (table, crit) = ability_table(
            PlayerOffense {
                crit_pct: stats.crit_pct,
            },
            mob.defense(),
            from_front,
            unavoidable,
        );
        let ranged = skill.meta.shape.defense_type.as_deref() == Some("ranged");
        let (table, crit) = if ranged {
            let more = self.skills.ranged_crit_pct() * 100;
            (formulas::ranged_table(table, from_front), crit + more)
        } else {
            (table, crit)
        };
        match table.outcome(rng.below(ROLL)) {
            Outcome::Miss => Landed::Missed(SPELL_MISS_MISS),
            Outcome::Dodge => Landed::Missed(SPELL_MISS_DODGE),
            Outcome::Parry => Landed::Missed(SPELL_MISS_PARRY),
            Outcome::Block => Landed::Missed(SPELL_MISS_BLOCK),
            // A talent's extra chance for this skill (Improved Overpower), in basis points.
            // The player's buffs for the skill (Cutthroat's Momentum) add to it.
            _ if rng.below(ROLL)
                < crit
                    + mana_crit
                    + (crate::talents::skill_crit_pct(c, &skill.key)
                        + self.skills.skill_crit_pct(skill))
                        * 100 =>
            {
                Landed::Crit
            }
            _ => Landed::Hit,
        }
    }

    /// One effect of a landed cast on `target`.
    #[allow(clippy::too_many_arguments)]
    fn effect(
        &mut self,
        skill: &Skill,
        effect: &Effect,
        target: u64,
        how: Landed,
        level: u8,
        combo: u8,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let stats = buffed(&self.player.stats, &self.skills);
        let ap = stats.attack_power;
        // Talents that raise the skill's amounts (Improved Rend, Improved Battle Shout), and the
        // player's buffs for it (Arcane Momentum).
        let pct = crate::talents::amount_pct(c, &skill.key) + self.skills.skill_amount_pct(skill);
        let sized = |l: formulas::Linear| scaled(l.at(level, ap), pct);
        // A hit's and a tick's own talents, some only against a target wearing the caster's aura
        // (Combustion Engine, Volatile Embers).
        let wears = |key: &str| {
            crate::callings::of(c.calling)
                .auras
                .iter()
                .find(|a| a.key == key)
                .is_some_and(|a| self.skills.on(target, a.spell_id))
        };
        let part_pct = |part| {
            crate::talents::amount_pct_on(c, &skill.key, part, wears)
                + self.skills.skill_amount_pct(skill)
        };
        let (hit_pct, tick_pct) = (
            part_pct(crate::callings::AppliesTo::Hit),
            part_pct(crate::callings::AppliesTo::Tick),
        );
        // Worn Spell Power (#272): a direct magic hit or heal takes the cast-time share, a
        // periodic one the duration share per tick. A skill with both parts gives the direct
        // part the whole direct share and the periodic part half its own.
        let direct_bonus = crate::rules::spell_power::spell_power_bonus(
            stats.spell_power,
            skill.meta.shape.cast_time_ms,
        );
        let has_direct = skill.effects.iter().any(|e| {
            matches!(e, Effect::Heal { .. })
                || matches!(e, Effect::SchoolDamage { school, .. } if school.is_magic())
        });
        let periodic_bonus = |duration_ms: u32, tick_ms: u32| {
            let ticks = if tick_ms == 0 {
                0
            } else {
                duration_ms / tick_ms
            };
            let bonus = crate::rules::spell_power::periodic_spell_power_bonus(
                stats.spell_power,
                duration_ms,
                ticks,
            );
            if has_direct {
                bonus / 2
            } else {
                bonus
            }
        };
        // A finisher's block, once per combo point held on the target.
        let per_combo = |l: Option<formulas::Linear>| l.map_or(0, |l| sized(l) * u32::from(combo));
        // Talents that make the skill's auras last longer (Booming Voice).
        let longer = crate::talents::duration_pct(c, &skill.key);
        // ... and by a flat time (Bloodletting), and a next-cast buff's (Arcane Substrate).
        let later = (crate::talents::duration_ms(c, &skill.key) as i32
            + self.skills.next_duration_ms(skill))
        .max(0) as u32;
        let aura = |kind: AuraKind, spell_id: u32, duration_ms: u32, period_ms: u32| Aura {
            unit: target,
            spell_id,
            slot: 0,
            level,
            left: clock::secs(scaled(duration_ms + later, longer)),
            period: clock::secs(period_ms),
            next_tick: clock::secs(period_ms),
            kind,
            charges: 0,
        };
        match *effect {
            Effect::WeaponStrike {
                weapon,
                bonus,
                per_combo: more,
                weapon_pct,
                school,
                armor_ignore,
                controlled_pct,
                cleave_pct,
                below_health_pct,
                below_bonus_pct,
                block_value_pct,
                heal_pct,
                bleeding_pct,
                kill_heal_pct,
                kill_energize,
                crit_cooldown_spell_id,
                crit_cooldown_ms,
                crit_damage_pct,
                ..
            } => {
                let cond = self.condition_pct(
                    target,
                    controlled_pct,
                    below_health_pct,
                    below_bonus_pct,
                    bleeding_pct,
                );
                let worth = self.kill_yields_xp(target);
                let roll = weapon_roll(&stats, weapon, rng) * weapon_pct as f32 / 100.0;
                let shield = stats.block_value
                    * (block_value_pct + crate::talents::block_value_pct(c, &skill.key))
                    / 100;
                // The talents and buffs for the skill raise the whole strike (Guerilla
                // Tactics), some only close in (Point Blank).
                let near = self.mobs.iter().find(|m| m.guid == target).map_or(0, |m| {
                    let d = distance(m.position, c.position);
                    crate::talents::amount_pct_near(c, &skill.key, d)
                });
                let raw = roll + (bonus.at(level, ap) + per_combo(more) + shield) as f32;
                let raw = raw * (100 + pct + near + cond) as f32 / 100.0;
                let ignore = armor_ignore.map_or(0, |l| l.at(level, ap))
                    + crate::talents::skill_armor_ignore(c, &skill.key);
                let dealt = self.strike_through(
                    skill.spell_id,
                    target,
                    raw as u32,
                    school,
                    how,
                    ignore,
                    crit_damage_pct,
                    c,
                    rng,
                    events,
                    ticked,
                );
                self.skills.cast_dealt += dealt;
                // A crit cuts another skill's cooldown, the version set on its row (Adrenaline
                // Rush).
                if let (Landed::Crit, Some(id)) = (how, crit_cooldown_spell_id) {
                    if let Some(cooled) = crate::talents::current_of(c, id) {
                        let ms = -(crit_cooldown_ms as i32);
                        self.skills.change_cooldown(cooled.spell_id, ms, events);
                    }
                }
                if dealt > 0 && heal_pct > 0 {
                    self.heal_player(c, skill.spell_id, dealt * heal_pct / 100, events);
                }
                let reward = (kill_heal_pct, kill_energize);
                self.killing_blow(c, target, worth, reward, skill.spell_id, events);
                if dealt > 0 && weapon != Weapon::Ranged {
                    self.melee_hit(target, c, rng, events, ticked);
                }
                if dealt > 0 && cleave_pct > 0 {
                    let share = dealt * cleave_pct / 100;
                    self.cleave(
                        target,
                        share,
                        school,
                        skill.spell_id,
                        c,
                        rng,
                        events,
                        ticked,
                    );
                }
            }
            Effect::SchoolDamage {
                school,
                amount,
                per_combo: more,
                controlled_pct,
                bleeding_pct,
                kill_heal_pct,
                kill_energize,
                vs_aura_spell_id,
                vs_aura_pct,
                vs_aura_per_stack,
                vs_aura_max_pct,
                splash_pct,
                splash_yd,
                ..
            } => {
                let raw = scaled(amount.at(level, ap), hit_pct)
                    + per_combo(more)
                    + if school.is_magic() { direct_bonus } else { 0 };
                // More against a target wearing the caster's aura (Combustion Engine), or so much
                // a stack of it, to a most (Exsanguinate), beside the other conditions.
                let worn = match vs_aura_spell_id {
                    Some(id) if vs_aura_per_stack => {
                        (vs_aura_pct * self.skills.stacks_on(target, id)).min(vs_aura_max_pct)
                    }
                    Some(id) if self.skills.on(target, id) => vs_aura_pct,
                    _ => 0,
                };
                let cond = self.condition_pct(target, controlled_pct, 0, 0, bleeding_pct) + worn;
                let raw = scaled(raw, cond);
                let worth = self.kill_yields_xp(target);
                let at = self
                    .mobs
                    .iter()
                    .find(|m| m.guid == target)
                    .map(|m| m.position);
                let dealt = self.strike(
                    skill.spell_id,
                    target,
                    raw,
                    school,
                    how,
                    c,
                    rng,
                    events,
                    ticked,
                );
                self.skills.cast_dealt += dealt;
                let reward = (kill_heal_pct, kill_energize);
                self.killing_blow(c, target, worth, reward, skill.spell_id, events);
                // The enemies around the target take a share, which the cast's tally leaves out
                // (Splintering Lance).
                if let (true, Some(at)) = (dealt > 0 && splash_pct > 0, at) {
                    let share = dealt * splash_pct / 100;
                    self.splash(
                        skill.spell_id,
                        target,
                        at,
                        splash_yd,
                        share,
                        school,
                        c,
                        rng,
                        events,
                        ticked,
                    );
                }
            }
            Effect::PeriodicDamage {
                tick_ms,
                duration_ms,
                tick,
                per_combo: more,
                duration_per_combo: stretch,
                max_stacks,
                aura_spell_id,
                school,
                pct_of,
                pct,
                leech_pct,
                stack_rule,
                stacks_per_tick,
                if_worn,
                drop_chance,
                tick_aura_spell_id,
                tick_aura_chance,
                tick_aura_on,
                weaken_pct,
                tick_energize_pct_of_max,
                tick_energize_pct_of_base,
                tick_spread_chance,
                tick_spread_yd,
                tick_aura_chance_per_stack,
                weaken_per_stack_pct,
                weaken_max_pct,
            } => {
                let riders = TickRiders {
                    aura: tick_aura_spell_id
                        .and_then(|id| callings::of(c.calling).aura_by_spell(id)),
                    aura_chance: tick_aura_chance,
                    aura_chance_per_stack: tick_aura_chance_per_stack,
                    aura_on_target: tick_aura_on == callings::TickAuraOn::Target,
                    energize_pct_of_max: tick_energize_pct_of_max,
                    energize_pct_of_base: tick_energize_pct_of_base,
                    weaken_pct,
                    spread_chance: tick_spread_chance,
                    spread_yd: tick_spread_yd,
                    weaken_per_stack_pct,
                    weaken_max_pct,
                };
                let aura_id = aura_spell_id.unwrap_or(skill.spell_id);
                if if_worn == Some(IfWorn::Grow) && self.skills.grow(target, aura_id, 1, events) {
                    return;
                }
                let duration_ms =
                    duration_ms + stretch.map_or(0, |l| l.at(level, ap) * u32::from(combo));
                let share = self.share(pct_of, pct, duration_ms, tick_ms);
                let per_tick = scaled(tick.at(level, ap), tick_pct)
                    + per_combo(more)
                    + share
                    + if school.is_magic() {
                        periodic_bonus(duration_ms, tick_ms)
                    } else {
                        0
                    };
                let a = aura(
                    AuraKind::PeriodicDamage {
                        per_tick,
                        carry: 0,
                        school,
                        caster: self.guid,
                        stacks: 1,
                        max_stacks,
                        leech_pct,
                        rule: stack_rule,
                        stacks_per_tick,
                        drop_chance,
                        riders,
                    },
                    aura_id,
                    duration_ms,
                    tick_ms,
                );
                self.skills.apply(a, events);
            }
            Effect::Charge { speed_yd_s } => {
                let Some(mob) = self.mobs.iter().find(|m| m.guid == target) else {
                    return;
                };
                let (from, to) = (c.position, mob.position);
                let d = distance(from, to);
                let t = ((d - CHARGE_STOP_YD) / d).max(0.0);
                let dest = std::array::from_fn(|i| from[i] + (to[i] - from[i]) * t);
                let duration = distance(from, dest) / speed_yd_s;
                c.position = dest;
                c.orientation = (to[1] - from[1])
                    .atan2(to[0] - from[0])
                    .rem_euclid(std::f32::consts::TAU);
                events.push(SessionEvent::MonsterMove {
                    guid: self.guid,
                    transport: None,
                    start: from,
                    spline_id: 1,
                    path: vec![from, dest],
                    facing: MonsterMoveFacing::Target(target),
                    stop: false,
                    duration_ms: clock::ms(duration),
                    flying: false,
                    run_mode: true,
                });
            }
            Effect::Energize {
                power,
                amount,
                pct_of_max,
            } => {
                // A power the caster's bar does not hold: nothing happens, nothing is logged.
                if self.player.power.kind != power {
                    return;
                }
                let bar = &self.player.power;
                let mut gained =
                    amount.at(level, ap) + bar.max / power.stored(1) * pct_of_max / 100;
                if power == PowerKind::Rage {
                    gained += crate::talents::extra_rage(c, &skill.key);
                }
                self.player.power.gain(power, gained);
                events.push(SessionEvent::SpellEnergizeLog(SpellEnergizeLog {
                    target: self.guid,
                    caster: self.guid,
                    spell_id: skill.spell_id,
                    power: power.id(),
                    amount: self.player.power.earned(power, gained),
                }));
            }
            Effect::PeriodicRage {
                aura_spell_id,
                per_tick,
                tick_ms,
                duration_ms,
            } => {
                let a = aura(
                    AuraKind::Rage { per_tick },
                    aura_spell_id,
                    duration_ms,
                    tick_ms,
                );
                self.skills.apply(a, events);
            }
            Effect::Stun {
                aura_spell_id,
                duration_ms,
                per_combo: more,
            } => {
                let duration_ms = duration_ms + per_combo(more);
                let duration_ms = if self.is_diminished_mob(target) {
                    self.skills.control(target, ControlKind::Stun, duration_ms)
                } else {
                    Some(duration_ms)
                };
                if let Some(duration_ms) = duration_ms {
                    self.skills
                        .apply(aura(AuraKind::Stun, aura_spell_id, duration_ms, 0), events);
                    self.interrupt_mob_cast(target, events);
                }
            }
            Effect::Incapacitate {
                duration_ms,
                break_on_damage,
            } => {
                // Damage in the same cast came first, so it does not wake the target.
                // A boss takes nothing, and no clock starts. A duel's stand-in is a player, and
                // takes the full duration.
                let alive = self.mobs.iter().any(|m| m.guid == target && m.alive());
                let boss = self
                    .mobs
                    .iter()
                    .any(|m| m.guid == target && !m.remote && m.template.raises_boss_bar());
                let duration_ms = if !alive || boss {
                    None
                } else if self.is_diminished_mob(target) {
                    self.skills
                        .control(target, ControlKind::Incapacitate, duration_ms)
                } else {
                    Some(duration_ms)
                };
                if let Some(duration_ms) = duration_ms {
                    let kind = AuraKind::Incapacitate { break_on_damage };
                    self.skills
                        .apply(aura(kind, skill.spell_id, duration_ms, 0), events);
                    self.interrupt_mob_cast(target, events);
                    // The caster stops swinging at it; the mob stays in the fight (#165) but
                    // cannot act on the caster, so the caster is out of combat (#184).
                    if self.player.attacking == Some(target) {
                        self.player.attacking = None;
                        events.push(SessionEvent::AttackStop {
                            attacker: self.guid,
                            victim: target,
                        });
                    }
                }
            }
            // Handled in the per-target loop of the cast, which decides whether the effects after
            // it run.
            Effect::Interrupt { .. } => {}
            Effect::Knockback { distance_yd } => {
                let hit = self
                    .mobs
                    .iter()
                    .any(|m| m.guid == target && m.alive() && !m.template.raises_boss_bar());
                if hit {
                    self.skills.knocks.push(PendingKnock {
                        mob: target,
                        from: c.position,
                        distance_yd,
                    });
                }
            }
            Effect::Taunt { duration_ms } => {
                let me = self.guid;
                if let Some(mob) = self
                    .mobs
                    .iter_mut()
                    .find(|m| m.guid == target && m.alive() && !m.remote)
                {
                    if mob.state == State::Idle {
                        super::engage(mob, me, events);
                    }
                    mob.threat.raise_to_top(me);
                    mob.taunted = Some((me, clock::secs(duration_ms)));
                }
            }
            Effect::AddComboPoints {
                amount,
                bleeding_bonus,
            } => {
                let bleeding = self.bleeding(target);
                // A talent's chance of one more point, more so on a bleeding target.
                let chance = crate::talents::bonus_combo_pct(c, &skill.key, bleeding);
                let more = u32::from(chance > 0 && rng.below(100) < chance);
                // The skill's own points more on a bleeding target (Serrated Strikes).
                let bonus = if bleeding { bleeding_bonus } else { 0 };
                self.skills.combo.add(target, amount + more + bonus);
                self.skills.send_combo(events);
            }
            Effect::AttackSpeedSlow { pct, duration_ms } => {
                let a = aura(AuraKind::AttackSlow { pct }, skill.spell_id, duration_ms, 0);
                self.skills.apply(a, events);
            }
            Effect::MovementSlow { pct, duration_ms } => {
                let a = aura(AuraKind::MoveSlow { pct }, skill.spell_id, duration_ms, 0);
                self.skills.apply(a, events);
                self.update_speed(target, events);
            }
            Effect::ModRunSpeed {
                pct,
                duration_ms,
                aura_spell_id,
                radius_yd,
            } => {
                let kind = run_speed(pct);
                let a = aura(
                    kind,
                    aura_spell_id.unwrap_or(skill.spell_id),
                    duration_ms,
                    0,
                );
                // The same snare on every other living enemy near the target (Splintering Cold).
                let at = self
                    .mobs
                    .iter()
                    .find(|m| m.guid == target)
                    .map(|m| m.position);
                let around: Vec<u64> = match at {
                    Some(at) if radius_yd > 0.0 => {
                        enemies_within(&self.mobs, self.guid, at, radius_yd)
                            .filter(|m| m.guid != target)
                            .map(|m| m.guid)
                            .collect()
                    }
                    _ => Vec::new(),
                };
                for unit in std::iter::once(target).chain(around) {
                    self.skills.apply(Aura { unit, ..a }, events);
                    self.update_speed(unit, events);
                }
            }
            Effect::AuraAttackPower {
                duration_ms,
                amount,
            } => {
                let n = sized(amount);
                let a = aura(AuraKind::AttackPower(n), skill.spell_id, duration_ms, 0);
                self.skills.apply(a, events);
                let sheet = buff_sheet(&self.player.stats, &self.skills);
                events.push(values(self.guid, &sheet));
            }
            Effect::DodgePct { pct, duration_ms } => {
                let mut a = aura(AuraKind::Dodge { pct }, skill.spell_id, duration_ms, 0);
                a.unit = self.guid;
                self.skills.apply(a, events);
                events.push(values(self.guid, &self.sheet_now()));
            }
            Effect::ParryPct { .. }
            | Effect::BlockPct { .. }
            | Effect::BlockValuePct { .. }
            | Effect::ArmorItemsPct { .. } => {
                let (stat, pct, duration_ms) = AuraKind::defense(effect).expect("a defense effect");
                let kind = AuraKind::Defense { stat, pct };
                let mut a = aura(kind, skill.spell_id, duration_ms, 0);
                a.unit = self.guid;
                self.skills.apply(a, events);
                events.push(values(self.guid, &self.sheet_now()));
            }
            Effect::Stealth {
                duration_ms,
                opener_crit,
            } => {
                let mut a = aura(
                    AuraKind::Stealth { opener_crit },
                    skill.spell_id,
                    duration_ms,
                    0,
                );
                if duration_ms == 0 {
                    a.left = UNTIL_BROKEN_SECS;
                }
                a.unit = self.guid;
                self.skills.apply(a, events);
            }
            Effect::HealingReduction { duration_ms, .. } => {
                let a = aura(AuraKind::HealingReduced, skill.spell_id, duration_ms, 0);
                self.skills.apply(a, events);
            }
            Effect::Execute {
                base,
                per_extra_rage,
            } => {
                // The cost is paid; every rage point left goes into the blow.
                let extra = self.player.power.take_all(PowerKind::Rage);
                let raw = sized(base) + extra * sized(per_extra_rage);
                self.strike(
                    skill.spell_id,
                    target,
                    raw,
                    School::Physical,
                    how,
                    c,
                    rng,
                    events,
                    ticked,
                );
            }
            Effect::AuraMaxHealthPct {
                aura_spell_id,
                pct,
                duration_ms,
            } => {
                let before = buffed(&self.player.stats, &self.skills).max_health;
                let a = aura(AuraKind::MaxHealthPct(pct), aura_spell_id, duration_ms, 0);
                self.skills.apply(a, events);
                let after = buffed(&self.player.stats, &self.skills).max_health;
                self.player.health += after.saturating_sub(before);
                events.push(values(self.guid, &self.sheet_now()));
            }
            Effect::AuraDamagePct {
                pct,
                armor_loss_pct,
                duration_ms,
            } => {
                let kind = AuraKind::DamagePct {
                    pct,
                    armor_loss_pct,
                    swings: None,
                };
                self.skills
                    .apply(aura(kind, skill.spell_id, duration_ms, 0), events);
                events.push(values(self.guid, &self.sheet_now()));
            }
            Effect::SweepingStrikes {
                strikes,
                duration_ms,
                radius_yd,
                strike_spell_id,
            } => {
                let kind = AuraKind::Sweeping {
                    strikes,
                    radius_yd,
                    strike_spell_id,
                };
                self.skills
                    .apply(aura(kind, skill.spell_id, duration_ms, 0), events);
            }
            Effect::Seal { duration_ms, seal } => {
                // One Seal at a time: the new one replaces any other.
                let me = self.guid;
                let gone = self.skills.remove_where(
                    |a| {
                        a.unit == me
                            && a.spell_id != skill.spell_id
                            && matches!(a.kind, AuraKind::Seal { .. })
                    },
                    events,
                );
                self.faded(&gone, events);
                // Talents that raise the Seal's amounts (Improved Seal of the Crusader) raise its
                // attack power and its judgement's holy damage taken alike.
                let mut raised = seal;
                let attack_power = match &mut raised {
                    Seal::Crusader {
                        attack_power,
                        holy_taken_pct,
                        ..
                    } => {
                        *holy_taken_pct = scaled(*holy_taken_pct, pct);
                        sized(*attack_power)
                    }
                    Seal::Command { .. } => 0,
                };
                let kind = AuraKind::Seal {
                    seal: raised,
                    attack_power,
                };
                self.skills
                    .apply(aura(kind, skill.spell_id, duration_ms, 0), events);
                let sheet = buff_sheet(&self.player.stats, &self.skills);
                events.push(values(self.guid, &sheet));
            }
            Effect::AddAuraStacks { .. } => {
                let more = self.skills.skill_stacks(skill);
                self.add_aura_stacks(effect, more, level, events)
            }
            Effect::GrowAura {
                aura_spell_id,
                stacks,
                radius_yd,
            } => {
                self.skills.grow(target, aura_spell_id, stacks, events);
                // And on each enemy around the target that wears it (Splintering Lance).
                let at = self
                    .mobs
                    .iter()
                    .find(|m| m.guid == target)
                    .map(|m| m.position);
                if let (true, Some(at)) = (radius_yd > 0.0, at) {
                    let around: Vec<u64> = enemies_within(&self.mobs, self.guid, at, radius_yd)
                        .filter(|m| m.guid != target)
                        .map(|m| m.guid)
                        .collect();
                    for unit in around {
                        self.skills.grow(unit, aura_spell_id, stacks, events);
                    }
                }
            }
            Effect::GrantAura { aura_spell_id } => {
                let auras = &callings::of(c.calling).auras;
                // Not the skill's own crowd control, which may share the spell (Blindside).
                let control = |a: &callings::AuraDef| {
                    a.effects.iter().any(|e| {
                        matches!(
                            e,
                            callings::AuraEffect::Incapacitate | callings::AuraEffect::Root
                        )
                    })
                };
                if let Some(def) = auras
                    .iter()
                    .find(|a| a.spell_id == aura_spell_id && !control(a))
                {
                    self.grant_aura(def, level, 0, events);
                }
            }
            Effect::GroundEffect {
                radius_yd,
                tick_ms,
                duration_ms,
                school,
                amount,
                ground_spell_id,
                caster_aura_spell_id,
            } => {
                // Where the cast ended: after a Charge, the caster stands at its destination.
                let more = crate::talents::ground_amount_pct(c, &skill.key);
                let longer = crate::talents::ground_duration_ms(c, &skill.key);
                let ground = Ground {
                    object: self.skills.next_patch(self.guid),
                    caster: self.guid,
                    spell_id: ground_spell_id.unwrap_or(skill.spell_id),
                    centre: c.position,
                    radius_yd,
                    school,
                    damage: scaled(sized(amount), more),
                    left: clock::secs(duration_ms + longer),
                    period: clock::secs(tick_ms),
                    next_tick: clock::secs(tick_ms),
                    caster_aura: caster_aura_spell_id
                        .and_then(|id| callings::of(c.calling).aura_by_spell(id)),
                    caster_inside: false,
                };
                let old: Vec<Ground> = self
                    .skills
                    .grounds
                    .iter()
                    .filter(|g| g.caster == ground.caster)
                    .copied()
                    .collect();
                for gone in &old {
                    self.ward_off(gone, events);
                    events.push(SessionEvent::ObjectDestroyed(gone.object));
                }
                self.skills.grounds.retain(|g| g.caster != ground.caster);
                events.push(patch_create(
                    ground.object,
                    ground.caster,
                    ground.spell_id,
                    ground.centre,
                    ground.radius_yd,
                ));
                self.skills.grounds.push(ground);
                self.ground_ward(c, events);
            }
            // Takes the stacks once for every target: `cast` does it after the loop.
            Effect::ConsumeAuraStacks { .. } => {}
            Effect::Absorb {
                pct,
                pct_of,
                duration_ms,
                radius_yd,
                school,
                if_crit,
                energize_pct,
                attacker_aura_spell_id,
                burst_radius_yd,
                burst_school,
            } => {
                let amount = scaled_share(self.of_pct(pct_of), pct);
                // Only off a critical strike, and then only when larger than the one up (Warding
                // Spark).
                if if_crit
                    && (how != Landed::Crit
                        || self
                            .skills
                            .shield_left(skill.spell_id)
                            .is_some_and(|left| left >= amount))
                {
                    return;
                }
                let ward = Ward {
                    amount,
                    energize_pct,
                    attacker_aura: attacker_aura_spell_id
                        .and_then(|id| callings::of(c.calling).aura_by_spell(id)),
                    burst_radius_yd,
                    burst_school,
                };
                let allies: Vec<u64> = self
                    .friendly_players(c)
                    .into_iter()
                    .filter(|(_, p, _)| distance(*p, c.position) <= radius_yd)
                    .map(|(guid, _, _)| guid)
                    .collect();
                for guid in allies {
                    // Only the caster is shielded yet; a friend in the radius gets none.
                    if guid == self.guid {
                        self.warded_shield(
                            skill.spell_id,
                            amount,
                            duration_ms,
                            school,
                            ward,
                            level,
                            events,
                        );
                    }
                }
            }
            Effect::Root {
                aura_spell_id,
                duration_ms,
            } => {
                self.skills
                    .apply(aura(AuraKind::Root, aura_spell_id, duration_ms, 0), events);
                if let Some(mob) = self.mobs.iter_mut().find(|m| m.guid == target) {
                    events.extend(mob.halt());
                }
            }
            Effect::RefundCost {
                chance,
                pct,
                full_absorb_pct,
                absorb_duration_ms,
            } => {
                if rng.below(100) < chance {
                    self.refund_cost(
                        skill.spell_id,
                        pct,
                        full_absorb_pct,
                        absorb_duration_ms,
                        level,
                        events,
                    );
                }
            }
            Effect::GroundArea {
                radius_yd,
                duration_ms,
                enemy_aura_spell_id,
                ally_aura_spell_id,
                arm_ms,
                first_only,
                grow_aura_spell_id,
                grow_every_ms,
                follow_caster,
            } => {
                let of = callings::of(c.calling);
                let row = |id: Option<u32>| id.and_then(|id| of.aura_by_spell(id));
                let every = clock::secs(grow_every_ms);
                let grow = grow_aura_spell_id
                    .filter(|_| grow_every_ms > 0)
                    .map(|id| (id, every, every));
                let area = Area {
                    object: self.skills.next_patch(self.guid),
                    caster: self.guid,
                    spell_id: skill.spell_id,
                    // A skill that is not aimed at the ground drops it at the caster's feet, not
                    // at an earlier cast's destination.
                    centre: if skill.target == Target::Ground {
                        self.skills.dest.unwrap_or(c.position)
                    } else {
                        c.position
                    },
                    radius_yd,
                    left: clock::secs(duration_ms),
                    arming: clock::secs(arm_ms),
                    first_only,
                    enemy_aura: row(enemy_aura_spell_id),
                    ally_aura: row(ally_aura_spell_id),
                    inside: Vec::new(),
                    grow,
                    follow: follow_caster,
                };
                // A new one from the same caster replaces the old.
                let old: Vec<Area> = self
                    .skills
                    .areas
                    .iter()
                    .filter(|a| a.caster == area.caster && a.spell_id == area.spell_id)
                    .cloned()
                    .collect();
                for gone in &old {
                    self.leave_area(gone, &gone.inside, events);
                    events.push(SessionEvent::ObjectDestroyed(gone.object));
                }
                self.skills
                    .areas
                    .retain(|a| !(a.caster == area.caster && a.spell_id == area.spell_id));
                events.push(patch_create(
                    area.object,
                    area.caster,
                    area.spell_id,
                    area.centre,
                    area.radius_yd,
                ));
                self.skills.areas.push(area);
                self.area_tick(0.0, c, events);
            }
            Effect::Drain {
                amount,
                heal_pct,
                school,
                tick_grow_aura_spell_id,
                tick_grow_chance,
                overheal_absorb_max_pct,
                overheal_absorb_ms,
            } => {
                let dealt = self.strike(
                    skill.spell_id,
                    target,
                    sized(amount),
                    school,
                    how,
                    c,
                    rng,
                    events,
                    ticked,
                );
                self.skills.cast_dealt += dealt;
                if dealt > 0 && heal_pct > 0 {
                    let asked = scaled_share(dealt, heal_pct);
                    let healed = self.heal_player(c, skill.spell_id, asked, events);
                    // What it could not heal grows a shield (Gorging Siphon).
                    if overheal_absorb_max_pct > 0 && asked > healed {
                        self.grow_shield(
                            skill.spell_id,
                            asked - healed,
                            overheal_absorb_max_pct,
                            overheal_absorb_ms,
                            level,
                            events,
                        );
                    }
                }
                // A tick that hits may grow the caster's damage over time on the target (Feeding
                // Siphon); nothing without it.
                if let (true, Some(id)) = (dealt > 0, tick_grow_aura_spell_id) {
                    if rng.below(100) < tick_grow_chance {
                        self.skills.grow(target, id, 1, events);
                    }
                }
            }
            Effect::Heal {
                amount,
                overheal_burst_yd,
                overheal_burst_school,
            } => {
                let amount = sized(amount) + direct_bonus;
                let burst =
                    (overheal_burst_yd > 0.0).then_some((overheal_burst_yd, overheal_burst_school));
                if target == self.guid {
                    let (healed, full) = self.heal_bursting(
                        self.guid,
                        skill.spell_id,
                        amount,
                        burst,
                        c,
                        rng,
                        events,
                        ticked,
                    );
                    self.skills.cast_healed += healed;
                    self.skills.cast_full_healed += full;
                } else if crate::shard::is_player(target) {
                    // Their session heals them, and bursts what it could not heal.
                    self.skills.cast_full_healed += amount;
                    self.helps.push(super::Help::Heal {
                        to: target,
                        healer: self.guid,
                        spell_id: skill.spell_id,
                        amount,
                        burst,
                    });
                }
            }
            Effect::PeriodicHeal {
                tick_ms,
                duration_ms,
                tick,
                pct_of,
                pct,
                aura_spell_id,
                ally,
                radius_yd,
                center,
            } => {
                let unit = match ally {
                    Some(callings::AllyPick::LowestHealth) => {
                        let at = match center {
                            callings::Center::Caster => Some(c.position),
                            callings::Center::Target => self
                                .mobs
                                .iter()
                                .find(|m| m.guid == target)
                                .map(|m| m.position)
                                .or((target == self.guid).then_some(c.position)),
                        };
                        at.and_then(|at| self.lowest_health_ally(c, at, radius_yd))
                    }
                    None => crate::shard::is_player(target).then_some(target),
                };
                let Some(unit) = unit else {
                    return;
                };
                let per_tick = tick.map_or(0, sized)
                    + self.share(pct_of, pct, scaled(duration_ms + later, longer), tick_ms)
                    + periodic_bonus(duration_ms, tick_ms);
                if per_tick == 0 {
                    return;
                }
                let mut a = aura(
                    AuraKind::Heal { per_tick },
                    aura_spell_id.unwrap_or(skill.spell_id),
                    duration_ms,
                    tick_ms,
                );
                a.unit = unit;
                self.skills.apply(a, events);
            }
            Effect::ResetCooldowns { spell_ids } => {
                for id in spell_ids.into_iter().filter(|&id| id != 0) {
                    let s = &mut self.skills;
                    if s.cooldowns.iter().any(|(s, _)| *s == id) {
                        s.cooldowns.retain(|(s, _)| *s != id);
                        events.push(SessionEvent::ClearCooldown {
                            spell_id: id,
                            caster: self.guid,
                        });
                    }
                }
            }
            Effect::Blink {
                distance_yd,
                backward,
                shed_snares,
            } => {
                let me = self.guid;
                let gone = self.skills.remove_where(
                    |a| {
                        a.unit == me
                            && (matches!(
                                a.kind,
                                AuraKind::Stun | AuraKind::Incapacitate { .. } | AuraKind::Root
                            ) || (shed_snares && matches!(a.kind, AuraKind::MoveSlow { .. })))
                    },
                    events,
                );
                // A shed snare gives the run speed back (Unbound Leap).
                self.faded(&gone, events);
                let back = if backward { std::f32::consts::PI } else { 0.0 };
                self.skills.blink = Some(PendingBlink {
                    from: c.position,
                    orientation: (c.orientation + back).rem_euclid(std::f32::consts::TAU),
                    faces: c.orientation,
                    distance_yd,
                });
            }
            Effect::CastSkill { skill_spell_id } => {
                // At the cast's enemy target, or the unit the player has selected (a self cast
                // carries none); it goes off once any Blink has landed (Parting Shot).
                let at = self.skills.cast_target.filter(|&t| t != self.guid).or((self
                    .skills
                    .selection
                    != 0)
                    .then_some(self.skills.selection));
                self.skills.follow_ups.push(FollowUp {
                    skill_spell_id,
                    target: at,
                });
            }
            Effect::ConsumeAura {
                aura_spell_id,
                duration_ms,
                amount,
                school,
                amount_pct,
                energize_pct_of_max,
                per_stack,
                heal_allies_pct,
                radius_yd,
                spread_yd,
            } => {
                let at = self
                    .mobs
                    .iter()
                    .find(|m| m.guid == target)
                    .map(|m| m.position);
                let Some(gone) = self.skills.take_aura(target, aura_spell_id, events) else {
                    return;
                };
                if spread_yd > 0.0 {
                    // A fresh copy on another enemy near the target (Catalytic Chain).
                    self.skills.consumed = Some((gone, duration_ms));
                    self.spread_consumed(target, spread_yd, events);
                    self.skills.consumed = None;
                }
                let stacks = if per_stack { gone.kind.stacks() } else { 1 };
                self.update_speed(target, events);
                // Talents that raise the consume's outcomes (Thermal Shock).
                let more = crate::talents::consume_amount_pct(c, &skill.key);
                if let Some(amount) = amount {
                    let raw = scaled_share(amount.at(level, ap), amount_pct) * stacks;
                    // The shatter crits on the caster's chance and the skill's own (Elemental
                    // Convergence's buff).
                    let chance = (stats.crit_pct * 100.0) as u32
                        + (crate::talents::skill_crit_pct(c, &skill.key)
                            + self.skills.skill_crit_pct(skill))
                            * 100;
                    let how = if rng.below(ROLL) < chance {
                        Landed::Crit
                    } else {
                        Landed::Hit
                    };
                    let dealt = self.strike(
                        skill.spell_id,
                        target,
                        scaled(raw, more),
                        school,
                        how,
                        c,
                        rng,
                        events,
                        ticked,
                    );
                    self.skills.cast_dealt += dealt;
                    if heal_allies_pct > 0 {
                        if let Some(at) = at {
                            let total = scaled_share(dealt, heal_allies_pct);
                            self.heal_allies(c, at, radius_yd, total, skill.spell_id, events);
                        }
                    }
                }
                let p = &mut self.player;
                let kind = p.power.kind;
                let gain = scaled(
                    scaled_share(p.power.max / kind.stored(1), energize_pct_of_max),
                    more,
                );
                if energize_pct_of_max > 0 && p.alive() && p.power.gain(kind, gain) {
                    events.push(SessionEvent::SpellEnergizeLog(SpellEnergizeLog {
                        target: self.guid,
                        caster: self.guid,
                        spell_id: skill.spell_id,
                        power: kind.id(),
                        amount: gain,
                    }));
                    events.push(values(self.guid, &[p.power.field()]));
                }
                self.skills.consumed = Some((gone, duration_ms));
                self.proc(
                    crate::rules::procs::Happened {
                        was_ms: 0,
                        event: crate::callings::ProcEvent::Consume,
                        skill: Some(&skill.key),
                        plain: skill.replaces.as_deref(),
                        yields_xp: false,
                        target: Some(target),
                        amount: 0,
                        school: None,
                    },
                    c,
                    rng,
                    events,
                    ticked,
                );
                self.skills.consumed = None;
            }
            Effect::Judgement => {
                let me = self.guid;
                let gone = self.skills.remove_where(
                    |a| a.unit == me && matches!(a.kind, AuraKind::Seal { .. }),
                    events,
                );
                self.faded(&gone, events);
                let judged = gone.iter().find_map(|a| match a.kind {
                    AuraKind::Seal { seal, .. } => Some(seal),
                    _ => None,
                });
                match judged {
                    Some(Seal::Command {
                        judgement_spell_id,
                        judgement,
                        ..
                    }) => {
                        let raw = sized(judgement);
                        self.holy_hit(judgement_spell_id, target, raw, c, rng, events, ticked);
                    }
                    Some(Seal::Crusader {
                        judgement_spell_id,
                        judgement_duration_ms,
                        holy_taken_pct,
                        ..
                    }) => {
                        let kind = AuraKind::HolyTaken {
                            pct: holy_taken_pct,
                            duration_ms: judgement_duration_ms,
                        };
                        let a = aura(kind, judgement_spell_id, judgement_duration_ms, 0);
                        self.skills.apply(a, events);
                    }
                    None => {}
                }
            }
        }
    }

    /// A white swing of the player's landed on `target`: its judgement lasts afresh, and the
    /// active Seal procs (Seal of Command's holy hit).
    pub(super) fn seal_swing_landed(
        &mut self,
        target: u64,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        self.skills.refresh_judgement(target);
        let Some(Seal::Command {
            procs_per_minute,
            weapon_pct,
            proc_spell_id,
            ..
        }) = self.skills.seal()
        else {
            return;
        };
        if !self.mobs.iter().any(|m| m.guid == target && m.alive()) {
            return;
        }
        let stats = white(&self.player.stats, &self.skills);
        if rng.below(ROLL) >= seals::ppm_chance(procs_per_minute, stats.attack_time_ms) {
            return;
        }
        let roll = weapon_roll(&stats, Weapon::Normal, rng);
        let raw = seals::weapon_pct_damage(roll, weapon_pct);
        self.holy_hit(proc_spell_id, target, raw, c, rng, events, ticked);
    }

    /// `raw` holy damage at `target`: no armour stops it. Logged as holy, taken off, maybe a
    /// kill.
    #[allow(clippy::too_many_arguments)]
    fn holy_hit(
        &mut self,
        spell_id: u32,
        target: u64,
        raw: u32,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let Some(mob) = self.mobs.iter().find(|m| m.guid == target) else {
            return;
        };
        let school = School::Holy;
        let damage = mitigated(
            school,
            raw,
            mob.stats.armor.saturating_sub(self.player.stats.armor_pen),
            self.player.stats.level,
        );
        let damage = seals::holy_taken(damage, self.skills.holy_taken_pct(target));
        events.push(SessionEvent::SpellDamageLog(damage_log(
            self.guid, target, spell_id, damage, school, false,
        )));
        self.hurt(target, damage, c, rng, events, ticked);
    }

    /// A bonus hit of `raw` in `school` at `target` that cannot miss or crit, logged under
    /// `spell_id`: the damage dealt.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn bonus_hit(
        &mut self,
        spell_id: u32,
        target: u64,
        raw: u32,
        school: School,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) -> u32 {
        let Some(mob) = self.mobs.iter().find(|m| m.guid == target && m.alive()) else {
            return 0;
        };
        let mut damage = mitigated(
            school,
            raw,
            mob.stats.armor.saturating_sub(self.player.stats.armor_pen),
            self.player.stats.level,
        );
        if school == School::Holy {
            damage = seals::holy_taken(damage, self.skills.holy_taken_pct(target));
        }
        let damage = self.dealt(c, damage);
        events.push(SessionEvent::SpellDamageLog(damage_log(
            self.guid, target, spell_id, damage, school, false,
        )));
        self.hurt(target, damage, c, rng, events, ticked);
        damage
    }

    /// `share` of `school` damage at every other living enemy within `radius_yd` of `at`, where
    /// `target` stood when it was hit, each logged under `spell_id`: a splash (a talent proc's,
    /// Splintering Lance's).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn splash(
        &mut self,
        spell_id: u32,
        target: u64,
        at: [f32; 3],
        radius_yd: f32,
        share: u32,
        school: School,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let around: Vec<u64> = enemies_within(&self.mobs, self.guid, at, radius_yd)
            .filter(|m| m.guid != target)
            .map(|m| m.guid)
            .collect();
        for guid in around {
            self.bonus_hit(spell_id, guid, share, school, c, rng, events, ticked);
        }
    }

    /// The landed hit of an `AddAuraStacks` effect: the caster's aura takes its stacks, or `more`
    /// when a buff raises them past that.
    pub(super) fn add_aura_stacks(
        &mut self,
        effect: &Effect,
        more: u32,
        level: u8,
        events: &mut Vec<SessionEvent>,
    ) {
        let Effect::AddAuraStacks {
            aura_spell_id,
            stacks,
            duration_ms,
            max_stacks,
            stack_rule,
        } = *effect
        else {
            return;
        };
        let aura = Aura {
            unit: self.guid,
            spell_id: aura_spell_id,
            slot: 0,
            level,
            left: clock::secs(duration_ms),
            period: 0.0,
            next_tick: 0.0,
            kind: AuraKind::Stacks {
                count: stacks.max(more),
                max: max_stacks.max(1),
                rule: stack_rule,
            },
            charges: 0,
        };
        self.skills.apply(aura, events);
    }

    /// `raw` damage at `target`, after its armour: logged, taken off, maybe a kill; what landed.
    #[allow(clippy::too_many_arguments)]
    fn strike(
        &mut self,
        spell_id: u32,
        target: u64,
        raw: u32,
        school: School,
        how: Landed,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) -> u32 {
        self.strike_through(
            spell_id, target, raw, school, how, 0, 0, c, rng, events, ticked,
        )
    }

    /// [`Self::strike`] skipping `armor_ignore` more of the target's armour, never below none, and
    /// a crit dealing `crit_more_pct` percent more on top of every other crit bonus (Piercing
    /// Force).
    #[allow(clippy::too_many_arguments)]
    fn strike_through(
        &mut self,
        spell_id: u32,
        target: u64,
        raw: u32,
        school: School,
        how: Landed,
        armor_ignore: u32,
        crit_more_pct: u32,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) -> u32 {
        let Some(mob) = self.mobs.iter().find(|m| m.guid == target) else {
            return 0;
        };
        let armor_ignore = armor_ignore + self.skills.armor_loss(target);
        let armor = mob
            .stats
            .armor
            .saturating_sub(self.player.stats.armor_pen + armor_ignore);
        let more = crate::talents::damage_pct_vs(c, school, |key| {
            callings::aura(key).is_some_and(|a| self.skills.on(target, a.spell_id))
        });
        let raw = scaled(raw, more);
        let mut damage = mitigated(school, raw, armor, self.player.stats.level);
        if how == Landed::Crit {
            let spell = crate::talents::skill(c, spell_id);
            let builder = spell.is_some_and(Skill::adds_combo_points);
            damage = crit_scaled(
                school,
                damage,
                crate::talents::skill_crit_damage_pct(c, spell.map(|s| s.key.as_str()), builder)
                    + self.player.stats.crit_damage_pct,
            );
            damage = (u64::from(damage) * u64::from(100 + crit_more_pct) / 100) as u32;
        }
        damage = self.dealt(c, damage);
        if school.is_magic() {
            damage = scaled(damage, self.skills.spell_damage_pct());
        }
        events.push(SessionEvent::SpellDamageLog(damage_log(
            self.guid,
            target,
            spell_id,
            damage,
            school,
            how == Landed::Crit,
        )));
        self.hurt(target, damage, c, rng, events, ticked);
        if how == Landed::Crit {
            self.crit_landed(target, damage, c, events);
        }
        // A blow at one enemy sweeps on; an area's blows do not.
        if !school.is_magic()
            && crate::talents::skill(c, spell_id).is_some_and(|s| s.target == Target::Enemy)
        {
            self.sweep(target, damage, c, rng, events, ticked);
        }
        damage
    }

    /// The player's melee blow landed on `target`: the procs on a melee hit, and a Well Fed
    /// damage stack grows.
    pub(super) fn melee_hit(
        &mut self,
        target: u64,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        self.fed_stack_hit();
        self.proc(
            crate::rules::procs::Happened {
                was_ms: 0,
                event: crate::callings::ProcEvent::MeleeHit,
                skill: None,
                plain: None,
                yields_xp: false,
                target: Some(target),
                amount: 0,
                school: None,
            },
            c,
            rng,
            events,
            ticked,
        );
    }

    /// `damage` the player deals, raised by their talents for the weapon in hand and by their
    /// buffs, then by a Well Fed damage stack.
    pub(super) fn dealt(&self, c: &Char, damage: u32) -> u32 {
        let pct = crate::talents::weapon_damage_pct(c) + self.skills.damage_pct();
        damage * (100 + pct) / 100 * (100 + self.damage_stack) / 100
    }

    /// A melee blow of the player's landed `damage` on `from`: under Sweeping Strikes, the
    /// nearest other living enemy in reach takes as much.
    pub(super) fn sweep(
        &mut self,
        from: u64,
        damage: u32,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let me = self.guid;
        let Some(i) = self
            .skills
            .auras
            .iter()
            .position(|a| a.unit == me && matches!(a.kind, AuraKind::Sweeping { .. }))
        else {
            return;
        };
        let AuraKind::Sweeping {
            strikes,
            radius_yd,
            strike_spell_id,
        } = self.skills.auras[i].kind
        else {
            return;
        };
        let at = c.position;
        let other = enemies_within(&self.mobs, self.guid, at, radius_yd)
            .filter(|m| m.guid != from)
            .min_by(|a, b| distance(a.position, at).total_cmp(&distance(b.position, at)))
            .map(|m| m.guid);
        let Some(other) = other.filter(|_| damage > 0) else {
            return;
        };
        if strikes <= 1 {
            let gone = self.skills.remove_where(
                |a| a.unit == me && matches!(a.kind, AuraKind::Sweeping { .. }),
                events,
            );
            self.faded(&gone, events);
        } else if let AuraKind::Sweeping { strikes, .. } = &mut self.skills.auras[i].kind {
            *strikes -= 1;
        }
        events.push(SessionEvent::SpellDamageLog(damage_log(
            me,
            other,
            strike_spell_id,
            damage,
            School::Physical,
            false,
        )));
        self.hurt(other, damage, c, rng, events, ticked);
    }

    /// A mob's swing at the player went `outcome`: what the player's talents make of a block or
    /// a crit taken (Shield Specialization's rage, Blood Craze, Enrage).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn struck(
        &mut self,
        outcome: Outcome,
        attacker: u64,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        if !self.player.alive() {
            return;
        }
        let on = crate::talents::on_struck(c);
        let event = match outcome {
            // A combo point from dodging lands on the mob the player fights.
            Outcome::Dodge => Some((crate::callings::ProcEvent::Dodge, self.player.attacking)),
            Outcome::Parry => Some((crate::callings::ProcEvent::Parry, Some(attacker))),
            Outcome::Block => Some((crate::callings::ProcEvent::Block, Some(attacker))),
            Outcome::Crit => Some((crate::callings::ProcEvent::CritTaken, Some(attacker))),
            _ => None,
        };
        if let Some((event, target)) = event {
            self.proc(
                crate::rules::procs::Happened {
                    was_ms: 0,
                    event,
                    skill: None,
                    plain: None,
                    yields_xp: false,
                    target,
                    amount: 0,
                    school: None,
                },
                c,
                rng,
                events,
                ticked,
            );
        }
        match outcome {
            Outcome::Block => {
                if let Some((spell_id, pct)) = on.block_rage {
                    if rng.below(100) < pct {
                        self.gain_rage(spell_id, 1, events);
                    }
                }
            }
            Outcome::Crit => {
                if let Some((spell_id, tick_ms, duration_ms, pct)) = on.crit_heal {
                    let max = buffed(&self.player.stats, &self.skills).max_health;
                    let per_tick = heal_tick(max, pct, duration_ms / tick_ms.max(1));
                    let kind = AuraKind::Heal { per_tick };
                    self.buff_self(
                        kind,
                        spell_id,
                        c.power_level(),
                        duration_ms,
                        tick_ms,
                        events,
                    );
                }
                if let Some((spell_id, duration_ms, swings, pct)) = on.crit_damage {
                    let kind = AuraKind::DamagePct {
                        pct,
                        armor_loss_pct: 0,
                        swings: Some(swings),
                    };
                    self.buff_self(kind, spell_id, c.power_level(), duration_ms, 0, events);
                }
            }
            _ => {}
        }
    }

    /// An aura of the player's own on the player.
    fn buff_self(
        &mut self,
        kind: AuraKind,
        spell_id: u32,
        level: u8,
        duration_ms: u32,
        period_ms: u32,
        events: &mut Vec<SessionEvent>,
    ) {
        let aura = Aura {
            unit: self.guid,
            spell_id,
            slot: 0,
            level,
            left: clock::secs(duration_ms),
            period: clock::secs(period_ms),
            next_tick: clock::secs(period_ms),
            kind,
            charges: 0,
        };
        self.skills.apply(aura, events);
    }

    /// `amount` rage for the player, logged under `spell_id`.
    pub(super) fn gain_rage(&mut self, spell_id: u32, amount: u32, events: &mut Vec<SessionEvent>) {
        let p = &mut self.player;
        p.power.gain(PowerKind::Rage, amount);
        events.push(SessionEvent::SpellEnergizeLog(SpellEnergizeLog {
            target: self.guid,
            caster: self.guid,
            spell_id,
            power: POWER_RAGE,
            amount: p.power.earned(PowerKind::Rage, amount),
        }));
        events.push(values(self.guid, &[p.power.field()]));
    }

    /// `amount` of the living player's own power, logged under `spell_id` as the bar's gain
    /// before its cap; nothing when the bar is already full.
    pub(super) fn energize_player(
        &mut self,
        spell_id: u32,
        amount: u32,
        events: &mut Vec<SessionEvent>,
    ) {
        let p = &mut self.player;
        let kind = p.power.kind;
        if amount > 0 && p.alive() && p.power.gain(kind, amount) {
            events.push(SessionEvent::SpellEnergizeLog(SpellEnergizeLog {
                target: self.guid,
                caster: self.guid,
                spell_id,
                power: kind.id(),
                amount: p.power.earned(kind, amount),
            }));
            events.push(values(self.guid, &[p.power.field()]));
        }
    }

    /// `unit` is stunned for `duration_ms`, under `spell_id` (Mace Specialization).
    pub(super) fn stun(
        &mut self,
        unit: u64,
        spell_id: u32,
        duration_ms: u32,
        level: u8,
        events: &mut Vec<SessionEvent>,
    ) {
        let duration_ms = if self.is_diminished_mob(unit) {
            self.skills.control(unit, ControlKind::Stun, duration_ms)
        } else {
            Some(duration_ms)
        };
        let Some(duration_ms) = duration_ms else {
            return;
        };
        let aura = Aura {
            unit,
            spell_id,
            slot: 0,
            level,
            left: clock::secs(duration_ms),
            period: 0.0,
            next_tick: 0.0,
            kind: AuraKind::Stun,
            charges: 0,
        };
        self.skills.apply(aura, events);
    }

    /// What the buffs move on the sheet besides attack power: maximum health and armour, and
    /// the health.
    pub(super) fn sheet_now(&self) -> Vec<(u16, u32)> {
        let s = buffed(&self.player.stats, &self.skills);
        vec![
            (UNIT_FIELD_MAXHEALTH, s.max_health),
            (UNIT_FIELD_RESISTANCES, s.armor),
            (UNIT_FIELD_HEALTH, self.player.health),
            (PLAYER_DODGE_PERCENTAGE, float(s.dodge_pct as f32)),
            (PLAYER_PARRY_PERCENTAGE, float(s.parry_pct as f32)),
            (PLAYER_BLOCK_PERCENTAGE, float(s.block_pct as f32)),
        ]
    }

    /// A melee crit of `damage` landed on `target`: the bleeds talents leave (Deep Wounds, Scent
    /// of Blood), which each new crit refreshes, sized afresh.
    pub(super) fn crit_landed(
        &mut self,
        target: u64,
        damage: u32,
        c: &Char,
        events: &mut Vec<SessionEvent>,
    ) {
        let bleeds = crate::talents::crit_bleeds(c);
        if bleeds.is_empty() || !self.mobs.iter().any(|m| m.guid == target && m.alive()) {
            return;
        }
        let s = buffed(&self.player.stats, &self.skills);
        for (spell_id, tick_ms, duration_ms, pct, of) in bleeds {
            let (per_tick, carry) = match of {
                CritBleedOf::AverageSwing => (deep_wound_tick(s.min_damage, s.max_damage, pct), 0),
                CritBleedOf::CritDamage => {
                    let ticks = (duration_ms / tick_ms.max(1)).max(1);
                    let total = scaled_share(damage, pct);
                    (total / ticks, total % ticks)
                }
            };
            if per_tick == 0 && carry == 0 {
                continue;
            }
            let period = clock::secs(tick_ms);
            let bleed = Aura {
                unit: target,
                spell_id,
                slot: 0,
                level: c.power_level(),
                left: clock::secs(duration_ms),
                period,
                next_tick: period,
                kind: AuraKind::PeriodicDamage {
                    per_tick,
                    carry,
                    school: School::Physical,
                    caster: self.guid,
                    stacks: 1,
                    max_stacks: 1,
                    leech_pct: 0,
                    rule: StackRule::Stack,
                    stacks_per_tick: 0,
                    drop_chance: None,
                    riders: TickRiders::default(),
                },
                charges: 0,
            };
            self.skills.apply(bleed, events);
        }
    }

    /// Whether `guid` is a living mob that diminishing returns apply to: not a duel's stand-in,
    /// which is another session's player.
    fn is_diminished_mob(&self, guid: u64) -> bool {
        self.mobs
            .iter()
            .any(|m| m.guid == guid && m.alive() && !m.remote)
    }

    /// A stun or incapacitate landed on `target`: a mob of that guid loses the cast it is under way.
    fn interrupt_mob_cast(&mut self, target: u64, events: &mut Vec<SessionEvent>) {
        if let Some(mob) = self.mobs.iter_mut().find(|m| m.guid == target && m.alive()) {
            super::interrupt_cast(mob, events);
        }
    }

    /// `unit` took damage: every incapacitate on it that breaks on damage ends.
    pub(super) fn woken_by_damage(&mut self, unit: u64, events: &mut Vec<SessionEvent>) {
        let gone = self.skills.remove_where(
            |a| {
                a.unit == unit
                    && matches!(
                        a.kind,
                        AuraKind::Incapacitate {
                            break_on_damage: true
                        }
                    )
            },
            events,
        );
        self.faded(&gone, events);
    }

    /// `target` loses `damage`; at 0 it dies at the player's hand.
    pub(super) fn hurt(
        &mut self,
        target: u64,
        damage: u32,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        self.hurt_school(target, damage, School::Physical, c, rng, events, ticked);
    }

    /// [`Self::hurt`] in `school`.
    #[allow(clippy::too_many_arguments)]
    fn hurt_school(
        &mut self,
        target: u64,
        damage: u32,
        school: School,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let Some(i) = self.mobs.iter().position(|m| m.guid == target && m.alive()) else {
            return;
        };
        let mob = &mut self.mobs[i];
        let damage = self.skills.absorb_on(mob.guid, damage, school, events);
        super::wound(mob, damage, self.guid, school, &mut self.harms, events);
        if mob.state == State::Idle {
            super::engage(mob, self.guid, events);
        }
        if mob.health == 0 {
            self.kill(i, c, rng, events, ticked);
        } else {
            self.woken_by_damage(target, events);
        }
    }

    /// The percent more a strike deals `target`: `controlled_pct` when it wears a stun or an
    /// incapacitate, `below_bonus_pct` when its health is under `below_health_pct`, and
    /// `bleeding_pct` when it is [`Self::bleeding`].
    fn condition_pct(
        &self,
        target: u64,
        controlled_pct: u32,
        below_health_pct: u32,
        below_bonus_pct: u32,
        bleeding_pct: u32,
    ) -> u32 {
        let controlled = if controlled_pct > 0 && self.skills.stunned(target) {
            controlled_pct
        } else {
            0
        };
        let low = self
            .mobs
            .iter()
            .find(|m| m.guid == target)
            .filter(|m| m.health * 100 < m.stats.max_health * below_health_pct)
            .map_or(0, |_| below_bonus_pct);
        let bleeding = if bleeding_pct > 0 && self.bleeding(target) {
            bleeding_pct
        } else {
            0
        };
        controlled + low + bleeding
    }

    /// After a hit at `target`, which [`InWorld::kill_yields_xp`] read as `worth` just before:
    /// when the hit killed it and the kill yields experience, the player heals `heal_pct` percent
    /// of their maximum health and gains `energize` of their power, under `spell_id`
    /// (Executioner's Toll).
    fn killing_blow(
        &mut self,
        c: &Char,
        target: u64,
        worth: Option<bool>,
        (heal_pct, energize): (u32, u32),
        spell_id: u32,
        events: &mut Vec<SessionEvent>,
    ) {
        let killed = self.mobs.iter().any(|m| m.guid == target && !m.alive());
        if worth != Some(true) || !killed {
            return;
        }
        if heal_pct > 0 {
            let max = buffed(&self.player.stats, &self.skills).max_health;
            self.heal_player(c, spell_id, scaled_share(max, heal_pct), events);
        }
        self.energize_player(spell_id, energize, events);
    }

    /// `target` wears a Physical damage over time the player put on it (Rupture, Garrote).
    fn bleeding(&self, target: u64) -> bool {
        self.skills.auras.iter().any(|a| {
            a.unit == target
                && matches!(
                    a.kind,
                    AuraKind::PeriodicDamage { school: School::Physical, caster, .. }
                        if caster == self.guid
                )
        })
    }

    /// The nearest other living enemy in melee reach in front of the caster takes `damage` of
    /// `school` from `spell_id`, with no threat bonus (Cleave).
    #[allow(clippy::too_many_arguments)]
    fn cleave(
        &mut self,
        target: u64,
        damage: u32,
        school: School,
        spell_id: u32,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let Some(other) = self
            .mobs
            .iter()
            .filter(|m| m.guid != target && m.alive())
            .filter(|m| distance(m.position, c.position) <= super::melee_reach())
            .filter(|m| in_front(c.orientation, c.position, m.position))
            .min_by(|a, b| {
                distance(a.position, c.position).total_cmp(&distance(b.position, c.position))
            })
            .map(|m| m.guid)
        else {
            return;
        };
        events.push(SessionEvent::SpellDamageLog(damage_log(
            self.guid, other, spell_id, damage, school, false,
        )));
        self.hurt_school(other, damage, school, c, rng, events, ticked);
    }

    /// The queued on-next-swing strike replaces this white swing at `target`, when its rage is
    /// still there. Whether it did.
    pub(super) fn next_swing_strike(
        &mut self,
        target: u64,
        from_front: bool,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) -> bool {
        let Some(spell_id) = self.skills.take_queued() else {
            return false;
        };
        let Some(skill) = crate::talents::skill(c, spell_id) else {
            return false;
        };
        if !self.player.power.has(self.power_cost(c, skill)) {
            events.push(refused(spell_id, SPELL_FAILED_NO_POWER));
            return false;
        }
        let Some(i) = self.mobs.iter().position(|m| m.guid == target && m.alive()) else {
            return false;
        };
        let strike = skill.effects.iter().find_map(|e| match *e {
            Effect::WeaponStrike {
                bonus,
                cleave_pct,
                controlled_pct,
                below_health_pct,
                below_bonus_pct,
                block_value_pct,
                heal_pct,
                bleeding_pct,
                kill_heal_pct,
                kill_energize,
                ..
            } => Some((
                bonus,
                cleave_pct,
                self.condition_pct(
                    target,
                    controlled_pct,
                    below_health_pct,
                    below_bonus_pct,
                    bleeding_pct,
                ),
                block_value_pct,
                heal_pct,
                (kill_heal_pct, kill_energize),
            )),
            _ => None,
        });
        let Some((bonus, cleave_pct, more, block_value_pct, heal_pct, reward)) = strike else {
            return false;
        };
        self.pay(skill, true, c, events);
        let opener = self.skills.opens_from_stealth(skill, events);
        // A next-swing strike rolls exactly as the white swing it replaces, its bonus added.
        let mut stats = buffed(&self.player.stats, &self.skills);
        // A yellow attack: the main hand's, with no dual-wield miss penalty.
        stats.off_hand = None;
        let pct = crate::talents::amount_pct(c, &skill.key);
        let shield = stats.block_value
            * (block_value_pct + crate::talents::block_value_pct(c, &skill.key))
            / 100;
        let extra = (scaled(bonus.at(c.power_level(), stats.attack_power), pct) + shield) as f32;
        stats.min_damage += extra;
        stats.max_damage += extra;
        // The player's buffs for the skill (Cutthroat's Momentum) add crit chance.
        stats.crit_pct += f64::from(self.skills.skill_crit_pct(skill));
        // Cleave's other targets: the nearest living mobs beside the first, each on its own roll.
        let at = self.mobs[i].position;
        let mut beside: Vec<usize> = (0..self.mobs.len())
            .filter(|&j| {
                j != i
                    && self.mobs[j].alive()
                    && distance(self.mobs[j].position, at) <= skill.extra_target_radius_yd
            })
            .collect();
        beside.sort_by(|&a, &b| {
            distance(self.mobs[a].position, at).total_cmp(&distance(self.mobs[b].position, at))
        });
        beside.truncate(skill.extra_targets as usize);
        let mut swings = vec![(
            target,
            crate::combat::player_swing(&stats, &self.mobs[i], from_front, opener, rng),
        )];
        for j in beside {
            let m = &self.mobs[j];
            let front = in_front(m.orientation, m.position, c.position);
            swings.push((
                m.guid,
                crate::combat::player_swing(&stats, m, front, false, rng),
            ));
        }
        events.push(SessionEvent::SpellStart {
            caster: self.guid,
            spell_id,
            cast_flags: 0,
            cast_time_ms: 0,
            target: Some(target),
            ammo_display_id: None,
        });
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
            hits: swings
                .iter()
                .filter(|(_, s)| missed_by(s.outcome).is_none())
                .map(|(guid, _)| *guid)
                .collect(),
            misses: swings
                .iter()
                .filter_map(|(guid, s)| missed_by(s.outcome).map(|m| (*guid, m)))
                .collect(),
            target: Some(target),
            go_target: None,
            dest: None,
            ammo_display_id: None,
            item_caster: None,
        });
        events.push(values(self.guid, &[self.player.power.field()]));
        // The first target's dodge opens Overpower, as a white swing's would.
        if missed_by(swings[0].1.outcome) == Some(SPELL_MISS_DODGE) {
            self.skills
                .dodged_by(target, crate::callings::opens_on_dodge(c.calling), events);
        }
        if missed_by(swings[0].1.outcome).is_none() {
            // A buff that raises the stacks is spent by this same hit.
            let more = self.skills.skill_stacks(skill);
            for effect in &skill.effects {
                self.add_aura_stacks(effect, more, c.power_level(), events);
            }
            let gone = self.skills.spend_charges(Some(skill), events);
            self.faded(&gone, events);
        }
        let impale =
            crate::talents::skill_crit_damage_pct(c, Some(&skill.key), skill.adds_combo_points())
                + self.player.stats.crit_damage_pct;
        for (guid, mut swing) in swings {
            if missed_by(swing.outcome).is_some() {
                continue;
            }
            if swing.outcome == Outcome::Crit {
                swing.damage = crit_damage(swing.would_be, impale);
            }
            if guid == target {
                swing.damage = scaled(swing.damage, more);
            }
            swing.damage = self.dealt(c, swing.damage);
            events.push(SessionEvent::SpellDamageLog(SpellDamageLog {
                blocked: swing.blocked,
                ..damage_log(
                    self.guid,
                    guid,
                    spell_id,
                    swing.damage,
                    School::Physical,
                    swing.outcome == Outcome::Crit,
                )
            }));
            let worth = self.kill_yields_xp(guid);
            self.hurt(guid, swing.damage, c, rng, events, ticked);
            if swing.damage > 0 && heal_pct > 0 {
                self.heal_player(c, spell_id, swing.damage * heal_pct / 100, events);
            }
            self.killing_blow(c, guid, worth, reward, spell_id, events);
            if swing.outcome == Outcome::Crit {
                self.crit_landed(guid, swing.damage, c, events);
            }
            if guid == target {
                self.sweep(guid, swing.damage, c, rng, events, ticked);
                if swing.damage > 0 && cleave_pct > 0 {
                    let share = swing.damage * cleave_pct / 100;
                    self.cleave(
                        guid,
                        share,
                        School::Physical,
                        spell_id,
                        c,
                        rng,
                        events,
                        ticked,
                    );
                }
            }
            if swing.damage > 0 {
                self.melee_hit(guid, c, rng, events, ticked);
            }
        }
        true
    }

    /// The skills' clocks: cooldowns, the dodge window, and every aura's ticks and fading.
    pub(super) fn skills_tick(
        &mut self,
        dt: f32,
        wall: f64,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let s = &mut self.skills;
        s.gcd = (s.gcd - dt).max(0.0);
        s.combat_hold = (s.combat_hold - dt).max(0.0);
        for clock in &mut s.control_clocks {
            clock.3 += dt;
        }
        s.control_clocks
            .retain(|&(_, _, _, since)| since < CONTROL_RESET_SECS);
        for (_, left) in &mut s.cooldowns {
            *left -= dt;
        }
        s.cooldowns.retain(|(_, left)| *left > 0.0);
        c.item_cooldowns.retain(|&(_, ready)| ready > wall);
        self.timed_cast_tick(dt, wall, c, rng, events, ticked);
        self.channel_tick(dt, c, rng, events, ticked);
        self.area_tick(dt, c, events);
        let s = &mut self.skills;
        if let Some((_, left)) = &mut s.dodged {
            *left -= dt;
            if *left <= 0.0 {
                s.close_dodge_window(events);
            }
        }

        // Ticks first, then fading: the last tick lands as the aura runs out.
        let mut due = Vec::new();
        let mut grown = Vec::new();
        for a in &mut s.auras {
            a.left -= dt;
            if a.period > 0.0 {
                a.next_tick -= dt;
                while a.next_tick <= CLOCK_EPSILON {
                    a.next_tick += a.period;
                    let mut tick = *a;
                    if let AuraKind::PeriodicDamage {
                        per_tick,
                        carry,
                        stacks,
                        max_stacks,
                        stacks_per_tick,
                        ..
                    } = &mut a.kind
                    {
                        if *carry > 0 {
                            *carry -= 1;
                            if let AuraKind::PeriodicDamage { per_tick: p, .. } = &mut tick.kind {
                                *p = *per_tick + 1;
                            }
                        }
                        // The tick pushed below deals its damage at the old count.
                        if *stacks_per_tick > 0 {
                            *stacks = (*stacks + *stacks_per_tick).min(*max_stacks);
                            if !grown.contains(&(a.unit, a.slot)) {
                                grown.push((a.unit, a.slot));
                            }
                        }
                    }
                    due.push(tick);
                }
            }
        }
        for (unit, slot) in grown {
            events.push(s.slot_fields(unit, slot));
        }
        for a in due {
            match a.kind {
                AuraKind::PeriodicDamage {
                    per_tick,
                    school,
                    caster,
                    stacks,
                    leech_pct,
                    drop_chance,
                    riders,
                    ..
                } => {
                    let per_tick = per_tick * stacks;
                    let on_player = a.unit == self.guid;
                    let alive = if on_player {
                        self.player.alive()
                    } else {
                        self.mobs.iter().any(|m| m.guid == a.unit && m.alive())
                    };
                    if !alive {
                        continue;
                    }
                    events.push(SessionEvent::PeriodicAuraLog(PeriodicAuraLog {
                        target: a.unit,
                        caster,
                        spell_id: a.spell_id,
                        ticks: vec![PeriodicTick::Damage {
                            amount: per_tick,
                            school: u32::from(school.index()),
                            absorb: 0,
                            resist: 0,
                        }],
                    }));
                    if on_player {
                        // The caster's own toll on themselves (Dark Mending) never kills.
                        let per_tick = if caster == self.guid {
                            per_tick.min(self.player.health.saturating_sub(1))
                        } else {
                            per_tick
                        };
                        self.player_takes_from(caster, per_tick, school, events);
                    } else {
                        self.hurt(a.unit, per_tick, c, rng, events, ticked);
                        // The caster's own damage over time ticked: its leech, and its procs
                        // (Thermal Feedback, Morbid Fascination, Epidemic).
                        if caster == self.guid {
                            if school == School::Physical {
                                // The bleed's own chance (Open Veins), or the Calling's.
                                let of = crate::callings::of(c.calling);
                                let chance = drop_chance.unwrap_or(of.bleed_drop_chance);
                                if of.bleed_drop.is_some() && rng.below(100) < chance {
                                    self.drop_blood(c, a.unit, rng, events);
                                }
                            }
                            if leech_pct > 0 {
                                self.heal_player(
                                    c,
                                    a.spell_id,
                                    scaled_share(per_tick, leech_pct),
                                    events,
                                );
                            }
                            let level = c.power_level();
                            self.skills.ticking = Some(a);
                            self.tick_riders(riders, a, level, rng, events);
                            let cast_by = crate::talents::skill(c, a.spell_id);
                            let skill = cast_by.map(|s| s.key.as_str());
                            let plain = cast_by.and_then(|s| s.replaces.as_deref());
                            for event in [
                                crate::callings::ProcEvent::PeriodicTick,
                                crate::callings::ProcEvent::DamageDone,
                            ] {
                                self.proc(
                                    crate::rules::procs::Happened {
                                        was_ms: 0,
                                        event,
                                        skill,
                                        plain: plain,
                                        yields_xp: false,
                                        target: Some(a.unit),
                                        amount: per_tick,
                                        school: Some(school),
                                    },
                                    c,
                                    rng,
                                    events,
                                    ticked,
                                );
                            }
                            self.skills.ticking = None;
                        }
                    }
                }
                AuraKind::Heal { per_tick } => {
                    let max = buffed(&self.player.stats, &self.skills).max_health;
                    let per_tick = scaled(per_tick, self.healing_taken(c, max));
                    let p = &mut self.player;
                    if !p.alive() {
                        continue;
                    }
                    p.health = (p.health + per_tick).min(max);
                    events.push(SessionEvent::PeriodicAuraLog(PeriodicAuraLog {
                        target: self.guid,
                        caster: self.guid,
                        spell_id: a.spell_id,
                        ticks: vec![PeriodicTick::Heal { amount: per_tick }],
                    }));
                    events.push(values(self.guid, &[(UNIT_FIELD_HEALTH, p.health)]));
                }
                AuraKind::Rage { per_tick } => {
                    let p = &mut self.player;
                    p.power.gain(PowerKind::Rage, per_tick);
                    events.push(SessionEvent::PeriodicAuraLog(PeriodicAuraLog {
                        target: self.guid,
                        caster: self.guid,
                        spell_id: a.spell_id,
                        ticks: vec![PeriodicTick::Energize {
                            power: POWER_RAGE,
                            amount: per_tick,
                        }],
                    }));
                    events.push(values(self.guid, &[p.power.field()]));
                }
                _ => {}
            }
        }
        let gone = self
            .skills
            .remove_where(|a| a.left <= CLOCK_EPSILON, events);
        self.faded(&gone, events);
        self.ground_tick(dt, c, rng, events, ticked);
        self.drop_tick(dt, c, events);
    }

    /// What one tick of the player's damage over time `tick` does, once whatever its stacks (#506,
    /// #507, #508): its aura at its chance, flat or by the stacks the tick dealt its damage at, on
    /// the player (Thermal Feedback) or on the ticking unit (Jagged Barbs); a percent of their
    /// maximum power (Kindling Ember) and of their base mana (Bloodscent); and a copy of it on
    /// another enemy at its spread chance (Epidemic), which reads the tick from `ticking`.
    fn tick_riders(
        &mut self,
        riders: TickRiders,
        tick: Aura,
        level: u8,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
    ) {
        let (unit, spell_id) = (tick.unit, tick.spell_id);
        if let Some(def) = riders.aura {
            // So much a stack, counted as the talent proc's `chance_per_stack` counts them.
            let lands = if riders.aura_chance_per_stack > 0.0 {
                super::procs::lands_per_stack(riders.aura_chance_per_stack, tick.kind.stacks(), rng)
            } else {
                rng.below(100) < riders.aura_chance
            };
            if lands {
                if !riders.aura_on_target {
                    self.grant_aura(def, level, 0, events);
                } else if self.mobs.iter().any(|m| m.guid == unit && m.alive()) {
                    // Not on a unit the tick killed.
                    self.put_aura(def, unit, level, events);
                }
            }
        }
        if riders.energize_pct_of_max > 0 {
            let p = &self.player.power;
            let max = p.max / p.kind.stored(1);
            let gain = scaled_share(max, riders.energize_pct_of_max);
            self.energize_player(spell_id, gain, events);
        }
        if riders.energize_pct_of_base > 0 {
            let s = &self.player.stats;
            let base = crate::rules::power::base_mana(s.class, s.level);
            let gain = base * riders.energize_pct_of_base / 100;
            self.energize_player(spell_id, gain, events);
        }
        if riders.spread_chance > 0 && rng.below(100) < riders.spread_chance {
            self.spread_ticking(riders.spread_yd, events);
        }
    }

    /// Spills a blood droplet 3 to 6 yd (the row's scatter) around the mob `unit`.
    fn drop_blood(&mut self, c: &Char, unit: u64, rng: &mut Rng, events: &mut Vec<SessionEvent>) {
        let Some(def) = crate::callings::of(c.calling)
            .bleed_drop
            .as_deref()
            .and_then(crate::callings::aura)
        else {
            return;
        };
        let Some(callings::AuraEffect::Pickup {
            grants,
            radius_yd,
            scatter_min_yd,
            scatter_max_yd,
        }) = def
            .effects
            .iter()
            .find(|e| matches!(e, callings::AuraEffect::Pickup { .. }))
        else {
            return;
        };
        let Some(at) = self
            .mobs
            .iter()
            .find(|m| m.guid == unit)
            .map(|m| m.position)
        else {
            return;
        };
        let angle = rng.frand(0.0, std::f32::consts::TAU);
        let away = rng.frand(*scatter_min_yd, *scatter_max_yd);
        let centre = [
            at[0] + angle.cos() * away,
            at[1] + angle.sin() * away,
            at[2],
        ];
        let object = self.skills.next_patch(self.guid);
        events.push(patch_create(
            object,
            self.guid,
            def.spell_id,
            centre,
            *radius_yd,
        ));
        self.skills.drops.push(Drop {
            object,
            spell_id: def.spell_id,
            centre,
            radius_yd: *radius_yd,
            left: clock::secs(def.duration_ms),
            grants: grants.as_str(),
        });
    }

    /// The blood droplets: the player who stands within one's radius (measured flat) takes its
    /// aura and the droplet goes; one nobody reaches fades when its time runs out.
    fn drop_tick(&mut self, dt: f32, c: &Char, events: &mut Vec<SessionEvent>) {
        let at = c.position;
        let alive = self.player.alive();
        let reach = crate::talents::pickup_radius_yd(c);
        let mut grants = Vec::new();
        for d in &mut self.skills.drops {
            d.left -= dt;
            let reached =
                alive && (d.centre[0] - at[0]).hypot(d.centre[1] - at[1]) <= d.radius_yd + reach;
            if reached {
                grants.push((d.spell_id, d.grants));
            }
            if reached || d.left <= CLOCK_EPSILON {
                events.push(SessionEvent::ObjectDestroyed(d.object));
                d.left = 0.0;
            }
        }
        self.skills.drops.retain(|d| d.left > 0.0);
        let heal_pct = crate::talents::pickup_heal_pct(c);
        let energize = crate::talents::pickup_energize(c);
        for (spell_id, key) in grants {
            if heal_pct > 0 {
                let max = buffed(&self.player.stats, &self.skills).max_health;
                self.heal_player(c, spell_id, scaled_share(max, heal_pct), events);
            }
            if energize > 0 {
                self.energize_player(spell_id, energize, events);
            }
            if let Some(def) = crate::callings::aura(key) {
                let more = crate::talents::aura_max_stacks(c, key);
                self.grant_aura(def, c.level, more, events);
            }
        }
    }

    /// The patches on the ground: each tick strikes every living enemy inside, and the last lands
    /// as the patch runs out.
    fn ground_tick(
        &mut self,
        dt: f32,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let mut due = Vec::new();
        let mut ended = Vec::new();
        for g in &mut self.skills.grounds {
            g.left -= dt;
            g.next_tick -= dt;
            while g.next_tick <= CLOCK_EPSILON {
                g.next_tick += g.period;
                due.push(*g);
            }
            if g.left <= CLOCK_EPSILON {
                events.push(SessionEvent::ObjectDestroyed(g.object));
                ended.push(*g);
            }
        }
        self.skills.grounds.retain(|g| g.left > CLOCK_EPSILON);
        for g in &ended {
            self.ward_off(g, events);
        }
        self.ground_ward(c, events);
        for g in due {
            let inside: Vec<u64> = self
                .mobs
                .iter()
                .filter(|m| {
                    m.attackable_by(self.guid) && distance(g.centre, m.position) <= g.radius_yd
                })
                .map(|m| m.guid)
                .collect();
            for guid in inside {
                self.bonus_hit(g.spell_id, guid, g.damage, g.school, c, rng, events, ticked);
            }
        }
    }

    /// The caster's aura from their own patches (Consecrated Ground): on while they stand alive
    /// inside, by flat distance to the centre, and off as they step out. One that ran out while
    /// the patch still holds them (a patch Hallowed Ground made longer) goes back on.
    fn ground_ward(&mut self, c: &Char, events: &mut Vec<SessionEvent>) {
        let me = self.guid;
        let alive = self.player.alive();
        let mut grounds = std::mem::take(&mut self.skills.grounds);
        for g in grounds.iter_mut().filter(|g| g.caster == me) {
            let Some(def) = g.caster_aura else {
                continue;
            };
            let inside = alive && flat_distance(g.centre, c.position) <= g.radius_yd;
            if inside && !self.skills.on(me, def.spell_id) {
                self.put_aura(def, me, c.power_level(), events);
            } else if !inside {
                self.ward_off(g, events);
            }
            g.caster_inside = inside;
        }
        self.skills.grounds = grounds;
    }

    /// The caster's aura from patch `g` comes off them, if they stood inside it.
    fn ward_off(&mut self, g: &Ground, events: &mut Vec<SessionEvent>) {
        let Some(def) = g.caster_aura.filter(|_| g.caster_inside) else {
            return;
        };
        let gone = self
            .skills
            .remove_where(|a| a.unit == g.caster && a.spell_id == def.spell_id, events);
        self.faded(&gone, events);
    }

    /// What an aura's going undoes.
    pub(super) fn faded(&mut self, gone: &[Aura], events: &mut Vec<SessionEvent>) {
        for a in gone {
            match a.kind {
                AuraKind::MoveSlow { .. } | AuraKind::RunSpeed { .. } => {
                    self.update_speed(a.unit, events)
                }
                AuraKind::AttackPower(_) | AuraKind::Seal { .. } => {
                    let sheet = buff_sheet(&self.player.stats, &self.skills);
                    events.push(values(self.guid, &sheet));
                }
                AuraKind::Dodge { .. } | AuraKind::Defense { .. } => {
                    events.push(values(self.guid, &self.sheet_now()));
                }
                AuraKind::ArmorLoss { .. } if a.unit == self.guid => {
                    events.push(values(self.guid, &self.sheet_now()));
                }
                AuraKind::MaxHealthPct(_) | AuraKind::DamagePct { .. } => {
                    // What was over the maximum goes with it.
                    let max = buffed(&self.player.stats, &self.skills).max_health;
                    self.player.health = self.player.health.min(max);
                    events.push(values(self.guid, &self.sheet_now()));
                }
                _ => {}
            }
        }
    }

    /// `CMSG_CANCEL_AURA`: the player clicks off one of their own buffs; debuffs stay.
    pub(crate) fn cancel_aura(&mut self, spell_id: u32, events: &mut Vec<SessionEvent>) {
        let me = self.guid;
        let gone = self.skills.remove_where(
            |a| a.unit == me && a.spell_id == spell_id && a.kind.helpful(),
            events,
        );
        self.faded(&gone, events);
    }

    /// `unit` died: its auras go with it.
    /// Puts a [`AuraKind::Marker`] of `spell_id` on the player with `stacks`, held until taken
    /// off; one already up is replaced.
    pub(super) fn show_marker(
        &mut self,
        spell_id: u32,
        helpful: bool,
        stacks: u32,
        events: &mut Vec<SessionEvent>,
    ) {
        let aura = Aura {
            unit: self.guid,
            spell_id,
            slot: 0,
            level: 0,
            left: UNTIL_BROKEN_SECS,
            period: 0.0,
            next_tick: 0.0,
            kind: AuraKind::Marker { helpful, stacks },
            charges: 0,
        };
        self.skills.apply(aura, events);
    }

    /// Takes the player's [`AuraKind::Marker`] of `spell_id` off.
    pub(super) fn hide_marker(&mut self, spell_id: u32, events: &mut Vec<SessionEvent>) {
        let me = self.guid;
        self.skills.hide_marker_on(me, spell_id, events);
    }

    pub(super) fn auras_die_with(&mut self, unit: u64, events: &mut Vec<SessionEvent>) {
        let gone = self.skills.remove_where(|a| a.unit == unit, events);
        self.faded(&gone, events);
    }

    /// Sets and clears each mob's health-under-20% aura state as its health crosses a fifth: the
    /// client greys Execute unless its target has it.
    pub(super) fn update_aura_states(&mut self, events: &mut Vec<SessionEvent>) {
        let low: Vec<u64> = self
            .mobs
            .iter()
            .filter(|m| m.alive() && u64::from(m.health) * 5 <= u64::from(m.stats.max_health))
            .map(|m| m.guid)
            .collect();
        for m in &self.mobs {
            let (was, is) = (
                self.skills.low_health.contains(&m.guid),
                low.contains(&m.guid),
            );
            if was != is {
                let state = if is { AURA_STATE_HEALTHLESS_20 } else { 0 };
                events.push(values(m.guid, &[(UNIT_FIELD_AURASTATE, state)]));
            }
        }
        self.skills.low_health = low;
    }

    /// `unit`'s run speed after its auras changed: a mob's multiplier, or the player's new speed
    /// sent to the client (`SMSG_FORCE_RUN_SPEED_CHANGE`).
    /// A fresh copy of the aura a consume just took goes on the living enemy nearest `from`
    /// within `radius_yd`, other than `from`: one stack, its row's full duration.
    pub(super) fn spread_consumed(
        &mut self,
        from: u64,
        radius_yd: f32,
        events: &mut Vec<SessionEvent>,
    ) {
        let (Some((gone, duration_ms)), Some(at)) = (
            self.skills.consumed,
            self.mobs
                .iter()
                .find(|m| m.guid == from)
                .map(|m| m.position),
        ) else {
            return;
        };
        let Some(other) = enemies_within(&self.mobs, self.guid, at, radius_yd)
            .filter(|m| m.guid != from)
            .min_by(|a, b| distance(a.position, at).total_cmp(&distance(b.position, at)))
            .map(|m| m.guid)
        else {
            return;
        };
        let mut copy = gone;
        copy.unit = other;
        copy.slot = 0;
        copy.left = clock::secs(duration_ms);
        copy.next_tick = copy.period;
        if let AuraKind::PeriodicDamage { stacks, .. } = &mut copy.kind {
            *stacks = 1;
        }
        self.skills.apply(copy, events);
        self.update_speed(other, events);
    }

    pub(super) fn update_speed(&mut self, unit: u64, events: &mut Vec<SessionEvent>) {
        let mul = if unit == self.guid {
            self.player_speed_mul()
        } else {
            self.skills.speed_mul(unit)
        };
        if let Some(mob) = self.mobs.iter_mut().find(|m| m.guid == unit) {
            mob.speed_mul = felt_speed_mul(mul, !mob.remote && mob.template.raises_boss_bar());
        }
        if unit == self.guid {
            self.skills.speed_changes += 1;
            events.push(SessionEvent::ForceSpeedChange {
                guid: unit,
                kind: SpeedKind::Run,
                counter: self.skills.speed_changes,
                speed: self.player_speeds().run * mul,
            });
        }
    }

    /// A creature's `spell` went off at the player: its effect lands, under the client's spell id
    /// so the debuff shows that spell's icon.
    pub(super) fn creature_spell_lands(
        &mut self,
        caster: u64,
        level: u8,
        from: [f32; 3],
        spell: &crate::creature_spells::CreatureSpell,
        events: &mut Vec<SessionEvent>,
    ) {
        // A heal lands on its caster in `mob_cast`, which holds the mob.
        use crate::creature_spells::CreatureEffect;
        if let CreatureEffect::Fear { duration_ms } = spell.effect {
            let duration_ms = self.fed_cc_ms(duration_ms, true);
            let aura = Aura {
                unit: self.guid,
                spell_id: spell.spell_id,
                slot: 0,
                level,
                left: clock::secs(duration_ms),
                period: 0.0,
                next_tick: 0.0,
                kind: AuraKind::Fear,
                charges: 0,
            };
            self.skills.apply(aura, events);
            self.start_fear(from, duration_ms, events);
            return;
        }
        if let CreatureEffect::Snare { pct, duration_ms } = spell.effect {
            let aura = Aura {
                unit: self.guid,
                spell_id: spell.spell_id,
                slot: 0,
                level,
                left: clock::secs(duration_ms),
                period: 0.0,
                next_tick: 0.0,
                kind: AuraKind::MoveSlow { pct },
                charges: 0,
            };
            self.skills.apply(aura, events);
            self.update_speed(self.guid, events);
            return;
        }
        if let CreatureEffect::ArmorDebuff {
            amount,
            max_stacks,
            duration_ms,
        } = spell.effect
        {
            let aura = Aura {
                unit: self.guid,
                spell_id: spell.spell_id,
                slot: 0,
                level,
                left: clock::secs(duration_ms),
                period: 0.0,
                next_tick: 0.0,
                kind: AuraKind::ArmorLoss {
                    amount,
                    stacks: 1,
                    max_stacks,
                },
                charges: 0,
            };
            self.skills.apply(aura, events);
            events.push(values(self.guid, &self.sheet_now()));
            return;
        }
        let CreatureEffect::PeriodicDamage {
            tick_ms,
            duration_ms,
            tick,
        } = spell.effect
        else {
            return;
        };
        let aura = Aura {
            unit: self.guid,
            spell_id: spell.spell_id,
            slot: 0,
            level,
            left: clock::secs(duration_ms),
            period: clock::secs(tick_ms),
            next_tick: clock::secs(tick_ms),
            kind: AuraKind::PeriodicDamage {
                per_tick: tick.at(level, 0),
                carry: 0,
                school: spell.school,
                caster,
                stacks: 1,
                max_stacks: 1,
                leech_pct: 0,
                rule: StackRule::Stack,
                stacks_per_tick: 0,
                drop_chance: None,
                riders: TickRiders::default(),
            },
            charges: 0,
        };
        self.skills.apply(aura, events);
    }

    /// The debuff a creature's root puts on the player, under the spell's client row.
    pub(super) fn show_root(
        &mut self,
        spell_id: u32,
        duration_ms: u32,
        events: &mut Vec<SessionEvent>,
    ) {
        let aura = Aura {
            unit: self.guid,
            spell_id,
            slot: 0,
            level: self.player.stats.level,
            left: clock::secs(duration_ms),
            period: 0.0,
            next_tick: 0.0,
            kind: AuraKind::Root,
            charges: 0,
        };
        self.skills.apply(aura, events);
    }

    /// The shield of a creature's absorb spell goes up on `mob`, under the spell's client row.
    pub(super) fn mob_shield(
        &mut self,
        mob: u64,
        level: u8,
        spell_id: u32,
        amount: u32,
        duration_ms: u32,
        events: &mut Vec<SessionEvent>,
    ) {
        let aura = Aura {
            unit: mob,
            spell_id,
            slot: 0,
            level,
            left: clock::secs(duration_ms),
            period: 0.0,
            next_tick: 0.0,
            kind: AuraKind::Absorb {
                left: amount,
                school: None,
                ward: Ward::default(),
            },
            charges: 0,
        };
        self.skills.apply(aura, events);
    }

    /// The buff that shows a creature's stance goes up on `mob`, under the spell's client row,
    /// for `duration_ms`, or until taken off when 0. The stance itself is on the mob.
    pub(super) fn show_stance(
        &mut self,
        mob: u64,
        level: u8,
        spell_id: u32,
        duration_ms: u32,
        events: &mut Vec<SessionEvent>,
    ) {
        let aura = Aura {
            unit: mob,
            spell_id,
            slot: 0,
            level,
            left: if duration_ms == 0 {
                UNTIL_BROKEN_SECS
            } else {
                clock::secs(duration_ms)
            },
            period: 0.0,
            next_tick: 0.0,
            kind: AuraKind::Marker {
                helpful: true,
                stacks: 1,
            },
            charges: 0,
        };
        self.skills.apply(aura, events);
    }

    /// Puts the buff `def` on the player, one aura per effect, its most stacks raised by
    /// `more_stacks`.
    pub(super) fn grant_aura(
        &mut self,
        def: &'static callings::AuraDef,
        level: u8,
        more_stacks: u32,
        events: &mut Vec<SessionEvent>,
    ) {
        self.put_aura_more(def, self.guid, level, more_stacks, events);
    }

    /// Lands a Blink cast since the last tick: the caster steps along their facing over the
    /// ground, stopping where there is none or it rises or falls too sharply, and is teleported
    /// there.
    pub(super) fn land_blink(
        &mut self,
        host: &dyn crate::Host,
        c: &mut Char,
        events: &mut Vec<SessionEvent>,
    ) {
        let Some(b) = self.skills.blink.take() else {
            return;
        };
        let (dx, dy) = (b.orientation.cos(), b.orientation.sin());
        let mut at = b.from;
        let mut gone = 0.0;
        while gone < b.distance_yd {
            let step = BLINK_STEP_YD.min(b.distance_yd - gone);
            let (x, y) = (at[0] + dx * step, at[1] + dy * step);
            match host.ground_height(self.map, x, y, at[2] + BLINK_MAX_CLIMB_YD) {
                Some(z) if (z - at[2]).abs() <= BLINK_MAX_CLIMB_YD => at = [x, y, z],
                _ => break,
            }
            gone += step;
        }
        self.teleport(c, at, b.faces, events);
    }

    /// Slides the mobs a knockback hit since the last tick: each steps straight away from where
    /// the caster stood, stopping where the ground rises or falls too sharply (as a Blink does)
    /// or line of sight breaks, and runs there facing the caster.
    pub(super) fn land_knockbacks(
        &mut self,
        host: &dyn crate::Host,
        events: &mut Vec<SessionEvent>,
    ) {
        for k in std::mem::take(&mut self.skills.knocks) {
            let caster = self.guid;
            let map = self.map;
            let Some(mob) = self.mobs.iter_mut().find(|m| m.guid == k.mob && m.alive()) else {
                continue;
            };
            let (mut dx, mut dy) = (mob.position[0] - k.from[0], mob.position[1] - k.from[1]);
            let len = dx.hypot(dy);
            if len < 0.001 {
                (dx, dy) = (1.0, 0.0);
            } else {
                (dx, dy) = (dx / len, dy / len);
            }
            let start = mob.position;
            let mut at = start;
            let mut gone = 0.0;
            while gone < k.distance_yd {
                let step = BLINK_STEP_YD.min(k.distance_yd - gone);
                let (x, y) = (at[0] + dx * step, at[1] + dy * step);
                match host.ground_height(map, x, y, at[2] + BLINK_MAX_CLIMB_YD) {
                    Some(z)
                        if (z - at[2]).abs() <= BLINK_MAX_CLIMB_YD
                            && host.line_of_sight(map, at, [x, y, z]) =>
                    {
                        at = [x, y, z];
                    }
                    _ => break,
                }
                gone += step;
            }
            if at == start {
                continue;
            }
            let slide = distance(start, at) / KNOCKBACK_SPEED_YD_S;
            events.push(mob.charge_to(at, KNOCKBACK_SPEED_YD_S, caster));
            mob.knocked = slide;
        }
    }

    /// A shield on the player under `spell_id` that soaks `amount` for `duration_ms`; a new one
    /// replaces the old.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn shield(
        &mut self,
        spell_id: u32,
        amount: u32,
        duration_ms: u32,
        school: Option<School>,
        level: u8,
        events: &mut Vec<SessionEvent>,
    ) {
        let ward = Ward::default();
        self.warded_shield(spell_id, amount, duration_ms, school, ward, level, events);
    }

    /// [`Self::shield`] with `ward`'s riders (#506).
    #[allow(clippy::too_many_arguments)]
    fn warded_shield(
        &mut self,
        spell_id: u32,
        amount: u32,
        duration_ms: u32,
        school: Option<School>,
        ward: Ward,
        level: u8,
        events: &mut Vec<SessionEvent>,
    ) {
        if amount == 0 {
            return;
        }
        let aura = Aura {
            unit: self.guid,
            spell_id,
            slot: 0,
            level,
            left: clock::secs(duration_ms),
            period: 0.0,
            next_tick: 0.0,
            kind: AuraKind::Absorb {
                left: amount,
                school,
                ward: Ward { amount, ..ward },
            },
            charges: 0,
        };
        self.skills.apply(aura, events);
    }

    /// A cast's cost comes back (Ley-Line Tap): at a bar that was full before the cast, with
    /// `full_absorb_pct` set, a shield of that percent of the cost for `absorb_ms` and nothing
    /// else; otherwise `pct` percent of the cost, under `spell_id`. Whether the shield went up in
    /// place of the refund.
    pub(super) fn refund_cost(
        &mut self,
        spell_id: u32,
        pct: u32,
        full_absorb_pct: u32,
        absorb_ms: u32,
        level: u8,
        events: &mut Vec<SessionEvent>,
    ) -> bool {
        let cost = self.skills.last_cost;
        if full_absorb_pct > 0 && self.skills.was_full {
            let amount = scaled_share(cost, full_absorb_pct);
            self.shield(spell_id, amount, absorb_ms, None, level, events);
            return true;
        }
        self.energize_player(spell_id, cost * pct / 100, events);
        false
    }

    /// The riders of the player's shields that soaked a blow from `from` (#506): power back, the
    /// attacker's aura on an enemy unit, and a broken shield's burst, which the world tick pays.
    pub(super) fn warded(&mut self, from: u64, soaks: &[Soak], events: &mut Vec<SessionEvent>) {
        for soak in soaks {
            let ward = soak.ward;
            self.energize_player(soak.spell_id, soak.soaked * ward.energize_pct / 100, events);
            if let Some(def) = ward.attacker_aura {
                if self.mobs.iter().any(|m| m.guid == from && m.alive()) {
                    self.put_aura(def, from, soak.level, events);
                }
            }
            if soak.broke && ward.burst_radius_yd > 0.0 {
                self.skills.bursts.push(Burst {
                    spell_id: soak.spell_id,
                    amount: ward.amount,
                    radius_yd: ward.burst_radius_yd,
                    school: ward.burst_school,
                });
            }
        }
    }

    /// The shields that broke since the last payment burst: each living enemy within the burst's
    /// yards of the player takes its amount, with its school's miss and crit rolls.
    pub(super) fn pay_bursts(
        &mut self,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        for burst in std::mem::take(&mut self.skills.bursts) {
            let at = c.position;
            let Burst {
                spell_id,
                amount,
                radius_yd,
                school,
            } = burst;
            self.burst_at(
                spell_id, amount, school, at, radius_yd, c, rng, events, ticked,
            );
        }
    }

    /// `amount` of `school` under `spell_id` at each living enemy within `radius_yd` of `at`, with
    /// the school's miss and crit rolls: a broken shield's burst (Volatile Ward), a heal's
    /// overhealing (Dark Surplus).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn burst_at(
        &mut self,
        spell_id: u32,
        amount: u32,
        school: School,
        at: [f32; 3],
        radius_yd: f32,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let around: Vec<u64> = enemies_within(&self.mobs, self.guid, at, radius_yd)
            .map(|m| m.guid)
            .collect();
        for guid in around {
            let how = self.spell_verdict(guid, c, rng);
            if matches!(how, Landed::Missed(_)) {
                continue;
            }
            self.strike(spell_id, guid, amount, school, how, c, rng, events, ticked);
        }
    }

    /// Heals the player `amount` of `healer`'s `spell_id`. With `burst`, what it heals past their
    /// maximum bursts at the enemies within so many yards of them, in that school (Dark
    /// Surplus). What it healed, and what it would have healed with no maximum.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn heal_bursting(
        &mut self,
        healer: u64,
        spell_id: u32,
        amount: u32,
        burst: Option<(f32, School)>,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) -> (u32, u32) {
        let (healed, full) = self.heal_full_from(healer, c, spell_id, amount, events);
        if let (Some((yd, school)), true) = (burst, full > healed) {
            let at = c.position;
            self.burst_at(
                spell_id,
                full - healed,
                school,
                at,
                yd,
                c,
                rng,
                events,
                ticked,
            );
        }
        (healed, full)
    }

    /// A magic blow at `target` with no skill behind it (a shield's burst): the level-based miss,
    /// then the player's crit chance.
    fn spell_verdict(&self, target: u64, c: &Char, rng: &mut Rng) -> Landed {
        let Some(mob) = self.mobs.iter().find(|m| m.guid == target) else {
            return Landed::Hit;
        };
        let stats = buffed(&self.player.stats, &self.skills);
        let from_front = in_front(mob.orientation, mob.position, c.position);
        let offense = PlayerOffense {
            crit_pct: stats.crit_pct,
        };
        let (table, crit) = ability_table(offense, mob.defense(), from_front, true);
        match table.outcome(rng.below(ROLL)) {
            Outcome::Miss => Landed::Missed(SPELL_MISS_MISS),
            _ if rng.below(ROLL) < crit => Landed::Crit,
            _ => Landed::Hit,
        }
    }

    /// Puts the aura row `def` on `unit`, one aura per effect.
    pub(super) fn put_aura(
        &mut self,
        def: &'static callings::AuraDef,
        unit: u64,
        level: u8,
        events: &mut Vec<SessionEvent>,
    ) {
        self.put_aura_more(def, unit, level, 0, events);
    }

    /// [`Self::put_aura`] with the most stacks of the kinds that stack raised by `more_stacks`.
    pub(super) fn put_aura_more(
        &mut self,
        def: &'static callings::AuraDef,
        unit: u64,
        level: u8,
        more_stacks: u32,
        events: &mut Vec<SessionEvent>,
    ) {
        let stacks = def.row.stack_rule.as_deref() == Some("stack");
        let stack_max = stack_limit(def) + if stacks { more_stacks } else { 0 };
        for effect in &def.effects {
            let kind = match effect {
                // A droplet is a patch on the ground (`drop_blood`), never worn.
                callings::AuraEffect::Pickup { .. } => continue,
                callings::AuraEffect::SkillCritPct { skill, pct } => AuraKind::SkillCrit {
                    skill: skill.as_str(),
                    pct: *pct,
                },
                callings::AuraEffect::ModRunSpeed { pct } => run_speed(*pct),
                callings::AuraEffect::DodgePct { pct } => AuraKind::Dodge { pct: *pct },
                callings::AuraEffect::ModSpellDamagePct { pct } => {
                    AuraKind::SpellDamagePct { pct: *pct }
                }
                callings::AuraEffect::Root => AuraKind::Root,
                callings::AuraEffect::Incapacitate => AuraKind::Incapacitate {
                    break_on_damage: def.row.break_on.contains_key("damage"),
                },
                callings::AuraEffect::PeriodicDamage { tick, school, .. } => {
                    AuraKind::PeriodicDamage {
                        per_tick: tick.at(level, 0),
                        carry: 0,
                        school: *school,
                        caster: self.guid,
                        stacks: 1,
                        max_stacks: 1,
                        leech_pct: 0,
                        rule: StackRule::Stack,
                        stacks_per_tick: 0,
                        drop_chance: None,
                        riders: TickRiders::default(),
                    }
                }
                callings::AuraEffect::ModHitChance { pct } => AuraKind::HitChance { pct: *pct },
                callings::AuraEffect::ModCastSpeedPct { pct, family_bits } => AuraKind::CastSpeed {
                    pct: *pct,
                    bits: family_bits.as_slice(),
                },
                callings::AuraEffect::PeriodicHeal {
                    tick_ms,
                    tick,
                    pct_of,
                    pct,
                } => AuraKind::Heal {
                    per_tick: tick.map_or(0, |t| t.at(level, 0))
                        + self.share(*pct_of, *pct, def.duration_ms, *tick_ms),
                },
                callings::AuraEffect::ModDamageTakenPct { pct, school } => AuraKind::DamageTaken {
                    pct: *pct,
                    school: *school,
                },
                callings::AuraEffect::ModHealingTakenPct { pct } => {
                    AuraKind::HealingTaken { pct: *pct }
                }
                callings::AuraEffect::SkillAmountPct { skill, pct } => AuraKind::SkillAmount {
                    skill: skill.as_str(),
                    pct: *pct,
                    stacks: 1,
                    max_stacks: stack_max,
                },
                callings::AuraEffect::ModDamageDonePct { pct } => AuraKind::DamageDone {
                    pct: *pct,
                    stacks: 1,
                    max_stacks: stack_max,
                },
                callings::AuraEffect::ModMeleeHastePct { pct } => AuraKind::MeleeHaste {
                    pct: *pct,
                    stacks: 1,
                    max_stacks: stack_max,
                },
                callings::AuraEffect::ModRangedHastePct { pct } => {
                    AuraKind::RangedHaste { pct: *pct }
                }
                callings::AuraEffect::ModRangedCritPct { pct } => {
                    AuraKind::RangedCrit { pct: *pct }
                }
                callings::AuraEffect::NextCast(next) => AuraKind::NextCast {
                    next,
                    group: def.row.exclusive_group.as_deref(),
                    stacks: 1,
                    max_stacks: stack_max,
                },
                callings::AuraEffect::ModArmor { amount } => AuraKind::ArmorLoss {
                    amount: amount.unsigned_abs(),
                    stacks: 1,
                    max_stacks: stack_max,
                },
                callings::AuraEffect::ExtraStrike {
                    skill,
                    weapon,
                    weapon_pct,
                } => AuraKind::ExtraStrike {
                    skill: skill.as_str(),
                    weapon: *weapon,
                    weapon_pct: *weapon_pct,
                },
                callings::AuraEffect::AtMaxStacks { becomes } => AuraKind::StackTo {
                    count: 1,
                    max: def.row.max_stacks.max(1),
                    then: becomes.as_str(),
                },
                callings::AuraEffect::Proc(p) => AuraKind::Proc(p),
                callings::AuraEffect::SkillCost { skill, amount } => AuraKind::SkillCost {
                    skill: skill.as_str(),
                    amount: *amount,
                },
                callings::AuraEffect::SkillStacks { skill, stacks } => AuraKind::SkillStacks {
                    skill: skill.as_str(),
                    stacks: *stacks,
                },
                callings::AuraEffect::ParryPct { pct } => AuraKind::Defense {
                    stat: DefenseStat::Parry,
                    pct: *pct,
                },
                callings::AuraEffect::BlockPct { pct } => AuraKind::Defense {
                    stat: DefenseStat::Block,
                    pct: *pct,
                },
                callings::AuraEffect::BlockValuePct { pct } => AuraKind::Defense {
                    stat: DefenseStat::BlockValue,
                    pct: *pct,
                },
                callings::AuraEffect::ArmorItemsPct { pct } => AuraKind::Defense {
                    stat: DefenseStat::ArmorItems,
                    pct: *pct,
                },
                callings::AuraEffect::Stealth { opener_crit } => AuraKind::Stealth {
                    opener_crit: *opener_crit,
                },
                callings::AuraEffect::CastWhileMoving { skill } => AuraKind::CastWhileMoving {
                    skill: skill.as_str(),
                },
            };
            let period = match effect {
                callings::AuraEffect::PeriodicDamage { tick_ms, .. }
                | callings::AuraEffect::PeriodicHeal { tick_ms, .. } => clock::secs(*tick_ms),
                _ => 0.0,
            };
            // An incapacitate on a mob goes through diminishing returns; a boss takes nothing.
            let mut duration_ms = def.duration_ms;
            if matches!(kind, AuraKind::Incapacitate { .. }) && unit != self.guid {
                let boss = self
                    .mobs
                    .iter()
                    .any(|m| m.guid == unit && !m.remote && m.template.raises_boss_bar());
                let alive = self.mobs.iter().any(|m| m.guid == unit && m.alive());
                let ms = if !alive || boss {
                    None
                } else if self.is_diminished_mob(unit) {
                    self.skills
                        .control(unit, ControlKind::Incapacitate, duration_ms)
                } else {
                    Some(duration_ms)
                };
                match ms {
                    Some(ms) => duration_ms = ms,
                    None => continue,
                }
            }
            let aura = Aura {
                unit,
                spell_id: def.spell_id,
                slot: 0,
                level,
                left: if duration_ms == 0 {
                    UNTIL_BROKEN_SECS
                } else {
                    clock::secs(duration_ms)
                },
                period,
                next_tick: period,
                kind,
                charges: def.charges,
            };
            self.skills.apply(aura, events);
            if unit == self.guid && def.row.break_on.contains_key("move") {
                self.skills.break_on_move.push(def.spell_id);
            }
            if matches!(kind, AuraKind::RunSpeed { .. } | AuraKind::MoveSlow { .. }) {
                self.update_speed(unit, events);
            }
            if kind == AuraKind::Root {
                if let Some(mob) = self.mobs.iter_mut().find(|m| m.guid == unit) {
                    events.extend(mob.halt());
                }
            }
            if matches!(kind, AuraKind::Incapacitate { .. }) {
                self.interrupt_mob_cast(unit, events);
            }
            // A count that reached its top gives way to the aura it builds to.
            let full = self.skills.auras.iter().find_map(|a| match a.kind {
                AuraKind::StackTo { count, max, then }
                    if a.unit == unit && a.spell_id == def.spell_id && count >= max =>
                {
                    Some(then)
                }
                _ => None,
            });
            if let Some(then) = full {
                let spell_id = def.spell_id;
                let gone = self.skills.remove_where(
                    |a| {
                        a.unit == unit
                            && a.spell_id == spell_id
                            && matches!(a.kind, AuraKind::StackTo { .. })
                    },
                    events,
                );
                self.faded(&gone, events);
                if let Some(next) = crate::callings::aura(then) {
                    self.put_aura(next, unit, level, events);
                }
            }
            if matches!(kind, AuraKind::Dodge { .. } | AuraKind::Defense { .. }) {
                events.push(values(self.guid, &self.sheet_now()));
            }
        }
    }
}

/// A weapon roll before armour: a white swing's, or a normalized one.
fn weapon_roll(stats: &CharacterStats, weapon: Weapon, rng: &mut Rng) -> f32 {
    if weapon == Weapon::Ranged {
        return stats
            .ranged
            .map_or(0.0, |r| rng.frand(r.min_damage, r.max_damage));
    }
    let roll = rng.frand(stats.min_damage, stats.max_damage);
    match weapon {
        Weapon::Normal => roll,
        Weapon::Normalized => {
            formulas::normalized_roll(roll, stats.attack_power, clock::secs(stats.attack_time_ms))
        }
        Weapon::Ranged => unreachable!("rolled above"),
    }
}

// ── Item casts ────────────────────────────────────────────────────────────────────────────────────
//
// An item's on-use spell with a cast bar: `SMSG_SPELL_START` with its cast time, then, unless
// moving or a hit interrupts it, `SMSG_CAST_RESULT` (ok), `SMSG_SPELL_GO` naming the item, its
// effect, and the item's cooldown, which the character keeps (it is saved).

/// `SPELL_FAILED_INTERRUPTED`.
const SPELL_FAILED_INTERRUPTED: u8 = 0x23;
/// `SPELL_FAILED_SPELL_IN_PROGRESS`.
const SPELL_FAILED_SPELL_IN_PROGRESS: u8 = 0x61;
/// `CAST_FLAG_UNKNOWN2`, which vmangos sets on every player cast's start and go.
const CAST_FLAG_UNKNOWN2: u16 = 0x02;

impl InWorld {
    /// `CMSG_USE_ITEM` on the item at player slot `slot`: its on-use spell starts casting, or is
    /// refused with the stock reason. An item with no on-use spell the realm runs does nothing.
    pub(crate) fn use_item(
        &mut self,
        slot: crate::inventory::Pos,
        c: &Char,
        wall: f64,
        events: &mut Vec<SessionEvent>,
    ) {
        let Some(item) = c.inventory.get(slot) else {
            return;
        };
        let Some(t) = items::template(item.entry) else {
            return;
        };
        let Some(cast_ms) = super::hearth::cast_time_ms(t.use_spell) else {
            return;
        };
        let spell_id = t.use_spell;
        let reason = if !self.player.alive() {
            Some(SPELL_FAILED_CASTER_DEAD)
        } else if self.skills.casting.is_some() {
            Some(SPELL_FAILED_SPELL_IN_PROGRESS)
        } else if c
            .item_cooldowns
            .iter()
            .any(|&(e, ready)| e == item.entry && ready - wall > f64::from(READY_SLACK_SECS))
        {
            Some(SPELL_FAILED_NOT_READY)
        } else {
            None
        };
        if let Some(reason) = reason {
            events.push(refused(spell_id, reason));
            return;
        }
        self.skills.casting = Some(TimedCast {
            spell_id,
            item: item.guid,
            entry: item.entry,
            skill: None,
            left: clock::secs(cast_ms),
        });
        events.push(SessionEvent::SpellStart {
            caster: self.guid,
            spell_id,
            cast_flags: CAST_FLAG_UNKNOWN2,
            cast_time_ms: cast_ms,
            target: None,
            ammo_display_id: None,
        });
    }

    /// An item's timed cast under way stops short, as damage taken does (a skill's bar is only
    /// pushed back in vanilla, which the realm does not model).
    pub(crate) fn interrupt_item_cast(&mut self, events: &mut Vec<SessionEvent>) {
        self.lose_fish(events);
        if self.skills.casting.is_some_and(|c| c.skill.is_none()) {
            self.interrupt_cast(events);
        }
    }

    /// A step off the spot: it stops the timed cast under way, and a channel whose skill a step
    /// interrupts (`interrupt_on` `move`, as the cast bar reads it); a channel that is not goes on
    /// (Unbroken Siphon).
    pub(crate) fn interrupt_on_step(&mut self, events: &mut Vec<SessionEvent>) {
        if self.skills.channel.is_some_and(|ch| !ch.step_ends) {
            return;
        }
        self.interrupt_cast(events);
    }

    /// The timed cast under way stops short (`Spell::cancel`): the bar shows "Interrupted", and
    /// nothing goes on cooldown.
    pub(crate) fn interrupt_cast(&mut self, events: &mut Vec<SessionEvent>) {
        self.lose_fish(events);
        self.end_channel(true, events);
        let Some(cast) = self.skills.casting.take() else {
            return;
        };
        events.push(SessionEvent::SpellFailedOther {
            caster: self.guid,
            spell_id: cast.spell_id,
        });
        events.push(refused(cast.spell_id, SPELL_FAILED_INTERRUPTED));
    }

    /// The timed cast's clock: when it runs out the spell goes off.
    #[allow(clippy::too_many_arguments)]
    fn timed_cast_tick(
        &mut self,
        dt: f32,
        wall: f64,
        c: &mut Char,
        rng: &mut Rng,
        events: &mut Vec<SessionEvent>,
        ticked: &mut Ticked,
    ) {
        let Some(cast) = &mut self.skills.casting else {
            return;
        };
        cast.left -= dt;
        if cast.left > CLOCK_EPSILON {
            return;
        }
        let cast = *cast;
        self.skills.casting = None;
        if let Some(target) = cast.skill {
            // The bar filled: the cast goes off if it still can.
            let Some(skill) = crate::talents::skill(c, cast.spell_id) else {
                return;
            };
            // Its own global cooldown, started with the bar, does not hold it back.
            match self.check(skill, target, false, c) {
                Ok(targets) => {
                    let done = self.go(skill, target, targets, false, c, rng, events);
                    ticked.leveled |= done.leveled;
                    ticked.discovered |= done.discovered;
                }
                Err(reason) => {
                    events.push(SessionEvent::SpellFailedOther {
                        caster: self.guid,
                        spell_id: cast.spell_id,
                    });
                    events.push(refused(cast.spell_id, reason));
                }
            }
            return;
        }
        events.push(SessionEvent::CastResult {
            spell_id: cast.spell_id,
            success: true,
            reason: None,
            arg: None,
        });
        events.push(SessionEvent::SpellGo {
            caster: self.guid,
            spell_id: cast.spell_id,
            cast_flags: CAST_FLAG_UNKNOWN2,
            hits: vec![self.guid],
            misses: Vec::new(),
            target: None,
            go_target: None,
            dest: None,
            ammo_display_id: None,
            item_caster: Some(cast.item),
        });
        if let Some(t) = items::template(cast.entry) {
            if t.use_cooldown_ms > 0 {
                c.item_cooldowns.retain(|&(e, _)| e != cast.entry);
                let rest = f64::from(clock::secs(t.use_cooldown_ms as u32));
                c.item_cooldowns.push((cast.entry, wall + rest));
            }
        }
        match items::template(cast.entry).and_then(|t| t.teleport_to) {
            Some(to) => self.read_scroll(cast.item, to, c, events),
            None => self.item_spell_effect(cast.spell_id, c, events),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_boss_feels_half_of_a_slow() {
        assert!((felt_speed_mul(0.4, false) - 0.4).abs() < 1e-6);
        assert!((felt_speed_mul(0.4, true) - 0.7).abs() < 1e-6);
        assert!((felt_speed_mul(0.7, true) - 0.85).abs() < 1e-6);
        assert!((felt_speed_mul(1.5, true) - 1.5).abs() < 1e-6);
    }

    #[test]
    fn control_halves_quarters_then_refuses_and_resets_after_fifteen_seconds() {
        const MOB: u64 = 0xF130 << 48 | 7;
        let mut s = Skills::new(1);
        let c = |s: &mut Skills, k| s.control(MOB, k, 8000);
        assert_eq!(c(&mut s, ControlKind::Incapacitate), Some(8000));
        assert_eq!(c(&mut s, ControlKind::Incapacitate), Some(4000));
        assert_eq!(c(&mut s, ControlKind::Stun), Some(8000));
        assert_eq!(c(&mut s, ControlKind::Incapacitate), Some(2000));
        assert_eq!(c(&mut s, ControlKind::Incapacitate), None);
        for clock in &mut s.control_clocks {
            clock.3 = 15.0;
        }
        assert_eq!(c(&mut s, ControlKind::Incapacitate), Some(8000));
    }

    /// A dodge shows the client no combo point unless the Calling has a skill that needs it.
    #[test]
    fn an_unmarked_dodge_sends_and_shows_no_combo_point() {
        const MOB: u64 = 0xF130 << 48 | 99;
        let mut skills = Skills::new(1);
        let mut events = Vec::new();
        skills.dodged_by(MOB, false, &mut events);
        assert!(events.is_empty(), "no combo update for a dodge");
        assert_eq!(skills.combo_pairs()[0], (PLAYER_FIELD_BYTES, 0));
        assert!(skills.dodged.is_some(), "the window still opens");
        skills.close_dodge_window(&mut events);
        assert!(events.is_empty(), "nor when it closes");
    }

    /// Overpower's Calling still gets the marker point for the window.
    #[test]
    fn a_marked_dodge_shows_one_combo_point_on_the_mob() {
        const MOB: u64 = 0xF130 << 48 | 99;
        let mut skills = Skills::new(1);
        let mut events = Vec::new();
        skills.dodged_by(MOB, true, &mut events);
        assert_eq!(events.len(), 1);
        assert_eq!(skills.combo_pairs()[0], (PLAYER_FIELD_BYTES, 1 << 8));
    }

    /// A shield soaks blows up to its size, then breaks; the overflow comes through.
    #[test]
    fn a_shield_soaks_up_to_its_size_then_breaks() {
        let mut skills = Skills::new(1);
        let mut events = Vec::new();
        let shield = Aura {
            unit: 1,
            spell_id: 12577,
            slot: 0,
            level: 10,
            left: 10.0,
            period: 0.0,
            next_tick: 0.0,
            kind: AuraKind::Absorb {
                left: 40,
                school: None,
                ward: Ward::default(),
            },
            charges: 0,
        };
        skills.apply(shield, &mut events);
        assert_eq!(skills.absorb(30, School::Physical, &mut events), 0);
        assert_eq!(
            skills.absorb(30, School::Physical, &mut events),
            20,
            "10 left to soak"
        );
        assert!(!skills.on(1, 12577), "spent, it breaks");
        assert_eq!(skills.absorb(30, School::Physical, &mut events), 30);
    }

    /// A creature's shield soaks what hits that creature, and the player's shields stay theirs.
    #[test]
    fn a_creatures_shield_soaks_blows_at_it_and_lets_the_overflow_through() {
        // A creature's guid, not a player's (`HIGHGUID_PLAYER` is 0).
        const MOB: u64 = 0xF130 << 48 | 99;
        let mut skills = Skills::new(1);
        let mut events = Vec::new();
        let shield = Aura {
            unit: MOB,
            spell_id: 1900021,
            slot: 0,
            level: 10,
            left: 10.0,
            period: 0.0,
            next_tick: 0.0,
            kind: AuraKind::Absorb {
                left: 50,
                school: None,
                ward: Ward::default(),
            },
            charges: 0,
        };
        skills.apply(shield, &mut events);
        assert_eq!(skills.absorb(30, School::Physical, &mut events), 30);
        assert_eq!(skills.absorb_on(MOB, 30, School::Physical, &mut events), 0);
        assert_eq!(
            skills.absorb_on(MOB, 30, School::Fire, &mut events),
            10,
            "20 left to soak"
        );
        assert!(!skills.on(MOB, 1900021), "spent, it breaks");
        assert_eq!(skills.absorb_on(MOB, 30, School::Physical, &mut events), 30);
    }

    /// A physical-only shield lets other schools through untouched.
    #[test]
    fn a_physical_shield_soaks_only_physical_damage() {
        let mut skills = Skills::new(1);
        let mut events = Vec::new();
        let shield = Aura {
            unit: 1,
            spell_id: 14912,
            slot: 0,
            level: 10,
            left: 10.0,
            period: 0.0,
            next_tick: 0.0,
            kind: AuraKind::Absorb {
                left: 40,
                school: Some(School::Physical),
                ward: Ward::default(),
            },
            charges: 0,
        };
        skills.apply(shield, &mut events);
        assert_eq!(skills.absorb(30, School::Shadow, &mut events), 30);
        assert_eq!(skills.absorb(30, School::Physical, &mut events), 0);
    }

    fn crit(skill: &'static str, pct: u32) -> Aura {
        Aura {
            unit: 1,
            spell_id: 14177,
            slot: 0,
            level: 10,
            left: 10.0,
            period: 0.0,
            next_tick: 0.0,
            kind: AuraKind::SkillCrit { skill, pct },
            charges: 1,
        }
    }

    /// Two crit effects of one spell on different skills each keep their own aura.
    #[test]
    fn two_effects_of_one_kind_on_one_spell_do_not_overwrite_each_other() {
        let mut skills = Skills::new(1);
        let mut events = Vec::new();
        skills.apply(crit("heroic_strike", 10), &mut events);
        skills.apply(crit("cleave", 20), &mut events);
        assert_eq!(skills.auras.len(), 2);
        // Granting one again refreshes it rather than adding a third.
        skills.apply(crit("cleave", 30), &mut events);
        assert_eq!(skills.auras.len(), 2);
    }

    fn bloodlust(kind: AuraKind) -> Aura {
        Aura {
            kind,
            spell_id: 1000411,
            ..crit("", 0)
        }
    }

    /// Bloodlust's damage and haste effects each gain a stack a application, up to five.
    #[test]
    fn bloodlust_stacks_damage_and_haste_to_the_limit() {
        let mut skills = Skills::new(1);
        let mut events = Vec::new();
        for n in 1..=6u32 {
            skills.apply(
                bloodlust(AuraKind::MeleeHaste {
                    pct: 3,
                    stacks: 1,
                    max_stacks: 5,
                }),
                &mut events,
            );
            skills.apply(
                bloodlust(AuraKind::DamageDone {
                    pct: 3,
                    stacks: 1,
                    max_stacks: 5,
                }),
                &mut events,
            );
            let stacks = n.min(5);
            assert_eq!(skills.haste_pct(), 3 * stacks);
            assert_eq!(skills.damage_pct(), 3 * stacks);
        }
        assert_eq!(skills.auras.len(), 2);
        assert!(skills.auras.iter().all(|a| a.kind.stacks() == 5));
    }

    /// The units an area reaches around a point (the growing area, a snare's spread, a bonus hit,
    /// a burst) are the living ones the caster may attack: a friendly creature, one whose
    /// Audience leaves the caster out and a dead one, each 2 yd from the target, are skipped.
    #[test]
    fn the_growing_area_skips_a_unit_the_caster_cannot_attack() {
        use crate::phased::Phase;
        use crate::shard::{Place, Shard};
        const ME: u64 = 1;
        let place = Place {
            map: 0,
            phase: Phase::Dungeon(1),
        };
        let mut mobs = Shard::new(place, 7).mobs;
        assert!(mobs.len() >= 4, "the Abbey holds creatures");
        let at = mobs[0].position;
        for (i, m) in mobs.iter_mut().enumerate().skip(1) {
            let yd = if i < 4 { 2.0 } else { 100.0 };
            m.position = [at[0] + yd, at[1], at[2]];
        }
        mobs[1].template = crate::mobs::content()
            .template
            .iter()
            .find(|t| t.friendly)
            .expect("a friendly template");
        mobs[2].audience = Some(crate::audience::Audience::of(&[ME + 1]));
        mobs[3].state = State::Dead;
        let reached: Vec<u64> = enemies_within(&mobs, ME, at, 5.0).map(|m| m.guid).collect();
        assert_eq!(reached, [mobs[0].guid]);
    }
}

/// The aura a run speed change of `pct` percent puts up: a sprint, or a snare when negative.
fn run_speed(pct: i32) -> AuraKind {
    if pct < 0 {
        AuraKind::MoveSlow {
            pct: pct.unsigned_abs(),
        }
    } else {
        AuraKind::RunSpeed { pct: pct as u32 }
    }
}

/// The living units among `mobs` that `caster` may attack, within `radius_yd` of `at`.
fn enemies_within(
    mobs: &[crate::mobs::Mob],
    caster: u64,
    at: [f32; 3],
    radius_yd: f32,
) -> impl Iterator<Item = &crate::mobs::Mob> {
    mobs.iter()
        .filter(move |m| m.attackable_by(caster) && distance(m.position, at) <= radius_yd)
}

/// The school a skill's damage is in: its first damaging effect's, physical with none.
fn skill_school(skill: &Skill) -> School {
    skill
        .effects
        .iter()
        .find_map(|e| match *e {
            Effect::SchoolDamage { school, .. }
            | Effect::WeaponStrike { school, .. }
            | Effect::Drain { school, .. }
            | Effect::PeriodicDamage { school, .. } => Some(school),
            Effect::ConsumeAura { school, .. } => Some(school),
            _ => None,
        })
        .unwrap_or(School::Physical)
}
