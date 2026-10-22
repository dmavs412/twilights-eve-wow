# The Launcher and two release channels on GitHub Releases

Davis gives friends a copy of the game, and each fix means sending them a new exe by hand. A
friend on an old build cannot join his realm, because the server refuses a client whose build id
differs from its own (#144). Spec #306 adds a Launcher that patches the game before it starts,
and two release channels, so friends only get builds Davis has played first.

- **A separate Launcher exe.** `crates/twilights-launcher` builds `twilights-launcher.exe`, the
  file a friend opens. It patches `game/twilights-eve.exe` beside it, then starts the game and
  closes. The game exe never updates itself. The Launcher has a window built with eframe, at the
  egui version the workspace already locks.
- **Content stays compiled in, so a patch is whole exes.** Content and the DBC patch are inside
  `twilights-eve.exe`, and each friend supplies their own 1.12.1 `Data` folder. A release
  therefore carries no game data and no diff format. It carries `twilights-eve.exe`,
  `twilights-launcher.exe`, `twilight-server.exe` and `manifest.json`.
- **GitHub Releases on this repo is the patch server.** The repo is public, so the Launcher reads
  the GitHub API with no token. Davis's PC does not need to be on for a friend to patch. Each
  build is one release tagged `b-<short-commit>`.
- **Two channels, read from release flags.** Testing is a pre-release made by
  `scripts/release-testing.ps1`. Live is the release GitHub marks latest, set by
  `scripts/promote-live.ps1`, which changes flags on the same release and never rebuilds. Live
  is therefore the byte-identical exe that was tested. Only Davis's Launcher follows Testing,
  through `channel = "testing"` in its settings file. The window has no channel picker.
- **A manifest of SHA-256 hashes, no code signing.** `manifest.json` lists each file's hash and
  size. The Launcher downloads to a temporary name, checks the hash and only then renames the
  file into place. A mismatch leaves the installed file alone.
- **The Launcher replaces itself by renaming.** Windows lets a running exe be renamed but not
  overwritten. The Launcher renames itself to `twilights-launcher.old.exe`, moves the new file
  in, starts it and exits. The next start deletes the old file.
- **The host server follows Live.** `promote-live.ps1` stops Davis's `twilight-server` through
  a stop file, which saves every character online. It then swaps in the server exe from the same
  release and starts it again. The server and Live therefore always share a build id.

## Considered options

- **The game exe updates itself.** Rejected in #306. One file is simpler, but the window
  with a progress bar, the data folder picker and the Solo / Join fields belong before the game
  loads.
- **Serve patches from Davis's PC.** Rejected: friends could not patch while it is off, and it
  needs an open port.
- **Every push to main goes to Testing through CI.** Rejected: unattended runs push many
  unplayed commits a day. A build reaches Testing only when Davis runs the script.
- **Rebuild a commit when promoting it.** Rejected: the Live exe would not be the file that was
  tested.
- **Pay for code signing.** Rejected for now. A friend clicks past SmartScreen once, on the first
  download of the Launcher. The Launcher writes later patches itself, so they get no warning.
- **Patch diffs instead of whole exes.** Rejected: the exe is the only file that changes, and a
  diff format is a second thing to get wrong.

## Consequences

- Shipping to friends is two commands: `release-testing.ps1`, play it, then `promote-live.ps1`.
- A rollback is `promote-live.ps1 <commit>` on an older release.
- An unauthenticated GitHub API call is limited to 60 an hour per IP address. The Launcher makes
  one or two per start.
- The join password sits in plain text in the Launcher's settings file. It guards a friends
  server, not an account.
- A friend's first install is `twilights-launcher.exe` alone. It downloads the game on first run.
