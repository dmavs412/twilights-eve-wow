//! The item templates (`content/items.toml`, and Gear's `content/gear.toml`, generated from
//! `data/twilights-eve/gear.csv`), and the `SMSG_ITEM_QUERY_SINGLE_RESPONSE` each one answers with.

use std::collections::HashMap;
use std::sync::OnceLock;

use benilla_protocol::messages::{ItemDamage, ItemInfo, ItemSpellEntry, ItemUseSpell};
use benilla_protocol::SessionEvent;
use serde::Deserialize;

/// A profession with a Journal (ADR 0023), as an item row's `journal` key spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Profession {
    Fishing,
    Herbalism,
}

/// One item template, as the realm's table spells it. A field the table leaves out is at its
/// default.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub(crate) struct Template {
    pub(crate) entry: u32,
    pub(crate) name: String,
    pub(crate) quality: u8,
    pub(crate) item_level: u8,
    pub(crate) required_level: u8,
    pub(crate) class: u32,
    pub(crate) subclass: u32,
    pub(crate) inventory_type: u32,
    pub(crate) allowable_class: i32,
    pub(crate) allowable_race: i32,
    pub(crate) display_id: u32,
    /// `[stat id, value]`.
    pub(crate) stats: Vec<[i32; 2]>,
    /// Holy, fire, nature, frost, shadow, arcane.
    pub(crate) resistances: [i32; 6],
    /// `[min, max, school]`.
    pub(crate) damage: Vec<[f32; 3]>,
    pub(crate) equip_spells: Vec<u32>,
    /// Flat attack power from the equip spells.
    pub(crate) attack_power: i32,
    /// Defense skill from the equip spells.
    pub(crate) defense: u16,
    /// Flat Spell Power from the sheet's `spell_power` column (ADR 0007); the "Increase Spell
    /// Dam N" spell among `equip_spells` is the client's tooltip, never the realm's number.
    pub(crate) spell_power: u32,
    /// A quiver's or ammo pouch's ranged haste from its equip spell (aura 141,
    /// `SPELL_AURA_MOD_RANGED_AMMO_HASTE`): the percent the ranged weapon shoots faster while it is
    /// in a bag slot.
    pub(crate) ranged_haste_pct: u32,
    pub(crate) armor: u32,
    pub(crate) block: u32,
    pub(crate) speed_ms: u32,
    pub(crate) durability: u32,
    pub(crate) sell_price: u32,
    pub(crate) buy_price: u32,
    pub(crate) buy_count: u32,
    pub(crate) bonding: u32,
    pub(crate) max_count: u32,
    pub(crate) stackable: u32,
    pub(crate) container_slots: u32,
    pub(crate) sheath: u32,
    pub(crate) material: i32,
    pub(crate) pool_level: u8,
    /// The on-use spell, 0 for none.
    pub(crate) use_spell: u32,
    /// The on-use cooldown, -1 for the spell's own.
    pub(crate) use_cooldown_ms: i32,
    /// The on-use category cooldown, -1 for the spell's own.
    pub(crate) use_category_cooldown_ms: i32,
    /// Food and drink (`content/goods.toml`): what eating or drinking restores over
    /// `duration_ms`.
    pub(crate) restores_health: u32,
    pub(crate) restores_mana: u32,
    pub(crate) duration_ms: u32,
    /// A Chest's Wardrobe (ADR 0007): the look template worn on each hidden body part, in
    /// [`WARDROBE_SLOTS`] order, 0 for bare. Empty on anything but a Chest.
    pub(crate) wardrobe: Vec<u32>,
    /// The quest it begins (a Quest Starter), 0 for none: the client shows "This item begins a
    /// quest", and using it opens the quest's offer.
    pub(crate) start_quest: u32,
    /// A teleport scroll: where its use spell (the Hearthstone's) takes the reader on the same map,
    /// `[x, y, z, facing]`, using the scroll up. None for any other item.
    pub(crate) teleport_to: Option<[f32; 4]>,
    /// The tooltip's yellow flavor line; empty for none.
    pub(crate) description: String,
    /// The profession Journal this item belongs to (ADR 0023): a received one goes to its
    /// Satchel, never a bag slot.
    pub(crate) journal: Option<Profession>,
    /// A readable item's pages, one string each. The client's reader expands `$N`, `$B` and the
    /// other macros; the realm sends the text as written.
    pub(crate) pages: Vec<String>,
}

