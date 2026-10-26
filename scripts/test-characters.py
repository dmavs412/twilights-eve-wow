"""Replace every Twilight's Eve character with one level-60 test character per Calling.

Each one stands in the Test Grounds facing the dummies, wears its Calling's outfit
(crates/twilight-realm/content/starting_outfits.toml, worn by the same slot rule the realm
uses; an entry that is not gear, such as arrows, is a stack in the backpack), carries a hearthstone, and has its Calling's talents at every
rank, as many as a level-60 save's 59 points hold. The save folder is backed up first.

    ~/.local/bin/python3.15 scripts/test-characters.py [--greens LEVEL] [save-folder]

`--greens LEVEL` makes them level LEVEL instead, with LEVEL - 1 talent points, each wearing its
Calling's GREEN_KITS: the green of each world Loot Band line (gear.csv's `line`) at the highest
level at or below LEVEL, so `--greens 20` dresses them in what a level-20 kill drops.

The save folder defaults to benilla-config/twilight, where play/ copies keep their saves.
Run it with the game closed: a running game saves over the files on exit.
"""

import argparse
import csv
import json
import shutil
import sys
import time
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

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
ROUGH_ARROW = 2512
# Equipment slots, as `rules::gear::equip_slots` places gear.csv's slot column, the first free one
# first: a second one-hander goes in the off hand.
EQUIP = {
    "Amulet": [1],
    "Chest": [4],
    "Ring": [10, 11],
    "Weapon": [15, 16],
    "Offhand": [16],
    "Ranged": [17],
}
# The backpack's first slot.
BACKPACK = 23
MIGHT = ["world-amulet-might", "world-ring-might-a", "world-ring-might-b"]
SWIFT = ["world-amulet-swift", "world-ring-swift-a", "world-ring-swift-b"]
MIND = ["world-amulet-mind", "world-ring-mind-a", "world-ring-mind-b"]
# Each Calling's `--greens` kit, by world Loot Band line: its outfit's armour and weapons, and
# the jewellery of its main stat. A bow brings a stack of Rough Arrows.
GREEN_KITS = {
    "basic_warrior": ["world-chest-mail", "world-sword", "world-shield", "world-bow", *MIGHT],
    "vanguard": ["world-chest-plate", "world-sword", "world-shield", "world-bow", *MIGHT],
    "zealot": ["world-chest-plate", "world-sword", "world-shield", "world-bow", *MIGHT],
    "cutthroat": ["world-chest-leather", "world-dagger", "world-dagger", "world-bow", *SWIFT],
    "initiate": ["world-chest-cloth", "world-staff", *MIND],
    "ranger": ["world-chest-mail", "world-dagger", "world-bow", *SWIFT],
    "acolyte": ["world-chest-cloth", "world-mace", "world-held", *MIND],
}


def green_kit(calling, level, gear):
    """The entries of `calling`'s green kit at the highest level at or below `level`."""
    entries = []
    for line in GREEN_KITS[calling]:
        greens = [
            g
            for g in gear.values()
            if g["line"] == line
            and g["quality"] == "Uncommon"
            and int(g["required_level"]) <= level
        ]
        if not greens:
            sys.exit(f"no green {line} at level {level} or below")
        entries.append(int(max(greens, key=lambda g: int(g["required_level"]))["entry"]))
    if "world-bow" in GREEN_KITS[calling]:
        entries.append(ROUGH_ARROW)
    return entries


def items_for(entries, gear):
    rows, bag, worn = [], BACKPACK, set()
    for entry in entries:
        g = gear.get(str(entry))
        if g is None:
            # Not gear: an ammo stack (Rough Arrow) that goes in the backpack.
            rows.append((bag, entry, 200, 0))
            bag += 1
            continue
        slot = next(s for s in EQUIP[g["slot"]] if s not in worn)
        worn.add(slot)
        count = 200 if g["type"] == "Thrown" else 1
        rows.append((slot, entry, count, int(g["durability"] or 0)))
    rows.append((bag, HEARTHSTONE, 1, 0))
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--greens", type=int, metavar="LEVEL")
    parser.add_argument("home", nargs="?", type=Path, default=ROOT / "benilla-config" / "twilight")
    args = parser.parse_args()
    HOME = args.home
    level = args.greens or 60
    # One Talent Point a level from level 2 (`rules::talents::points_at`).
    points_at_level = level - 1

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
                if points + ranks <= points_at_level:
                    talents.append(f"[{t['ranks'][0]['talent_id']}, {ranks}]")
                    points += ranks
        if args.greens:
            entries = green_kit(c["key"], level, gear)
        else:
            entries = outfits.get(c["key"], {}).get("items", [])
        items = items_for(entries, gear)
        body = "\n".join(
            [
                "version = 2",
                f"guid = {guid}",
                f'name = "{NAMES[c["key"]]}"',
                "race = 1",
                "gender = 0",
                "look = [0, 0, 0, 0, 0]",
                f'calling = "{c["key"]}"',
                f"level = {level}",
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
