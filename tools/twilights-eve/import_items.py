#!/usr/bin/env python3
"""Import Twilight's Eve item data from the vmangos world DB and the 1.12.1 client DBCs.

Writes, under --out (default data/twilights-eve):
  loot_items.json    world-drop gear pools for levels 1-32 (grey/white/green/blue) + the WC blue pool
  token_vendor.json  proposed Token-vendor greens by level band, with a proposed Token cost
  goods_vendor.json  food and drink for levels 1-30, durability rules and the repair-cost tables
  warrior_proficiency.json  which item class/subclass a Warrior can use, and from what level

Inputs:
  --db    vmangos `mangos.sqlite` (the `db-sqlite-*.zip` asset of github.com/vmangos/core release
          `db_latest`). With --fetch, it is downloaded and unpacked into --cache when missing.
  --data  the WoW 1.12.1 `Data` directory (MPQs). DurabilityCosts.dbc, DurabilityQuality.dbc and
          SpellDuration.dbc are read from the patch chain.

Everything is patch-gated to 1.12 (vmangos patch 10, client build 5875). Stdlib only; see
docs/reference/items-and-vendors.md for the filters and how to re-run.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import re
import sqlite3
import sys
import urllib.request
import zipfile
from collections import defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mpq_dbc import parse_dbc, read_dbc_file  # noqa: E402

PATCH = 10          # vmangos patch index for 1.12
BUILD = 5875        # 1.12.1 client build
MAX_LEVEL = 32      # loot pools cover item levels 1..MAX_LEVEL
WARRIOR = 1         # class id; class mask bit is 1 << (id - 1)
WC_UNSPAWNED_BOSSES = (3652, 3654, 3655, 3672)  # Trigore, Mutanus, Mad Magglish, Boahn

REPO = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
DEFAULT_CACHE = os.path.join(os.environ.get("LOCALAPPDATA", os.path.expanduser("~")),
                             "Temp", "claude", "twe-cache", "vmangos")
DEFAULT_DATA = os.environ.get("WOW_DATA", r"C:\Users\davis\wow-data\Data")

QUALITY = {0: "poor", 1: "common", 2: "uncommon", 3: "rare", 4: "epic"}
STAT = {0: "mana", 1: "health", 3: "agility", 4: "strength", 5: "intellect", 6: "spirit",
        7: "stamina"}
ITEM_CLASS = {0: "consumable", 1: "container", 2: "weapon", 4: "armor", 7: "trade_goods",
              12: "quest", 15: "miscellaneous"}
WEAPON_SUB = {0: "axe_1h", 1: "axe_2h", 2: "bow", 3: "gun", 4: "mace_1h", 5: "mace_2h",
              6: "polearm", 7: "sword_1h", 8: "sword_2h", 10: "staff", 13: "fist", 14: "misc",
              15: "dagger", 16: "thrown", 17: "spear", 18: "crossbow", 19: "wand",
              20: "fishing_pole"}
ARMOR_SUB = {0: "misc", 1: "cloth", 2: "leather", 3: "mail", 4: "plate", 5: "buckler",
             6: "shield", 7: "libram", 8: "idol", 9: "totem"}
INV_TYPE = {0: "none", 1: "head", 2: "neck", 3: "shoulder", 4: "shirt", 5: "chest", 6: "waist",
            7: "legs", 8: "feet", 9: "wrist", 10: "hands", 11: "finger", 12: "trinket",
            13: "one_hand", 14: "shield", 15: "ranged", 16: "back", 17: "two_hand", 18: "bag",
            19: "tabard", 20: "robe", 21: "main_hand", 22: "off_hand", 23: "holdable",
            24: "ammo", 25: "thrown", 26: "ranged_right", 27: "quiver", 28: "relic"}
SCHOOL = {0: "physical", 1: "holy", 2: "fire", 3: "nature", 4: "frost", 5: "shadow", 6: "arcane"}

# ---------------------------------------------------------------- Warrior proficiency (vanilla)
# Weapon skill spells known at creation, by race (playercreateinfo_spell, class 1). Every other
# Warrior weapon skill is bought from a Weapon Master at level 1, except Polearms (level 20).
SKILL_SPELL_TO_WEAPON = {196: 0, 197: 1, 264: 2, 266: 3, 198: 4, 199: 5, 200: 6, 201: 7, 202: 8,
                         227: 10, 15590: 13, 1180: 15, 2567: 16, 5011: 18}
RACE = {1: "human", 2: "orc", 3: "dwarf", 4: "night_elf", 5: "undead", 6: "tauren", 7: "gnome",
        8: "troll"}
# weapon subclass -> level a Warrior can first use it (trainable skills, vanilla Weapon Masters)
WARRIOR_WEAPON_LEVEL = {0: 1, 1: 1, 2: 1, 3: 1, 4: 1, 5: 1, 6: 20, 7: 1, 8: 1, 10: 1, 13: 1,
                        14: 1, 15: 1, 16: 1, 18: 1, 20: 1}
# armor subclass -> level (Plate Mail is trained at 40; the rest is known at creation)
WARRIOR_ARMOR_LEVEL = {0: 1, 1: 1, 2: 1, 3: 1, 4: 40, 5: 1, 6: 1}

# ---------------------------------------------------------------- equip auras we can honour
# aura id -> stat key. The value is the spell's effect amount; skill/resistance keep their misc.
SIMPLE_EQUIP_AURAS = {
    99: "attack_power", 124: "ranged_attack_power", 52: "crit_pct", 54: "hit_pct",
    49: "dodge_pct", 47: "parry_pct", 51: "block_pct", 158: "block_value", 30: "skill",
    22: "resistance", 161: "health_regen_in_combat_p5", 84: "health_regen_p5", 29: "stat",
}
# SPELL_AURA_MOD_STAT misc value -> stat (-1 = all stats)
UNIT_STAT = {0: "strength", 1: "agility", 2: "stamina", 3: "intellect", 4: "spirit", -1: "all"}
RESIST_MASK = {1: "armor", 2: "holy_res", 4: "fire_res", 8: "nature_res", 16: "frost_res",
               32: "shadow_res", 64: "arcane_res", 126: "all_res", 127: "armor_and_all_res"}


def aura_mod(sp, i):
    """One equip-aura effect as {"stat", "value"}; None when the realm cannot honour it."""
    aura = sp[f"effectApplyAuraName{i}"]
    if sp[f"effect{i}"] != 6 or aura not in SIMPLE_EQUIP_AURAS:
        return None
    misc = sp[f"effectMiscValue{i}"]
    value = spell_amount(sp, i)
    if aura == 29:
        return {"stat": UNIT_STAT.get(misc, f"stat_{misc}"), "value": value}
    if aura == 22:
        return {"stat": RESIST_MASK.get(misc, f"resist_mask_{misc}"), "value": value}
    if aura == 30:
        return {"stat": "skill", "skill": misc, "value": value}
    return {"stat": SIMPLE_EQUIP_AURAS[aura], "value": value}
NAME_JUNK = re.compile(r"(?i)\b(test|testing|deprecated|placeholder|unused)\b|\[ph\]|\(ph\)|"
                       r"^monster\s*-|^zz|^qa|(?-i:\bOLD\b)")


def connect(path: str) -> sqlite3.Connection:
    con = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    con.row_factory = sqlite3.Row
    return con


def fetch_db(cache: str) -> str:
    """Download and unpack vmangos' db_latest sqlite snapshot into `cache`; return mangos.sqlite."""
    os.makedirs(cache, exist_ok=True)
    api = "https://api.github.com/repos/vmangos/core/releases/tags/db_latest"
    with urllib.request.urlopen(api) as r:
        rel = json.load(r)
    asset = next(a for a in rel["assets"] if a["name"].startswith("db-sqlite-"))
    dest = os.path.join(cache, asset["name"])
    if not os.path.exists(dest):
        tmp = dest + ".part"
        print(f"downloading {asset['browser_download_url']}", file=sys.stderr)
        urllib.request.urlretrieve(asset["browser_download_url"], tmp)
        os.replace(tmp, dest)
    with zipfile.ZipFile(dest) as z:
        member = next(n for n in z.namelist() if n.endswith("mangos.sqlite"))
        out = os.path.join(cache, member)
        if not os.path.exists(out):
            tmp = out + ".part"
            os.makedirs(os.path.dirname(out), exist_ok=True)
            with z.open(member) as src, open(tmp, "wb") as dst:
                while chunk := src.read(1 << 20):
                    dst.write(chunk)
            os.replace(tmp, out)
    return out