impl Default for Template {
    fn default() -> Self {
        Self {
            entry: 0,
            name: String::new(),
            quality: 0,
            item_level: 0,
            required_level: 0,
            class: 0,
            subclass: 0,
            inventory_type: 0,
            allowable_class: -1,
            allowable_race: -1,
            display_id: 0,
            stats: Vec::new(),
            resistances: [0; 6],
            damage: Vec::new(),
            equip_spells: Vec::new(),
            attack_power: 0,
            defense: 0,
            spell_power: 0,
            ranged_haste_pct: 0,
            armor: 0,
            block: 0,
            speed_ms: 0,
            durability: 0,
            sell_price: 0,
            buy_price: 0,
            buy_count: 0,
            bonding: 0,
            max_count: 0,
            start_quest: 0,
            teleport_to: None,
            description: String::new(),
            journal: None,
            pages: Vec::new(),
            stackable: 0,
            container_slots: 0,
            sheath: 0,
            material: 0,
            pool_level: 0,
            use_spell: 0,
            use_cooldown_ms: -1,
            use_category_cooldown_ms: -1,
            restores_health: 0,
            restores_mana: 0,
            duration_ms: 0,
            wardrobe: Vec::new(),
        }
    }
}

impl Template {
    /// How many fit in one bag slot.
    pub(crate) fn stack_size(&self) -> u32 {
        self.stackable.max(1)
    }

    /// What the content load refuses in the item's `pages`.
    pub(crate) fn check_pages(&self) {
        let entry = self.entry;
        assert!(
            self.pages.len() <= MAX_PAGES,
            "item {entry} has more than {MAX_PAGES} pages"
        );
        for (i, page) in self.pages.iter().enumerate() {
            assert!(
                !page.is_empty() && page.len() <= MAX_PAGE_BYTES,
                "item {entry}'s page {i} must be 1 to {MAX_PAGE_BYTES} bytes"
            );
        }
    }
}

/// At most this many pages per item (the page id formula leaves room for 16).
const MAX_PAGES: usize = 10;
/// The longest page body, in bytes.
const MAX_PAGE_BYTES: usize = 2000;
/// `PageTextMaterial.dbc` id 1, Parchment.
const PARCHMENT: u32 = 1;

/// The id of page `index` (0-based) of item `entry`. `entry * 16 + index` never collides across
/// items, since an item has at most 16 pages.
pub(crate) fn page_id(entry: u32, index: usize) -> u32 {
    entry * 16 + index as u32
}

/// The `PageText` events for the chain that starts at `page_id`, in order: one per page to the last,
/// each pointing at the next (0 on the last), as vmangos answers a first query. An unknown page id
/// answers nothing.
pub(crate) fn page_chain(page_id: u32) -> Vec<SessionEvent> {
    let (entry, first) = (page_id / 16, (page_id % 16) as usize);
    let Some(t) = template(entry) else {
        return Vec::new();
    };
    (first..t.pages.len())
        .map(|i| SessionEvent::PageText {
            page_id: self::page_id(entry, i),
            text: t.pages[i].clone(),
            next_page_id: if i + 1 < t.pages.len() {
                self::page_id(entry, i + 1)
            } else {
                0
            },
        })
        .collect()
}

#[derive(Deserialize)]
struct Table {
    item: Vec<Template>,
}

/// The equipment slots a Chest's Wardrobe dresses (`EquipmentSlots`), in `wardrobe` order: head,
/// shoulders, back, wrists, hands, waist, legs, feet.
pub(crate) const WARDROBE_SLOTS: [u8; 8] = [0, 2, 14, 8, 9, 5, 6, 7];

/// Every world-drop template, by entry: what the loot pools draw from. The junk and bags of
/// `content/items.toml`, and the Gear a world pool holds (a `pool_level`).
pub(crate) fn all() -> &'static HashMap<u32, Template> {
    static ITEMS: OnceLock<HashMap<u32, Template>> = OnceLock::new();
    ITEMS.get_or_init(|| {
        let table: Table = toml::from_str(include_str!("../content/items.toml"))
            .expect("content/items.toml parses");
        table
            .item
            .into_iter()
            .map(|t| (t.entry, t))
            .chain(
                gear()
                    .values()
                    .filter(|t| t.pool_level > 0)
                    .map(|t| (t.entry, t.clone())),
            )
            .collect()
    })
}

