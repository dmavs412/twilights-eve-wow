//! The quest content: the Hunting, Repeatable and Advancement Quests the Hub's givers offer
//! (`content/quests.toml`), and the items those quests use (`content/quest_items.toml`).

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use serde::Deserialize;

use crate::{items, rules, rumours};

/// A collect objective: `count` of `item`, dropping `from` these creatures at `chance` percent
/// while the quest wants more.
#[derive(Deserialize)]
pub(crate) struct Collect {
    pub(crate) item: u32,
    pub(crate) count: u32,
    /// None for a Quest Starter, whose own drop rule ([`StarterDrop`]) brings it.
    #[serde(default)]
    pub(crate) chance: u32,
    #[serde(default)]
    pub(crate) from: Vec<u32>,
    /// Drops that differ by creature, each with its own chance and count; a creature named here
    /// drops by its row instead of by `from` and `chance`.
    #[serde(default)]
    pub(crate) sources: Vec<Source>,
}

/// One kind of creature's drop of a collect objective's item.
#[derive(Deserialize)]
pub(crate) struct Source {
    pub(crate) from: Vec<u32>,
    pub(crate) chance: u32,
    /// How many one corpse holds.
    #[serde(default = "one")]
    pub(crate) count: u32,
}

fn one() -> u32 {
    1
}

/// A Quest Starter's drop (`content/quest_items.toml`'s `starter_drop`): `item` drops `from`
/// these creatures at `chance` percent, but only while the player has neither taken nor
/// completed the quest it starts and carries none already.
#[derive(Deserialize)]
pub(crate) struct StarterDrop {
    pub(crate) item: u32,
    pub(crate) from: Vec<u32>,
    pub(crate) chance: u32,
    /// An item that is no Quest Starter but a quest's own to hold ahead of time (the Dig Orders):
    /// it drops while this quest is not done and none is carried, taken or not.
    #[serde(default)]
    pub(crate) until_done: Option<u32>,
}

/// An explore objective: a spot on a map, met when the player walks into it.
#[derive(Deserialize)]
pub(crate) struct Explore {
    /// What the log calls the place.
    pub(crate) place: String,
    /// The map the spot is on: Kalimdor unless the content says otherwise.
    #[serde(default = "kalimdor")]
    pub(crate) map: u32,
    /// WoW `[x, y]`.
    pub(crate) position: [f32; 2],
    /// Yards.
    pub(crate) radius: f32,
}

fn kalimdor() -> u32 {
    crate::KALIMDOR
}

impl Explore {
    /// Whether the player at `at` on `map` stands in the spot.
    pub(crate) fn contains(&self, map: u32, at: [f32; 3]) -> bool {
        let [x, y] = self.position;
        map == self.map && (at[0] - x).hypot(at[1] - y) <= self.radius
    }

    /// The log's exploration line.
    pub(crate) fn text(&self) -> String {
        format!("{} explored", self.place)
    }
}

/// A quest's explore objectives: one spot (`explore = { ... }`), whose credit is the quest's
/// `explored` flag, or a list (`explore = [{ ... }, { ... }]`), each spot a log row of its own
/// with a credit in the counter after the use objectives.
#[derive(Default)]
pub(crate) struct Explores {
    spots: Vec<Explore>,
    listed: bool,
}

impl<'de> Deserialize<'de> for Explores {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            One(Explore),
            Many(Vec<Explore>),
        }
        Ok(match Written::deserialize(d)? {
            Written::One(e) => Explores {
                spots: vec![e],
                listed: false,
            },
            Written::Many(spots) => Explores {
                spots,
                listed: true,
            },
        })
    }
}

impl Explores {
    pub(crate) fn is_empty(&self) -> bool {
        self.spots.is_empty()
    }

    /// The lone spot of the unlisted form.
    pub(crate) fn single(&self) -> Option<&Explore> {
        self.spots.first().filter(|_| !self.listed)
    }

    /// The spots of the list form; none for the single form.
    pub(crate) fn listed(&self) -> &[Explore] {
        if self.listed {
            &self.spots
        } else {
            &[]
        }
    }
}

/// How often a quest can be done (`GLOSSARY.md`).
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    /// A Hub giver's: once per character, like [`Kind::Once`]; the kind names where it comes from.
    Hunting,
    /// A Dungeon's: taken again as soon as it is handed in.
    Repeatable,
    /// A field giver's: taken again as soon as it is handed in, paying half a level's XP and
    /// its money. Advancement never forgets it.
    Bounty,
    /// Into another Calling, with an [`Advancement`].
    Advancement,
    /// Once per character: Advancement never forgets it.
    Once,
    /// A Story Quest: once per character, Advancement never forgets it, and it pays XP as a
    /// Hunting Quest does.
    Story,
    /// A step of a Rumour (`content/rumours.toml`): once per Rumour instance, offered only by that
    /// instance's giver to its Audience, and failed when the Rumour ends.
    Rumour,
    /// A Writ (spec #557): a board GameObject offers it while its pool shows it (`crate::writs`),
    /// once per showing, and it pays its `levels` of XP.
    Writ,
}

/// Who offers a quest: an NPC's creature entry (a Hub NPC or a Field Giver), or `"item"` for a
/// Quest Starter's quest, which the item naming it in its `start_quest` offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Giver {
    Npc(u32),
    Item,
    /// Written as no `giver`: the quest names a `giver_object` instead.
    #[default]
    Unset,
}

impl<'de> Deserialize<'de> for Giver {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            Npc(u32),
            Word(String),
        }
        match Written::deserialize(d)? {
            Written::Npc(entry) => Ok(Giver::Npc(entry)),
            Written::Word(w) if w == "item" => Ok(Giver::Item),
            Written::Word(w) => Err(serde::de::Error::custom(format!(
                "a quest's giver is an NPC entry or \"item\", not {w:?}"
            ))),
        }
    }
}

impl Giver {
    /// The NPC entry that offers it; `None` for a Quest Starter's quest.
    pub(crate) fn npc(self) -> Option<u32> {
        match self {
            Giver::Npc(entry) => Some(entry),
            Giver::Item | Giver::Unset => None,
        }
    }
}

/// The quests that must be handed in before a quest is offered: one id (`after = 94001`) or a
/// list (`after = [94201, 94202]`), none by default.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct After(Vec<u32>);

impl<'de> Deserialize<'de> for After {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            One(u32),
            Many(Vec<u32>),
        }
        Ok(After(match Written::deserialize(d)? {
            Written::One(id) => vec![id],
            Written::Many(ids) => ids,
        }))
    }
}

impl After {
    pub(crate) fn ids(&self) -> &[u32] {
        &self.0
    }
}

/// A kill objective: `count` kills of any of its creatures. Written `[entry, count]` for one
/// creature, or `{ family, entries, count }` for a line that counts several, which the log names
/// by its `family` text in place of a creature name.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Kill {
    pub(crate) entries: Vec<u32>,
    pub(crate) count: u32,
    pub(crate) family: Option<String>,
}