# ---------------------------------------------------------------- loading
def latest_by(con, table, key, ver_col, ver_max, where=""):
    """Rows of `table` with the highest `ver_col` <= ver_max per `key` (vmangos patch/build gating)."""
    out = {}
    sql = f"SELECT * FROM {table} WHERE {ver_col} <= ? {where} ORDER BY {ver_col}"
    for r in con.execute(sql, (ver_max,)):
        out[r[key]] = r
    return out


def load(con):
    items = latest_by(con, "item_template", "entry", "patch", PATCH)
    forbidden = {r[0] for r in con.execute(
        "SELECT entry FROM forbidden_items WHERE (after_or_before = 0 AND patch <= ?) "
        "OR (after_or_before = 1 AND patch >= ?)", (PATCH, PATCH))}
    spells = latest_by(con, "spell_template", "entry", "build", BUILD)
    creatures = latest_by(con, "creature_template", "entry", "patch", PATCH)
    maps = latest_by(con, "map_template", "entry", "patch", PATCH)
    icons = {r[0]: r[1] for r in con.execute("SELECT ID, icon FROM item_display_info")}
    spawns = defaultdict(set)  # creature entry -> {map}
    for r in con.execute("SELECT id, id2, id3, id4, id5, map FROM creature "
                         "WHERE patch_min <= ? AND patch_max >= ?", (PATCH, PATCH)):
        for cid in r[:5]:
            if cid:
                spawns[cid].add(r[5])
    loot = {}
    for t in ("creature_loot_template", "reference_loot_template"):
        rows = defaultdict(list)
        for r in con.execute(f"SELECT entry, item, ChanceOrQuestChance, groupid, mincountOrRef, "
                             f"maxcount FROM {t} WHERE patch_min <= ? AND patch_max >= ?",
                             (PATCH, PATCH)):
            if r[4] >= 0 and r[1] in forbidden:
                continue
            rows[r[0]].append(tuple(r))
        loot[t] = rows
    return items, forbidden, spells, creatures, maps, icons, spawns, loot


def resolve_loot(loot):
    """creature loot_id -> {item: (max chance seen, quest_drop)}, references expanded."""
    refs = loot["reference_loot_template"]
    memo = {}

    def expand(ref_id, stack=()):
        if ref_id in memo:
            return memo[ref_id]
        if ref_id in stack:
            return {}
        out = {}
        for _e, item, chance, _g, mref, _mx in refs.get(ref_id, ()):
            if mref < 0:
                for k, v in expand(-mref, stack + (ref_id,)).items():
                    out.setdefault(k, v)
            else:
                out.setdefault(item, (chance, chance < 0))
        memo[ref_id] = out
        return out

    by_loot = {}
    for lid, rows in loot["creature_loot_template"].items():
        out = {}
        for _e, item, chance, _g, mref, _mx in rows:
            if mref < 0:
                for k, v in expand(-mref).items():
                    out.setdefault(k, v)
            else:
                out[item] = (chance, chance < 0)
        by_loot[lid] = out
    return by_loot


ENCH_STAT = {0: "mana", 1: "health", 3: "agility", 4: "strength", 5: "intellect", 6: "spirit",
             7: "stamina"}
