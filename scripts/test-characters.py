"""Replace every Twilight's Eve character with one level-60 test character per Calling.

Each one stands in the Test Grounds facing the dummies, wears its Calling's outfit
(crates/twilight-realm/content/starting_outfits.toml, worn by the same slot rule the realm
uses; an entry that is not gear, such as arrows, is a stack in the backpack), carries a hearthstone, and has its Calling's talents at every
rank, as many as a level-60 save's 59 points hold. The save folder is backed up first.

    ~/.local/bin/python3.15 scripts/test-characters.py [save-folder]

The save folder defaults to benilla-config/twilight, where play/ copies keep their saves.
Run it with the game closed: a running game saves over the files on exit.
"""

import csv
import json
import shutil
import sys
import time
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HOME = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "benilla-config" / "twilight"

POINTS_AT_SIXTY = 59
# The Test Grounds' start (crates/twilight-realm/content/test_grounds.toml), on the ground.
START = "map = 0\nzone = 12\nposition = [-9662.0, 255.0, 47.5]\norientation = 0.0"
FIRST_GUID = 1
NAMES = {
    "basic_warrior": "Basicwar",
    "vanguard": "Vanguard",
    "zealot": "Zealot",
    "cutthroat": "Cutthroat",
    "initiate": "Initiate",
    "ranger": "Ranger",
    "acolyte": "Acolyte",
}
HEARTHSTONE = 6948
# Equipment slots, as `rules::gear::equip_slots` places gear.csv's slot column.
EQUIP = {"Chest": 4, "Weapon": 15, "Offhand": 16, "Ranged": 17}
# The backpack's first slot.
BACKPACK = 23


def items_for(calling, outfits, gear):
    rows, bag = [], BACKPACK
    for entry in outfits.get(calling, {}).get("items", []):
        g = gear.get(str(entry))
        if g is None:
            # Not gear: an ammo stack (Rough Arrow) that goes in the backpack.
            rows.append((bag, entry, 200, 0))
            bag += 1
            continue
        slot = EQUIP[g["slot"]]
        count = 200 if g["type"] == "Thrown" else 1
        rows.append((slot, entry, count, int(g["durability"] or 0)))
    rows.append((bag, HEARTHSTONE, 1, 0))
    return rows


def main():
    spells = json.loads((ROOT / "data/twilights-eve/server_spells.json").read_text("utf-8"))
    outfits = tomllib.loads(
        (ROOT / "crates/twilight-realm/content/starting_outfits.toml").read_text("utf-8")
    )
    with open(ROOT / "data/twilights-eve/gear.csv", encoding="utf-8") as f:
        gear = {r["entry"]: r for r in csv.DictReader(f)}

    if HOME.exists():
        backup = HOME.with_name(f"{HOME.name}.backup-{time.strftime('%Y%m%d-%H%M%S')}")
        shutil.copytree(HOME, backup)
        print(f"backed up to {backup}")
        for old in HOME.glob("character-*.toml"):
            old.unlink()
    HOME.mkdir(parents=True, exist_ok=True)

    guids = []
    for guid, c in enumerate(spells, FIRST_GUID):
        points, talents = 0, []
        for tab in c["talent_tabs"]:
            for t in tab["talents"]:
                ranks = len(t["ranks"])
                if points + ranks <= POINTS_AT_SIXTY:
                    talents.append(f"[{t['ranks'][0]['talent_id']}, {ranks}]")
                    points += ranks
        items = items_for(c["key"], outfits, gear)
        body = "\n".join(
            [
                "version = 2",
                f"guid = {guid}",
                f'name = "{NAMES[c["key"]]}"',
                "race = 1",
                "gender = 0",
                "look = [0, 0, 0, 0, 0]",
                f'calling = "{c["key"]}"',
                "level = 60",
                "xp = 0",
                START,
                "action_bar = []",
                "money = 100000",
                f"talents = [{', '.join(talents)}]",
                "talent_wipes = 0",
                "",
                "[bind]",
                "map = 0",
                "position = [-9464.0, 62.0, 56.0]",
                "orientation = 0.0",
                "area = 87",
            ]
        )
        for slot, entry, count, durability in items:
            body += (
                f"\n\n[[items]]\nslot = {slot}\nentry = {entry}\ncount = {count}"
                f"\ndurability = {durability}"
            )
        (HOME / f"character-{guid}.toml").write_text(body + "\n", "utf-8", newline="\n")
        guids.append(str(guid))
        print(f"{NAMES[c['key']]:<10} {c['key']:<14} {points:>2} pts, {len(items)} items")
    (HOME / "characters.txt").write_text("\n".join(guids) + "\n", "utf-8", newline="\n")


if __name__ == "__main__":
    main()
