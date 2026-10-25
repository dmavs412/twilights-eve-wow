//! The Reeling Bar (ADR 0018): the realm rolls the fish and sends `FishingBite`; the client runs
//! the bar and answers with `RealmCommand::FishingResult`, then waits for the realm's
//! `FishingEnd`.
//!
//! The simulation is Stardew Valley's `BobberBar` at a fixed 60 Hz, in its own pixel space: a
//! 568-pixel track measured down from the top. The catch zone accelerates up by 0.25 px/frame²
//! while the left button is held and falls as fast when it is not, at 0.6 of that while the fish
//! is inside it. It bounces off the bottom with half its speed reversed. The fish moves by
//! Stardew's five motion types. Positions reach Lua as fractions of the track, 0 at the bottom.
//!
//! While a bar is open, and until the left button is next released, the button belongs to the
//! bar ([`ReelHoldsMouse`]): the camera's latch withholds it from the world, so neither a click
//! nor a mouse-look starts from it.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use benilla_protocol::{SatchelPick, SessionEvent};
use benilla_ui::script::{
    JournalLegend, JournalPage, JournalView, JournalZone, ReelView, ScriptValue, UiScript,
};
use benilla_world::view::WorldCamera;
use twilight_realm::{FishingGearSlot, RealmCommand};

use crate::area::AreaTableRes;
use crate::net::{ObjectStore, Objects, SelfPlayer};
use crate::ui_script::UiFeed;

/// One simulation step: Stardew's frame.
const STEP_SECS: f32 = 1.0 / 60.0;
/// At most this many steps a frame, so a long hitch does not spiral.
const MAX_STEPS: u32 = 30;
/// The track's length, in Stardew's pixels.
const TRACK_PX: f32 = 568.0;
/// The fish's top edge never passes this, nor 0; its centre is this far below its top edge.
const FISH_MAX_PX: f32 = 532.0;
const FISH_HALF_PX: f32 = 16.0;
/// The catch zone's acceleration, px/frame², up while held and down while not.
const ZONE_ACCEL: f32 = 0.25;
/// The share of that acceleration while the fish is inside the zone.
const IN_ZONE_ACCEL: f32 = 0.6;
/// The share of its speed the zone keeps, reversed, off the bottom.
const BOUNCE: f32 = 0.5;
/// How long the bite alert shows over the bobber.
const ALERT_SECS: f32 = 0.8;
/// The bite alert's art, and its size on screen.
const ALERT_ART: &str = "fishing/alert.tga";
const ALERT_PX: f32 = 32.0;

/// What a bite told the client, from `SessionEvent::FishingBite`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Bite {
    pub(crate) bobber: u64,
    pub(crate) fish: u32,
    /// 0 mixed, 1 dart, 2 smooth, 3 sinker, 4 floater.
    pub(crate) style: u8,
    pub(crate) difficulty: f32,
    pub(crate) zone_height: f32,
    pub(crate) start_fill: f32,
    pub(crate) fill_per_sec: f32,
    pub(crate) drain_per_sec: f32,
    pub(crate) timeout_secs: f32,
    pub(crate) seed: u64,
    pub(crate) treasure: Option<(f32, f32)>,
    pub(crate) legendary: bool,
}

impl Bite {
    fn of(event: &SessionEvent) -> Option<Self> {
        let SessionEvent::FishingBite {
            bobber,
            fish,
            style,
            difficulty,
            zone_height,
            start_fill,
            fill_per_sec,
            drain_per_sec,
            timeout_secs,
            seed,
            treasure,
            legendary,
            ..
        } = *event
        else {
            return None;
        };
        Some(Self {
            bobber,
            fish,
            style,
            difficulty,
            zone_height,
            start_fill,
            fill_per_sec,
            drain_per_sec,
            timeout_secs,
            seed,
            treasure,
            legendary,
        })
    }
}

/// The bar's dice, SplitMix64 from the bite's seed: the same seed gives the same fish.
struct Dice(u64);