ENCH_RES = {0: "armor", 1: "holy_res", 2: "fire_res", 3: "nature_res", 4: "frost_res",
            5: "shadow_res", 6: "arcane_res"}


def load_random_suffixes(con, data_dir, spells):
    """random_property id -> [{property, suffix, chance_pct, mods, supported}].

    vmangos rolls item_enchantment_template(entry = item.random_property) by chance into an
    ItemRandomProperties.dbc id; its enchantments (SpellItemEnchantment.dbc) carry the stats.
    The client renders the suffix itself from ITEM_FIELD_RANDOM_PROPERTIES_ID."""
    irp = {r[0]: r for r in parse_dbc(read_dbc_file(data_dir, "ItemRandomProperties.dbc")[0],
                                      "us" + "uuuuu" + "s" * 8 + "u")}
    sie = {r[0]: r for r in parse_dbc(read_dbc_file(data_dir, "SpellItemEnchantment.dbc")[0],
                                      "u" + "u" * 12 + "s" * 8 + "uuu")}
    out = defaultdict(list)
    for entry, ench, chance in con.execute(
            "SELECT entry, ench, chance FROM item_enchantment_template "
            "WHERE patch_min <= ? AND patch_max >= ?", (PATCH, PATCH)):
        p = irp.get(ench)
        if not p:
            continue
        mods, ok = [], True
        for eid in p[2:5]:
            e = sie.get(eid)
            if not eid or not e:
                continue
            for k in range(3):
                typ, amt, arg = e[1 + k], e[4 + k], e[10 + k]
                if typ == 0:
                    continue
                if typ == 5 and arg in ENCH_STAT:
                    mods.append({"stat": ENCH_STAT[arg], "value": amt})
                elif typ == 4 and arg in ENCH_RES:
                    mods.append({"stat": ENCH_RES[arg], "value": amt})
                elif typ == 3 and arg in spells:
                    # equip spell: 1.12 suffixes are MOD_STAT / MOD_RESISTANCE / AP auras
                    sp = spells[arg]
                    for i in (1, 2, 3):
                        if sp[f"effect{i}"] == 0:
                            continue
                        m = aura_mod(sp, i)
                        if m is None:
                            ok = False
                            m = {"spell": arg, "name": sp["name"],
                                 "aura": sp[f"effectApplyAuraName{i}"], "unsupported": True}
                        mods.append(m)
                else:
                    ok = False
                    mods.append({"enchant_type": typ, "value": amt, "arg": arg,
                                 "unsupported": True})
        out[entry].append({"property": ench, "suffix": p[7], "chance_pct": round(chance, 2),
                           "mods": mods, "supported": ok})
    for v in out.values():
        v.sort(key=lambda s: -s["chance_pct"])
    return out


# ---------------------------------------------------------------- per-item helpers
def spell_amount(sp, i):
    base = sp[f"effectBasePoints{i}"]
    dice, sides = sp[f"effectBaseDice{i}"], sp[f"effectDieSides{i}"]
    return base + dice if sides <= 1 else base + max(dice, 1)


def item_spells(it, spells):
    """(decoded effects, supported?) for an item's spell slots."""
    effects, ok = [], True
    for n in range(1, 6):
        sid = it[f"spellid_{n}"]
        if not sid:
            continue
        trig = it[f"spelltrigger_{n}"]
        sp = spells.get(sid)
        name = sp["name"] if sp else None
        eff = {"spell": sid, "name": name,
               "trigger": {0: "use", 1: "equip", 2: "chance_on_hit", 4: "soulstone",
                           5: "use_no_delay", 6: "learn"}.get(trig, trig)}
        if trig == 1 and sp:
            mods = []
            for i in (1, 2, 3):
                if sp[f"effect{i}"] == 0:
                    continue
                m = aura_mod(sp, i)
                if m is None:
                    ok = False
                    m = {"effect": sp[f"effect{i}"], "aura": sp[f"effectApplyAuraName{i}"],
                         "unsupported": True}
                mods.append(m)
            eff["mods"] = mods
        else:
            ok = False
        effects.append(eff)
    return effects, ok


def warrior_use(it):
    """(usable_at_all, from_level, reason) for a Warrior, ignoring the item's own required level."""
    mask = it["allowable_class"]
    if mask not in (-1, 0) and not mask & (1 << (WARRIOR - 1)) and mask & 0x7FF != 0x7FF:
        return False, None, "class_mask"
    cls, sub = it["class"], it["subclass"]
    if cls == 2:
        lvl = WARRIOR_WEAPON_LEVEL.get(sub)
        return (lvl is not None), lvl, None if lvl else f"weapon_{WEAPON_SUB.get(sub, sub)}"
    if cls == 4:
        if it["inventory_type"] in (4, 19):
            return True, 1, None  # shirt/tabard: cosmetic
        lvl = WARRIOR_ARMOR_LEVEL.get(sub)
        return (lvl is not None), lvl, None if lvl else f"armor_{ARMOR_SUB.get(sub, sub)}"
    return True, 1, None


def caster_only(it):
    stats = {it[f"stat_type{i}"]: it[f"stat_value{i}"] for i in range(1, 11) if it[f"stat_value{i}"]}
    return bool(stats) and set(stats) <= {0, 5, 6}


