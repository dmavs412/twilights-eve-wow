//! The item templates (`content/items.toml`), and the `SMSG_ITEM_QUERY_SINGLE_RESPONSE` each one
//! answers with.

use std::collections::HashMap;
use std::sync::OnceLock;

use benilla_protocol::messages::{ItemDamage, ItemInfo, ItemSpellEntry, ItemUseSpell};
use benilla_protocol::SessionEvent;
use serde::Deserialize;

/// One item template, as the realm's table spells it. A field the table leaves out is at its
/// default.
#[derive(Debug, Deserialize)]
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
            armor: 0,
            block: 0,
            speed_ms: 0,
            durability: 0,
            sell_price: 0,
            buy_price: 0,
            buy_count: 0,
            bonding: 0,
            max_count: 0,
            stackable: 0,
            container_slots: 0,
            sheath: 0,
            material: 0,
            pool_level: 0,
            use_spell: 0,
            use_cooldown_ms: -1,
            use_category_cooldown_ms: -1,
        }
    }
}

impl Template {
    /// How many fit in one bag slot.
    pub(crate) fn stack_size(&self) -> u32 {
        self.stackable.max(1)
    }
}

#[derive(Deserialize)]
struct Table {
    item: Vec<Template>,
}

/// Every world-drop template, by entry: what the loot pools draw from.
pub(crate) fn all() -> &'static HashMap<u32, Template> {
    static ITEMS: OnceLock<HashMap<u32, Template>> = OnceLock::new();
    ITEMS.get_or_init(|| {
        let table: Table = toml::from_str(include_str!("../content/items.toml"))
            .expect("content/items.toml parses");
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

pub(crate) fn template(entry: u32) -> Option<&'static Template> {
    all().get(&entry).or_else(|| given().get(&entry))
}

/// `ITEM_SPELLTRIGGER_ON_USE`.
const ON_USE: u32 = 0;
/// `ITEM_SPELLTRIGGER_ON_EQUIP`.
const ON_EQUIP: u32 = 1;

/// `SMSG_ITEM_QUERY_SINGLE_RESPONSE` for `entry`; an unknown entry answers with none.
pub(crate) fn query_response(entry: u32) -> SessionEvent {
    SessionEvent::ItemTemplate {
        entry,
        info: template(entry).map(|t| Box::new(info(t))),
    }
}

fn info(t: &Template) -> ItemInfo {
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
        category: 0,
        category_cooldown_ms: t.use_category_cooldown_ms,
    });
    let mut spells: Vec<ItemSpellEntry> = use_spell
        .iter()
        .map(|u| ItemSpellEntry {
            index: 0,
            spell_id: u.spell_id,
            trigger: ON_USE,
            charges: 0,
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
        required_level: u32::from(t.required_level),
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
        spells,
        spell_charges_0: 0,
        use_spell,
        bonding: t.bonding,
        description: String::new(),
        page_text: 0,
        language_id: 0,
        page_material: 0,
        start_quest: 0,
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
