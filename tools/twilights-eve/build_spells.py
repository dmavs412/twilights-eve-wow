#!/usr/bin/env python3
"""Build the generated spell data from the master CSV.

    python tools/twilights-eve/build_spells.py [--csv FILE] [--out-dir DIR] [--data DIR] [--no-dbc-check]

Reads data/twilights-eve/master.csv and writes server_spells.json, the realm's view: a list of
Callings, each with its skills and talent tabs, every row carrying its realm columns, a `modelled`
flag and its effects as {kind, params, realm_only}. Effects 4 and 5 are realm-only. A row with
modelled=no carries no effects.

It also writes client_dbc_patch.json, {spells, talents}: `spells` holds one Spell.dbc override per
row with a spell_id (its client columns, effects 1 to 3 only, and its spellmods), `talents` each
talent's seat, talent_id, prereq and per-rank spell ids. The column set and the formula fields are in
docs/spell_pipeline.md.

Exits non-zero, writing nothing, on malformed params, a bad formula, realm_cost below
client_cost, a prereq or seat that does not resolve, a duplicate key, or an unknown effect kind on
a modelled row.
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import re
import sys

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
DEFAULT_CSV = os.path.join(REPO, "data", "twilights-eve", "master.csv")
DEFAULT_OUT_DIR = os.path.join(REPO, "data", "twilights-eve")
MAX_EFFECTS = 5
CLIENT_EFFECTS = 3

# Every effect kind the realm has components for (docs/bevy_guidelines.md). A modelled row naming
# any other kind fails the build; add the kind here when its components land.
KNOWN_KINDS = {
    "add_aura_stacks", "add_combo_points", "at_max_stacks", "blink", "next_cast", "root", "grant_aura", "skill_stacks", "skill_block_value_pct", "armor_pen", "aura_attack_power", "dodge_pct", "parry_pct", "block_pct", "block_value_pct", "armor_items_pct", "max_power", "consume_aura", "consume_aura_stacks", "consume_amount_pct", "consume_heal_pct", "mod_spell_damage_pct", "charge", "crit_bleed", "energize", "ground_amount_pct", "ground_duration_ms", "ground_effect", "healing_reduction",
    "incapacitate", "judgement", "mod_crit_chance", "mod_crit_damage",
    "mod_health_regen_out_of_combat", "mod_run_speed", "mod_stat", "offhand_damage_pct", "periodic_damage", "proc",
    "proc_on_cast", "proc_on_kill", "school_damage", "seal", "skill_amount_pct", "skill_cost", "skill_crit_damage_pct",
    "skill_crit_pct", "skill_duration_ms", "skill_rage", "skill_bonus_combo", "skill_extra_target", "stealth", "stun", "weapon_strike",
}
ROW_KINDS = ("skill", "talent", "calling", "talent_tab", "aura")
STATS = ("strength", "agility", "stamina", "intellect", "spirit")
ARMOUR_KINDS = ("cloth", "leather", "mail", "plate", "shield")
PROC_EVENTS = ("cast", "kill", "dodge", "finisher", "block", "parry", "crit_taken", "melee_hit", "consume", "periodic_tick")
PROC_ACTIONS = ("apply_aura", "heal_pct", "energize", "add_combo_points", "damage_attacker", "block_value_pct", "weapon_pct", "spread", "apply_aura_area", "refund_cost_pct", "absorb_cost_pct")
WEAPON_ROLLS = ("normal", "normalized")
FORMULA_FIELDS = ("base", "per_level")
OPTIONAL_FORMULA_FIELDS = ("per_ap", "min")
# What an `energize` may name in `power`.
POWERS = ("rage", "energy", "mana")
# A Calling's `class` (ChrClasses.dbc id) and the `Powers` value its bar uses: Warrior rage (1),
# Rogue energy (3), every other Class mana (0). The client patch carries it as `power_type`.
CLASS_POWER_TYPE = {1: 1, 2: 0, 3: 0, 4: 3, 5: 0, 7: 0, 8: 0, 9: 0, 11: 0}

REALM_NUMBERS = (
    "realm_cost", "realm_cooldown_ms", "realm_gcd_ms", "realm_range_min", "realm_range_max",
    "realm_radius_yd",
)
ID_COLUMNS = ("client_spell_id", "talent_id", "source_tab_id", "icon_id")
# The CSV's `client_spell_id` is the JSON's `spell_id`, so realm and client code read the same key.
JSON_KEY = {"client_spell_id": "spell_id"}
STATUSES = ("placeholder", "wip", "done")
MIN_REALM_SPELL_ID = 1_000_000
MAX_TALENT_RANKS = 5
CLIENT_NUMBERS = (
    "client_cost", "client_cooldown_ms", "client_gcd_ms", "client_cast_time_ms", "client_range_min",
    "client_range_max", "client_duration_ms", "borrowed_from_class", "borrowed_family_bit",
)
CLIENT_FLAGS = ("client_not_in_combat",)
SPELLMOD_COLUMNS = ("type", "op", "target", "value")
MAX_SPELLMODS = 5
# The 1.12.1 client's SpellModOp names (vmangos `SpellModOp`; op 13 and 25 and up do not exist).
SPELLMOD_OPS = (
    "DAMAGE", "DURATION", "THREAT", "EFFECT1", "CHARGES", "RANGE", "RADIUS", "CRITICAL_CHANCE",
    "ALL_EFFECTS", "NOT_LOSE_CASTING_TIME", "CASTING_TIME", "COOLDOWN", "EFFECT2", "COST",
    "CRIT_DAMAGE_BONUS", "RESIST_MISS_CHANCE", "JUMP_TARGETS", "CHANCE_OF_SUCCESS", "ACTIVATION_TIME",
    "EFFECT_PAST_FIRST", "CASTING_TIME_OLD", "DOT", "EFFECT3", "SPELL_BONUS_DAMAGE",
)
SCHOOLS = ("physical", "holy", "fire", "nature", "frost", "shadow", "arcane")
# Spell.dbc PreventionType: 0 none, 1 silence, 2 pacify.
PREVENTIONS = {"none": 0, "silence": 1, "pacify": 2}
# Category numbers handed to `cooldown_category` names start here; no stock 1.12.1 Spell.dbc
# Category is this high (the stock ones stop near 1,300).
CATEGORY_BASE = 10_000
# The client's aura states (vmangos `AuraStateType`), as Spell.dbc's CasterAuraState and
# TargetAuraState store them.
AURA_STATES = {"defense": 1, "health_below_20": 2, "berserking": 3, "frozen": 4, "judgement": 5,
               "hunter_parry": 7}
# Weapon item subclasses (item class 2) -> (subclass, the inventory types it sits in). Inventory
# types: 13 weapon, 14 shield, 15 ranged, 17 two-hand, 21 main hand, 22 off hand, 25 thrown,
# 26 ranged right.
_ONE_HAND_SLOTS = (13, 21, 22)
WEAPON_SUBCLASSES = {
    "one_hand_axe": (0, _ONE_HAND_SLOTS), "two_hand_axe": (1, (17,)), "bow": (2, (15,)), "gun": (3, (26,)),
    "one_hand_mace": (4, _ONE_HAND_SLOTS), "two_hand_mace": (5, (17,)), "polearm": (6, (17,)),
    "one_hand_sword": (7, _ONE_HAND_SLOTS), "two_hand_sword": (8, (17,)), "staff": (10, (17,)),
    "fist": (13, _ONE_HAND_SLOTS), "dagger": (15, _ONE_HAND_SLOTS), "thrown": (16, (25,)),
    "crossbow": (18, (26,)), "wand": (19, (26,)),
}
SHIELD = ("shield", 4, 6, (14,))  # name, item class (armour), subclass, inventory types
WEAPON_ALIASES = {
    "any_melee": ("one_hand_axe", "two_hand_axe", "one_hand_mace", "two_hand_mace", "polearm",
                  "one_hand_sword", "two_hand_sword", "staff", "fist", "dagger"),
    "any_ranged": ("bow", "gun", "crossbow", "thrown", "wand"),
    "one_hand": ("one_hand_axe", "one_hand_mace", "one_hand_sword", "fist", "dagger"),
    "two_hand": ("two_hand_axe", "two_hand_mace", "polearm", "two_hand_sword", "staff"),
}
REQUIRES_VALUES = {
    "position": ("behind", "front"), "facing": ("yes", "no"), "stealth": ("yes",), "combat": ("in", "out"),
    "target_state": ("bleeding", "stunned", "rooted", "feared", "incapacitated", "poisoned"),
    "caster_state": ("after_crit", "after_dodge", "after_parry", "after_block", "after_kill"),
    "offhand": ("weapon",), "los": ("yes", "no"),
}
USABLE_WHILE = ("stunned", "feared", "rooted", "charmed", "silenced")
GATE_COLUMNS = ("requires_weapon", "cooldown_category", "category_cooldown_ms", "prevention",
                "requires_caster_state", "requires_target_state", "requires")
COST_KINDS = ("flat", "pct_base", "all_remaining", "per_sec", "health")
# Spell.dbc PowerType of a health cost (-2 as the u32 the DBC stores).
HEALTH_POWER_TYPE = 0xFFFFFFFE
CAST_KINDS = ("instant", "cast", "channel", "next_swing", "auto_repeat")
# Spell.dbc InterruptFlags bits (vmangos SpellInterruptFlags). The patch uses the same bits for
# ChannelInterruptFlags; the client patch ticket checks them against the DBC.
INTERRUPT_FLAGS = {"move": 0x01, "pushback": 0x02, "kick": 0x08, "damage": 0x10}
DEFAULT_INTERRUPTS = ("move", "pushback", "kick")
# Spell.dbc DmgClass.
DEFENSE_TYPES = {"none": 0, "magic": 1, "melee": 2, "ranged": 3}
OUTCOMES = ("miss", "dodge", "parry", "block", "crit")
ON_CAST_KEYS = ("breaks_stealth", "starts_auto_attack", "resets_swing")
WEAPON_HANDS = ("main", "off", "both")
AREA_SHAPES = ("circle", "cone", "chain")
AREA_CENTERS = ("caster", "target", "ground")
SHAPE_COLUMNS = ("cost_kind", "cost_extra_max", "cast_kind", "channel_tick_ms", "interrupt_on", "on_cast",
                 "defense_type", "outcomes", "weapon_hand", "max_targets", "area_shape", "area_center")
AURA_ONLY_COLUMNS = ("max_stacks", "stack_rule", "unique_per", "exclusive_group", "group_scope", "dispel_type",
                     "mechanic", "aura_positive", "aura_hidden", "cancelable", "persist_death", "aura_description",
                     "break_on", "grants_form")
STACK_RULES = ("refresh", "replace", "ignore", "independent", "stack")
AURA_SCOPES = ("caster", "target")
# Spell.dbc Dispel.
DISPEL_TYPES = {"none": 0, "magic": 1, "curse": 2, "disease": 3, "poison": 4}
# Spell.dbc Mechanic (vmangos `Mechanics`; the client's own names, 5 fear, 7 root, 11 snare, 12 stun
# and 17 polymorph are confirmed against the client's table). `incapacitate` is the name the spec
# gives vanilla's knockout (14), the mechanic of Gouge and Sap.
MECHANICS = {"charm": 1, "disoriented": 2, "disarm": 3, "distract": 4, "fear": 5, "fumble": 6, "root": 7,
             "pacify": 8, "silence": 9, "sleep": 10, "snare": 11, "stun": 12, "freeze": 13, "incapacitate": 14,
             "knockout": 14, "bleed": 15, "bandage": 16, "polymorph": 17, "banish": 18, "shield": 19,
             "shackle": 20, "mount": 21, "infected": 22, "turn": 23, "horror": 24, "invulnerability": 25,
             "interrupt": 26, "daze": 27, "discovery": 28, "immune_shield": 29, "sapped": 30, "enraged": 31}
BREAK_ON_FLAGS = ("damage", "attack", "move", "stealth_break", "leave_combat")
# Spell.dbc columns of SpellFamilyFlags (low, high 32 bits), as extract_spells.py reads them.
SPELL_FAMILY_FLAGS_COLUMNS = (161, 162)


class BuildError(Exception):
    pass


# The id columns the DBC check verifies, and the table each one must exist in.
DBC_ID_COLUMNS = {
    "client_spell_id": "Spell.dbc",
    "talent_id": "Talent.dbc",
    "icon_id": "SpellIcon.dbc",
    "source_tab_id": "TalentTab.dbc",
}


def check_dbc_ids(rows: list[dict], data_dir: str | None) -> None:
    """Fail on any id the game's DBCs don't hold, naming every bad one on its own line."""
    if not data_dir:
        raise BuildError("the DBC check needs --data or WOW_DATA; pass --no-dbc-check to skip it")
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from wowdata import Chain, load_dbc

    try:
        chain = Chain(data_dir)
        tables = {name: load_dbc(chain, name) for name in set(DBC_ID_COLUMNS.values())}
    except (OSError, KeyError, ValueError) as error:
        raise BuildError(f"cannot read the DBCs in {data_dir}: {error}") from error
    bad = []
    for line, row in enumerate(rows, start=2):
        for column, table in DBC_ID_COLUMNS.items():
            text = (row.get(column) or "").strip()
            if text and (not text.isdigit() or tables[table].row(int(text)) is None):
                bad.append(f"line {line} ({row.get('calling')}/{row.get('key')}): {column} {text} is not in {table}")
    if bad:
        raise BuildError(f"{len(bad)} ids not in the DBCs:\n" + "\n".join(bad))


def read_spell_flags(data_dir: str | None, flags_path: str | None, rows: list[dict]) -> dict | None:
    """SpellFamilyFlags of each stock skill's client_spell_id, from a JSON file or Spell.dbc; None
    when neither is given."""
    if flags_path:
        with open(flags_path, encoding="utf-8") as f:
            return {str(k): int(v) for k, v in json.load(f).items()}
    if not data_dir:
        return None
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from wowdata import Chain, load_dbc

    try:
        spells = load_dbc(Chain(data_dir), "Spell.dbc")
    except (OSError, KeyError, ValueError) as error:
        raise BuildError(f"cannot read the DBCs in {data_dir}: {error}") from error
    flags = {}
    low, high = SPELL_FAMILY_FLAGS_COLUMNS
    for row in rows:
        text = row["client_spell_id"]
        if row["kind"] == "skill" and text.isdigit() and spells.row(int(text)) is not None:
            r = spells.row(int(text))
            flags[text] = (spells.u32(r, high) << 32) | spells.u32(r, low)
    return flags


def read_row_tables(data_dir: str | None) -> dict:
    """SpellRange, SpellCastTimes and SpellDuration as plain tuples, for `resolve_dbc_rows`."""
    if not data_dir:
        raise BuildError("resolving range, cast time and duration needs --data or WOW_DATA; "
                         "pass --no-dbc-check to leave them as values")
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from wowdata import Chain, load_dbc

    try:
        chain = Chain(data_dir)
        ranges, times, durations = (load_dbc(chain, n) for n in ("SpellRange.dbc", "SpellCastTimes.dbc", "SpellDuration.dbc"))
    except (OSError, KeyError, ValueError) as error:
        raise BuildError(f"cannot read the DBCs in {data_dir}: {error}") from error
    return {
        "ranges": [(ranges.u32(i, 0), ranges.f32(i, 1), ranges.f32(i, 2)) for i in range(ranges.nrec)],
        "cast_times": [(times.u32(i, 0), times.i32(i, 1), times.i32(i, 2)) for i in range(times.nrec)],
        "durations": [(durations.u32(i, 0), durations.i32(i, 1)) for i in range(durations.nrec)],
    }


def resolve_dbc_rows(patch: dict, ranges: list, cast_times: list, durations: list) -> None:
    """Replace each override's range, cast time and duration values with the index of the stock
    row that holds them (the lowest id on a tie). The client overwrites the index. A value no
    stock row holds fails the build."""
    for entry in patch["spells"]:
        fields = entry["fields"]
        where = f"{entry['calling']}/{entry['key']}" if "key" in entry else str(entry["spell_id"])
        if "range_min" in fields or "range_max" in fields:
            low, high = float(fields.pop("range_min", 0)), float(fields.pop("range_max", 0))
            fields["range_index"] = _lowest(
                (i for i, a, b in ranges if abs(a - low) < 1e-3 and abs(b - high) < 1e-3),
                f"{where}: no SpellRange.dbc row for range {low}-{high}")
        if "cast_time_ms" in fields:
            ms = fields.pop("cast_time_ms")
            fields["cast_time_index"] = _lowest(
                (i for i, base, per_level in cast_times if base == ms and per_level == 0),
                f"{where}: no SpellCastTimes.dbc row for cast time {ms} ms")
        if "duration_ms" in fields:
            ms = fields.pop("duration_ms")
            fields["duration_index"] = _lowest(
                (i for i, base in durations if base == ms),
                f"{where}: no SpellDuration.dbc row for duration {ms} ms")


def _lowest(ids, message: str) -> int:
    found = min(ids, default=None)
    if found is None:
        raise BuildError(message)
    return found


def number(text: str, where: str) -> int | float:
    try:
        value = float(text)
    except ValueError:
        raise BuildError(f"{where}: {text!r} is not a number") from None
    return int(value) if value.is_integer() and "." not in text else value


def scalar(text: str) -> object:
    if text in ("yes", "no"):
        return text == "yes"
    try:
        return number(text, "")
    except BuildError:
        return text


def param_value(text: str) -> object:
    if "|" in text:
        return [scalar(part) for part in text.split("|")]
    return scalar(text)


def parse_params(text: str, where: str) -> dict:
    params: dict = {}
    for pair in text.split(";") if text else []:
        key, sep, value = pair.partition("=")
        if not sep or not key or not value or any(not part for part in key.split(".")):
            raise BuildError(f"{where}: malformed param {pair!r}")
        node = params
        *parents, leaf = key.split(".")
        for part in parents:
            node = node.setdefault(part, {})
            if not isinstance(node, dict):
                raise BuildError(f"{where}: {key!r} nests under a value")
        if leaf in node:
            raise BuildError(f"{where}: {key!r} given twice")
        node[leaf] = param_value(value)
    check_formulas(params, where)
    return params


def check_formulas(node: dict, where: str, path: str = "") -> None:
    """A dict holding base or per_level is a formula; per_ap and min are optional."""
    for key, value in node.items():
        if isinstance(value, dict):
            check_formulas(value, where, f"{path}{key}.")
    if not any(field in node for field in FORMULA_FIELDS):
        return
    for field in FORMULA_FIELDS:
        if field not in node:
            raise BuildError(f"{where}: formula {path.rstrip('.')!r} lacks {field}")
    for field, value in node.items():
        if field not in (*FORMULA_FIELDS, *OPTIONAL_FORMULA_FIELDS):
            raise BuildError(f"{where}: formula {path.rstrip('.')!r} has stray field {field!r}")
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            raise BuildError(f"{where}: formula {path}{field} is {value!r}, not a number")


def whole(text: str, where: str) -> int:
    value = number(text, where)
    if not isinstance(value, int):
        raise BuildError(f"{where}: {text!r} is not a whole number")
    return value


def build_calling(row: dict, where: str) -> dict:
    """A calling row: the Calling's name, tier, Tier Cap, Tier Bonus, form and armour."""
    out: dict = {"name": row["name"]}
    # A row with no tier only names the Calling; the realm reads the Calling's rest elsewhere.
    if row["tier"] or row["tier_cap"]:
        for column in ("tier", "tier_cap"):
            if not row[column]:
                raise BuildError(f"{where}: a calling row needs {column}")
        out["tier"] = whole(row["tier"], f"{where} tier")
        out["tier_cap"] = whole(row["tier_cap"], f"{where} tier_cap")
    if row["tier_bonus"]:
        bonus = parse_params(row["tier_bonus"], f"{where} tier_bonus")
        for stat, value in bonus.items():
            if stat not in STATS or isinstance(value, (dict, list, bool)) or not isinstance(value, (int, float)):
                raise BuildError(f"{where}: tier_bonus {stat!r} is not a stat with a number (stats: {', '.join(STATS)})")
        out["tier_bonus"] = bonus
    if row["shapeshift_form"]:
        out["shapeshift_form"] = whole(row["shapeshift_form"], f"{where} shapeshift_form")
    if row["armour"]:
        kinds = row["armour"].split("|")
        for kind in kinds:
            if kind not in ARMOUR_KINDS:
                raise BuildError(f"{where}: armour {kind!r} is not one of {', '.join(ARMOUR_KINDS)}")
        out["armour"] = kinds
    return out


