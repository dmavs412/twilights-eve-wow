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
    "mod_health_regen_out_of_combat", "mod_stat", "periodic_damage", "proc_on_cast",
    "proc_on_kill", "seal", "spell_damage", "stun", "weapon_strike",
}
FORMULA_FIELDS = ("base", "per_level")
OPTIONAL_FORMULA_FIELDS = ("per_ap", "min")
# What an `energize` may name in `power`.
POWERS = ("rage", "energy", "mana")

REALM_NUMBERS = (
    "realm_cost", "realm_cooldown_ms", "realm_gcd_ms", "realm_range_min", "realm_range_max",
    "realm_radius_yd",
)
ID_COLUMNS = ("spell_id", "talent_id", "source_tab_id", "icon_id")
CLIENT_NUMBERS = (
    "client_cost", "client_cooldown_ms", "client_gcd_ms", "client_cast_time_ms", "client_range_min",
    "client_range_max", "client_duration_ms", "borrowed_from_class", "borrowed_family_bit",
)
CLIENT_FLAGS = ("client_on_next_swing", "client_not_in_combat")
SPELLMOD_COLUMNS = ("type", "op", "family_bit", "value")


class BuildError(Exception):
    pass


# The id columns the DBC check verifies, and the table each one must exist in.
DBC_ID_COLUMNS = {
    "spell_id": "Spell.dbc",
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


def build_row(row: dict, where: str) -> dict:
    modelled = row["modelled"]
    if modelled not in ("yes", "no"):
        raise BuildError(f"{where}: modelled is {modelled!r}, want yes or no")
    out: dict = {"key": row["key"], "name": row["name"], "rank": number(row["rank"], f"{where} rank")}
    for column in ID_COLUMNS:
        if row[column]:
            out[column] = number(row[column], f"{where} {column}")
            if not isinstance(out[column], int):
                raise BuildError(f"{where} {column}: {row[column]!r} is not a whole id")
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
        effects.append({"kind": kind, "params": parsed, "realm_only": i > CLIENT_EFFECTS})
    out["modelled"] = modelled == "yes"
    out["effects"] = effects if out["modelled"] else []
    out["description"] = row["description"]
    return out


def client_override(row: dict, built: dict, where: str) -> dict | None:
    """The row's Spell.dbc override: its client columns, effects 1 to 3 and its spellmods."""
    if "spell_id" not in built:
        return None
    fields: dict = {}
    for column in CLIENT_NUMBERS:
        if row[column]:
            fields[column.removeprefix("client_")] = number(row[column], f"{where} {column}")
    for column in CLIENT_FLAGS:
        if row[column]:
            fields[column.removeprefix("client_")] = row[column] == "yes"
    if row["client_stances"]:
        fields["stances"] = row["client_stances"].split("|")
    # No effects key on an unmodelled row: the client keeps the stock row's effects.
    if built["modelled"]:
        fields["effects"] = [{"kind": e["kind"], "params": e["params"]} for e in built["effects"] if not e["realm_only"]]
    spellmods = []
    for i in (1, 2):
        values = {c: row[f"spellmod{i}_{c}"] for c in SPELLMOD_COLUMNS}
        if not any(values.values()):
            continue
        if not all(values.values()):
            raise BuildError(f"{where}: spellmod{i} needs all of {', '.join(SPELLMOD_COLUMNS)}")
        if values["type"] not in ("flat", "pct"):
            raise BuildError(f"{where}: spellmod{i}_type is {values['type']!r}, want flat or pct")
        spellmods.append({"type": values["type"], "op": values["op"],
                          "family_bit": number(values["family_bit"], f"{where} spellmod{i}_family_bit"),
                          "value": number(values["value"], f"{where} spellmod{i}_value")})
    out = {"spell_id": built["spell_id"], "calling": row["calling"], "key": row["key"], "rank": built["rank"],
           "fields": fields}
    if spellmods:
        out["spellmods"] = spellmods
    return out


def build(rows: list[dict]) -> tuple[list[dict], dict]:
    callings: dict[str, dict] = {}
    overrides: list[dict] = []
    seen: set[tuple] = set()
    talent_rows: dict[tuple, list[tuple[dict, dict, str]]] = {}
    for line, row in enumerate(rows, start=2):
        where = f"line {line} ({row['calling']}/{row['key']})"
        kind = row["kind"]
        if kind not in ("skill", "talent"):
            raise BuildError(f"{where}: kind is {kind!r}, want skill or talent")
        if not row["calling"] or not row["key"]:
            raise BuildError(f"{where}: calling and key are required")
        built = build_row(row, where)
        identity = (row["calling"], kind, row["key"], built["rank"])
        if identity in seen:
            raise BuildError(f"{where}: duplicate {kind} rank")
        seen.add(identity)
        override = client_override(row, built, where)
        if override:
            overrides.append(override)
        calling = callings.setdefault(row["calling"], {"key": row["calling"], "skills": [], "talent_tabs": []})
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
        seat = {"row": number(first["tab_row"], where), "column": number(first["tab_column"], where)}
        tab = next((t for t in calling["talent_tabs"] if t["name"] == tab_name), None)
        if tab is None:
            tab = {"name": tab_name, "talents": []}
            calling["talent_tabs"].append(tab)
        for other in tab["talents"]:
            if other["seat"] == seat:
                raise BuildError(f"{where}: talent {key} sits on {other['key']}'s seat in {tab_name}")
        talent = {"key": key, "name": first["name"], "seat": seat,
                  "exceptional": first["exceptional"] == "yes",
                  "ranks": sorted((b for _, b, _ in ranks), key=lambda b: b["rank"])}
        if [b["rank"] for b in talent["ranks"]] != list(range(1, len(ranks) + 1)):
            raise BuildError(f"{where}: talent {key} ranks are not 1..{len(ranks)}")
        if first["prereq_key"]:
            talent["prereq"] = {"key": first["prereq_key"], "rank": number(first["prereq_rank"] or "1", where)}
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
    args = parser.parse_args(argv)

    with open(args.csv, newline="", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))
    try:
        server, patch = build(rows)
        if not args.no_dbc_check:
            check_dbc_ids(rows, args.data)
            print(f"DBC check: every id is in the DBCs in {args.data}")
    except BuildError as error:
        print(f"build_spells: {error}", file=sys.stderr)
        return 1

    for name, data in (("server_spells.json", server), ("client_dbc_patch.json", patch)):
        with open(os.path.join(args.out_dir, name), "w", encoding="utf-8", newline="\n") as f:
            json.dump(data, f, indent=1, ensure_ascii=False)
            f.write("\n")
    print(f"wrote {args.out_dir}: {len(server)} Callings, {len(rows)} rows, "
          f"{len(patch['spells'])} spell overrides, {len(patch['talents'])} talent seats")
    return 0


if __name__ == "__main__":
    sys.exit(main())