impl<'de> Deserialize<'de> for Kill {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            One(u32, u32),
            Family {
                family: String,
                entries: Vec<u32>,
                count: u32,
            },
        }
        Ok(match Written::deserialize(d)? {
            Written::One(entry, count) => Kill {
                entries: vec![entry],
                count,
                family: None,
            },
            Written::Family {
                family,
                entries,
                count,
            } => Kill {
                entries,
                count,
                family: Some(family),
            },
        })
    }
}

impl Kill {
    /// Whether a kill of creature `entry` counts toward it.
    pub(crate) fn counts(&self, entry: u32) -> bool {
        self.entries.contains(&entry)
    }

    /// The `[entry, count]` form's line.
    #[cfg(test)]
    pub(crate) fn one(entry: u32, count: u32) -> Kill {
        Kill {
            entries: vec![entry],
            count,
            family: None,
        }
    }
}

/// A talk-to objective: while the quest is active and the credit not yet given, `npc`'s gossip
/// menu shows `option`; choosing it shows `reply` and gives the credit.
#[derive(Deserialize)]
pub(crate) struct Talk {
    pub(crate) npc: u32,
    pub(crate) option: String,
    pub(crate) reply: String,
}

/// A hint line (#676): while the quest is held and not handed in, `npc`'s gossip menu shows
/// `option`; choosing it shows `reply` and gives nothing, as often as asked.
#[derive(Deserialize)]
pub(crate) struct Hint {
    pub(crate) npc: u32,
    pub(crate) option: String,
    pub(crate) reply: String,
}

/// At most this many hint lines on one quest: each has its own reply text id.
pub(crate) const HINTS: usize = 8;

/// A GameObject standing out in the open world for a use objective: it despawns for a character
/// who uses it and returns `respawn` seconds later.
#[derive(Deserialize)]
pub(crate) struct QuestObject {
    /// Distinguishes this placement from every other GameObject's guid.
    pub(crate) guid: u32,
    pub(crate) entry: u32,
    pub(crate) display: u32,
    #[serde(default = "kalimdor")]
    pub(crate) map: u32,
    pub(crate) position: [f32; 3],
    /// It stands on the host's ground (`crate::ground`); `false` keeps its written Z.
    #[serde(default = "crate::ground::yes")]
    pub(crate) grounded: bool,
    #[serde(default)]
    pub(crate) orientation: f32,
    /// Seconds until it returns once used.
    #[serde(default = "a_minute")]
    pub(crate) respawn: f32,
    /// The Milestones it waits on (ADR 0034): out of the world while they are not met.
    #[serde(flatten)]
    pub(crate) milestones: crate::milestones::Gate,
}

fn a_minute() -> f32 {
    60.0
}

impl QuestObject {
    pub(crate) fn object_guid(&self) -> u64 {
        crate::phased::HIGHGUID_GAMEOBJECT << 48 | u64::from(self.guid)
    }

    /// Its create, standing on `host`'s ground.
    pub(crate) fn create(&self, host: &dyn crate::Host) -> benilla_protocol::SessionEvent {
        crate::phased::gameobject_create(&crate::phased::GameObject {
            guid: self.object_guid(),
            entry: self.entry,
            display_id: self.display,
            type_id: crate::phased::GOOBER,
            flags: 0,
            faction: 0,
            position: crate::ground::grounded_at(host, self.map, self.position, self.grounded),
            orientation: self.orientation,
        })
    }
}

/// A quest.
#[derive(Deserialize)]
pub(crate) struct Quest {
    pub(crate) id: u32,
    pub(crate) kind: Kind,
    #[serde(default)]
    pub(crate) giver: Giver,
    /// The entry of an outdoor quest GameObject that offers it, in place of `giver`. It must name
    /// a `gameobject` row, and the quest names its `ender`.
    #[serde(default)]
    pub(crate) giver_object: Option<u32>,
    /// Who takes it in, when not its giver: a person found in the field.
    #[serde(default)]
    ender: Option<u32>,
    /// The quests that must all be handed in before this one is offered.
    #[serde(default)]
    pub(crate) after: After,
    /// A quest that must be in the log or handed in before this one is offered: `after` asks for
    /// the hand-in alone.
    #[serde(default)]
    pub(crate) after_taken: Option<u32>,
    pub(crate) level: u8,
    /// The Herbalist Level it asks of a character who knows Herbalism, 0 for none.
    #[serde(default)]
    pub(crate) min_herbalist_level: u8,
    /// The lowest Tier it is offered at, when the Tier of its `level` is not the one.
    #[serde(default)]
    min_tier: Option<u8>,
    /// A Writ's pay: the levels, 1 to 2, a character of the Tier it is offered from gains from it.
    #[serde(default)]
    levels: Option<f64>,
    pub(crate) title: String,
    /// Listed in the Calling Choice popup; picking it there puts it in the log.
    #[serde(default)]
    pub(crate) calling_choice: bool,
    pub(crate) objectives: String,
    pub(crate) details: String,
    pub(crate) progress: String,
    pub(crate) completion: String,
    /// Copper on hand-in.
    pub(crate) money: u32,
    /// The reward panel's "You will be able to choose one of these": item entries, one of the
    /// chosen one on hand-in. At most six (`QUEST_REWARD_CHOICES_COUNT`).
    #[serde(default)]
    pub(crate) choices: Vec<u32>,
    /// The reward panel's "You will receive": `[item entry, count]`, all of them on hand-in. Ammo
    /// among them is set as the ammo when none is (the Ranger's arrows).
    #[serde(default)]
    pub(crate) rewards: Vec<(u32, u32)>,
    /// `[item entry, count]` the giver hands over when the quest is taken: a quest item to carry
    /// to the ender, which a `collect` objective then takes at the hand-in.
    #[serde(default)]
    pub(crate) gives: Vec<(u32, u32)>,
    #[serde(default)]
    pub(crate) kill: Vec<Kill>,
    /// Talk-to objectives, after the kills in the log's four creature/GameObject slots.
    #[serde(default)]
    pub(crate) talk: Vec<Talk>,
    /// Hint lines: gossip an NPC says only while the quest is held. No objective, no credit.
    #[serde(default)]
    pub(crate) hint: Vec<Hint>,
    /// Use-GameObject objectives, `(GameObject entry, count)`, after the talk objectives.
    #[serde(default, rename = "use")]
    pub(crate) use_object: Vec<(u32, u32)>,
    #[serde(default)]
    pub(crate) collect: Vec<Collect>,
    #[serde(default)]
    pub(crate) explore: Explores,
    /// A discover objective: these `AreaTable.dbc` areas (content/areas.toml), met once the
    /// character has discovered every one.
    #[serde(default)]
    pub(crate) discover: Vec<u32>,
    /// The quest fails, and this text goes to the character as a system message, when a creature
    /// enters combat with them while it is active.
    #[serde(default)]
    pub(crate) fail_if_seen: Option<String>,
    /// The quest log's header: a negative `QuestSort.dbc` id, or a zone; The Barrens by default.
    /// A Writ's names its pool.
    #[serde(default)]
    zone_or_sort: Option<i32>,
    /// An Advancement Quest's Callings: offered at the Tier Cap of one, it advances into the
    /// other. It asks for nothing but a word with its giver.
    #[serde(default)]
    pub(crate) advancement: Option<Advancement>,
    /// The Milestones its offer waits on (ADR 0034). A character who holds it keeps it, and its
    /// ender takes it in whatever they say.
    #[serde(flatten)]
    pub(crate) milestones: crate::milestones::Gate,
}

