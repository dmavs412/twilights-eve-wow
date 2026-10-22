"""Check a release manifest against the files beside it (#310, ADR 0016).

    python3.15 scripts/check-manifest.py target/release-stage/b-<short>

Reads <dir>/manifest.json in the shape crates/twilights-launcher/src/manifest.rs expects and
confirms it has no BOM, names the three exes, and that each listed file's SHA-256 (lowercase)
and size match the file on disk. Exits 1 on the first problem.
"""

import hashlib
import json
import sys
from pathlib import Path

EXES = ("twilights-eve.exe", "twilights-launcher.exe", "twilight-server.exe")


def check(stage: Path) -> list[str]:
    problems = []
    raw = (stage / "manifest.json").read_bytes()
    if raw.startswith(b"\xef\xbb\xbf"):
        problems.append("manifest.json starts with a BOM")
        raw = raw[3:]
    manifest = json.loads(raw.decode("utf-8"))
    for key in ("tag", "commit", "files"):
        if key not in manifest:
            problems.append(f"manifest has no {key}")
    files = manifest.get("files", {})
    for exe in EXES:
        if exe not in files:
            problems.append(f"manifest has no entry for {exe}")
    tag, commit = manifest.get("tag", ""), manifest.get("commit", "")
    if len(commit) != 40 or not commit.startswith(tag.removeprefix("b-")):
        problems.append(f"tag {tag!r} does not match commit {commit!r}")
    for name, entry in files.items():
        path = stage / name
        if not path.is_file():
            problems.append(f"{name} is listed but missing")
            continue
        data = path.read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        if entry.get("sha256") != digest:
            problems.append(f"{name}: hash {digest}, manifest says {entry.get('sha256')}")
        if entry.get("size") != len(data):
            problems.append(f"{name}: size {len(data)}, manifest says {entry.get('size')}")
    return problems


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    problems = check(Path(sys.argv[1]))
    for p in problems:
        print(p)
    if not problems:
        print(f"{sys.argv[1]}: manifest matches its files")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
