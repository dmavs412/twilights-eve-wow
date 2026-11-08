"""File a spec folder written by a grill as GitHub issues.

A folder holds 00-parent.md (the spec), NN-name.md tickets and 99-index.md (not filed). Each
filed file starts with "Title: <title>" and "Labels: <comma list>". A ticket's title ends in
"(#PARENT)" and its body names other tickets as "NN-name". Cross-spec names are written
"PREFIX:name", and --map PREFIX=<folder> resolves them from that folder's .filed.json.

Pass 1 creates every issue with its title, pass 2 writes each body with every ticket name
replaced by its issue number and a "## Blocked by" heading that lists the blockers as "- #N",
which is the form scripts/overnight-lib.ps1 reads. A ticket whose blockers name a cross-spec
ticket not yet filed is labelled on-hold instead of ready-for-agent; re-run with --refresh once
that spec is filed, and it fixes those bodies and labels. The numbers go to <folder>/.filed.json.

    python3.15 -I scripts/file-spec.py docs/specs/redridge-duskwood
    python3.15 -I scripts/file-spec.py docs/specs/redridge-duskwood --map DUNGEONS=docs/specs/dungeons-30-50 --refresh
    python3.15 -I scripts/file-spec.py docs/specs/x --dry-run
"""
import argparse, json, re, subprocess, sys, tempfile
from pathlib import Path

REPO = "dmavs412/twilights-eve-wow"
NAME = r"\d\d-[a-z0-9][a-z0-9-]*[a-z0-9]"


def gh(*args, input_text=None):
    r = subprocess.run(["gh", *args, "-R", REPO], capture_output=True, text=True, encoding="utf-8")
    if r.returncode != 0:
        sys.exit(f"gh {' '.join(args[:3])} failed: {r.stderr.strip()}")
    return r.stdout.strip()


def parse(path):
    lines = path.read_text(encoding="utf-8").split("\n")
    title = lines[0].removeprefix("Title:").strip()
    labels = [l.strip() for l in lines[1].removeprefix("Labels:").split(",") if l.strip()]
    body = "\n".join(lines[2:]).strip() + "\n"
    return title, labels, body


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("folder")
    ap.add_argument("--map", action="append", default=[], help="PREFIX=folder of another filed spec")
    ap.add_argument("--refresh", action="store_true", help="rewrite bodies and labels of an already filed folder")
    ap.add_argument("--dry-run", action="store_true")
    a = ap.parse_args()

    folder = Path(a.folder)
    files = sorted(p for p in folder.glob("[0-9][0-9]-*.md") if not p.name.startswith("99-"))
    stems = {p.stem for p in files}
    filed_path = folder / ".filed.json"
    filed = json.loads(filed_path.read_text()) if filed_path.exists() else {}

    external = {}
    for m in a.map:
        prefix, other = m.split("=", 1)
        op = Path(other) / ".filed.json"
        external[prefix] = json.loads(op.read_text()) if op.exists() else {}

    # Pass 1: create the issues not filed yet.
    for p in files:
        if p.stem in filed:
            continue
        title, labels, _ = parse(p)
        if a.dry_run:
            filed[p.stem] = 9000 + len(filed)
            print(f"would create #{filed[p.stem]} {p.stem}: {title}")
            continue
        url = gh("issue", "create", "--title", title.replace("(#PARENT)", "(#0)"), "--body", "Filing...")
        filed[p.stem] = int(url.rstrip("/").split("/")[-1])
        print(f"created #{filed[p.stem]} {p.stem}")
        filed_path.write_text(json.dumps(filed, indent=1))
    parent = filed.get("00-parent", 0)

    # Pass 2: bodies, titles and labels.
    for p in files:
        title, labels, body = parse(p)
        n = filed[p.stem]
        blockers, unresolved = [], []
        for sent in re.finditer(r"(?is)blocked by(.*?)(?:\.\s|\.$|\n\n)", body):
            text = sent.group(1)
            for m in re.finditer(NAME, text):
                if m.group(0) in stems and m.group(0) != p.stem:
                    blockers.append(filed[m.group(0)])
            for m in re.finditer(r"#(\d+)", text):
                blockers.append(int(m.group(1)))
            for m in re.finditer(r"([A-Z]+):(" + NAME + r"|[a-z0-9][a-z0-9-]*)", text):
                pre, name = m.group(1), m.group(2)
                ext = external.get(pre, {})
                key = name if name in ext else next((k for k in ext if k.endswith("-" + name) or k[3:] == name), None)
                if key:
                    blockers.append(ext[key])
                else:
                    unresolved.append(f"{pre}:{name}")

        def sub_name(m):
            s = m.group(0)
            return f"#{filed[s]}" if s in stems else s
        out = re.sub(r"(?<![\w/.-])" + NAME + r"(?![\w.])", sub_name, body)
        for pre, ext in external.items():
            def sub_ext(m, ext=ext):
                key = m.group(1) if m.group(1) in ext else next((k for k in ext if k[3:] == m.group(1)), None)
                return f"#{ext[key]}" if key else m.group(0)
            out = re.sub(pre + r":(" + NAME + r"|[a-z0-9][a-z0-9-]*)", sub_ext, out)
        out = out.replace("#PARENT", f"#{parent}")
        blockers = sorted(set(b for b in blockers if b and b != n))
        if p.stem != "00-parent" and (blockers or unresolved):
            out += "\n## Blocked by\n\n" + "".join(f"- #{b}\n" for b in blockers)
            out += "".join(f"- {u} (not filed yet; this ticket is on-hold until it is)\n" for u in unresolved)
        title = title.replace("(#PARENT)", f"(#{parent})")
        if unresolved and "ready-for-agent" in labels:
            labels = [l for l in labels if l != "ready-for-agent"] + ["on-hold"]
        if a.dry_run:
            print(f"{p.stem}: labels={labels} blockers={blockers} unresolved={unresolved}")
            continue
        with tempfile.NamedTemporaryFile("w", suffix=".md", delete=False, encoding="utf-8", newline="\n") as t:
            t.write(out)
        args = ["issue", "edit", str(n), "--title", title, "--body-file", t.name]
        if a.refresh:
            have = json.loads(gh("issue", "view", str(n), "--json", "labels"))["labels"]
            drop = [l["name"] for l in have if l["name"] in ("ready-for-agent", "on-hold") and l["name"] not in labels]
            if drop:
                args += ["--remove-label", ",".join(drop)]
        if labels:
            args += ["--add-label", ",".join(labels)]
        gh(*args)
        Path(t.name).unlink()
        print(f"#{n} {p.stem}: {len(blockers)} blocker(s){', on-hold' if unresolved else ''}")
    if not a.dry_run:
        filed_path.write_text(json.dumps(filed, indent=1))


if __name__ == "__main__":
    main()