/// What makes a quest an Advancement Quest.
#[derive(Deserialize)]
pub(crate) struct Advancement {
    /// The Calling it is offered to, at that Calling's Tier Cap.
    pub(crate) from: String,
    /// The Calling it advances into.
    pub(crate) to: String,
    /// `false` for a quest that is offered and never finished: its Calling has no kit.
    #[serde(default = "yes")]
    pub(crate) completable: bool,
    /// The last quest of its Trial (#676): until the character has handed that in, it is not
    /// complete. The Trial's quests chain back to this one through `after` and `after_taken`.
    #[serde(default)]
    pub(crate) trial: Option<u32>,
}

fn yes() -> bool {
    true
}

impl Quest {
    /// The NPC entry that takes it in.
    pub(crate) fn ender(&self) -> u32 {
        self.ender
            .or(self.giver.npc())
            .expect("the content load checks an item's or a GameObject's quest names its ender")
    }

    /// The quests its ender may offer next, once it is handed in: those with this quest among
    /// their `after` ids. Whether the others are done is the offer's check.
    pub(crate) fn followers(&self) -> impl Iterator<Item = &'static Quest> + '_ {
        quests()
            .into_iter()
            .filter(|n| n.after.ids().contains(&self.id) && n.giver.npc() == Some(self.ender()))
    }

    /// The quest its ender offers next, once it is handed in (`NextQuestInChain`).
    pub(crate) fn next_in_chain(&self) -> Option<&'static Quest> {
        self.followers().next()
    }

    /// The log slot of talk objective `i`.
    pub(crate) fn talk_slot(&self, i: usize) -> usize {
        self.kill.len() + i
    }

    /// The log slot of use objective `i`.
    pub(crate) fn use_slot(&self, i: usize) -> usize {
        self.kill.len() + self.talk.len() + i
    }

    /// The log slot of listed explore spot `i`.
    pub(crate) fn explore_slot(&self, i: usize) -> usize {
        self.use_slot(self.use_object.len()) + i
    }

    /// The quest log's header the quest goes under.
    pub(crate) fn zone_or_sort(&self) -> i32 {
        self.zone_or_sort.unwrap_or(crate::THE_BARRENS as i32)
    }

    /// The XP a player of level `player` earns handing it in, before the multipliers: an
    /// Advancement Quest pays in a new Calling instead. A Writ pays its `levels` of its own
    /// level's XP divided by the Tier XP Rate of the Tier it is offered from, whatever the
    /// player's level, so that rate pays it back in full.
    pub(crate) fn reward_xp(&self, player: u8) -> u32 {
        match self.kind {
            Kind::Bounty | Kind::Hunting | Kind::Story => {
                rules::quest::hunting_reward_xp(player, self.level)
            }
            Kind::Repeatable => rules::quest::reward_xp(player),
            Kind::Rumour => {
                rules::quest::hunting_reward_xp(player, self.level) * rumours::xp_factor(self.id)
            }
            Kind::Writ => {
                let next = rules::xp::xp_to_next(self.level).unwrap_or(0);
                let rate = rules::xp::tier_rate(self.min_tier());
                (self.levels.unwrap_or(0.0) * f64::from(next) / f64::from(rate)).ceil() as u32
            }
            Kind::Advancement | Kind::Once => 0,
        }
    }

    /// Whether a character who has handed it in may take it again: a Writ once per showing.
    pub(crate) fn retakable(&self) -> bool {
        matches!(self.kind, Kind::Repeatable | Kind::Bounty | Kind::Writ)
    }

    /// Whether the quest is offered to a character whose Calling is at Tier `tier`: from its
    /// minimum Tier up, with no maximum. A copy accepted earlier is handed in at any Tier.
    pub(crate) fn offered_at_tier(&self, tier: u8) -> bool {
        tier >= self.min_tier()
    }

    /// The lowest Tier it is offered at: the content's `min_tier`, else the Tier of its level.
    pub(crate) fn min_tier(&self) -> u8 {
        self.min_tier
            .unwrap_or_else(|| rules::quest::min_tier_for_level(self.level))
    }
}

#[derive(Deserialize)]
struct QuestFile {
    #[serde(default)]
    quest: Vec<Quest>,
    /// Only through the test seam: quest items, as `content/quest_items.toml` writes them.
    #[serde(default)]
    item: Vec<items::Template>,
    #[serde(default)]
    starter_drop: Vec<StarterDrop>,
    /// The GameObjects the file's use objectives click.
    #[serde(default)]
    gameobject: Vec<QuestObject>,
}

/// At most this many kill, talk, use and listed explore objectives together, and as many collect
/// objectives (`QUEST_OBJECTIVES_COUNT`).
const OBJECTIVES: usize = 4;

/// Whether the creature `entry` is a Field Giver's: a friendly spawn the realm places anywhere,
/// on the open world or in a Phased Dungeon.
fn friendly_spawn(entry: u32) -> bool {
    crate::mobs::template(entry).is_some_and(|t| t.friendly)
        && crate::mobs::content()
            .spawn
            .iter()
            .any(|s| s.entry == entry)
}