def build_talent_tab(row: dict, where: str) -> dict:
    for column in ("tab_id", "background", "tab_order"):
        if not row[column]:
            raise BuildError(f"{where}: a talent_tab row needs {column}")
    return {"name": row["name"], "id": whole(row["tab_id"], f"{where} tab_id"), "background": row["background"],
            "order": whole(row["tab_order"], f"{where} tab_order"), "talents": []}


def yes_no(text: str, where: str, column: str) -> bool:
    return one_of(text, ("yes", "no"), where, column) == "yes"


def parse_break_on(text: str, where: str) -> dict:
    """The aura's `break_on` cell: `key=yes` pairs joined by `;` from `BREAK_ON_FLAGS`, or
    `damage_threshold=N` (damage taken in one go). `damage` and `damage_threshold` are exclusive."""
    out: dict = {}
    for pair in text.split(";"):
        key, sep, value = pair.partition("=")
        if not sep or not key or not value:
            raise BuildError(f"{where} break_on: malformed pair {pair!r}")
        if key in out:
            raise BuildError(f"{where} break_on: {key!r} given twice")
        if key == "damage_threshold":
            out[key] = positive_whole(value, where, "break_on damage_threshold")
        elif key in BREAK_ON_FLAGS:
            if value != "yes":
                raise BuildError(f"{where} break_on: {key} is {value!r}, want yes")
            out[key] = True
        else:
            raise BuildError(f"{where} break_on: unknown key {key!r}, want one of "
                             f"{', '.join([*BREAK_ON_FLAGS, 'damage_threshold'])}")
    if "damage" in out and "damage_threshold" in out:
        raise BuildError(f"{where} break_on: damage and damage_threshold are exclusive")
    return out


