# Bevy guidelines for the Local Realm

The realm (`crates/twilight-realm`) uses `bevy_ecs` alone: the ECS crate, with no Bevy app,
renderer, window, asset server or input. The client's Bevy and the realm's `bevy_ecs` are the
same version, pinned in the workspace `Cargo.toml`. The realm's public surface stays what ADR
0001 made it: commands in, events out, ticked by its host, ground and line of sight through a
trait. A future server hosts the same `World` behind a socket.

## Components, not spells

An ability is an entity made of generic components, spawned from a `server_spells.json` row. A
talent rank is a set of modifier and proc components attached to the character on learn and
removed on unlearn. There is no `MortalStrike` component and no `fn cast_heroic_strike`.

Each effect `kind` in the CSV maps to one component:

| kind | component | fields |
|---|---|---|
| `weapon_strike` | `WeaponStrike` | weapon roll (`W` or `Wn`), bonus amount, on next swing, unavoidable set, threat bonus |
| `periodic_damage` | `PeriodicDamage` | school, mechanic, tick amount, tick interval, duration |
| `stun` | `Stun` | duration, aura spell id |
| `energize` | `Energize` | power kind (`rage`, `energy`, `mana`), amount; nothing on a bar of another kind |
| `add_combo_points` | `AddComboPoints` | amount, on the target, five at most |
| `per_combo` (a block on a damage or duration kind) | `PerCombo` | the amount added once per combo point; its presence makes the ability a finisher (`Finisher`: needs points, spends them when it lands) |
| `aura_attack_power` | `AuraAttackPower` | amount, duration |
| `healing_reduction` | `HealingReduction` | percent, duration |
| `charge` | `Charge` | speed |
| `seal`, `judgement` | `Seal`, `Judgement` | the seal kind and its judgement spell |

Talents add the generic families: `ProcOnCrit`, `ProcOnHit`, `CleaveOnHit` (a chance and what it
triggers), and modifiers such as `CostMod`, `CritChanceMod`, `DamagePctMod`, `DurationMod`, each
carrying `source: (talent key, rank)` so unlearning removes exactly its own contribution.

A new `kind` means a new component and a new generic system. That is an ECS component family,
so it goes through `/grill-me` first (`CLAUDE.md`).

## Systems are generic and ordered

One system per component kind, each reading its components and writing events. They run in fixed
sets, in this order every tick:

1. **Commands**: the host's `ClientCommand`s become cast requests.
2. **Cast**: cost, cooldown, range and target checks against the caster's components.
3. **Hit**: weapon rolls, hit tables, damage, with modifiers folded in.
4. **Proc**: `ProcOnCrit` and friends react to this tick's hit events.
5. **Auras**: periodic ticks, durations, expiry.
6. **Output**: `SessionEvent`s to the host, in protocol order.

A system reads what its components say and never asks which spell it is. A `match` on a spell id
or a talent key inside a system is exactly the bug the composition rule exists to catch.

## Hidden components

An effect past the third slot arrives with `realm_only: true` and becomes a component like any
other, with one difference: the output set never emits it as an aura slot or a visible spell
effect. The client learns of it only through what it changes (a damage number, a power value).

## Rules stay pure

A vanilla formula is a pure function in `src/rules/` with an exact-value test (ADR 0002). A
system calls the rule; it never inlines the arithmetic.

## Determinism

No `std::time`; the tick count drives every duration and cooldown. Randomness comes from a seeded
RNG resource on the `World`, so a fight replays the same from the same seed. Both are what a
server needs later, and what makes a realm test exact.

## Testing

A realm test builds a `World`, spawns a character and an ability from content, ticks, and asserts
on the events. `cargo test -p twilight-realm` runs in seconds with no game data. Player-script
helpers live in `tests/common/`; a test file defines only what is unique to it
(`CODING_STANDARDS.md`).