/// The shipped quests: `content/quests.toml`, then `content/quests_elwynn.toml`, then
/// `content/quests_stormwind.toml`, then Redridge's and Duskwood's (`quests_redridge.toml`,
/// `quests_duskwood.toml`, `quests_redridge_dungeons.toml`, `quests_duskwood_dungeons.toml`),
/// then the Writs in `content/writs.toml`, then the Burning Steppes opening's
/// (`quests_blackrock_pass.toml`).
pub(crate) fn shipped() -> &'static [Quest] {
    static FILE: OnceLock<Vec<Quest>> = OnceLock::new();
    FILE.get_or_init(|| {
        let mut quests = Vec::new();
        let mut objects = Vec::new();
        for (name, text) in [
            ("quests.toml", include_str!("../../content/quests.toml")),
            (
                "quests_elwynn.toml",
                include_str!("../../content/quests_elwynn.toml"),
            ),
            (
                "quests_stormwind.toml",
                include_str!("../../content/quests_stormwind.toml"),
            ),
            (
                "quests_redridge.toml",
                include_str!("../../content/quests_redridge.toml"),
            ),
            (
                "quests_duskwood.toml",
                include_str!("../../content/quests_duskwood.toml"),
            ),
            (
                "quests_redridge_dungeons.toml",
                include_str!("../../content/quests_redridge_dungeons.toml"),
            ),
            (
                "quests_duskwood_dungeons.toml",
                include_str!("../../content/quests_duskwood_dungeons.toml"),
            ),
            ("writs.toml", include_str!("../../content/writs.toml")),
            (
                "quests_blackrock_pass.toml",
                include_str!("../../content/quests_blackrock_pass.toml"),
            ),
        ] {
            let file: QuestFile =
                toml::from_str(text).unwrap_or_else(|e| panic!("content/{name} parses: {e}"));
            quests.extend(file.quest);
            objects.extend(file.gameobject);
        }
        for o in &objects {
            o.milestones.check(&format!("quest GameObject {}", o.guid));
        }
        for q in &quests {
            check(q, &|id| quests.iter().any(|o| o.id == id), &|entry| {
                objects.iter().any(|o| o.entry == entry) || crate::phased::is_door(entry)
            });
            check_trial(q, &|id| quests.iter().find(|o| o.id == id));
            assert_eq!(
                quests.iter().filter(|o| o.id == q.id).count(),
                1,
                "quest id {} is used twice in the quest files",
                q.id
            );
        }
        let mut chosen: Vec<&str> = Vec::new();
        for a in quests
            .iter()
            .filter(|q| q.calling_choice)
            .filter_map(|q| q.advancement.as_ref())
        {
            assert!(
                !chosen.contains(&a.to.as_str()),
                "two Calling Choice quests advance into {}",
                a.to
            );
            chosen.push(&a.to);
            assert!(
                crate::callings::calling(&a.to).is_some_and(|c| !c.blurb.is_empty()),
                "the Calling Choice offers {}, which has no blurb in callings.csv",
                a.to
            );
        }
        for t in shipped_items().item.iter().filter(|t| t.start_quest != 0) {
            assert!(
                quests
                    .iter()
                    .any(|q| q.id == t.start_quest && q.giver == Giver::Item),
                "item {} starts quest {}, which is no quest with giver = \"item\"",
                t.entry,
                t.start_quest
            );
        }
        let _ = SHIPPED_OBJECTS.set(objects);
        quests
    })
}

static SHIPPED_OBJECTS: OnceLock<Vec<QuestObject>> = OnceLock::new();

fn added_objects() -> &'static RwLock<Vec<&'static QuestObject>> {
    static ADDED: RwLock<Vec<&'static QuestObject>> = RwLock::new(Vec::new());
    &ADDED
}

/// Every GameObject the use objectives click: the shipped ones, then any a test added.
pub(crate) fn objects() -> Vec<&'static QuestObject> {
    shipped();
    SHIPPED_OBJECTS
        .get()
        .into_iter()
        .flatten()
        .chain(added_objects().read().unwrap().iter().copied())
        .collect()
}

fn added() -> &'static RwLock<Vec<&'static Quest>> {
    static ADDED: RwLock<Vec<&'static Quest>> = RwLock::new(Vec::new());
    &ADDED
}

/// Adds the quests in `toml` (the shape of `content/quests.toml`) alongside the shipped ones, for
/// the rest of the process; an id already present is left as it is. A test seam
/// (`Realm::with_quests`), so a player script can try a quest the content does not ship.
pub(crate) fn add(toml: &str) {
    let parsed: QuestFile = toml::from_str(toml).expect("the added quests parse");
    parsed.item.iter().for_each(items::Template::check_pages);
    {
        // Looked up before the write lock, which a lookup would wait on.
        let new: Vec<_> = parsed
            .item
            .into_iter()
            .filter(|t| item_template(t.entry).is_none())
            .collect();
        let mut extra = added_items().write().unwrap();
        for t in new {
            extra.item.push(Box::leak(Box::new(t)));
        }
        for d in parsed.starter_drop {
            extra.starter_drop.push(Box::leak(Box::new(d)));
        }
    }
    for o in parsed.gameobject {
        o.milestones.check(&format!("quest GameObject {}", o.guid));
        shipped();
        let shipped_here = SHIPPED_OBJECTS
            .get()
            .into_iter()
            .flatten()
            .any(|s| s.guid == o.guid);
        let mut extra = added_objects().write().unwrap();
        if !shipped_here && extra.iter().all(|known| known.guid != o.guid) {
            extra.push(Box::leak(Box::new(o)));
        }
    }
    // A quest may name one later in the same file: a Trial's last quest comes after its
    // Advancement Quest.
    let ids: Vec<u32> = parsed.quest.iter().map(|q| q.id).collect();
    for q in parsed.quest {
        if quest(q.id).is_some() {
            continue;
        }
        check(
            &q,
            &|id| quest(id).is_some() || ids.contains(&id),
            &|entry| objects().iter().any(|o| o.entry == entry) || crate::phased::is_door(entry),
        );
        // Parallel tests add the same quests; the check and the push share one lock.
        let mut extra = added().write().unwrap();
        if extra.iter().all(|a| a.id != q.id) {
            extra.push(Box::leak(Box::new(q)));
        }
    }
    for id in ids {
        if let Some(q) = quest(id) {
            check_trial(q, &quest);
        }
    }
}

/// Panics unless the Trial of Advancement Quest `q`, if it names one, chains back to `q` through
/// the `after` and `after_taken` of its quests; `lookup` finds a quest by id.
fn check_trial<'a>(q: &Quest, lookup: &dyn Fn(u32) -> Option<&'a Quest>) {
    let Some(trial) = q.advancement.as_ref().and_then(|a| a.trial) else {
        return;
    };
    let before = |id| {
        lookup(id)
            .map(|t| t.after.ids().iter().copied().chain(t.after_taken).collect())
            .unwrap_or_else(Vec::new)
    };
    let mut seen = vec![trial];
    let mut open = before(trial);
    while let Some(id) = open.pop() {
        if id == q.id {
            return;
        }
        if !seen.contains(&id) {
            seen.push(id);
            open.extend(before(id));
        }
    }
    panic!(
        "Advancement Quest {}'s trial {trial} does not chain back to it through `after` and \
         `after_taken`",
        q.id
    );
}

