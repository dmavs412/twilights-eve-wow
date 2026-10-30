#!/usr/bin/env python3
"""Verify and dump the 1.12.1 spell records behind data/twilights-eve/callings.json, and list the
stock talents.

Reads the client's own tables (Spell, SpellIcon, SpellRange, SpellDuration, SpellCastTimes,
SpellRadius, Talent, TalentTab) straight off the MPQ patch chain with the stdlib-only reader in
wowdata.py. Column numbers are the ones crates/benilla-formats documents (spells/mod.rs,
talents.rs), which are vmangos's 1.12 SpellEntry / TalentEntry / TalentTabEntry.

Usage (any Python 3.9+, no installs):

    python tools/twilights-eve/extract_spells.py verify  [--data DIR] [--callings FILE] [--out FILE]
    python tools/twilights-eve/extract_spells.py spell   ID [ID ...]
    python tools/twilights-eve/extract_spells.py search  "Heroic Strike" [--family 4]
    python tools/twilights-eve/extract_spells.py talents [--class 1]

`verify` checks every skill's spell id in callings.json against the DBCs (names, ranks, icons,
costs, cooldowns) and writes the full field dump to
data/twilights-eve/spells-extracted.json. It exits non-zero on any mismatch, so it can gate a
data change. --data defaults to $WOW_DATA, then C:\\Users\\davis\\wow-data\\Data.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from typing import Any, Dict, List, Optional, Tuple

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wowdata import Chain, Dbc, load_dbc  # noqa: E402

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
DEFAULT_CALLINGS = os.path.join(REPO, "data", "twilights-eve", "callings.json")
DEFAULT_OUT = os.path.join(REPO, "data", "twilights-eve", "spells-extracted.json")


def default_data_dir() -> str:
    return os.environ.get("WOW_DATA") or r"C:\Users\davis\wow-data\Data"


# --- Spell.dbc columns (1.12.1, 173 fields) ----------------------------------------------------
C_SCHOOL, C_CATEGORY, C_DISPEL, C_MECHANIC = 1, 2, 4, 5
C_ATTR = 6  # ..10: Attributes, Ex, Ex2, Ex3, Ex4
C_STANCES, C_STANCES_NOT, C_TARGETS = 11, 12, 13
C_CASTER_AURA_STATE, C_TARGET_AURA_STATE = 16, 17
C_CAST_TIME_INDEX, C_RECOVERY, C_CATEGORY_RECOVERY = 18, 19, 20
C_PROC_FLAGS, C_PROC_CHANCE, C_PROC_CHARGES = 24, 25, 26
C_MAX_LEVEL, C_BASE_LEVEL, C_SPELL_LEVEL, C_DURATION_INDEX = 27, 28, 29, 30
C_POWER_TYPE, C_MANA_COST, C_RANGE_INDEX, C_SPEED, C_STACK = 31, 32, 36, 37, 39
C_EFFECT, C_DIE_SIDES, C_BASE_DICE, C_DICE_PER_LEVEL = 61, 64, 67, 70
C_REAL_PPL, C_BASE_POINTS, C_EFFECT_MECHANIC = 73, 76, 79
C_IMPLICIT_A, C_IMPLICIT_B, C_RADIUS_INDEX, C_AURA = 82, 85, 88, 91
C_AMPLITUDE, C_MULTIPLE_VALUE, C_CHAIN_TARGETS, C_MISC, C_TRIGGER = 94, 97, 100, 106, 109
C_ITEM_TYPE, C_MANA_COST_PCT = 103, 156

# SpellModOp (vmangos SpellDefines.h), the misc value of ADD_FLAT/PCT_MODIFIER auras.
SPELLMOD_OPS = {
    0: "DAMAGE", 1: "DURATION", 2: "THREAT", 3: "EFFECT1", 4: "CHARGES", 5: "RANGE", 6: "RADIUS",
    7: "CRITICAL_CHANCE", 8: "ALL_EFFECTS", 9: "NOT_LOSE_CASTING_TIME", 10: "CASTING_TIME",
    11: "COOLDOWN", 12: "EFFECT2", 14: "COST", 15: "CRIT_DAMAGE_BONUS", 16: "RESIST_MISS_CHANCE",
    17: "JUMP_TARGETS", 18: "CHANCE_OF_SUCCESS", 19: "ACTIVATION_TIME", 20: "EFFECT_PAST_FIRST",
    21: "CASTING_TIME_OLD", 22: "DOT", 23: "EFFECT3", 24: "SPELL_BONUS_DAMAGE",
}
C_ICON, C_ACTIVE_ICON, C_NAME, C_RANK, C_DESC, C_AURA_DESC = 117, 118, 120, 129, 138, 147
C_GCD_CATEGORY, C_GCD, C_MAX_TARGET_LEVEL, C_FAMILY = 157, 158, 159, 160
C_FAMILY_FLAGS_LO, C_FAMILY_FLAGS_HI, C_MAX_TARGETS, C_DMG_CLASS, C_PREVENTION = 161, 162, 163, 164, 165

POWER_NAMES = {0: "mana", 1: "rage", 2: "focus", 3: "energy", 4: "happiness", 0xFFFFFFFE: "health"}
SCHOOLS = {0: "physical", 1: "holy", 2: "fire", 3: "nature", 4: "frost", 5: "shadow", 6: "arcane"}
DMG_CLASS = {0: "none", 1: "magic", 2: "melee", 3: "ranged"}

# The warrior forms (SpellShapeshiftForm.dbc ids); a Stances bit is 1 << (form - 1).
FORMS = {17: "Battle Stance", 18: "Defensive Stance", 19: "Berserker Stance"}

# Spell effect and aura names for the ids the warrior set uses (vmangos SharedDefines.h /
# SpellAuraDefines.h). Unknown ids print as numbers.
EFFECTS = {
    2: "SCHOOL_DAMAGE", 3: "DUMMY", 6: "APPLY_AURA", 17: "WEAPON_DAMAGE_NOSCHOOL",
    30: "ENERGIZE", 31: "WEAPON_PERCENT_DAMAGE", 35: "APPLY_AREA_AURA_PARTY", 36: "LEARN_SPELL",
    58: "WEAPON_DAMAGE", 64: "TRIGGER_SPELL", 68: "INTERRUPT_CAST", 96: "CHARGE",
    114: "ATTACK_ME", 121: "NORMALIZED_WEAPON_DMG", 77: "SCRIPT_EFFECT", 125: "?",
    126: "STEAL_BENEFICIAL_BUFF", 20: "DODGE", 21: "EVADE", 22: "PARRY", 23: "BLOCK",
    57: "?", 76: "SCRIPT_EFFECT?", 63: "THREAT", 42: "?", 44: "SKILL_STEP", 7: "ENVIRONMENTAL",
    10: "HEAL", 24: "CREATE_ITEM", 41: "?", 62: "POWER_BURN", 1: "INSTAKILL",
}
AURAS = {
    3: "PERIODIC_DAMAGE", 4: "DUMMY", 7: "MOD_FEAR", 12: "MOD_STUN", 13: "MOD_DAMAGE_DONE",
    22: "MOD_RESISTANCE", 24: "PERIODIC_ENERGIZE", 26: "MOD_ROOT", 27: "MOD_SILENCE",
    29: "MOD_STAT", 31: "MOD_INCREASE_SPEED", 33: "MOD_DECREASE_SPEED", 42: "PROC_TRIGGER_SPELL",
    47: "MOD_PARRY_PERCENT", 49: "MOD_DODGE_PERCENT", 51: "MOD_BLOCK_PERCENT",
    52: "MOD_CRIT_PERCENT", 54: "MOD_HIT_CHANCE", 56: "TRANSFORM", 65: "MOD_CASTING_SPEED",
    69: "SCHOOL_ABSORB", 79: "MOD_DAMAGE_PERCENT_DONE", 85: "MOD_POWER_REGEN", 87: "MOD_DAMAGE_PERCENT_TAKEN",
    94: "INTERRUPT_REGEN", 99: "MOD_ATTACK_POWER", 101: "MOD_RESISTANCE_PCT", 102: "MOD_MELEE_ATTACK_POWER_VERSUS",
    107: "ADD_FLAT_MODIFIER", 108: "ADD_PCT_MODIFIER", 118: "MOD_HEALING_PCT", 123: "MOD_TARGET_RESISTANCE",
    133: "MOD_INCREASE_HEALTH_PERCENT", 137: "MOD_TOTAL_STAT_PERCENTAGE", 138: "MOD_HASTE",
    140: "MOD_RANGED_HASTE", 142: "MOD_BASE_RESISTANCE_PCT", 158: "MOD_SHIELD_BLOCKVALUE",
    167: "MOD_ATTACK_POWER_PCT", 184: "MOD_ATTACKER_MELEE_HIT_CHANCE", 189: "MOD_RATING?",
    10: "MOD_THREAT", 23: "PERIODIC_TRIGGER_SPELL", 8: "PERIODIC_HEAL", 15: "DAMAGE_SHIELD",
    33_000: "?", 80: "MOD_PERCENT_STAT", 14: "MOD_DAMAGE_TAKEN", 1: "BIND_SIGHT", 11: "MOD_TAUNT",
    98: "MOD_SKILL_TALENT", 30: "MOD_SKILL", 112: "OVERRIDE_CLASS_SCRIPTS", 66: "FEIGN_DEATH",
    53: "PERIODIC_LEECH", 163: "MOD_CRIT_DAMAGE_BONUS", 111: "ADD_CASTER_HIT_TRIGGER",
    132: "MOD_INCREASE_ENERGY_PERCENT", 34: "MOD_INCREASE_HEALTH", 157: "?",
}


class Tables:
    def __init__(self, data_dir: str):
        self.chain = Chain(data_dir)
        self.spell = load_dbc(self.chain, "Spell.dbc")
        self.icon = load_dbc(self.chain, "SpellIcon.dbc")
        self.range = load_dbc(self.chain, "SpellRange.dbc")
        self.duration = load_dbc(self.chain, "SpellDuration.dbc")
        self.cast_times = load_dbc(self.chain, "SpellCastTimes.dbc")
        self.radius = load_dbc(self.chain, "SpellRadius.dbc")
        self.talent = load_dbc(self.chain, "Talent.dbc")
        self.tab = load_dbc(self.chain, "TalentTab.dbc")
        self.sources = {
            n: self.chain.source(f"DBFilesClient\\{n}")
            for n in ["Spell.dbc", "SpellIcon.dbc", "Talent.dbc", "TalentTab.dbc"]
        }

    # --- spell --------------------------------------------------------------------------------
    def icon_path(self, icon_id: int) -> Optional[str]:
        r = self.icon.row(icon_id)
        return self.icon.str(r, 1) if r is not None else None

    def spell_fields(self, sid: int) -> Optional[Dict[str, Any]]:
        s = self.spell
        r = s.row(sid)
        if r is None:
            return None
        u, i, f = (lambda c: s.u32(r, c)), (lambda c: s.i32(r, c)), (lambda c: s.f32(r, c))

        rng = self.range.row(u(C_RANGE_INDEX))
        dur = self.duration.row(u(C_DURATION_INDEX))
        ct = self.cast_times.row(u(C_CAST_TIME_INDEX))
        stances = u(C_STANCES)
        effects = []
        for k in range(3):
            eff = u(C_EFFECT + k)
            if eff == 0:
                continue
            bp, dice, sides = i(C_BASE_POINTS + k), i(C_BASE_DICE + k), i(C_DIE_SIDES + k)
            lo, hi = bp + dice, bp + max(dice, sides)
            rad = self.radius.row(u(C_RADIUS_INDEX + k))
            e = {
                "effect": eff,
                "effect_name": EFFECTS.get(eff, str(eff)),
                "aura": u(C_AURA + k),
                "aura_name": AURAS.get(u(C_AURA + k), str(u(C_AURA + k))) if u(C_AURA + k) else None,
                "base_points": bp,
                "value": lo if lo == hi else [lo, hi],  # the tooltip's $sN (bp + dice roll)
                "real_points_per_level": round(f(C_REAL_PPL + k), 4),
                "amplitude_ms": u(C_AMPLITUDE + k),
                "misc_value": i(C_MISC + k),
                "trigger_spell": u(C_TRIGGER + k),
                "implicit_target": [u(C_IMPLICIT_A + k), u(C_IMPLICIT_B + k)],
                "radius_yd": round(self.radius.f32(rad, 1), 2) if rad is not None else None,
                "multiple_value": round(f(C_MULTIPLE_VALUE + k), 4),
                "chain_targets": u(C_CHAIN_TARGETS + k),
                # For ADD_FLAT/PCT_MODIFIER auras (107/108) misc_value is the SpellModOp and
                # EffectItemType is the low 32 bits of the spell-family mask it applies to.
                "item_type_or_family_mask": hex(u(C_ITEM_TYPE + k)),
            }
            if u(C_AURA + k) in (107, 108):
                e["spellmod_op"] = SPELLMOD_OPS.get(i(C_MISC + k), i(C_MISC + k))
            effects.append(e)
        power = u(C_POWER_TYPE)
        cost = u(C_MANA_COST)
        return {
            "id": sid,
            "name": s.str(r, C_NAME),
            "rank": s.str(r, C_RANK),
            "description": s.str(r, C_DESC),
            "aura_description": s.str(r, C_AURA_DESC),
            "icon_id": u(C_ICON),
            "icon": self.icon_path(u(C_ICON)),
            "school": SCHOOLS.get(u(C_SCHOOL), u(C_SCHOOL)),
            "dmg_class": DMG_CLASS.get(u(C_DMG_CLASS), u(C_DMG_CLASS)),
            "family": u(C_FAMILY),
            "family_flags": (u(C_FAMILY_FLAGS_HI) << 32) | u(C_FAMILY_FLAGS_LO),
            "spell_level": u(C_SPELL_LEVEL),
            "base_level": u(C_BASE_LEVEL),
            "max_level": u(C_MAX_LEVEL),
            "power_type": POWER_NAMES.get(power, power),
            # Rage is stored in tenths: 150 is 15 rage.
            "cost_raw": cost,
            "cost": cost / 10 if power == 1 else cost,
            "cost_pct": u(C_MANA_COST_PCT),
            "range_index": u(C_RANGE_INDEX),
            "range_yd": [round(self.range.f32(rng, 1), 2), round(self.range.f32(rng, 2), 2)] if rng is not None else None,
            "range_name": self.range.str(rng, 4) if rng is not None else None,
            "cast_time_ms": self.cast_times.i32(ct, 1) if ct is not None else None,
            "cooldown_ms": u(C_RECOVERY),
            "category": u(C_CATEGORY),
            "category_cooldown_ms": u(C_CATEGORY_RECOVERY),
            "gcd_ms": u(C_GCD),
            "gcd_category": u(C_GCD_CATEGORY),
            "duration_ms": self.duration.i32(dur, 1) if dur is not None else None,
            "stances_mask": stances,
            "stances": [n for form, n in FORMS.items() if stances & (1 << (form - 1))],
            "stances_not_mask": u(C_STANCES_NOT),
            "caster_aura_state": u(C_CASTER_AURA_STATE),
            "target_aura_state": u(C_TARGET_AURA_STATE),
            "attributes": [hex(u(C_ATTR + k)) for k in range(5)],
            "not_in_combat": bool(u(C_ATTR) & 0x10000000),
            "on_next_swing": bool(u(C_ATTR) & 0x4),
            "passive": bool(u(C_ATTR) & 0x40),
            # The stock spellbook's add gate (benilla-formats SpellDisplay::in_spellbook,
            # 0x4b25b0): out for Attributes 0x80 (do-not-display) or 0x20 (tradeskill), or a
            # nonzero castUI. Passive alone does not hide a spell.
            "in_spellbook": not (u(C_ATTR) & 0xA0) and u(3) == 0,
            "mechanic": u(C_MECHANIC),
            "dispel": u(C_DISPEL),
            "max_targets": u(C_MAX_TARGETS),
            "proc": {"flags": hex(u(C_PROC_FLAGS)), "chance": u(C_PROC_CHANCE), "charges": u(C_PROC_CHARGES)},
            "effects": effects,
        }

    def search(self, name: str, family: Optional[int]) -> List[Dict[str, Any]]:
        s = self.spell
        out = []
        for r in range(s.nrec):
            if s.str(r, C_NAME).lower() == name.lower():
                if family is not None and s.u32(r, C_FAMILY) != family:
                    continue
                out.append(self.spell_fields(s.u32(r, 0)))
        return out

    # --- talents ------------------------------------------------------------------------------
    def tab_fields(self, tab_id: int) -> Optional[Dict[str, Any]]:
        r = self.tab.row(tab_id)
        if r is None:
            return None
        t = self.tab
        return {
            "id": tab_id,
            "name": t.str(r, 1),
            "icon_id": t.u32(r, 10),
            "icon": self.icon_path(t.u32(r, 10)),
            "race_mask": t.u32(r, 11),
            "class_mask": t.u32(r, 12),
            "order_index": t.u32(r, 13),
            "background": t.str(r, 14),
            "file_row": r,
        }

    def talent_fields(self, talent_id: int) -> Optional[Dict[str, Any]]:
        t = self.talent
        r = t.row(talent_id)
        if r is None:
            return None
        ranks = [t.u32(r, 4 + k) for k in range(9)]
        n = 0
        while n < 9 and ranks[n]:
            n += 1
        sp = self.spell_fields(ranks[0]) if ranks[0] else None
        return {
            "id": talent_id,
            "tab": t.u32(r, 1),
            "row": t.u32(r, 2),
            "col": t.u32(r, 3),
            "rank_spells": ranks[:n],
            "max_rank": n,
            "prereq_talent": t.u32(r, 13),
            "prereq_rank": t.u32(r, 16),  # 0-based
            "flags": t.u32(r, 19),
            "required_spell": t.u32(r, 20),
            "file_row": r,
            "name": sp["name"] if sp else None,
            "icon": sp["icon"] if sp else None,
            "icon_id": sp["icon_id"] if sp else None,
            "description_rank1": sp["description"] if sp else None,
        }

    def tabs_for_class(self, class_id: int) -> List[Dict[str, Any]]:
        bit = 1 << (class_id - 1)
        return [self.tab_fields(i) for i in self.tab.ids() if self.tab.u32(self.tab.row(i), 12) & bit]

    def talents_in_tab(self, tab_id: int) -> List[Dict[str, Any]]:
        return [self.talent_fields(i) for i in self.talent.ids() if self.talent.u32(self.talent.row(i), 1) == tab_id]


# --- verify -----------------------------------------------------------------------------------

CLIENT_KEYS = ("cost", "cooldown_ms", "cast_time_ms", "range_yd", "stances", "caster_aura_state",
               "target_aura_state", "duration_ms", "gcd_ms", "not_in_combat", "on_next_swing")


def linear(c: Dict[str, float], level: int, ap: float) -> int:
    v = c.get("base", 0) + c.get("per_level", 0) * level + c.get("per_ap", 0) * ap
    r = int(abs(v) + 0.5) * (1 if v >= 0 else -1)  # half away from zero
    return max(r, c.get("min", 0))


def ap_estimate(level: int, tier_str: int) -> int:
    """A naked warrior's attack power: 3*level + 2*str - 20, str ~ 22 + level (+ Tier Bonus)."""
    return 3 * level + 2 * (22 + level + tier_str) - 20


