"""Run a cargo command that can outlast the Bash tool's 10-minute limit, from a headless session.

A builder's session cannot background a command (it never hears back) and cannot run one past
10 minutes (the tool kills it, and a cargo stopped mid-compile corrupts the build). This starts
cargo detached, writing to a log, and lets the session check on it in short calls:

    python3.15 -I scripts/cargo-long.py start test -q -p benilla-app --lib mist
    python3.15 -I scripts/cargo-long.py wait <id>      # blocks up to 8 minutes, then reports

`start` prints an id. `wait` prints "running" with the log's last lines, or "done, exit N" with
the lines that matter (errors, test results). Call `wait` again until it says done. A client
build (a benilla-* crate, the twilights-eve launcher, or the whole workspace) first takes the
machine-wide client lock (scripts/client-lock.ps1) and holds it while cargo runs.
"""
import json, os, re, subprocess, sys, time, uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUNS = ROOT / "target" / "agent" / "cargo-long"
LOCK_PS = ROOT / "scripts" / "client-lock.ps1"
WAIT_S = 480


def is_client(args):
    s = " ".join(args)
    if re.search(r"(-p|--package)[\s=]+(benilla-|twilights-eve\b)", s) or re.search(r"--workspace|--all\b", s):
        return True
    return not re.search(r"(-p|--package)[\s=]", s)


def ps(command):
    return subprocess.run(["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command",
                           f". '{LOCK_PS}'; {command}"], capture_output=True, text=True)


def start(args):
    run_id = uuid.uuid4().hex[:8]
    holder = f"cargo-long-{run_id}"
    client = is_client(args)
    if client:
        r = ps(f"if (Enter-ClientLock '{holder}' 0 8) {{ exit 0 }} else {{ $h = Get-ClientLockHolder; Write-Output $h.holder; exit 1 }}")
        if r.returncode != 0:
            print(f"client lock busy (held by {r.stdout.strip()}); nothing started. Try start again in a few minutes.")
            return 1
    RUNS.mkdir(parents=True, exist_ok=True)
    log, code = RUNS / f"{run_id}.log", RUNS / f"{run_id}.exit"
    line = "cargo " + subprocess.list2cmdline(args)
    bat = RUNS / f"{run_id}.cmd"
    bat.write_bytes((f'@echo off\r\ncd /d "{ROOT}"\r\n{line} > "{log}" 2>&1\r\n'
                     f'echo %errorlevel% > "{code}"\r\n').encode())
    # The tool that runs this script kills its child processes when the call ends (a Windows job
    # object), so the build must break away from that job to outlive the call.
    detached, new_group, breakaway = 0x00000008, 0x00000200, 0x01000000
    try:
        subprocess.Popen(["cmd", "/c", str(bat)], cwd=ROOT, creationflags=detached | new_group | breakaway,
                         stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except OSError:
        subprocess.Popen(["cmd", "/c", str(bat)], cwd=ROOT, creationflags=detached | new_group,
                         stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    (RUNS / f"{run_id}.json").write_text(json.dumps({"args": args, "client": client, "holder": holder,
                                                     "started": time.time()}))
    if client:
        # From here the lock holds while a client cargo process runs, even if this session dies.
        time.sleep(3)
        ps(f"Exit-ClientLock '{holder}' -Background")
    print(f"started {run_id}: {line}")
    print(f"next: python3.15 -I scripts/cargo-long.py wait {run_id}")
    return 0


def report(log, done):
    text = log.read_text(encoding="utf-8", errors="replace") if log.exists() else ""
    lines = text.splitlines()
    if not done:
        return "\n".join(lines[-5:])
    keep = [l for l in lines if re.search(r"^error|^warning: unused|FAILED|panicked|test result|^\s+-->|could not compile", l)]
    return "\n".join((keep or lines)[-40:])


def wait(run_id):
    meta_path = RUNS / f"{run_id}.json"
    if not meta_path.exists():
        print(f"no run {run_id}")
        return 1
    meta = json.loads(meta_path.read_text())
    log, code = RUNS / f"{run_id}.log", RUNS / f"{run_id}.exit"
    deadline = time.time() + WAIT_S
    while time.time() < deadline:
        if code.exists() and code.read_text().strip():
            exit_code = code.read_text().strip()
            if meta["client"]:
                ps(f"Exit-ClientLock '{meta['holder']}'")
            mins = (time.time() - meta["started"]) / 60
            print(f"done, exit {exit_code}, after {mins:.0f} min")
            print(report(log, True))
            return 0 if exit_code == "0" else 1
        time.sleep(10)
    mins = (time.time() - meta["started"]) / 60
    print(f"running, {mins:.0f} min so far. Call wait {run_id} again.")
    print(report(log, False))
    return 2


def main():
    if len(sys.argv) < 3 or sys.argv[1] not in ("start", "wait"):
        print(__doc__)
        return 1
    return start(sys.argv[2:]) if sys.argv[1] == "start" else wait(sys.argv[2])


if __name__ == "__main__":
    sys.exit(main())