/// What the content load refuses in `q`; `known` says whether a quest id exists and `object`
/// whether a use objective's GameObject entry is declared.
fn check(q: &Quest, known: &dyn Fn(u32) -> bool, object: &dyn Fn(u32) -> bool) {
    q.milestones.check(&format!("quest {}", q.id));
    assert!(
        (q.giver == Giver::Unset) != q.giver_object.is_none(),
        "quest {} needs exactly one of `giver` and `giver_object`",
        q.id
    );
    if q.kind == Kind::Writ {
        assert!(
            q.levels.is_some_and(|l| (1.0..=2.0).contains(&l)),
            "Writ {} needs `levels` from 1 to 2",
            q.id
        );
        assert!(
            q.giver_object.is_some(),
            "Writ {} needs a `giver_object`: a board offers it",
            q.id
        );
        assert!(
            q.zone_or_sort.is_some(),
            "Writ {} needs a `zone_or_sort`: it names the Writ's pool",
            q.id
        );
    } else {
        assert!(
            q.levels.is_none(),
            "quest {} names `levels`, and only a Writ names `levels`",
            q.id
        );
    }
    if let Some(entry) = q.giver_object {
        assert!(
            object(entry),
            "quest {} is given by GameObject {entry}, which no `gameobject` row declares",
            q.id
        );
        assert!(
            q.ender.is_some(),
            "quest {} is given by a GameObject and names no ender",
            q.id
        );
    }
    if q.giver == Giver::Item {
        assert!(
            q.ender.is_some(),
            "quest {} starts from an item and names no ender",
            q.id
        );
        assert!(
            starter_of(q.id).is_some(),
            "quest {} starts from an item, and no item in content/quest_items.toml starts it",
            q.id
        );
    }
    for npc in [q.giver.npc(), q.ender].into_iter().flatten() {
        let hub = crate::hub::npcs()
            .iter()
            .any(|n| n.entry == npc && n.gives_quests());
        // A Rumour's giver is made at run time: any friendly template will do. Its ender may be
        // a Hub quest giver instead (a calling Rumour ends at a trainer; `rumours` checks which).
        if q.kind == Kind::Rumour {
            let made = !hub && crate::mobs::template(npc).is_some_and(|t| t.friendly);
            assert!(
                made || (hub && q.ender == Some(npc)),
                "rumour quest {} names {npc}, who is no friendly creature template or is a Hub NPC",
                q.id,
            );
            continue;
        }
        assert!(
            hub || friendly_spawn(npc),
            "quest {} names {npc}, who is no quest giver in content/hub.toml and no \
                     friendly spawn anywhere",
            q.id,
        );
    }
    if q.calling_choice {
        assert!(
            q.kind == Kind::Advancement
                && q.advancement.as_ref().is_some_and(|a| {
                    crate::callings::calling(&a.from).is_some_and(|from| from.tier <= 1)
                })
                && q.giver.npc().is_some(),
            "quest {} is a Calling Choice, so it must be an advancement from a Tier 0 or Tier 1 \
             Calling with a giver NPC",
            q.id
        );
    }
    assert!(
        q.choices.len() <= 6,
        "quest {} offers more than six reward choices",
        q.id
    );
    assert!(
        q.rewards.len() <= 4,
        "quest {} gives more than four reward items",
        q.id
    );
    for &item in q
        .choices
        .iter()
        .chain(q.rewards.iter().chain(&q.gives).map(|(e, _)| e))
    {
        assert!(
            crate::items::template(item).is_some(),
            "quest {} offers item {item}, which the content does not have",
            q.id
        );
    }
    for &p in q.after.ids() {
        assert!(
            known(p),
            "quest {} comes after quest {p}, which the content does not have",
            q.id
        );
    }
    for t in &q.talk {
        let hub = crate::hub::npcs()
            .iter()
            .any(|n| n.entry == t.npc && n.gossips());
        assert!(
            hub || friendly_spawn(t.npc),
            "quest {} talks to {}, who is no gossiping Hub NPC and no friendly spawn",
            q.id,
            t.npc
        );
    }
    for h in &q.hint {
        let hub = crate::hub::npcs()
            .iter()
            .any(|n| n.entry == h.npc && n.gossips());
        assert!(
            hub || friendly_spawn(h.npc),
            "quest {} hints at {}, who is no gossiping Hub NPC and no friendly spawn",
            q.id,
            h.npc
        );
    }
    assert!(
        q.hint.len() <= HINTS,
        "quest {} has more than {HINTS} hint lines",
        q.id
    );
    for &(entry, count) in &q.use_object {
        assert!(
            count > 0 && object(entry),
            "quest {} uses GameObject {entry}, which no `gameobject` row declares (or asks for 0)",
            q.id
        );
    }
    assert_eq!(
        q.kind == Kind::Advancement,
        q.advancement.is_some(),
        "quest {}'s kind and its `advancement` table disagree",
        q.id
    );
    if let Some(a) = &q.advancement {
        assert!(
            crate::callings::calling(&a.from).is_some()
                && (!a.completable || crate::content::Calling::from_key(&a.to).is_some()),
            "quest {}'s Advancement names a Calling the realm does not have",
            q.id
        );
        assert!(
            q.kill.is_empty()
                && q.talk.is_empty()
                && q.use_object.is_empty()
                && q.collect.is_empty()
                && q.explore.is_empty()
                && q.discover.is_empty(),
            "Advancement Quest {} asks for more than a word",
            q.id
        );
        if let Some(trial) = a.trial {
            assert!(
                trial != q.id && known(trial),
                "Advancement Quest {}'s trial names quest {trial}, which the content does not have",
                q.id
            );
        }
    }
    assert!(
        q.kill.len() + q.talk.len() + q.use_object.len() + q.explore.listed().len() <= OBJECTIVES
            && q.collect.len() <= OBJECTIVES,
        "quest {} has more than {OBJECTIVES} kill, talk and use objectives (listed explore \
         spots count too), or collect objectives",
        q.id
    );
    for k in &q.kill {
        assert!(
            !k.entries.is_empty(),
            "quest {}'s kill line {:?} counts no creature",
            q.id,
            k.family.as_deref().unwrap_or_default()
        );
        assert!(
            k.count <= u32::from(super::COUNTER_MAX),
            "quest {} asks for {} kills on one line, more than the log's counter holds ({})",
            q.id,
            k.count,
            super::COUNTER_MAX
        );
    }
    for &area in &q.discover {
        assert!(
            crate::areas::get(area).is_some(),
            "quest {} discovers area {area}, which is not in content/areas.toml",
            q.id
        );
    }
    for c in &q.collect {
        assert!(
            items::template(c.item).is_some(),
            "quest {}'s item {} is in no content item table",
            q.id,
            c.item
        );
    }
}

/// Every quest: the shipped ones, then any a test added.
pub(crate) fn quests() -> Vec<&'static Quest> {
    shipped()
        .iter()
        .chain(added().read().unwrap().iter().copied())
        .collect()
}

/// The quests the Calling Choice popup lists, in the content's order.
pub(crate) fn calling_choices() -> impl Iterator<Item = &'static Quest> {
    quests().into_iter().filter(|q| q.calling_choice)
}

pub(crate) fn quest(id: u32) -> Option<&'static Quest> {
    shipped()
        .iter()
        .find(|q| q.id == id)
        .or_else(|| added().read().unwrap().iter().copied().find(|q| q.id == id))
}

/// The quests `giver` offers, in the content's order.
pub(crate) fn offered_by(giver: u32) -> impl Iterator<Item = &'static Quest> {
    quests()
        .into_iter()
        .filter(move |q| q.giver.npc() == Some(giver))
}

/// The quests the GameObject with entry `object` offers, in the content's order.
pub(crate) fn offered_by_object(object: u32) -> impl Iterator<Item = &'static Quest> {
    quests()
        .into_iter()
        .filter(move |q| q.giver_object == Some(object))
}