def build_aura_columns(row: dict, where: str) -> dict:
    """The aura-only columns with their defaults. Positive follows the row's target: an enemy
    target makes a debuff, anything else (or none) a buff; cancelable follows positive."""
    out: dict = {}
    out["max_stacks"] = positive_whole(row["max_stacks"], where, "max_stacks") if row["max_stacks"] else 1
    out["stack_rule"] = one_of(row["stack_rule"] or "refresh", STACK_RULES, where, "stack_rule")
    out["unique_per"] = one_of(row["unique_per"] or "caster", AURA_SCOPES, where, "unique_per")
    if row["group_scope"] and not row["exclusive_group"]:
        raise BuildError(f"{where} group_scope: needs exclusive_group")
    if row["exclusive_group"]:
        out["exclusive_group"] = row["exclusive_group"]
    out["group_scope"] = one_of(row["group_scope"] or "caster", AURA_SCOPES, where, "group_scope")
    out["dispel_type"] = one_of(row["dispel_type"] or "none", DISPEL_TYPES, where, "dispel_type")
    if row["mechanic"]:
        out["mechanic"] = one_of(row["mechanic"], MECHANICS, where, "mechanic")
    positive = yes_no(row["aura_positive"], where, "aura_positive") if row["aura_positive"] else row["target"] != "enemy"
    out["aura_positive"] = positive
    out["aura_hidden"] = yes_no(row["aura_hidden"], where, "aura_hidden") if row["aura_hidden"] else False
    out["cancelable"] = yes_no(row["cancelable"], where, "cancelable") if row["cancelable"] else positive
    out["persist_death"] = yes_no(row["persist_death"], where, "persist_death") if row["persist_death"] else False
    if row["aura_description"]:
        out["aura_description"] = row["aura_description"]
    if row["break_on"]:
        out["break_on"] = parse_break_on(row["break_on"], where)
    if row["grants_form"]:
        out["grants_form"] = positive_whole(row["grants_form"], where, "grants_form")
    return out


def aura_override(row: dict, built: dict, where: str) -> dict | None:
    """The Spell.dbc fields an aura row sets on its borrowed spell: dispel type, mechanic and aura
    description, each only when its cell is set. None when it sets none, or is hidden."""
    if "spell_id" not in built or built.get("client_visible") is False:
        return None
    fields: dict = {}
    if row["dispel_type"]:
        fields["dispel_type"] = DISPEL_TYPES[built["dispel_type"]]
    if row["mechanic"]:
        fields["mechanic"] = MECHANICS[built["mechanic"]]
    if row["aura_description"]:
        fields["aura_description"] = row["aura_description"]
    if not fields:
        return None
    return {"spell_id": built["spell_id"], "calling": row["calling"], "key": row["key"],
            "rank": number(row["rank"], f"{where} rank"), "fields": fields}


def merge_override(overrides: list[dict], extra: dict, where: str) -> None:
    """Add an aura's override, folding it into the override of the same spell id (a skill that is
    also the spell its aura shows as): one Spell.dbc row, one override. Two rows that set one field
    to different values fail."""
    for entry in overrides:
        if entry["spell_id"] == extra["spell_id"]:
            for field, value in extra["fields"].items():
                if entry["fields"].setdefault(field, value) != value:
                    raise BuildError(f"{where}: {field} {value!r} differs from {entry['fields'][field]!r} on "
                                     f"{entry['calling']}/{entry['key']}, which also overrides spell {extra['spell_id']}")
            return
    overrides.append(extra)


