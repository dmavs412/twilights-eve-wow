# Releasing

How a build reaches friends (ADR 0016). GitHub Releases on this repo is the patch server. Each
build is one release tagged `b-<short-commit>` carrying `twilights-eve.exe`,
`twilights-launcher.exe`, `twilight-server.exe` and `manifest.json`. Testing is the newest
pre-release. Live is the release GitHub marks latest. The Launcher reads a channel from
`launcher.toml` beside its exe and installs whatever that channel points at, after checking
every file's SHA-256 against the manifest.

## Shipping

1. Commit to `main` and push. The release tag must point at a commit GitHub has.
2. `.\scripts\release-testing.ps1 -Notes "<patch notes>"`. It refuses a dirty tree, an
   unpushed HEAD and a tag that already exists. It takes the build lock, builds the three
   exes in release, stages them with the manifest under `target\release-stage\b-<short>\`,
   and creates the pre-release. Nobody on Live sees it.
3. Play it. Set `channel = "testing"` in the `launcher.toml` beside a Launcher and start the
   Launcher. It patches itself to the Testing build.
4. `.\scripts\promote-live.ps1`. It flips the newest pre-release to Live, then restarts the
   host server on the same build (next section). Every Launcher on Live patches on its next
   start.

Add `-DryRun` to step 2 to build and stage without creating a release. Check a staged
manifest with:

```
python3.15 scripts/check-manifest.py target/release-stage/b-<short>
```

## Rolling back

`.\scripts\promote-live.ps1 -Commit <short>` makes that older release Live again and restarts
the host server on it. Launchers on Live follow it on their next start. Nothing is rebuilt.

## The host folder

Davis's server runs from `C:\Users\davis\twilight-host` (override with `$env:TWILIGHT_HOST_DIR`):

- `twilight-server.exe`, placed there by `promote-live.ps1` from the Live release.
- `host.json`, read by `scripts\host-server.ps1`:
  `{"listen": "0.0.0.0:3724", "join_password": "...", "data": "C:\\Users\\davis\\wow-data\\Data"}`.
- `server-data\`, the saves. Creating `server-data\stop` saves everyone and stops the server.

`promote-live.ps1` asks a running host server to stop through that stop file, waits up to
60 s, swaps in the new exe and starts `host-server.ps1` in a new window. If the server does
not stop in time the old exe stays running and the release is Live anyway; the script says
so, and the fix is to stop the server by hand and run `.\scripts\host-server.ps1`.

The first `host-server.ps1` run writes the `host.json` template and exits. Fill in the join
password, then run it again. A server that ran from elsewhere before keeps its saves in its
own `server-data`; copy that folder into the host folder by hand before the first start.

## What to send a friend

The link to `twilights-launcher.exe` on the latest release:
`https://github.com/dmavs412/twilights-eve-wow/releases/latest`. Tell them to put it in its
own folder, and that Windows will warn on the first start because the exe is unsigned: click
"More info", then "Run anyway". The Launcher asks for their 1.12.1 `Data` folder, then
patches and plays. To join Davis's realm they set `mode = "join"`, `server = "<host>:3724"`
and `join_password = "..."` in `launcher.toml`.
