# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- Rust workspace scaffolding with 8 library crates and 2 application crates.
- `worldspace-common`: shared types (`EntityId`, `Transform`).
- `worldspace-kernel`: world state with spawn/despawn/step and basic tests.
- `worldspace-kernel`: deterministic stepping with seeded RNG (splitmix64).
- `worldspace-kernel`: append-only event log (`WorldEvent`) for all mutations.
- `worldspace-kernel`: event replay via `World::replay()` for state reconstruction.
- `worldspace-kernel`: `set_transform()` with old/new tracking for undo support.
- `worldspace-input`: action enum expanded with SpawnEntity, DespawnEntity, Select, Deselect, Undo, Redo.
- `worldspace-persist`: content-addressed snapshots with FNV-1a integrity verification.
- `worldspace-persist`: append-only `EventLog` with replay support.
- `worldspace-persist`: `SnapshotStore` with snapshot/rollback (in-memory workaround for disk I/O).
- `worldspace-author`: `Editor` with full undo/redo via inverse `EditCommand` operations.
- `worldspace-author`: `EditCommand` variants: Spawn, Despawn, SetTransform.
- `worldspace-render`: `Renderer` trait (renderer-agnostic interface).
- `worldspace-render`: `DebugTextRenderer` (text-based workaround for wgpu GPU backend).
- `worldspace-render`: `RenderView` camera configuration.
- `worldspace-stream`: `GridPartition` fixed-size cell partitioning (workaround for full LOD).
- `worldspace-stream`: `CellCoord` with cell and radius queries.
- `worldspace-tools`: `WorldInspector` with summary, entity inspection, and entity listing.
- `worldspace-desktop`: demonstrates all subsystems (kernel, persistence, authoring, rendering, streaming, tools).
- `worldspace-cli`: `replay` subcommand for deterministic replay verification.
- `worldspace-cli`: `snapshot` subcommand for snapshot/rollback demonstration.
- `worldspace-cli`: `info` subcommand now prints all crate versions.
- CI workflow with fmt, clippy, and test jobs across platforms.
- Documentation skeleton: MDX site pages, ADR 0001, postdoc writeup outline.
- Quality configs: `rustfmt.toml`, `clippy.toml`, `deny.toml`.
- Postdoc writeup filled in: determinism model, snapshot correctness, undo/redo semantics, streaming complexity.

### Changed
- `worldspace-kernel`: `World` now derives `Clone`, `Serialize`, `Deserialize`.
- `worldspace-kernel`: `EntityData` now derives `Serialize`, `Deserialize`.
- `worldspace-kernel`: `World::step()` now advances a deterministic seed and emits `Stepped` events.
- `worldspace-desktop`: now exercises all subsystems instead of just stepping.
- `worldspace-cli`: expanded from 1 to 3 subcommands.