def build_aura(row: dict, built: dict, where: str) -> dict:
    """An aura row: a borrowed spell id, a duration, charges and effects, granted by key."""
    out = {"key": built["key"], "name": built["name"]}
    for column in ("spell_id", "client_visible", "icon_id", "status", "school", "prevention", "target"):
        if column in built:
            out[column] = built[column]
    out.update(build_aura_columns(row, where))
    for column in ("duration_ms", "charges"):
        if row[column]:
            out[column] = number(row[column], f"{where} {column}")
    out["modelled"] = built["modelled"]
    out["effects"] = built["effects"]
    out["description"] = built["description"]
    return out


def check_proc(params: dict, auras: set[str], where: str) -> None:
    if params.get("on") not in PROC_EVENTS:
        raise BuildError(f"{where}: proc on {params.get('on')!r}, want one of {', '.join(PROC_EVENTS)}")
    if not any(a in params for a in PROC_ACTIONS):
        raise BuildError(f"{where}: a proc needs at least one of {', '.join(PROC_ACTIONS)}")
    if "apply_aura" in params and "heal_pct" in params:
        raise BuildError(f"{where}: a proc takes apply_aura or heal_pct, not both")
    if "while_aura" in params and params["while_aura"] not in auras:
        raise BuildError(f"{where}: while_aura {params['while_aura']!r} is not an aura row of this Calling")
    for action in ("apply_aura", "apply_aura_area"):
        if action not in params:
            continue
        keys = params[action] if isinstance(params[action], list) else [params[action]]
        for key in keys:
            if key not in auras:
                raise BuildError(f"{where}: {action} {key!r} is not an aura row of this Calling")
    if params.get("at_full_power", "absorb") != "absorb":
        raise BuildError(f"{where}: at_full_power is {params['at_full_power']!r}, want absorb")
    if "at_full_power" in params and "absorb_cost_pct" not in params:
        raise BuildError(f"{where}: at_full_power=absorb needs absorb_cost_pct")
    if "requires_target_aura" in params and params["requires_target_aura"] not in {*auras, "slowed"}:
        raise BuildError(f"{where}: requires_target_aura {params['requires_target_aura']!r} is not an aura row or slowed")
    if "apply_aura_area" in params:
        if params.get("center", "origin") != "origin":
            raise BuildError(f"{where}: apply_aura_area center is {params['center']!r}, want origin")
        radius = params.get("radius_yd")
        if isinstance(radius, bool) or not isinstance(radius, (int, float)) or radius <= 0:
            raise BuildError(f"{where}: apply_aura_area needs a positive radius_yd, has {radius!r}")


def check_apply_aura(kind: str, params: dict, auras: dict[str, set[str]], where: str) -> None:
    """An effect other than a proc that applies a lasting effect names one aura row of the Calling,
    and that aura must hold an effect of the same kind: the realm takes its params from there."""
    key = params["apply_aura"]
    if not isinstance(key, str):
        raise BuildError(f"{where}: apply_aura names one aura key, has {key!r}")
    if key not in auras:
        raise BuildError(f"{where}: apply_aura {key!r} is not an aura row of this Calling")
    if kind not in auras[key]:
        raise BuildError(f"{where}: aura {key!r} has no {kind} effect for apply_aura to apply")


def check_vocabulary(kind: str, params: dict, where: str) -> None:
    """The CSV uses the realm's names: normal or normalized weapon rolls, a boolean unavoidable."""
    if kind == "weapon_strike":
        if params.get("weapon") not in WEAPON_ROLLS:
            raise BuildError(f"{where}: weapon is {params.get('weapon')!r}, want {' or '.join(WEAPON_ROLLS)}")
        if "unavoidable" in params:
            raise BuildError(f"{where}: the unavoidable param is gone; write dodge=no;parry=no in the outcomes column")
        if params.get("crit_if_target_in", "ground_effect") != "ground_effect":
            raise BuildError(f"{where}: crit_if_target_in is {params['crit_if_target_in']!r}, want ground_effect")


def check_kind_params(kind: str, params: dict, where: str) -> None:
    """The params a kind cannot do without: energize's power and amount, add_combo_points' positive
    whole amount, and per_combo (on any kind) as a formula block."""
    if kind == "energize":
        if params.get("power") not in POWERS:
            raise BuildError(f"{where}: energize needs power={'|'.join(POWERS)}, has {params.get('power')!r}")
        if not isinstance(params.get("amount"), dict):
            raise BuildError(f"{where}: energize lacks an amount formula")
    if kind == "add_combo_points":
        amount = params.get("amount")
        if isinstance(amount, bool) or not isinstance(amount, int) or amount <= 0:
            raise BuildError(f"{where}: add_combo_points needs a positive whole amount, has {amount!r}")
    if kind == "add_aura_stacks":
        stacks = params.get("stacks", 1)
        if isinstance(stacks, bool) or not isinstance(stacks, int) or stacks <= 0:
            raise BuildError(f"{where}: add_aura_stacks needs a positive whole stacks, has {stacks!r}")
    if kind == "stealth" and not isinstance(params.get("opener_crit", False), bool):
        raise BuildError(f"{where}: stealth opener_crit is yes or no, has {params['opener_crit']!r}")
    if kind == "ground_effect":
        if not isinstance(params.get("amount"), dict):
            raise BuildError(f"{where}: ground_effect lacks an amount formula (amount.base, amount.per_level)")
        for field in ("radius_yd", "tick_ms", "duration_ms"):
            value = params.get(field)
            if isinstance(value, bool) or not isinstance(value, (int, float)) or value <= 0:
                raise BuildError(f"{where}: ground_effect needs a positive {field}, has {value!r}")
    if kind == "consume_aura_stacks" and not isinstance(params.get("per_stack"), dict):
        raise BuildError(f"{where}: consume_aura_stacks lacks a per_stack formula (per_stack.base, per_stack.per_level)")
    for block in ("per_combo", "duration_per_combo"):
        if block in params and not isinstance(params[block], dict):
            raise BuildError(f"{where}: {block} is a formula block ({block}.base, {block}.per_level)")


def class_id(text: str, where: str) -> int:
    """The row's `class`: a 1.12.1 class id the realm has tables for."""
    if not text:
        raise BuildError(f"{where}: class is required (the Calling's 1.12.1 class id)")
    value = number(text, f"{where} class")
    if not isinstance(value, int) or value not in CLASS_POWER_TYPE:
        raise BuildError(f"{where}: class {text!r} is not one of {sorted(CLASS_POWER_TYPE)}")
    return value


def client_visible(row: dict, where: str) -> bool:
    """The row's `client_visible` (empty means yes), with the hidden-row rules checked."""
    flag = row["client_visible"]
    if flag not in ("", "yes", "no"):
        raise BuildError(f"{where}: client_visible is {flag!r}, want yes or no")
    if flag != "no":
        if row["realm_spell_id"]:
            raise BuildError(f"{where}: realm_spell_id is set on a row that is visible to the client")
        return True
    if not row["realm_spell_id"]:
        raise BuildError(f"{where}: a client_visible=no row needs realm_spell_id")
    realm = whole(row["realm_spell_id"], f"{where} realm_spell_id")
    if realm < MIN_REALM_SPELL_ID:
        raise BuildError(f"{where}: realm_spell_id {realm} is below {MIN_REALM_SPELL_ID}")
    for column in row:
        if column.startswith("client_") and column != "client_visible" and row[column]:
            raise BuildError(f"{where}: a client_visible=no row sets {column}")
    return False


def max_rank(row: dict, where: str) -> int:
    """A talent rank row's `max_rank`: a whole number from 1 to 5."""
    if not row["max_rank"]:
        raise BuildError(f"{where}: a talent rank row needs max_rank")
    value = whole(row["max_rank"], f"{where} max_rank")
    if not 1 <= value <= MAX_TALENT_RANKS:
        raise BuildError(f"{where}: max_rank {value} is not 1 to {MAX_TALENT_RANKS}")
    return value


