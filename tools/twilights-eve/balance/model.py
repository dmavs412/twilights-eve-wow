#!/usr/bin/env python3
"""The balance model: one Basic Warrior, Tier 1 Calling or Tier 2 Calling against one creature row.

Every file it reads comes through one DataRoot (`--data-root`, default the repo): the data in
data/twilights-eve/ (server_spells.json, gear.csv, class level tables, callings.csv, master.csv)
and the content in crates/twilight-realm/content/ (mob_class_stats.toml, power_curve.toml,
mobs.toml's difficulty and zones, starting_outfits.toml, creature_spells.toml). The fight mirrors
the realm's rules (rules/stats.rs, melee.rs, spell_power.rs, power.rs, rage.rs, school.rs, and
mobs.rs Difficulty::muls for the creature). A tuning aid, not the realm: docs/balance/rules.md
says what it is judged against, and run.py drives it.
"""
from __future__ import annotations

import csv
import json
import math
import os
import tomllib
from collections import Counter, defaultdict

REPO = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))

DT = 0.05
FIGHT_CAP_S = 240.0    # a fight still going after this is lost
MOB_RUN = 7.0 * 1.143
PLAYER_RUN = 7.0
DROPLET_PICKUP = 0.5   # share of blood droplets the Cutthroat steps on
AMMO_DPS = 1.5         # Rough Arrow
DEFAULT_GCD_MS = 1500  # the client's global cooldown for an empty client_gcd_ms

CLASS_ID = {"warrior": 1, "hunter": 3, "rogue": 4, "priest": 5, "mage": 8}
CALLINGS = ("cutthroat", "ranger", "initiate", "vanguard", "acolyte")
# The ten Tier 2 Callings docs/balance/rules.md judges at Power Levels 30 to 50, whether or not the
# data or a sim has them yet.
TIER2_CALLINGS = ("bloodletter", "nightblade", "pyromancer", "leyweaver", "sharpshooter", "trapper",
                  "ironclad", "zealot", "blightcaller", "bloodmender")


class DataRoot:
    """Every file the model reads, under one root that holds data/twilights-eve/ and
    crates/twilight-realm/content/: the repo, or a scratch copy a proposal edits (#489)."""

    def __init__(self, root=REPO):
        self.root = os.path.abspath(root)
        self.data = os.path.join(self.root, "data", "twilights-eve")
        self.content = os.path.join(self.root, "crates", "twilight-realm", "content")
        self.spells = {c["key"]: c for c in self.json("server_spells.json")}
        # The global cooldown the client enforces before it sends a cast: a row's client_gcd_ms,
        # which the patch carries as gcd_ms only when the cell is set (docs/spell_pipeline.md).
        self.client_gcd = {r["spell_id"]: r["fields"].get("gcd_ms") for r in self.json("client_dbc_patch.json")["spells"]}
        self.tables = {k: self.json(f"{k}_level_stats.json") for k in CLASS_ID}
        self.gear = {int(r["entry"]): r for r in self.csv("gear.csv")}
        self.callings_csv = {r["calling"]: r for r in self.csv("callings.csv")}
        self.outfits = self.toml("starting_outfits.toml")
        mobs = self.toml("mobs.toml")
        self.difficulty = mobs["difficulty"]
        self.zones = {z["area"]: (z["health"], z["damage"]) for z in mobs["zone"]}
        self.area_parents = {a["id"]: a["zone"] for a in self.toml("area_parents.toml")["area"]}
        self.curve = self.toml("power_curve.toml")
        self.mob_class = self.toml("mob_class_stats.toml")
        self.creature_spells = {s["key"]: s for s in self.toml("creature_spells.toml")["spell"]}

    def json(self, name):
        with open(os.path.join(self.data, name), encoding="utf-8-sig") as f:
            return json.load(f)

    def csv(self, name):
        with open(os.path.join(self.data, name), encoding="utf-8-sig", newline="") as f:
            return list(csv.DictReader(f))

    def toml(self, name):
        with open(os.path.join(self.content, name), "rb") as f:
            return tomllib.load(f)


DATA = None


def use(root=REPO):
    """Reads every file from `root`; the model's tables below are its."""
    global DATA, SPELLS, TABLES, GEAR, OUTFITS, CALLINGS_CSV
    DATA = DataRoot(root)
    SPELLS, TABLES, GEAR = DATA.spells, DATA.tables, DATA.gear
    OUTFITS, CALLINGS_CSV = DATA.outfits, DATA.callings_csv
    return DATA


use()

def fnum(s, default=0.0):
    try:
        return float(s)
    except (TypeError, ValueError):
        return default


def linear(l, level, ap=0):
    if l is None:
        return 0
    v = l.get("base", 0) + l.get("per_level", 0) * level + l.get("per_ap", 0) * ap
    return max(int(math.floor(v + 0.5)) if v >= 0 else 0, l.get("min", 0))


def armor_dr(armor, attacker_level):
    return min(max(armor / (armor + 85.0 * attacker_level + 400.0), 0.0), 0.75)


def armored(dmg, armor, attacker_level):
    if dmg <= 0:
        return 0
    return max(int(dmg - dmg * armor_dr(armor, attacker_level)), 1)


def rage_conv(level):
    return 0.0091107836 * level * level + 3.225598133 * level + 4.2652911


def sp_direct(sp, cast_ms):
    return sp * min(max(cast_ms, 1500), 3500) // 3500


def sp_periodic(sp, duration_ms, ticks):
    return 0 if ticks == 0 else sp * min(duration_ms, 15000) // 15000 // ticks


# ---------------------------------------------------------------- gear
ARMOR_ALLOWED = {
    "vanguard": {"Cloth", "Leather", "Mail", "Plate"},
    "cutthroat": {"Cloth", "Leather"},
    "initiate": {"Cloth", "Robe"},
    "acolyte": {"Cloth", "Robe"},
    "ranger": {"Cloth", "Leather", "Mail"},
    "basic_warrior": {"Cloth", "Leather", "Mail"},
}
WEIGHTS = {
    "vanguard": dict(str=1.0, sta=0.5, agi=0.3, armor=0.004, block=0.6),
    "cutthroat": dict(agi=1.0, str=0.5, sta=0.3, armor=0.002),
    "initiate": dict(int=1.0, spi=0.3, sta=0.3, spell_power=1.5, armor=0.001),
    "acolyte": dict(spi=0.9, int=0.9, sta=0.3, spell_power=1.5, armor=0.001),
    "ranger": dict(agi=1.0, sta=0.3, int=0.2, armor=0.002),
    "basic_warrior": dict(str=1.0, sta=0.5, agi=0.3, armor=0.004, block=0.6),
}
CASTERS = {"initiate", "acolyte"}
# A Tier 2 Calling wears what the Tier 1 Calling of its stat profile wears, and weighs stats as it does.
GEAR_LIKE = {"warrior": "vanguard", "rogue": "cutthroat", "mage": "initiate", "priest": "acolyte", "hunter": "ranger"}


def gear_like(calling):
    """The Calling whose armour, weapons and stat weights `calling` takes: itself when it has its
    own, else the Tier 1 Calling of its stat profile."""
    return calling if calling in WEIGHTS else GEAR_LIKE[SPELLS[calling]["stat_profile"]]


def wdps(r):
    sp = fnum(r["speed"])
    return (fnum(r["dmg_min"]) + fnum(r["dmg_max"])) / 2 / sp if sp else 0


def score(calling, r):
    calling = gear_like(calling)
    w = WEIGHTS[calling]
    s = sum(fnum(r[k]) * v for k, v in w.items() if k in r)
    # A weapon is chosen by its damage row first; stats only break ties. Melee Callings want a
    # slow weapon (a normalized strike swings harder), the Ranger a hard-hitting bow.
    if r["slot"] == "Weapon" and calling not in CASTERS:
        s = 0.2 * s + wdps(r) * 4.0 + fnum(r["speed"]) * 2.0
    if r["slot"] == "Ranged" and calling == "ranger":
        s = 0.2 * s + wdps(r) * 4.0
    return s


def usable(r, level, qualities):
    return r["quality"] in qualities and fnum(r["required_level"], 1) <= level


def pick(calling, level, qualities, slot, pred=lambda r: True, also=()):
    """The usable rows of `slot` in `qualities`, and the entries `also`, best first."""
    cands = [r for r in GEAR.values()
             if r["slot"] == slot and (usable(r, level, qualities) or int(r["entry"]) in also) and pred(r)]
    if not cands:
        return []
    cands.sort(key=lambda r: -score(calling, r))
    return cands


QUALITY = {"white": "Common", "green": "Uncommon", "blue": "Rare"}


def kit(calling, level, tier):
    """What `calling` wears at `level` in `tier`: white (the starting outfit and the best Common
    row a slot), green (the best Uncommon) or blue (the best Rare, else the green). A row stays
    usable from its required level up, so a level with no row of its own wears an earlier one.
    Returns (entries, off_hand, fell_back, empty): off_hand the off-hand weapon's entry or None,
    fell_back the slots of a blue kit that wear a green because no Rare row is usable there, empty
    the slots the kit wants and wears nothing in because no row is usable there at all."""
    outfit = tuple(e for e in OUTFITS.get(calling, {}).get("items", []) if e in GEAR) if tier == "white" else ()

    def best(slot, pred=lambda r: True):
        cands = pick(calling, level, (QUALITY[tier],), slot, pred, outfit)
        if not cands and tier == "blue":
            cands = pick(calling, level, ("Uncommon",), slot, pred)
        return cands

    like = gear_like(calling)
    chosen = []
    wanted = ["Chest", "Amulet", "Ring", "Ring", "Weapon"]
    chest = best("Chest", lambda r: r["type"] in ARMOR_ALLOWED[like])
    if chest:
        chosen.append(int(chest[0]["entry"]))
    neck = best("Amulet")
    if neck:
        chosen.append(int(neck[0]["entry"]))
    rings = best("Ring")
    chosen += [int(r["entry"]) for r in rings[:2]]
    off = None
    if like == "cutthroat":
        wanted.append("Weapon")
        mh = best("Weapon", lambda r: r["hand"] in ("MH", "1H") and fnum(r["speed"]) >= 2.0)
        oh = best("Weapon", lambda r: r["hand"] == "1H" and fnum(r["speed"]) <= 1.9)
        if mh:
            chosen.append(int(mh[0]["entry"]))
        if oh:
            off = int(oh[0]["entry"])
    elif like in ("vanguard", "basic_warrior"):
        wanted.append("Offhand")
        mh = best("Weapon", lambda r: r["hand"] in ("MH", "1H"))
        sh = best("Offhand", lambda r: r["type"] == "Shield")
        if mh:
            chosen.append(int(mh[0]["entry"]))
        if sh:
            chosen.append(int(sh[0]["entry"]))
    elif like == "ranger":
        wanted.append("Ranged")
        mh = best("Weapon", lambda r: r["hand"] in ("MH", "1H", "2H"))
        bow = best("Ranged", lambda r: r["type"] in ("Bow", "Gun", "Crossbow"))
        if mh:
            chosen.append(int(mh[0]["entry"]))
        if bow:
            chosen.append(int(bow[0]["entry"]))
    else:
        wanted.append("Ranged")
        staff = best("Weapon", lambda r: r["hand"] == "2H")
        mh = best("Weapon", lambda r: r["hand"] in ("MH", "1H"))
        held = best("Offhand", lambda r: r["type"] == "Held")
        two = score(calling, staff[0]) if staff else -1
        one = (score(calling, mh[0]) if mh else -1) + (score(calling, held[0]) if held else 0)
        if staff and two >= one:
            chosen.append(int(staff[0]["entry"]))
        else:
            if mh:
                chosen.append(int(mh[0]["entry"]))
            if held:
                chosen.append(int(held[0]["entry"]))
        wand = best("Ranged", lambda r: r["type"] == "Wand")
        if wand:
            chosen.append(int(wand[0]["entry"]))
    worn = chosen + ([off] if off else [])
    fell_back = [GEAR[e]["slot"] for e in worn if tier == "blue" and GEAR[e]["quality"] != "Rare"]
    have = Counter(GEAR[e]["slot"] for e in worn)
    empty = []
    for slot in wanted:
        if have[slot]:
            have[slot] -= 1
        else:
            empty.append(slot)
    return chosen, off, fell_back, empty



# ---------------------------------------------------------------- stats
class Stats:
    pass


def build_stats(calling, level, entries, off_entry, rows):
    """`calling`'s stats at `level` in the gear `entries`, in the build `rows`: callings.csv's
    health_pct and mana_pct keep that percent of the base health and mana, rounded down, as
    rules/stats.rs `scale_base_pools` does, and a passive's max_health_pct raises the maximum
    health."""
    c = SPELLS[calling]
    pools = CALLINGS_CSV[calling]
    prof = c["stat_profile"]
    cls = CLASS_ID[prof]
    t = TABLES[prof]
    race = next(r for r in t["race_base_stats"]["human"] if r["level"] == level)
    per = next(r for r in t["per_level"] if r["level"] == level)
    tb = c.get("tier_bonus") or {}
    base = [race["str"] + tb.get("strength", 0), race["agi"] + tb.get("agility", 0),
            race["sta"] + tb.get("stamina", 0), race["int"] + tb.get("intellect", 0),
            race["spi"] + tb.get("spirit", 0)]
    s = Stats()
    s.calling = calling
    s.cls = prof
    s.level = level
    gear = dict(str=0, agi=0, sta=0, int=0, spi=0, armor=0, spell_power=0, block=None)
    s.mh = None
    s.oh = None
    s.ranged = None
    for e in entries + ([off_entry] if off_entry else []):
        r = GEAR[e]
        for k in ("str", "agi", "sta", "int", "spi", "armor", "spell_power"):
            gear[k] += int(fnum(r[k]))
        if r["slot"] == "Offhand" and r["type"] == "Shield":
            gear["block"] = int(fnum(r["block"]))
        if r["slot"] == "Weapon":
            w = (fnum(r["dmg_min"]), fnum(r["dmg_max"]), fnum(r["speed"]))
            if e == off_entry:
                s.oh = w
            else:
                s.mh = w
        if r["slot"] == "Ranged" and r["type"] in ("Bow", "Gun", "Crossbow"):
            s.ranged = (fnum(r["dmg_min"]), fnum(r["dmg_max"]), fnum(r["speed"]))
    names = ["str", "agi", "sta", "int", "spi"]
    s.str, s.agi, s.sta, s.int, s.spi = (base[i] + gear[n] for i, n in enumerate(names))
    s.base_health = per["base_health"] * int(pools["health_pct"] or 100) // 100
    s.max_hp = s.base_health + min(s.sta, 20) + max(s.sta - 20, 0) * 10
    s.max_hp = s.max_hp * (100 + sum(p["pct"] for p in passive_effects(calling, rows, "max_health_pct"))) // 100
    # A cost in percent of base mana reads the unscaled base, as cast.rs does.
    bm = per["base_mana"]
    s.base_mana = bm
    s.max_mana = 0 if bm == 0 else bm * int(pools["mana_pct"] or 100) // 100 + min(s.int, 20) + max(s.int - 20, 0) * 15
    if prof in ("mage", "priest"):
        s.mana_tick = s.spi / 4.0 + 12.5
    elif prof == "hunter":
        s.mana_tick = s.spi / 5.0 + 15.0
    else:
        s.mana_tick = 0.0
    if prof == "warrior":
        s.ap = 3 * level + 2 * s.str - 20
    elif prof in ("rogue", "hunter"):
        s.ap = 2 * level + s.str + s.agi - 20
    else:
        s.ap = s.str - 10
    if prof == "hunter":
        s.rap = 2 * level + 2 * s.agi - 10
    elif prof in ("warrior", "rogue"):
        s.rap = level + s.agi - 10
    else:
        s.rap = s.agi - 10
    s.crit = s.agi / per["agi_per_crit_pct"]
    s.dodge = s.agi / per["agi_per_dodge_pct"]
    s.parry = 0.0
    s.spell_power = gear["spell_power"]
    s.armor_items = gear["armor"]
    s.armor = 2 * s.agi + gear["armor"]
    s.defense = 5 * level
    s.block_pct = 0.0
    s.block_value = 0
    s.shield = gear["block"]
    if gear["block"] is not None:
        s.block_pct = 5.0
        s.block_value = max(int(gear["block"] + s.str / 20.0 - 1), 0)
    s.max_energy = 100
    s.energy_tick = int(CALLINGS_CSV[calling]["energy_per_tick"] or 20) if c["power"] == "energy" else 0
    s.power = c["power"]
    mh = s.mh or (1.0, 2.0, 2.0)
    s.mh_min = mh[0] + s.ap / 14.0 * mh[2]
    s.mh_max = mh[1] + s.ap / 14.0 * mh[2]
    s.mh_speed = mh[2]
    if s.oh:
        s.oh_min = s.oh[0] + s.ap / 14.0 * s.oh[2]
        s.oh_max = s.oh[1] + s.ap / 14.0 * s.oh[2]
        s.oh_speed = s.oh[2]
    if s.ranged:
        rmin, rmax, rsp = s.ranged
        s.r_min = rmin + s.rap / 14.0 * rsp + AMMO_DPS * rsp
        s.r_max = rmax + s.rap / 14.0 * rsp + AMMO_DPS * rsp
        s.r_speed = rsp
    return s