/// Every piece of Gear and the Wardrobe's look templates (`content/gear.toml`), by entry.
fn gear() -> &'static HashMap<u32, Template> {
    static ITEMS: OnceLock<HashMap<u32, Template>> = OnceLock::new();
    ITEMS.get_or_init(|| {
        let table: Table =
            toml::from_str(include_str!("../content/gear.toml")).expect("content/gear.toml parses");
        table.item.into_iter().map(|t| (t.entry, t)).collect()
    })
}

/// The goods vendor's food and drink (`content/goods.toml`), by entry.
fn goods() -> &'static HashMap<u32, Template> {
    static ITEMS: OnceLock<HashMap<u32, Template>> = OnceLock::new();
    ITEMS.get_or_init(|| {
        let table: Table = toml::from_str(include_str!("../content/goods.toml"))
            .expect("content/goods.toml parses");
        table.item.into_iter().map(|t| (t.entry, t)).collect()
    })
}

/// Fishing's fish, poles, lures, baits and treasure (`content/fish.toml`), by entry.
fn fishing() -> &'static HashMap<u32, Template> {
    static ITEMS: OnceLock<HashMap<u32, Template>> = OnceLock::new();
    ITEMS.get_or_init(|| {
        let table: Table =
            toml::from_str(include_str!("../content/fish.toml")).expect("content/fish.toml parses");
        table.item.into_iter().map(|t| (t.entry, t)).collect()
    })
}

/// Herbalism's herbs, finds, tools and consumables (`content/herbs.toml`), by entry.
fn herbalism() -> &'static HashMap<u32, Template> {
    static ITEMS: OnceLock<HashMap<u32, Template>> = OnceLock::new();
    ITEMS.get_or_init(|| {
        let table: Table = toml::from_str(include_str!("../content/herbs.toml"))
            .expect("content/herbs.toml parses");
        table.item.into_iter().map(|t| (t.entry, t)).collect()
    })
}

/// The items the realm hands out itself (`content/given_items.toml`), by entry.
fn given() -> &'static HashMap<u32, Template> {
    static ITEMS: OnceLock<HashMap<u32, Template>> = OnceLock::new();
    ITEMS.get_or_init(|| {
        let table: Table = toml::from_str(include_str!("../content/given_items.toml"))
            .expect("content/given_items.toml parses");
        table.item.into_iter().map(|t| (t.entry, t)).collect()
    })
}

/// Whether `entry` is Gear the ten Gear Slots retired (`content/retired_items.toml`): a save that
/// holds one loads without it.
pub(crate) fn retired(entry: u32) -> bool {
    #[derive(Deserialize)]
    struct Retired {
        entries: Vec<u32>,
    }
    static RETIRED: OnceLock<Vec<u32>> = OnceLock::new();
    RETIRED
        .get_or_init(|| {
            let table: Retired = toml::from_str(include_str!("../content/retired_items.toml"))
                .expect("content/retired_items.toml parses");
            table.entries
        })
        .binary_search(&entry)
        .is_ok()
}

pub(crate) fn template(entry: u32) -> Option<&'static Template> {
    all()
        .get(&entry)
        .or_else(|| given().get(&entry))
        .or_else(|| goods().get(&entry))
        .or_else(|| fishing().get(&entry))
        .or_else(|| herbalism().get(&entry))
        .or_else(|| crate::quests::item_template(entry))
        .or_else(|| gear().get(&entry))
}

/// `ITEM_SPELLTRIGGER_ON_USE`.
const ON_USE: u32 = 0;
/// `ITEM_SPELLTRIGGER_ON_EQUIP`.
const ON_EQUIP: u32 = 1;

/// The spell category an on-use spell shares (`Spell.dbc`'s `Category`): food 11, drink 59.
fn use_category(t: &Template) -> u32 {
    if t.restores_mana > 0 {
        59
    } else if t.restores_health > 0 {
        11
    } else {
        0
    }
}