def level_table(calling: Dict[str, Any], tier_bonus: Dict[str, Any]) -> Dict[str, Any]:
    """Each skill's linear terms at sample levels, with a naked-warrior AP estimate."""
    cap = calling["cap"]
    levels = sorted({1, cap // 2, cap} | ({10, 20} if cap >= 30 else {5}))
    tstr = tier_bonus.get(str(calling["tier"]), {}).get("strength", 0)
    out: Dict[str, Any] = {"levels": levels, "ap_estimate": [ap_estimate(lv, tstr) for lv in levels], "skills": {}}
    for sk in calling.get("skills", []):
        rows = {}
        for e in sk["realm"]["effects"]:
            for name in ("bonus", "amount", "tick", "base", "per_extra_rage"):
                if isinstance(e.get(name), dict):
                    rows[f"{e['kind']}.{name}"] = [linear(e[name], lv, ap_estimate(lv, tstr)) for lv in levels]
        out["skills"][sk["key"]] = rows
    return out


def verify(tables: Tables, callings: Dict[str, Any]) -> Tuple[List[str], Dict[str, Any]]:
    errors: List[str] = []
    dump: Dict[str, Any] = {"sources": tables.sources, "spells": {}, "level_tables": {}}

    def check_spell(sid: int, where: str, want: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        f = tables.spell_fields(sid)
        if f is None:
            errors.append(f"{where}: spell {sid} not in Spell.dbc")
            return None
        dump["spells"][str(sid)] = f
        for key in ("name", "rank", "icon_id"):
            if key in want and f[key] != want[key]:
                errors.append(f"{where}: spell {sid} {key} is {f[key]!r}, callings says {want[key]!r}")
        if "icon" in want and (f["icon"] or "").lower() != want["icon"].lower():
            errors.append(f"{where}: spell {sid} icon path {f['icon']!r}, callings says {want['icon']!r}")
        if "family_flags" in want and int(want["family_flags"], 16) != f["family_flags"]:
            errors.append(f"{where}: spell {sid} family flags {f['family_flags']:#x}, callings says {want['family_flags']}")
        for key in CLIENT_KEYS:
            if key in want.get("client", {}) and want["client"][key] != f[key]:
                errors.append(f"{where}: spell {sid} {key} is {f[key]!r}, callings says {want['client'][key]!r}")
        realm = want.get("realm")
        if realm and f["power_type"] == "rage" and realm.get("rage_cost", 0) < f["cost"]:
            errors.append(f"{where}: realm rage cost {realm.get('rage_cost')} below the client's {f['cost']}; the client would refuse casts the realm allows")
        # The client runs the longer of RecoveryTime and CategoryRecoveryTime off SMSG_SPELL_GO;
        # a realm cooldown shorter than that leaves the button grey while the realm would allow it.
        if realm and "cooldown_ms" in realm:
            client_cd = max(f["cooldown_ms"], f["category_cooldown_ms"])
            if realm["cooldown_ms"] != client_cd:
                errors.append(f"{where}: realm cooldown {realm['cooldown_ms']} ms, client runs {client_cd} ms")
        if realm and not f["in_spellbook"]:
            errors.append(f"{where}: spell {sid} is hidden from the stock spellbook (Attributes 0x80/0x20 or castUI)")
        if realm and "gcd_ms" in realm and realm["gcd_ms"] != f["gcd_ms"]:
            errors.append(f"{where}: realm GCD {realm['gcd_ms']} ms, Spell.dbc StartRecoveryTime {f['gcd_ms']} ms")
        return f

    for cname, calling in callings.get("callings", {}).items():
        for sk in calling.get("skills", []):
            check_spell(sk["spell_id"], f"{cname}.skills.{sk['key']}", sk)
        if calling.get("skills"):
            if len(calling["skills"]) > 8:
                errors.append(f"{cname}: {len(calling['skills'])} skills; a Calling has at most 8")
            dump["level_tables"][cname] = level_table(calling, callings.get("tier_bonus", {}))
    return errors, dump


def selftest(data_dir: str) -> int:
    """Checks of the reader and the formula helper; the DBC facts run only when the data is there."""
    from wowdata import _hash  # noqa: PLC0415
    fails: List[str] = []

    def eq(what: str, got: Any, want: Any) -> None:
        if got != want:
            fails.append(f"{what}: got {got!r}, want {want!r}")

    # The MPQ table keys every MPQ tool agrees on.
    eq("hash('(hash table)', 3)", _hash("(hash table)", 3), 0xC3AF3770)
    eq("hash('(block table)', 3)", _hash("(block table)", 3), 0xEC83B3A3)
    eq("hash is case- and slash-blind", _hash("dbfilesclient/spell.dbc", 1), _hash("DBFilesClient\\Spell.dbc", 1))
    eq("linear rounds half away from zero", linear({"base": 14.5, "per_level": 0.5}, 10, 0), 20)
    eq("linear honours min", linear({"base": -3, "min": 1}, 1, 0), 1)
    eq("linear AP term", linear({"base": 0, "per_ap": 0.2}, 1, 100), 20)
    eq("ap_estimate L1 human", ap_estimate(1, 0), 29)
    if os.path.isdir(data_dir):
        t = Tables(data_dir)
        hs = t.spell_fields(78)
        eq("78 name/rank", (hs["name"], hs["rank"]), ("Heroic Strike", "Rank 1"))
        eq("78 icon", (hs["icon_id"], hs["icon"]), (856, "Interface\\Icons\\Ability_Rogue_Ambush"))
        eq("78 cost (rage tenths)", (hs["cost_raw"], hs["cost"]), (150, 15.0))
        eq("100 Charge stances", t.spell_fields(100)["stances"], ["Battle Stance"])
        eq("6343 Thunder Clap category cooldown", t.spell_fields(6343)["category_cooldown_ms"], 4000)
        eq("2687 Bloodrage health cost %", t.spell_fields(2687)["cost_pct"], 20)
        eq("12320 Cruelty hidden from the spellbook", t.spell_fields(12320)["in_spellbook"], False)
        eq("12292 Sweeping Strikes in the spellbook", t.spell_fields(12292)["in_spellbook"], True)
        imp = t.talent_fields(124)
        eq("talent 124", (imp["tab"], imp["row"], imp["col"], imp["rank_spells"]), (161, 0, 0, [12282, 12663, 12664]))
        eq("talent 121 prereq", (t.talent_fields(121)["prereq_talent"], t.talent_fields(121)["prereq_rank"]), (127, 2))
        eq("warrior tabs", [x["background"] for x in t.tabs_for_class(1)], ["WarriorArms", "WarriorFury", "WarriorProtection"])
    else:
        print(f"no game data at {data_dir}: DBC checks skipped")
    for f in fails:
        print("FAIL", f)
    print(f"selftest: {len(fails)} failure(s)")
    return 1 if fails else 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--data", default=default_data_dir())
    sub = ap.add_subparsers(dest="cmd", required=True)
    v = sub.add_parser("verify")
    v.add_argument("--callings", default=DEFAULT_CALLINGS)
    v.add_argument("--out", default=DEFAULT_OUT)
    sp = sub.add_parser("spell")
    sp.add_argument("ids", type=int, nargs="+")
    se = sub.add_parser("search")
    se.add_argument("name")
    se.add_argument("--family", type=int)
    se.add_argument("--brief", action="store_true")
    sub.add_parser("selftest")
    ta = sub.add_parser("talents")
    ta.add_argument("--class", dest="cls", type=int, default=1)
    args = ap.parse_args()

    if args.cmd == "selftest":
        return selftest(args.data)
    tables = Tables(args.data)
    if args.cmd == "spell":
        print(json.dumps([tables.spell_fields(i) for i in args.ids], indent=1))
    elif args.cmd == "search":
        rows = tables.search(args.name, args.family)
        if args.brief:
            for f in rows:
                print(f"{f['id']:>6} {f['name']!r:24} {f['rank']!r:10} lvl {f['spell_level']:>2} fam {f['family']} "
                      f"cost {f['cost']} cd {f['cooldown_ms']} stances {f['stances']} icon {f['icon_id']}")
        else:
            print(json.dumps(rows, indent=1))
    elif args.cmd == "talents":
        for tab in tables.tabs_for_class(args.cls):
            print(f"== tab {tab['id']} {tab['name']} bg={tab['background']} order={tab['order_index']} row={tab['file_row']}")
            for t in tables.talents_in_tab(tab["id"]):
                print(f"  talent {t['id']:>4} r{t['row'] + 1}c{t['col'] + 1} x{t['max_rank']} {t['name']!r:28} "
                      f"ranks={t['rank_spells']} prereq={t['prereq_talent']}@{t['prereq_rank'] + 1 if t['prereq_talent'] else 0} "
                      f"req_spell={t['required_spell']} flags={t['flags']} icon={t['icon']}")
                print(f"        {t['description_rank1']}")
    elif args.cmd == "verify":
        with open(args.callings, encoding="utf-8") as fh:
            callings = json.load(fh)
        errors, dump = verify(tables, callings)
        with open(args.out, "w", encoding="utf-8", newline="\n") as fh:
            json.dump(dump, fh, indent=1, sort_keys=True)
            fh.write("\n")
        print(f"checked {len(dump['spells'])} spells; wrote {args.out}")
        for cname, tbl in dump["level_tables"].items():
            print(f"\n{cname}: levels {tbl['levels']}, naked AP ~{tbl['ap_estimate']}")
            for key, rows in tbl["skills"].items():
                for term, vals in rows.items():
                    print(f"  {key:16} {term:28} {vals}")
        print()
        for e in errors:
            print("ERROR", e)
        return 1 if errors else 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
