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
| `serde` + `serde_cbor` | Serialization | Schema-versioned binary format |
| `wgpu` | GPU abstraction | Cross-platform, Vulkan/Metal/DX12/WebGPU |
| `clap` | CLI parsing | Derive-based, well-maintained |
| `tracing` | Structured logging | Async-compatible, industry standard |

No game engine framework (Bevy, Fyrox, etc.) is used. The kernel and persistence are
implemented from first principles to maintain full control over invariants.

---

## 3. Formal Definitions

### 3.1 World State

A world state `W` at tick `t` is a tuple:

```
W(t) = (E, T, C, t)
```

Where:
- `E` is the set of entity identifiers (UUID v4).
- `T: E → Transform` maps entities to spatial transforms.
- `C: E → ComponentSets` maps entities to sparse component sets.
- `t ∈ ℕ` is the simulation tick counter.

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

*To be completed in M1 when deterministic stepping with seeded RNG is implemented.*

The determinism guarantee applies to the kernel simulation step function. Given identical
inputs and seed, the same sequence of world states is produced. Floating-point determinism
across platforms is not guaranteed; cross-platform replay requires fixed-point or tolerance.

**Code path:** `crates/kernel/src/world.rs` — `World::step()`
**Test:** `crates/kernel/src/world.rs` — `step_increments_tick` (M0 placeholder)

---

## 5. Event Log and Snapshot Correctness Argument

*To be completed in M2 when persistence is implemented.*

The correctness argument will demonstrate:

1. **Append-only invariant:** Events are never modified after writing.
2. **Content addressing:** Snapshots are hashed; corruption is detected on load.
3. **Reconstruction equivalence:** `reconstruct(S(t₀), L[t₀..t]) = W(t)` holds for all valid logs.

---

## 6. Undo/Redo and Non-Destructive Editing Semantics

*To be completed in M3 when authoring is implemented.*

The undo/redo model will be based on inverse operations:

- Each authoring operation has a corresponding inverse.
- The undo stack records (operation, inverse) pairs.
- Redo replays the original operation.
- Commit finalizes a sequence of operations into a named edit block.

---

## 7. Streaming Model and Complexity

*To be completed in M4 when streaming is implemented.*

---

## 8. Benchmarks and Profiling Methodology

*To be completed as benchmarks are added per milestone.*

See [Benchmarks](../mdx/benchmarks.mdx) for methodology.

---

## 9. Threat Model

See [Security & Integrity](../mdx/security-and-integrity.mdx).

Key concerns:
- Snapshot and event log corruption detection.
- Rollback abuse prevention via explicit commits and branching.
- Provenance tracking via event records.

---

## 10. Future Work

- **Renderer plugins:** Support multiple rendering backends beyond wgpu.
- **Multiplayer:** Authoritative server with client prediction.
- **Higher-fidelity physics:** Integration with a deterministic physics engine.
- **Asset pipeline:** Import/export workflows for meshes, materials, terrain.
- **Scripting:** Embedded scripting for in-world behaviors.

---

## References

This writeup references code in the World Engine repository. All code paths cited are
relative to the repository root.

| Section | Code Path | Test |
|---------|-----------|------|
| World State | `crates/kernel/src/world.rs` | `world_starts_empty`, `spawn_and_despawn` |
| Entity ID | `crates/common/src/types.rs` | `entity_id_uniqueness` |
| Transform | `crates/common/src/types.rs` | `transform_default_is_identity` |
| Actions | `crates/input/src/action.rs` | `action_move_is_constructible` |
