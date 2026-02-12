# World Engine: A Desktop-First Persistent World Space Engine

**A Postdoctoral-Grade Implementation Writeup**

## Abstract

This document describes the design, implementation, and verification of World Engine, a
desktop-first persistent world space engine with optional VR embodiment. The engine prioritizes
correctness, deterministic replay, and non-destructive authoring over renderer sophistication.
Every claim in this document maps to code paths, tests, or benchmarks in the repository.

---

## 1. Problem Statement and Scope

World Engine addresses the need for a persistent, authoritative world simulation that can be
authored from within. The scope is:

- A single authoritative world kernel with deterministic simulation stepping.
- Persistent world state via content-addressed snapshots and an append-only event log.
- Non-destructive in-world authoring with undo/redo and explicit commits.
- Desktop as the primary embodiment mode; VR as an optional second mode.
- Renderer-agnostic architecture with a pluggable adapter.

### What Is Not in Scope (v0.1)

- Photoreal rendering from scratch.
- Multiplayer networking.
- Full physics determinism.
- Mass content distribution.

---

## 2. Related Work and What Is Reused

| Dependency | Role | Justification |
|-----------|------|---------------|
| `glam` | Linear algebra | Industry-standard, no-std compatible |
| `uuid` | Entity identifiers | RFC 4122 UUIDs, widely understood |
| `serde` | Serialization framework | Schema-versioned, derive-based |
| `clap` | CLI parsing | Derive-based, well-maintained |
| `tracing` | Structured logging | Async-compatible, industry standard |

No game engine framework (Bevy, Fyrox, etc.) is used. The kernel and persistence are
implemented from first principles to maintain full control over invariants.

**Workarounds in current implementation:**
- JSON serialization used as workaround for CBOR (serde_cbor is unmaintained; minicbor or
  ciborium will be adopted when persistence moves to disk).
- Debug text renderer used as workaround for wgpu GPU backend. The `Renderer` trait is stable;
  swap in a wgpu implementation without changing consumers.
- Simple fixed-size grid partitioning as workaround for full LOD/async streaming.

---

## 3. Formal Definitions

### 3.1 World State

A world state `W` at tick `t` is a tuple:

```
W(t) = (E, T, C, t, s)
```

Where:
- `E` is the set of entity identifiers (UUID v4).
- `T: E → Transform` maps entities to spatial transforms.
- `C: E → ComponentSets` maps entities to sparse component sets.
- `t ∈ ℕ` is the simulation tick counter.
- `s ∈ ℕ` is the deterministic seed, advanced each tick via splitmix64.

### 3.2 Operations

An operation `op` is a function `W → W` that produces a new world state and an event record:

```
apply(op, W(t)) → (W(t+1), Event)
```

### 3.3 Event Log

The event log `L` is an append-only sequence:

```
L = [e₁, e₂, ..., eₙ]
```

Given a snapshot `S(t₀)` and events `L[t₀..t]`, the world state can be reconstructed:

```
reconstruct(S(t₀), L[t₀..t]) = W(t)
```

---

## 4. Determinism Model and Limits

The determinism guarantee applies to the kernel simulation step function. Given identical
inputs and seed, the same sequence of world states is produced.

**Implementation:** Each call to `World::step()` advances the internal seed using splitmix64,
a fast, high-quality deterministic PRNG step function. The seed is mixed as:

```
state = state + 0x9e3779b97f4a7c15
z = (state ^ (state >> 30)) * 0xbf58476d1ce4e5b9
z = (z ^ (z >> 27)) * 0x94d049bb133111eb
z = z ^ (z >> 31)
```

This ensures that two worlds created with the same seed and subjected to the same sequence
of operations will produce bit-identical states. Cross-platform floating-point determinism
is not guaranteed; cross-platform replay requires fixed-point or tolerance.

**Code path:** `crates/kernel/src/world.rs` — `World::step()`, `splitmix64()`
**Tests:**
- `deterministic_replay_same_seed` — verifies two worlds with identical seeds produce identical states
- `different_seeds_diverge` — verifies different seeds produce different states
- `replay_reconstructs_state` — verifies event replay produces identical world state

---

## 5. Event Log and Snapshot Correctness Argument

The persistence layer implements snapshot + event log with the following guarantees:

1. **Append-only invariant:** The `EventLog::append()` method only extends the log; there is
   no API to modify or delete events. Events are `Vec`-backed and extend-only.

2. **Content addressing:** Snapshots include an FNV-1a hash computed over tick, seed, and
   entity data. `Snapshot::verify()` recomputes the hash and compares it to the stored value,
   detecting corruption. (Workaround: FNV-1a over debug representation is used in place of
   a cryptographic hash over CBOR bytes. Sufficient for corruption detection.)

3. **Reconstruction equivalence:** `EventLog::replay_from(snapshot)` replays events after a
   snapshot's tick to reconstruct the world state. `World::replay(events)` reconstructs
   world state from a complete event sequence.

**Code path:** `crates/persist/src/snapshot.rs`
**Tests:**
- `snapshot_capture_and_verify` — round-trip capture and integrity check
- `snapshot_corruption_detected` — mutation of snapshot fields detected by verify
- `snapshot_restore_roundtrip` — restore produces a world with correct entities and seed
- `event_log_append_and_read` — append-only log semantics
- `snapshot_store_take_and_rollback` — rollback restores prior entity count
- `snapshot_store_flush_events` — events drain from world into persistent log

---