def parse_requires(text: str, where: str) -> dict:
    """The realm-only `requires` cell: `key=value` pairs joined by `;`, each key and value from the
    spec's table. yes/no become booleans, `usable_while` a list, `window_ms` a whole number."""
    out: dict = {}
    for pair in text.split(";"):
        key, sep, value = pair.partition("=")
        if not sep or not key or not value:
            raise BuildError(f"{where} requires: malformed pair {pair!r}")
        if key in out:
            raise BuildError(f"{where} requires: {key!r} given twice")
        if key == "usable_while":
            states = value.split("|")
            for state in states:
                if state not in USABLE_WHILE:
                    raise BuildError(f"{where} requires: usable_while {state!r} is not one of {', '.join(USABLE_WHILE)}")
            out[key] = states
        elif key == "window_ms":
            out[key] = whole(value, f"{where} requires window_ms")
            if out[key] <= 0:
                raise BuildError(f"{where} requires: window_ms must be above 0")
        elif key in REQUIRES_VALUES:
            if value not in REQUIRES_VALUES[key]:
                raise BuildError(f"{where} requires: {key} is {value!r}, want one of {', '.join(REQUIRES_VALUES[key])}")
            out[key] = value == "yes" if value in ("yes", "no") else value
        else:
            raise BuildError(f"{where} requires: unknown key {key!r}, want one of "
                             f"{', '.join([*REQUIRES_VALUES, 'usable_while', 'window_ms'])}")
    if ("window_ms" in out) != ("caster_state" in out):
        raise BuildError(f"{where} requires: caster_state and window_ms go together")
    return out


def weapon_requirement(text: str, where: str) -> tuple[list[str], dict]:
    """`requires_weapon` names (joined by `|`, aliases allowed) -> the names as written and the
    Spell.dbc equipped item class, subclass mask and inventory type mask."""
    names = text.split("|")
    subclasses: dict[str, tuple[int, tuple]] = {}
    shield = False
    for name in names:
        if name in WEAPON_ALIASES:
            subclasses.update({n: WEAPON_SUBCLASSES[n] for n in WEAPON_ALIASES[name]})
        elif name in WEAPON_SUBCLASSES:
            subclasses[name] = WEAPON_SUBCLASSES[name]
        elif name == SHIELD[0]:
            shield = True
        else:
            wanted = [*WEAPON_SUBCLASSES, SHIELD[0], *WEAPON_ALIASES]
            raise BuildError(f"{where} requires_weapon: {name!r} is not one of {', '.join(wanted)}")
    if shield and subclasses:
        raise BuildError(f"{where} requires_weapon: shield (armour) cannot be mixed with weapons in one cell")
    if shield:
        return names, {"item_class": SHIELD[1], "subclass_mask": 1 << SHIELD[2],
                       "inventory_mask": sum(1 << t for t in SHIELD[3])}
    return names, {"item_class": 2, "subclass_mask": sum(1 << sub for sub, _ in subclasses.values()),
                   "inventory_mask": sum(1 << t for t in {t for _, slots in subclasses.values() for t in slots})}


def build_gates(row: dict, kind: str, where: str) -> dict:
    """The gate columns: school and prevention with their defaults, and whatever of weapon,
    cooldown category, aura states and `requires` the row sets. An aura row carries only a school."""
    school = row["school"] or "physical"
    if school not in SCHOOLS:
        raise BuildError(f"{where} school: {school!r} is not one of {', '.join(SCHOOLS)}")
    prevention = row["prevention"] or ("pacify" if school == "physical" else "silence")
    if prevention not in PREVENTIONS:
        raise BuildError(f"{where} prevention: {prevention!r} is not one of {', '.join(PREVENTIONS)}")
    out: dict = {"school": school, "prevention": prevention}
    if kind == "aura":
        for column in GATE_COLUMNS:
            if row[column]:
                raise BuildError(f"{where} {column}: set on an aura row; only skill and talent rows carry gates")
        return out
    if row["requires_weapon"]:
        out["requires_weapon"] = weapon_requirement(row["requires_weapon"], where)[0]
    if bool(row["cooldown_category"]) != bool(row["category_cooldown_ms"]):
        raise BuildError(f"{where} cooldown_category: needs category_cooldown_ms, and the other way round")
    if row["cooldown_category"]:
        out["cooldown_category"] = row["cooldown_category"]
        out["category_cooldown_ms"] = whole(row["category_cooldown_ms"], f"{where} category_cooldown_ms")
        if out["category_cooldown_ms"] <= 0:
            raise BuildError(f"{where} category_cooldown_ms: must be above 0")
    for column in ("requires_caster_state", "requires_target_state"):
        if row[column]:
            if row[column] not in AURA_STATES:
                raise BuildError(f"{where} {column}: {row[column]!r} is not one of {', '.join(AURA_STATES)}")
            out[column] = row[column]
    requires = parse_requires(row["requires"], where) if row["requires"] else {}
    if requires:
        out["requires"] = requires
    if row["client_not_in_combat"] == "yes" and requires.get("combat") != "out":
        raise BuildError(f"{where} client_not_in_combat: yes needs requires: combat=out, or the realm is looser than the client")
    if requires.get("combat") == "out":
        out["requires_out_of_combat"] = True
    return out


def yes_no_pairs(text: str, keys: tuple, where: str, column: str) -> dict[str, bool]:
    """A cell of `key=yes|no` pairs joined by `;`, each key from `keys`."""
    out: dict[str, bool] = {}
    for pair in text.split(";"):
        key, sep, value = pair.partition("=")
        if not sep or not key or not value:
            raise BuildError(f"{where} {column}: malformed pair {pair!r}")
        if key not in keys:
            raise BuildError(f"{where} {column}: unknown key {key!r}, want one of {', '.join(keys)}")
        if key in out:
            raise BuildError(f"{where} {column}: {key!r} given twice")
        if value not in ("yes", "no"):
            raise BuildError(f"{where} {column}: {key} is {value!r}, want yes or no")
        out[key] = value == "yes"
    return out


def positive_whole(text: str, where: str, column: str) -> int:
    value = whole(text, f"{where} {column}")
    if value <= 0:
        raise BuildError(f"{where} {column}: must be above 0")
    return value


def one_of(text: str, allowed, where: str, column: str) -> str:
    if text not in allowed:
        raise BuildError(f"{where} {column}: {text!r} is not one of {', '.join(allowed)}")
    return text


def build_shape(row: dict, kind: str, school: str, hidden: bool, where: str) -> dict:
    """Cost kind and cast shape with their defaults. An aura row carries none of these columns. A
    hidden row has no client cast time or duration, so `cast` and `channel` skip those checks."""
    if kind == "aura":
        for column in SHAPE_COLUMNS:
            if row[column]:
                raise BuildError(f"{where} {column}: set on an aura row; only skill and talent rows carry cost and cast shape")
        return {}
    out: dict = {"cost_kind": one_of(row["cost_kind"] or "flat", COST_KINDS, where, "cost_kind")}
    if row["cost_extra_max"]:
        if out["cost_kind"] != "all_remaining":
            raise BuildError(f"{where} cost_extra_max: only with cost_kind=all_remaining")
        out["cost_extra_max"] = positive_whole(row["cost_extra_max"], where, "cost_extra_max")

    cast_kind = out["cast_kind"] = one_of(row["cast_kind"] or "instant", CAST_KINDS, where, "cast_kind")
    if cast_kind == "cast" and not hidden and not (row["client_cast_time_ms"] and number(row["client_cast_time_ms"], where) > 0):
        raise BuildError(f"{where} cast_kind: cast needs a client_cast_time_ms above 0")
    if cast_kind == "cast" and row["client_cast_time_ms"]:
        # The realm runs the same bar the client shows.
        out["cast_time_ms"] = int(number(row["client_cast_time_ms"], where))
    if cast_kind == "channel":
        if not hidden and not (row["client_duration_ms"] and number(row["client_duration_ms"], where) > 0):
            raise BuildError(f"{where} cast_kind: channel needs a client_duration_ms above 0")
        if not row["channel_tick_ms"]:
            raise BuildError(f"{where} cast_kind: channel needs channel_tick_ms")
    if row["channel_tick_ms"]:
        if cast_kind != "channel":
            raise BuildError(f"{where} channel_tick_ms: only with cast_kind=channel")
        out["channel_tick_ms"] = positive_whole(row["channel_tick_ms"], where, "channel_tick_ms")

    if not row["interrupt_on"]:
        out["interrupt_on"] = list(DEFAULT_INTERRUPTS) if cast_kind in ("cast", "channel") else []
    elif row["interrupt_on"] == "none":
        out["interrupt_on"] = []
    else:
        names = row["interrupt_on"].split("|")
        for name in names:
            one_of(name, (*INTERRUPT_FLAGS, "none"), where, "interrupt_on")
        if "none" in names or len(set(names)) != len(names):
            raise BuildError(f"{where} interrupt_on: {row['interrupt_on']!r} repeats a name or mixes none with others")
        out["interrupt_on"] = names

    if row["defense_type"]:
        defense = one_of(row["defense_type"], DEFENSE_TYPES, where, "defense_type")
    elif row["target"] == "enemy":
        defense = "melee" if school == "physical" else "magic"
    else:
        defense = "none"
    out["defense_type"] = defense
    on_cast = {"breaks_stealth": True, "starts_auto_attack": defense == "melee" and row["target"] == "enemy",
               "resets_swing": False}
    if row["on_cast"]:
        on_cast.update(yes_no_pairs(row["on_cast"], ON_CAST_KEYS, where, "on_cast"))
    out["on_cast"] = on_cast
    if row["outcomes"]:
        out["outcomes"] = yes_no_pairs(row["outcomes"], OUTCOMES, where, "outcomes")
    out["weapon_hand"] = one_of(row["weapon_hand"] or "main", WEAPON_HANDS, where, "weapon_hand")

    area = bool(row["area_shape"] or (row["realm_radius_yd"] and number(row["realm_radius_yd"], where) > 0))
    if row["area_shape"]:
        out["area_shape"] = one_of(row["area_shape"], AREA_SHAPES, where, "area_shape")
    if row["area_center"]:
        if not row["area_shape"]:
            raise BuildError(f"{where} area_center: needs area_shape")
        out["area_center"] = one_of(row["area_center"], AREA_CENTERS, where, "area_center")
    if row["max_targets"]:
        out["max_targets"] = positive_whole(row["max_targets"], where, "max_targets")
        if out["max_targets"] > 1 and not area:
            raise BuildError(f"{where} max_targets: {out['max_targets']} needs area_shape or realm_radius_yd")
    else:
        # Unlimited (null) for an area row, one target otherwise.
        out["max_targets"] = None if area else 1
    return out


