"""Paste the story rewrite's reviewed text into the realm's content files.

The draft lives in docs/specs/story-rewrite/[1-6]-*.md, one file per band. This script reads
every quest field, talk reply and note page there and writes it into
crates/twilight-realm/content/quests_elwynn.toml, quests.toml and quest_items.toml. A field the
draft marks "(unchanged)" is left as it is.

The draft's grammar, which the parser relies on:

- A quest starts at a heading `## 94005 Title` or `### 64009 Title` (a five-digit id).
- `- **Title:**`, `**Objectives:**`, `**Details:**`, `**Progress:**` or `**Completion:**`
  followed by the text. Continuation lines are indented by two spaces.
- `- **<Name>'s reply (npc 295):**` followed by the text: that NPC's `reply` in the quest's
  `talk` list.
- `- **980030, <landmark>:** "page one" / "page two"`: a note's `pages`, anywhere in a file.

Usage, from the repo root:

    python3.15 -I scripts/story-text.py          # write the content files
    python3.15 -I scripts/story-text.py --check  # report what would change, write nothing

It exits 1 and writes nothing when a quest id, field, talk row or note in the draft has no match
in the content files. Run it again after a write and it reports nothing to change.
"""

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DRAFT = ROOT / "docs" / "specs" / "story-rewrite"
CONTENT = ROOT / "crates" / "twilight-realm" / "content"
QUEST_FILES = ["quests_elwynn.toml", "quests.toml"]
ITEM_FILE = "quest_items.toml"

FIELDS = {
    "Title": "title",
    "Objectives": "objectives",
    "Details": "details",
    "Progress": "progress",
    "Completion": "completion",
}
TOML_STRING = r'"(?:[^"\\]|\\.)*"'

HEADING = re.compile(r"^#{2,3} (\d{5}) ")
FIELD = re.compile(r"^- \*\*(Title|Objectives|Details|Progress|Completion):\*\* (.*)$")
REPLY = re.compile(r"^- \*\*[^*]* reply \(npc (\d+)\):\*\* (.*)$")
NOTE = re.compile(r'^- \*\*(\d{6}), [^*]*:\*\* "(.*)" / "(.*)"$')


def bullets(lines):
    """Join each bullet with its two-space continuation lines; yield the other lines as they are."""
    current = None
    for line in lines:
        if current is not None and line.startswith("  ") and line.strip():
            current += " " + line.strip()
            continue
        if current is not None:
            yield current
            current = None
        if line.startswith("- "):
            current = line.rstrip()
        else:
            yield line.rstrip()
    if current is not None:
        yield current


def parse_draft():
    """Return (quests, notes): quests maps id to {"fields": {...}, "replies": {...}}."""
    quests, notes = {}, {}
    files = sorted(DRAFT.glob("[1-9]-*.md"))
    if not files:
        sys.exit(f"no draft files in {DRAFT}")
    for path in files:
        quest = None
        for line in bullets(path.read_text(encoding="utf-8").splitlines()):
            heading = HEADING.match(line)
            if heading:
                quest = quests.setdefault(int(heading.group(1)), {"fields": {}, "replies": {}})
                continue
            if line.startswith("#"):
                quest = None
            note = NOTE.match(line)
            if note:
                notes[int(note.group(1))] = [note.group(2), note.group(3)]
                continue
            if quest is None:
                continue
            field = FIELD.match(line)
            if field:
                text = field.group(2).strip()
                if not text.endswith("(unchanged)"):
                    quest["fields"][FIELDS[field.group(1)]] = text
                continue
            reply = REPLY.match(line)
            if reply:
                quest["replies"][int(reply.group(1))] = reply.group(2).strip()
    return quests, notes


def toml_string(text):
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


def quest_block(lines, quest_id):
    """The [start, end) line range of the quest whose `id = quest_id`, or None."""
    for i, line in enumerate(lines):
        if line == f"id = {quest_id}":
            start = i
            end = next(
                (j for j in range(i + 1, len(lines)) if lines[j].startswith("[[")),
                len(lines),
            )
            return start, end
    return None


def apply(quests, notes):
    """Edit the content files in memory. Return ({file: new text}, changes, errors)."""
    texts = {name: (CONTENT / name).read_bytes().decode("utf-8") for name in QUEST_FILES}
    texts[ITEM_FILE] = (CONTENT / ITEM_FILE).read_bytes().decode("utf-8")
    lines = {name: text.split("\n") for name, text in texts.items()}
    changes, errors = [], []

    for quest_id, quest in sorted(quests.items()):
        found = None
        for name in QUEST_FILES:
            block = quest_block(lines[name], quest_id)
            if block:
                found = (name, *block)
                break
        if not found:
            errors.append(f"quest {quest_id}: no `id = {quest_id}` in {', '.join(QUEST_FILES)}")
            continue
        name, start, end = found
        rows = lines[name]
        for field, text in quest["fields"].items():
            hits = [i for i in range(start, end) if rows[i].startswith(f"{field} = ")]
            if len(hits) != 1:
                errors.append(f"quest {quest_id}: {len(hits)} `{field}` lines in {name}")
                continue
            new = f"{field} = {toml_string(text)}"
            if rows[hits[0]] != new:
                rows[hits[0]] = new
                changes.append(f"{name}: quest {quest_id} {field}")
        for npc, text in quest["replies"].items():
            hits = [i for i in range(start, end) if f"{{ npc = {npc}," in rows[i]]
            if len(hits) != 1:
                errors.append(f"quest {quest_id}: {len(hits)} talk rows for npc {npc} in {name}")
                continue
            row = rows[hits[0]]
            new = re.sub(
                r"reply = " + TOML_STRING,
                lambda _: "reply = " + toml_string(text),
                row,
                count=1,
            )
            if new == row and "reply = " not in row:
                errors.append(f"quest {quest_id}: npc {npc}'s talk row has no reply")
            elif new != row:
                rows[hits[0]] = new
                changes.append(f"{name}: quest {quest_id} reply from npc {npc}")

    items = lines[ITEM_FILE]
    for entry, pages in sorted(notes.items()):
        hits = [i for i, row in enumerate(items) if f"{{ entry = {entry}," in row]
        if len(hits) != 1:
            errors.append(f"note {entry}: {len(hits)} rows in {ITEM_FILE}")
            continue
        row = items[hits[0]]
        array = "pages = [" + ", ".join(toml_string(page) for page in pages) + "]"
        pattern = r"pages = \[(?:" + TOML_STRING + r"(?:, )?)*\]"
        if not re.search(pattern, row):
            errors.append(f"note {entry}: its row has no pages")
            continue
        new = re.sub(pattern, lambda _: array, row, count=1)
        if new != row:
            items[hits[0]] = new
            changes.append(f"{ITEM_FILE}: note {entry} pages")

    out = {name: "\n".join(rows) for name, rows in lines.items()}
    return {name: text for name, text in out.items() if text != texts[name]}, changes, errors


def main():
    check = "--check" in sys.argv[1:]
    quests, notes = parse_draft()
    edited, changes, errors = apply(quests, notes)
    for name, text in edited.items():
        try:
            tomllib.loads(text)
        except tomllib.TOMLDecodeError as err:
            errors.append(f"{name} would not parse after the edit: {err}")
    for error in errors:
        print("error:", error)
    if errors:
        sys.exit(1)
    for change in changes:
        print(change)
    print(f"{len(changes)} change(s) in {len(edited)} file(s)" + (" (check only)" if check else ""))
    if not check:
        for name, text in edited.items():
            (CONTENT / name).write_bytes(text.encode("utf-8"))


if __name__ == "__main__":
    main()
