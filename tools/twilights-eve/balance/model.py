#!/usr/bin/env python3
"""The balance model: one Basic Warrior or Tier 1 Calling against one creature row.

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
from collections import defaultdict

REPO = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))

DT = 0.05
FIGHT_CAP_S = 240.0    # a fight still going after this is lost
MOB_RUN = 7.0 * 1.143
PLAYER_RUN = 7.0
DROPLET_PICKUP = 0.5   # share of blood droplets the Cutthroat steps on
AMMO_DPS = 1.5         # Rough Arrow

CLASS_ID = {"warrior": 1, "hunter": 3, "rogue": 4, "priest": 5, "mage": 8}
CALLINGS = ("cutthroat", "ranger", "initiate", "vanguard", "acolyte")


class DataRoot:
    """Every file the model reads, under one root that holds data/twilights-eve/ and
    crates/twilight-realm/content/: the repo, or a scratch copy a proposal edits (#489)."""

    def __init__(self, root=REPO):
        self.root = os.path.abspath(root)
        self.data = os.path.join(self.root, "data", "twilights-eve")
        self.content = os.path.join(self.root, "crates", "twilight-realm", "content")
        self.spells = {c["key"]: c for c in self.json("server_spells.json")}
        self.tables = {k: self.json(f"{k}_level_stats.json") for k in CLASS_ID}
        self.gear = {int(r["entry"]): r for r in self.csv("gear.csv")}
        self.callings_csv = {r["calling"]: r for r in self.csv("callings.csv")}
        self.talent_row = {(r["calling"], r["key"]): (r["tab"], int(r["tab_row"]))
                           for r in self.csv("master.csv") if r["kind"] == "talent"}
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
    global DATA, SPELLS, TABLES, GEAR, OUTFITS, CALLINGS_CSV, TALENT_ROW
    DATA = DataRoot(root)
    SPELLS, TABLES, GEAR = DATA.spells, DATA.tables, DATA.gear
    OUTFITS, CALLINGS_CSV, TALENT_ROW = DATA.outfits, DATA.callings_csv, DATA.talent_row
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


def wdps(r):
    sp = fnum(r["speed"])
    return (fnum(r["dmg_min"]) + fnum(r["dmg_max"])) / 2 / sp if sp else 0


def score(calling, r):
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
    row a slot), green (the best Uncommon) or blue (the best Rare, else the green). Returns
    (entries, off_hand, fell_back): off_hand the off-hand weapon's entry or None, fell_back the
    slots of a blue kit that wear a green because no Rare row is usable there."""
    outfit = tuple(e for e in OUTFITS.get(calling, {}).get("items", []) if e in GEAR) if tier == "white" else ()

    def best(slot, pred=lambda r: True):
        cands = pick(calling, level, (QUALITY[tier],), slot, pred, outfit)
        if not cands and tier == "blue":
            cands = pick(calling, level, ("Uncommon",), slot, pred)
        return cands

    chosen = []
    chest = best("Chest", lambda r: r["type"] in ARMOR_ALLOWED[calling])
    if chest:
        chosen.append(int(chest[0]["entry"]))
    neck = best("Amulet")
    if neck:
        chosen.append(int(neck[0]["entry"]))
    rings = best("Ring")
    chosen += [int(r["entry"]) for r in rings[:2]]
    off = None
    if calling == "cutthroat":
        mh = best("Weapon", lambda r: r["hand"] in ("MH", "1H") and fnum(r["speed"]) >= 2.0)
        oh = best("Weapon", lambda r: r["hand"] == "1H" and fnum(r["speed"]) <= 1.9)
        if mh:
            chosen.append(int(mh[0]["entry"]))
        if oh:
            off = int(oh[0]["entry"])
    elif calling in ("vanguard", "basic_warrior"):
        mh = best("Weapon", lambda r: r["hand"] in ("MH", "1H"))
        sh = best("Offhand", lambda r: r["type"] == "Shield")
        if mh:
            chosen.append(int(mh[0]["entry"]))
        if sh:
            chosen.append(int(sh[0]["entry"]))
    elif calling == "ranger":
        mh = best("Weapon", lambda r: r["hand"] in ("MH", "1H", "2H"))
        bow = best("Ranged", lambda r: r["type"] in ("Bow", "Gun", "Crossbow"))
        if mh:
            chosen.append(int(mh[0]["entry"]))
        if bow:
            chosen.append(int(bow[0]["entry"]))
    else:
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
    return chosen, off, fell_back



# ---------------------------------------------------------------- stats
class Stats:
    pass


def build_stats(calling, level, entries, off_entry, talents):
    c = SPELLS[calling]
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
    # Talents on stats.
    stat_pct = dict(str=0, agi=0, sta=0, int=0, spi=0)
    for key, rank in talents.items():
        if key == "divine_strength":
            stat_pct["str"] += 2 * rank
        if key == "vitality":
            stat_pct["sta"] += 2 * rank
    names = ["str", "agi", "sta", "int", "spi"]
    tot = {}
    for i, n in enumerate(names):
        v = base[i] + gear[n]
        tot[n] = int(v * (100 + stat_pct[n]) / 100)
    s.str, s.agi, s.sta, s.int, s.spi = (tot[n] for n in names)
    s.base_health = per["base_health"]
    s.max_hp = s.base_health + min(s.sta, 20) + max(s.sta - 20, 0) * 10
    if "vitality" in talents:
        s.max_hp = s.max_hp * (100 + 2 * talents["vitality"]) // 100
    bm = per["base_mana"]
    s.base_mana = bm
    s.max_mana = 0 if bm == 0 else bm + min(s.int, 20) + max(s.int - 20, 0) * 15
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
    s.crit += talents.get("reflexive_strike", 0)
    s.dodge += talents.get("reflexive_strike", 0)
    s.parry += talents.get("reactive_strike", 0)
    s.spell_power = gear["spell_power"]
    s.armor_items = gear["armor"]
    s.armor_items_pct = 2 * talents.get("tempered_steel", 0)
    s.armor = 2 * s.agi + gear["armor"]
    s.defense = 5 * level
    s.block_pct = 0.0
    s.block_value = 0
    s.shield = gear["block"]
    if gear["block"] is not None:
        s.block_pct = 5.0 + talents.get("shield_specialization", 0)
        s.block_value = max(int(gear["block"] + s.str / 20.0 - 1), 0)
        s.block_value = s.block_value * (100 + 5 * talents.get("iron_retribution", 0)) // 100
    s.armor_pen = 50 * talents.get("serrated_strikes", 0)
    s.max_energy = 100 + 5 * talents.get("predators_step", 0)
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
    """docs/balance/rules.md: Northshire at 10 and below, Elwynn Forest from 11."""
    return NORTHSHIRE_VALLEY if level <= 10 else ELWYNN_FOREST


def mob_stats(level, unit_class="warrior", rank=0, area=None, health_mul=1.0, damage_mul=1.0,
              armor_mul=1.0, swing_s=2.0, name=None):
    """A creature as Mob::stats_for makes it: its unit class's level-10 vanilla row x P(level) x
    [difficulty] health and damage_at(level) x the Tier Step x its rank's factors x the Zone
    Factor x the template's own multipliers; armour the vanilla row's at its level. `area` None
    is the on-level area."""
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
                dmax=avg * (1 + DAMAGE_VARIANCE), swing=swing_s)


# ---------------------------------------------------------------- skills
def skill(calling, key):
    return next(s for s in SPELLS[calling]["skills"] if s["key"] == key)


def fx(sk, kind):
    return next(e["params"] for e in sk["effects"] if e["kind"] == kind)


def aura(calling, key):
    return next(a for a in SPELLS[calling]["auras"] if a["key"] == key)


# ---------------------------------------------------------------- sim core
class Sim:
    def __init__(self, calling, level, s, talents, rng, opts, mob=None):
        self.calling, self.L, self.s, self.T, self.rng, self.opts = calling, level, s, talents, rng, opts
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

    def heal(self, amount, src):
        amount = int(amount)
        before = self.hp
        self.hp = min(self.hp + amount, self.s.max_hp)
        self.healed += self.hp - before

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
            d = armored(raw, max(self.m_armor - self.s.armor_pen - armor_ignore, 0), self.L)
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

    # ---- mob swing
    def mob_swing(self, from_front=True):
        s = self.s
        raw = self.rng.uniform(self.m["dmin"], self.m["dmax"])
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

    def mob_miss_more(self):
        return 0

    def dodge_more(self):
        return 0

    def block_pct(self):
        return self.s.block_pct

    def block_value(self):
        return self.s.block_value

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
        self.seal_armor_until = -1
        self.sop_until = -1
        self.ground_until = -1
        self.ground_next = 0
        self.smash_queued = False
        self.power = min(self.power, 100)
        # Bulwark Charge opener: 15 rage and the ground effect under the mob.
        if self.ready("charge"):
            self.cd("charge", 15000)
            self.power = min(self.power + 15, 100)
            ch = skill("vanguard", "bulwark_charge")
            g = fx(ch, "ground_effect")
            hg = self.T.get("hallowed_ground", 0)
            self.ground_dmg = linear(g["amount"], self.L) * (100 + 15 * hg) // 100
            self.ground_until = self.t + (g["duration_ms"] + 2000 * hg) / 1000
            self.ground_next = self.t + 1.0

    def player_armor(self):
        pct = self.s.armor_items_pct + (20 if self.t < self.seal_armor_until else 0)
        return 2 * self.s.agi + self.s.armor_items * (100 + pct) // 100

    def block_pct(self):
        return self.s.block_pct + (50 if self.t < self.sop_until else 0)

    def rage_dealt(self, d):
        self.power = min(self.power + d / rage_conv(self.L) * 7.5 * 2.0, 100)

    def rage_taken(self, d):
        self.power = min(self.power + d / rage_conv(self.L) * 2.5 * 2.0, 100)

    def melee_hit_procs(self):
        if self.t < self.seal_until:
            if self.rng.random() < 0.25:
                self.heal(self.s.max_hp * 4 // 100, "seal")
            if self.rng.random() < 0.25:
                self.seal_armor_until = self.t + 10
            wi = self.T.get("wrathful_iron", 0)
            if wi and self.rng.random() * 100 < 10 * wi:
                raw = self.rng.uniform(self.s.mh_min, self.s.mh_max) * 0.25
                self.hit_mob(raw, "wrathful_iron", physical=False)

    def step(self):
        s, T = self.s, self.T
        L = self.L
        if self.conviction and self.t > self.conv_until:
            self.conviction = 0
        # Ground effect ticks.
        if self.t < self.ground_until and self.t >= self.ground_next:
            self.ground_next += 1.0
            self.hit_mob(self.ground_dmg, "consecration", physical=False)
        # Instants.
        if self.t >= self.gcd:
            if self.t >= self.seal_until and self.power >= 10 - 2 * T.get("spiritual_focus", 0):
                self.power -= 10 - 2 * T.get("spiritual_focus", 0)
                self.seal_until = self.t + 30
                self.gcd = self.t + 1.5
            elif self.ready("sop") and self.power >= max(10 - 5 * T.get("improved_shield_of_penitent", 0) - 2 * T.get("spiritual_focus", 0), 0) and s.shield is not None:
                self.power -= max(10 - 5 * T.get("improved_shield_of_penitent", 0) - 2 * T.get("spiritual_focus", 0), 0)
                self.cd("sop", 15000)
                self.sop_until = self.t + 10
            elif T.get("hammer_of_vanguard") and self.ready("hammer") and self.power >= 30 and self.t < self.ground_until:
                self.power -= 30
                self.cd("hammer", 6000)
                o = self.yellow(s.crit)
                if o in ("hit", "crit"):
                    raw = self.rng.uniform(s.mh_min, s.mh_max) * 1.5 + 10 + L
                    self.hit_mob(raw, "hammer", physical=False, crit=True)
                    self.melee_hit_procs()
            elif self.power >= 20 and (self.conviction >= 5 or (self.conviction >= 3 and self.power >= 60)):
                self.power -= 20
                self.gcd = self.t + 1.5
                o = self.yellow(s.crit + T.get("righteous_fury", 0))
                if o in ("hit", "crit"):
                    cl = skill("vanguard", "zealous_cleave")
                    raw = self.rng.uniform(s.mh_min, s.mh_max) * self.opts.get("cleave_pct", 100) / 100 + linear(fx(cl, "weapon_strike")["bonus"], L)
                    if T.get("iron_juggernaut"):
                        raw += self.block_value() * 50 * T["iron_juggernaut"] // 100
                    self.hit_mob(raw, "cleave", True, o == "crit")
                    self.melee_hit_procs()
                    stacks = self.conviction
                    self.conviction = 0
                    holy = int((5 + L) * self.opts.get("conv_mult", 1.0)) * stacks
                    d = self.hit_mob(holy, "conviction", physical=False)
                    zr = T.get("zealous_restoration", 0)
                    if zr:
                        self.heal(d * 25 * zr // 100, "zealous_restoration")
        # Queue Righteous Smash.
        if not self.smash_queued and self.power >= self.opts.get("smash_cost", 15):
            self.smash_queued = True
        # Swing.
        if self.t >= self.swing_t:
            self.swing_t = self.t + s.mh_speed
            if self.smash_queued and self.power >= self.opts.get("smash_cost", 15):
                self.smash_queued = False
                self.power -= self.opts.get("smash_cost", 15)
                o = self.yellow(s.crit + T.get("righteous_fury", 0))
                if o in ("hit", "crit"):
                    rs = skill("vanguard", "righteous_smash")
                    raw = self.rng.uniform(s.mh_min, s.mh_max) + linear(fx(rs, "weapon_strike")["bonus"], L) * self.opts.get("smash_mult", 1.0)
                    if T.get("iron_juggernaut"):
                        raw += self.block_value() * 50 * T["iron_juggernaut"] // 100
                    self.hit_mob(raw, "smash", True, o == "crit")
                    self.melee_hit_procs()
                    self.conviction = min(self.conviction + 1, 5)
                    self.conv_until = self.t + 15
            else:
                self.smash_queued = False
                d, o = self.white()
                if o in ("hit", "crit"):
                    self.rage_dealt(d)
                    self.melee_hit_procs()
                elif o in ("dodge", "parry"):
                    self.rage_dealt(int(self.rng.uniform(s.mh_min, s.mh_max) * 0.75 * (1 - armor_dr(self.m_armor, L))))
        # Mob swing.
        if self.t >= self.mob_swing_t:
            self.mob_swing_t = self.t + self.m["swing"]
            d, o = self.mob_swing()
            if o == "block":
                if self.t < self.sop_until:
                    self.hit_mob(12 + 1.2 * L, "penitent_thorns", physical=False)
                ir = T.get("iron_retribution", 0)
                if ir:
                    self.hit_mob(self.block_value() * ir, "iron_retribution", physical=False)
                ss = T.get("shield_specialization", 0)
                if ss and self.rng.random() * 100 < 20 * ss:
                    self.power = min(self.power + 2, 100)
            if d > 0:
                d = self.take(d)
                self.rage_taken(d)


# ---------------------------------------------------------------- Cutthroat
class Cutthroat(Sim):
    def dmg_pct(self):
        return 3 * self.bloodlust if self.t < self.bl_until else 0

    def haste(self):
        return 3 * self.bloodlust if self.t < self.bl_until else 0

    def setup_fight(self):
        s = self.s
        self.combo = 0
        self.swing_t = 0.0
        self.oh_t = s.oh_speed / 2 if s.oh else 1e9
        self.mob_swing_t = 0.5
        self.gcd = 0.0
        self.bloodlust = 0
        self.bl_until = -1
        self.rupture = None  # (until, next, tick, leech)
        self.garrote = None
        self.evasion_until = -1
        self.bleeds = []     # scent of blood: (until, next, tick)
        # Garrote from stealth.
        g = skill("cutthroat", "garrote")
        if self.power >= 40:
            self.power -= 40
            self.gcd = self.t + 1.0
            o = self.yellow(100)  # opener: a crit if it lands
            if o not in ("dodge", "parry", "miss"):
                a = aura("cutthroat", "aura_garrote")
                p = a["effects"][0]["params"]
                self.garrote = [self.t + a["duration_ms"] / 1000, self.t + p["tick_ms"] / 1000, linear(p["tick"], self.L), p["tick_ms"] / 1000]
                self.combo = 1

    def dodge_more(self):
        return 50 if self.t < self.evasion_until else 0

    def add_combo(self, n=1):
        self.combo = min(self.combo + n, 5)

    def droplet(self):
        if self.rng.random() < 0.5 and self.rng.random() < self.opts.get("pickup", DROPLET_PICKUP):
            self.bloodlust = min(self.bloodlust + 1, 5 + self.T.get("blood_frenzy", 0))
            self.bl_until = self.t + 10
            th = self.T.get("thirst", 0)
            if th:
                self.heal(self.s.max_hp * 2 * th // 100, "thirst")

    def on_crit(self, d):
        sb = self.T.get("scent_of_blood", 0)
        if sb:
            self.bleeds.append([self.t + 8, self.t + 2, d * 10 * sb // 100 // 4])

    def finisher_procs(self):
        ru = self.T.get("ruthlessness", 0)
        if ru and self.rng.random() * 100 < 33 * ru:
            self.add_combo(1)

    def on_kill(self):
        et = self.T.get("executioners_toll", 0)
        if et:
            self.heal(self.s.max_hp * (5 + 5 * et) // 100, "toll")
            self.power = min(self.power + 20 + 20 * et, self.s.max_energy)

    def step(self):
        s, T, L = self.s, self.T, self.L
        if self.t > self.bl_until:
            self.bloodlust = 0
        # Bleeds.
        for b in (self.rupture, self.garrote):
            if b and self.t < b[0] and self.t >= b[1]:
                b[1] += b[3]
                d = self.hit_mob(b[2], "rupture" if b is self.rupture else "garrote", physical=False, dmg_pct=self.dmg_pct())
                if b is self.rupture:
                    self.heal(d * 40 // 100, "rupture_leech")
                self.droplet()
        for b in self.bleeds:
            if self.t < b[0] and self.t >= b[1]:
                b[1] += 2
                self.hit_mob(b[2], "scent_of_blood", physical=False)
                self.droplet()
        self.bleeds = [b for b in self.bleeds if self.t < b[0]]
        # Evasion when hurt.
        if self.ready("evasion") and self.hp < s.max_hp * 0.6:
            self.cd("evasion", 120000)
            self.evasion_until = self.t + 15
        # GCD skills.
        if self.t >= self.gcd:
            ss_cost = 35 - 2 * T.get("flurry_of_blades", 0)
            mob_frac = self.mob_hp / self.m["hp"]
            rupture_up = self.rupture and self.t < self.rupture[0]
            if self.combo >= 5 or (self.combo >= 3 and mob_frac < 0.25):
                if not rupture_up and mob_frac > 0.45 and self.power >= 25:
                    self.power -= 25
                    self.gcd = self.t + 1.0
                    o = self.yellow(s.crit)
                    if o in ("hit", "crit"):
                        p = fx(skill("cutthroat", "rupture"), "periodic_damage")
                        bl = T.get("bloodletting", 0)
                        dur = (p["duration_ms"] + linear(p["duration_per_combo"], L) * self.combo + 2000 * bl) / 1000
                        tick = (linear(p["tick"], L) + linear(p["per_combo"], L) * self.combo) * (100 + 10 * bl) // 100
                        self.rupture = [self.t + dur, self.t + 2.0, tick, 2.0]
                    self.combo = 0
                    self.finisher_procs()
                elif self.power >= 35:
                    self.power -= 35
                    self.gcd = self.t + 1.0
                    o = self.yellow(s.crit)
                    if o in ("hit", "crit"):
                        p = fx(skill("cutthroat", "eviscerate"), "school_damage")
                        raw = linear(p["amount"], L) + linear(p["per_combo"], L) * self.combo
                        d = self.hit_mob(raw, "eviscerate", True, o == "crit", dmg_pct=self.dmg_pct())
                        if o == "crit":
                            self.on_crit(d)
                    self.combo = 0
                    self.finisher_procs()
            elif self.power >= ss_cost:
                self.power -= ss_cost
                self.gcd = self.t + 1.0
                o = self.yellow(s.crit)
                if o in ("hit", "crit"):
                    p = fx(skill("cutthroat", "sinister_strike"), "weapon_strike")
                    raw = self.rng.uniform(s.mh_min, s.mh_max) * p["weapon_pct"] / 100 + linear(p["bonus"], L)
                    d = self.hit_mob(raw, "sinister_strike", True, o == "crit", dmg_pct=self.dmg_pct())
                    if o == "crit":
                        self.on_crit(d)
                    self.add_combo(1)
                    srr = T.get("serrated_strikes", 0)
                    bleeding = (self.rupture and self.t < self.rupture[0]) or (self.garrote and self.t < self.garrote[0])
                    if srr and bleeding and self.rng.random() * 100 < 20 * srr:
                        self.add_combo(1)
        # Whites.
        h = self.haste()
        if self.t >= self.swing_t:
            self.swing_t = self.t + s.mh_speed * 100 / (100 + h)
            d, o = self.white("mh")
            if o == "crit":
                self.on_crit(d)
        if s.oh and self.t >= self.oh_t:
            self.oh_t = self.t + s.oh_speed * 100 / (100 + h)
            d, o = self.white("oh")
            if o == "crit":
                self.on_crit(d)
        # Mob.
        if self.t >= self.mob_swing_t:
            self.mob_swing_t = self.t + self.m["swing"]
            d, o = self.mob_swing()
            if o == "dodge":
                pg = T.get("phantom_guile", 0)
                if pg and self.rng.random() * 100 < 33 * pg:
                    self.add_combo(1)
                    self.power = min(self.power + 10, s.max_energy)
            if d > 0:
                self.take(d)


# ---------------------------------------------------------------- Initiate
class Initiate(Sim):
    def setup_fight(self):
        self.dist = 30.0
        self.mob_swing_t = 0.0
        self.gcd = 0.0
        self.cast = None   # (done_at, key)
        self.ember = None  # [until, next, tick, stacks]
        self.cold_until = -1
        self.slow_until = -1
        self.absorb = 0
        self.absorb_until = -1
        self.resonance_until = -1
        self.resonance_pct = 0
        self.momentum_until = -1
        self.substrate = 0
        self.attune = 0
        self.free_cold = False
        self.instant_cold = False
        self.ember_more_ms = 0

    def spell_dmg_pct(self):
        return self.resonance_pct if self.t < self.resonance_until else 0

    def cast_time(self, key):
        base = {"ley_spark": 1500, "biting_cold": 2000}.get(key, 0)
        if key == "biting_cold":
            if self.instant_cold:
                return 0
            base = max(base - 100 * self.substrate, 0)
        return base

    def start(self, key):
        self.cast = (self.t + self.cast_time(key) / 1000, key)
        self.gcd = self.t + 1.5

    def finish(self, key):
        s, T, L = self.s, self.T, self.L
        sp = s.spell_power
        crit = s.crit
        if key == "ley_spark":
            cost = self.pct_base(5)
            self.spend_mana(cost)
            lt = T.get("ley_line_tap", 0)
            if lt and self.rng.random() * 100 < 10 * lt:
                self.power = min(self.power + cost, s.max_mana)
            p = fx(skill("initiate", "ley_spark"), "school_damage")
            crit += p["crit_pct_at_full_mana"] * self.power / s.max_mana
            o = self.spell_roll(crit)
            if o != "miss":
                raw = linear(p["amount"], L)
                mom = 15 * T.get("arcane_momentum", 0) if self.t < self.momentum_until else 0
                raw = raw * (100 + mom) // 100 + sp_direct(sp, 1500)
                self.hit_mob(raw, "ley_spark", False, o == "crit", dmg_pct=self.spell_dmg_pct())
            self.substrate = T.get("arcane_substrate", 0)
            if T.get("ley_line_attunement"):
                self.attune += 1
                if self.attune >= 3:
                    self.attune = 0
                    self.free_cold = True
        elif key == "searing_ember":
            ve = T.get("volatile_embers", 0)
            self.spend_mana(self.pct_base(4) * (100 - 10 * ve) // 100)
            sk = skill("initiate", "searing_ember")
            o = self.spell_roll(crit)
            if o != "miss":
                raw = linear(fx(sk, "school_damage")["amount"], L) + sp_direct(sp, 0)
                self.hit_mob(raw, "ember_hit", False, o == "crit", dmg_pct=self.spell_dmg_pct())
                a = aura("initiate", "aura_searing_ember")
                p = a["effects"][0]["params"]
                dur = a["duration_ms"] + 1000 * self.substrate
                ticks = dur // p["tick_ms"]
                tick = linear(p["tick"], L) * (100 + 10 * ve) // 100 + sp_periodic(sp, dur, ticks) // 2
                stacks = min((self.ember[3] if self.ember and self.t < self.ember[0] else 0) + 1, 3)
                self.ember = [self.t + dur / 1000, self.t + 3.0, tick, stacks]
                self.substrate = 0
        elif key == "biting_cold":
            if self.free_cold or self.instant_cold:
                self.free_cold = False
                self.instant_cold = False
            else:
                cost = self.pct_base(8)
                fc = T.get("frigid_conservation", 0)
                if fc and self.t < self.slow_until:
                    cost = cost * (100 - 25 * fc) // 100
                self.spend_mana(cost)
            sk = skill("initiate", "biting_cold")
            o = self.spell_roll(crit)
            if o != "miss":
                raw = linear(fx(sk, "school_damage")["amount"], L) + sp_direct(sp, 2000)
                ce = T.get("combustion_engine", 0)
                if ce and self.ember and self.t < self.ember[0]:
                    raw = raw * (100 + 15 * ce) // 100
                self.hit_mob(raw, "biting_cold", False, o == "crit", dmg_pct=self.spell_dmg_pct())
                self.cold_until = self.t + 6
                self.slow_until = self.t + 6
            self.substrate = 0
        elif key == "catalyst":
            self.cd("catalyst", 12000)
            ts = T.get("thermal_shock", 0)
            if self.t < self.cold_until:
                self.cold_until = -1
                p = fx(skill("initiate", "catalyst"), "consume_aura")
                raw = linear(p["amount"], L) * 2 * (100 + 10 * ts) // 100
                crit_c = crit + (100 if self.t < getattr(self, "convergence_until", -1) else 0)
                o = "crit" if self.rng.random() * 100 < crit_c else "hit"
                self.hit_mob(raw, "catalyst", False, o == "crit", dmg_pct=self.spell_dmg_pct())
            if self.ember and self.t < self.ember[0]:
                self.ember = None
                self.power = min(self.power + s.max_mana * 10 * (100 + 10 * ts) // 10000, s.max_mana)
            cr = T.get("catalytic_resonance", 0)
            if cr:
                self.resonance_until = self.t + 8
                self.resonance_pct = 2 * cr

    def step(self):
        s, T, L = self.s, self.T, self.L
        # Ember ticks.
        if self.ember and self.t < self.ember[0] and self.t >= self.ember[1]:
            self.ember[1] += 3.0
            self.hit_mob(self.ember[2] * self.ember[3], "ember_dot", False, dmg_pct=self.spell_dmg_pct())
            tf = T.get("thermal_feedback", 0)
            if tf and self.rng.random() * 100 < 4 * tf:
                self.instant_cold = True
        if self.absorb and self.t > self.absorb_until:
            self.absorb = 0
        # Casting.
        if self.cast and self.t >= self.cast[0]:
            key = self.cast[1]
            self.cast = None
            self.finish(key)
        in_melee = self.dist <= 5
        if not self.cast and self.t >= self.gcd:
            if self.ready("ward") and self.power >= self.pct_base(10):
                self.spend_mana(self.pct_base(10))
                self.cd("ward", 30000)
                self.absorb = s.max_mana * self.opts.get("ward_pct", 10) // 100
                self.absorb_until = self.t + 20
            elif in_melee and self.ready("blink") and self.power >= self.pct_base(10) and self.t < self.slow_until:
                self.spend_mana(self.pct_base(10))
                self.cd("blink", 15000)
                self.dist = 20.0
                self.gcd = self.t + 1.5
                if T.get("arcane_momentum"):
                    self.momentum_until = self.t + 5
            elif self.ready("catalyst") and self.t < self.cold_until and (self.ember and self.t < self.ember[0]) and not self.opts.get("smart"):
                self.start("catalyst")
            elif T.get("elemental_convergence") and self.ready("convergence") and not (self.ember and self.t < self.ember[0]):
                self.cd("convergence", 30000)
                self.gcd = self.t + 1.5
                self.convergence_until = self.t + 6
                self.cold_until = self.t + 6
                self.slow_until = self.t + 6
                a = aura("initiate", "aura_searing_ember")
                p = a["effects"][0]["params"]
                self.ember = [self.t + 12, self.t + 3.0, linear(p["tick"], L), 1]
            elif not (self.ember and self.t < self.ember[0]) and self.power >= self.pct_base(4):
                self.start("searing_ember")
            elif self.t >= self.cold_until and (self.power >= self.pct_base(8) or self.free_cold or self.instant_cold) and ((not self.opts.get("smart") and (self.ready("catalyst") or self.cds["catalyst"] - self.t < 3)) or (in_melee and self.ready("blink"))):
                self.start("biting_cold")
            elif self.power >= self.pct_base(5):
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
        s = self.s
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
        self.rapid_until = -1
        self.dressing = None  # [until, next, tick]
        self.gt_stacks = 0
        self.gt_until = -1
        self.rd_stacks = 0
        self.flurry = 0
        self.sundered = 0
        self.sundered_until = -1

    def ranged_crit(self):
        c = self.s.crit
        sa = self.T.get("steadfast_aim", 0)
        if sa and not self.moving and self.t - self.still_since >= 2:
            c += 3 * sa
        return c

    def ranged_haste(self):
        h = 40 if self.t < self.rapid_until else 0
        sa = self.T.get("steadfast_aim", 0)
        if sa and not self.moving and self.t - self.still_since >= 2:
            h += 5 * sa
        return h

    def dmg_vs(self):
        sd = self.T.get("shattered_defenses", 0)
        return 5 * sd if sd and self.sundered and self.t < self.sundered_until else 0

    def mob_armor_now(self):
        return max(self.m["armor"] - (50 * self.T.get("jagged_barbs", 0) * self.sundered if self.t < self.sundered_until else 0), 0)

    def shoot(self, src, weapon_pct=100, bonus=0, armor_ignore=0, crit_bonus=0, extra_pct=0):
        s = self.s
        o = self.ranged_roll(self.ranged_crit() + (100 if src == "snapshot" and self.flurry else 0))
        if src == "snapshot" and self.flurry:
            self.flurry -= 1
        if o in ("miss", "dodge"):
            return 0, o
        raw = (self.rng.uniform(s.r_min, s.r_max) * weapon_pct / 100 + bonus) * (100 + extra_pct) / 100
        self.m_armor = self.mob_armor_now()
        d = self.hit_mob(raw, src, True, o == "crit", crit_bonus, armor_ignore, dmg_pct=self.dmg_vs())
        return d, o

    def start(self, key, ms):
        self.cast = (self.t + ms / 1000, key)
        self.gcd = self.t + 1.5

    def snapshot(self):
        s, T, L = self.s, self.T, self.L
        if not self.flurry:
            self.spend_mana(self.pct_base(5))
        self.gcd = self.t + 1.5
        p = fx(skill("ranger", "snapshot"), "weapon_strike")
        extra = 10 * T.get("guerilla_tactics", 0) * self.gt_stacks if self.t < self.gt_until else 0
        self.gt_stacks = 0
        bonus = linear(self.opts.get("snap_bonus", p["bonus"]), L)
        d, o = self.shoot("snapshot", self.opts.get("snap_pct", 100), bonus, extra_pct=extra)
        if o == "crit" and T.get("adrenaline_rush"):
            self.cds["vault"] = self.cds.get("vault", self.t) - 2 * T["adrenaline_rush"]
        if T.get("rhythmic_draw"):
            self.rd_stacks = min(self.rd_stacks + 1, 15)
        ta = T.get("twin_arrows", 0)
        if ta and self.rng.random() * 100 < 10 * ta:
            self.shoot("snapshot_twin", self.opts.get("snap_pct", 100), bonus, extra_pct=extra)

    def step(self):
        s, T, L = self.s, self.T, self.L
        # Dots and hots.
        if self.barbed and self.t < self.barbed[0] and self.t >= self.barbed[1]:
            self.barbed[1] += 2.0
            self.hit_mob(self.barbed[2], "barbed", False, dmg_pct=self.dmg_vs())
            jb = T.get("jagged_barbs", 0)
            if jb and self.rng.random() * 100 < 4 * jb:
                self.sundered = min(self.sundered + 1, 3)
                self.sundered_until = self.t + 10
        if self.dressing and self.t < self.dressing[0] and self.t >= self.dressing[1]:
            self.dressing[1] += 2.0
            self.heal(self.dressing[2], "field_dressing")
        slowed = self.t < self.slow_until
        in_melee = self.dist <= 5
        can_shoot = self.dist > 8 or T.get("point_blank")
        # Finish casts.
        if self.cast and self.t >= self.cast[0]:
            key = self.cast[1]
            self.cast = None
            if key == "puncturing_shaft":
                p = fx(skill("ranger", "puncturing_shaft"), "weapon_strike")
                pf = T.get("piercing_force", 0)
                pb = T.get("point_blank", 0)
                extra = 10 * pb if pb and self.dist <= 8 else 0
                self.shoot("puncturing_shaft", 100, linear(p["bonus"], L), linear(p["armor_ignore"], L) + 50 * pf, 5 * pf, extra)
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
        if self.moving:
            gt = T.get("guerilla_tactics", 0)
            if gt and int((self.t - self.moving_since) / 2) > int((self.t - DT - self.moving_since) / 2):
                self.gt_stacks = min(self.gt_stacks + 1, 3)
                self.gt_until = self.t + 15
        # Instants and casts.
        if not self.cast and self.t >= self.gcd:
            if self.hp < s.max_hp * 0.5 and self.ready("dressing") and self.power >= self.pct_base(10):
                self.spend_mana(self.pct_base(10))
                self.cd("dressing", 45000)
                self.dressing = [self.t + 10, self.t + 2, s.max_hp * 20 // 100 // 5]
                self.gcd = self.t + 1.5
            elif in_melee and self.ready("vault"):
                self.cd("vault", 15000)
                self.dist = 15.0
                self.gcd = self.t + 1.0
            elif self.ready("rapid") and can_shoot and not self.moving:
                self.cd("rapid", 60000)
                self.rapid_until = self.t + 10
                self.flurry = T.get("flurry", 0)
            elif can_shoot and not (self.barbed and self.t < self.barbed[0] - 2.5) and self.power >= self.pct_base(5):
                self.spend_mana(self.pct_base(5))
                self.gcd = self.t + 1.5
                o = self.ranged_roll(0)
                if o not in ("miss", "dodge"):
                    a = aura("ranger", "aura_barbed_arrow")
                    p = a["effects"][0]["params"]
                    self.barbed = [self.t + 8, self.t + 2, linear(p["tick"], L)]
                    self.slow_until = self.t + 8
            elif can_shoot and not self.moving and self.power >= self.pct_base(8) and (self.dist - 8) / (MOB_RUN * (0.7 if slowed else 1.0)) >= (3.0 - 0.1 * self.rd_stacks) * 0.9:
                self.spend_mana(self.pct_base(8))
                ms = 3000 - 100 * self.rd_stacks
                self.rd_stacks = 0
                self.start("puncturing_shaft", ms)
            elif can_shoot and (self.power >= self.pct_base(5) or self.flurry):
                self.snapshot()
        # Auto shot.
        if can_shoot and not self.moving and not self.cast and self.t >= self.shot_t:
            self.shot_t = self.t + s.r_speed * 100 / (100 + self.ranged_haste())
            d, o = self.shoot("auto_shot")
            if d and self.t < self.rapid_until:
                self.power = min(self.power + self.pct_base(1), s.max_mana)
        # Melee when stuck.
        if in_melee and not self.moving and not can_shoot and self.t >= self.swing_t:
            self.swing_t = self.t + s.mh_speed
            self.white("mh")
        # Mob movement and swings.
        spd = MOB_RUN * (0.7 if slowed else 1.0)
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
        self.dist = 30.0
        self.mob_swing_t = 0.0
        self.gcd = 0.0
        self.cast = None
        self.channel = None   # [until, next]
        self.blight = None    # [until, next, tick, stacks]
        self.miasma = None    # [until, next, tick]
        self.mending_dot = None  # [until, next, tick] on self
        self.lance_hots = []  # [until, next, tick]
        self.maso = []        # [until, next, tick]
        self.virulence = False
        self.surge_until = -1
        self.absorb = 0

    def spell_crit(self):
        return self.s.crit

    def blight_tick(self):
        s, L = self.s, self.L
        a = aura("acolyte", "aura_creeping_blight")
        p = a["effects"][0]["params"]
        dur = int(self.opts.get("blight_dur", a["duration_ms"] / 1000) * 1000)
        tick_ms = int(self.opts.get("blight_tick_s", p["tick_ms"] / 1000) * 1000)
        ticks = dur // tick_ms
        return linear(self.opts.get("blight_tick", p["tick"]), L) + sp_periodic(s.spell_power, dur, ticks)

    def grow_blight(self):
        if self.opts.get("lance_grow", True) and self.blight and self.t < self.blight[0]:
            self.blight[3] = min(self.blight[3] + 1, self.opts.get("blight_cap", 255))

    def health_cost(self, pct):
        cost = self.s.max_hp * pct // 100
        self.hp -= cost
        self.costs += cost
        ms = self.T.get("masochism", 0)
        if ms:
            self.maso.append([self.t + 5, self.t + 1, cost * 6 * ms // 100])

    def start(self, key, ms):
        if self.t < self.surge_until:
            ms = ms * 100 // (100 + 10 * self.T.get("crimson_surge", 0))
        self.cast = (self.t + ms / 1000, key)
        self.gcd = self.t + 1.5

    def step(self):
        s, T, L = self.s, self.T, self.L
        # Ticks.
        if self.blight and self.t < self.blight[0] and self.t >= self.blight[1]:
            self.blight[1] += self.opts.get("blight_tick_s", 3.0)
            d = self.hit_mob(self.blight[2] * self.blight[3], "blight", False)
            self.heal(d * self.opts.get("leech", 50) // 100, "blight_leech")
            self.blight[3] = min(self.blight[3] + 1, self.opts.get("blight_cap", 255))
            vi = T.get("virulence", 0)
            if vi and self.rng.random() * 100 < 0.5 * vi * self.blight[3]:
                self.virulence = True
            self.symbiotic(d)
        if self.miasma and self.t < self.miasma[0] and self.t >= self.miasma[1] and self.dist <= 8:
            self.miasma[1] += 1.0
            d = self.hit_mob(self.miasma[2], "miasma", False)
            self.symbiotic(d)
            mf = T.get("morbid_fascination", 0)
            if mf:
                self.power = min(self.power + self.pct_base(mf), s.max_mana)
        elif self.miasma and self.t >= self.miasma[1]:
            self.miasma[1] += 1.0
        if self.mending_dot and self.t < self.mending_dot[0] and self.t >= self.mending_dot[1]:
            self.mending_dot[1] += 2.0
            self.hp -= min(self.mending_dot[2], max(self.hp - 1, 0))
            self.costs += self.mending_dot[2]
        for h in self.lance_hots + self.maso:
            if self.t < h[0] and self.t >= h[1]:
                h[1] += 2.0 if h in self.lance_hots else 1.0
                self.heal(h[2], "hot")
        self.lance_hots = [h for h in self.lance_hots if self.t < h[0]]
        self.maso = [h for h in self.maso if self.t < h[0]]
        # Channel ticks.
        if self.channel:
            if self.t >= self.channel[1]:
                self.channel[1] += 1.0
                p = fx(skill("acolyte", "siphon_life"), "drain")
                o = self.spell_roll(self.spell_crit())
                if o != "miss":
                    d = self.hit_mob(linear(p["amount"], L), "siphon", False, o == "crit")
                    self.heal(d, "siphon")
                    self.symbiotic(d)
            if self.t >= self.channel[0]:
                self.channel = None
        # Finish casts.
        if self.cast and self.t >= self.cast[0]:
            key = self.cast[1]
            self.cast = None
            if key == "blood_lance":
                self.health_cost(3)
                p = fx(skill("acolyte", "blood_lance"), "school_damage")
                o = self.spell_roll(self.spell_crit())
                if o != "miss":
                    amount = self.opts.get("lance_amount", p["amount"])
                    raw = linear(amount, L) + sp_direct(s.spell_power, 1500)
                    d = self.hit_mob(raw, "blood_lance", False, o == "crit")
                    self.lance_hots.append([self.t + 6, self.t + 2, d * 20 // 100 // 3])
                    self.grow_blight()
                    self.symbiotic(d)
                if T.get("crimson_surge"):
                    self.surge_until = self.t + 6
        in_melee = self.dist <= 5
        if not self.cast and not self.channel and self.t >= self.gcd:
            hp_frac = self.hp / s.max_hp
            if hp_frac < 0.55 and self.ready("mending"):
                self.cd("mending", 8000)
                self.health_cost(15)
                self.gcd = self.t + 1.5
                p = fx(skill("acolyte", "dark_mending"), "heal")
                heal = linear(p["amount"], L) + sp_direct(s.spell_power, 0)
                if T.get("will_to_survive") and hp_frac < 0.5:
                    heal = heal * (100 + 10 * T["will_to_survive"]) // 100
                self.heal(heal, "dark_mending")
                self.mending_dot = [self.t + 6, self.t + 2, heal * 15 // 100]
                co = T.get("coagulation", 0)
                if co:
                    self.absorb += heal * 10 * co // 100
            elif hp_frac < 0.45 and self.power >= self.pct_base(8):
                self.spend_mana(self.pct_base(8))
                self.channel = [self.t + 5.0, self.t + 1.0]
                self.gcd = self.t + 1.5
            elif not (self.blight and self.t < self.blight[0]) and self.power >= self.pct_base(5):
                self.spend_mana(self.pct_base(5))
                self.gcd = self.t + 1.5
                if self.spell_roll(0) != "miss":
                    self.blight = [self.t + self.opts.get("blight_dur", aura("acolyte", "aura_creeping_blight")["duration_ms"] / 1000), self.t + self.opts.get("blight_tick_s", 3.0), self.blight_tick(), 1]
            elif self.ready("miasma") and in_melee and self.power >= self.pct_base(15):
                self.spend_mana(self.pct_base(15))
                self.cd("miasma", 15000)
                self.gcd = self.t + 1.5
                a = aura("acolyte", "aura_miasma")
                self.miasma = [self.t + 10, self.t + 1, linear(a["effects"][0]["params"]["tick"], L)]
            elif T.get("necrotic_detonation") and self.ready("detonate") and self.blight and self.t < self.blight[0] and self.blight[3] >= 4 and self.mob_hp < self.m["hp"] * 0.5:
                self.cd("detonate", 20000)
                self.gcd = self.t + 1.5
                raw = (12 + L) * self.blight[3]
                d = self.hit_mob(raw, "detonation", False)
                self.heal(d * 150 // 100, "detonation")
                self.blight = None
            elif self.hp > s.max_hp * 0.25:
                ms = 1500 - (500 if self.virulence else 0)
                self.virulence = False
                self.start("blood_lance", ms)
        # Mob.
        if self.dist > 5:
            self.dist = max(self.dist - MOB_RUN * DT, 0)
            self.mob_swing_t = max(self.mob_swing_t, self.t + 0.3)
        elif self.t >= self.mob_swing_t:
            self.mob_swing_t = self.t + self.m["swing"]
            d, o = self.mob_swing()
            if d > 0:
                self.take(d)

    def symbiotic(self, d):
        sr = self.T.get("symbiotic_rot", 0)
        if sr:
            self.heal(d * 5 * sr // 100, "symbiotic")

    def fight(self):
        self.costs = 0
        r = super().fight()
        r["costs"] = self.costs / self.s.max_hp
        return r


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
            acolyte=Acolyte)

# ---------------------------------------------------------------- talent builds
# Ordered (talent, ranks) wish lists; filled in order while the row rule allows.
BUILDS = {
    "vanguard": [("divine_strength", 5), ("wrathful_iron", 5), ("righteous_fury", 5), ("zealous_restoration", 3),
                 ("hallowed_ground", 3), ("hammer_of_vanguard", 1), ("shield_specialization", 5), ("iron_retribution", 5)],
    "cutthroat": [("reflexive_strike", 5), ("blood_frenzy", 5), ("phantom_guile", 3), ("thirst", 2), ("flurry_of_blades", 3),
                  ("bloodletting", 5), ("executioners_toll", 3), ("scent_of_blood", 3), ("ruthlessness", 3), ("serrated_strikes", 5)],
    "initiate": [("ley_line_tap", 5), ("volatile_embers", 5), ("ley_line_attunement", 3), ("frigid_conservation", 2),
                 ("catalytic_resonance", 5), ("arcane_momentum", 3), ("thermal_feedback", 5), ("combustion_engine", 3)],
    "ranger": [("piercing_force", 5), ("jagged_barbs", 5), ("steadfast_aim", 2), ("shattered_defenses", 3), ("point_blank", 2),
               ("guerilla_tactics", 5), ("rhythmic_draw", 5), ("twin_arrows", 3), ("adrenaline_rush", 3)],
    "acolyte": [("virulence", 5), ("morbid_fascination", 3), ("symbiotic_rot", 3), ("necrotic_detonation", 1),
                ("vitality", 5), ("masochism", 5), ("crimson_surge", 2), ("will_to_survive", 3)],
}



# ---------------------------------------------------------------- levels and talents
def tier_cap(tier):
    return max((c.get("tier_cap") or 0) for c in SPELLS.values() if c.get("tier") == tier)


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
    Tier 1 Calling at 1 (talents.rs counts points from it), and its Power Level runs 10 to 30. Past
    the band's top the Calling stays at its cap."""
    tier = SPELLS[calling].get("tier") or 0
    if tier == 0:
        return level
    return next((cl for cl in range(1, tier_cap(tier) + 1) if power_level(tier, cl) >= level), tier_cap(tier))


def points_at(level):
    """rules/talents.rs points_at: one Talent Point a level from level 2."""
    return max(level - 1, 0)


def build(calling, level):
    pts = points_at(calling_level(calling, level))
    spent = defaultdict(int)
    out = {}
    wish = BUILDS.get(calling, [])
    progress = True
    while pts > 0 and progress:
        progress = False
        for key, want in wish:
            have = out.get(key, 0)
            if have >= want:
                continue
            tab, row = TALENT_ROW[(calling, key)]
            if spent[tab] < 5 * (row - 1):
                continue
            out[key] = have + 1
            spent[tab] += 1
            pts -= 1
            progress = True
            if pts == 0:
                break
    return out


def character(calling, level, tier):
    """`calling` at Power Level `level` in a `tier` kit, in its damage build."""
    entries, off, fell_back = kit(calling, level, tier)
    talents = build(calling, level)
    stats = build_stats(calling, level, entries, off, talents)
    return dict(stats=stats, talents=talents, entries=entries, off=off, fell_back=fell_back)


def describe_kit(entries, off):
    names = [GEAR[e]["name"] for e in entries]
    if off:
        names.append(GEAR[off]["name"] + " (OH)")
    return ", ".join(names)