impl Dice {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`, as .NET's `NextDouble`.
    fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Uniform in `lo..hi` (`hi` excluded), as .NET's `Next(lo, hi)`; `lo` when the range is empty.
    fn range(&mut self, lo: i32, hi: i32) -> f32 {
        if hi <= lo {
            return lo as f32;
        }
        (lo + (self.unit() * (hi - lo) as f32) as i32).min(hi - 1) as f32
    }
}

/// The treasure chest on the track.
struct Chest {
    appears_at: f32,
    fill_secs: f32,
    /// Its centre, px down from the top.
    pos: f32,
    fill: f32,
    shown: bool,
    /// It reached a full fill.
    won: bool,
}

/// An open Reeling Bar.
pub(crate) struct Reel {
    bite: Bite,
    dice: Dice,
    zone_height: f32,
    /// The zone's top edge, px down from the top of the track.
    zone_pos: f32,
    zone_speed: f32,
    /// The fish's top edge.
    fish_pos: f32,
    fish_speed: f32,
    /// Where the fish heads; `None` while it drifts.
    fish_target: Option<f32>,
    sink: f32,
    progress: f32,
    perfect: bool,
    secs: f32,
    held: bool,
    chest: Option<Chest>,
}

/// How a bar ended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Outcome {
    pub(crate) won: bool,
    pub(crate) perfect: bool,
    pub(crate) treasure: bool,
    pub(crate) secs: f32,
}

impl Reel {
    pub(crate) fn new(bite: Bite) -> Self {
        let mut dice = Dice(bite.seed);
        let zone_height = (bite.zone_height * TRACK_PX).clamp(1.0, TRACK_PX);
        let zone_pos = TRACK_PX - zone_height;
        let chest = bite.treasure.map(|(appears_at, fill_secs)| Chest {
            appears_at,
            fill_secs: fill_secs.max(STEP_SECS),
            pos: 32.0 + dice.unit() * (TRACK_PX - 64.0),
            fill: 0.0,
            shown: false,
            won: false,
        });
        Self {
            bite,
            dice,
            zone_height,
            zone_pos,
            zone_speed: 0.0,
            // The fish starts in the middle of the zone.
            fish_pos: (zone_pos + zone_height / 2.0 - FISH_HALF_PX).clamp(0.0, FISH_MAX_PX),
            fish_speed: 0.0,
            fish_target: None,
            sink: 0.0,
            progress: bite.start_fill,
            perfect: true,
            secs: 0.0,
            held: false,
            chest,
        }
    }

    fn fish_centre(&self) -> f32 {
        self.fish_pos + FISH_HALF_PX
    }

    fn holds(&self, centre: f32) -> bool {
        centre >= self.zone_pos && centre <= self.zone_pos + self.zone_height
    }

    fn fish_in_zone(&self) -> bool {
        self.holds(self.fish_centre())
    }

    /// The fish's frame (`BobberBar.update`'s motion block).
    fn move_fish(&mut self) {
        let d = self.bite.difficulty.clamp(0.0, 100.0);
        let style = self.bite.style;
        let smooth = style == 2;
        let pick = d * if smooth { 20.0 } else { 1.0 } / 4000.0;
        if self.dice.unit() < pick && (!smooth || self.fish_target.is_none()) {
            let below = TRACK_PX - 20.0 - self.fish_pos;
            let above = self.fish_pos;
            let pct = (d + self.dice.range(10, 45)).min(99.0) / 100.0;
            let span = self.dice.range((-above).min(below) as i32, below as i32);
            self.fish_target = Some(self.fish_pos + span * pct);
        }
        match style {
            4 => self.sink = (self.sink - 0.01).max(-1.5),
            3 => self.sink = (self.sink + 0.01).min(1.5),
            _ => {}
        }
        match self.fish_target {
            Some(t) if (self.fish_pos - t).abs() > 3.0 => {
                let accel =
                    (t - self.fish_pos) / (self.dice.range(10, 30) + (100.0 - d.min(100.0)));
                self.fish_speed += (accel - self.fish_speed) / 5.0;
            }
            _ if !smooth && self.dice.unit() < d / 2000.0 => {
                let jump = if self.dice.unit() < 0.5 {
                    self.dice.range(-100, -51)
                } else {
                    self.dice.range(50, 101)
                };
                self.fish_target = Some(self.fish_pos + jump);
            }
            _ => self.fish_target = None,
        }
        if style == 1 && self.dice.unit() < d / 1000.0 {
            let extra = 2 * d as i32;
            let jump = if self.dice.unit() < 0.5 {
                self.dice.range(-100 - extra, -51)
            } else {
                self.dice.range(50, 101 + extra)
            };
            self.fish_target = Some(self.fish_pos + jump);
        }
        self.fish_target = self.fish_target.map(|t| t.clamp(0.0, TRACK_PX - 20.0));
        self.fish_pos = (self.fish_pos + self.fish_speed + self.sink).clamp(0.0, FISH_MAX_PX);
    }

    /// The catch zone's frame: up while `held`, down while not, slower with the fish inside.
    fn move_zone(&mut self, held: bool, in_zone: bool) {
        let bottom = TRACK_PX - self.zone_height;
        if held && (self.zone_pos <= 0.0 || self.zone_pos >= bottom) {
            self.zone_speed = 0.0;
        }
        let mut accel = if held { -ZONE_ACCEL } else { ZONE_ACCEL };
        if in_zone {
            accel *= IN_ZONE_ACCEL;
        }
        self.zone_speed += accel;
        self.zone_pos += self.zone_speed;
        if self.zone_pos > bottom {
            self.zone_pos = bottom;
            self.zone_speed = -self.zone_speed * BOUNCE;
        } else if self.zone_pos < 0.0 {
            self.zone_pos = 0.0;
            self.zone_speed = 0.0;
        }
    }

    /// One 60 Hz step with the button `held`: `Some` when the bar ends on it.
    pub(crate) fn step(&mut self, held: bool) -> Option<Outcome> {
        self.held = held;
        self.move_fish();
        let in_zone = self.fish_in_zone();
        self.move_zone(held, in_zone);
        let b = &self.bite;
        if in_zone {
            self.progress += b.fill_per_sec * STEP_SECS;
        } else {
            self.progress -= b.drain_per_sec * STEP_SECS;
            self.perfect = false;
        }
        self.secs += STEP_SECS;
        let (secs, zone, height) = (self.secs, self.zone_pos, self.zone_height);
        if let Some(c) = &mut self.chest {
            c.shown |= secs >= c.appears_at;
            if c.shown && !c.won {
                let inside = c.pos >= zone && c.pos <= zone + height;
                let rate = STEP_SECS / c.fill_secs;
                c.fill = (c.fill + if inside { rate } else { -rate }).clamp(0.0, 1.0);
                c.won = c.fill >= 1.0;
            }
        }
        let won = self.progress >= 1.0;
        if !won && self.progress > 0.0 && self.secs < self.bite.timeout_secs {
            return None;
        }
        self.progress = self.progress.clamp(0.0, 1.0);
        Some(Outcome {
            won,
            perfect: won && self.perfect,
            treasure: won && self.chest.as_ref().is_some_and(|c| c.won),
            secs: self.secs,
        })
    }

    /// The bar as Lua reads it: fractions of the track, 0 at the bottom.
    pub(crate) fn view(&self) -> ReelView {
        let up = |px: f32| 1.0 - px / TRACK_PX;
        let chest = self.chest.as_ref();
        ReelView {
            zone_bottom: up(self.zone_pos + self.zone_height),
            zone_height: self.zone_height / TRACK_PX,
            fish_pos: up(self.fish_centre()),
            progress: self.progress,
            chest_shown: chest.is_some_and(|c| c.shown),
            chest_pos: chest.map_or(0.0, |c| up(c.pos)),
            chest_fill: chest.map_or(0.0, |c| c.fill),
            legendary: self.bite.legendary,
            fish_item: self.bite.fish,
            fish_in_zone: self.fish_in_zone(),
            reeling: self.held,
        }
    }
}

/// The realm's fishing events, which the Twilight pump hands over instead of the net layer.
#[derive(Resource, Default)]
pub(crate) struct ReelInbox(pub(crate) Vec<SessionEvent>);

/// The bar's results, which the Twilight pump sends to the realm.
#[derive(Resource, Default)]
pub(crate) struct ReelOutbox(pub(crate) Vec<RealmCommand>);

/// The left button belongs to the Reeling Bar: from the bite until it is released after the bar.
#[derive(Resource, Default)]
pub(crate) struct ReelHoldsMouse(pub(crate) bool);

/// The bar, its clock, and what the feed owes Lua.
#[derive(Resource, Default)]
struct Fishing {
    reel: Option<Reel>,
    /// The result went out; the bar waits for `FishingEnd`.
    sent: bool,
    /// Seconds not yet stepped.
    carry: f32,
    /// The Angler Level to push, `(level, xp, to next)`.
    angler: Option<(u32, u32, u32)>,
    /// The Fishing Journal to push, as the realm sent it.
    journal: Option<SessionEvent>,
    /// Events for Lua, oldest first.
    events: Vec<(&'static str, Vec<ScriptValue>)>,
    /// The bite alert: the bobber and the seconds left.
    alert: Option<(u64, f32)>,
}

/// Reads the realm's fishing events, steps the open bar and sends its result.
fn run_reel(
    time: Res<Time>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut inbox: ResMut<ReelInbox>,
    mut outbox: ResMut<ReelOutbox>,
    mut holds: ResMut<ReelHoldsMouse>,
    mut fishing: ResMut<Fishing>,
) {
    let f = &mut *fishing;
    for event in inbox.0.drain(..) {
        match event {
            SessionEvent::FishingEnd {
                caught,
                perfect,
                treasure,
            } => {
                f.reel = None;
                f.sent = false;
                let item = caught.map_or(ScriptValue::Nil, |i| ScriptValue::Int(i64::from(i)));
                f.events.push((
                    "FISHING_REEL_END",
                    vec![
                        item,
                        ScriptValue::Bool(perfect),
                        ScriptValue::Bool(treasure),
                    ],
                ));
            }
            SessionEvent::Angler {
                level,
                xp,
                xp_to_next,
            } => {
                f.angler = Some((u32::from(level), xp, xp_to_next));
                f.events.push(("ANGLER_UPDATE", Vec::new()));
            }
            journal @ SessionEvent::FishingJournal { .. } => {
                f.journal = Some(journal);
                f.events.push(("FISHING_JOURNAL_UPDATE", Vec::new()));
            }
            SessionEvent::LoggedOut => {
                f.reel = None;
                f.sent = false;
                f.alert = None;
            }
            bite => {
                if let Some(bite) = Bite::of(&bite) {
                    f.reel = Some(Reel::new(bite));
                    f.sent = false;
                    f.carry = 0.0;
                    f.alert = Some((bite.bobber, ALERT_SECS));
                    f.events.push(("FISHING_REEL_START", Vec::new()));
                }
            }
        }
    }
    let dt = time.delta_secs();
    if let Some((_, left)) = &mut f.alert {
        *left -= dt;
        if *left <= 0.0 {
            f.alert = None;
        }
    }
    let held = buttons.pressed(MouseButton::Left);
    if let (Some(reel), false) = (&mut f.reel, f.sent) {
        f.carry += dt;
        let mut steps = 0;
        while f.carry >= STEP_SECS && steps < MAX_STEPS {
            f.carry -= STEP_SECS;
            steps += 1;
            if let Some(end) = reel.step(held) {
                outbox.0.push(RealmCommand::FishingResult {
                    won: end.won,
                    perfect: end.perfect,
                    treasure: end.treasure,
                    secs: end.secs,
                });
                f.sent = true;
                break;
            }
        }
        f.carry = f.carry.min(STEP_SECS * MAX_STEPS as f32);
    }
    holds.0 = f.reel.is_some() || (holds.0 && held);
}

/// The Fishing Journal as Lua reads it, each zone named from `AreaTable` (empty with no row).
fn journal_view(event: &SessionEvent, areas: Option<&AreaTableRes>) -> JournalView {
    let SessionEvent::FishingJournal {
        bait,
        pole,
        lure,
        zones,
        legends,
        pages,
        ..
    } = event
    else {
        return JournalView::default();
    };
    let name = |id: u32| {
        areas
            .and_then(|a| a.0.name(id))
            .unwrap_or_default()
            .to_string()
    };
    JournalView {
        bait: *bait,
        pole: *pole,
        lure: *lure,
        pages: pages
            .iter()
            .map(|p| JournalPage {
                item: p.item,
                caught: p.caught,
                perfect: p.perfect,
                places: p
                    .places
                    .iter()
                    .map(|(zone, water, n)| (name(*zone), water.clone(), *n))
                    .collect(),
                revealed: p.revealed,
                next_reveal: p.next_reveal,
                lives_in: p
                    .lives_in
                    .iter()
                    .map(|(zone, water)| (name(*zone), water.clone()))
                    .collect(),
                kinds: p.kinds.clone(),
                peak_hours: p.peak_hours,
                favourite_bait: p.favourite_bait,
            })
            .collect(),
        zones: zones
            .iter()
            .map(|(id, fish)| JournalZone {
                name: name(*id),
                fish: fish.clone(),
            })
            .collect(),
        legends: legends
            .iter()
            .map(|&(item, zone, hint_found, caught)| JournalLegend {
                item,
                zone: name(zone),
                hint_found,
                caught,
            })
            .collect(),
    }
}

/// Pushes the bar, the Angler Level and the Journal into Lua, fires what happened since the last
/// frame and sends the bait the Journal chose.
fn feed_reel(
    script: Option<NonSendMut<UiScript>>,
    mut fishing: ResMut<Fishing>,
    mut outbox: ResMut<ReelOutbox>,
    areas: Option<Res<AreaTableRes>>,
    objects: Objects,
    self_q: Query<&ObjectStore, With<SelfPlayer>>,
) {
    let Some(mut script) = script else {
        return;
    };
    script.set_reel(fishing.reel.as_ref().map(Reel::view));
    if let Some((level, xp, to_next)) = fishing.angler.take() {
        script.set_angler(level, xp, to_next);
    }
    if let Some(journal) = fishing.journal.take() {
        script.set_fishing_journal(journal_view(&journal, areas.as_deref()));
    }
    for item in script.take_bait_requests() {
        outbox.0.push(RealmCommand::SetBait { item });
    }
    for req in script.take_gear_requests() {
        let slot = if req.lure {
            FishingGearSlot::Lure
        } else {
            FishingGearSlot::Pole
        };
        // The Journal's slots fill from the Satchel (ADR 0023): a bag item goes in first.
        let from = match req.from {
            Some((bag, at)) => {
                let Some(pos) = crate::ui_items::wire_pos(bag, at) else {
                    continue;
                };
                let Ok(store) = self_q.single() else {
                    continue;
                };
                let held = crate::ui_items::collect_inventory(
                    &store.0,
                    &objects,
                    crate::ui_items::InventoryScope::DEFAULT,
                );
                let Some(item) = held
                    .into_iter()
                    .find(|&(b, s, _)| (b, s) == pos)
                    .and_then(|(_, _, guid)| objects.object(guid)?.object_entry())
                else {
                    continue;
                };
                outbox.0.push(RealmCommand::DepositToJournal { from: pos });
                Some(SatchelPick {
                    item,
                    durability: None,
                })
            }
            None => None,
        };
        outbox.0.push(RealmCommand::SetFishingGear { slot, from });
    }
    for (name, args) in std::mem::take(&mut fishing.events) {
        match name {
            "FISHING_REEL_START" => script.fire_event("FISHING_REEL_START", args),
            "FISHING_REEL_END" => script.fire_event("FISHING_REEL_END", args),
            "FISHING_JOURNAL_UPDATE" => script.fire_event("FISHING_JOURNAL_UPDATE", args),
            _ => script.fire_event("ANGLER_UPDATE", args),
        }
    }
}

/// The bite alert's node, made on the first bite.
#[derive(Component)]
struct BiteAlert;

/// Shows the alert over the bobber while it lasts, placed by the world camera.
fn place_bite_alert(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    fishing: Res<Fishing>,
    index: Option<Res<crate::net::GuidIndex>>,
    bobbers: Query<&GlobalTransform>,
    camera: Query<(&Camera, &Transform), With<WorldCamera>>,
    mut alert: Query<(&mut Node, &mut Visibility), With<BiteAlert>>,
) {
    let seat = (|| {
        let (bobber, _) = fishing.alert?;
        let entity = *index.as_deref()?.0.get(&bobber)?;
        let at = bobbers.get(entity).ok()?.translation();
        let (cam, pose) = camera.single().ok()?;
        cam.world_to_viewport(&GlobalTransform::from(*pose), at)
            .ok()
    })();
    let Ok((mut node, mut shown)) = alert.single_mut() else {
        if seat.is_some() {
            spawn_alert(&mut commands, &mut images);
        }
        return;
    };
    match seat {
        Some(at) => {
            node.left = Val::Px(at.x - ALERT_PX / 2.0);
            node.top = Val::Px(at.y - ALERT_PX * 1.5);
            *shown = Visibility::Visible;
        }
        None => *shown = Visibility::Hidden,
    }
}

/// The alert node, hidden until the next frame places it; nothing when the art does not decode.
fn spawn_alert(commands: &mut Commands, images: &mut Assets<Image>) {
    let Some((w, h, rgba)) = crate::ui_script::layer_image(ALERT_ART)
        .and_then(|bytes| benilla_formats::tga_to_rgba(bytes).ok())
    else {
        warn!("fishing: {ALERT_ART} does not decode; the bite alert stays hidden");
        commands.spawn((BiteAlert, Node::default(), Visibility::Hidden));
        return;
    };
    let image = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    commands.spawn((
        BiteAlert,
        ImageNode::new(images.add(image)),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(ALERT_PX),
            height: Val::Px(ALERT_PX),
            ..default()
        },
        Visibility::Hidden,
    ));
}

pub(crate) struct FishingReelPlugin;

impl Plugin for FishingReelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ReelInbox>()
            .init_resource::<ReelOutbox>()
            .init_resource::<ReelHoldsMouse>()
            .init_resource::<Fishing>()
            .add_systems(
                Update,
                (run_reel, feed_reel, place_bite_alert)
                    .chain()
                    .in_set(UiFeed),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bite(style: u8) -> Bite {
        Bite {
            bobber: 1,
            fish: 6291,
            style,
            difficulty: 40.0,
            zone_height: 0.17,
            start_fill: 0.3,
            fill_per_sec: 0.12,
            drain_per_sec: 0.18,
            timeout_secs: 60.0,
            seed: 99,
            treasure: None,
            legendary: false,
        }
    }

    #[test]
    fn holding_the_button_for_a_second_lifts_the_zone() {
        let mut reel = Reel::new(bite(0));
        let start = reel.view().zone_bottom;
        for _ in 0..60 {
            reel.step(true);
        }
        assert!(
            reel.view().zone_bottom > start,
            "{start} -> {:?}",
            reel.view()
        );
    }

    #[test]
    fn released_the_zone_rests_on_the_bottom() {
        let mut reel = Reel::new(bite(0));
        for _ in 0..60 {
            reel.step(true);
        }
        for _ in 0..600 {
            reel.step(false);
        }
        assert!(reel.view().zone_bottom < 0.01, "{:?}", reel.view());
    }

    /// Pins the fish's centre at `at` px down the track before each step.
    fn run_pinned(reel: &mut Reel, at: impl Fn(&Reel) -> f32) -> (u32, Outcome) {
        for n in 1..=10_000 {
            reel.fish_target = None;
            reel.fish_speed = 0.0;
            reel.sink = 0.0;
            reel.fish_pos = at(reel) - FISH_HALF_PX;
            if let Some(end) = reel.step(false) {
                return (n, end);
            }
            reel.fish_pos = at(reel) - FISH_HALF_PX;
        }
        panic!("the bar never ended");
    }

    #[test]
    fn a_fish_held_inside_the_zone_fills_the_meter_in_time_and_perfectly() {
        let b = bite(2);
        let mut reel = Reel::new(b);
        // Each step moves the fish first, so the pin goes in before the zone moves.
        let (steps, end) = run_pinned(&mut reel, |r| r.zone_pos + r.zone_height / 2.0);
        let want = (1.0 - b.start_fill) / b.fill_per_sec / STEP_SECS;
        assert!(
            (steps as f32 - want).abs() <= 1.0,
            "{steps} steps, want {want}"
        );
        assert!(end.won && end.perfect, "{end:?}");
        assert!((end.secs - steps as f32 * STEP_SECS).abs() < 1e-3);
    }

    #[test]
    fn a_fish_held_outside_the_zone_empties_the_meter() {
        let mut reel = Reel::new(bite(0));
        let (_, end) = run_pinned(&mut reel, |_| 20.0);
        assert!(!end.won && !end.perfect, "{end:?}");
        assert_eq!(reel.view().progress, 0.0);
    }

    #[test]
    fn the_same_seed_swims_the_same_fish() {
        for style in 0..5 {
            let (mut a, mut b) = (Reel::new(bite(style)), Reel::new(bite(style)));
            for n in 0..600 {
                let held = n % 90 < 40;
                a.step(held);
                b.step(held);
                assert_eq!(a.fish_pos, b.fish_pos, "style {style}, step {n}");
            }
        }
        let mut other = bite(0);
        other.seed = 100;
        let (mut a, mut b) = (Reel::new(bite(0)), Reel::new(other));
        let moved = (0..600).any(|_| {
            a.step(false);
            b.step(false);
            a.fish_pos != b.fish_pos
        });
        assert!(moved, "two seeds swam the same fish");
    }

    #[test]
    fn a_chest_inside_the_zone_is_won_with_the_fish_and_one_outside_is_not() {
        let mut b = bite(2);
        b.treasure = Some((0.5, 1.0));
        // The zone rests on the bottom with the fish pinned in it; the chest sits in it too.
        let mut inside = Reel::new(b);
        inside.chest.as_mut().unwrap().pos = TRACK_PX - inside.zone_height / 2.0;
        let (_, end) = run_pinned(&mut inside, |r| r.zone_pos + r.zone_height / 2.0);
        assert!(end.won && end.treasure, "{end:?}");

        let mut outside = Reel::new(b);
        outside.chest.as_mut().unwrap().pos = 40.0;
        let (_, end) = run_pinned(&mut outside, |r| r.zone_pos + r.zone_height / 2.0);
        assert!(end.won && !end.treasure, "{end:?}");
        assert!(outside.view().chest_shown);
    }

    /// Every fishing sprite decodes, the alert among them.
    #[test]
    fn the_fishing_art_decodes() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/ui/fishing");
        let mut n = 0;
        for e in std::fs::read_dir(&dir)
            .expect("assets/ui/fishing")
            .flatten()
        {
            let bytes = std::fs::read(e.path()).unwrap();
            let (w, h, rgba) = benilla_formats::tga_to_rgba(&bytes)
                .unwrap_or_else(|err| panic!("{}: {err}", e.path().display()));
            assert_eq!(rgba.len(), (w * h * 4) as usize);
            n += 1;
        }
        assert_eq!(n, 32);
        assert!(crate::ui_script::layer_image(ALERT_ART).is_some());
    }
}
