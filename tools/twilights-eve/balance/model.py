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


def build_stats(calling, level, entries, off_entry, rows):
    """`calling`'s stats at `level` in the gear `entries`, in the build `rows`: a passive's
    max_health_pct raises the maximum health."""
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
    names = ["str", "agi", "sta", "int", "spi"]
    s.str, s.agi, s.sta, s.int, s.spi = (base[i] + gear[n] for i, n in enumerate(names))
    s.base_health = per["base_health"]
    s.max_hp = s.base_health + min(s.sta, 20) + max(s.sta - 20, 0) * 10
    s.max_hp = s.max_hp * (100 + sum(p["pct"] for p in passive_effects(calling, rows, "max_health_pct"))) // 100
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


def passive_effects(calling, rows, kind):
    """The params of every `kind` effect on the passives the build `rows` sets: the choices of
    `calling`'s talent rows, which replace no skill."""
    return [p for r in talent_rows(calling) if r["kind"] == "talent" and r["key"] in rows
            for p in fxs(choice(calling, r["key"], rows[r["key"]]), kind)]


# Tier 1 choices that change nothing in the sim's fights, with the reason the report prints
# (test_every_tier1_choice_changes_the_fight_or_is_ignored).
IGNORED = {
    ("ranger", "explosive_payload"): "not modelled yet",
    ("ranger", "guerilla_tactics"): "not modelled yet",
    ("ranger", "point_blank"): "not modelled yet",
    ("ranger", "twin_arrows"): "not modelled yet",
    ("ranger", "rhythmic_draw"): "not modelled yet",
    ("ranger", "adrenaline_rush"): "not modelled yet",
    ("ranger", "parting_gift"): "not modelled yet",
    ("ranger", "parting_shot"): "not modelled yet",
    ("ranger", "unbound_leap"): "not modelled yet",
    ("ranger", "jagged_barbs"): "not modelled yet",
    ("ranger", "pinning_barb"): "not modelled yet",
    ("ranger", "bloodscent"): "not modelled yet",
    ("ranger", "piercing_force"): "not modelled yet",
    ("ranger", "staggering_shaft"): "not modelled yet",
    ("ranger", "killshot"): "not modelled yet",
    ("ranger", "flurry"): "not modelled yet",
    ("ranger", "rapid_assault"): "not modelled yet",
    ("ranger", "running_fire"): "not modelled yet",
    ("initiate", "frost_ward"): "movement slow",
    ("initiate", "splintering_cold"): "movement slow",
    ("initiate", "frostbite"): "root",
    ("initiate", "shattered_blink"): "root",
    ("vanguard", "sweeping_zeal"): "no pull holds more than 3 mobs",
    ("acolyte", "epidemic"): "not modelled yet",
    ("acolyte", "virulence"): "not modelled yet",
    ("acolyte", "withering_blight"): "not modelled yet",
    ("acolyte", "exsanguinate"): "not modelled yet",
    ("acolyte", "splintering_lance"): "not modelled yet",
    ("acolyte", "sanguine_barrier"): "not modelled yet",
    ("acolyte", "necrotic_detonation"): "not modelled yet",
    ("acolyte", "masochism"): "not modelled yet",
    ("acolyte", "will_to_survive"): "not modelled yet",
    ("acolyte", "feeding_siphon"): "not modelled yet",
    ("acolyte", "gorging_siphon"): "not modelled yet",
    ("acolyte", "unbroken_siphon"): "not modelled yet",
    ("acolyte", "coagulation"): "not modelled yet",
    ("acolyte", "dark_surplus"): "not modelled yet",
    ("acolyte", "sanguine_flow"): "not modelled yet",
    ("acolyte", "noxious_vapors"): "not modelled yet",
    ("acolyte", "breeding_ground"): "not modelled yet",
    ("acolyte", "shroud_of_rot"): "not modelled yet",
}


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
            d = armored(raw, max(self.m_armor - armor_ignore, 0), self.L)
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

    def splash(self, d, src):
        """`d` damage, past armour already, to the nearest other living mob: none in a fight with
        one (multi.Multi has the others)."""

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
        self.buffs = {}    # aura key -> (until, aura row) on the Initiate
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

    def grant(self, key):
        a = aura(self.calling, key)
        self.buffs[key] = (self.t + a["duration_ms"] / 1000, a)

    def buffed(self, kind):
        """The params of every `kind` effect on the Initiate's buffs still up."""
        return [p for until, a in self.buffs.values() if self.t < until for p in fxs(a, kind)]

    def spell_pct(self):
        return sum(p["pct"] for p in self.buffed("mod_spell_damage_pct"))

    def amount_pct(self, key):
        return sum(p["pct"] for p in self.buffed("skill_amount_pct") if p["skill"] == key)

    def next_casts(self, key):
        return [p for p in self.buffed("next_cast") if p["skill"] == key]

    def price(self, key, mods=None):
        """Skill `key`'s cost after the next_cast changes `mods` (the buffs' now when None)."""
        mods = self.next_casts(key) if mods is None else mods
        return self.cost(self.skill(key)) * (100 + sum(p.get("cost_pct", 0) for p in mods)) // 100

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
        for k in [k for k, (until, a) in self.buffs.items() if any(p["skill"] == key for p in fxs(a, "next_cast"))]:
            del self.buffs[k]
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

    def ranged_haste(self):
        return 40 if self.t < self.rapid_until else 0

    def shoot(self, src, weapon_pct=100, bonus=0, armor_ignore=0):
        s = self.s
        o = self.ranged_roll(s.crit)
        if o in ("miss", "dodge"):
            return 0, o
        raw = self.rng.uniform(s.r_min, s.r_max) * weapon_pct / 100 + bonus
        d = self.hit_mob(raw, src, True, o == "crit", armor_ignore=armor_ignore)
        return d, o

    def start(self, key, ms):
        self.cast = (self.t + ms / 1000, key)
        self.gcd = self.t + 1.5

    def snapshot(self):
        sk = self.skill("snapshot")
        self.spend_mana(self.cost(sk))
        self.gcd = self.t + 1.5
        p = fx(sk, "weapon_strike")
        bonus = linear(self.opts.get("snap_bonus", p["bonus"]), self.L)
        self.shoot("snapshot", self.opts.get("snap_pct", 100), bonus)

    def step(self):
        s, L = self.s, self.L
        # Dots and hots.
        if self.barbed and self.t < self.barbed[0] and self.t >= self.barbed[1]:
            self.barbed[1] += 2.0
            self.hit_mob(self.barbed[2], "barbed", False)
        if self.dressing and self.t < self.dressing[0] and self.t >= self.dressing[1]:
            self.dressing[1] += 2.0
            self.heal(self.dressing[2], "field_dressing")
        slowed = self.t < self.slow_until
        in_melee = self.dist <= 5
        can_shoot = self.dist > 8
        # Finish casts.
        if self.cast and self.t >= self.cast[0]:
            key = self.cast[1]
            self.cast = None
            if key == "puncturing_shaft":
                p = fx(self.skill("puncturing_shaft"), "weapon_strike")
                self.shoot("puncturing_shaft", 100, linear(p["bonus"], L), linear(p["armor_ignore"], L))
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
            vault, rapid = self.skill("vault"), self.skill("rapid_fire")
            barbed, shaft = self.skill("barbed_arrow"), self.skill("puncturing_shaft")
            if self.hp < s.max_hp * 0.5 and self.ready("dressing") and self.power >= self.pct_base(10):
                self.spend_mana(self.pct_base(10))
                self.cd("dressing", 45000)
                self.dressing = [self.t + 10, self.t + 2, s.max_hp * 20 // 100 // 5]
                self.gcd = self.t + 1.5
            elif in_melee and self.ready("vault"):
                self.cd("vault", vault["realm_cooldown_ms"])
                self.dist = 15.0
                self.gcd = self.t + 1.0
            elif self.ready("rapid") and can_shoot and not self.moving:
                self.cd("rapid", rapid["realm_cooldown_ms"])
                self.rapid_until = self.t + 10
            elif can_shoot and not (self.barbed and self.t < self.barbed[0] - 2.5) and self.power >= self.cost(barbed):
                self.spend_mana(self.cost(barbed))
                self.gcd = self.t + 1.5
                o = self.ranged_roll(0)
                if o not in ("miss", "dodge"):
                    a = aura("ranger", fx(barbed, "periodic_damage")["apply_aura"])
                    p = a["effects"][0]["params"]
                    self.barbed = [self.t + 8, self.t + 2, linear(p["tick"], L)]
                    self.slow_until = self.t + 8
            elif can_shoot and not self.moving and self.power >= self.cost(shaft) and (self.dist - 8) / (MOB_RUN * (0.7 if slowed else 1.0)) >= 3.0 * 0.9:
                self.spend_mana(self.cost(shaft))
                self.start("puncturing_shaft", 3000)
            elif can_shoot and self.power >= self.cost(self.skill("snapshot")):
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
        self.absorb = 0

    def spell_crit(self):
        return self.s.crit

    def blight_aura(self):
        return aura("acolyte", fx(self.skill("creeping_blight"), "periodic_damage")["apply_aura"])

    def blight_tick(self):
        s, L = self.s, self.L
        a = self.blight_aura()
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

    def start(self, key, ms):
        self.cast = (self.t + ms / 1000, key)
        self.gcd = self.t + 1.5

    def step(self):
        s, L = self.s, self.L
        # Ticks.
        if self.blight and self.t < self.blight[0] and self.t >= self.blight[1]:
            self.blight[1] += self.opts.get("blight_tick_s", 3.0)
            d = self.hit_mob(self.blight[2] * self.blight[3], "blight", False)
            self.heal(d * self.opts.get("leech", 50) // 100, "blight_leech")
            self.blight[3] = min(self.blight[3] + 1, self.opts.get("blight_cap", 255))
        if self.miasma and self.t < self.miasma[0] and self.t >= self.miasma[1] and self.dist <= 8:
            self.miasma[1] += 1.0
            self.hit_mob(self.miasma[2], "miasma", False)
        elif self.miasma and self.t >= self.miasma[1]:
            self.miasma[1] += 1.0
        if self.mending_dot and self.t < self.mending_dot[0] and self.t >= self.mending_dot[1]:
            self.mending_dot[1] += 2.0
            self.hp -= min(self.mending_dot[2], max(self.hp - 1, 0))
            self.costs += self.mending_dot[2]
        for h in self.lance_hots:
            if self.t < h[0] and self.t >= h[1]:
                h[1] += 2.0
                self.heal(h[2], "hot")
        self.lance_hots = [h for h in self.lance_hots if self.t < h[0]]
        # Channel ticks.
        if self.channel:
            if self.t >= self.channel[1]:
                self.channel[1] += 1.0
                p = fx(self.skill("siphon_life"), "drain")
                o = self.spell_roll(self.spell_crit())
                if o != "miss":
                    d = self.hit_mob(linear(p["amount"], L), "siphon", False, o == "crit")
                    self.heal(d, "siphon")
            if self.t >= self.channel[0]:
                self.channel = None
        # Finish casts.
        if self.cast and self.t >= self.cast[0]:
            key = self.cast[1]
            self.cast = None
            if key == "blood_lance":
                lance = self.skill("blood_lance")
                self.health_cost(lance["realm_cost"])
                p = fx(lance, "school_damage")
                o = self.spell_roll(self.spell_crit())
                if o != "miss":
                    amount = self.opts.get("lance_amount", p["amount"])
                    raw = linear(amount, L) + sp_direct(s.spell_power, 1500)
                    d = self.hit_mob(raw, "blood_lance", False, o == "crit")
                    self.lance_hots.append([self.t + 6, self.t + 2, d * 20 // 100 // 3])
                    self.grow_blight()
        in_melee = self.dist <= 5
        if not self.cast and not self.channel and self.t >= self.gcd:
            hp_frac = self.hp / s.max_hp
            mending, siphon = self.skill("dark_mending"), self.skill("siphon_life")
            blight, miasma = self.skill("creeping_blight"), self.skill("miasma")
            if hp_frac < 0.55 and self.ready("mending"):
                self.cd("mending", mending["realm_cooldown_ms"])
                self.health_cost(mending["realm_cost"])
                self.gcd = self.t + 1.5
                p = fx(mending, "heal")
                heal = linear(p["amount"], L) + sp_direct(s.spell_power, 0)
                self.heal(heal, "dark_mending")
                self.mending_dot = [self.t + 6, self.t + 2, heal * 15 // 100]
            elif hp_frac < 0.45 and self.power >= self.cost(siphon):
                self.spend_mana(self.cost(siphon))
                self.channel = [self.t + 5.0, self.t + 1.0]
                self.gcd = self.t + 1.5
            elif not (self.blight and self.t < self.blight[0]) and self.power >= self.cost(blight):
                self.spend_mana(self.cost(blight))
                self.gcd = self.t + 1.5
                if self.spell_roll(0) != "miss":
                    self.blight = [self.t + self.opts.get("blight_dur", self.blight_aura()["duration_ms"] / 1000), self.t + self.opts.get("blight_tick_s", 3.0), self.blight_tick(), 1]
            elif self.ready("miasma") and in_melee and self.power >= self.cost(miasma):
                self.spend_mana(self.cost(miasma))
                self.cd("miasma", miasma["realm_cooldown_ms"])
                self.gcd = self.t + 1.5
                a = aura("acolyte", fx(miasma, "ground_area")["enemy_aura"])
                self.miasma = [self.t + 10, self.t + 1, linear(a["effects"][0]["params"]["tick"], L)]
            elif self.hp > s.max_hp * 0.25:
                self.start("blood_lance", 1500)
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

# ---------------------------------------------------------------- levels and Talent Rows
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
    Tier 1 Calling at 1, and its Power Level runs 10 to 30. Past the band's top the Calling stays
    at its cap."""
    tier = SPELLS[calling].get("tier") or 0
    if tier == 0:
        return level
    return next((cl for cl in range(1, tier_cap(tier) + 1) if power_level(tier, cl) >= level), tier_cap(tier))


def open_rows(calling, level):
    """The Talent Rows a Tier 1 `calling` at Power Level `level` has open: each whose `level` is at
    most its real level in the Calling. A Tier 0 character's rows stay empty (docs/balance/rules.md)."""
    if not SPELLS[calling].get("tier"):
        return []
    real = calling_level(calling, level)
    return [r for r in talent_rows(calling) if r["level"] <= real]


def character(calling, level, tier, rows={}):
    """`calling` at Power Level `level` in a `tier` kit, in the build `rows` ({row key: choice key})."""
    entries, off, fell_back = kit(calling, level, tier)
    stats = build_stats(calling, level, entries, off, rows)
    return dict(stats=stats, rows=dict(rows), entries=entries, off=off, fell_back=fell_back)


def describe_kit(entries, off):
    names = [GEAR[e]["name"] for e in entries]
    if off:
        names.append(GEAR[off]["name"] + " (OH)")
    return ", ".join(names)