def describe(it, icons, spells, drop=None):
    stats = [{"stat": STAT.get(it[f"stat_type{i}"], it[f"stat_type{i}"]),
              "value": it[f"stat_value{i}"]}
             for i in range(1, 11) if it[f"stat_value{i}"]]
    dmg = [{"min": it[f"dmg_min{i}"], "max": it[f"dmg_max{i}"],
            "school": SCHOOL.get(it[f"dmg_type{i}"], it[f"dmg_type{i}"])}
           for i in range(1, 6) if it[f"dmg_max{i}"]]
    res = {k: it[f"{k}_res"] for k in ("holy", "fire", "nature", "frost", "shadow", "arcane")
           if it[f"{k}_res"]}
    effects, _ = item_spells(it, spells)
    usable, lvl, why = warrior_use(it)
    cls, sub = it["class"], it["subclass"]
    d = {
        "entry": it["entry"], "name": it["name"], "quality": it["quality"],
        "quality_name": QUALITY.get(it["quality"]), "item_level": it["item_level"],
        "required_level": it["required_level"], "class": cls,
        "class_name": ITEM_CLASS.get(cls, str(cls)), "subclass": sub,
        "subclass_name": (WEAPON_SUB if cls == 2 else ARMOR_SUB if cls == 4 else {}).get(sub),
        "inventory_type": it["inventory_type"],
        "inventory_type_name": INV_TYPE.get(it["inventory_type"]),
        "allowable_class": it["allowable_class"], "allowable_race": it["allowable_race"],
        "display_id": it["display_id"], "icon": icons.get(it["display_id"]),
        "stats": stats, "armor": it["armor"], "block": it["block"], "resistances": res,
        "damage": dmg, "speed_ms": it["delay"] if dmg else 0,
        "dps": round(sum((x["min"] + x["max"]) / 2 for x in dmg) / (it["delay"] / 1000), 1)
        if dmg and it["delay"] else 0,
        "durability": it["max_durability"], "sell_price": it["sell_price"],
        "buy_price": it["buy_price"], "buy_count": it["buy_count"],
        "bonding": it["bonding"], "max_count": it["max_count"], "stackable": it["stackable"],
        "container_slots": it["container_slots"], "sheath": it["sheath"],
        "material": it["material"], "effects": effects,
        "warrior": {"usable": usable, "from_level": lvl,
                    **({"blocked_by": why} if why else {})},
    }
    if drop:
        d["drop"] = drop
    return d


def junk_name(name):
    return bool(NAME_JUNK.search(name))


def gear_kind(it):
    cls, sub, inv = it["class"], it["subclass"], it["inventory_type"]
    if cls == 2 and sub not in (14, 20):
        return "weapon"
    if cls == 4 and inv not in (0, 4, 19):
        return "armor"
    if cls == 1 and sub == 0 and it["container_slots"]:
        return "bag"
    if cls in (12, 15) and it["quality"] == 0 and sub == 0 and not it["spellid_1"]:
        return "junk"  # vendor trash; class 12 greys are mostly sellable junk too, filtered below
    return None