/// The quests `ender` takes in, in the content's order.
pub(crate) fn ended_by(ender: u32) -> impl Iterator<Item = &'static Quest> {
    quests().into_iter().filter(move |q| q.ender() == ender)
}

/// Whether the creature `entry` gives or takes in any quest: a Field Giver, whose create carries
/// the quest giver flag.
pub(crate) fn field_giver(entry: u32) -> bool {
    let gives = |q: &&Quest| q.giver.npc() == Some(entry) || q.ender() == entry;
    // A Rumour's giver has no spawn row: the realm makes it at run time.
    (friendly_spawn(entry) || quests().iter().any(|q| q.kind == Kind::Rumour && gives(q)))
        && quests().iter().any(gives)
}

/// Whether any quest has a talk objective or a hint line with the creature `entry`.
pub(crate) fn talk_target(entry: u32) -> bool {
    friendly_spawn(entry)
        && quests()
            .iter()
            .any(|q| q.talk.iter().any(|t| t.npc == entry) || q.hint.iter().any(|h| h.npc == entry))
}

#[derive(Deserialize)]
struct ItemTable {
    item: Vec<items::Template>,
    #[serde(default)]
    starter_drop: Vec<StarterDrop>,
}

/// The item files `(name, text)` as one table, in order: their `item` and `starter_drop` rows
/// concatenated. Panics with the file's name if one does not parse, and if an item entry is in
/// two files.
fn merge_items(files: &[(&str, &str)]) -> ItemTable {
    let mut table = ItemTable {
        item: Vec::new(),
        starter_drop: Vec::new(),
    };
    let mut from: HashMap<u32, &str> = HashMap::new();
    for &(name, text) in files {
        let file: ItemTable =
            toml::from_str(text).unwrap_or_else(|e| panic!("content/{name} parses: {e}"));
        for t in &file.item {
            if let Some(first) = from.get(&t.entry).filter(|first| **first != name) {
                panic!("quest item {} is in both {first} and {name}", t.entry);
            }
            from.insert(t.entry, name);
        }
        table.item.extend(file.item);
        table.starter_drop.extend(file.starter_drop);
    }
    table
}

/// The shipped quest items: `content/quest_items.toml`, then `content/quest_items_redridge.toml`,
/// then `content/quest_items_duskwood.toml`.
fn shipped_items() -> &'static ItemTable {
    static ITEMS: OnceLock<ItemTable> = OnceLock::new();
    ITEMS.get_or_init(|| {
        let table = merge_items(&[
            (
                "quest_items.toml",
                include_str!("../../content/quest_items.toml"),
            ),
            (
                "quest_items_redridge.toml",
                include_str!("../../content/quest_items_redridge.toml"),
            ),
            (
                "quest_items_duskwood.toml",
                include_str!("../../content/quest_items_duskwood.toml"),
            ),
        ]);
        table.item.iter().for_each(items::Template::check_pages);
        table
    })
}

/// The quest items and starter drops a test added.
#[derive(Default)]
struct AddedItems {
    item: Vec<&'static items::Template>,
    starter_drop: Vec<&'static StarterDrop>,
}

fn added_items() -> &'static RwLock<AddedItems> {
    static ADDED: OnceLock<RwLock<AddedItems>> = OnceLock::new();
    ADDED.get_or_init(Default::default)
}

/// A quest item's template.
pub(crate) fn item_template(entry: u32) -> Option<&'static items::Template> {
    static BY_ENTRY: OnceLock<HashMap<u32, &'static items::Template>> = OnceLock::new();
    BY_ENTRY
        .get_or_init(|| shipped_items().item.iter().map(|t| (t.entry, t)).collect())
        .get(&entry)
        .copied()
        .or_else(|| {
            let added = added_items().read().unwrap();
            added.item.iter().copied().find(|t| t.entry == entry)
        })
}

/// The Quest Starter that begins `quest`.
pub(crate) fn starter_of(quest: u32) -> Option<&'static items::Template> {
    shipped_items()
        .item
        .iter()
        .find(|t| t.start_quest == quest)
        .or_else(|| {
            let added = added_items().read().unwrap();
            added.item.iter().copied().find(|t| t.start_quest == quest)
        })
}

