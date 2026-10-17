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
    "add_combo_points", "aura_attack_power", "charge", "energize", "healing_reduction",
    "incapacitate", "judgement", "mod_crit_chance", "mod_crit_damage",
    "mod_health_regen_out_of_combat", "mod_run_speed", "mod_stat", "periodic_damage", "proc",
    "proc_on_cast", "proc_on_kill", "school_damage", "seal", "skill_amount_pct", "skill_crit_pct",
    "skill_rage", "stun", "weapon_strike",
}
ROW_KINDS = ("skill", "talent", "calling", "talent_tab", "aura")
STATS = ("strength", "agility", "stamina", "intellect", "spirit")
ARMOUR_KINDS = ("cloth", "leather", "mail", "plate", "shield")
PROC_EVENTS = ("cast", "kill")
PROC_ACTIONS = ("apply_aura", "heal_pct")
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
CLIENT_FLAGS = ("client_on_next_swing", "client_not_in_combat")
SPELLMOD_COLUMNS = ("type", "op", "target", "value")
MAX_SPELLMODS = 5
# The 1.12.1 client's SpellModOp names (vmangos `SpellModOp`; op 13 and 25 and up do not exist).
SPELLMOD_OPS = (
    "DAMAGE", "DURATION", "THREAT", "EFFECT1", "CHARGES", "RANGE", "RADIUS", "CRITICAL_CHANCE",
    "ALL_EFFECTS", "NOT_LOSE_CASTING_TIME", "CASTING_TIME", "COOLDOWN", "EFFECT2", "COST",
    "CRIT_DAMAGE_BONUS", "RESIST_MISS_CHANCE", "JUMP_TARGETS", "CHANCE_OF_SUCCESS", "ACTIVATION_TIME",
    "EFFECT_PAST_FIRST", "CASTING_TIME_OLD", "DOT", "EFFECT3", "SPELL_BONUS_DAMAGE",
)
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


def build_aura(row: dict, built: dict, where: str) -> dict:
    """An aura row: a borrowed spell id, a duration, charges and effects, granted by key."""
    out = {"key": built["key"], "name": built["name"]}
    for column in ("spell_id", "client_visible", "icon_id", "status"):
        if column in built:
            out[column] = built[column]
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
    if len([a for a in PROC_ACTIONS if a in params]) != 1:
        raise BuildError(f"{where}: a proc needs exactly one of {', '.join(PROC_ACTIONS)}")
    if "apply_aura" in params:
        keys = params["apply_aura"] if isinstance(params["apply_aura"], list) else [params["apply_aura"]]
        for key in keys:
            if key not in auras:
                raise BuildError(f"{where}: apply_aura {key!r} is not an aura row of this Calling")


def check_vocabulary(kind: str, params: dict, where: str) -> None:
    """The CSV uses the realm's names: normal or normalized weapon rolls, a boolean unavoidable."""
    if kind == "weapon_strike":
        if params.get("weapon") not in WEAPON_ROLLS:
            raise BuildError(f"{where}: weapon is {params.get('weapon')!r}, want {' or '.join(WEAPON_ROLLS)}")
        if "unavoidable" in params and not isinstance(params["unavoidable"], bool):
            raise BuildError(f"{where}: unavoidable is {params['unavoidable']!r}, want yes or no")


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
    if "per_combo" in params and not isinstance(params["per_combo"], dict):
        raise BuildError(f"{where}: per_combo is a formula block (per_combo.base, per_combo.per_level)")


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


def build_row(row: dict, where: str, auras: set[str] = frozenset()) -> dict:
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
    if row["requires_out_of_combat"]:
        out["requires_out_of_combat"] = row["requires_out_of_combat"] == "yes"
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
        if kind == "proc":
            check_proc(parsed, auras, here)
        check_vocabulary(kind, parsed, here)
        effects.append({"kind": kind, "params": parsed, "realm_only": i > CLIENT_EFFECTS})
    out["modelled"] = modelled == "yes"
    out["effects"] = effects if out["modelled"] else []
    out["description"] = row["description"]
    return out


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


def client_override(row: dict, built: dict, where: str) -> dict | None:
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
    masks = skill_masks(rows, spell_flags)
    callings: dict[str, dict] = {}
    overrides: list[dict] = []
    seen: set[tuple] = set()
    realm_ids: dict[str, str] = {}
    talent_rows: dict[tuple, list[tuple[dict, dict, str]]] = {}
    # A calling's aura keys are read before any row, so row order never matters.
    auras = {c: {r["key"] for r in rows if r["kind"] == "aura" and r["calling"] == c} for c in
             {r["calling"] for r in rows}}
    for line, row in enumerate(rows, start=2):
        where = f"line {line} ({row['calling']}/{row['key']})"
        kind = row["kind"]
        if kind not in ROW_KINDS:
            raise BuildError(f"{where}: kind is {kind!r}, want one of {', '.join(ROW_KINDS)}")
        if not row["calling"] or not row["key"]:
            raise BuildError(f"{where}: calling and key are required")
        if row["status"] not in STATUSES:
            raise BuildError(f"{where}: status is {row['status']!r}, want one of {', '.join(STATUSES)}")
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
            calling["auras"].append(build_aura(row, built, where))
            continue
        spellmods = build_spellmods(row, where, masks.get(row["calling"], {}), spell_flags is not None)
        if spellmods and kind == "aura":
            raise BuildError(f"{where}: spellmod cells are set on an aura row; only skill and talent rows carry them")
        if spellmods:
            built["spellmods"] = spellmods
        override = client_override(row, built, where)
        if override:
            overrides.append(override)
        del built["class"]
        if kind == "skill":
            calling["skills"].append(built)
        else:
            talent_rows.setdefault((row["calling"], row["key"]), []).append((row, built, where))

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
        rows = list(csv.DictReader(f))
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