# ---------------------------------------------------------------- creatures
CURVE_ANCHOR = 10        # rules/stats.rs: the level whose vanilla row the Power Curve multiplies
DAMAGE_VARIANCE = 0.14   # rules/stats.rs
RANK_KEY = {1: "elite", 2: "rare_elite", 3: "boss", 4: "rare"}
ELWYNN_FOREST = 12
NORTHSHIRE_VALLEY = 9
REDRIDGE_MOUNTAINS = 44
DUSKWOOD = 10
# 1.12's CreatureType ids, a template's `creature_type` (callings.rs CreatureType).
CREATURE_TYPES = {1: "beast", 2: "dragonkin", 3: "demon", 4: "elemental", 5: "giant", 6: "undead", 7: "humanoid",
                  8: "critter", 9: "mechanical"}


def round_half_up(v):
    """Rust's f32::round on a positive number."""
    return int(math.floor(v + 0.5))


def curve_at(level):
    """P(level), rules/power_curve.rs: level / 10 to `linear_to`, then `late_growth` a level."""
    c = DATA.curve
    level = max(level, 1)
    if level <= c["linear_to"]:
        return level / 10.0
    return c["linear_to"] / 10.0 * (1.0 + c["late_growth"]) ** (level - c["linear_to"])


def damage_at(level):
    """mobs.rs Difficulty::damage_at: `damage_at_1` faded in to `damage` by `fade_to`."""
    d = DATA.difficulty
    if level >= d["fade_to"]:
        return d["damage"]
    t = (max(level, 1) - 1) / (d["fade_to"] - 1)
    return d["damage_at_1"] + (d["damage"] - d["damage_at_1"]) * t


def zone_factor(area):
    """mobs.rs zone_factor: the area's [[zone]] row, else its parent zone's, else none."""
    z = DATA.zones.get(area)
    if z is None:
        z = DATA.zones.get(DATA.area_parents.get(area))
    return z or (1.0, 1.0)


def on_level_area(level):
    """docs/balance/rules.md: Northshire at 10 and below, Elwynn Forest from 11 to 30, Redridge
    Mountains above 30 (Duskwood's Zone Factor is the same)."""
    if level <= 10:
        return NORTHSHIRE_VALLEY
    return ELWYNN_FOREST if level <= 30 else REDRIDGE_MOUNTAINS


def mob_stats(level, unit_class="warrior", rank=0, area=None, health_mul=1.0, damage_mul=1.0,
              armor_mul=1.0, swing_s=2.0, name=None, creature_type="humanoid"):
    """A creature as Mob::stats_for makes it: its unit class's level-10 vanilla row x P(level) x
    [difficulty] health and damage_at(level) x the Tier Step x its rank's factors x the Zone
    Factor x the template's own multipliers; armour the vanilla row's at its level. `area` None
    is the on-level area. `creature_type` (CREATURE_TYPES) is what a hit's vs_type and a stun's
    only_type read: an on-level mob is humanoid, a named boss its template's."""
    d = DATA.difficulty
    rank_h, rank_d = d["rank"][RANK_KEY[rank]] if rank in RANK_KEY else (1.0, 1.0)
    step = d["tier_step"]
    step_h, step_d = (step["health"], step["damage"]) if level >= step["from"] else (1.0, 1.0)
    zone_h, zone_d = zone_factor(on_level_area(level) if area is None else area)
    p = curve_at(level)
    t = DATA.mob_class[unit_class]
    hm = health_mul * d["health"] * step_h * rank_h * zone_h * p
    dm = damage_mul * damage_at(level) * step_d * rank_d * zone_d * p
    avg = t["melee_damage_avg"][CURVE_ANCHOR - 1] * dm
    return dict(level=level, name=name or f"{unit_class} {level}", hp=round_half_up(t["health"][CURVE_ANCHOR - 1] * hm),
                armor=int(t["armor"][level - 1] * armor_mul), dmin=avg * (1 - DAMAGE_VARIANCE),
                dmax=avg * (1 + DAMAGE_VARIANCE), swing=swing_s, type=creature_type)


# ---------------------------------------------------------------- skills
def talent_rows(calling):
    """`calling`'s Talent Rows (ADR 0028) from server_spells.json, in level order."""
    return SPELLS[calling].get("rows", [])


def choice(calling, row, key):
    """The built spell of choice `key` in `calling`'s Talent Row `row`."""
    r = next(r for r in talent_rows(calling) if r["key"] == row)
    return next(c for c in r["choices"] if c["key"] == key)


def skill(calling, key, rows=None):
    """`calling`'s skill `key`; when the build `rows` ({row key: choice key}) sets the skill's
    row, the set choice's own built spell."""
    if rows and key in rows:
        return choice(calling, key, rows[key])
    return next(s for s in SPELLS[calling]["skills"] if s["key"] == key)


def fx(sk, kind):
    return next(e["params"] for e in sk["effects"] if e["kind"] == kind)


def fxs(sk, kind):
    """The params of every `kind` effect `sk` (a spell or an aura row) carries."""
    return [e["params"] for e in sk["effects"] if e["kind"] == kind]


def aura(calling, key):
    return next(a for a in SPELLS[calling]["auras"] if a["key"] == key)


def key_of(calling, spell_id):
    """The key of `calling`'s plain skill with spell id `spell_id`."""
    return next(s["key"] for s in SPELLS[calling]["skills"] if s["spell_id"] == spell_id)


def passives(calling, rows):
    """The choices the build `rows` sets in `calling`'s talent rows, which replace no skill."""
    return [choice(calling, r["key"], rows[r["key"]]) for r in talent_rows(calling)
            if r["kind"] == "talent" and r["key"] in rows]


def passive_effects(calling, rows, kind):
    """The params of every `kind` effect on the passives the build `rows` sets."""
    return [p for sk in passives(calling, rows) for p in fxs(sk, kind)]


# Tier 1 choices that change nothing in the sim's fights, with the reason the report prints
# (test_every_tier1_choice_changes_the_fight_or_is_ignored).
IGNORED = {
    ("ranger", "guerilla_tactics"): "the sim never moves for 2 sec",
    ("ranger", "running_fire"): "the sim never moves while it shoots",
    ("ranger", "parting_gift"): "root",
    ("ranger", "unbound_leap"): "run speed",
    ("ranger", "pinning_barb"): "root",
    ("ranger", "rhythmic_draw"): "the sim casts no Shaft after a Snapshot",
    ("ranger", "adrenaline_rush"): "the sim casts no Snapshot after a Vault",
    ("ranger", "bloodscent"): "the sim's Ranger never runs short of mana",
    ("initiate", "frost_ward"): "movement slow",
    ("initiate", "splintering_cold"): "movement slow",
    ("initiate", "frostbite"): "root",
    ("initiate", "shattered_blink"): "root",
    ("vanguard", "sweeping_zeal"): "no pull holds more than 3 mobs",
    ("acolyte", "unbroken_siphon"): "no mob interrupts",
    ("acolyte", "shroud_of_rot"): "range change",
}
# The Tier 2 choices, and the skills a row starts from, that change nothing in the sim's fights.
KICK = "the Kick boss.py casts is the plain skill"
RADIUS = "a larger radius: every mob of a pull stands in melee"
IGNORED.update({
    ("ironclad", "long_rebuke"): KICK,
    ("ironclad", "rebuke_of_conviction"): KICK,
    ("ironclad", "shield_rebuke"): KICK,
    ("ironclad", "wider_line"): RADIUS,
    ("ironclad", "hard_line"): "taunt",
    ("zealot", "wide_ground"): RADIUS,
    ("zealot", "righteous_fury"): "threat",
    ("zealot", "blazing_fury"): "threat",
    ("zealot", "long_rebuke"): KICK,
    ("zealot", "rebuke_of_iron"): KICK,
    ("zealot", "judging_rebuke"): KICK,
    ("zealot", "thrown_hammer"): "range change",
    ("blightcaller", "wide_miasma"): RADIUS,
    ("blightcaller", "wide_spread"): "no pull holds more than 3 mobs",
    ("blightcaller", "far_spread"): RADIUS,
    ("blightcaller", "long_hush"): KICK,
    ("blightcaller", "choking_hush"): KICK,
    ("blightcaller", "mending_hush"): KICK,
    ("blightcaller", "wide_communion"): RADIUS,
    ("blightcaller", "strong_return"): "resurrection",
    ("blightcaller", "quick_rites"): "resurrection",
    ("blightcaller", "far_rites"): "resurrection",
    ("bloodmender", "wide_ward"): RADIUS,
    ("bloodmender", "long_hush"): KICK,
    ("bloodmender", "deep_drink"): KICK,
    ("bloodmender", "mana_hush"): KICK,
    ("bloodmender", "wide_circle"): RADIUS,
    ("bloodmender", "full_rites"): "resurrection",
    ("bloodmender", "quick_rites"): "resurrection",
    ("bloodmender", "cheap_rites"): "resurrection",
})


# ---------------------------------------------------------------- sim core
class Sim:
    def __init__(self, calling, level, s, rows, rng, opts, mob=None):
        self.calling, self.L, self.s, self.rows, self.rng, self.opts = calling, level, s, rows, rng, opts
        self.spells = {}   # skill key -> the spell skill() found
        self.m = mob or mob_stats(level)
        self.hp = s.max_hp
        self.power = 0 if s.power == "rage" else self.power_max()
        self.cds = {}
        self.t = 0.0
        self.dmg = defaultdict(float)
        self.healed = 0.0
        self.deaths = 0
        self.mana_pause_until = -1.0
        self.regen_next = 2.0
        self.fight_log = []
        self.healing_taken = passive_effects(calling, rows, "healing_taken_pct")

    # ---- power
    def power_max(self):
        s = self.s
        return {"rage": 100, "energy": s.max_energy, "mana": s.max_mana}[s.power]

    def regen_tick(self, in_combat):
        s = self.s
        if s.power == "energy":
            self.power = min(self.power + s.energy_tick, s.max_energy)
        elif s.power == "mana":
            if self.t >= self.mana_pause_until:
                self.power = min(self.power + s.mana_tick, s.max_mana)
        elif s.power == "rage" and not in_combat:
            self.power = max(self.power - 2, 0)
        if not in_combat:
            self.heal(max(round(s.spi * 0.25), 1), "regen")

    def spend_mana(self, cost):
        self.power -= cost
        self.mana_pause_until = self.t + 5.0

    def pct_base(self, pct):
        return self.s.base_mana * pct // 100

    def skill(self, key):
        """The spell this fighter casts for skill `key`: its build's choice when the row is set."""
        if key not in self.spells:
            self.spells[key] = skill(self.calling, key, self.rows)
        return self.spells[key]

    def granted(self, key):
        """The aura row the spell for skill `key` grants: its choice's own aura when the build sets
        the row."""
        return aura(self.calling, fx(self.skill(key), "grant_aura")["aura"])

    def cost(self, sk):
        """`sk`'s realm_cost in power: a share of base mana for a `pct_base` cost."""
        c = sk.get("realm_cost") or 0
        return self.pct_base(c) if sk.get("cost_kind") == "pct_base" else c

    # ---- buffs: self.buffs is aura key -> [until, aura row, stacks, charges], set by setup_fight
    def grant(self, key):
        """Aura `key` on the character for its duration_ms, with its charges; a `stack` aura still
        up gains a stack, up to its max_stacks."""
        a = aura(self.calling, key)
        held, stacks = self.buffs.get(key), 1
        if held and self.t < held[0] and a.get("stack_rule") == "stack":
            stacks = min(held[2] + 1, a["max_stacks"])
        self.buffs[key] = [self.t + a["duration_ms"] / 1000, a, stacks, a.get("charges", 0)]

    def buffed(self, kind):
        """The params of every `kind` effect on the character's buffs still up, once a stack."""
        return [p for b in self.buffs.values() if self.t < b[0] for p in fxs(b[1], kind) * b[2]]

    def next_casts(self, key):
        return [p for p in self.buffed("next_cast") if p["skill"] == key]

    def use_next_casts(self, key):
        """Skill `key` went off: each buff with a next_cast for it loses a charge, and goes at none
        left; one with no charges lasts its duration."""
        for k, b in list(self.buffs.items()):
            if b[3] and any(p["skill"] == key for p in fxs(b[1], "next_cast")):
                b[3] -= 1
                if not b[3]:
                    del self.buffs[k]

    def price(self, key, mods=None):
        """Skill `key`'s cost after the next_cast changes `mods` (the buffs' now when None)."""
        mods = self.next_casts(key) if mods is None else mods
        return self.cost(self.skill(key)) * (100 + max(sum(p.get("cost_pct", 0) for p in mods), -100)) // 100

    def skill_pct(self, kind, key, part=None):
        """The pct the build's passives' `kind` effects (skill_crit_pct, skill_amount_pct) give skill
        `key`: those for every part of it, and with `part` those for that part too (`applies_to`)."""
        return sum(p["pct"] for p in passive_effects(self.calling, self.rows, kind)
                   if p["skill"] == key and p.get("applies_to") in (None, part))

    def cooldown_ms(self, key):
        """Skill `key`'s cooldown, less the flat COOLDOWN spellmods the build's passives put on it
        (Harvest Moon)."""
        mod = sum(m["value"] for p in passives(self.calling, self.rows) for m in p.get("spellmods", [])
                  if m["op"] == "COOLDOWN" and m["type"] == "flat" and key in m["target"])
        return max((self.skill(key).get("realm_cooldown_ms") or 0) + mod, 0)

    def gcd_of(self, key):
        """The global cooldown skill `key` starts, in seconds: the client's client_gcd_ms, which it
        enforces before it sends the cast (an empty cell is the from-scratch 1.5 s, 0 is none), or
        the realm's realm_gcd_ms when that is longer (docs/spell_pipeline.md)."""
        sk = self.skill(key)
        client = DATA.client_gcd.get(sk["spell_id"])
        return max(DEFAULT_GCD_MS if client is None else client, sk.get("realm_gcd_ms") or 0) / 1000

    def heal(self, amount, src):
        """`amount` healed up to the maximum health, more while below a passive's health line (Will
        to Survive). Returns what it would heal with no maximum."""
        amount = int(amount)
        for p in self.healing_taken:
            if not p.get("below_health_pct") or self.hp * 100 < self.s.max_hp * p["below_health_pct"]:
                amount = amount * (100 + p["pct"]) // 100
        before = self.hp
        self.hp = min(self.hp + amount, self.s.max_hp)
        self.healed += self.hp - before
        return amount

    # ---- rolls
    def yellow(self, crit_pct, unavoidable=False):
        r = self.rng.random() * 100
        if r < 5:
            return "miss"
        if not unavoidable:
            if r < 10:
                return "dodge"
            if r < 15:
                return "parry"
        return "crit" if self.rng.random() * 100 < crit_pct else "hit"

    def ranged_roll(self, crit_pct):
        r = self.rng.random() * 100
        if r < 5:
            return "miss"
        if r < 10:
            return "dodge"
        return "crit" if self.rng.random() * 100 < crit_pct else "hit"

    def spell_roll(self, crit_pct):
        if self.rng.random() * 100 < 5:
            return "miss"
        return "crit" if self.rng.random() * 100 < crit_pct else "hit"

    def hit_mob(self, raw, src, physical=True, crit=False, crit_bonus_pct=0, armor_ignore=0, dmg_pct=0):
        raw = int(raw * (100 + dmg_pct) / 100)
        if physical:
            d = armored(raw, max(self.m_armor - armor_ignore - self.armor_loss(), 0), self.L)
        else:
            d = raw
        if crit:
            d = d * (200 if physical else 150) * (100 + crit_bonus_pct) // 10000
        self.mob_hp -= d
        self.dmg[src] += d
        return d

    def white(self, hand="mh"):
        s = self.s
        if hand == "mh":
            raw = self.rng.uniform(s.mh_min, s.mh_max)
        else:
            raw = self.rng.uniform(s.oh_min, s.oh_max) * 0.5
        raw *= (100 + self.dmg_pct()) / 100
        r = self.rng.random() * 100
        miss = 5 + (19 if s.oh else 0)
        if r < miss:
            return 0, "miss"
        if r < miss + 5:
            return 0, "dodge"
        if r < miss + 10:
            return 0, "parry"
        crit = self.rng.random() * 100 < s.crit
        return self.hit_mob(raw, "white_" + hand, True, crit), ("crit" if crit else "hit")

    def dmg_pct(self):
        return 0

    def armor_loss(self):
        """The armour the target has lost to the character's auras (Jagged Barbs' sunder)."""
        return 0

    # ---- mob swing
    def mob_swing(self, from_front=True, i=None):
        """A swing of mob `i` of a pull (the target when None) at the character."""
        s = self.s
        raw = self.rng.uniform(self.m["dmin"], self.m["dmax"])
        weaken = self.weaken_pct(i)
        if weaken:
            raw = raw * (100 - weaken) / 100
        arm = armored(raw, self.player_armor(), self.m["level"])
        r = self.rng.random() * 100
        miss = 5 + self.mob_miss_more()
        if r < miss:
            return 0, "miss"
        r -= miss
        if r < s.dodge + self.dodge_more():
            return 0, "dodge"
        r -= s.dodge + self.dodge_more()
        if r < s.parry:
            return 0, "parry"
        r -= s.parry
        if r < self.block_pct():
            blocked = min(self.block_value(), arm)
            return arm - blocked, "block"
        r -= self.block_pct()
        if r < 5:
            return arm * 2, "crit"
        return arm, "hit"

    def player_armor(self):
        return self.s.armor

    def weaken_pct(self, i=None):
        """The percent less mob `i` (the target when None) hits for (Smoldering Ember)."""
        return 0

    def mob_miss_more(self):
        return 0

    def dodge_more(self):
        return 0

    def block_pct(self):
        return self.s.block_pct

    def block_value(self):
        return self.s.block_value

    def stun(self, p, combo=0):
        """The stun effect `p` on the target: its next swing waits for the stun to end, after its
        aura's duration, or `duration_ms` and `per_combo` for each of the `combo` points spent."""
        if p.get("apply_aura"):
            ms = aura(self.calling, p["apply_aura"])["duration_ms"]
        else:
            ms = p["duration_ms"] + linear(p.get("per_combo"), self.L) * combo
        self.mob_swing_t = max(self.mob_swing_t, self.t + ms / 1000)

    def splash(self, d, src, n=1):
        """`d` damage, past armour already, to the `n` nearest other living mobs (0 every one):
        none in a fight with one (multi.Multi has the others)."""

    def take(self, dmg):
        ab = min(getattr(self, "absorb", 0), dmg)
        if ab:
            self.absorb -= ab
            dmg -= ab
        self.hp -= dmg
        self.taken += dmg
        return dmg

    # ---- framework
    def ready(self, key):
        return self.cds.get(key, -1) <= self.t

    def cd(self, key, ms):
        self.cds[key] = self.t + ms / 1000

    def fight(self):
        s = self.s
        self.mob_hp = self.m["hp"]
        self.m_armor = self.m["armor"]
        self.taken = 0
        start_hp, start_power = self.hp, self.power
        t0 = self.t
        self.dmg_fight = defaultdict(float)
        self.setup_fight()
        while self.mob_hp > 0 and self.hp > 0 and self.t - t0 < FIGHT_CAP_S:
            if self.t >= self.regen_next:
                self.regen_tick(True)
                self.regen_next += 2.0
            self.step()
            self.t += DT
        dur = self.t - t0
        died = self.hp <= 0
        if not died:
            self.on_kill()
        return dict(time=dur, died=died, hp_lost=(start_hp - max(self.hp, 0)) / s.max_hp,
                    taken=self.taken / s.max_hp, power_end=self.power, power_start=start_power)

    def on_kill(self):
        pass