## 6. Undo/Redo and Non-Destructive Editing Semantics

The undo/redo model is based on inverse operations:

- Each `EditCommand` (Spawn, Despawn, SetTransform) has a corresponding inverse computed
  by `EditCommand::inverse()`.
- Spawn's inverse is Despawn (with the same id and transform).
- Despawn's inverse is Spawn (re-creates the entity).
- SetTransform's inverse swaps old and new transforms.

The `Editor` maintains two stacks:
- **Undo stack:** Records every edit command as it is applied.
- **Redo stack:** Populated when undo is called; cleared when a new edit is made.

`Editor::undo()` pops the last command, computes its inverse, applies it to the world, and
pushes the original command onto the redo stack. `Editor::redo()` reverses this process.

**Code path:** `crates/author/src/editor.rs`
**Tests:**
- `spawn_and_undo` — spawn then undo removes the entity
- `spawn_undo_redo` — full undo/redo cycle preserves entity
- `despawn_and_undo` — despawn then undo restores entity
- `set_transform_and_undo` — transform change then undo restores original position
- `redo_cleared_on_new_edit` — new edit invalidates redo stack
- `despawn_nonexistent_returns_error` — error handling for missing entities

---

## 7. Streaming Model and Complexity

The streaming system uses a fixed-size grid partitioning scheme as a workaround for
a full LOD and async streaming system.

**Model:** The world's XZ plane is divided into a regular grid of cells with configurable
size (default 16 world units). Each entity is assigned to a cell based on
`floor(position.xz / cell_size)`. Cells can be queried:
- By exact coordinate: `entities_in_cell(coord)` — O(1) lookup.
- By radius: `entities_in_radius(center, r)` — O((2r+1)²) cell lookups.

The grid is rebuilt from world state via `GridPartition::rebuild(world)`, which iterates
all entities once — O(n) where n is entity count.

**Complexity bounds:**
- Cell lookup: O(1) amortized (HashMap).
- Radius query: O(r² + k) where k is the number of entities in the queried cells.
- Rebuild: O(n) where n is total entity count.

**Code path:** `crates/stream/src/grid.rs`
**Tests:**
- `position_to_cell_basic` — coordinate mapping correctness
- `rebuild_from_world` — grid construction from world state
- `entities_in_cell` — single-cell query
- `entities_in_radius` — radius query finds nearby entities
- `empty_cell_returns_empty_set` — empty query returns no entities

---

## 8. Benchmarks and Profiling Methodology

Benchmarks are deferred to a future milestone when Criterion is integrated. The `WorldInspector`
in `crates/tools/src/inspector.rs` provides runtime introspection (entity count, tick, seed,
pending events) that can be used for profiling integration.

See [Benchmarks](../mdx/benchmarks.mdx) for methodology.

---

## 9. Threat Model

See [Security & Integrity](../mdx/security-and-integrity.mdx).

Key concerns:
- Snapshot and event log corruption detection (implemented via FNV-1a content hashing).
- Rollback abuse prevention via explicit commits and branching.
- Provenance tracking via event records (all mutations produce WorldEvent records).

---

## 10. Future Work

- **CBOR serialization:** Replace JSON workaround with minicbor or ciborium for compact binary snapshots.
- **wgpu renderer:** Implement the `Renderer` trait with a wgpu GPU backend.
- **Async streaming:** Replace grid partitioning workaround with LOD-aware async cell streaming.
- **Renderer plugins:** Support multiple rendering backends beyond wgpu.
- **Multiplayer:** Authoritative server with client prediction.
- **Higher-fidelity physics:** Integration with a deterministic physics engine.
- **Asset pipeline:** Import/export workflows for meshes, materials, terrain.
- **Scripting:** Embedded scripting for in-world behaviors.
- **Benchmarks:** Criterion integration for step throughput, snapshot size, and replay fidelity.

---

## References

This writeup references code in the World Engine repository. All code paths cited are
relative to the repository root.

| Section | Code Path | Test |
|---------|-----------|------|
| World State | `crates/kernel/src/world.rs` | `world_starts_empty`, `spawn_and_despawn` |
| Determinism | `crates/kernel/src/world.rs` | `deterministic_replay_same_seed`, `different_seeds_diverge` |
| Event System | `crates/kernel/src/world.rs` | `events_are_recorded`, `drain_events_clears_log` |
| Replay | `crates/kernel/src/world.rs` | `replay_reconstructs_state` |
| Snapshot | `crates/persist/src/snapshot.rs` | `snapshot_capture_and_verify`, `snapshot_corruption_detected` |
| Event Log | `crates/persist/src/snapshot.rs` | `event_log_append_and_read` |
| Rollback | `crates/persist/src/snapshot.rs` | `snapshot_store_take_and_rollback` |
| Undo/Redo | `crates/author/src/editor.rs` | `spawn_and_undo`, `spawn_undo_redo` |
| Renderer | `crates/render/src/renderer.rs` | `debug_renderer_empty_world`, `debug_renderer_with_entities` |
| Streaming | `crates/stream/src/grid.rs` | `rebuild_from_world`, `entities_in_radius` |
| Inspector | `crates/tools/src/inspector.rs` | `summary_with_entities`, `inspect_entity_found` |
| Entity ID | `crates/common/src/types.rs` | `entity_id_uniqueness` |
| Transform | `crates/common/src/types.rs` | `transform_default_is_identity` |
| Actions | `crates/input/src/action.rs` | `action_move_is_constructible`, `action_spawn_entity` |