def build_row(row: dict, where: str, auras: dict[str, set[str]] = {}) -> dict:
    modelled = row["modelled"]
    if modelled not in ("yes", "no"):
        raise BuildError(f"{where}: modelled is {modelled!r}, want yes or no")
    out: dict = {"key": row["key"], "name": row["name"], "rank": number(row["rank"], f"{where} rank"),
                 "class": class_id(row["class"], where)}
    if row["status"] not in STATUSES:
        raise BuildError(f"{where}: status is {row['status']!r}, want one of {', '.join(STATUSES)}")
    out["status"] = row["status"]
    hidden = not client_visible(row, where)
    for column in ID_COLUMNS:
        if row[column]:
            key = JSON_KEY.get(column, column)
            out[key] = number(row[column], f"{where} {column}")
            if not isinstance(out[key], int):
                raise BuildError(f"{where} {column}: {row[column]!r} is not a whole id")
    if hidden:
        # A hidden row's one number is its realm id; the realm reads it as `spell_id`.
        out["spell_id"] = whole(row["realm_spell_id"], f"{where} realm_spell_id")
        out["client_visible"] = False
    for column in REALM_NUMBERS:
        if row[column]:
            out[column] = number(row[column], f"{where} {column}")
    if row["target"]:
        out["target"] = row["target"]
    out.update(build_gates(row, row["kind"], where))
    out.update(build_shape(row, row["kind"], out["school"], hidden, where))
    outcomes = out.get("outcomes", {})
    if row["realm_cost"] and row["client_cost"]:
        realm, client = number(row["realm_cost"], where), number(row["client_cost"], where)
        if realm < client:
            raise BuildError(f"{where}: realm_cost {realm} is below client_cost {client}")

    effects = []
    for i in range(1, MAX_EFFECTS + 1):
        kind, params = row[f"effect{i}_kind"], row[f"effect{i}_params"]
        if not kind:
            if params:
                raise BuildError(f"{where}: effect{i}_params set with no effect{i}_kind")
            continue
        here = f"{where} effect{i}"
        parsed = parse_params(params, here)
        if modelled == "yes":
            if kind not in KNOWN_KINDS:
                raise BuildError(f"{here}: unknown effect kind {kind!r}")
            check_kind_params(kind, parsed, here)
        if kind in ("add_aura_stacks", "consume_aura_stacks", "grant_aura", "consume_aura"):
            if parsed.get("aura") not in auras:
                raise BuildError(f"{here}: aura {parsed.get('aura')!r} is not an aura row of this Calling")
        if kind == "skill_amount_pct":
            if parsed.get("applies_to", "both") not in ("hit", "tick", "both"):
                raise BuildError(f"{here}: applies_to is {parsed['applies_to']!r}, want hit, tick or both")
            if "requires_target_aura" in parsed and parsed["requires_target_aura"] not in auras:
                raise BuildError(f"{here}: requires_target_aura {parsed['requires_target_aura']!r} is not an aura row of this Calling")
        if kind == "proc":
            check_proc(parsed, auras, here)
        elif "apply_aura" in parsed:
            check_apply_aura(kind, parsed, auras, here)
        check_vocabulary(kind, parsed, here)
        if kind == "weapon_strike" and outcomes.get("dodge") is False and outcomes.get("parry") is False:
            # The realm reads an unavoidable strike from the effect; the outcomes cell is its one source.
            parsed = {"weapon": parsed["weapon"], "unavoidable": True,
                      **{k: v for k, v in parsed.items() if k != "weapon"}}
        if kind in ("school_damage", "weapon_strike"):
            if "school" in parsed:
                raise BuildError(f"{here}: {kind} takes no school param; set the row's school column")
            # The realm reads the school from the effect's params; the row's school is the one source.
            if out["school"] != "physical":
                parsed["school"] = out["school"]
        effects.append({"kind": kind, "params": parsed, "realm_only": i > CLIENT_EFFECTS})
    out["modelled"] = modelled == "yes"
    out["effects"] = effects if out["modelled"] else []
    out["description"] = row["description"]
    return out


def resolve_amount_of(calling: dict) -> None:
    """A `consume_aura`'s `amount_of=skill` becomes that skill's `school_damage` amount formula and
    school, so the realm reads the hit it is a multiple of without looking the skill up."""
    for skill in calling["skills"]:
        for effect in skill["effects"]:
            name = effect["params"].pop("amount_of", None) if effect["kind"] == "consume_aura" else None
            if name is None:
                continue
            where = f"{calling['key']}/{skill['key']} consume_aura"
            hit = next((e for s in calling["skills"] if s["key"] == name and s["modelled"]
                        for e in s["effects"] if e["kind"] == "school_damage"), None)
            if hit is None:
                raise BuildError(f"{where}: amount_of {name!r} is not a modelled skill with a school_damage")
            effect["params"]["amount"] = hit["params"]["amount"]
            if "school" in hit["params"]:
                effect["params"]["school"] = hit["params"]["school"]


def resolve_next_cast(calling: dict, masks: dict[str, int | None]) -> None:
    """A `next_cast` aura names skills of its Calling; the realm gets their `SpellFamilyFlags`
    bits as `family_bits`, for the client's cast bar: each its skill's lowest bit no other skill
    shares. A skill whose flags are unknown (no --data) or all shared adds none. An `at_max_stacks` aura names the aura row it becomes."""
    keys = {s["key"] for s in calling["skills"]}
    auras = {a["key"] for a in calling["auras"]}
    for aura in calling["auras"]:
        where = f"{calling['key']}/{aura['key']}"
        for effect in aura.get("effects", []):
            params = effect["params"]
            if effect["kind"] == "at_max_stacks" and params.get("becomes") not in auras:
                raise BuildError(f"{where} at_max_stacks: becomes {params.get("becomes")!r} is not an aura row")
            if effect["kind"] != "next_cast":
                continue
            skills = params.get("skill")
            skills = skills if isinstance(skills, list) else [skills]
            bits = []
            for key in skills:
                if key not in keys:
                    raise BuildError(f"{where} next_cast: skill {key!r} is not a skill of {calling['key']}")
                # The lowest bit of the skill's mask that no other skill of the Calling shares.
                others = 0
                for other, other_mask in masks.items():
                    if other != key and other_mask:
                        others |= other_mask
                own = (masks.get(key) or 0) & ~others
                if own:
                    bits.append((own & -own).bit_length() - 1)
            params["family_bits"] = bits