# ---------------------------------------------------------------- Vanguard
class Vanguard(Sim):
    def setup_fight(self):
        s = self.s
        self.dist = 0.0
        self.mob_swing_t = 0.5
        self.swing_t = 0.0
        self.gcd = 0.0
        self.conviction = 0
        self.conv_until = 0
        self.seal_until = -1
        self.armor_pct = 0
        self.seal_armor_until = -1
        self.sop_until = -1
        self.ground_until = -1
        self.ground_next = 0
        self.smash_queued = False
        self.power = min(self.power, 100)
        # The auras the build's Seal of Iron and Shield of the Penitent grant, Conviction, and the
        # passive's procs.
        self.seal_aura = self.granted("seal_of_iron")
        self.sop_aura = self.granted("shield_of_penitent")
        self.sop_block = sum(p["pct"] for p in fxs(self.sop_aura, "block_pct"))
        self.conv_aura = aura(self.calling, fx(self.skill("righteous_smash"), "add_aura_stacks")["aura"])
        self.passive_procs = passive_effects(self.calling, self.rows, "proc")
        # Bulwark Charge opener: its rage, the ground effect under the mob, and the strike or the
        # stun its choice adds on arrival.
        if self.ready("charge"):
            ch = self.skill("bulwark_charge")
            self.cd("charge", ch["realm_cooldown_ms"])
            self.gain_rage(linear(fx(ch, "energize")["amount"], self.L))
            g = fx(ch, "ground_effect")
            self.ground_dmg = linear(g["amount"], self.L)
            self.ground_until = self.t + g["duration_ms"] / 1000
            self.ground_next = self.t + 1.0
            for p in fxs(ch, "weapon_strike"):
                o = self.spell_roll(s.crit) if ch["defense_type"] == "magic" else self.yellow(s.crit)
                if o in ("hit", "crit"):
                    raw = self.rng.uniform(s.mh_min, s.mh_max) * p.get("weapon_pct", 100) / 100 + linear(p["bonus"], self.L)
                    self.hit_mob(raw, "charge_strike", p.get("school", "physical") == "physical", o == "crit")
            for p in fxs(ch, "stun"):
                self.stun(p)

    def player_armor(self):
        pct = self.armor_pct if self.t < self.seal_armor_until else 0
        return 2 * self.s.agi + self.s.armor_items * (100 + pct) // 100

    def block_pct(self):
        return self.s.block_pct + (self.sop_block if self.t < self.sop_until else 0)

    def gain_rage(self, n):
        self.power = min(self.power + n, 100)

    def rage_dealt(self, d):
        self.gain_rage(d / rage_conv(self.L) * 7.5 * 2.0)

    def rage_taken(self, d):
        self.gain_rage(d / rage_conv(self.L) * 2.5 * 2.0)

    def add_conviction(self, n):
        self.conviction = min(self.conviction + n, self.conv_aura["max_stacks"])
        self.conv_until = self.t + self.conv_aura["duration_ms"] / 1000

    def fire(self, on):
        """The procs on `on` (melee_hit, block, parry): the Seal's and the Shield's while their aura
        is up, then the passive's."""
        for a, until in ((self.seal_aura, self.seal_until), (self.sop_aura, self.sop_until)):
            if self.t < until:
                for p in fxs(a, "proc"):
                    if p["on"] == on:
                        self.proc(p, a["key"])
        for p in self.passive_procs:
            if p["on"] == on:
                self.proc(p, "passive")

    def proc(self, p, src):
        """The proc `p` from `src`, when its chance comes up: a share of the maximum health, rage,
        a Conviction stack or an armour spike, Holy damage to the attacker, and a share of the
        average weapon swing to the target."""
        if p.get("chance", 100) < 100 and self.rng.random() * 100 >= p["chance"]:
            return
        s = self.s
        if p.get("heal_pct"):
            self.heal(s.max_hp * p["heal_pct"] // 100, src)
        if p.get("energize"):
            self.gain_rage(p["energize"])
        if p.get("apply_aura") == self.conv_aura["key"]:
            self.add_conviction(1)
        elif p.get("apply_aura"):
            a = aura(self.calling, p["apply_aura"])
            self.armor_pct = sum(e["pct"] for e in fxs(a, "armor_items_pct"))
            self.seal_armor_until = self.t + a["duration_ms"] / 1000
        physical = p.get("school", "physical") == "physical"
        if p.get("damage_attacker"):
            self.hit_mob(linear(p["damage_attacker"], self.L), src, physical=physical)
        if p.get("weapon_pct"):
            self.hit_mob((s.mh_min + s.mh_max) / 2 * p["weapon_pct"] / 100, src, physical=physical)

    def step(self):
        s, L = self.s, self.L
        if self.conviction and self.t > self.conv_until:
            self.conviction = 0
        # Ground effect ticks.
        if self.t < self.ground_until and self.t >= self.ground_next:
            self.ground_next += 1.0
            self.hit_mob(self.ground_dmg, "consecration", physical=False)
        # Instants.
        if self.t >= self.gcd:
            seal, sop, cl = self.skill("seal_of_iron"), self.skill("shield_of_penitent"), self.skill("zealous_cleave")
            if self.t >= self.seal_until and self.power >= self.cost(seal):
                self.power -= self.cost(seal)
                self.seal_until = self.t + self.seal_aura["duration_ms"] / 1000
                self.gcd = self.t + 1.5
            elif self.ready("sop") and self.power >= self.cost(sop) and s.shield is not None:
                self.power -= self.cost(sop)
                self.cd("sop", sop["realm_cooldown_ms"])
                self.sop_until = self.t + self.sop_aura["duration_ms"] / 1000
            elif self.power >= self.cost(cl) and (self.conviction >= 5 or (self.conviction >= 3 and self.power >= 60)):
                self.power -= self.cost(cl)
                self.gcd = self.t + 1.5
                o = self.yellow(s.crit)
                if o in ("hit", "crit"):
                    raw = self.rng.uniform(s.mh_min, s.mh_max) * self.opts.get("cleave_pct", 100) / 100 + linear(fx(cl, "weapon_strike")["bonus"], L)
                    self.hit_mob(raw, "cleave", True, o == "crit")
                    self.fire("melee_hit")
                    stacks = self.conviction
                    self.conviction = 0
                    c = fx(cl, "consume_aura_stacks")
                    holy = int(linear(c["per_stack"], L) * self.opts.get("conv_mult", 1.0)) * stacks
                    before = self.dmg["conviction"]
                    self.hit_mob(holy, "conviction", physical=False)
                    # The heal is a share of what every target took; the rage comes a stack.
                    self.heal((self.dmg["conviction"] - before) * c.get("heal_pct", 0) // 100, "conviction")
                    self.gain_rage(c.get("refund_per_stack", 0) * stacks)
        # Queue Righteous Smash.
        rs = self.skill("righteous_smash")
        smash_cost = self.opts.get("smash_cost", self.cost(rs))
        if not self.smash_queued and self.power >= smash_cost:
            self.smash_queued = True
        # Swing.
        if self.t >= self.swing_t:
            self.swing_t = self.t + s.mh_speed
            if self.smash_queued and self.power >= smash_cost:
                self.smash_queued = False
                self.power -= smash_cost
                o = self.yellow(s.crit)
                if o in ("hit", "crit"):
                    ws = fx(rs, "weapon_strike")
                    raw = (self.rng.uniform(s.mh_min, s.mh_max) + linear(ws["bonus"], L) * self.opts.get("smash_mult", 1.0)
                           + self.block_value() * ws.get("block_value_pct", 0) // 100)
                    d = self.hit_mob(raw, "smash", True, o == "crit")
                    self.heal(d * ws.get("heal_pct", 0) // 100, "smash")
                    self.fire("melee_hit")
                    self.add_conviction(fx(rs, "add_aura_stacks")["stacks"])
            else:
                self.smash_queued = False
                d, o = self.white()
                if o in ("hit", "crit"):
                    self.rage_dealt(d)
                    self.fire("melee_hit")
                elif o in ("dodge", "parry"):
                    self.rage_dealt(int(self.rng.uniform(s.mh_min, s.mh_max) * 0.75 * (1 - armor_dr(self.m_armor, L))))
        # Mob swing.
        if self.t >= self.mob_swing_t:
            self.mob_swing_t = self.t + self.m["swing"]
            d, o = self.mob_swing()
            if o in ("block", "parry"):
                self.fire(o)
            if d > 0:
                d = self.take(d)
                self.rage_taken(d)


# ---------------------------------------------------------------- Ironclad and Zealot
class Sturdy(Vanguard):
    """The Tier 2 plate Callings' core, the Vanguard's rage and swing: rage from white swings and
    blows taken, buffs kept as aura rows (self.buffs, as Sim.grant keeps them) whose block_pct,
    armor_items_pct, mod_damage_taken_pct, mod_spell_damage_pct and procs count while they are up,
    and the build's passives: their procs, armour, block, rage cap, crit and amounts."""

    def setup_sturdy(self):
        self.dist = 0.0
        self.mob_swing_t = 0.5
        self.swing_t = 0.0
        self.gcd = 0.0
        self.buffs = {}
        self.since_block = None   # when the character last blocked
        c, rows = self.calling, self.rows
        self.passive_procs = passive_effects(c, rows, "proc")
        # A max_power passive's amount is in tenths of rage, as the realm keeps rage (Fervent: +10).
        self.rage_cap = 100 + sum(p["amount"] for p in passive_effects(c, rows, "max_power")) // 10
        self.power = min(self.power, self.rage_cap)
        self.passive_armor = sum(p["pct"] for p in passive_effects(c, rows, "armor_items_pct"))
        self.passive_block = sum(p["pct"] for p in passive_effects(c, rows, "block_pct"))

    def gain_rage(self, n):
        self.power = min(self.power + n, self.rage_cap)

    def up(self, key):
        b = self.buffs.get(key)
        return b is not None and self.t < b[0]

    def can(self, key):
        """Skill `key` is off its cooldown, paid for, and free of the global cooldown if it starts one."""
        return (self.ready(key) and self.power >= self.cost(self.skill(key))
                and (self.gcd_of(key) == 0 or self.t >= self.gcd))

    def pay(self, key):
        """Skill `key` takes its rage and starts its cooldown and global cooldown."""
        self.power -= self.cost(self.skill(key))
        self.cd(key, self.cooldown_ms(key))
        self.gcd = max(self.gcd, self.t + self.gcd_of(key))

    def cast_aura(self, key):
        self.pay(key)
        self.grant(fx(self.skill(key), "grant_aura")["aura"])

    def pull_size(self):
        """The living mobs of the pull: one outside multi.Multi."""
        return len(self.alive()) if hasattr(self, "alive") else 1

    def player_armor(self):
        pct = self.passive_armor + sum(p["pct"] for p in self.buffed("armor_items_pct"))
        return 2 * self.s.agi + self.s.armor_items * (100 + pct) // 100

    def block_pct(self):
        if self.s.shield is None:
            return 0.0
        return self.s.block_pct + self.passive_block + sum(p["pct"] for p in self.buffed("block_pct"))

    def taken_pct(self):
        """The percent more (less, when negative) damage the character takes."""
        return sum(p["pct"] for p in self.buffed("mod_damage_taken_pct"))

    def take(self, dmg):
        return Sim.take(self, dmg * max(100 + self.taken_pct(), 0) // 100)

    def mob_swing(self, from_front=True, i=None):
        d, o = super().mob_swing(from_front, i)
        if o == "block":
            self.since_block = self.t
        return d, o

    def crit_of(self, key):
        return self.s.crit + self.skill_pct("skill_crit_pct", key)

    def school_hit(self, raw, src, school="physical", crit=False):
        """`raw` of `school` at the target: physical past its armour, any other school more by the
        buffs' mod_spell_damage_pct (Fury's Edge)."""
        if school == "physical":
            return self.hit_mob(raw, src, True, crit)
        return self.hit_mob(raw, src, False, crit, dmg_pct=sum(p["pct"] for p in self.buffed("mod_spell_damage_pct")))

    def weapon_strike(self, key, src):
        """Skill `key`'s weapon_strike, rolled as its defense_type says: its weapon_pct of a swing,
        its bonus, its block_value_pct of the block value, in its school, healing its heal_pct of
        what it dealt. Returns (damage, outcome)."""
        s, sk = self.s, self.skill(key)
        p = fx(sk, "weapon_strike")
        crit = self.crit_of(key)
        o = self.spell_roll(crit) if sk["defense_type"] == "magic" else self.yellow(crit)
        if o not in ("hit", "crit"):
            return 0, o
        raw = (self.rng.uniform(s.mh_min, s.mh_max) * p.get("weapon_pct", 100) / 100 + linear(p["bonus"], self.L)
               + self.block_value() * p.get("block_value_pct", 0) // 100)
        d = self.school_hit(raw, src, p.get("school", "physical"), o == "crit")
        self.heal(d * p.get("heal_pct", 0) // 100, src)
        self.fire("melee_hit")
        return d, o

    def white_swing(self):
        s = self.s
        d, o = self.white()
        if o in ("hit", "crit"):
            self.rage_dealt(d)
            self.fire("melee_hit")
        elif o in ("dodge", "parry"):
            self.rage_dealt(int(self.rng.uniform(s.mh_min, s.mh_max) * 0.75 * (1 - armor_dr(self.m_armor, self.L))))

    def fire(self, on):
        """The procs on `on` (melee_hit, block, parry): each buff's still up, then the passives'."""
        for b in list(self.buffs.values()):
            if self.t < b[0]:
                for p in fxs(b[1], "proc"):
                    if p["on"] == on:
                        self.proc(p, b[1]["key"])
        for p in self.passive_procs:
            if p["on"] == on:
                self.proc(p, "passive")

    def proc(self, p, src):
        """The proc `p` from `src`, when its chance comes up: a share of the maximum health, rage,
        an aura, and damage to the attacker."""
        if p.get("chance", 100) < 100 and self.rng.random() * 100 >= p["chance"]:
            return
        if p.get("heal_pct"):
            self.heal(self.s.max_hp * p["heal_pct"] // 100, src)
        if p.get("energize"):
            self.gain_rage(p["energize"])
        if p.get("apply_aura"):
            self.apply(p["apply_aura"])
        if p.get("damage_attacker"):
            self.school_hit(linear(p["damage_attacker"], self.L), src, p.get("school", "physical"))

    def apply(self, key):
        self.grant(key)

    def mob_turn(self):
        if self.t >= self.mob_swing_t:
            self.mob_swing_t = self.t + self.m["swing"]
            d, o = self.mob_swing()
            if o in ("block", "parry"):
                self.fire(o)
            if d > 0:
                self.rage_taken(self.take(d))


class Ironclad(Sturdy):
    """Righteous Smash on every swing it can afford; Shield Slam on its cooldown at SLAM_AT
    Conviction or more; Retort while a block's window is open; Shield of the Penitent and Steel
    Resolve on their cooldowns; Iron Wall under WALL_BELOW health; Hold the Line on a pull of 2 or
    more, for its block stacks (its taunt is nothing to the model). Rebuke is the Kick boss.py
    casts. self.fight_log holds (time, "shield_slam", Conviction spent) for each Shield Slam."""
    SLAM_AT = 3
    WALL_BELOW = 0.40

    def setup_fight(self):
        self.setup_sturdy()
        self.smash_queued = False
        self.conviction = 0
        self.conv_until = 0
        self.conv_aura = aura(self.calling, fx(self.skill("righteous_smash"), "add_aura_stacks")["aura"])

    def apply(self, key):
        if key == self.conv_aura["key"]:
            self.add_conviction(1)
        else:
            self.grant(key)

    def window_open(self):
        """Retort's caster_state=after_block window: a block within its window_ms."""
        window = self.skill("retort")["requires"]["window_ms"]
        return self.since_block is not None and (self.t - self.since_block) * 1000 <= window

    def hold_the_line(self):
        """Hold the Line's stacks, its stacks a mob for each mob of the pull, up to the aura's
        max_stacks."""
        self.pay("hold_the_line")
        p = fx(self.skill("hold_the_line"), "add_aura_stacks")
        a = aura(self.calling, p["aura"])
        held = self.buffs[a["key"]][2] if self.up(a["key"]) else 0
        self.grant(a["key"])
        self.buffs[a["key"]][2] = min(held + p["stacks"] * (self.pull_size() if p.get("per_target") else 1),
                                      a["max_stacks"])

    def shield_slam(self):
        sk = self.skill("shield_slam")
        stacks = self.conviction
        self.pay("shield_slam")
        self.fight_log.append((self.t, "shield_slam", stacks))
        d, o = self.weapon_strike("shield_slam", "shield_slam")
        if o not in ("hit", "crit"):
            return
        c = fx(sk, "consume_aura_stacks")
        self.conviction = 0
        self.school_hit(linear(c["per_stack"], self.L) * stacks, "conviction", c.get("school", "holy"))
        self.gain_rage(c.get("refund_per_stack", 0) * stacks)
        for p in fxs(sk, "stun"):
            self.stun(p)

    def retort(self):
        self.pay("retort")
        d, o = self.weapon_strike("retort", "retort")
        if o in ("hit", "crit"):
            self.add_conviction(fx(self.skill("retort"), "add_aura_stacks")["stacks"])

    def step(self):
        s = self.s
        if self.conviction and self.t > self.conv_until:
            self.conviction = 0
        # Off the global cooldown.
        if self.hp < s.max_hp * self.WALL_BELOW and self.can("iron_wall"):
            self.cast_aura("iron_wall")
        if s.shield is not None and self.can("shield_of_penitent"):
            self.cast_aura("shield_of_penitent")
        if self.can("steel_resolve"):
            self.cast_aura("steel_resolve")
        if self.pull_size() >= 2 and self.can("hold_the_line"):
            self.hold_the_line()
        # On it.
        if s.shield is not None and self.conviction >= self.SLAM_AT and self.can("shield_slam"):
            self.shield_slam()
        elif self.window_open() and self.can("retort"):
            self.retort()
        # Righteous Smash on the next swing.
        rs = self.skill("righteous_smash")
        if not self.smash_queued and self.power >= self.cost(rs):
            self.smash_queued = True
        if self.t >= self.swing_t:
            self.swing_t = self.t + s.mh_speed
            if self.smash_queued and self.power >= self.cost(rs):
                self.smash_queued = False
                self.power -= self.cost(rs)
                d, o = self.weapon_strike("righteous_smash", "smash")
                if o in ("hit", "crit"):
                    self.add_conviction(fx(rs, "add_aura_stacks")["stacks"])
            else:
                self.smash_queued = False
                self.white_swing()
        self.mob_turn()


class Zealot(Sturdy):
    """Bulwark Charge to open; Seal of Iron kept up, Judgement on its cooldown while the Seal is up,
    then the Seal again; Hallowed Ground and Holy Wrath on their cooldowns, the Zealot standing in
    its ground; Righteous Fury up from the pull (its threat is nothing to the model; a choice's
    damage taken or spell damage counts). Rebuke is the Kick boss.py casts, and Hammer of Justice
    stuns a boss's cast that Kick is not ready for."""

    def setup_fight(self):
        self.setup_sturdy()
        self.ground = None   # [until, next tick, damage, source, caster aura row, every]
        self.seal_key = fx(self.skill("seal_of_iron"), "grant_aura")["aura"]
        self.grant(fx(self.skill("righteous_fury"), "grant_aura")["aura"])
        if self.ready("bulwark_charge"):
            ch = self.skill("bulwark_charge")
            self.cd("bulwark_charge", self.cooldown_ms("bulwark_charge"))
            self.gain_rage(linear(fx(ch, "energize")["amount"], self.L))
            self.lay_ground(fx(ch, "ground_effect"), "consecration")

    def lay_ground(self, g, src):
        """Ground effect `g` at the Zealot's feet, in place of the one before: its damage a tick to
        every mob within its yards, and its caster aura on the Zealot standing in it."""
        ward = next(a for a in SPELLS[self.calling]["auras"] if a["spell_id"] == g["caster_aura_spell_id"])
        self.ground = [self.t + g["duration_ms"] / 1000, self.t + g["tick_ms"] / 1000, linear(g["amount"], self.L),
                       src, ward, g["tick_ms"] / 1000]

    def taken_pct(self):
        ward = sum(p["pct"] for p in fxs(self.ground[4], "mod_damage_taken_pct")) if self.on_ground() else 0
        return super().taken_pct() + ward

    def on_ground(self):
        return self.ground is not None and self.t < self.ground[0]

    def tick_ground(self):
        """A ground tick: its holy damage, and its caster aura's heal (Hallowed Ground's, more by its
        skill_amount_pct: Sanctified)."""
        g = self.ground
        if not self.on_ground() or self.t < g[1]:
            return
        g[1] += g[5]
        self.school_hit(g[2], g[3], "holy")
        for h in fxs(g[4], "periodic_heal"):
            self.heal(linear(h["tick"], self.L) * (100 + self.skill_pct("skill_amount_pct", "hallowed_ground")) // 100,
                      g[3])

    def judgement(self):
        """Judgement takes the Seal off: its holy amount, never missed, critting at the character's
        chance and the build's, and its heal_pct of the maximum health (rules/seals.rs, cast.rs)."""
        p = fx(self.skill("judgement"), "judgement")
        self.pay("judgement")
        del self.buffs[self.seal_key]
        crit = self.rng.random() * 100 < self.crit_of("judgement")
        self.school_hit(linear(p["amount"], self.L), "judgement", p.get("school", "holy"), crit)
        self.heal(self.s.max_hp * p["heal_pct"] // 100, "judgement")

    def holy_wrath(self):
        """Holy Wrath on every mob within its yards: its amount, vs_type_pct more on a mob of its
        vs_type, and its stun on only_type. Its defense_type none never misses or crits."""
        sk = self.skill("holy_wrath")
        p = fx(sk, "school_damage")
        raw = linear(p["amount"], self.L)
        if p.get("vs_type") and self.m.get("type") == p["vs_type"]:
            raw = raw * (100 + p["vs_type_pct"]) // 100
        self.school_hit(raw, "holy_wrath", p.get("school", "holy"))
        for st in fxs(sk, "stun"):
            if st.get("only_type") in (None, self.m.get("type")):
                self.stun(st)

    def hammer(self):
        """Hammer of Justice on a boss's cast: the cast stops, and the boss neither swings nor casts
        until the stun ends (boss.Boss.stun_cast)."""
        sk = self.skill("hammer_of_justice")
        self.pay("hammer_of_justice")
        if self.spell_roll(0) == "miss":
            return
        for p in fxs(sk, "stun"):
            self.stun(p)
            self.stun_cast(self.mob_swing_t)

    def step(self):
        s = self.s
        self.tick_ground()
        if getattr(self, "stunnable", None) and self.stunnable() and self.can("hammer_of_justice"):
            self.hammer()
        # Off the global cooldown.
        if self.up(self.seal_key) and self.can("judgement"):
            self.judgement()
        # On it.
        if not self.up(self.seal_key) and self.can("seal_of_iron"):
            self.cast_aura("seal_of_iron")
        elif self.can("hallowed_ground"):
            self.pay("hallowed_ground")
            self.lay_ground(fx(self.skill("hallowed_ground"), "ground_effect"), "hallowed_ground")
        elif self.can("holy_wrath"):
            self.pay("holy_wrath")
            self.holy_wrath()
        if self.t >= self.swing_t:
            self.swing_t = self.t + s.mh_speed
            self.white_swing()
        self.mob_turn()


# ---------------------------------------------------------------- Cutthroat
class Cutthroat(Sim):
    def dmg_pct(self):
        return self.bl_dmg * self.bloodlust if self.t < self.bl_until else 0

    def haste(self):
        return self.bl_haste * self.bloodlust if self.t < self.bl_until else 0

    def setup_fight(self):
        s, c = self.s, self.calling
        self.combo = 0
        self.swing_t = 0.0
        self.oh_t = s.oh_speed / 2 if s.oh else 1e9
        self.mob_swing_t = 0.5
        self.gcd = 0.0
        # The Bloodlust a bleed's droplet grants: its gains a stack, its stacks and its length, the
        # stacks a passive adds, and a passive's heal and energy on each pickup.
        self.drop_chance = SPELLS[c]["bleed_drop_chance"]
        self.bl_aura = aura(c, fx(aura(c, SPELLS[c]["bleed_drop"]), "pickup")["grants"])
        self.bl_dmg = sum(p["pct"] for p in fxs(self.bl_aura, "mod_damage_done_pct"))
        self.bl_haste = sum(p["pct"] for p in fxs(self.bl_aura, "mod_melee_haste_pct"))
        self.bl_max = self.bl_aura["max_stacks"] + sum(p["amount"] for p in passive_effects(c, self.rows, "aura_max_stacks")
                                                        if p["aura"] == self.bl_aura["key"])
        self.pickup_heal = sum(p["pct"] for p in passive_effects(c, self.rows, "pickup_heal_pct"))
        self.pickup_energy = sum(p["amount"] for p in passive_effects(c, self.rows, "pickup_energize"))
        self.bloodlust = 0
        self.bl_until = -1
        self.rupture = None  # [until, next tick, tick, every, leech pct, droplet chance]
        self.garrote = None
        # The aura the build's Evasion grants: its dodge and its procs on a dodge (Phantom Guile).
        ev = self.skill("evasion")
        self.ev_aura = self.granted("evasion") if fxs(ev, "grant_aura") else aura(c, fx(ev, "dodge_pct")["apply_aura"])
        self.ev_dodge = sum(p["pct"] for p in fxs(self.ev_aura, "dodge_pct"))
        self.ev_procs = [p for p in fxs(self.ev_aura, "proc") if p["on"] == "dodge"]
        self.evasion_until = -1
        self.mending = None  # [ticks left, next tick, heal, every]
        # Garrote from stealth: a crit if it lands.
        self.garrote_cast(100)

    def garrote_cast(self, crit_pct):
        """Garrote, when the energy is there: its bleed, its combo points and the stun its choice
        adds."""
        g = self.skill("garrote")
        if self.power < self.cost(g):
            return
        self.power -= self.cost(g)
        self.gcd = self.t + 1.0
        o = self.yellow(crit_pct)
        if o in ("hit", "crit"):
            a = aura(self.calling, fx(g, "periodic_damage")["apply_aura"])
            p = fx(a, "periodic_damage")
            every = p["tick_ms"] / 1000
            self.garrote = [self.t + a["duration_ms"] / 1000, self.t + every, linear(p["tick"], self.L), every,
                            p.get("leech_pct", 0), p.get("drop_chance", self.drop_chance)]
            self.add_combo(self.combo_points(g))
            for st in fxs(g, "stun"):
                self.stun(st)

    def dodge_more(self):
        return self.ev_dodge if self.t < self.evasion_until else 0

    def add_combo(self, n=1):
        self.combo = min(self.combo + n, 5)

    def bleeding(self):
        return any(b and self.t < b[0] for b in (self.rupture, self.garrote))

    def combo_points(self, sk):
        """The combo points `sk` awards: `bleeding_bonus` more on a bleeding target (Serrated
        Strikes)."""
        p = fx(sk, "add_combo_points")
        return p["amount"] + (p.get("bleeding_bonus", 0) if self.bleeding() else 0)

    def gain_energy(self, n):
        self.power = min(self.power + n, self.s.max_energy)

    def droplet(self, chance):
        """A bleed's tick spills a droplet `chance` percent of the time, and the character picks up
        some of them: a Bloodlust stack and the passive's heal and energy."""
        if self.rng.random() * 100 < chance and self.rng.random() < self.opts.get("pickup", DROPLET_PICKUP):
            self.bloodlust = min(self.bloodlust + 1, self.bl_max)
            self.bl_until = self.t + self.bl_aura["duration_ms"] / 1000
            self.heal(self.s.max_hp * self.pickup_heal // 100, "pickup")
            self.gain_energy(self.pickup_energy)

    def dodged(self):
        """The character dodged a swing: the procs of Evasion's aura while it is up."""
        if self.t < self.evasion_until:
            for p in self.ev_procs:
                if p.get("chance", 100) < 100 and self.rng.random() * 100 >= p["chance"]:
                    continue
                self.gain_energy(p.get("energize", 0))
                self.add_combo(p.get("add_combo_points", 0))

    def step(self):
        s, L = self.s, self.L
        if self.t > self.bl_until:
            self.bloodlust = 0
        # Bleeds.
        for b in (self.rupture, self.garrote):
            if b and self.t < b[0] and self.t >= b[1]:
                b[1] += b[3]
                d = self.hit_mob(b[2], "rupture" if b is self.rupture else "garrote", physical=False, dmg_pct=self.dmg_pct())
                self.heal(d * b[4] // 100, "leech")
                self.droplet(b[5])
        # Evasion when hurt, and the heal its choice adds over its ticks (Second Wind).
        if self.ready("evasion") and self.hp < s.max_hp * 0.6:
            ev = self.skill("evasion")
            self.cd("evasion", ev["realm_cooldown_ms"])
            self.evasion_until = self.t + self.ev_aura["duration_ms"] / 1000
            for p in fxs(ev, "periodic_heal"):
                ticks = p["duration_ms"] // p["tick_ms"]
                self.mending = [ticks, self.t + p["tick_ms"] / 1000, s.max_hp * p["pct"] // 100 // ticks, p["tick_ms"] / 1000]
        if self.mending and self.mending[0] > 0 and self.t >= self.mending[1]:
            self.mending[0] -= 1
            self.mending[1] += self.mending[3]
            self.heal(self.mending[2], "evasion_heal")
        # GCD skills.
        if self.t >= self.gcd:
            ss, ev, ru = self.skill("sinister_strike"), self.skill("eviscerate"), self.skill("rupture")
            g = self.skill("garrote")
            mob_frac = self.mob_hp / self.m["hp"]
            rupture_up = self.rupture and self.t < self.rupture[0]
            garrote_up = self.garrote and self.t < self.garrote[0]
            if self.combo >= 5 or (self.combo >= 3 and mob_frac < 0.25):
                if not rupture_up and mob_frac > 0.45 and self.power >= self.cost(ru):
                    self.power -= self.cost(ru)
                    self.gcd = self.t + 1.0
                    o = self.yellow(s.crit)
                    if o in ("hit", "crit"):
                        p = fx(ru, "periodic_damage")
                        dur = (p["duration_ms"] + linear(p["duration_per_combo"], L) * self.combo) / 1000
                        tick = linear(p["tick"], L) + linear(p["per_combo"], L) * self.combo
                        every = p["tick_ms"] / 1000
                        self.rupture = [self.t + dur, self.t + every, tick, every, p.get("leech_pct", 0),
                                        p.get("drop_chance", self.drop_chance)]
                    self.combo = 0
                elif self.power >= self.cost(ev):
                    self.power -= self.cost(ev)
                    self.gcd = self.t + 1.0
                    o = self.yellow(s.crit)
                    if o in ("hit", "crit"):
                        p = fx(ev, "school_damage")
                        raw = linear(p["amount"], L) + linear(p["per_combo"], L) * self.combo
                        more = p.get("bleeding_pct", 0) if self.bleeding() else 0
                        self.hit_mob(raw, "eviscerate", True, o == "crit", dmg_pct=self.dmg_pct() + more)
                        for st in fxs(ev, "stun"):
                            self.stun(st, self.combo)
                        if self.mob_hp <= 0:
                            # A killing blow (Executioner's Toll).
                            self.heal(s.max_hp * p.get("kill_heal_pct", 0) // 100, "kill")
                            self.gain_energy(p.get("kill_energize", 0))
                    self.combo = 0
            elif (not garrote_up and not g.get("requires_stealth") and mob_frac > 0.45 and self.hp >= s.max_hp * 0.6
                  and self.power >= self.cost(g)):
                # Garrote again once its bleed is gone, when it needs no Stealth (Brazen Garrote).
                self.garrote_cast(s.crit)
            elif self.power >= self.cost(ss):
                self.power -= self.cost(ss)
                self.gcd = self.t + 1.0
                o = self.yellow(s.crit)
                if o in ("hit", "crit"):
                    p = fx(ss, "weapon_strike")
                    raw = self.rng.uniform(s.mh_min, s.mh_max) * p["weapon_pct"] / 100 + linear(p["bonus"], L)
                    d = self.hit_mob(raw, "sinister_strike", True, o == "crit", dmg_pct=self.dmg_pct())
                    # A second mob in reach takes a share of the strike (Flurry of Blades).
                    self.splash(d * p.get("cleave_pct", 0) // 100, "sinister_strike_cleave")
                    self.add_combo(self.combo_points(ss))
        # Whites.
        h = self.haste()
        if self.t >= self.swing_t:
            self.swing_t = self.t + s.mh_speed * 100 / (100 + h)
            self.white("mh")
        if s.oh and self.t >= self.oh_t:
            self.oh_t = self.t + s.oh_speed * 100 / (100 + h)
            self.white("oh")
        # Mob.
        if self.t >= self.mob_swing_t:
            self.mob_swing_t = self.t + self.m["swing"]
            d, o = self.mob_swing()
            if o == "dodge":
                self.dodged()
            if d > 0:
                self.take(d)


# ---------------------------------------------------------------- Initiate
class Initiate(Sim):
    def setup_fight(self):
        c = self.calling
        self.dist = 30.0
        self.mob_swing_t = 0.0
        self.gcd = 0.0
        self.cast = None   # (done_at, key, the next_cast changes it took)
        self.ember = None  # [until, next, tick, stacks]
        self.cold_until = -1
        self.slow_until = -1
        self.shields = {}  # skill key -> [left, until, amount, absorb params], first up first
        self.buffs = {}    # aura key -> [until, aura row, stacks, charges] on the Initiate
        self.ember_fx = fx(self.skill("searing_ember"), "periodic_damage")
        self.ember_aura = aura(c, self.ember_fx["apply_aura"])
        self.ember_every = fx(self.ember_aura, "periodic_damage")["tick_ms"]
        self.cold_aura = aura(c, fx(self.skill("biting_cold"), "mod_run_speed")["apply_aura"])
        self.procs = passive_effects(c, self.rows, "proc")

    def cast_time(self, key):
        return self.skill(key).get("cast_time_ms", 0)

    def gain_mana(self, n):
        self.power = min(self.power + n, self.s.max_mana)

    def burning(self, e):
        return e is not None and self.t < e[0]

    def on_target(self, key):
        """Whether the target wears aura `key` of the Initiate's: its Ember or its Cold."""
        if key == self.ember_aura["key"]:
            return self.burning(self.ember)
        return key == self.cold_aura["key"] and self.t < self.cold_until

    def spell_pct(self):
        return sum(p["pct"] for p in self.buffed("mod_spell_damage_pct"))

    def amount_pct(self, key):
        return sum(p["pct"] for p in self.buffed("skill_amount_pct") if p["skill"] == key)

    def fire(self, on, key):
        """The passives' procs on `on` ("cast", "consume") of skill `key`: an aura on the
        Initiate. A proc that roots the mobs around it is in IGNORED."""
        for p in self.procs:
            if p["on"] == on and p["skill"] == key and p.get("apply_aura"):
                self.grant(p["apply_aura"])

    def shield(self, key, amount, ms, p):
        """A shield of `amount` from skill `key` for `ms`, in place of the one `key` put up before;
        `p` holds its riders (energize_pct, burst_radius_yd)."""
        self.shields.pop(key, None)
        self.shields[key] = [amount, self.t + ms / 1000, amount, p]

    def take(self, dmg):
        """`dmg` past the shields up: each pays its riders on what it soaks, and a ward that
        breaks with burst_radius_yd bursts for its amount at every mob in the pull, all standing
        within its yards when a blow breaks it."""
        for key, w in list(self.shields.items()):
            soaked = min(w[0], dmg)
            if not soaked:
                continue
            w[0] -= soaked
            dmg -= soaked
            self.gain_mana(soaked * w[3].get("energize_pct", 0) // 100)
            if not w[0]:
                del self.shields[key]
                if w[3].get("burst_radius_yd"):
                    o = self.spell_roll(self.s.crit)
                    if o != "miss":
                        self.hit_mob(w[2], "ward_burst", False, o == "crit", dmg_pct=self.spell_pct())
        return Sim.take(self, dmg)

    def weaken_pct(self, i=None):
        return self.burn_weaken(self.ember)

    def burn_weaken(self, e):
        """The percent less a mob burning with Ember `e` hits the Initiate for."""
        return self.ember_fx.get("weaken_pct", 0) if self.burning(e) else 0

    def new_ember(self, stacks, longer_ms=0):
        """A Searing Ember of `stacks` from now, `longer_ms` past its aura's duration."""
        L, sp = self.L, self.s.spell_power
        dur = self.ember_aura["duration_ms"]
        tick = linear(fx(self.ember_aura, "periodic_damage")["tick"], L) + sp_periodic(sp, dur, dur // self.ember_every) // 2
        return [self.t + (dur + longer_ms) / 1000, self.t + self.ember_every / 1000, tick, stacks]

    def tick_ember(self, e):
        """Ember `e` on the target ticks: its damage, then its riders, mana back and a chance of
        the next-cast aura."""
        e[1] += self.ember_every / 1000
        self.hit_mob(e[2] * e[3], "ember_dot", False)
        p = self.ember_fx
        self.gain_mana(self.s.max_mana * p.get("tick_energize_pct_of_max", 0) // 100)
        if p.get("tick_aura") and self.rng.random() * 100 < p.get("tick_aura_chance", 100):
            self.grant(p["tick_aura"])

    def spread(self, ember=False, cold=False):
        """Copies of the Ember or the Cold a consume took, to the next mob of a pull: none in a
        fight with one (multi.InitiateN has the others)."""

    def start(self, key):
        # The buffs that change the cast go with it.
        mods = self.next_casts(key)
        self.use_next_casts(key)
        ms = 0 if any(p.get("instant") for p in mods) else self.cast_time(key) + sum(p.get("cast_time_ms", 0) for p in mods)
        self.cast = (self.t + max(ms, 0) / 1000, key, mods)
        self.gcd = self.t + 1.5

    def finish(self, key, mods):
        s, L = self.s, self.L
        sp = s.spell_power
        crit = s.crit
        sk = self.skill(key)
        if key == "ley_spark":
            was_full = self.power >= s.max_mana
            cost = self.price(key, mods)
            self.spend_mana(cost)
            p = fx(sk, "school_damage")
            crit += p["crit_pct_at_full_mana"] * self.power / s.max_mana
            o = self.spell_roll(crit)
            if o != "miss":
                raw = linear(p["amount"], L) * (100 + self.amount_pct(key)) // 100 + sp_direct(sp, self.cast_time(key))
                d = self.hit_mob(raw, "ley_spark", False, o == "crit", dmg_pct=self.spell_pct())
                # A ward off a crit, when larger than the one it put up (Warding Spark).
                for a in fxs(sk, "absorb"):
                    amount = d * a["pct"] // 100
                    if (o == "crit" or not a.get("if_crit")) and self.shields.get(key, [0])[0] < amount:
                        self.shield(key, amount, a["duration_ms"], a)
                for g in fxs(sk, "grant_aura"):
                    self.grant(g["aura"])
                # The cost back, or at a bar full before the cast a shield of it (Ley-Line Tap).
                for r in fxs(sk, "refund_cost"):
                    if self.rng.random() * 100 < r["chance"]:
                        if r.get("full_absorb_pct") and was_full:
                            self.shield(key, cost * r["full_absorb_pct"] // 100, r["absorb_duration_ms"], {})
                        else:
                            self.gain_mana(cost * r["pct"] // 100)
        elif key == "searing_ember":
            self.spend_mana(self.price(key, mods))
            o = self.spell_roll(crit)
            if o != "miss":
                raw = linear(fx(sk, "school_damage")["amount"], L) + sp_direct(sp, 0)
                self.hit_mob(raw, "ember_hit", False, o == "crit", dmg_pct=self.spell_pct())
                stacks = min((self.ember[3] if self.burning(self.ember) else 0) + 1, self.ember_aura["max_stacks"])
                self.ember = self.new_ember(stacks, sum(p.get("duration_ms", 0) for p in mods))
        elif key == "biting_cold":
            self.spend_mana(self.price(key, mods))
            o = self.spell_roll(crit)
            if o != "miss":
                p = fx(sk, "school_damage")
                raw = linear(p["amount"], L) + sp_direct(sp, self.cast_time(key))
                if p.get("vs_aura") and self.on_target(p["vs_aura"]):
                    raw = raw * (100 + p["vs_aura_pct"]) // 100
                self.hit_mob(raw, "biting_cold", False, o == "crit", dmg_pct=self.spell_pct())
                self.cold_until = self.slow_until = self.t + self.cold_aura["duration_ms"] / 1000
        elif key == "catalyst":
            self.cd("catalyst", sk["realm_cooldown_ms"])
            took = {p["aura"]: p for p in fxs(sk, "consume_aura")}
            cold, ember = took[self.cold_aura["key"]], took[self.ember_aura["key"]]
            consumed = False
            if self.t < self.cold_until:
                self.cold_until = -1
                raw = linear(cold["amount"], L) * cold["amount_pct"] // 100
                o = "crit" if self.rng.random() * 100 < crit else "hit"
                self.hit_mob(raw, "catalyst", False, o == "crit", dmg_pct=self.spell_pct())
                # A stun that needs the Cold the shatter took (Deep Freeze).
                for st in fxs(sk, "stun"):
                    self.stun(st)
                if cold.get("spread_yd"):
                    self.spread(cold=True)
                consumed = True
            if self.burning(self.ember):
                self.ember = None
                self.gain_mana(s.max_mana * ember["energize_pct_of_max"] // 100)
                if ember.get("spread_yd"):
                    self.spread(ember=True)
                consumed = True
            if consumed:
                self.fire("consume", key)

    def step(self):
        s = self.s
        # Ember ticks.
        if self.burning(self.ember) and self.t >= self.ember[1]:
            self.tick_ember(self.ember)
        for key in [k for k, w in self.shields.items() if self.t > w[1]]:
            del self.shields[key]
        # Casting.
        if self.cast and self.t >= self.cast[0]:
            _, key, mods = self.cast
            self.cast = None
            self.finish(key, mods)
        in_melee = self.dist <= 5
        if not self.cast and self.t >= self.gcd:
            ward, blink = self.skill("ley_ward"), self.skill("blink")
            if self.ready("ward") and self.power >= self.cost(ward):
                self.spend_mana(self.cost(ward))
                self.cd("ward", ward["realm_cooldown_ms"])
                p = fx(ward, "absorb")
                self.shield("ley_ward", s.max_mana * p["pct"] // 100, p["duration_ms"], p)
            elif in_melee and self.ready("blink") and self.power >= self.cost(blink) and self.t < self.slow_until:
                self.spend_mana(self.cost(blink))
                self.cd("blink", blink["realm_cooldown_ms"])
                self.dist = float(fx(blink, "blink")["distance_yd"])
                self.gcd = self.t + 1.5
                self.fire("cast", "blink")
            elif self.ready("catalyst") and self.t < self.cold_until and self.burning(self.ember) and not self.opts.get("smart"):
                self.start("catalyst")
            elif not self.burning(self.ember) and self.power >= self.price("searing_ember"):
                self.start("searing_ember")
            elif self.t >= self.cold_until and self.power >= self.price("biting_cold") and ((not self.opts.get("smart") and (self.ready("catalyst") or self.cds["catalyst"] - self.t < 3)) or (in_melee and self.ready("blink"))):
                self.start("biting_cold")
            elif self.power >= self.price("ley_spark"):
                self.start("ley_spark")
        # Mob movement.
        if self.dist > 5:
            spd = MOB_RUN * (0.5 if self.t < self.slow_until else 1.0)
            self.dist = max(self.dist - spd * DT, 0)
            self.mob_swing_t = max(self.mob_swing_t, self.t + 0.3)
        elif self.t >= self.mob_swing_t:
            self.mob_swing_t = self.t + self.m["swing"]
            d, o = self.mob_swing()
            if d > 0:
                self.take(d)


# ---------------------------------------------------------------- Ranger
class Ranger(Sim):
    def setup_fight(self):
        c, rows = self.calling, self.rows
        self.dist = 30.0
        self.moving = False
        self.moving_since = None
        self.still_since = self.t
        self.mob_swing_t = 0.0
        self.gcd = 0.0
        self.cast = None
        self.shot_t = self.t + 0.3
        self.swing_t = 0.0
        self.barbed = None  # [until, next, tick]
        self.slow_until = -1
        self.sunder = None  # [until, stacks, aura row] of the armour aura a barb left (Jagged Barbs)
        self.dressing = None  # [until, next, tick, every]
        self.buffs = {}  # aura key -> [until, aura row, stacks, charges] on the Ranger
        self.barbed_fx = fx(self.skill("barbed_arrow"), "periodic_damage")
        self.barbed_aura = aura(c, self.barbed_fx["apply_aura"])
        self.barbed_every = fx(self.barbed_aura, "periodic_damage")["tick_ms"] / 1000
        self.slow = (100 + fx(self.barbed_aura, "mod_run_speed")["pct"]) / 100
        # No minimum range with Point Blank: the Ranger shoots in melee.
        self.min_range = 0 if passive_effects(c, rows, "no_min_range") else self.skill("auto_shot")["realm_range_min"]
        self.near = [p for p in passive_effects(c, rows, "skill_amount_pct") if p.get("max_distance_yd")]
        # The talent row's choice the Ranger casts: a buff on a cooldown (Explosive Payload).
        self.talent_cast = next((sk for sk in passives(c, rows) if sk.get("realm_cooldown_ms")), None)

    def ranged_haste(self):
        return sum(p["pct"] for p in self.buffed("mod_ranged_haste_pct"))

    def in_range(self):
        return not self.min_range or self.dist > self.min_range

    def armor_loss(self):
        if self.sunder and self.t < self.sunder[0]:
            return -sum(p["amount"] for p in fxs(self.sunder[2], "mod_armor")) * self.sunder[1]
        return 0

    def sunder_on(self, key):
        """A stack of armour aura `key` on the target, up to its max_stacks, for its duration."""
        a = aura(self.calling, key)
        stacks = self.sunder[1] if self.sunder and self.t < self.sunder[0] else 0
        self.sunder = [self.t + a["duration_ms"] / 1000, min(stacks + 1, a["max_stacks"]), a]

    def shoot(self, src, weapon_pct=100, bonus=0, armor_ignore=0, crit_pct=0, crit_bonus=0, amount_pct=0,
              physical=True):
        """A ranged strike: `crit_pct` more crit chance, `crit_bonus` percent more on a crit and
        `amount_pct` percent more on the whole strike, as the realm's weapon_strike reads them."""
        s = self.s
        o = self.ranged_roll(s.crit + crit_pct)
        if o in ("miss", "dodge"):
            return 0, o
        raw = self.rng.uniform(s.r_min, s.r_max) * weapon_pct / 100 + bonus
        if amount_pct:
            raw = raw * (100 + amount_pct) / 100
        d = self.hit_mob(raw, src, physical, o == "crit", crit_bonus, armor_ignore)
        return d, o

    def cast_ms(self, key):
        """Skill `key`'s cast time now: none against a target under its strike's below_health_pct
        when below_instant (Killshot), else its cast_time_ms with the next_cast buffs' changes, a
        stack each, never below none (Rhythmic Draw)."""
        sk, mods = self.skill(key), self.next_casts(key)
        p = fx(sk, "weapon_strike")
        if any(m.get("instant") for m in mods) or (
                p.get("below_instant") and self.mob_hp * 100 < self.m["hp"] * p["below_health_pct"]):
            return 0
        return max(sk.get("cast_time_ms", 0) + sum(m.get("cast_time_ms", 0) for m in mods), 0)

    def start(self, key, ms):
        self.cast = (self.t + ms / 1000, key)
        self.gcd = self.t + 1.5

    def snapshot(self, free=False, repeat=False):
        """The build's Snapshot: paid with a global cooldown, or `free` (a repeat, a cast on a
        Vault's landing). Its buffs' changes, the extra arrows, a crit's cut to another skill's
        cooldown and a free repeat go as its spell says."""
        s, L = self.s, self.L
        sk = self.skill("snapshot")
        if not free:
            cost = self.price("snapshot")
            if cost:
                self.spend_mana(cost)
            self.gcd = self.t + 1.5
        p = fx(sk, "weapon_strike")
        bonus = linear(self.opts.get("snap_bonus", p["bonus"]), L)
        crit = sum(q["pct"] for q in self.buffed("skill_crit_pct") if q["skill"] == "snapshot")
        d, o = self.shoot("snapshot", self.opts.get("snap_pct", 100), bonus, crit_pct=crit)
        landed = o not in ("miss", "dodge")
        if landed:
            # An extra arrow shares the Snapshot's outcome (Rapid Assault).
            for q in self.buffed("extra_strike"):
                if q["skill"] == "snapshot":
                    self.hit_mob(self.rng.uniform(s.r_min, s.r_max) * q["weapon_pct"] / 100, "snapshot", True, o == "crit")
            for g in fxs(sk, "grant_aura"):
                self.grant(g["aura"])
            if o == "crit" and p.get("crit_cooldown_spell_id"):
                cooled = key_of(self.calling, p["crit_cooldown_spell_id"])
                if cooled in self.cds:
                    self.cds[cooled] -= p["crit_cooldown_ms"] / 1000
        self.use_next_casts("snapshot")
        # A repeat never repeats (Twin Arrows).
        if landed and p.get("repeat_chance") and not repeat and self.rng.random() * 100 < p["repeat_chance"]:
            self.snapshot(free=True, repeat=True)

    def step(self):
        s, L = self.s, self.L
        # Dots and hots: a barb's tick pays its riders, mana back and a chance of an armour aura
        # on the target (Bloodscent, Jagged Barbs).
        if self.barbed and self.t < self.barbed[0] and self.t >= self.barbed[1]:
            self.barbed[1] += self.barbed_every
            self.hit_mob(self.barbed[2], "barbed", False)
            p = self.barbed_fx
            if p.get("tick_energize_pct_of_base"):
                self.power = min(self.power + self.pct_base(p["tick_energize_pct_of_base"]), s.max_mana)
            if p.get("tick_aura") and self.rng.random() * 100 < p.get("tick_aura_chance", 100):
                self.sunder_on(p["tick_aura"])
        if self.dressing and self.t < self.dressing[0] and self.t >= self.dressing[1]:
            self.dressing[1] += self.dressing[3]
            self.heal(self.dressing[2], "field_dressing")
        slowed = self.t < self.slow_until
        spd = MOB_RUN * (self.slow if slowed else 1.0)
        in_melee = self.dist <= 5
        can_shoot = self.in_range()
        # Finish casts.
        if self.cast and self.t >= self.cast[0]:
            key = self.cast[1]
            self.cast = None
            if key == "puncturing_shaft":
                sk = self.skill(key)
                p = fx(sk, "weapon_strike")
                near = sum(q["pct"] for q in self.near if q["skill"] == key and self.dist <= q["max_distance_yd"])
                d, o = self.shoot(key, p.get("weapon_pct", 100), linear(p["bonus"], L), linear(p.get("armor_ignore"), L),
                                  crit_bonus=p.get("crit_damage_pct", 0), amount_pct=near)
                if o not in ("miss", "dodge"):
                    for st in fxs(sk, "stun"):
                        self.stun(st)
        # Decide movement: kite when in melee range and the mob is slowed.
        # Kite while the mob is slowed: start when it is within 8 yd and Vault is down, stop at 15 yd.
        want_move = self.moving
        if self.opts.get("kite", False) and slowed:
            if self.dist <= 8 and not self.ready("vault"):
                want_move = True
            if self.dist >= 15:
                want_move = False
        else:
            want_move = False
        if want_move != self.moving:
            self.moving = want_move
            if want_move:
                self.moving_since = self.t
            else:
                self.still_since = self.t
            if self.cast:
                self.cast = None  # a move breaks the cast
        # Instants and casts.
        if not self.cast and self.t >= self.gcd:
            vault, rapid, dressing = self.skill("vault"), self.skill("rapid_fire"), self.skill("field_dressing")
            barbed, shaft = self.skill("barbed_arrow"), self.skill("puncturing_shaft")
            shaft_ms = self.cast_ms("puncturing_shaft")
            if self.hp < s.max_hp * 0.5 and self.ready("dressing") and self.power >= self.cost(dressing):
                self.spend_mana(self.cost(dressing))
                self.cd("dressing", dressing["realm_cooldown_ms"])
                p = fx(dressing, "periodic_heal")
                every = p["tick_ms"] / 1000
                self.dressing = [self.t + p["duration_ms"] / 1000, self.t + every,
                                 s.max_hp * p["pct"] // 100 // (p["duration_ms"] // p["tick_ms"]), every]
                self.gcd = self.t + 1.5
            elif in_melee and self.ready("vault"):
                self.cd("vault", vault["realm_cooldown_ms"])
                self.dist = float(fx(vault, "blink")["distance_yd"])
                self.gcd = self.t + 1.0
                # A Snapshot on landing, free, the build's own (Parting Shot).
                for p in fxs(vault, "cast_skill"):
                    if key_of(self.calling, p["skill_spell_id"]) == "snapshot" and self.in_range():
                        self.snapshot(free=True)
            elif self.ready("rapid") and can_shoot and not self.moving:
                self.cd("rapid", rapid["realm_cooldown_ms"])
                for g in fxs(rapid, "grant_aura"):
                    self.grant(g["aura"])
            elif self.talent_cast and self.ready("talent_cast") and can_shoot and not self.moving:
                self.cd("talent_cast", self.talent_cast["realm_cooldown_ms"])
                for g in fxs(self.talent_cast, "grant_aura"):
                    self.grant(g["aura"])
            elif can_shoot and not (self.barbed and self.t < self.barbed[0] - 2.5) and self.power >= self.cost(barbed):
                self.spend_mana(self.cost(barbed))
                self.gcd = self.t + 1.5
                o = self.ranged_roll(0)
                if o not in ("miss", "dodge"):
                    a = self.barbed_aura
                    self.barbed = [self.t + a["duration_ms"] / 1000, self.t + self.barbed_every,
                                   linear(fx(a, "periodic_damage")["tick"], L)]
                    self.slow_until = self.t + a["duration_ms"] / 1000
            # A Shaft starts when it ends before the mob is inside the minimum range.
            elif can_shoot and not self.moving and self.power >= self.cost(shaft) and (
                    not self.min_range or (self.dist - self.min_range) / spd >= shaft_ms / 1000 * 0.9):
                self.spend_mana(self.cost(shaft))
                self.use_next_casts("puncturing_shaft")
                self.start("puncturing_shaft", shaft_ms)
            elif can_shoot and self.power >= self.price("snapshot"):
                self.snapshot()
        # Auto shot, or the skill a buff makes it (Explosive Payload), which takes the buff.
        if can_shoot and not self.moving and not self.cast and self.t >= self.shot_t:
            self.shot_t = self.t + s.r_speed * 100 / (100 + self.ranged_haste())
            becomes = [p["becomes"] for p in self.next_casts("auto_shot") if p.get("becomes")]
            if becomes:
                self.use_next_casts("auto_shot")
                p = fx(skill(self.calling, becomes[0]), "weapon_strike")
                self.shoot(becomes[0], p.get("weapon_pct", 100), linear(p["bonus"], L),
                           physical=p.get("school", "physical") == "physical")
            else:
                d, o = self.shoot("auto_shot")
                for p in self.buffed("proc") if d else []:
                    if p["on"] == "damage_done" and p.get("skill") == "auto_shot":
                        self.power = min(self.power + self.pct_base(p.get("energize_pct_of_base", 0)), s.max_mana)
        # Melee when stuck.
        if in_melee and not self.moving and not can_shoot and self.t >= self.swing_t:
            self.swing_t = self.t + s.mh_speed
            self.white("mh")
        # Mob movement and swings.
        if self.moving:
            self.dist += (PLAYER_RUN - spd) * DT
        if self.dist > 5:
            self.dist = max(self.dist - spd * DT, 0) if not self.moving else self.dist
            if self.moving and self.dist > 5:
                pass
            self.mob_swing_t = max(self.mob_swing_t, self.t + 0.3)
        if self.dist <= 5 and self.t >= self.mob_swing_t:
            self.mob_swing_t = self.t + self.m["swing"]
            d, o = self.mob_swing()
            if d > 0:
                self.take(d)


# ---------------------------------------------------------------- Acolyte
class Acolyte(Sim):
    def setup_fight(self):
        c = self.calling
        self.dist = 30.0
        self.mob_swing_t = 0.0
        self.gcd = 0.0
        self.cast = None      # (done_at, key)
        self.channel = None   # [until, next]
        self.blight = None    # [until, next, tick, stacks] on the target
        self.miasma = None    # [until, next, tick, next growth]
        self.mending_dot = None  # [until, next, tick] on self
        self.lance_hots = []  # [until, next, tick]
        self.maso = None      # [until, next, tick, every] on self (Masochism)
        self.shields = {}     # skill key -> [left, until], first up first
        self.buffs = {}       # aura key -> [until, aura row, stacks, charges] on the Acolyte
        self.blight_fx = fx(self.skill("creeping_blight"), "periodic_damage")
        self.blight_aura = aura(c, self.blight_fx["apply_aura"])
        self.blight_dot = fx(self.blight_aura, "periodic_damage")
        self.lance_hot = fx(self.skill("blood_lance"), "periodic_heal")
        self.toll_aura = aura(c, fx(self.skill("dark_mending"), "periodic_damage")["apply_aura"])
        self.toll = fx(self.toll_aura, "periodic_damage")
        self.cloud = fx(self.skill("miasma"), "ground_area")
        self.cloud_aura = aura(c, self.cloud["enemy_aura"])
        self.cloud_dot = fx(self.cloud_aura, "periodic_damage")
        self.cloud_miss = -sum(p["pct"] for p in fxs(self.cloud_aura, "mod_hit_chance"))
        self.procs = passive_effects(c, self.rows, "proc")
        # The talent row's choice the Acolyte casts: a consume on a cooldown (Necrotic Detonation).
        self.talent_cast = next((sk for sk in passives(c, self.rows) if sk.get("realm_cooldown_ms")), None)

    def spell_crit(self):
        return self.s.crit

    def blighted(self, b):
        return b is not None and self.t < b[0]

    def new_blight(self):
        """A Creeping Blight of one stack from now, for its aura's duration."""
        dur, every = self.blight_aura["duration_ms"], self.blight_dot["tick_ms"]
        tick = linear(self.blight_dot["tick"], self.L) + sp_periodic(self.s.spell_power, dur, dur // every)
        return [self.t + dur / 1000, self.t + every / 1000, tick, 1]

    def grow(self, b, n=1):
        """Blight `b`, while it lasts, gains `n` stacks, up to its aura's max_stacks."""
        if self.blighted(b):
            b[3] = min(b[3] + n, self.blight_aura["max_stacks"])

    def grow_others(self, n=1):
        """The Blights on the other mobs of a pull gain `n` stacks: none in a fight with one
        (multi.AcolyteN has the others)."""

    def spread_blight(self, b):
        """A copy of Blight `b` on the next mob of a pull that wears none: none in a fight with one
        (multi.AcolyteN has the others)."""

    def tick_blight(self, b):
        """Blight `b` on the target ticks: its damage once a stack, the leech and a stack more, then
        its riders: the next-cast aura on a chance a stack (Virulence) and a copy on the next mob
        of a pull on a chance (Epidemic)."""
        p, dot = self.blight_fx, self.blight_dot
        b[1] += dot["tick_ms"] / 1000
        stacks = b[3]
        d = self.hit_mob(b[2] * stacks, "blight", False)
        self.heal(d * dot["leech_pct"] // 100, "blight_leech")
        b[3] = min(stacks + dot["stacks_per_tick"], self.blight_aura["max_stacks"])
        if p.get("tick_aura") and self.rng.random() * 100 < min(p["tick_aura_chance_per_stack"] * stacks, 100):
            self.grant(p["tick_aura"])
        if p.get("tick_spread_chance") and self.rng.random() * 100 < p["tick_spread_chance"]:
            self.spread_blight(b)

    def weaken_pct(self, i=None):
        return self.blight_weaken(self.blight)

    def blight_weaken(self, b):
        """The percent less a mob wearing Blight `b` hits for: a share a stack, up to a cap
        (Withering Blight)."""
        p = self.blight_fx
        if not p.get("weaken_per_stack_pct") or not self.blighted(b):
            return 0
        return min(p["weaken_per_stack_pct"] * b[3], p["weaken_max_pct"])

    def in_cloud(self):
        """Whether the Miasma is up with the mob inside its radius."""
        return self.miasma is not None and self.t < self.miasma[0] and self.dist <= self.cloud["radius_yd"]

    def mob_miss_more(self):
        # Enemies inside miss more (Noxious Vapors).
        return self.cloud_miss if self.in_cloud() else 0

    def shield(self, key, amount, ms):
        """A shield of `amount` from skill `key` for `ms`, in place of the one `key` put up before."""
        self.shields.pop(key, None)
        if amount > 0:
            self.shields[key] = [amount, self.t + ms / 1000]

    def grow_shield(self, key, amount, cap, ms):
        """Skill `key`'s shield grows by `amount`, up to `cap`, and lasts `ms` from now (Gorging
        Siphon)."""
        w = self.shields.get(key)
        left = w[0] if w and self.t < w[1] else 0
        self.shield(key, min(left + amount, cap), ms)

    def take(self, dmg):
        """`dmg` past the shields up, first up first."""
        for key, w in list(self.shields.items()):
            soaked = min(w[0], dmg) if self.t < w[1] else 0
            w[0] -= soaked
            dmg -= soaked
            if not w[0] or self.t >= w[1]:
                del self.shields[key]
        return Sim.take(self, dmg)

    def health_cost(self, sk):
        """`sk`'s cost paid in health, its realm_cost percent of the maximum. A passive's proc on a
        cast that costs health heals its pct of the cost over its aura's ticks (Masochism)."""
        cost = self.s.max_hp * sk["realm_cost"] // 100
        self.hp -= cost
        self.costs += cost
        for p in self.procs:
            if p["on"] == "cast" and p.get("cost_kind") == "health" and cost:
                a = aura(self.calling, p["apply_aura"])
                h = fx(a, "periodic_heal")
                ticks = a["duration_ms"] // h["tick_ms"]
                self.maso = [self.t + a["duration_ms"] / 1000, self.t + h["tick_ms"] / 1000,
                             cost * h["pct"] // 100 // ticks, h["tick_ms"] / 1000]

    def start(self, key):
        # The buffs that change the cast go with it (Virulence, Sanguine Flow).
        mods = self.next_casts(key)
        self.use_next_casts(key)
        ms = 0 if any(p.get("instant") for p in mods) else self.skill(key)["cast_time_ms"] + sum(p.get("cast_time_ms", 0) for p in mods)
        self.cast = (self.t + max(ms, 0) / 1000, key)
        self.gcd = self.t + 1.5

    def lance(self):
        """Blood Lance lands: its health cost, then the hit, more a stack of the target's Blight
        (Exsanguinate), a heal over time of its share of the hit, a shield of a share (Sanguine
        Barrier), a share on the other mobs of a pull (Splintering Lance), and a stack on the
        target's Blight and, with a radius, on theirs."""
        s = self.s
        sk = self.skill("blood_lance")
        self.health_cost(sk)
        p = fx(sk, "school_damage")
        o = self.spell_roll(self.spell_crit())
        if o == "miss":
            return
        raw = linear(p["amount"], self.L) + sp_direct(s.spell_power, sk["cast_time_ms"])
        if p.get("vs_aura") == self.blight_aura["key"] and self.blighted(self.blight):
            more = p["vs_aura_pct"] * (self.blight[3] if p.get("vs_aura_per_stack") else 1)
            raw = raw * (100 + min(more, p.get("vs_aura_max_pct") or more)) // 100
        d = self.hit_mob(raw, "blood_lance", False, o == "crit")
        h = self.lance_hot
        ticks = h["duration_ms"] // h["tick_ms"]
        self.lance_hots.append([self.t + h["duration_ms"] / 1000, self.t + h["tick_ms"] / 1000, d * h["pct"] // 100 // ticks])
        for a in fxs(sk, "absorb"):
            self.shield("blood_lance", d * a["pct"] // 100, a["duration_ms"])
        if p.get("splash_pct"):
            self.splash(d * p["splash_pct"] // 100, "splintering_lance", n=0)
        g = fx(sk, "grow_aura")
        self.grow(self.blight, g["stacks"])
        if g.get("radius_yd"):
            self.grow_others(g["stacks"])

    def mending_amount(self):
        return linear(fx(self.skill("dark_mending"), "heal")["amount"], self.L) + sp_direct(self.s.spell_power, 0)

    def surplus(self):
        """(what Dark Mending would heal past the maximum health now, its cost paid; the cost)."""
        cost = self.s.max_hp * self.skill("dark_mending")["realm_cost"] // 100
        return self.hp - cost + self.mending_amount() - self.s.max_hp, cost

    def mend(self):
        """Dark Mending: its health cost, the heal, the toll of its pct of what the heal healed over
        the aura's ticks, and the choice's riders: a shield of a share of the full heal
        (Coagulation), a burst of what it heals past the maximum at the mobs within its yards
        (Dark Surplus) and a next-cast aura (Sanguine Flow)."""
        sk = self.skill("dark_mending")
        self.cd("mending", sk["realm_cooldown_ms"])
        self.health_cost(sk)
        self.gcd = self.t + 1.5
        p = fx(sk, "heal")
        before = self.hp
        full = self.heal(self.mending_amount(), "dark_mending")
        healed = self.hp - before
        ticks = self.toll_aura["duration_ms"] // self.toll["tick_ms"]
        self.mending_dot = [self.t + self.toll_aura["duration_ms"] / 1000, self.t + self.toll["tick_ms"] / 1000,
                            healed * self.toll["pct"] // 100 // ticks]
        for a in fxs(sk, "absorb"):
            self.shield("dark_mending", (full if a["pct_of"] == "full_heal" else healed) * a["pct"] // 100, a["duration_ms"])
        if p.get("overheal_burst_yd") and full > healed and self.dist <= p["overheal_burst_yd"]:
            o = self.spell_roll(self.spell_crit())
            if o != "miss":
                self.hit_mob(full - healed, "dark_surplus", False, o == "crit")
        for g in fxs(sk, "grant_aura"):
            self.grant(g["aura"])

    def siphon(self):
        sk = self.skill("siphon_life")
        self.spend_mana(self.cost(sk))
        self.channel = [self.t + sk["channel_ms"] / 1000, self.t + sk["channel_tick_ms"] / 1000]
        self.gcd = self.t + 1.5

    def drain(self):
        """A Siphon Life tick: the drain and its heal, a shield of what it heals past the maximum
        (Gorging Siphon), and on a chance a stack on the target's Blight (Feeding Siphon)."""
        s = self.s
        sk = self.skill("siphon_life")
        self.channel[1] += sk["channel_tick_ms"] / 1000
        p = fx(sk, "drain")
        o = self.spell_roll(self.spell_crit())
        if o == "miss":
            return
        d = self.hit_mob(linear(p["amount"], self.L), "siphon", False, o == "crit")
        before = self.hp
        asked = self.heal(d * p["heal_pct"] // 100, "siphon")
        if p.get("overheal_absorb_max_pct") and asked > self.hp - before:
            cap = s.max_hp * p["overheal_absorb_max_pct"] // 100
            self.grow_shield("siphon_life", asked - (self.hp - before), cap, p["overheal_absorb_ms"])
        if p.get("tick_grow_aura") and self.rng.random() * 100 < p["tick_grow_chance"]:
            self.grow(self.blight)

    def detonate(self):
        """The talent row's cast takes the target's Blight: its amount once a stack the Blight held,
        rolled for a crit but never a miss, and its share of the damage heals the Acolyte when it
        stands within its radius of the target (Necrotic Detonation)."""
        sk = self.talent_cast
        p = fx(sk, "consume_aura")
        self.cd("detonate", sk["realm_cooldown_ms"])
        self.gcd = self.t + 1.5
        stacks = self.blight[3] if p.get("per_stack") else 1
        self.blight = None
        o = "crit" if self.rng.random() * 100 < self.spell_crit() else "hit"
        raw = linear(p["amount"], self.L) * p.get("amount_pct", 100) // 100 * stacks
        d = self.hit_mob(raw, "detonation", False, o == "crit")
        if self.dist <= p.get("radius_yd", 0):
            self.heal(d * p["heal_allies_pct"] // 100, "detonation")

    def step(self):
        s, L = self.s, self.L
        # Ticks.
        if self.blighted(self.blight) and self.t >= self.blight[1]:
            self.tick_blight(self.blight)
        if self.in_cloud() and self.t >= self.miasma[1]:
            self.miasma[1] += self.cloud_dot["tick_ms"] / 1000
            self.hit_mob(self.miasma[2], "miasma", False)
        elif self.miasma and self.t >= self.miasma[1]:
            self.miasma[1] += self.cloud_dot["tick_ms"] / 1000
        # Every grow_every_ms the Blights of the mobs inside gain a stack (Breeding Ground).
        if self.cloud.get("grow_every_ms") and self.miasma and self.t < self.miasma[0] and self.t >= self.miasma[3]:
            self.miasma[3] += self.cloud["grow_every_ms"] / 1000
            if self.in_cloud():
                self.grow(self.blight)
                self.grow_others()
        if self.mending_dot and self.t < self.mending_dot[0] and self.t >= self.mending_dot[1]:
            self.mending_dot[1] += self.toll["tick_ms"] / 1000
            self.hp -= min(self.mending_dot[2], max(self.hp - 1, 0))
            self.costs += self.mending_dot[2]
        for h in self.lance_hots:
            if self.t < h[0] and self.t >= h[1]:
                h[1] += self.lance_hot["tick_ms"] / 1000
                self.heal(h[2], "hot")
        self.lance_hots = [h for h in self.lance_hots if self.t < h[0]]
        if self.maso and self.t < self.maso[0] and self.t >= self.maso[1]:
            self.maso[1] += self.maso[3]
            self.heal(self.maso[2], "masochism")
        # Channel ticks.
        if self.channel:
            if self.t >= self.channel[1]:
                self.drain()
            if self.t >= self.channel[0]:
                self.channel = None
        # Finish casts.
        if self.cast and self.t >= self.cast[0]:
            self.cast = None
            self.lance()
        in_melee = self.dist <= 5
        if not self.cast and not self.channel and self.t >= self.gcd:
            hp_frac = self.hp / s.max_hp
            siphon, blight, miasma = self.skill("siphon_life"), self.skill("creeping_blight"), self.skill("miasma")
            burst_yd = fx(self.skill("dark_mending"), "heal").get("overheal_burst_yd")
            over, mend_cost = self.surplus()
            det = self.talent_cast
            if hp_frac < 0.55 and self.ready("mending"):
                self.mend()
            elif det and self.ready("detonate") and self.blighted(self.blight) and (
                    self.blight[0] - self.blight[1] < DT
                    or hp_frac < 0.55 and self.dist <= fx(det, "consume_aura")["radius_yd"]):
                # Once the Blight has no tick left, or for the heal below Dark Mending's line.
                self.detonate()
            elif hp_frac < 0.45 and self.power >= self.cost(siphon):
                self.siphon()
            elif (fx(siphon, "drain").get("overheal_absorb_max_pct") and self.hp >= s.max_hp and not in_melee
                  and self.power >= self.cost(siphon)):
                # At full health with the mob on its way, the whole drain grows a shield (Gorging Siphon).
                self.siphon()
            elif burst_yd and self.ready("mending") and self.dist <= burst_yd and over >= mend_cost:
                # A burst that outdoes the health it costs (Dark Surplus).
                self.mend()
            elif not self.blighted(self.blight) and self.power >= self.cost(blight):
                self.spend_mana(self.cost(blight))
                self.gcd = self.t + 1.5
                if self.spell_roll(0) != "miss":
                    self.blight = self.new_blight()
            elif self.ready("miasma") and in_melee and self.power >= self.cost(miasma):
                self.spend_mana(self.cost(miasma))
                self.cd("miasma", miasma["realm_cooldown_ms"])
                self.gcd = self.t + 1.5
                self.miasma = [self.t + self.cloud["duration_ms"] / 1000, self.t + self.cloud_dot["tick_ms"] / 1000,
                               linear(self.cloud_dot["tick"], L), self.t + self.cloud.get("grow_every_ms", 0) / 1000]
            elif self.hp > s.max_hp * 0.25:
                self.start("blood_lance")
        # Mob.
        if self.dist > 5:
            self.dist = max(self.dist - MOB_RUN * DT, 0)
            self.mob_swing_t = max(self.mob_swing_t, self.t + 0.3)
        elif self.t >= self.mob_swing_t:
            self.mob_swing_t = self.t + self.m["swing"]
            d, o = self.mob_swing()
            if d > 0:
                self.take(d)

    def fight(self):
        self.costs = 0
        r = super().fight()
        r["costs"] = self.costs / self.s.max_hp
        return r


# ---------------------------------------------------------------- Blightcaller and Bloodmender
class Mending:
    """What the Tier 2 healers share, ahead of Sim: the mob runs in from 30 yd as the Acolyte's
    does; a heal on "the most hurt ally" lands on the caster, or on H-1's dummy friend
    (multi.Guard) when there is one; a heal split among the allies near splits between the two;
    heals over time; and the healing-taken bonus of the ground area the allies stand in."""

    def setup_mending(self):
        self.dist = 30.0
        self.mob_swing_t = 0.0
        self.gcd = 0.0
        self.cast = None      # when the cast lands
        self.channel = None   # [until, next tick]
        self.buffs = {}       # aura key -> [until, aura row, stacks, charges] on the caster
        self.hots = []        # [until, next, tick, every, who, source]
        self.costs = 0

    def friend_up(self):
        return getattr(self, "friend", None) is not None

    def who(self):
        """The most hurt ally: H-1's friend, which the mob swings at, else the caster."""
        return "friend" if self.friend_up() else "self"

    def allies(self):
        return ["self", "friend"] if self.friend_up() else ["self"]

    def health_of(self, who):
        return self.friend_hp / self.friend["hp"] if who == "friend" else self.hp / self.s.max_hp

    def in_melee(self):
        return self.dist <= 5

    def heal_bonus(self):
        """The percent more healing the allies take inside a ground area of the caster's."""
        return 0

    def heal_on(self, who, amount, src):
        """`amount` healed on `who`, more by heal_bonus. Returns what it healed."""
        amount = int(amount) * (100 + self.heal_bonus()) // 100
        if who == "friend":
            return self.heal_friend(amount)
        before = self.hp
        self.heal(amount, src)
        return self.hp - before

    def split(self, amount, src):
        """`amount` split evenly among the allies."""
        for who in self.allies():
            self.heal_on(who, amount // len(self.allies()), src)

    def hot(self, who, total, duration_ms, tick_ms, src):
        """`total` healed on `who` over `duration_ms`, a tick each `tick_ms`."""
        ticks = max(duration_ms // tick_ms, 1)
        self.hots.append([self.t + duration_ms / 1000, self.t + tick_ms / 1000, total // ticks, tick_ms / 1000, who, src])

    def hot_on(self, who, src):
        return any(h[4] == who and h[5] == src and self.t < h[0] for h in self.hots)

    def tick_hots(self):
        for h in self.hots:
            if self.t < h[0] and self.t >= h[1]:
                h[1] += h[3]
                self.heal_on(h[4], h[2], h[5])
        self.hots = [h for h in self.hots if self.t < h[0]]

    def mob_turn(self):
        if self.dist > 5:
            self.dist = max(self.dist - MOB_RUN * DT, 0)
            self.mob_swing_t = max(self.mob_swing_t, self.t + 0.3)
        elif self.t >= self.mob_swing_t:
            self.mob_swing_t = self.t + self.m["swing"]
            d, o = self.mob_swing()
            if d > 0:
                self.take(d)


class Blightcaller(Mending, Sim):
    """Creeping Blight on every mob; Harvest at HARVEST_AT stacks or more; Spread Blight on a pull
    of 2 or more once the target wears the Blight and another mob does not; Miasma and Wasting
    Communion on their cooldowns once the mobs are in melee; Plague Tithe as the filler. Hush is
    the Kick boss.py casts. Solo, every heal for the most hurt ally or the allies near reaches the
    caster; the Blight's leech always does."""
    HARVEST_AT = 8

    def setup_fight(self):
        self.setup_mending()
        c = self.calling
        self.blight = None      # [until, next tick, tick, stacks] on the target
        self.miasma = None      # the Miasma's cloud: dict(until, next, tick, every, heal_pct, grow_every, grow_next)
        self.communion = None   # Wasting Communion's: dict(until, next, tick, every, grow_every, grow_next)
        bf = fx(self.skill("creeping_blight"), "periodic_damage")
        self.blight_fx = bf
        self.blight_aura = aura(c, bf["apply_aura"])
        # A choice's own tick or leech (Deep Blight, Hungry Blight) replaces the aura's.
        self.blight_dot = {**fx(self.blight_aura, "periodic_damage"), **{k: bf[k] for k in ("tick", "leech_pct") if k in bf}}
        self.procs = passive_effects(c, self.rows, "proc")
        self.withering = sum(p["pct"] for p in passive_effects(c, self.rows, "damage_pct_vs_aura")
                             if p["aura"] == self.blight_aura["key"])

    def blighted(self, b):
        return b is not None and self.t < b[0]

    def shadow(self, raw, src, crit=False):
        """`raw` Shadow damage at the target, more on one wearing the Blight (Withering)."""
        more = self.withering if self.blighted(self.blight) else 0
        return self.hit_mob(raw, src, False, crit, dmg_pct=more)

    def new_blight(self):
        """A Creeping Blight of one stack from now: its tick with the duration share of the spell
        power, more by the build's tick pct (Blight Mastery)."""
        dur, every = self.blight_aura["duration_ms"], self.blight_dot["tick_ms"]
        tick = linear(self.blight_dot["tick"], self.L) + sp_periodic(self.s.spell_power, dur, dur // every)
        tick = tick * (100 + self.skill_pct("skill_amount_pct", "creeping_blight", "tick")) // 100
        return [self.t + dur / 1000, self.t + every / 1000, tick, 1]

    def grow(self, b, n=1):
        if self.blighted(b):
            b[3] = min(b[3] + n, self.blight_aura["max_stacks"])

    def grow_others(self, n=1):
        """The Blights on the other mobs of a pull gain `n` stacks: none in a fight with one."""

    def spread_blight(self, b, n):
        """Copies of Blight `b` on up to `n` other mobs that wear none: none in a fight with one."""
        return 0

    def bare_other(self):
        """Another living mob of a pull that wears no Blight: none in a fight with one."""
        return None

    def tick_blight(self, b):
        """Blight `b` on the target ticks: its damage once a stack, its leech on the caster, a share
        split among the allies (Shared Rot), a stack more, and on a chance a copy on another mob
        (Virulent Blight)."""
        dot = self.blight_dot
        b[1] += dot["tick_ms"] / 1000
        d = self.shadow(b[2] * b[3], "blight")
        self.heal_on("self", d * dot["leech_pct"] // 100, "blight_leech")
        for p in self.procs:
            if p["on"] == "periodic_tick" and p.get("skill") == "creeping_blight":
                self.split(d * p["heal_allies_pct"] // 100, "shared_rot")
        b[3] = min(b[3] + dot["stacks_per_tick"], self.blight_aura["max_stacks"])
        if self.blight_fx.get("tick_spread_chance") and self.rng.random() * 100 < self.blight_fx["tick_spread_chance"]:
            self.spread_blight(b, 1)

    def heal_bonus(self):
        m = self.miasma
        return m["heal_pct"] if m and self.t < m["until"] else 0

    def spend(self, key):
        """Skill `key` takes its mana and starts its cooldown and global cooldown."""
        self.spend_mana(self.cost(self.skill(key)))
        self.cd(key, self.cooldown_ms(key))
        self.gcd = self.t + self.gcd_of(key)

    def affords(self, key):
        return self.ready(key) and self.power >= self.cost(self.skill(key))

    def tithe_tick(self):
        """A Plague Tithe tick: its hit, more by the build's pct (Sustaining Rot), a heal over time of
        its pct of the hit on the most hurt ally, and stacks on the target's Blight."""
        sk = self.skill("plague_tithe")
        self.channel[1] += sk["channel_tick_ms"] / 1000
        o = self.spell_roll(self.s.crit)
        if o == "miss":
            return
        p = fx(sk, "school_damage")
        raw = (linear(p["amount"], self.L) + sp_direct(self.s.spell_power, 0)) * (
            100 + self.skill_pct("skill_amount_pct", "plague_tithe")) // 100
        d = self.shadow(raw, "plague_tithe", o == "crit")
        h = fx(sk, "periodic_heal")
        self.hot(self.who(), d * h["pct"] // 100, h["duration_ms"], h["tick_ms"], "plague_tithe")
        self.grow(self.blight, fx(sk, "grow_aura")["stacks"])

    def harvest(self):
        """Harvest takes the target's Blight: its amount once a stack, split as healing among the
        allies at its heal_allies_pct, then a fresh Blight on another mob with a spread_yd (Seeded
        Harvest)."""
        p = fx(self.skill("harvest"), "consume_aura")
        self.spend("harvest")
        o = self.spell_roll(self.s.crit)
        if o == "miss":
            return
        d = self.shadow(linear(p["amount"], self.L) * self.blight[3], "harvest", o == "crit")
        self.blight = None
        self.split(d * p["heal_allies_pct"] // 100, "harvest")
        if p.get("spread_yd"):
            i = self.bare_other()
            if i is not None:
                self.blights[i] = self.new_blight()

    def cloud(self):
        """The Miasma: its enemy aura's damage a tick on every mob inside, its ally aura's healing
        taken on the allies inside, and with a grow_every_ms a stack on the Blights inside
        (Breeding Miasma)."""
        g = fx(self.skill("miasma"), "ground_area")
        dot = fx(aura(self.calling, g["enemy_aura"]), "periodic_damage")
        heal_pct = sum(p["pct"] for p in fxs(aura(self.calling, g["ally_aura"]), "mod_healing_taken_pct"))
        self.spend("miasma")
        every = g.get("grow_every_ms", 0) / 1000
        self.miasma = dict(until=self.t + g["duration_ms"] / 1000, next=self.t + dot["tick_ms"] / 1000,
                           tick=linear(dot["tick"], self.L), every=dot["tick_ms"] / 1000, heal_pct=heal_pct,
                           grow_every=every, grow_next=self.t + every)

    def commune(self):
        """Wasting Communion: its ally aura's heal a tick on each ally inside, and a stack each
        grow_every_ms on every Blight inside."""
        g = fx(self.skill("wasting_communion"), "ground_area")
        h = fx(aura(self.calling, g["ally_aura"]), "periodic_heal")
        self.spend("wasting_communion")
        every = g["grow_every_ms"] / 1000
        self.communion = dict(until=self.t + g["duration_ms"] / 1000, next=self.t + h["tick_ms"] / 1000,
                              tick=linear(h["tick"], self.L), every=h["tick_ms"] / 1000, grow_every=every,
                              grow_next=self.t + every)

    def tick_areas(self):
        m, w = self.miasma, self.communion
        if m and self.t < m["until"]:
            if self.t >= m["next"]:
                m["next"] += m["every"]
                self.shadow(m["tick"], "miasma")
            if m["grow_every"] and self.t >= m["grow_next"]:
                m["grow_next"] += m["grow_every"]
                self.grow(self.blight)
                self.grow_others()
        if w and self.t < w["until"]:
            if self.t >= w["next"]:
                w["next"] += w["every"]
                for who in self.allies():
                    self.heal_on(who, w["tick"], "wasting_communion")
            if self.t >= w["grow_next"]:
                w["grow_next"] += w["grow_every"]
                self.grow(self.blight)
                self.grow_others()

    def step(self):
        self.tick_hots()
        self.tick_areas()
        if self.blighted(self.blight) and self.t >= self.blight[1]:
            self.tick_blight(self.blight)
        if self.channel:
            if self.t >= self.channel[1]:
                self.tithe_tick()
            if self.t >= self.channel[0]:
                self.channel = None
        if not self.channel and self.t >= self.gcd:
            b = self.blight
            if not self.blighted(b) and self.affords("creeping_blight"):
                self.spend("creeping_blight")
                if self.spell_roll(0) != "miss":
                    self.blight = self.new_blight()
            elif self.blighted(b) and b[3] >= self.HARVEST_AT and self.affords("harvest"):
                self.harvest()
            elif self.blighted(b) and self.bare_other() is not None and self.affords("spread_blight"):
                self.spend("spread_blight")
                self.spread_blight(b, fx(self.skill("spread_blight"), "copy_aura")["targets"])
            elif self.bare_other() is not None and self.affords("creeping_blight"):
                self.spend("creeping_blight")
                if self.spell_roll(0) != "miss":
                    self.blights[self.bare_other()] = self.new_blight()
            elif self.in_melee() and self.affords("miasma"):
                self.cloud()
            elif self.in_melee() and self.affords("wasting_communion"):
                self.commune()
            elif self.affords("plague_tithe"):
                sk = self.skill("plague_tithe")
                self.spend("plague_tithe")
                self.channel = [self.t + sk["channel_ms"] / 1000, self.t + sk["channel_tick_ms"] / 1000]
        self.mob_turn()


class Bloodmender(Mending, Sim):
    """Dark Mending, else Transfusion, on the most hurt ally under MEND_BELOW health while the
    Bloodmender's health stays above the cost; Blood Feast under FEAST_BELOW health; Blood Ward
    and Circle of Blood on their cooldowns once the mob is in melee; Blood Lance as the damage.
    Hush is the Kick boss.py casts. Every health cost fires the build's on-cast procs: Masochism's
    heal over time, Sanguine Flow's instant Blood Lance (a Lance costs health, so each one makes
    the next instant), Bloodsurge's mana. self.fight_log holds (time, "blood_lance", instant) for
    each Lance started."""
    MEND_BELOW = 0.40
    FEAST_BELOW = 0.60

    def setup_fight(self):
        self.setup_mending()
        self.shields = {}    # skill key -> [left, until] on the caster, first up first
        self.tolls = []      # [until, next, tick, every, who]: Dark Mending's toll
        self.circle = None   # [until, next, tick, every]
        self.procs = passive_effects(self.calling, self.rows, "proc")

    def health_cost(self, key):
        return self.s.max_hp * self.skill(key)["realm_cost"] // 100

    def affords(self, key):
        """Skill `key` is off its cooldown and its health cost leaves the Bloodmender alive (cast.rs
        refuses a health cost that would kill)."""
        return self.ready(key) and self.hp > self.health_cost(key)

    def pay_health(self, key):
        """Skill `key`'s health cost, and the on-cast procs of a health cost. Returns the cost."""
        cost = self.health_cost(key)
        self.hp -= cost
        self.costs += cost
        for p in self.procs:
            if p["on"] != "cast" or p.get("cost_kind") != "health" or not cost:
                continue
            if p.get("apply_aura"):
                a = aura(self.calling, p["apply_aura"])
                h = fxs(a, "periodic_heal")
                if h:
                    self.hot("self", cost * h[0]["pct"] // 100, a["duration_ms"], h[0]["tick_ms"], a["key"])
                else:
                    self.grant(a["key"])
            if p.get("energize_pct_of_base"):
                self.power = min(self.power + self.pct_base(p["energize_pct_of_base"]), self.s.max_mana)
        return cost

    def start(self, key):
        self.cd(key, self.cooldown_ms(key))
        self.gcd = self.t + self.gcd_of(key)

    def start_lance(self):
        mods = self.next_casts("blood_lance")
        self.use_next_casts("blood_lance")
        instant = any(p.get("instant") for p in mods)
        self.fight_log.append((self.t, "blood_lance", instant))
        self.cast = self.t + (0 if instant else self.skill("blood_lance")["cast_time_ms"] / 1000)
        self.gcd = self.t + self.gcd_of("blood_lance")

    def lance(self):
        """Blood Lance lands: its health cost, the hit, and a heal over time of its pct of the hit on
        the most hurt ally within its yards of the target: the friend beside the mob, or the caster
        once the mob is that near."""
        sk = self.skill("blood_lance")
        self.pay_health("blood_lance")
        o = self.spell_roll(self.s.crit)
        if o == "miss":
            return
        p = fx(sk, "school_damage")
        d = self.hit_mob(linear(p["amount"], self.L) + sp_direct(self.s.spell_power, sk["cast_time_ms"]),
                         "blood_lance", False, o == "crit")
        h = fx(sk, "periodic_heal")
        who = self.who() if self.friend_up() or self.dist <= h["radius_yd"] else None
        if who:
            self.hot(who, d * h["pct"] // 100, h["duration_ms"], h["tick_ms"], "blood_lance")

    def drink(self):
        """A Blood Feast tick: the drain, its heal_pct of it on the caster, and a shield of what it
        heals past the maximum (Sated Feast)."""
        sk = self.skill("blood_feast")
        self.channel[1] += sk["channel_tick_ms"] / 1000
        p = fx(sk, "drain")
        o = self.spell_roll(self.s.crit)
        if o == "miss":
            return
        d = self.hit_mob(linear(p["amount"], self.L), "blood_feast", False, o == "crit")
        asked = d * p["heal_pct"] // 100
        healed = self.heal_on("self", asked, "blood_feast")
        if p.get("overheal_absorb_max_pct") and asked > healed:
            w = self.shields.get("blood_feast")
            left = w[0] if w and self.t < w[1] else 0
            cap = self.s.max_hp * p["overheal_absorb_max_pct"] // 100
            self.shield("self", "blood_feast", min(left + asked - healed, cap), p["overheal_absorb_ms"])

    def shield(self, who, key, amount, ms):
        if who == "friend":
            self.shield_friend(amount, ms)
        elif amount > 0:
            self.shields[key] = [amount, self.t + ms / 1000]

    def take(self, dmg):
        for key, w in list(self.shields.items()):
            soaked = min(w[0], dmg) if self.t < w[1] else 0
            w[0] -= soaked
            dmg -= soaked
            if not w[0] or self.t >= w[1]:
                del self.shields[key]
        return Sim.take(self, dmg)

    def mend(self, who):
        """Dark Mending on `who`: its health cost, the heal, more by the build's pct (Crimson Gift),
        and its toll on `who`: its pct of what it healed over the toll aura's ticks."""
        sk = self.skill("dark_mending")
        self.start("dark_mending")
        self.pay_health("dark_mending")
        amount = (linear(fx(sk, "heal")["amount"], self.L) + sp_direct(self.s.spell_power, 0)) * (
            100 + self.skill_pct("skill_amount_pct", "dark_mending")) // 100
        healed = self.heal_on(who, amount, "dark_mending")
        a = aura(self.calling, fx(sk, "periodic_damage")["apply_aura"])
        toll = fx(a, "periodic_damage")
        ticks = a["duration_ms"] // toll["tick_ms"]
        self.tolls = [x for x in self.tolls if x[4] != who]
        self.tolls.append([self.t + a["duration_ms"] / 1000, self.t + toll["tick_ms"] / 1000,
                           healed * toll["pct"] // 100 // ticks, toll["tick_ms"] / 1000, who])

    def transfuse(self, who):
        """Transfusion on `who`: its health cost, and its aura's pct of the cost healed over the
        aura's duration, more by the build's pct (Crimson Gift)."""
        sk = self.skill("transfusion")
        self.start("transfusion")
        cost = self.pay_health("transfusion")
        a = aura(self.calling, fx(sk, "periodic_heal")["apply_aura"])
        h = fx(a, "periodic_heal")
        total = cost * h["pct"] // 100 * (100 + self.skill_pct("skill_amount_pct", "transfusion")) // 100
        self.hot(who, total, a["duration_ms"], h["tick_ms"], "transfusion")

    def ward(self):
        """Blood Ward: its health cost, and a shield of its pct of the cost on every ally."""
        p = fx(self.skill("blood_ward"), "absorb")
        self.start("blood_ward")
        cost = self.pay_health("blood_ward")
        for who in self.allies():
            self.shield(who, "blood_ward", cost * p["pct"] // 100, p["duration_ms"])

    def draw_circle(self):
        """Circle of Blood: its health cost, and its ally aura's heal a tick on every ally inside."""
        g = fx(self.skill("circle_of_blood"), "ground_area")
        h = fx(aura(self.calling, g["ally_aura"]), "periodic_heal")
        self.start("circle_of_blood")
        self.pay_health("circle_of_blood")
        self.circle = [self.t + g["duration_ms"] / 1000, self.t + h["tick_ms"] / 1000, linear(h["tick"], self.L),
                       h["tick_ms"] / 1000]

    def tick_tolls(self):
        for x in self.tolls:
            if self.t < x[0] and self.t >= x[1]:
                x[1] += x[3]
                if x[4] == "friend":
                    self.hurt_friend(x[2])
                else:
                    self.hp -= min(x[2], max(self.hp - 1, 0))
                    self.costs += x[2]
        self.tolls = [x for x in self.tolls if self.t < x[0]]
        c = self.circle
        if c and self.t < c[0] and self.t >= c[1]:
            c[1] += c[3]
            for who in self.allies():
                self.heal_on(who, c[2], "circle_of_blood")

    def step(self):
        s = self.s
        self.tick_hots()
        self.tick_tolls()
        if self.channel:
            if self.t >= self.channel[1]:
                self.drink()
            if self.t >= self.channel[0]:
                self.channel = None
        if self.cast is not None and self.t >= self.cast:
            self.cast = None
            self.lance()
        if self.cast is None and not self.channel and self.t >= self.gcd:
            who = self.who()
            low = self.health_of(who) < self.MEND_BELOW
            feast = self.skill("blood_feast")
            if low and self.affords("dark_mending"):
                self.mend(who)
            elif low and not self.hot_on(who, "transfusion") and self.affords("transfusion"):
                self.transfuse(who)
            elif self.hp < s.max_hp * self.FEAST_BELOW and self.power >= self.cost(feast):
                self.spend_mana(self.cost(feast))
                self.gcd = self.t + self.gcd_of("blood_feast")
                self.channel = [self.t + feast["channel_ms"] / 1000, self.t + feast["channel_tick_ms"] / 1000]
            elif self.in_melee() and self.affords("blood_ward"):
                self.ward()
            elif self.in_melee() and self.affords("circle_of_blood"):
                self.draw_circle()
            elif self.affords("blood_lance"):
                self.start_lance()
        self.mob_turn()


# ---------------------------------------------------------------- Basic Warrior
class BasicWarrior(Sim):
    """Tier 0, levels 1 to 9: Charge in, Fire Blast and Gouge on their cooldowns, Heroic Strike on
    the next swing as the rage dump. The kit has no Battle Shout or Rend, and no talents."""
    HS_DUMP_RAGE = 30

    def setup_fight(self):
        self.swing_t = 0.0
        self.mob_swing_t = 0.5
        self.gcd = 0.0
        self.hs_queued = False
        ch = skill("basic_warrior", "charge")
        if self.ready("charge"):
            self.cd("charge", ch["realm_cooldown_ms"])
            self.power = min(self.power + linear(fx(ch, "energize")["amount"], self.L), 100)
            stun = aura("basic_warrior", "charge_stun")["duration_ms"] / 1000
            self.mob_swing_t = max(self.mob_swing_t, self.t + stun)

    def rage_dealt(self, d):
        self.power = min(self.power + d / rage_conv(self.L) * 7.5 * 2.0, 100)

    def rage_taken(self, d):
        self.power = min(self.power + d / rage_conv(self.L) * 2.5 * 2.0, 100)

    def step(self):
        s, L = self.s, self.L
        if self.t >= self.gcd:
            fb = skill("basic_warrior", "fire_blast")
            gouge = skill("basic_warrior", "gouge")
            if self.ready("fire_blast") and self.power >= fb["realm_cost"]:
                self.power -= fb["realm_cost"]
                self.cd("fire_blast", fb["realm_cooldown_ms"])
                self.gcd = self.t + 1.5
                o = self.spell_roll(s.crit)
                if o != "miss":
                    self.hit_mob(linear(fx(fb, "school_damage")["amount"], L), "fire_blast", False, o == "crit")
            elif self.ready("gouge") and self.power >= gouge["realm_cost"]:
                self.power -= gouge["realm_cost"]
                self.cd("gouge", gouge["realm_cooldown_ms"])
                self.gcd = self.t + 1.5
                o = self.yellow(s.crit)
                if o in ("hit", "crit"):
                    self.hit_mob(linear(fx(gouge, "school_damage")["amount"], L), "gouge", True, o == "crit")
                    # Incapacitated until the next blow lands, the next white swing at the latest.
                    held = aura("basic_warrior", "gouge_incapacitate")["duration_ms"] / 1000
                    self.mob_swing_t = max(self.mob_swing_t, min(self.t + held, self.swing_t))
        hs = skill("basic_warrior", "heroic_strike")
        if not self.hs_queued and self.power >= self.HS_DUMP_RAGE:
            self.hs_queued = True
        if self.t >= self.swing_t:
            self.swing_t = self.t + s.mh_speed
            if self.hs_queued and self.power >= hs["realm_cost"]:
                self.hs_queued = False
                self.power -= hs["realm_cost"]
                o = self.yellow(s.crit)
                if o in ("hit", "crit"):
                    raw = self.rng.uniform(s.mh_min, s.mh_max) + linear(fx(hs, "weapon_strike")["bonus"], L)
                    self.hit_mob(raw, "heroic_strike", True, o == "crit")
            else:
                self.hs_queued = False
                d, o = self.white()
                if o in ("hit", "crit"):
                    self.rage_dealt(d)
        if self.t >= self.mob_swing_t:
            self.mob_swing_t = self.t + self.m["swing"]
            d, o = self.mob_swing()
            if d > 0:
                d = self.take(d)
                self.rage_taken(d)


SIMS = dict(basic_warrior=BasicWarrior, vanguard=Vanguard, cutthroat=Cutthroat, initiate=Initiate, ranger=Ranger,
            acolyte=Acolyte, ironclad=Ironclad, zealot=Zealot, blightcaller=Blightcaller, bloodmender=Bloodmender)

# ---------------------------------------------------------------- levels and Talent Rows
def tier_cap(tier):
    return max(((c.get("tier_cap") or 0) for c in SPELLS.values() if c.get("tier") == tier), default=0)


def power_level(tier, level):
    """rules/stats.rs power_level: a level inside `tier` mapped onto the Tier's band."""
    cap = tier_cap(tier)
    floor = 1 if tier == 0 else tier_cap(tier - 1)
    if cap <= 1 or floor == 0 or level == 0:
        return level
    span, denom = cap - floor, cap - 1
    return (floor * denom + (level - 1) * span + denom // 2) // denom


def calling_level(calling, level):
    """The level a character of Power Level `level` has in its Calling: Advancement starts a
    Calling at 1, and its Power Level runs over its Tier's band (10 to 30 for Tier 1, 30 to 50 for
    Tier 2): the lowest real level the realm gives that Power Level. At the band's top and past
    it the Calling is at its cap (Tier 2's real 49 and 50 are both Power Level 50)."""
    tier = SPELLS[calling].get("tier") or 0
    if tier == 0:
        return level
    cap = tier_cap(tier)
    if level >= cap:
        return cap
    return next((cl for cl in range(1, cap + 1) if power_level(tier, cl) >= level), cap)


def open_rows(calling, level):
    """The Talent Rows a Tier 1 or 2 `calling` at Power Level `level` has open: each whose `level` is at
    most its real level in the Calling. A Tier 0 character's rows stay empty (docs/balance/rules.md)."""
    if not SPELLS[calling].get("tier"):
        return []
    real = calling_level(calling, level)
    return [r for r in talent_rows(calling) if r["level"] <= real]


def character(calling, level, tier, rows={}):
    """`calling` at Power Level `level` in a `tier` kit, in the build `rows` ({row key: choice key})."""
    entries, off, fell_back, empty = kit(calling, level, tier)
    stats = build_stats(calling, level, entries, off, rows)
    return dict(stats=stats, rows=dict(rows), entries=entries, off=off, fell_back=fell_back, empty=empty)


def describe_kit(entries, off):
    names = [GEAR[e]["name"] for e in entries]
    if off:
        names.append(GEAR[off]["name"] + " (OH)")
    return ", ".join(names)
