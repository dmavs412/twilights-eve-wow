#!/usr/bin/env python3
"""Offline fight simulator: a Warrior Calling vs a level-matched normal mob, to tune skill formulas.

A rough Monte Carlo of one-on-one melee, built on the vanilla 1.12 rules in
docs/reference/vanilla-combat-rules.md (hit table, armour, rage) and the skill formulas in
data/twilights-eve/callings.json. It is a tuning aid, not the realm: the realm's tests are the
authority. The target is a fight of roughly 15-25 s against a same-level mob.

    python tools/twilights-eve/simulate_fights.py --mobs mob_base_stats.json --stats warrior_level_stats.json
    python tools/twilights-eve/simulate_fights.py --calling vanguard --levels 1 10 20 30

    python tools/twilights-eve/simulate_fights.py --calling vanguard --levels 14 18 22 26 30 --kit world --mob normal
    python tools/twilights-eve/simulate_fights.py --calling vanguard --levels 14 18 22 26 30 --kit dungeon --mob elite

--kit and --gear wear gear.csv rows (Might kit, six slots) in place of the WEAPONS and GEAR_ARMOR
tables; Spell Power, block and defence are not modelled. --mob scales the mob as the realm does.

--mobs and --stats default to data/twilights-eve/{mob_base_stats,warrior_level_stats}.json (from the
combat-reference branch). Stdlib only.

Assumptions (all named constants below):
- Human warrior, naked base stats plus the Calling's Tier Bonus, plus a level-appropriate white
  one-hander (WEAPONS) and armour (GEAR_ARMOR). Weapon skill and defence at 5 x level.
- The mob is a creature_classlevelstats "warrior" row at the player's level: 2.0 s swings of
  melee_damage_avg +/- 15%, 5% dodge/parry/block, 5% crit; it stands in front of the player.
- White swings: one roll (miss, dodge, parry, glancing, block, crit, hit). Abilities: the
  two-roll model (miss/dodge/parry, then crit), no glancing, as the reference recommends.
- Rotation: Bloodrage on pull, Charge opener, Battle Shout when missing, Rend when missing,
  Overpower on a dodge, Mortal Strike/Execute when usable (Tier 1), Heroic Strike/Cleave as a rage
  dump above HS_DUMP_RAGE. Thunder Clap and Hamstring are not used against a single mob.
- No talents.
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import random
import statistics
import sys
from typing import Any, Dict, List

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
DATA = os.path.join(REPO, "data", "twilights-eve")

# Level -> (min, max, speed s) of a typical white one-hander a questing warrior holds.
WEAPONS = {1: (2, 4, 2.0), 5: (4, 8, 2.3), 10: (8, 15, 2.5), 15: (13, 24, 2.6), 20: (20, 37, 2.6),
           25: (26, 48, 2.7), 30: (32, 59, 2.7)}  # 15+: Token-vendor greens (~11-17 dps)
GEAR_ARMOR = {1: 20, 5: 90, 10: 200, 15: 320, 20: 450, 25: 600, 30: 750}
# content/mobs.toml: [difficulty] health, damage; its per-level growth; [difficulty.rank] elite; the
# Elwynn Forest [[zone]]. Normal rank is [1, 1].
DIFFICULTY = (2.0, 1.5)
GROWTH = (0.05, 0.04)
RANKS = {"normal": (1.0, 1.0), "elite": (2.0, 1.5)}
ZONES = {"none": (1.0, 1.0), "elwynn": (1.5, 1.3)}
# Loot Bands (#448): the highest level of each, and the dungeon blues' first entry by the highest
# level it serves (Jasperlode 14-18, Azora 22, Hogger's camp 26-30).
BAND_TOPS = [14, 18, 22, 26, 30]
DUNGEON_BASES = [(18, 905000), (22, 905020), (30, 905040)]
# Offsets in a 20-row set: plate Chest, sword, shield, Might amulet, two Might rings.
MIGHT_OFFSETS = [3, 4, 5, 11, 14, 15]
HS_DUMP_RAGE = 40
TICK = 0.1
MOB_SWING = 2.0


def nearest(table: Dict[int, Any], level: int) -> Any:
    return table[min(table, key=lambda k: (abs(k - level), k))]


def linear(c: Dict[str, float], level: int, ap: float) -> int:
    v = c.get("base", 0) + c.get("per_level", 0) * level + c.get("per_ap", 0) * ap
    r = int(abs(v) + 0.5) * (1 if v >= 0 else -1)
    return max(r, c.get("min", 0))


def rage_conv(level: int) -> float:
    return 0.0091107836 * level * level + 3.225598133 * level + 4.2652911


def armor_dr(armor: float, attacker_level: int) -> float:
    k = 0.1 * armor / (8.5 * attacker_level + 40)
    return min(max(k / (1 + k), 0.0), 0.75)


class Fighter:
    pass


def load_gear(entries: List[int]) -> List[Dict[str, str]]:
    with open(os.path.join(DATA, "gear.csv"), encoding="utf-8", newline="") as f:
        rows = {int(r["entry"]): r for r in csv.DictReader(f)}
    return [rows[e] for e in entries]


def kit_entries(kit: str, level: int) -> List[int]:
    """The Might kit of a Loot Band's World greens or a dungeon's blues for a character at `level`.

    Both pools list a set as Chests (cloth, leather, mail, plate), then sword, shield, dagger, bow,
    staff, mace, held item, three amulets, six rings (Might first, two of them)."""
    if kit == "world":
        base = 902000 + 20 * next(i for i, top in enumerate(BAND_TOPS) if level <= top or i == len(BAND_TOPS) - 1)
    else:
        base = next(b for top, b in DUNGEON_BASES if level <= top)
    return [base + i for i in MIGHT_OFFSETS]


def build(calling: Dict[str, Any], tier_bonus: Dict[str, int], stats: Dict[str, Any], mobs: Dict[str, Any], level: int,
          gear: List[Dict[str, str]] = None, mob_rank: str = None, zone: str = "elwynn"):
    race = next(r for r in stats["race_base_stats"]["human"] if r["level"] == level)
    base_hp = next(r for r in stats["per_level"] if r["level"] == level)["base_health"]
    crit_div = next(r for r in stats["per_level"] if r["level"] == level)["agi_per_crit_pct"]
    num = lambda row, col: float(row[col] or 0)
    p = Fighter()
    p.level = level
    p.str = race["str"] + tier_bonus.get("strength", 0)
    p.agi = race["agi"] + tier_bonus.get("agility", 0)
    sta = race["sta"] + tier_bonus.get("stamina", 0)
    gear_ap = gear_crit = 0.0
    p.armor = GEAR_ARMOR[min(GEAR_ARMOR, key=lambda k: abs(k - level))]
    p.wmin, p.wmax, p.speed = nearest(WEAPONS, level)
    if gear is not None:
        p.armor = 0
        for row in gear:
            p.str += num(row, "str")
            p.agi += num(row, "agi")
            sta += num(row, "sta")
            gear_ap += num(row, "attack_power")
            gear_crit += num(row, "crit")
            p.armor += num(row, "armor")
            if row["slot"] == "Weapon":
                p.wmin, p.wmax, p.speed = num(row, "dmg_min"), num(row, "dmg_max"), num(row, "speed")
    p.base_hp = base_hp
    p.max_hp = base_hp + min(sta, 20) + max(sta - 20, 0) * 10
    p.base_ap = 3 * level + 2 * p.str - 20 + gear_ap
    p.crit = p.agi / crit_div / 100 + gear_crit / 100
    p.armor += 2 * p.agi
    m = next(r for r in mobs["by_unit_class"]["warrior"] if r["level"] == level)
    mob = Fighter()
    mob.level = level
    mob.max_hp = m["health"]
    mob.avg = m["melee_damage_avg"]
    if mob_rank is not None:
        # mobs.toml [difficulty], its per-level growth, the Zone Factor, then the rank (ADR 0012).
        health_mul = DIFFICULTY[0] * (1 + GROWTH[0]) ** (level - 1) * ZONES[zone][0] * RANKS[mob_rank][0]
        damage_mul = DIFFICULTY[1] * (1 + GROWTH[1]) ** (level - 1) * ZONES[zone][1] * RANKS[mob_rank][1]
        mob.max_hp *= health_mul
        mob.avg *= damage_mul
    mob.armor = m["armor"]
    mob.block_value = level / 2 + m["str"] / 20
    skills = {s["key"]: s for s in calling["skills"]}
    return p, mob, skills


def fight(p, mob, skills, rng: random.Random) -> Dict[str, float]:
    L = p.level
    conv = rage_conv(L)
    hp, mob_hp = p.max_hp, mob.max_hp
    rage = 0.0
    t = 0.0
    p_swing, m_swing = 0.0, 0.5  # the player's first swing lands on arrival
    gcd = 0.0
    cds: Dict[str, float] = {}
    bs_until = -1.0
    rend_until, rend_next = -1.0, 0.0
    op_window = -1.0
    hs_queued = None
    mob_stunned_until = 0.0
    dmg_by: Dict[str, float] = {}

    def ap() -> float:
        return p.base_ap + (bs_amount if t < bs_until else 0)

    def weapon_roll(normalized: bool) -> float:
        spd = 2.4 if normalized else p.speed
        return rng.uniform(p.wmin, p.wmax) + ap() / 14 * spd

    def deal(src: str, dmg: float, physical: bool = True) -> None:
        nonlocal mob_hp
        if physical:
            dmg = max(1.0, dmg * (1 - armor_dr(mob.armor, L)))
        dmg = int(dmg)
        mob_hp -= dmg
        dmg_by[src] = dmg_by.get(src, 0) + dmg

    def yellow(unavoidable: bool = False) -> str:
        r = rng.random()
        if r < 0.05:
            return "miss"
        if not unavoidable and r < 0.15:
            return "dodge" if r < 0.10 else "parry"
        return "crit" if rng.random() < p.crit else "hit"

    def can(key: str, cost: float) -> bool:
        return key in skills and rage >= cost and cds.get(key, 0) <= t

    def spend(key: str, cost: float) -> None:
        nonlocal rage, gcd
        rage -= cost
        realm = skills[key]["realm"]
        if realm.get("cooldown_ms"):
            cds[key] = t + realm["cooldown_ms"] / 1000
        if realm.get("gcd_ms"):
            gcd = t + realm["gcd_ms"] / 1000

    def fx(key: str, kind: str) -> Dict[str, Any]:
        return next(e for e in skills[key]["realm"]["effects"] if e["kind"] == kind)

    bs_amount = 0
    # Pull: Bloodrage, then Charge.
    if "bloodrage" in skills:
        rage += linear(fx("bloodrage", "energize")["amount"], L, ap())
        cds["bloodrage"] = 60
        bloodrage_ticks = 10
        hp -= int(0.2 * p.base_hp)  # 20% of base health (Spell.dbc ManaCostPercentage)
    else:
        bloodrage_ticks = 0
    if "charge" in skills:
        rage += linear(fx("charge", "energize")["amount"], L, ap())
        cds["charge"] = 15
        mob_stunned_until = 1.0
    next_bloodrage_tick = 1.0

    while mob_hp > 0 and hp > 0 and t < 120:
        rage = min(rage, 100)
        if bloodrage_ticks and t >= next_bloodrage_tick:
            rage += 1
            bloodrage_ticks -= 1
            next_bloodrage_tick += 1
        # Bleeds.
        if t < rend_until and t >= rend_next:
            deal("rend", linear(fx("rend", "periodic_damage")["tick"], L, ap()), physical=False)
            rend_next += 3
        # Abilities on the GCD.
        if t >= gcd:
            if "battle_shout" in skills and t >= bs_until and can("battle_shout", 10):
                spend("battle_shout", 10)
                bs_amount = linear(fx("battle_shout", "aura_attack_power")["amount"], L, ap())
                bs_until = t + 120
            elif "execute" in skills and mob_hp <= 0.2 * mob.max_hp and can("execute", 15):
                e = fx("execute", "execute")
                extra = rage - 15
                spend("execute", 15)
                rage = 0
                o = yellow()
                if o in ("hit", "crit"):
                    deal("execute", (linear(e["base"], L, ap()) + extra * linear(e["per_extra_rage"], L, ap())) * (2 if o == "crit" else 1))
            elif "overpower" in skills and t < op_window and can("overpower", 5):
                spend("overpower", 5)
                op_window = -1
                o = yellow(unavoidable=True)
                if o != "miss":
                    deal("overpower", (weapon_roll(True) + linear(fx("overpower", "weapon_strike")["bonus"], L, ap())) * (2 if o == "crit" else 1))
            elif "mortal_strike" in skills and can("mortal_strike", 30):
                spend("mortal_strike", 30)
                o = yellow()
                if o in ("hit", "crit"):
                    deal("mortal_strike", (weapon_roll(True) + linear(fx("mortal_strike", "weapon_strike")["bonus"], L, ap())) * (2 if o == "crit" else 1))
            elif ("rend" in skills and t >= rend_until and can("rend", 10) and mob_hp > 0.3 * mob.max_hp
                  # Tier 1 pools rage for a ready Mortal Strike.
                  and ("mortal_strike" not in skills or rage >= 40 or cds.get("mortal_strike", 0) > t + 3)):
                spend("rend", 10)
                o = yellow()
                if o in ("hit", "crit"):
                    e = fx("rend", "periodic_damage")
                    rend_until, rend_next = t + e["duration_ms"] / 1000, t + 3
        # On-next-swing dump.
        dump = "cleave" if "cleave" in skills else "heroic_strike"
        if hs_queued is None and dump in skills and rage >= HS_DUMP_RAGE:
            hs_queued = dump
        # Player swing.
        if t >= p_swing:
            p_swing = t + p.speed
            if hs_queued and rage >= skills[hs_queued]["realm"]["rage_cost"]:
                key = hs_queued
                hs_queued = None
                rage -= skills[key]["realm"]["rage_cost"]
                o = yellow()
                if o == "dodge":
                    op_window = t + 5
                if o in ("hit", "crit"):
                    deal(key, (weapon_roll(False) + linear(fx(key, "weapon_strike")["bonus"], L, ap())) * (2 if o == "crit" else 1))
            else:
                hs_queued = None
                r = rng.random()
                glance = 0.10
                if r < 0.05:
                    pass
                elif r < 0.10:
                    op_window = t + 5
                    rage += weapon_roll(False) * 0.75 / conv * 7.5
                elif r < 0.15:
                    rage += weapon_roll(False) * 0.75 / conv * 7.5
                else:
                    raw = weapon_roll(False)
                    if r < 0.15 + glance:
                        raw *= 0.95
                    elif r < 0.30:  # block, 0.25..0.30
                        raw = max(0, raw - mob.block_value)
                    elif rng.random() < p.crit:
                        raw *= 2
                    before = mob_hp
                    deal("white", raw)
                    rage += (before - mob_hp) / conv * 7.5
        # Mob swing.
        if t >= m_swing and t >= mob_stunned_until:
            m_swing = t + MOB_SWING
            r = rng.random()
            dodge = p.crit  # naked dodge% equals naked crit% for warriors
            if r < 0.05 + dodge + 0.05:
                pass  # miss, dodge, parry
            else:
                dmg = rng.uniform(mob.avg * 0.85, mob.avg * 1.15)
                if rng.random() < 0.05:
                    dmg *= 2
                dmg = max(1, int(dmg * (1 - armor_dr(p.armor, mob.level))))
                hp -= dmg
                rage += dmg / conv * 2.5
        t += TICK
    return {"time": t, "won": mob_hp <= 0, "hp_lost": (p.max_hp - max(hp, 0)) / p.max_hp, "dmg_by": dmg_by,
            "mob_left": max(mob_hp, 0) / mob.max_hp}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--callings", default=os.path.join(DATA, "callings.json"))
    ap.add_argument("--mobs", default=os.path.join(DATA, "mob_base_stats.json"))
    ap.add_argument("--stats", default=os.path.join(DATA, "warrior_level_stats.json"))
    ap.add_argument("--calling", default="basic_warrior")
    ap.add_argument("--levels", type=int, nargs="+")
    ap.add_argument("--fights", type=int, default=2000)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--gear", type=int, nargs="+", metavar="ENTRY",
                    help="wear these gear.csv entries instead of the WEAPONS and GEAR_ARMOR tables, e.g. "
                         "902003 902004 902005 902011 902014 902015 (band 1's Might kit)")
    ap.add_argument("--kit", choices=["world", "dungeon"],
                    help="wear the Might kit of each level's Loot Band World greens, or of the dungeon "
                         "that serves the level (Jasperlode 14-18, Azora 22, Hogger's camp 26-30); "
                         "the six slots are plate Chest, sword, shield, amulet and two rings")
    ap.add_argument("--mob", choices=list(RANKS),
                    help="scale the mob by mobs.toml [difficulty], its level growth, the Zone Factor "
                         "and this rank; without it the mob is the vanilla class row")
    ap.add_argument("--zone", choices=list(ZONES), default="elwynn", help="Zone Factor for --mob")
    args = ap.parse_args()
    if args.gear and args.kit:
        ap.error("--gear and --kit are exclusive")
    load = lambda p: json.load(open(p, encoding="utf-8-sig"))
    callings, mobs, stats = load(args.callings), load(args.mobs), load(args.stats)
    calling = callings["callings"][args.calling]
    tb = callings["tier_bonus"][str(calling["tier"])]
    levels = args.levels or ([1, 5, 10] if calling["cap"] <= 10 else [1, 10, 20, 30])
    rng = random.Random(args.seed)
    what = f"{args.mob} " if args.mob else ""
    gear = f", {args.kit} kit" if args.kit else (f", gear {args.gear}" if args.gear else "")
    print(f"{args.calling}: {args.fights} fights per level vs a same-level {what}warrior-class mob{gear}")
    print(f"{'L':>3} {'pHP':>5} {'mobHP':>6} {'dps':>6} {'median s':>9} {'p10-p90 s':>11} {'win%':>6} {'hp lost':>8} {'mob left':>9}  damage share")
    for lv in levels:
        entries = kit_entries(args.kit, lv) if args.kit else args.gear
        p, mob, skills = build(calling, tb, stats, mobs, lv, load_gear(entries) if entries else None,
                               args.mob, args.zone)
        res = [fight(p, mob, skills, rng) for _ in range(args.fights)]
        dps = statistics.mean(sum(r["dmg_by"].values()) / r["time"] for r in res)
        times = sorted(r["time"] for r in res)
        tot: Dict[str, float] = {}
        for r in res:
            for k, v in r["dmg_by"].items():
                tot[k] = tot.get(k, 0) + v
        s = sum(tot.values()) or 1
        share = ", ".join(f"{k} {v / s:.0%}" for k, v in sorted(tot.items(), key=lambda kv: -kv[1]))
        print(f"{lv:>3} {p.max_hp:>5} {mob.max_hp:>6.0f} {dps:>6.1f} {statistics.median(times):>9.1f} "
              f"{times[len(times) // 10]:>5.1f}-{times[len(times) * 9 // 10]:<5.1f} "
              f"{sum(r['won'] for r in res) / len(res):>6.0%} {statistics.mean(r['hp_lost'] for r in res):>8.0%} "
              f"{statistics.mean(r['mob_left'] for r in res):>9.0%}  {share}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