def skill_masks(rows: list[dict], spell_flags: dict | None) -> dict[str, dict[str, int | None]]:
    """Per calling, each skill key's SpellFamilyFlags mask: a borrowed skill's own bit, a stock
    skill's from its Spell.dbc row. None where the flags are unknown (no --data or --spell-flags)."""
    masks: dict[str, dict[str, int | None]] = {}
    for line, row in enumerate(rows, start=2):
        if row["kind"] != "skill":
            continue
        mine = masks.setdefault(row["calling"], {})
        if mine.get(row["key"]):
            continue
        if row["borrowed_family_bit"]:
            bit = whole(row["borrowed_family_bit"], f"line {line} ({row['calling']}/{row['key']}) borrowed_family_bit")
            mine[row["key"]] = 1 << bit
        elif spell_flags is not None and row["client_spell_id"]:
            mine[row["key"]] = spell_flags.get(row["client_spell_id"])
        else:
            mine.setdefault(row["key"], None)
    return masks


def build_spellmods(row: dict, where: str, masks: dict[str, int | None], flags_known: bool) -> list[dict]:
    """The row's spellmod groups, each target resolved from skill keys to a family mask."""
    spellmods = []
    for i in range(1, MAX_SPELLMODS + 1):
        values = {c: row[f"spellmod{i}_{c}"] for c in SPELLMOD_COLUMNS}
        if not any(values.values()):
            continue
        if not all(values.values()):
            raise BuildError(f"{where}: spellmod{i} needs all of {', '.join(SPELLMOD_COLUMNS)}")
        if values["type"] not in ("flat", "pct"):
            raise BuildError(f"{where}: spellmod{i}_type is {values['type']!r}, want flat or pct")
        if values["op"] not in SPELLMOD_OPS:
            raise BuildError(f"{where}: spellmod{i}_op is {values['op']!r}, want one of {', '.join(SPELLMOD_OPS)}")
        keys = values["target"].split("|")
        mask = 0
        for key in keys:
            if key not in masks:
                raise BuildError(f"{where}: spellmod{i}_target {key!r} is not a skill of {row['calling']}")
            if not masks[key]:
                why = "has no family flags" if flags_known else "needs --data or --spell-flags for its Spell.dbc flags"
                raise BuildError(f"{where}: spellmod{i}_target {key!r} {why}")
            mask |= masks[key]
        for other, other_mask in masks.items():
            if other not in keys and other_mask and other_mask & mask:
                raise BuildError(f"{where}: spellmod{i}_target {values['target']!r} also reaches {other!r}, "
                                 f"which the cell does not name; name it too")
        mod: dict = {"type": values["type"], "op": values["op"]}
        # One skill's mask goes out as a bit index, as the client patch always has; several as a mask.
        if mask & (mask - 1):
            mod["family_mask"] = mask
        else:
            mod["family_bit"] = mask.bit_length() - 1
        mod["value"] = number(values["value"], f"{where} spellmod{i}_value")
        mod["target"] = keys
        spellmods.append(mod)
    return spellmods


def client_override(row: dict, built: dict, where: str, categories: dict[str, int]) -> dict | None:
    """The row's Spell.dbc override: its client columns, effects 1 to 3 and its spellmods."""
    if "spell_id" not in built or built.get("client_visible") is False:
        return None
    # The Calling's power, so a borrowed spell gates against the right bar.
    fields: dict = {"power_type": CLASS_POWER_TYPE[built["class"]]}
    # The sheet's name replaces the borrowed spell's, so a talent reads as the sheet calls it.
    if row["name"]:
        fields["name"] = row["name"]
    for column in CLIENT_NUMBERS:
        if row[column]:
            fields[column.removeprefix("client_")] = number(row[column], f"{where} {column}")
    for column in CLIENT_FLAGS:
        if row[column]:
            fields[column.removeprefix("client_")] = row[column] == "yes"
    # Cost kind: the amount moves to the field Spell.dbc keeps it in; a health cost is a power type.
    if built["cost_kind"] == "pct_base" and "cost" in fields:
        fields["cost_percent"] = fields.pop("cost")
    elif built["cost_kind"] == "per_sec" and "cost" in fields:
        fields["cost_per_second"] = fields.pop("cost")
    elif built["cost_kind"] == "health":
        fields["power_type"] = HEALTH_POWER_TYPE
    # Cast shape: the attribute bits; interrupt flags and DmgClass only when the cell is set, so a
    # default leaves the borrowed row's own values.
    cast_kind = built["cast_kind"]
    if cast_kind == "next_swing":
        fields["on_next_swing"] = True
    elif cast_kind == "channel":
        fields["channeled"] = True
    elif cast_kind == "auto_repeat":
        fields["auto_repeat"] = True
    if row["interrupt_on"]:
        flags = sum(INTERRUPT_FLAGS[name] for name in built["interrupt_on"])
        fields["channel_interrupt_flags" if cast_kind == "channel" else "interrupt_flags"] = flags
    if row["defense_type"]:
        fields["dmg_class"] = DEFENSE_TYPES[built["defense_type"]]
    fields["school"] = SCHOOLS.index(built["school"])
    fields["prevention_type"] = PREVENTIONS[built["prevention"]]
    if row["requires_weapon"]:
        for key, value in weapon_requirement(row["requires_weapon"], where)[1].items():
            fields["equipped_" + key] = value
    if row["cooldown_category"]:
        fields["category"] = CATEGORY_BASE + categories[row["cooldown_category"]]
        fields["category_cooldown_ms"] = built["category_cooldown_ms"]
    for column, field in (("requires_caster_state", "caster_aura_state"), ("requires_target_state", "target_aura_state")):
        if row[column]:
            fields[field] = AURA_STATES[row[column]]
    if row["description"]:
        fields["description"] = row["description"]
    if row["client_stances"]:
        fields["stances"] = row["client_stances"].split("|")
    # No effects key on an unmodelled row: the client keeps the stock row's effects.
    if built["modelled"]:
        fields["effects"] = [{"kind": e["kind"], "params": e["params"]} for e in built["effects"] if not e["realm_only"]]
    # The client's mods name no skill keys; the realm's do.
    spellmods = [{k: v for k, v in m.items() if k != "target"} for m in built.get("spellmods", [])]
    out = {"spell_id": built["spell_id"], "calling": row["calling"], "key": row["key"], "rank": built["rank"],
           "fields": fields}
    if spellmods:
        out["spellmods"] = spellmods
    return out