/// Every Quest Starter's drop.
pub(crate) fn starter_drops() -> Vec<&'static StarterDrop> {
    shipped_items()
        .starter_drop
        .iter()
        .chain(added_items().read().unwrap().starter_drop.iter().copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A copy of the shipped quest item `entry`, as one `[[item]]` row of its own file.
    fn item_file(entry: u32) -> String {
        let shipped: toml::Table = include_str!("../../content/quest_items.toml")
            .parse()
            .unwrap();
        let row = shipped["item"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["entry"].as_integer() == Some(i64::from(entry)))
            .unwrap()
            .clone();
        let mut file = toml::Table::new();
        file.insert("item".into(), toml::Value::Array(vec![row]));
        toml::to_string(&file).unwrap()
    }

    #[test]
    fn the_item_files_concatenate_in_order() {
        let a = item_file(980002);
        let b = item_file(980003);
        let table = merge_items(&[("a.toml", &a), ("b.toml", &b), ("c.toml", "item = []")]);
        let entries: Vec<u32> = table.item.iter().map(|t| t.entry).collect();
        assert_eq!(entries, [980002, 980003]);
    }

    #[test]
    #[should_panic(expected = "quest item 980002 is in both a.toml and b.toml")]
    fn an_item_in_two_item_files_fails_the_load() {
        let row = item_file(980002);
        merge_items(&[("a.toml", &row), ("b.toml", &row)]);
    }

    #[test]
    fn northshires_bounties_and_the_abbey_relics_have_their_range_giver_and_objective() {
        // id, kind, giver, Tier offered from, kills, collected item.
        let rows: [(u32, Kind, u32, u8, &[u32], Option<u32>); 5] = [
            (94005, Kind::Bounty, 196, 0, &[299, 69], None),
            (94006, Kind::Bounty, 196, 0, &[], Some(980002)),
            (94007, Kind::Bounty, 823, 0, &[6, 257], None),
            (94008, Kind::Bounty, 197, 0, &[38], None),
            (94009, Kind::Repeatable, 197, 1, &[], Some(980003)),
        ];
        for (id, kind, giver, tier, kills, item) in rows {
            let q = super::quest(id).unwrap_or_else(|| panic!("no quest {id}"));
            assert!(q.kind == kind, "{id}'s kind");
            assert!(
                matches!(q.giver, Giver::Npc(g) if g == giver),
                "{id}'s giver"
            );
            assert_eq!(q.min_tier(), tier, "{id}'s Tier");
            let killed: Vec<u32> = q.kill.iter().flat_map(|k| &k.entries).copied().collect();
            assert_eq!(killed, kills, "{id}'s kills");
            assert_eq!(q.collect.first().map(|c| c.item), item, "{id}'s item");
        }
        let relics = &super::quest(94009).unwrap().collect[0];
        let source = |entry| {
            relics
                .sources
                .iter()
                .find(|s| s.from.contains(&entry))
                .unwrap()
        };
        assert_eq!((source(96003).chance, source(96003).count), (35, 1));
        assert_eq!((source(96004).chance, source(96004).count), (100, 1));
        assert_eq!((source(96005).chance, source(96005).count), (100, 3));
    }

    #[test]
    fn the_eyes_corpse_holds_three_resonant_ore_and_the_dig_orders() {
        let ore = &super::quest(94404).unwrap().collect[0];
        let source = |entry| {
            ore.sources
                .iter()
                .find(|s| s.from.contains(&entry))
                .unwrap()
        };
        assert_eq!(ore.item, 980017);
        assert_eq!((source(96017).chance, source(96017).count), (35, 1));
        assert_eq!((source(96018).chance, source(96018).count), (35, 1));
        assert_eq!((source(96019).chance, source(96019).count), (100, 1));
        assert_eq!((source(96022).chance, source(96022).count), (100, 3));
        let orders = starter_drops()
            .into_iter()
            .find(|d| d.item == 980018)
            .expect("the Dig Orders drop");
        assert_eq!((orders.from.as_slice(), orders.chance), (&[96022][..], 100));
        assert_eq!(orders.until_done, Some(94403));
        let wanted = &super::quest(94403).unwrap().collect;
        assert_eq!((wanted[0].item, wanted[0].count), (980018, 1));
    }

    #[test]
    fn jasperlodes_quests_chain_from_the_keystone_to_the_orders() {
        let q = |id| super::quest(id).unwrap();
        assert_eq!(q(94401).after.ids(), [94303]);
        assert!(q(94401).use_object.is_empty());
        assert_eq!(q(94401).ender(), 96015);
        assert_eq!(q(94402).after.ids(), [94401]);
        assert_eq!(q(94402).kill, [Kill::one(96022, 1)]);
        assert_eq!(q(94402).money, 1000);
        assert_eq!(q(94402).choices.len(), 3);
        assert_eq!(q(94403).after.ids(), [94402]);
        assert!(matches!(q(94403).giver, Giver::Npc(240)));
        let ore = q(94404);
        assert!(matches!(ore.giver, Giver::Npc(514)));
        assert!(ore.after.ids().is_empty());
        assert!(!ore.offered_at_tier(0) && ore.offered_at_tier(1) && ore.offered_at_tier(3));
        assert_eq!(ore.money, 823);
    }

    #[test]
    fn stone_cairns_story_quests_chain_and_serane_drops_the_shard() {
        let q = |id| super::quest(id).unwrap();
        assert!(q(94501).after.ids().is_empty());
        assert_eq!(q(94501).use_object, [(980104, 5)]);
        assert_eq!(q(94501).kill, [Kill::one(96023, 6)]);
        assert_eq!(q(94502).after.ids(), [94501, 94403]);
        assert_eq!(q(94502).kill, [Kill::one(96025, 1)]);
        assert_eq!(q(94502).money, 800);
        assert_eq!(q(94502).choices.len(), 3);
        assert_eq!(q(94503).after.ids(), [94502]);
        assert!(matches!(q(94503).giver, Giver::Item));
        assert_eq!(q(94503).ender(), 190062);
        let shard = starter_drops()
            .into_iter()
            .find(|d| d.item == 980019)
            .expect("the Resonant Shard drops");
        assert_eq!((shard.from.as_slice(), shard.chance), (&[96025][..], 100));
        assert_eq!(crate::items::template(980019).unwrap().start_quest, 94503);
    }

    #[test]
    fn stone_cairns_encounters_have_their_waves() {
        let all = crate::encounters::content();
        let by = |key: &str| all.iter().find(|e| e.key == key).unwrap();
        let daubers = by("vigil_daubers");
        assert_eq!(daubers.quest, 94501);
        assert!(matches!(daubers.start, crate::encounters::Start::UseDone));
        assert_eq!(daubers.wave.len(), 1);
        assert_eq!(daubers.wave[0].creatures[0].count, 6);
        let rites = by("cairn_rites");
        assert_eq!(rites.quest, 94502);
        assert!(matches!(rites.start, crate::encounters::Start::Accepted));
        assert_eq!(rites.wave.len(), 3);
        let last: Vec<u32> = rites.wave[2].creatures.iter().map(|c| c.entry).collect();
        assert!(last.contains(&96025), "Serane leads the last wave");
        assert_eq!(rites.wave[0].creatures[0].count, 3);
    }

    #[test]
    fn stone_cairns_bounties_have_their_range_giver_and_objective() {
        // id, giver, copper, kills, uses, collected item; all offered from Tier 1.
        let rows: [(u32, u32, u32, &[u32], &[u32], Option<u32>); 4] = [
            (94505, 190061, 468, &[96026, 96027], &[], None),
            (94506, 190061, 478, &[], &[980105], None),
            (94507, 190061, 505, &[96028], &[], None),
            (94508, 190060, 495, &[], &[], Some(980020)),
        ];
        for (id, giver, money, kills, uses, item) in rows {
            let q = super::quest(id).unwrap_or_else(|| panic!("no quest {id}"));
            assert!(q.kind == Kind::Bounty, "{id}'s kind");
            assert!(
                matches!(q.giver, Giver::Npc(g) if g == giver),
                "{id}'s giver"
            );
            assert_eq!(q.min_tier(), 1, "{id}'s Tier");
            assert_eq!(q.money, money, "{id}'s copper");
            let killed: Vec<u32> = q.kill.iter().flat_map(|k| &k.entries).copied().collect();
            assert_eq!(killed, kills, "{id}'s kills");
            let used: Vec<u32> = q.use_object.iter().map(|&(e, _)| e).collect();
            assert_eq!(used, uses, "{id}'s uses");
            assert_eq!(q.collect.first().map(|c| c.item), item, "{id}'s item");
        }
        assert_eq!(super::quest(94506).unwrap().use_object, [(980105, 6)]);
        let marks = &super::quest(94508).unwrap().collect[0];
        assert_eq!(
            (marks.count, marks.chance, marks.from.as_slice()),
            (8, 50, &[96028][..])
        );
    }

    #[test]
    fn goldshires_bounties_and_wrecked_cart_have_their_range_giver_and_objective() {
        // id, giver, copper, kills, collected item; all offered from Tier 1.
        let rows: [(u32, u32, u32, &[u32], Option<u32>); 4] = [
            (94105, 963, 260, &[118, 30], None),
            (94106, 514, 280, &[], Some(980006)),
            (94107, 963, 316, &[116], None),
            (94108, 241, 290, &[], Some(980007)),
        ];
        for (id, giver, money, kills, item) in rows {
            let q = super::quest(id).unwrap_or_else(|| panic!("no quest {id}"));
            assert!(q.kind == Kind::Bounty, "{id}'s kind");
            assert!(
                matches!(q.giver, Giver::Npc(g) if g == giver),
                "{id}'s giver"
            );
            assert_eq!(q.min_tier(), 1, "{id}'s Tier");
            assert_eq!(q.money, money, "{id}'s copper");
            let killed: Vec<u32> = q.kill.iter().flat_map(|k| &k.entries).copied().collect();
            assert_eq!(killed, kills, "{id}'s kills");
            assert_eq!(q.collect.first().map(|c| c.item), item, "{id}'s item");
        }
        let cart = super::quest(94103).unwrap();
        assert_eq!(
            (cart.giver, cart.ender(), cart.money),
            (Giver::Npc(295), 240, 400)
        );
        assert_eq!(cart.collect[0].from, [96011]);
        assert_eq!(cart.collect[0].chance, 100);
        assert!(crate::mobs::template(96011).is_some_and(|t| t.rank == 0));
        assert_eq!(super::quest(94101).unwrap().gives, [(980004, 1)]);
    }

    /// A quest of `kind` at level 10, as the content would write it.
    fn quest(kind: &str) -> Quest {
        toml::from_str(&format!(
            r#"
            id = 1
            kind = "{kind}"
            giver = 1
            level = 10
            title = ""
            objectives = ""
            details = ""
            progress = ""
            completion = ""
            money = 0
            "#
        ))
        .expect("a quest")
    }

    #[test]
    fn a_repeatable_quest_can_be_retaken_and_pays_a_full_level() {
        let q = quest("repeatable");
        assert!(q.retakable());
        // Level 10's whole 7600 XP; level 15's 22600 for a level 15 player.
        assert_eq!((q.reward_xp(10), q.reward_xp(15)), (7600, 22600));
    }

    #[test]
    fn a_hunting_quest_cannot_be_retaken_and_pays_half_its_level() {
        let q = quest("hunting");
        assert!(!q.retakable());
        assert_eq!((q.reward_xp(10), q.reward_xp(15)), (3800, 3800));
    }

    #[test]
    fn a_bounty_can_be_retaken_and_pays_half_a_level() {
        let q = quest("bounty");
        assert!(q.retakable());
        assert_eq!((q.reward_xp(10), q.reward_xp(15)), (3800, 3800));
    }

    /// A Writ of `level` paying `levels`.
    fn writ(level: u8, levels: &str) -> Quest {
        toml::from_str(&format!(
            "kind = \"writ\"\nlevels = {levels}\n{}",
            quest_body().replace("level = 10", &format!("level = {level}"))
        ))
        .expect("a Writ")
    }

    #[test]
    fn a_writ_can_be_retaken_and_pays_its_levels_over_its_tiers_rate() {
        assert!(writ(33, "1").retakable());
        // 326,400 / 5 and 719,100 x 2 / 5, whatever the player's level.
        assert_eq!(writ(33, "1").reward_xp(33), 65_280);
        assert_eq!(writ(47, "2").reward_xp(47), 287_640);
        assert_eq!(writ(47, "2").reward_xp(12), 287_640);
        // 112,400 x 1.5 / 3 and 505,200 x 1.5 / 5.
        assert_eq!(writ(24, "1.5").reward_xp(24), 56_200);
        assert_eq!(writ(40, "1.5").reward_xp(40), 151_560);
    }

    #[test]
    fn a_quest_is_offered_from_its_levels_tier_up_unless_the_content_sets_one() {
        let q: Quest =
            toml::from_str(&format!("kind = \"bounty\"\n{}", quest_body())).expect("a quest");
        // Level 10 is Tier 1; there is no maximum.
        assert!(!q.offered_at_tier(0) && q.offered_at_tier(1) && q.offered_at_tier(3));
        let later: Quest = toml::from_str(&format!(
            "kind = \"bounty\"\nmin_tier = 2\n{}",
            quest_body()
        ))
        .expect("a quest");
        assert!(!later.offered_at_tier(1) && later.offered_at_tier(2));
        // A Tier 2 character is offered a level 3 quest.
        let low: Quest = toml::from_str(&format!(
            "kind = \"hunting\"\n{}",
            quest_body().replace("level = 10", "level = 3")
        ))
        .expect("a quest");
        assert!(low.offered_at_tier(0) && low.offered_at_tier(2));
        // Unshrunk however far above it the player stands.
        assert_eq!(q.reward_xp(11), q.reward_xp(40));
    }

    fn quest_body() -> &'static str {
        r#"id = 1
        giver = 1
        level = 10
        title = ""
        objectives = ""
        details = ""
        progress = ""
        completion = ""
        money = 0"#
    }

    /// [`quest_body`] given by Uzzek, a gossiping Hub quest giver, with `extra` keys before it.
    fn at_uzzek(extra: &str) -> Quest {
        toml::from_str(&format!(
            "{extra}\n{}",
            quest_body().replace("giver = 1", "giver = 5810")
        ))
        .expect("a quest")
    }

    #[test]
    #[should_panic(expected = "trial names quest 99999, which the content does not have")]
    fn a_trial_naming_a_missing_quest_fails_the_load() {
        let q = at_uzzek(
            "kind = \"advancement\"\n\
             advancement = { from = \"vanguard\", to = \"zealot\", trial = 99999 }",
        );
        check(&q, &|_| false, &|_| false);
    }

    #[test]
    #[should_panic(
        expected = "quest 1 hints at 1, who is no gossiping Hub NPC and no friendly spawn"
    )]
    fn a_hint_at_an_npc_who_neither_gossips_nor_is_a_friendly_spawn_fails_the_load() {
        let q = at_uzzek("kind = \"story\"\nhint = [{ npc = 1, option = \"?\", reply = \"!\" }]");
        check(&q, &|_| false, &|_| false);
    }

    #[test]
    #[should_panic(expected = "trial 2 does not chain back to it")]
    fn a_trial_that_does_not_chain_back_fails_the_load() {
        let q = at_uzzek(
            "kind = \"advancement\"\n\
             advancement = { from = \"vanguard\", to = \"zealot\", trial = 2 }",
        );
        let unchained = at_uzzek("kind = \"story\"").with_id(2);
        check_trial(&q, &|id| (id == 2).then_some(&unchained));
    }

    #[test]
    fn a_trial_reached_through_after_and_after_taken_passes_the_load() {
        let q = at_uzzek(
            "kind = \"advancement\"\n\
             advancement = { from = \"vanguard\", to = \"zealot\", trial = 3 }",
        );
        let first = at_uzzek("kind = \"story\"\nafter_taken = 1").with_id(2);
        let last = at_uzzek("kind = \"story\"\nafter = 2").with_id(3);
        check_trial(&q, &|id| [&first, &last].into_iter().find(|t| t.id == id));
    }

    impl Quest {
        fn with_id(mut self, id: u32) -> Quest {
            self.id = id;
            self
        }
    }

    #[test]
    fn a_quest_without_a_kind_is_no_quest() {
        let no_kind = r#"
            id = 1
            giver = 1
            level = 10
            title = ""
            objectives = ""
            details = ""
            progress = ""
            completion = ""
            money = 0
        "#;
        assert!(toml::from_str::<Quest>(no_kind).is_err());
    }
}
