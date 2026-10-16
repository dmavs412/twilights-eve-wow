# Twilight's Eve: Callings

A single-player, offline WoW-style ORPG: a from-scratch World of Warcraft 1.12.1 client in Rust
and Bevy, playing against a Local Realm that runs inside the client process. The realm is built so
it can later sit behind a socket as a server for friends.

The client began as [samwhosung/benilla](https://github.com/samwhosung/benilla); every file here
is now ours. The game data is a WoW 1.12.1 client's `Data` folder, which is never committed.

## Running it

```powershell
$env:WOW_DATA = 'C:\path\to\WoW\Data'
$env:WOW_TWILIGHT = '1'
cargo run --release -p warcraft-survivors
```

`run.ps1 native` does the same with this machine's defaults.

## Layout

| path | what |
|---|---|
| `crates/benilla-*` | the client: formats, world, UI, protocol, app |
| `crates/twilight-realm` | the Local Realm: the game's rules, tested with no game data |
| `crates/benilla-app/src/twilight/` | the glue between the client and the realm |
| `data/twilights-eve/` | the master CSV and the files generated from it |
| `tools/twilights-eve/` | the Python scripts that read the DBCs and build the data |
| `docs/` | specs, ADRs, reference notes, and the architecture guides |

## Working here

`CLAUDE.md` holds the rules for agents and people alike. The three architecture guides are
`docs/spell_pipeline.md`, `docs/bevy_guidelines.md` and `docs/dbc_structure.md`.

Licensed MIT OR Apache-2.0. Not affiliated with Blizzard Entertainment.