def build(rows: list[dict], spell_flags: dict | None = None) -> tuple[list[dict], dict]:
    for column in (rows[0] if rows else {}):
        found = re.fullmatch(r"spellmod(\d+)_\w+", column)
        if found and int(found.group(1)) > MAX_SPELLMODS:
            raise BuildError(f"column {column}: a row has at most {MAX_SPELLMODS} spellmod groups")
    if rows and "client_on_next_swing" in rows[0]:
        raise BuildError("column client_on_next_swing is gone; write cast_kind=next_swing in the cast_kind column")
    if rows and "requires_out_of_combat" in rows[0]:
        raise BuildError("column requires_out_of_combat is gone; write `requires: combat=out` in the requires column")
    masks = skill_masks(rows, spell_flags)
    # Each cooldown_category name gets a number in order of first appearance; every row sharing a
    # name must carry the same category_cooldown_ms.
    categories: dict[str, int] = {}
    category_ms: dict[str, tuple[str, str]] = {}
    for line, row in enumerate(rows, start=2):
        name = row["cooldown_category"]
        if not name:
            continue
        categories.setdefault(name, len(categories))
        first = category_ms.setdefault(name, (row["category_cooldown_ms"], f"line {line} ({row['calling']}/{row['key']})"))
        if row["category_cooldown_ms"] != first[0]:
            raise BuildError(f"line {line} ({row['calling']}/{row['key']}) category_cooldown_ms: "
                             f"{row['category_cooldown_ms']!r} differs from {first[0]!r} on {first[1]}, "
                             f"which shares cooldown_category {name!r}")
    callings: dict[str, dict] = {}
    overrides: list[dict] = []
    aura_overrides: list[tuple[dict, str]] = []
    seen: set[tuple] = set()
    realm_ids: dict[str, str] = {}
    talent_rows: dict[tuple, list[tuple[dict, dict, str]]] = {}
    # A calling's aura keys, each with the effect kinds it holds, are read before any row, so row
    # order never matters.
    auras = {c: {r["key"]: {r[f"effect{i}_kind"] for i in range(1, MAX_EFFECTS + 1) if r[f"effect{i}_kind"]}
                 for r in rows if r["kind"] == "aura" and r["calling"] == c}
             for c in {r["calling"] for r in rows}}
    for line, row in enumerate(rows, start=2):
        where = f"line {line} ({row['calling']}/{row['key']})"
        kind = row["kind"]
        if kind not in ROW_KINDS:
            raise BuildError(f"{where}: kind is {kind!r}, want one of {', '.join(ROW_KINDS)}")
        if not row["calling"] or not row["key"]:
            raise BuildError(f"{where}: calling and key are required")
        if row["status"] not in STATUSES:
            raise BuildError(f"{where}: status is {row['status']!r}, want one of {', '.join(STATUSES)}")
        if kind != "aura":
            for column in AURA_ONLY_COLUMNS:
                if row[column]:
                    raise BuildError(f"{where} {column}: set on a {kind} row; only aura rows carry it")
        row_class = class_id(row["class"], where)
        calling = callings.setdefault(row["calling"], {"key": row["calling"], "class": row_class, "skills": [],
                                                      "talent_tabs": [], "auras": []})
        if calling["class"] != row_class:
            raise BuildError(f"{where}: rows of {row['calling']} disagree on class "
                             f"({calling['class']} and {row_class})")
        if kind == "calling":
            if "name" in calling:
                raise BuildError(f"{where}: second calling row for {row['calling']}")
            calling.update(build_calling(row, where))
            continue
        if kind == "talent_tab":
            tab = build_talent_tab(row, where)
            tab["key"] = row["key"]
            if any(t["key"] == tab["key"] or t["name"] == tab["name"] or t["id"] == tab["id"]
                   for t in calling["talent_tabs"]):
                raise BuildError(f"{where}: duplicate talent_tab {tab['name']!r}")
            calling["talent_tabs"].append(tab)
            continue
        built = build_row(row, where, auras[row["calling"]])
        if row["realm_spell_id"]:
            if row["realm_spell_id"] in realm_ids:
                raise BuildError(f"{where}: realm_spell_id {row['realm_spell_id']} is already used by "
                                 f"{realm_ids[row['realm_spell_id']]}")
            realm_ids[row["realm_spell_id"]] = where
        if kind == "talent":
            built["max_rank"] = max_rank(row, where)
        elif row["max_rank"]:
            raise BuildError(f"{where}: max_rank is set on a {kind} row; only talent ranks carry it")
        identity = (row["calling"], kind, row["key"], built["rank"])
        if identity in seen:
            raise BuildError(f"{where}: duplicate {kind} rank")
        seen.add(identity)
        if kind == "aura":
            aura = build_aura(row, built, where)
            calling["auras"].append(aura)
            if override := aura_override(row, aura, where):
                aura_overrides.append((override, where))
            continue
        spellmods = build_spellmods(row, where, masks.get(row["calling"], {}), spell_flags is not None)
        if spellmods and kind == "aura":
            raise BuildError(f"{where}: spellmod cells are set on an aura row; only skill and talent rows carry them")
        if spellmods:
            built["spellmods"] = spellmods
        override = client_override(row, built, where, categories)
        if override:
            overrides.append(override)
        del built["class"]
        if kind == "skill":
            calling["skills"].append(built)
        else:
            talent_rows.setdefault((row["calling"], row["key"]), []).append((row, built, where))

    # An aura's Spell.dbc fields join the override of a skill that is the same spell, whatever the row order.
    for extra, where in aura_overrides:
        merge_override(overrides, extra, where)

    for calling in callings.values():
        resolve_amount_of(calling)
        resolve_next_cast(calling, masks.get(calling["key"], {}))

    for (calling_key, key), ranks in talent_rows.items():
        calling = callings[calling_key]
        first, _, where = ranks[0]
        for column in ("tab", "tab_row", "tab_column"):
            values = {r[column] for r, _, _ in ranks}
            if len(values) != 1 or not first[column]:
                raise BuildError(f"{where}: talent {key} has no single {column}")
        tab_name = first["tab"]
        seat = {"row": whole(first["tab_row"], f"{where} tab_row"),
                "column": whole(first["tab_column"], f"{where} tab_column")}
        tab = next((t for t in calling["talent_tabs"] if t.get("key", t["name"]) == tab_name), None)
        if tab is None:
            if any("id" in t for t in calling["talent_tabs"]):
                raise BuildError(f"{where}: talent {key} tab {tab_name!r} is not a talent_tab key of {calling['key']}")
            tab = {"name": tab_name, "talents": []}
            calling["talent_tabs"].append(tab)
        for other in tab["talents"]:
            if other["seat"] == seat:
                raise BuildError(f"{where}: talent {key} sits on {other['key']}'s seat in {tab_name}")
        talent = {"key": key, "name": first["name"], "seat": seat,
                  "exceptional": first["exceptional"] == "yes",
                  "ranks": sorted((b for _, b, _ in ranks), key=lambda b: b["rank"])}
        wanted = {b["max_rank"] for b in talent["ranks"]}
        if len(wanted) != 1:
            raise BuildError(f"{where}: talent {key} ranks disagree on max_rank ({sorted(wanted)})")
        if [b["rank"] for b in talent["ranks"]] != list(range(1, wanted.pop() + 1)):
            raise BuildError(f"{where}: talent {key} ranks are not 1..max_rank")
        for b in talent["ranks"]:
            del b["max_rank"]
        if first["prereq_key"]:
            talent["prereq"] = {"key": first["prereq_key"], "rank": whole(first["prereq_rank"] or "1", where)}
        tab["talents"].append(talent)

    for calling in callings.values():
        for tab in calling["talent_tabs"]:
            for talent in tab["talents"]:
                prereq = talent.get("prereq")
                if not prereq:
                    continue
                target = next((t for t in tab["talents"] if t["key"] == prereq["key"]), None)
                where = f"{calling['key']}/{talent['key']}"
                if target is None:
                    raise BuildError(f"{where}: prereq {prereq['key']!r} is not a talent in {tab['name']}")
                if prereq["rank"] > len(target["ranks"]):
                    raise BuildError(f"{where}: prereq {prereq['key']} has no rank {prereq['rank']}")
    seats = []
    for calling in callings.values():
        for tab in calling["talent_tabs"]:
            for talent in tab["talents"]:
                seat = {"calling": calling["key"], "key": talent["key"], "tab": tab["name"], **talent["seat"]}
                if "talent_id" in talent["ranks"][0]:
                    seat["talent_id"] = talent["ranks"][0]["talent_id"]
                seat["rank_spell_ids"] = [r.get("spell_id") for r in talent["ranks"]]
                if "prereq" in talent:
                    seat["prereq"] = talent["prereq"]
                seats.append(seat)
    return list(callings.values()), {"spells": overrides, "talents": seats}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--csv", default=DEFAULT_CSV)
    parser.add_argument("--out-dir", default=DEFAULT_OUT_DIR)
    parser.add_argument("--data", default=os.environ.get("WOW_DATA"), help="game Data folder for the DBC check")
    parser.add_argument("--no-dbc-check", action="store_true", help="skip checking ids against the DBCs")
    parser.add_argument("--spell-flags", help="JSON {spell id: SpellFamilyFlags} standing in for Spell.dbc")
    args = parser.parse_args(argv)

    with open(args.csv, newline="", encoding="utf-8") as f:
        # A row shorter than the header (columns added at the end) reads its missing cells as empty.
        rows = [{k: v or "" for k, v in row.items() if k is not None} for row in csv.DictReader(f)]
    try:
        server, patch = build(rows, read_spell_flags(args.data, args.spell_flags, rows))
        if not args.no_dbc_check:
            check_dbc_ids(rows, args.data)
            resolve_dbc_rows(patch, **read_row_tables(args.data))
            print(f"DBC check: every id is in the DBCs in {args.data}")
    except BuildError as error:
        print(f"build_spells: {error}", file=sys.stderr)
        return 1

    # A talent's ranks usually share one note; print it once.
    carried = list(dict.fromkeys((f"{row['calling']}/{row['key']}", row["assumptions"]) for row in rows if row["assumptions"]))
    counts = {s: sum(1 for row in rows if row["status"] == s) for s in STATUSES}
    print("status: " + ", ".join(f"{n} {s}" for s, n in counts.items()))
    for row in rows:
        if row["status"] == "wip":
            print(f"  wip: {row['calling']}/{row['key']}" + (f" rank {row['rank']}" if row["rank"] else ""))
    print(f"{len(carried)} rows carry assumptions:")
    for who, text in carried:
        print(f"  {who}: {text}")
    for name, data in (("server_spells.json", server), ("client_dbc_patch.json", patch)):
        with open(os.path.join(args.out_dir, name), "w", encoding="utf-8", newline="\n") as f:
            json.dump(data, f, indent=1, ensure_ascii=False)
            f.write("\n")
    print(f"wrote {args.out_dir}: {len(server)} Callings, {len(rows)} rows, "
          f"{len(patch['spells'])} spell overrides, {len(patch['talents'])} talent seats")
    return 0


if __name__ == "__main__":
    sys.exit(main())
