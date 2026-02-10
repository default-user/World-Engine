# ADR 0001: Architecture — One Kernel, Two Embodiment Modes

**Status:** Accepted
**Date:** 2026-02-10
**Authors:** Ande Turner, Claude Code

## Context

We are building a persistent, authoritative world engine. The engine must support desktop
interaction as the primary mode and VR as an optional second embodiment mode. Both modes must
share the same world truth, persistence layer, and authoring operations.

## Decision

### Core Architecture

The engine uses a **single world kernel** that owns all authoritative state. Embodiment modes
(desktop, VR) are input/output adapters that do not fork world logic.

```
┌─────────────┐     ┌─────────────┐
│  Desktop IO │     │   VR IO     │  (optional, feature-flagged)
└──────┬──────┘     └──────┬──────┘
       │                   │
       └───────┬───────────┘
               │ Actions
       ┌───────▼───────┐
       │  World Kernel  │  ← authoritative state, deterministic stepping
       └───────┬───────┘
               │
    ┌──────────┼──────────┐
    │          │          │
┌───▼──┐  ┌───▼───┐  ┌───▼────┐
│Persist│  │Stream │  │ Render │
└──────┘  └───────┘  └────────┘
```

### Module Responsibilities

| Module   | Responsibility |
|----------|---------------|
| `kernel` | World state, entities, transforms, simulation stepping |
| `persist`| Snapshot + event log, rollback, branching edits |
| `stream` | World partition, cell streaming, LOD budgets |
| `author` | Non-destructive editing, undo/redo, commit edits |
| `render` | Renderer-agnostic interface (wgpu backend) |
| `input`  | Action graph shared by Desktop and VR |
| `tools`  | Inspector, timeline scrubber, profiling |
| `common` | Shared types: `EntityId`, `Transform`, etc. |

### Key Invariants

1. **World truth is kernel-owned.** Renderers and tools derive state; they never mutate it directly.
2. **Deterministic mode.** Simulation step is pure with respect to inputs (seeded RNG, no time-of-day).
3. **Append-only event log.** All mutations produce event records for persistence and replay.
4. **VR does not fork logic.** VR is gated behind `cargo feature = vr` and only adds input mapping.
5. **Edits are reversible.** All authoring operations support undo/redo via event records.

### Data Model (v0.1)

- **Entity:** `EntityId` (UUID v4) + `Transform` (position, rotation, scale) + sparse component sets.
- **Persistence:** CBOR snapshots (content-addressed) + CBOR append-only event log.
- **Terrain:** Heightfield chunks (256x256 samples), compressed per chunk.

## Consequences

- Desktop mode is always complete and functional; VR can lag behind.
- Persistence and replay depend on the event log being a faithful record of all mutations.
- Renderer plugins are possible because the render interface is abstract.
- Deterministic replay is limited to kernel logic; full physics determinism is deferred.

## Alternatives Considered

1. **ECS framework (e.g., bevy_ecs).** Deferred to keep the kernel minimal and under our control
   for v0.1. May adopt later if the component model grows complex.
2. **Separate VR kernel.** Rejected. Shared kernel is a core architectural requirement.
3. **File-based persistence (JSON/YAML).** Rejected for world state. CBOR is more compact and
   supports schema versioning. YAML is used only for human-readable manifests.