# ---------------------------------------------------------------- main build
def build(args):
    con = connect(args.db)
    items, forbidden, spells, creatures, maps, icons, spawns, loot = load(con)
    by_loot = resolve_loot(loot)
    # Wailing Caverns bosses with no static spawn row: Mutanus is summoned by the Disciple of
    # Naralex event; Trigore, Boahn and Mad Magglish are script/pool spawns. Count them as WC.
    for cid in WC_UNSPAWNED_BOSSES:
        spawns[cid] = spawns.get(cid, set()) | {43}
    instance_maps = {m for m, r in maps.items() if r["map_type"] in (1, 2, 3)}

    # who drops what
    sources = defaultdict(list)  # item -> [(creature entry, open_world, quest_drop)]
    for cid, ct in creatures.items():
        if not ct["loot_id"] or cid not in spawns:
            continue
        drops = by_loot.get(ct["loot_id"], {})
        open_world = bool(spawns[cid] & {0, 1})
        for item, (_chance, quest) in drops.items():
            sources[item].append((cid, open_world, quest))

    random_suffixes = load_random_suffixes(con, args.data, spells)
    excluded = defaultdict(list)
    pool_items, random_items = {}, {}
    for entry, it in items.items():
        kind = gear_kind(it)
        if kind is None or it["quality"] > 3:
            continue
        if kind != "junk" and not 1 <= it["item_level"] <= MAX_LEVEL:
            continue
        if it["required_level"] > MAX_LEVEL:
            continue
        reason = None
        world = [s for s in sources.get(entry, ()) if s[1] and
                 creatures[s[0]]["rank"] != 3]  # rank 3 = boss
        world_nonquest = [s for s in world if not s[2]]
        if entry in forbidden:
            reason = "forbidden_in_1_12"
        elif it["extra_flags"] & 0x04:
            reason = "not_obtainable"
        elif it["flags"] & 0x10:
            reason = "deprecated_flag"
        elif junk_name(it["name"]):
            reason = "test_or_placeholder_name"
        elif it["bonding"] == 4 or it["start_quest"] or (it["class"] == 12 and it["quality"] > 0):
            reason = "quest_item"
        elif kind == "junk" and it["class"] == 12:
            reason = "quest_item"
        elif not world_nonquest:
            reason = "quest_drop_only" if world else "no_open_world_drop"
        elif it["bonding"] == 1 and it["quality"] >= 2:
            reason = "bind_on_pickup"
        else:
            _eff, ok = item_spells(it, spells)
            if not ok:
                reason = "spell_effect_unsupported"
        pool_level = it["item_level"]
        if not reason:
            avg = sorted((creatures[c]["level_min"] + creatures[c]["level_max"]) / 2
                         for c, _, _ in world_nonquest)
            if kind == "junk":
                # vendor trash has item level 1; it belongs to the level of the mobs that drop it
                pool_level = max(1, round(avg[len(avg) // 2]))
                if pool_level > MAX_LEVEL:
                    reason = "drops_only_above_level_32"
            elif avg[0] > MAX_LEVEL + 5:
                reason = "drops_only_above_level_32"
        if not reason and it["random_property"]:
            reason = "random_enchant"
        if reason:
            excluded[reason].append({"entry": entry, "name": it["name"],
                                     "quality": it["quality"], "item_level": it["item_level"]})
            if reason != "random_enchant":
                continue
        lv = [creatures[c]["level_min"] for c, _, _ in world_nonquest] + \
             [creatures[c]["level_max"] for c, _, _ in world_nonquest]
        drop = {"open_world_creatures": len({c for c, _, _ in world_nonquest}),
                "creature_level_min": min(lv), "creature_level_max": max(lv)}
        d = describe(it, icons, spells, drop)
        d["kind"] = kind
        d["pool_level"] = pool_level
        if kind != "junk" and caster_only(it):
            d["caster_stats_only"] = True
        if reason == "random_enchant":
            d["random_property"] = it["random_property"]
            d["random_suffix_count"] = len(random_suffixes.get(it["random_property"], []))
            random_items[entry] = d
        else:
            pool_items[entry] = d

    # indexes
    bands = [(1, 5), (6, 10), (11, 15), (16, 20), (21, 25), (26, 30), (31, 32)]

    def index(pool):
        by_l = defaultdict(lambda: defaultdict(list))
        by_b = {f"{a}-{b}": defaultdict(list) for a, b in bands}
        for e, d in sorted(pool.items(), key=lambda kv: (kv[1]["pool_level"], kv[0])):
            by_l[d["pool_level"]][d["quality"]].append(e)
            for a, b in bands:
                if a <= d["pool_level"] <= b:
                    by_b[f"{a}-{b}"][d["quality"]].append(e)
        return by_l, by_b

    by_ilvl, by_band = index(pool_items)
    rnd_ilvl, rnd_band = index(random_items)
    def band_counts(bb, pool):
        out = {}
        for band, q in bb.items():
            out[band] = {QUALITY[k]: len(v) for k, v in sorted(q.items())}
            out[band]["warrior_usable_gear"] = {
                QUALITY[k]: sum(1 for e in v if pool[e]["kind"] in ("weapon", "armor")
                                and pool[e]["warrior"]["usable"]
                                and not pool[e].get("caster_stats_only"))
                for k, v in sorted(q.items())}
        return out

    counts = band_counts(by_band, pool_items)
    rnd_counts = band_counts(rnd_band, random_items)

    # ---------------------------------------------------- Wailing Caverns blues
    wc_map = 43
    wc_creatures = {c for c, m in spawns.items() if wc_map in m and c in creatures}
    wc_pool = {}
    for cid in sorted(wc_creatures):
        ct = creatures[cid]
        for item, (_ch, quest) in by_loot.get(ct["loot_id"], {}).items():
            it = items.get(item)
            if not it or it["quality"] != 3 or quest or item in forbidden:
                continue
            if it["extra_flags"] & 0x04:
                continue
            wc_pool.setdefault(item, set()).add(ct["name"])
    wc_list = []
    for item, names in sorted(wc_pool.items()):
        it = items[item]
        d = describe(it, icons, spells)
        _eff, ok = item_spells(it, spells)
        d["effects_supported"] = ok
        if caster_only(it):
            d["caster_stats_only"] = True
        # WC-only: every creature that drops it lives on map 43 (the named boss/rare drops);
        # otherwise it is a world-drop BoE blue that WC trash can also drop.
        d["wailing_caverns_only"] = all(spawns[c] == {wc_map} for c, _, q in sources[item]
                                        if not q)
        d["dropped_by"] = sorted(names)[:8]
        usable, lvl, _ = warrior_use(it)
        d["proposed_for_boss"] = bool(usable and lvl <= 30 and ok
                                      and not d.get("caster_stats_only")
                                      and not (it["class"] == 4 and it["subclass"] == 1
                                               and it["inventory_type"] != 16))
        wc_list.append(d)
    wc_list.sort(key=lambda d: (not d["wailing_caverns_only"], d["required_level"], d["entry"]))

    # extra: level 17-25 warrior blues from any creature drop (dungeons or world), fixed stats
    extra = []
    map_name = {m: r["map_name"] for m, r in maps.items()}
    for entry, it in items.items():
        if it["quality"] != 3 or entry in wc_pool or entry in forbidden:
            continue
        if not 17 <= it["required_level"] <= 25 or gear_kind(it) not in ("weapon", "armor"):
            continue
        if it["extra_flags"] & 0x04 or it["flags"] & 0x10 or junk_name(it["name"])                 or it["random_property"] or it["bonding"] == 4:
            continue
        srcs = [s for s in sources.get(entry, ()) if not s[2]]
        if not srcs:
            continue
        usable, lvl, _ = warrior_use(it)
        if not usable or lvl > it["required_level"] or caster_only(it):
            continue
        if it["class"] == 4 and it["subclass"] == 1 and it["inventory_type"] != 16:
            continue  # cloth body armour
        _eff, ok = item_spells(it, spells)
        if not ok:
            continue
        d = describe(it, icons, spells)
        d["effects_supported"] = True
        d["dropped_in"] = sorted({map_name.get(m, str(m)) for c, _, _ in srcs for m in spawns[c]})
        d["dropped_by"] = sorted({creatures[c]["name"] for c, _, _ in srcs})[:8]
        d["world_drop"] = any(s[1] for s in srcs)
        extra.append(d)
    extra.sort(key=lambda d: (d["required_level"], d["entry"]))
    boss_pool = [d["entry"] for d in wc_list if d["proposed_for_boss"]] +         [d["entry"] for d in extra if 18 <= d["required_level"] <= 22]

    loot_out = {
        "meta": meta(args, "World-drop gear pools by level and quality. Items are keyed by "
                           "entry; pools list entries. See docs/reference/items-and-vendors.md."),
        "quality_names": QUALITY,
        "bands": [f"{a}-{b}" for a, b in bands],
        "pool_level_rule": "pool_level = item_level for gear and bags; for grey vendor trash "
                           "(item level 1 in the DB) the median level of the open-world mobs "
                           "that drop it. A mob of level L should roll pools L-2..L+2.",
        "counts_by_band": counts,
        "pools_by_band": {b: {str(k): v for k, v in sorted(q.items())} for b, q in by_band.items()},
        "pools_by_level": {str(l): {str(k): v for k, v in sorted(q.items())}
                           for l, q in sorted(by_ilvl.items())},
        "items": {str(k): v for k, v in sorted(pool_items.items())},
        "random_enchant": {
            "note": "World-drop items whose stats are rolled at drop time (\"of the Bear\" etc.). "
                    "Not in the fixed pools above. To use one, roll suffix_tables[random_property] by "
                    "chance_pct, "
                    "send the property id in ITEM_FIELD_RANDOM_PROPERTIES_ID (the client renders "
                    "the suffix and stat lines from ItemRandomProperties.dbc) and apply mods "
                    "server-side. Without that the item would show and act stat-less.",
            "counts_by_band": rnd_counts,
            "pools_by_band": {b: {str(k): v for k, v in sorted(q.items())}
                              for b, q in rnd_band.items()},
            "items": {str(k): v for k, v in sorted(random_items.items())},
            "suffix_tables": {str(rp): random_suffixes.get(rp, []) for rp in
                              sorted({d["random_property"] for d in random_items.values()})},
        },
        "dungeon_blues": {
            "note": "wailing_caverns: every blue any Wailing Caverns creature can drop in 1.12. "
                    "wailing_caverns_only marks the instance's own named drops; the rest are "
                    "world-drop BoE blues WC trash shares with the open world. "
                    "level_17_25_warrior_blues: other blue drops (dungeon or world) with "
                    "required level 17-25, fixed stats and no unsupported spell, usable by a "
                    "Warrior. proposed_boss_pool: the WC blues a Warrior can sensibly use "
                    "(fixed stats, no unsupported spell, no caster-only stats, no cloth body "
                    "armour) plus the required-level 18-22 extras.",
            "proposed_boss_pool": boss_pool,
            "wailing_caverns": wc_list,
            "level_17_25_warrior_blues": extra,
        },
        "excluded_counts": {k: len(v) for k, v in sorted(excluded.items())},
        # the big buckets (no open-world drop, never obtainable, high-level-only) are counts only
        "excluded": {k: v for k, v in sorted(excluded.items())
                     if k not in ("random_enchant", "no_open_world_drop", "not_obtainable",
                                  "drops_only_above_level_32")},
    }

    token_out = token_vendor(pool_items, items, sources, creatures, icons, spells, forbidden, args)
    goods_out = goods_vendor(con, items, spells, creatures, icons, args)
    prof_out = proficiency(con, args)
    return loot_out, token_out, goods_out, prof_out


def meta(args, desc):
    return {"description": desc, "source": "vmangos world DB (github.com/vmangos/core release "
            "db_latest, sqlite) + 1.12.1 client DBCs", "db": os.path.basename(args.db),
            "patch": PATCH, "client_build": BUILD,
            "generator": "tools/twilights-eve/import_items.py"}


# ---------------------------------------------------------------- Token vendor
SLOT_GROUP = {1: "head", 3: "shoulder", 5: "chest", 20: "chest", 6: "waist", 7: "legs", 8: "feet",
              9: "wrist", 10: "hands", 2: "neck", 11: "finger", 16: "back", 14: "shield",
              13: "one_hand", 21: "one_hand", 17: "two_hand", 15: "ranged", 26: "ranged",
              25: "ranged"}
SLOT_TOKEN_MOD = {"two_hand": 1.0, "chest": 0.9, "legs": 0.9, "head": 0.9, "one_hand": 0.8,
                  "shield": 0.7, "shoulder": 0.7, "hands": 0.7, "feet": 0.7, "waist": 0.6,
                  "wrist": 0.5, "back": 0.5, "neck": 0.6, "finger": 0.6, "ranged": 0.5}
WARRIOR_STAT_W = {"strength": 1.0, "stamina": 0.8, "agility": 0.7, "spirit": 0.1,
                  "intellect": 0.0, "health": 0.1, "mana": 0.0}


def score(d):
    s = sum(WARRIOR_STAT_W.get(x["stat"], 0) * x["value"] for x in d["stats"])
    s += d["armor"] / 20 + d["dps"] / 2
    for e in d["effects"]:
        for m in e.get("mods", ()):
            if m.get("unsupported") or m["stat"] == "ranged_attack_power":
                continue
            if m["stat"] == "attack_power":
                s += m["value"] / 2
            elif m["stat"] in WARRIOR_STAT_W:
                s += WARRIOR_STAT_W[m["stat"]] * m["value"]
            elif m["stat"] == "skill":
                s += m["value"] * 0.5
            else:
                s += m["value"] * 2
    return s


def token_vendor(pool_items, items, sources, creatures, icons, spells, forbidden, args):
    bands = [("1-10", 1, 10), ("11-20", 11, 20), ("21-30", 21, 30)]
    out = {"meta": meta(args, "Proposed Token-vendor stock: Warrior-usable fixed-stat greens by "
                              "required-level band, best one or two per slot by a Warrior stat "
                              "score, with a proposed Token cost."),
           "token_cost_formula": "max(1, round((item_level / 4) * slot_mod))",
           "slot_mod": SLOT_TOKEN_MOD, "bands": {}}
    for name, lo, hi in bands:
        cands = defaultdict(list)
        for d in pool_items.values():
            if d["quality"] != 2 or d["kind"] not in ("weapon", "armor"):
                continue
            w = d["warrior"]
            if not w["usable"] or w["from_level"] > max(d["required_level"], 1):
                continue
            if d.get("caster_stats_only") or d["subclass_name"] in ("cloth", "staff", "dagger") \
                    and d["class"] in (2, 4) and not d["stats"]:
                continue
            req = max(d["required_level"], 1)
            if not lo <= req <= hi:
                continue
            slot = SLOT_GROUP.get(d["inventory_type"])
            if not slot:
                continue
            if d["class"] == 4 and d["subclass_name"] == "cloth" and slot not in ("back",):
                continue  # cloth body armour: legal, but never a sensible Warrior buy
            cands[slot].append(d)
        stock = []
        for slot, ds in sorted(cands.items()):
            ds = [d for d in ds if score(d) > 0]
            if not ds:
                continue
            # the best item near the top of the band, and the best around its middle
            best = lambda pool: max(pool, key=lambda d: (score(d), d["required_level"], -d["entry"]))
            top_pool = [d for d in ds if d["required_level"] >= hi - 3] or ds
            top = best(top_pool)
            picks = [top]
            mid = (lo + hi) // 2
            mids = [d for d in ds if mid - 3 <= max(d["required_level"], 1) <= mid + 1
                    and d is not top]
            if mids:
                picks.append(best(mids))
            for d in picks:
                cost = max(1, round(d["item_level"] / 4 * SLOT_TOKEN_MOD[slot]))
                stock.append({"entry": d["entry"], "name": d["name"], "slot": slot,
                              "required_level": d["required_level"],
                              "item_level": d["item_level"], "token_cost": cost,
                              "warrior_score": round(score(d), 1),
                              "item": d})
        stock.sort(key=lambda s: (s["required_level"], s["slot"], s["entry"]))
        out["bands"][name] = stock
    return out


# ---------------------------------------------------------------- goods vendor
CROSSROADS_VENDORS = (3934, 3480, 3481, 3489, 5871)  # innkeeper, baker, general, butcher, drinks


def vendor_items(con, creature_entry, creatures):
    ct = creatures.get(creature_entry)
    out = [r[0] for r in con.execute("SELECT item FROM npc_vendor WHERE entry = ?",
                                     (creature_entry,))]
    if ct and ct["vendor_id"]:
        out += [r[0] for r in con.execute("SELECT item FROM npc_vendor_template WHERE entry = ?",
                                          (ct["vendor_id"],))]
    return out


def goods_vendor(con, items, spells, creatures, icons, args):
    durations = {r[0]: r[1] for r in parse_dbc(read_dbc_file(args.data, "SpellDuration.dbc")[0],
                                               "uiii")}
    crossroads = {}
    for v in CROSSROADS_VENDORS:
        for i in vendor_items(con, v, creatures):
            crossroads.setdefault(i, []).append(creatures[v]["name"])
    # vendors with at least one permanent spawn (not only a game-event spawn) in 1.12
    permanent = {r[0] for r in con.execute(
        "SELECT DISTINCT c.id FROM creature c WHERE c.patch_min <= ? AND c.patch_max >= ? "
        "AND c.guid NOT IN (SELECT guid FROM game_event_creature)", (PATCH, PATCH))}
    sold = set()
    for cid in permanent:
        ct = creatures.get(cid)
        if ct and ct["npc_flags"] & 4:  # UNIT_NPC_FLAG_VENDOR
            sold.update(vendor_items(con, cid, creatures))
    food = []
    for entry in sorted(sold):
        it = items.get(entry)
        if not it or it["class"] != 0 or it["required_level"] > 30 or it["quality"] > 1:
            continue
        sp = spells.get(it["spellid_1"])
        if not sp:
            continue
        kinds = []
        for i in (1, 2, 3):
            aura = sp[f"effectApplyAuraName{i}"]
            if sp[f"effect{i}"] == 6 and aura in (84, 85, 20, 21):
                kinds.append((i, aura))
        if not kinds or sp["name"] not in ("Food", "Drink", "Refreshment"):
            continue
        dur = durations.get(sp["durationIndex"], 0)
        restores = {}
        for i, aura in kinds:
            amt = spell_amount(sp, i)
            per = sp[f"effectAmplitude{i}"] or 5000
            # MOD_REGEN / MOD_POWER_REGEN tick their amount every 5 s (vmangos Aura::HandleModRegen)
            total = amt * dur // per
            restores["health" if aura in (84, 20) else "mana"] = total
        food.append({
            "entry": entry, "name": it["name"], "type": "drink" if "mana" in restores
            and "health" not in restores else "food",
            "required_level": it["required_level"], "item_level": it["item_level"],
            "buy_price": it["buy_price"], "buy_count": it["buy_count"],
            "sell_price": it["sell_price"], "stackable": it["stackable"],
            "restores": restores, "duration_s": dur / 1000, "spell": sp["entry"],
            "food_type": it["food_type"], "display_id": it["display_id"],
            "icon": icons.get(it["display_id"]),
            "crossroads_vendors": sorted(set(crossroads.get(entry, []))),
        })
    food.sort(key=lambda f: (f["type"], f["required_level"], f["entry"]))
    proposed = [f["entry"] for f in food if f["crossroads_vendors"]]

    dq = {r[0]: round(r[1], 4) for r in parse_dbc(
        read_dbc_file(args.data, "DurabilityQuality.dbc")[0], "uf")}
    dc_blob, dc_src = read_dbc_file(args.data, "DurabilityCosts.dbc")
    dc = {r[0]: list(r[1:]) for r in parse_dbc(dc_blob, "u" * 30)}
    lv = range(1, 61)
    return {
        "meta": meta(args, "Food and drink sold by 1.12 vendors for levels 1-30, the Crossroads "
                           "stock, and the vanilla durability/repair rules."),
        "food_and_drink": food,
        "proposed_crossroads_stock": proposed,
        "selling": {"rule": "A vendor pays item.sell_price * count. Items with sell_price 0 "
                            "cannot be sold. Buyback (12 slots) refunds the same amount.",},
        "repair": {
            "formula": "cost = floor(lost_durability * DurabilityCosts[item_level].multiplier"
                       "[mult_index] * DurabilityQuality[(quality + 1) * 2]); cost = round(cost "
                       "* discount); if cost == 0: cost = 1",
            "mult_index": "weapon: subclass; armor: subclass + 21; anything else: 0",
            "discount": "1.0 (vanilla reputation discount: friendly 0.95, honored 0.9, revered "
                        "0.85, exalted 0.8; not modelled in the slice)",
            "source": "vmangos Player::DurabilityRepair, ItemSubClassToDurabilityMultiplierId",
            "durability_quality": {str(k): v for k, v in dq.items()},
            "quality_mod_by_item_quality": {QUALITY[q]: dq.get((q + 1) * 2) for q in range(5)},
            "durability_costs_dbc_source": dc_src,
            "durability_costs": {str(k): dc[k] for k in lv if k in dc},
        },
        "durability_loss": {
            "death": "every equipped item loses floor(max_durability * 0.10) points (minimum 1)",
            "spirit_healer": "every item, equipped and in bags, loses 25% (not used by the slice)",
            "combat": "each damaging hit dealt or taken has a 0.5% chance to take 1 point from a "
                      "random equipment slot (vmangos DurabilityLossChance.Damage = 0.5); "
                      "parry/block/ranged shots have their own (equally small) chances",
            "broken": "at 0 durability the item's stats are removed until repaired "
                      "(Player::_ApplyItemMods)",
            "source": "vmangos Player::DurabilityLossAll / DurabilityPointsLoss, "
                      "Unit::DealDamage, mangosd.conf",
        },
    }


# ---------------------------------------------------------------- proficiency
def proficiency(con, args):
    start = {}
    for race, spells in con.execute("SELECT race, group_concat(spell) FROM playercreateinfo_spell "
                                    "WHERE class = 1 AND build_min <= ? GROUP BY race", (BUILD,)):
        ids = {int(s) for s in spells.split(",")}
        start[RACE[race]] = sorted(WEAPON_SUB[SKILL_SPELL_TO_WEAPON[s]] for s in ids
                                   if s in SKILL_SPELL_TO_WEAPON)
    return {
        "meta": meta(args, "Vanilla 1.12 Warrior item proficiency and the rule proposed for the "
                           "realm."),
        "armor": {ARMOR_SUB[k]: v for k, v in WARRIOR_ARMOR_LEVEL.items()},
        "weapons": {WEAPON_SUB[k]: v for k, v in WARRIOR_WEAPON_LEVEL.items()},
        "never": ["wand", "libram", "idol", "totem"],
        "starting_weapon_skills_by_race": start,
        "vanilla_notes": [
            "Cloth, leather, mail, shields and Block are known at creation.",
            "Plate Mail (spell 750) is taught by Warrior trainers at level 40 (trainer spell 7109).",
            "Weapon Masters teach every other Warrior weapon skill for 10s at any level; "
            "Polearms (trainer spell 15991) requires level 20.",
            "Starting weapon skills differ by race (see starting_weapon_skills_by_race); "
            "a Warrior without a skill cannot equip that weapon type until trained.",
            "Fist weapons were trainable by Warriors (spell 15590 via 15992).",
        ],
        "proposed_realm_rule": (
            "can_equip(item) = item.required_level <= level AND (allowable_class == -1 OR "
            "allowable_class & 1) AND class/subclass level gate below is met. The slice has no "
            "Weapon Master, so every Warrior weapon type is known from level 1 except polearms "
            "(level 20); plate from level 40 (beyond the slice's level 30 cap, so never). "
            "Wands, librams, idols and totems are never usable."),
    }


def dumps(obj, depth=0):
    """JSON with one line per record: containers nested 2+ deep stay on one line when short."""
    flat = json.dumps(obj, ensure_ascii=False, separators=(",", ":"))
    if not isinstance(obj, (dict, list)) or not obj or (depth >= 2 and len(flat) <= 4000):
        return flat
    pad, inner = " " * depth, " " * (depth + 1)
    if isinstance(obj, dict):
        body = ",\n".join(f"{inner}{json.dumps(str(k))}: {dumps(v, depth + 1)}"
                          for k, v in obj.items())
        return "{\n" + body + "\n" + pad + "}"
    body = ",\n".join(inner + dumps(v, depth + 1) for v in obj)
    return "[\n" + body + "\n" + pad + "]"


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    ap.add_argument("--db", default=os.path.join(DEFAULT_CACHE, "sqlite-dump", "mangos.sqlite"))
    ap.add_argument("--cache", default=DEFAULT_CACHE)
    ap.add_argument("--fetch", action="store_true", help="download the DB if --db is missing")
    ap.add_argument("--data", default=DEFAULT_DATA, help="WoW 1.12.1 Data directory (MPQs)")
    ap.add_argument("--out", default=os.path.join(REPO, "data", "twilights-eve"))
    args = ap.parse_args()
    if not os.path.exists(args.db):
        if not args.fetch:
            sys.exit(f"{args.db} not found; pass --db or --fetch")
        args.db = fetch_db(args.cache)
    loot, token, goods, prof = build(args)
    os.makedirs(args.out, exist_ok=True)
    for name, obj in (("loot_items.json", loot), ("token_vendor.json", token),
                      ("goods_vendor.json", goods), ("warrior_proficiency.json", prof)):
        path = os.path.join(args.out, name)
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(dumps(obj) + "\n")
        print(f"wrote {path}")
    print(json.dumps(loot["counts_by_band"], indent=1))
    print(json.dumps(loot["excluded_counts"], indent=1))
    print(json.dumps(loot["random_enchant"]["counts_by_band"]))
    print("boss pool:", len(loot["dungeon_blues"]["proposed_boss_pool"]))
    print("WC blues:", len(loot["dungeon_blues"]["wailing_caverns"]),
          "extra 17-25 blues:", len(loot["dungeon_blues"]["level_17_25_warrior_blues"]))
    print("token vendor:", {k: len(v) for k, v in token["bands"].items()})
    print("food/drink:", len(goods["food_and_drink"]))


if __name__ == "__main__":
    main()
