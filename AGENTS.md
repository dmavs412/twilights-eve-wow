# Working in this repository

Two offline game modes on a from-scratch WoW 1.12.1 client in Rust + Bevy: **Survivors**, a
roguelite, and **Twilight's Eve**, a WoW-style RPG played against an in-process Local Realm. The
client began as [samwhosung/benilla](https://github.com/samwhosung/benilla) and its browser build
as [Arnesen/wenilla](https://github.com/Arnesen/wenilla). Nothing is merged from either any more:
every file here is ours, so edit any of them directly.

Read these before changing client code: `docs/METHOD.md` (how work is done), `docs/MAP.md`
(where things are) and `docs/CONTRIBUTING.md` (the gates). The domain vocabulary is in
`GLOSSARY.md`, the decisions are in `docs/adr/`, and the specs are in `docs/specs/`.

## What lives where

| path | what |
|---|---|
| `crates/benilla-*` | the client: formats, world, UI, protocol, app |
| `crates/benilla-app/src/survivors/` | Survivors |
| `crates/benilla-app/src/twilight/` | Twilight's Eve's glue: the boot flag, and the realm on the net layer's in-process backend |
| `crates/twilight-realm` | the Local Realm: Twilight's Eve's rules, free of Bevy, tested with no game data |
| `crates/warcraft-survivors` | the native launcher, for both modes (`WOW_TWILIGHT=1` picks Twilight's Eve) |
| `crates/wenilla` | the wasm entry crate (`?twilight=1` in the URL picks Twilight's Eve) |
| `crates/wenilla-host` | the local dev server: static files and `/data/*`. Loopback only. |
| `web/` | the page side: `index.html`, `boot.js`, `platform.js`, `bridge.js`, `wasi_stubs.js` |
| `third_party/` | vendored forks (Lua, mlua-sys, kira). Each records its delta from its original in `BENILLA.md`. |

## Commands

```bash
scripts/gates.sh            # the full gate chain: fmt, clippy -D warnings, tests, doc and lint scripts
cargo test -p twilight-realm                # the realm, in seconds; no game data
cargo check --target wasm32-unknown-unknown -p wenilla --no-default-features --features webgpu
                            # the wasm build without linking, as CI runs it
scripts/web-setup.sh        # once per machine: wasm target, wasm-bindgen-cli, wasi-sdk, binaryen
scripts/web-build.sh        # → web/dist. WEB_DEBUG=1 keeps symbols for readable stack traces.
cargo run --release -p wenilla-host -- --www web/dist --data /path/to/WoW/Data
                            # http://127.0.0.1:8090/?twilight=1
```

A browser for testing needs WebGPU; the world does not run without it. Without an adapter the
client aborts in `wgpu::create_bind_group_layout` at world entry: that is the missing adapter, not
your change.

## Invariants

- **No threads and no std time on wasm.** `std::time::Instant`, `SystemTime` and `env::vars_os`
  panic there; use `bevy::platform::time::Instant` or `web-time`. Only the wasm `cargo check`
  catches this.
- **The `wasm-bindgen` crate and CLI versions match exactly.** `web-setup.sh` enforces this.
- **What ships is the `cp` line in `scripts/web-build.sh`.** A page-side file missing from it
  never reaches `web/dist`.
- **A vendored fork's `BENILLA.md` lists every hunk.** A change to a `third_party/` source adds
  its row.
- **`ring_reaction` returns 0..7**, and every consumer adds `+ 1` for the Lua 1..8 scale.

## How a change lands

Branch from `main` and open a pull request with `gh pr create -R dmavs412/twilights-eve-wow`. A branch is done when
`scripts/gates.sh` is green on it: CI runs the gates only for pull requests from forks, so for
this repository's own branches the local run is the guardrail.

## Where to look when

| symptom | look at |
|---|---|
| the wasm panics at boot with `RuntimeError: unreachable` | rebuild with `WEB_DEBUG=1`; check for a WebGPU adapter first; `?bridge=0` rules the bridge out |
| wasm-bindgen glue errors at load | version drift: rerun `scripts/web-setup.sh` |
| a debug build exits 127 on `Assertion failed … lapi.c` | the `lua apicheck:` lines above it name the call chain and the stack (`third_party/lua-src/BENILLA.md`) |
| no sound in the browser | the page resumes the AudioContext on the first gesture; the per-target mixer backend is in `sound/mixer.rs` |
| the world freezes ~200 ms at zone lines in the browser | the soundscape loader, `sound/zone.rs` and `sound/web_load.rs` |
| short stalls in the browser on the first play of a sound | the deferred SFX path, `sound/kit/deferred.rs` |
| driving a run with no one at the keyboard | `WOW_PROBE=list` lists the probe variables and the create and select drivers |