/// Food and drink are eaten: the use takes one of the stack (charges -1).
fn use_charges(t: &Template) -> i32 {
    if t.restores_health > 0 || t.restores_mana > 0 {
        -1
    } else {
        0
    }
}

/// `SMSG_ITEM_QUERY_SINGLE_RESPONSE` for `entry`; an unknown entry answers with none.
pub(crate) fn query_response(entry: u32) -> SessionEvent {
    SessionEvent::ItemTemplate {
        entry,
        info: template(entry)
            .map(info)
            .or_else(|| crate::token_vendor::row_info(entry))
            .map(Box::new),
    }
}

pub(crate) fn info(t: &Template) -> ItemInfo {
    let damages: Vec<ItemDamage> = t
        .damage
        .iter()
        .map(|&[min, max, school]| ItemDamage {
            min,
            max,
            school: school as u32,
        })
        .collect();
    let first = damages.first().copied();
    let [holy, fire, nature, frost, shadow, arcane] = t.resistances;
    // The on-use spell takes the first block, as the stock Hearthstone's does.
    let use_spell = (t.use_spell != 0).then_some(ItemUseSpell {
        spell_id: t.use_spell,
        cooldown_ms: t.use_cooldown_ms,
        category: use_category(t),
        category_cooldown_ms: t.use_category_cooldown_ms,
    });
    let mut spells: Vec<ItemSpellEntry> = use_spell
        .iter()
        .map(|u| ItemSpellEntry {
            index: 0,
            spell_id: u.spell_id,
            trigger: ON_USE,
            charges: use_charges(t),
            cooldown_ms: u.cooldown_ms,
            category: u.category,
            category_cooldown_ms: u.category_cooldown_ms,
        })
        .collect();
    let first_equip = spells.len();
    spells.extend(
        t.equip_spells
            .iter()
            .enumerate()
            .map(|(i, &spell_id)| ItemSpellEntry {
                index: (first_equip + i) as u8,
                spell_id,
                trigger: ON_EQUIP,
                charges: 0,
                cooldown_ms: -1,
                category: 0,
                category_cooldown_ms: -1,
            }),
    );
    ItemInfo {
        class: t.class,
        subclass: t.subclass,
        name: t.name.clone(),
        display_info_id: t.display_id,
        quality: u32::from(t.quality),
        flags: 0,
        buy_price: t.buy_price,
        sell_price: t.sell_price,
        inventory_type: t.inventory_type,
        allowable_class: t.allowable_class,
        allowable_race: t.allowable_race,
        item_level: u32::from(t.item_level),
        required_level: crate::rules::gear::SHOWN_REQUIRED_LEVEL,
        required_skill: 0,
        required_skill_rank: 0,
        required_spell: 0,
        required_honor_rank: 0,
        required_city_rank: 0,
        required_rep_faction: 0,
        required_rep_rank: 0,
        max_count: t.max_count,
        stackable: t.stackable,
        container_slots: t.container_slots,
        stats: t
            .stats
            .iter()
            .map(|&[stat, value]| (stat as u32, value))
            .collect(),
        dmg_min: first.map_or(0.0, |d| d.min),
        dmg_max: first.map_or(0.0, |d| d.max),
        dmg_type: first.map_or(0, |d| d.school),
        damages,
        armor: t.armor,
        resistances: [holy, fire, nature, frost, shadow, arcane],
        delay_ms: t.speed_ms,
        ammo_type: 0,
        ranged_mod_range: 0.0,
        spell_charges_0: spells.first().map_or(0, |s| s.charges),
        spells,
        use_spell,
        // Items never bind, so every item can be traded: no tooltip bind line, no bind popup.
        bonding: 0,
        description: t.description.clone(),
        page_text: if t.pages.is_empty() {
            0
        } else {
            page_id(t.entry, 0)
        },
        language_id: 0,
        page_material: if t.pages.is_empty() { 0 } else { PARCHMENT },
        start_quest: t.start_quest,
        lock_id: 0,
        material: t.material as u32,
        sheath: t.sheath,
        random_property: 0,
        block: t.block,
        item_set: 0,
        max_durability: t.durability,
        area: 0,
        map: 0,
        bag_family: 0,
    }
}
